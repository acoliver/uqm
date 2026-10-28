use super::abi::*;

/// # Safety
/// `conn` is connected and `packet` contains its declared wire length.
#[no_mangle]
pub unsafe extern "C" fn sendPacket(conn: *mut NetConnection, packet: *mut Packet) -> i32 {
    // SAFETY: the function contract requires live, game-thread-owned records
    // and callback arguments for all legacy accesses in this operation.
    unsafe {
        assert!((*conn).stateFlags.connected);
        let socket = (*conn)
            .socket
            .unwrap_or_else(|| panic!("send without transport"));
        let bytes = (*packet).encode();
        while (*conn).write_offset < bytes.len() {
            let result = (*(*conn).session)
                .transport
                .borrow_mut()
                .send(socket, &bytes[(*conn).write_offset..]);
            match result {
                Ok(0) => {
                    super::set_errno(libc::EPIPE);
                    return -1;
                }
                Ok(count) => (*conn).write_offset += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let err = if error.kind() == std::io::ErrorKind::WouldBlock {
                        libc::EAGAIN
                    } else {
                        error.raw_os_error().unwrap_or(libc::EIO)
                    };
                    super::set_errno(err);
                    return -1;
                }
            }
        }
        (*conn).write_offset = 0;
        (*conn).statistics.packetsSent += 1;
        (*conn).statistics.packetTypeSent[(*packet).kind()] += 1;
        0
    }
}
