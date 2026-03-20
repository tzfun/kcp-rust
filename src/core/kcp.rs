//! Core KCP control object wrapper.
use std::io;
use std::os::raw::{c_char, c_int, c_long, c_void};
use super::config::KcpConfig;
use super::error::{KcpError, KcpResult};
use crate::sys::IKCPCB;

type OutputCallback = Box<dyn FnMut(&[u8]) -> io::Result<usize>>;

struct KcpContext {
    output: OutputCallback,
}

unsafe extern "C" fn kcp_output_cb(
    buf: *const c_char, len: c_int, _kcp: *mut IKCPCB, user: *mut c_void,
) -> c_int {
    if user.is_null() || buf.is_null() || len <= 0 { return -1; }
    let ctx = &mut *(user as *mut KcpContext);
    let data = std::slice::from_raw_parts(buf as *const u8, len as usize);
    match (ctx.output)(data) { Ok(_) => 0, Err(_) => -1 }
}

/// Safe wrapper around the KCP control block.
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
    kcp: *mut IKCPCB,
    _ctx: *mut KcpContext,
}

unsafe impl Send for Kcp {}

impl Kcp {
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

    pub fn with_config<F>(conv: u32, config: &KcpConfig, output: F) -> KcpResult<Self>
    where F: FnMut(&[u8]) -> io::Result<usize> + 'static {
        let mut kcp = Self::new(conv, output)?;
        kcp.apply_config(config)?;
        Ok(kcp)
    }

    pub fn apply_config(&mut self, config: &KcpConfig) -> KcpResult<()> {
        self.set_nodelay(config.nodelay, config.interval, config.resend, config.nc);
        self.set_mtu(config.mtu)?;
        self.set_wndsize(config.snd_wnd, config.rcv_wnd);
        self.set_stream_mode(config.stream_mode);
        Ok(())
    }

    pub fn send(&mut self, buf: &[u8]) -> KcpResult<usize> {
        let ret = unsafe { crate::sys::ikcp_send(self.kcp, buf.as_ptr() as *const c_char, buf.len() as c_int) };
        if ret < 0 { Err(KcpError::SendFailed(ret)) } else { Ok(buf.len()) }
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> KcpResult<usize> {
        let ret = unsafe { crate::sys::ikcp_recv(self.kcp, buf.as_mut_ptr() as *mut c_char, buf.len() as c_int) };
        if ret >= 0 { Ok(ret as usize) }
        else if ret == -1 { Err(KcpError::RecvWouldBlock) }
        else if ret == -2 { let need = self.peeksize().unwrap_or(0); Err(KcpError::RecvBufferTooSmall { need, got: buf.len() }) }
        else { Err(KcpError::RecvFailed(ret)) }
    }

    pub fn input(&mut self, data: &[u8]) -> KcpResult<()> {
        let ret = unsafe { crate::sys::ikcp_input(self.kcp, data.as_ptr() as *const c_char, data.len() as c_long) };
        if ret < 0 { Err(KcpError::InputFailed(ret)) } else { Ok(()) }
    }

    pub fn update(&mut self, current: u32) { unsafe { crate::sys::ikcp_update(self.kcp, current); } }
    pub fn check(&self, current: u32) -> u32 { unsafe { crate::sys::ikcp_check(self.kcp, current) } }
    pub fn flush(&mut self) { unsafe { crate::sys::ikcp_flush(self.kcp); } }

    pub fn peeksize(&self) -> KcpResult<usize> {
        let ret = unsafe { crate::sys::ikcp_peeksize(self.kcp) };
        if ret < 0 { Err(KcpError::RecvWouldBlock) } else { Ok(ret as usize) }
    }

    pub fn waitsnd(&self) -> u32 { unsafe { crate::sys::ikcp_waitsnd(self.kcp) as u32 } }
    pub fn conv(&self) -> u32 { unsafe { (*self.kcp).conv } }

    pub fn set_mtu(&mut self, mtu: u32) -> KcpResult<()> {
        let ret = unsafe { crate::sys::ikcp_setmtu(self.kcp, mtu as c_int) };
        if ret < 0 { Err(KcpError::SetMtuFailed { mtu, code: ret }) } else { Ok(()) }
    }

    pub fn set_wndsize(&mut self, snd_wnd: u32, rcv_wnd: u32) {
        unsafe { crate::sys::ikcp_wndsize(self.kcp, snd_wnd as c_int, rcv_wnd as c_int); }
    }

    pub fn set_nodelay(&mut self, nodelay: bool, interval: u32, resend: i32, nc: bool) {
        unsafe { crate::sys::ikcp_nodelay(self.kcp, nodelay as c_int, interval as c_int, resend as c_int, nc as c_int); }
    }

    pub fn set_stream_mode(&mut self, enabled: bool) { unsafe { (*self.kcp).stream = enabled as c_int; } }
    pub fn is_stream_mode(&self) -> bool { unsafe { (*self.kcp).stream != 0 } }

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
