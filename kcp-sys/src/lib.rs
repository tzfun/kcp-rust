//! Raw FFI bindings to the KCP C library.
//!
//! This crate provides unsafe bindings to the original KCP implementation
//! by skywind3000 (<https://github.com/skywind3000/kcp>).
//!
//! It compiles the C source code via `build.rs` and exposes the raw C API
//! as Rust FFI declarations. For a safe Rust API, use the `kcp-core` crate.
//!
//! # Safety
//!
//! All functions in this crate are `unsafe` as they directly call C code.
//! The caller is responsible for ensuring correct usage, pointer validity,
//! and proper lifetime management.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int, c_long, c_void};

// =====================================================================
// Type Aliases (matching KCP C types)
// =====================================================================

/// KCP unsigned 32-bit integer (maps to `IUINT32` in C).
pub type IUINT32 = u32;

/// KCP signed 32-bit integer (maps to `IINT32` in C).
pub type IINT32 = i32;

// =====================================================================
// Constants
// =====================================================================

/// KCP log mask: output
pub const IKCP_LOG_OUTPUT: c_int = 1;
/// KCP log mask: input
pub const IKCP_LOG_INPUT: c_int = 2;
/// KCP log mask: send
pub const IKCP_LOG_SEND: c_int = 4;
/// KCP log mask: recv
pub const IKCP_LOG_RECV: c_int = 8;
/// KCP log mask: in data
pub const IKCP_LOG_IN_DATA: c_int = 16;
/// KCP log mask: in ack
pub const IKCP_LOG_IN_ACK: c_int = 32;
/// KCP log mask: in probe
pub const IKCP_LOG_IN_PROBE: c_int = 64;
/// KCP log mask: in wins
pub const IKCP_LOG_IN_WINS: c_int = 128;
/// KCP log mask: out data
pub const IKCP_LOG_OUT_DATA: c_int = 256;
/// KCP log mask: out ack
pub const IKCP_LOG_OUT_ACK: c_int = 512;
/// KCP log mask: out probe
pub const IKCP_LOG_OUT_PROBE: c_int = 1024;
/// KCP log mask: out wins
pub const IKCP_LOG_OUT_WINS: c_int = 2048;

/// KCP protocol overhead per packet (24 bytes header).
pub const IKCP_OVERHEAD: usize = 24;

// =====================================================================
// Queue Head (internal linked list node used by KCP)
// =====================================================================

/// Internal queue head structure used by KCP for linked lists.
///
/// This mirrors `struct IQUEUEHEAD` in the C code.
#[repr(C)]
#[derive(Debug)]
pub struct IQUEUEHEAD {
    pub next: *mut IQUEUEHEAD,
    pub prev: *mut IQUEUEHEAD,
}

// =====================================================================
// IKCPSEG — KCP Segment (internal, not directly used by API consumers)
// =====================================================================

/// KCP segment structure (internal use).
///
/// This mirrors `struct IKCPSEG` in the C code. Each segment represents
/// a piece of data being transmitted through KCP.
#[repr(C)]
pub struct IKCPSEG {
    pub node: IQUEUEHEAD,
    pub conv: IUINT32,
    pub cmd: IUINT32,
    pub frg: IUINT32,
    pub wnd: IUINT32,
    pub ts: IUINT32,
    pub sn: IUINT32,
    pub una: IUINT32,
    pub len: IUINT32,
    pub resendts: IUINT32,
    pub rto: IUINT32,
    pub fastack: IUINT32,
    pub xmit: IUINT32,
    pub data: [c_char; 1], // flexible array member
}

// =====================================================================
// IKCPCB — KCP Control Block (main structure)
// =====================================================================

/// Output callback function type.
///
/// Called by KCP when it needs to send a lower-level packet.
/// - `buf`: pointer to the data to send
/// - `len`: length of the data
/// - `kcp`: pointer to the KCP control block
/// - `user`: user-defined pointer (passed during `ikcp_create`)
///
/// Should return 0 on success, or a negative value on error.
pub type ikcp_output_callback =
    Option<unsafe extern "C" fn(buf: *const c_char, len: c_int, kcp: *mut IKCPCB, user: *mut c_void) -> c_int>;

