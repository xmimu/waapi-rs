//! WAAPI client implementation: async [WaapiClient] and sync [WaapiClientSync].
//!
//! Connection lifecycle: after `connect`, joins the default realm; on disconnect,
//! all subscriptions are cancelled first, then `leave_realm`, then `disconnect`.
//! Subscriptions are managed via [SubscriptionHandle]; explicit [SubscriptionHandle::unsubscribe]
//! or drop will cancel automatically. For sync clients, [SubscriptionHandleSync] is used.
//!
//! ---
//!
//! WAAPI 客户端实现：异步 [WaapiClient] 与同步 [WaapiClientSync]。
//!
//! 连接生命周期：`connect` 后加入默认 realm，断开时先取消所有订阅、再 leave_realm、再 disconnect。
//! 订阅通过 [SubscriptionHandle] 管理，显式 [SubscriptionHandle::unsubscribe] 或 drop 时自动取消；同步客户端下由 [SubscriptionHandleSync] 管理。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;

use serde_json::Value;
use tokio::sync::Mutex as TokioMutex;
use wamp_async::{Client, ClientConfig, SerializerType, WampError, WampId, WampKwArgs};

use log::{debug, info, warn};

use crate::args::{value_to_kwargs, value_to_wamp_dict, wamp_result_to_value};

/// Default WAAPI WebSocket URL (default port for Wwise local Authoring API).
///
/// ---
///
/// 默认 WAAPI WebSocket 地址（Wwise 本机 Authoring API 默认端口）。
const DEFAULT_WAAPI_URL: &str = "ws://localhost:8080/waapi";

/// Default WAMP realm name, matching the Wwise WAAPI server default.
///
/// ---
///
/// 连接时使用的默认 WAMP realm 名称，与 Wwise WAAPI 服务端默认一致。
const DEFAULT_REALM: &str = "realm1";

