//! Production observations for PlanetSide automation and diagnostics.
//!
//! Observations are published from the single game thread that owns a session
//! and are read by the automation coordinator on that same thread, so
//! [`observation`] is a plain field-by-field read rather than a coherent
//! snapshot. Reading it from another thread is not supported.

/// Terminal codes published by [`finish`]. These are the sole definition; the
/// automation script layer maps its outcome names onto them.
pub mod terminal {
    pub const RUNNING: u32 = 0;
    pub const RETURNED: u32 = 1;
    pub const DESTROYED: u32 = 2;
    pub const ABORTED: u32 = 3;
    pub const ADAPTER_FAILURE: u32 = 4;
    pub const FRAME_BUDGET: u32 = 5;
}

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use crate::battle::battle_types::shortest_path_delta;

use super::cargo::NUM_ELEMENT_CATEGORIES;
use super::hazards::SoundCue;
use super::model::SurfacePoint;
use super::session::{PlanetSideSession, SessionOutcome};
use super::world::WORLD_WIDTH;

/// Deposits kept per element category. The nearest ones to the lander are
/// kept, so the closest deposit of every category is always published.
pub const DEPOSITS_PER_CATEGORY: usize = 64;

/// A mineral deposit that is still on the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MineralDeposit {
    pub x: i32,
    pub y: i32,
    pub category: u8,
}

static GENERATION: AtomicU64 = AtomicU64::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static START_X: AtomicI32 = AtomicI32::new(0);
static START_Y: AtomicI32 = AtomicI32::new(0);
static CURRENT_X: AtomicI32 = AtomicI32::new(0);
static CURRENT_Y: AtomicI32 = AtomicI32::new(0);
static START_CREW: AtomicU32 = AtomicU32::new(0);
static CURRENT_CREW: AtomicU32 = AtomicU32::new(0);
static FACING: AtomicU32 = AtomicU32::new(0);
static MINERAL_UNITS: [AtomicU32; NUM_ELEMENT_CATEGORIES] =
    [const { AtomicU32::new(0) }; NUM_ELEMENT_CATEGORIES];
/// `None` until the surface world has published for this trip, so an absent
/// snapshot is never mistaken for a surface with no deposits.
static DEPOSITS: Mutex<Option<Vec<MineralDeposit>>> = Mutex::new(None);
static SOUND_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_SOUND: AtomicU32 = AtomicU32::new(0);
static TERMINAL: AtomicU32 = AtomicU32::new(0);
static RETURNED_CREW: AtomicI32 = AtomicI32::new(0);
static RETURNED_MINERALS: AtomicU32 = AtomicU32::new(0);
static PHASE: AtomicU32 = AtomicU32::new(0);
/// Graphics batch levels Rust has had to restore after a native callback.
static BATCH_DEPTH_CORRECTIONS: AtomicU64 = AtomicU64::new(0);
/// Mineral deposits collected on a generated node (presented frames).
static MINERAL_PICKUPS: AtomicU64 = AtomicU64::new(0);
/// Stun-bolt impacts that landed on a live creature (presented frames).
static CREATURE_HITS: AtomicU64 = AtomicU64::new(0);
/// Collisions that only connected across the wrapped horizontal seam.
static SEAM_HITS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlanetSideObservation {
    pub generation: u64,
    pub active: bool,
    pub frames: u64,
    pub start_x: i32,
    pub start_y: i32,
    pub current_x: i32,
    pub current_y: i32,
    pub start_crew: u8,
    pub current_crew: u8,
    /// Lander facing, one of the sixteen.
    pub facing: u8,
    /// Mineral units in the lander hold, per element category.
    pub mineral_units: [u16; NUM_ELEMENT_CATEGORIES],
    pub sound_count: u64,
    pub last_sound: u32,
    pub terminal: u32,
    pub returned_crew: i16,
    pub returned_minerals: u16,
    /// Live `SessionPhase` code, so a stalled trip can be told apart from one
    /// that never requested takeoff at all. See [`phase_name`].
    pub phase: u32,
    /// Graphics batch levels Rust had to restore after a native callback during
    /// this trip. Any non-zero value is a defect in the callback.
    pub batch_depth_corrections: u64,
    /// Mineral deposits collected while surface frames were presented.
    pub mineral_pickups: u64,
    /// Stun-bolt impacts on live creatures while surface frames were presented.
    pub creature_hits: u64,
    /// Collisions that connected only across the wrapped horizontal seam.
    pub seam_hits: u64,
}

