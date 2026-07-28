//! Lock-free production observations for PlanetSide automation and diagnostics.

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
}

pub fn begin(session: &PlanetSideSession) {
    GENERATION.fetch_add(1, Ordering::AcqRel);
    FRAMES.store(0, Ordering::Release);
    START_X.store(session.lander.position.x, Ordering::Release);
    START_Y.store(session.lander.position.y, Ordering::Release);
    CURRENT_X.store(session.lander.position.x, Ordering::Release);
    CURRENT_Y.store(session.lander.position.y, Ordering::Release);
    START_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
    CURRENT_CREW.store(u32::from(session.lander.crew.get()), Ordering::Release);
    SOUND_COUNT.store(0, Ordering::Release);
    LAST_SOUND.store(0, Ordering::Release);
    TERMINAL.store(0, Ordering::Release);
    RETURNED_CREW.store(0, Ordering::Release);
    RETURNED_MINERALS.store(0, Ordering::Release);
    ACTIVE.store(true, Ordering::Release);
}

pub fn frame(session: &PlanetSideSession) {
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
            TERMINAL.store(1, Ordering::Release);
            RETURNED_CREW.store(i32::from(delta.crew), Ordering::Release);
            RETURNED_MINERALS.store(u32::from(delta.element_mass), Ordering::Release);
        }
        SessionOutcome::LanderDestroyed(_) => TERMINAL.store(2, Ordering::Release),
        SessionOutcome::Aborted => TERMINAL.store(3, Ordering::Release),
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
    }
}

pub fn adapter_failure(operation: &'static str) {
    TERMINAL.store(4, Ordering::Release);
    LAST_SOUND.store(super::ffi::adapter_error_code(operation), Ordering::Release);
    ACTIVE.store(false, Ordering::Release);
}

pub fn frame_budget_failure() {
    TERMINAL.store(5, Ordering::Release);
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
