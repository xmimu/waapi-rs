//! 订阅回调测试：对应 Python 的 client.subscribe(topic, lambda obj: print(obj))
//!
//! 参考：https://github.com/audiokinetic/waapi-client-python
//! 需要本机开启 Wwise 并启用 Authoring API。
//! 运行方式：`cargo test`；若未开 Wwise 则部分测试会 skip。

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use waapi_rs::ak;
use waapi_rs::WaapiClient;

#[tokio::test]
async fn test_subscribe_with_callback() {
    let client: WaapiClient = match WaapiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({})", e);
            return;
        }
    };

    // 对应 Python: handler = client.subscribe("ak.wwise.ui.selectionChanged", lambda object: print(...))
    let count = Arc::new(AtomicU32::new(0));
    let count_clone = Arc::clone(&count);
    let handler = client
        .subscribe_with_callback(ak::wwise::ui::SELECTION_CHANGED, move |_args, kwargs| {
            count_clone.fetch_add(1, Ordering::Relaxed);
            println!("[test] selectionChanged: {:?}", kwargs);
        })
        .await
        .expect("subscribe_with_callback failed");

    // 短暂等待，看是否收到事件（不强制）
    let _ = timeout(Duration::from_millis(500), async {
        tokio::time::sleep(Duration::from_millis(400)).await;
    })
    .await;

    // 对应 Python: handler.unsubscribe()
    handler.unsubscribe().await.expect("unsubscribe failed");

    // 对应 Python: client.disconnect()
    client.disconnect().await;

    let _n = count.load(Ordering::Relaxed);
}

#[tokio::test]
async fn test_subscribe_callback_drop_handle() {
    let client: WaapiClient = match WaapiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({})", e);
            return;
        }
    };

    let handler = client
        .subscribe_with_callback(ak::wwise::ui::SELECTION_CHANGED, |_args, kwargs| {
            println!("[test] selectionChanged (drop_handle): {:?}", kwargs);
        })
        .await
        .expect("subscribe_with_callback failed");

    // drop 句柄应自动取消订阅（与 Python 的 with 块结束类似）
    drop(handler);

    client.disconnect().await;
}
