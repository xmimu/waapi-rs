use std::sync::Arc;
use tokio::sync::Mutex;
use wamp_async::{Client, ClientConfig, SerializerType, WampDict, WampError, WampKwArgs};

const DEFAULT_WAAPI_URL: &str = "ws://localhost:8080/waapi";
const DEFAULT_REALM: &str = "realm1";

/// WAAPI 异步客户端
/// 
/// 提供异步接口访问 Wwise Authoring API (WAAPI)。
/// 客户端在 Drop 时会自动清理资源。
pub struct WaapiClient {
    client: Option<Client<'static>>,
    event_loop_handle: Option<tokio::task::JoinHandle<Result<(), WampError>>>,
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

        Ok(Self {
            client: Some(client),
            event_loop_handle: Some(handle),
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
        let (_, result) = client.call(uri, None, args, options).await?;
        Ok(result)
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

    /// 内部清理方法
    async fn cleanup(&mut self) {
        if let Some(mut client) = self.client.take() {
            if let Err(e) = client.leave_realm().await {
                eprintln!("Failed to leave realm: {}", e);
            }
            client.disconnect().await;
        }

        if let Some(handle) = self.event_loop_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for WaapiClient {
    fn drop(&mut self) {
        if self.client.is_some() {
            // 尝试在当前运行时中清理
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let mut client = self.client.take();
                let mut event_loop = self.event_loop_handle.take();
                
                handle.spawn(async move {
                    if let Some(mut c) = client.take() {
                        let _ = c.leave_realm().await;
                        c.disconnect().await;
                    }
                    if let Some(h) = event_loop.take() {
                        h.abort();
                    }
                });
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
    client: Arc<Mutex<Option<WaapiClient>>>,
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
            client: Arc::new(Mutex::new(Some(client))),
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
        self.runtime.block_on(async {
            let client_guard = self.client.lock().await;
            let client = client_guard
                .as_ref()
                .ok_or("Client already disconnected")?;
            client.call(uri, args, options).await
        })
    }

    /// 检查客户端是否已连接
    pub fn is_connected(&self) -> bool {
        self.runtime.block_on(async {
            let client_guard = self.client.lock().await;
            client_guard.as_ref().map_or(false, |c| c.is_connected())
        })
    }

    /// 显式断开连接
    /// 
    /// 注意：即使不调用此方法，Drop 时也会自动断开
    pub fn disconnect(self) {
        self.runtime.block_on(async {
            let mut client_guard = self.client.lock().await;
            if let Some(client) = client_guard.take() {
                client.disconnect().await;
            }
        });
    }
}

impl Drop for WaapiClientSync {
    fn drop(&mut self) {
        self.runtime.block_on(async {
            let mut client_guard = self.client.lock().await;
            if let Some(client) = client_guard.take() {
                client.disconnect().await;
            }
        });
    }
}
