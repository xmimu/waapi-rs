use std::time::Duration;
use waapi_rs::{ak, WaapiClient};

/// Monitors WAAPI connection: reconnects automatically when Wwise closes.
///
/// Run: `cargo run --example monitor`
/// Requires Wwise running with Authoring API enabled. Stop/start Wwise to see reconnection.
///
/// ---
///
/// 监控 WAAPI 连接：Wwise 关闭后自动重连。
///
/// 运行：`cargo run --example monitor`
/// 需要 Wwise 启动并启用 Authoring API。关闭/启动 Wwise 可观察重连行为。
#[tokio::main]
async fn main() {
    env_logger::init();

    loop {
        match WaapiClient::connect().await {
            Ok(client) => {
                println!("Connected to Wwise.");

                // Subscribe to an event to demonstrate receiving events and to keep the connection alive.
                //
                // 订阅一个事件以演示接收事件并保持连接活跃。
                let sub_result = client
                    .subscribe(ak::wwise::ui::SELECTION_CHANGED, None, |kwargs| {
                        println!("Selection changed: {kwargs:#?}");
                    })
                    .await;
                if let Err(e) = sub_result {
                    eprintln!("Failed to subscribe: {e}");
                }

                // Poll until the event loop dies (Wwise closed / network drop).
                // is_connected() returns false once the event loop terminates.
                //
                // 轮询直到事件循环结束（Wwise 关闭/网络断开）。
                // 事件循环终止后 is_connected() 立即返回 false。
                while client.is_connected() {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    println!("Still connected...");
                }

                // client dropped here: Drop triggers do_cleanup.
                // do_cleanup detects the dead event loop and skips WAMP ops safely.
                //
                // client 在此 drop：触发 do_cleanup。
                // do_cleanup 检测到事件循环已结束，安全跳过 WAMP 层操作，不会 panic。
                println!("Disconnected from Wwise.");
            }
            Err(e) => {
                eprintln!("Failed to connect: {e}");
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
        println!("Attempting to reconnect...");
    }
}
