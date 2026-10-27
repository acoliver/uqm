//! Admission of the explicit bounded parity fixtures.

use super::*;
use crate::automation::melee_fixture;

impl Coordinator {
    pub(super) fn parity_fixture_input(
        &self,
        inner: &mut CoordInner,
        event: SchedulerEvent,
    ) -> bool {
        let result = self.execute_parity_fixture(inner, event);
        match result {
            Ok(Some(label)) => {
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
                false
            }
            Ok(None) => false,
            Err(error) => {
                self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!("parity_fixture_failed:{error}"),
                );
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                true
            }
        }
    }

    fn execute_parity_fixture(
        &self,
        inner: &mut CoordInner,
        event: SchedulerEvent,
    ) -> Result<Option<String>, String> {
        let witnesses = crate::collections::observation::take_saves();
        if let Some(run) = inner.parity_fixture.as_mut() {
            for (slot, loading) in witnesses {
                if loading {
                    run.loaded(slot)?;
                } else {
                    run.saved(slot)?;
                }
            }
            run.advance()?;
        }
        if inner.sched_state.phase != ActionPhase::WaitingForInput
            || event != SchedulerEvent::AdmittedInput
        {
            return Ok(None);
        }
        match self.actions.get(inner.sched_state.step_index) {
            Some(Action::ConfigureParityMelee(request)) => {
                melee_fixture::configure(request)?;
                Ok(Some("parity_melee_configured".into()))
            }
            Some(Action::RunParityCorpus(request)) => {
                melee_fixture::corpus(*request)?;
                Ok(Some(format!("parity_corpus_completed:{}", request.fleet)))
            }
            Some(Action::RunParityFixture(request)) => {
                if inner.parity_fixture.is_some() {
                    return Err("a parity fixture is already active".into());
                }
                inner.parity_fixture = Some(melee_fixture::run(request)?);
                Ok(Some("parity_fixture_started".into()))
            }
            Some(Action::AssertParityFixture) => {
                let proof = inner
                    .parity_fixture
                    .as_ref()
                    .ok_or("no active parity fixture")?;
                Ok(Some(proof.verify()?))
            }
            _ => Ok(None),
        }
    }
}
