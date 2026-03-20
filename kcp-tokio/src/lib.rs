//! Async KCP protocol implementation based on tokio.
//!
//! This crate provides an async-ready KCP implementation built on top of
//! `kcp-core` and `tokio`, enabling reliable UDP communication with
//! `AsyncRead`/`AsyncWrite` support.
//!
//! # Features
//!
//! - [`KcpStream`] — async read/write stream over KCP
//! - [`KcpListener`] — accept incoming KCP connections
//! - Configurable KCP parameters and session management
//!
//! # Example: Client
//!
//! ```no_run
//! use kcp_tokio::KcpStream;
//! use kcp_tokio::config::KcpSessionConfig;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut stream = KcpStream::connect("127.0.0.1:8080", KcpSessionConfig::fast()).await?;
//! stream.send_kcp(b"hello").await?;
//!
//! let mut buf = [0u8; 1024];
//! let n = stream.recv_kcp(&mut buf).await?;
//! println!("Received: {:?}", &buf[..n]);
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Server
//!
//! ```no_run
//! use kcp_tokio::KcpListener;
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

pub mod config;
pub mod error;
pub mod listener;
pub mod session;
pub mod stream;

pub use config::KcpSessionConfig;
pub use error::{KcpTokioError, KcpTokioResult};
pub use listener::KcpListener;
pub use stream::KcpStream;