/// WAAPI client error type.
///
/// ---
///
/// WAAPI 客户端错误类型。
#[derive(Debug, thiserror::Error)]
pub enum WaapiError {
    /// Client already disconnected.
    ///
    /// 客户端已断开连接。
    #[error("client already disconnected")]
    Disconnected,
    /// WAMP protocol error.
    ///
    /// WAMP 协议层错误。
    #[error("{0}")]
    Wamp(#[from] WampError),
    /// Serialization / deserialization error.
    ///
    /// 序列化/反序列化错误。
    #[error("{0}")]
    Serde(#[from] serde_json::Error),
    /// IO error (e.g. failed to create tokio runtime).
    ///
    /// IO 错误（如 tokio runtime 创建失败）。
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Subscription event payload: `(PublicationId, args, kwargs)`.
///
/// Received from a receiver or callback; typically parse event content from
/// `args` / `kwargs`. `PublicationId` can be used for deduplication or logging.
///
/// ---
///
/// 订阅事件 payload：`(PublicationId, args, kwargs)`。
///
/// 从 receiver 或回调中收到；通常使用 `args` / `kwargs` 解析事件内容，PublicationId 可用于去重或日志。
pub type SubscribeEvent = (WampId, Option<wamp_async::WampArgs>, Option<WampKwArgs>);

/// Subscription handle: used to cancel a subscription; automatically unsubscribes
/// in the background on drop.
///
/// Holds a recv-loop task that will be aborted on unsubscribe or drop.
///
/// ---
///
/// 订阅句柄：用于取消订阅；drop 时会自动在后台执行 unsubscribe。
///
/// 内部持有一个 recv 循环 task，unsubscribe 或 drop 时会先 abort 该 task。
pub struct SubscriptionHandle {
    sub_id: WampId,
    client: Arc<TokioMutex<Option<Client<'static>>>>,
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
    recv_task: Option<tokio::task::JoinHandle<()>>,
    is_unsubscribed: bool,
}

/// Marks as unsubscribed; returns whether this is the first time
/// (true = should proceed with network unsubscribe).
///
/// ---
///
/// 标记为已退订，返回本次是否首次标记（true 表示应继续执行网络退订）。
fn mark_unsubscribed(flag: &mut bool) -> bool {
    if *flag {
        false
    } else {
        *flag = true;
        true
    }
}

/// Async WAAPI client.
///
/// Provides async access to the Wwise Authoring API (WAAPI);
/// can be shared across tasks (internal Arc + Mutex).
///
/// The client makes a best-effort cleanup on Drop, but spawned async tasks
/// are not guaranteed to finish before process exit.
/// **It is recommended to call [`disconnect`](WaapiClient::disconnect) explicitly**
/// for graceful shutdown.
///
/// ---
///
/// WAAPI 异步客户端。
///
/// 提供异步接口访问 Wwise Authoring API (WAAPI)；可在多任务间共享使用（内部 Arc + Mutex）。
///
/// 客户端在 Drop 时会尽力清理资源，但 spawn 的异步任务不保证在进程退出前完成，
/// **建议显式调用 [`disconnect`](WaapiClient::disconnect)** 以确保优雅关闭。
pub struct WaapiClient {
    /// Outer Option for take() during cleanup/disconnect;
    /// inner Option (inside Mutex) for take() to call `Client::disconnect(self)`.
    ///
    /// 外层 Option 供 cleanup/disconnect 时 take() 转移所有权；
    /// 内层 Option（Mutex 内）供 disconnect 逻辑 take() 出 Client 以调用 `Client::disconnect(self)`。
    client: Option<Arc<TokioMutex<Option<Client<'static>>>>>,
    event_loop_handle: Option<tokio::task::JoinHandle<Result<(), WampError>>>,
    /// Active subscription IDs; all unsubscribed on disconnect.
    ///
    /// 当前活跃的订阅 ID，disconnect 时统一 unsubscribe。
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
    /// Connection liveness flag: set to `false` when cleanup begins or the event loop terminates
    /// (e.g. Wwise process exit / network drop). Shared with the event-loop monitor task.
    ///
    /// 连接存活标志：cleanup 开始时或事件循环终止时（如 Wwise 进程退出/网络断开）置 `false`。
    /// 与事件循环监控 task 共享。
    connected: Arc<AtomicBool>,
}

/// Async cleanup from taken fields: unsubscribe all → leave_realm → disconnect → abort event loop.
///
/// Shared by [WaapiClient::cleanup] and `Drop for WaapiClient` to avoid logic duplication.
///
/// ---
///
/// 从已取出的字段执行异步清理：取消所有订阅 → leave_realm → disconnect → abort 事件循环。
///
/// 供 [WaapiClient::cleanup] 与 `Drop for WaapiClient` 共用，避免逻辑重复。
async fn do_cleanup(
    client_arc: Option<Arc<TokioMutex<Option<Client<'static>>>>>,
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
    event_loop: Option<tokio::task::JoinHandle<Result<(), WampError>>>,
    connected: Arc<AtomicBool>,
) {
    // Mark disconnected immediately so is_connected() reflects reality at once.
    // 立即标记为断开，使 is_connected() 第一时间反映真实状态。
    connected.store(false, Ordering::Release);

    // If the event loop has already finished (e.g. Wwise crashed / network drop),
    // the underlying WAMP channel is closed and calling unsubscribe/leave_realm/disconnect
    // would panic inside wamp_async. Skip WAMP-level ops; only clear local state.
    //
    // 若事件循环已结束（如 Wwise 崩溃/网络断开），WAMP 底层 channel 已关闭，
    // 继续调用 unsubscribe/leave_realm/disconnect 会在 wamp_async 内 panic。
    // 跳过 WAMP 层操作，仅清理本地状态。
    let wamp_alive = event_loop.as_ref().map(|h| !h.is_finished()).unwrap_or(false);

    if let Some(arc) = client_arc {
        let ids: Vec<WampId> = {
            let mut guard = subscription_ids.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *guard)
        };
        let mut client_guard = arc.lock().await;
        if wamp_alive {
            if let Some(ref mut c) = *client_guard {
                for sub_id in ids {
                    let _ = c.unsubscribe(sub_id).await;
                }
                let _ = c.leave_realm().await;
            }
            if let Some(c) = client_guard.take() {
                c.disconnect().await;
            }
        } else {
            // Drop the inner Client without WAMP calls; subsequent SubscriptionHandle
            // drops will see None and skip network unsubscribe safely.
            //
            // 直接 drop 内层 Client，不执行 WAMP 调用；
            // 后续 SubscriptionHandle drop 时看到 None，安全跳过网络退订。
            client_guard.take();
        }
    }

    if let Some(handle) = event_loop {
        handle.abort();
    }
}

impl WaapiClient {
    /// Connect to WAAPI using the default URL.
    ///
    /// Connects to `ws://localhost:8080/waapi` with the default realm.
    ///
    /// ---
    ///
    /// 使用默认 URL 连接到 WAAPI。
    ///
    /// 默认连接到 `ws://localhost:8080/waapi`，使用默认 realm。
    pub async fn connect() -> Result<Self, WaapiError> {
        Self::connect_with_url(DEFAULT_WAAPI_URL).await
    }

    /// Connect to WAAPI at the specified URL.
    ///
    /// Joins the default realm after connecting; serialization is JSON, SSL verification disabled.
    ///
    /// ---
    ///
    /// 使用指定 URL 连接到 WAAPI。
    ///
    /// 连接后加入默认 realm；序列化为 JSON，SSL 校验关闭。
    pub async fn connect_with_url(url: &str) -> Result<Self, WaapiError> {
        info!("Connecting to WAAPI at {url}");
        let (mut client, (event_loop, _)) = Client::connect(
            url,
            Some(
                ClientConfig::default()
                    .set_ssl_verify(false)
                    .set_serializers(vec![SerializerType::Json]),
            ),
        )
        .await?;

        let connected = Arc::new(AtomicBool::new(true));
        let connected_flag = Arc::clone(&connected);
        let handle = tokio::spawn(async move {
            let result = event_loop.await;
            // Event loop terminated (normal disconnect or network/Wwise crash).
            // 事件循环结束（正常断开或网络/Wwise 崩溃），立即更新连接标志。
            connected_flag.store(false, Ordering::Release);
            result
        });
        client.join_realm(DEFAULT_REALM).await?;
        info!("Connected to WAAPI at {url}");

        let client = Arc::new(TokioMutex::new(Some(client)));
        Ok(Self {
            client: Some(client),
            event_loop_handle: Some(handle),
            subscription_ids: Arc::new(StdMutex::new(Vec::new())),
            connected,
        })
    }

    /// Call a WAAPI method.
    ///
    /// # Parameters
    ///
    /// * `uri` - URI of the WAAPI method, e.g. `"ak.wwise.core.getInfo"` or `ak::wwise::core::GET_INFO`
    /// * `args` - Optional keyword arguments (`serde_json::Value`, e.g. `json!({...})`)
    /// * `options` - Optional options dict (`serde_json::Value`, may differ from `args`)
    ///
    /// Returns `Option<Value>`: WAAPI kwargs deserialized as JSON; `None` when no result.
    ///
    /// Note: holds the connection lock for the entire RPC call; only one call executes at a time.
    ///
    /// ---
    ///
    /// 调用 WAAPI 方法。
    ///
    /// # 参数
    ///
    /// * `uri` - WAAPI 方法的 URI，如 `"ak.wwise.core.getInfo"` 或 `ak::wwise::core::GET_INFO`
    /// * `args` - 可选的关键字参数（`serde_json::Value`，如 `json!({...})`）
    /// * `options` - 可选的选项字典（`serde_json::Value`，可与 args 不同）
    ///
    /// 返回 `Option<Value>`：WAAPI kwargs 反序列化为 JSON；无结果时为 `None`。
    ///
    /// 注意：内部在整个 RPC 调用期间持有连接锁，同一时间只能有一个调用在执行。
    pub async fn call(
        &self,
        uri: &str,
        args: Option<Value>,
        options: Option<Value>,
    ) -> Result<Option<Value>, WaapiError> {
        let args = args.map(value_to_kwargs).transpose()?;
        let options = options.map(value_to_wamp_dict).transpose()?;
        let client = self.client.as_ref().ok_or(WaapiError::Disconnected)?;
        debug!("Calling WAAPI: {uri}");
        let (_, result) = client
            .lock()
            .await
            .as_ref()
            .ok_or(WaapiError::Disconnected)?
            .call(uri, None, args, options)
            .await?;
        let out = result.map(wamp_result_to_value).transpose()?;
        Ok(out)
    }

    /// Internal subscribe: returns handle and receiver. Used by [WaapiClientSync] to bridge events.
    pub(crate) async fn subscribe_inner(
        &self,
        topic: &str,
        options: Option<Value>,
    ) -> Result<
        (
            SubscriptionHandle,
            tokio::sync::mpsc::UnboundedReceiver<SubscribeEvent>,
        ),
        WaapiError,
    > {
        let options = options.map(value_to_wamp_dict).transpose()?;
        let client = self.client.as_ref().ok_or(WaapiError::Disconnected)?;
        let (sub_id, queue) = client
            .lock()
            .await
            .as_ref()
            .ok_or(WaapiError::Disconnected)?
            .subscribe(topic, options)
            .await?;
        self.subscription_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(sub_id);
        debug!("Subscribed to {topic} (sub_id={sub_id})");
        let handle = SubscriptionHandle {
            sub_id,
            client: Arc::clone(client),
            subscription_ids: Arc::clone(&self.subscription_ids),
            recv_task: None,
            is_unsubscribed: false,
        };
        Ok((handle, queue))
    }

    /// Subscribe to a topic with a callback: spawns a task to receive events
    /// and invoke `callback(args, kwargs)`.
    ///
    /// The callback runs in a dedicated task and does not block the event loop.
    /// The returned handle is used to cancel; on drop it aborts the task and auto-unsubscribes.
    ///
    /// # Parameters
    ///
    /// * `topic` - WAMP topic URI, e.g. `"ak.wwise.ui.selectionChanged"` or `ak::wwise::ui::SELECTION_CHANGED`
    /// * `options` - Optional subscription options (`serde_json::Value`), e.g. filtering or return fields
    /// * `callback` - Callback invoked on each event with `(args, kwargs)`
    ///
    /// ---
    ///
    /// 订阅主题并绑定回调：内部 spawn 一个 task 循环接收事件并调用 `callback(args, kwargs)`。
    ///
    /// 回调在独立 task 中执行，不阻塞事件循环。返回的句柄用于取消订阅；drop 时会 abort 该 task 并自动 unsubscribe。
    ///
    /// # 参数
    ///
    /// * `topic` - WAMP 主题 URI，如 `"ak.wwise.ui.selectionChanged"` 或 `ak::wwise::ui::SELECTION_CHANGED`
    /// * `options` - 可选的订阅选项（`serde_json::Value`），如过滤、返回字段等
    /// * `callback` - 每次事件触发时调用的回调，参数为 `(args, kwargs)`
    pub async fn subscribe<F>(
        &self,
        topic: &str,
        options: Option<Value>,
        callback: F,
    ) -> Result<SubscriptionHandle, WaapiError>
    where
        F: Fn(Option<wamp_async::WampArgs>, Option<WampKwArgs>) + Send + Sync + 'static,
    {
        let options = options.map(value_to_wamp_dict).transpose()?;
        let client = self.client.as_ref().ok_or(WaapiError::Disconnected)?;
        let (sub_id, mut queue) = client
            .lock()
            .await
            .as_ref()
            .ok_or(WaapiError::Disconnected)?
            .subscribe(topic, options)
            .await?;
        self.subscription_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(sub_id);
        debug!("Subscribed to {topic} (sub_id={sub_id})");
        let recv_task = tokio::spawn(async move {
            while let Some((_, args, kwargs)) = queue.recv().await {
                callback(args, kwargs);
            }
        });
        let handle = SubscriptionHandle {
            sub_id,
            client: Arc::clone(client),
            subscription_ids: Arc::clone(&self.subscription_ids),
            recv_task: Some(recv_task),
            is_unsubscribed: false,
        };
        Ok(handle)
    }

    /// Check whether the client is still connected.
    ///
    /// Returns `false` if:
    /// - [`disconnect`](WaapiClient::disconnect) has been called, or
    /// - the event loop has terminated (e.g. Wwise process exit or network drop).
    ///
    /// Note: updated reactively when the event loop terminates; does not actively probe
    /// the underlying WebSocket.
    ///
    /// ---
    ///
    /// 检查客户端是否仍处于连接状态。
    ///
    /// 以下情况返回 `false`：
    /// - 已调用 [`disconnect`](WaapiClient::disconnect)，或
    /// - 事件循环已终止（如 Wwise 进程退出或网络断开）。
    ///
    /// 注意：事件循环终止时被动更新，不主动探测底层 WebSocket 是否存活。
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.is_some() && self.connected.load(Ordering::Acquire)
    }

    /// Explicitly disconnect.
    ///
    /// Even without calling this, Drop will try to clean up,
    /// but **explicit call is recommended** for graceful shutdown.
    ///
    /// ---
    ///
    /// 显式断开连接。
    ///
    /// 即使不调用此方法，Drop 时也会尽力清理，但 **推荐显式调用** 以确保优雅关闭。
    pub async fn disconnect(mut self) {
        info!("Disconnecting from WAAPI");
        self.cleanup().await;
        info!("Disconnected from WAAPI");
    }

    /// Internal cleanup: delegates to [do_cleanup].
    ///
    /// 内部清理：委托给 [do_cleanup]。
    async fn cleanup(&mut self) {
        do_cleanup(
            self.client.take(),
            Arc::clone(&self.subscription_ids),
            self.event_loop_handle.take(),
            Arc::clone(&self.connected),
        )
        .await;
    }
}

impl SubscriptionHandle {
    /// Cancel the subscription and stop the callback loop (if any).
    ///
    /// ---
    ///
    /// 取消订阅并停止回调循环（若有）。
    pub async fn unsubscribe(mut self) -> Result<(), WaapiError> {
        debug!("Unsubscribing sub_id={}", self.sub_id);
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
        self.subscription_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|&id| id != self.sub_id);
        if !mark_unsubscribed(&mut self.is_unsubscribed) {
            return Ok(());
        }
        if let Some(ref c) = *self.client.lock().await {
            c.unsubscribe(self.sub_id).await?;
        }
        Ok(())
    }
}

