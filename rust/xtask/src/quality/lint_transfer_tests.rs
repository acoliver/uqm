use super::baseline::Baseline;
use super::metrics::{Finding, Measurement};
use std::collections::BTreeMap;

#[test]
fn landed_moves_keep_test_only_lint_credits_until_every_host_migrates() {
    let old = "uqm::graphics";
    let new = "uqm-engine::graphics";
    let old_key = "clippy::undocumented_unsafe_blocks|uqm::@path::src::graphics::tests.rs";
    let new_key = "clippy::undocumented_unsafe_blocks|uqm-engine::@path::src::graphics::tests.rs";
    let measurement = Measurement {
        findings: vec![Finding {
            counter: "loc".into(),
            metric: "loc".into(),
            identity: format!("{new}::@file"),
            file: "crates/engine/src/graphics/mod.rs".into(),
            package: "uqm-engine".into(),
            subsystem: "graphics".into(),
            value: 1,
        }],
        ..Default::default()
    };
    let mut baseline = Baseline {
        moved: [(old.into(), new.into())].into(),
        lint: [
            ("linux".into(), [(old_key.into(), 2)].into()),
            ("macos".into(), [(new_key.into(), 2)].into()),
        ]
        .into(),
        ..Default::default()
    };
    baseline.retire_moves(&measurement);
    assert_eq!(baseline.moved.get(old).map(String::as_str), Some(new));
    baseline
        .tighten(
            &measurement,
            Some(&Baseline {
                entries: [(format!("loc|loc|{new}::@file"), 1)].into(),
                ..Default::default()
            }),
        )
        .unwrap();
    assert_eq!(baseline.moved.get(old).map(String::as_str), Some(new));
    baseline
        .lint
        .insert("linux".into(), [(new_key.into(), 2)].into());
    baseline
        .tighten(
            &measurement,
            Some(&Baseline {
                entries: [(format!("loc|loc|{new}::@file"), 1)].into(),
                ..Default::default()
            }),
        )
        .unwrap();
    assert_eq!(baseline.moved, BTreeMap::new());
    baseline.moved.insert(old.into(), new.into());
    baseline.retire_moves(&measurement);
    assert!(baseline.moved.is_empty());
}

#[test]
fn registering_another_move_preserves_pending_lint_credits() {
    let finding = |identity: &str, subsystem: &str| Finding {
        counter: "ffi".into(),
        metric: "static_mut".into(),
        identity: identity.into(),
        file: "src/lib.rs".into(),
        package: "uqm".into(),
        subsystem: subsystem.into(),
        value: 1,
    };
    let engine = finding("uqm-engine::graphics::existing", "graphics");
    let base = Measurement {
        findings: vec![engine.clone(), finding("uqm::battle::old", "battle")],
        ..Default::default()
    };
    let current = Measurement {
        findings: vec![engine, finding("uqm::app::new", "app")],
        ..Default::default()
    };
    let mut baseline = Baseline {
        entries: base.baseline(),
        moved: [("uqm::graphics".into(), "uqm-engine::graphics".into())].into(),
        lint: [(
            "linux".into(),
            [(
                "clippy::unwrap_used|uqm::@path::src::graphics::tests.rs".into(),
                2,
            )]
            .into(),
        )]
        .into(),
        ..Default::default()
    };
    baseline
        .add_move("uqm::battle::old", "uqm::app::new", &current, &base)
        .unwrap();
    assert_eq!(baseline.moved.len(), 2);
    assert_eq!(baseline.entries["ffi|static_mut|uqm::app"], 1);
    assert!(!baseline.entries.contains_key("ffi|static_mut|uqm::battle"));
}
