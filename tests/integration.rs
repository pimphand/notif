//! Integration tests: health, auth (register/login), broadcast (legacy app_key).
//!
//! Run with `cargo test`. For integration tests that need DB/Redis, set:
//! - `TEST_DATABASE_URL` (Postgres, run migrations first)
//! - `TEST_REDIS_URL` (defaults to redis://127.0.0.1:6379 if unset)
//! - `TEST_APP_KEY` / `TEST_APP_SECRET` (optional, for broadcast test)

use axum::body::Body;
use axum::http::{Request, StatusCode};
use notif::repositories::RedisRepository;
use notif::services::{AuthService, ChannelService, PresenceService};
use notif::{create_app, auth::JwtSecret, db, AppState};
use std::sync::Arc;
use tower::util::ServiceExt;

async fn test_state(
    database_url: &str,
    redis_url: &str,
    app_key: &str,
    app_secret: &str,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let db_pool = db::create_pool(database_url).await?;
    let repo = Arc::new(RedisRepository::new(redis_url)?);
    let channel_service = ChannelService::new(repo.clone());
    let auth_service = AuthService::new(app_secret.to_string(), app_key.to_string());
    let presence_service = PresenceService::new(repo);
    let jwt_secret = JwtSecret::new("test-jwt-secret-min-32-chars!!".to_string());
    Ok(AppState {
        app_key: app_key.to_string(),
        app_secret: app_secret.to_string(),
        channel_service,
        auth_service,
        presence_service,
        db: db_pool,
        jwt_secret,
    })
}

#[tokio::test]
async fn health_returns_ok() {
    let database_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("Skip integration test: set TEST_DATABASE_URL and TEST_REDIS_URL");
            return;
        }
    };
    let redis_url = std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let app_key = std::env::var("TEST_APP_KEY").unwrap_or_else(|_| "test-key".to_string());
    let app_secret = std::env::var("TEST_APP_SECRET").unwrap_or_else(|_| "test-secret".to_string());

    let state = match test_state(&database_url, &redis_url, &app_key, &app_secret).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skip integration test: {}", e);
            return;
        }
    };

    let app = create_app(state);
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.get("status").and_then(|v| v.as_str()), Some("ok"));
}

#[tokio::test]
async fn register_and_login() {
    let database_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) => u,
        Err(_) => return,
    };
    let redis_url = std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let app_key = std::env::var("TEST_APP_KEY").unwrap_or_else(|_| "test-key".to_string());
    let app_secret = std::env::var("TEST_APP_SECRET").unwrap_or_else(|_| "test-secret".to_string());

    let state = match test_state(&database_url, &redis_url, &app_key, &app_secret).await {
        Ok(s) => s,
        Err(_) => return,
    };
    let app = create_app(state);

    let email = format!("test-{}@example.com", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let register_body = serde_json::json!({ "email": email, "password": "password123" });
    let req = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(register_body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "register should succeed");

    let login_body = serde_json::json!({ "email": email, "password": "password123" });
    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(login_body.to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "login should succeed");
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("token").and_then(|v| v.as_str()).is_some(), "response should contain token");
}

#[tokio::test]
async fn broadcast_requires_app_key() {
    let database_url = match std::env::var("TEST_DATABASE_URL") {
        Ok(u) => u,
        Err(_) => return,
    };
    let redis_url = std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let app_key = std::env::var("TEST_APP_KEY").unwrap_or_else(|_| "test-key".to_string());
    let app_secret = std::env::var("TEST_APP_SECRET").unwrap_or_else(|_| "test-secret".to_string());

    let state = match test_state(&database_url, &redis_url, &app_key, &app_secret).await {
        Ok(s) => s,
        Err(_) => return,
    };
    let app = create_app(state);

    let body = serde_json::json!({ "channel": "test-channel", "event": "test", "data": {} });
    let req = Request::builder()
        .method("POST")
        .uri("/api/broadcast")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "broadcast without key should be 401");

    let req = Request::builder()
        .method("POST")
        .uri("/api/broadcast")
        .header("content-type", "application/json")
        .header("x-app-key", &app_key)
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK, "broadcast with valid app_key should succeed");
}