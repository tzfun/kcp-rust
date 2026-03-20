//! Error types for KCP operations.
//!
//! Defines [`KcpError`] which covers all failure modes of the KCP protocol
//! engine, and [`KcpResult<T>`] as a convenient type alias.

use thiserror::Error;

/// Error type for KCP protocol operations.
///
/// Each variant maps to a specific failure condition when interacting with
/// the underlying KCP C library through the safe [`Kcp`](super::Kcp) wrapper.
#[derive(Debug, Error)]
pub enum KcpError {
    /// Failed to create a KCP instance (e.g., `ikcp_create` returned null).
    #[error("failed to create KCP instance")]
    CreateFailed,
    /// Failed to send data through KCP. The inner value is the C error code.
    #[error("send failed (error code: {0})")]
    SendFailed(i32),
    /// No data is available to receive right now. This is not a fatal error;
    /// the caller should retry after feeding more input data and calling `update()`.
    #[error("no data available to receive (would block)")]
    RecvWouldBlock,
    /// The provided receive buffer is too small for the next message.
    #[error("receive buffer too small (need {need} bytes, got {got} bytes)")]
    RecvBufferTooSmall {
        /// The size of the next available message.
        need: usize,
        /// The size of the buffer that was provided.
        got: usize,
    },
    /// A generic receive failure with the C error code.
    #[error("recv failed (error code: {0})")]
    RecvFailed(i32),
    /// Failed to feed input data to the KCP engine (e.g., corrupted packet).
    #[error("input failed (error code: {0})")]
    InputFailed(i32),
    /// Failed to set the MTU to the specified value.
    #[error("failed to set MTU to {mtu} (error code: {code})")]
    SetMtuFailed {
        /// The MTU value that was attempted.
        mtu: u32,
        /// The C error code returned.
        code: i32,
    },
    /// An invalid configuration parameter was provided.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    /// The conversation ID in the received packet does not match the expected one.
    #[error("conversation ID mismatch (expected {expected}, got {got})")]
    ConvMismatch {
        /// The expected conversation ID.
        expected: u32,
        /// The conversation ID found in the packet.
        got: u32,
    },
    /// An I/O error occurred in the output callback.
    #[error("output callback I/O error: {0}")]
    OutputError(#[from] std::io::Error),
}

/// A convenience type alias for `Result<T, KcpError>`.
pub type KcpResult<T> = Result<T, KcpError>;