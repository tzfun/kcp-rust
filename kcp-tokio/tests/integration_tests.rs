//! Integration tests for kcp-tokio.

use kcp_tokio::{KcpListener, KcpStream};
use kcp_tokio::config::KcpSessionConfig;
use std::time::Duration;
#[allow(unused_imports)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time;

/// Helper: get a fast config with short timeout for tests.
fn test_config() -> KcpSessionConfig {
    KcpSessionConfig {
        timeout: Some(Duration::from_secs(5)),
        ..KcpSessionConfig::fast()
    }
}

#[tokio::test]
async fn test_client_server_basic_communication() {
    let config = test_config();

    // Start server
    let mut listener = KcpListener::bind("127.0.0.1:0", config.clone())
        .await
        .expect("Failed to bind listener");

    let server_addr = listener.local_addr();

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let (mut stream, addr) = listener.accept().await.expect("Failed to accept");
        println!("Server: accepted connection from {}", addr);

        let mut buf = [0u8; 1024];
        let n = stream.recv_kcp(&mut buf).await.expect("Server recv failed");
        println!("Server: received {} bytes", n);

        // Echo back
        stream.send_kcp(&buf[..n]).await.expect("Server send failed");
        println!("Server: echoed {} bytes", n);
    });

    // Give server time to start
    time::sleep(Duration::from_millis(50)).await;

    // Client connects
    let mut client = KcpStream::connect_with_conv(
        server_addr,
        config,
        0x12345678,
    )
    .await
    .expect("Client connect failed");

    // Client sends data
    let msg = b"Hello, KCP server!";
    client.send_kcp(msg).await.expect("Client send failed");

    // Client receives echo
    let mut buf = [0u8; 1024];
    let n = client.recv_kcp(&mut buf).await.expect("Client recv failed");

    assert_eq!(&buf[..n], msg);
    println!("Client: received echo: {:?}", std::str::from_utf8(&buf[..n]));

    // Wait for server to finish
    let _ = time::timeout(Duration::from_secs(5), server_handle).await;
}

#[tokio::test]
async fn test_bidirectional_communication() {
    let config = test_config();

    let mut listener = KcpListener::bind("127.0.0.1:0", config.clone())
        .await
        .unwrap();
    let server_addr = listener.local_addr();

    let server_handle = tokio::spawn(async move {
        let (mut stream, _addr) = listener.accept().await.unwrap();

        // Server sends first
        stream.send_kcp(b"Hello from server").await.unwrap();

        // Then receives
        let mut buf = [0u8; 1024];
        let n = stream.recv_kcp(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"Hello from client");
    });

    time::sleep(Duration::from_millis(50)).await;

    let mut client = KcpStream::connect_with_conv(server_addr, config, 0x100)
        .await
        .unwrap();

    // Client sends
    client.send_kcp(b"Hello from client").await.unwrap();

    // Client receives
    let mut buf = [0u8; 1024];
    let n = client.recv_kcp(&mut buf).await.unwrap();
    assert_eq!(&buf[..n], b"Hello from server");

    let _ = time::timeout(Duration::from_secs(5), server_handle).await;
}

#[tokio::test]
async fn test_large_data_transfer() {
    // Use stream mode with larger window for big data transfer
    let mut config = test_config();
    config.kcp_config.stream_mode = true;
    config.kcp_config.snd_wnd = 512;
    config.kcp_config.rcv_wnd = 512;

    let mut listener = KcpListener::bind("127.0.0.1:0", config.clone())
        .await
        .unwrap();
    let server_addr = listener.local_addr();

    // Generate a large message (2KB - fits within window)
    let large_data: Vec<u8> = (0..2048u16).map(|i| (i % 256) as u8).collect();
    let expected = large_data.clone();

    let server_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // In stream mode, we may receive data in chunks
        let mut total_recv = Vec::new();
        let mut buf = vec![0u8; 8192];
        while total_recv.len() < expected.len() {
            let n = stream.recv_kcp(&mut buf).await.unwrap();
            total_recv.extend_from_slice(&buf[..n]);
        }
        assert_eq!(total_recv.len(), expected.len());
        assert_eq!(&total_recv, &expected);

        // Echo back
        stream.send_kcp(&total_recv).await.unwrap();
    });

    time::sleep(Duration::from_millis(50)).await;

    let mut client = KcpStream::connect_with_conv(server_addr, config, 0x200)
        .await
        .unwrap();

    client.send_kcp(&large_data).await.unwrap();

    // Receive echo in stream mode (may come in chunks)
    let mut total_recv = Vec::new();
    let mut buf = vec![0u8; 8192];
    while total_recv.len() < large_data.len() {
        let n = client.recv_kcp(&mut buf).await.unwrap();
        total_recv.extend_from_slice(&buf[..n]);
    }
    assert_eq!(total_recv.len(), large_data.len());
    assert_eq!(&total_recv, &large_data);

    let _ = time::timeout(Duration::from_secs(5), server_handle).await;
}

#[tokio::test]
async fn test_multiple_messages() {
    let config = test_config();

    let mut listener = KcpListener::bind("127.0.0.1:0", config.clone())
        .await
        .unwrap();
    let server_addr = listener.local_addr();

    let server_handle = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();

        // Receive 3 messages and echo each
        for _i in 0..3 {
            let mut buf = [0u8; 1024];
            let n = stream.recv_kcp(&mut buf).await.unwrap();
            stream.send_kcp(&buf[..n]).await.unwrap();
        }
    });

    time::sleep(Duration::from_millis(50)).await;

    let mut client = KcpStream::connect_with_conv(server_addr, config, 0x300)
        .await
        .unwrap();

    let messages = [b"First message" as &[u8], b"Second message", b"Third message"];

    for msg in &messages {
        client.send_kcp(msg).await.unwrap();

        let mut buf = [0u8; 1024];
        let n = client.recv_kcp(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], *msg);
    }

    let _ = time::timeout(Duration::from_secs(5), server_handle).await;
}
