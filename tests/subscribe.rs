//! 订阅测试：对应 Python 的 client.subscribe(topic, callback) / handler.unsubscribe()
//!
//! 参考：https://github.com/audiokinetic/waapi-client-python
//! 需要本机开启 Wwise 并启用 Authoring API (Project > User Preferences... > Enable Wwise Authoring API)。
//! 运行方式：`cargo test`；若未开 Wwise 则部分测试会 skip。

use std::time::Duration;
use tokio::time::timeout;
use waapi_rs::ak::wwise::ui::SELECTION_CHANGED;
use waapi_rs::{SubscribeEvent, WaapiClient};

#[tokio::test]
async fn test_subscribe_and_unsubscribe() {
    let client = match WaapiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({})", e);
            return;
        }
    };

    // 对应 Python: handler = client.subscribe("ak.wwise.ui.selectionChanged", ...)
    let (handle, mut rx) = client
        .subscribe(SELECTION_CHANGED)
        .await
        .expect("subscribe failed");

    // 短时等待一条事件（无事件则超时，不要求一定有）
    let _: Option<SubscribeEvent> = timeout(Duration::from_millis(800), rx.recv()).await.ok().flatten();

    // 对应 Python: handler.unsubscribe()
    handle.unsubscribe().await.expect("unsubscribe failed");

    // 对应 Python: client.disconnect()
    client.disconnect().await;
}

#[tokio::test]
async fn test_subscribe_receiver_dropped_then_unsubscribe() {
    let client = match WaapiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({})", e);
            return;
        }
    };

    let (handle, rx) = match client.subscribe(SELECTION_CHANGED).await {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("Skip: subscribe failed ({})", e);
            client.disconnect().await;
            return;
        }
    };
    drop(rx);
    // 显式取消订阅后断开（与 Python handler.unsubscribe() 一致）
    handle.unsubscribe().await.expect("unsubscribe failed");
    client.disconnect().await;
}
