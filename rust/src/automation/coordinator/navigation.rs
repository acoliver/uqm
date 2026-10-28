//! Orbital navigation input and arrival observation.

use super::*;

impl Coordinator {
    pub(super) fn service_orbit_navigation(
        &self,
        inner: &mut CoordInner,
        navigation: &crate::automation::script::NavigateToOrbitStep,
        mut scheduler_event: SchedulerEvent,
    ) -> SchedulerEvent {
        let snapshot =
            crate::automation::input_ffi::navigation_snapshot(i32::from(navigation.planet), None);
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
            if !Self::set_navigation_controls(
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
        } else if !Self::set_navigation_controls(control) {
            self.reject_input(inner, "navigation", -1, -1);
            return scheduler_event;
        }
        scheduler_event
    }
}