pub fn begin(session: &PlanetSideSession) {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    PHASE.store(phase_code(session.phase), Ordering::Release);
    BATCH_DEPTH_CORRECTIONS.store(0, Ordering::Relaxed);
    MINERAL_PICKUPS.store(0, Ordering::Release);
    CREATURE_HITS.store(0, Ordering::Release);
    SEAM_HITS.store(0, Ordering::Release);
    FRAMES.store(0, Ordering::Release);
    START_X.store(session.lander.position.x, Ordering::Release);
    START_Y.store(session.lander.position.y, Ordering::Release);
    CURRENT_X.store(session.lander.position.x, Ordering::Release);
    CURRENT_Y.store(session.lander.position.y, Ordering::Release);
    START_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
    CURRENT_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
    publish_lander_state(session);
    *lock_deposits() = None;
    SOUND_COUNT.store(0, Ordering::Release);
    LAST_SOUND.store(0, Ordering::Release);
    TERMINAL.store(terminal::RUNNING, Ordering::Release);
    RETURNED_CREW.store(0, Ordering::Release);
    RETURNED_MINERALS.store(0, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);
}

pub fn frame(session: &PlanetSideSession) {
    PHASE.store(phase_code(session.phase), Ordering::Release);
    FRAMES.fetch_add(1, Ordering::AcqRel);
    CURRENT_X.store(session.lander.position.x, Ordering::Release);
    CURRENT_Y.store(session.lander.position.y, Ordering::Release);
    CURRENT_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
    publish_lander_state(session);
}

fn publish_lander_state(session: &PlanetSideSession) {
    FACING.store(u32::from(session.lander.facing), Ordering::Release);
    for (slot, units) in MINERAL_UNITS.iter().zip(session.minerals.categories()) {
        slot.store(u32::from(*units), Ordering::Release);
    }
}

fn lock_deposits() -> MutexGuard<'static, Option<Vec<MineralDeposit>>> {
    DEPOSITS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Squared distance with the horizontal seam wrapped, as the world measures
/// it for creature awareness.
fn wrapped_distance_squared(a: SurfacePoint, b: SurfacePoint) -> i64 {
    let dx = i64::from(shortest_path_delta(a.x, b.x, WORLD_WIDTH));
    let dy = i64::from(b.y) - i64::from(a.y);
    dx * dx + dy * dy
}

/// Publish the deposits still on the surface, nearest to `near` first within
/// each category and bounded per category by [`DEPOSITS_PER_CATEGORY`].
pub fn publish_deposits(deposits: impl IntoIterator<Item = MineralDeposit>, near: SurfacePoint) {
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
    *lock_deposits() = Some(kept);
}

/// The published deposits, nearest first within each category, or `None` when
/// this trip has not published a surface snapshot yet.
#[must_use]
pub fn deposits() -> Option<Vec<MineralDeposit>> {
    lock_deposits().clone()
}

/// The lander position published by the last presented frame.
#[must_use]
pub fn lander_position() -> SurfacePoint {
    SurfacePoint {
        x: CURRENT_X.load(Ordering::Acquire),
        y: CURRENT_Y.load(Ordering::Acquire),
    }
}

pub fn sound(cue: SoundCue) {
    SOUND_COUNT.fetch_add(1, Ordering::AcqRel);
    LAST_SOUND.store(sound_code(cue), Ordering::Release);
}

