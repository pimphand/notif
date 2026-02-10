//! Authentication: register, login, JWT.

mod jwt;
mod handlers;
mod service;

pub use handlers::{login, register};
pub use jwt::{Claims, JwtSecret};
pub use service::AuthAppService;
