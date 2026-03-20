//! Runtime configuration for KCP sessions.
use crate::core::KcpConfig;
use std::time::Duration;

/// Configuration for async KCP sessions.
///
/// # Example
///
/// ```
/// use kcp_io::tokio_rt::KcpSessionConfig;
/// let config = KcpSessionConfig::fast();
/// ```
#[derive(Debug, Clone)]
pub struct KcpSessionConfig {
    pub kcp_config: KcpConfig,
    pub flush_interval: Duration,
    pub timeout: Option<Duration>,
    pub flush_write: bool,
    pub recv_buf_size: usize,
}

impl Default for KcpSessionConfig {
    fn default() -> Self {
        let kcp_config = KcpConfig::default();
        Self { flush_interval: Duration::from_millis(kcp_config.interval as u64), kcp_config, timeout: None, flush_write: true, recv_buf_size: 65536 }
    }
}

impl KcpSessionConfig {
    pub fn fast() -> Self {
        let kcp_config = KcpConfig::fast();
        Self { flush_interval: Duration::from_millis(kcp_config.interval as u64), kcp_config, timeout: Some(Duration::from_secs(30)), flush_write: true, recv_buf_size: 65536 }
    }
    pub fn normal() -> Self {
        let kcp_config = KcpConfig::normal();
        Self { flush_interval: Duration::from_millis(kcp_config.interval as u64), kcp_config, timeout: Some(Duration::from_secs(60)), flush_write: true, recv_buf_size: 65536 }
    }
}
