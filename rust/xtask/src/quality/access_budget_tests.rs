use super::*;
use crate::quality::metrics::{Finding, Measurement};

#[test]
fn access_budget_is_phase_two_exact_scoped_and_leaves_line_and_function_ratchets_intact() {
    let mut budgets = BTreeMap::new();
    let id = "uqm::engine::battle::access::<Cx<'_>>::ship";
    let scope = format!("unsafe|unsafe_blocks|{id}");
    let lines = format!("unsafe|unsafe_block_lines|{id}");
    set_scoped(
        &mut budgets,
        "stage_b_access_unsafe",
        3,
        [(scope.clone(), 1), (lines.clone(), 2)].into(),
        false,
    )
    .unwrap();
    assert_eq!(budgets["stage_b_access_unsafe"].ends, "phase-2");
    let finding = |metric: &str, identity: &str, value| Finding {
        counter: "unsafe".into(),
        metric: metric.into(),
        identity: identity.into(),
        file: "src/engine/battle/access.rs".into(),
        package: "uqm".into(),
        subsystem: "engine".into(),
        value,
    };
    let mut measurement = Measurement {
        findings: vec![
            finding("unsafe_blocks", id, 1),
            finding("unsafe_block_lines", id, 2),
        ],
        ..Default::default()
    };
    check(&measurement, &budgets).unwrap();
    assert!(scoped_entries(&measurement, &budgets)
        .values()
        .all(|v| *v == 0));
    for metric in ["unsafe_lines", "unsafe_fn", "unsafe_fn_lines"] {
        measurement.findings.push(finding(metric, id, 1));
        let measured = scoped_entries(&measurement, &budgets);
        assert_eq!(measured[&format!("unsafe|{metric}|uqm::engine")], 1);
    }
    measurement
        .findings
        .push(finding("unsafe_blocks", "uqm::engine::elsewhere", 1));
    assert_eq!(
        scoped_entries(&measurement, &budgets)["unsafe|unsafe_blocks|uqm::engine"],
        1
    );
    measurement.findings[0].value = 2;
    assert!(check(&measurement, &budgets).is_err());
    assert!(check_growth(&budgets, &BTreeMap::new()).is_err());
    for key in [
        "unsafe|unsafe_lines|uqm::engine::battle::access::f",
        "unsafe|unsafe_fn|uqm::engine::battle::access::f",
        "unsafe|unsafe_blocks|uqm::engine::battle::record::abi::other",
        "unsafe|unsafe_blocks|uqm::ships::races::vux_native::f",
        "unsafe|unsafe_blocks|uqm::engine::battle::access::*",
        "unsafe|unsafe_blocks|uqm::engine::battle::access",
    ] {
        assert!(
            set_scoped(
                &mut BTreeMap::new(),
                "stage_b_access_unsafe",
                1,
                [(key.into(), 1)].into(),
                false
            )
            .is_err(),
            "{key}"
        );
    }
    assert!(set(&mut BTreeMap::new(), "stage_b_access_unsafe", 1, false).is_err());
    assert!(set_scoped(
        &mut BTreeMap::new(),
        "stage_b_access_unsafe",
        4,
        [(scope, 1), (lines, 2)].into(),
        false
    )
    .is_err());
}
