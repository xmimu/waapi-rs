//! 同步 WAAPI 调用测试：使用 WaapiClientSync 连接、调用 getInfo、校验返回、断开。
//!
//! 运行方式：`cargo test`。若本机未运行 Wwise 或未启用 Authoring API，测试会自动跳过（eprintln + return）。

use waapi_rs::WaapiClientSync;

#[test]
fn test_waapi_get_info_sync() {
    let client = match WaapiClientSync::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({})", e);
            return;
        }
    };

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
