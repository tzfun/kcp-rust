//! Safe Rust wrapper for the KCP protocol.

mod config;
mod error;
mod kcp;

pub use config::KcpConfig;
pub use error::{KcpError, KcpResult};
pub use kcp::Kcp;
