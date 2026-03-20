//! KCP Echo Client Example
//!
//! This client connects to a KCP echo server and sends messages.
//!
//! Usage: cargo run --example echo_client

use kcp_tokio::config::KcpSessionConfig;
use kcp_tokio::KcpStream;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let server_addr = "127.0.0.1:9090";
    let config = KcpSessionConfig::fast();

    println!("Connecting to KCP server at {}...", server_addr);
    let mut stream = KcpStream::connect(server_addr, config).await?;
    println!(
        "Connected! (conv={}, local={})",
        stream.conv(),
        stream.local_addr()?
    );

    let messages = [
        "Hello, KCP!",
        "This is a reliable UDP message.",
        "KCP provides low-latency reliable transport.",
        "Goodbye!",
    ];

    for msg in &messages {
        // Send message
        stream.send_kcp(msg.as_bytes()).await?;
        println!("Sent: {:?}", msg);

        // Receive echo
        let mut buf = [0u8; 4096];
        match tokio::time::timeout(Duration::from_secs(5), stream.recv_kcp(&mut buf)).await {
            Ok(Ok(n)) => {
                let reply = String::from_utf8_lossy(&buf[..n]);
                println!("Echo: {:?}", reply);
            }
            Ok(Err(e)) => {
                eprintln!("Recv error: {}", e);
                break;
            }
            Err(_) => {
                eprintln!("Timeout waiting for echo");
                break;
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    println!("Done!");
    Ok(())
}
