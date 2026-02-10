//! Channel types and naming conventions.

use serde::{Deserialize, Serialize};

/// Channel type based on prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    /// Public channel: no auth required.
    Public,
    /// Private channel: requires auth token.
    Private,
    /// Presence channel: auth + track who is online.
    Presence,
}

impl ChannelType {
    /// Derive channel type from name. Pusher-style: `private-*`, `presence-*`.
    pub fn from_name(name: &str) -> Self {
        if name.starts_with("presence-") {
            ChannelType::Presence
        } else if name.starts_with("private-") {
            ChannelType::Private
        } else {
            ChannelType::Public
        }
    }

    pub fn is_private(&self) -> bool {
        matches!(self, ChannelType::Private | ChannelType::Presence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_type_from_name_public() {
        assert_eq!(ChannelType::from_name("my-channel"), ChannelType::Public);
        assert_eq!(ChannelType::from_name("foo"), ChannelType::Public);
    }

    #[test]
    fn channel_type_from_name_private() {
        assert_eq!(
            ChannelType::from_name("private-user-1"),
            ChannelType::Private
        );
    }

    #[test]
    fn channel_type_from_name_presence() {
        assert_eq!(
            ChannelType::from_name("presence-chat"),
            ChannelType::Presence
        );
    }
}
