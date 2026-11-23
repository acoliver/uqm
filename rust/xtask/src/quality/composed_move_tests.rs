use super::baseline::Baseline;
use super::metrics::{Finding, Measurement};

#[test]
fn enclosing_move_composes_a_prior_declaration_rename() {
    let finding = |name: &str| Finding {
        counter: "ffi".into(),
        metric: "extern_c_fn".into(),
        identity: name.into(),
        file: "crates/engine/src/observe.rs".into(),
        package: "uqm-engine".into(),
        subsystem: "uqm-engine".into(),
        value: 1,
    };
    let measurement = |names: &[&str]| Measurement {
        findings: names.iter().map(|name| finding(name)).collect(),
        ..Default::default()
    };
    let base = measurement(&["uqm-engine::observe::Io", "uqm-engine::observe::other"]);
    let intermediate = measurement(&[
        "uqm-engine::observe::ObservationIo",
        "uqm-engine::observe::other",
    ]);
    let current = measurement(&[
        "uqm-engine::observe_port::ObservationIo",
        "uqm-engine::observe_port::other",
    ]);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move(
            "uqm-engine::observe::Io",
            "uqm-engine::observe::ObservationIo",
            &intermediate,
            &base,
        )
        .unwrap();
    baseline
        .add_move(
            "uqm-engine::observe",
            "uqm-engine::observe_port",
            &current,
            &base,
        )
        .unwrap();
    assert_eq!(baseline.entries, current.baseline());
    assert_eq!(
        baseline.moved["uqm-engine::observe::Io"],
        "uqm-engine::observe_port::ObservationIo"
    );
    super::transfers::validate(&baseline.entries, &current, &base, &baseline.moved).unwrap();
}

#[test]
fn chained_move_follows_a_previously_relocated_destination() {
    let finding = |identity: &str, subsystem: &str| Finding {
        counter: "ffi".into(),
        metric: "static_mut".into(),
        identity: identity.into(),
        file: "src/lib.rs".into(),
        package: "uqm".into(),
        subsystem: subsystem.into(),
        value: 1,
    };
    let measurement = |identity, subsystem| Measurement {
        findings: vec![finding(identity, subsystem)],
        ..Default::default()
    };
    let base = measurement("uqm::save::state", "save");
    let intermediate = measurement("uqm::engine::save::state", "engine");
    let current = measurement("uqm::extracted::save::state", "extracted");
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move("uqm::save", "uqm::engine::save", &intermediate, &base)
        .unwrap();
    baseline
        .add_move("uqm::engine", "uqm::extracted", &current, &base)
        .unwrap();
    assert_eq!(baseline.entries, current.baseline());
    assert_eq!(baseline.moved.len(), 1);
    assert_eq!(baseline.moved["uqm::save"], "uqm::extracted::save");
}

fn composition_state(namespace: &str, item: &str) -> Measurement {
    let identity = format!("{namespace}::{item}");
    let scope = format!("allows|clippy::missing_safety_doc|{namespace}::@file");
    let finding = Finding {
        counter: "ffi".into(),
        metric: "raw_records".into(),
        identity: identity.clone(),
        file: "src/observe.rs".into(),
        package: "uqm-engine".into(),
        subsystem: "uqm-engine".into(),
        value: 1,
    };
    Measurement {
        findings: vec![
            finding.clone(),
            Finding {
                counter: "unsafe".into(),
                metric: "unsafe_lines".into(),
                ..finding.clone()
            },
            Finding {
                counter: "allows".into(),
                metric: "clippy::missing_safety_doc".into(),
                identity: format!("{namespace}::@file"),
                value: 2,
                ..finding
            },
        ],
        allow_coverage: vec![super::metrics::AllowCoverage {
            item: identity,
            scope,
            file: "src/observe.rs".into(),
            lines: [1, 2].into(),
        }],
        ..Default::default()
    }
}

#[test]
fn composed_moves_conserve_budgets_allows_and_workspace_unsafe() {
    let old = "uqm-engine::observe";
    let new = "uqm-engine::observe_port";
    let base = composition_state(old, "Io");
    let intermediate = composition_state(old, "ObservationIo");
    let mut current = composition_state(new, "ObservationIo");
    let old_key = format!("ffi|raw_records|{old}::Io");
    let new_key = format!("ffi|raw_records|{new}::ObservationIo");
    let mut baseline = Baseline::default();
    super::budgets::set_scoped(
        &mut baseline.budget,
        "stage_b_raw",
        1,
        [(old_key, 1)].into(),
        false,
    )
    .unwrap();
    baseline.entries = super::budgets::scoped_entries(&base, &baseline.budget);
    baseline
        .add_move(
            &format!("{old}::Io"),
            &format!("{old}::ObservationIo"),
            &intermediate,
            &base,
        )
        .unwrap();
    baseline.add_move(old, new, &current, &base).unwrap();
    assert_eq!(baseline.budget["stage_b_raw"].limit, 1);
    assert_eq!(baseline.budget["stage_b_raw"].scopes, [(new_key, 1)].into());
    assert_eq!(
        baseline.entries,
        super::budgets::scoped_entries(&current, &baseline.budget)
    );
    current.findings[0].value = 2;
    assert!(super::budgets::check(&current, &baseline.budget).is_err());
    current.findings[0].value = 1;
    current.findings[1].value = 2;
    assert!(super::baseline::check_limits(
        &current.baseline(),
        &baseline.entries,
        &baseline.moved,
        true
    )
    .is_err());
}

