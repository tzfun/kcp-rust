//! Async KCP stream with AsyncRead/AsyncWrite support.
//!
//! This module provides [`KcpStream`], which wraps a KCP session and
//! implements `tokio::io::AsyncRead` and `tokio::io::AsyncWrite`.
//!
//! # Example
//!
//! ```no_run
//! use kcp_tokio::KcpStream;
//! use kcp_tokio::config::KcpSessionConfig;
//! use tokio::io::{AsyncReadExt, AsyncWriteExt};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut stream = KcpStream::connect("127.0.0.1:8080", KcpSessionConfig::fast()).await?;
//! stream.write_all(b"hello").await?;
//!
//! let mut buf = [0u8; 1024];
//! let n = stream.read(&mut buf).await?;
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

use crate::config::KcpSessionConfig;
use crate::error::{KcpTokioError, KcpTokioResult};
use crate::session::KcpSession;

/// An async KCP stream that implements `AsyncRead` and `AsyncWrite`.
///
/// `KcpStream` provides a TCP-like interface over KCP+UDP. It handles
/// the KCP protocol internally, including periodic updates and packet
/// reassembly.
///
/// # Usage
///
/// Use [`KcpStream::connect`] to create a client-side stream, or obtain
/// one from [`KcpListener::accept`](crate::listener::KcpListener::accept).
pub struct KcpStream {
    /// The underlying KCP session.
    session: KcpSession,
    /// Read buffer for partial reads.
    read_buf: Vec<u8>,
    /// Current position in the read buffer.
    read_pos: usize,
    /// Number of valid bytes in the read buffer.
    read_len: usize,
}

impl KcpStream {
    /// Connect to a remote KCP server.
    ///
    /// Creates a new UDP socket, binds it to a random local port,
    /// and establishes a KCP session with the remote address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Remote server address (e.g., "127.0.0.1:8080")
    /// * `config` - Session configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use kcp_tokio::KcpStream;
    /// use kcp_tokio::config::KcpSessionConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let stream = KcpStream::connect("127.0.0.1:8080", KcpSessionConfig::fast()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect<A: tokio::net::ToSocketAddrs>(
        addr: A,
        config: KcpSessionConfig,
    ) -> KcpTokioResult<Self> {
        Self::connect_with_conv(addr, config, rand_conv()).await
    }

    /// Connect to a remote KCP server with a specific conversation ID.
    ///
    /// # Arguments
    ///
    /// * `addr` - Remote server address
    /// * `config` - Session configuration
    /// * `conv` - Conversation ID to use
    pub async fn connect_with_conv<A: tokio::net::ToSocketAddrs>(
        addr: A,
        config: KcpSessionConfig,
        conv: u32,
    ) -> KcpTokioResult<Self> {
        // Resolve the remote address
        let remote_addr = tokio::net::lookup_host(addr)
            .await?
            .next()
            .ok_or_else(|| {
                KcpTokioError::ConnectionFailed("could not resolve address".to_string())
            })?;

        // Bind to a local address matching the remote address family
        let local_addr = if remote_addr.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };

        let socket = UdpSocket::bind(local_addr).await?;
        let socket = Arc::new(socket);

        let recv_buf_size = config.recv_buf_size;
        let session = KcpSession::new(conv, socket, remote_addr, config)?;

        Ok(Self {
            session,
            read_buf: vec![0u8; recv_buf_size],
            read_pos: 0,
            read_len: 0,
        })
    }

    /// Create a KcpStream from an existing session.
    ///
    /// This is used internally by `KcpListener` when accepting connections.
    pub(crate) fn from_session(session: KcpSession) -> Self {
        let recv_buf_size = session.config().recv_buf_size;
        Self {
            session,
            read_buf: vec![0u8; recv_buf_size],
            read_pos: 0,
            read_len: 0,
        }
    }

    /// Get the conversation ID.
    pub fn conv(&self) -> u32 {
        self.session.conv()
    }

    /// Get the remote peer address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.session.remote_addr()
    }

    /// Get the local address.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.session.socket().local_addr()
    }

    /// Send data through the KCP stream.
    ///
    /// This is a convenience method that wraps `kcp.send()`.
    pub async fn send_kcp(&mut self, data: &[u8]) -> KcpTokioResult<usize> {
        self.session.send(data)
    }

    /// Receive data from the KCP stream.
    ///
    /// Waits for data to arrive, processing UDP packets and KCP updates
    /// as needed.
    pub async fn recv_kcp(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> {
        self.session.recv(buf).await
    }
}

