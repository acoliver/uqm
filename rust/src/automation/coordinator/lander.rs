//! Landed-trip lifecycle waits and their trace observations.

use super::*;

impl AutomationObserver {
    /// Service whichever planet-side wait is the current action.
    ///
    /// Returns `None` when the current action is not a planet-side wait, so the
    /// caller can fall through to the remaining action handlers.
    pub(super) fn service_planet_side_wait(
        &self,
        inner: &mut CoordInner,
    ) -> Option<PlanetSideWaitResult> {
        match self.actions.get(inner.sched_state.step_index) {
            Some(Action::WaitForPlanetSideStart(_)) => Some(self.observe_planet_side_start(inner)),
            Some(Action::WaitForPlanetSideEnd(wait)) => {
                Some(self.observe_planet_side_end(inner, wait.outcome))
            }
            _ => None,
        }
    }

    /// Record a serviced planet-side wait, returning the scheduler event to use.
    pub(super) fn record_planet_side_result(
        &self,
        inner: &mut CoordInner,
        result: PlanetSideWaitResult,
        current: SchedulerEvent,
    ) -> SchedulerEvent {
        for label in result.labels {
            self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
        }
        if result.reached {
            SchedulerEvent::ConditionReached
        } else {
            current
        }
    }

    /// Latch the generation of a newly started planet-side trip.
    fn observe_planet_side_start(&self, inner: &mut CoordInner) -> PlanetSideWaitResult {
        let observation = crate::planet_side::telemetry::observation();
        if !observation.active {
            return PlanetSideWaitResult::pending();
        }
        inner.awaited_planet_side_generation = Some(observation.generation);
        inner.observed_planet_side_phase = None;
        PlanetSideWaitResult::reached(format!(
            "planet_side_started:generation={}:crew={}:position={},{}",
            observation.generation,
            observation.start_crew,
            observation.start_x,
            observation.start_y
        ))
    }

    /// Accept the settled outcome of the trip this wait is bound to, tracing
    /// every lifecycle transition seen along the way.
    ///
    /// A trip publishes its terminal code only once it has settled, and that
    /// code stays resident afterwards. Matching on the latched generation is
    /// therefore required: without it a later wait would immediately consume an
    /// earlier trip's result. Transitions are rare, so recording each one shows
    /// whether a trip that never ends is stuck before takeoff or stalled in the
    /// takeoff animation. Deduplication is keyed by generation as well as phase
    /// so the same phase in a later trip is not swallowed.
    fn observe_planet_side_end(
        &self,
        inner: &mut CoordInner,
        outcome: crate::automation::script::PlanetSideOutcomeName,
    ) -> PlanetSideWaitResult {
        let observation = crate::planet_side::telemetry::observation();
        let mut result = PlanetSideWaitResult::pending();

        let seen = (observation.generation, observation.phase);
        if inner.observed_planet_side_phase != Some(seen) {
            inner.observed_planet_side_phase = Some(seen);
            result.labels.push(format!(
                "planet_side_phase:generation={}:phase={}",
                observation.generation,
                crate::engine::observe_port::planet_side::phase_name(observation.phase)
            ));
        }

        if !planet_side_wait_satisfied(inner.awaited_planet_side_generation, &observation, outcome)
        {
            return result;
        }
        inner.awaited_planet_side_generation = None;
        result.reached = true;
        result.labels.push(format!(
            "planet_side_completed:generation={}:outcome={outcome:?}:crew={}:minerals={}",
            observation.generation, observation.returned_crew, observation.returned_minerals
        ));
        result
    }
}
