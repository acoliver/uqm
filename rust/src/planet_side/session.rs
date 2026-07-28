//! Planet-side trip lifecycle and orbit writeback transaction.

use super::cargo::{BioCargo, MineralCargo, NUM_ELEMENT_CATEGORIES};
use super::model::CrewCount;
use super::simulation::LanderState;

/// Explicit phase of the lander trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Warmup,
    Launch,
    Landing,
    Active,
    TakingOff,
    Explosion,
    Return,
    Complete,
    Aborted,
}

/// Ship values captured before dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipStatus {
    pub crew_enlisted: u16,
    pub landers: u8,
    pub total_element_mass: u16,
    pub element_amounts: [u16; NUM_ELEMENT_CATEGORIES],
    pub total_bio_mass: u16,
}

/// Atomic ship mutation produced when a trip settles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipDelta {
    pub crew: i16,
    pub landers: i8,
    pub element_mass: u16,
    pub element_amounts: [u16; NUM_ELEMENT_CATEGORIES],
    pub biological_mass: u16,
}

impl Default for ShipDelta {
    fn default() -> Self {
        Self {
            crew: 0,
            landers: 0,
            element_mass: 0,
            element_amounts: [0; NUM_ELEMENT_CATEGORIES],
            biological_mass: 0,
        }
    }
}

/// Terminal result of one planet-side session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionOutcome {
    Returned(ShipDelta),
    LanderDestroyed(ShipDelta),
    Aborted,
}

/// Complete deterministic state of one lander trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetSideSession {
    pub phase: SessionPhase,
    pub lander: LanderState,
    pub minerals: MineralCargo,
    pub biological: BioCargo,
    settled: Option<SessionOutcome>,
}

impl PlanetSideSession {
    #[must_use]
    pub fn new(lander: LanderState, minerals: MineralCargo) -> Self {
        Self {
            phase: SessionPhase::Warmup,
            lander,
            minerals,
            biological: BioCargo::default(),
            settled: None,
        }
    }

    /// Transition into manual or scripted takeoff when crew remains.
    pub fn request_takeoff(&mut self) -> bool {
        if self.lander.crew.get() == 0
            || matches!(self.phase, SessionPhase::Aborted | SessionPhase::Complete)
        {
            false
        } else {
            self.lander.in_transit = true;
            self.phase = SessionPhase::TakingOff;
            true
        }
    }

    /// Abort the session without survivor/death writeback, matching legacy flow.
    pub fn abort(&mut self) {
        self.phase = SessionPhase::Aborted;
    }

    /// Produce the single orbit writeback transaction for the terminal state.
    #[must_use]
    pub fn settle(&mut self) -> SessionOutcome {
        if let Some(outcome) = &self.settled {
            return outcome.clone();
        }
        let outcome = if self.phase == SessionPhase::Aborted {
            SessionOutcome::Aborted
        } else if self.lander.crew.get() == 0 {
            SessionOutcome::LanderDestroyed(ShipDelta {
                landers: -1,
                ..ShipDelta::default()
            })
        } else {
            SessionOutcome::Returned(ShipDelta {
                crew: i16::from(self.lander.crew.get()),
                element_mass: self.minerals.level(),
                element_amounts: *self.minerals.categories(),
                biological_mass: self.biological.level(),
                ..ShipDelta::default()
            })
        };
        self.phase = SessionPhase::Complete;
        self.settled = Some(outcome.clone());
        outcome
    }
}

/// Move up to twelve crew from the ship into the lander before launch.
#[must_use]
pub fn dispatch_crew(ship_crew: u16) -> (CrewCount, i16) {
    let dispatched = ship_crew.min(12) as u8;
    (CrewCount::new(dispatched), -i16::from(dispatched))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::model::{LanderUpgrades, SurfacePoint};

    fn session(crew: u8) -> PlanetSideSession {
        PlanetSideSession::new(
            LanderState::new(
                SurfacePoint::default(),
                0,
                CrewCount::new(crew),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(100, 0, false),
        )
    }

    #[test]
    fn warmup_dispatches_at_most_twelve_crew() {
        assert_eq!(dispatch_crew(20), (CrewCount::new(12), -12));
        assert_eq!(dispatch_crew(5), (CrewCount::new(5), -5));
    }

    #[test]
    fn surviving_return_commits_crew_and_both_cargo_holds() {
        let mut trip = session(7);
        trip.minerals.collect(3, 4);
        trip.biological.collect(6);
        trip.request_takeoff();
        let SessionOutcome::Returned(delta) = trip.settle() else {
            panic!("expected surviving return");
        };
        assert_eq!(delta.crew, 7);
        assert_eq!(delta.element_mass, 4);
        assert_eq!(delta.element_amounts[3], 4);
        assert_eq!(delta.biological_mass, 6);
        assert_eq!(delta.landers, 0);
    }

    #[test]
    fn destroyed_lander_discards_trip_cargo_and_decrements_landers() {
        let mut trip = session(0);
        trip.minerals.collect(2, 9);
        trip.biological.collect(8);
        let SessionOutcome::LanderDestroyed(delta) = trip.settle() else {
            panic!("expected destroyed lander");
        };
        assert_eq!(
            delta,
            ShipDelta {
                landers: -1,
                ..ShipDelta::default()
            }
        );
    }

    #[test]
    fn abort_produces_no_ship_writeback() {
        let mut trip = session(4);
        trip.minerals.collect(1, 5);

        trip.abort();
        assert_eq!(trip.settle(), SessionOutcome::Aborted);
    }

    #[test]
    fn settlement_is_idempotent_and_completed_session_cannot_take_off() {
        let mut trip = session(6);
        let first = trip.settle();
        assert_eq!(trip.settle(), first);
        assert!(!trip.request_takeoff());
    }
    #[test]
    fn dead_lander_cannot_take_off() {
        let mut trip = session(0);
        assert!(!trip.request_takeoff());
    }
}
