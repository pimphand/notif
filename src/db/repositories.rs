//! Repositories: users, domains (1 domain = 1 key), channels, ws_connections.

use crate::error::{AppError, AppResult};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::DbPool;

// ---- User ----

#[derive(Debug, FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

pub async fn user_create(
    pool: &DbPool,
    name: &str,
    email: &str,
    password_hash: &str,
) -> AppResult<UserRow> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        INSERT INTO users (name, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id, name, email, password_hash, created_at
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn user_find_by_email(pool: &DbPool, email: &str) -> AppResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, email, password_hash, created_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn user_get_by_id(pool: &DbPool, id: Uuid) -> AppResult<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, email, password_hash, created_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ---- Domains (1 domain = 1 API key) ----

#[derive(Debug, FromRow)]
pub struct DomainRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub domain_name: String,
    pub key: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

pub async fn domain_create(
    pool: &DbPool,
    user_id: Uuid,
    domain_name: &str,
    key: &str,
) -> AppResult<DomainRow> {
    let domain_name = domain_name.trim().to_lowercase();
    let row = sqlx::query_as::<_, DomainRow>(
        r#"
        INSERT INTO domains (user_id, domain_name, key)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, domain_name) DO NOTHING
        RETURNING id, user_id, domain_name, key, created_at, is_active
        "#,
    )
    .bind(user_id)
    .bind(&domain_name)
    .bind(key)
    .fetch_optional(pool)
    .await?;
    row.ok_or_else(|| AppError::Validation("Domain already exists for this user".to_string()))
}

pub async fn domains_list_by_user(pool: &DbPool, user_id: Uuid) -> AppResult<Vec<DomainRow>> {
    let rows = sqlx::query_as::<_, DomainRow>(
        "SELECT id, user_id, domain_name, key, created_at, is_active FROM domains WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn domain_find_by_key(pool: &DbPool, key: &str) -> AppResult<Option<DomainRow>> {
    let row = sqlx::query_as::<_, DomainRow>(
        "SELECT id, user_id, domain_name, key, created_at, is_active FROM domains WHERE key = $1 AND is_active = true",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn domain_set_active(
    pool: &DbPool,
    id: Uuid,
    user_id: Uuid,
    is_active: bool,
) -> AppResult<()> {
    let r = sqlx::query("UPDATE domains SET is_active = $1 WHERE id = $2 AND user_id = $3")
        .bind(is_active)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::Auth("Domain not found".to_string()));
    }
    Ok(())
}

pub async fn domain_delete(pool: &DbPool, id: Uuid, user_id: Uuid) -> AppResult<()> {
    let r = sqlx::query("DELETE FROM domains WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::Auth("Domain not found".to_string()));
    }
    Ok(())
}

// ---- Channels ----

#[derive(Debug, FromRow)]
pub struct ChannelRow {
    pub id: Uuid,
    pub name: String,
    pub domain_id: Uuid,
    pub created_at: DateTime<Utc>,
}

pub async fn channel_ensure(pool: &DbPool, name: &str, domain_id: Uuid) -> AppResult<ChannelRow> {
    sqlx::query(
        r#"
        INSERT INTO channels (name, domain_id)
        VALUES ($1, $2)
        ON CONFLICT (name, domain_id) DO NOTHING
        "#,
    )
    .bind(name)
    .bind(domain_id)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, ChannelRow>(
        "SELECT id, name, domain_id, created_at FROM channels WHERE name = $1 AND domain_id = $2",
    )
    .bind(name)
    .bind(domain_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn channels_list_by_user(pool: &DbPool, user_id: Uuid) -> AppResult<Vec<ChannelRow>> {
    let rows = sqlx::query_as::<_, ChannelRow>(
        r#"
        SELECT c.id, c.name, c.domain_id, c.created_at
        FROM channels c
        JOIN domains d ON d.id = c.domain_id
        WHERE d.user_id = $1
        ORDER BY c.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ---- WS connections ----

#[derive(Debug, FromRow)]
pub struct WsConnectionRow {
    pub id: Uuid,
    pub channel_id: Option<Uuid>,
    pub channel_name: String,
    pub domain_id: Uuid,
    pub socket_id: String,
    pub connected_user: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub status: String,
}

pub async fn ws_connection_insert(
    pool: &DbPool,
    channel_id: Option<Uuid>,
    channel_name: &str,
    domain_id: Uuid,
    socket_id: &str,
    connected_user: Option<&str>,
) -> AppResult<Uuid> {
    let row: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO ws_connections (channel_id, channel_name, domain_id, socket_id, connected_user, status)
        VALUES ($1, $2, $3, $4, $5, 'connected')
        RETURNING id
        "#,
    )
    .bind(channel_id)
    .bind(channel_name)
    .bind(domain_id)
    .bind(socket_id)
    .bind(connected_user)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn ws_connection_mark_disconnected(pool: &DbPool, socket_id: &str) -> AppResult<()> {
    sqlx::query(
        "UPDATE ws_connections SET status = 'disconnected', disconnected_at = NOW() WHERE socket_id = $1 AND status = 'connected'",
    )
    .bind(socket_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn ws_connection_mark_disconnected_by_channel(
    pool: &DbPool,
    socket_id: &str,
    channel_name: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE ws_connections SET status = 'disconnected', disconnected_at = NOW() WHERE socket_id = $1 AND channel_name = $2 AND status = 'connected'",
    )
    .bind(socket_id)
    .bind(channel_name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn ws_connections_active_by_user(
    pool: &DbPool,
    user_id: Uuid,
) -> AppResult<Vec<WsConnectionRow>> {
    let rows = sqlx::query_as::<_, WsConnectionRow>(
        r#"
        SELECT w.id, w.channel_id, w.channel_name, w.domain_id, w.socket_id, w.connected_user, w.connected_at, w.disconnected_at, w.status
        FROM ws_connections w
        JOIN domains d ON d.id = w.domain_id
        WHERE d.user_id = $1 AND w.status = 'connected'
        ORDER BY w.connected_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn ws_status_aggregate_by_user(
    pool: &DbPool,
    user_id: Uuid,
) -> AppResult<Vec<(String, i64)>> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT w.channel_name, COUNT(*)::bigint
        FROM ws_connections w
        JOIN domains d ON d.id = w.domain_id
        WHERE d.user_id = $1 AND w.status = 'connected'
        GROUP BY w.channel_name
        ORDER BY COUNT(*) DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
