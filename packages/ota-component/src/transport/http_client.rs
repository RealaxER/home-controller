use crate::error::OtaErr;
use super::{Transport,TransportIn,TransportOut};
use rumqttc::{QoS,Event};
use tokio::sync::mpsc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use reqwest;
use rumqttc::{self, AsyncClient, MqttOptions};
use std::time::Duration;

pub struct HttpClient {
    pub tx: mpsc::Sender<Result<TransportOut, OtaErr>>,
    pub rx: mpsc::Receiver<Result<TransportOut, OtaErr>>
}

impl HttpClient {}

#[async_trait::async_trait]
impl Transport for HttpClient {
    async fn send(&mut self, mut data: TransportIn)  -> Result<(), OtaErr> {
        match data {
            TransportIn::CheckOtaHc(ref mut client) => {
                let _ = client.send().await;
                let response = client.recv().await;
                self.tx.send(response).await.unwrap();
            }
            TransportIn::GetLink(link) => {
                let tx_clone = self.tx.clone(); // Clone the Sender for the spawned task
                tokio::spawn(async move{
                    let client = reqwest::Client::new();
                    // get link
                    match client.get(link).send().await {
                        Ok(response) =>{
                            match File::create("update_ota.bin").await {
                                Ok(mut dest) => {
                                    let bytes = response.bytes().await.unwrap();
                                    match dest.write_all(&bytes).await {
                                        Ok(_) => {
                                            let _ = tx_clone.send(Ok(TransportOut::ResponseLink)).await.unwrap();
                                        }
                                        Err(_) => {
                                            let _ = tx_clone.send(Err(OtaErr::NotEnoughMemoryErr)).await.unwrap();
                                        }
                                    }
                                   
                                }
                                Err(_) => {
                                    let _ = tx_clone.send(Err(OtaErr::NotEnoughMemoryErr)).await.unwrap();
                                }
                            }
                        }

                        Err(_) => {
                            let _ = tx_clone.send(Err(OtaErr::DownloadErr)).await.unwrap();
                        }
                    };
                });
            }

            TransportIn::KeepAlive => {
                let mut mqttoptions = MqttOptions::new("keep_alive", "localhost", 1883);
                mqttoptions.set_keep_alive(Duration::from_secs(5));
                let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

                client
                    .publish("master_service", QoS::ExactlyOnce, false, "keep alive")
                    .await
                    .unwrap();
                
                client
                    .publish("master_service", QoS::ExactlyOnce, false, "ota")
                    .await
                    .unwrap();
                
                loop {
                    let event = eventloop.poll().await;
                    match &event {
                        Ok(v) => {
                            match v {
                                Event::Incoming(pack) => {
                                    match pack {
                                        rumqttc::Packet::PubComp(pubcomp) => {
                                            log::info!("pubcomp: {:?}",pubcomp);
                                            self.tx.send(Ok(TransportOut::ResponseKeepAlive)).await.unwrap();
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                                Event::Outgoing(_out) =>{}
                            }
                        }
                        Err(e) => {
                            log::info!("Error = {e:?}");
                            return Err(OtaErr::MqttErr)
                        }
                    }
                }
            }

            TransportIn::Suppend(pid) => { 
                let pid_bytes: Vec<u8> = pid.iter().map(|&x| x as u8).collect();
                let mut mqttoptions = MqttOptions::new("suppend", "localhost", 1883);
                mqttoptions.set_keep_alive(Duration::from_secs(5));
                let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

                client
                    .publish("master_service", QoS::ExactlyOnce, false, "ota suppend")
                    .await
                    .unwrap();
                
                client
                    .publish("master_service", QoS::ExactlyOnce, false, pid_bytes.clone())
                    .await
                    .unwrap();
                loop {
                    let event: Result<Event, rumqttc::ConnectionError> = eventloop.poll().await;
                    match &event {
                        Ok(v) => {
                            match v {
                                Event::Incoming(pack) => {
                                    match pack {
                                        rumqttc::Packet::PubComp(pubcomp) => {
                                            log::info!("pubcomp: {:?}",pubcomp);
                                            self.tx.send(Ok(TransportOut::ResponseSuppend)).await.unwrap();
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                                Event::Outgoing(_out) =>{}
                            }
                        }
                        Err(e) => {
                            log::info!("Error = {e:?}");
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<TransportOut, OtaErr> {
        self.rx.recv().await.unwrap()
    }  
}

