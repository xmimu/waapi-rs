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
//! - **RPC**: `call(uri, args, options)`, `call_no_args(uri)`
//! - **Subscribe**: `subscribe(topic)` returns an event stream, or `subscribe_with_callback(topic, callback)` binds a callback
//! - **Cleanup**: connections and subscriptions auto-clean on Drop; explicit `disconnect` / `SubscriptionHandle::unsubscribe()` also available
//!
//! ---
//!
//! # 功能概览
//!
//! - **连接**：`WaapiClient::connect()` / `connect_with_url(url)`；同步客户端 [WaapiClientSync] 同样提供 `connect()` 与 `connect_with_url(url)`
//! - **RPC 调用**：`call(uri, args, options)`、`call_no_args(uri)`
//! - **订阅**：`subscribe(topic)` 返回事件流，或 `subscribe_with_callback(topic, callback)` 绑定回调
//! - **资源**：连接与订阅在 Drop 时自动清理，也可显式 `disconnect` / `SubscriptionHandle::unsubscribe()`
//!
//! # Examples / 示例
//!
//! Async client — connect and call a WAAPI method (e.g. get Wwise version):
//!
//! 异步客户端 - 连接后调用 WAAPI 方法（如获取 Wwise 版本）：
//!
//! ```rust,no_run
//! use serde_json::Value;
//! use waapi_rs::WaapiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let result = client.call_no_args::<Value>("ak.wwise.core.getInfo").await?;
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
//! use serde_json::{json, Value};
//! use waapi_rs::WaapiClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClient::connect().await?;
//!     let result = client
//!         .call::<Value>(
//!             "ak.wwise.core.object.get",
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
//! use serde_json::Value;
//! use waapi_rs::WaapiClientSync;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = WaapiClientSync::connect()?;
//!     let result = client.call_no_args::<Value>("ak.wwise.core.getInfo")?;
//!     if let Some(info) = result {
//!         println!("Info: {:?}", info);
//!     }
//!     client.disconnect();
//!     Ok(())
//! }
//! ```
//!
//! # `call` constraints
//!
//! The return type of `call` / `call_no_args` is `T: DeserializeOwned`
//! (e.g. `serde_json::Value` or a custom struct), returning `Option<T>`.
//! `args` / `options` only need to be serializable (`impl Serialize`).
//!
//! ---
//!
//! # call 约束
//!
//! `call` / `call_no_args` 的返回值类型为 `T: DeserializeOwned`（如 `serde_json::Value`、自定义结构体），返回 `Option<T>`；
//! `args` / `options` 仅需可序列化（`impl Serialize`）。

mod args;
mod client;
mod uris;
pub use uris::ak;

pub use client::{
    SubscribeEvent, SubscriptionHandle, SubscriptionHandleSync, WaapiClient, WaapiClientSync,
    WaapiError,
};