impl Drop for SubscriptionHandle {
    /// Async cleanup via try_current(): spawns an unsubscribe task on the existing runtime,
    /// avoiding `.await` inside drop. Falls back to local-only cleanup if no runtime is available.
    ///
    /// 异步清理：通过 try_current() 在已有 runtime 上 spawn unsubscribe 任务，避免在 drop 中 .await 阻塞；
    /// 若无当前 runtime 则仅清理本地状态，跳过网络取消（连接已失效）。
    fn drop(&mut self) {
        let sub_id = self.sub_id;
        let client = Arc::clone(&self.client);
        let subscription_ids = Arc::clone(&self.subscription_ids);
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
        subscription_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|&id| id != sub_id);
        if !mark_unsubscribed(&mut self.is_unsubscribed) {
            return;
        }
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            debug!("SubscriptionHandle dropped, spawning unsubscribe for sub_id={sub_id}");
            rt.spawn(async move {
                if let Some(ref c) = *client.lock().await {
                    let _ = c.unsubscribe(sub_id).await;
                }
            });
        } else {
            warn!("SubscriptionHandle dropped without runtime, skipping network unsubscribe for sub_id={sub_id}");
        }
    }
}

impl Drop for WaapiClient {
    /// Spawns async cleanup if a runtime is available; otherwise only aborts the event loop.
    ///
    /// 若有当前 runtime 则 spawn 异步清理，否则仅 abort 事件循环。
    fn drop(&mut self) {
        if self.client.is_some() || self.event_loop_handle.is_some() {
            let client_arc = self.client.take();
            let event_loop = self.event_loop_handle.take();
            let subscription_ids = Arc::clone(&self.subscription_ids);
            let connected = Arc::clone(&self.connected);
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                debug!("WaapiClient dropped, spawning async cleanup");
                rt.spawn(do_cleanup(client_arc, subscription_ids, event_loop, connected));
            } else {
                warn!("WaapiClient dropped without runtime, skipping graceful cleanup");
                connected.store(false, Ordering::Release);
                if let Some(h) = event_loop {
                    h.abort();
                }
            }
        }
    }
}

