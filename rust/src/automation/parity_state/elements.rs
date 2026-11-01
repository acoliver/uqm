//! Scalar element observation, shared by the live journal and checksum.
use super::{value, Fields};
use crate::automation::parity::Journal;
use crate::battle::context_access::observe::ElementObservation;
use crate::battle::velocity::VelocityDesc;

pub(super) fn element(
    fields: &mut Fields,
    prefix: &str,
    e: &ElementObservation,
    journal: &Journal,
    hooks: &[(usize, &'static str)],
    owners: &std::collections::BTreeSet<usize>,
) {
    for (name, address) in [("identity", e.address), ("pred", e.pred), ("succ", e.succ)] {
        value(fields, prefix, name, journal.identity(address));
    }
    value(fields, prefix, "parent", journal.parent(e.parent));
    for (name, number) in [
        ("side", i64::from(e.player)),
        ("flags", i64::from(e.flags)),
        ("life_span", i64::from(e.life)),
        ("crew_or_hp", i64::from(e.hits)),
        ("mass", i64::from(e.mass)),
        ("turn_wait", i64::from(e.turn)),
        ("thrust_or_blast", i64::from(e.thrust)),
        ("color_cycle", i64::from(e.color_cycle)),
        ("primitive", i64::from(e.primitive)),
    ] {
        value(fields, prefix, name, number);
    }
    target(fields, prefix, e, journal, owners);
    velocity(fields, prefix, &e.velocity);
    for (name, address) in [
        ("preprocess", e.pre.map_or(0, |f| f.address())),
        ("postprocess", e.post.map_or(0, |f| f.address())),
        ("collision", e.collide.map_or(0, |f| f.address())),
        ("death", e.death.map_or(0, |f| f.address())),
    ] {
        value(
            fields,
            prefix,
            name,
            crate::ships::hook_registry::name_for_species(
                hooks,
                address,
                journal.species(e.parent),
            ),
        );
    }
    positions(fields, prefix, e);
}

fn target(
    fields: &mut Fields,
    prefix: &str,
    element: &ElementObservation,
    journal: &Journal,
    owners: &std::collections::BTreeSet<usize>,
) {
    let address = element.target;
    let identity = if address != 0 && (owners.contains(&address) || address == element.parent) {
        journal.parent(address)
    } else {
        journal.identity(address)
    };
    value(fields, prefix, "target", identity);
}

fn positions(fields: &mut Fields, prefix: &str, e: &ElementObservation) {
    for (name, point) in [
        ("current", e.current.location),
        ("next", e.next.location),
        ("intersect.origin", e.intersect.IntersectStamp.origin),
        ("intersect.end", e.intersect.EndPoint),
    ] {
        value(fields, prefix, &format!("{name}.x"), point.x);
        value(fields, prefix, &format!("{name}.y"), point.y);
    }
    value(fields, prefix, "intersect.time", e.intersect.last_time_val);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::element::Element;
    fn element(
        fields: &mut Fields,
        prefix: &str,
        e: &Element,
        journal: &Journal,
        hooks: &[(usize, &'static str)],
        owners: &BTreeSet<usize>,
    ) {
        // SAFETY: the test owns the source and matching record layout.
        let view = unsafe { ElementObservation::decode(&*std::ptr::from_ref(e).cast()) };
        super::element(fields, prefix, &view, journal, hooks, owners);
    }
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
