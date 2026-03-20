//! Core KCP control object wrapper.
//!
//! This module provides [`Kcp`], a safe Rust wrapper around the C `IKCPCB` struct.
//! It handles memory management (creation/release), output callback bridging, and
//! exposes all KCP operations through a safe API with proper error handling.

use std::io;
use std::os::raw::{c_char, c_int, c_long, c_void};
use super::config::KcpConfig;
use super::error::{KcpError, KcpResult};
use crate::sys::IKCPCB;

/// Type alias for the output callback closure.
///
/// The callback receives a byte slice of the raw KCP packet data and should
/// send it via the underlying transport (e.g., UDP). Returns the number of
/// bytes sent or an I/O error.
type OutputCallback = Box<dyn FnMut(&[u8]) -> io::Result<usize>>;

/// Internal context passed through the KCP `user` pointer to bridge
/// C callbacks to Rust closures.
struct KcpContext {
    output: OutputCallback,
}

/// C-compatible output callback function that bridges KCP's C output callback
/// to the Rust closure stored in [`KcpContext`].
///
/// # Safety
///
/// This function is called by the KCP C library. The `user` pointer must be
/// a valid `*mut KcpContext` that was set via `ikcp_create`.
unsafe extern "C" fn kcp_output_cb(
    buf: *const c_char, len: c_int, _kcp: *mut IKCPCB, user: *mut c_void,
) -> c_int {
    if user.is_null() || buf.is_null() || len <= 0 { return -1; }
    let ctx = &mut *(user as *mut KcpContext);
    let data = std::slice::from_raw_parts(buf as *const u8, len as usize);
    match (ctx.output)(data) { Ok(_) => 0, Err(_) => -1 }
}

/// Safe wrapper around the KCP control block (`IKCPCB`).
///
/// `Kcp` manages the lifecycle of a KCP protocol instance. It handles:
/// - Creating and releasing the underlying C object
/// - Bridging the C output callback to a Rust closure
/// - Providing safe methods for all KCP operations
///
/// # Thread Safety
///
/// `Kcp` implements `Send` but not `Sync`. It can be moved between threads
/// but cannot be shared by reference across threads. All mutating operations
/// require `&mut self`, ensuring exclusive access.
///
/// # Example
///
/// ```no_run
/// use kcp_io::core::{Kcp, KcpConfig};
/// use std::io;
///
/// let mut kcp = Kcp::new(0x01, |data: &[u8]| -> io::Result<usize> {
///     Ok(data.len())
/// }).unwrap();
///
/// kcp.apply_config(&KcpConfig::fast());
/// kcp.send(b"Hello, KCP!").unwrap();
/// kcp.update(0);
/// ```
pub struct Kcp {
    /// Raw pointer to the C KCP control block.
    kcp: *mut IKCPCB,
    /// Raw pointer to the KcpContext (Box-allocated, freed on Drop).
    _ctx: *mut KcpContext,
}

// SAFETY: Kcp can be moved between threads safely because:
// 1. The KCP C library does not use thread-local storage.
// 2. All mutable operations require &mut self, preventing concurrent access.
// 3. The KcpContext is only accessed through the single Kcp owner.
unsafe impl Send for Kcp {}

