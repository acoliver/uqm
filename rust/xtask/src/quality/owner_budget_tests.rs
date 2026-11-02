use super::*;

#[test]
fn owner_budgets_keep_fixed_phases_exact_scopes_and_growth_approval() {
    let scope = "unsafe|unsafe_blocks|uqm::battle::context_access::navigation::f";
    for (name, ends) in [
        ("phase4_state_access_unsafe", "phase-4"),
        ("phase6_bridge_access_unsafe", "phase-6"),
        ("phase7_render_access_unsafe", "phase-7"),
        ("phase7_assets_access_unsafe", "phase-7"),
        ("phase7_sound_access_unsafe", "phase-7"),
        ("phase7_input_access_unsafe", "phase-7"),
    ] {
        let mut budgets = BTreeMap::new();
        set_scoped(&mut budgets, name, 1, [(scope.into(), 1)].into(), false).unwrap();
        assert_eq!(budgets[name].ends, ends);
        assert_eq!(parse(&write(&budgets).unwrap()).unwrap(), budgets);
        assert!(check_growth(&budgets, &BTreeMap::new()).is_err());
        let before = budgets.clone();
        assert!(set_scoped(&mut budgets, name, 2, [(scope.into(), 2)].into(), false).is_err());
        assert_eq!(budgets, before);
        set_scoped(&mut budgets, name, 0, [(scope.into(), 0)].into(), false).unwrap();
        check_growth(&budgets, &before).unwrap();
        assert!(set(&mut budgets, name, 0, false).is_err());
        for bad in [
            "unsafe|unsafe_blocks|uqm::battle::context_access::*",
            "unsafe|unsafe_lines|uqm::battle::context_access::navigation::f",
            "unsafe|unsafe_fn|uqm::battle::context_access::navigation::f",
            "unsafe|unsafe_blocks|uqm::ships::races::f",
            "unsafe|unsafe_blocks|uqm::engine::battle::record::<ELEMENT>::life_span",
        ] {
            assert!(
                set_scoped(
                    &mut BTreeMap::new(),
                    name,
                    1,
                    [(bad.into(), 1)].into(),
                    false
                )
                .is_err(),
                "{bad}"
            );
        }
    }
    assert!(phase("phase4_other_access_unsafe").is_err());
    assert!(phase("phase7_other_access_unsafe").is_err());
    assert!(phase("phase6_other_access_unsafe").is_err());
}
