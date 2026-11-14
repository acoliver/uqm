//! Bidirectional credits scrolling and communication outtakes, ported from uqm/credits.c.

mod control;
mod render;
use control::{CreditsControl, CreditsKeys};
use uqm_engine::time::counter_abi::ONE_SECOND;

#[cfg(not(test))]
pub(crate) mod runtime;
#[cfg(not(test))]
pub use runtime::credits;
#[cfg(test)]
mod owned_tests;

const CREDITS_MAX_RATE: i32 = 130;
const CREDITS_FRAME_RATE: i32 = 36;
const MAX_CREDIT_FRAMES: usize = 32;

fn frame_index(index: i32) -> usize {
    ((index + MAX_CREDIT_FRAMES as i32) % MAX_CREDIT_FRAMES as i32) as usize
}

fn scroll_step(rate: i32) -> (i32, i32, u32) {
    let speed = rate.abs();
    if speed != 0 {
        let direction = rate / speed;
        let step = (speed + CREDITS_FRAME_RATE - 1) / CREDITS_FRAME_RATE;
        let delay = ONE_SECOND as i32 * step / speed;
        (direction, step * direction, delay as u32)
    } else {
        (0, 0, ONE_SECOND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_indices_wrap_in_both_directions() {
        assert_eq!(frame_index(-1), 31);
        assert_eq!(frame_index(0), 0);
        assert_eq!(frame_index(31), 31);
        assert_eq!(frame_index(32), 0);
        for index in 0..32 {
            assert_eq!(
                frame_index(frame_index(index + 1) as i32 - 1),
                index as usize
            );
        }
    }

    #[test]
    fn scrolling_uses_integer_frame_delay_and_directional_steps() {
        assert_eq!(scroll_step(0), (0, 0, 840));
        assert_eq!(scroll_step(9), (1, 1, 93));
        assert_eq!(scroll_step(-9), (-1, -1, 93));
        assert_eq!(scroll_step(36), (1, 1, 23));
        assert_eq!(scroll_step(37), (1, 2, 45));
        assert_eq!(scroll_step(130), (1, 4, 25));
        assert_eq!(scroll_step(-130), (-1, -4, 25));
        for rate in 1..=CREDITS_MAX_RATE {
            let (direction, step, delay) = scroll_step(rate);
            assert_eq!(direction, 1);
            assert_eq!(step, (rate + 35) / 36);
            assert_eq!(delay, (840 * step / rate) as u32);
            assert_eq!(scroll_step(-rate), (-1, -step, delay));
        }
    }
}
