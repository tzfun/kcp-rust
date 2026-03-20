# KCP-Rust 项目计划清单

> 项目目标：实现一个 Rust 版本的 KCP 算法封装库，通过 FFI 调用原生 C 版本 KCP，并基于 UDP + KCP 实现可靠的异步 UDP 通信协议（兼容 tokio）。
>
> KCP 原始库地址：https://github.com/skywind3000/kcp
>
> 创建日期：2026-03-20

---

## 一、项目结构规划

```
kcp-rust/
├── Cargo.toml                  # 工作空间配置
├── PLAN.md                     # 本计划文件
├── README.md                   # 项目说明文档
├── LICENSE                     # 开源许可证
│
├── kcp-sys/                    # 底层 FFI 绑定 crate
│   ├── Cargo.toml
│   ├── build.rs                # 编译 C 代码的构建脚本
│   ├── src/
│   │   └── lib.rs              # FFI 绑定声明
│   ├── kcp/                    # KCP C 源码（git submodule 或直接引入）
│   │   ├── ikcp.h
│   │   └── ikcp.c
│   └── wrapper.h               # 用于 bindgen 的头文件包装
│
├── kcp-core/                   # 安全的 Rust 封装 crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # 模块导出
│   │   ├── kcp.rs              # KCP 控制对象的安全封装
│   │   ├── config.rs           # KCP 配置参数
│   │   └── error.rs            # 错误类型定义
│   └── tests/
│       └── kcp_tests.rs        # 核心功能单元测试
│
├── kcp-tokio/                  # 基于 tokio 的异步 UDP+KCP crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # 模块导出
│   │   ├── session.rs          # KCP 会话管理
│   │   ├── listener.rs         # KCP 服务端监听器
│   │   ├── stream.rs           # KCP 流（AsyncRead/AsyncWrite）
│   │   ├── config.rs           # 运行时配置
│   │   └── error.rs            # 错误类型定义
│   ├── tests/
│   │   └── integration_tests.rs
│   └── examples/
│       ├── echo_server.rs      # 回显服务器示例
│       └── echo_client.rs      # 回显客户端示例
│
└── examples/                   # 顶层示例
    └── basic_usage.rs
```

---

## 二、技术选型与依赖

| 用途 | 依赖 | 说明 |
|------|------|------|
| C 代码编译 | `cc` crate | 在 build.rs 中编译 ikcp.c |
| FFI 绑定生成 | `bindgen` (可选) | 自动生成绑定，或手动编写 |
| 异步运行时 | `tokio` | 异步 UDP 通信 |
| 日志 | `log` + `env_logger` | 调试日志 |
| 字节处理 | `bytes` | 高效字节缓冲区 |
| 错误处理 | `thiserror` | 错误类型派生 |

---

## 三、详细 TODO List

### 阶段 0：项目初始化

- [x] **0.1** 初始化 Cargo workspace（根目录 Cargo.toml） ✅ 2026-03-20
- [x] **0.2** 创建 `kcp-sys` crate 骨架 ✅ 2026-03-20
- [x] **0.3** 创建 `kcp-core` crate 骨架 ✅ 2026-03-20
- [x] **0.4** 创建 `kcp-tokio` crate 骨架 ✅ 2026-03-20
- [x] **0.5** 引入 KCP C 源码（从 https://github.com/skywind3000/kcp 获取 `ikcp.h` 和 `ikcp.c`） ✅ 2026-03-20
- [x] **0.6** 初始化 git 仓库，配置 `.gitignore` ✅ 2026-03-20
- [x] **0.7** 创建 README.md 项目说明 ✅ 2026-03-20

### 阶段 1：kcp-sys — FFI 绑定层

> 目标：编译 KCP C 源码并暴露原始 FFI 接口给 Rust

- [x] **1.1** 编写 `build.rs`，使用 `cc` crate 编译 `ikcp.c` ✅ 2026-03-20
  - 配置编译选项（优化级别、警告等）
  - 处理跨平台编译（Windows/Linux/macOS）
- [x] **1.2** 编写 `wrapper.h` 头文件包装 ✅ 2026-03-20
- [x] **1.3** 编写 FFI 绑定声明（`src/lib.rs`），已绑定所有 C 函数 ✅ 2026-03-20
  - `ikcp_create(conv, user)` — 创建 KCP 控制对象
  - `ikcp_release(kcp)` — 释放 KCP 控制对象
  - `ikcp_setoutput(kcp, output)` — 设置下层协议输出回调
  - `ikcp_recv(kcp, buffer, len)` — 接收数据
  - `ikcp_send(kcp, buffer, len)` — 发送数据
  - `ikcp_update(kcp, current)` — 更新状态（需定期调用）
  - `ikcp_check(kcp, current)` — 查询下次 update 的时间
  - `ikcp_input(kcp, data, size)` — 底层数据包输入
  - `ikcp_flush(kcp)` — 立即刷新待发送数据
  - `ikcp_peeksize(kcp)` — 查看下一个消息大小
  - `ikcp_setmtu(kcp, mtu)` — 设置 MTU
  - `ikcp_wndsize(kcp, sndwnd, rcvwnd)` — 设置发送/接收窗口大小
  - `ikcp_waitsnd(kcp)` — 获取待发送数据包数量
  - `ikcp_nodelay(kcp, nodelay, interval, resend, nc)` — 设置 nodelay 模式
  - `ikcp_log` — 变参函数，无法直接FFI绑定，通过 writelog 回调替代
  - `ikcp_allocator(new_malloc, new_free)` — 自定义内存分配器
  - `ikcp_getconv(ptr)` — 从数据包头获取 conv
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

