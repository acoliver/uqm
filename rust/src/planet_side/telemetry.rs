//! Production observations for PlanetSide automation and diagnostics.
//!
//! A trip owns these values; observers receive copies at synchronous boundaries.

#[cfg(test)]
use crate::engine::observe_port::planet_side::phase_name;
use crate::engine::observe_port::planet_side::{terminal, MineralDeposit, PlanetSideObservation};

use crate::battle::battle_types::shortest_path_delta;

use super::hazards::SoundCue;
use super::model::SurfacePoint;
use super::session::{PlanetSideSession, SessionOutcome};
use super::world::WORLD_WIDTH;

/// Deposits kept per element category. The nearest ones to the lander are
/// kept, so the closest deposit of every category is always published.
pub const DEPOSITS_PER_CATEGORY: usize = 64;

/// Observations owned by one synchronous lander trip.
#[derive(Debug)]
pub struct TripTelemetry {
    snapshot: crate::engine::observe_port::planet_side::PlanetSideSnapshot,
}

impl TripTelemetry {
    pub fn pending(generation: u64) -> Self {
        Self {
            snapshot: crate::engine::observe_port::planet_side::PlanetSideSnapshot {
                observation: PlanetSideObservation {
                    generation,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    pub fn begin(&mut self, session: &PlanetSideSession) {
        *self = Self::new(self.snapshot.observation.generation, session);
    }

    pub fn new(generation: u64, session: &PlanetSideSession) -> Self {
        Self {
            snapshot: crate::engine::observe_port::planet_side::PlanetSideSnapshot {
                observation: PlanetSideObservation {
                    generation,
                    active: true,
                    start_x: session.lander.position.x,
                    start_y: session.lander.position.y,
                    current_x: session.lander.position.x,
                    current_y: session.lander.position.y,
                    start_crew: session.lander.crew.get(),
                    current_crew: session.lander.crew.get(),
                    facing: session.lander.facing,
                    mineral_units: *session.minerals.categories(),
                    phase: phase_code(session.phase),
                    ..PlanetSideObservation::default()
                },
                ..Default::default()
            },
        }
    }

    pub fn snapshot(&self) -> &crate::engine::observe_port::planet_side::PlanetSideSnapshot {
        &self.snapshot
    }

    pub fn frame(&mut self, session: &PlanetSideSession) {
        let observed = &mut self.snapshot.observation;
        observed.frames += 1;
        observed.phase = phase_code(session.phase);
        observed.current_x = session.lander.position.x;
        observed.current_y = session.lander.position.y;
        observed.current_crew = session.lander.crew.get();
        observed.facing = session.lander.facing;
        observed.mineral_units = *session.minerals.categories();
    }

    pub fn publish_deposits(
        &mut self,
        deposits: impl IntoIterator<Item = MineralDeposit>,
        near: SurfacePoint,
    ) {
        self.snapshot.deposits = Some(ordered_deposits(deposits, near));
    }

    pub fn publish_energy(&mut self, nodes: Vec<(u8, i32, i32)>) {
        self.snapshot.energy_nodes = nodes;
    }

    pub fn energy_retrieved(&mut self, node: u8) {
        self.snapshot.energy_retrieved |= 1 << node;
    }

    pub fn sound(&mut self, cue: SoundCue) {
        self.snapshot.observation.sound_count += 1;
        self.snapshot.observation.last_sound = sound_code(cue);
    }

    pub fn mineral_pickup(&mut self) {
        self.snapshot.observation.mineral_pickups += 1;
    }

    pub fn creature_hit(&mut self) {
        self.snapshot.observation.creature_hits += 1;
    }

    pub fn seam_hit(&mut self) {
        self.snapshot.observation.seam_hits += 1;
    }

    pub fn batch_depth_corrected(&mut self, levels: i32) {
        self.snapshot.observation.batch_depth_corrections += u64::from(levels.unsigned_abs());
    }

    pub fn finish(&mut self, outcome: &SessionOutcome) {
        let observed = &mut self.snapshot.observation;
        match outcome {
            SessionOutcome::Returned(delta) => {
                observed.terminal = terminal::RETURNED;
                observed.returned_crew = delta.crew;
                observed.returned_minerals = delta.element_mass;
            }
            SessionOutcome::LanderDestroyed(_) => observed.terminal = terminal::DESTROYED,
            SessionOutcome::Aborted => observed.terminal = terminal::ABORTED,
        }
        observed.active = false;
        self.snapshot.deposits = None;
    }

    pub fn adapter_failure(&mut self, operation: &'static str) {
        self.snapshot.observation.terminal = terminal::ADAPTER_FAILURE;
        self.snapshot.observation.last_sound = super::ffi::adapter_error_code(operation);
        self.snapshot.observation.active = false;
    }

    pub fn frame_budget_failure(&mut self) {
        self.snapshot.observation.terminal = terminal::FRAME_BUDGET;
        self.snapshot.observation.active = false;
    }
}

/// Squared distance with the horizontal seam wrapped, as the world measures
/// it for creature awareness.
fn wrapped_distance_squared(a: SurfacePoint, b: SurfacePoint) -> i64 {
    let dx = i64::from(shortest_path_delta(a.x, b.x, WORLD_WIDTH));
    let dy = i64::from(b.y) - i64::from(a.y);
    dx * dx + dy * dy
}

fn ordered_deposits(
    deposits: impl IntoIterator<Item = MineralDeposit>,
    near: SurfacePoint,
) -> Vec<MineralDeposit> {
    let mut kept: Vec<MineralDeposit> = deposits.into_iter().collect();
    kept.sort_by_key(|deposit| {
        (
            deposit.category,
            wrapped_distance_squared(
                near,
                SurfacePoint {
                    x: deposit.x,
                    y: deposit.y,
                },
            ),
        )
    });
    let mut in_category = 0usize;
    let mut previous = None;
    kept.retain(|deposit| {
        if previous != Some(deposit.category) {
            previous = Some(deposit.category);
            in_category = 0;
        }
        in_category += 1;
        in_category <= DEPOSITS_PER_CATEGORY
    });
    kept
}

const fn phase_code(phase: super::session::SessionPhase) -> u32 {
    use super::session::SessionPhase;
    match phase {
        SessionPhase::Warmup => 1,
        SessionPhase::Launch => 2,
        SessionPhase::Landing => 3,
        SessionPhase::Active => 4,
        SessionPhase::TakingOff => 5,
        SessionPhase::Explosion => 6,
        SessionPhase::Return => 7,
        SessionPhase::Complete => 8,
        SessionPhase::Aborted => 9,
    }
}

const fn sound_code(cue: SoundCue) -> u32 {
    match cue {
        SoundCue::BiologicalDisaster => 1,
        SoundCue::Earthquake => 2,
        SoundCue::Lightning => 3,
        SoundCue::Lava => 4,
        SoundCue::LanderInjured => 5,
        SoundCue::LanderShoots => 6,
        SoundCue::LanderHits => 7,
        SoundCue::LifeformCanned => 8,
        SoundCue::Pickup => 9,
        SoundCue::Full => 10,
        SoundCue::Departs => 11,
        SoundCue::Returns => 12,
        SoundCue::Destroyed => 13,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::planet_side::cargo::MineralCargo;
    use crate::planet_side::model::{CrewCount, LanderUpgrades, SurfacePoint};
    use crate::planet_side::session::{PlanetSideSession, SessionPhase};
    use crate::planet_side::simulation::LanderState;

    pub(crate) fn trip() -> std::rc::Rc<std::cell::RefCell<TripTelemetry>> {
        std::rc::Rc::new(std::cell::RefCell::new(TripTelemetry::new(
            1,
            &session(SessionPhase::Active),
        )))
    }

    fn session(phase: SessionPhase) -> PlanetSideSession {
        let mut session = PlanetSideSession::new(
            LanderState::new(
                SurfacePoint::default(),
                0,
                CrewCount::new(6),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(50, 0, false),
        );
        session.phase = phase;
        session
    }

    #[test]
    fn owned_trip_observations_do_not_share_counters_or_deposits() {
        let mut first = TripTelemetry::new(7, &session(SessionPhase::Active));
        let second = TripTelemetry::new(8, &session(SessionPhase::Warmup));
        first.publish_deposits([deposit(3, 4, 6)], SurfacePoint::default());
        first.publish_energy(vec![(31, 3, 4)]);
        first.energy_retrieved(31);
        first.sound(SoundCue::Pickup);
        first.mineral_pickup();
        first.creature_hit();
        first.seam_hit();
        first.batch_depth_corrected(-2);
        first.frame(&session(SessionPhase::Active));
        let observed = first.snapshot();
        assert_eq!(observed.observation.generation, 7);
        assert_eq!(observed.observation.frames, 1);
        assert_eq!(observed.observation.sound_count, 1);
        assert_eq!(observed.observation.mineral_pickups, 1);
        assert_eq!(observed.observation.creature_hits, 1);
        assert_eq!(observed.observation.seam_hits, 1);
        assert_eq!(observed.observation.batch_depth_corrections, 2);
        assert_eq!(observed.energy_nodes, [(31, 3, 4)]);
        assert_eq!(observed.energy_retrieved, 1 << 31);
        assert_eq!(observed.deposits, Some(vec![deposit(3, 4, 6)]));
        let fresh = second.snapshot();
        assert_eq!(fresh.observation.generation, 8);
        assert_eq!(fresh.observation.frames, 0);
        assert_eq!(fresh.observation.sound_count, 0);
        assert_eq!(fresh.observation.mineral_pickups, 0);
        assert_eq!(fresh.observation.phase, 1);
        assert!(fresh.deposits.is_none());
        assert!(fresh.energy_nodes.is_empty());
        assert_eq!(fresh.energy_retrieved, 0);
        first.finish(&SessionOutcome::Aborted);
        assert!(!first.snapshot().observation.active);
        assert!(second.snapshot().observation.active);
    }

    #[test]
    fn owned_snapshots_keep_terminal_values_after_the_trip_changes() {
        let mut trip = TripTelemetry::new(3, &session(SessionPhase::Active));
        let before_return = trip.snapshot().clone();
        trip.finish(&SessionOutcome::Returned(
            super::super::session::ShipDelta {
                crew: 4,
                element_mass: 9,
                ..Default::default()
            },
        ));
        assert!(before_return.observation.active);
        assert_eq!(before_return.observation.terminal, terminal::RUNNING);
        assert!(!trip.snapshot().observation.active);
        assert_eq!(trip.snapshot().observation.terminal, terminal::RETURNED);
        assert_eq!(trip.snapshot().observation.returned_crew, 4);
        assert_eq!(trip.snapshot().observation.returned_minerals, 9);
    }

    #[test]
    fn owned_trip_failure_outcomes_do_not_change_other_observers() {
        let mut destroyed = TripTelemetry::new(1, &session(SessionPhase::Explosion));
        destroyed.finish(&SessionOutcome::LanderDestroyed(Default::default()));
        assert_eq!(
            destroyed.snapshot().observation.terminal,
            terminal::DESTROYED
        );
        assert!(!destroyed.snapshot().observation.active);
        let mut failed = TripTelemetry::new(2, &session(SessionPhase::Active));
        failed.adapter_failure("render");
        assert_eq!(
            failed.snapshot().observation.terminal,
            terminal::ADAPTER_FAILURE
        );
        assert_eq!(
            failed.snapshot().observation.last_sound,
            super::super::ffi::adapter_error_code("render")
        );
        assert!(!failed.snapshot().observation.active);
        let mut budget = TripTelemetry::new(3, &session(SessionPhase::Active));
        budget.frame_budget_failure();
        assert_eq!(
            budget.snapshot().observation.terminal,
            terminal::FRAME_BUDGET
        );
        assert!(!budget.snapshot().observation.active);
        assert_eq!(
            destroyed.snapshot().observation.terminal,
            terminal::DESTROYED
        );
        assert_eq!(
            failed.snapshot().observation.terminal,
            terminal::ADAPTER_FAILURE
        );
    }

    #[test]
    fn a_new_trip_never_reports_the_previous_trip_phase() {
        let mut trip = TripTelemetry::pending(1);
        let finished = session(SessionPhase::Complete);
        trip.begin(&finished);
        trip.frame(&finished);
        assert_eq!(phase_name(trip.snapshot().observation.phase), "Complete");

        // The next trip must publish its own phase before it is observable,
        // otherwise a waiter can see the previous trip's terminal phase.
        let fresh = session(SessionPhase::Warmup);
        trip.begin(&fresh);
        assert_eq!(phase_name(trip.snapshot().observation.phase), "Warmup");
    }

    #[test]
    fn each_trip_gets_a_new_generation_and_clears_the_terminal_code() {
        let mut trip = TripTelemetry::pending(1);
        let first = session(SessionPhase::Active);
        trip.begin(&first);
        let generation = trip.snapshot().observation.generation;
        trip.finish(&SessionOutcome::Aborted);
        assert_eq!(trip.snapshot().observation.terminal, terminal::ABORTED);
        assert!(!trip.snapshot().observation.active);

        let second = session(SessionPhase::Active);
        trip = TripTelemetry::new(generation + 1, &second);
        let next = trip.snapshot().observation;
        assert!(next.generation > generation, "generation must advance");
        assert_eq!(next.terminal, terminal::RUNNING, "terminal must be cleared");
        assert!(next.active);
    }

    #[test]
    fn phase_names_cover_every_published_code() {
        for phase in [
            SessionPhase::Warmup,
            SessionPhase::Launch,
            SessionPhase::Landing,
            SessionPhase::Active,
            SessionPhase::TakingOff,
            SessionPhase::Explosion,
            SessionPhase::Return,
            SessionPhase::Complete,
            SessionPhase::Aborted,
        ] {
            assert_ne!(
                phase_name(phase_code(phase)),
                "None",
                "every phase needs a name"
            );
        }
        assert_eq!(phase_name(u32::MAX), "None");
    }

    #[test]
    fn frames_publish_the_lander_facing_and_per_category_cargo() {
        let mut trip = TripTelemetry::pending(1);
        let mut session = session(SessionPhase::Active);
        session.lander.facing = 11;
        session.minerals.collect(6, 3);
        session.minerals.collect(1, 2);
        trip.begin(&session);
        trip.frame(&session);
        let obs = trip.snapshot().observation;
        assert_eq!(obs.facing, 11);
        assert_eq!(obs.mineral_units, [0, 2, 0, 0, 0, 0, 3, 0]);
        trip.finish(&SessionOutcome::Aborted);
    }

    fn deposit(x: i32, y: i32, category: u8) -> MineralDeposit {
        MineralDeposit { x, y, category }
    }

    #[test]
    fn deposits_are_ordered_nearest_first_across_the_wrapped_seam() {
        let mut trip = TripTelemetry::pending(1);
        let near = SurfacePoint { x: 5, y: 40 };
        let far_inside = deposit(300, 40, 6);
        let across_seam = deposit(WORLD_WIDTH - 10, 40, 6);
        trip.publish_deposits([far_inside, across_seam], near);
        assert_eq!(
            trip.snapshot().deposits.clone(),
            Some(vec![across_seam, far_inside])
        );
    }

    #[test]
    fn deposits_are_bounded_per_category_keeping_the_nearest() {
        let mut trip = TripTelemetry::pending(1);
        let near = SurfacePoint { x: 0, y: 0 };
        let mut all: Vec<MineralDeposit> = (0..(DEPOSITS_PER_CATEGORY as i32 + 20))
            .map(|offset| deposit(offset + 1, 0, 6))
            .collect();
        all.push(deposit(400, 0, 2));
        trip.publish_deposits(all, near);
        let published = trip.snapshot().deposits.clone().unwrap();
        let radioactive: Vec<_> = published.iter().filter(|d| d.category == 6).collect();
        assert_eq!(radioactive.len(), DEPOSITS_PER_CATEGORY);
        assert_eq!(radioactive[0].x, 1);
        assert_eq!(published.iter().filter(|d| d.category == 2).count(), 1);
    }

    #[test]
    fn a_new_trip_and_a_finished_trip_publish_no_deposits() {
        let mut trip = TripTelemetry::pending(1);
        trip.publish_deposits([deposit(1, 1, 6)], SurfacePoint::default());
        trip.begin(&session(SessionPhase::Active));
        assert_eq!(trip.snapshot().deposits.clone(), None);
        trip.publish_deposits([deposit(1, 1, 6)], SurfacePoint::default());
        assert_eq!(
            trip.snapshot().deposits.clone(),
            Some(vec![deposit(1, 1, 6)])
        );
        trip.finish(&SessionOutcome::Aborted);
        assert_eq!(trip.snapshot().deposits.clone(), None);
    }

    #[test]
    fn collision_verdict_counters_reset_and_sample() {
        let mut trip = TripTelemetry::pending(1);
        let session = dummy();
        trip.begin(&session);
        assert_eq!(trip.snapshot().observation.mineral_pickups, 0);
        trip.mineral_pickup();
        trip.creature_hit();
        trip.seam_hit();
        let obs = trip.snapshot().observation;
        assert_eq!(
            (obs.mineral_pickups, obs.creature_hits, obs.seam_hits),
            (1, 1, 1)
        );
        trip.finish(&SessionOutcome::Aborted);
    }
    fn dummy() -> PlanetSideSession {
        PlanetSideSession::new(
            crate::planet_side::simulation::LanderState::new(
                crate::planet_side::model::SurfacePoint { x: 0, y: 0 },
                0,
                crate::planet_side::model::CrewCount::new(1),
                crate::planet_side::model::LanderUpgrades::default(),
            ),
            crate::planet_side::cargo::MineralCargo::new(50, 0, false),
        )
    }
}
