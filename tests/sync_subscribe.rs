//! 同步客户端订阅测试：使用 WaapiClientSync 连接、订阅（回调）、取消订阅、断开。
//!
//! 需要本机开启 Wwise 并启用 Authoring API。运行方式：`cargo test`；若未开 Wwise 则 skip。

use std::time::Duration;
use waapi_rs::ak;
use waapi_rs::WaapiClientSync;

#[test]
fn test_sync_subscribe() {
    let client = match WaapiClientSync::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skip: WAAPI not available ({e})");
            return;
        }
    };

    let handle = match client.subscribe(ak::wwise::ui::SELECTION_CHANGED, None, |_kwargs| {})
    {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Skip: subscribe failed ({e})");
            client.disconnect();
            return;
        }
    };

    std::thread::sleep(Duration::from_millis(500));
    handle.unsubscribe().expect("unsubscribe failed");
    client.disconnect();
}
