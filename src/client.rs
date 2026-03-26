//! WAAPI client implementation: async [WaapiClient] and sync [WaapiClientSync].
//!
//! Connection lifecycle: after `connect`, joins the default realm; on disconnect,
//! all subscriptions are cancelled first, then GOODBYE, then close.
//! Subscriptions are managed via [SubscriptionHandle]; explicit [SubscriptionHandle::unsubscribe]
//! or drop will cancel automatically. For sync clients, [SubscriptionHandleSync] is used.
//!
//! ---
//!
//! WAAPI 客户端实现：异步 [WaapiClient] 与同步 [WaapiClientSync]。
//!
//! 连接生命周期：`connect` 后加入默认 realm，断开时先取消所有订阅、再发 GOODBYE、再关闭连接。
//! 订阅通过 [SubscriptionHandle] 管理，显式 [SubscriptionHandle::unsubscribe] 或 drop 时自动取消；
//! 同步客户端下由 [SubscriptionHandleSync] 管理。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::oneshot;
use tokio::sync::{mpsc as async_mpsc, Mutex as TokioMutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use log::{debug, info, warn};

use crate::wamp;

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
    /// WAMP protocol error (e.g. server returned ERROR message).
    ///
    /// WAMP 协议层错误（如服务端返回 ERROR 消息）。
    #[error("WAMP error: {0}")]
    Wamp(String),
    /// WebSocket error.
    ///
    /// WebSocket 层错误。
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),
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

// ── 内部类型别名 ────────────────────────────────────────────────

type CallResult = Result<Option<Value>, WaapiError>;
type SubResult = Result<u64, WaapiError>;
type UnsubResult = Result<(), WaapiError>;

/// 订阅事件 payload：`(pub_id, kwargs)`
pub type EventPayload = (u64, Option<Value>);

// ── 内部连接状态 ─────────────────────────────────────────────────

type WsSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Message,
>;

struct WampConn {
    ws_tx: TokioMutex<WsSink>,
    pending_calls: StdMutex<HashMap<u64, oneshot::Sender<CallResult>>>,
    pending_subs: StdMutex<HashMap<u64, oneshot::Sender<SubResult>>>,
    pending_unsubs: StdMutex<HashMap<u64, oneshot::Sender<UnsubResult>>>,
    event_senders: StdMutex<HashMap<u64, async_mpsc::UnboundedSender<EventPayload>>>,
    next_id: AtomicU64,
}

impl WampConn {
    fn new(sink: WsSink) -> Self {
        Self {
            ws_tx: TokioMutex::new(sink),
            pending_calls: StdMutex::new(HashMap::new()),
            pending_subs: StdMutex::new(HashMap::new()),
            pending_unsubs: StdMutex::new(HashMap::new()),
            event_senders: StdMutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    async fn send(&self, text: String) -> Result<(), WaapiError> {
        self.ws_tx
            .lock()
            .await
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| WaapiError::WebSocket(Box::new(e)))
    }
}

// ── 事件循环 ──────────────────────────────────────────────────────

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

async fn run_event_loop(
    conn: Arc<WampConn>,
    mut ws_rx: futures_util::stream::SplitStream<WsStream>,
    connected: Arc<AtomicBool>,
) {
    while let Some(msg) = ws_rx.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Some(wamp_msg) = wamp::parse(&text) {
                    dispatch(&conn, wamp_msg);
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
    connected.store(false, Ordering::Release);
    // 连接断开时，唤醒所有等待中的 pending futures 并报错
    drain_pending(&conn);
}

fn dispatch(conn: &WampConn, msg: wamp::WampMessage) {
    match msg {
        wamp::WampMessage::Result { request_id, kwargs } => {
            if let Some(tx) = conn
                .pending_calls
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&request_id)
            {
                let _ = tx.send(Ok(kwargs));
            }
        }
        wamp::WampMessage::Error {
            request_type,
            request_id,
            error,
        } => {
            let err_str = error.clone();
            // CALL error (type 48)
            if request_type == 48 {
                if let Some(tx) = conn
                    .pending_calls
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&request_id)
                {
                    let _ = tx.send(Err(WaapiError::Wamp(err_str)));
                    return;
                }
            }
            // SUBSCRIBE error (type 32)
            if request_type == 32 {
                if let Some(tx) = conn
                    .pending_subs
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&request_id)
                {
                    let _ = tx.send(Err(WaapiError::Wamp(error)));
                    return;
                }
            }
            // UNSUBSCRIBE error (type 34)
            if request_type == 34 {
                if let Some(tx) = conn
                    .pending_unsubs
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&request_id)
                {
                    let _ = tx.send(Err(WaapiError::Wamp(error)));
                }
            }
        }
        wamp::WampMessage::Subscribed {
            request_id,
            sub_id,
        } => {
            if let Some(tx) = conn
                .pending_subs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&request_id)
            {
                let _ = tx.send(Ok(sub_id));
            }
        }
        wamp::WampMessage::Unsubscribed { request_id } => {
            if let Some(tx) = conn
                .pending_unsubs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&request_id)
            {
                let _ = tx.send(Ok(()));
            }
        }
        wamp::WampMessage::Event {
            sub_id,
            pub_id,
            kwargs,
        } => {
            let senders = conn
                .event_senders
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(tx) = senders.get(&sub_id) {
                let _ = tx.send((pub_id, kwargs));
            }
        }
        wamp::WampMessage::Goodbye | wamp::WampMessage::Welcome { .. } => {}
    }
}

