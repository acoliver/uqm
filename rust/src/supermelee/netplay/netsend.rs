use super::abi::*;

/// # Safety
/// `conn` is connected and `packet` contains its declared wire length.
#[no_mangle]
pub unsafe extern "C" fn sendPacket(conn: *mut NetConnection, packet: *mut Packet) -> i32 {
    assert!((*conn).stateFlags.connected);
    let socket = NetDescriptor_getSocket((*conn).nd);
    let mut len = u16::from_be((*packet).header.len) as usize;
    while len > 0 {
        // Preserve the existing transport's base pointer on each send.
        let result = Socket_send(socket, packet.cast(), len, 0);
        if result >= 0 {
            len -= result as usize;
            continue;
        }
        let err = crate::platform::errno();
        if err == libc::EINTR {
            continue;
        }
        if err != libc::ECONNRESET {
            log::error!("send() failed: {}.", std::io::Error::from_raw_os_error(err));
            super::set_errno(err);
        }
        return -1;
    }
    (*conn).statistics.packetsSent += 1;
    (*conn).statistics.packetTypeSent[u16::from_be((*packet).header.type_) as usize] += 1;
    0
}
