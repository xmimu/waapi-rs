//! WAQL 查询示例：通过 Wwise Authoring Query Language 查询 Wwise 工程中的对象。
//!
//! 本示例演示如何使用 `ak::wwise::core::object::get` 接口，
//! 配合 WAQL 语句获取指定类型的 Wwise 对象（此处为 Event 类型）。

use serde_json::json;
use waapi_rs::{ak, WaapiClient};

#[tokio::main]
async fn main() {
    // 连接到 Wwise 的 WAAPI 服务（需 Wwise 已启动且启用 WAAPI）
    let client = WaapiClient::connect().await.expect("Failed to connect");

    // WAQL 查询：获取所有类型为 Event 的对象（$ 表示根，from type Event 表示筛选 Event 类型）
    let waql = "$ from type Event";

    let result = client
        .call(
            ak::wwise::core::OBJECT_GET,
            Some(json!({ "waql": waql })),
            Some(json!({ "return": ["id", "name", "type"] })),
        )
        .await
        .expect("WAAPI call failed");

    if let Some(map) = result {
        println!("Objects: {:#?}", map);
    }

    client.disconnect().await;
}
