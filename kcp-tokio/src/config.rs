//! Runtime configuration for KCP sessions.
//!
//! This module provides [`KcpSessionConfig`] for configuring async KCP
//! sessions, including KCP protocol parameters and session-level settings.

use std::time::Duration;
use kcp_core::KcpConfig;

/// Configuration for async KCP sessions.
///
/// Combines the underlying KCP protocol configuration with session-level
/// settings like update interval, timeouts, and flush behavior.
///
/// # Example
///
/// ```
/// use kcp_tokio::config::KcpSessionConfig;
///
/// // Use fast preset
/// let config = KcpSessionConfig::fast();
///
/// // Or customize
/// let config = KcpSessionConfig {
///     flush_write: true,
///     ..KcpSessionConfig::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct KcpSessionConfig {
    /// Underlying KCP protocol configuration.
    pub kcp_config: KcpConfig,

    /// How often to call `kcp.update()`.
    ///
    /// This should match or be slightly smaller than `kcp_config.interval`.
    /// Default: matches `kcp_config.interval`.
    pub flush_interval: Duration,

    /// Session timeout duration.
    ///
    /// If no data is received from the remote end within this duration,
    /// the session is considered dead and will be closed.
    ///
    /// - `None`: No timeout (default)
    /// - `Some(duration)`: Timeout after the specified duration
    pub timeout: Option<Duration>,

    /// Whether to flush KCP immediately after each write.
    ///
    /// When enabled, every `send` or `write` will trigger an immediate
    /// `kcp.flush()`, reducing latency at the cost of more packets.
    ///
    /// Default: `true`
    pub flush_write: bool,

    /// Maximum receive buffer size in bytes.
    ///
    /// This is the size of the internal buffer used for receiving
    /// UDP packets and KCP messages.
    ///
    /// Default: 65536 bytes
    pub recv_buf_size: usize,
}

impl Default for KcpSessionConfig {
    fn default() -> Self {
        let kcp_config = KcpConfig::default();
        Self {
            flush_interval: Duration::from_millis(kcp_config.interval as u64),
            kcp_config,
            timeout: None,
            flush_write: true,
            recv_buf_size: 65536,
        }
    }
}

impl KcpSessionConfig {
    /// Creates a fast (low-latency) session configuration.
    ///
    /// Uses `KcpConfig::fast()` with a 10ms flush interval.
    pub fn fast() -> Self {
        let kcp_config = KcpConfig::fast();
        Self {
            flush_interval: Duration::from_millis(kcp_config.interval as u64),
            kcp_config,
            timeout: Some(Duration::from_secs(30)),
            flush_write: true,
            recv_buf_size: 65536,
        }
    }

    /// Creates a normal (balanced) session configuration.
    ///
    /// Uses `KcpConfig::normal()` with a 40ms flush interval.
    pub fn normal() -> Self {
        let kcp_config = KcpConfig::normal();
        Self {
            flush_interval: Duration::from_millis(kcp_config.interval as u64),
            kcp_config,
            timeout: Some(Duration::from_secs(60)),
            flush_write: true,
            recv_buf_size: 65536,
        }
    }
}