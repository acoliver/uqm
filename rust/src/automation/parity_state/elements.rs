//! Scalar element observation, shared by the live journal and checksum.
use super::{value, Fields};
use crate::automation::parity::Journal;
use crate::battle::{element::Element, velocity::VelocityDesc};

pub(super) fn element(
    fields: &mut Fields,
    prefix: &str,
    e: &Element,
    journal: &Journal,
    hooks: &[(usize, &'static str)],
    owners: &std::collections::BTreeSet<usize>,
) {
    for (name, address) in [
        ("identity", std::ptr::from_ref(e) as usize),
        ("pred", e.pred as usize),
        ("succ", e.succ as usize),
    ] {
        value(fields, prefix, name, journal.identity(address));
    }
    value(
        fields,
        prefix,
        "parent",
        journal.parent(e.p_parent as usize),
    );
    for (name, number) in [
        ("side", i64::from(e.player_nr)),
        ("flags", i64::from(e.state_flags.bits())),
        ("life_span", i64::from(e.life_span)),
        ("crew_or_hp", i64::from(e.crew_or_hp)),
        ("mass", i64::from(e.mass_points)),
        ("turn_wait", i64::from(e.turn_wait)),
        ("thrust_or_blast", i64::from(e.thrust_or_blast)),
        ("color_cycle", i64::from(e.color_cycle_index)),
        ("primitive", i64::from(e.prim_index)),
    ] {
        value(fields, prefix, name, number);
    }
    target(fields, prefix, e, journal, owners);
    velocity(fields, prefix, &e.velocity);
    for (name, address) in [
        ("preprocess", e.preprocess_func.map_or(0, |f| f.address())),
        ("postprocess", e.postprocess_func.map_or(0, |f| f.address())),
        ("collision", e.collision_func.map_or(0, |f| f.address())),
        ("death", e.death_func.map_or(0, |f| f.address())),
    ] {
        value(
            fields,
            prefix,
            name,
            crate::ships::hook_registry::name_for_species(
                hooks,
                address,
                journal.species(e.p_parent as usize),
            ),
        );
    }
    positions(fields, prefix, e);
}

fn target(
    fields: &mut Fields,
    prefix: &str,
    element: &Element,
    journal: &Journal,
    owners: &std::collections::BTreeSet<usize>,
) {
    let address = element.h_target as usize;
    let identity =
        if address != 0 && (owners.contains(&address) || address == element.p_parent as usize) {
            journal.parent(address)
        } else {
            journal.identity(address)
        };
    value(fields, prefix, "target", identity);
}

fn positions(fields: &mut Fields, prefix: &str, e: &Element) {
    for (name, point) in [
        ("current", e.current.location),
        ("next", e.next.location),
        (
            "intersect.origin",
            e.intersect_control.intersect_stamp.origin,
        ),
        ("intersect.end", e.intersect_control.end_point),
    ] {
        value(fields, prefix, &format!("{name}.x"), point.x);
        value(fields, prefix, &format!("{name}.y"), point.y);
    }
    value(
        fields,
        prefix,
        "intersect.time",
        e.intersect_control.last_time_val,
    );
}

pub(super) fn velocity(fields: &mut Fields, prefix: &str, v: &VelocityDesc) {
    value(fields, prefix, "velocity.angle", v.travel_angle);
    for (name, extent) in [
        ("vector", v.vector),
        ("fract", v.fract),
        ("error", v.error),
        ("incr", v.incr),
    ] {
        value(
            fields,
            prefix,
            &format!("velocity.{name}.width"),
            extent.width,
        );
        value(
            fields,
            prefix,
            &format!("velocity.{name}.height"),
            extent.height,
        );
    }
}

impl crate::savegame::runtime::queues::QueueElement for Element {
    fn pred(&self) -> *mut std::ffi::c_void {
        self.pred.cast()
    }
    fn succ(&self) -> *mut std::ffi::c_void {
        self.succ.cast()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn links_and_overloaded_targets_use_their_own_identity_domains() {
        let directory = super::super::test_directory();
        let mut journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create element test journal: {error}"));
        let mut first = Element::new();
        let mut second = Element::new();
        let first_address = std::ptr::from_ref(&first) as usize;
        let second_address = std::ptr::from_ref(&second) as usize;
        journal.allocate(first_address);
        journal.allocate(second_address);
        journal.register_parent(71, "ship:0:3:5".into());
        first.succ = &mut second;
        second.pred = &mut first;
        first.h_target = 71usize as _;
        first.p_parent = 71usize as _;
        first.velocity.error.width = -23;
        first.intersect_control.last_time_val = 19;
        let owners = BTreeSet::from([71]);
        let mut fields = Fields::new();
        element(&mut fields, "first", &first, &journal, &[], &owners);
        assert_eq!(fields["first.identity"], "0:1");
        assert_eq!(fields["first.succ"], "1:1");
        assert_eq!(fields["first.target"], "ship:0:3:5");
        assert_eq!(fields["first.velocity.error.width"], "-23");
        assert_eq!(fields["first.intersect.time"], "19");
        first.h_target = &mut second;
        element(&mut fields, "first", &first, &journal, &[], &owners);
        assert_eq!(fields["first.target"], "1:1");
        assert!(!fields.values().any(|value| value.contains("0x")));
    }
}
