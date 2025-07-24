use waapi_rs::WaapiClient;

#[tokio::main]
async fn main() {
    let mut client = WaapiClient::connect()
        .await
        .expect("Failed to connect");

    let (_, kwargs) = client
        .call("ak.wwise.core.getInfo", None, None)
        .await
        .expect("WAAPI call failed");

    if let Some(map) = kwargs {
        let version = map
            .get("version")
            .and_then(|v| v.get("displayName"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        println!("Wwise Version: {}", version);
    }

    client.disconnect().await;
}
