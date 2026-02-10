//! Auth HTTP handlers: register, login.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::auth::AuthAppService;
use crate::db::{user_create, user_find_by_email};
use crate::error::AppError;
use crate::handlers::http::AppState;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub email: String,
}

/// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    body.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    AuthAppService::validate_email(&body.email)?;

    if user_find_by_email(state.db(), &body.email).await?.is_some() {
        return Err(AppError::Validation("Email already registered".to_string()));
    }

    let password_hash = AuthAppService::hash_password(&body.password)?;
    let user = user_create(state.db(), &body.name, &body.email, &password_hash).await?;
    let token = state.jwt_secret().issue(user.id)?;

    Ok(Json(RegisterResponse {
        id: user.id.to_string(),
        name: user.name,
        email: user.email,
        token,
    }))
}

/// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = user_find_by_email(state.db(), &body.email)
        .await?
        .ok_or_else(|| AppError::Auth("Invalid email or password".to_string()))?;

    if !AuthAppService::verify_password(&body.password, &user.password_hash)? {
        return Err(AppError::Auth("Invalid email or password".to_string()));
    }

    let token = state.jwt_secret().issue(user.id)?;

    Ok(Json(LoginResponse {
        token,
        user: UserInfo {
            id: user.id.to_string(),
            name: user.name,
            email: user.email,
        },
    }))
}
