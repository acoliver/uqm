//! Retained contact capabilities mutate the supplied campaign, without a published owner.
use std::cell::RefCell;
use uqm_rust::{
    engine::species::ship,
    engine::{
        battle::{Battle, Cx},
        observe::{Io, Unobserved},
        world::generation::SolarSys,
        Game, StoryKey,
    },
    planets::{
        generate::{
            chmmr::ChmmrSite,
            home_contact::HomeContactSite,
            identities::*,
            myc::MyconSite,
            orz::OrzSite,
            pet::{TalkingPetSite, UrquanEncounterSite},
            sam::SamatraSite,
            special::Story,
            utw::UtwigSite,
            vux::VuxSite,
            yehat::YehatSite,
        },
        solarsys::site::Site,
    },
};

fn campaign() -> Game {
    let mut game = Game::new();
    uqm_rust::state::ffi::rust_init_game_state(&mut game.world);
    {
        let mut fleets = game.world.avail_races();
        while let Some(id) = fleets.alloc() {
            let fleet = fleets.get_mut(id);
            fleet.actual_strength = 100;
            fleet.crew_level = 12;
            fleet.max_crew = 12;
            fleet.loc.x = 123;
            fleet.loc.y = 456;
            fleets.put(id);
        }
    }
    game
}

fn with_site(
    game: &mut Game,
    index: u8,
    run: impl FnOnce(&mut Site<'_, '_, '_>, &mut SolarSys<'_>),
) {
    let mut pickup = uqm_rust::planet_side::special_effects::PickupContext::NoLander;
    with_pickup_site(game, index, &mut pickup, run);
}

fn with_pickup_site(
    game: &mut Game,
    index: u8,
    pickup: &mut uqm_rust::planet_side::special_effects::PickupContext,
    run: impl FnOnce(&mut Site<'_, '_, '_>, &mut SolarSys<'_>),
) {
    let mut battle = Battle::new();
    let owner = RefCell::new(Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    ));
    let mut observer = Unobserved;
    let io = RefCell::new(Io {
        observer: &mut observer,
    });
    let mut system = SolarSys::new(&[], 0, index, uqm_rust::math::RandomContext::with_seed(123));
    let mut host = Site::new(
        &owner,
        &io,
        uqm_rust::engine::StarId(0),
        index,
        false,
        pickup,
        None,
    );
    run(&mut host, &mut system);
}

fn race_of_first_npc(game: &Game) -> u8 {
    let queue = game.world.npc_built_ships_ref();
    queue
        .get(
            queue
                .head()
                .unwrap_or_else(|| panic!("expected an NPC ship")),
        )
        .race_id
}

#[test]
#[serial_test::serial]
fn encounter_selection_and_reset_use_only_the_supplied_campaign() {
    use uqm_rust::planets::generate::sol::orbital::Orbital;
    let mut first = campaign();
    let mut second = campaign();
    first.world.last_encounter_group = 123;
    first.world.encounter_race = 9;
    with_site(&mut first, SOL_DEFINED, |host, _| {
        host.set_encounter_group(7)
    });
    with_site(&mut second, SOL_DEFINED, |host, _| {
        host.set_encounter_group(11)
    });
    assert_eq!(first.world.encounter_group, 7);
    assert_eq!(second.world.encounter_group, 11);
    with_site(&mut first, SAMATRA_DEFINED, |host, _| {
        host.reset_encounter()
    });
    assert_eq!(first.world.encounter_group, 0);
    assert_eq!(first.world.encounter_race, -1);
    assert_eq!(first.world.last_encounter_group, 123);
    assert_eq!(second.world.encounter_group, 11);
}

