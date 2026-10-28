use super::abi::{NetConnection, Packet, PacketQueue, PacketQueueLink};
use super::packet::packet_delete;
use std::ptr;

/// # Safety
/// `queue` must point to its final, writable location and not own existing links.
#[no_mangle]
pub unsafe extern "C" fn PacketQueue_init(queue: *mut PacketQueue) {
    (*queue).size = 0;
    (*queue).first = ptr::null_mut();
    (*queue).end = &raw mut (*queue).first;
}

/// # Safety
/// `queue` must be initialized and its packets must not be in use.
#[no_mangle]
pub unsafe extern "C" fn PacketQueue_uninit(queue: *mut PacketQueue) {
    let mut link = (*queue).first;
    while !link.is_null() {
        let next = (*link).next;
        packet_delete((*link).packet);
        libc::free(link.cast());
        link = next;
    }
}

/// # Safety
/// `conn` must be connected with an initialized queue. Ownership of `packet` is transferred.
pub unsafe fn queue_packet(conn: *mut NetConnection, packet: Packet) {
    assert!((*conn).stateFlags.connected);
    let queue = &raw mut (*conn).queue;
    let link = libc::malloc(size_of::<PacketQueueLink>()).cast::<PacketQueueLink>();
    assert!(!link.is_null());
    link.write(PacketQueueLink {
        packet: Box::into_raw(Box::new(packet)),
        next: ptr::null_mut(),
    });
    *(*queue).end = link;
    (*queue).end = &raw mut (*link).next;
    (*queue).size += 1;
}

unsafe fn flush(queue: *mut PacketQueue, mut send: impl FnMut(*mut Packet) -> i32) -> i32 {
    let mut link = (*queue).first;
    while !link.is_null() {
        if send((*link).packet) == -1 {
            (*queue).first = link;
            return -1;
        }
        let next = (*link).next;
        packet_delete((*link).packet);
        libc::free(link.cast());
        (*queue).size -= 1;
        link = next;
    }
    (*queue).first = ptr::null_mut();
    (*queue).end = &raw mut (*queue).first;
    0
}

/// # Safety
/// `conn` must be connected with an initialized queue, exclusively owned by the game thread.
#[no_mangle]
pub unsafe extern "C" fn flushPacketQueue(conn: *mut NetConnection) -> i32 {
    assert!((*conn).stateFlags.connected);
    flush(&raw mut (*conn).queue, |packet| {
        super::abi::sendPacket(conn, packet)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_retry_retains_unsent_suffix_and_resets_tail() {
        unsafe {
            let mut conn: NetConnection = std::mem::zeroed();
            conn.stateFlags.connected = true;
            PacketQueue_init(&raw mut conn.queue);
            for id in 1..=3 {
                queue_packet(&mut conn, Packet::Ping(id));
            }
            assert_eq!(conn.queue.size, 3);
            let mut sent = Vec::new();
            assert_eq!(
                flush(&raw mut conn.queue, |packet| {
                    let id = match &*packet {
                        Packet::Ping(id) => *id,
                        _ => unreachable!(),
                    };
                    if id == 2 {
                        return -1;
                    }
                    sent.push(id);
                    0
                }),
                -1
            );
            assert_eq!(sent, [1]);
            assert_eq!(conn.queue.size, 2);
            queue_packet(&mut conn, Packet::Ping(4));
            assert_eq!(
                flush(&raw mut conn.queue, |packet| {
                    sent.push(match &*packet {
                        Packet::Ping(id) => *id,
                        _ => unreachable!(),
                    });
                    0
                }),
                0
            );
            assert_eq!(sent, [1, 2, 3, 4]);
            assert_eq!(conn.queue.size, 0);
            assert!(conn.queue.first.is_null());
            assert_eq!(conn.queue.end, &raw mut conn.queue.first);
            queue_packet(&mut conn, Packet::Ping(5));
            assert_eq!(conn.queue.size, 1);
            PacketQueue_uninit(&raw mut conn.queue);
        }
    }
}
