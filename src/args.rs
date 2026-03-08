//! call 入参/返回值转换：通过 [serde::Serialize] / [serde::de::DeserializeOwned] 与 WAMP 类型互转。
//! 仅内部使用，供 [crate::client] 将 T 转为 [WampKwArgs]/[WampDict]，并将结果转回 T。

use serde_json::Value;
use wamp_async::{try_into_kwargs, try_into_wamp_dict, WampDict, WampError, WampKwArgs};

/// 将已序列化得到的 `Value` 转为 WAMP kwargs，供 client 内部使用。
pub(crate) fn value_to_kwargs(value: Value) -> Result<WampKwArgs, WampError> {
    try_into_kwargs(value)
}

/// 将已序列化得到的 `Value` 转为 WAMP 字典，供 client 内部使用。
pub(crate) fn value_to_wamp_dict(value: Value) -> Result<WampDict, WampError> {
    try_into_wamp_dict(value)
}

/// 将 WAMP 调用结果转为 `Value`，以便再通过 `serde_json::from_value::<T>` 得到 `T`。
pub(crate) fn wamp_result_to_value(kw: WampKwArgs) -> Result<Value, serde_json::Error> {
    serde_json::to_value(&kw)
}
