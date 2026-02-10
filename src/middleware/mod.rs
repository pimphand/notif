//! Middleware: auth for private/presence channels is applied in WebSocket handler, not HTTP.
//! This module can hold shared auth extractors if we add HTTP auth later.

pub mod auth;

pub use auth::AuthLayer;
