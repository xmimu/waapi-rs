# waapi-rs

English | [中文](docs/README_CN.md)

A Rust client for the Wwise Authoring API (WAAPI), based on WAMP over WebSocket, supporting both async and sync usage.

### Features

- **Async client** [`WaapiClient`](https://docs.rs/waapi-rs/): async connect, RPC calls, topic subscriptions; shareable across tasks
- **Sync client** [`WaapiClientSync`](https://docs.rs/waapi-rs/): internally manages a tokio runtime, blocking calls; ideal for scripts or non-async code
- **RPC calls**: `call<T>` / `call_no_args<T>` to invoke WAAPI methods; generic `T` is the deserialization type for the return value (`DeserializeOwned`), returning `Result<Option<T>, Error>`
- **URI constants**: `waapi_rs::ak` provides nested modules and constants matching WAAPI URI paths (e.g. `ak::wwise::core::GET_INFO`, `ak::wwise::waapi::GET_TOPICS`), avoiding hand-written strings
- **Topic subscriptions**: `subscribe` returns an event stream, or `subscribe_with_callback` binds a callback; cancel via `SubscriptionHandle` / `SubscriptionHandleSync`; auto-cleaned on drop
- **Resource cleanup**: connections and subscriptions auto-disconnect/cancel on `Drop`; explicit `disconnect` / `unsubscribe` also available

### Prerequisites

- **Wwise**: installed and running, with Authoring API enabled in the project
  (Project > User Preferences > Enable Wwise Authoring API)
- **Rust**: 1.70+ recommended, with `tokio` and async support

### Installation

Add the dependency to `Cargo.toml` (currently a git dependency):

```toml
[dependencies]
waapi-rs = { git = "https://github.com/xmimu/waapi-rs.git", branch = "dev" }
tokio = { version = "1", features = ["full"] }
```

From a local path:

```toml
waapi-rs = { path = "../waapi-rs" }
```

### Quick Example

Import `waapi_rs::ak` and write paths from `ak::` (consistent with C++ WAAPI URI style). `call_no_args::<Value>` returns `Option<Value>`:

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

#### URI Constants (`uris`)

Import with `use waapi_rs::ak`, then write paths from `ak::`, matching the WAAPI/C++ URI hierarchy (e.g. `ak.wwise.core.getInfo` → `ak::wwise::core::GET_INFO`):

- `ak::soundengine::*` — runtime interfaces (e.g. `POST_EVENT`, `SET_STATE`)
- `ak::wwise::core::*` — core interfaces (e.g. `GET_INFO`, `OBJECT_GET`) and topics (e.g. `OBJECT_CREATED`, `PROJECT_LOADED`)
- `ak::wwise::debug::*`, `ak::wwise::ui::*`, `ak::wwise::waapi::*` — debug, UI, WAAPI meta-info

Examples: `client.call_no_args::<Value>(ak::wwise::core::GET_INFO)`, `client.call(ak::wwise::waapi::GET_TOPICS, None, None)`, subscribe with `ak::wwise::ui::SELECTION_CHANGED`.

#### `call` Generics and Return Values

- The generic `T` in `call<T>(uri, args, options)` / `call_no_args<T>(uri)` is the **return value** deserialization type, requiring `DeserializeOwned` (e.g. `serde_json::Value` or a custom struct).
- `args` and `options` only need to be serializable (`impl Serialize`); they don't have to match `T`.
- Returns `Result<Option<T>, Error>`: on success, WAAPI kwargs are deserialized into `T`; `None` when there's no result.

### Examples and Tests

- Get Wwise version: `cargo run --example get_info`
- Subscribe to selection changes (callback): `cargo run --example subscribe`
- Run tests: `cargo test` (some tests require a local WAAPI, otherwise they skip)

### Docs and Design

- Generate and open API docs: `cargo doc --open`
- Development design and architecture: [DESIGN.md](docs/DESIGN.md)

### References

- [Wwise Authoring API official docs](https://www.audiokinetic.com/library/edge/?source=SDK&id=waapi.html)
- [waapi-client-python](https://github.com/audiokinetic/waapi-client-python) (API usage reference)
