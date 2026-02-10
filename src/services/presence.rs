//! Presence channel: track who is online and broadcast join/leave.

use crate::error::{AppError, AppResult};
use crate::models::presence::{PresenceMember, PresenceUser};
use crate::repositories::RedisRepository;
use serde_json;
use std::sync::Arc;
use tracing::{info, instrument};

/// Presence channel operations: add/remove members, list members.
#[derive(Clone)]
pub struct PresenceService {
    repo: Arc<RedisRepository>,
}

impl PresenceService {
    pub fn new(repo: Arc<RedisRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self))]
    pub async fn add_member(
        &self,
        channel: &str,
        socket_id: &str,
        user_id: &str,
        user_info: Option<serde_json::Value>,
    ) -> AppResult<()> {
        let member = PresenceMember {
            user_id: user_id.to_string(),
            user_info: user_info.clone(),
            socket_id: socket_id.to_string(),
        };
        let data = serde_json::to_string(&member).map_err(AppError::from)?;
        self.repo.presence_add(channel, socket_id, &data).await?;
        info!(channel = %channel, socket_id = %socket_id, user_id = %user_id, "presence member added");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn remove_member(&self, channel: &str, socket_id: &str) -> AppResult<()> {
        self.repo.presence_remove(channel, socket_id).await?;
        info!(channel = %channel, socket_id = %socket_id, "presence member removed");
        Ok(())
    }

    /// List all members currently on the channel.
    pub async fn list_members(&self, channel: &str) -> AppResult<Vec<PresenceUser>> {
        let raw = self.repo.presence_members(channel).await?;
        let mut users = Vec::new();
        for (_socket_id, data) in raw {
            if let Ok(member) = serde_json::from_str::<PresenceMember>(&data) {
                users.push(PresenceUser {
                    user_id: member.user_id,
                    user_info: member.user_info,
                });
            }
        }
        Ok(users)
    }
}
