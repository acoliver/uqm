//! FIFO packet ownership, including a partially written front packet.
use super::packet::Packet;
use std::{collections::VecDeque, io};

#[derive(Debug, Default)]
pub struct PacketQueue {
    packets: VecDeque<Packet>,
    offset: usize,
    pub sent_by_type: [usize; 18],
}
impl PacketQueue {
    pub fn push(&mut self, packet: Packet) {
        self.packets.push_back(packet);
    }
    pub fn len(&self) -> usize {
        self.packets.len()
    }
    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }
    pub fn front(&self) -> Option<&Packet> {
        self.packets.front()
    }
    pub fn flush(&mut self, mut send: impl FnMut(&[u8]) -> io::Result<usize>) -> io::Result<()> {
        while let Some(packet) = self.packets.front() {
            let bytes = packet.encode();
            while self.offset < bytes.len() {
                match send(&bytes[self.offset..]) {
                    Ok(0) => return Err(io::ErrorKind::BrokenPipe.into()),
                    Ok(count) => self.offset += count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => return Err(error),
                }
            }
            self.sent_by_type[packet.kind()] += 1;
            self.offset = 0;
            self.packets.pop_front();
        }
        Ok(())
    }
}
