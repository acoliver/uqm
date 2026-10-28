use super::abi::*;
use super::checkbuf::*;
use super::crc::*;
use crate::battle::netplay::CrcState;

pub fn crc_process_extent(state: &mut CrcState, value: &EXTENT) {
    crc_process_u16(state, value.width as u16);
    crc_process_u16(state, value.height as u16);
}

pub fn crc_process_velocity(state: &mut CrcState, value: &VELOCITY_DESC) {
    crc_process_u16(state, value.travel_angle);
    crc_process_extent(state, &value.vector);
    crc_process_extent(state, &value.fract);
    crc_process_extent(state, &value.error);
    crc_process_extent(state, &value.incr);
}

pub fn crc_process_point(state: &mut CrcState, value: &POINT) {
    crc_process_u16(state, value.x as u16);
    crc_process_u16(state, value.y as u16);
}

pub fn crc_process_element_state(state: &mut CrcState, value: &STATE) {
    crc_process_point(state, &value.location);
}

pub fn crc_process_element(state: &mut CrcState, value: &ELEMENT) {
    if value.state_flags & (1 << 15) != 0 {
        return;
    }
    crc_process_u16(state, value.state_flags);
    crc_process_u16(state, unsafe { value.__bindgen_anon_1.life_span });
    crc_process_u16(state, unsafe { value.__bindgen_anon_2.crew_level });
    crc_process_u8(state, unsafe { value.__bindgen_anon_3.mass_points });
    crc_process_u8(state, unsafe { value.__bindgen_anon_4.turn_wait });
    crc_process_u8(state, unsafe { value.__bindgen_anon_5.thrust_wait });
    crc_process_velocity(state, &value.velocity);
    crc_process_element_state(state, &value.current);
    crc_process_element_state(state, &value.next);
}

/// # Safety
/// `state` must be writable; the display queue must not be mutated during traversal.
pub unsafe fn crc_process_display_queue(state: &mut CrcState) {
    let mut handle = disp_q.head;
    while !handle.is_null() {
        let element = handle.cast::<ELEMENT>();
        crc_process_element(state, &*element);
        handle = (*element).succ;
    }
}

/// # Safety
/// `state` must be writable; called on the game thread owning the RNG.
pub unsafe fn crc_process_rng(state: &mut CrcState) {
    let seed = TFB_SeedRandom(0);
    crc_process_u32(state, seed);
    TFB_SeedRandom(seed);
}

/// # Safety
/// Called on the game thread with a writable CRC state and stable display queue.
pub unsafe fn crc_process_game_state(state: &mut CrcState) {
    crc_process_rng(state);
    crc_process_display_queue(state);
}

impl super::session::SessionCore {
    /// # Safety
    /// Called on the game thread before recording checksums, with live connections.
    pub unsafe fn init_checksum_buffers(&self) {
        for conn in self.connections.get() {
            if !conn.is_null() {
                ChecksumBuffer_init(
                    NetConnection_getChecksumBuffer(conn),
                    self.get_battle_input_delay(),
                    NETPLAY_CHECKSUM_INTERVAL as usize,
                );
            }
        }
        ChecksumBuffer_init(
            &mut *self.checksums.borrow_mut(),
            self.get_battle_input_delay(),
            NETPLAY_CHECKSUM_INTERVAL as usize,
        );
    }

    /// # Safety
    /// Called on the game thread after checksum recording has stopped.
    pub unsafe fn uninit_checksum_buffers(&self) {
        for conn in self.connections.get() {
            if !conn.is_null() {
                ChecksumBuffer_uninit(NetConnection_getChecksumBuffer(conn));
            }
        }
        ChecksumBuffer_uninit(&mut *self.checksums.borrow_mut());
    }

    /// # Safety
    /// Called on the game thread with initialized buffers for the current frame.
    pub unsafe fn add_local_checksum(&self, frame: u32, checksum: u32) {
        assert_eq!(frame, self.frame_count.get());
        ChecksumBuffer_addChecksum(&mut *self.checksums.borrow_mut(), frame, checksum);
    }

    /// # Safety
    /// `conn` must be live with an initialized checksum buffer.
    pub unsafe fn add_remote_checksum(&self, conn: *mut NetConnection, frame: u32, checksum: u32) {
        let delay = self.get_battle_input_delay();
        assert!(frame as usize <= self.frame_count.get() as usize + delay + 1);
        assert!(frame as usize + delay >= self.frame_count.get() as usize);
        ChecksumBuffer_addChecksum(NetConnection_getChecksumBuffer(conn), frame, checksum);
    }

    /// # Safety
    /// Called on the game thread for a frame present in all initialized buffers.
    pub unsafe fn verify_checksums(&self, frame: u32) -> bool {
        let mut local = 0;
        if !ChecksumBuffer_getChecksum(&mut *self.checksums.borrow_mut(), frame, &mut local) {
            return false;
        }
        for conn in self.connections.get() {
            if conn.is_null() {
                continue;
            }
            let mut remote = 0;
            if !ChecksumBuffer_getChecksum(
                NetConnection_getChecksumBuffer(conn),
                frame,
                &mut remote,
            ) {
                return false;
            }
            if local != remote {
                log::error!("Network connections have gone out of sync.");
                return false;
            }
        }
        true
    }
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
            element.velocity.travel_angle = 42;
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
            let mut state = CrcState::new();
            crc_init(&mut state);
            crc_process_element(&mut state, &element);
            // Captured from the original checksum.c and crc.c integer routines.
            assert_eq!(crc_finish(&state), 0x6850e519);
            element.state_flags |= 1 << 15;
            crc_process_element(&mut state, &element);
            assert_eq!(crc_finish(&state), 0x6850e519);
        }
    }
}
