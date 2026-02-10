//! Entry point: load config, wire dependencies, and run the server.

use axum::routing::get_service;
use notif::config::Config;
use notif::db;
use notif::repositories::RedisRepository;
use notif::services::{AuthService, ChannelService, PresenceService};
use notif::{create_app, AppState};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env().map_err(|e| anyhow::anyhow!("config: {}", e))?;

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.log_level))?;
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_pool = db::create_pool(&config.database_url).await?;
    let repo = Arc::new(RedisRepository::new(&config.redis_url)?);
    let channel_service = ChannelService::new(repo.clone());
    let auth_service = AuthService::new(config.app_secret.clone(), config.app_key.clone());
    let presence_service = PresenceService::new(repo);
    let jwt_secret = notif::auth::JwtSecret::new(config.jwt_secret.clone());

    let state = AppState {
        app_key: config.app_key.clone(),
        app_secret: config.app_secret.clone(),
        channel_service,
        auth_service,
        presence_service,
        db: db_pool,
        jwt_secret,
    };

    let app = create_app(state)
        // Root (/) and /docs.html: serve docs.html
        .route_service(
            "/",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/docs.html",
            )),
        )
        .route_service(
            "/docs.html",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/docs.html",
            )),
        )
        // Dashboard page
        .route_service(
            "/index.html",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/index.html",
            )),
        )
        // Auth pages
        .route_service(
            "/login.html",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/login.html",
            )),
        )
        .route_service(
            "/register.html",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/register.html",
            )),
        )
        // Chat demo and JS client
        .route_service(
            "/chat-demo.html",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/chat-demo.html",
            )),
        )
        .route_service(
            "/notifmoo.js",
            get_service(tower_http::services::ServeFile::new(
                "dashboard_static/notifmoo.js",
            )),
        );

    tracing::info!(addr = %config.server_addr, "listening");
    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
