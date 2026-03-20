//! Async KCP stream with `AsyncRead`/`AsyncWrite` support.
//!
//! This module provides [`KcpStream`], the primary user-facing type for async
//! KCP communication. It wraps a [`KcpSession`](super::session::KcpSession) and
//! provides both high-level methods ([`send_kcp`](KcpStream::send_kcp),
//! [`recv_kcp`](KcpStream::recv_kcp)) and Tokio trait implementations
//! ([`AsyncRead`], [`AsyncWrite`]).
//!
//! # Example
//!
//! ```no_run
//! use kcp_io::tokio_rt::{KcpStream, KcpSessionConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut stream = KcpStream::connect("127.0.0.1:9090", KcpSessionConfig::fast()).await?;
//! stream.send_kcp(b"hello").await?;
//!
//! let mut buf = [0u8; 1024];
//! let n = stream.recv_kcp(&mut buf).await?;
//! println!("Received: {:?}", &buf[..n]);
//! # Ok(())
//! # }
//! ```

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::UdpSocket;
use super::config::KcpSessionConfig;
use super::error::{KcpTokioError, KcpTokioResult};
use super::session::KcpSession;

/// An async KCP stream for reliable UDP communication.
///
/// `KcpStream` is analogous to `TcpStream` but uses the KCP protocol over UDP.
/// It implements [`AsyncRead`] and [`AsyncWrite`] for compatibility with the
/// Tokio ecosystem (e.g., `tokio::io::copy`, `BufReader`, etc.).
///
/// # Connection
///
/// - **Client**: Use [`connect()`](KcpStream::connect) or
///   [`connect_with_conv()`](KcpStream::connect_with_conv) to connect to a server.
/// - **Server**: Obtained from [`KcpListener::accept()`](super::KcpListener::accept).
///
/// # Sending and Receiving
///
/// - [`send_kcp()`](KcpStream::send_kcp) / [`recv_kcp()`](KcpStream::recv_kcp) —
///   High-level KCP-aware methods.
/// - [`AsyncRead`] / [`AsyncWrite`] — Standard Tokio trait implementations for
///   interoperability with the async I/O ecosystem.
pub struct KcpStream {
    /// The underlying KCP session.
    session: KcpSession,
    /// Internal read buffer for `AsyncRead` implementation.
    read_buf: Vec<u8>,
    /// Current read position in the internal buffer.
    read_pos: usize,
    /// Number of valid bytes in the internal buffer.
    read_len: usize,
}

impl KcpStream {
    /// Connects to a remote KCP server with an auto-generated conversation ID.
    ///
    /// Binds a new UDP socket to an ephemeral local port and creates a KCP
    /// session to the specified remote address.
    ///
    /// # Arguments
    ///
    /// * `addr` — Remote server address (e.g., `"127.0.0.1:9090"`).
    /// * `config` — Session configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if address resolution, socket binding, or session creation fails.
    pub async fn connect<A: tokio::net::ToSocketAddrs>(addr: A, config: KcpSessionConfig) -> KcpTokioResult<Self> {
        Self::connect_with_conv(addr, config, rand_conv()).await
    }

    /// Connects to a remote KCP server with a specific conversation ID.
    ///
    /// Both endpoints must use the same `conv` for the session to work.
    ///
    /// # Arguments
    ///
    /// * `addr` — Remote server address.
    /// * `config` — Session configuration.
    /// * `conv` — KCP conversation ID.
    pub async fn connect_with_conv<A: tokio::net::ToSocketAddrs>(addr: A, config: KcpSessionConfig, conv: u32) -> KcpTokioResult<Self> {
        let remote_addr = tokio::net::lookup_host(addr).await?.next()
            .ok_or_else(|| KcpTokioError::ConnectionFailed("could not resolve address".to_string()))?;
        let local_addr = if remote_addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let socket = Arc::new(UdpSocket::bind(local_addr).await?);
        let recv_buf_size = config.recv_buf_size;
        let session = KcpSession::new(conv, socket, remote_addr, config)?;
        Ok(Self { session, read_buf: vec![0u8; recv_buf_size], read_pos: 0, read_len: 0 })
    }