### 阶段 2：kcp-core — 安全 Rust 封装层

> 目标：将 unsafe FFI 调用封装为安全、符合 Rust 惯例的 API

- [x] **2.1** 定义错误类型 `error.rs` ✅ 2026-03-20
  - `KcpError` 枚举：CreateFailed, SendFailed, RecvWouldBlock, RecvBufferTooSmall, RecvFailed, InputFailed, SetMtuFailed, InvalidConfig, ConvMismatch, OutputError
  - 通过 `thiserror` 实现 `std::error::Error` trait
  - C 层返回值到 Rust Result 的映射
  - `KcpResult<T>` 类型别名
- [x] **2.2** 定义配置结构 `config.rs` ✅ 2026-03-20
  - `KcpConfig` 结构体：nodelay, interval, resend, nc, mtu, snd_wnd, rcv_wnd, stream_mode
  - `KcpConfig::default()` — 默认保守配置
  - `KcpConfig::fast()` — 低延迟配置 (nodelay=1, interval=10, resend=2, nc=1)
  - `KcpConfig::normal()` — 平衡配置 (nodelay=1, interval=40, resend=2, nc=0)
  - 完整文档注释和示例
- [x] **2.3** 实现 KCP 控制对象封装 `kcp.rs` ✅ 2026-03-20
  - `Kcp` 结构体（持有 `*mut IKCPCB` + `*mut KcpContext`）
  - 所有核心方法已实现：new, with_config, apply_config, send, recv, input, update, check, flush, peeksize, waitsnd, conv, set_mtu, set_wndsize, set_nodelay, set_stream_mode, is_stream_mode, get_conv
  - 实现 `Drop` trait 调用 `ikcp_release` 并释放 KcpContext
  - 实现 `unsafe impl Send`（附安全性论证注释）
- [x] **2.4** 处理 output 回调机制 ✅ 2026-03-20
  - 使用 `Box<dyn FnMut(&[u8]) -> io::Result<usize>>` 存储回调
  - 通过 `KcpContext` 结构体 + `user` 指针桥接 C 回调到 Rust 闭包
  - `kcp_output_cb` 作为 C 兼容的 extern "C" 函数
  - 生命周期安全：KcpContext 与 Kcp 同生命周期，Drop 时释放
- [x] **2.5** 线程安全性设计 ✅ 2026-03-20
  - `Kcp: Send`（可转移线程） + `!Sync`（不可跨线程共享引用）
  - 所有可变操作都要求 `&mut self`
  - 安全性论证注释已添加
- [x] **2.6** 编写单元测试（20 个测试全部通过） ✅ 2026-03-20
  - 创建/销毁测试 (test_create_and_drop, test_create_with_config)
  - 配置测试 (test_default/fast/normal_config, test_apply_config, test_set_mtu, test_stream_mode)
  - 回环通信测试 (test_loopback_message_mode, test_loopback_bidirectional)
  - 流模式测试 (test_loopback_stream_mode)
  - 大数据分片测试 (test_large_data_fragmentation, 5000字节)
  - 错误处理测试 (test_error_display, test_set_mtu_too_small, test_peeksize_empty, test_recv_empty)
  - 回调测试 (test_output_callback_called, test_get_conv_from_packet)
- [x] **2.7** 编写文档注释（rustdoc） ✅ 2026-03-20
  - 所有公开类型和方法的文档注释
  - 模块级文档（lib.rs, kcp.rs, config.rs, error.rs）
  - 4 个文档示例测试全部通过

### 阶段 3：kcp-tokio — 异步 UDP+KCP 通信层

> 目标：基于 tokio 的 UdpSocket 和 kcp-core，实现完整的可靠 UDP 异步通信

- [x] **3.1** 定义错误类型 `error.rs` ✅ 2026-03-20
  - `KcpTokioError`：Io, Kcp, Timeout, Closed, ConnectionFailed
  - `KcpTokioResult<T>` 类型别名
- [x] **3.2** 定义运行时配置 `config.rs` ✅ 2026-03-20
  - `KcpSessionConfig`：kcp_config, flush_interval, timeout, flush_write, recv_buf_size
  - 预设：default(), fast(), normal()
