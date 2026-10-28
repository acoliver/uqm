//! In-memory byte streams with the same pending/read/close contract as sockets.
use crate::host::{Connection, Net};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    io,
    rc::Rc,
};

#[derive(Default)]
struct Stream {
    incoming: VecDeque<u8>,
    peer: Option<Connection>,
    closed: bool,
}
#[derive(Default)]
struct Bus {
    next: u64,
    streams: HashMap<Connection, Stream>,
    listeners: HashMap<String, Connection>,
}
impl Bus {
    fn allocate(&mut self) -> Connection {
        self.next += 1;
        let connection = Connection(self.next);
        self.streams.insert(connection, Stream::default());
        connection
    }
}

#[derive(Clone, Default)]
pub struct LoopbackNet {
    bus: Rc<RefCell<Bus>>,
}
impl LoopbackNet {
    pub fn pair() -> (Self, Self) {
        let net = Self::default();
        (net.clone(), net)
    }
}
fn would_block() -> io::Error {
    io::ErrorKind::WouldBlock.into()
}

impl Net for LoopbackNet {
    fn connect(&mut self, _host: &str, port: &str) -> io::Result<Connection> {
        let mut bus = self.bus.borrow_mut();
        let listener = bus
            .listeners
            .remove(port)
            .ok_or(io::ErrorKind::ConnectionRefused)?;
        let client = bus.allocate();
        bus.streams.get_mut(&listener).unwrap().peer = Some(client);
        bus.streams.get_mut(&client).unwrap().peer = Some(listener);
        Ok(client)
    }
    fn listen(&mut self, port: &str) -> io::Result<Connection> {
        let mut bus = self.bus.borrow_mut();
        if bus.listeners.contains_key(port) {
            return Err(io::ErrorKind::AddrInUse.into());
        }
        let listener = bus.allocate();
        bus.listeners.insert(port.to_owned(), listener);
        Ok(listener)
    }
    fn poll(&mut self, connection: Connection) -> io::Result<bool> {
        Ok(self.bus.borrow().streams[&connection].peer.is_some())
    }
    fn send(&mut self, connection: Connection, bytes: &[u8]) -> io::Result<usize> {
        let mut bus = self.bus.borrow_mut();
        let peer = bus.streams[&connection].peer.ok_or_else(would_block)?;
        let peer = bus.streams.get_mut(&peer).unwrap();
        if peer.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        peer.incoming.extend(bytes);
        Ok(bytes.len())
    }
    fn recv(&mut self, connection: Connection, bytes: &mut [u8]) -> io::Result<usize> {
        let mut bus = self.bus.borrow_mut();
        let stream = &bus.streams[&connection];
        let peer = stream.peer.ok_or_else(would_block)?;
        let peer_closed = bus.streams[&peer].closed;
        let stream = bus.streams.get_mut(&connection).unwrap();
        let count = stream.incoming.len().min(bytes.len());
        if count == 0 && !bytes.is_empty() && !peer_closed {
            return Err(would_block());
        }
        for byte in &mut bytes[..count] {
            *byte = stream.incoming.pop_front().unwrap();
        }
        Ok(count)
    }
    fn close(&mut self, connection: Connection) {
        let mut bus = self.bus.borrow_mut();
        bus.streams.get_mut(&connection).unwrap().closed = true;
        bus.listeners.retain(|_, listener| *listener != connection);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_fifo_partial_reads_and_peer_close() {
        let (mut a, mut b) = LoopbackNet::pair();
        let server = a.listen("21837").unwrap();
        assert!(!a.poll(server).unwrap());
        let client = b.connect("localhost", "21837").unwrap();
        assert!(a.poll(server).unwrap());
        assert!(b.poll(client).unwrap());
        let mut bytes = [0; 2];
        assert_eq!(
            a.recv(server, &mut bytes).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(b.send(client, &[1, 2, 3]).unwrap(), 3);
        assert_eq!(a.recv(server, &mut bytes).unwrap(), 2);
        assert_eq!(bytes, [1, 2]);
        b.close(client);
        assert_eq!(a.recv(server, &mut bytes).unwrap(), 1);
        assert_eq!(bytes[0], 3);
        assert_eq!(a.recv(server, &mut bytes).unwrap(), 0);
        assert_eq!(
            a.send(server, &[4]).unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        a.close(server);
    }
}
