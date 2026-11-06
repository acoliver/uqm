use super::*;

#[test]
fn unit_budgets_own_exact_layer_findings_without_hiding_growth() {
    let name = "u5_01_world_projection";
    let key = "layers|module_edges|uqm::planets::generate::late_exports::<MyconSite>::built_queue_has_ships";
    let mut budgets = BTreeMap::new();
    set_scoped(&mut budgets, name, 2, [(key.into(), 2)].into(), false).unwrap();
    assert_eq!(budgets[name].ends, "U5-01");
    assert_eq!(parse(&write(&budgets).unwrap()).unwrap(), budgets);
    assert!(check_growth(&budgets, &BTreeMap::new()).is_err());
    let base = budgets.clone();
    assert!(set_scoped(&mut budgets, name, 3, [(key.into(), 3)].into(), false).is_err());
    let finding = |identity: &str, value| crate::quality::metrics::Finding {
        counter: "layers".into(),
        metric: "module_edges".into(),
        identity: identity.into(),
        file: String::new(),
        package: "uqm".into(),
        subsystem: "planets".into(),
        value,
    };
    let mut measured = Measurement {
        findings: vec![
            finding(key.rsplit('|').next().unwrap(), 2),
            finding("uqm::planets::generate::other", 1),
        ],
        ..Default::default()
    };
    check(&measured, &budgets).unwrap();
    assert_eq!(
        scoped_entries(&measured, &budgets)["layers|module_edges|uqm::planets::generate::other"],
        1
    );
    measured.findings[0].value = 3;
    assert!(check(&measured, &budgets).is_err());
    set_scoped(&mut budgets, name, 1, [(key.into(), 1)].into(), false).unwrap();
    check_growth(&budgets, &base).unwrap();
    for bad in [
        "layers|module_edges|uqm::planets::generate::*",
        "layers|module_edges|uqm::savegame::runtime::queues::capture",
        "layers|planned.uqm-modes|uqm::planets::generate::late_exports",
        "ffi|bridge_calls|uqm::planets::generate::late_exports",
    ] {
        assert!(
            set_scoped(&mut budgets, name, 1, [(bad.into(), 1)].into(), true).is_err(),
            "{bad}"
        );
    }
    let save = "layers|module_edges|uqm::savegame::runtime::layout::capture";
    set_scoped(
        &mut budgets,
        "u4_04_world_projection",
        1,
        [(save.into(), 1)].into(),
        false,
    )
    .unwrap();
    assert_eq!(budgets["u4_04_world_projection"].ends, "U4-04");
}

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

#[test]
fn remaining_mode_helpers_retire_with_their_reserved_crate_move() {
    for scope in [
        "uqm::planets::cargo::live::display",
        "uqm::planets::report::live::make_report",
        "uqm::planets::sphere::live::tint_frame",
        "uqm::comm::glue::resolve_phrase",
        "uqm::comm::speech_graphics::activity_frame",
    ] {
        let mut budgets = BTreeMap::new();
        set_scoped(
            &mut budgets,
            "u6_04_world_projection",
            1,
            [(format!("ffi|owner_projections|{scope}"), 1)].into(),
            false,
        )
        .unwrap();
        assert_eq!(budgets["u6_04_world_projection"].ends, "U6-04");
    }
}

#[test]
fn exposed_content_edges_have_exact_scopes_and_retire_with_the_mode_move() -> Result<(), String> {
    let name = "u6_04_content_edges";
    let module = "layers|module_edges|uqm::battle::ship_native::@file";
    let content = "layers|planned.uqm-content|uqm::ships::races::arilou_native::@file";
    let scopes = [(module.into(), 2), (content.into(), 1)].into();
    let mut budgets = BTreeMap::new();
    set_scoped(&mut budgets, name, 3, scopes, false)?;
    assert_eq!(budgets[name].ends, "U6-04");
    assert_eq!(parse(&write(&budgets)?)?, budgets);
    assert!(check_growth(&budgets, &BTreeMap::new()).is_err());
    let before = budgets.clone();
    assert!(set_scoped(
        &mut budgets,
        name,
        2,
        [(module.into(), 0), (content.into(), 2)].into(),
        false,
    )
    .is_err());
    assert_eq!(budgets, before);
    set_scoped(
        &mut budgets,
        name,
        2,
        [(module.into(), 1), (content.into(), 1)].into(),
        false,
    )?;
    check_growth(&budgets, &before)?;
    assert!(set(&mut BTreeMap::new(), name, 0, false).is_err());
    for invalid in [
        "layers|module_edges|uqm::ships::races::*::@file",
        "layers|module_edges|uqm::battle::ship_native::update",
        "layers|module_edges|uqm::comm::glue::@file",
        "layers|planned.uqm-modes|uqm::battle::ship_native::@file",
        "layers|planned_engine_core|uqm::battle::ship_native::@file",
        "ffi|owner_projections|uqm::battle::ship_native::@file",
        "ffi|bridge_calls|uqm::battle::ship_native::@file",
        "unsafe|unsafe_lines|uqm::battle::ship_native::@file",
        "layers|module_edges|uqm-engine::battle::@file",
    ] {
        assert!(
            set_scoped(
                &mut BTreeMap::new(),
                name,
                1,
                [(invalid.into(), 1)].into(),
                true
            )
            .is_err(),
            "{invalid}"
        );
    }
    Ok(())
}
