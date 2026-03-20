//! KCP protocol configuration parameters and presets.
//!
//! This module defines [`KcpConfig`], which controls the behavior of the KCP
//! protocol engine including nodelay mode, update interval, retransmission
//! strategy, window sizes, MTU, and stream mode.

/// KCP protocol configuration parameters.
///
/// Controls the behavior of the underlying KCP engine. Use the provided presets
/// ([`default()`](KcpConfig::default), [`fast()`](KcpConfig::fast),
/// [`normal()`](KcpConfig::normal)) or customize individual fields.
///
/// # Example
///
/// ```
/// use kcp_io::core::KcpConfig;
///
/// // Use a preset
/// let config = KcpConfig::fast();
///
/// // Customize from a preset
/// let config = KcpConfig {
///     mtu: 1200,
///     snd_wnd: 256,
///     rcv_wnd: 256,
///     ..KcpConfig::fast()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct KcpConfig {
    /// Enable nodelay mode. When `true`, KCP disables the wait-to-send delay
    /// and sends packets as soon as possible, reducing latency.
    pub nodelay: bool,
    /// Internal update interval in milliseconds. Lower values reduce latency
    /// but increase CPU usage. Typical range: 10–100 ms.
    pub interval: u32,
    /// Fast retransmit trigger count. When an out-of-order ACK is received
    /// this many times, KCP immediately retransmits the packet without waiting
    /// for a timeout. Set to 0 to disable fast retransmit.
    pub resend: i32,
    /// Disable congestion control. When `true`, KCP ignores the congestion
    /// window and sends at full speed (uses `nocwnd` mode). Useful for
    /// low-latency scenarios where bandwidth is not a concern.
    pub nc: bool,
    /// Maximum Transmission Unit in bytes. This is the maximum size of a single
    /// KCP output packet (including the 24-byte KCP header). Must account for
    /// UDP header overhead (28 bytes for IPv4). Default: 1400.
    pub mtu: u32,
    /// Send window size (number of packets). Larger values allow higher
    /// throughput but consume more memory. Default: 32.
    pub snd_wnd: u32,
    /// Receive window size (number of packets). Should generally be >= `snd_wnd`.
    /// Larger values allow higher throughput. Default: 128.
    pub rcv_wnd: u32,
    /// Enable stream mode. When `true`, KCP operates like a byte stream (similar
    /// to TCP), merging small packets and splitting large ones. When `false`
    /// (default), KCP preserves message boundaries.
    pub stream_mode: bool,
}

impl Default for KcpConfig {
    /// Returns a conservative default configuration.
    ///
    /// - nodelay: `false`
    /// - interval: `100` ms
    /// - resend: `0` (no fast retransmit)
    /// - nc: `false` (congestion control enabled)
    /// - mtu: `1400`
    /// - snd_wnd: `32`, rcv_wnd: `128`
    /// - stream_mode: `false`
    fn default() -> Self {
        Self {
            nodelay: false,
            interval: 100,
            resend: 0,
            nc: false,
            mtu: 1400,
            snd_wnd: 32,
            rcv_wnd: 128,
            stream_mode: false,
        }
    }
}

impl KcpConfig {
    /// Returns a low-latency (fast) configuration preset.
    ///
    /// Optimized for minimal latency at the cost of higher bandwidth usage.
    ///
    /// - nodelay: `true`
    /// - interval: `10` ms
    /// - resend: `2` (fast retransmit after 2 out-of-order ACKs)
    /// - nc: `true` (congestion control disabled)
    /// - mtu: `1400`
    /// - snd_wnd: `128`, rcv_wnd: `128`
    /// - stream_mode: `false`
    pub fn fast() -> Self {
        Self {
            nodelay: true,
            interval: 10,
            resend: 2,
            nc: true,
            mtu: 1400,
            snd_wnd: 128,
            rcv_wnd: 128,
            stream_mode: false,
        }
    }

    /// Returns a balanced (normal) configuration preset.
    ///
    /// Balances latency and bandwidth usage. Good for most use cases.
    ///
    /// - nodelay: `true`
    /// - interval: `40` ms
    /// - resend: `2`
    /// - nc: `false` (congestion control enabled)
    /// - mtu: `1400`
    /// - snd_wnd: `64`, rcv_wnd: `128`
    /// - stream_mode: `false`
    pub fn normal() -> Self {
        Self {
            nodelay: true,
            interval: 40,
            resend: 2,
            nc: false,
            mtu: 1400,
            snd_wnd: 64,
            rcv_wnd: 128,
            stream_mode: false,
        }
    }
}
