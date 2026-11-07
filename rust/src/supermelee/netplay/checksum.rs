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

pub fn crc_process_game_state(cx: &crate::engine::battle::Cx<'_>, state: &mut CrcState) {
    let seed = crate::math::seed_random(0);
    crc_process_u32(state, seed);
    crate::math::seed_random(seed);
    let mut link = cx.elements().head();
    while let Some(id) = link {
        crc_process_element(state, &cx.element(id));
        link = cx.elements().succ(id);
    }
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
    #[serial_test::serial]
    fn game_checksum_uses_the_supplied_context_and_preserves_the_rng() {
        let mut game = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let cx = crate::engine::battle::Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let previous = crate::math::seed_random(1234);
        let mut expected = CrcState::new();
        crc_process_u32(&mut expected, 1234);
        let mut actual = CrcState::new();
        crc_process_game_state(&cx, &mut actual);
        let seed_after = crate::math::seed_random(previous);
        assert_eq!(seed_after, 1234);
        assert_eq!(crc_finish(&actual), crc_finish(&expected));
        assert!(cx.elements().head().is_none());
    }

    #[test]
    fn c_element_checksum_vector_excludes_background_objects() {
        use crate::engine::battle::views::{FrameSelection, Frames, Image};
        let visual = Visual {
            location: POINT { x: 0, y: 0 },
            image: Image {
                frame: FrameSelection::ABSENT,
                frames: Frames::default(),
            },
        };
        let mut element = ElementState {
            player: 0,
            flags: 0,
            life: 0,
            hits: 0,
            mass: 0,
            turn: 0,
            thrust: 0,
            color_cycle: 0,
            velocity: crate::battle::velocity::VelocityDesc::new(),
            current: visual.clone(),
            next: visual,
            pre: None,
            post: None,
            collide: None,
            death: None,
        };
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
