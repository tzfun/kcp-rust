//! KCP server listener for accepting incoming connections.
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use crate::core::Kcp;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use super::config::KcpSessionConfig;
use super::error::{KcpTokioError, KcpTokioResult};
use super::session::KcpSession;
use super::stream::KcpStream;

type SessionKey = (SocketAddr, u32);

pub struct KcpListener {
    incoming_rx: mpsc::Receiver<(KcpStream, SocketAddr)>,
    local_addr: SocketAddr,
}

impl KcpListener {
    pub async fn bind<A: tokio::net::ToSocketAddrs>(addr: A, config: KcpSessionConfig) -> KcpTokioResult<Self> {
        let socket = UdpSocket::bind(addr).await?;
        let local_addr = socket.local_addr()?;
        let socket = Arc::new(socket);
        let (incoming_tx, incoming_rx) = mpsc::channel(64);
        let accept_socket = socket.clone();
        tokio::spawn(async move { Self::accept_loop(accept_socket, config, incoming_tx).await; });
        Ok(Self { incoming_rx, local_addr })
    }

    pub async fn accept(&mut self) -> KcpTokioResult<(KcpStream, SocketAddr)> {
        self.incoming_rx.recv().await.ok_or(KcpTokioError::Closed)
    }

    pub fn local_addr(&self) -> SocketAddr { self.local_addr }

    async fn accept_loop(socket: Arc<UdpSocket>, config: KcpSessionConfig, incoming_tx: mpsc::Sender<(KcpStream, SocketAddr)>) {
        let mut recv_buf = vec![0u8; config.recv_buf_size];
        let mut sessions: HashMap<SessionKey, mpsc::Sender<Vec<u8>>> = HashMap::new();
        loop {
            let (n, addr) = match socket.recv_from(&mut recv_buf).await {
                Ok(result) => result,
                Err(e) => { log::error!("KcpListener: UDP recv error: {}", e); continue; }
            };
            let packet = recv_buf[..n].to_vec();
            if n < crate::sys::IKCP_OVERHEAD { continue; }
            let conv = Kcp::get_conv(&packet);
            let key = (addr, conv);
            if let Some(tx) = sessions.get(&key) {
                if tx.send(packet).await.is_err() { sessions.remove(&key); }
                continue;
            }
            let (pkt_tx, pkt_rx) = mpsc::channel::<Vec<u8>>(256);
            let session = match KcpSession::new_with_channel(conv, socket.clone(), addr, config.clone(), pkt_rx) {
                Ok(s) => s,
                Err(e) => { log::error!("KcpListener: failed to create session: {}", e); continue; }
            };
            let stream = KcpStream::from_session(session);
            if pkt_tx.send(packet).await.is_err() { continue; }
            sessions.insert(key, pkt_tx);
            if incoming_tx.send((stream, addr)).await.is_err() { break; }
        }
    }
}
