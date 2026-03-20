# kcp-rust

A Rust wrapper for the [KCP](https://github.com/skywind3000/kcp) protocol, providing safe bindings to the original C implementation and an async UDP communication layer based on [tokio](https://tokio.rs/).

## Overview

KCP is a fast and reliable ARQ (Automatic Repeat reQuest) protocol that can reduce average latency by 30%-40% compared to TCP, at the cost of 10%-20% more bandwidth. This library provides:

- **`kcp-sys`** — Raw FFI bindings to the KCP C library
- **`kcp-core`** — Safe, idiomatic Rust wrapper around KCP
- **`kcp-tokio`** — Async reliable UDP communication using KCP + tokio

## Project Structure

```
kcp-rust/
├── kcp-sys/      # Raw FFI bindings (compiles C code)
├── kcp-core/     # Safe Rust API wrapper
└── kcp-tokio/    # Async tokio integration
```

## Quick Start

> 🚧 This project is under active development. API is not yet stable.

Add to your `Cargo.toml`:

```toml
[dependencies]
kcp-tokio = { path = "path/to/kcp-tokio" }
```

### Echo Server Example

```rust
// Coming soon — see kcp-tokio/examples/
```

### Echo Client Example

```rust
// Coming soon — see kcp-tokio/examples/
```

## Building

```bash
cargo build
```

## Testing

```bash
cargo test --workspace
```

## License

MIT License. See [LICENSE](LICENSE) for details.

## Credits

- [KCP](https://github.com/skywind3000/kcp) by skywind3000 — the original C implementation
