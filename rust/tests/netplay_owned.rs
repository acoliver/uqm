//! Ownership contracts at the protocol's stream boundary.
use std::io;
use uqm_rust::supermelee::netplay::{
    checkbuf::ChecksumBuffer, packet::Packet, packetq::PacketQueue,
};

#[test]
fn short_writes_keep_the_unsent_suffix_and_fifo_order() {
    let mut queue = PacketQueue::default();
    queue.push(Packet::Ping(1));
    queue.push(Packet::Ping(2));
    let first = Packet::Ping(1).encode();
    let second = Packet::Ping(2).encode();
    let mut wire = Vec::new();
    let mut calls = 0;
    assert_eq!(
        queue
            .flush(|bytes| {
                calls += 1;
                if calls == 2 {
                    return Err(io::ErrorKind::WouldBlock.into());
                }
                wire.extend_from_slice(&bytes[..3]);
                Ok(3)
            })
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(wire, first[..3]);
    assert_eq!(queue.len(), 2);
    queue.push(Packet::Ping(3));
    queue
        .flush(|bytes| {
            wire.extend_from_slice(bytes);
            Ok(bytes.len())
        })
        .unwrap();
    assert_eq!(wire, [first, second, Packet::Ping(3).encode()].concat());
    assert!(queue.is_empty());
}

#[test]
fn checksum_storage_preserves_interval_rounding_and_cyclic_reuse() {
    for delay in 0..9 {
        for interval in 1..5 {
            let mut checksums = ChecksumBuffer::new(delay, interval);
            assert_eq!(checksums.len(), (2 * delay + 2).div_ceil(interval));
            for frame in (0..100).step_by(interval) {
                checksums.add(frame as u32, frame as u32 ^ 0x1234);
                assert_eq!(checksums.get(frame as u32), frame as u32 ^ 0x1234);
            }
        }
    }
}

#[test]
fn completed_packet_statistics_survive_a_later_short_write_failure() {
    let mut queue = PacketQueue::default();
    queue.push(Packet::Ping(1));
    queue.push(Packet::Ping(2));
    let mut calls = 0;
    assert_eq!(
        queue
            .flush(|bytes| {
                calls += 1;
                match calls {
                    1 => Ok(bytes.len()),
                    2 => Ok(3),
                    _ => Err(io::ErrorKind::WouldBlock.into()),
                }
            })
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(queue.sent_by_type[1], 1);
    assert_eq!(queue.len(), 1);
    queue.flush(|bytes| Ok(bytes.len())).unwrap();
    assert_eq!(queue.sent_by_type[1], 2);
    assert!(queue.is_empty());
}
