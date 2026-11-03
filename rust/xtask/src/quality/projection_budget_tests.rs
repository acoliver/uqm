use super::*;

#[test]
fn picker_projection_retires_with_its_melee_session_owner() {
    let scope = "ffi|owner_projections|uqm::pickship::ffi::encounter_ship";
    let mut budgets = BTreeMap::new();
    set_scoped(
        &mut budgets,
        "u3_02_world_projection",
        1,
        [(scope.into(), 1)].into(),
        false,
    )
    .unwrap();
    assert_eq!(budgets["u3_02_world_projection"].ends, "U3-02");
    assert!(set_scoped(
        &mut budgets,
        "u6_04_world_projection",
        1,
        [(scope.into(), 1)].into(),
        false
    )
    .is_err());
}

#[test]
fn projection_budgets_have_exact_callers_unit_retirement_and_shrink_only_limits() {
    let name = "u5_02_world_projection";
    let scope = "ffi|owner_projections|uqm::planets::solarsys::lifecycle::ExploreSolarSys";
    let mut budgets = BTreeMap::new();
    set_scoped(&mut budgets, name, 2, [(scope.into(), 2)].into(), false).unwrap();
    assert_eq!(budgets[name].ends, "U5-02");
    assert_eq!(parse(&write(&budgets).unwrap()).unwrap(), budgets);
    assert!(check_growth(&budgets, &BTreeMap::new()).is_err());
    let before = budgets.clone();
    assert!(set_scoped(&mut budgets, name, 3, [(scope.into(), 3)].into(), false).is_err());
    assert_eq!(before, budgets);
    set_scoped(&mut budgets, name, 1, [(scope.into(), 1)].into(), false).unwrap();
    check_growth(&budgets, &before).unwrap();
    assert!(set(&mut budgets, name, 1, false).is_err());
    for bad in [
        "ffi|owner_projections|uqm::planets::*",
        "ffi|owner_projections|uqm::state::ffi::read_story",
        "ffi|owner_projections|uqm::game_init::kernel_abi::LoadKernel",
        "ffi|owner_projections|uqm::navigation::globals::InitGlobData",
        "ffi|owner_projections|uqm::ui::sis::DrawSISTitle",
        "ffi|bridge_calls|uqm::planets::solarsys::lifecycle::ExploreSolarSys",
        "unsafe|unsafe_lines|uqm::planets::solarsys::lifecycle::ExploreSolarSys",
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
    assert!(phase("u4_01_world_projection").is_err());
    assert!(phase("u8_01_world_projection").is_err());
}

#[test]
fn projection_scope_cannot_grow_even_when_its_unit_total_falls() {
    let one = "ffi|owner_projections|uqm::planets::solarsys::lifecycle::one";
    let two = "ffi|owner_projections|uqm::planets::solarsys::lifecycle::two";
    let mut budgets = BTreeMap::new();
    set_scoped(
        &mut budgets,
        "u5_02_world_projection",
        4,
        [(one.into(), 2), (two.into(), 2)].into(),
        false,
    )
    .unwrap();
    assert!(set_scoped(
        &mut budgets,
        "u5_02_world_projection",
        3,
        [(one.into(), 0), (two.into(), 3)].into(),
        false
    )
    .is_err());
}

#[test]
fn battle_and_ship_projection_budgets_retire_at_the_phase_two_exit() {
    for (name, scope, foreign) in [
        (
            "u2_10a_projection",
            "uqm::battle::battle_native::engine::Battle",
            "uqm::ships::master_native::engine::LoadMasterShipList",
        ),
        (
            "u2_10b_projection",
            "uqm::ships::master_native::engine::LoadMasterShipList",
            "uqm::battle::battle_native::engine::Battle",
        ),
    ] {
        let mut budgets = BTreeMap::new();
        set_scoped(
            &mut budgets,
            name,
            1,
            [(format!("ffi|owner_projections|{scope}"), 1)].into(),
            false,
        )
        .unwrap();
        assert_eq!(budgets[name].ends, "phase-2");
        assert!(set_scoped(
            &mut BTreeMap::new(),
            name,
            1,
            [(format!("ffi|owner_projections|{foreign}"), 1)].into(),
            false
        )
        .is_err());
        assert!(set_scoped(
            &mut BTreeMap::new(),
            "u6_04_world_projection",
            1,
            [(format!("ffi|owner_projections|{scope}"), 1)].into(),
            false
        )
        .is_err());
        assert!(check_growth(&budgets, &BTreeMap::new()).is_err());
    }
}
