//! Error types for async KCP operations.
//!
//! Defines [`KcpTokioError`] which covers all failure modes of the async
//! KCP session layer, and [`KcpTokioResult<T>`] as a convenient type alias.

use thiserror::Error;

/// Error type for async KCP session operations.
///
/// Wraps both I/O errors and lower-level [`KcpError`](crate::core::KcpError)s,
/// plus session-level errors like timeout and connection closure.
#[derive(Debug, Error)]
pub enum KcpTokioError {
    /// An I/O error occurred on the underlying UDP socket.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// An error from the core KCP engine.
    #[error("KCP error: {0}")]
    Kcp(#[from] crate::core::KcpError),
    /// The session timed out due to no data received within the configured timeout period.
    #[error("session timed out")]
    Timeout,
    /// The session has been closed (either locally or by the remote peer).
    #[error("session closed")]
    Closed,
    /// Failed to establish a connection (e.g., address resolution failure).
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
}

/// A convenience type alias for `Result<T, KcpTokioError>`.
pub type KcpTokioResult<T> = Result<T, KcpTokioError>;