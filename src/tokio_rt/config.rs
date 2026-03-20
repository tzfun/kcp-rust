//! Runtime configuration for async KCP sessions.
//!
//! This module defines [`KcpSessionConfig`], which combines the low-level
//! KCP protocol parameters ([`KcpConfig`](crate::core::KcpConfig)) with
//! session-level runtime settings such as flush interval, timeout, and
//! buffer sizes.

use crate::core::KcpConfig;
use std::time::Duration;

/// Configuration for async KCP sessions.
///
/// Combines the KCP protocol parameters ([`KcpConfig`](crate::core::KcpConfig))
/// with runtime settings that control session behavior in the async layer.
///
/// # Example
///
/// ```
/// use kcp_io::tokio_rt::KcpSessionConfig;
///
/// // Use a preset
/// let config = KcpSessionConfig::fast();
///
/// // Customize from a preset
/// let config = KcpSessionConfig {
///     timeout: Some(std::time::Duration::from_secs(60)),
///     ..KcpSessionConfig::fast()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct KcpSessionConfig {
    /// The underlying KCP protocol configuration.
    pub kcp_config: KcpConfig,
    /// How often the KCP state machine is updated (drives retransmission and flushing).
    /// This is automatically derived from [`KcpConfig::interval`] in the presets.
    pub flush_interval: Duration,
    /// Session timeout. If no data is received within this duration, the session
    /// is considered dead and will return [`KcpTokioError::Timeout`](super::KcpTokioError::Timeout).
    /// Set to `None` to disable timeout.
    pub timeout: Option<Duration>,
    /// Whether to immediately flush after each write operation. When `true`,
    /// `send_kcp()` calls `kcp.flush()` after sending, reducing latency at
    /// the cost of potentially more packets.
    pub flush_write: bool,
    /// Size of the internal UDP receive buffer in bytes. Should be large enough
    /// to hold the largest expected UDP datagram. Default: 65536.
    pub recv_buf_size: usize,
}

impl Default for KcpSessionConfig {
    /// Returns a default session configuration using [`KcpConfig::default()`].
    ///
    /// - flush_interval: derived from `KcpConfig.interval` (100ms)
    /// - timeout: `None` (no timeout)
    /// - flush_write: `true`
    /// - recv_buf_size: `65536`
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
    /// Returns a low-latency (fast) session configuration.
    ///
    /// Uses [`KcpConfig::fast()`] with a 30-second timeout.
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

    /// Returns a balanced (normal) session configuration.
    ///
    /// Uses [`KcpConfig::normal()`] with a 60-second timeout.
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
