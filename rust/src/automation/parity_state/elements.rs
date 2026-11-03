//! Scalar element observation, shared by the live journal and checksum.
use super::{value, Fields};
use crate::automation::parity::Journal;
use crate::battle::velocity::VelocityDesc;
use crate::engine::battle::access::observe::ElementObservation;

pub(super) fn element(
    fields: &mut Fields,
    prefix: &str,
    e: &ElementObservation,
    journal: &Journal,
    hooks: &[(usize, &'static str)],
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
    value(fields, prefix, "target", journal.identity(e.target));
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
    use crate::engine::battle::ElementTable;

    #[test]
    fn links_and_targets_keep_their_element_identity_domain() {
        let directory = super::super::test_directory();
        let mut journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create element test journal: {error}"));
        let mut elements = ElementTable::with_capacity(2);
        let first = elements.alloc().unwrap();
        let second = elements.alloc().unwrap();
        elements.put(first);
        elements.put(second);
        let first_address = elements.raw(first) as usize;
        let second_address = elements.raw(second) as usize;
        journal.allocate(first_address);
        journal.allocate(second_address);
        journal.register_parent(second_address, "ship:0:3:5".into());
        elements.fixture_record_mut(first).target = Some(second);
        elements.fixture_record_mut(first).velocity.error.width = -23;
        elements
            .fixture_record_mut(first)
            .IntersectControl
            .last_time_val = 19;
        let mut view = crate::engine::battle::access::observe::element(&elements, first, None);
        let mut fields = Fields::new();
        element(&mut fields, "first", &view, &journal, &[]);
        assert_eq!(fields["first.identity"], "0:1");
        assert_eq!(fields["first.succ"], "1:1");
        assert_eq!(fields["first.target"], "1:1");
        assert_eq!(fields["first.velocity.error.width"], "-23");
        assert_eq!(fields["first.intersect.time"], "19");
        let second_view = crate::engine::battle::access::observe::element(&elements, second, None);
        element(&mut fields, "second", &second_view, &journal, &[]);
        assert_eq!(fields["second.pred"], "0:1");
        view.target = 0;
        element(&mut fields, "first", &view, &journal, &[]);
        assert_eq!(fields["first.target"], journal.identity(0));
        assert!(!fields.values().any(|value| value.contains("0x")));
    }
}
