//! Animation phase retained between orbit visits by one kernel.

use std::cell::{Cell, RefCell};

pub struct PlanetAnimation {
    rotation_deadline: Cell<u32>,
    topography_seed: Cell<u32>,
    cursor: RefCell<CursorPulse>,
}

impl Default for PlanetAnimation {
    fn default() -> Self {
        Self {
            rotation_deadline: Cell::new(0),
            topography_seed: Cell::new(0x12345678),
            cursor: RefCell::new(CursorPulse::default()),
        }
    }
}

impl PlanetAnimation {
    pub fn rotation_due(&self, now: u32, keep_rate: bool, width: i32) -> bool {
        if keep_rate && now < self.rotation_deadline.get() {
            return false;
        }
        self.rotation_deadline
            .set(now.wrapping_add((840 * 12 / width) as u32));
        true
    }

    pub fn topography_seed(&self) -> u32 {
        self.topography_seed.get()
    }

    pub fn set_topography_seed(&self, seed: u32) {
        self.topography_seed.set(seed);
    }

    pub fn cursor_shade(&self, position: (i16, i16), now: u32) -> Option<u8> {
        self.cursor.borrow_mut().advance(position, now)
    }
}

struct CursorPulse {
    shade: u8,
    delta: i32,
    previous: (i16, i16),
    next_time: u32,
}

impl Default for CursorPulse {
    fn default() -> Self {
        Self {
            shade: 0,
            delta: -2,
            previous: (0, 0),
            next_time: 0,
        }
    }
}

impl CursorPulse {
    fn advance(&mut self, position: (i16, i16), now: u32) -> Option<u8> {
        let changed = self.previous != position;
        if !changed && now < self.next_time {
            return None;
        }
        if changed {
            self.shade = 0;
            self.delta = -2;
            self.previous = position;
            self.next_time = now.wrapping_add(840 / 16);
        } else {
            if self.shade == 0 || self.shade == 0x1a {
                self.delta = -self.delta;
            }
            self.shade = (i32::from(self.shade) + self.delta) as u8;
            if now.wrapping_sub(self.next_time) > 840 / 16 {
                self.next_time = now.wrapping_add(840 / 16);
            } else {
                self.next_time = self.next_time.wrapping_add(840 / 16);
            }
        }
        Some(self.shade)
    }
}

#[cfg(test)]
mod tests {
    use crate::Kernel;

    #[test]
    fn retained_phase_survives_visits_without_crossing_kernel_owners() {
        let first = Kernel::new();
        let second = Kernel::new();
        let visit = first.planet_animation.clone();
        assert!(visit.rotation_due(0, true, 242));
        assert!(!visit.rotation_due(40, true, 242));
        assert!(visit.rotation_due(40, false, 242));
        assert!(!first.planet_animation.rotation_due(80, true, 242));
        assert!(first.planet_animation.rotation_due(81, true, 242));
        assert!(second.planet_animation.rotation_due(0, true, 242));
        assert_eq!(visit.topography_seed(), 0x12345678);
        visit.set_topography_seed(0xfeedbeef);
        assert_eq!(first.planet_animation.topography_seed(), 0xfeedbeef);
        assert_eq!(second.planet_animation.topography_seed(), 0x12345678);
        assert_eq!(visit.cursor_shade((0, 0), 0), Some(2));
        assert_eq!(first.planet_animation.cursor_shade((0, 0), 52), Some(4));
        assert_eq!(second.planet_animation.cursor_shade((0, 0), 0), Some(2));
    }

    #[test]
    fn cursor_motion_resets_shade_and_late_frames_keep_the_original_cadence() {
        let kernel = Kernel::new();
        let phase = &kernel.planet_animation;
        assert_eq!(phase.cursor_shade((1, 2), 10), Some(0));
        assert_eq!(phase.cursor_shade((1, 2), 61), None);
        assert_eq!(phase.cursor_shade((1, 2), 62), Some(2));
        assert_eq!(phase.cursor_shade((1, 2), 166), Some(4));
        assert_eq!(phase.cursor_shade((1, 2), 166), Some(6));
        assert_eq!(phase.cursor_shade((1, 2), 167), None);
        assert_eq!(phase.cursor_shade((3, 2), 167), Some(0));
        assert_eq!(phase.cursor_shade((3, 2), 218), None);
        assert_eq!(phase.cursor_shade((3, 2), 219), Some(2));
        assert_eq!(phase.cursor_shade((3, 2), 1000), Some(4));
        assert_eq!(phase.cursor_shade((3, 2), 1051), None);
        assert_eq!(phase.cursor_shade((3, 2), 1052), Some(6));
    }

    #[test]
    fn cursor_pulse_reflects_at_both_endpoints() {
        let kernel = Kernel::new();
        let phase = &kernel.planet_animation;
        let shades = (2..=26).step_by(2).chain((0..=24).rev().step_by(2));
        for (index, shade) in shades.enumerate() {
            assert_eq!(phase.cursor_shade((0, 0), index as u32 * 52), Some(shade));
        }
        assert_eq!(phase.cursor_shade((0, 0), 26 * 52), Some(2));
    }

    #[test]
    fn rotation_deadline_preserves_wrapping_clock_arithmetic() {
        let kernel = Kernel::new();
        let phase = &kernel.planet_animation;
        assert!(phase.rotation_due(u32::MAX - 10, false, 242));
        assert!(!phase.rotation_due(29, true, 242));
        assert!(phase.rotation_due(30, true, 242));
    }
}
