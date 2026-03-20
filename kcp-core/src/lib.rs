//! Safe Rust wrapper for the KCP protocol.
//!
//! This crate provides a safe, idiomatic Rust API on top of the raw
//! KCP FFI bindings from `kcp-sys`.
//!
//! # Overview
//!
//! KCP is a fast and reliable ARQ (Automatic Repeat reQuest) protocol
//! that can reduce average latency by 30%-40% compared to TCP, at the
//! cost of 10%-20% more bandwidth.
//!
//! # Quick Start
//!
//! ```no_run
//! use kcp_core::{Kcp, KcpConfig};
//! use std::io;
//!
//! // Create a KCP instance with a fast preset
//! let mut kcp = Kcp::with_config(0x01, &KcpConfig::fast(), |data: &[u8]| -> io::Result<usize> {
//!     // Send data via your transport (UDP, etc.)
//!     Ok(data.len())
//! }).unwrap();
//!
//! // Send data
//! kcp.send(b"Hello, KCP!").unwrap();
//!
//! // Update state periodically
//! kcp.update(0);
//! ```
//!
//! # Modules
//!
//! - [`kcp`] - Core KCP control object wrapper
//! - [`config`] - KCP configuration presets
//! - [`error`] - Error types

pub mod config;
pub mod error;
pub mod kcp;

pub use config::KcpConfig;
pub use error::{KcpError, KcpResult};
pub use kcp::Kcp;