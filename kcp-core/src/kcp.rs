//! Core KCP control object wrapper.
//!
//! This module provides [`Kcp`], a safe Rust wrapper around the C KCP control
//! block (`IKCPCB`). It manages the lifecycle, output callback, and provides
//! safe methods for all KCP operations.
//!
//! # Architecture
//!
//! The output callback mechanism works as follows:
//! 1. User provides a closure `FnMut(&[u8]) -> io::Result<usize>` when creating `Kcp`
//! 2. The closure is boxed and stored in a `KcpContext` on the heap
//! 3. A pointer to `KcpContext` is passed as the `user` pointer to `ikcp_create`
//! 4. When KCP calls the C output callback, it bridges back to the Rust closure
//!
//! # Thread Safety
//!
//! `Kcp` implements `Send` but not `Sync`:
//! - It can be moved between threads
//! - It cannot be shared between threads without external synchronization
//! - Use `Mutex<Kcp>` or `tokio::sync::Mutex<Kcp>` for concurrent access

use std::os::raw::{c_char, c_int, c_long, c_void};
use std::io;

use kcp_sys::{self, IKCPCB};
use crate::config::KcpConfig;
use crate::error::{KcpError, KcpResult};

/// Internal context stored alongside the KCP instance.
///
/// This is heap-allocated and its pointer is passed as the `user` data
/// to the C KCP library. It holds the Rust output callback closure.
struct KcpContext {
    /// The output callback that sends lower-level packets.
    output: Box<dyn FnMut(&[u8]) -> io::Result<usize>>,
}

