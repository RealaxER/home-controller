use std::collections::VecDeque;
use rand::Rng;
extern crate chrono;
use chrono::{DateTime, Utc, Timelike, FixedOffset};
use crate::logic::chrono::TimeZone;
use crate::error::OtaErr;
use crate::transport::TransportOut;
use std::time::Duration;

#[derive(PartialEq, Clone)]
pub enum OtaLogicIn { 
    Transport(Result<TransportOut, OtaErr>),
    Push(OtaLogicOut)
}

#[derive(Clone, PartialEq, Debug)]
pub enum HcType {
    Hc01,
    Hc02
}

#[derive(PartialEq, Debug,Clone)]
pub enum OtaLogicOut {
    CheckOtaEvent,
    UpdateOtaEvent(HcType),
    CompareVersionEvent,
    VerifyEvent,
    GetLinkEvent,
    KeepAliveEvent,
    SuppentEvent,
}
pub struct HcDriver {
    pub hc_type :HcType,
    pub allow_ota: bool,
    pub pid : u32,
    pub version_name :String,
    pub link: String
}


pub struct OtaLogic {
    pub outputs: VecDeque<OtaLogicOut>,
    pub rnd_check: i64,
    pub rnd_update_ota: u8,
    pub last_date_time: DateTime<FixedOffset>,
    pub hc : HcDriver,
    pub timeout: u64 
}

impl OtaLogic {
    pub fn new() -> Self {
        let temp = Utc::now().with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap());
        let hc = HcDriver {
            version_name: "".to_string(),
            link: "".to_string(),
            hc_type : HcType::Hc01,
            allow_ota: false,
            pid: std::process::id(),
        };
        let outputs = std::iter::once(OtaLogicOut::CheckOtaEvent).collect();
        OtaLogic {
            outputs: outputs,
            rnd_check: rand::thread_rng().gen_range(30..=50),
            rnd_update_ota: rand::thread_rng().gen_range(0..=120),
            last_date_time: temp,
            hc:hc,
            timeout: 3
        }
    }
    fn compare_hour_ota(&mut self, hour: u32, minute: u32, allow: bool) {
        // Convert rand to hour
        let hour_ota = 2 + (self.rnd_update_ota / 60);
        let minute_ota = self.rnd_update_ota % 60;

        if (hour_ota as u32 == hour && minute_ota as u32 == minute) || allow {
            // call update ota
            self.outputs.push_back(OtaLogicOut::SuppentEvent);
            self.hc.allow_ota = false;

        }

        log::info!("Time update ota {}:{}", hour_ota, minute_ota);

    }

    pub fn on_tick(&mut self, now_ms: u64) {

        let timestamp = now_ms; // Example: 1st January 2021, 00:00:00 UTC
        let datetime_utc: DateTime<Utc> = Utc.timestamp_opt((timestamp / 1000) as i64, 0).unwrap();
        let datetime_vn = datetime_utc.with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap());
        let hour = datetime_vn.hour();
        let minute = datetime_vn.minute();
        // Check if it has been 30 minutes since the last "Hello"
        if datetime_vn.signed_duration_since(self.last_date_time).num_minutes() >= 1 {
            self.outputs.push_back(OtaLogicOut::CheckOtaEvent);
            self.outputs.push_back(OtaLogicOut::KeepAliveEvent);
            self.last_date_time = datetime_vn;
            
        }
        if (hour >=2 && hour < 4) || self.hc.allow_ota {
            self.compare_hour_ota(hour, minute, self.hc.allow_ota);
        }

    }

    pub fn on_event(&mut self, _event:OtaLogicIn) {
        match _event {
            OtaLogicIn::Transport(result) => {
                match result {
                    Ok(transport ) => {
                        match transport {
                            TransportOut::ResponseRequest(response) => {
                                // send pack get link 
                                log::info!("Response successfully with link : {}", response.data.link);
                                self.hc.link = response.data.link;
                                self.outputs.push_back(OtaLogicOut::CompareVersionEvent);
                            } 
                            TransportOut::ResponseLink                             => {
                                log::info!("Get link successfully");    
                                self.outputs.push_back(OtaLogicOut::VerifyEvent);
                            }  

                            TransportOut::ResponseKeepAlive                        => {
                                log::info!("Keep alive to manager service successfully");
                            }  

                            TransportOut::ResponseSuppend                          => {
                                log::info!("Suppend to manager service successfully");
                                self.outputs.push_back(OtaLogicOut::UpdateOtaEvent(self.hc.hc_type.clone()));
                            }  
                        }
                    }

                    Err(e) => {
                        match e {
                            OtaErr::DownloadErr | OtaErr::LinkErr |  OtaErr::NoLinkResErr | OtaErr::ServerNoReturnErr => {
                                std::thread::sleep(Duration::from_secs(self.timeout));
                                self.outputs.push_back(OtaLogicOut::CheckOtaEvent);
                                self.timeout *= 100;
                            }
                            OtaErr::NotEnoughMemoryErr => {
                                std::thread::sleep(Duration::from_secs(self.timeout));
                                self.outputs.push_back(OtaLogicOut::SuppentEvent);
                                self.timeout *= 100;
                            }
                            OtaErr::VerifyErr | OtaErr::VerifyNotEqualErr => {
                                std::thread::sleep(Duration::from_secs(self.timeout));
                                self.outputs.push_back(OtaLogicOut::GetLinkEvent);
                                self.timeout *= 100;
                            }

                            OtaErr::UserCalendarErr => {

                            }
                            _ => {

                            }
                        }
                    }
                }  
            }
            OtaLogicIn::Push(event) => {
                match event {
                    OtaLogicOut::GetLinkEvent => {
                        self.outputs.push_back(event);
                    }
                    _ => {

                    }
                }
            }
        }
    }
    pub fn pop_action(&mut self) -> Option<OtaLogicOut> {
        self.outputs.pop_front()
    }

}

