use uqm_save::statefiles::{
    capture_from_images, initial_defined_image, initial_random_image, plan_restore,
    StateFileSections, StateSectionError,
};
use uqm_save::{BattleFleet, BattleGroup, BattleShip};

#[test]
fn group_images_round_trip_with_explicit_defined_offsets() {
    let group = BattleGroup {
        encounter_id: 2,
        current: true,
        star_index: 14,
        day_index: 3,
        month_index: 11,
        year_index: 2155,
        fleets: vec![BattleFleet {
            icon: 7,
            ships: vec![BattleShip {
                race_outer: 7,
                captains_name_index: 4,
                race_id: 7,
                index: 2,
                crew_level: 11,
                max_crew: 20,
                energy_level: 8,
                max_energy: 16,
            }],
        }],
    };
    let sections = StateFileSections {
        scan: Some(vec![0x1234_5678]),
        battle_groups: vec![group.clone()],
        ..Default::default()
    };
    let plan = plan_restore(&sections, &[0; 3]).unwrap();
    assert_eq!(plan.defined_offsets, [0, 1, 0]);
    assert_eq!(plan.battle_group_ref, Some(1));
    assert_eq!(
        plan.star_info.as_deref(),
        Some([0x78, 0x56, 0x34, 0x12].as_slice())
    );
    let (random, defined) = plan.groups.unwrap();
    assert_eq!(random, initial_random_image());
    assert_eq!(&defined[..1], initial_defined_image());
    assert_eq!(&defined[1..9], &[1, 3, 11, 0, 14, 0, 107, 8]);
    assert_eq!(
        &defined[269..],
        &[7, 1, 7, 0, 0, 4, 0, 0, 0, 7, 2, 11, 20, 8, 16, 0, 0, 0, 0]
    );
    let captured = capture_from_images(
        plan.star_info.as_deref().unwrap(),
        &random,
        &defined,
        &plan.defined_offsets,
        1,
    )
    .unwrap();
    assert_eq!(captured.scan, sections.scan);
    assert_eq!(captured.battle_groups[1], group);
}

#[test]
fn stale_offsets_are_rejected_without_game_types() {
    let sections = StateFileSections {
        group_list: Some(uqm_save::GroupList {
            last_encounter: 0,
            entries: vec![],
        }),
        ..Default::default()
    };
    assert_eq!(
        plan_restore(&sections, &[0, 45]),
        Err(StateSectionError::StaleDefinedGroupOffset {
            encounter_id: 2,
            offset: 45,
        })
    );
}
