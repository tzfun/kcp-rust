# KCP-IO 项目计划清单

> 项目目标：实现一个 Rust 版本的 KCP 算法封装库，通过 FFI 调用原生 C 版本 KCP，并基于 UDP + KCP 实现可靠的异步 UDP 通信协议（兼容 tokio）。
>
> KCP 原始库地址：https://github.com/skywind3000/kcp
>
> 创建日期：2026-03-20

---

## 一、项目结构规划

> **注意**：项目已从多 crate workspace 结构重构为单一 crate + features 结构。

```
kcp-io/
├── Cargo.toml                  # 单 crate 配置（含 features 定义）
├── build.rs                    # 编译 C 代码的构建脚本
├── PLAN.md                     # 本计划文件
├── README.md                   # 项目说明文档
├── LICENSE                     # 开源许可证（MIT）
│
├── kcp/                        # KCP C 源码
│   ├── ikcp.h
│   ├── ikcp.c
│   └── wrapper.h               # 用于 bindgen 的头文件包装
│
├── src/
│   ├── lib.rs                  # 根模块：feature 条件导出 + 便捷 re-exports
│   ├── sys.rs                  # FFI 绑定声明（feature: `kcp-sys`）
│   ├── core/                   # 安全的 Rust 封装（feature: `kcp-core`）
│   │   ├── mod.rs              # 模块导出
│   │   ├── kcp.rs              # KCP 控制对象的安全封装
│   │   ├── config.rs           # KCP 配置参数
│   │   └── error.rs            # 错误类型定义
│   └── tokio_rt/               # 基于 tokio 的异步 UDP+KCP（feature: `kcp-tokio`，默认启用）
│       ├── mod.rs              # 模块导出
│       ├── session.rs          # KCP 会话管理
│       ├── listener.rs         # KCP 服务端监听器
│       ├── stream.rs           # KCP 流（AsyncRead/AsyncWrite）
│       ├── config.rs           # 运行时配置
│       └── error.rs            # 错误类型定义
│
├── tests/
│   └── integration_tests.rs    # 集成测试
│
├── benches/
│   └── throughput.rs           # 性能基准测试（criterion）
│
└── examples/
    ├── echo_server.rs          # 回显服务器示例
    └── echo_client.rs          # 回显客户端示例
```

### Features 层级关系

```
kcp-tokio（默认启用）
  └── kcp-core
        └── kcp-sys
```

- **`kcp-sys`**：暴露 `pub mod sys`，提供原始 FFI 绑定
- **`kcp-core`**：暴露 `pub mod core`，隐含 `kcp-sys`，提供安全 Rust 封装
- **`kcp-tokio`**：暴露 `pub mod tokio_rt`，隐含 `kcp-core`，提供异步 tokio 集成（额外依赖 `tokio`, `bytes`）

便捷 re-exports（在 `lib.rs` 中）：
- 启用 `kcp-tokio` 时：`KcpListener`, `KcpSessionConfig`, `KcpStream`, `KcpTokioError`, `KcpTokioResult`
- 启用 `kcp-core` 时：`Kcp`, `KcpConfig`, `KcpError`, `KcpResult`

---

## 二、技术选型与依赖

| 用途 | 依赖 | 说明 |
|------|------|------|
| C 代码编译 | `cc` crate (build-dep) | 在 build.rs 中编译 ikcp.c |
| 异步运行时 | `tokio` (optional) | 异步 UDP 通信，仅 `kcp-tokio` feature 引入 |
| 字节处理 | `bytes` (optional) | 高效字节缓冲区，仅 `kcp-tokio` feature 引入 |
| 日志 | `log` | 调试日志 |
| 错误处理 | `thiserror` | 错误类型派生 |
| 基准测试 | `criterion` (dev-dep) | 性能基准测试框架 |
| 日志输出 | `env_logger` (dev-dep) | 测试/示例中的日志输出 |

---

## 三、详细 TODO List

### 阶段 0：项目初始化