    /// Creates a `KcpStream` from an existing [`KcpSession`].
    ///
    /// Used internally by [`KcpListener`](super::KcpListener) to wrap accepted sessions.
    pub(crate) fn from_session(session: KcpSession) -> Self {
        let recv_buf_size = session.config().recv_buf_size;
        Self { session, read_buf: vec![0u8; recv_buf_size], read_pos: 0, read_len: 0 }
    }

    /// Returns the KCP conversation ID for this stream.
    pub fn conv(&self) -> u32 { self.session.conv() }

    /// Returns the remote peer's address.
    pub fn remote_addr(&self) -> SocketAddr { self.session.remote_addr() }

    /// Returns the local socket address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> { self.session.socket().local_addr() }

    /// Sends data reliably through the KCP protocol.
    ///
    /// The data is queued in the KCP engine and transmitted via the output callback.
    /// If `flush_write` is enabled in the config, data is flushed immediately.
    pub async fn send_kcp(&mut self, data: &[u8]) -> KcpTokioResult<usize> { self.session.send(data) }

    /// Receives data from the KCP protocol asynchronously.
    ///
    /// Blocks until data is available, the session times out, or the session is closed.
    pub async fn recv_kcp(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> { self.session.recv(buf).await }
}

impl AsyncRead for KcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Return buffered data first
        if this.read_pos < this.read_len {
            let remaining = &this.read_buf[this.read_pos..this.read_len];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            this.read_pos += to_copy;
            return Poll::Ready(Ok(()));
        }
        // Reset buffer positions
        this.read_pos = 0; this.read_len = 0;
        // Try to receive from KCP (non-blocking)
        match this.session.try_recv(&mut this.read_buf) {
            Ok(n) => {
                let to_copy = n.min(buf.remaining());
                buf.put_slice(&this.read_buf[..to_copy]);
                if to_copy < n { this.read_pos = to_copy; this.read_len = n; }
                return Poll::Ready(Ok(()));
            }
            Err(KcpTokioError::Kcp(crate::core::KcpError::RecvWouldBlock)) => {}
            Err(KcpTokioError::Closed) => return Poll::Ready(Ok(())),
            Err(KcpTokioError::Timeout) => return Poll::Ready(Err(io::Error::new(io::ErrorKind::TimedOut, "KCP session timed out"))),
            Err(KcpTokioError::Io(e)) => return Poll::Ready(Err(e)),
            Err(e) => return Poll::Ready(Err(io::Error::other(e.to_string()))),
        }
        // Poll the UDP socket for new data
        let socket = this.session.socket().clone();
        let mut udp_buf = [0u8; 65536];
        let mut read_buf = ReadBuf::new(&mut udp_buf);
        match socket.poll_recv_from(cx, &mut read_buf) {
            Poll::Ready(Ok(addr)) => {
                let n = read_buf.filled().len();
                if addr == this.session.remote_addr() { this.session.input(&udp_buf[..n]).ok(); }
                this.session.update();
                match this.session.try_recv(&mut this.read_buf) {
                    Ok(n) => {
                        let to_copy = n.min(buf.remaining());
                        buf.put_slice(&this.read_buf[..to_copy]);
                        if to_copy < n { this.read_pos = to_copy; this.read_len = n; }
                        Poll::Ready(Ok(()))
                    }
                    Err(KcpTokioError::Kcp(crate::core::KcpError::RecvWouldBlock)) => { cx.waker().wake_by_ref(); Poll::Pending }
                    Err(KcpTokioError::Closed) => Poll::Ready(Ok(())),
                    Err(e) => Poll::Ready(Err(io::Error::other(e.to_string()))),
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => { this.session.update(); cx.waker().wake_by_ref(); Poll::Pending }
        }
    }
}

impl AsyncWrite for KcpStream {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.session.send(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(KcpTokioError::Closed) => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "KCP session closed"))),
            Err(KcpTokioError::Io(e)) => Poll::Ready(Err(e)),
            Err(e) => Poll::Ready(Err(io::Error::other(e.to_string()))),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { self.session.update(); Poll::Ready(Ok(())) }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> { self.session.close(); Poll::Ready(Ok(())) }
}

/// Generates a pseudo-random conversation ID based on the current system time.
fn rand_conv() -> u32 {
    use std::time::SystemTime;
    let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos();
    ((seed ^ (seed >> 16) ^ (seed >> 32)) & 0xFFFFFFFF) as u32
}