//! KCP Echo Server Example

use kcp_io::tokio_rt::{KcpListener, KcpSessionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "0.0.0.0:9090";
    let config = KcpSessionConfig::fast();

    println!("KCP Echo Server starting on {}", addr);
    let mut listener = KcpListener::bind(addr, config).await?;
    println!("Listening...");

    loop {
        let (mut stream, remote_addr) = listener.accept().await?;
        println!("[{}] New connection (conv={})", remote_addr, stream.conv());

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stream.recv_kcp(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = &buf[..n];
                        println!(
                            "[{}] Received {} bytes: {:?}",
                            remote_addr,
                            n,
                            String::from_utf8_lossy(data)
                        );
                        if let Err(e) = stream.send_kcp(data).await {
                            eprintln!("[{}] Send error: {}", remote_addr, e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("[{}] Recv error: {}", remote_addr, e);
                        break;
                    }
                }
            }
            println!("[{}] Connection closed", remote_addr);
        });
    }
}
