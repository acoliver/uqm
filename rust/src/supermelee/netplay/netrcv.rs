use super::abi::packetTypeData;
use super::abi::*;
use super::netconnection::NetConnection_doErrorCallback;

fn packet_frame_length(data: &[u8]) -> isize {
    if data.len() < size_of::<PacketHeader>() {
        return 0;
    }
    // The socket byte buffer need not be aligned for PacketHeader.
    let header = unsafe { data.as_ptr().cast::<PacketHeader>().read_unaligned() };
    let packet_len = u16::from_be(header.len) as usize;
    let kind = u16::from_be(header.type_) as usize;
    if kind >= PacketType_PACKET_NUM as usize {
        log::warn!("Packet with invalid type {kind} received.");
        // errno belongs to the calling thread.
        unsafe { super::set_errno(libc::EBADMSG) };
        return -1;
    }
    if packet_len < super::packet::minimum_length(kind) {
        log::warn!("Packet with bad length field received (type={kind}, lenfield={packet_len}).");
        // errno belongs to the calling thread.
        unsafe { super::set_errno(libc::EBADMSG) };
        return -1;
    }
    if data.len() < packet_len {
        return 0;
    }
    packet_len as isize
}

unsafe fn received_single(conn: *mut NetConnection, data: *const u8, len: usize) -> isize {
    let packet_len = packet_frame_length(std::slice::from_raw_parts(data, len));
    if packet_len <= 0 {
        return packet_len;
    }
    let header = data.cast::<PacketHeader>().read_unaligned();
    let kind = u16::from_be(header.type_) as usize;
    let entry = &*std::ptr::addr_of!(packetTypeData)
        .cast::<PacketTypeData>()
        .add(kind);
    (*conn).statistics.packetsReceived += 1;
    (*conn).statistics.packetTypeReceived[kind] += 1;
    if entry.handler.expect("packet handler")(conn, data.cast()) == -1 {
        return -1;
    }
    packet_len as isize
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
pub unsafe extern "C" fn data_ready_callback(nd: *mut NetDescriptor) {
    let conn = NetDescriptor_getExtra(nd).cast::<NetConnection>();
    let socket = NetDescriptor_getSocket(nd);
    loop {
        let used = (*conn).readEnd.offset_from((*conn).readBuf) as usize;
        let result = Socket_recv(
            socket,
            (*conn).readEnd.cast(),
            NETPLAY_READBUFSIZE as usize - used,
            0,
        );
        if result == 0 {
            NetDescriptor_close(nd);
            return;
        }
        if result == -1 {
            let err = std::io::Error::last_os_error()
                .raw_os_error()
                .expect("errno");
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                return;
            }
            if err == libc::EINTR {
                continue;
            }
            log::error!("recv() failed: {}.", std::io::Error::from_raw_os_error(err));
            NetConnection_doErrorCallback(conn, err);
            NetDescriptor_close(nd);
            return;
        }
        (*conn).readEnd = (*conn).readEnd.add(result as usize);
        let used = (*conn).readEnd.offset_from((*conn).readBuf) as usize;
        let processed = received_multi(conn, (*conn).readBuf, used);
        if processed == -1 {
            let err = std::io::Error::last_os_error()
                .raw_os_error()
                .expect("errno");
            NetConnection_doErrorCallback(conn, err);
            NetDescriptor_close(nd);
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
