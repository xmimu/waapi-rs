use waapi_rs::WaapiClient;

#[tokio::test]
async fn test_waapi_get_info() {
    let mut client = WaapiClient::connect()
        .await
        .expect("Failed to connect");

    let (_, kwargs) = client
        .call("ak.wwise.core.getInfo", None, None)
        .await
        .expect("WAAPI call failed");

    let map = kwargs.expect("Expected kwargs");
    assert!(map.contains_key("version"));
    client.disconnect().await;
}