#[test]
fn composed_moves_reject_missing_targets_and_retained_sources_atomically() {
    let old = "uqm-engine::observe";
    let base = composition_state(old, "Io");
    let intermediate = composition_state(old, "ObservationIo");
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move(
            &format!("{old}::Io"),
            &format!("{old}::ObservationIo"),
            &intermediate,
            &base,
        )
        .unwrap();
    let state = |baseline: &Baseline| {
        (
            baseline.entries.clone(),
            baseline.budget.clone(),
            baseline.moved.clone(),
        )
    };
    let previous = state(&baseline);
    let mut current = composition_state("uqm-engine::observe_port", "ObservationIo");
    current
        .findings
        .retain(|f| !f.identity.ends_with("ObservationIo"));
    assert!(baseline
        .add_move(old, "uqm-engine::observe_port", &current, &base)
        .unwrap_err()
        .contains("absent"));
    assert_eq!(state(&baseline), previous);
    let mut current = composition_state("uqm-engine::observe_port", "ObservationIo");
    current.findings.extend(intermediate.findings);
    assert!(baseline
        .add_move(old, "uqm-engine::observe_port", &current, &base)
        .unwrap_err()
        .contains("still exists"));
    assert_eq!(state(&baseline), previous);
}

#[test]
fn specific_move_credit_cannot_be_reused_at_the_old_target_name() {
    let moves = [
        ("uqm::old".into(), "uqm::new".into()),
        ("uqm::old::Io".into(), "uqm::new::ObservationIo".into()),
    ]
    .into();
    let before = [("loc|loc|uqm::old::Io".into(), 1000)].into();
    let current = [("loc|loc|uqm::new::Io".into(), 1000)].into();
    assert!(super::baseline::check_limits(&current, &before, &moves, true).is_err());
}

#[test]
fn a_child_rename_after_its_module_move_keeps_original_source_credit() {
    let old = "uqm-engine::observe";
    let new = "uqm-engine::observe_port";
    let base = composition_state(old, "Io");
    let intermediate = composition_state(new, "Io");
    let current = composition_state(new, "ObservationIo");
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline.add_move(old, new, &intermediate, &base).unwrap();
    baseline
        .add_move(
            &format!("{new}::Io"),
            &format!("{new}::ObservationIo"),
            &current,
            &base,
        )
        .unwrap();
    assert_eq!(baseline.entries, current.baseline());
    assert_eq!(
        baseline.moved[&format!("{old}::Io")],
        format!("{new}::ObservationIo")
    );
    assert!(baseline.add_move(new, new, &current, &base).is_err());
}

#[test]
fn final_child_destination_can_be_registered_before_its_enclosing_move() {
    let old = "uqm-engine::observe";
    let new = "uqm-engine::observe_port";
    let mut base = composition_state(old, "Io");
    let mut current = composition_state(new, "ObservationIo");
    for (state, namespace) in [(&mut base, old), (&mut current, new)] {
        state.findings.push(Finding {
            identity: format!("{namespace}::Marker"),
            ..state.findings[0].clone()
        });
    }
    let mut baseline = Baseline::default();
    super::budgets::set_scoped(
        &mut baseline.budget,
        "stage_b_raw",
        1,
        [(format!("ffi|raw_records|{old}::Io"), 1)].into(),
        false,
    )
    .unwrap();
    baseline.entries = super::budgets::scoped_entries(&base, &baseline.budget);
    baseline
        .add_move(
            &format!("{old}::Io"),
            &format!("{new}::ObservationIo"),
            &current,
            &base,
        )
        .unwrap();
    baseline.add_move(old, new, &current, &base).unwrap();
    assert_eq!(
        baseline.entries,
        super::budgets::scoped_entries(&current, &baseline.budget)
    );
    assert_eq!(baseline.budget["stage_b_raw"].limit, 1);
    assert_eq!(
        baseline.budget["stage_b_raw"].scopes,
        [(format!("ffi|raw_records|{new}::ObservationIo"), 1)].into()
    );
    assert_eq!(baseline.moved[old], new);
    assert_eq!(
        baseline.moved[&format!("{old}::Io")],
        format!("{new}::ObservationIo")
    );
    let mut duplicate = current.clone();
    duplicate.findings.push(Finding {
        identity: format!("{new}::Io"),
        ..duplicate.findings[0].clone()
    });
    assert!(super::baseline::check_limits(
        &duplicate.baseline(),
        &baseline.entries,
        &baseline.moved,
        true
    )
    .is_err());
}

