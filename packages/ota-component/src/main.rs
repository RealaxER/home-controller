use clap::Parser;
use system_intergration::SystemIntergration;

pub mod system_intergration;
pub mod logic;
pub mod transport;
pub mod security;
pub mod error;
// Import các thành phần từ modules transport::http_client_json

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {

}

#[tokio::main]
async fn main() {
    env_logger::builder().format_timestamp_millis().init();

    let args = Args::parse();
    log::info!("args: {:?}", args);
    //TestHttpJsonResponse!();
    let mut system_intergration = SystemIntergration::new().await;
    loop {
        match system_intergration.recv().await {
            Ok(_) => {
                
            },
            Err(e) => {
                log::error!("{:?}", e);
                break;
            }
        }

    }
}