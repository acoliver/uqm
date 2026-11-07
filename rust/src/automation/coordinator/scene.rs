//! Scene assertion handling on the automation owner.

use super::*;

impl AutomationObserver {
    pub(super) fn verify_scene_assertion(&self, inner: &mut CoordInner) -> bool {
        if let Some(Action::AssertScene(assertion)) = self.actions.get(inner.sched_state.step_index)
        {
            match self.scene.lock().verify(assertion.scene) {
                Ok(plan) => self.write_trace_labeled(
                    inner,
                    RecordKind::SemanticAssertion,
                    format!(
                        "scene_verified:{}:encounter={}:dialogue={}",
                        plan.scene.name(),
                        plan.expected_encounter_conversation,
                        plan.expected_dialogue_conversation
                    ),
                ),
                Err(error) => {
                    self.write_trace_labeled(
                        inner,
                        RecordKind::SemanticAssertion,
                        error.to_string(),
                    );
                    self.set_terminal(inner, TerminalClass::SemanticMismatch);
                    return true;
                }
            }
        }

        false
    }
}
