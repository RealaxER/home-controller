use rumqttc::{MqttOptions, AsyncClient, EventLoop, Event, QoS};
use crate::error::OtaErr;
use tokio::time::Duration;




#[derive(Debug)]
pub struct ResponseMqtt {
    pub topic: String,
    pub message: String,
}
pub struct MqttDriver {
    pub options: MqttOptions, 
    pub client: AsyncClient,
    pub eventloop: EventLoop,
    pub flag:bool,
}

impl MqttDriver { 
    pub async fn new(id:String, host:String, port: u16, keep_alive:u64) -> Self {
        let mut mqttoptions = MqttOptions::new(id, host, port);
        mqttoptions.set_keep_alive(Duration::from_secs(keep_alive));

        let (client, eventloop) = AsyncClient::new(mqttoptions.clone(), 10);
        client
            .subscribe("master/ota", QoS::AtMostOnce)
            .await
            .unwrap();
    
        MqttDriver {
            options: mqttoptions.clone(),
            client: client,
            eventloop: eventloop, 
            flag:false                                        
        }
    }
    pub async fn send(&mut self, topic: String, message: Vec<u8>, qos: QoS, retain: bool)-> Result<(),OtaErr> {
        match self.client.publish(topic, qos, retain, message).await {
            Ok(res) => {
                Ok(res)
            }
            Err(_) => {
                Err(OtaErr::MqttErr)
            }
        }
    }

    pub async fn recv(&mut self) -> Result<ResponseMqtt, OtaErr> {
        loop {
            let event = self.eventloop.poll().await;
            match &event {
                Ok(v) => {
                    match v {
                        Event::Incoming(packet) => {
                            match packet {
                                rumqttc::Packet::Publish(publish) => {
                                    let payload_str: String = String::from_utf8_lossy(&publish.payload).to_string();
                                    let res = ResponseMqtt {
                                        topic: publish.topic.clone(),
                                        message: payload_str,
                                        
                                    };
                                    log::info!("res: {:?}", res);
                                    return Ok(res);
                                }
                                _ => {
                                }
                            }
                        }
                        Event::Outgoing(_) => {}
                    }
                }
                Err(e) => {
                    log::info!("Error = {e:?}");
                    return Err(OtaErr::MqttErr);
                }
            }
        }
    }
}