impl AsyncRead for KcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();

        // If we have buffered data, return it
        if this.read_pos < this.read_len {
            let remaining = &this.read_buf[this.read_pos..this.read_len];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            this.read_pos += to_copy;
            return Poll::Ready(Ok(()));
        }

        // Reset buffer positions
        this.read_pos = 0;
        this.read_len = 0;

        // First try a synchronous recv from KCP (data may already be assembled)
        match this.session.try_recv(&mut this.read_buf) {
            Ok(n) => {
                let to_copy = n.min(buf.remaining());
                buf.put_slice(&this.read_buf[..to_copy]);
                if to_copy < n {
                    this.read_pos = to_copy;
                    this.read_len = n;
                }
                return Poll::Ready(Ok(()));
            }
            Err(KcpTokioError::Kcp(kcp_core::KcpError::RecvWouldBlock)) => {
                // No data assembled yet, need to poll for UDP data
            }
            Err(KcpTokioError::Closed) => return Poll::Ready(Ok(())), // EOF
            Err(KcpTokioError::Timeout) => {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "KCP session timed out",
                )));
            }
            Err(KcpTokioError::Io(e)) => return Poll::Ready(Err(e)),
            Err(e) => {
                return Poll::Ready(Err(io::Error::other(e.to_string())));
            }
        }

        // Poll the UDP socket for new data
        let socket = this.session.socket().clone();
        let mut udp_buf = [0u8; 65536];
        let mut read_buf = ReadBuf::new(&mut udp_buf);

        match socket.poll_recv_from(cx, &mut read_buf) {
            Poll::Ready(Ok(addr)) => {
                let n = read_buf.filled().len();
                if addr == this.session.remote_addr() {
                    this.session.input(&udp_buf[..n]).ok();
                }
                // Update KCP state
                this.session.update();

                // Try to recv again after processing
                match this.session.try_recv(&mut this.read_buf) {
                    Ok(n) => {
                        let to_copy = n.min(buf.remaining());
                        buf.put_slice(&this.read_buf[..to_copy]);
                        if to_copy < n {
                            this.read_pos = to_copy;
                            this.read_len = n;
                        }
                        Poll::Ready(Ok(()))
                    }
                    Err(KcpTokioError::Kcp(kcp_core::KcpError::RecvWouldBlock)) => {
                        // Still no complete message, schedule wakeup
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Err(KcpTokioError::Closed) => Poll::Ready(Ok(())),
                    Err(e) => Poll::Ready(Err(io::Error::other(e.to_string()))),
                }
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => {
                // No UDP data available, update KCP state anyway
                this.session.update();
                // Schedule a wakeup so we can try again
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

impl AsyncWrite for KcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.session.send(buf) {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(KcpTokioError::Closed) => {
                Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "KCP session closed")))
            }
            Err(KcpTokioError::Io(e)) => Poll::Ready(Err(e)),
            Err(e) => {
                Poll::Ready(Err(io::Error::other(e.to_string())))
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.session.update();
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.session.close();
        Poll::Ready(Ok(()))
    }
}

/// Generate a random conversation ID.
fn rand_conv() -> u32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Simple hash to generate a pseudo-random conv
    ((seed ^ (seed >> 16) ^ (seed >> 32)) & 0xFFFFFFFF) as u32
}