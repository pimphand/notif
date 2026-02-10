//! Auth application service: register, login (password hash/verify).

// user_create, user_find_by_email used by auth handlers
use crate::error::{AppError, AppResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use validator::ValidateEmail;

pub struct AuthAppService;

impl AuthAppService {
    pub fn hash_password(password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("hash: {}", e)))?
            .to_string();
        Ok(hash)
    }

    pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
        let parsed =
            PasswordHash::new(hash).map_err(|e| AppError::Internal(anyhow::anyhow!("parse hash: {}", e)))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }

    pub fn validate_email(email: &str) -> AppResult<()> {
        if !email.validate_email() {
            return Err(AppError::Validation("Invalid email".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_password() {
        let hash = AuthAppService::hash_password("mypassword").unwrap();
        assert!(AuthAppService::verify_password("mypassword", &hash).unwrap());
        assert!(!AuthAppService::verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn validate_email_accepts_valid() {
        assert!(AuthAppService::validate_email("user@example.com").is_ok());
        assert!(AuthAppService::validate_email("a@b.co").is_ok());
    }

    #[test]
    fn validate_email_rejects_invalid() {
        assert!(AuthAppService::validate_email("invalid").is_err());
        assert!(AuthAppService::validate_email("@nodomain").is_err());
        assert!(AuthAppService::validate_email("").is_err());
    }
}
