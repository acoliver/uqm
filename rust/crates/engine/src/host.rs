//! Host ports used by engine clients without exposing native socket handles.
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Connection(pub u64);

/// One game-thread network endpoint. Connect and listen start asynchronous work;
/// poll reports when the connection or the first accepted peer is ready.
pub trait Net {
    fn connect(&mut self, host: &str, port: &str) -> io::Result<Connection>;
    fn listen(&mut self, port: &str) -> io::Result<Connection>;
    fn poll(&mut self, connection: Connection) -> io::Result<bool>;
    fn send(&mut self, connection: Connection, bytes: &[u8]) -> io::Result<usize>;
    fn recv(&mut self, connection: Connection, bytes: &mut [u8]) -> io::Result<usize>;
    fn close(&mut self, connection: Connection);
}

/// Construct the OS transport at the host boundary; modes only retain the port.
pub fn native_net() -> Box<dyn Net> {
    Box::new(crate::network::NativeNet::default())
}
