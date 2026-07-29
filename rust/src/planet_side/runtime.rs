//! Synchronous PlanetSide orchestration over typed runtime adapters.

use super::collision::{
    resolve_lander_collision, CollisionOutcome, CollisionRolls, CollisionState, LanderCollision,
};
use super::entities::SurfaceEntityId;
use super::hazards::SoundCue;
use super::session::{PlanetSideSession, SessionOutcome, SessionPhase, ShipDelta};
use super::simulation::{self, FrameInput, LanderState, Shot, SimulationEffect, TickResult};
use super::special_effects::SpecialPickupEffects;
use super::world::{HazardChances, HazardSpawn, WorldStepEffects};

/// Canonical planet-side simulation cadence.
pub const PLANET_SIDE_HZ: u32 = 35;

/// Monotonic runtime deadline in engine clock units.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(pub u32);

/// Rendering snapshot. It contains no resource handle or C pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderSnapshot {
    pub phase: SessionPhase,
    pub lander: LanderState,
    pub mineral_level: u16,
    pub biological_level: u16,
    /// Lifecycle animation state (current frame counter).
    pub animation_frame: u16,
    /// Vertical offset for landing/takeoff transitions (≤ 0).
    pub lifecycle_offset: i32,
}

impl RenderSnapshot {
    /// Current animation frame within the lifecycle phase.
    #[must_use]
    pub const fn animation_frame(&self) -> u16 {
        self.animation_frame
    }

    /// Vertical pixel offset for the landing/takeoff transition (≤ 0).
    #[must_use]
    pub const fn lifecycle_offset(&self) -> i32 {
        self.lifecycle_offset
    }
}

/// Error returned by a concrete runtime adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterError {
    pub operation: &'static str,
}

impl AdapterError {
    #[must_use]
    pub const fn new(operation: &'static str) -> Self {
        Self { operation }
    }
}

/// Gameplay input adapter.
pub trait PlanetSideInput {
    fn poll(&mut self) -> Result<FrameInput, AdapterError>;
}

/// One geometry contact with its Rust entity identity preserved for commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionContact {
    pub entity: Option<SurfaceEntityId>,
    pub collision: LanderCollision,
    pub rolls: CollisionRolls,
}

/// Result of stepping the surface world for one frame.
///
/// The collision adapter returns this so the runtime can play hazard sounds,
/// apply lightning crew kills, and request takeoff when the lander is destroyed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldStepResult {
    /// World simulation effects (canned creatures, spawns, sounds).
    pub effects: WorldStepEffects,
    /// Hazard spawns produced this frame, ready for entity insertion.
    pub hazard_spawns: Vec<HazardSpawn>,
    /// Crew killed by lightning strikes this frame.
    pub lightning_kills: u8,
}

/// Collision geometry and interaction adapter. Gameplay resolution stays in
/// Rust; `commit` applies accepted pickup outcomes to world persistence.
///
/// The adapter also owns the synchronized surface world (entities, frames,
/// masks), so it is responsible for registering shots spawned by the lander
/// reducer and stepping the deterministic world simulation each frame.
pub trait PlanetSideCollision {
    fn contacts(&mut self, lander: &LanderState) -> Result<Vec<CollisionContact>, AdapterError>;

    fn commit(
        &mut self,
        _contact: CollisionContact,
        _outcome: &CollisionOutcome,
        _crew: u8,
    ) -> Result<SpecialPickupEffects, AdapterError> {
        Ok(SpecialPickupEffects::default())
    }

    /// Register a shot spawned by the lander reducer into the shared world,
    /// frames, and masks.
    fn register_shot(&mut self, _shot: Shot) -> Result<(), AdapterError> {
        Ok(())
    }

    /// Step the surface world for one frame: creature AI, shot movement/lifetime,
    /// shot-creature collisions, and hazard spawning.
    fn step_world(
        &mut self,
        _lander: &LanderState,
        _chances: HazardChances,
    ) -> Result<WorldStepResult, AdapterError> {
        Ok(WorldStepResult::default())
    }

