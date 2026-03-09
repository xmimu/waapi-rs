//! waapi-rs: A Rust client for the Wwise Authoring API (WAAPI).
//!
//! Based on WAMP over WebSocket, supporting both async and sync usage.
//! Main entry points are the async client [WaapiClient] and the sync client [WaapiClientSync].
//!
//! ---
//!
//! waapi-rs：Wwise Authoring API (WAAPI) 的 Rust 客户端。
//!
//! 基于 WAMP over WebSocket，支持异步与同步两种用法。主要入口为异步客户端
//! [WaapiClient] 与同步客户端 [WaapiClientSync]。
//!
//! # Features
//!
//! - **Connect**: `WaapiClient::connect()` / `connect_with_url(url)`; sync client [WaapiClientSync] provides the same
//! - **RPC**: `call(uri, args, options)`
//! - **Subscribe**: `subscribe(topic, options, callback)` binds a callback for receiving events
//! - **Cleanup**: connections and subscriptions auto-clean on Drop; explicit `disconnect` / `SubscriptionHandle::unsubscribe()` also available
//!
//! ---
//!
//! # 功能概览
//!
//! - **连接**：`WaapiClient::connect()` / `connect_with_url(url)`；同步客户端 [WaapiClientSync] 同样提供 `connect()` 与 `connect_with_url(url)`
//! - **RPC 调用**：`call(uri, args, options)`
//! - **订阅**：`subscribe(topic, options, callback)` 绑定回调接收事件
//! - **资源**：连接与订阅在 Drop 时自动清理，也可显式 `disconnect` / `SubscriptionHandle::unsubscribe()`
//!
//! # Examples / 示例
//!
//! Async client — connect and call a WAAPI method (e.g. get Wwise version):
//!
//! 异步客户端 - 连接后调用 WAAPI 方法（如获取 Wwise 版本）：
//!
//! ```rust,no_run
//! use waapi_rs::{ak, WaapiClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let result = client.call(ak::wwise::core::GET_INFO, None, None).await?;
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
//! RPC call with `json!` args and options (e.g. WAQL query):
//!
//! 使用 `json!` 构造参数与选项进行 RPC 调用（如 WAQL 查询）：
//!
//! ```rust,no_run
//! use serde_json::json;
//! use waapi_rs::{ak, WaapiClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let result = client
//!         .call(
//!             ak::wwise::core::OBJECT_GET,
//!             Some(json!({ "waql": "$ from type Event" })),
//!             Some(json!({ "return": ["id", "name", "type"] })),
//!         )
//!         .await?;
//!     if let Some(obj) = result {
//!         println!("Objects: {:?}", obj);
//!     }
//!     client.disconnect().await;
//!     Ok(())
//! }
//! ```
//!
//! Subscribe to a topic with a callback:
//!
//! 订阅主题并用回调接收事件：
//!
//! ```rust,no_run
//! use waapi_rs::{ak, WaapiClient};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let handle = client
//!         .subscribe(ak::wwise::ui::SELECTION_CHANGED, None, |_args, kwargs| {
//!             println!("Selection changed: {:?}", kwargs);
//!         })
//!         .await?;
//!     handle.unsubscribe().await?;
//!     client.disconnect().await;
//!     Ok(())
//! }
//! ```
//!
//! Sync client (for non-async code or scripts):
//!
//! 同步客户端（适用于非 async 代码或脚本）：
//!
//! ```rust,no_run
//! use waapi_rs::{ak, WaapiClientSync};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClientSync::connect()?;
//!     let result = client.call(ak::wwise::core::GET_INFO, None, None)?;
//!     if let Some(info) = result {
//!         println!("Info: {:?}", info);
//!     }
//!     client.disconnect();
//!     Ok(())
//! }
//! ```
//!
//! # `call` types
//!
//! `call` returns `Option<serde_json::Value>`: the WAAPI response as JSON.
//! `args` / `options` are `Option<serde_json::Value>` (e.g. `Some(json!({...}))` or `None`).
//!
//! ---
//!
//! # call 类型
//!
//! `call` 返回 `Option<serde_json::Value>`：WAAPI 响应的 JSON 值。
//! `args` / `options` 为 `Option<serde_json::Value>`（如 `Some(json!({...}))` 或 `None`）。

mod args;
mod client;
mod uris;
pub use uris::ak;

pub use client::{
    SubscriptionHandle, SubscriptionHandleSync, WaapiClient, WaapiClientSync, WaapiError,
};