/// 连接断开时，向所有等待中的 pending futures 发送 Disconnected 错误。
fn drain_pending(conn: &WampConn) {
    let calls: Vec<_> = conn
        .pending_calls
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .drain()
        .collect();
    for (_, tx) in calls {
        let _ = tx.send(Err(WaapiError::Disconnected));
    }
    let subs: Vec<_> = conn
        .pending_subs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .drain()
        .collect();
    for (_, tx) in subs {
        let _ = tx.send(Err(WaapiError::Disconnected));
    }
    let unsubs: Vec<_> = conn
        .pending_unsubs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .drain()
        .collect();
    for (_, tx) in unsubs {
        let _ = tx.send(Err(WaapiError::Disconnected));
    }
}

// ── 连接握手辅助 ─────────────────────────────────────────────────

/// 从 WebSocket 流读取第一条消息，期望是 WELCOME，否则返回错误。
async fn read_welcome(
    ws_rx: &mut futures_util::stream::SplitStream<WsStream>,
) -> Result<u64, WaapiError> {
    loop {
        match ws_rx.next().await {
            Some(Ok(Message::Text(text))) => {
                if let Some(wamp::WampMessage::Welcome { session_id }) = wamp::parse(&text) {
                    return Ok(session_id);
                }
                return Err(WaapiError::Wamp(format!("expected WELCOME, got: {text}")));
            }
            Some(Ok(_)) => continue, // 忽略非文本帧（如 Ping）
            Some(Err(e)) => return Err(WaapiError::WebSocket(Box::new(e))),
            None => return Err(WaapiError::Disconnected),
        }
    }
}

// ── 公共 API ──────────────────────────────────────────────────────

/// Subscription handle: used to cancel a subscription; automatically unsubscribes
/// in the background on drop.
///
/// ---
///
/// 订阅句柄：用于取消订阅；drop 时会自动在后台执行 unsubscribe。
pub struct SubscriptionHandle {
    sub_id: u64,
    conn: Arc<WampConn>,
    subscription_ids: Arc<StdMutex<Vec<u64>>>,
    recv_task: Option<tokio::task::JoinHandle<()>>,
    is_unsubscribed: bool,
}

fn mark_unsubscribed(flag: &mut bool) -> bool {
    if *flag {
        false
    } else {
        *flag = true;
        true
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
        // Drop the event sender so receivers (e.g. sync bridge thread) see channel closed.
        self.conn
            .event_senders
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.sub_id);
        if !mark_unsubscribed(&mut self.is_unsubscribed) {
            return Ok(());
        }
        do_network_unsubscribe(&self.conn, self.sub_id).await
    }
}

async fn do_network_unsubscribe(conn: &WampConn, sub_id: u64) -> Result<(), WaapiError> {
    let id = conn.next_id();
    let (tx, rx) = oneshot::channel();
    conn.pending_unsubs
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(id, tx);
    conn.send(wamp::unsubscribe_msg(id, sub_id)).await?;
    rx.await.unwrap_or(Err(WaapiError::Disconnected))
}

