//! Boundary observers share the coordinator's lifetime, not another global slot.

use super::*;

impl AutomationObserver {
    pub(super) fn trace_initialization_failed<T, E: std::fmt::Display>(
        result: &Result<T, E>,
        name: &str,
    ) -> bool {
        if let Err(error) = result {
            eprintln!("automation: cannot create {name} trace journal: {error}");
            true
        } else {
            false
        }
    }

    pub(super) fn watchdog_limits(budgets: &super::super::script::Budgets) -> WatchdogLimits {
        #[cfg(feature = "debug-process")]
        let native_ticks = super::super::native_window::active_native_window_config()
            .and_then(|config| config.runtime_contract.expected_input_ticks);
        #[cfg(not(feature = "debug-process"))]
        let native_ticks = None;
        let max_wallclock = native_ticks.map_or_else(
            || Duration::from_secs(budgets.max_wallclock_seconds),
            |ticks| {
                super::super::native_timing::wall_budget(ticks)
                    .unwrap_or_else(|error| panic!("invalid native timing contract: {error}"))
            },
        );
        WatchdogLimits {
            max_input_ticks: budgets.max_input_ticks,
            max_presentations: budgets.max_presentations,
            max_wallclock,
            max_startup: Duration::from_secs(budgets.startup_seconds()),
            max_idle: Duration::from_secs(budgets.idle_seconds()),
        }
    }

    pub(super) fn write_action_phase(&self, inner: &mut CoordInner) {
        if !crate::time::source::is_replay() {
            return;
        }
        let index = inner.sched_state.step_index;
        let action = self.actions.get(index).map_or_else(
            || "end".to_string(),
            |action| {
                serde_json::to_value(action)
                    .unwrap_or_else(|error| panic!("serialize script action: {error}"))["action"]
                    .as_str()
                    .unwrap_or_else(|| panic!("serialized Action has no action name"))
                    .to_string()
            },
        );
        self.write_trace_labeled(
            inner,
            RecordKind::SemanticAssertion,
            format!(
                "action_phase:step={index}:action={action}:phase={:?}",
                inner.sched_state.phase
            ),
        );
    }
    pub(super) fn create_parity(
        output: &std::path::Path,
    ) -> std::io::Result<Option<Mutex<super::super::parity::Journal>>> {
        if !crate::time::source::is_replay() || std::env::var_os("UQM_PARITY").is_none() {
            return Ok(None);
        }
        let journal = super::super::parity::Journal::create(output)?;
        crate::collections::observation::enable();
        Ok(Some(Mutex::new(journal)))
    }

    pub(super) fn parity_present(
        &self,
        frame: &crate::graphics::presented_frame::PresentedFrame,
    ) -> std::io::Result<()> {
        if let Some(journal) = &self.parity {
            journal.lock().present(frame)?;
        }
        Ok(())
    }

    fn parity_failure(&self, boundary: &str, error: std::io::Error) {
        eprintln!("automation: parity {boundary} failed: {error}");
        // A fixture checkpoint can run while the input boundary owns CoordInner.
        // Publish through the existing first-writer terminal mirror instead.
        self.runtime
            .mirror
            .terminal
            .try_set(TerminalClass::TraceFailure);
    }

    pub fn parity_checkpoint(&self, boundary: &str) {
        let coord = self;
        if coord.runtime.mirror.is_terminal() {
            return;
        }
        let Some(journal) = &coord.parity else {
            return;
        };
        let result = super::super::parity_state::checkpoint(&mut journal.lock(), boundary);
        if let Err(error) = result {
            coord.parity_failure(boundary, error);
        }
    }

    pub fn parity_input(&self, activity: u16) {
        let coord = self;
        let Some(journal) = &coord.parity else {
            return;
        };
        let count = coord.inner.lock().input_seen;
        let keys = super::super::input_ffi::current_inputs();
        let result = journal.lock().input(
            count,
            activity,
            &format!("menu={:?};players={:?}", keys.menu, keys.key),
        );
        if let Err(error) = result {
            coord.parity_failure("input", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reentrant_checkpoint_failure_publishes_before_another_boundary_can_succeed() {
        const CHILD: &str = "UQM_REENTRANT_PARITY_FAILURE";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "automation::coordinator::parity::tests::reentrant_checkpoint_failure_publishes_before_another_boundary_can_succeed"])
                .env(CHILD, "1").status().unwrap();
            assert!(status.success());
            return;
        }
        let source = serde_json::to_vec(&serde_json::json!({
            "version":1,"name":"failure","steps":[{"action":"finish"}],
            "budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":10}
        }))
        .unwrap();
        let script = crate::automation::validate_script(
            crate::automation::parse_script(&source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let _app = {
            let game = crate::engine::Game::new();
            let observer = Some(AutomationObserver::new(
                script,
                directory.path().into(),
                game.world.activity_control(),
            ));
            crate::app::Application::new(game, observer)
        };
        let coord = _app.observer().cloned().unwrap();
        let mut inner = coord.inner.lock();
        coord.parity_failure(
            "fixture",
            std::io::Error::new(std::io::ErrorKind::WriteZero, "injected output failure"),
        );
        assert_eq!(
            coord.runtime.mirror.terminal.load(),
            Some(TerminalClass::TraceFailure)
        );
        coord.set_terminal(&mut inner, TerminalClass::Success);
        assert_eq!(inner.terminal_class, Some(TerminalClass::TraceFailure));
        assert!(coord.admit_boundary(&mut inner, CallbackKind::Input));
        drop(inner);
        assert_ne!(_app.observer().unwrap().finalize(0).unwrap(), 0);
    }
}
