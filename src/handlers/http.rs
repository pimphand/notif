//! HTTP handlers: broadcast trigger and health.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde_json::json;

use crate::auth::JwtSecret;
use crate::db::DbPool;
use crate::error::AppError;
use crate::models::event::BroadcastRequest;
use crate::services::{AuthService, ChannelService, PresenceService};

/// Shared application state for HTTP/WS and dashboard.
#[derive(Clone)]
pub struct AppState {
    pub app_key: String,
    pub app_secret: String,
    pub channel_service: ChannelService,
    pub auth_service: AuthService,
    pub presence_service: PresenceService,
    pub db: DbPool,
    pub jwt_secret: JwtSecret,
}

impl AppState {
    pub fn db(&self) -> &DbPool {
        &self.db
    }
    pub fn jwt_secret(&self) -> &JwtSecret {
        &self.jwt_secret
    }
    pub fn auth_service(&self) -> &AuthService {
        &self.auth_service
    }
    pub fn presence_service(&self) -> &PresenceService {
        &self.presence_service
    }
}

const HEADER_APP_KEY: &str = "x-app-key";

/// POST /api/broadcast — trigger a push notification to a channel.
/// Requires header: x-app-key: <app_key> (legacy config key or API key from dashboard).
pub async fn broadcast(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BroadcastRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let key = headers
        .get(HEADER_APP_KEY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    validate_api_key(state.db(), &state.app_key, key).await?;

    let count = state
        .channel_service
        .broadcast(&body.channel, &body.event, body.data)
        .await?;

    Ok(Json(json!({
        "ok": true,
        "channel": body.channel,
        "event": body.event,
        "subscriber_count": count
    })))
}

/// Validates API key: either legacy app_key or active key from domains table (1 domain = 1 key).
async fn validate_api_key(
    pool: &crate::db::DbPool,
    legacy_app_key: &str,
    key: &str,
) -> Result<(), AppError> {
    if key.is_empty() {
        return Err(AppError::Auth("invalid or missing x-app-key".to_string()));
    }
    if key == legacy_app_key {
        return Ok(());
    }
    let row = crate::db::domain_find_by_key(pool, key).await?;
    match row {
        Some(r) if r.is_active => Ok(()),
        _ => Err(AppError::Auth("invalid or inactive x-app-key".to_string())),
    }
}

/// GET /health — liveness probe.
pub async fn health() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "service": "notif" })),
    )
}
