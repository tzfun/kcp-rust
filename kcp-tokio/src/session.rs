//! KCP session management.
//!
//! This module provides [`KcpSession`], which manages the interaction between
//! a KCP instance and a UDP socket. It handles:
//! - Periodic KCP updates via a timer
//! - Routing output from KCP to the UDP socket
//! - Processing incoming UDP data through KCP
//!
//! `KcpSession` is the foundation for [`KcpStream`](crate::stream::KcpStream).

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use kcp_core::Kcp;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};
use tokio::time;

use crate::config::KcpSessionConfig;
use crate::error::{KcpTokioError, KcpTokioResult};

/// How the session receives incoming UDP data.
enum RecvMode {
    /// Directly from a UDP socket (client mode).
    Socket,
    /// From an mpsc channel (server mode — packets forwarded by listener).
    Channel(mpsc::Receiver<Vec<u8>>),
}

/// A KCP session that manages the underlying KCP instance and UDP socket.
///
/// The session coordinates:
/// - Sending data via `kcp.send()` → output callback → UDP
/// - Receiving data via UDP → `kcp.input()` → `kcp.recv()`
/// - Periodic `kcp.update()` calls
pub struct KcpSession {
    /// The KCP instance.
    kcp: Kcp,
    /// The UDP socket (used for sending and optionally receiving).
    socket: Arc<UdpSocket>,
    /// Remote peer address.
    remote_addr: SocketAddr,
    /// Session configuration.
    config: KcpSessionConfig,
    /// Internal buffer for receiving UDP packets.
    udp_recv_buf: Vec<u8>,
    /// Start time for KCP timestamp calculation.
    start_time: Instant,
    /// Last time data was received (for timeout detection).
    last_recv_time: Instant,
    /// Whether the session is closed.
    closed: bool,
    /// How to receive incoming data.
    recv_mode: RecvMode,
}

impl KcpSession {
    /// Creates a new KCP session (client mode — receives directly from socket).
    pub fn new(
        conv: u32,
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        config: KcpSessionConfig,
    ) -> KcpTokioResult<Self> {
        Self::new_inner(conv, socket, remote_addr, config, RecvMode::Socket)
    }

    /// Creates a new KCP session (server mode — receives from channel).
    pub(crate) fn new_with_channel(
        conv: u32,
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        config: KcpSessionConfig,
        pkt_rx: mpsc::Receiver<Vec<u8>>,
    ) -> KcpTokioResult<Self> {
        Self::new_inner(conv, socket, remote_addr, config, RecvMode::Channel(pkt_rx))
    }

    fn new_inner(
        conv: u32,
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        config: KcpSessionConfig,
        recv_mode: RecvMode,
    ) -> KcpTokioResult<Self> {
        let socket_clone = socket.clone();
        let remote = remote_addr;

        let kcp = Kcp::with_config(
            conv,
            &config.kcp_config,
            move |data: &[u8]| -> io::Result<usize> {
                match socket_clone.try_send_to(data, remote) {
                    Ok(n) => Ok(n),
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(data.len()),
                    Err(e) => Err(e),
                }
            },
        )?;

        let now = Instant::now();

        Ok(Self {
            kcp,
            socket,
            remote_addr,
            udp_recv_buf: vec![0u8; config.recv_buf_size],
            config,
            start_time: now,
            last_recv_time: now,
            closed: false,
            recv_mode,
        })
    }

    /// Get the current KCP timestamp in milliseconds.
    fn current_ms(&self) -> u32 {
        self.start_time.elapsed().as_millis() as u32
    }

    /// Send data through the KCP session.
    pub fn send(&mut self, data: &[u8]) -> KcpTokioResult<usize> {
        if self.closed {
            return Err(KcpTokioError::Closed);
        }
        let n = self.kcp.send(data)?;
        if self.config.flush_write {
            self.kcp.flush();
        }
        Ok(n)
    }

    /// Try to receive data from the KCP session (non-blocking).
    pub fn try_recv(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> {
        if self.closed {
            return Err(KcpTokioError::Closed);
        }
        Ok(self.kcp.recv(buf)?)
    }

    /// Process a received UDP packet through KCP.
    pub fn input(&mut self, data: &[u8]) -> KcpTokioResult<()> {
        self.last_recv_time = Instant::now();
        self.kcp.input(data)?;
        Ok(())
    }

    /// Update the KCP state. Should be called periodically.
    pub fn update(&mut self) {
        let current = self.current_ms();
        self.kcp.update(current);
    }

    /// Check if the session has timed out.
    pub fn is_timed_out(&self) -> bool {
        if let Some(timeout) = self.config.timeout {
            self.last_recv_time.elapsed() > timeout
        } else {
            false
        }
    }

    /// Check if the session is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Close the session.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// Get the conversation ID.
    pub fn conv(&self) -> u32 {
        self.kcp.conv()
    }

    /// Get the remote address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Get how many packets are waiting to be sent.
    pub fn waitsnd(&self) -> u32 {
        self.kcp.waitsnd()
    }

    /// Get a reference to the underlying UDP socket.
    pub fn socket(&self) -> &Arc<UdpSocket> {
        &self.socket
    }

    /// Get the session configuration.
    pub fn config(&self) -> &KcpSessionConfig {
        &self.config
    }

    /// Async receive: waits for data, processing UDP/channel packets and KCP updates.
    pub async fn recv(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> {
        if self.closed {
            return Err(KcpTokioError::Closed);
        }

        loop {
            // Check if KCP has assembled data ready
            match self.kcp.recv(buf) {
                Ok(n) => return Ok(n),
                Err(kcp_core::KcpError::RecvWouldBlock) => {}
                Err(e) => return Err(e.into()),
            }

            if self.is_timed_out() {
                self.closed = true;
                return Err(KcpTokioError::Timeout);
            }

            let flush_interval = self.config.flush_interval;

            match &mut self.recv_mode {
                RecvMode::Socket => {
                    // Client mode: recv directly from UDP socket
                    tokio::select! {
                        result = self.socket.recv_from(&mut self.udp_recv_buf) => {
                            let (n, addr) = result?;
                            if addr == self.remote_addr {
                                self.last_recv_time = Instant::now();
                                self.kcp.input(&self.udp_recv_buf[..n]).ok();
                            }
                        }
                        _ = time::sleep(flush_interval) => {}
                    }
                }
                RecvMode::Channel(rx) => {
                    // Server mode: recv from channel (packets forwarded by listener)
                    tokio::select! {
                        pkt = rx.recv() => {
                            match pkt {
                                Some(data) => {
                                    self.last_recv_time = Instant::now();
                                    self.kcp.input(&data).ok();
                                }
                                None => {
                                    self.closed = true;
                                    return Err(KcpTokioError::Closed);
                                }
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
pub type SharedKcpSession = Arc<Mutex<KcpSession>>;
