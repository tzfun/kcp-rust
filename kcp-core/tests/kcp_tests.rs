//! Unit tests for kcp-core.

use kcp_core::{Kcp, KcpConfig, KcpError};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

/// Helper: create a KCP instance with a buffer-capturing output callback.
fn create_kcp_with_buffer(conv: u32, buf: Rc<RefCell<Vec<Vec<u8>>>>) -> Kcp {
    Kcp::new(conv, move |data: &[u8]| -> io::Result<usize> {
        buf.borrow_mut().push(data.to_vec());
        Ok(data.len())
    })
    .expect("Failed to create KCP instance")
}

// =====================================================================
// Lifecycle Tests
// =====================================================================

#[test]
fn test_create_and_drop() {
    let _kcp = Kcp::new(1, |_data: &[u8]| -> io::Result<usize> { Ok(0) }).unwrap();
    // Drop should clean up without issues
}

#[test]
fn test_create_with_config() {
    let config = KcpConfig::fast();
    let kcp = Kcp::with_config(1, &config, |_data: &[u8]| -> io::Result<usize> { Ok(0) }).unwrap();
    assert_eq!(kcp.conv(), 1);
}

#[test]
fn test_conv() {
    let kcp = Kcp::new(0xDEADBEEF, |_: &[u8]| -> io::Result<usize> { Ok(0) }).unwrap();
    assert_eq!(kcp.conv(), 0xDEADBEEF);
}

// =====================================================================
// Configuration Tests
// =====================================================================

#[test]
fn test_default_config() {
    let config = KcpConfig::default();
    assert!(!config.nodelay);
    assert_eq!(config.interval, 100);
    assert_eq!(config.resend, 0);
    assert!(!config.nc);
    assert_eq!(config.mtu, 1400);
    assert_eq!(config.snd_wnd, 32);
    assert_eq!(config.rcv_wnd, 128);
    assert!(!config.stream_mode);
}

#[test]
fn test_fast_config() {
    let config = KcpConfig::fast();
    assert!(config.nodelay);
    assert_eq!(config.interval, 10);
    assert_eq!(config.resend, 2);
    assert!(config.nc);
    assert_eq!(config.snd_wnd, 128);
    assert_eq!(config.rcv_wnd, 128);
}

#[test]
fn test_normal_config() {
    let config = KcpConfig::normal();
    assert!(config.nodelay);
    assert_eq!(config.interval, 40);
    assert_eq!(config.resend, 2);
    assert!(!config.nc);
}

#[test]
fn test_apply_config() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut kcp = create_kcp_with_buffer(1, buf);

    let config = KcpConfig {
        mtu: 1200,
        snd_wnd: 256,
        rcv_wnd: 256,
        stream_mode: true,
        ..KcpConfig::fast()
    };
    kcp.apply_config(&config).unwrap();
    assert!(kcp.is_stream_mode());
}

#[test]
fn test_set_mtu() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut kcp = create_kcp_with_buffer(1, buf);
    kcp.set_mtu(1200).unwrap();
}

#[test]
fn test_set_mtu_too_small() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut kcp = create_kcp_with_buffer(1, buf);

    // MTU too small to hold KCP header should fail
    let result = kcp.set_mtu(10);
    assert!(result.is_err());
    match result.unwrap_err() {
        KcpError::SetMtuFailed { mtu, .. } => assert_eq!(mtu, 10),
        other => panic!("Expected SetMtuFailed, got {:?}", other),
    }
}

#[test]
fn test_stream_mode() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut kcp = create_kcp_with_buffer(1, buf);

    assert!(!kcp.is_stream_mode());
    kcp.set_stream_mode(true);
    assert!(kcp.is_stream_mode());
    kcp.set_stream_mode(false);
    assert!(!kcp.is_stream_mode());
}

// =====================================================================
// Data Transfer Tests
// =====================================================================

#[test]
fn test_send_increases_waitsnd() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut kcp = create_kcp_with_buffer(1, buf);

    assert_eq!(kcp.waitsnd(), 0);
    kcp.send(b"hello").unwrap();
    assert!(kcp.waitsnd() > 0);
}

#[test]
fn test_peeksize_empty() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let kcp = create_kcp_with_buffer(1, buf);

    match kcp.peeksize() {
        Err(KcpError::RecvWouldBlock) => {} // expected
        other => panic!("Expected RecvWouldBlock, got {:?}", other),
    }
}