#[test]
#[serial_test::serial]
fn every_home_contact_tracks_and_clones_its_own_race() {
    let mut game = campaign();
    for (index, race) in [
        (ILWRATH_DEFINED, ship::ILWRATH_SHIP),
        (TALKING_PET_DEFINED, ship::UMGAH_SHIP),
        (PKUNK_DEFINED, ship::PKUNK_SHIP),
        (SUPOX_DEFINED, ship::SUPOX_SHIP),
        (DRUUGE_DEFINED, ship::DRUUGE_SHIP),
        (AQUA_HELIX_DEFINED, ship::THRADDASH_SHIP),
        (THRADD_DEFINED, ship::THRADDASH_SHIP),
        (TAALO_PROTECTOR_DEFINED, ship::ORZ_SHIP),
        (ORZ_DEFINED, ship::ORZ_SHIP),
        (ZOQFOT_DEFINED, ship::ZOQFOTPIK_SHIP),
        (BOMB_DEFINED, ship::UTWIG_SHIP),
        (UTWIG_DEFINED, ship::UTWIG_SHIP),
        (SUN_DEVICE_DEFINED, ship::MYCON_SHIP),
        (MYCON_DEFINED, ship::MYCON_SHIP),
        (EGG_CASE0_DEFINED, ship::MYCON_SHIP),
        (MAIDENS_DEFINED, ship::VUX_SHIP),
        (VUX_BEAST_DEFINED, ship::VUX_SHIP),
        (VUX_DEFINED, ship::VUX_SHIP),
        (SAMATRA_DEFINED, ship::BLACK_URQUAN_SHIP),
    ] {
        with_site(&mut game, index, |host, _| {
            assert!(HomeContactSite::start_sphere_tracking(host));
            HomeContactSite::notify_others_all_clear(host);
            HomeContactSite::set_current_activity(host, 4);
            assert_eq!(HomeContactSite::current_activity(host), 4);
            HomeContactSite::set_global_flags_and_data(host, 19);
            assert_eq!(host.flag(StoryKey::GLOBAL_FLAGS_AND_DATA), 19);
            HomeContactSite::reinit_built_ship_q(host);
            assert_eq!(HomeContactSite::built_ship_count(host), 0);
            HomeContactSite::clone_fleet_into_built_queue(host);
            assert_eq!(HomeContactSite::built_ship_count(host), 1);
            HomeContactSite::reinit_ip_group_q(host);
        });
        assert_eq!(race_of_first_npc(&game), race);
        let fleet = game.world.avail_races_ref();
        let tracked = fleet.get(
            fleet
                .at(usize::from(race))
                .unwrap_or_else(|| panic!("missing fleet")),
        );
        assert_eq!(tracked.known_strength, 1);
        assert_eq!((tracked.known_loc.x, tracked.known_loc.y), (123, 456));
    }
}

#[test]
#[serial_test::serial]
fn yehat_and_chmmr_use_retained_activity_flags_and_guard_queues() {
    let mut game = campaign();
    with_site(&mut game, YEHAT_DEFINED, |host, _| {
        assert!(YehatSite::start_sphere_tracking(host));
        YehatSite::notify_others_all_clear(host);
        YehatSite::set_current_activity(host, 4);
        assert_eq!(YehatSite::current_activity(host), 4);
        YehatSite::set_global_flags_and_data(host, 23);
        assert_eq!(host.flag(StoryKey::GLOBAL_FLAGS_AND_DATA), 23);
        YehatSite::reinit_built_ship_q(host);
        YehatSite::clone_yehat_fleet_into_built_queue(host);
        assert_eq!(YehatSite::built_ship_count(host), 1);
        YehatSite::reinit_ip_group_q(host);
    });
    assert_eq!(race_of_first_npc(&game), ship::YEHAT_SHIP);
    with_site(&mut game, CHMMR_DEFINED, |host, _| {
        assert!(ChmmrSite::start_ilwrath_sphere_tracking(host));
        ChmmrSite::set_current_activity(host, 4);
        assert_eq!(ChmmrSite::current_activity(host), 4);
        ChmmrSite::set_global_flags_and_data(host, 17);
        assert_eq!(host.flag(StoryKey::GLOBAL_FLAGS_AND_DATA), 17);
        ChmmrSite::reinit_built_ship_q(host);
        ChmmrSite::clone_ilwrath_fleet_into_built_queue(host);
        assert_eq!(ChmmrSite::built_ship_count(host), 1);
        ChmmrSite::reinit_ip_group_q(host);
    });
    assert_eq!(race_of_first_npc(&game), ship::ILWRATH_SHIP);
}

