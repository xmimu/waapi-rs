//! WAAPI 客户端实现：异步 [WaapiClient] 与同步 [WaapiClientSync]。
//!
//! 连接生命周期：`connect` 后加入默认 realm，断开时先取消所有订阅、再 leave_realm、再 disconnect。
//! 订阅通过 [SubscriptionHandle] 管理，显式 [SubscriptionHandle::unsubscribe] 或 drop 时自动取消；同步客户端下由 [SubscriptionHandleSync] 管理。

use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;
use tokio::sync::Mutex as TokioMutex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wamp_async::{Client, ClientConfig, SerializerType, WampError, WampId, WampKwArgs};

use crate::args::{value_to_kwargs, value_to_wamp_dict, wamp_result_to_value};

/// 默认 WAAPI WebSocket 地址（Wwise 本机 Authoring API 默认端口）。
const DEFAULT_WAAPI_URL: &str = "ws://localhost:8080/waapi";

/// 连接时使用的默认 WAMP realm 名称，与 Wwise WAAPI 服务端默认一致。
const DEFAULT_REALM: &str = "realm1";

/// 订阅事件 payload：`(PublicationId, args, kwargs)`。
///
/// 从 receiver 或回调中收到；通常使用 `args` / `kwargs` 解析事件内容，PublicationId 可用于去重或日志。
pub type SubscribeEvent = (WampId, Option<wamp_async::WampArgs>, Option<WampKwArgs>);

/// 订阅句柄：用于取消订阅；drop 时会自动在后台执行 unsubscribe。
///
/// 若由 [WaapiClient::subscribe_with_callback] 创建，内部还持有一个 recv 循环 task，unsubscribe 或 drop 时会先 abort 该 task。
pub struct SubscriptionHandle {
    sub_id: WampId,
    client: Arc<TokioMutex<Option<Client<'static>>>>,
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
    recv_task: Option<tokio::task::JoinHandle<()>>,
}

/// WAAPI 异步客户端
///
/// 提供异步接口访问 Wwise Authoring API (WAAPI)；可在多任务间共享使用（内部 Arc + Mutex）。
/// 客户端在 Drop 时会自动清理资源（先取消订阅，再 leave_realm、disconnect）。
pub struct WaapiClient {
    client: Option<Arc<TokioMutex<Option<Client<'static>>>>>,
    event_loop_handle: Option<tokio::task::JoinHandle<Result<(), WampError>>>,
    /// 当前活跃的订阅 ID，disconnect 时统一 unsubscribe
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
}

