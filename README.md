# waapi-rs

Wwise Authoring API (WAAPI) 的 Rust 客户端，基于 WAMP over WebSocket，支持异步与同步两种用法。

## 功能

- **异步客户端** [`WaapiClient`](https://docs.rs/waapi-rs/)：`async` 连接、RPC 调用、主题订阅，可在多任务中使用
- **同步客户端** [`WaapiClientSync`](https://docs.rs/waapi-rs/)：内部管理 tokio 运行时，阻塞式 call，适合脚本或非 async 代码
- **RPC 调用**：`call` / `call_no_args` 调用 WAAPI 方法（如 `ak.wwise.core.getInfo`）
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
waapi_rs = { git = "https://github.com/xmimu/waapi-rs.git", branch = "dev" }
tokio = { version = "1", features = ["full"] }
```

若从本地路径依赖：

```toml
waapi_rs = { path = "../waapi-rs" }
```

## 快速示例

```rust
use waapi_rs::WaapiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = WaapiClient::connect().await?;
    let result = client.call("ak.wwise.core.getInfo", None, None).await?;
    if let Some(info) = result {
        let version = info.get("version").and_then(|v| v.get("displayName")).and_then(|v| v.as_str()).unwrap_or("Unknown");
        println!("Wwise Version: {}", version);
    }
    client.disconnect().await;
    Ok(())
}
```

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
