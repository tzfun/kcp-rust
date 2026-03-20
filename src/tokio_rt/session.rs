//! KCP session management.
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use crate::core::Kcp;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};
use tokio::time;
use super::config::KcpSessionConfig;
use super::error::{KcpTokioError, KcpTokioResult};

enum RecvMode { Socket, Channel(mpsc::Receiver<Vec<u8>>) }

pub struct KcpSession {
    kcp: Kcp, socket: Arc<UdpSocket>, remote_addr: SocketAddr,
    config: KcpSessionConfig, udp_recv_buf: Vec<u8>,
    start_time: Instant, last_recv_time: Instant,
    closed: bool, recv_mode: RecvMode,
}

impl KcpSession {
    pub fn new(conv: u32, socket: Arc<UdpSocket>, remote_addr: SocketAddr, config: KcpSessionConfig) -> KcpTokioResult<Self> {
        Self::new_inner(conv, socket, remote_addr, config, RecvMode::Socket)
    }

    pub(crate) fn new_with_channel(conv: u32, socket: Arc<UdpSocket>, remote_addr: SocketAddr, config: KcpSessionConfig, pkt_rx: mpsc::Receiver<Vec<u8>>) -> KcpTokioResult<Self> {
        Self::new_inner(conv, socket, remote_addr, config, RecvMode::Channel(pkt_rx))
    }

    fn new_inner(conv: u32, socket: Arc<UdpSocket>, remote_addr: SocketAddr, config: KcpSessionConfig, recv_mode: RecvMode) -> KcpTokioResult<Self> {
        let socket_clone = socket.clone();
        let remote = remote_addr;
        let kcp = Kcp::with_config(conv, &config.kcp_config, move |data: &[u8]| -> io::Result<usize> {
            match socket_clone.try_send_to(data, remote) {
                Ok(n) => Ok(n),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(data.len()),
                Err(e) => Err(e),
            }
        })?;
        let now = Instant::now();
        Ok(Self { kcp, socket, remote_addr, udp_recv_buf: vec![0u8; config.recv_buf_size], config, start_time: now, last_recv_time: now, closed: false, recv_mode })
    }

    fn current_ms(&self) -> u32 { self.start_time.elapsed().as_millis() as u32 }

    pub fn send(&mut self, data: &[u8]) -> KcpTokioResult<usize> {
        if self.closed { return Err(KcpTokioError::Closed); }
        let n = self.kcp.send(data)?;
        if self.config.flush_write { self.kcp.flush(); }
        Ok(n)
    }

    pub fn try_recv(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> {
        if self.closed { return Err(KcpTokioError::Closed); }
        Ok(self.kcp.recv(buf)?)
    }

    pub fn input(&mut self, data: &[u8]) -> KcpTokioResult<()> {
        self.last_recv_time = Instant::now();
        self.kcp.input(data)?;
        Ok(())
    }

    pub fn update(&mut self) { let current = self.current_ms(); self.kcp.update(current); }
    pub fn is_timed_out(&self) -> bool { self.config.timeout.map_or(false, |t| self.last_recv_time.elapsed() > t) }
    #[allow(dead_code)]
    pub fn is_closed(&self) -> bool { self.closed }
    pub fn close(&mut self) { self.closed = true; }
    pub fn conv(&self) -> u32 { self.kcp.conv() }
    pub fn remote_addr(&self) -> SocketAddr { self.remote_addr }
    #[allow(dead_code)]
    pub fn waitsnd(&self) -> u32 { self.kcp.waitsnd() }
    pub fn socket(&self) -> &Arc<UdpSocket> { &self.socket }
    pub fn config(&self) -> &KcpSessionConfig { &self.config }

    pub async fn recv(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> {
        if self.closed { return Err(KcpTokioError::Closed); }
        loop {
            match self.kcp.recv(buf) {
                Ok(n) => return Ok(n),
                Err(crate::core::KcpError::RecvWouldBlock) => {}
                Err(e) => return Err(e.into()),
            }
            if self.is_timed_out() { self.closed = true; return Err(KcpTokioError::Timeout); }
            let flush_interval = self.config.flush_interval;
            match &mut self.recv_mode {
                RecvMode::Socket => {
                    tokio::select! {
                        result = self.socket.recv_from(&mut self.udp_recv_buf) => {
                            let (n, addr) = result?;
                            if addr == self.remote_addr { self.last_recv_time = Instant::now(); self.kcp.input(&self.udp_recv_buf[..n]).ok(); }
                        }
                        _ = time::sleep(flush_interval) => {}
                    }
                }
                RecvMode::Channel(rx) => {
                    tokio::select! {
                        pkt = rx.recv() => {
                            match pkt {
                                Some(data) => { self.last_recv_time = Instant::now(); self.kcp.input(&data).ok(); }
                                None => { self.closed = true; return Err(KcpTokioError::Closed); }
                            }
                        }
                        _ = time::sleep(flush_interval) => {}
                    }
                }
            }
            self.update();
        }
    }
}

/// A KCP session wrapped in `Arc<Mutex>` for shared access.
#[allow(dead_code)]
pub type SharedKcpSession = Arc<Mutex<KcpSession>>;
