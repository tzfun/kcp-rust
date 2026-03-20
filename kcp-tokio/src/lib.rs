//! Async KCP protocol implementation based on tokio.
//!
//! This crate provides an async-ready KCP implementation built on top of
//! `kcp-core` and `tokio`, enabling reliable UDP communication with
//! `AsyncRead`/`AsyncWrite` support.
//!
//! # Features
//!
//! - `KcpStream` — async read/write stream over KCP
//! - `KcpListener` — accept incoming KCP connections
//! - Configurable KCP parameters and session management
//!
//! # Modules
//!
//! - [`session`] - KCP session management
//! - [`stream`] - Async KCP stream (AsyncRead/AsyncWrite)
//! - [`listener`] - KCP server listener
//! - [`config`] - Runtime configuration
//! - [`error`] - Error types

pub mod config;
pub mod error;
pub mod listener;
pub mod session;
pub mod stream;