impl Drop for SubscriptionHandle {
    fn drop(&mut self) {
        let sub_id = self.sub_id;
        let conn = Arc::clone(&self.conn);
        let subscription_ids = Arc::clone(&self.subscription_ids);
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
        subscription_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|&id| id != sub_id);
        // Drop the event sender so receivers see channel closed.
        conn.event_senders
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&sub_id);
        if !mark_unsubscribed(&mut self.is_unsubscribed) {
            return;
        }
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            debug!("SubscriptionHandle dropped, spawning unsubscribe for sub_id={sub_id}");
            rt.spawn(async move {
                let _ = do_network_unsubscribe(&conn, sub_id).await;
            });
        } else {
            warn!("SubscriptionHandle dropped without runtime, skipping network unsubscribe for sub_id={sub_id}");
        }
    }
}

/// Async WAAPI client.
///
/// Provides async access to the Wwise Authoring API (WAAPI);
/// can be shared across tasks (internal Arc).
///
/// **It is recommended to call [`disconnect`](WaapiClient::disconnect) explicitly**
/// for graceful shutdown.
///
/// ---
///
/// WAAPI 异步客户端。
///
/// **建议显式调用 [`disconnect`](WaapiClient::disconnect)** 以确保优雅关闭。
pub struct WaapiClient {
    conn: Option<Arc<WampConn>>,
    event_loop_handle: Option<tokio::task::JoinHandle<()>>,
    subscription_ids: Arc<StdMutex<Vec<u64>>>,
    connected: Arc<AtomicBool>,
}

impl WaapiClient {
    /// Connect to WAAPI using the default URL.
    ///
    /// Connects to `ws://localhost:8080/waapi` with the default realm.
    ///
    /// ---
    ///
    /// 使用默认 URL 连接到 WAAPI。
    pub async fn connect() -> Result<Self, WaapiError> {
        Self::connect_with_url(DEFAULT_WAAPI_URL).await
    }

