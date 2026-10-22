use super::abi::*;
use super::checkbuf::*;
use super::crc::*;

#[no_mangle]
pub static mut localChecksumBuffer: ChecksumBuffer = ChecksumBuffer {
    checksums: std::ptr::null_mut(),
    maxSize: 0,
    interval: 0,
};

extern "C" {
    static mut netConnections: [*mut NetConnection; 2];
}

/// # Safety
/// Pointers must refer to a writable CRC state and a readable extent.
#[no_mangle]
pub unsafe extern "C" fn crc_processEXTENT(state: *mut crc_State, value: *const EXTENT) {
    crc_processUint16(state, (*value).width as u16);
    crc_processUint16(state, (*value).height as u16);
}

/// # Safety
/// Pointers must refer to a writable CRC state and a readable velocity descriptor.
#[no_mangle]
pub unsafe extern "C" fn crc_processVELOCITY_DESC(
    state: *mut crc_State,
    value: *const VELOCITY_DESC,
) {
    crc_processUint16(state, (*value).TravelAngle);
    crc_processEXTENT(state, &(*value).vector);
    crc_processEXTENT(state, &(*value).fract);
    crc_processEXTENT(state, &(*value).error);
    crc_processEXTENT(state, &(*value).incr);
}

/// # Safety
/// Pointers must refer to a writable CRC state and a readable point.
#[no_mangle]
pub unsafe extern "C" fn crc_processPOINT(state: *mut crc_State, value: *const POINT) {
    crc_processUint16(state, (*value).x as u16);
    crc_processUint16(state, (*value).y as u16);
}

/// # Safety
/// Pointers must refer to a writable CRC state and a readable element state.
#[no_mangle]
pub unsafe extern "C" fn crc_processSTATE(state: *mut crc_State, value: *const STATE) {
    crc_processPOINT(state, &(*value).location);
}

/// # Safety
/// Pointers must refer to a writable CRC state and a readable element.
#[no_mangle]
pub unsafe extern "C" fn crc_processELEMENT(state: *mut crc_State, value: *const ELEMENT) {
    let value = &*value;
    if value.state_flags & (1 << 15) != 0 {
        return;
    }
    crc_processUint16(state, value.state_flags);
    crc_processUint16(state, value.__bindgen_anon_1.life_span);
    crc_processUint16(state, value.__bindgen_anon_2.crew_level);
    crc_processUint8(state, value.__bindgen_anon_3.mass_points);
    crc_processUint8(state, value.__bindgen_anon_4.turn_wait);
    crc_processUint8(state, value.__bindgen_anon_5.thrust_wait);
    crc_processVELOCITY_DESC(state, &value.velocity);
    crc_processSTATE(state, &value.current);
    crc_processSTATE(state, &value.next);
}

/// # Safety
/// `state` must be writable; the display queue must not be mutated during traversal.
#[no_mangle]
pub unsafe extern "C" fn crc_processDispQueue(state: *mut crc_State) {
    let mut handle = disp_q.head;
    while !handle.is_null() {
        let element = handle.cast::<ELEMENT>();
        crc_processELEMENT(state, element);
        handle = (*element).succ;
    }
}

/// # Safety
/// `state` must be writable; called on the game thread owning the RNG.
#[no_mangle]
pub unsafe extern "C" fn crc_processRNG(state: *mut crc_State) {
    let seed = TFB_SeedRandom(0);
    crc_processUint32(state, seed);
    TFB_SeedRandom(seed);
}

/// # Safety
/// Called on the game thread with a writable CRC state and stable display queue.
#[no_mangle]
pub unsafe extern "C" fn crc_processState(state: *mut crc_State) {
    crc_processRNG(state);
    crc_processDispQueue(state);
}

/// # Safety
/// Called on the game thread before recording checksums, with live connections.
#[no_mangle]
pub unsafe extern "C" fn initChecksumBuffers() {
    for conn in netConnections {
        if !conn.is_null() {
            ChecksumBuffer_init(
                NetConnection_getChecksumBuffer(conn),
                getBattleInputDelay(),
                NETPLAY_CHECKSUM_INTERVAL as usize,
            );
        }
    }
    ChecksumBuffer_init(
        &raw mut localChecksumBuffer,
        getBattleInputDelay(),
        NETPLAY_CHECKSUM_INTERVAL as usize,
    );
}

/// # Safety
/// Called on the game thread after checksum recording has stopped.
#[no_mangle]
pub unsafe extern "C" fn uninitChecksumBuffers() {
    for conn in netConnections {
        if !conn.is_null() {
            ChecksumBuffer_uninit(NetConnection_getChecksumBuffer(conn));
        }
    }
    ChecksumBuffer_uninit(&raw mut localChecksumBuffer);
}

/// # Safety
/// Called on the game thread with initialized buffers for the current frame.
#[no_mangle]
pub unsafe extern "C" fn addLocalChecksum(frame: u32, checksum: u32) {
    assert_eq!(frame, { battleFrameCount });
    ChecksumBuffer_addChecksum(&raw mut localChecksumBuffer, frame, checksum);
}

/// # Safety
/// `conn` must be live with an initialized checksum buffer.
#[no_mangle]
pub unsafe extern "C" fn addRemoteChecksum(conn: *mut NetConnection, frame: u32, checksum: u32) {
    let delay = getBattleInputDelay();
    assert!(frame as usize <= battleFrameCount as usize + delay + 1);
    assert!(frame as usize + delay >= battleFrameCount as usize);
    ChecksumBuffer_addChecksum(NetConnection_getChecksumBuffer(conn), frame, checksum);
}

/// # Safety
/// Called on the game thread for a frame present in all initialized buffers.
#[no_mangle]
pub unsafe extern "C" fn verifyChecksums(frame: u32) -> bool {
    let mut local = 0;
    if !ChecksumBuffer_getChecksum(&raw mut localChecksumBuffer, frame, &mut local) {
        return false;
    }
    for conn in netConnections {
        if conn.is_null() {
            continue;
        }
        let mut remote = 0;
        if !ChecksumBuffer_getChecksum(NetConnection_getChecksumBuffer(conn), frame, &mut remote) {
            return false;
        }
        if local != remote {
            log::error!("Network connections have gone out of sync.");
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_element_checksum_vector_excludes_background_objects() {
        unsafe {
            let mut element: ELEMENT = std::mem::zeroed();
            element.state_flags = 0x4567;
            element.__bindgen_anon_1.life_span = 1234;
            element.__bindgen_anon_2.crew_level = 5678;
            element.__bindgen_anon_3.mass_points = 255;
            element.__bindgen_anon_4.turn_wait = 17;
            element.__bindgen_anon_5.thrust_wait = 33;
            element.velocity.TravelAngle = 42;
            element.velocity.vector = EXTENT {
                width: -123,
                height: 456,
            };
            element.velocity.fract = EXTENT {
                width: -1,
                height: 32767,
            };
            element.velocity.error = EXTENT {
                width: -32768,
                height: 123,
            };
            element.velocity.incr = EXTENT {
                width: 15,
                height: -25,
            };
            element.current.location = POINT { x: -456, y: 789 };
            element.next.location = POINT {
                x: 30000,
                y: -30000,
            };
            let mut state = crc_State { crc: 0 };
            crc_init(&mut state);
            crc_processELEMENT(&mut state, &element);
            // Captured from the original checksum.c and crc.c integer routines.
            assert_eq!(crc_finish(&state), 0x6850e519);
            element.state_flags |= 1 << 15;
            crc_processELEMENT(&mut state, &element);
            assert_eq!(crc_finish(&state), 0x6850e519);
        }
    }
}