fn extraction_measurement(items: &[(&str, &str, u64)]) -> Measurement {
    Measurement {
        findings: items
            .iter()
            .map(|&(identity, metric, value)| {
                let (package, path) = identity.split_once("::").unwrap();
                Finding {
                    counter: if metric == "loc" { "loc" } else { "ffi" }.into(),
                    metric: metric.into(),
                    identity: identity.into(),
                    file: "src/lib.rs".into(),
                    package: package.into(),
                    subsystem: if package == "uqm" {
                        path.split("::").next().unwrap()
                    } else {
                        package
                    }
                    .into(),
                    value,
                }
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn seeded_engine_container_accepts_distinct_core_declarations() {
    let old = "uqm::engine";
    let new = "uqm-engine";
    let mut base = extraction_measurement(&[
        ("uqm::engine::world::state", "static_mut", 1),
        ("uqm-engine::graphics::native", "static_mut", 1),
        ("uqm::engine::@file", "loc", 17),
        ("uqm-engine::@file", "loc", 23),
    ]);
    let mut current = extraction_measurement(&[
        ("uqm-engine::world::state", "static_mut", 1),
        ("uqm-engine::graphics::native", "static_mut", 1),
        ("uqm-engine::@file", "loc", 40),
    ]);
    let retained = composition_state("uqm-engine::graphics", "retained");
    base.findings.extend(retained.findings.clone());
    base.allow_coverage.extend(retained.allow_coverage.clone());
    current.findings.extend(retained.findings);
    current.allow_coverage.extend(retained.allow_coverage);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline.add_move(old, new, &current, &base).unwrap();
    assert_eq!(baseline.entries, current.baseline());
    assert_eq!(baseline.moved[old], new);
    let mut oversized = current.clone();
    oversized
        .findings
        .iter_mut()
        .find(|finding| finding.counter == "loc")
        .unwrap()
        .value = 801;
    assert!(super::baseline::check_limits(
        &oversized.baseline(),
        &baseline.entries,
        &baseline.moved,
        true
    )
    .is_err());

    let mut collision = base.clone();
    collision.findings.push(
        extraction_measurement(&[("uqm-engine::world::state", "static_mut", 1)])
            .findings
            .remove(0),
    );
    let mut baseline = Baseline {
        entries: collision.baseline(),
        ..Default::default()
    };
    assert!(baseline.add_move(old, new, &current, &collision).is_err());
    assert!(baseline.moved.is_empty());
}

#[test]
fn vacated_observation_namespace_accepts_independently_owned_campaign_data() {
    let old = "uqm-engine::observe";
    let port = "uqm-engine::observe_port";
    let core = "uqm::engine::observe";
    let base = extraction_measurement(&[
        ("uqm-engine::observe::Io", "static_mut", 1),
        ("uqm-engine::observe::InputView", "static_mut", 1),
        ("uqm::engine::observe::Io", "static_mut", 3),
        ("uqm::engine::observe::CampaignView", "static_mut", 2),
    ]);
    let prepared = extraction_measurement(&[
        ("uqm-engine::observe_port::ObservationIo", "static_mut", 1),
        ("uqm-engine::observe_port::InputView", "static_mut", 1),
        ("uqm::engine::observe::Io", "static_mut", 3),
        ("uqm::engine::observe::CampaignView", "static_mut", 2),
    ]);
    let current = extraction_measurement(&[
        ("uqm-engine::observe_port::ObservationIo", "static_mut", 1),
        ("uqm-engine::observe_port::InputView", "static_mut", 1),
        ("uqm-engine::observe::Io", "static_mut", 3),
        ("uqm-engine::observe::CampaignView", "static_mut", 2),
    ]);
    let mut baseline = Baseline {
        entries: base.baseline(),
        ..Default::default()
    };
    baseline
        .add_move(
            &format!("{old}::Io"),
            &format!("{port}::ObservationIo"),
            &prepared,
            &base,
        )
        .unwrap();
    baseline.add_move(old, port, &prepared, &base).unwrap();
    baseline.add_move(core, old, &current, &base).unwrap();
    assert_eq!(baseline.entries, current.baseline());
    assert_eq!(baseline.moved[old], port);
    assert_eq!(baseline.moved[core], old);
}
