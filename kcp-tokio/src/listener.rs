//! KCP server listener for accepting incoming connections.
//!
//! This module provides [`KcpListener`], which listens on a UDP port
//! and accepts incoming KCP connections. Each new conversation ID
//! from a new address creates a new KCP session.
//!
//! # Example
//!
//! ```no_run
//! use kcp_tokio::{KcpListener, KcpStream};
//! use kcp_tokio::config::KcpSessionConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut listener = KcpListener::bind("0.0.0.0:8080", KcpSessionConfig::fast()).await?;
//! loop {
//!     let (mut stream, addr) = listener.accept().await?;
//!     tokio::spawn(async move {
//!         let mut buf = [0u8; 1024];
//!         while let Ok(n) = stream.recv_kcp(&mut buf).await {
//!             if n == 0 { break; }
//!             stream.send_kcp(&buf[..n]).await.ok();
//!         }
//!     });
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use kcp_core::Kcp;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::config::KcpSessionConfig;
use crate::error::{KcpTokioError, KcpTokioResult};
use crate::session::KcpSession;
use crate::stream::KcpStream;

/// Session key: (remote address, conversation ID)
type SessionKey = (SocketAddr, u32);

/// A KCP listener that accepts incoming KCP connections over UDP.
pub struct KcpListener {
    /// Receiver for new incoming connections.
    incoming_rx: mpsc::Receiver<(KcpStream, SocketAddr)>,
    /// Local address the listener is bound to.
    local_addr: SocketAddr,
}

impl KcpListener {
    /// Bind a KCP listener to the given address.
    pub async fn bind<A: tokio::net::ToSocketAddrs>(
        addr: A,
        config: KcpSessionConfig,
    ) -> KcpTokioResult<Self> {
        let socket = UdpSocket::bind(addr).await?;
        let local_addr = socket.local_addr()?;
        let socket = Arc::new(socket);

        let (incoming_tx, incoming_rx) = mpsc::channel(64);

        let accept_socket = socket.clone();
        tokio::spawn(async move {
            Self::accept_loop(accept_socket, config, incoming_tx).await;
        });

        Ok(Self {
            incoming_rx,
            local_addr,
        })
    }

    /// Accept a new incoming KCP connection.
    pub async fn accept(&mut self) -> KcpTokioResult<(KcpStream, SocketAddr)> {
        self.incoming_rx
            .recv()
            .await
            .ok_or(KcpTokioError::Closed)
    }

    /// Get the local address the listener is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Background task: receive UDP packets and route to sessions.
    async fn accept_loop(
        socket: Arc<UdpSocket>,
        config: KcpSessionConfig,
        incoming_tx: mpsc::Sender<(KcpStream, SocketAddr)>,
    ) {
        let mut recv_buf = vec![0u8; config.recv_buf_size];
        let mut sessions: HashMap<SessionKey, mpsc::Sender<Vec<u8>>> = HashMap::new();

        loop {
            let (n, addr) = match socket.recv_from(&mut recv_buf).await {
                Ok(result) => result,
                Err(e) => {
                    log::error!("KcpListener: UDP recv error: {}", e);
                    continue;
                }
            };

            let packet = recv_buf[..n].to_vec();

            // Need at least KCP overhead for a valid packet
            if n < kcp_sys::IKCP_OVERHEAD {
                continue;
            }

            let conv = Kcp::get_conv(&packet);
            let key = (addr, conv);

            // Route to existing session
            if let Some(tx) = sessions.get(&key) {
                if tx.send(packet).await.is_err() {
                    sessions.remove(&key);
                }
                continue;
            }

            // New session: create channel and session
            let (pkt_tx, pkt_rx) = mpsc::channel::<Vec<u8>>(256);

            let session = match KcpSession::new_with_channel(
                conv,
                socket.clone(),
                addr,
                config.clone(),
                pkt_rx,
            ) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("KcpListener: failed to create session: {}", e);
                    continue;
                }
            };

            let stream = KcpStream::from_session(session);

            // Feed the first packet through the channel
            if pkt_tx.send(packet).await.is_err() {
                continue;
            }

            sessions.insert(key, pkt_tx);

            if incoming_tx.send((stream, addr)).await.is_err() {
                break; // Listener dropped
            }
        }
    }
}