impl WaapiClient {
    /// 使用默认 URL 连接到 WAAPI
    ///
    /// 默认连接到 `ws://localhost:8080/waapi`，使用默认 realm；
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect_with_url(DEFAULT_WAAPI_URL).await
    }

    /// 使用指定 URL 连接到 WAAPI
    ///
    /// 连接后加入默认 realm；序列化为 JSON，SSL 校验关闭。
    pub async fn connect_with_url(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (mut client, (event_loop, _)) = Client::connect(
            url,
            Some(
                ClientConfig::default()
                    .set_ssl_verify(false)
                    .set_serializers(vec![SerializerType::Json]),
            ),
        )
        .await?;

        let handle = tokio::spawn(event_loop);
        client.join_realm(DEFAULT_REALM).await?;

        let client = Arc::new(TokioMutex::new(Some(client)));
        Ok(Self {
            client: Some(client),
            event_loop_handle: Some(handle),
            subscription_ids: Arc::new(StdMutex::new(Vec::new())),
        })
    }

    /// 调用 WAAPI 方法
    ///
    /// # 参数
    ///
    /// * `uri` - WAAPI 方法的 URI，如 "ak.wwise.core.getInfo"
    /// * `args` - 可选的关键字参数（`T: Serialize + DeserializeOwned`，如 `json!` 或带 `#[derive(Serialize, Deserialize)]` 的结构体）
    /// * `options` - 可选的选项字典（与 `args` 同类型 `T`）
    ///
    /// 返回 `Option<T>`，与入参类型一致。
    pub async fn call<T>(
        &self,
        uri: &str,
        args: Option<T>,
        options: Option<T>,
    ) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: Serialize + DeserializeOwned,
    {
        let args = args
            .map(serde_json::to_value)
            .transpose()?
            .map(value_to_kwargs)
            .transpose()?;
        let options = options
            .map(serde_json::to_value)
            .transpose()?
            .map(value_to_wamp_dict)
            .transpose()?;
        let client = self
            .client
            .as_ref()
            .ok_or("Client already disconnected")?;
        let (_, result) = client
            .lock()
            .await
            .as_ref()
            .ok_or("Client already disconnected")?
            .call(uri, None, args, options)
            .await?;
        let out = result
            .map(wamp_result_to_value)
            .transpose()?
            .map(serde_json::from_value)
            .transpose()?;
        Ok(out)
    }

    /// 无参便捷调用，等价于 `call(uri, None, None)`；返回类型由泛型指定，如 `call_no_args::<serde_json::Value>(uri)`。
    pub async fn call_no_args<T>(&self, uri: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: Serialize + DeserializeOwned,
    {
        self.call::<T>(uri, None, None).await
    }

    /// 订阅主题，返回事件流与句柄。
    ///
    /// 调用方应在单独 task 中消费返回的 receiver，否则会积压；sender 在 client 内部，断开连接时 channel 会关闭。
    /// 取消订阅：调用返回的 [SubscriptionHandle::unsubscribe]，或 drop handle（会在后台自动取消）。
    pub async fn subscribe(
        &self,
        topic: &str,
    ) -> Result<
        (
            SubscriptionHandle,
            tokio::sync::mpsc::UnboundedReceiver<SubscribeEvent>,
        ),
        Box<dyn std::error::Error>,
    > {
        let client = self
            .client
            .as_ref()
            .ok_or("Client already disconnected")?;
        let (sub_id, queue) = client
            .lock()
            .await
            .as_ref()
            .ok_or("Client already disconnected")?
            .subscribe(topic)
            .await?;
        self.subscription_ids.lock().unwrap().push(sub_id);
        let handle = SubscriptionHandle {
            sub_id,
            client: Arc::clone(client),
            subscription_ids: Arc::clone(&self.subscription_ids),
            recv_task: None,
        };
        Ok((handle, queue))
    }

    /// 订阅主题并绑定回调：内部 spawn 一个 task 循环接收事件并调用 `callback(args, kwargs)`。
    ///
    /// 回调在独立 task 中执行，不阻塞事件循环。返回的句柄用于取消订阅；drop 时会 abort 该 task 并自动 unsubscribe。
    pub async fn subscribe_with_callback<F>(
        &self,
        topic: &str,
        callback: F,
    ) -> Result<SubscriptionHandle, Box<dyn std::error::Error>>
    where
        F: Fn(Option<wamp_async::WampArgs>, Option<WampKwArgs>) + Send + Sync + 'static,
    {
        let client = self
            .client
            .as_ref()
            .ok_or("Client already disconnected")?;
        let (sub_id, mut queue) = client
            .lock()
            .await
            .as_ref()
            .ok_or("Client already disconnected")?
            .subscribe(topic)
            .await?;
        self.subscription_ids.lock().unwrap().push(sub_id);
        let callback = Arc::new(callback);
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
        };
        Ok(handle)
    }

    /// 检查客户端是否已连接
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// 显式断开连接
    ///
    /// 注意：即使不调用此方法，Drop 时也会自动断开
    pub async fn disconnect(mut self) {
        self.cleanup().await;
    }

    /// 内部清理：先取消所有已登记订阅，再 leave_realm，再 disconnect，最后 abort 事件循环。
    async fn cleanup(&mut self) {
        let client_arc = self.client.take();
        if let Some(arc) = client_arc {
            let ids: Vec<WampId> = {
                let mut ids = self.subscription_ids.lock().unwrap();
                std::mem::take(ids.as_mut())
            };
            let mut guard = arc.lock().await;
            if let Some(ref mut c) = *guard {
                for sub_id in ids {
                    let _ = c.unsubscribe(sub_id).await;
                }
                if let Err(e) = c.leave_realm().await {
                    eprintln!("Failed to leave realm: {}", e);
                }
            }
            if let Some(c) = guard.take() {
                c.disconnect().await;
            }
        }

        if let Some(handle) = self.event_loop_handle.take() {
            handle.abort();
        }
    }
}

impl SubscriptionHandle {
    /// 取消订阅并停止回调循环（若有）
    pub async fn unsubscribe(mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
        self.subscription_ids
            .lock()
            .unwrap()
            .retain(|&id| id != self.sub_id);
        if let Some(ref c) = *self.client.lock().await {
            c.unsubscribe(self.sub_id).await?;
        }
        Ok(())
    }
}

