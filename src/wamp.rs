//! WAMP Basic Profile 消息层（最小子集）。
//!
//! 只实现 waapi-rs 所需的消息类型：
//! HELLO / WELCOME / GOODBYE / ERROR / CALL / RESULT /
//! SUBSCRIBE / SUBSCRIBED / UNSUBSCRIBE / UNSUBSCRIBED / EVENT

use serde_json::{json, Value};

// WAMP 消息类型编号
const WELCOME: u64 = 2;
const GOODBYE: u64 = 6;
const ERROR: u64 = 8;
const RESULT: u64 = 50;
const SUBSCRIBED: u64 = 33;
const UNSUBSCRIBED: u64 = 35;
const EVENT: u64 = 36;

/// 解析后的入站 WAMP 消息（只含项目所需子集）。
#[derive(Debug)]
pub enum WampMessage {
    Welcome {
        session_id: u64,
    },
    Goodbye,
    Error {
        request_type: u64,
        request_id: u64,
        error: String,
    },
    Result {
        request_id: u64,
        kwargs: Option<Value>,
    },
    Subscribed {
        request_id: u64,
        sub_id: u64,
    },
    Unsubscribed {
        request_id: u64,
    },
    Event {
        sub_id: u64,
        pub_id: u64,
        kwargs: Option<Value>,
    },
}

/// 解析入站 WAMP JSON 文本帧。
///
/// 返回 `None` 表示无需处理的消息（如 Ping/Pong 等）。
pub fn parse(text: &str) -> Option<WampMessage> {
    let arr = serde_json::from_str::<Value>(text).ok()?;
    let arr = arr.as_array()?;
    let msg_type = arr.first()?.as_u64()?;

    match msg_type {
        WELCOME => Some(WampMessage::Welcome {
            session_id: arr.get(1)?.as_u64().unwrap_or(0),
        }),
        GOODBYE => Some(WampMessage::Goodbye),
        ERROR => {
            let request_type = arr.get(1)?.as_u64()?;
            let request_id = arr.get(2)?.as_u64()?;
            let error = arr.get(4)?.as_str()?.to_string();
            Some(WampMessage::Error {
                request_type,
                request_id,
                error,
            })
        }
        RESULT => {
            let request_id = arr.get(1)?.as_u64()?;
            let kwargs = arr.get(4).cloned().filter(|v| v.is_object());
            Some(WampMessage::Result { request_id, kwargs })
        }
        SUBSCRIBED => {
            let request_id = arr.get(1)?.as_u64()?;
            let sub_id = arr.get(2)?.as_u64()?;
            Some(WampMessage::Subscribed { request_id, sub_id })
        }
        UNSUBSCRIBED => {
            let request_id = arr.get(1)?.as_u64()?;
            Some(WampMessage::Unsubscribed { request_id })
        }
        EVENT => {
            let sub_id = arr.get(1)?.as_u64()?;
            let pub_id = arr.get(2)?.as_u64()?;
            let kwargs = arr.get(5).cloned().filter(|v| v.is_object());
            Some(WampMessage::Event {
                sub_id,
                pub_id,
                kwargs,
            })
        }
        _ => None,
    }
}

// ── 出站消息序列化 ──────────────────────────────────────────────

/// `[1, realm, {"roles": {...}}]`
pub fn hello_msg(realm: &str) -> String {
    json!([
        1,
        realm,
        {
            "roles": {
                "caller": {},
                "subscriber": {}
            }
        }
    ])
    .to_string()
}

/// `[48, request_id, options, uri, [], kwargs]`
pub fn call_msg(id: u64, uri: &str, kwargs: Option<&Value>, options: Option<&Value>) -> String {
    let opts = options.cloned().unwrap_or_else(|| json!({}));
    let kw = kwargs.cloned().unwrap_or_else(|| json!({}));
    json!([48, id, opts, uri, [], kw]).to_string()
}

/// `[32, request_id, options, topic]`
pub fn subscribe_msg(id: u64, topic: &str, options: Option<&Value>) -> String {
    let opts = options.cloned().unwrap_or_else(|| json!({}));
    json!([32, id, opts, topic]).to_string()
}

/// `[34, request_id, sub_id]`
pub fn unsubscribe_msg(id: u64, sub_id: u64) -> String {
    json!([34, id, sub_id]).to_string()
}

/// `[6, {}, "wamp.close.normal"]`
pub fn goodbye_msg() -> String {
    json!([6, {}, "wamp.close.normal"]).to_string()
}
