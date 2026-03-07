//! WAAPI Rust client: call and subscribe (see [WaapiClient](client::WaapiClient)).

mod client;

pub use client::{
    SubscribeEvent, SubscriptionHandle, WaapiClient, WaapiClientSync,
};
pub use wamp_async::{WampArgs, WampDict, WampId, WampKwArgs};
