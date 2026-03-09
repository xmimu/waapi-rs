use waapi_rs::{ak, WaapiClient};

#[tokio::main]
async fn main() {
    env_logger::init();
    let client = WaapiClient::connect().await.expect("Failed to connect");

    let result = client
        .call(ak::wwise::waapi::GET_FUNCTIONS, None, None)
        .await
        .expect("WAAPI call failed");

    if let Some(map) = result {
        println!("Functions: {map:?}");
    }

    client.disconnect().await;
}
