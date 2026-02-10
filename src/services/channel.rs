//! Channel subscription and broadcast: one Redis subscription per channel, fan-out to local receivers.

use crate::error::AppResult;
use crate::models::event::WsEvent;
use crate::repositories::RedisRepository;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

/// Manages channel subscriptions: ensures one Redis subscriber per channel and distributes messages.
#[derive(Clone)]
pub struct ChannelService {
    repo: Arc<RedisRepository>,
    /// channel_name -> (broadcast Sender, subscriber count). When count drops to 0 we could unsubscribe from Redis.
    subscribers: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl ChannelService {
    pub fn new(repo: Arc<RedisRepository>) -> Self {
        Self {
            repo,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a broadcast receiver for the channel. Multiple callers get the same channel's receiver.
    pub async fn subscribe(&self, channel: &str) -> AppResult<broadcast::Receiver<String>> {
        let rx = {
            let mut subs = self.subscribers.write().await;
            if let Some(tx) = subs.get(channel) {
                tx.subscribe()
            } else {
                let redis_rx = self.repo.subscribe_to_channel(channel).await?;
                let (tx, _rx) = broadcast::channel(64);
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let mut redis_rx = redis_rx;
                    while let Ok(msg) = redis_rx.recv().await {
                        let _ = tx_clone.send(msg);
                    }
                });
                subs.insert(channel.to_string(), tx);
                subs.get(channel).unwrap().subscribe()
            }
        };
        Ok(rx)
    }

    /// Broadcast an event to a channel (publish to Redis; all subscribers receive it).
    pub async fn broadcast(&self, channel: &str, event: &str, data: serde_json::Value) -> AppResult<u64> {
        let ws_event = WsEvent {
            event: event.to_string(),
            channel: channel.to_string(),
            data,
        };
        let payload = serde_json::to_string(&ws_event)?;
        let count = self.repo.publish(channel, &payload).await?;
        info!(channel = %channel, event = %event, count, "broadcast");
        Ok(count)
    }

    /// Remove channel from local cache when no more subscribers (optional cleanup).
    pub async fn unsubscribe(&self, channel: &str) {
        let mut subs = self.subscribers.write().await;
        subs.remove(channel);
        debug!(channel = %channel, "unsubscribed from channel");
    }
}
