//! Auth middleware: JWT extractor for dashboard; app key for broadcast.

use axum::{
    extract::Request,
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use tracing::debug;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::http::AppState;

const HEADER_APP_KEY: &str = "x-app-key";
const BEARER_PREFIX: &str = "Bearer ";

/// Extractor: authenticated user ID from JWT (Bearer token).
#[derive(Clone, Copy, Debug)]
pub struct AuthUser(pub Uuid);

#[axum::async_trait]
impl axum::extract::FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix(BEARER_PREFIX));
        let token = auth.ok_or_else(|| AppError::Auth("Missing or invalid Authorization header".to_string()))?;
        let user_id = state.jwt_secret().validate(token)?;
        Ok(AuthUser(user_id))
    }
}

/// Middleware: require `x-app-key` header for legacy broadcast API.
pub async fn auth_middleware(
    request: Request,
    next: Next,
    app_key: String,
) -> Response {
    let headers = request.headers();
    let key = headers
        .get(HEADER_APP_KEY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if key != app_key {
        debug!("rejected request: invalid or missing x-app-key");
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::UNAUTHORIZED)
            .body(axum::body::Body::from(r#"{"error":"invalid or missing x-app-key"}"#))
            .unwrap();
    }

    next.run(request).await
}

#[derive(Clone)]
pub struct AuthLayer {
    pub app_key: String,
}

impl AuthLayer {
    pub fn new(app_key: String) -> Self {
        Self { app_key }
    }
}
