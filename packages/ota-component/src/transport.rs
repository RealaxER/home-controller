use serde::{Deserialize, Serialize};
pub mod http_client;
pub mod mqtt;
use crate::error::OtaErr;


pub struct HeaderJson {
    pub name: &'static str,
    pub value: &'static str,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BodyJson {
    pub mac: String,
    pub version_name: String,
    pub version_min_id: u32,
    pub version_number: u32
}

#[derive(Debug,Clone ,Default, Serialize, Deserialize, PartialEq)]
pub struct Data {
    pub version_number: u16,
    pub version_name: String,
    pub version_min_id: u16,
    pub download_id: u64,
    pub link: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Default ,Serialize, Deserialize, PartialEq)]
pub struct ResponseOtaHc {
    pub success: bool,
    #[serde(rename="statusCode")]
    pub status_code: u16,
    pub data: Data
}
// Define the HttpClientJson struct
pub struct HttpClientJson {
    pub url: &'static str,
    pub headers: HeaderJson,
    pub body: BodyJson,
    pub response: ResponseOtaHc// Thêm một trường để giữ giá trị response
}
pub enum TransportIn {
    CheckOtaHc(HttpClientJson),
    GetLink(String),
    KeepAlive,
    Suppend(Vec<i32>),
}

#[derive(PartialEq, Clone)]
pub enum TransportOut {
    ResponseRequest(ResponseOtaHc),
    ResponseLink,
    ResponseKeepAlive,
    ResponseSuppend
}
impl HttpClientJson {
    pub fn new(url: &'static str, headers: HeaderJson, body: BodyJson) -> Self {
        Self { url, headers, body, response: ResponseOtaHc::default()}
    }
    pub fn new_template() -> Self {
        HttpClientJson {
            url: "https://api.smarthome.lumi.com.vn/ota/check-update-ota",
            headers: HeaderJson {
                name: "x-lumi-api-key",
                value: "98CPB8ITIRGHVO3OJ5QT",
            },
            body: BodyJson {
                mac: "14:c9:cf:17:af:8e".to_string(),
                version_name: "1.0.1".to_string(),
                version_min_id: 0, 
                version_number: 1,
            },
            response: ResponseOtaHc::default()
        }
    }
    pub async fn send(&mut self) -> Result<(), OtaErr>
    {
        let client = reqwest::Client::new();
        let json_body = serde_json::to_value(&self.body).map_err(|_| {OtaErr::HttpErr})?;
        
        let response = client
            .post(self.url)
            .header(self.headers.name, self.headers.value)
            .json(&json_body)
            .send()
            .await.map_err(|_| {OtaErr::HttpErr})?;

        if response.status().is_success() {
            log::info!("Response successfully");
            let json_result: Result<ResponseOtaHc, _> = response.json().await;
            match json_result {
                Ok(parsed_response) => {
                    log::info!("Response successfully");
                    self.response = parsed_response;
                }
                Err(_) => {
                    // Print more information about the error
                    log::info!("The final version");
                }
            }
        }
        else {
            return Err(OtaErr::HttpErr);
        }
        Ok(())
    }
    pub async fn recv(&mut self) -> Result<TransportOut, OtaErr> {
        // Kiểm tra xem response có tồn tại hay không
        if self.response.success == true {
            let res = self.response.clone();
            Ok(TransportOut::ResponseRequest(res))
        } 
        else {
            Err(OtaErr::HttpErr)
        }
    }
}


#[async_trait::async_trait]
pub trait Transport {
    async fn send(&mut self, data: TransportIn) -> Result<(), OtaErr>;
    async fn recv(&mut self) -> Result<TransportOut, OtaErr>;
}