pub fn finish(outcome: &SessionOutcome) {
    match outcome {
        SessionOutcome::Returned(delta) => {
            TERMINAL.store(terminal::RETURNED, Ordering::Release);
            RETURNED_CREW.store(i32::from(delta.crew), Ordering::Release);
            RETURNED_MINERALS.store(u32::from(delta.element_mass), Ordering::Release);
        }
        SessionOutcome::LanderDestroyed(_) => {
            TERMINAL.store(terminal::DESTROYED, Ordering::Release)
        }
        SessionOutcome::Aborted => TERMINAL.store(terminal::ABORTED, Ordering::Release),
    }
    ACTIVE.store(false, Ordering::Release);
    *lock_deposits() = None;
}

#[must_use]
pub fn observation() -> PlanetSideObservation {
    PlanetSideObservation {
        generation: GENERATION.load(Ordering::Acquire),
        active: ACTIVE.load(Ordering::Acquire),
        frames: FRAMES.load(Ordering::Acquire),
        start_x: START_X.load(Ordering::Acquire),
        start_y: START_Y.load(Ordering::Acquire),
        current_x: CURRENT_X.load(Ordering::Acquire),
        current_y: CURRENT_Y.load(Ordering::Acquire),
        start_crew: START_CREW.load(Ordering::Acquire) as u8,
        current_crew: CURRENT_CREW.load(Ordering::Acquire) as u8,
        facing: FACING.load(Ordering::Acquire) as u8,
        mineral_units: std::array::from_fn(|index| {
            MINERAL_UNITS[index].load(Ordering::Acquire) as u16
        }),
        sound_count: SOUND_COUNT.load(Ordering::Acquire),
        last_sound: LAST_SOUND.load(Ordering::Acquire),
        terminal: TERMINAL.load(Ordering::Acquire),
        returned_crew: RETURNED_CREW.load(Ordering::Acquire) as i16,
        returned_minerals: RETURNED_MINERALS.load(Ordering::Acquire) as u16,
        phase: PHASE.load(Ordering::Acquire),
        batch_depth_corrections: BATCH_DEPTH_CORRECTIONS.load(Ordering::Relaxed),
        mineral_pickups: MINERAL_PICKUPS.load(Ordering::Acquire),
        creature_hits: CREATURE_HITS.load(Ordering::Acquire),
        seam_hits: SEAM_HITS.load(Ordering::Acquire),
    }
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

/// Human-readable name for an observed phase code.
#[must_use]
pub const fn phase_name(code: u32) -> &'static str {
    match code {
        1 => "Warmup",
        2 => "Launch",
        3 => "Landing",
        4 => "Active",
        5 => "TakingOff",
        6 => "Explosion",
        7 => "Return",
        8 => "Complete",
        9 => "Aborted",
        _ => "None",
    }
}

/// Record that a transitional callback left the graphics batch depth changed
/// and Rust had to restore it. A non-zero count for a trip means some native
/// callback still assumes the retired lander loop's ambient batch.
///
/// This is an independent diagnostic counter, so it needs no ordering relative
/// to the lifecycle fields published around it.
pub fn batch_depth_corrected(levels: i32) {
    BATCH_DEPTH_CORRECTIONS.fetch_add(levels.unsigned_abs().into(), Ordering::Relaxed);
}

/// Record a collected mineral deposit.
pub fn mineral_pickup() {
    MINERAL_PICKUPS.fetch_add(1, Ordering::AcqRel);
}

/// Record a stun-bolt hit on a live creature.
pub fn creature_hit() {
    CREATURE_HITS.fetch_add(1, Ordering::AcqRel);
}

/// Record a collision that connected only across the wrapped seam.
pub fn seam_hit() {
    SEAM_HITS.fetch_add(1, Ordering::AcqRel);
}

