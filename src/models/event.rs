//! Event and message models for WebSocket and HTTP API.

use serde::{Deserialize, Serialize};

/// Event sent over WebSocket to clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event: String,
    pub channel: String,
    pub data: serde_json::Value,
}

/// Payload for HTTP API to trigger a broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastRequest {
    pub channel: String,
    pub event: String,
    pub data: serde_json::Value,
}

/// WebSocket client message: subscribe / unsubscribe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe { data: SubscribePayload },
    Unsubscribe { data: UnsubscribePayload },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribePayload {
    pub channel: String,
    /// For private/presence: auth signature or token (e.g. HMAC or JWT).
    #[serde(default)]
    pub auth: Option<String>,
    /// For presence: optional channel_data (user info).
    #[serde(default)]
    pub channel_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribePayload {
    pub channel: String,
}
