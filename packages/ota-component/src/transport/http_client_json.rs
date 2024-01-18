/*
* JSON HTTP client module for Rust
*
* Copyright (C) 2023 Bui Hien (Mark) - RealaxER
*
* Description:
* This Rust module provides a simple HTTP client for asynchronous generation
* request to web server. It is designed for ease of use and integration
* into Rust projects that require HTTP communication.
*
* Featured:
* - Asynchronous HTTP requests
* - Simple and intuitive API
* - Friendly integration with Rust projects
*
* License: This program is free software; You can redistribute it and/or
* modify it under the terms of the GNU General Public License by
* Free Software Foundation; version 2 of the License, or
* (at your option) any later version.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* Merchantability or FITNESS FOR A PARTICULAR PURPOSE. Please contact buihien29112002@gmail.com
* Request:
* You need to add the following modules to be able to use it.
   [dependencies]
    reqwest = { version = "0.11.23", features = ["blocking", "json"] }
    serde = { version = "1.0", features = ["derive"] }
    serde_json = "1.0"
    tokio = { version = "1", features = ["full"] }
    anyhow = "1.0"
*/

use reqwest;
use serde::{Serialize, Deserialize};
use anyhow::Error; // Import the Error type from the anyhow crate


#[derive(Debug, Serialize, Deserialize)]
pub struct Headers {
    pub name: &'static str,
    pub value: &'static str,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Body {
    pub mac: String,
    pub version_name: String,
    pub version_min_id: u32,
    pub version_number: u32
}
// Define the HttpClientJson struct
pub struct HttpClientJson {
    pub url: &'static str,
    pub headers: Headers,
    pub body: Body,
    pub response: Option<reqwest::Response>// Thêm một trường để giữ giá trị response
}
// Implement the HttpClientJson methods
impl HttpClientJson {
    pub fn new(url: &'static str, headers: Headers, body: Body) -> Self {
        Self { url, headers, body, response: None }
    }

    pub async fn send(&mut self) -> Result<(), Error>
    {
        let client = reqwest::Client::new();
        let json_body = serde_json::to_value(&self.body)?;

        let response = client
            .post(self.url)
            .header(self.headers.name, self.headers.value)
            .json(&json_body)
            .send()
            .await?;
          // Lưu trữ response trong trường self.response
          self.response = Some(response);
          // Trả về nội dung của response dưới dạng String
          Ok(())
    }
}
#[macro_export]
macro_rules! TestHttpJsonResponse {
    () => {
        let mut client = HttpClientJson {
            url: "https://api.smarthome.lumi.com.vn/ota/check-update-ota",
            headers: Headers {
                name: "x-lumi-api-key",
                value: "98CPB8ITIRGHVO3OJ5QT",
            },
            body: Body {
                mac: "14:c9:cf:16:dd:2c".to_string(),
                version_name: "1.0.1".to_string(),
                version_min_id: 0, 
                version_number: 1,
            },
        };
        match client.send().await {
            Ok(response) => println!("Response: {}", response),
            Err(err) => eprintln!("Error: {}", err),
        }
    };
}
