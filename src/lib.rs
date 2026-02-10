//! Real-time push notification system (Pusher-like) built with Rust.
//!
//! Provides WebSocket-based real-time channels with Redis pub/sub,
//! support for public, private, and presence channels.

pub mod auth;
pub mod config;
pub mod dashboard;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod services;

pub use config::Config;
pub use error::AppError;
pub use handlers::http::AppState;
pub use services::channel::ChannelService;
pub use services::presence::PresenceService;

use axum::routing::{get, post};
use handlers::http;

/// Build the API router (ws, broadcast, health, auth, dashboard). Used by main and by integration tests.
pub fn create_app(state: AppState) -> axum::Router {
    let auth_routes = axum::Router::new()
        .route("/register", post(auth::register))
        .route("/login", post(auth::login));

    let dashboard_routes = axum::Router::new()
        .route("/user", get(dashboard::get_user))
        .route(
            "/domains",
            get(dashboard::list_domains).post(dashboard::create_domain),
        )
        .route(
            "/domains/:id",
            axum::routing::patch(dashboard::set_domain_active).delete(dashboard::delete_domain),
        )
        .route("/channels", get(dashboard::list_channels))
        .route("/ws-status", get(dashboard::get_ws_status));

    axum::Router::new()
        .route("/ws", get(handlers::ws_handler))
        .route("/api/broadcast", post(handlers::broadcast))
        .route("/health", get(http::health))
        .nest("/auth", auth_routes)
        .nest("/dashboard", dashboard_routes)
        .with_state(state)
}
