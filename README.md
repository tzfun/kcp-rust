# kcp-io

[![CI](https://github.com/tzfun/kcp-io/actions/workflows/ci.yml/badge.svg)](https://github.com/tzfun/kcp-io/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/kcp-io.svg)](https://crates.io/crates/kcp-io)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust wrapper for the [KCP](https://github.com/skywind3000/kcp) protocol, providing safe bindings to the original C implementation and an async UDP communication layer based on [tokio](https://tokio.rs/).

## What is KCP?

KCP is a fast and reliable ARQ (Automatic Repeat reQuest) protocol that can **reduce average latency by 30%–40%** compared to TCP, at the cost of 10%–20% more bandwidth. It sits above the transport layer (UDP) and provides:

- ✅ Reliable, ordered delivery
- ✅ Automatic retransmission with configurable strategy
- ✅ Congestion control (optional)
- ✅ Stream mode and message mode
- ✅ No kernel dependency — runs entirely in user space

## Features

This crate uses Cargo features to provide a layered architecture:

- **`kcp-sys`** — Raw FFI bindings to the KCP C library (compiled from source via `cc`)
- **`kcp-core`** — Safe, idiomatic Rust wrapper around KCP with `Send` support (implies `kcp-sys`)
- **`kcp-tokio`** *(default)* — Fully async `KcpStream` + `KcpListener` powered by tokio, with `AsyncRead`/`AsyncWrite` support (implies `kcp-core`)

Feature dependency chain: `kcp-tokio` → `kcp-core` → `kcp-sys`

## Project Structure

```
kcp-io/
├── Cargo.toml              # Single crate with feature flags
├── build.rs                # Compiles KCP C code via cc crate
├── kcp/                    # Original KCP C source
│   ├── ikcp.c
│   ├── ikcp.h
│   └── wrapper.h
├── src/
│   ├── lib.rs              # Root module: feature-gated exports + re-exports
│   ├── sys.rs              # Raw FFI bindings (feature: kcp-sys)
│   ├── core/               # Safe Rust API wrapper (feature: kcp-core)
│   │   ├── mod.rs
│   │   ├── kcp.rs          # Kcp struct (safe wrapper around IKCPCB)
│   │   ├── config.rs       # KcpConfig presets
│   │   └── error.rs        # KcpError enum
│   └── tokio_rt/           # Async tokio integration (feature: kcp-tokio)
│       ├── mod.rs
│       ├── stream.rs       # KcpStream (AsyncRead + AsyncWrite)
│       ├── listener.rs     # KcpListener (accept incoming connections)
│       ├── session.rs      # KcpSession (internal state machine)
│       ├── config.rs       # KcpSessionConfig
│       └── error.rs        # KcpTokioError
├── tests/                  # Integration tests
├── benches/                # Criterion benchmarks
└── examples/               # Echo server & client
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
kcp-io = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Echo Server

```rust
use kcp_io::{KcpListener, KcpSessionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = KcpListener::bind("0.0.0.0:9090", KcpSessionConfig::fast()).await?;
    println!("Listening on {}", listener.local_addr());

    loop {
        let (mut stream, addr) = listener.accept().await?;
        println!("[{}] connected (conv={})", addr, stream.conv());

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stream.recv_kcp(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        stream.send_kcp(&buf[..n]).await.ok();
                    }
                    Err(_) => break,
                }
            }
        });
    }
}
```

### Echo Client

```rust
use kcp_io::{KcpStream, KcpSessionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = KcpStream::connect("127.0.0.1:9090", KcpSessionConfig::fast()).await?;
    println!("Connected! conv={}", stream.conv());

    // Send
    stream.send_kcp(b"Hello, KCP!").await?;

    // Receive echo
    let mut buf = [0u8; 4096];
    let n = stream.recv_kcp(&mut buf).await?;
    println!("Echo: {}", String::from_utf8_lossy(&buf[..n]));

    Ok(())
}
```

### Using `AsyncRead` / `AsyncWrite`

`KcpStream` implements Tokio's `AsyncRead` and `AsyncWrite` traits, so you can use it with any Tokio-compatible ecosystem:

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use kcp_io::{KcpStream, KcpSessionConfig};

async fn example() -> std::io::Result<()> {
    let mut stream = KcpStream::connect("127.0.0.1:9090", KcpSessionConfig::fast())
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Write with AsyncWrite
    stream.write_all(b"hello via AsyncWrite").await?;

    // Read with AsyncRead
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await?;
    println!("Received: {}", String::from_utf8_lossy(&buf[..n]));

    Ok(())
}
```

### Using Only the Core Layer

If you only need the safe KCP wrapper without async support:

```toml
[dependencies]
kcp-io = { version = "0.1", default-features = false, features = ["kcp-core"] }
```

```rust
use kcp_io::core::{Kcp, KcpConfig};
use std::io;

let mut kcp = Kcp::new(0x01, |data: &[u8]| -> io::Result<usize> {
    // Send data via your own transport
    Ok(data.len())
}).unwrap();

kcp.apply_config(&KcpConfig::fast()).unwrap();
kcp.send(b"Hello, KCP!").unwrap();
kcp.update(0);
```

## API Overview

### `tokio_rt` module (default feature: `kcp-tokio`)

| Type | Description |
|------|-------------|
| `KcpStream` | Async KCP stream — implements `AsyncRead` + `AsyncWrite` |
| `KcpListener` | Accepts incoming KCP connections on a UDP socket |
| `KcpSessionConfig` | Runtime configuration (KCP params + session settings) |
| `KcpTokioError` | Error type for async operations |

**`KcpStream` methods:**

