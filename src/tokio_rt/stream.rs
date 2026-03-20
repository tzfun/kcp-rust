//! Async KCP stream with AsyncRead/AsyncWrite support.
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

pub struct KcpStream {
    session: KcpSession, read_buf: Vec<u8>, read_pos: usize, read_len: usize,
}

impl KcpStream {
    pub async fn connect<A: tokio::net::ToSocketAddrs>(addr: A, config: KcpSessionConfig) -> KcpTokioResult<Self> {
        Self::connect_with_conv(addr, config, rand_conv()).await
    }

    pub async fn connect_with_conv<A: tokio::net::ToSocketAddrs>(addr: A, config: KcpSessionConfig, conv: u32) -> KcpTokioResult<Self> {
        let remote_addr = tokio::net::lookup_host(addr).await?.next()
            .ok_or_else(|| KcpTokioError::ConnectionFailed("could not resolve address".to_string()))?;
        let local_addr = if remote_addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let socket = Arc::new(UdpSocket::bind(local_addr).await?);
        let recv_buf_size = config.recv_buf_size;
        let session = KcpSession::new(conv, socket, remote_addr, config)?;
        Ok(Self { session, read_buf: vec![0u8; recv_buf_size], read_pos: 0, read_len: 0 })
    }

    pub(crate) fn from_session(session: KcpSession) -> Self {
        let recv_buf_size = session.config().recv_buf_size;
        Self { session, read_buf: vec![0u8; recv_buf_size], read_pos: 0, read_len: 0 }
    }

    pub fn conv(&self) -> u32 { self.session.conv() }
    pub fn remote_addr(&self) -> SocketAddr { self.session.remote_addr() }
    pub fn local_addr(&self) -> io::Result<SocketAddr> { self.session.socket().local_addr() }
    pub async fn send_kcp(&mut self, data: &[u8]) -> KcpTokioResult<usize> { self.session.send(data) }
    pub async fn recv_kcp(&mut self, buf: &mut [u8]) -> KcpTokioResult<usize> { self.session.recv(buf).await }
}

impl AsyncRead for KcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if this.read_pos < this.read_len {
            let remaining = &this.read_buf[this.read_pos..this.read_len];
            let to_copy = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_copy]);
            this.read_pos += to_copy;
            return Poll::Ready(Ok(()));
        }
        this.read_pos = 0; this.read_len = 0;
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

fn rand_conv() -> u32 {
    use std::time::SystemTime;
    let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_nanos();
    ((seed ^ (seed >> 16) ^ (seed >> 32)) & 0xFFFFFFFF) as u32
}
