//! Raw FFI bindings to the KCP C library.
//!
//! This module provides unsafe bindings to the original KCP implementation
//! by skywind3000 (<https://github.com/skywind3000/kcp>).
//!
//! It compiles the C source code via `build.rs` and exposes the raw C API
//! as Rust FFI declarations. For a safe Rust API, enable the `kcp-core` feature.
//!
//! # Safety
//!
//! All functions in this module are `unsafe` as they directly call C code.
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
#[repr(C)]
#[derive(Debug)]
pub struct IQUEUEHEAD {
    pub next: *mut IQUEUEHEAD,
    pub prev: *mut IQUEUEHEAD,
}

// =====================================================================
// IKCPSEG — KCP Segment (internal)
// =====================================================================

/// KCP segment structure (internal use).
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
    pub data: [c_char; 1],
}

// =====================================================================
// IKCPCB — KCP Control Block (main structure)
// =====================================================================

/// Output callback function type.
pub type ikcp_output_callback = Option<
    unsafe extern "C" fn(
        buf: *const c_char,
        len: c_int,
        kcp: *mut IKCPCB,
        user: *mut c_void,
    ) -> c_int,
>;

/// Write log callback function type.
pub type ikcp_writelog_callback =
    Option<unsafe extern "C" fn(log: *const c_char, kcp: *mut IKCPCB, user: *mut c_void)>;

/// KCP Control Block — the main KCP protocol state.
#[repr(C)]
pub struct IKCPCB {
    pub conv: IUINT32,
    pub mtu: IUINT32,
    pub mss: IUINT32,
    pub state: IUINT32,

    pub snd_una: IUINT32,
    pub snd_nxt: IUINT32,
    pub rcv_nxt: IUINT32,

    pub ts_recent: IUINT32,
    pub ts_lastack: IUINT32,
    pub ssthresh: IUINT32,

    pub rx_rttval: IINT32,
    pub rx_srtt: IINT32,
    pub rx_rto: IINT32,
    pub rx_minrto: IINT32,

    pub snd_wnd: IUINT32,
    pub rcv_wnd: IUINT32,
    pub rmt_wnd: IUINT32,
    pub cwnd: IUINT32,
    pub probe: IUINT32,

    pub current: IUINT32,
    pub interval: IUINT32,
    pub ts_flush: IUINT32,
    pub xmit: IUINT32,

    pub nrcv_buf: IUINT32,
    pub nsnd_buf: IUINT32,
    pub nrcv_que: IUINT32,
    pub nsnd_que: IUINT32,

    pub nodelay: IUINT32,
    pub updated: IUINT32,

    pub ts_probe: IUINT32,
    pub probe_wait: IUINT32,

    pub dead_link: IUINT32,
    pub incr: IUINT32,

    pub snd_queue: IQUEUEHEAD,
    pub rcv_queue: IQUEUEHEAD,
    pub snd_buf: IQUEUEHEAD,
    pub rcv_buf: IQUEUEHEAD,

    pub acklist: *mut IUINT32,
    pub ackcount: IUINT32,
    pub ackblock: IUINT32,

    pub user: *mut c_void,
    pub buffer: *mut c_char,

    pub fastresend: c_int,
    pub fastlimit: c_int,

    pub nocwnd: c_int,
    pub stream: c_int,
    pub logmask: c_int,

    pub output: ikcp_output_callback,
    pub writelog: ikcp_writelog_callback,
}

// =====================================================================
// Extern C function declarations
// =====================================================================

extern "C" {
    pub fn ikcp_create(conv: IUINT32, user: *mut c_void) -> *mut IKCPCB;
    pub fn ikcp_release(kcp: *mut IKCPCB);
    pub fn ikcp_setoutput(
        kcp: *mut IKCPCB,
        output: unsafe extern "C" fn(
            buf: *const c_char,
            len: c_int,
            kcp: *mut IKCPCB,
            user: *mut c_void,
        ) -> c_int,
    );
    pub fn ikcp_setmtu(kcp: *mut IKCPCB, mtu: c_int) -> c_int;
    pub fn ikcp_wndsize(kcp: *mut IKCPCB, sndwnd: c_int, rcvwnd: c_int) -> c_int;
    pub fn ikcp_nodelay(
        kcp: *mut IKCPCB,
        nodelay: c_int,
        interval: c_int,
        resend: c_int,
        nc: c_int,
    ) -> c_int;
    pub fn ikcp_send(kcp: *mut IKCPCB, buffer: *const c_char, len: c_int) -> c_int;
    pub fn ikcp_recv(kcp: *mut IKCPCB, buffer: *mut c_char, len: c_int) -> c_int;
    pub fn ikcp_input(kcp: *mut IKCPCB, data: *const c_char, size: c_long) -> c_int;
    pub fn ikcp_update(kcp: *mut IKCPCB, current: IUINT32);
    pub fn ikcp_check(kcp: *const IKCPCB, current: IUINT32) -> IUINT32;
    pub fn ikcp_flush(kcp: *mut IKCPCB);
    pub fn ikcp_peeksize(kcp: *const IKCPCB) -> c_int;
    pub fn ikcp_waitsnd(kcp: *const IKCPCB) -> c_int;
    pub fn ikcp_getconv(ptr: *const c_void) -> IUINT32;
    pub fn ikcp_allocator(
        new_malloc: unsafe extern "C" fn(size: usize) -> *mut c_void,
        new_free: unsafe extern "C" fn(ptr: *mut c_void),
    );
}