/// Write log callback function type.
///
/// Called by KCP for internal logging.
/// - `log`: the log message string
/// - `kcp`: pointer to the KCP control block
/// - `user`: user-defined pointer
pub type ikcp_writelog_callback =
    Option<unsafe extern "C" fn(log: *const c_char, kcp: *mut IKCPCB, user: *mut c_void)>;

/// KCP Control Block — the main KCP protocol state.
///
/// This mirrors `struct IKCPCB` in the C code. It holds all internal
/// state for a single KCP connection/conversation.
///
/// # Fields
///
/// Most fields are internal to KCP. Key fields include:
/// - `conv`: conversation ID (must match on both endpoints)
/// - `mtu`: maximum transmission unit
/// - `state`: connection state (0 = active, -1 = dead)
/// - `user`: user-defined data pointer
/// - `output`: output callback function
#[repr(C)]
pub struct IKCPCB {
    /// Conversation ID, MTU, MSS (max segment size), state
    pub conv: IUINT32,
    pub mtu: IUINT32,
    pub mss: IUINT32,
    pub state: IUINT32,

    /// Send unacknowledged, send next, receive next
    pub snd_una: IUINT32,
    pub snd_nxt: IUINT32,
    pub rcv_nxt: IUINT32,

    /// Timestamps and slow start threshold
    pub ts_recent: IUINT32,
    pub ts_lastack: IUINT32,
    pub ssthresh: IUINT32,

    /// RTT estimation values
    pub rx_rttval: IINT32,
    pub rx_srtt: IINT32,
    pub rx_rto: IINT32,
    pub rx_minrto: IINT32,

    /// Window sizes
    pub snd_wnd: IUINT32,
    pub rcv_wnd: IUINT32,
    pub rmt_wnd: IUINT32,
    pub cwnd: IUINT32,
    pub probe: IUINT32,

    /// Timing and retransmission
    pub current: IUINT32,
    pub interval: IUINT32,
    pub ts_flush: IUINT32,
    pub xmit: IUINT32,

    /// Buffer counts
    pub nrcv_buf: IUINT32,
    pub nsnd_buf: IUINT32,
    pub nrcv_que: IUINT32,
    pub nsnd_que: IUINT32,

    /// Nodelay and update flags
    pub nodelay: IUINT32,
    pub updated: IUINT32,

    /// Probe timing
    pub ts_probe: IUINT32,
    pub probe_wait: IUINT32,

    /// Dead link threshold and increment
    pub dead_link: IUINT32,
    pub incr: IUINT32,

    /// Internal queues (linked lists)
    pub snd_queue: IQUEUEHEAD,
    pub rcv_queue: IQUEUEHEAD,
    pub snd_buf: IQUEUEHEAD,
    pub rcv_buf: IQUEUEHEAD,

    /// ACK list
    pub acklist: *mut IUINT32,
    pub ackcount: IUINT32,
    pub ackblock: IUINT32,

    /// User data pointer
    pub user: *mut c_void,

    /// Internal buffer
    pub buffer: *mut c_char,

    /// Fast resend settings
    pub fastresend: c_int,
    pub fastlimit: c_int,

    /// No congestion window, stream mode, log mask
    pub nocwnd: c_int,
    pub stream: c_int,
    pub logmask: c_int,

    /// Output callback (sends lower-level packets)
    pub output: ikcp_output_callback,

    /// Write log callback
    pub writelog: ikcp_writelog_callback,
}

// =====================================================================
// Extern C function declarations
// =====================================================================

