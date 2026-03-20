//! Error types for KCP operations.
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KcpError {
    #[error("failed to create KCP instance")]
    CreateFailed,
    #[error("send failed (error code: {0})")]
    SendFailed(i32),
    #[error("no data available to receive (would block)")]
    RecvWouldBlock,
    #[error("receive buffer too small (need {need} bytes, got {got} bytes)")]
    RecvBufferTooSmall { need: usize, got: usize },
    #[error("recv failed (error code: {0})")]
    RecvFailed(i32),
    #[error("input failed (error code: {0})")]
    InputFailed(i32),
    #[error("failed to set MTU to {mtu} (error code: {code})")]
    SetMtuFailed { mtu: u32, code: i32 },
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("conversation ID mismatch (expected {expected}, got {got})")]
    ConvMismatch { expected: u32, got: u32 },
    #[error("output callback I/O error: {0}")]
    OutputError(#[from] std::io::Error),
}

pub type KcpResult<T> = Result<T, KcpError>;
