//! Complete synchronous PlanetSide session controller.

use super::runtime::{run_frame, RuntimeAdapters, RuntimeError, RuntimeStep};
use super::session::{PlanetSideSession, SessionOutcome};

/// Controller-level failure. A frame budget protects transitional callers from
/// re-entering or hanging indefinitely while runtime ownership moves to Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerError {
    Runtime(RuntimeError),
    FrameBudgetExceeded,
}

impl From<RuntimeError> for ControllerError {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

/// Own one PlanetSide trip until a single terminal outcome is produced.
pub struct PlanetSideController<I, C, G, A, K, S> {
    pub session: PlanetSideSession,
    pub adapters: RuntimeAdapters<I, C, G, A, K, S>,
    tick_period: u32,
    frame_budget: u32,
}

impl<I, C, G, A, K, S> PlanetSideController<I, C, G, A, K, S>
where
    I: super::runtime::PlanetSideInput,
    C: super::runtime::PlanetSideCollision,
    G: super::runtime::PlanetSideGraphics,
    A: super::runtime::PlanetSideAudio,
    K: super::runtime::PlanetSideClock,
    S: super::runtime::ShipStatusPort,
{
    #[must_use]
    pub fn new(
        session: PlanetSideSession,
        adapters: RuntimeAdapters<I, C, G, A, K, S>,
        tick_period: u32,
        frame_budget: u32,
    ) -> Self {
        Self {
            session,
            adapters,
            tick_period,
            frame_budget,
        }
    }

    pub fn run(
        &mut self,
        cx: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
    ) -> Result<SessionOutcome, ControllerError> {
        self.run_with_frame(|session, adapters, period| run_frame(cx, session, adapters, period))
    }

    pub(crate) fn run_with_frame(
        &mut self,
        mut frame: impl FnMut(
            &mut PlanetSideSession,
            &mut RuntimeAdapters<I, C, G, A, K, S>,
            u32,
        ) -> Result<RuntimeStep, RuntimeError>,
    ) -> Result<SessionOutcome, ControllerError> {
        for _ in 0..self.frame_budget {
            match frame(&mut self.session, &mut self.adapters, self.tick_period)? {
                RuntimeStep::Continue => crate::planet_side::telemetry::frame(&self.session),
                RuntimeStep::Complete(outcome) => {
                    crate::planet_side::telemetry::frame(&self.session);
                    return Ok(outcome);
                }
            }
        }
        Err(ControllerError::FrameBudgetExceeded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::cargo::MineralCargo;
    use crate::planet_side::model::{CrewCount, LanderUpgrades, SurfacePoint};
    use crate::planet_side::runtime::{
        AdapterError, PlanetSideAudio, PlanetSideClock, PlanetSideCollision, PlanetSideGraphics,
        PlanetSideInput, RenderSnapshot, ShipStatusPort, Tick,
    };
    use crate::planet_side::session::{SessionPhase, ShipDelta};
    use crate::planet_side::simulation::{FrameInput, LanderState};

    struct Input(Vec<FrameInput>);
    impl PlanetSideInput for Input {
        fn poll(&mut self) -> Result<FrameInput, AdapterError> {
            Ok(self.0.remove(0))
        }
    }
    struct Collision;
    impl PlanetSideCollision for Collision {
        fn contacts(
            &mut self,
            _lander: &LanderState,
        ) -> Result<Vec<super::super::runtime::CollisionContact>, AdapterError> {
            Ok(Vec::new())
        }
    }
    struct Graphics;
    impl PlanetSideGraphics for Graphics {
        fn render(&mut self, _snapshot: &RenderSnapshot) -> Result<(), AdapterError> {
            Ok(())
        }
    }
    struct Audio;
    impl PlanetSideAudio for Audio {
        fn play(
            &mut self,
            _world: &crate::engine::World,
            _cue: super::super::hazards::SoundCue,
        ) -> Result<(), AdapterError> {
            Ok(())
        }
    }
    struct Clock;
    impl PlanetSideClock for Clock {
        fn now(&self) -> Tick {
            Tick(0)
        }
        fn sleep_until(&mut self, _deadline: Tick) -> Result<(), AdapterError> {
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
                CrewCount::new(4),
                LanderUpgrades::default(),
            ),
            MineralCargo::new(50, 0, false),
        );
        session.phase = SessionPhase::Active;
        session
    }

    fn adapters(
        input: Vec<FrameInput>,
    ) -> RuntimeAdapters<Input, Collision, Graphics, Audio, Clock, Ship> {
        RuntimeAdapters {
            input: Input(input),
            collision: Collision,
            graphics: Graphics,
            audio: Audio,
            clock: Clock,
            ship: Ship::default(),
        }
    }

    #[test]
    #[serial_test::serial]
    fn controller_runs_until_one_terminal_outcome() {
        let mut owner = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let cx = &std::cell::RefCell::new(crate::engine::battle::Cx::new(
            &mut battle,
            &mut owner.world,
            &mut owner.kernel,
            crate::math::random,
        ));

        // Takeoff now runs through TakingOff + Return animation phases before
        // settling, so the budget must cover those extra frames.
        let takeoff_total = super::super::lifecycle::LifecycleAnimation::takeoff_total();
        let budget = 2 + u32::from(takeoff_total) + 1;
        let mut input = vec![FrameInput::default()];
        input.resize(budget as usize, FrameInput::default());
        input[1].takeoff = true;
        let mut controller = PlanetSideController::new(session(), adapters(input), 1, budget);
        assert!(matches!(
            controller.run(cx),
            Ok(SessionOutcome::Returned(_))
        ));
        assert_eq!(controller.adapters.ship.0.len(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn controller_has_a_hard_frame_budget() {
        let mut owner = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let cx = &std::cell::RefCell::new(crate::engine::battle::Cx::new(
            &mut battle,
            &mut owner.world,
            &mut owner.kernel,
            crate::math::random,
        ));

        let mut controller =
            PlanetSideController::new(session(), adapters(vec![FrameInput::default()]), 1, 1);
        assert_eq!(
            controller.run(cx),
            Err(ControllerError::FrameBudgetExceeded)
        );
    }

    #[test]
    #[serial_test::serial]
    fn prepared_input_preserves_simulation_frames_and_terminal_settlement() {
        let mut owner = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let cx = &std::cell::RefCell::new(crate::engine::battle::Cx::new(
            &mut battle,
            &mut owner.world,
            &mut owner.kernel,
            crate::math::random,
        ));
        let budget = 3 + u32::from(super::super::lifecycle::LifecycleAnimation::takeoff_total());
        let mut input = vec![FrameInput::default(); budget as usize];
        input[1].takeoff = true;
        let mut direct = PlanetSideController::new(session(), adapters(input.clone()), 24, budget);
        let mut prepared = PlanetSideController::new(session(), adapters(input), 24, budget);
        let expected = direct.run(cx);
        let mut active_frames = 0;
        let actual = prepared.run_with_frame(|session, adapters, period| {
            let deadline = Tick(adapters.clock.now().0.wrapping_add(period));
            let input = if session.phase == SessionPhase::Active {
                active_frames += 1;
                Some(adapters.input.poll()?)
            } else {
                None
            };
            super::super::runtime::run_frame_at(cx, session, adapters, deadline, input)
        });
        assert_eq!(actual, expected);
        assert!(matches!(actual, Ok(SessionOutcome::Returned(_))));
        assert_eq!(active_frames, 2);
        assert_eq!(prepared.adapters.ship.0, direct.adapters.ship.0);
        assert_eq!(prepared.adapters.input.0, direct.adapters.input.0);
    }
}
