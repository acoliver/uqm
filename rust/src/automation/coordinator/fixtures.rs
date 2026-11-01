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
                let label = successful_label(
                    self.native_checkpoint_plan.as_ref(),
                    inner.sched_state.step_index,
                    label,
                );
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
            Some(Action::ConnectNetplay(request)) => {
                crate::automation::netplay_fixture::connect(request)?;
                Ok(Some("netplay_connected".into()))
            }
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

fn successful_label(
    plan: Option<&crate::automation::native_checkpoint::NativeCheckpointPlan>,
    action_index: usize,
    headless_label: String,
) -> String {
    let Some(plan) = plan else {
        return headless_label;
    };
    plan.checkpoints
        .iter()
        .find(|checkpoint| checkpoint.action_index == action_index)
        .and_then(|checkpoint| checkpoint.parity_label.clone())
        .unwrap_or_else(|| panic!("admitted native parity action has a bound proof obligation"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_parity_labels_change_only_in_native_proof_mode() {
        for (action, legacy) in [
            (
                serde_json::json!({"action":"configure_parity_melee","ships":[[7,8,9,10,11,12,13],[7,8,9,10,11,12,13]],"cyborg":true}),
                "parity_melee_configured",
            ),
            (
                serde_json::json!({"action":"run_parity_fixture","fixture":"mixed-collision"}),
                "parity_fixture_started",
            ),
            (
                serde_json::json!({"action":"run_parity_corpus","fleet":1}),
                "parity_corpus_completed:1",
            ),
        ] {
            let source = serde_json::to_vec(&serde_json::json!({
                "version":2,"name":"native-parity-test","fixture":"parity-test","seed":42,
                "budgets":{"max_input_ticks":100,"max_presentations":100,"max_wallclock_seconds":60},
                "steps":[action,{"action":"finish"}],
            })).unwrap();
            let document = crate::automation::script::parse_script(&source, "test.json").unwrap();
            let script = crate::automation::script::validate_script(document, "test.json").unwrap();
            let plan = crate::automation::native_checkpoint::NativeCheckpointPlan::derive(
                &source, &script,
            )
            .unwrap();
            assert_eq!(
                successful_label(None, 0, legacy.into()).as_bytes(),
                legacy.as_bytes()
            );
            let native = successful_label(Some(&plan), 0, legacy.into());
            let proof: serde_json::Value =
                serde_json::from_str(native.strip_prefix("native_parity:").unwrap()).unwrap();
            assert_eq!(
                proof["request"],
                serde_json::to_value(&script.steps()[0]).unwrap()
            );
            assert_eq!(proof["seed"], 42);
            assert_eq!(proof["fixture"], "parity-test");
            assert_eq!(proof["checkpoint"], plan.checkpoints[0].id);
        }
    }
}
