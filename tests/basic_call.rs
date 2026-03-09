//! 异步 WAAPI 调用测试：连接、调用 `ak.wwise.core.getInfo`、校验返回、断开。
//!
//! 运行方式：`cargo test`。若本机未运行 Wwise 或未启用 Authoring API，测试会自动跳过（eprintln + return）。

use waapi_rs::{ak, WaapiClient};

#[tokio::test]
async fn test_waapi_get_info() {
    let client = match WaapiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({e})");
            return;
        }
    };

    // 调用 getInfo 接口
    let result = client
        .call(ak::wwise::core::GET_INFO, None, None)
        .await
        .expect("WAAPI call failed");

    // 验证返回结果（result 为 Option<Value>）
    let info = result.expect("Expected response to contain kwargs");
    assert!(
        info.get("version").is_some(),
        "Response should contain 'version' field"
    );

    // 可选：打印版本信息用于调试
    if let Some(version) = info.get("version") {
        println!("Wwise version: {version:?}");
    }

    // 断开连接
    client.disconnect().await;
}