/// The C-compatible output callback function.
///
/// This is set as the output function on the C KCP instance. When KCP
/// needs to send a packet, it calls this function, which bridges to
/// the Rust closure stored in `KcpContext`.
///
/// # Safety
///
/// This function is called from C code. The `user` pointer must be a valid
/// pointer to a `KcpContext` that was created by `Kcp::new` or `Kcp::with_config`.
unsafe extern "C" fn kcp_output_cb(
    buf: *const c_char,
    len: c_int,
    _kcp: *mut IKCPCB,
    user: *mut c_void,
) -> c_int {
    if user.is_null() || buf.is_null() || len <= 0 {
        return -1;
    }

    let ctx = &mut *(user as *mut KcpContext);
    let data = std::slice::from_raw_parts(buf as *const u8, len as usize);

    match (ctx.output)(data) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Safe wrapper around the KCP control block.
///
/// Manages the lifecycle of a KCP instance, including the output callback
/// and memory cleanup. All unsafe FFI calls are encapsulated within safe methods.
///
/// # Example
///
/// ```no_run
/// use kcp_core::{Kcp, KcpConfig};
/// use std::io;
///
/// // Create a KCP instance with an output callback
/// let mut kcp = Kcp::new(0x01, |data: &[u8]| -> io::Result<usize> {
///     // Send data via UDP or other transport
///     println!("KCP wants to send {} bytes", data.len());
///     Ok(data.len())
/// }).unwrap();
///
/// // Configure for fast mode
/// kcp.apply_config(&KcpConfig::fast());
///
/// // Send data
/// kcp.send(b"Hello, KCP!").unwrap();
///
/// // Update state (call periodically)
/// kcp.update(0);
/// ```
pub struct Kcp {
    /// Raw pointer to the C KCP control block.
    kcp: *mut IKCPCB,
    /// Heap-allocated context holding the output callback.
    /// Stored here to ensure it lives as long as the Kcp instance.
    _ctx: *mut KcpContext,
}

// SAFETY: Kcp can be sent between threads because:
// 1. The C KCP library doesn't use thread-local storage
// 2. All state is contained within the IKCPCB struct and our KcpContext
// 3. The raw pointers are only accessed through &mut self methods
// However, Kcp is NOT Sync because concurrent access to the C library is unsafe.
unsafe impl Send for Kcp {}

impl Kcp {
    /// Creates a new KCP instance with the given conversation ID and output callback.
    ///
    /// The `output` callback is called whenever KCP needs to send a lower-level
    /// packet (e.g., via UDP). It receives the packet data as a byte slice and
    /// should return the number of bytes sent.
    ///
    /// # Arguments
    ///
    /// * `conv` - Conversation ID. Must be the same on both endpoints.
    /// * `output` - Callback function for sending lower-level packets.
    ///
    /// # Errors
    ///
    /// Returns `KcpError::CreateFailed` if the C library fails to allocate memory.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use kcp_core::Kcp;
    /// use std::io;
    ///
    /// let mut kcp = Kcp::new(1, |data: &[u8]| -> io::Result<usize> {
    ///     Ok(data.len())
    /// }).unwrap();
    /// ```
    pub fn new<F>(conv: u32, output: F) -> KcpResult<Self>
    where
        F: FnMut(&[u8]) -> io::Result<usize> + 'static,
    {
        // Allocate the context on the heap
        let ctx = Box::new(KcpContext {
            output: Box::new(output),
        });
        let ctx_ptr = Box::into_raw(ctx);

        // Create the C KCP instance with context pointer as user data
        let kcp = unsafe { kcp_sys::ikcp_create(conv, ctx_ptr as *mut c_void) };

        if kcp.is_null() {
            // Clean up context if create failed
            unsafe { drop(Box::from_raw(ctx_ptr)); }
            return Err(KcpError::CreateFailed);
        }

        // Set the output callback
        unsafe {
            kcp_sys::ikcp_setoutput(kcp, kcp_output_cb);
        }

        Ok(Kcp { kcp, _ctx: ctx_ptr })
    }

    /// Creates a new KCP instance with the given configuration.
    ///
    /// This is a convenience method that creates a new instance and applies
    /// the given configuration in one step.
    ///
    /// # Arguments
    ///
    /// * `conv` - Conversation ID.
    /// * `config` - KCP configuration to apply.
    /// * `output` - Callback function for sending lower-level packets.
    ///
    /// # Errors
    ///
    /// Returns an error if creation fails or if the configuration is invalid.
    pub fn with_config<F>(conv: u32, config: &KcpConfig, output: F) -> KcpResult<Self>
    where
        F: FnMut(&[u8]) -> io::Result<usize> + 'static,
    {
        let mut kcp = Self::new(conv, output)?;
        kcp.apply_config(config)?;
        Ok(kcp)
    }

    /// Applies a [`KcpConfig`] to this KCP instance.
    ///
    /// This sets nodelay, interval, resend, congestion control, MTU,
    /// window sizes, and stream mode all at once.
    ///
    /// # Errors
    ///
    /// Returns an error if setting the MTU fails.
    pub fn apply_config(&mut self, config: &KcpConfig) -> KcpResult<()> {
        self.set_nodelay(config.nodelay, config.interval, config.resend, config.nc);
        self.set_mtu(config.mtu)?;
        self.set_wndsize(config.snd_wnd, config.rcv_wnd);
        self.set_stream_mode(config.stream_mode);
        Ok(())
    }

    // -----------------------------------------------------------------
    // Data Transfer
    // -----------------------------------------------------------------

    /// Send data through KCP.
    ///
    /// In message mode, the data will be sent as a single message and
    /// received as a single message on the other end.
    ///
    /// In stream mode, the data may be merged with other sends or split
    /// across multiple receives.
    ///
    /// # Arguments
    ///
    /// * `buf` - The data to send.
    ///
    /// # Returns
    ///
    /// The number of bytes queued for sending (always equal to `buf.len()` on success).
    ///
    /// # Errors
    ///
    /// Returns `KcpError::SendFailed` if KCP rejects the data.
    pub fn send(&mut self, buf: &[u8]) -> KcpResult<usize> {
        let ret = unsafe {
            kcp_sys::ikcp_send(
                self.kcp,
                buf.as_ptr() as *const c_char,
                buf.len() as c_int,
            )
        };
        if ret < 0 {
            Err(KcpError::SendFailed(ret))
        } else {
            Ok(buf.len())
        }
    }

    /// Receive data from KCP.
    ///
    /// Attempts to read a complete message from KCP's receive queue.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to receive data into. Should be large enough to
    ///   hold the complete message (use [`peeksize`](Self::peeksize) to check).
    ///
    /// # Returns
    ///
    /// The number of bytes received.
    ///
    /// # Errors
    ///
    /// - `KcpError::RecvWouldBlock` if no complete message is available.
    /// - `KcpError::RecvBufferTooSmall` if the buffer is too small.
    /// - `KcpError::RecvFailed` for other receive errors.
    pub fn recv(&mut self, buf: &mut [u8]) -> KcpResult<usize> {
        let ret = unsafe {
            kcp_sys::ikcp_recv(
                self.kcp,
                buf.as_mut_ptr() as *mut c_char,
                buf.len() as c_int,
            )
        };
        if ret >= 0 {
            Ok(ret as usize)
        } else if ret == -1 {
            // No data available
            Err(KcpError::RecvWouldBlock)
        } else if ret == -2 {
            // Buffer too small
            let need = self.peeksize().unwrap_or(0);
            Err(KcpError::RecvBufferTooSmall {
                need,
                got: buf.len(),
            })
        } else {
            Err(KcpError::RecvFailed(ret))
        }
    }

    /// Input a lower-level packet received from the network.
    ///
    /// Call this when you receive a UDP packet. KCP will parse the packet,
    /// process ACKs, and reassemble data for later retrieval via [`recv`](Self::recv).
    ///
    /// # Arguments
    ///
    /// * `data` - The raw packet data received from the network.
    ///
    /// # Errors
    ///
    /// Returns `KcpError::InputFailed` if the packet is malformed or invalid.
    pub fn input(&mut self, data: &[u8]) -> KcpResult<()> {
        let ret = unsafe {
            kcp_sys::ikcp_input(
                self.kcp,
                data.as_ptr() as *const c_char,
                data.len() as c_long,
            )
        };
        if ret < 0 {
            Err(KcpError::InputFailed(ret))
        } else {
            Ok(())
        }
    }

    // -----------------------------------------------------------------
    // State Management
    // -----------------------------------------------------------------

    /// Update KCP state.
    ///
    /// **Must be called periodically** (typically every 10-100ms) to drive
    /// the KCP protocol state machine. This handles retransmissions,
    /// ACK sending, window probing, etc.
    ///
    /// # Arguments
    ///
    /// * `current` - Current timestamp in milliseconds (monotonically increasing).
    pub fn update(&mut self, current: u32) {
        unsafe {
            kcp_sys::ikcp_update(self.kcp, current);
        }
    }

    /// Determine when [`update`](Self::update) should next be called.
    ///
    /// Returns a timestamp (in ms) indicating when the next `update` should
    /// occur. This can be used to avoid calling `update` too frequently.
    ///
    /// # Arguments
    ///
    /// * `current` - Current timestamp in milliseconds.
    ///
    /// # Returns
    ///
    /// The timestamp (in ms) when `update` should next be called.
    pub fn check(&self, current: u32) -> u32 {
        unsafe { kcp_sys::ikcp_check(self.kcp, current) }
    }

    /// Flush all pending data immediately.
    ///
    /// Forces KCP to send all queued data right away, without waiting
    /// for the next scheduled update.
    pub fn flush(&mut self) {
        unsafe {
            kcp_sys::ikcp_flush(self.kcp);
        }
    }

    // -----------------------------------------------------------------
    // Status Queries
    // -----------------------------------------------------------------

    /// Check the size of the next message in the receive queue.
    ///
    /// This can be used to allocate a buffer of the right size before
    /// calling [`recv`](Self::recv).
    ///
    /// # Returns
    ///
    /// The size of the next message in bytes.
    ///
    /// # Errors
    ///
    /// Returns `KcpError::RecvWouldBlock` if no complete message is available.
    pub fn peeksize(&self) -> KcpResult<usize> {
        let ret = unsafe { kcp_sys::ikcp_peeksize(self.kcp) };
        if ret < 0 {
            Err(KcpError::RecvWouldBlock)
        } else {
            Ok(ret as usize)
        }
    }

    /// Get the number of packets waiting to be sent.
    ///
    /// This includes packets in both the send queue and send buffer
    /// (i.e., packets not yet acknowledged by the remote end).
    pub fn waitsnd(&self) -> u32 {
        unsafe { kcp_sys::ikcp_waitsnd(self.kcp) as u32 }
    }

    /// Get the conversation ID of this KCP instance.
    pub fn conv(&self) -> u32 {
        unsafe { (*self.kcp).conv }
    }

    // -----------------------------------------------------------------
    // Configuration
    // -----------------------------------------------------------------

    /// Set the Maximum Transmission Unit (MTU).
    ///
    /// Default is 1400 bytes. Should account for UDP/IP header overhead
    /// (typically 28 bytes).
    ///
    /// # Errors
    ///
    /// Returns `KcpError::SetMtuFailed` if the MTU value is invalid
    /// (e.g., too small to hold KCP header).
    pub fn set_mtu(&mut self, mtu: u32) -> KcpResult<()> {
        let ret = unsafe { kcp_sys::ikcp_setmtu(self.kcp, mtu as c_int) };
        if ret < 0 {
            Err(KcpError::SetMtuFailed {
                mtu,
                code: ret,
            })
        } else {
            Ok(())
        }
    }

    /// Set the send and receive window sizes.
    ///
    /// # Arguments
    ///
    /// * `snd_wnd` - Send window size (default: 32)
    /// * `rcv_wnd` - Receive window size (default: 128)
    pub fn set_wndsize(&mut self, snd_wnd: u32, rcv_wnd: u32) {
        unsafe {
            kcp_sys::ikcp_wndsize(self.kcp, snd_wnd as c_int, rcv_wnd as c_int);
        }
    }

    /// Configure nodelay mode and related parameters.
    ///
    /// # Arguments
    ///
    /// * `nodelay` - Enable nodelay mode (lower latency RTO calculations)
    /// * `interval` - Internal update interval in milliseconds (default: 100)
    /// * `resend` - Fast resend trigger count (0=disabled, 2=recommended)
    /// * `nc` - Disable congestion control
    pub fn set_nodelay(&mut self, nodelay: bool, interval: u32, resend: i32, nc: bool) {
        unsafe {
            kcp_sys::ikcp_nodelay(
                self.kcp,
                nodelay as c_int,
                interval as c_int,
                resend as c_int,
                nc as c_int,
            );
        }
    }

    /// Set stream mode.
    ///
    /// In stream mode, KCP behaves like a byte stream (similar to TCP).
    /// In message mode (default), message boundaries are preserved.
    pub fn set_stream_mode(&mut self, enabled: bool) {
        unsafe {
            (*self.kcp).stream = enabled as c_int;
        }
    }

    /// Get whether stream mode is enabled.
    pub fn is_stream_mode(&self) -> bool {
        unsafe { (*self.kcp).stream != 0 }
    }

    /// Read the conversation ID from a raw KCP packet header.
    ///
    /// This is a static utility method that can be used to determine
    /// which KCP instance should handle a received packet.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw packet data (at least 4 bytes for the conv field).
    ///
    /// # Returns
    ///
    /// The conversation ID from the packet header.
    pub fn get_conv(data: &[u8]) -> u32 {
        unsafe { kcp_sys::ikcp_getconv(data.as_ptr() as *const c_void) }
    }
}

impl Drop for Kcp {
    fn drop(&mut self) {
        unsafe {
            // Release the C KCP instance
            kcp_sys::ikcp_release(self.kcp);

            // Free the context
            if !self._ctx.is_null() {
                drop(Box::from_raw(self._ctx));
            }
        }
    }
}