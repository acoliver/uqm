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

    pub fn run(&mut self) -> Result<SessionOutcome, ControllerError> {
        for _ in 0..self.frame_budget {
            match run_frame(&mut self.session, &mut self.adapters, self.tick_period)? {
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
        fn play(&mut self, _cue: super::super::hazards::SoundCue) -> Result<(), AdapterError> {
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
    fn controller_runs_until_one_terminal_outcome() {
        // Takeoff now runs through TakingOff + Return animation phases before
        // settling, so the budget must cover those extra frames.
        let takeoff_total = super::super::lifecycle::LifecycleAnimation::takeoff_total();
        let budget = 2 + u32::from(takeoff_total) + 1;
        let mut input = vec![FrameInput::default()];
        input.resize(budget as usize, FrameInput::default());
        input[1].takeoff = true;
        let mut controller = PlanetSideController::new(session(), adapters(input), 1, budget);
        assert!(matches!(controller.run(), Ok(SessionOutcome::Returned(_))));
        assert_eq!(controller.adapters.ship.0.len(), 1);
    }

    #[test]
    fn controller_has_a_hard_frame_budget() {
        let mut controller =
            PlanetSideController::new(session(), adapters(vec![FrameInput::default()]), 1, 1);
        assert_eq!(controller.run(), Err(ControllerError::FrameBudgetExceeded));
    }
}
