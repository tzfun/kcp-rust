//! KCP configuration presets.

/// KCP protocol configuration parameters.
///
/// # Example
///
/// ```
/// use kcp_io::core::KcpConfig;
///
/// let config = KcpConfig::fast();
///
/// let config = KcpConfig {
///     mtu: 1200,
///     snd_wnd: 256,
///     rcv_wnd: 256,
///     ..KcpConfig::fast()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct KcpConfig {
    pub nodelay: bool,
    pub interval: u32,
    pub resend: i32,
    pub nc: bool,
    pub mtu: u32,
    pub snd_wnd: u32,
    pub rcv_wnd: u32,
    pub stream_mode: bool,
}

impl Default for KcpConfig {
    fn default() -> Self {
        Self { nodelay: false, interval: 100, resend: 0, nc: false, mtu: 1400, snd_wnd: 32, rcv_wnd: 128, stream_mode: false }
    }
}

impl KcpConfig {
    pub fn fast() -> Self {
        Self { nodelay: true, interval: 10, resend: 2, nc: true, mtu: 1400, snd_wnd: 128, rcv_wnd: 128, stream_mode: false }
    }
    pub fn normal() -> Self {
        Self { nodelay: true, interval: 40, resend: 2, nc: false, mtu: 1400, snd_wnd: 64, rcv_wnd: 128, stream_mode: false }
    }
}