#[test]
fn test_recv_empty() {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut kcp = create_kcp_with_buffer(1, buf);

    let mut recv_buf = [0u8; 1024];
    match kcp.recv(&mut recv_buf) {
        Err(KcpError::RecvWouldBlock) => {} // expected
        other => panic!("Expected RecvWouldBlock, got {:?}", other),
    }
}

#[test]
fn test_loopback_message_mode() {
    // Two KCP instances communicating in message mode
    let buf_a = Rc::new(RefCell::new(Vec::new()));
    let buf_b = Rc::new(RefCell::new(Vec::new()));

    let mut kcp_a = create_kcp_with_buffer(0x01, buf_a.clone());
    let mut kcp_b = create_kcp_with_buffer(0x01, buf_b.clone());

    kcp_a.apply_config(&KcpConfig::fast()).unwrap();
    kcp_b.apply_config(&KcpConfig::fast()).unwrap();

    // A sends a message
    let msg = b"Hello from A to B!";
    kcp_a.send(msg).unwrap();

    // Simulate time and packet exchange
    let mut current: u32 = 0;
    let mut received = false;

    for _ in 0..200 {
        current += 10;

        kcp_a.update(current);
        kcp_b.update(current);

        // Transfer A -> B
        let packets: Vec<Vec<u8>> = buf_a.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_b.input(pkt).unwrap();
        }

        // Transfer B -> A (ACKs)
        let packets: Vec<Vec<u8>> = buf_b.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_a.input(pkt).unwrap();
        }

        // Try to receive on B
        let mut recv_buf = [0u8; 1024];
        match kcp_b.recv(&mut recv_buf) {
            Ok(n) => {
                assert_eq!(&recv_buf[..n], msg);
                received = true;
                break;
            }
            Err(KcpError::RecvWouldBlock) => continue,
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    assert!(received, "B should have received the message from A");
}

#[test]
fn test_loopback_bidirectional() {
    // Both sides send and receive
    let buf_a = Rc::new(RefCell::new(Vec::new()));
    let buf_b = Rc::new(RefCell::new(Vec::new()));

    let mut kcp_a = create_kcp_with_buffer(0x02, buf_a.clone());
    let mut kcp_b = create_kcp_with_buffer(0x02, buf_b.clone());

    kcp_a.apply_config(&KcpConfig::fast()).unwrap();
    kcp_b.apply_config(&KcpConfig::fast()).unwrap();

    let msg_a = b"Message from A";
    let msg_b = b"Message from B";
    kcp_a.send(msg_a).unwrap();
    kcp_b.send(msg_b).unwrap();

    let mut current: u32 = 0;
    let mut a_received = false;
    let mut b_received = false;

    for _ in 0..200 {
        current += 10;

        kcp_a.update(current);
        kcp_b.update(current);

        // Transfer packets
        let packets: Vec<Vec<u8>> = buf_a.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_b.input(pkt).unwrap();
        }
        let packets: Vec<Vec<u8>> = buf_b.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_a.input(pkt).unwrap();
        }

        // Try receive on both sides
        if !b_received {
            let mut recv_buf = [0u8; 1024];
            if let Ok(n) = kcp_b.recv(&mut recv_buf) {
                assert_eq!(&recv_buf[..n], msg_a);
                b_received = true;
            }
        }
        if !a_received {
            let mut recv_buf = [0u8; 1024];
            if let Ok(n) = kcp_a.recv(&mut recv_buf) {
                assert_eq!(&recv_buf[..n], msg_b);
                a_received = true;
            }
        }

        if a_received && b_received {
            break;
        }
    }

    assert!(a_received, "A should have received message from B");
    assert!(b_received, "B should have received message from A");
}