    /// Apply the results of a world step: insert hazard spawns, transform canned
    /// creatures, and play hazard sounds.
    fn apply_world_step(
        &mut self,
        _result: &WorldStepResult,
        _audio: &mut impl PlanetSideAudio,
    ) -> Result<(), AdapterError> {
        Ok(())
    }
}

/// Graphics adapter for a complete deterministic frame snapshot.
pub trait PlanetSideGraphics {
    fn render(&mut self, snapshot: &RenderSnapshot) -> Result<(), AdapterError>;
}

/// Audio adapter consuming typed gameplay cues.
pub trait PlanetSideAudio {
    fn play(&mut self, cue: SoundCue) -> Result<(), AdapterError>;
}

/// Monotonic clock adapter. The core never sleeps directly.
pub trait PlanetSideClock {
    fn now(&self) -> Tick;
    fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError>;
}

/// Orbit ship-state adapter. A trip is committed with one typed delta.
pub trait ShipStatusPort {
    fn apply(&mut self, delta: &ShipDelta) -> Result<(), AdapterError>;
}

/// Failure from one synchronous runtime frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Adapter(AdapterError),
}

impl From<AdapterError> for RuntimeError {
    fn from(value: AdapterError) -> Self {
        Self::Adapter(value)
    }
}

/// Result of one runtime frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStep {
    Continue,
    Complete(SessionOutcome),
}

/// Concrete adapter bundle used by the synchronous runtime.
pub struct RuntimeAdapters<I, C, G, A, K, S> {
    pub input: I,
    pub collision: C,
    pub graphics: G,
    pub audio: A,
    pub clock: K,
    pub ship: S,
}

