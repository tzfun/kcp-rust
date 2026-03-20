//! Error types for KCP operations.
//!
//! This module defines the error types used throughout `kcp-core`.
//! All KCP C library error codes are mapped to meaningful Rust error variants.

use thiserror::Error;

/// Errors that can occur during KCP operations.
#[derive(Debug, Error)]
pub enum KcpError {
    /// Failed to create a KCP instance (ikcp_create returned null).
    #[error("failed to create KCP instance")]
    CreateFailed,

    /// Failed to send data through KCP.
    /// The inner value is the error code returned by `ikcp_send`.
    #[error("send failed (error code: {0})")]
    SendFailed(i32),

    /// No complete message is available for receiving (EAGAIN-like).
    #[error("no data available to receive (would block)")]
    RecvWouldBlock,

    /// The provided receive buffer is too small to hold the message.
    #[error("receive buffer too small (need {need} bytes, got {got} bytes)")]
    RecvBufferTooSmall {
        /// The required buffer size in bytes.
        need: usize,
        /// The actual buffer size provided.
        got: usize,
    },

    /// Failed to receive data from KCP.
    /// The inner value is the error code returned by `ikcp_recv`.
    #[error("recv failed (error code: {0})")]
    RecvFailed(i32),

    /// Failed to process input data.
    /// The inner value is the error code returned by `ikcp_input`.
    #[error("input failed (error code: {0})")]
    InputFailed(i32),

    /// Failed to set MTU (invalid value or internal error).
    #[error("failed to set MTU to {mtu} (error code: {code})")]
    SetMtuFailed {
        /// The MTU value that was attempted.
        mtu: u32,
        /// The error code from the C library.
        code: i32,
    },

    /// Invalid configuration parameter.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Conversation ID mismatch.
    #[error("conversation ID mismatch (expected {expected}, got {got})")]
    ConvMismatch {
        /// The expected conversation ID.
        expected: u32,
        /// The actual conversation ID.
        got: u32,
    },

    /// An I/O error occurred in the output callback.
    #[error("output callback I/O error: {0}")]
    OutputError(#[from] std::io::Error),
}

/// Result type alias for KCP operations.
pub type KcpResult<T> = Result<T, KcpError>;