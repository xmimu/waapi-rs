//! 订阅示例：对应 Python waapi-client 的 subscribe 用法
//!
//! 运行方式：`cargo run --example subscribe`
//! 运行前提：Wwise 已启动并启用 Authoring API；运行后可在 Wwise 中切换选中对象以触发回调。
//!
//! Python 参考：
//!   https://github.com/audiokinetic/waapi-client-python
//!
//! ```python
//! from waapi import WaapiClient
//! client = WaapiClient()
//! handler = client.subscribe(
//!     "ak.wwise.ui.selectionChanged",
//!     lambda object: print("Selection changed: " + str(object))
//! )
//! handler.unsubscribe()
//! client.disconnect()
//! ```

use std::time::Duration;
use waapi_rs::ak;
use waapi_rs::WaapiClient;

#[tokio::main]
async fn main() {
    // Connect (default URL ws://localhost:8080/waapi)
    let client = WaapiClient::connect()
        .await
        .expect("Failed to connect to WAAPI. Ensure Wwise is running with Authoring API enabled.");

    // 方式一：subscribe_with_callback —— 对应 Python 的 subscribe(topic, lambda obj: ...)
    println!(
        "Subscribing to {} with callback...",
        ak::wwise::ui::SELECTION_CHANGED
    );
    let handler = client
        .subscribe_with_callback(ak::wwise::ui::SELECTION_CHANGED, |_args, kwargs| {
            println!("Selection changed: {:?}", kwargs);
        })
        .await
        .expect("Subscribe failed");

    // 简单演示：等待几秒（期间在 Wwise 里切换选中会触发回调），然后取消订阅并断开
    println!("Waiting 5s (change selection in Wwise to see events)...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // 对应 Python: handler.unsubscribe()
    handler.unsubscribe().await.expect("Unsubscribe failed");

    // 对应 Python: client.disconnect()
    client.disconnect().await;
    println!("Disconnected.");
}
