//! JWT issue and validation.

use crate::error::{AppError, AppResult};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // user_id
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct JwtSecret {
    secret: String,
}

impl JwtSecret {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }

    pub fn issue(&self, user_id: Uuid) -> AppResult<String> {
        let now = Utc::now();
        let exp = (now + Duration::days(7)).timestamp();
        let claims = Claims {
            sub: user_id.to_string(),
            exp,
            iat: now.timestamp(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AppError::Jwt(e.to_string()))?;
        Ok(token)
    }

    pub fn validate(&self, token: &str) -> AppResult<Uuid> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map_err(|e| AppError::Jwt(e.to_string()))?;
        let id = Uuid::parse_str(&data.claims.sub).map_err(|e| AppError::Jwt(e.to_string()))?;
        Ok(id)
    }
}
