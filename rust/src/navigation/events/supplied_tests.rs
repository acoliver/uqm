use super::*;
use crate::navigation::hyper_units::Geometry;

fn world() -> World {
    let mut world = World::new();
    world.story = Some(crate::engine::world::story::StoryState::new());
    let clock = &mut world.data.game.game_clock;
    clock.year_index = 2155;
    clock.month_index = 2;
    clock.day_index = 17;
    world.clock.init();
    world
}

fn services(random: &mut dyn FnMut() -> u32) -> EventServices<'_> {
    EventServices {
        geometry: Geometry::new((320, 240)),
        random,
    }
}

fn scheduled(world: &World) -> Vec<(u16, u8, u8, u8)> {
    let queue = world.clock.events_ref();
    let mut id = queue.head();
    let mut events = Vec::new();
    while let Some(current) = id {
        let event = queue.get(current);
        events.push((
            event.year_index,
            event.month_index,
            event.day_index,
            event.func_index,
        ));
        id = queue.succ(current);
    }
    events
}

fn populate_fleets(world: &mut World) {
    let mut fleets = world.avail_races();
    for _ in 0..=YEHAT_REBEL_SHIP {
        let id = fleets.alloc().unwrap();
        fleets.put(id);
    }
}

#[test]
fn initial_events_use_the_supplied_clock_without_sampling_rng() {
    let mut world = world();
    schedule_initial_events(
        &mut world,
        &mut services(&mut || panic!("initial event RNG")),
    );
    assert_eq!(world.story_state(StoryKey::SLYLANDRO_MULTIPLIER), 1);
    assert_eq!(
        scheduled(&world),
        [
            (2155, 2, 18, HYPERSPACE_ENCOUNTER_EVENT as u8),
            (2155, 3, 17, ARILOU_ENTRANCE_EVENT as u8),
            (2155, 8, 18, SLYLANDRO_RAMP_UP as u8),
            (
                2155 + YEARS_TO_KOHRAH_VICTORY as u16,
                2,
                17,
                KOHR_AH_VICTORIOUS_EVENT as u8
            ),
        ]
    );
}

#[test]
fn real_dispatch_updates_only_the_supplied_owner_and_schedules_portal_transitions() {
    let mut first = world();
    let second = world();
    let mut random = || panic!("portal event RNG");
    let mut services = services(&mut random);
    dispatch_event(&mut first, &mut services, ARILOU_ENTRANCE_EVENT as u8);
    assert_eq!(first.story_state(StoryKey::ARILOU_SPACE), 1);
    assert_eq!(second.story_state(StoryKey::ARILOU_SPACE), 0);
    assert_eq!(scheduled(&first), [(2155, 2, 20, ARILOU_EXIT_EVENT as u8)]);
    dispatch_event(&mut first, &mut services, ARILOU_EXIT_EVENT as u8);
    assert_eq!(first.story_state(StoryKey::ARILOU_SPACE), 0);
    assert_eq!(
        scheduled(&first)[1],
        (2155, 3, 17, ARILOU_ENTRANCE_EVENT as u8)
    );
    dispatch_event(&mut first, &mut services, ARILOU_UMGAH_CHECK as u8);
    assert_eq!(first.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 2);
    assert_eq!(second.story_state(StoryKey::ARILOU_CHECKED_UMGAH), 0);
}