| Method | Description |
|--------|-------------|
| `connect(addr, config)` | Connect to a remote KCP server |
| `connect_with_conv(addr, conv, config)` | Connect with a specific conversation ID |
| `send_kcp(data)` | Send data reliably |
| `recv_kcp(buf)` | Receive data |
| `conv()` | Get the conversation ID |
| `remote_addr()` | Get the remote address |
| `local_addr()` | Get the local address |

**`KcpListener` methods:**

| Method | Description |
|--------|-------------|
| `bind(addr, config)` | Bind to a local address and start listening |
| `accept()` | Accept the next incoming connection |
| `local_addr()` | Get the listener's local address |

### `core` module (feature: `kcp-core`)

| Type | Description |
|------|-------------|
| `Kcp` | Safe wrapper around the C `IKCPCB` struct |
| `KcpConfig` | Protocol configuration (nodelay, interval, resend, etc.) |
| `KcpError` | Error type for KCP operations |

### `sys` module (feature: `kcp-sys`)

Direct bindings to all `ikcp_*` functions. You generally don't need to use this directly.

## Configuration

### Presets

| Preset | nodelay | interval | resend | nc | snd_wnd | rcv_wnd | Use Case |
|--------|---------|----------|--------|----|---------|---------|----------|
| `default()` | off | 100ms | 0 | off | 32 | 128 | Conservative, low bandwidth |
| `normal()` | on | 40ms | 2 | off | 64 | 128 | Balanced latency/bandwidth |
| `fast()` | on | 10ms | 2 | on | 128 | 128 | Lowest latency, more bandwidth |

### Custom Configuration

```rust
use kcp_io::tokio_rt::KcpSessionConfig;
use kcp_io::core::KcpConfig;
use std::time::Duration;

let config = KcpSessionConfig {
    kcp_config: KcpConfig {
        nodelay: true,
        interval: 20,
        resend: 2,
        nc: true,
        mtu: 1200,          // Smaller MTU for restrictive networks
        snd_wnd: 256,        // Larger window for high-throughput
        rcv_wnd: 256,
        stream_mode: true,   // Byte-stream mode (like TCP)
    },
    flush_interval: Duration::from_millis(20),
    timeout: Some(Duration::from_secs(30)),
    flush_write: true,       // Flush immediately on write
    recv_buf_size: 65536,
};
```

### Stream Mode vs Message Mode

| Feature | Message Mode (default) | Stream Mode |
|---------|----------------------|-------------|
| Boundary | Preserves message boundaries | Byte stream (like TCP) |
| Large data | Limited by fragment count | Unlimited size |
| Small data | Each message is a packet | Merges small messages |
| Use case | Game packets, RPC | File transfer, bulk data |

## Building

```bash
# Build (default features: kcp-tokio)
cargo build

# Build with only core features
cargo build --no-default-features --features kcp-core

# Build in release mode
cargo build --release
```

**Requirements:**
- Rust 1.85+ (2021 edition)
- A C compiler (MSVC on Windows, gcc/clang on Linux/macOS) — needed to compile `ikcp.c`

## Testing

```bash
# Run all tests (unit + integration + doc tests)
cargo test

# Run only integration tests
cargo test --test integration_tests

# Run with logging
RUST_LOG=debug cargo test -- --nocapture
```

## Running Examples

```bash
# Terminal 1: Start the echo server
cargo run --example echo_server

# Terminal 2: Run the echo client
cargo run --example echo_client
```

## Architecture

```
Client Side                              Server Side
┌──────────────────┐                     ┌──────────────────┐
│   Application    │                     │   Application    │
│                  │                     │                  │
│  KcpStream       │                     │  KcpListener     │
│  ├─ send_kcp()   │                     │  ├─ bind()       │
│  ├─ recv_kcp()   │                     │  ├─ accept()     │
│  └─ AsyncR/W     │                     │  └─ KcpStream    │
├──────────────────┤                     ├──────────────────┤
│  KcpSession      │                     │  KcpSession      │
│  ├─ KCP engine   │                     │  ├─ KCP engine   │
│  ├─ update timer │                     │  ├─ mpsc channel │
│  └─ UdpSocket    │                     │  └─ (shared UDP) │
├──────────────────┤                     ├──────────────────┤
│  core::Kcp       │                     │  core::Kcp       │
│  └─ output cb ───┼── UDP packets ──▶ ──┼─ input()        │
├──────────────────┤                     ├──────────────────┤
│  sys (FFI)       │                     │  sys (FFI)       │
│  └─ ikcp.c       │                     │  └─ ikcp.c       │
└──────────────────┘                     └──────────────────┘
         │                                        │
         └──────── UDP / Internet ────────────────┘
```

**Key design decisions:**
- **Client sessions** own their UDP socket directly (`RecvMode::Socket`)
- **Server sessions** receive packets via `mpsc::channel` from the listener's shared socket (`RecvMode::Channel`)
- The `KcpListener` runs a background `tokio::spawn` task that multiplexes incoming UDP packets by `(SocketAddr, conv)` to the correct session channel
- `KcpSession` drives `ikcp_update()` during send/recv calls

## Performance Characteristics

| Aspect | Details |
|--------|---------|
| Latency | 30–40% lower than TCP on lossy networks |
| Bandwidth | ~10–20% overhead vs raw UDP |
| CPU | Minimal — KCP is lightweight C code |
| Memory | One `Kcp` instance per session (~10 KB) |
| Concurrency | One `KcpSession` per connection, managed by tokio runtime |

## License

MIT License. See [LICENSE](LICENSE) for details.

## Credits

- [KCP](https://github.com/skywind3000/kcp) by skywind3000 — the original C implementation
- [tokio](https://tokio.rs/) — the async runtime powering the async layer