/// Sync subscription handle: cancels subscriptions created by [WaapiClientSync::subscribe].
///
/// Calls [SubscriptionHandleSync::unsubscribe] or drop to cancel and wait for the bridge thread.
/// **Do not drop this handle inside a callback — it may deadlock.**
///
/// ---
///
/// 同步订阅句柄：用于取消通过 [WaapiClientSync::subscribe] 创建的订阅。
///
/// 调用 [SubscriptionHandleSync::unsubscribe] 或 drop 时取消订阅并等待桥接线程结束。
/// **注意：不要在回调内部 drop 本句柄，否则可能死锁。**
pub struct SubscriptionHandleSync {
    runtime: Arc<tokio::runtime::Runtime>,
    inner: Option<SubscriptionHandle>,
    bridge_join: Option<thread::JoinHandle<()>>,
    bridge_thread_id: Option<thread::ThreadId>,
}

impl SubscriptionHandleSync {
    /// Cancel the subscription and wait for the event bridge thread to finish.
    ///
    /// ---
    ///
    /// 取消订阅并等待事件桥接线程结束。
    pub fn unsubscribe(mut self) -> Result<(), WaapiError> {
        let inner = self.inner.take();
        let bridge_join = self.bridge_join.take();
        if let Some(h) = inner {
            self.runtime.block_on(h.unsubscribe())?;
        }
        if let Some(jh) = bridge_join {
            let _ = jh.join();
        }
        Ok(())
    }
}

