//! Error types for async KCP operations.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KcpTokioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("KCP error: {0}")]
    Kcp(#[from] crate::core::KcpError),
    #[error("session timed out")]
    Timeout,
    #[error("session closed")]
    Closed,
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
}

pub type KcpTokioResult<T> = Result<T, KcpTokioError>;