#[test]
fn fleet_arrival_dispatches_recursively_before_daily_encounter_work() {
    let mut world = world();
    populate_fleets(&mut world);
    let utwig = world.avail_races_ref().at(UTWIG_SHIP as usize).unwrap();
    let supox = world.avail_races_ref().at(SUPOX_SHIP as usize).unwrap();
    world.avail_races().get_mut(utwig).actual_strength = 100;
    world.avail_races().get_mut(utwig).growth_err_term = 127;
    world.avail_races().get_mut(supox).actual_strength = 100;
    world.avail_races().get_mut(supox).growth_err_term = 127;
    let mut random = || panic!("interplanetary event RNG");
    let mut services = services(&mut random);
    dispatch_event(&mut world, &mut services, ADVANCE_UTWIG_SUPOX_MISSION as u8);
    assert_eq!(world.story_state(StoryKey::UTWIG_SUPOX_MISSION), 1);
    world.avail_races().get_mut(supox).days_left = 1;
    world.avail_races().get_mut(supox).func_index = ADVANCE_UTWIG_SUPOX_MISSION as u8;
    world.avail_races().get_mut(utwig).days_left = 2;
    world.avail_races().get_mut(utwig).growth_err_term = 0;
    dispatch_event(&mut world, &mut services, HYPERSPACE_ENCOUNTER_EVENT as u8);
    assert_eq!(world.story_state(StoryKey::UTWIG_SUPOX_MISSION), 2);
    assert_eq!(world.avail_races_ref().get(supox).actual_strength, 100);
    assert_eq!(world.avail_races_ref().get(utwig).actual_strength, 99);
    assert_eq!(world.avail_races_ref().get(utwig).days_left, 1);
    assert_eq!(scheduled(&world).len(), 2);
    assert_eq!(scheduled(&world)[0].3, HYPERSPACE_ENCOUNTER_EVENT as u8);
    assert_eq!(scheduled(&world)[1].3, ADVANCE_UTWIG_SUPOX_MISSION as u8);
}

#[test]
fn hyperspace_daily_event_uses_the_supplied_geometry_and_rng_stream() {
    let mut world = world();
    populate_fleets(&mut world);
    world.set_activity(IN_HYPERSPACE as u16 | IN_BATTLE as u16);
    let geometry = Geometry::new((640, 480));
    world.data.flagship.log_x = geometry.log_x(1000);
    world.data.flagship.log_y = geometry.log_y(2000);
    let first = world.avail_races_ref().at(0).unwrap();
    let orz = world.avail_races_ref().at(ORZ_SHIP as usize).unwrap();
    for id in [first, orz] {
        let mut fleets = world.avail_races();
        let fleet = fleets.get_mut(id);
        fleet.actual_strength = 100;
        fleet.growth_err_term = 127;
        fleet.loc = POINT { x: 1000, y: 2000 };
    }
    let mut samples = Vec::new();
    let mut random = || {
        let value = if samples.is_empty() { 19 } else { 20 };
        samples.push(value);
        value
    };
    dispatch_event(
        &mut world,
        &mut EventServices {
            geometry,
            random: &mut random,
        },
        HYPERSPACE_ENCOUNTER_EVENT as u8,
    );
    assert_eq!(samples, [19, 20]);
    let encounters = world.encounters_ref();
    assert_eq!(encounters.count(), 1);
    assert_eq!(encounters.get(encounters.head().unwrap()).race_id, 0);
    assert_eq!(
        scheduled(&world),
        [(2155, 2, 18, HYPERSPACE_ENCOUNTER_EVENT as u8)]
    );
}

#[test]
fn probe_ramp_down_and_unknown_events_preserve_scheduling_and_wrapping() {
    let mut world = world();
    let mut random = || panic!("probe event RNG");
    let mut services = services(&mut random);
    world.set_story_state(StoryKey::SLYLANDRO_MULTIPLIER, 2);
    dispatch_event(&mut world, &mut services, SLYLANDRO_RAMP_DOWN as u8);
    assert_eq!(world.story_state(StoryKey::SLYLANDRO_MULTIPLIER), 1);
    assert_eq!(
        scheduled(&world),
        [(2155, 3, 12, SLYLANDRO_RAMP_DOWN as u8)]
    );
    dispatch_event(&mut world, &mut services, SLYLANDRO_RAMP_DOWN as u8);
    assert_eq!(world.story_state(StoryKey::SLYLANDRO_MULTIPLIER), 0);
    assert_eq!(scheduled(&world).len(), 1);
    dispatch_event(&mut world, &mut services, u8::MAX);
    assert_eq!(scheduled(&world).len(), 1);
}

