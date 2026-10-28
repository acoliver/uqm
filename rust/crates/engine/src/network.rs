//! Safe native byte streams behind the host network port.
pub mod loopback;

/// Nonblocking streams; resolution and timed connection attempts run off-thread.
#[derive(Default)]
pub struct NativeNet {
    next: u64,
    entries: std::collections::HashMap<crate::host::Connection, PortEntry>,
}

enum PortEntry {
    Connecting {
        result: std::sync::mpsc::Receiver<std::io::Result<std::net::TcpStream>>,
        cancel: std::sync::mpsc::Sender<()>,
    },
    Listening(Vec<std::net::TcpListener>),
    Connected(std::net::TcpStream),
}

fn port_number(port: &str) -> std::io::Result<u16> {
    port.parse()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))
}

fn connect_stream(
    host: String,
    port: u16,
    cancel: std::sync::mpsc::Receiver<()>,
) -> std::io::Result<std::net::TcpStream> {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;
    let addresses: Vec<_> = (host.as_str(), port).to_socket_addrs()?.collect();
    if addresses.is_empty() {
        return Err(std::io::ErrorKind::AddrNotAvailable.into());
    }
    loop {
        for address in &addresses {
            if cancel.try_recv() != Err(std::sync::mpsc::TryRecvError::Empty) {
                return Err(std::io::ErrorKind::Interrupted.into());
            }
            if let Ok(stream) = TcpStream::connect_timeout(address, Duration::from_millis(2000)) {
                return Ok(stream);
            }
        }
        if cancel.recv_timeout(Duration::from_millis(2000))
            != Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        {
            return Err(std::io::ErrorKind::Interrupted.into());
        }
    }
}

impl NativeNet {
    fn insert(&mut self, entry: PortEntry) -> crate::host::Connection {
        self.next += 1;
        let id = crate::host::Connection(self.next);
        self.entries.insert(id, entry);
        id
    }
    fn stream(&mut self, connection: crate::host::Connection) -> &mut std::net::TcpStream {
        match self
            .entries
            .get_mut(&connection)
            .unwrap_or_else(|| panic!("unknown connection"))
        {
            PortEntry::Connected(stream) => stream,
            _ => panic!("stream operation before connection ready"),
        }
    }
}

impl crate::host::Net for NativeNet {
    fn connect(&mut self, host: &str, port: &str) -> std::io::Result<crate::host::Connection> {
        let port = port_number(port)?;
        let host = host.to_owned();
        let (sender, result) = std::sync::mpsc::channel();
        let (cancel, cancelled) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("netplay-connect".into())
            .spawn(move || {
                let _ = sender.send(connect_stream(host, port, cancelled));
            })?;
        Ok(self.insert(PortEntry::Connecting { result, cancel }))
    }
    fn listen(&mut self, port: &str) -> std::io::Result<crate::host::Connection> {
        let port = port_number(port)?;
        let mut listeners = Vec::new();
        let mut last_error = None;
        for host in ["::", "0.0.0.0"] {
            match std::net::TcpListener::bind((host, port)) {
                Ok(listener) => {
                    listener.set_nonblocking(true)?;
                    listeners.push(listener);
                }
                Err(error) => last_error = Some(error),
            }
        }
        if listeners.is_empty() {
            return Err(last_error.unwrap_or_else(|| panic!("both address families attempted")));
        }
        Ok(self.insert(PortEntry::Listening(listeners)))
    }
    fn poll(&mut self, connection: crate::host::Connection) -> std::io::Result<bool> {
        let stream = match self
            .entries
            .get_mut(&connection)
            .unwrap_or_else(|| panic!("unknown connection"))
        {
            PortEntry::Connected(_) => return Ok(true),
            PortEntry::Connecting { result, .. } => match result.try_recv() {
                Ok(result) => Some(result?),
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err(std::io::ErrorKind::ConnectionAborted.into())
                }
            },
            PortEntry::Listening(listeners) => {
                let mut accepted = None;
                for listener in listeners {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            accepted = Some(stream);
                            break;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(error) => return Err(error),
                    }
                }
                accepted
            }
        };
        if let Some(stream) = stream {
            stream.set_nonblocking(true)?;
            stream.set_nodelay(true)?;
            self.entries
                .insert(connection, PortEntry::Connected(stream));
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn send(
        &mut self,
        connection: crate::host::Connection,
        bytes: &[u8],
    ) -> std::io::Result<usize> {
        use std::io::Write;
        self.stream(connection).write(bytes)
    }
    fn recv(
        &mut self,
        connection: crate::host::Connection,
        bytes: &mut [u8],
    ) -> std::io::Result<usize> {
        use std::io::Read;
        self.stream(connection).read(bytes)
    }
    fn close(&mut self, connection: crate::host::Connection) {
        let entry = self
            .entries
            .remove(&connection)
            .unwrap_or_else(|| panic!("closing unknown connection"));
        if let PortEntry::Connecting { cancel, .. } = entry {
            drop(cancel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::Net;
    use std::{
        io::ErrorKind,
        time::{Duration, Instant},
    };

    #[test]
    fn nonblocking_listener_and_client_exchange_bytes_and_close() {
        let mut server = NativeNet::default();
        let listener = server.listen("0").unwrap();
        assert!(!server.poll(listener).unwrap());
        let PortEntry::Listening(listeners) = &server.entries[&listener] else {
            panic!("listener")
        };
        let address = listeners[0].local_addr().unwrap();
        let host = if address.is_ipv6() {
            "::1"
        } else {
            "127.0.0.1"
        };
        let mut client = NativeNet::default();
        let peer = client.connect(host, &address.port().to_string()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !server.poll(listener).unwrap() || !client.poll(peer).unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let mut bytes = [0; 3];
        assert_eq!(
            server.recv(listener, &mut bytes).unwrap_err().kind(),
            ErrorKind::WouldBlock
        );
        assert_eq!(client.send(peer, b"UQM").unwrap(), 3);
        loop {
            match server.recv(listener, &mut bytes) {
                Ok(count) => {
                    assert_eq!(count, 3);
                    break;
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                result => panic!("{result:?}"),
            }
        }
        assert_eq!(&bytes, b"UQM");
        client.close(peer);
        loop {
            match server.recv(listener, &mut bytes) {
                Ok(count) => {
                    assert_eq!(count, 0);
                    break;
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                result => panic!("{result:?}"),
            }
        }
        server.close(listener);
        assert!(client.entries.is_empty());
        assert!(server.entries.is_empty());
    }
    #[test]
    fn invalid_ports_fail_at_the_boundary_and_pending_work_is_cancellable() {
        let mut net = NativeNet::default();
        for port in ["-1", "65536", "not-a-port"] {
            assert_eq!(
                net.listen(port).unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
            assert_eq!(
                net.connect("localhost", port).unwrap_err().kind(),
                ErrorKind::InvalidInput
            );
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(sender);
        assert_eq!(
            connect_stream("127.0.0.1".into(), 1, receiver)
                .unwrap_err()
                .kind(),
            ErrorKind::Interrupted
        );
        let pending = net.connect("127.0.0.1", "1").unwrap();
        net.close(pending);
        assert!(net.entries.is_empty());
    }
}
