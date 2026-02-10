//! Authentication for private and presence channels (Pusher-compatible HMAC).

use crate::error::{AppError, AppResult};
use crate::models::channel::ChannelType;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::debug;

type HmacSha256 = Hmac<Sha256>;

/// Validates auth signature for private/presence channels.
/// Pusher-style: HMAC-SHA256(app_secret, socket_id:channel_name[:channel_data]).
#[derive(Clone)]
pub struct AuthService {
    app_secret: String,
    #[allow(dead_code)]
    app_key: String,
}

impl AuthService {
    pub fn new(app_secret: String, app_key: String) -> Self {
        Self {
            app_secret,
            app_key,
        }
    }

    /// Verify that the client is allowed to subscribe to the channel.
    /// For private: auth = HMAC(socket_id:channel_name).
    /// For presence: auth = HMAC(socket_id:channel_name:channel_data), and channel_data must be valid JSON with user_id.
    pub fn verify_channel_auth(
        &self,
        channel: &str,
        socket_id: &str,
        auth: Option<&str>,
        channel_data: Option<&str>,
    ) -> AppResult<()> {
        let channel_type = ChannelType::from_name(channel);
        if !channel_type.is_private() {
            return Ok(());
        }

        let auth = auth.ok_or_else(|| {
            AppError::Auth("missing auth for private/presence channel".to_string())
        })?;

        let mut mac = HmacSha256::new_from_slice(self.app_secret.as_bytes())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("HMAC init: {}", e)))?;

        let sign_payload = if channel_type == ChannelType::Presence {
            format!("{}:{}:{}", socket_id, channel, channel_data.unwrap_or(""))
        } else {
            format!("{}:{}", socket_id, channel)
        };

        mac.update(sign_payload.as_bytes());
        let result = mac.finalize();
        let expected = hex::encode(result.into_bytes());

        if auth != expected {
            debug!(channel = %channel, "auth signature mismatch");
            return Err(AppError::Auth("invalid auth signature".to_string()));
        }

        Ok(())
    }

    /// Generate auth signature (for server-side use, e.g. in tests or server-sent auth).
    pub fn sign_channel(
        &self,
        socket_id: &str,
        channel: &str,
        channel_data: Option<&str>,
    ) -> AppResult<String> {
        let channel_type = ChannelType::from_name(channel);
        let sign_payload = if channel_type == ChannelType::Presence {
            format!(
                "{}:{}:{}",
                socket_id,
                channel,
                channel_data.unwrap_or("{}")
            )
        } else {
            format!("{}:{}", socket_id, channel)
        };

        let mut mac = HmacSha256::new_from_slice(self.app_secret.as_bytes())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("HMAC init: {}", e)))?;
        mac.update(sign_payload.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_private_channel_auth() {
        let auth = AuthService::new("secret".to_string(), "key".to_string());
        let sig = auth.sign_channel("123.456", "private-foo", None).unwrap();
        assert!(auth
            .verify_channel_auth("private-foo", "123.456", Some(&sig), None)
            .is_ok());
    }

    #[test]
    fn test_verify_private_channel_auth_fail_wrong_sig() {
        let auth = AuthService::new("secret".to_string(), "key".to_string());
        assert!(auth
            .verify_channel_auth("private-foo", "123.456", Some("wrong"), None)
            .is_err());
    }

    #[test]
    fn test_public_channel_no_auth_required() {
        let auth = AuthService::new("secret".to_string(), "key".to_string());
        assert!(auth
            .verify_channel_auth("public-foo", "123.456", None, None)
            .is_ok());
    }
}
