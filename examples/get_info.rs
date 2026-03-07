//! 获取 Wwise 版本示例：调用 `ak.wwise.core.getInfo` 并打印版本信息。
//!
//! 运行前提：Wwise 已启动，且工程中已启用 Authoring API（Project > User Preferences > Enable Wwise Authoring API）。
//!
//! 运行方式：`cargo run --example get_info`

use serde_json::Value;
use waapi_rs::{uris::ak::wwise::core, WaapiClient};

#[tokio::main]
async fn main() {
    let client = WaapiClient::connect().await.expect("Failed to connect");

    let result = client
        .call_no_args::<Value>(core::GET_INFO)
        .await
        .expect("WAAPI call failed");

    if let Some(map) = result {
        let version = map
            .get("version")
            .and_then(|v| v.get("displayName"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        println!("Wwise Version: {}", version);
    }

    client.disconnect().await;
}
