use super::*;

impl AutomationObserver {
    pub(super) fn service_device(
        &self,
        world: &crate::engine::observe::CampaignView,
        inner: &mut CoordInner,
        step: crate::automation::story_actions::UseDeviceStep,
        event: SchedulerEvent,
    ) -> SchedulerEvent {
        use crate::automation::story_actions::{DeviceProgress, DeviceTracker};
        if inner.sched_state.phase == ActionPhase::WaitingForInput {
            inner.device = DeviceTracker::default();
        }
        let mut observation = self.ui.devices();
        observation.in_orbit =
            input_ffi::departure_snapshot(world, self.ui.solar_snapshot()).in_orbit != 0;
        observation.pc_menu = self._configuration.preferences().menu == 2;
        let progress = inner.device.observe(step, observation);
        let input = match &progress {
            DeviceProgress::Input(key) => *key,
            _ => None,
        };
        if !self.set_navigation_controls(Default::default()) {
            self.reject_input(inner, "use_device", -1, -1);
            return event;
        }
        for key in crate::automation::script::MenuKey::ALL {
            if !self.inject_menu_key(i32::from(key.index()), i32::from(input == Some(key))) {
                self.reject_input(inner, "use_device", -1, -1);
                return event;
            }
        }
        match progress {
            DeviceProgress::Input(_) => event,
            DeviceProgress::Complete(label) => {
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
                SchedulerEvent::NavigationReached
            }
            DeviceProgress::Failed(label) => {
                self.write_trace_labeled(inner, RecordKind::SemanticAssertion, label);
                self.set_terminal(inner, TerminalClass::SemanticMismatch);
                event
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::story_actions::{Device, DeviceObservation, UseDeviceStep};

    #[test]
    fn copied_device_events_complete_only_after_a_new_verified_activation() {
        let source = br#"{"version":1,"name":"device-service","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":100,"max_presentations":100,"max_wallclock_seconds":60}}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let output = tempfile::tempdir().unwrap();
        let game = crate::engine::Game::new();
        let observer = AutomationObserver::init_with_native_checkpoints(
            script,
            output.path().into(),
            None,
            game.world.activity_control(),
            Default::default(),
            Default::default(),
        );
        let world = crate::engine::observe::CampaignView::capture(&game.world, &game.kernel);
        let mut observation = DeviceObservation {
            list: Some(vec![Device::PortalSpawner.id()]),
            activation: Some((1, Device::PortalSpawner.id(), "old".into(), true)),
            ..Default::default()
        };
        observer.ui.observe_devices(&observation);
        let step = UseDeviceStep {
            device: Device::PortalSpawner,
            max_ticks: 20,
        };
        let event = SchedulerEvent::AdmittedInput;
        let mut inner = observer.inner.lock();
        assert_eq!(
            observer.service_device(&world, &mut inner, step, event),
            event
        );
        inner.sched_state.phase = ActionPhase::WaitingCondition { remaining: 20 };
        assert_eq!(
            observer.service_device(&world, &mut inner, step, event),
            event
        );
        observation.activation = Some((2, Device::PortalSpawner.id(), "new".into(), true));
        observer.ui.observe_devices(&observation);
        assert_eq!(
            observer.service_device(&world, &mut inner, step, event),
            SchedulerEvent::NavigationReached
        );
        assert_eq!(inner.terminal_class, None);

        inner.sched_state.phase = ActionPhase::WaitingForInput;
        assert_eq!(
            observer.service_device(&world, &mut inner, step, event),
            event
        );
        assert_eq!(inner.terminal_class, None);
        let expired = UseDeviceStep {
            max_ticks: 1,
            ..step
        };
        assert_eq!(
            observer.service_device(&world, &mut inner, expired, event),
            event
        );
        assert_eq!(inner.terminal_class, Some(TerminalClass::SemanticMismatch));
    }
}