    /// Connect to WAAPI at the specified URL.
    ///
    /// ---
    ///
    /// 使用指定 URL 连接到 WAAPI。
    pub async fn connect_with_url(url: &str) -> Result<Self, WaapiError> {
        info!("Connecting to WAAPI at {url}");
        let (ws_stream, _) = connect_async(url).await.map_err(|e| WaapiError::WebSocket(Box::new(e)))?;
        let (ws_tx, mut ws_rx) = ws_stream.split();

        let conn = Arc::new(WampConn::new(ws_tx));

        // HELLO handshake
        conn.send(wamp::hello_msg(DEFAULT_REALM)).await?;
        let _session_id = read_welcome(&mut ws_rx).await?;

        let connected = Arc::new(AtomicBool::new(true));
        let connected_flag = Arc::clone(&connected);
        let conn_for_loop = Arc::clone(&conn);
        let handle = tokio::spawn(async move {
            run_event_loop(conn_for_loop, ws_rx, connected_flag).await;
        });

        info!("Connected to WAAPI at {url}");
        Ok(Self {
            conn: Some(conn),
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
    /// * `options` - Optional options dict (`serde_json::Value`)
    ///
    /// Returns `Option<Value>`: WAAPI response as JSON; `None` when no result.
    ///
    /// ---
    ///
    /// 调用 WAAPI 方法。
    ///
    /// 返回 `Option<Value>`：WAAPI 响应的 JSON 值；无结果时为 `None`。
    pub async fn call(
        &self,
        uri: &str,
        args: Option<Value>,
        options: Option<Value>,
    ) -> Result<Option<Value>, WaapiError> {
        let conn = self.conn.as_ref().ok_or(WaapiError::Disconnected)?;
        let id = conn.next_id();
        let (tx, rx) = oneshot::channel();
        conn.pending_calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        debug!("Calling WAAPI: {uri} (id={id})");
        conn.send(wamp::call_msg(id, uri, args.as_ref(), options.as_ref()))
            .await?;
        rx.await.unwrap_or(Err(WaapiError::Disconnected))
    }

    /// Internal subscribe: returns handle and receiver. Used by [WaapiClientSync].
    pub(crate) async fn subscribe_inner(
        &self,
        topic: &str,
        options: Option<Value>,
    ) -> Result<
        (
            SubscriptionHandle,
            async_mpsc::UnboundedReceiver<EventPayload>,
        ),
        WaapiError,
    > {
        let conn = self.conn.as_ref().ok_or(WaapiError::Disconnected)?;
        let id = conn.next_id();
        let (tx, rx) = oneshot::channel();
        conn.pending_subs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        conn.send(wamp::subscribe_msg(id, topic, options.as_ref()))
            .await?;
        let sub_id = rx.await.unwrap_or(Err(WaapiError::Disconnected))?;
        debug!("Subscribed to {topic} (sub_id={sub_id})");

        let (event_tx, event_rx) = async_mpsc::unbounded_channel();
        conn.event_senders
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(sub_id, event_tx);
        self.subscription_ids
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(sub_id);

        let handle = SubscriptionHandle {
            sub_id,
            conn: Arc::clone(conn),
            subscription_ids: Arc::clone(&self.subscription_ids),
            recv_task: None,
            is_unsubscribed: false,
        };
        Ok((handle, event_rx))
    }

    /// Subscribe to a topic with a callback.
    ///
    /// The callback runs in a dedicated task with signature `callback(kwargs)`.
    /// The returned handle is used to cancel; on drop it auto-unsubscribes.
    ///
    /// # Parameters
    ///
    /// * `topic` - WAMP topic URI
    /// * `options` - Optional subscription options (`serde_json::Value`)
    /// * `callback` - Callback invoked on each event with `kwargs` (`Option<Value>`)
    ///
    /// ---
    ///
    /// 订阅主题并绑定回调（参数为 `Option<Value>`）。
    pub async fn subscribe<F>(
        &self,
        topic: &str,
        options: Option<Value>,
        callback: F,
    ) -> Result<SubscriptionHandle, WaapiError>
    where
        F: Fn(Option<Value>) + Send + Sync + 'static,
    {
        let (mut handle, mut event_rx) = self.subscribe_inner(topic, options).await?;
        let recv_task = tokio::spawn(async move {
            while let Some((_pub_id, kwargs)) = event_rx.recv().await {
                callback(kwargs);
            }
        });
        handle.recv_task = Some(recv_task);
        Ok(handle)
    }

    /// Check whether the client is still connected.
    ///
    /// ---
    ///
    /// 检查客户端是否仍处于连接状态。
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.conn.is_some() && self.connected.load(Ordering::Acquire)
    }

    /// Explicitly disconnect.
    ///
    /// **Explicit call is recommended** for graceful shutdown.
    ///
    /// ---
    ///
    /// 显式断开连接。**推荐显式调用**以确保优雅关闭。
    pub async fn disconnect(mut self) {
        info!("Disconnecting from WAAPI");
        self.cleanup().await;
        info!("Disconnected from WAAPI");
    }

    async fn cleanup(&mut self) {
        self.connected.store(false, Ordering::Release);
        if let Some(conn) = self.conn.take() {
            // Unsubscribe all active subscriptions
            let ids: Vec<u64> = {
                let mut guard = self.subscription_ids.lock().unwrap_or_else(|e| e.into_inner());
                std::mem::take(&mut *guard)
            };
            for sub_id in ids {
                let id = conn.next_id();
                let (tx, rx) = oneshot::channel();
                conn.pending_unsubs
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(id, tx);
                if conn.send(wamp::unsubscribe_msg(id, sub_id)).await.is_ok() {
                    let _ = rx.await;
                }
            }
            // GOODBYE
            let _ = conn.send(wamp::goodbye_msg()).await;
            // Close WebSocket
            let _ = conn.ws_tx.lock().await.close().await;
        }
        if let Some(handle) = self.event_loop_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for WaapiClient {
    fn drop(&mut self) {
        if self.conn.is_some() || self.event_loop_handle.is_some() {
            let conn = self.conn.take();
            let event_loop = self.event_loop_handle.take();
            let subscription_ids = Arc::clone(&self.subscription_ids);
            let connected = Arc::clone(&self.connected);
            connected.store(false, Ordering::Release);
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                debug!("WaapiClient dropped, spawning async cleanup");
                rt.spawn(async move {
                    if let Some(conn) = conn {
                        let ids: Vec<u64> = {
                            let mut guard =
                                subscription_ids.lock().unwrap_or_else(|e| e.into_inner());
                            std::mem::take(&mut *guard)
                        };
                        for sub_id in ids {
                            let id = conn.next_id();
                            let (tx, rx) = oneshot::channel::<UnsubResult>();
                            conn.pending_unsubs
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .insert(id, tx);
                            if conn.send(wamp::unsubscribe_msg(id, sub_id)).await.is_ok() {
                                let _ = rx.await;
                            }
                        }
                        let _ = conn.send(wamp::goodbye_msg()).await;
                        let _ = conn.ws_tx.lock().await.close().await;
                    }
                    if let Some(h) = event_loop {
                        h.abort();
                    }
                });
            } else {
                warn!("WaapiClient dropped without runtime, skipping graceful cleanup");
                if let Some(h) = event_loop {
                    h.abort();
                }
            }
        }
    }
}

// ── Sync client ───────────────────────────────────────────────────

/// Sync subscription handle: cancels subscriptions created by [WaapiClientSync::subscribe].
///
/// Calls [SubscriptionHandleSync::unsubscribe] or drop to cancel and wait for the bridge thread.
/// **Do not drop this handle inside a callback — it may deadlock.**
///
/// ---
///
/// 同步订阅句柄。**注意：不要在回调内部 drop 本句柄，否则可能死锁。**
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
/// tokio runtime and wraps [WaapiClient] via `block_on`.
///
/// **Explicit [`disconnect`](WaapiClientSync::disconnect) is recommended** for graceful shutdown.
///
/// ---
///
/// WAAPI 同步客户端。**推荐显式调用 [`disconnect`](WaapiClientSync::disconnect)**。
pub struct WaapiClientSync {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Option<WaapiClient>,
}

impl WaapiClientSync {
    /// Connect to WAAPI using the default URL.
    ///
    /// ---
    ///
    /// 使用默认 URL 连接到 WAAPI。
    pub fn connect() -> Result<Self, WaapiError> {
        Self::connect_with_url(DEFAULT_WAAPI_URL)
    }

    /// Connect to WAAPI at the specified URL.
    ///
    /// ---
    ///
    /// 使用指定 URL 连接到 WAAPI。
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
    /// ---
    ///
    /// 调用 WAAPI 方法。
    pub fn call(
        &self,
        uri: &str,
        args: Option<Value>,
        options: Option<Value>,
    ) -> Result<Option<Value>, WaapiError> {
        let client = self.client.as_ref().ok_or(WaapiError::Disconnected)?;
        self.runtime.block_on(client.call(uri, args, options))
    }

    /// Subscribe to a topic with a callback.
    ///
    /// To unsubscribe: call [SubscriptionHandleSync::unsubscribe] or drop the handle.
    /// Do not drop the handle inside the callback.
    ///
    /// ---
    ///
    /// 订阅主题并绑定回调。取消订阅：调用返回的 [SubscriptionHandleSync::unsubscribe]，或 drop 句柄。
    /// 不要在 callback 内 drop 句柄。
    pub fn subscribe<F>(
        &self,
        topic: &str,
        options: Option<Value>,
        callback: F,
    ) -> Result<SubscriptionHandleSync, WaapiError>
    where
        F: Fn(Option<Value>) + Send + Sync + 'static,
    {
        let client = self.client.as_ref().ok_or(WaapiError::Disconnected)?;
        let (inner, mut async_rx) = self
            .runtime
            .block_on(client.subscribe_inner(topic, options))?;
        let (id_tx, id_rx) = mpsc::channel();
        let runtime = Arc::clone(&self.runtime);
        let bridge_join = thread::spawn(move || {
            let _ = id_tx.send(thread::current().id());
            while let Some((_pub_id, kwargs)) = runtime.block_on(async_rx.recv()) {
                callback(kwargs);
            }
        });
        let bridge_thread_id = id_rx.recv().ok();
        Ok(SubscriptionHandleSync {
            runtime: Arc::clone(&self.runtime),
            inner: Some(inner),
            bridge_join: Some(bridge_join),
            bridge_thread_id,
        })
    }

    /// Check whether the client is still logically connected.
    ///
    /// ---
    ///
    /// 检查客户端是否仍处于逻辑连接状态。
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.as_ref().is_some_and(|c| c.is_connected())
    }

    /// Explicitly disconnect.
    ///
    /// ---
    ///
    /// 显式断开连接。
    pub fn disconnect(mut self) {
        info!("Disconnecting from WAAPI (sync)");
        if let Some(client) = self.client.take() {
            self.runtime.block_on(client.disconnect());
        }
        info!("Disconnected from WAAPI (sync)");
    }
}

impl Drop for WaapiClientSync {
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

// ── Tests ─────────────────────────────────────────────────────────

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
            conn: None,
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