impl Drop for SubscriptionHandle {
    /// 异步清理：通过 try_current() 在已有 runtime 上 spawn unsubscribe 任务，避免在 drop 中 .await 阻塞；
    /// 若无当前 runtime 则仅清理本地状态，跳过网络取消（连接已失效）。
    fn drop(&mut self) {
        let sub_id = self.sub_id;
        let client = Arc::clone(&self.client);
        let subscription_ids = Arc::clone(&self.subscription_ids);
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
        subscription_ids.lock().unwrap().retain(|&id| id != sub_id);
        if let Ok(rt) = tokio::runtime::Handle::try_current() {
            rt.spawn(async move {
                if let Some(ref c) = *client.lock().await {
                    let _ = c.unsubscribe(sub_id).await;
                }
            });
        }
    }
}

impl Drop for WaapiClient {
    /// 若有当前 runtime 则 spawn 异步清理（unsubscribe → leave_realm → disconnect → abort 事件循环），避免阻塞。
    fn drop(&mut self) {
        if self.client.is_some() || self.event_loop_handle.is_some() {
            if let Ok(rt) = tokio::runtime::Handle::try_current() {
                let client_arc = self.client.take();
                let event_loop = self.event_loop_handle.take();
                let subscription_ids = Arc::clone(&self.subscription_ids);
                rt.spawn(async move {
                    if let Some(arc) = client_arc {
                        let ids: Vec<WampId> = {
                            let mut ids = subscription_ids.lock().unwrap();
                            std::mem::take(ids.as_mut())
                        };
                        let mut guard = arc.lock().await;
                        if let Some(ref mut c) = *guard {
                            for sub_id in ids {
                                let _ = c.unsubscribe(sub_id).await;
                            }
                            let _ = c.leave_realm().await;
                        }
                        if let Some(c) = guard.take() {
                            c.disconnect().await;
                        }
                    }
                    if let Some(h) = event_loop {
                        h.abort();
                    }
                });
            } else {
                if let Some(h) = self.event_loop_handle.take() {
                    h.abort();
                }
            }
        }
    }
}

/// 同步订阅句柄：用于取消通过 [WaapiClientSync::subscribe] 或 [WaapiClientSync::subscribe_with_callback] 创建的订阅。
///
/// 调用 [SubscriptionHandleSync::unsubscribe] 或 drop 时取消订阅并等待桥接线程结束。
/// 注意：不要在回调内部 drop 本句柄，否则可能死锁。
pub struct SubscriptionHandleSync {
    runtime: Arc<tokio::runtime::Runtime>,
    inner: Option<SubscriptionHandle>,
    bridge_join: Option<thread::JoinHandle<()>>,
    bridge_thread_id: Option<thread::ThreadId>,
}

impl SubscriptionHandleSync {
    /// 取消订阅并等待事件桥接线程结束。
    pub fn unsubscribe(mut self) -> Result<(), Box<dyn std::error::Error>> {
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
    /// 不在桥接线程时：block_on 执行 unsubscribe 并 join 桥接线程；在桥接线程内则只做部分清理，避免死锁。
    fn drop(&mut self) {
        let is_bridge_thread = self.bridge_thread_id.as_ref() == Some(&thread::current().id());
        let inner = self.inner.take();
        let bridge_join = self.bridge_join.take();
        let runtime = Arc::clone(&self.runtime);
        if let Some(h) = inner {
            let _ = runtime.block_on(h.unsubscribe());
        }
        if !is_bridge_thread {
            if let Some(jh) = bridge_join {
                let _ = jh.join();
            }
        }
    }
}

/// WAAPI 同步客户端
///
/// 提供同步接口访问 Wwise Authoring API (WAAPI)；内部使用多线程 tokio runtime，通过 `block_on` 封装 [WaapiClient]。
/// 适用于脚本、非 async 代码；若已在 async 环境中，建议直接使用 [WaapiClient]。
/// 客户端在 Drop 时会自动清理资源。
pub struct WaapiClientSync {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Option<WaapiClient>,
}

impl WaapiClientSync {
    /// 使用默认 URL 连接到 WAAPI
    ///
    /// 默认连接到 `ws://localhost:8080/waapi`，使用默认 realm；
    pub fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect_with_url(DEFAULT_WAAPI_URL)
    }

