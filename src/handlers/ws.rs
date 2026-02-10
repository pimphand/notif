//! WebSocket handler: subscribe, unsubscribe, message forwarding, API key and domain validation.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::HeaderMap,
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::domain_find_by_key;
use crate::error::AppError;
use crate::handlers::http::AppState;
use crate::models::channel::ChannelType;
use crate::models::event::ClientMessage;
use crate::models::presence::generate_socket_id;

const HEADER_APP_KEY: &str = "x-app-key";
const HEADER_ORIGIN: &str = "origin";

/// Upgrade HTTP to WebSocket. Validates API key and origin (if API key has domains) before upgrade.
pub async fn ws_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let api_key = params
        .get("api_key")
        .cloned()
        .or_else(|| headers.get(HEADER_APP_KEY).and_then(|v| v.to_str().ok()).map(String::from));
    let origin = headers
        .get(HEADER_ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_lowercase());

    let domain_id = if let Some(key) = &api_key {
        let row = domain_find_by_key(state.db(), key).await?;
        let row = row.ok_or_else(|| AppError::Auth("Invalid or inactive API key".to_string()))?;
        let origin_host = origin
            .as_ref()
            .and_then(|o| parse_origin_host(o))
            .ok_or_else(|| AppError::Auth("Origin required and must match domain".to_string()))?;
        if !domain_matches(&row.domain_name, &origin_host) {
            return Err(AppError::Auth("Origin does not match domain for this key".to_string()));
        }
        Some(row.id)
    } else {
        None
    };

    Ok(ws.on_upgrade(move |socket| handle_socket(state, socket, domain_id)))
}

/// Parse host from Origin header (e.g. "https://app.example.com" -> "app.example.com").
pub(crate) fn parse_origin_host(origin: &str) -> Option<String> {
    let u = origin.strip_prefix("https://").or_else(|| origin.strip_prefix("http://"))?;
    let host = u.split('/').next()?.to_lowercase();
    Some(host)
}

/// Check if origin host matches allowed domain (exact or *.example.com suffix).
pub(crate) fn domain_matches(allowed: &str, origin_host: &str) -> bool {
    let allowed = allowed.trim().to_lowercase();
    if allowed.starts_with('*') {
        origin_host.ends_with(allowed.trim_start_matches('*').trim_start_matches('.'))
    } else {
        allowed == origin_host
    }
}