/// Drive one canonical frame and execute its typed effects in source order.
pub fn run_frame<I, C, G, A, K, S>(
    session: &mut PlanetSideSession,
    adapters: &mut RuntimeAdapters<I, C, G, A, K, S>,
    tick_period: u32,
) -> Result<RuntimeStep, RuntimeError>
where
    I: PlanetSideInput,
    C: PlanetSideCollision,
    G: PlanetSideGraphics,
    A: PlanetSideAudio,
    K: PlanetSideClock,
    S: ShipStatusPort,
{
    let deadline = Tick(adapters.clock.now().0.wrapping_add(tick_period));

    match session.phase {
        SessionPhase::Warmup => {
            session.phase = SessionPhase::Launch;
            session.animation.reset();
            adapters.audio.play(SoundCue::Departs)?;
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        SessionPhase::Launch => {
            if session.advance_launch() {
                session.phase = SessionPhase::Landing;
                session.animation.reset();
            }
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        SessionPhase::Landing => {
            if session.advance_landing() {
                session.phase = SessionPhase::Active;
                session.lander.in_transit = false;
                session.animation.reset();
            }
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        SessionPhase::TakingOff => {
            if session.advance_takeoff() {
                session.phase = SessionPhase::Return;
                session.animation.reset();
                adapters.audio.play(SoundCue::Returns)?;
            }
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        SessionPhase::Return => {
            if session.advance_return() {
                let outcome = session.settle();
                if let SessionOutcome::Returned(delta) | SessionOutcome::LanderDestroyed(delta) =
                    &outcome
                {
                    adapters.ship.apply(delta)?;
                }
                adapters.clock.sleep_until(deadline)?;
                return Ok(RuntimeStep::Complete(outcome));
            }
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        SessionPhase::Explosion => {
            if session.advance_explosion() {
                let outcome = session.settle();
                if let SessionOutcome::LanderDestroyed(delta) = &outcome {
                    adapters.ship.apply(delta)?;
                }
                render_lifecycle(session, adapters)?;
                adapters.clock.sleep_until(deadline)?;
                return Ok(RuntimeStep::Complete(outcome));
            }
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        SessionPhase::Active | SessionPhase::Complete | SessionPhase::Aborted => {}
    }

    debug_assert!(
        session.phase == SessionPhase::Active,
        "run_frame gameplay body reached in non-Active phase: {:?}",
        session.phase
    );

    let input = adapters.input.poll()?;

    match simulation::tick(&mut session.lander, input) {
        TickResult::Aborted => {
            session.abort();
            return Ok(RuntimeStep::Complete(SessionOutcome::Aborted));
        }
        TickResult::Takeoff => {
            // Enter the takeoff animation without settling. Settlement happens
            // only after the TakingOff → Return sequence completes.
            session.request_takeoff();
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        TickResult::Continue(effects) => {
            execute_simulation_effects(&mut adapters.collision, &mut adapters.audio, effects)?
        }
    }

    // Step the surface world: creature AI, shot movement, hazard spawning.
    let world_result = adapters
        .collision
        .step_world(&session.lander, session.hazard_chances)?;
    adapters
        .collision
        .apply_world_step(&world_result, &mut adapters.audio)?;
    if world_result.lightning_kills > 0 {
        session.lander.crew.lose(world_result.lightning_kills);
        if session.lander.crew.get() == 0 {
            session.phase = SessionPhase::Explosion;
            session.animation.reset();
            adapters.audio.play(SoundCue::Destroyed)?;
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
    }

    for contact in adapters.collision.contacts(&session.lander)? {
        let outcome = resolve_lander_collision(
            &mut CollisionState {
                crew: &mut session.lander.crew,
                shields: session.lander.upgrades.shields,
                minerals: &mut session.minerals,
                biological: &mut session.biological,
            },
            contact.collision,
            contact.rolls,
        );
        execute_collision_effects(&mut adapters.audio, &outcome)?;
        let effects = adapters
            .collision
            .commit(contact, &outcome, session.lander.crew.get())?;
        session.lander.crew.lose(effects.crew_killed);
        if effects.takeoff_requested {
            session.request_takeoff();
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
        if session.lander.crew.get() == 0 {
            session.phase = SessionPhase::Explosion;
            session.animation.reset();
            adapters.audio.play(SoundCue::Destroyed)?;
            render_lifecycle(session, adapters)?;
            adapters.clock.sleep_until(deadline)?;
            return Ok(RuntimeStep::Continue);
        }
    }

    // Active gameplay frame: render with current cargo meters so the life
    // meter visibly fills on every pickup.
    adapters.graphics.render(&RenderSnapshot {
        phase: session.phase,
        lander: session.lander.clone(),
        mineral_level: session.minerals.level(),
        biological_level: session.biological.level(),
        animation_frame: 0,
        lifecycle_offset: 0,
    })?;
    adapters.clock.sleep_until(deadline)?;
    Ok(RuntimeStep::Continue)
}

/// Render one lifecycle animation frame (Launch, Landing, TakingOff, Return,
/// Explosion).
fn render_lifecycle<I, C, G, A, K, S>(
    session: &PlanetSideSession,
    adapters: &mut RuntimeAdapters<I, C, G, A, K, S>,
) -> Result<(), RuntimeError>
where
    G: PlanetSideGraphics,
{
    let offset = match session.phase {
        SessionPhase::Landing => session.animation.landing_offset(),
        SessionPhase::TakingOff => session.animation.takeoff_offset(),
        _ => 0,
    };
    adapters.graphics.render(&RenderSnapshot {
        phase: session.phase,
        lander: session.lander.clone(),
        mineral_level: session.minerals.level(),
        biological_level: session.biological.level(),
        animation_frame: session.animation.frame(),
        lifecycle_offset: offset,
    })?;
    Ok(())
}

fn execute_simulation_effects<C: PlanetSideCollision, A: PlanetSideAudio>(
    collision: &mut C,
    audio: &mut A,
    effects: Vec<SimulationEffect>,
) -> Result<(), AdapterError> {
    for effect in effects {
        match effect {
            SimulationEffect::Play(cue) => audio.play(cue)?,
            SimulationEffect::SpawnShot(shot) => collision.register_shot(shot)?,
        }
    }
    Ok(())
}

fn execute_collision_effects<A: PlanetSideAudio>(
    audio: &mut A,
    outcome: &CollisionOutcome,
) -> Result<(), AdapterError> {
    if let CollisionOutcome::CrewDamage(damage) = outcome {
        for cue in &damage.sounds {
            audio.play(*cue)?;
        }
    } else if let CollisionOutcome::Cargo { pickup, .. } = outcome {
        let cue = if matches!(pickup, super::cargo::CargoPickup::Full) {
            SoundCue::Full
        } else {
            SoundCue::Pickup
        };
        audio.play(cue)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::cargo::MineralCargo;
    use crate::planet_side::creatures::CreatureDanger;
    use crate::planet_side::model::{CrewCount, LanderUpgrades, SurfacePoint};
    use crate::planet_side::simulation::LanderState;

    struct Input(FrameInput);
    impl PlanetSideInput for Input {
        fn poll(&mut self) -> Result<FrameInput, AdapterError> {
            Ok(self.0)
        }
    }

    struct Collisions(Vec<(LanderCollision, CollisionRolls)>);
    impl PlanetSideCollision for Collisions {
        fn contacts(
            &mut self,
            _lander: &LanderState,
        ) -> Result<Vec<CollisionContact>, AdapterError> {
            Ok(std::mem::take(&mut self.0)
                .into_iter()
                .map(|(collision, rolls)| CollisionContact {
                    entity: None,
                    collision,
                    rolls,
                })
                .collect())
        }
    }

    struct EffectCollision(SpecialPickupEffects);
    impl PlanetSideCollision for EffectCollision {
        fn contacts(
            &mut self,
            _lander: &LanderState,
        ) -> Result<Vec<CollisionContact>, AdapterError> {
            Ok(vec![CollisionContact {
                entity: None,
                collision: LanderCollision::Energy { node: 0 },
                rolls: CollisionRolls::default(),
            }])
        }

        fn commit(
            &mut self,
            _contact: CollisionContact,
            _outcome: &CollisionOutcome,
            _crew: u8,
        ) -> Result<SpecialPickupEffects, AdapterError> {
            Ok(self.0)
        }
    }
    #[derive(Default)]
    struct Graphics(Vec<RenderSnapshot>);
    impl PlanetSideGraphics for Graphics {
        fn render(&mut self, snapshot: &RenderSnapshot) -> Result<(), AdapterError> {
            self.0.push(snapshot.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct Audio(Vec<SoundCue>);
    impl PlanetSideAudio for Audio {
        fn play(&mut self, cue: SoundCue) -> Result<(), AdapterError> {
            self.0.push(cue);
            Ok(())
        }
    }

    #[derive(Default)]
    struct Clock {
        now: Tick,
        slept: Vec<Tick>,
    }
    impl PlanetSideClock for Clock {
        fn now(&self) -> Tick {
            self.now
        }

        fn sleep_until(&mut self, deadline: Tick) -> Result<(), AdapterError> {
            self.slept.push(deadline);
            self.now = deadline;
            Ok(())
        }
    }

    #[derive(Default)]
    struct Ship(Vec<ShipDelta>);
    impl ShipStatusPort for Ship {
        fn apply(&mut self, delta: &ShipDelta) -> Result<(), AdapterError> {
            self.0.push(delta.clone());
            Ok(())
        }
    }

    fn session() -> PlanetSideSession {
        let mut session = PlanetSideSession::new(
            LanderState::new(
                SurfacePoint::default(),
                0,
                CrewCount::new(12),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(100, 0, false),
        );
        session.phase = SessionPhase::Active;
        session
    }

    #[test]
    fn warmup_transitions_to_launch_and_plays_departs_sound() {
        let mut session = PlanetSideSession::new(
            LanderState::new(
                SurfacePoint { x: 40, y: 20 },
                0,
                CrewCount::new(12),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(100, 0, false),
        );
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput {
                thrust: true,
                ..FrameInput::default()
            }),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.phase, SessionPhase::Launch);
        assert_eq!(session.lander.position, SurfacePoint { x: 40, y: 20 });
        assert_eq!(adapters.graphics.0.len(), 1);
        assert_eq!(adapters.graphics.0[0].phase, SessionPhase::Launch);
        assert_eq!(adapters.audio.0, [SoundCue::Departs]);
        assert_eq!(adapters.clock.slept, [Tick(1)]);
    }

    #[test]
    fn frame_routes_dangerous_collision_damage_and_sounds_before_render() {
        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: Collisions(vec![(
                LanderCollision::LiveCreature {
                    danger: CreatureDanger::Monstrous,
                },
                CollisionRolls {
                    biological_attack: 0,
                    shield: 99,
                    ..CollisionRolls::default()
                },
            )]),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.lander.crew, CrewCount::new(11));
        assert_eq!(
            adapters.audio.0,
            [SoundCue::BiologicalDisaster, SoundCue::LanderInjured]
        );
        assert_eq!(adapters.graphics.0[0].lander.crew, CrewCount::new(11));
        assert_eq!(adapters.clock.slept, [Tick(1)]);
    }

    #[test]
    fn takeoff_enters_takingoff_phase_before_return_sound() {
        let mut session = session();
        session.minerals.collect(2, 4);
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput {
                takeoff: true,
                ..FrameInput::default()
            }),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        // First frame: takeoff is requested; session enters TakingOff phase
        // but does NOT settle immediately.
        let result = run_frame(&mut session, &mut adapters, 1);
        assert_eq!(result, Ok(RuntimeStep::Continue));
        assert_eq!(session.phase, SessionPhase::TakingOff);
        assert!(adapters.audio.0.is_empty());
        assert!(session.lander.in_transit);
        // No ship delta applied yet — settlement happens after Return anim.
        assert!(adapters.ship.0.is_empty());
        // Lifecycle frame is rendered.
        assert_eq!(adapters.graphics.0.len(), 1);
        assert_eq!(adapters.graphics.0[0].phase, SessionPhase::TakingOff);
    }

    #[test]
    fn abort_never_applies_ship_writeback() {
        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput {
                abort: true,
                ..FrameInput::default()
            }),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Complete(SessionOutcome::Aborted))
        );
        assert!(adapters.ship.0.is_empty());
    }

    #[test]
    fn fatal_collision_enters_explosion_phase_and_delays_settlement() {
        let mut session = session();
        session.lander.crew = CrewCount::new(1);
        session.minerals.collect(2, 9);
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: Collisions(vec![(
                LanderCollision::LiveCreature {
                    danger: CreatureDanger::Monstrous,
                },
                CollisionRolls {
                    biological_attack: 0,
                    shield: 99,
                    ..CollisionRolls::default()
                },
            )]),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        // First frame: fatal collision enters Explosion phase. The session
        // does NOT settle immediately — it animates the explosion first.
        let result = run_frame(&mut session, &mut adapters, 1);
        assert_eq!(result, Ok(RuntimeStep::Continue));
        assert_eq!(session.phase, SessionPhase::Explosion);
        assert_eq!(
            adapters.audio.0,
            [
                SoundCue::BiologicalDisaster,
                SoundCue::LanderInjured,
                SoundCue::Destroyed
            ]
        );
        assert!(adapters.ship.0.is_empty());
        assert_eq!(adapters.graphics.0.len(), 1);
        assert_eq!(adapters.graphics.0[0].phase, SessionPhase::Explosion);
        assert_eq!(adapters.clock.slept.len(), 1);

        // Run enough frames to complete the explosion animation.
        for _ in 0..super::super::lifecycle::EXPLOSION_TOTAL_FRAMES {
            let step = run_frame(&mut session, &mut adapters, 1);
            if let Ok(RuntimeStep::Complete(SessionOutcome::LanderDestroyed(delta))) = step {
                assert_eq!(delta.landers, -1);
                assert_eq!(delta.element_mass, 0);
                assert_eq!(adapters.ship.0.len(), 1);
                assert_eq!(adapters.ship.0[0].landers, -1);
                return;
            }
        }
        panic!("explosion should settle as destroyed after animation");
    }

    #[test]
    fn special_pickup_effects_update_crew_and_request_takeoff() {
        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: EffectCollision(SpecialPickupEffects {
                crew_killed: 3,
                takeoff_requested: true,
            }),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.lander.crew, CrewCount::new(9));
        assert_eq!(session.phase, SessionPhase::TakingOff);
        assert!(session.lander.in_transit);
        // Returns sound is played when entering TakingOff via special effects.
        assert!(adapters.audio.0.is_empty());
    }

    #[test]
    fn special_pickup_auto_takeoff_completes_without_input() {
        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: EffectCollision(SpecialPickupEffects {
                crew_killed: 0,
                takeoff_requested: true,
            }),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.phase, SessionPhase::TakingOff);
        adapters.collision.0 = SpecialPickupEffects::default();

        let frame_limit = super::super::lifecycle::LifecycleAnimation::takeoff_total()
            + session.return_frame_count.max(1)
            + 2;
        let mut completed = None;
        for _ in 0..frame_limit {
            match run_frame(&mut session, &mut adapters, 1).expect("special takeoff frame") {
                RuntimeStep::Continue => {}
                RuntimeStep::Complete(outcome) => {
                    completed = Some(outcome);
                    break;
                }
            }
        }

        assert!(matches!(completed, Some(SessionOutcome::Returned(_))));
        assert_eq!(adapters.audio.0, [SoundCue::Returns]);
        assert_eq!(session.phase, SessionPhase::Complete);
    }

    #[test]
    fn full_phase_progression_warmup_to_complete_via_takeoff() {
        let mut session = PlanetSideSession::new(
            LanderState::new(
                SurfacePoint::default(),
                0,
                CrewCount::new(5),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(100, 0, false),
        );
        session.set_lifecycle_frame_counts(2, 2);

        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        // Warmup → Launch
        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.phase, SessionPhase::Launch);

        // Launch (2 frames)
        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.phase, SessionPhase::Landing);

        // Landing (48 frames)
        let landing_total = super::super::lifecycle::LifecycleAnimation::landing_total();
        for _ in 0..landing_total {
            assert_eq!(
                run_frame(&mut session, &mut adapters, 1),
                Ok(RuntimeStep::Continue)
            );
        }
        assert_eq!(session.phase, SessionPhase::Active);

        // Collect minerals so we can verify the delta at settlement.
        session.minerals.collect(1, 5);

        // Active: one idle frame
        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.phase, SessionPhase::Active);

        // Request takeoff via simulation input
        let mut takeoff_input = RuntimeAdapters {
            input: Input(FrameInput {
                takeoff: true,
                ..FrameInput::default()
            }),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };
        assert_eq!(
            run_frame(&mut session, &mut takeoff_input, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(session.phase, SessionPhase::TakingOff);

        // Takeoff (31 frames)
        let takeoff_total = super::super::lifecycle::LifecycleAnimation::takeoff_total();
        for _ in 0..takeoff_total {
            assert_eq!(
                run_frame(&mut session, &mut takeoff_input, 1),
                Ok(RuntimeStep::Continue)
            );
        }
        assert_eq!(session.phase, SessionPhase::Return);
        assert_eq!(takeoff_input.audio.0, [SoundCue::Returns]);

        // Return (2 frames)
        assert_eq!(
            run_frame(&mut session, &mut takeoff_input, 1),
            Ok(RuntimeStep::Continue)
        );
        let final_result = run_frame(&mut session, &mut takeoff_input, 1);
        let Ok(RuntimeStep::Complete(SessionOutcome::Returned(delta))) = final_result else {
            panic!("expected returned outcome, got {:?}", final_result);
        };
        assert_eq!(delta.crew, 5);
        assert_eq!(delta.element_mass, 5);
        assert_eq!(delta.element_amounts[1], 5);
        assert_eq!(takeoff_input.ship.0.len(), 1);
        assert_eq!(takeoff_input.ship.0[0].element_mass, 5);
        assert_eq!(session.phase, SessionPhase::Complete);
    }

    #[test]
    fn cargo_meters_update_every_active_frame() {
        let mut session = session();
        session.minerals.collect(0, 3);
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        run_frame(&mut session, &mut adapters, 1).unwrap();
        assert_eq!(adapters.graphics.0.last().unwrap().mineral_level, 3);
        assert_eq!(adapters.graphics.0.last().unwrap().biological_level, 0);

        // Simulate a bio pickup mid-session.
        session.biological.collect(2);
        run_frame(&mut session, &mut adapters, 1).unwrap();
        assert_eq!(adapters.graphics.0.last().unwrap().biological_level, 2);
    }

    #[test]
    fn lifecycle_phases_render_with_animation_frame_and_offset() {
        let mut session = PlanetSideSession::new(
            LanderState::new(
                SurfacePoint::default(),
                0,
                CrewCount::new(5),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(100, 0, false),
        );
        session.set_lifecycle_frame_counts(5, 3);
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        // Warmup → Launch, first launch frame
        run_frame(&mut session, &mut adapters, 1).unwrap();
        assert_eq!(session.phase, SessionPhase::Launch);
        let snap = &adapters.graphics.0[0];
        assert_eq!(snap.phase, SessionPhase::Launch);
        assert_eq!(snap.animation_frame, 0);

        // Advance one launch frame
        run_frame(&mut session, &mut adapters, 1).unwrap();
        let snap = adapters.graphics.0.last().unwrap();
        assert_eq!(snap.phase, SessionPhase::Launch);
        assert_eq!(snap.animation_frame, 1);

        // Skip to Landing
        for _ in 0..5 {
            run_frame(&mut session, &mut adapters, 1).unwrap();
        }
        assert_eq!(session.phase, SessionPhase::Landing);
        let snap = adapters.graphics.0.last().unwrap();
        assert_eq!(snap.phase, SessionPhase::Landing);
        // During idle portion, offset is -DISTANCE_COVERED.
        assert_eq!(
            snap.lifecycle_offset,
            -super::super::lifecycle::DISTANCE_COVERED
        );
    }

    #[test]
    fn adapter_failure_stops_the_frame_as_a_typed_error() {
        struct FailingInput;
        impl PlanetSideInput for FailingInput {
            fn poll(&mut self) -> Result<FrameInput, AdapterError> {
                Err(AdapterError::new("poll_input"))
            }
        }

        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: FailingInput,
            collision: Collisions(Vec::new()),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };
        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Err(RuntimeError::Adapter(AdapterError::new("poll_input")))
        );
    }

    /// Collision adapter that tracks registered shots and world steps for
    /// integration testing of the frame loop.
    #[derive(Default)]
    struct WorldTrackingCollision {
        registered_shots: Vec<super::super::simulation::Shot>,
        world_steps: usize,
    }

    impl PlanetSideCollision for WorldTrackingCollision {
        fn contacts(
            &mut self,
            _lander: &LanderState,
        ) -> Result<Vec<CollisionContact>, AdapterError> {
            Ok(Vec::new())
        }

        fn register_shot(
            &mut self,
            shot: super::super::simulation::Shot,
        ) -> Result<(), AdapterError> {
            self.registered_shots.push(shot);
            Ok(())
        }

        fn step_world(
            &mut self,
            _lander: &LanderState,
            _chances: super::super::world::HazardChances,
        ) -> Result<WorldStepResult, AdapterError> {
            self.world_steps += 1;
            Ok(WorldStepResult::default())
        }
    }

    #[test]
    fn spawn_shot_effect_registers_shot_in_collision_adapter() {
        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput {
                fire: true,
                ..FrameInput::default()
            }),
            collision: WorldTrackingCollision::default(),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        assert_eq!(
            run_frame(&mut session, &mut adapters, 1),
            Ok(RuntimeStep::Continue)
        );
        assert_eq!(
            adapters.collision.registered_shots.len(),
            1,
            "SpawnShot effect must reach the collision adapter"
        );
        assert_eq!(
            adapters.collision.registered_shots[0].life, 12,
            "shot must have 12-tick lifetime"
        );
    }

    #[test]
    fn world_step_advances_each_active_frame() {
        let mut session = session();
        let mut adapters = RuntimeAdapters {
            input: Input(FrameInput::default()),
            collision: WorldTrackingCollision::default(),
            graphics: Graphics::default(),
            audio: Audio::default(),
            clock: Clock::default(),
            ship: Ship::default(),
        };

        for _ in 0..3 {
            assert_eq!(
                run_frame(&mut session, &mut adapters, 1),
                Ok(RuntimeStep::Continue)
            );
        }
        assert_eq!(
            adapters.collision.world_steps, 3,
            "world must advance once per active frame"
        );
    }
}