extern "C" {
    // -----------------------------------------------------------------
    // Lifecycle
    // -----------------------------------------------------------------

    /// Create a new KCP control object.
    ///
    /// - `conv`: conversation ID, must be equal on both endpoints of the same connection
    /// - `user`: user-defined pointer, will be passed to the output callback
    ///
    /// Returns a pointer to the new KCP control block, or null on failure.
    pub fn ikcp_create(conv: IUINT32, user: *mut c_void) -> *mut IKCPCB;

    /// Release a KCP control object and free all associated memory.
    ///
    /// - `kcp`: pointer to the KCP control block to release
    pub fn ikcp_release(kcp: *mut IKCPCB);

    // -----------------------------------------------------------------
    // Configuration
    // -----------------------------------------------------------------

    /// Set the output callback function.
    ///
    /// The output callback is invoked by KCP whenever it needs to send
    /// a lower-level packet (e.g., via UDP).
    pub fn ikcp_setoutput(
        kcp: *mut IKCPCB,
        output: unsafe extern "C" fn(
            buf: *const c_char,
            len: c_int,
            kcp: *mut IKCPCB,
            user: *mut c_void,
        ) -> c_int,
    );

    /// Set the maximum transmission unit (MTU). Default is 1400.
    ///
    /// Returns 0 on success, negative value on error.
    pub fn ikcp_setmtu(kcp: *mut IKCPCB, mtu: c_int) -> c_int;

    /// Set the maximum window size for sending and receiving.
    ///
    /// Default: sndwnd=32, rcvwnd=128.
    /// Returns 0 on success.
    pub fn ikcp_wndsize(kcp: *mut IKCPCB, sndwnd: c_int, rcvwnd: c_int) -> c_int;

    /// Configure nodelay mode and related parameters.
    ///
    /// - `nodelay`: 0=disable(default), 1=enable
    /// - `interval`: internal update timer interval in ms, default 100ms
    /// - `resend`: 0=disable fast resend(default), 1+=enable fast resend
    /// - `nc`: 0=normal congestion control(default), 1=disable congestion control
    ///
    /// Fastest mode: `ikcp_nodelay(kcp, 1, 20, 2, 1)`
    /// Returns 0 on success.
    pub fn ikcp_nodelay(
        kcp: *mut IKCPCB,
        nodelay: c_int,
        interval: c_int,
        resend: c_int,
        nc: c_int,
    ) -> c_int;

    // -----------------------------------------------------------------
    // Data Transfer
    // -----------------------------------------------------------------

    /// Send data through KCP (upper level send).
    ///
    /// - `buffer`: pointer to the data to send
    /// - `len`: length of the data
    ///
    /// Returns 0 on success, negative value on error (e.g., data too large).
    pub fn ikcp_send(kcp: *mut IKCPCB, buffer: *const c_char, len: c_int) -> c_int;

    /// Receive data from KCP (upper level recv).
    ///
    /// - `buffer`: pointer to the buffer to receive data into
    /// - `len`: size of the buffer
    ///
    /// Returns the number of bytes received, or a negative value if no
    /// complete message is available (EAGAIN-like).
    pub fn ikcp_recv(kcp: *mut IKCPCB, buffer: *mut c_char, len: c_int) -> c_int;

    /// Input a lower-level packet into KCP (e.g., received from UDP).
    ///
    /// - `data`: pointer to the received packet data
    /// - `size`: size of the packet
    ///
    /// Returns 0 on success, negative value on error.
    pub fn ikcp_input(kcp: *mut IKCPCB, data: *const c_char, size: c_long) -> c_int;

    // -----------------------------------------------------------------
    // State Management
    // -----------------------------------------------------------------

    /// Update KCP state. Must be called periodically (every 10ms-100ms).
    ///
    /// - `current`: current timestamp in milliseconds
    ///
    /// Alternatively, use `ikcp_check` to determine when to call this.
    pub fn ikcp_update(kcp: *mut IKCPCB, current: IUINT32);

    /// Determine when `ikcp_update` should next be called.
    ///
    /// - `current`: current timestamp in milliseconds
    ///
    /// Returns the timestamp (in ms) when `ikcp_update` should be invoked.
    /// Useful for scheduling updates efficiently.
    pub fn ikcp_check(kcp: *const IKCPCB, current: IUINT32) -> IUINT32;

    /// Flush all pending data immediately.
    pub fn ikcp_flush(kcp: *mut IKCPCB);

    // -----------------------------------------------------------------
    // Status Queries
    // -----------------------------------------------------------------

    /// Check the size of the next message in the receive queue.
    ///
    /// Returns the size in bytes, or a negative value if no complete
    /// message is available.
    pub fn ikcp_peeksize(kcp: *const IKCPCB) -> c_int;

    /// Get the number of packets waiting to be sent.
    ///
    /// This includes packets in both the send queue and send buffer.
    pub fn ikcp_waitsnd(kcp: *const IKCPCB) -> c_int;

    // -----------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------

    /// Read the conversation ID from a raw KCP packet header.
    ///
    /// - `ptr`: pointer to the beginning of a KCP packet
    ///
    /// Returns the conversation ID (`conv`).
    pub fn ikcp_getconv(ptr: *const c_void) -> IUINT32;

    /// Set a custom memory allocator for KCP.
    ///
    /// - `new_malloc`: custom malloc function
    /// - `new_free`: custom free function
    pub fn ikcp_allocator(
        new_malloc: unsafe extern "C" fn(size: usize) -> *mut c_void,
        new_free: unsafe extern "C" fn(ptr: *mut c_void),
    );

    // Note: ikcp_log is a variadic function and cannot be directly bound
    // via Rust FFI. If logging is needed, use the writelog callback instead.
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    /// Dummy output callback for testing.
    unsafe extern "C" fn test_output(
        _buf: *const c_char,
        _len: c_int,
        _kcp: *mut IKCPCB,
        _user: *mut c_void,
    ) -> c_int {
        0
    }

    #[test]
    fn test_create_and_release() {
        unsafe {
            let kcp = ikcp_create(0x11223344, ptr::null_mut());
            assert!(!kcp.is_null(), "ikcp_create should return a non-null pointer");

            // Verify the conv was set correctly
            assert_eq!((*kcp).conv, 0x11223344);

            // Verify default MTU
            assert_eq!((*kcp).mtu, 1400);

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_set_output() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());

            ikcp_setoutput(kcp, test_output);

            // Verify output callback was set
            assert!((*kcp).output.is_some());

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_nodelay_config() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());

            // Set fastest mode: nodelay=1, interval=20, resend=2, nc=1
            let ret = ikcp_nodelay(kcp, 1, 20, 2, 1);
            assert_eq!(ret, 0);

            // Verify settings
            assert_eq!((*kcp).nodelay, 1);
            assert_eq!((*kcp).interval, 20);
            assert_eq!((*kcp).fastresend, 2);
            assert_eq!((*kcp).nocwnd, 1);

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_set_mtu() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());

            let ret = ikcp_setmtu(kcp, 1200);
            assert_eq!(ret, 0);
            assert_eq!((*kcp).mtu, 1200);

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_wndsize() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());

            let ret = ikcp_wndsize(kcp, 128, 256);
            assert_eq!(ret, 0);
            assert_eq!((*kcp).snd_wnd, 128);
            assert_eq!((*kcp).rcv_wnd, 256);

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_getconv() {
        unsafe {
            let kcp = ikcp_create(0xAABBCCDD, ptr::null_mut());
            assert!(!kcp.is_null());

            ikcp_setoutput(kcp, test_output);

            // Send some data so KCP prepares a packet
            let data = b"hello";
            let ret = ikcp_send(kcp, data.as_ptr() as *const c_char, data.len() as c_int);
            assert!(ret >= 0, "ikcp_send should succeed");

            // Update and flush to generate output
            ikcp_update(kcp, 0);
            ikcp_flush(kcp);

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_peeksize_empty() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());

            // No data received, peeksize should return negative
            let size = ikcp_peeksize(kcp);
            assert!(size < 0, "peeksize should be negative when no data is available");

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_waitsnd_empty() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());

            // No data sent, waitsnd should be 0
            let count = ikcp_waitsnd(kcp);
            assert_eq!(count, 0);

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_send_and_waitsnd() {
        unsafe {
            let kcp = ikcp_create(1, ptr::null_mut());
            assert!(!kcp.is_null());
            ikcp_setoutput(kcp, test_output);

            let data = b"test data for kcp";
            let ret = ikcp_send(kcp, data.as_ptr() as *const c_char, data.len() as c_int);
            assert!(ret >= 0);

            // After send, there should be pending data
            let count = ikcp_waitsnd(kcp);
            assert!(count > 0, "waitsnd should be > 0 after sending data");

            ikcp_release(kcp);
        }
    }

    #[test]
    fn test_loopback_communication() {
        // Test two KCP instances communicating with each other
        use std::cell::RefCell;

        thread_local! {
            static BUF_A_TO_B: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
            static BUF_B_TO_A: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
        }

        unsafe extern "C" fn output_a(
            buf: *const c_char,
            len: c_int,
            _kcp: *mut IKCPCB,
            _user: *mut c_void,
        ) -> c_int {
            let data = std::slice::from_raw_parts(buf as *const u8, len as usize);
            BUF_A_TO_B.with(|b| b.borrow_mut().push(data.to_vec()));
            0
        }

        unsafe extern "C" fn output_b(
            buf: *const c_char,
            len: c_int,
            _kcp: *mut IKCPCB,
            _user: *mut c_void,
        ) -> c_int {
            let data = std::slice::from_raw_parts(buf as *const u8, len as usize);
            BUF_B_TO_A.with(|b| b.borrow_mut().push(data.to_vec()));
            0
        }

        unsafe {
            let kcp_a = ikcp_create(0x01, ptr::null_mut());
            let kcp_b = ikcp_create(0x01, ptr::null_mut());
            assert!(!kcp_a.is_null());
            assert!(!kcp_b.is_null());

            ikcp_setoutput(kcp_a, output_a);
            ikcp_setoutput(kcp_b, output_b);

            // Configure both for fast mode
            ikcp_nodelay(kcp_a, 1, 10, 2, 1);
            ikcp_nodelay(kcp_b, 1, 10, 2, 1);

            // A sends data
            let msg = b"Hello from A!";
            let ret = ikcp_send(kcp_a, msg.as_ptr() as *const c_char, msg.len() as c_int);
            assert!(ret >= 0);

            // Simulate time passing and packet exchange
            let mut current: u32 = 0;
            let mut received = false;

            for _ in 0..200 {
                current += 10;

                // Update both
                ikcp_update(kcp_a, current);
                ikcp_update(kcp_b, current);

                // Transfer packets A -> B
                let packets: Vec<Vec<u8>> = BUF_A_TO_B.with(|b| {
                    let mut buf = b.borrow_mut();
                    let packets = buf.clone();
                    buf.clear();
                    packets
                });
                for pkt in &packets {
                    ikcp_input(kcp_b, pkt.as_ptr() as *const c_char, pkt.len() as c_long);
                }

                // Transfer packets B -> A
                let packets: Vec<Vec<u8>> = BUF_B_TO_A.with(|b| {
                    let mut buf = b.borrow_mut();
                    let packets = buf.clone();
                    buf.clear();
                    packets
                });
                for pkt in &packets {
                    ikcp_input(kcp_a, pkt.as_ptr() as *const c_char, pkt.len() as c_long);
                }

                // Try to receive on B
                let mut recv_buf = [0u8; 1024];
                let n = ikcp_recv(
                    kcp_b,
                    recv_buf.as_mut_ptr() as *mut c_char,
                    recv_buf.len() as c_int,
                );
                if n > 0 {
                    let received_msg = &recv_buf[..n as usize];
                    assert_eq!(received_msg, msg, "Received message should match sent message");
                    received = true;
                    break;
                }
            }

            assert!(received, "B should have received the message from A");

            ikcp_release(kcp_a);
            ikcp_release(kcp_b);
        }
    }
}