- [x] **0.1** 初始化 Cargo 项目（单 crate 结构 + features） ✅ 2026-03-20
- [x] **0.2** 定义 features 层级：`kcp-sys` → `kcp-core` → `kcp-tokio`（默认） ✅ 2026-03-20
- [x] **0.3** 引入 KCP C 源码（`kcp/ikcp.h`, `kcp/ikcp.c`, `kcp/wrapper.h`） ✅ 2026-03-20
- [x] **0.4** 初始化 git 仓库，配置 `.gitignore` ✅ 2026-03-20
- [x] **0.5** 创建 README.md 项目说明 ✅ 2026-03-20

### 阶段 1：sys 模块 — FFI 绑定层

> 目标：编译 KCP C 源码并暴露原始 FFI 接口给 Rust
> 模块路径：`src/sys.rs`（feature: `kcp-sys`）

- [x] **1.1** 编写 `build.rs`，使用 `cc` crate 编译 `kcp/ikcp.c` ✅ 2026-03-20
  - 配置编译选项（优化级别、警告等）
  - 处理跨平台编译（Windows/Linux/macOS）
- [x] **1.2** 编写 `kcp/wrapper.h` 头文件包装 ✅ 2026-03-20
- [x] **1.3** 编写 FFI 绑定声明（`src/sys.rs`），已绑定所有 C 函数 ✅ 2026-03-20
  - `ikcp_create`, `ikcp_release`, `ikcp_setoutput`, `ikcp_recv`, `ikcp_send`
  - `ikcp_update`, `ikcp_check`, `ikcp_input`, `ikcp_flush`, `ikcp_peeksize`
  - `ikcp_setmtu`, `ikcp_wndsize`, `ikcp_waitsnd`, `ikcp_nodelay`
  - `ikcp_allocator`, `ikcp_getconv`
  - `ikcp_log` — 变参函数，无法直接FFI绑定，通过 writelog 回调替代
- [x] **1.4** 定义对应的 C 结构体绑定 ✅ 2026-03-20
  - `IKCPSEG` — KCP 分片结构
  - `IKCPCB` — KCP 控制块主结构体
  - `IQUEUEHEAD` — 内部链表节点
  - 相关常量（`IKCP_LOG_*`, `IKCP_OVERHEAD`）
- [x] **1.5** 定义回调函数类型 ✅ 2026-03-20
  - `ikcp_output_callback` — output 回调类型
  - `ikcp_writelog_callback` — writelog 回调类型
- [x] **1.6** 编写编译验证测试（10个测试全部通过） ✅ 2026-03-20
  - test_create_and_release
  - test_set_output
  - test_nodelay_config
  - test_set_mtu
  - test_wndsize
  - test_getconv
  - test_peeksize_empty
  - test_waitsnd_empty
  - test_send_and_waitsnd
  - test_loopback_communication（双向通信验证）
- [x] **1.7** 验证 Windows 编译通过 ✅ 2026-03-20（Linux 待后续 CI 验证）

### 阶段 2：core 模块 — 安全 Rust 封装层

> 目标：将 unsafe FFI 调用封装为安全、符合 Rust 惯例的 API
> 模块路径：`src/core/`（feature: `kcp-core`）

- [x] **2.1** 定义错误类型 `src/core/error.rs` ✅ 2026-03-20
  - `KcpError` 枚举：CreateFailed, SendFailed, RecvWouldBlock, RecvBufferTooSmall, RecvFailed, InputFailed, SetMtuFailed, InvalidConfig, ConvMismatch, OutputError
  - 通过 `thiserror` 实现 `std::error::Error` trait
  - `KcpResult<T>` 类型别名
- [x] **2.2** 定义配置结构 `src/core/config.rs` ✅ 2026-03-20
  - `KcpConfig` 结构体：nodelay, interval, resend, nc, mtu, snd_wnd, rcv_wnd, stream_mode
  - 预设：`default()`, `fast()`, `normal()`