impl Drop for SubscriptionHandleSync {
    /// When not on the bridge thread: block_on unsubscribe and join the bridge thread.
    /// Inside the bridge thread or async context: falls back to spawn to avoid deadlock/panic.
    ///
    /// 不在桥接线程时：block_on 执行 unsubscribe 并 join 桥接线程；
    /// 在桥接线程或 async 上下文中则降级为 spawn，避免死锁/panic。
    fn drop(&mut self) {
        let is_bridge_thread = self.bridge_thread_id.as_ref() == Some(&thread::current().id());
        let inner = self.inner.take();
        let bridge_join = self.bridge_join.take();
        let runtime = Arc::clone(&self.runtime);
        if let Some(h) = inner {
            if tokio::runtime::Handle::try_current().is_ok() {
                warn!("SubscriptionHandleSync dropped inside async context, falling back to spawn");
                runtime.handle().spawn(async move {
                    let _ = h.unsubscribe().await;
                });
            } else {
                let _ = runtime.block_on(h.unsubscribe());
            }
        }
        if !is_bridge_thread {
            if let Some(jh) = bridge_join {
                let _ = jh.join();
            }
        }
    }
}

/// Sync WAAPI client.
///
/// Provides sync access to the Wwise Authoring API (WAAPI); internally uses a multi-threaded
/// tokio runtime and wraps [WaapiClient] via `block_on`. Suitable for scripts and non-async code;
/// if already in an async context, prefer [WaapiClient] directly.
///
/// The client auto-cleans on Drop, but **explicit [`disconnect`](WaapiClientSync::disconnect)
/// is recommended** for graceful shutdown.
///
/// ---
///
/// WAAPI 同步客户端。
///
/// 提供同步接口访问 Wwise Authoring API (WAAPI)；内部使用多线程 tokio runtime，通过 `block_on` 封装 [WaapiClient]。
/// 适用于脚本、非 async 代码；若已在 async 环境中，建议直接使用 [WaapiClient]。
///
/// 客户端在 Drop 时会自动清理资源，但 **推荐显式调用 [`disconnect`](WaapiClientSync::disconnect)** 以确保优雅关闭。
pub struct WaapiClientSync {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Option<WaapiClient>,
}

