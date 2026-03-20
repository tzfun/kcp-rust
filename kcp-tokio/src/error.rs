//! Error types for async KCP operations.
//!
//! This module provides [`KcpTokioError`], which unifies I/O errors,
//! KCP protocol errors, and session-level errors.

use thiserror::Error;

/// Errors that can occur during async KCP operations.
#[derive(Debug, Error)]
pub enum KcpTokioError {
    /// An I/O error occurred (e.g., UDP socket error).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A KCP protocol error occurred.
    #[error("KCP error: {0}")]
    Kcp(#[from] kcp_core::KcpError),

    /// The session has timed out (no data received within the timeout period).
    #[error("session timed out")]
    Timeout,

    /// The session has been closed.
    #[error("session closed")]
    Closed,

    /// The connection was refused or the handshake failed.
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
}

/// Result type alias for async KCP operations.
pub type KcpTokioResult<T> = Result<T, KcpTokioError>;