async fn handle_socket(state: AppState, socket: WebSocket, domain_id: Option<Uuid>) {
    let socket_id = generate_socket_id();
    info!(socket_id = %socket_id, "ws connected");

    let (mut sender, mut receiver) = socket.split();
    let mut subscribed_channels: HashSet<String> = HashSet::new();

    let conn_msg = json!({
        "event": "connection_established",
        "data": { "socket_id": socket_id }
    });
    if sender.send(Message::Text(conn_msg.to_string())).await.is_err() {
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match client_msg {
                        ClientMessage::Subscribe { data } => {
                            let channel = data.channel.clone();
                            let channel_type = ChannelType::from_name(&channel);

                            let auth_ok = state
                                .auth_service()
                                .verify_channel_auth(
                                    &channel,
                                    &socket_id,
                                    data.auth.as_deref(),
                                    data.channel_data.as_ref().map(|v: &serde_json::Value| v.to_string()).as_deref(),
                                )
                                .is_ok();

                            if channel_type.is_private() && !auth_ok {
                                let err = json!({
                                    "event": "pusher:error",
                                    "data": { "message": "Auth failed for channel", "code": 4009 }
                                });
                                let _ = tx.send(err.to_string());
                                continue;
                            }

                            match state.channel_service.subscribe(&channel).await {
                                Ok(mut channel_rx) => {
                                    subscribed_channels.insert(channel.clone());

                                    if let Some(did) = domain_id {
                                        if let Ok(ch_row) = crate::db::channel_ensure(state.db(), &channel, did).await {
                                            let user_str = data
                                                .channel_data
                                                .as_ref()
                                                .and_then(|v: &serde_json::Value| v.get("user_id"))
                                                .and_then(|v: &serde_json::Value| v.as_str());
                                            let _ = crate::db::ws_connection_insert(
                                                state.db(),
                                                Some(ch_row.id),
                                                &channel,
                                                did,
                                                &socket_id,
                                                user_str,
                                            )
                                            .await;
                                        }
                                    }

                                    if channel_type == ChannelType::Presence {
                                        let user_id = data
                                            .channel_data
                                            .as_ref()
                                            .and_then(|v: &serde_json::Value| v.get("user_id"))
                                            .and_then(|v: &serde_json::Value| v.as_str())
                                            .unwrap_or("anonymous");
                                        let user_info = data.channel_data.clone();
                                        if state
                                            .presence_service()
                                            .add_member(&channel, &socket_id, user_id, user_info)
                                            .await
                                            .is_ok()
                                        {
                                            let members: Vec<crate::models::PresenceUser> = state
                                                .presence_service()
                                                .list_members(&channel)
                                                .await
                                                .unwrap_or_default();
                                            let sub_ok = json!({
                                                "event": "pusher_internal:subscription_succeeded",
                                                "channel": channel,
                                                "data": json!({
                                                    "presence": {
                                                        "ids": members.iter().map(|u| u.user_id.clone()).collect::<Vec<_>>(),
                                                        "hash": {},
                                                        "count": members.len()
                                                    }
                                                })
                                            });
                                            let _ = tx.send(sub_ok.to_string());
                                        }
                                    } else {
                                        let sub_ok = json!({
                                            "event": "pusher_internal:subscription_succeeded",
                                            "channel": channel
                                        });
                                        let _ = tx.send(sub_ok.to_string());
                                    }

                                    let tx_fwd = tx.clone();
                                    tokio::spawn(async move {
                                        while let Ok(payload) = channel_rx.recv().await {
                                            let _ = tx_fwd.send(payload);
                                        }
                                    });
                                }
                                Err(e) => {
                                    warn!(channel = %channel, error = %e, "subscribe failed");
                                    let err = json!({
                                        "event": "pusher:error",
                                        "data": { "message": format!("Subscribe failed: {}", e), "code": 4009 }
                                    });
                                    let _ = tx.send(err.to_string());
                                }
                            }
                        }
                        ClientMessage::Unsubscribe { data } => {
                            let channel = data.channel.clone();
                            let channel_type = ChannelType::from_name(&channel);
                            if channel_type == ChannelType::Presence {
                                let _ = state.presence_service().remove_member(&channel, &socket_id).await;
                            }
                            if domain_id.is_some() {
                                let _ = crate::db::ws_connection_mark_disconnected_by_channel(
                                    state.db(),
                                    &socket_id,
                                    &channel,
                                )
                                .await;
                            }
                            subscribed_channels.remove(&channel);
                            debug!(socket_id = %socket_id, channel = %channel, "unsubscribed");
                        }
                        ClientMessage::Ping => {
                            let pong = json!({ "event": "pusher:pong", "data": {} });
                            let _ = tx.send(pong.to_string());
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    for channel in &subscribed_channels {
        if ChannelType::from_name(channel) == ChannelType::Presence {
            let _ = state.presence_service().remove_member(channel, &socket_id).await;
        }
    }
    if domain_id.is_some() {
        let _ = crate::db::ws_connection_mark_disconnected(state.db(), &socket_id).await;
    }

    send_task.abort();
    info!(socket_id = %socket_id, "ws disconnected");
}

#[cfg(test)]
mod tests {
    use super::{domain_matches, parse_origin_host};

    #[test]
    fn parse_origin_host_http_https() {
        assert_eq!(parse_origin_host("https://app.example.com"), Some("app.example.com".to_string()));
        assert_eq!(parse_origin_host("http://localhost:3000"), Some("localhost:3000".to_string()));
        assert_eq!(parse_origin_host("https://sub.domain.com/path"), Some("sub.domain.com".to_string()));
    }

    #[test]
    fn parse_origin_host_invalid() {
        assert_eq!(parse_origin_host("not-a-url"), None);
        assert_eq!(parse_origin_host(""), None);
    }

    #[test]
    fn domain_matches_exact() {
        assert!(domain_matches("app.example.com", "app.example.com"));
        assert!(domain_matches("localhost", "localhost"));
        assert!(!domain_matches("other.com", "app.example.com"));
    }

    #[test]
    fn domain_matches_wildcard() {
        assert!(domain_matches("*.example.com", "app.example.com"));
        assert!(domain_matches("*.example.com", "example.com"));
        assert!(!domain_matches("*.example.com", "other.com"));
    }
}
