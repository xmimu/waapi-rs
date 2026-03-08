# waapi-rs Design

[English](#english) | [中文](#中文)

---

<a id="english"></a>

## English

### Goals and Scope

- **Positioning**: waapi-rs is a Rust client for the Wwise Authoring API (WAAPI), communicating with the Wwise editor via the WAMP protocol.
- **Target users**: developers who need to call WAAPI from Rust (toolchains, automation, plugins, etc.).
- **Scope**: provides connection, RPC calls (call), topic subscriptions (subscribe), WAAPI URI constants (`ak::*`), and resource cleanup. Does not cover all advanced WAAPI features — only the WAMP subset currently in use (JSON serialization, default realm, etc.).

### Dependencies and Protocol

- **Core dependency**: `wamp_async` ([xmimu/wamp_async](https://github.com/xmimu/wamp_async), branch: dev), handling WebSocket connections and the WAMP protocol.
- **Communication**: WebSocket + WAMP, serialized as JSON; default URL `ws://localhost:8080/waapi`, default realm `realm1`.
- **Runtime**: `tokio` for async IO and task scheduling; the sync client internally uses a multi-threaded runtime.

### Architecture Overview

```mermaid
flowchart LR
    App[Application]
    Async[WaapiClient]
    Sync[WaapiClientSync]
    Wamp[wamp_async Client]
    WS[WebSocket]
    App --> Async
    App --> Sync
    Sync --> Async
    Async --> Wamp
    Wamp --> WS
```

- Applications use `WaapiClient` (async) or `WaapiClientSync` (sync).
- `WaapiClientSync` internally holds a `tokio::runtime::Runtime` and a `WaapiClient`, converting async calls to blocking via `block_on`.
- `WaapiClient` holds a `wamp_async::Client` and a `JoinHandle` for the event loop; the event loop is spawned via `tokio::spawn` on `connect` and aborted on `cleanup` or `Drop`.

### Core Type Responsibilities

| Type | Responsibility |
|------|---------------|
| **WaapiClient** | Async connect (`connect` / `connect_with_url`), RPC (`call` / `call_no_args`), subscriptions (`subscribe` / `subscribe_with_callback`), lifecycle management (`disconnect`, `cleanup`); shareable across tasks (internal `Arc` + `Mutex`). |
| **WaapiClientSync** | Creates and holds a multi-threaded runtime; exposes sync `connect` / `connect_with_url`, `call` / `call_no_args`, `subscribe` / `subscribe_with_callback`, `is_connected`, `disconnect`; for non-async environments. |
| **SubscriptionHandle** | Holds subscription ID, shared `Arc` with client, and optional `recv_task` (present when `subscribe_with_callback` is used); `unsubscribe()` cancels explicitly, `Drop` spawns async cancel in background to avoid blocking. |
| **SubscriptionHandleSync** | Cancels subscriptions created by the sync client; `unsubscribe()` or drop cancels and joins the bridge thread. **Do not drop inside a callback — may deadlock.** |

### RPC and URI Constants

- **call generics**: `call<T>(uri, args, options)` / `call_no_args<T>(uri)` — generic `T` is the **return value** deserialization type, requiring `DeserializeOwned` (e.g. `serde_json::Value` or a custom struct). Returns `Result<Option<T>, Error>`: on success, WAAPI kwargs are deserialized into `T`; `None` if no result. `args`/`options` are serializable types (typically `Value` or `impl Serialize`).
- **URI constants (uris)**: `src/uris.rs` organizes nested modules by WAAPI URI path (`ak::soundengine`, `ak::wwise::core`, `ak::wwise::debug`, `ak::wwise::ui`, `ak::wwise::waapi`), each providing `pub const XXX: &str = "ak.xxx.xxx"`. The library re-exports via `pub use uris::ak`; users only need `use waapi_rs::ak` and write paths from `ak::` (e.g. `ak::wwise::core::GET_INFO`), matching C++ WAAPI / official URI naming for easy autocomplete and avoiding hand-written strings.

### Subscription Model

- **`subscribe(topic)`**: returns `(SubscriptionHandle, UnboundedReceiver<SubscribeEvent>)`. Callers should consume the receiver in a separate task; backpressure is buffered by the unbounded channel. Cancel by calling `handle.unsubscribe()` or dropping the handle.
- **`subscribe_with_callback(topic, callback)`**: internally spawns a task that loops `recv()` and invokes `callback(args, kwargs)`; the callback runs in a dedicated task without blocking the event loop. On cancel, `SubscriptionHandle` aborts the task and unsubscribes.
- In both cases, dropping `SubscriptionHandle` removes from the client's `subscription_ids` and runs `unsubscribe` in the background, avoiding `.await` inside `Drop`.
- **Sync client**: `WaapiClientSync::subscribe(topic)` returns `(SubscriptionHandleSync, mpsc::Receiver<SubscribeEvent>)`; `subscribe_with_callback(topic, callback)` returns `SubscriptionHandleSync`. Cancel by calling `unsubscribe()` or dropping the handle. **Do not drop `SubscriptionHandleSync` inside a callback — may deadlock.**

### Resource and Lifecycle

- **Disconnect order**: unsubscribe all registered subscriptions → `leave_realm` → `disconnect`; finally abort the event loop task. Both `cleanup()` and `Drop` follow this order.
- **Drop behavior**: `WaapiClient` and `SubscriptionHandle` use `tokio::runtime::Handle::try_current()` to `spawn` async cleanup on an existing runtime when possible, avoiding blocking inside drop. If no runtime is available, only the event loop handle is aborted.

### Errors and Boundaries

- **Error type**: public APIs use `Result<T, WaapiError>`; `WaapiError` aggregates WAMP protocol (`WampError`), serialization (`serde_json::Error`), IO (`std::io::Error`), and client-disconnected (`Disconnected`) errors via `thiserror`.
- **"Client already disconnected"**: returned when `client` or `client.lock().await` is `None` (e.g. after `disconnect` or `cleanup` has been called).
- **Tests**: some tests depend on a local WAAPI (Wwise running with Authoring API enabled); on connection failure they `eprintln` an explanation and return without panicking, implementing a "WAAPI-optional CI-friendly" skip strategy.

### Python waapi-client Mapping

| Python (waapi-client-python) | waapi-rs |
|------------------------------|----------|
| `WaapiClient()` / `connect()` | `WaapiClient::connect().await` or `WaapiClientSync::connect()` |
| `client.call(uri, options=...)` | `client.call::<T>(uri, args, options)` or `call_no_args::<T>(uri)`; generic `T` is the return type, returns `Result<Option<T>, Error>`; URI via constants like `ak::wwise::core::GET_INFO` |
| `client.subscribe(topic, callback)` | `subscribe_with_callback(topic, \|args, kwargs\| { ... })` or `subscribe(topic)` + consume receiver manually; topic via `ak::wwise::ui::SELECTION_CHANGED` etc. |
| `handler.unsubscribe()` | `handle.unsubscribe().await` or drop `SubscriptionHandle` |
| `client.disconnect()` | `client.disconnect().await` or drop `WaapiClient` |

Useful for migration from Python.

### Future Directions (Optional)

- Common WAAPI URIs are already provided as `ak::*` constants; could further add typed wrappers (e.g. generate request/response structs from URI schemas).
- Configurable SSL verification and realm.
- Reconnection strategies and connection status callbacks.

---

<a id="中文"></a>

## 中文

### 目标与范围

- **定位**：waapi-rs 是 Wwise Authoring API (WAAPI) 的 Rust 客户端，通过 WAMP 协议与 Wwise 编辑器通信。
- **目标用户**：需要在 Rust 中调用 WAAPI 的开发者（工具链、自动化、插件等）。
- **范围**：提供连接、RPC 调用 (call)、主题订阅 (subscribe)、WAAPI URI 常量（`ak::*`）及资源清理；不覆盖 WAAPI 的全部高级特性，仅支持当前使用的 WAMP 子集（JSON 序列化、默认 realm 等）。

### 依赖与协议

- **核心依赖**：`wamp_async`（[xmimu/wamp_async](https://github.com/xmimu/wamp_async)，branch: dev），负责 WebSocket 连接与 WAMP 协议。
- **通信方式**：WebSocket + WAMP，序列化使用 JSON；默认 URL `ws://localhost:8080/waapi`，默认 realm `realm1`。
- **运行时**：`tokio`，用于异步 IO 与任务调度；同步客户端内部使用多线程 runtime。

### 架构概览

```mermaid
flowchart LR
    App[应用代码]
    Async[WaapiClient]
    Sync[WaapiClientSync]
    Wamp[wamp_async Client]
    WS[WebSocket]
    App --> Async
    App --> Sync
    Sync --> Async
    Async --> Wamp
    Wamp --> WS
```

- 应用层使用 `WaapiClient`（异步）或 `WaapiClientSync`（同步）。
- `WaapiClientSync` 内部持有一个 `tokio::runtime::Runtime` 和 `WaapiClient`，通过 `block_on` 将异步调用转为阻塞。
- `WaapiClient` 持有 `wamp_async::Client` 和事件循环的 `JoinHandle`；事件循环在 `connect` 时由 `tokio::spawn` 启动，在 `cleanup` 或 `Drop` 时被 abort。

### 核心类型职责

| 类型 | 职责 |
|------|------|
| **WaapiClient** | 异步连接（`connect` / `connect_with_url`）、RPC（`call` / `call_no_args`）、订阅（`subscribe` / `subscribe_with_callback`）、生命周期管理（`disconnect`、`cleanup`）；可在多任务间共享（内部用 `Arc` + `Mutex`）。 |
| **WaapiClientSync** | 内部创建并持有多线程 runtime，对外提供同步的 `connect` / `connect_with_url`、`call` / `call_no_args`、`subscribe` / `subscribe_with_callback`、`is_connected`、`disconnect`；适用于非 async 环境。 |
| **SubscriptionHandle** | 持有订阅 ID、与 client 共享的 `Arc`、以及可选的 `recv_task`（`subscribe_with_callback` 时存在）；`unsubscribe()` 显式取消，`Drop` 时也会在后台 spawn 异步取消，避免阻塞。 |
| **SubscriptionHandleSync** | 用于取消同步客户端创建的订阅；`unsubscribe()` 或 drop 时取消订阅并 join 桥接线程；**禁止在回调内部 drop，否则可能死锁。** |

### RPC 与 URI 常量

- **call 泛型**：`call<T>(uri, args, options)`、`call_no_args<T>(uri)` 的泛型 `T` 表示**返回值**的反序列化类型，需满足 `DeserializeOwned`（如 `serde_json::Value` 或自定义结构体）。返回 `Result<Option<T>, Error>`：成功时 WAAPI 的 kwargs 反序列化为 `T`，无结果时为 `None`；args/options 仍为可序列化类型（通常 `Value` 或 `impl Serialize`）。
- **URI 常量（uris）**：`src/uris.rs` 中按 WAAPI URI 路径组织嵌套模块（`ak::soundengine`、`ak::wwise::core`、`ak::wwise::debug`、`ak::wwise::ui`、`ak::wwise::waapi`），每层提供 `pub const XXX: &str = "ak.xxx.xxx"`。库通过 `pub use uris::ak` 重导出，用户只需 `use waapi_rs::ak`，调用时从 `ak::` 写路径（如 `ak::wwise::core::GET_INFO`），与 C++ WAAPI / 官方 URI 命名一致，便于补全与避免手写字符串。

### 订阅模型

- **`subscribe(topic)`**：返回 `(SubscriptionHandle, UnboundedReceiver<SubscribeEvent>)`。调用方需在单独 task 中消费 receiver；背压由 unbounded channel 缓冲。取消方式：调用 `handle.unsubscribe()` 或 drop handle。
- **`subscribe_with_callback(topic, callback)`**：内部 spawn 一个 task 循环 `recv()` 并调用 `callback(args, kwargs)`；回调在独立 task 中运行，不阻塞事件循环。取消时 `SubscriptionHandle` 会 abort 该 task 并 unsubscribe。
- 两种方式下，drop `SubscriptionHandle` 都会从 client 的 `subscription_ids` 中移除并在后台执行 `unsubscribe`，避免在 `Drop` 里做 `.await`。
- **同步客户端**：`WaapiClientSync::subscribe(topic)` 返回 `(SubscriptionHandleSync, mpsc::Receiver<SubscribeEvent>)`，`subscribe_with_callback(topic, callback)` 返回 `SubscriptionHandleSync`。取消方式为调用 `unsubscribe()` 或 drop 句柄。**注意：不要在回调内部 drop `SubscriptionHandleSync`，否则可能死锁。**

### 资源与生命周期

- **断开顺序**：先对所有已登记订阅执行 `unsubscribe`，再 `leave_realm`，再 `disconnect`；最后 abort 事件循环 task。`cleanup()` 与 `Drop` 均遵循该顺序。
- **Drop 行为**：`WaapiClient` 和 `SubscriptionHandle` 的 `Drop` 在可能的情况下通过 `tokio::runtime::Handle::try_current()` 在已有 runtime 上 `spawn` 异步清理，避免在 drop 中阻塞；若无当前 runtime 则仅 abort 事件循环 handle。

### 错误与边界

- **错误类型**：公开 API 使用 `Result<T, WaapiError>`；`WaapiError` 通过 `thiserror` 聚合了 WAMP 协议（`WampError`）、序列化（`serde_json::Error`）、IO（`std::io::Error`）及客户端已断开（`Disconnected`）四类错误。
- **"Client already disconnected"**：在 `client` 或 `client.lock().await` 为 `None` 时返回（例如已调用 `disconnect` 或 `cleanup` 之后再次 call/subscribe）。
- **测试**：部分测试依赖本机 WAAPI（Wwise 已启动且启用 Authoring API）；若连接失败则 `eprintln` 说明并 return，不 panic，实现"可选 WAAPI 的 CI 友好"的跳过策略。

### 与 Python waapi-client 的对应关系

| Python (waapi-client-python) | waapi-rs |
|------------------------------|----------|
| `WaapiClient()` / `connect()` | `WaapiClient::connect().await` 或 `WaapiClientSync::connect()` |
| `client.call(uri, options=...)` | `client.call::<T>(uri, args, options)` 或 `call_no_args::<T>(uri)`，泛型 `T` 为返回值类型，返回 `Result<Option<T>, Error>`；URI 可用常量如 `ak::wwise::core::GET_INFO` |
| `client.subscribe(topic, callback)` | `subscribe_with_callback(topic, \|args, kwargs\| { ... })` 或 `subscribe(topic)` + 自行消费 receiver；主题可用 `ak::wwise::ui::SELECTION_CHANGED` 等 |
| `handler.unsubscribe()` | `handle.unsubscribe().await` 或 drop `SubscriptionHandle` |
| `client.disconnect()` | `client.disconnect().await` 或 drop `WaapiClient` |

便于从 Python 迁移时对照使用。

### 未来可扩展方向（可选）

- 常用 WAAPI URI 已以 `ak::*` 常量形式提供；可进一步做类型化封装（如按 URI 的 schema 生成请求/响应结构体）。
- 可配置 SSL 校验与 realm。
- 重连策略与连接状态回调。
