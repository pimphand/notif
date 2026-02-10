//! Dashboard HTTP handlers. 1 domain = 1 API key.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    channels_list_by_user, domain_create, domain_delete, domain_set_active, domains_list_by_user,
    user_get_by_id, ws_connections_active_by_user, ws_status_aggregate_by_user,
};
use crate::error::AppError;
use crate::handlers::http::AppState;
use crate::middleware::auth::AuthUser;

// ---- User ----

#[derive(Debug, Serialize)]
pub struct DashboardUserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

/// GET /dashboard/user
pub async fn get_user(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<DashboardUserResponse>, AppError> {
    let user = user_get_by_id(state.db(), user_id)
        .await?
        .ok_or_else(|| AppError::Auth("User not found".to_string()))?;
    Ok(Json(DashboardUserResponse {
        id: user.id.to_string(),
        name: user.name,
        email: user.email,
        created_at: user.created_at.to_rfc3339(),
    }))
}

// ---- Domains (1 domain = 1 API key) ----

#[derive(Debug, Serialize)]
pub struct DomainResponse {
    pub id: String,
    pub domain_name: String,
    pub key: String,
    pub is_active: bool,
    pub created_at: String,
}

/// GET /dashboard/domains
pub async fn list_domains(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<Vec<DomainResponse>>, AppError> {
    let rows = domains_list_by_user(state.db(), user_id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| DomainResponse {
                id: r.id.to_string(),
                domain_name: r.domain_name,
                key: r.key,
                is_active: r.is_active,
                created_at: r.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct CreateDomainRequest {
    pub domain_name: String,
}

/// POST /dashboard/domains — create domain + generate API key (1 domain = 1 key)
pub async fn create_domain(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(body): Json<CreateDomainRequest>,
) -> Result<Json<DomainResponse>, AppError> {
    let domain_name = body.domain_name.trim().to_lowercase();
    if domain_name.is_empty() {
        return Err(AppError::Validation("domain_name required".to_string()));
    }
    let key = format!("nk_{}", Uuid::new_v4().simple());
    let row = domain_create(state.db(), user_id, &domain_name, &key).await?;
    Ok(Json(DomainResponse {
        id: row.id.to_string(),
        domain_name: row.domain_name,
        key: row.key,
        is_active: row.is_active,
        created_at: row.created_at.to_rfc3339(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct SetDomainActiveRequest {
    pub is_active: bool,
}

/// PATCH /dashboard/domains/:id
pub async fn set_domain_active(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(body): Json<SetDomainActiveRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    domain_set_active(state.db(), id, user_id, body.is_active).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// DELETE /dashboard/domains/:id
pub async fn delete_domain(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    domain_delete(state.db(), id, user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---- Channels ----

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub name: String,
    pub domain_id: String,
    pub created_at: String,
}

/// GET /dashboard/channels
pub async fn list_channels(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<Vec<ChannelResponse>>, AppError> {
    let rows = channels_list_by_user(state.db(), user_id).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ChannelResponse {
                id: r.id.to_string(),
                name: r.name,
                domain_id: r.domain_id.to_string(),
                created_at: r.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

// ---- WS status ----

/// GET /dashboard/ws-status
pub async fn get_ws_status(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let by_channel = ws_status_aggregate_by_user(state.db(), user_id).await?;
    let connections = ws_connections_active_by_user(state.db(), user_id).await?;
    Ok(Json(serde_json::json!({
        "by_channel": by_channel.into_iter().map(|(name, count)| serde_json::json!({ "channel_name": name, "connection_count": count })).collect::<Vec<_>>(),
        "connections": connections.into_iter().map(|c| serde_json::json!({
            "id": c.id.to_string(),
            "channel_name": c.channel_name,
            "socket_id": c.socket_id,
            "connected_user": c.connected_user,
            "connected_at": c.connected_at.to_rfc3339(),
            "status": c.status
        })).collect::<Vec<_>>()
    })))
}
