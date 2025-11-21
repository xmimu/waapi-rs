use waapi_rs::WaapiClientSync;

#[test]
fn test_waapi_get_info_sync() {
    // 建立连接 - 不需要 tokio::test 宏
    let client = WaapiClientSync::connect()
        .expect("Failed to connect to WAAPI");

    // 同步调用 getInfo 接口
    let result = client
        .call("ak.wwise.core.getInfo", None, None)
        .expect("WAAPI call failed");

    // 验证返回结果
    let info = result.expect("Expected response to contain kwargs");
    assert!(
        info.contains_key("version"),
        "Response should contain 'version' field"
    );

    // 可选：打印版本信息用于调试
    if let Some(version) = info.get("version") {
        println!("Wwise version: {:?}", version);
    }

    // 断开连接
    client.disconnect();
}
