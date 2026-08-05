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

use super::hazards::SoundCue;
use super::session::{PlanetSideSession, SessionOutcome};

static GENERATION: AtomicU64 = AtomicU64::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static START_X: AtomicI32 = AtomicI32::new(0);
static START_Y: AtomicI32 = AtomicI32::new(0);
static CURRENT_X: AtomicI32 = AtomicI32::new(0);
static CURRENT_Y: AtomicI32 = AtomicI32::new(0);
static START_CREW: AtomicU32 = AtomicU32::new(0);
static CURRENT_CREW: AtomicU32 = AtomicU32::new(0);
static SOUND_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_SOUND: AtomicU32 = AtomicU32::new(0);
static TERMINAL: AtomicU32 = AtomicU32::new(0);
static RETURNED_CREW: AtomicI32 = AtomicI32::new(0);
static RETURNED_MINERALS: AtomicU32 = AtomicU32::new(0);
static PHASE: AtomicU32 = AtomicU32::new(0);
/// Graphics batch levels Rust has had to restore after a native callback.
static BATCH_DEPTH_CORRECTIONS: AtomicU64 = AtomicU64::new(0);

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
    pub sound_count: u64,
    pub last_sound: u32,
    pub terminal: u32,
    pub returned_crew: i16,
    pub returned_minerals: u16,
    /// Live `SessionPhase` code, so a stalled trip can be told apart from one
    /// that never requested takeoff at all. See [`phase_name`].
    pub phase: u32,
}

pub fn begin(session: &PlanetSideSession) {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    PHASE.store(phase_code(session.phase), Ordering::Release);
    FRAMES.store(0, Ordering::Release);
    START_X.store(session.lander.position.x, Ordering::Release);
    START_Y.store(session.lander.position.y, Ordering::Release);
    CURRENT_X.store(session.lander.position.x, Ordering::Release);
    CURRENT_Y.store(session.lander.position.y, Ordering::Release);
    START_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
    CURRENT_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
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
        sound_count: SOUND_COUNT.load(Ordering::Acquire),
        last_sound: LAST_SOUND.load(Ordering::Acquire),
        terminal: TERMINAL.load(Ordering::Acquire),
        returned_crew: RETURNED_CREW.load(Ordering::Acquire) as i16,
        returned_minerals: RETURNED_MINERALS.load(Ordering::Acquire) as u16,
        phase: PHASE.load(Ordering::Acquire),
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
/// and Rust had to restore it. A non-zero count means some native callback
/// still assumes the retired lander loop's ambient batch.
pub fn batch_depth_corrected(levels: i32) {
    BATCH_DEPTH_CORRECTIONS.fetch_add(levels.unsigned_abs().into(), Ordering::AcqRel);
}

/// Total graphics batch levels Rust has had to correct this run.
#[must_use]
pub fn batch_depth_corrections() -> u64 {
    BATCH_DEPTH_CORRECTIONS.load(Ordering::Acquire)
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
mod tests {
    use super::*;
    use crate::planet_side::cargo::MineralCargo;
    use crate::planet_side::model::{CrewCount, LanderUpgrades, SurfacePoint};
    use crate::planet_side::session::{PlanetSideSession, SessionPhase};
    use std::sync::{Mutex, MutexGuard};

    /// These observations are process-global, and the module contract is that
    /// one thread owns them, so the tests take turns rather than racing.
    static SERIALIZE: Mutex<()> = Mutex::new(());

    fn exclusive() -> MutexGuard<'static, ()> {
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
}