    /// 使用指定 URL 连接到 WAAPI
    ///
    /// 连接后加入默认 realm；内部通过 block_on 调用异步客户端，序列化与 SSL 行为与异步版一致。
    pub fn connect_with_url(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?,
        );
        let client = runtime.block_on(WaapiClient::connect_with_url(url))?;

        Ok(Self {
            runtime,
            client: Some(client),
        })
    }

    /// 调用 WAAPI 方法
    ///
    /// # 参数
    ///
    /// * `uri` - WAAPI 方法的 URI，如 "ak.wwise.core.getInfo"
    /// * `args` - 可选的关键字参数（`T: Serialize + DeserializeOwned`）
    /// * `options` - 可选的选项字典（与 `args` 同类型 `T`）
    ///
    /// 返回 `Option<T>`，与入参类型一致。
    pub fn call<T>(
        &self,
        uri: &str,
        args: Option<T>,
        options: Option<T>,
    ) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: Serialize + DeserializeOwned,
    {
        let client = self.client.as_ref().ok_or("Client already disconnected")?;
        self.runtime
            .block_on(client.call(uri, args, options))
    }

    /// 无参便捷调用，等价于 `call(uri, None, None)`；返回类型由泛型指定，如 `call_no_args::<serde_json::Value>(uri)`。
    pub fn call_no_args<T>(&self, uri: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: Serialize + DeserializeOwned,
    {
        self.call::<T>(uri, None, None)
    }

    /// 订阅主题，返回同步句柄与同步 channel 的 receiver；从 receiver 上 `recv()` 或 `recv_timeout()` 收取事件。
    ///
    /// 取消订阅：调用返回的 [SubscriptionHandleSync::unsubscribe]，或 drop 句柄。不要在回调中 drop 句柄。
    pub fn subscribe(
        &self,
        topic: &str,
    ) -> Result<
        (SubscriptionHandleSync, mpsc::Receiver<SubscribeEvent>),
        Box<dyn std::error::Error>,
    > {
        let client = self
            .client
            .as_ref()
            .ok_or("Client already disconnected")?;
        let (inner, mut async_rx) = self
            .runtime
            .block_on(client.subscribe(topic))?;
        let (sync_tx, sync_rx) = mpsc::channel();
        let (id_tx, id_rx) = mpsc::channel();
        let runtime = Arc::clone(&self.runtime);
        let bridge_join = thread::spawn(move || {
            let _ = id_tx.send(thread::current().id());
            while let Some(ev) = runtime.block_on(async_rx.recv()) {
                if sync_tx.send(ev).is_err() {
                    break;
                }
            }
        });
        let bridge_thread_id = id_rx.recv().ok();
        let handle = SubscriptionHandleSync {
            runtime: Arc::clone(&self.runtime),
            inner: Some(inner),
            bridge_join: Some(bridge_join),
            bridge_thread_id,
        };
        Ok((handle, sync_rx))
    }

    /// 订阅主题并绑定回调：在独立线程中接收事件并同步调用 `callback(args, kwargs)`。
    ///
    /// 取消订阅：调用返回的 [SubscriptionHandleSync::unsubscribe]，或 drop 句柄。不要在 callback 内 drop 句柄。
    pub fn subscribe_with_callback<F>(
        &self,
        topic: &str,
        callback: F,
    ) -> Result<SubscriptionHandleSync, Box<dyn std::error::Error>>
    where
        F: Fn(Option<wamp_async::WampArgs>, Option<WampKwArgs>) + Send + Sync + 'static,
    {
        let client = self
            .client
            .as_ref()
            .ok_or("Client already disconnected")?;
        let (inner, mut async_rx) = self
            .runtime
            .block_on(client.subscribe(topic))?;
        let (id_tx, id_rx) = mpsc::channel();
        let runtime = Arc::clone(&self.runtime);
        let callback = Arc::new(callback);
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

    /// 检查客户端是否已连接
    pub fn is_connected(&self) -> bool {
        self.client.as_ref().map_or(false, |c| c.is_connected())
    }

    /// 显式断开连接
    /// 
    /// 注意：即使不调用此方法，Drop 时也会自动断开
    pub fn disconnect(mut self) {
        if let Some(client) = self.client.take() {
            self.runtime
                .block_on(client.disconnect());
        }
    }
}

impl Drop for WaapiClientSync {
    /// Drop 时在当前线程 block_on 断开连接（先取消订阅、leave_realm、disconnect），并释放内部 runtime。
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            self.runtime
                .block_on(client.disconnect());
        }
    }
}
