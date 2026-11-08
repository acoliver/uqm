//! Caller-owned effects requested by one synchronous special-system pickup.

/// Effects emitted while one selected-system pickup callback is running.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpecialPickupEffects {
    pub crew_killed: u8,
    pub takeoff_requested: bool,
}

/// Generation and shot-retirement calls have no lander to affect.
#[derive(Debug)]
pub enum PickupContext {
    NoLander,
    Lander {
        remaining_crew: u8,
        effects: SpecialPickupEffects,
    },
}

impl PickupContext {
    #[must_use]
    pub fn lander(crew: u8) -> Self {
        Self::Lander {
            remaining_crew: crew,
            effects: SpecialPickupEffects::default(),
        }
    }

    pub fn request_takeoff(&mut self) {
        if let Self::Lander { effects, .. } = self {
            effects.takeoff_requested = true;
        }
    }

    /// The interval remains presentation-only; crew effects are synchronous.
    pub fn kill_crew(&mut self, count: u16, _interval_ticks: u32) -> bool {
        let Self::Lander {
            remaining_crew,
            effects,
        } = self
        else {
            return false;
        };
        let requested = u8::try_from(count).unwrap_or(u8::MAX);
        let killed = (*remaining_crew).min(requested);
        *remaining_crew -= killed;
        effects.crew_killed += killed;
        *remaining_crew != 0
    }

    #[must_use]
    pub fn effects(&self) -> SpecialPickupEffects {
        match self {
            Self::NoLander => SpecialPickupEffects::default(),
            Self::Lander { effects, .. } => *effects,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_and_sequential_pickups_do_not_share_effects() {
        let mut first = PickupContext::lander(5);
        let mut second = PickupContext::lander(12);
        assert!(first.kill_crew(2, 10));
        first.request_takeoff();
        assert_eq!(second.effects(), SpecialPickupEffects::default());
        assert!(second.kill_crew(8, 10));
        assert_eq!(second.effects().crew_killed, 8);
        assert_eq!(
            first.effects(),
            SpecialPickupEffects {
                crew_killed: 2,
                takeoff_requested: true,
            }
        );
        assert_eq!(
            PickupContext::lander(5).effects(),
            SpecialPickupEffects::default()
        );
    }

    #[test]
    fn multiple_crew_requests_saturate_at_remaining_crew() {
        let mut pickup = PickupContext::lander(u8::MAX);
        assert!(pickup.kill_crew(0, 10));
        assert!(pickup.kill_crew(2, 10));
        assert!(!pickup.kill_crew(u16::MAX, 10));
        assert!(!pickup.kill_crew(1, 10));
        assert_eq!(pickup.effects().crew_killed, u8::MAX);
        assert!(!PickupContext::lander(0).kill_crew(0, 10));
    }

    #[test]
    fn no_lander_has_no_crew_or_takeoff_effects() {
        let mut pickup = PickupContext::NoLander;
        pickup.request_takeoff();
        assert!(!pickup.kill_crew(1, 10));
        assert_eq!(pickup.effects(), SpecialPickupEffects::default());
    }
}
