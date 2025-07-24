use serde_json::{Map, Value};
use wamp_async::{Client, ClientConfig, SerializerType, WampError};

pub struct WaapiClient {
    client: Client<'static>,
    _event_loop_handle: tokio::task::JoinHandle<Result<(), WampError>>,
}

impl WaapiClient {
    pub async fn connect_with_url(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect_internal(url).await
    }

    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        Self::connect_internal("ws://localhost:8080/waapi").await
    }

    async fn connect_internal(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
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
        client.join_realm("realm1").await?;

        Ok(Self {
            client,
            _event_loop_handle: handle,
        })
    }

    pub async fn call(
        &mut self,
        uri: &str,
        args: Option<Vec<Value>>,
        kwargs: Option<Map<String, Value>>,
    ) -> Result<(Option<Vec<Value>>, Option<Map<String, Value>>), Box<dyn std::error::Error>> {
        let result = self.client.call(uri, args, kwargs).await?;
        Ok(result)
    }

    pub async fn disconnect(mut self) {
        if let Err(e) = self.client.leave_realm().await {
            eprintln!("Leave realm error: {}", e);
        }
        self.client.disconnect().await
    }
}
