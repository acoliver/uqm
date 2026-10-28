use super::abi::*;
use super::netconnection::NetConnection_doErrorCallback;
use super::packet::{DecodeError, Packet};

fn packet_frame_length(data: &[u8]) -> isize {
    match Packet::decode(data) {
        Ok((_, length)) => length as isize,
        Err(DecodeError::Incomplete) => 0,
        Err(_) => {
            unsafe { super::set_errno(libc::EBADMSG) };
            -1
        }
    }
}

unsafe fn received_single(conn: *mut NetConnection, data: *const u8, len: usize) -> isize {
    let bytes = std::slice::from_raw_parts(data, len);
    match Packet::decode(bytes) {
        Ok((packet, length)) => {
            (*conn).statistics.packetsReceived += 1;
            (*conn).statistics.packetTypeReceived[packet.kind()] += 1;
            if super::packethandlers::dispatch(conn, &packet) == -1 {
                -1
            } else {
                length as isize
            }
        }
        Err(DecodeError::Incomplete) => 0,
        Err(_) => {
            super::set_errno(libc::EBADMSG);
            -1
        }
    }
}

unsafe fn received_multi(conn: *mut NetConnection, data: *const u8, len: usize) -> isize {
    let mut processed = 0;
    while processed < len {
        let result = received_single(conn, data.add(processed), len - processed);
        if result == -1 {
            return -1;
        }
        if result == 0 {
            break;
        }
        processed += result as usize;
    }
    processed as isize
}

/// # Safety
/// `nd` belongs to a live connection with its receive buffer, on the event thread.
pub(super) unsafe fn receive(conn: *mut NetConnection) {
    let socket = (*conn).socket.expect("receive without transport");
    loop {
        let used = (*conn).readEnd.offset_from((*conn).readBuf) as usize;
        let bytes =
            std::slice::from_raw_parts_mut((*conn).readEnd, NETPLAY_READBUFSIZE as usize - used);
        let result = (*(*conn).session)
            .transport
            .borrow_mut()
            .recv(socket, bytes);
        let result = match result {
            Ok(0) => {
                super::netconnection::disconnected(conn);
                return;
            }
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                NetConnection_doErrorCallback(conn, error.raw_os_error().unwrap_or(libc::EIO));
                super::netconnection::disconnected(conn);
                return;
            }
        };
        (*conn).readEnd = (*conn).readEnd.add(result as usize);
        let used = (*conn).readEnd.offset_from((*conn).readBuf) as usize;
        let processed = received_multi(conn, (*conn).readBuf, used);
        if processed == -1 {
            let err = crate::platform::errno();
            NetConnection_doErrorCallback(conn, err);
            super::netconnection::disconnected(conn);
            return;
        }
        if processed == 0 {
            return;
        }
        std::ptr::copy(
            (*conn).readBuf.add(processed as usize),
            (*conn).readBuf,
            used - processed as usize,
        );
        (*conn).readEnd = (*conn).readEnd.sub(processed as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_framing_waits_for_payload_and_rejects_bad_headers() {
        for data in [&[0, 4, 0][..], &[0, 8, 0, 11, 0][..]] {
            assert_eq!(packet_frame_length(data), 0);
        }
        for data in [[0, 4, 0, 18], [0, 4, 0, 11]] {
            assert_eq!(packet_frame_length(&data), -1);
        }
        assert_eq!(packet_frame_length(&[0, 8, 0, 11, 0, 0, 0, 0]), 8);
    }
}
