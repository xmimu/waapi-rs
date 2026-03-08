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
//! # call 约束
//!
//! `call` 的入参与返回值统一为 `T: Serialize + DeserializeOwned`（如 `serde_json::Value`、自定义结构体），返回 `Option<T>`。

mod args;
mod client;
mod uris;
pub use uris::ak;

pub use client::{
    SubscribeEvent, SubscriptionHandle, SubscriptionHandleSync, WaapiClient, WaapiClientSync,
    WaapiError,
};