impl Kcp {
    /// Creates a new KCP instance with the given conversation ID and output callback.
    ///
    /// The `output` callback is invoked by KCP whenever it needs to send a raw
    /// protocol packet. You should implement this to send the data via your
    /// transport layer (e.g., `UdpSocket::send_to`).
    ///
    /// # Arguments
    ///
    /// * `conv` — Conversation ID. Both endpoints must use the same conv for a session.
    /// * `output` — Callback invoked when KCP needs to send a packet.
    ///
    /// # Errors
    ///
    /// Returns [`KcpError::CreateFailed`] if the C `ikcp_create` call fails.
    pub fn new<F>(conv: u32, output: F) -> KcpResult<Self>
    where F: FnMut(&[u8]) -> io::Result<usize> + 'static {
        let ctx = Box::new(KcpContext { output: Box::new(output) });
        let ctx_ptr = Box::into_raw(ctx);
        let kcp = unsafe { crate::sys::ikcp_create(conv, ctx_ptr as *mut c_void) };
        if kcp.is_null() {
            unsafe { drop(Box::from_raw(ctx_ptr)); }
            return Err(KcpError::CreateFailed);
        }
        unsafe { crate::sys::ikcp_setoutput(kcp, kcp_output_cb); }
        Ok(Kcp { kcp, _ctx: ctx_ptr })
    }

    /// Creates a new KCP instance with a configuration preset applied.
    ///
    /// Equivalent to calling [`Kcp::new`] followed by [`Kcp::apply_config`].
    ///
    /// # Errors
    ///
    /// Returns an error if creation or configuration fails.
    pub fn with_config<F>(conv: u32, config: &KcpConfig, output: F) -> KcpResult<Self>
    where F: FnMut(&[u8]) -> io::Result<usize> + 'static {
        let mut kcp = Self::new(conv, output)?;
        kcp.apply_config(config)?;
        Ok(kcp)
    }

    /// Applies a [`KcpConfig`] to this KCP instance.
    ///
    /// Sets nodelay, interval, resend, nc, MTU, window sizes, and stream mode.
    ///
    /// # Errors
    ///
    /// Returns [`KcpError::SetMtuFailed`] if the MTU value is invalid.
    pub fn apply_config(&mut self, config: &KcpConfig) -> KcpResult<()> {
        self.set_nodelay(config.nodelay, config.interval, config.resend, config.nc);
        self.set_mtu(config.mtu)?;
        self.set_wndsize(config.snd_wnd, config.rcv_wnd);
        self.set_stream_mode(config.stream_mode);
        Ok(())
    }

    /// Sends data through the KCP protocol.
    ///
    /// In message mode, each call to `send` is treated as a single message.
    /// In stream mode, data may be merged or split across KCP packets.
    ///
    /// The data is queued internally; call [`update`](Kcp::update) or
    /// [`flush`](Kcp::flush) to actually transmit it via the output callback.
    ///
    /// # Errors
    ///
    /// Returns [`KcpError::SendFailed`] if the C `ikcp_send` call returns an error.
    pub fn send(&mut self, buf: &[u8]) -> KcpResult<usize> {
        let ret = unsafe { crate::sys::ikcp_send(self.kcp, buf.as_ptr() as *const c_char, buf.len() as c_int) };
        if ret < 0 { Err(KcpError::SendFailed(ret)) } else { Ok(buf.len()) }
    }

    /// Receives data from the KCP protocol.
    ///
    /// In message mode, each successful call returns exactly one complete message.
    /// In stream mode, returns as many bytes as are available (up to `buf.len()`).
    ///
    /// # Errors
    ///
    /// - [`KcpError::RecvWouldBlock`] — No data available yet.
    /// - [`KcpError::RecvBufferTooSmall`] — Buffer is smaller than the next message.
    /// - [`KcpError::RecvFailed`] — Other receive error.
    pub fn recv(&mut self, buf: &mut [u8]) -> KcpResult<usize> {
        let ret = unsafe { crate::sys::ikcp_recv(self.kcp, buf.as_mut_ptr() as *mut c_char, buf.len() as c_int) };
        if ret >= 0 { Ok(ret as usize) }
        else if ret == -1 { Err(KcpError::RecvWouldBlock) }
        else if ret == -2 { let need = self.peeksize().unwrap_or(0); Err(KcpError::RecvBufferTooSmall { need, got: buf.len() }) }
        else { Err(KcpError::RecvFailed(ret)) }
    }

    /// Feeds raw packet data received from the transport layer into the KCP engine.
    ///
    /// Call this whenever you receive a UDP packet destined for this KCP session.
    /// KCP will parse the packet, process ACKs, and reassemble data for [`recv`](Kcp::recv).
    ///
    /// # Errors
    ///
    /// Returns [`KcpError::InputFailed`] if the data is corrupted or invalid.
    pub fn input(&mut self, data: &[u8]) -> KcpResult<()> {
        let ret = unsafe { crate::sys::ikcp_input(self.kcp, data.as_ptr() as *const c_char, data.len() as c_long) };
        if ret < 0 { Err(KcpError::InputFailed(ret)) } else { Ok(()) }
    }

    /// Drives the KCP state machine. **Must be called periodically.**
    ///
    /// This triggers internal timers for retransmission, window probing, and
    /// flushing queued data. The `current` parameter should be a monotonically
    /// increasing millisecond timestamp.
    ///
    /// # Arguments
    ///
    /// * `current` — Current time in milliseconds (monotonic clock).
    pub fn update(&mut self, current: u32) { unsafe { crate::sys::ikcp_update(self.kcp, current); } }

    /// Returns the earliest time (in ms) when [`update`](Kcp::update) should be called next.
    ///
    /// You can use this to optimize the update interval instead of calling
    /// `update()` at a fixed rate.
    pub fn check(&self, current: u32) -> u32 { unsafe { crate::sys::ikcp_check(self.kcp, current) } }

    /// Immediately flushes all pending data in the send queue.
    ///
    /// This forces KCP to invoke the output callback for any queued packets.
    pub fn flush(&mut self) { unsafe { crate::sys::ikcp_flush(self.kcp); } }

    /// Returns the size of the next available message in the receive queue.
    ///
    /// # Errors
    ///
    /// Returns [`KcpError::RecvWouldBlock`] if no complete message is available.
    pub fn peeksize(&self) -> KcpResult<usize> {
        let ret = unsafe { crate::sys::ikcp_peeksize(self.kcp) };
        if ret < 0 { Err(KcpError::RecvWouldBlock) } else { Ok(ret as usize) }
    }

    /// Returns the number of packets waiting to be sent (in the send queue + send buffer).
    pub fn waitsnd(&self) -> u32 { unsafe { crate::sys::ikcp_waitsnd(self.kcp) as u32 } }

    /// Returns the conversation ID of this KCP instance.
    pub fn conv(&self) -> u32 { unsafe { (*self.kcp).conv } }

    /// Sets the Maximum Transmission Unit (MTU).
    ///
    /// The MTU must be at least `IKCP_OVERHEAD` (24) bytes + 1.
    ///
    /// # Errors
    ///
    /// Returns [`KcpError::SetMtuFailed`] if the value is invalid or too small.
    pub fn set_mtu(&mut self, mtu: u32) -> KcpResult<()> {
        let ret = unsafe { crate::sys::ikcp_setmtu(self.kcp, mtu as c_int) };
        if ret < 0 { Err(KcpError::SetMtuFailed { mtu, code: ret }) } else { Ok(()) }
    }

    /// Sets the send and receive window sizes (in number of packets).
    pub fn set_wndsize(&mut self, snd_wnd: u32, rcv_wnd: u32) {
        unsafe { crate::sys::ikcp_wndsize(self.kcp, snd_wnd as c_int, rcv_wnd as c_int); }
    }

    /// Configures the nodelay mode and related parameters.
    ///
    /// # Arguments
    ///
    /// * `nodelay` — Enable nodelay mode (disable wait-to-send delay).
    /// * `interval` — Internal update interval in milliseconds.
    /// * `resend` — Fast retransmit trigger count (0 to disable).
    /// * `nc` — Disable congestion control (`nocwnd` mode).
    pub fn set_nodelay(&mut self, nodelay: bool, interval: u32, resend: i32, nc: bool) {
        unsafe { crate::sys::ikcp_nodelay(self.kcp, nodelay as c_int, interval as c_int, resend as c_int, nc as c_int); }
    }

    /// Enables or disables stream mode.
    ///
    /// In stream mode, KCP merges small packets and splits large ones,
    /// behaving like a TCP byte stream. In message mode (default), KCP
    /// preserves message boundaries.
    pub fn set_stream_mode(&mut self, enabled: bool) { unsafe { (*self.kcp).stream = enabled as c_int; } }

    /// Returns whether stream mode is currently enabled.
    pub fn is_stream_mode(&self) -> bool { unsafe { (*self.kcp).stream != 0 } }

    /// Extracts the conversation ID (conv) from a raw KCP packet header.
    ///
    /// This is a static method that reads the first 4 bytes of the data
    /// as a little-endian u32 conversation ID. Useful for routing packets
    /// to the correct KCP session before calling [`input`](Kcp::input).
    pub fn get_conv(data: &[u8]) -> u32 {
        unsafe { crate::sys::ikcp_getconv(data.as_ptr() as *const c_void) }
    }
}

impl Drop for Kcp {
    fn drop(&mut self) {
        unsafe {
            crate::sys::ikcp_release(self.kcp);
            if !self._ctx.is_null() { drop(Box::from_raw(self._ctx)); }
        }
    }
}