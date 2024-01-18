use psutil::process::processes;
use lumi_utils::timer::{SystemTimer, Timer};
use tokio::{time::{interval, Interval, Duration}, select};
use crate::{transport::{http_client::HttpClient, Transport,TransportIn, HttpClientJson, TransportOut ,mqtt::MqttDriver}, logic::OtaLogic,};
use crate::logic::{OtaLogicOut,OtaLogicIn,HcType};
use tokio::sync::mpsc;
use crate::security::DsaType;
use crate::error::OtaErr;
use tokio::fs::File;
use sysinfo::System;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[derive(Debug)]
pub enum SystemIntergrationErr {
    TranSportErr,
}

pub struct SystemIntergration {
    interval: Interval,
    timer: SystemTimer,
    transport: HttpClient,
    pub logic: OtaLogic,
    dsa: DsaType,
    mqtt: MqttDriver,
}

impl SystemIntergration {
    pub async fn new() -> Self {
        let ota_logic = OtaLogic::new();
        let (tx, rx) = mpsc::channel::<Result<TransportOut, OtaErr>>(5);
        let public_key_path = "public_key.pem";
        let dsa =  DsaType::new("update_ota.bin".to_string(), public_key_path.to_string());

        SystemIntergration {
            interval: interval(Duration::from_millis(100)),
            timer: SystemTimer::default(),
            transport: HttpClient {
               tx: tx,
               rx: rx,
            },
            logic: ota_logic,
            dsa: dsa,
            mqtt: MqttDriver::new(
                "ota".to_string(),
                "localhost".to_string(),
                1883,
                5,  // Thêm tham số keep_alive
            ).await,
        }
    }

    pub async fn recv(&mut self) -> Result<(),OtaErr> {
        select! {
            _ = self.interval.tick() => {
                self.logic.on_tick(self.timer.now_ms());
            },

            event = self.transport.recv() =>{
                self.logic.on_event(OtaLogicIn::Transport(event));
            },

            request_update = self.mqtt.recv() => {
                match request_update
                {
                    Ok(response) => {
                        if response.topic == "master/ota" {
                            if response.message == "true" {
                                self.logic.hc.allow_ota = true;
                            }
                            else {
                                self.logic.hc.allow_ota = false;
                            }
                        }
                    }
                    Err(_) => {

                    }
                }
                
            }
        }
    
        
        while let Some(out) = self.logic.pop_action() {
            match out {
                OtaLogicOut::CheckOtaEvent => {
                    let client = HttpClientJson::new_template();
                    log::info!("Check ota event");
                    let _ = self.transport.send(TransportIn::CheckOtaHc(client)).await;
                }

                OtaLogicOut::UpdateOtaEvent(hc) => {
                    let f_path = "/home/bhien/update_ota.bin".to_string();
                    log::info!("Updating ota for hc");
                    match hc {
                        HcType:: Hc01 => {
                            match File::open(f_path).await {
                                Ok(_) => {
                                    log::info!("Update ota successfully");
                                }
                                Err(e) => {
                                    log::error!("Update ota failed: {}", e);
                                }
                            }
                        }

                        HcType:: Hc02 => {

                        }
                    }
                }

                OtaLogicOut::VerifyEvent => {
                    // Đọc dữ liệu từ file
                    let fpath_ota = "update_ota.bin".to_string();
                    let mut data_file = OpenOptions::new()
                        .read(true)
                        .open(&fpath_ota)
                        .await.unwrap();
                
                    let mut data = Vec::new();
                    let _ = data_file.read_to_end(&mut data).await;
                    
                    // Thực hiện quá trình ký
                    let signature = match self.dsa.sign(&data) {
                        Ok(signature) => signature,
                        Err(_) => {
                            log::error!("Error signing data");
                            return Ok(());
                        }
                    };

                    // Ghi dữ liệu đã ký vào cuối file
                    let mut data_file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(&fpath_ota)
                        .await.unwrap();
                    
                    let _ = data_file.write_all(&signature).await;

                    match self.dsa.verify(signature.len()).await{
                        Ok(()) => {
                            log::info!("Verify successfully");
                        }
                        Err(e) => {
                            if e == OtaErr::VerifyErr {
                                log::error!("Verify processing error");
                                
                            }
                            else if e == OtaErr::VerifyNotEqualErr {
                                log::error!("Verify not equal");
                            }
                        }
                    }
                }

                OtaLogicOut::CompareVersionEvent => {
                    let mut version = Vec::new();
                    let _ = File::open("ota_version.txt") // Update the path to your private key
                        .await.expect("Failed to open version file")
                        .read_to_end(&mut version).await;

                    match String::from_utf8(version) {
                        Ok(s) => {
                            log::info!("Compare versioon successfully");
                            if self.logic.hc.version_name == s {
                                log::info!(
                                    "Version name equal"
                                ); 
                            }else {
                                log::info!(
                                    "Version name not equal"
                                );
                                self.logic.on_event(OtaLogicIn::Push(OtaLogicOut::GetLinkEvent));
                            }
                        }
                        Err(e) => {
                            log::error!("Error compare version: {}", e);
                        }
                    }
                }
                
                OtaLogicOut::GetLinkEvent => {
                    let _ =  self.transport.send(TransportIn::GetLink(self.logic.hc.link.clone())).await;
                }

                OtaLogicOut::KeepAliveEvent => {
                    let _ =  self.transport.send(TransportIn::KeepAlive).await;
                }

                OtaLogicOut::SuppentEvent => {
                    let processes = processes().unwrap();
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    log::info!("{:>6} {:>4}", "PID", "%CPU");
                    let mut cpu_percentages: Vec<(f32, i32)> = Vec::new();

                    for p in processes {
                        if let Ok(mut process) = p {
                            let cpu_percent = process.cpu_percent().unwrap();
                            let pid = process.pid();

                            cpu_percentages.push((cpu_percent, pid as i32));
                        }
                    }
                    
                    cpu_percentages.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

                    //kiểm tra dung lượngg
                    let mut sys = System::new_all();
                    sys.refresh_all();
                    let total_memory = sys.total_memory();
                    let used_memory = sys.used_memory();

                    if total_memory as i64 - used_memory as i64 >= (55000) {
                        //lấy các pid tốn nhất và gửi đi 
                        let mut proccess_suppend: Vec<i32> = Vec::new();
                        if let Some((max_cpu_percent, max_pid)) = cpu_percentages.first() {
                            log::info!("Processe uses the most memory:");
                            log::info!("PID: {}, CPU Percent: {}%", max_pid, max_cpu_percent);
                            proccess_suppend.push(*max_pid);
                        } else {
                            log::error!("No process found.");
                        }

                        if let Some((second_cpu_percent, second_pid)) = cpu_percentages.get(1) {
                            log::info!("Processe uses the second memory:");
                            log::info!("PID: {}, CPU Percent: {}%", second_pid, second_cpu_percent);
                            proccess_suppend.push(*second_pid);

                        } else {
                            log::error!("No process found.");
                        }

                        let _ =  self.transport.send(TransportIn::Suppend(proccess_suppend)).await;
                    }
                }
            }

        }
        Ok(())
    }
}