- [x] **3.3** 实现 KCP 会话 `session.rs` ✅ 2026-03-20
  - 双模式接收：Socket(客户端) / Channel(服务端)
  - `tokio::select!` 同时处理 UDP/Channel 接收和定时 update
  - `try_send_to` 在 KCP output 回调中发送 UDP
  - 超时检测和优雅关闭
- [x] **3.4** 实现 KCP 流 `stream.rs` ✅ 2026-03-20
  - `KcpStream` 实现 `AsyncRead` + `AsyncWrite`
  - `connect()` / `connect_with_conv()` 客户端连接
  - `send_kcp()` / `recv_kcp()` 高层 API
  - `poll_read` 使用 `poll_recv_from` 非阻塞 UDP 接收
  - 内部读缓冲区管理
- [x] **3.5** 实现 KCP 监听器 `listener.rs` ✅ 2026-03-20
  - 后台任务接收 UDP → 按 (addr, conv) 路由到 mpsc channel
  - 新连接通过 channel 发送给 accept()
  - `HashMap<SessionKey, Sender>` 管理已知会话
- [x] **3.6** 连接机制 ✅ 2026-03-20
  - 基于 conv 的自动会话识别（首包触发新会话）
  - `rand_conv()` 生成随机 conv ID
  - `connect_with_conv()` 支持指定 conv
- [x] **3.7** 实现优雅关闭 ✅ 2026-03-20
  - `KcpSession::close()` 设置关闭标志
  - `AsyncWrite::poll_shutdown` 实现
  - 发送/接收检查关闭状态
- [x] **3.8** 编写集成测试（4 个测试全部通过） ✅ 2026-03-20
  - test_client_server_basic_communication — 客户端/服务端基本回显
  - test_bidirectional_communication — 双向通信
  - test_large_data_transfer — 大数据传输 (2KB, stream 模式)
  - test_multiple_messages — 多消息连续发送/接收
- [x] **3.9** 编写示例程序 ✅ 2026-03-20
  - `echo_server.rs` — 回显服务器
  - `echo_client.rs` — 回显客户端

### 阶段 4：文档与完善

- [x] **4.1** 完善 README.md ✅ 2026-03-20
  - 项目简介 + What is KCP
  - 功能特性（三层架构说明）
  - 快速开始指南（Server/Client/AsyncRead/AsyncWrite 示例）
  - API 概览（KcpStream, KcpListener, KcpSessionConfig 表格）
  - 配置说明（预设表格 + 自定义配置 + Stream vs Message 模式对比）
  - 架构图 + 设计决策说明
  - 性能特点表格
- [x] **4.2** 编写各层的 rustdoc 文档 ✅ 2026-03-20
  - 所有公开类型、方法、模块的文档注释
  - 10 个 doc-test 全部通过
  - `cargo doc --workspace --no-deps` 零警告
- [x] **4.3** 添加 CI 配置（GitHub Actions） ✅ 2026-03-20
  - `.github/workflows/ci.yml`
  - 多平台编译测试（ubuntu/windows/macos × stable/1.70.0）
  - 单元测试运行
  - clippy 检查（-D warnings）
  - rustfmt 格式检查
  - 文档构建检查（RUSTDOCFLAGS=-D warnings）
  - Cargo 缓存优化
- [x] **4.4** 添加 LICENSE 文件（MIT） ✅ 2026-03-20
- [x] **4.5** 性能测试与基准测试 ✅ 2026-03-20
  - `kcp-tokio/benches/throughput.rs`（criterion 框架）
  - KCP 吞吐量测试（64/256/1024/4096 字节）
  - 原始 UDP 吞吐量基线
  - TCP 吞吐量基线
  - KCP 延迟测试（32 字节 ping-pong）
- [x] **4.6** 发布前检查 ✅ 2026-03-20
  - `cargo clippy --workspace --all-targets -- -D warnings` 零警告 ✅
  - `cargo test --workspace` 全部 44 测试通过 ✅
  - `cargo doc --workspace --no-deps` 生成无误 ✅
  - 版本号 0.1.0 已设置 ✅

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
| M1 | kcp-sys FFI 绑定层完成并通过编译测试 | ✅ 已完成 |
| M2 | kcp-core 安全封装完成并通过单元测试 | ✅ 已完成 |
| M3 | kcp-tokio 异步通信层完成并通过集成测试 | ✅ 已完成 |
| M4 | 文档、示例、CI 完善 | ✅ 已完成 |

---

## 六、风险与注意事项

1. **FFI 安全性**：C 回调中使用 Rust 闭包需要特别注意内存安全，`user` 指针的生命周期管理是关键
2. **跨平台编译**：需要确保 `cc` crate 在 Windows（MSVC/GNU）和 Linux/macOS 上都能正确编译 KCP C 代码
3. **异步设计复杂度**：tokio 集成中的 update 定时器和 UDP 收发的协调需要仔细设计，避免死锁和性能问题
4. **会话管理**：服务端多客户端场景下，共享 UdpSocket 的读写分离是一个挑战
5. **内存泄漏**：确保所有 C 分配的内存在 Rust 的 Drop 中正确释放