#[cfg(test)]
mod test {

    use super::*;
    use crate::transport::ResponseOtaHc;

    #[test]

    fn test_random_values() {
        let mut unique_values = std::collections::HashSet::new();

        for _ in 0..10 {
            let ota_logic = OtaLogic::new();

            // Check the number of elements in outputs
            assert_eq!(ota_logic.outputs.len(), 0);

            // Check the range for rnd_check
            assert!(ota_logic.rnd_check >= 30 && ota_logic.rnd_check <= 50);

            // Check the range for rnd_update_ota
            assert!(ota_logic.rnd_update_ota <= 120 && ota_logic.rnd_update_ota >= 1);

            // Insert the random values into the HashSet
            unique_values.insert((ota_logic.rnd_check, ota_logic.rnd_update_ota));
        }

        // If the size of the HashSet is equal to the number of iterations, it means all values were unique
        assert_eq!(unique_values.len(), 5);
    }
    
    #[test]
    fn test_compare_hour_ota() {
        let mut ota_logic = OtaLogic::new();

        // Test case when update_ota is allowed
        ota_logic.compare_hour_ota(2, 30, false);
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::VerifyEvent));

        // Test case when update_ota is not allowed, should not push VerifyEvent
        ota_logic.compare_hour_ota(4, 30, false);
        assert_eq!(ota_logic.outputs.len(), 0);

        ota_logic.compare_hour_ota(1, 59, false);
        assert_eq!(ota_logic.outputs.len(), 0);

        ota_logic.compare_hour_ota(4, 01, false);
        assert_eq!(ota_logic.outputs.len(), 0);


        ota_logic.compare_hour_ota(1, 30, true);
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::VerifyEvent));
    }

    
    #[test]
    fn test_on_tick() {
        let mut ota_logic = OtaLogic::new();
        ota_logic.on_tick(1705301096152);
        ota_logic.on_tick(1705301197152);
        assert_eq!(ota_logic.outputs.len(), 0);

        ota_logic.on_tick(1705301096152);
        ota_logic.on_tick(1705301196152);
        assert_eq!(ota_logic.outputs.len(), 2);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CheckOtaEvent));
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::KeepAliveEvent));

        ota_logic.hc.allow_ota = true;
        ota_logic.on_tick(1705301196152);
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::SuppentEvent));


    }
    
    #[test]
    fn test_on_event() {
        let mut ota_logic = OtaLogic::new();

        //check ota
        ota_logic.outputs.push_back(OtaLogicOut::CheckOtaEvent);
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CheckOtaEvent));

        //check response
        let res = ResponseOtaHc::default();
        ota_logic.on_event(OtaLogicIn::Transport(Ok(TransportOut::ResponseRequest(res))));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CompareVersionEvent));

        // check keep alive
        ota_logic.outputs.push_back(OtaLogicOut::KeepAliveEvent);
        assert_eq!(ota_logic.outputs.len(), 1);

        // check reponsselink và veriy
        ota_logic.on_event(OtaLogicIn::Transport(Ok(TransportOut::ResponseLink)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::VerifyEvent));

        // check err 
        ota_logic.on_event(OtaLogicIn::Transport(Err(OtaErr::DownloadErr)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CheckOtaEvent));

        ota_logic.on_event(OtaLogicIn::Transport(Err(OtaErr::LinkErr)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CheckOtaEvent));

        ota_logic.on_event(OtaLogicIn::Transport(Err(OtaErr::NoLinkResErr)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CheckOtaEvent));

        ota_logic.on_event(OtaLogicIn::Transport(Err(OtaErr::ServerNoReturnErr)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::CheckOtaEvent));

        ota_logic.on_event(OtaLogicIn::Transport(Err(OtaErr::NotEnoughMemoryErr)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::SuppentEvent));

        ota_logic.on_event(OtaLogicIn::Transport(Err(OtaErr::VerifyErr)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::GetLinkEvent));


        // check suppend and update
        ota_logic.on_event(OtaLogicIn::Transport(Ok(TransportOut::ResponseSuppend)));
        assert_eq!(ota_logic.outputs.len(), 1);
        assert_eq!(ota_logic.outputs.pop_front(), Some(OtaLogicOut::UpdateOtaEvent(ota_logic.hc.hc_type)));
        
    }
}
