//! 订阅测试：对应 Python 的 client.subscribe(topic, callback) / handler.unsubscribe()
//!
//! 参考：https://github.com/audiokinetic/waapi-client-python
//! 需要本机开启 Wwise 并启用 Authoring API (Project > User Preferences... > Enable Wwise Authoring API)。
//! 运行方式：`cargo test`；若未开 Wwise 则部分测试会 skip。

use std::time::Duration;
use tokio::time::timeout;
use waapi_rs::ak::wwise::ui::SELECTION_CHANGED;
use waapi_rs::WaapiClient;

#[tokio::test]
async fn test_subscribe_and_unsubscribe() {
    let client = match WaapiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({e})");
            return;
        }
    };

    // 对应 Python: handler = client.subscribe("ak.wwise.ui.selectionChanged", callback)
    let handle = client
        .subscribe(SELECTION_CHANGED, None, |_args, _kwargs| {})
        .await
        .expect("subscribe failed");

    // 短时等待（期间可能有事件触发回调）
    let _ = timeout(Duration::from_millis(800), tokio::time::sleep(Duration::from_millis(500)))
        .await;

    // 对应 Python: handler.unsubscribe()
    handle.unsubscribe().await.expect("unsubscribe failed");

    // 对应 Python: client.disconnect()
    client.disconnect().await;
}
