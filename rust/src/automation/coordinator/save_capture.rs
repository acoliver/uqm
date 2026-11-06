//! One-shot V3b load/capture lifecycle at production game-loop boundaries.

use super::*;

impl AutomationObserver {
    /// Admit V3b loading at the existing initialized-game hook.
    #[cfg(not(test))]
    pub(crate) fn activate_save_capture(&self) {
        if let Some(request) = self.inner.lock().save_capture.as_mut() {
            request.admit_load();
        }
    }

    pub(super) fn complete_admitted_load(&self, cx: &mut crate::engine::battle::Cx<'_>) {
        let mut inner = self.inner.lock();
        let Some(request) = inner.save_capture.as_mut() else {
            return;
        };
        if !request.load_admitted {
            return;
        }
        #[cfg(not(test))]
        if let Err(error) = request.load(cx) {
            self.write_trace_labeled(
                &mut inner,
                RecordKind::SemanticAssertion,
                format!("save_capture_load_failed:{error:#}"),
            );
            cx.with_legacy(|| self.set_terminal(&mut inner, TerminalClass::SemanticMismatch));
        }
        #[cfg(test)]
        {
            let _ = cx;
            panic!("runtime save load cannot run in unit tests: {request:?}");
        }
    }

    pub(super) fn save_input(
        &self,
        inner: &mut CoordInner,
        button_wait: bool,
        capture: &mut dyn FnMut() -> campaign_state::CampaignView,
    ) -> bool {
        let Some(request) = inner.save_capture.as_mut() else {
            return false;
        };
        if !request.needs_capture(button_wait) {
            return false;
        }
        request.reserve_capture(capture().activity, button_wait, inner.input_seen)
    }

    /// Complete the admitted save after the input observer returns its borrowed owner.
    #[cfg(not(test))]
    pub(crate) fn complete_save_capture(&self, cx: &mut crate::engine::battle::Cx<'_>) {
        let mut inner = self.inner.lock();
        let Some(request) = inner.save_capture.as_mut() else {
            return;
        };
        if request.pending_admission.is_none() {
            return;
        }
        let (class, label) = match request.capture(cx) {
            Ok(()) => (TerminalClass::Success, "save_capture_complete".to_string()),
            Err(error) => (
                TerminalClass::TraceFailure,
                format!("save_capture_failed:{error:#}"),
            ),
        };
        self.write_trace_labeled(&mut inner, RecordKind::SemanticAssertion, label);
        cx.with_legacy(|| self.set_terminal(&mut inner, class));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_admission_returns_the_owner_without_capturing_or_publishing_terminal_state() {
        let source = br#"{"version":1,"name":"save-admission","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":60}}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let output = tempfile::tempdir().unwrap();
        let observer = AutomationObserver::new(
            script,
            output.path().into(),
            crate::engine::World::new().activity_control(),
            crate::engine::input::InputControls::default(),
            Default::default(),
        );
        let mut inner = observer.inner.lock();
        inner.input_seen = 9;
        inner.save_capture = Some(crate::automation::save_capture::Request {
            source: "source".into(),
            output: "output".into(),
            loaded_activity: Some(0x0804),
            load_admitted: false,
            loaded_tick: 17,
            captured: false,
            pending_admission: None,
        });
        assert!(!observer.save_input(&mut inner, true, &mut || panic!("button wait captured")));
        let mut captures = 0;
        assert!(observer.save_input(&mut inner, false, &mut || {
            captures += 1;
            let mut view = campaign_state::empty_view();
            view.activity = 0x0804;
            view
        }));
        assert_eq!(captures, 1);
        let request = inner.save_capture.as_ref().unwrap();
        assert_eq!(request.pending_admission, Some(9));
        assert!(!request.captured);
        assert_eq!(request.loaded_tick, 17);
        assert!(inner.terminal_class.is_none());
        assert!(!observer.runtime.mirror.is_terminal());
        assert!(!observer.save_input(&mut inner, false, &mut || panic!("second reservation")));
    }
}
