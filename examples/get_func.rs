use serde_json::Value;
use waapi_rs::{ak, WaapiClient};

#[tokio::main]
async fn main() {
    env_logger::init();
    let client = WaapiClient::connect().await.expect("Failed to connect");

    let result = client
        .call_no_args::<Value>(ak::wwise::waapi::GET_FUNCTIONS)
        .await
        .expect("WAAPI call failed");

    if let Some(map) = result {
        println!("Functions: {map:?}");
    }

    client.disconnect().await;
}