#[test]
fn test_loopback_stream_mode() {
    let buf_a = Rc::new(RefCell::new(Vec::new()));
    let buf_b = Rc::new(RefCell::new(Vec::new()));

    let mut kcp_a = create_kcp_with_buffer(0x03, buf_a.clone());
    let mut kcp_b = create_kcp_with_buffer(0x03, buf_b.clone());

    let mut config = KcpConfig::fast();
    config.stream_mode = true;
    kcp_a.apply_config(&config).unwrap();
    kcp_b.apply_config(&config).unwrap();

    assert!(kcp_a.is_stream_mode());
    assert!(kcp_b.is_stream_mode());

    // Send multiple small messages (they may be merged in stream mode)
    kcp_a.send(b"AAA").unwrap();
    kcp_a.send(b"BBB").unwrap();
    kcp_a.send(b"CCC").unwrap();

    let mut current: u32 = 0;
    let mut total_received = Vec::new();

    for _ in 0..200 {
        current += 10;

        kcp_a.update(current);
        kcp_b.update(current);

        let packets: Vec<Vec<u8>> = buf_a.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_b.input(pkt).unwrap();
        }
        let packets: Vec<Vec<u8>> = buf_b.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_a.input(pkt).unwrap();
        }

        let mut recv_buf = [0u8; 1024];
        while let Ok(n) = kcp_b.recv(&mut recv_buf) {
            total_received.extend_from_slice(&recv_buf[..n]);
        }

        if total_received.len() >= 9 {
            break;
        }
    }

    assert_eq!(&total_received, b"AAABBBCCC");
}

#[test]
fn test_large_data_fragmentation() {
    let buf_a = Rc::new(RefCell::new(Vec::new()));
    let buf_b = Rc::new(RefCell::new(Vec::new()));

    let mut kcp_a = create_kcp_with_buffer(0x04, buf_a.clone());
    let mut kcp_b = create_kcp_with_buffer(0x04, buf_b.clone());

    kcp_a.apply_config(&KcpConfig::fast()).unwrap();
    kcp_b.apply_config(&KcpConfig::fast()).unwrap();

    // Send a large message (larger than MSS, will be fragmented by KCP)
    let large_msg: Vec<u8> = (0..5000u16).map(|i| (i % 256) as u8).collect();
    kcp_a.send(&large_msg).unwrap();

    let mut current: u32 = 0;
    let mut received = false;

    for _ in 0..500 {
        current += 10;

        kcp_a.update(current);
        kcp_b.update(current);

        let packets: Vec<Vec<u8>> = buf_a.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_b.input(pkt).unwrap();
        }
        let packets: Vec<Vec<u8>> = buf_b.borrow_mut().drain(..).collect();
        for pkt in &packets {
            kcp_a.input(pkt).unwrap();
        }

        let mut recv_buf = vec![0u8; 8192];
        if let Ok(n) = kcp_b.recv(&mut recv_buf) {
            assert_eq!(n, large_msg.len());
            assert_eq!(&recv_buf[..n], &large_msg[..]);
            received = true;
            break;
        }
    }

    assert!(
        received,
        "Should have received the large fragmented message"
    );
}

#[test]
fn test_get_conv_from_packet() {
    let buf_a = Rc::new(RefCell::new(Vec::new()));
    let mut kcp_a = create_kcp_with_buffer(0xAABBCCDD, buf_a.clone());
    kcp_a.apply_config(&KcpConfig::fast()).unwrap();

    kcp_a.send(b"test").unwrap();
    kcp_a.update(100);

    let packets = buf_a.borrow();
    assert!(!packets.is_empty(), "Should have generated output packets");

    // Extract conv from the first packet
    let conv = Kcp::get_conv(&packets[0]);
    assert_eq!(conv, 0xAABBCCDD);
}

#[test]
fn test_output_callback_called() {
    let call_count = Rc::new(RefCell::new(0u32));
    let count_clone = call_count.clone();

    let mut kcp = Kcp::new(1, move |_data: &[u8]| -> io::Result<usize> {
        *count_clone.borrow_mut() += 1;
        Ok(0)
    })
    .unwrap();
    kcp.apply_config(&KcpConfig::fast()).unwrap();

    kcp.send(b"hello").unwrap();
    kcp.update(100);

    assert!(
        *call_count.borrow() > 0,
        "Output callback should have been called"
    );
}

// =====================================================================
// Error Handling Tests
// =====================================================================

#[test]
fn test_error_display() {
    let err = KcpError::SendFailed(-1);
    assert_eq!(format!("{}", err), "send failed (error code: -1)");

    let err = KcpError::RecvWouldBlock;
    assert_eq!(
        format!("{}", err),
        "no data available to receive (would block)"
    );

    let err = KcpError::RecvBufferTooSmall { need: 100, got: 50 };
    assert_eq!(
        format!("{}", err),
        "receive buffer too small (need 100 bytes, got 50 bytes)"
    );

    let err = KcpError::SetMtuFailed { mtu: 10, code: -1 };
    assert_eq!(
        format!("{}", err),
        "failed to set MTU to 10 (error code: -1)"
    );
}