impl WaapiClientSync {
    /// Connect to WAAPI using the default URL.
    ///
    /// Connects to `ws://localhost:8080/waapi` with the default realm.
    ///
    /// ---
    ///
    /// 使用默认 URL 连接到 WAAPI。
    ///
    /// 默认连接到 `ws://localhost:8080/waapi`，使用默认 realm。
    pub fn connect() -> Result<Self, WaapiError> {
        Self::connect_with_url(DEFAULT_WAAPI_URL)
    }

    /// Connect to WAAPI at the specified URL.
    ///
    /// Joins the default realm after connecting; internally uses `block_on` to call
    /// the async client. Serialization and SSL behavior match the async variant.
    ///
    /// ---
    ///
    /// 使用指定 URL 连接到 WAAPI。
    ///
    /// 连接后加入默认 realm；内部通过 block_on 调用异步客户端，序列化与 SSL 行为与异步版一致。
    pub fn connect_with_url(url: &str) -> Result<Self, WaapiError> {
        info!("Connecting to WAAPI (sync) at {url}");
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?,
        );
        let client = runtime.block_on(WaapiClient::connect_with_url(url))?;
        info!("Connected to WAAPI (sync) at {url}");

        Ok(Self {
            runtime,
            client: Some(client),
        })
    }

    /// Call a WAAPI method.
    ///
    /// # Parameters
    ///
    /// * `uri` - URI of the WAAPI method, e.g. `"ak.wwise.core.getInfo"` or `ak::wwise::core::GET_INFO`
    /// * `args` - Optional keyword arguments (`serde_json::Value`)
    /// * `options` - Optional options dict (`serde_json::Value`, may differ from `args`)
    ///
    /// Returns `Option<Value>`: WAAPI kwargs deserialized as JSON; `None` when no result.
    ///
    /// ---
    ///
    /// 调用 WAAPI 方法。
    ///
    /// # 参数
    ///
    /// * `uri` - WAAPI 方法的 URI，如 `"ak.wwise.core.getInfo"` 或 `ak::wwise::core::GET_INFO`
    /// * `args` - 可选的关键字参数（`serde_json::Value`）
    /// * `options` - 可选的选项字典（`serde_json::Value`，可与 args 不同）
    ///
    /// 返回 `Option<Value>`：WAAPI kwargs 反序列化为 JSON；无结果时为 `None`。
    pub fn call(
        &self,
        uri: &str,
        args: Option<Value>,
        options: Option<Value>,
    ) -> Result<Option<Value>, WaapiError> {
        let client = self.client.as_ref().ok_or(WaapiError::Disconnected)?;
        self.runtime.block_on(client.call(uri, args, options))
    }

    /// Subscribe to a topic with a callback: receives events in a dedicated thread
    /// and synchronously calls `callback(args, kwargs)`.
    ///
    /// To unsubscribe: call [SubscriptionHandleSync::unsubscribe] or drop the handle.
    /// Do not drop the handle inside the callback.
    ///
    /// # Parameters
    ///
    /// * `topic` - WAMP topic URI, e.g. `"ak.wwise.ui.selectionChanged"` or `ak::wwise::ui::SELECTION_CHANGED`
    /// * `options` - Optional subscription options (`serde_json::Value`), e.g. filtering or return fields
    /// * `callback` - Callback invoked on each event with `(args, kwargs)`
    ///
    /// ---
    ///
    /// 订阅主题并绑定回调：在独立线程中接收事件并同步调用 `callback(args, kwargs)`。
    ///
    /// 取消订阅：调用返回的 [SubscriptionHandleSync::unsubscribe]，或 drop 句柄。不要在 callback 内 drop 句柄。
    ///
    /// # 参数
    ///
    /// * `topic` - WAMP 主题 URI，如 `"ak.wwise.ui.selectionChanged"` 或 `ak::wwise::ui::SELECTION_CHANGED`
    /// * `options` - 可选的订阅选项（`serde_json::Value`），如过滤、返回字段等
    /// * `callback` - 每次事件触发时调用的回调，参数为 `(args, kwargs)`
    pub fn subscribe<F>(
        &self,
        topic: &str,
        options: Option<Value>,
        callback: F,
    ) -> Result<SubscriptionHandleSync, WaapiError>
    where
        F: Fn(Option<wamp_async::WampArgs>, Option<WampKwArgs>) + Send + Sync + 'static,
    {
        let client = self
            .client
            .as_ref()
            .ok_or(WaapiError::Disconnected)?;
        let (inner, mut async_rx) = self
            .runtime
            .block_on(client.subscribe_inner(topic, options))?;
        let (id_tx, id_rx) = mpsc::channel();
        let runtime = Arc::clone(&self.runtime);
        let bridge_join = thread::spawn(move || {
            let _ = id_tx.send(thread::current().id());
            while let Some((_, args, kwargs)) = runtime.block_on(async_rx.recv()) {
                callback(args, kwargs);
            }
        });
        let bridge_thread_id = id_rx.recv().ok();
        let handle = SubscriptionHandleSync {
            runtime: Arc::clone(&self.runtime),
            inner: Some(inner),
            bridge_join: Some(bridge_join),
            bridge_thread_id,
        };
        Ok(handle)
    }

    /// Check whether the client is still logically connected.
    ///
    /// Note: reflects local state only (whether `disconnect` has been called);
    /// does not probe the underlying WebSocket.
    ///
    /// ---
    ///
    /// 检查客户端是否仍处于逻辑连接状态。
    ///
    /// 注意：仅反映本地状态（是否调用过 `disconnect`），不检测底层 WebSocket 是否存活。
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.as_ref().is_some_and(|c| c.is_connected())
    }

    /// Explicitly disconnect.
    ///
    /// Even without calling this, Drop will auto-disconnect,
    /// but **explicit call is recommended** for graceful shutdown.
    ///
    /// ---
    ///
    /// 显式断开连接。
    ///
    /// 即使不调用此方法，Drop 时也会自动断开，但 **推荐显式调用** 以确保优雅关闭。
    pub fn disconnect(mut self) {
        info!("Disconnecting from WAAPI (sync)");
        if let Some(client) = self.client.take() {
            self.runtime.block_on(client.disconnect());
        }
        info!("Disconnected from WAAPI (sync)");
    }
}

