//! Error types for async KCP operations.

use thiserror::Error;

/// Errors that can occur during async KCP operations.
#[derive(Debug, Error)]
pub enum KcpTokioError {
    // Will be implemented in Phase 3
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("KCP error: {0}")]
    Kcp(#[from] kcp_core::KcpError),
}
