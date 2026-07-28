//! Synchronous PlanetSide orchestration over typed runtime adapters.

use super::collision::{
    resolve_lander_collision, CollisionOutcome, CollisionRolls, CollisionState, LanderCollision,
};
use super::entities::SurfaceEntityId;
use super::hazards::SoundCue;
use super::session::{PlanetSideSession, SessionOutcome, SessionPhase, ShipDelta};
use super::simulation::{self, FrameInput, LanderState, SimulationEffect, TickResult};
use super::special_effects::SpecialPickupEffects;

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

/// Collision geometry and interaction adapter. Gameplay resolution stays in
/// Rust; `commit` applies accepted pickup outcomes to world persistence.
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
    if session.phase == SessionPhase::Warmup {
        session.phase = SessionPhase::Active;
        session.lander.in_transit = false;
        adapters.graphics.render(&RenderSnapshot {
            phase: session.phase,
            lander: session.lander.clone(),
            mineral_level: session.minerals.level(),
            biological_level: session.biological.level(),
        })?;
        adapters.clock.sleep_until(deadline)?;
        return Ok(RuntimeStep::Continue);
    }
    let input = adapters.input.poll()?;

    match simulation::tick(&mut session.lander, input) {
        TickResult::Aborted => {
            session.abort();
            return Ok(RuntimeStep::Complete(SessionOutcome::Aborted));
        }
        TickResult::Takeoff => {
            session.phase = SessionPhase::TakingOff;
            let outcome = session.settle();
            if let SessionOutcome::Returned(delta) | SessionOutcome::LanderDestroyed(delta) =
                &outcome
            {
                adapters.ship.apply(delta)?;
            }
            return Ok(RuntimeStep::Complete(outcome));
        }
        TickResult::Continue(effects) => execute_simulation_effects(&mut adapters.audio, effects)?,
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
        }
        if session.lander.crew.get() == 0 {
            session.phase = SessionPhase::Explosion;
            adapters.audio.play(SoundCue::Destroyed)?;
            let outcome = session.settle();
            if let SessionOutcome::LanderDestroyed(delta) = &outcome {
                adapters.ship.apply(delta)?;
            }
            return Ok(RuntimeStep::Complete(outcome));
        }
    }

    adapters.graphics.render(&RenderSnapshot {
        phase: session.phase,
        lander: session.lander.clone(),
        mineral_level: session.minerals.level(),
        biological_level: session.biological.level(),
    })?;
    adapters.clock.sleep_until(deadline)?;
    Ok(RuntimeStep::Continue)
}

fn execute_simulation_effects<A: PlanetSideAudio>(
    audio: &mut A,
    effects: Vec<SimulationEffect>,
) -> Result<(), AdapterError> {
    for effect in effects {
        if let SimulationEffect::Play(cue) = effect {
            audio.play(cue)?;
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
    fn warmup_renders_one_stationary_active_frame_before_polling_input() {
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
        assert_eq!(session.phase, SessionPhase::Active);
        assert_eq!(session.lander.position, SurfacePoint { x: 40, y: 20 });
        assert_eq!(adapters.graphics.0.len(), 1);
        assert_eq!(adapters.graphics.0[0].phase, SessionPhase::Active);
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
    fn takeoff_commits_exactly_one_ship_delta_without_rendering() {
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

        let result = run_frame(&mut session, &mut adapters, 1);
        assert!(matches!(
            result,
            Ok(RuntimeStep::Complete(SessionOutcome::Returned(_)))
        ));
        assert_eq!(adapters.ship.0.len(), 1);
        assert_eq!(adapters.ship.0[0].element_amounts[2], 4);
        assert!(adapters.graphics.0.is_empty());
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
    fn fatal_collision_plays_destruction_and_commits_lander_loss_once() {
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

        let result = run_frame(&mut session, &mut adapters, 1);
        let Ok(RuntimeStep::Complete(SessionOutcome::LanderDestroyed(delta))) = result else {
            panic!("expected destroyed lander");
        };
        assert_eq!(delta.landers, -1);
        assert_eq!(delta.element_mass, 0);
        assert_eq!(
            adapters.audio.0,
            [
                SoundCue::BiologicalDisaster,
                SoundCue::LanderInjured,
                SoundCue::Destroyed
            ]
        );
        assert_eq!(adapters.ship.0, [delta]);
        assert!(adapters.graphics.0.is_empty());
        assert!(adapters.clock.slept.is_empty());
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
}