impl Drop for WaapiClientSync {
    /// Disconnects on drop. If inside an async context, offloads cleanup to a dedicated thread
    /// to avoid runtime lifetime issues.
    /// Note: the spawned thread's JoinHandle is not joined; cleanup may not complete
    /// if the process is exiting. Prefer calling `disconnect` explicitly.
    ///
    /// Drop 时断开连接；若在 async 上下文中则将清理转移到独立线程执行，避免 runtime 生命周期导致清理任务丢失。
    /// 注意：该线程的 JoinHandle 未被 join，若进程正在退出，清理可能无法完成；建议显式调用 `disconnect`。
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            if tokio::runtime::Handle::try_current().is_ok() {
                warn!("WaapiClientSync dropped inside async context, offloading cleanup to a dedicated thread");
                let runtime = Arc::clone(&self.runtime);
                let _ = thread::Builder::new()
                    .name("waapi-sync-drop-cleanup".to_string())
                    .spawn(move || {
                        runtime.block_on(client.disconnect());
                    });
            } else {
                self.runtime.block_on(client.disconnect());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_unsubscribed_is_idempotent() {
        let mut is_unsubscribed = false;
        assert!(mark_unsubscribed(&mut is_unsubscribed));
        assert!(!mark_unsubscribed(&mut is_unsubscribed));
    }

    #[tokio::test]
    async fn test_sync_client_drop_inside_async_context_is_safe() {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to create runtime"),
        );
        let async_client = WaapiClient {
            client: None,
            event_loop_handle: None,
            subscription_ids: Arc::new(StdMutex::new(Vec::new())),
            connected: Arc::new(AtomicBool::new(false)),
        };
        let sync_client = WaapiClientSync {
            runtime,
            client: Some(async_client),
        };

        drop(sync_client);
    }
}
