//! Exercise the host network port without SDL, user configuration or saves.
use std::{
    error::Error,
    io::Read,
    time::{Duration, Instant},
};
use uqm_engine::{host::Net, network::NativeNet};

fn main() -> Result<(), Box<dyn Error>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let service = listener.local_addr()?.port().to_string();
    let mut network = NativeNet::default();
    let connection = network.connect("localhost", &service)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !network.poll(connection)? {
        assert!(Instant::now() < deadline, "connect deadline");
        std::thread::yield_now();
    }
    let mut byte = [0];
    assert_eq!(
        network.recv(connection, &mut byte).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(network.send(connection, b"UQM")?, 3);
    let (mut peer, _) = listener.accept()?;
    peer.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut received = [0; 3];
    peer.read_exact(&mut received)?;
    assert_eq!(&received, b"UQM");
    network.close(connection);
    assert_eq!(peer.read(&mut byte)?, 0);
    println!("Platform smoke passed: DNS, nonblocking connect/recv, send and socket cleanup");
    Ok(())
}
