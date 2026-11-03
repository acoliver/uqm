//! Orbital navigation and hyperspace departure observation.

use super::*;

#[derive(Default)]
pub(super) struct Departure {
    pub(super) flight_observed: bool,
    /// Scene initialization presents UI over the old solar drawing. Native
    /// checkpoints must wait until the hyperspace battle loop has drawn a frame.
    pub(super) native_frame_pending: bool,
}

impl AutomationObserver {
    pub(super) fn transit_budget(
        world: &campaign_state::CampaignView,
        action: Option<&Action>,
    ) -> Option<u64> {
        use crate::automation::script::PatrolPolicy;
        match action {
            Some(Action::NavigateToHyperspace(step)) if step.encounter_response.is_none() => {
                Some(step.max_ticks)
            }
            Some(Action::NavigateToMoon(step))
                if world.story_value(StoryKey::GLOBAL_FLAGS_AND_DATA) == Some(0) =>
            {
                Some(step.max_ticks)
            }
            Some(Action::NavigateToPlanet(step)) if step.patrol_policy != PatrolPolicy::Stop => {
                Some(step.max_ticks)
            }
            Some(Action::NavigateToOrbit(step))
                if step.patrol_policy != PatrolPolicy::Stop
                    && world.story_value(StoryKey::GLOBAL_FLAGS_AND_DATA) == Some(0) =>
            {
                Some(step.max_ticks)
            }
            _ => None,
        }
    }

    pub(in crate::automation) fn native_battle_frame_completed(
        activity: crate::mainloop::types::ActivityValue,
    ) {
        if activity.kind() == crate::mainloop::types::ActivityKind::InHyperspace {
            if let Some(coord) = Self::get() {
                coord.inner.lock().departure.native_frame_pending = false;
            }
        }
    }

    pub(super) fn service_hyperspace_departure(
        &self,
        world: &campaign_state::CampaignView,
        inner: &mut CoordInner,
        mut scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        let snapshot = crate::automation::input_ffi::departure_snapshot(world);
        let solar_system_active = snapshot.active != 0;
        if solar_system_active && snapshot.in_ip_flight != 0 && !inner.departure.flight_observed {
            inner.departure.flight_observed = true;
            inner.departure.native_frame_pending = self.native_checkpoint_plan.is_some();
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                format!(
                    "hyperspace_departure_started:ship={},{}:facing={}",
                    snapshot.ship_x, snapshot.ship_y, snapshot.ship_facing
                ),
            );
        }
        let activity = crate::mainloop::ffi::get_current_activity().0;
        if crate::automation::navigation::hyperspace_reached(
            inner.departure.flight_observed,
            solar_system_active,
            activity,
        ) {
            if !self.set_navigation_controls(
                crate::automation::navigation::NavigationControl::default(),
            ) {
                self.reject_input(inner, "navigation", -1, -1);
                return scheduler_event;
            }
            inner.departure.flight_observed = false;
            scheduler_event = SchedulerEvent::NavigationReached;
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                crate::automation::navigation::HYPERSPACE_REACHED_LABEL.to_string(),
            );
            return scheduler_event;
        }
        let control = crate::automation::navigation::steer_out_of_system(
            crate::automation::navigation::NavigationObservation {
                active: solar_system_active,
                inner_planet: u8::try_from(snapshot.inner_planet).ok(),
                in_orbit: snapshot.in_orbit != 0,
                ship_x: snapshot.ship_x,
                ship_y: snapshot.ship_y,
                ship_facing: u8::try_from(snapshot.ship_facing).unwrap_or(0),
                target_x: snapshot.target_x,
                target_y: snapshot.target_y,
                velocity_x: snapshot.velocity_x,
                velocity_y: snapshot.velocity_y,
                view_center_x: snapshot.view_center_x,
                view_center_y: snapshot.view_center_y,
                target_guard_active: false,
            },
        );
        if !self.set_navigation_controls(control) {
            self.reject_input(inner, "navigation", -1, -1);
        }
        scheduler_event
    }
}

impl AutomationObserver {
    pub(super) fn service_orbit_navigation(
        &self,
        world: &campaign_state::CampaignView,
        inner: &mut CoordInner,
        navigation: &crate::automation::script::NavigateToOrbitStep,
        mut scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        let snapshot = crate::automation::input_ffi::navigation_snapshot(
            world,
            i32::from(navigation.planet),
            None,
        );
        if inner.sched_state.phase == ActionPhase::WaitingForInput {
            inner.orbit = Default::default();
        }
        let fresh_orbit = inner
            .orbit
            .observe(snapshot.in_ip_flight != 0, snapshot.in_orbit != 0);
        let contact = navigation.stop_at_encounter
            && crate::automation::ui_observation::encounter_menu_active();
        let reached = snapshot.active != 0
            && snapshot.in_orbit != 0
            && snapshot.inner_planet == i32::from(navigation.planet)
            && snapshot.orbital_moon < 0
            && (!navigation.stop_at_encounter || fresh_orbit);
        // Inside the planet's inner system the flagship is inertial, so
        // aiming the nose alone accumulates drift and circles the planet
        // without touching it. Drive the velocity toward the planet, which
        // the snapshot reports at the view centre.
        let control = crate::automation::navigation::steer_planet_orbit(
            crate::automation::navigation::NavigationObservation {
                active: snapshot.active != 0,
                inner_planet: u8::try_from(snapshot.inner_planet).ok(),
                in_orbit: snapshot.in_orbit != 0,
                ship_x: snapshot.ship_x,
                ship_y: snapshot.ship_y,
                ship_facing: u8::try_from(snapshot.ship_facing).unwrap_or(0),
                target_x: snapshot.target_x,
                target_y: snapshot.target_y,
                velocity_x: snapshot.velocity_x,
                velocity_y: snapshot.velocity_y,
                view_center_x: snapshot.view_center_x,
                view_center_y: snapshot.view_center_y,
                target_guard_active: snapshot.wait_intersect
                    == crate::automation::navigation::planet_wait_intersect(navigation.planet),
            },
            navigation.planet,
        );
        if reached || contact {
            if !self.set_navigation_controls(
                crate::automation::navigation::NavigationControl::default(),
            ) {
                self.reject_input(inner, "navigation", -1, -1);
                return scheduler_event;
            }
            inner.orbit.transition_pending = !contact;
            scheduler_event = SchedulerEvent::NavigationReached;
            self.write_trace_labeled(
                inner,
                RecordKind::SemanticAssertion,
                format!(
                    "{}:planet={}",
                    if contact {
                        "contact_reached"
                    } else {
                        "orbit_reached"
                    },
                    navigation.planet
                ),
            );
        } else if !self.set_navigation_controls(control) {
            self.reject_input(inner, "navigation", -1, -1);
            return scheduler_event;
        }
        scheduler_event
    }
}
