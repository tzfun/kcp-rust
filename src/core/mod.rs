//! Safe Rust wrapper for the KCP protocol.
//!
//! This module provides a safe, idiomatic Rust API on top of the raw FFI bindings
//! in [`crate::sys`]. It encapsulates all `unsafe` C calls and exposes a memory-safe
//! interface with proper error handling.
//!
//! # Main Types
//!
//! - [`Kcp`] — The core KCP control object. Wraps the C `IKCPCB` struct and manages
//!   its lifecycle (creation, configuration, send/recv, and automatic cleanup via `Drop`).
//! - [`KcpConfig`] — Protocol configuration with convenient presets (`default()`,
//!   `fast()`, `normal()`).
//! - [`KcpError`] — Error enum covering all KCP operation failures.
//!
//! # Thread Safety
//!
//! [`Kcp`] implements `Send` (can be moved across threads) but not `Sync`
//! (cannot be shared across threads by reference). All mutating operations
//! require `&mut self`.
//!
//! # Example
//!
//! ```no_run
//! use kcp_io::core::{Kcp, KcpConfig};
//! use std::io;
//!
//! let mut kcp = Kcp::with_config(0x01, &KcpConfig::fast(), |data: &[u8]| -> io::Result<usize> {
//!     // Your UDP send logic here
//!     Ok(data.len())
//! }).unwrap();
//!
//! // Send data through KCP
//! kcp.send(b"Hello, KCP!").unwrap();
//!
//! // Drive the KCP state machine (must be called periodically)
//! kcp.update(0);
//! ```

mod config;
mod error;
mod kcp;

pub use config::KcpConfig;
pub use error::{KcpError, KcpResult};
pub use kcp::Kcp;
