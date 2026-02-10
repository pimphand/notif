//! Redis connection and pub/sub for channel messaging and presence storage.

use crate::error::AppError;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info};

use futures::StreamExt;

const CHANNEL_PREFIX: &str = "notif:channel:";
const PRESENCE_SET_PREFIX: &str = "notif:presence:";
const PRESENCE_HASH_PREFIX: &str = "notif:presence_hash:";

fn channel_key(channel: &str) -> String {
    format!("{}{}", CHANNEL_PREFIX, channel)
}

/// Redis-backed repository: pub/sub for events, sets/hash for presence.
#[derive(Clone)]
pub struct RedisRepository {
    client: Arc<redis::Client>,
    #[allow(dead_code)]
    redis_url: String,
}

impl RedisRepository {
    /// Create repository from Redis URL.
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            client: Arc::new(client),
            redis_url: redis_url.to_string(),
        })
    }

    /// Get a multiplexed connection for commands (publish, set, etc.).
    pub async fn connection(&self) -> Result<redis::aio::MultiplexedConnection, AppError> {
        let conn = self.client.get_multiplexed_async_connection().await?;
        Ok(conn)
    }

    /// Publish a message to a channel (Redis PUBLISH).
    pub async fn publish(&self, channel: &str, message: &str) -> Result<u64, AppError> {
        let mut conn = self.connection().await?;
        let key = channel_key(channel);
        let count: u64 = conn.publish(&key, message).await?;
        debug!(channel = %channel, count, "published");
        Ok(count)
    }

    /// Subscribe to a channel; returns a receiver that gets all messages published to that channel.
    /// Uses one Redis connection per channel, forwarding messages to a broadcast channel.
    pub async fn subscribe_to_channel(
        &self,
        channel: &str,
    ) -> Result<broadcast::Receiver<String>, AppError> {
        let conn = self.client.get_async_connection().await?;
        let mut pubsub = conn.into_pubsub();
        let key = channel_key(channel);
        pubsub.subscribe(&key).await?;
        info!(channel = %channel, "subscribed to redis channel");

        let (tx, rx) = broadcast::channel(64);
        let mut stream = pubsub.into_on_message();

        tokio::spawn(async move {
            while let Some(msg) = stream.next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    let _ = tx.send(payload);
                }
            }
        });

        Ok(rx)
    }

    // --- Presence: store socket_id -> member in Redis SET and HASH for presence-* channels ---

    /// Add a presence member to a channel.
    pub async fn presence_add(
        &self,
        channel: &str,
        socket_id: &str,
        member_data: &str,
    ) -> Result<(), AppError> {
        let mut conn = self.connection().await?;
        let set_key = format!("{}{}", PRESENCE_SET_PREFIX, channel);
        let hash_key = format!("{}{}", PRESENCE_HASH_PREFIX, channel);
        conn.sadd::<_, _, ()>(&set_key, socket_id).await?;
        conn.hset::<_, _, _, ()>(&hash_key, socket_id, member_data).await?;
        Ok(())
    }

    /// Remove a presence member.
    pub async fn presence_remove(&self, channel: &str, socket_id: &str) -> Result<(), AppError> {
        let mut conn = self.connection().await?;
        let set_key = format!("{}{}", PRESENCE_SET_PREFIX, channel);
        let hash_key = format!("{}{}", PRESENCE_HASH_PREFIX, channel);
        conn.srem::<_, _, ()>(&set_key, socket_id).await?;
        conn.hdel::<_, _, ()>(&hash_key, socket_id).await?;
        Ok(())
    }

    /// Get all presence members for a channel (socket_id -> member_data).
    pub async fn presence_members(&self, channel: &str) -> Result<Vec<(String, String)>, AppError> {
        let mut conn = self.connection().await?;
        let hash_key = format!("{}{}", PRESENCE_HASH_PREFIX, channel);
        let map: std::collections::HashMap<String, String> = conn.hgetall(&hash_key).await?;
        Ok(map.into_iter().collect())
    }
}
