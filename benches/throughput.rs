//! Throughput and latency benchmarks for KCP vs raw UDP vs TCP.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;

fn bench_kcp_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("kcp_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    for &size in &[64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("send_recv", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                use kcp_io::tokio_rt::{KcpListener, KcpSessionConfig, KcpStream};
                let config = KcpSessionConfig::fast();
                let mut listener = KcpListener::bind("127.0.0.1:0", config.clone())
                    .await
                    .unwrap();
                let addr = listener.local_addr();
                let server = tokio::spawn(async move {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut buf = vec![0u8; size];
                    let n = stream.recv_kcp(&mut buf).await.unwrap();
                    stream.send_kcp(&buf[..n]).await.unwrap();
                });
                let mut client = KcpStream::connect(addr, KcpSessionConfig::fast())
                    .await
                    .unwrap();
                let data = vec![0xABu8; size];
                client.send_kcp(&data).await.unwrap();
                let mut buf = vec![0u8; size];
                client.recv_kcp(&mut buf).await.unwrap();
                server.await.unwrap();
            });
        });
    }
    group.finish();
}

fn bench_udp_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("udp_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    for &size in &[64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("send_recv", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                use tokio::net::UdpSocket;
                let server_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
                let server_addr = server_socket.local_addr().unwrap();
                let server = tokio::spawn(async move {
                    let mut buf = vec![0u8; size];
                    let (n, addr) = server_socket.recv_from(&mut buf).await.unwrap();
                    server_socket.send_to(&buf[..n], addr).await.unwrap();
                });
                let client_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
                let data = vec![0xABu8; size];
                client_socket.send_to(&data, server_addr).await.unwrap();
                let mut buf = vec![0u8; size];
                client_socket.recv_from(&mut buf).await.unwrap();
                server.await.unwrap();
            });
        });
    }
    group.finish();
}

fn bench_tcp_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("tcp_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    for &size in &[64, 256, 1024, 4096] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("send_recv", size), &size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                use tokio::net::TcpListener;
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();
                let server = tokio::spawn(async move {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut buf = vec![0u8; size];
                    let n = stream.read(&mut buf).await.unwrap();
                    stream.write_all(&buf[..n]).await.unwrap();
                });
                let mut client = tokio::net::TcpStream::connect(addr).await.unwrap();
                let data = vec![0xABu8; size];
                client.write_all(&data).await.unwrap();
                let mut buf = vec![0u8; size];
                let mut total = 0;
                while total < size {
                    let n = client.read(&mut buf[total..]).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    total += n;
                }
                server.await.unwrap();
            });
        });
    }
    group.finish();
}

fn bench_kcp_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("kcp_latency");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    group.bench_function("ping_pong_32bytes", |b| {
        b.to_async(&rt).iter(|| async move {
            use kcp_io::tokio_rt::{KcpListener, KcpSessionConfig, KcpStream};
            let config = KcpSessionConfig::fast();
            let mut listener = KcpListener::bind("127.0.0.1:0", config.clone())
                .await
                .unwrap();
            let addr = listener.local_addr();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = [0u8; 32];
                let n = stream.recv_kcp(&mut buf).await.unwrap();
                stream.send_kcp(&buf[..n]).await.unwrap();
            });
            let mut client = KcpStream::connect(addr, KcpSessionConfig::fast())
                .await
                .unwrap();
            client
                .send_kcp(b"ping-kcp-latency-test-32-bytes!!")
                .await
                .unwrap();
            let mut buf = [0u8; 32];
            client.recv_kcp(&mut buf).await.unwrap();
            server.await.unwrap();
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_kcp_throughput,
    bench_udp_throughput,
    bench_tcp_throughput,
    bench_kcp_latency
);
criterion_main!(benches);
