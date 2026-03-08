# waapi-rs

Wwise Authoring API (WAAPI) 的 Rust 客户端，基于 WAMP over WebSocket，支持异步与同步两种用法。

## 功能

- **异步客户端** [`WaapiClient`](https://docs.rs/waapi-rs/)：`async` 连接、RPC 调用、主题订阅，可在多任务中使用
- **同步客户端** [`WaapiClientSync`](https://docs.rs/waapi-rs/)：内部管理 tokio 运行时，阻塞式 call，适合脚本或非 async 代码
- **RPC 调用**：`call<T>` / `call_no_args<T>` 调用 WAAPI 方法，泛型 `T` 为返回值反序列化类型（需 `DeserializeOwned`），返回 `Result<Option<T>, Error>`
- **URI 常量**：`waapi_rs::ak` 下提供与 WAAPI 路径对应的嵌套模块与常量（如 `ak::wwise::core::GET_INFO`、`ak::wwise::waapi::GET_TOPICS`），避免手写字符串
- **主题订阅**：`subscribe` 返回事件流，或 `subscribe_with_callback` 绑定回调；通过 `SubscriptionHandle` / `SubscriptionHandleSync` 取消订阅，drop 时自动清理
- **资源清理**：连接与订阅在 `Drop` 时自动断开/取消，也可显式 `disconnect` / `unsubscribe`

## 前置条件

- **Wwise**：已安装并运行，且在工程中启用 Authoring API  
  （Project > User Preferences > Enable Wwise Authoring API）
- **Rust**：建议 1.70+，需支持 `tokio` 与 async

## 安装

在 `Cargo.toml` 中添加依赖（当前为 git 依赖）：

```toml
[dependencies]
waapi-rs = { git = "https://github.com/xmimu/waapi-rs.git", branch = "dev" }
tokio = { version = "1", features = ["full"] }
```

若从本地路径依赖：

```toml
waapi-rs = { path = "../waapi-rs" }
```

## 快速示例

只引入 `waapi-rs::ak`，调用时从 `ak::` 起写路径（与 C++ WAAPI URI 风格一致），泛型 `call_no_args::<Value>` 返回 `Option<Value>`：

```rust
use serde_json::Value;
use waapi_rs::{ak, WaapiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WaapiClient::connect().await?;
    let result = client.call_no_args::<Value>(ak::wwise::core::GET_INFO).await?;
    if let Some(info) = result {
        let version = info.get("version").and_then(|v| v.get("displayName")).and_then(|v| v.as_str()).unwrap_or("Unknown");
        println!("Wwise Version: {}", version);
    }
    client.disconnect().await;
    Ok(())
}
```

### URI 常量（`uris`）

引入方式只需 `use waapi_rs::ak`，调用时从 `ak::` 开始写路径，与 WAAPI/C++ 的 URI 层级一致（如 `ak.wwise.core.getInfo` 对应 `ak::wwise::core::GET_INFO`）：

- `ak::soundengine::*` — 运行时接口（如 `POST_EVENT`、`SET_STATE`）
- `ak::wwise::core::*` — 核心接口（如 `GET_INFO`、`OBJECT_GET`）及主题（如 `OBJECT_CREATED`、`PROJECT_LOADED`）
- `ak::wwise::debug::*`、`ak::wwise::ui::*`、`ak::wwise::waapi::*` — 调试、UI、WAAPI 元信息

示例：`client.call_no_args::<Value>(ak::wwise::core::GET_INFO)`、`client.call(ak::wwise::waapi::GET_TOPICS, None, None)`、订阅时使用 `ak::wwise::ui::SELECTION_CHANGED`。

### call 泛型与返回值

- `call<T>(uri, args, options)`、`call_no_args<T>(uri)` 的泛型 `T` 为**返回值**的反序列化类型，需满足 `DeserializeOwned`（如 `serde_json::Value` 或自定义结构体）。
- `args` 与 `options` 仅需可序列化（`impl Serialize`），不要求与 `T` 相同类型。
- 返回 `Result<Option<T>, Error>`：成功时 WAAPI 的 kwargs 反序列化为 `T`，无结果时为 `None`。

## 示例与测试

- 获取 Wwise 版本：`cargo run --example get_info`
- 订阅选择变化事件（回调）：`cargo run --example subscribe`
- 运行测试：`cargo test`（部分测试需本机 WAAPI 可用，否则会 skip）

## 文档与设计

- 生成并打开 API 文档：`cargo doc --open`
- 开发设计与架构说明见 [DESIGN.md](DESIGN.md)

## 参考

- [Wwise Authoring API 官方文档](https://www.audiokinetic.com/library/edge/?source=SDK&id=waapi.html)
- [waapi-client-python](https://github.com/audiokinetic/waapi-client-python)（API 用法可对照参考）