- [x] **2.3** 实现 KCP 控制对象封装 `src/core/kcp.rs` ✅ 2026-03-20
  - `Kcp` 结构体（持有 `*mut IKCPCB` + `*mut KcpContext`）
  - 所有核心方法：new, with_config, apply_config, send, recv, input, update, check, flush, peeksize, waitsnd, conv, set_mtu, set_wndsize, set_nodelay, set_stream_mode, is_stream_mode, get_conv
  - 实现 `Drop` trait 调用 `ikcp_release` 并释放 KcpContext
  - 实现 `unsafe impl Send`（附安全性论证注释）
- [x] **2.4** 处理 output 回调机制 ✅ 2026-03-20
  - 使用 `Box<dyn FnMut(&[u8]) -> io::Result<usize>>` 存储回调
  - 通过 `KcpContext` 结构体 + `user` 指针桥接 C 回调到 Rust 闭包
  - `kcp_output_cb` 作为 C 兼容的 extern "C" 函数
- [x] **2.5** 线程安全性设计 ✅ 2026-03-20
  - `Kcp: Send`（可转移线程） + `!Sync`（不可跨线程共享引用）
- [x] **2.6** 编写单元测试（20 个测试全部通过） ✅ 2026-03-20
- [x] **2.7** 编写文档注释（rustdoc） ✅ 2026-03-20

### 阶段 3：tokio_rt 模块 — 异步 UDP+KCP 通信层

> 目标：基于 tokio 的 UdpSocket 和 core 模块，实现完整的可靠 UDP 异步通信
> 模块路径：`src/tokio_rt/`（feature: `kcp-tokio`，默认启用）

- [x] **3.1** 定义错误类型 `src/tokio_rt/error.rs` ✅ 2026-03-20
  - `KcpTokioError`：Io, Kcp, Timeout, Closed, ConnectionFailed
  - `KcpTokioResult<T>` 类型别名
- [x] **3.2** 定义运行时配置 `src/tokio_rt/config.rs` ✅ 2026-03-20
  - `KcpSessionConfig`：kcp_config, flush_interval, timeout, flush_write, recv_buf_size
  - 预设：`default()`, `fast()`, `normal()`
- [x] **3.3** 实现 KCP 会话 `src/tokio_rt/session.rs` ✅ 2026-03-20
  - `KcpSession` 结构体
  - 双模式接收：`RecvMode::Socket`（客户端） / `RecvMode::Channel`（服务端）
  - `tokio::select!` 同时处理 UDP/Channel 接收和定时 update
  - 超时检测和优雅关闭
  - `SharedKcpSession` 类型别名（`Arc<Mutex<KcpSession>>`）
- [x] **3.4** 实现 KCP 流 `src/tokio_rt/stream.rs` ✅ 2026-03-20
  - `KcpStream` 实现 `AsyncRead` + `AsyncWrite`
  - `connect()` / `connect_with_conv()` 客户端连接
  - `send_kcp()` / `recv_kcp()` 高层 API
  - 内部读缓冲区管理
- [x] **3.5** 实现 KCP 监听器 `src/tokio_rt/listener.rs` ✅ 2026-03-20
  - `KcpListener` 后台任务按 `(addr, conv)` 路由到 mpsc channel
  - 新连接通过 channel 发送给 `accept()`
  - `HashMap<SessionKey, Sender>` 管理已知会话
- [x] **3.6** 连接机制 ✅ 2026-03-20
  - 基于 conv 的自动会话识别
  - `rand_conv()` 生成随机 conv ID
  - `connect_with_conv()` 支持指定 conv
- [x] **3.7** 实现优雅关闭 ✅ 2026-03-20
  - `KcpSession::close()` + `AsyncWrite::poll_shutdown`
- [x] **3.8** 编写集成测试（`tests/integration_tests.rs`，4 个测试全部通过） ✅ 2026-03-20
  - test_client_server_basic_communication
  - test_bidirectional_communication
  - test_large_data_transfer
  - test_multiple_messages
- [x] **3.9** 编写示例程序 ✅ 2026-03-20
  - `examples/echo_server.rs` — 回显服务器
  - `examples/echo_client.rs` — 回显客户端

### 阶段 4：文档与完善

