//! Escort highlight phase, retained across separate roster visits.

use crate::graphics::native_abi::Color;

pub struct RosterFlash {
    color: Color,
    next_time: u32,
}

impl Default for RosterFlash {
    fn default() -> Self {
        Self {
            color: Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            next_time: 0,
        }
    }
}

impl RosterFlash {
    pub fn advance(&mut self, mut now: impl FnMut() -> u32) -> Option<Color> {
        if now() < self.next_time {
            return None;
        }
        self.next_time = now().wrapping_add(840 / 15);
        if self.color.g >= (25 << 3) | (25 >> 2) {
            self.color.g = 0;
            self.color.b = 0;
        } else {
            self.color.g += 2 << 3;
            self.color.b += 2 << 3;
        }
        Some(self.color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visits_retain_phase_deadline_and_the_two_original_clock_reads() {
        let mut flash = RosterFlash::default();
        let mut times = [100, 101].into_iter();
        assert_eq!(flash.advance(|| times.next().unwrap()).unwrap().g, 16);
        assert!(times.next().is_none());
        let mut calls = 0;
        assert!(flash
            .advance(|| {
                calls += 1;
                156
            })
            .is_none());
        assert_eq!(calls, 1);
        assert_eq!(flash.advance(|| 157).unwrap().g, 32);
        for index in 3..=13 {
            assert_eq!(flash.advance(|| index * 100).unwrap().g, (index * 16) as u8);
        }
        assert_eq!(flash.advance(|| 1400).unwrap().g, 0);
        let mut independent = RosterFlash::default();
        assert_eq!(independent.advance(|| 0).unwrap().g, 16);
        assert_eq!(flash.advance(|| 1500).unwrap().g, 16);
    }

    #[test]
    fn wrapped_deadlines_keep_the_original_unsigned_comparison() {
        let mut flash = RosterFlash::default();
        assert!(flash.advance(|| u32::MAX).is_some());
        assert!(flash.advance(|| 54).is_none());
        assert!(flash.advance(|| 55).is_some());
    }
}