pub fn adapter_failure(operation: &'static str) {
    TERMINAL.store(terminal::ADAPTER_FAILURE, Ordering::Release);
    LAST_SOUND.store(super::ffi::adapter_error_code(operation), Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
}

pub fn frame_budget_failure() {
    TERMINAL.store(terminal::FRAME_BUDGET, Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
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
    use std::sync::{Mutex, MutexGuard};

    /// These observations are process-global, and the module contract is that
    /// one thread owns them, so the tests take turns rather than racing.
    pub(crate) static SERIALIZE: Mutex<()> = Mutex::new(());

    pub(crate) fn exclusive() -> MutexGuard<'static, ()> {
        SERIALIZE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    use crate::planet_side::simulation::LanderState;

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
    fn a_new_trip_never_reports_the_previous_trip_phase() {
        let _guard = exclusive();
        let finished = session(SessionPhase::Complete);
        begin(&finished);
        frame(&finished);
        assert_eq!(phase_name(observation().phase), "Complete");

        // The next trip must publish its own phase before it is observable,
        // otherwise a waiter can see the previous trip's terminal phase.
        let fresh = session(SessionPhase::Warmup);
        begin(&fresh);
        assert_eq!(phase_name(observation().phase), "Warmup");
    }

    #[test]
    fn each_trip_gets_a_new_generation_and_clears_the_terminal_code() {
        let _guard = exclusive();
        let first = session(SessionPhase::Active);
        begin(&first);
        let generation = observation().generation;
        finish(&SessionOutcome::Aborted);
        assert_eq!(observation().terminal, terminal::ABORTED);
        assert!(!observation().active);

        let second = session(SessionPhase::Active);
        begin(&second);
        let next = observation();
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
        let _guard = exclusive();
        let mut session = session(SessionPhase::Active);
        session.lander.facing = 11;
        session.minerals.collect(6, 3);
        session.minerals.collect(1, 2);
        begin(&session);
        frame(&session);
        let obs = observation();
        assert_eq!(obs.facing, 11);
        assert_eq!(obs.mineral_units, [0, 2, 0, 0, 0, 0, 3, 0]);
        finish(&SessionOutcome::Aborted);
    }

    fn deposit(x: i32, y: i32, category: u8) -> MineralDeposit {
        MineralDeposit { x, y, category }
    }

    #[test]
    fn deposits_are_ordered_nearest_first_across_the_wrapped_seam() {
        let _guard = exclusive();
        let near = SurfacePoint { x: 5, y: 40 };
        let far_inside = deposit(300, 40, 6);
        let across_seam = deposit(WORLD_WIDTH - 10, 40, 6);
        publish_deposits([far_inside, across_seam], near);
        assert_eq!(deposits(), Some(vec![across_seam, far_inside]));
        *lock_deposits() = None;
    }

    #[test]
    fn deposits_are_bounded_per_category_keeping_the_nearest() {
        let _guard = exclusive();
        let near = SurfacePoint { x: 0, y: 0 };
        let mut all: Vec<MineralDeposit> = (0..(DEPOSITS_PER_CATEGORY as i32 + 20))
            .map(|offset| deposit(offset + 1, 0, 6))
            .collect();
        all.push(deposit(400, 0, 2));
        publish_deposits(all, near);
        let published = deposits().unwrap();
        let radioactive: Vec<_> = published.iter().filter(|d| d.category == 6).collect();
        assert_eq!(radioactive.len(), DEPOSITS_PER_CATEGORY);
        assert_eq!(radioactive[0].x, 1);
        assert_eq!(published.iter().filter(|d| d.category == 2).count(), 1);
        *lock_deposits() = None;
    }

    #[test]
    fn a_new_trip_and_a_finished_trip_publish_no_deposits() {
        let _guard = exclusive();
        publish_deposits([deposit(1, 1, 6)], SurfacePoint::default());
        begin(&session(SessionPhase::Active));
        assert_eq!(deposits(), None);
        publish_deposits([deposit(1, 1, 6)], SurfacePoint::default());
        assert_eq!(deposits(), Some(vec![deposit(1, 1, 6)]));
        finish(&SessionOutcome::Aborted);
        assert_eq!(deposits(), None);
    }

    #[test]
    fn collision_verdict_counters_reset_and_sample() {
        let _guard = exclusive();
        let session = dummy();
        begin(&session);
        assert_eq!(observation().mineral_pickups, 0);
        mineral_pickup();
        creature_hit();
        seam_hit();
        let obs = observation();
        assert_eq!(
            (obs.mineral_pickups, obs.creature_hits, obs.seam_hits),
            (1, 1, 1)
        );
        finish(&SessionOutcome::Aborted);
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
