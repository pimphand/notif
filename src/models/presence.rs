//! Presence channel: track online users.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User info for presence channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUser {
    pub user_id: String,
    pub user_info: Option<serde_json::Value>,
}

impl PresenceUser {
    pub fn new(user_id: impl Into<String>, user_info: Option<serde_json::Value>) -> Self {
        Self {
            user_id: user_id.into(),
            user_info,
        }
    }
}

/// Stored presence member (internal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceMember {
    pub user_id: String,
    pub user_info: Option<serde_json::Value>,
    pub socket_id: String,
}

impl PresenceMember {
    pub fn socket_id(&self) -> &str {
        &self.socket_id
    }
}

/// Generate a unique socket/connection id.
pub fn generate_socket_id() -> String {
    format!("{}.{}", std::process::id(), Uuid::new_v4().as_simple())
}