- [x] **4.1** 完善 README.md ✅ 2026-03-20
  - 项目简介 + What is KCP
  - Features 说明（feature 层级关系）
  - 项目结构图（已更新为单 crate 结构）
  - 快速开始指南（Server/Client/AsyncRead/AsyncWrite/Core-only 示例）
  - API 概览（`tokio_rt` / `core` / `sys` 模块表格）
  - 配置说明（预设表格 + 自定义配置 + Stream vs Message 模式对比）
  - 构建 / 测试 / 运行命令（已更新为单 crate 命令）
  - 架构图 + 设计决策说明
  - 性能特点表格
- [x] **4.2** 编写各层的 rustdoc 文档 ✅ 2026-03-20
  - `src/lib.rs` — crate 级文档 + Quick Start 示例
  - `src/sys.rs` — 模块文档 + Safety 说明，所有类型/常量/函数注释
  - `src/core/mod.rs` — 模块文档 + 主要类型说明 + 线程安全说明 + 示例
  - `src/core/config.rs` — 模块文档 + `KcpConfig` 所有字段注释 + 预设方法文档
  - `src/core/error.rs` — 模块文档 + `KcpError` 所有变体注释 + 字段注释
  - `src/core/kcp.rs` — 模块文档 + `Kcp` 结构体文档 + 所有方法文档（参数、错误、示例）
  - `src/tokio_rt/mod.rs` — 模块文档 + 架构说明 + 主要类型 + 示例
  - `src/tokio_rt/config.rs` — 模块文档 + `KcpSessionConfig` 所有字段注释 + 预设方法文档
  - `src/tokio_rt/error.rs` — 模块文档 + `KcpTokioError` 所有变体注释
  - `src/tokio_rt/session.rs` — 模块文档 + `KcpSession` 结构体/字段文档 + 所有方法文档
  - `src/tokio_rt/stream.rs` — 模块文档 + `KcpStream` 结构体文档 + 所有方法文档 + 示例
  - `src/tokio_rt/listener.rs` — 模块文档 + `KcpListener` 结构体/架构文档 + 所有方法文档 + 示例
  - 8 个 doc-test 全部通过
  - `cargo doc --no-deps` 零警告 ✅
- [x] **4.3** 添加 CI 配置（GitHub Actions） ✅ 2026-03-20
  - `.github/workflows/ci.yml`
  - 多平台编译测试（ubuntu/windows/macos × stable/1.85.0）
  - 单元测试运行
  - clippy 检查（-D warnings）
  - rustfmt 格式检查
  - 文档构建检查（RUSTDOCFLAGS=-D warnings）
  - Cargo 缓存优化
- [x] **4.4** 添加 LICENSE 文件（MIT） ✅ 2026-03-20
- [x] **4.5** 性能测试与基准测试（`benches/throughput.rs`，criterion 框架） ✅ 2026-03-20
  - KCP 吞吐量测试（64/256/1024/4096 字节）
  - 原始 UDP 吞吐量基线
  - TCP 吞吐量基线
  - KCP 延迟测试（32 字节 ping-pong）
- [x] **4.6** 发布前检查 ✅ 2026-03-20
  - `cargo clippy --all-targets -- -D warnings` 零警告 ✅
  - `cargo test` 全部 12 测试通过（4 集成 + 8 文档测试） ✅
  - `cargo doc --no-deps` 生成无误 ✅
  - 版本号 0.1.0 已设置 ✅

### 阶段 5：项目重构 ✅ 2026-03-20

> 目标：从多 crate workspace 结构重构为单一 crate + features 结构，简化依赖管理和发布流程

