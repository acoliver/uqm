//! Planet-side trip lifecycle and orbit writeback transaction.

use super::cargo::{BioCargo, MineralCargo, NUM_ELEMENT_CATEGORIES};
use super::lifecycle::LifecycleAnimation;
use super::model::CrewCount;
use super::simulation::LanderState;
use super::world::HazardChances;

/// Explicit phase of the lander trip.
///
/// The phase sequence mirrors the deleted `lander.c` `PlanetSide()` animation:
/// `Warmup` → `Launch` → `Landing` → `Active` → `TakingOff` → `Return` →
/// `Complete`, or `Active` → `Explosion` → `Complete` (destroyed), or
/// `Active` → `Aborted`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    /// Initial one-frame state before the launch animation starts.
    Warmup,
    /// `AnimateLaunch(LanderFrame[5])`: plays all launch graphic frames.
    Launch,
    /// `LandingTakeoffSequence(TRUE)`: idle pause + smooth descent.
    Landing,
    /// Main gameplay loop (`DoPlanetSide`).
    Active,
    /// `LandingTakeoffSequence(FALSE)`: smooth ascent + idle.  Does **not**
    /// settle until the animation completes.
    TakingOff,
    /// `LanderExplosion` + dramatic wait (`EXPLOSION_WAIT_FRAMES`).
    Explosion,
    /// `AnimateLaunch(LanderFrame[6])`: plays all return graphic frames.
    Return,
    /// Terminal success/destroyed: `settle()` has been called.
    Complete,
    /// Terminal: user aborted the trip without writeback.
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
    /// Per-planet hazard chances used by the world simulation each frame.
    pub hazard_chances: HazardChances,
    /// Monotonic animation counter for the current lifecycle phase.
    pub animation: LifecycleAnimation,
    /// Number of frames in the launch graphic (`LanderFrame[5]`). Zero means
    /// the launch animation is skipped.
    pub launch_frame_count: u16,
    /// Number of frames in the return graphic (`LanderFrame[6]`). Zero means
    /// the return animation is skipped.
    pub return_frame_count: u16,
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
            hazard_chances: HazardChances::default(),
            animation: LifecycleAnimation::default(),
            launch_frame_count: 0,
            return_frame_count: 0,
            settled: None,
        }
    }

    /// Set the per-planet hazard chances for the world simulation.
    pub fn set_hazard_chances(&mut self, chances: HazardChances) {
        self.hazard_chances = chances;
    }

    /// Set the launch and return graphic frame counts used by lifecycle
    /// animation phases.
    pub fn set_lifecycle_frame_counts(&mut self, launch: u16, ret: u16) {
        self.launch_frame_count = launch;
        self.return_frame_count = ret;
    }

    /// Transition into manual or scripted takeoff when crew remains.
    ///
    /// Returns `true` if the takeoff was accepted. The session enters the
    /// `TakingOff` phase but does **not** settle until the takeoff animation
    /// completes (see [`Self::advance_takeoff`]).
    pub fn request_takeoff(&mut self) -> bool {
        if self.lander.crew.get() == 0
            || matches!(self.phase, SessionPhase::Aborted | SessionPhase::Complete)
        {
            false
        } else {
            self.lander.in_transit = true;
            self.phase = SessionPhase::TakingOff;
            self.animation.reset();
            true
        }
    }

    /// Advance the launch animation. Returns `true` when the launch has
    /// completed and the session should transition to `Landing`.
    pub fn advance_launch(&mut self) -> bool {
        let total = self.launch_frame_count.max(1);
        self.animation.advance(total)
    }

    /// Advance the landing animation. Returns `true` when the landing descent
    /// has completed and the session should transition to `Active`.
    pub fn advance_landing(&mut self) -> bool {
        self.animation
            .advance(super::lifecycle::LifecycleAnimation::landing_total())
    }

    /// Advance the takeoff animation. Returns `true` when the takeoff ascent
    /// has completed and the session should transition to `Return`.
    ///
    /// The session does **not** settle here — only after the return animation.
    pub fn advance_takeoff(&mut self) -> bool {
        self.animation
            .advance(super::lifecycle::LifecycleAnimation::takeoff_total())
    }

    /// Advance the return animation. Returns `true` when the return graphic
    /// has played and the session should settle.
    pub fn advance_return(&mut self) -> bool {
        let total = self.return_frame_count.max(1);
        self.animation.advance(total)
    }

    /// Advance the explosion animation. Returns `true` when the explosion
    /// animation plus dramatic wait has completed.
    pub fn advance_explosion(&mut self) -> bool {
        self.animation
            .advance(super::lifecycle::EXPLOSION_TOTAL_FRAMES)
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
    use crate::planet_side::lifecycle;
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

    #[test]
    fn full_phase_progression_warmup_launch_landing_active_takeoff_return_complete() {
        let mut trip = session(5);
        trip.set_lifecycle_frame_counts(3, 2);

        // Warmup → Launch
        assert_eq!(trip.phase, SessionPhase::Warmup);
        // Launch: 3 frames
        assert!(!trip.advance_launch()); // frame 1
        assert!(!trip.advance_launch()); // frame 2
        assert!(trip.advance_launch()); // frame 3 → complete
        trip.phase = SessionPhase::Launch;
        trip.animation.reset();

        // Launch: 3 frames
        assert!(!trip.advance_launch()); // frame 1
        assert!(!trip.advance_launch()); // frame 2
        assert!(trip.advance_launch()); // frame 3 → complete

        // Landing: 35 + 13 = 48 frames
        trip.phase = SessionPhase::Landing;
        trip.animation.reset();
        let landing_total = lifecycle::LifecycleAnimation::landing_total();
        for _ in 1..landing_total {
            assert!(!trip.advance_landing());
        }
        assert!(trip.advance_landing());

        // Active
        trip.phase = SessionPhase::Active;
        trip.animation.reset();
        trip.minerals.collect(1, 10);

        // Takeoff: 13 + 18 = 31 frames
        assert!(trip.request_takeoff());
        assert_eq!(trip.phase, SessionPhase::TakingOff);
        let takeoff_total = lifecycle::LifecycleAnimation::takeoff_total();
        for _ in 1..takeoff_total {
            assert!(!trip.advance_takeoff());
        }
        assert!(trip.advance_takeoff());

        // Return: 2 frames
        trip.phase = SessionPhase::Return;
        trip.animation.reset();
        assert!(!trip.advance_return());
        assert!(trip.advance_return());

        // Settle
        let outcome = trip.settle();
        assert!(matches!(outcome, SessionOutcome::Returned(_)));
        assert_eq!(trip.phase, SessionPhase::Complete);
    }

    #[test]
    fn explosion_progression_advances_through_total_frames() {
        let mut trip = session(0);
        trip.phase = SessionPhase::Explosion;
        trip.animation.reset();
        let total = lifecycle::EXPLOSION_TOTAL_FRAMES;
        for _ in 1..total {
            assert!(!trip.advance_explosion());
        }
        assert!(trip.advance_explosion());
    }

    #[test]
    fn lifecycle_frame_counts_default_to_zero() {
        let trip = session(4);
        assert_eq!(trip.launch_frame_count, 0);
        assert_eq!(trip.return_frame_count, 0);
    }

    #[test]
    fn set_lifecycle_frame_counts_stores_both_values() {
        let mut trip = session(4);
        trip.set_lifecycle_frame_counts(8, 6);
        assert_eq!(trip.launch_frame_count, 8);
        assert_eq!(trip.return_frame_count, 6);
    }

    #[test]
    fn request_takeoff_resets_animation_counter() {
        let mut trip = session(5);
        // Advance the animation counter a few times so it's non-zero.
        trip.advance_launch();
        trip.advance_launch();
        assert_ne!(trip.animation.frame(), 0);
        assert!(trip.request_takeoff());
        assert_eq!(trip.animation.frame(), 0);
    }

    #[test]
    fn advance_launch_with_zero_frame_count_completes_in_one_frame() {
        let mut trip = session(5);
        trip.set_lifecycle_frame_counts(0, 0);
        assert!(trip.advance_launch());
    }

    #[test]
    fn settle_after_explosion_produces_lander_destroyed() {
        let mut trip = session(0);
        trip.phase = SessionPhase::Explosion;
        let outcome = trip.settle();
        assert!(matches!(outcome, SessionOutcome::LanderDestroyed(_)));
    }

    #[test]
    fn cargo_levels_reflect_pickups_immediately() {
        let mut trip = session(5);
        trip.minerals.collect(2, 10);
        assert_eq!(trip.minerals.level(), 10);
        trip.biological.collect(3);
        assert_eq!(trip.biological.level(), 3);
        // Additional pickup accumulates.
        trip.minerals.collect(1, 5);
        assert_eq!(trip.minerals.level(), 15);
    }
}
