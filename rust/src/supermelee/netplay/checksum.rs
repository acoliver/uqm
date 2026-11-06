use super::abi::*;
use super::checkbuf::*;
use super::crc::*;
use crate::battle::netplay::CrcState;
use crate::engine::battle::views::{ElementState, Visual};

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

pub fn crc_process_element_state(state: &mut CrcState, value: &Visual) {
    crc_process_point(state, &value.location);
}

pub fn crc_process_element(state: &mut CrcState, value: &ElementState) {
    if value.flags & (1 << 15) != 0 {
        return;
    }
    crc_process_u16(state, value.flags);
    crc_process_u16(state, value.life);
    crc_process_u16(state, value.hits);
    crc_process_u8(state, value.mass);
    crc_process_u8(state, value.turn);
    crc_process_u8(state, value.thrust);
    crc_process_velocity(state, &value.velocity);
    crc_process_element_state(state, &value.current);
    crc_process_element_state(state, &value.next);
}

/// # Safety
/// `state` must be writable; the display queue must not be mutated during traversal.
#[cfg(not(test))]
pub unsafe fn crc_process_display_queue(state: &mut CrcState) {
    crate::app::bridge::enter(|cx| {
        let mut link = cx.elements().head();
        while let Some(id) = link {
            crc_process_element(state, &cx.element(id));
            link = cx.elements().succ(id);
        }
    });
}

/// # Safety
/// `state` must be writable; called on the game thread owning the RNG.
#[cfg(not(test))]
pub unsafe fn crc_process_rng(state: &mut CrcState) {
    let seed = crate::math::seed_random(0);
    crc_process_u32(state, seed);
    crate::math::seed_random(seed);
}

/// # Safety
/// Called on the game thread with a writable CRC state and stable display queue.
#[cfg(not(test))]
pub unsafe fn crc_process_game_state(state: &mut CrcState) {
    crc_process_rng(state);
    crc_process_display_queue(state);
}

impl super::session::SessionCore {
    pub fn init_checksum_buffers(&self) {
        for side in 0..2 {
            if let Some(mut conn) = self.connection_mut(side) {
                conn.checksums = ChecksumBuffer::new(
                    self.get_battle_input_delay(),
                    NETPLAY_CHECKSUM_INTERVAL as usize,
                );
            }
        }
        *self.checksums.borrow_mut() = ChecksumBuffer::new(
            self.get_battle_input_delay(),
            NETPLAY_CHECKSUM_INTERVAL as usize,
        );
    }
    pub fn uninit_checksum_buffers(&self) {
        for side in 0..2 {
            if let Some(mut conn) = self.connection_mut(side) {
                conn.checksums = ChecksumBuffer::default();
            }
        }
        *self.checksums.borrow_mut() = ChecksumBuffer::default();
    }
    pub fn add_local_checksum(&self, frame: u32, checksum: u32) {
        assert_eq!(frame, self.frame_count.get());
        self.checksums.borrow_mut().add(frame, checksum);
    }
    pub fn verify_checksums(&self, frame: u32) -> bool {
        let local = self.checksums.borrow().get(frame);
        for side in 0..2 {
            if self
                .connection(side)
                .is_some_and(|conn| conn.checksums.get(frame) != local)
            {
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
        // SAFETY: this ABI record contains only zero-valid scalar, union and pointer fields.
        let mut element: ElementState = unsafe { std::mem::zeroed() };
        element.flags = 0x4567;
        element.life = 1234;
        element.hits = 5678;
        element.mass = 255;
        element.turn = 17;
        element.thrust = 33;
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
        element.flags |= 1 << 15;
        crc_process_element(&mut state, &element);
        assert_eq!(crc_finish(&state), 0x6850e519);
    }
}
