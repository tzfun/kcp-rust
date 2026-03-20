//! KCP configuration presets.
//!
//! This module provides the [`KcpConfig`] structure for configuring KCP
//! protocol parameters, along with several preset configurations for
//! common use cases.

/// KCP protocol configuration parameters.
///
/// Controls the behavior of the KCP protocol including nodelay mode,
/// update interval, fast resend, congestion control, MTU, window sizes,
/// and stream mode.
///
/// # Presets
///
/// - [`KcpConfig::default()`] — Standard configuration (conservative)
/// - [`KcpConfig::fast()`] — Low-latency configuration (aggressive)
/// - [`KcpConfig::normal()`] — Balanced configuration
///
/// # Example
///
/// ```
/// use kcp_core::KcpConfig;
///
/// // Use the fast preset
/// let config = KcpConfig::fast();
///
/// // Or customize individual parameters
/// let config = KcpConfig {
///     mtu: 1200,
///     snd_wnd: 256,
///     rcv_wnd: 256,
///     ..KcpConfig::fast()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct KcpConfig {
    /// Enable nodelay mode.
    ///
    /// When enabled, KCP will use a more aggressive retransmission strategy
    /// with smaller RTO calculations.
    ///
    /// - `false` (default): Normal mode
    /// - `true`: Nodelay mode (lower latency)
    pub nodelay: bool,

    /// Internal update timer interval in milliseconds.
    ///
    /// This controls how often `ikcp_update` should be called.
    /// Smaller values mean more responsive but more CPU usage.
    ///
    /// - Default: 100ms
    /// - Recommended for low latency: 10-20ms
    pub interval: u32,

    /// Fast resend trigger count.
    ///
    /// When a packet has been skipped by this many ACKs from the receiver,
    /// it will be resent immediately without waiting for timeout.
    ///
    /// - `0` (default): Disable fast resend
    /// - `1`: Resend after 1 skip
    /// - `2` (recommended): Resend after 2 skips
    pub resend: i32,

    /// Disable congestion control.
    ///
    /// When enabled, KCP will not reduce the sending window when
    /// packet loss is detected.
    ///
    /// - `false` (default): Normal congestion control
    /// - `true`: Disable congestion control (higher throughput but may cause more loss)
    pub nc: bool,

    /// Maximum Transmission Unit in bytes.
    ///
    /// This should be set according to the underlying transport's MTU.
    /// KCP will split messages larger than (MTU - overhead) into segments.
    ///
    /// Default: 1400 bytes (safe for most networks considering UDP/IP headers)
    pub mtu: u32,

    /// Send window size (number of packets).
    ///
    /// Controls how many packets can be in-flight (sent but not yet acknowledged).
    /// Larger values allow higher throughput on high-latency links.
    ///
    /// Default: 32
    pub snd_wnd: u32,

    /// Receive window size (number of packets).
    ///
    /// Controls how many out-of-order packets can be buffered.
    /// Should generally be >= snd_wnd.
    ///
    /// Default: 128
    pub rcv_wnd: u32,

    /// Enable stream mode.
    ///
    /// In stream mode, KCP will merge small messages together and
    /// split large messages, similar to TCP's byte-stream behavior.
    ///
    /// In message mode (default), each send/recv preserves message boundaries.
    ///
    /// - `false` (default): Message mode (preserves boundaries)
    /// - `true`: Stream mode (byte-stream, like TCP)
    pub stream_mode: bool,
}

impl Default for KcpConfig {
    /// Returns the default KCP configuration.
    ///
    /// This is a conservative configuration suitable for general use:
    /// - Nodelay: disabled
    /// - Interval: 100ms
    /// - Fast resend: disabled
    /// - Congestion control: enabled
    /// - MTU: 1400
    /// - Send window: 32
    /// - Receive window: 128
    /// - Stream mode: disabled
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
    /// Creates a fast (low-latency) configuration preset.
    ///
    /// Optimized for minimal latency at the cost of more bandwidth:
    /// - Nodelay: enabled
    /// - Interval: 10ms
    /// - Fast resend: 2 (resend after 2 ACK skips)
    /// - Congestion control: disabled
    /// - MTU: 1400
    /// - Send window: 128
    /// - Receive window: 128
    /// - Stream mode: disabled
    ///
    /// Equivalent to: `ikcp_nodelay(kcp, 1, 10, 2, 1)`
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

    /// Creates a normal (balanced) configuration preset.
    ///
    /// Balanced between latency and bandwidth efficiency:
    /// - Nodelay: enabled
    /// - Interval: 40ms
    /// - Fast resend: 2
    /// - Congestion control: enabled
    /// - MTU: 1400
    /// - Send window: 64
    /// - Receive window: 128
    /// - Stream mode: disabled
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