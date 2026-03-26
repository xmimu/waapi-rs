# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-26

### Breaking Changes

- `subscribe` callback signature changed from `|args, kwargs|` to `|kwargs|` — WAAPI events only carry kwargs, the args parameter was always `None`

### Added

- `src/wamp.rs`: self-contained WAMP Basic Profile implementation (HELLO/WELCOME, CALL/RESULT, SUBSCRIBE/SUBSCRIBED/EVENT, UNSUBSCRIBE/UNSUBSCRIBED, GOODBYE/ERROR) over JSON, no external WAMP library required
- `examples/monitor.rs`: auto-reconnect monitor example

### Changed

- Replaced `wamp_async` git dependency with `tokio-tungstenite` + `futures-util` (both crates.io); project is now publishable to crates.io
- Rewrote `src/client.rs` using an actor pattern (`WampConn`): WebSocket sink behind `tokio::sync::Mutex`, pending RPC/subscribe/unsubscribe responses in `std::sync::Mutex<HashMap>` keyed by request ID, event delivery via `mpsc::UnboundedSender`
- `WaapiError::WebSocket` variant now wraps `Box<tungstenite::Error>` to reduce stack size of `Result` return types
- Public API no longer exposes any `wamp_async` types; all values use `serde_json::Value`
- Merged `tests/subscribe_callback.rs` into `tests/subscribe.rs`

### Removed

- `src/args.rs` — logic inlined into `client.rs`

### Fixed

- Sync client subscription unsubscribe deadlock: `event_senders` entry is now removed before joining the bridge thread, allowing `recv()` to return `None` and exit cleanly

## [0.1.0] - 2025

### Added

- Async client `WaapiClient`: connect, `call`, `subscribe`, `disconnect`
- Sync client `WaapiClientSync`: blocking wrappers for non-async environments
- `SubscriptionHandle` / `SubscriptionHandleSync`: explicit `unsubscribe()` or auto-cancel on drop
- URI constants under `waapi_rs::ak` (`ak::wwise::core::*`, `ak::wwise::ui::*`, `ak::soundengine::*`, etc.)
- `subscribe(topic, options, callback)` with `options: Option<Value>` for server-side filtering
- Custom `WaapiError` type via `thiserror`
- Examples: `get_info`, `subscribe`, `get_func`, `waql`
- CI-friendly tests: skip gracefully when Wwise is not running
