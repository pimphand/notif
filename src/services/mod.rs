//! Business logic: channel subscription, presence, and auth.

pub mod auth;
pub mod channel;
pub mod presence;

pub use auth::AuthService;
pub use channel::ChannelService;
pub use presence::PresenceService;
