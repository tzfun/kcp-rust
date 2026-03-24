//! KCP Echo Client Example

use kcp_io::tokio_rt::{KcpSessionConfig, KcpStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "127.0.0.1:9090";
    let config = KcpSessionConfig::fast();

    println!("Connecting to {}...", addr);
    let mut stream = KcpStream::connect(addr, config).await?;
    println!("Connected! conv={}", stream.conv());

    let messages = ["Hello, KCP!", "This is a test", "Goodbye!"];

    for msg in &messages {
        stream.send_kcp(msg.as_bytes()).await?;
        println!("Sent: {}", msg);

        let data = stream.recv_kcp().await?;
        println!("Echo: {}", String::from_utf8_lossy(&data));
    }

    println!("Done!");
    Ok(())
}
