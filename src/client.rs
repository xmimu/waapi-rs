use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex as TokioMutex;
use wamp_async::{Client, ClientConfig, SerializerType, WampDict, WampError, WampId, WampKwArgs};

const DEFAULT_WAAPI_URL: &str = "ws://localhost:8080/waapi";
const DEFAULT_REALM: &str = "realm1";

/// 订阅事件 payload：`(PublicationId, args, kwargs)`
pub type SubscribeEvent = (WampId, Option<wamp_async::WampArgs>, Option<WampKwArgs>);

/// 订阅句柄：用于取消订阅，并在 drop 时自动取消
pub struct SubscriptionHandle {
    sub_id: WampId,
    client: Arc<TokioMutex<Option<Client<'static>>>>,
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
    recv_task: Option<tokio::task::JoinHandle<()>>,
}

/// WAAPI 异步客户端
///
/// 提供异步接口访问 Wwise Authoring API (WAAPI)。
/// 客户端在 Drop 时会自动清理资源。
pub struct WaapiClient {
    client: Option<Arc<TokioMutex<Option<Client<'static>>>>>,
    event_loop_handle: Option<tokio::task::JoinHandle<Result<(), WampError>>>,
    /// 当前活跃的订阅 ID，disconnect 时统一 unsubscribe
    subscription_ids: Arc<StdMutex<Vec<WampId>>>,
}

impl WaapiClient {
    /// 使用默认 URL 连接到 WAAPI
    /// 
    /// 默认连接到 `ws://localhost:8080/waapi`
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect_with_url(DEFAULT_WAAPI_URL).await
    }

    /// 使用指定 URL 连接到 WAAPI
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
    /// * `args` - 可选的关键字参数
    /// * `options` - 可选的选项字典
    pub async fn call(
        &self,
        uri: &str,
        args: Option<WampKwArgs>,
        options: Option<WampDict>,
    ) -> Result<Option<WampKwArgs>, Box<dyn std::error::Error>> {
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
        Ok(result)
    }

    /// 无参便捷调用，等价于 `call(uri, None, None)`
    pub async fn call_no_args(&self, uri: &str) -> Result<Option<WampKwArgs>, Box<dyn std::error::Error>> {
        self.call(uri, None, None).await
    }

    /// 订阅主题，返回事件流；由调用方在单独 task 中消费 receiver。
    /// 取消订阅请调用返回的 `SubscriptionHandle::unsubscribe()`，或 drop handle（会自动取消）。
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

    /// 订阅主题并绑定回调：内部循环接收事件并调用 `callback(args, kwargs)`。
    /// 返回的句柄用于取消订阅；drop 时会自动取消并停止回调循环。
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

    /// 内部清理方法：先取消所有订阅，再 leave_realm 和 disconnect
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
    fn drop(&mut self) {
        let sub_id = self.sub_id;
        let client = Arc::clone(&self.client);
        let subscription_ids = Arc::clone(&self.subscription_ids);
        if let Some(task) = self.recv_task.take() {
            task.abort();
        }
        subscription_ids.lock().unwrap().retain(|&id| id != sub_id);
        tokio::spawn(async move {
            if let Some(ref c) = *client.lock().await {
                let _ = c.unsubscribe(sub_id).await;
            }
        });
    }
}

impl Drop for WaapiClient {
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

/// WAAPI 同步客户端
/// 
/// 提供同步接口访问 Wwise Authoring API (WAAPI)。
/// 内部管理 tokio 运行时，用户无需关心异步细节。
/// 客户端在 Drop 时会自动清理资源。
pub struct WaapiClientSync {
    runtime: tokio::runtime::Runtime,
    client: Option<WaapiClient>,
}

impl WaapiClientSync {
    /// 使用默认 URL 连接到 WAAPI
    /// 
    /// 默认连接到 `ws://localhost:8080/waapi`
    pub fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect_with_url(DEFAULT_WAAPI_URL)
    }

    /// 使用指定 URL 连接到 WAAPI
    pub fn connect_with_url(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        
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
    /// * `args` - 可选的关键字参数
    /// * `options` - 可选的选项字典
    pub fn call(
        &self,
        uri: &str,
        args: Option<WampKwArgs>,
        options: Option<WampDict>,
    ) -> Result<Option<WampKwArgs>, Box<dyn std::error::Error>> {
        let client = self.client.as_ref().ok_or("Client already disconnected")?;
        self.runtime.block_on(client.call(uri, args, options))
    }

    /// 无参便捷调用，等价于 `call(uri, None, None)`
    pub fn call_no_args(&self, uri: &str) -> Result<Option<WampKwArgs>, Box<dyn std::error::Error>> {
        self.call(uri, None, None)
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
            self.runtime.block_on(client.disconnect());
        }
    }
}

impl Drop for WaapiClientSync {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            self.runtime.block_on(client.disconnect());
        }
    }
}
