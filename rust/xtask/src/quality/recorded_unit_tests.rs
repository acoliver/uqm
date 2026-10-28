use super::super::{baseline::Baseline, budgets, metrics::Measurement};
use super::*;

fn entries(rows: &[(&str, u64)]) -> Entries {
    rows.iter().map(|(k, v)| ((*k).into(), *v)).collect()
}

#[test]
fn recording_changes_only_grown_counters_and_preserves_other_sections() {
    let mut expected = Baseline {
        entries: entries(&[
            ("unsafe|unsafe_blocks|uqm::battle", 3),
            ("unsafe|unsafe_fn|uqm::app", 2),
        ]),
        moved: [("uqm::old".into(), "uqm::new".into())].into(),
        lint: [("linux".into(), entries(&[("unwrap|uqm::app", 2)]))].into(),
        coverage_debt: [("src/shim.rs".into(), "U7-02".into())].into(),
        layers: true,
        ..Default::default()
    };
    budgets::set(&mut expected.budget, "bridge_calls", 48, false).unwrap();
    let root = super::super::tool::fixture_directory();
    let path = root.path().join("baseline.toml");
    expected.write(&path).unwrap();
    let mut baseline = Baseline::read(&path).unwrap();
    let measured = entries(&[
        ("unsafe|unsafe_blocks|uqm::battle", 4),
        ("unsafe|unsafe_fn|uqm::app", 1),
    ]);
    growth(&mut baseline, &measured, false);
    expected
        .entries
        .insert("unsafe|unsafe_blocks|uqm::battle".into(), 4);
    assert_eq!(baseline, expected);
    growth(&mut baseline, &measured, false);
    assert_eq!(baseline, expected);
}

#[test]
fn recorded_growth_must_be_exact_but_inherited_headroom_is_not_new_growth() {
    let before = entries(&[("ffi|static_mut|uqm::battle", 2)]);
    let measured = entries(&[("ffi|static_mut|uqm::battle", 3)]);
    for recorded in [
        Entries::new(),
        before.clone(),
        entries(&[("ffi|static_mut|uqm::battle", 4)]),
    ] {
        assert!(check(&measured, &recorded, &before).is_err());
    }
    check(&measured, &measured, &before).unwrap();
    check(&Entries::new(), &before, &before).unwrap();
    check_measured(&Entries::new(), &before, &Entries::new()).unwrap();
    assert!(check(&Entries::new(), &before, &Entries::new()).is_err());
    assert!(check_measured(&measured, &before, &before).is_err());
}

#[test]
fn engine_recording_is_limited_to_the_reviewed_edges_and_counts() {
    let mut baseline = Baseline::default();
    let measured = entries(&[
        ("layers|crate_os|uqm-engine::sdl2", 1),
        ("layers|crate_os|uqm-engine::libc", 2),
        ("layers|crate_os|uqm-engine::unknown", 1),
        ("layers|crate_os|uqm::sdl2", 1),
        ("unsafe|unsafe_fn|uqm::app", 1),
    ]);
    growth(&mut baseline, &measured, true);
    assert_eq!(
        baseline.entries,
        entries(&[("layers|crate_os|uqm-engine::sdl2", 1)])
    );
    assert!(check(&measured, &baseline.entries, &Entries::new()).is_err());
    let allowed = baseline.entries.clone();
    assert!(check(&allowed, &Entries::new(), &Entries::new()).is_err());
    check(&allowed, &allowed, &Entries::new()).unwrap();
}

#[test]
fn budgets_remain_finite_and_do_not_need_duplicate_ordinary_entries() {
    let mut baseline = Baseline::default();
    budgets::set(&mut baseline.budget, "bridge_calls", 1, false).unwrap();
    let finding = super::super::metrics::Finding {
        counter: "ffi".into(),
        metric: "bridge_calls".into(),
        identity: "uqm::app".into(),
        file: "src/app.rs".into(),
        package: "uqm".into(),
        subsystem: "app".into(),
        value: 1,
    };
    let mut measurement = Measurement {
        findings: vec![finding],
        ..Default::default()
    };
    budgets::check(&measurement, &baseline.budget).unwrap();
    check_base(
        &measurement.baseline(),
        &baseline,
        &Entries::new(),
        Some(&Baseline::default()),
    )
    .unwrap();
    measurement.findings[0].value = 2;
    assert!(budgets::check(&measurement, &baseline.budget).is_err());
}
