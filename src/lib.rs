//! waapi-rs：Wwise Authoring API (WAAPI) 的 Rust 客户端
//!
//! 基于 WAMP over WebSocket，支持异步与同步两种用法。主要入口为异步客户端
//! [WaapiClient] 与同步客户端 [WaapiClientSync]。
//!
//! # 功能概览
//!
//! - **连接**：`WaapiClient::connect()` / `connect_with_url(url)`；同步客户端 [WaapiClientSync] 同样提供 `connect()` 与 `connect_with_url(url)`
//! - **RPC 调用**：`call(uri, args, options)`、`call_no_args(uri)`
//! - **订阅**：`subscribe(topic)` 返回事件流，或 `subscribe_with_callback(topic, callback)` 绑定回调
//! - **资源**：连接与订阅在 Drop 时自动清理，也可显式 `disconnect` / `SubscriptionHandle::unsubscribe()`
//!
//! # 示例
//!
//! 异步客户端：连接后调用 WAAPI 方法（如获取 Wwise 版本）：
//!
//! ```rust,no_run
//! use waapi_rs::WaapiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let result = client.call("ak.wwise.core.getInfo", None, None).await?;
//!     if let Some(info) = result {
//!         let version = info.get("version")
//!             .and_then(|v| v.get("displayName"))
//!             .and_then(|v| v.as_str())
//!             .unwrap_or("Unknown");
//!         println!("Wwise Version: {}", version);
//!     }
//!     client.disconnect().await;
//!     Ok(())
//! }
//! ```
//!
//! 订阅主题并用回调接收事件（需 Wwise 已启动并启用 Authoring API）：
//!
//! ```rust,no_run
//! use waapi_rs::WaapiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let handle = client
//!         .subscribe_with_callback("ak.wwise.ui.selectionChanged", |_args, kwargs| {
//!             println!("Selection changed: {:?}", kwargs);
//!         })
//!         .await?;
//!     // 使用完毕后取消订阅并断开
//!     handle.unsubscribe().await?;
//!     client.disconnect().await;
//!     Ok(())
//! }
//! ```
//!
//! 同步客户端（适用于非 async 代码或脚本）：
//!
//! ```rust,no_run
//! use waapi_rs::WaapiClientSync;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClientSync::connect()?;
//!     let result = client.call_no_args("ak.wwise.core.getInfo")?;
//!     if let Some(info) = result {
//!         println!("Info: {:?}", info);
//!     }
//!     client.disconnect();
//!     Ok(())
//! }
//! ```
//!
//! # Re-exports（与 WAAPI 参数/返回值交互）
//!
//! 从 `wamp_async` 重新导出：`WampArgs`（位置参数）、`WampDict`（字典）、`WampId`（ID）、
//! `WampKwArgs`（关键字参数），用于 `call` 的入参与返回值、以及订阅事件的 `args`/`kwargs`。

mod client;

pub use client::{
    SubscribeEvent, SubscriptionHandle, SubscriptionHandleSync, WaapiClient, WaapiClientSync,
};
pub use wamp_async::{WampArgs, WampDict, WampId, WampKwArgs};