#[test]
#[serial_test::serial]
fn special_contacts_share_queues_but_preserve_race_and_story_choices() {
    let mut game = campaign();
    with_site(&mut game, ORZ_DEFINED, |host, _| {
        OrzSite::clone_escort_into_built_queue(host)
    });
    assert_eq!(race_of_first_npc(&game), ship::ORZ_SHIP);
    game.world.npc_built_ships().reinit();
    with_site(&mut game, TALKING_PET_DEFINED, |host, _| {
        assert!(!TalkingPetSite::built_ship_survives(host));
        TalkingPetSite::clone_zombie_into_built_queue(host);
        assert!(TalkingPetSite::built_ship_survives(host));
    });
    assert_eq!(race_of_first_npc(&game), ship::UMGAH_SHIP);
    game.world.npc_built_ships().reinit();
    with_site(&mut game, BOMB_DEFINED, |host, _| {
        assert!(UtwigSite::start_druuge_tracking(host));
        assert!(!UtwigSite::built_queue_has_ships(host));
        UtwigSite::clone_druuge_into_built_queue(host);
        assert!(UtwigSite::built_queue_has_ships(host));
    });
    assert_eq!(race_of_first_npc(&game), ship::DRUUGE_SHIP);
    game.world.npc_built_ships().reinit();
    with_site(&mut game, MYCON_DEFINED, |host, _| {
        assert!(!MyconSite::built_queue_has_ships(host));
        MyconSite::clone_ambush_ship_into_built_queue(host);
        assert!(MyconSite::built_queue_has_ships(host));
    });
    assert_eq!(race_of_first_npc(&game), ship::MYCON_SHIP);
    with_site(&mut game, VUX_DEFINED, |host, _| {
        assert!(!VuxSite::zex_is_dead(host));
        host.set_flag(StoryKey::ZEX_IS_DEAD, 1);
        assert!(VuxSite::zex_is_dead(host));
    });
    for black in [false, true] {
        game.world.npc_built_ships().reinit();
        with_site(&mut game, SAMATRA_DEFINED, |host, _| {
            assert!(!SamatraSite::built_queue_has_ships(host));
            SamatraSite::clone_guard_fleet(host, black);
            assert!(SamatraSite::built_queue_has_ships(host));
            assert!(SamatraSite::track_yehat(host));
            assert!(SamatraSite::rebel_escort_feasible(host));
            SamatraSite::reset_encounter(host);
        });
        assert_eq!(
            race_of_first_npc(&game),
            if black {
                ship::BLACK_URQUAN_SHIP
            } else {
                ship::URQUAN_SHIP
            }
        );
        game.world.npc_built_ships().reinit();
        with_site(&mut game, SAMATRA_DEFINED, |host, _| {
            SamatraSite::clone_remnant(host, black)
        });
        assert_eq!(
            race_of_first_npc(&game),
            if black {
                ship::BLACK_URQUAN_SHIP
            } else {
                ship::URQUAN_SHIP
            }
        );
    }
    with_site(&mut game, TALKING_PET_DEFINED, |host, _| {
        assert!(UrquanEncounterSite::begin_urquan_encounter(
            host,
            (2000, 3000)
        ));
        assert_eq!(UrquanEncounterSite::fuel_on_board(host), 0);
        UrquanEncounterSite::finish_encounter(host);
    });
    let encounters = game.world.encounters_ref();
    let encounter = encounters.get(
        encounters
            .head()
            .unwrap_or_else(|| panic!("missing encounter")),
    );
    assert_eq!(encounter.race_id, ship::URQUAN_SHIP);
    assert_eq!(encounter.num_ships, 1);
}

#[path = "planet_site_owner/scenes.rs"]
mod scenes;

#[path = "planet_site_owner/pickup.rs"]
mod pickup;
