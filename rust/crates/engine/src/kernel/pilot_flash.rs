//! Persistent phase and deadline of the flagship's navigation message.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PilotUpdate {
    DefaultMessage,
    Pulse(u8),
}

#[derive(Default)]
pub struct PilotFlash {
    last_on: bool,
    next_time: u32,
    cycle: u8,
}

impl PilotFlash {
    pub fn reset(&mut self) {
        self.last_on = false;
    }

    pub fn update(
        &mut self,
        on: bool,
        can_draw: bool,
        mut now: impl FnMut() -> u32,
        mut draw: impl FnMut(PilotUpdate),
    ) {
        if !on && !self.last_on {
            return;
        }
        if !on {
            draw(PilotUpdate::DefaultMessage);
            self.cycle = 0;
        } else if now() >= self.next_time {
            if can_draw {
                draw(PilotUpdate::Pulse(self.cycle));
            }
            self.cycle = (self.cycle + 1) % 7;
            self.next_time = now().wrapping_add(840 * 3 / 40);
        }
        self.last_on = on;
    }
}

#[cfg(test)]
mod tests {
    use super::{PilotFlash, PilotUpdate};

    impl PilotFlash {
        fn advance(
            &mut self,
            on: bool,
            can_draw: bool,
            now: impl FnMut() -> u32,
        ) -> Option<PilotUpdate> {
            let mut update = None;
            self.update(on, can_draw, now, |value| update = Some(value));
            update
        }
    }

    #[test]
    fn deadline_is_sampled_after_the_draw() {
        let events = std::cell::RefCell::new(Vec::new());
        PilotFlash::default().update(
            true,
            true,
            || {
                events.borrow_mut().push("clock");
                0
            },
            |_| events.borrow_mut().push("draw"),
        );
        assert_eq!(*events.borrow(), ["clock", "draw", "clock"]);
    }

    #[test]
    fn phase_and_deadline_belong_to_the_kernel_and_survive_reset() {
        let first = crate::Kernel::new();
        let other = crate::Kernel::new();
        let mut samples = [10, 12].into_iter();
        assert_eq!(
            first
                .pilot_flash
                .borrow_mut()
                .advance(true, true, || samples.next().unwrap()),
            Some(PilotUpdate::Pulse(0))
        );
        assert_eq!(first.pilot_flash.borrow_mut().next_time, 75);
        assert_eq!(first.pilot_flash.borrow_mut().cycle, 1);
        assert_eq!(
            other.pilot_flash.borrow_mut().advance(true, true, || 0),
            Some(PilotUpdate::Pulse(0))
        );
        first.pilot_flash.borrow_mut().reset();
        assert!(!first.pilot_flash.borrow_mut().last_on);
        assert_eq!(first.pilot_flash.borrow_mut().cycle, 1);
        assert_eq!(first.pilot_flash.borrow_mut().next_time, 75);
        let mut checks = 0;
        assert_eq!(
            first.pilot_flash.borrow_mut().advance(true, true, || {
                checks += 1;
                74
            }),
            None
        );
        assert_eq!(checks, 1);
        assert_eq!(
            first.pilot_flash.borrow_mut().advance(true, true, || 75),
            Some(PilotUpdate::Pulse(1))
        );
    }

    #[test]
    fn inactive_does_not_sample_time_and_the_last_message_is_cleared_once() {
        let mut flash = PilotFlash::default();
        assert_eq!(
            flash.advance(false, true, || panic!("inactive clock sample")),
            None
        );
        assert_eq!(flash.advance(true, true, || 0), Some(PilotUpdate::Pulse(0)));
        assert_eq!(
            flash.advance(false, false, || panic!("clear clock sample")),
            Some(PilotUpdate::DefaultMessage)
        );
        assert_eq!(flash.cycle, 0);
        assert_eq!(flash.next_time, 63);
        assert_eq!(
            flash.advance(false, true, || panic!("already cleared")),
            None
        );
    }

    #[test]
    fn suppressed_drawing_still_advances_and_cycles_all_seven_colors() {
        let mut flash = PilotFlash::default();
        assert_eq!(flash.advance(true, false, || 1), None);
        assert_eq!(flash.cycle, 1);
        assert!(flash.last_on);
        for expected in (1..7).chain(0..2) {
            let now = flash.next_time;
            assert_eq!(
                flash.advance(true, true, || now),
                Some(PilotUpdate::Pulse(expected))
            );
        }
    }

    #[test]
    fn deadlines_keep_wrapping_counter_behavior() {
        let mut flash = PilotFlash::default();
        assert_eq!(
            flash.advance(true, true, || u32::MAX - 2),
            Some(PilotUpdate::Pulse(0))
        );
        assert_eq!(flash.next_time, 60);
        assert_eq!(flash.advance(true, true, || 59), None);
        assert_eq!(
            flash.advance(true, true, || 60),
            Some(PilotUpdate::Pulse(1))
        );
    }
}