#[test]
fn victorious_event_postpones_at_samatra_and_after_the_joint_mission() {
    let mut world = world();
    world.set_catalog(std::borrow::Cow::Borrowed(
        &crate::planets::tables::STARMAP_ARRAY,
    ));
    let samatra = world
        .catalog()
        .iter()
        .position(|star| u32::from(star.index) == SAMATRA_DEFINED)
        .unwrap();
    world.current_star = Some(crate::engine::world::StarId(samatra));
    world.set_activity(IN_INTERPLANETARY as u16);
    let mut random = || panic!("victory event RNG");
    let mut services = services(&mut random);
    dispatch_event(&mut world, &mut services, KOHR_AH_VICTORIOUS_EVENT as u8);
    assert_eq!(
        scheduled(&world),
        [(2155, 2, 24, KOHR_AH_GENOCIDE_EVENT as u8)]
    );
    world.current_star = None;
    world.set_story_state(StoryKey::UTWIG_SUPOX_MISSION, 1);
    dispatch_event(&mut world, &mut services, KOHR_AH_VICTORIOUS_EVENT as u8);
    assert_eq!(
        scheduled(&world)[1],
        (2156, 2, 17, KOHR_AH_GENOCIDE_EVENT as u8)
    );
}

#[test]
fn genocide_prioritizes_druuge_and_ends_the_campaign_after_reaching_sol() {
    let mut world = world();
    populate_fleets(&mut world);
    for (race, strength, x, y) in [
        (BLACK_URQUAN_SHIP, 100, 1000, 1000),
        (HUMAN_SHIP, 50, 1001, 1000),
        (DRUUGE_SHIP, 40, 2000, 1000),
    ] {
        let id = world.avail_races_ref().at(race as usize).unwrap();
        let mut fleets = world.avail_races();
        let fleet = fleets.get_mut(id);
        fleet.actual_strength = strength;
        fleet.loc = POINT { x, y };
    }
    let mut random = || panic!("genocide event RNG");
    let mut services = services(&mut random);
    dispatch_event(&mut world, &mut services, KOHR_AH_GENOCIDE_EVENT as u8);
    let black = world
        .avail_races_ref()
        .at(BLACK_URQUAN_SHIP as usize)
        .unwrap();
    let fleets = world.avail_races_ref();
    let fleet = fleets.get(black);
    assert_eq!(
        (fleet.dest_loc.x, fleet.dest_loc.y, fleet.days_left),
        (2000, 1000, 6)
    );
    assert_eq!(world.story_state(StoryKey::KOHR_AH_FRENZY), 1);
    let mut fleets = world.avail_races();
    let mut id = fleets.head();
    while let Some(current) = id {
        id = fleets.succ(current);
        fleets.get_mut(current).actual_strength = 0;
    }
    fleets.get_mut(black).loc = POINT {
        x: SOL_X as i16,
        y: SOL_Y as i16,
    };
    world.set_activity(IN_HYPERSPACE as u16 | IN_BATTLE as u16);
    dispatch_event(&mut world, &mut services, KOHR_AH_GENOCIDE_EVENT as u8);
    assert_eq!(world.activity() & IN_BATTLE as u16, 0);
    assert_eq!(world.data.flagship.crew_enlisted, u16::MAX);
    assert_eq!(world.story_state(StoryKey::KOHR_AH_KILLED_ALL), 1);
}

#[test]
fn thraddash_arrival_runs_the_immediate_ilwrath_callback_before_returning() {
    let mut world = world();
    populate_fleets(&mut world);
    let ilwrath = world.avail_races_ref().at(ILWRATH_SHIP as usize).unwrap();
    let thraddash = world.avail_races_ref().at(THRADDASH_SHIP as usize).unwrap();
    world.avail_races().get_mut(ilwrath).actual_strength = 200;
    world.avail_races().get_mut(ilwrath).loc = POINT {
        x: (2500 + 2535) / 2,
        y: (8070 + 8358) / 2,
    };
    world.set_story_state(StoryKey::THRADD_MISSION, 3);
    world.set_story_state(StoryKey::ILWRATH_FIGHT_THRADDASH, 1);
    dispatch_event(
        &mut world,
        &mut services(&mut || panic!("recursive event RNG")),
        ADVANCE_THRADD_MISSION as u8,
    );
    assert_eq!(world.story_state(StoryKey::THRADD_MISSION), 4);
    for id in [ilwrath, thraddash] {
        let fleets = world.avail_races_ref();
        let fleet = fleets.get(id);
        assert_eq!(fleet.actual_strength, 0);
        assert_eq!(fleet.allied_state, DEAD_GUY as u16);
    }
    assert!(scheduled(&world).is_empty());
}