- [x] **5.1** 将三个独立 crate（`kcp-sys`, `kcp-core`, `kcp-tokio`）合并为单一 crate `kcp-io` ✅ 2026-03-20
- [x] **5.2** 通过 Cargo features 实现层级控制：`kcp-sys` → `kcp-core` → `kcp-tokio` ✅ 2026-03-20
- [x] **5.3** 原 `kcp-sys/src/lib.rs` → `src/sys.rs` ✅ 2026-03-20
- [x] **5.4** 原 `kcp-core/src/` → `src/core/` ✅ 2026-03-20
- [x] **5.5** 原 `kcp-tokio/src/` → `src/tokio_rt/` ✅ 2026-03-20
- [x] **5.6** 移动测试到 `tests/integration_tests.rs` ✅ 2026-03-20
- [x] **5.7** 移动基准测试到 `benches/throughput.rs` ✅ 2026-03-20
- [x] **5.8** 移动示例到 `examples/` ✅ 2026-03-20
- [x] **5.9** 在 `lib.rs` 中添加便捷 re-exports ✅ 2026-03-20
- [x] **5.10** 更新 `Cargo.toml`（元信息、features、依赖） ✅ 2026-03-20
- [x] **5.11** 验证所有测试通过 ✅ 2026-03-20

---

## 四、KCP C 库关键概念备忘

### KCP 工作流程

```
发送端:                                    接收端:
  应用层                                    应用层
    │                                        ▲
    ▼                                        │
 ikcp_send()                             ikcp_recv()
    │                                        ▲
    ▼                                        │
  [KCP 协议层: 分片、ARQ、拥塞控制]      [KCP 协议层: 重组、ACK]
    │                                        ▲
    ▼                                        │
 output 回调                             ikcp_input()
    │                                        ▲
    ▼                                        │
  UDP sendto()                           UDP recvfrom()
```

### KCP 核心参数

| 参数 | 含义 | 默认值 | 推荐（低延迟） |
|------|------|--------|----------------|
| nodelay | 启用 nodelay 模式 | 0 | 1 |
| interval | update 间隔（ms） | 100 | 10-20 |
| resend | 快速重传触发次数 | 0 | 2 |
| nc | 关闭拥塞控制 | 0 | 1 |
| snd_wnd | 发送窗口 | 32 | 128-1024 |
| rcv_wnd | 接收窗口 | 128 | 128-1024 |
| mtu | 最大传输单元 | 1400 | 1400 |

### 关键注意事项

1. **ikcp_update** 必须定期调用，驱动 KCP 内部状态机
2. **output 回调** 是 KCP 输出底层数据包的唯一通道，必须正确实现
3. **conv** (Conversation ID) 用于区分会话，发送接收双方必须一致
4. **线程安全**：KCP C 库本身不是线程安全的，同一 KCP 实例不能并发调用
5. **时间戳**：KCP 使用毫秒级时间戳，需要提供单调递增的时钟
6. **MTU**：需要考虑 UDP 头部开销（20 IP + 8 UDP = 28 bytes），KCP 自身头部 24 bytes

---

## 五、实施顺序与里程碑

| 里程碑 | 内容 | 预期状态 |
|--------|------|----------|
| M0 | 项目初始化、目录结构、依赖配置 | ✅ 已完成 |
| M1 | sys 模块 FFI 绑定层完成并通过编译测试 | ✅ 已完成 |
| M2 | core 模块安全封装完成并通过单元测试 | ✅ 已完成 |
| M3 | tokio_rt 模块异步通信层完成并通过集成测试 | ✅ 已完成 |
| M4 | 文档、示例、CI 完善 | ✅ 已完成 |
| M5 | 项目重构：多 crate workspace → 单 crate + features | ✅ 已完成 |

---

## 六、风险与注意事项

1. **FFI 安全性**：C 回调中使用 Rust 闭包需要特别注意内存安全，`user` 指针的生命周期管理是关键
2. **跨平台编译**：需要确保 `cc` crate 在 Windows（MSVC/GNU）和 Linux/macOS 上都能正确编译 KCP C 代码
3. **异步设计复杂度**：tokio 集成中的 update 定时器和 UDP 收发的协调需要仔细设计，避免死锁和性能问题
4. **会话管理**：服务端多客户端场景下，共享 UdpSocket 的读写分离是一个挑战
5. **内存泄漏**：确保所有 C 分配的内存在 Rust 的 Drop 中正确释放
6. **Feature 兼容性**：确保各 feature 组合都能正确编译（仅 `kcp-sys`、仅 `kcp-core`、全功能 `kcp-tokio`）