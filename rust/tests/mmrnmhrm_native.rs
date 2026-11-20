//! Exercise the Mmrnmhrm callbacks through the linked engine ABI.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

#[path = "support/special_hooks.rs"]
mod special_hooks;
use uqm_rust::collections::observation;
use uqm_rust::engine::battle::ai_constants::{
    FIRES_FORE, IGNORE_SIMILAR, IMMEDIATE_WEAPON, SEEKING_WEAPON, SPECIAL,
};
use uqm_rust::engine::battle::flags::{
    APPEARING, CHANGING, PLAYER_SHIP, SHIP_AT_MAX_SPEED, SHIP_BEYOND_MAX_SPEED,
};
use uqm_rust::engine::battle::record::{CharacteristicRecord, RaceDescriptor};
use uqm_rust::engine::battle::RaceState;
use uqm_rust::engine::species::species_id::SPECIES_ID_MMRNMHRM_ID;
use uqm_rust::graphics::geometry::Point;

use battle::Battle as Battlefield;

#[test]
fn descriptor_exposes_xwing_defaults_and_both_form_resources() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let desc = fixture.descriptor_mut();
            assert_eq!(
                desc.ship_info.ship_flags,
                (FIRES_FORE | IMMEDIATE_WEAPON) as u16
            );
            assert_eq!(desc.ship_info.ship_cost, 19);
            assert_eq!(
                (desc.ship_info.crew_level, desc.ship_info.max_crew),
                (20, 20)
            );
            assert_eq!(
                (desc.ship_info.energy_level, desc.ship_info.max_energy),
                (10, 10)
            );
            assert_eq!(
                desc.ship_info.race_strings_rsc.unwrap(),
                c"ship.mmrnmhrm.text"
            );
            assert_eq!(desc.characteristics.max_thrust, 20);
            assert_eq!(desc.characteristics.thrust_increment, 5);
            assert_eq!(desc.characteristics.energy_regeneration, 2);
            assert_eq!(desc.characteristics.special_energy_cost, 10);
            assert_eq!(desc.characteristics.turn_wait, 2);
            assert_eq!(desc.characteristics.thrust_wait, 1);
            assert_eq!(desc.characteristics.weapon_wait, 0);
            assert_eq!(desc.characteristics.ship_mass, 3);
            assert_eq!(
                desc.ship_data.ship_rsc[0].unwrap(),
                c"ship.mmrnmhrm.graphics.xform.large"
            );
            assert_eq!(
                desc.ship_data.weapon_rsc[0].unwrap(),
                c"ship.mmrnmhrm.graphics.torpedo.large"
            );
            assert_eq!(
                desc.ship_data.special_rsc[0].unwrap(),
                c"ship.mmrnmhrm.graphics.ywing.large"
            );
            assert_eq!(desc.cyborg_control.weapon_range, 200);
            assert!(desc.callbacks()[1] != 0);
            assert!(desc.callbacks()[2] != 0);
            assert!(desc.callbacks()[3] != 0);
            assert!(desc.callbacks()[4] != 0);
            assert!(desc.callbacks()[0] != 0);
            drop(desc);
            fixture.uninit();
        }
    })
}

#[test]
fn special_transform_checks_appearance_energy_and_cooldown() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let ship = fixture.id;
            fixture.ship_mut().cur_status_flags = SPECIAL as u16;
            battle::get_mut(ship).next.image.frames = fixture.frame_set(0);
            let value = fixture.descriptor_mut().ship_data.ship.view(0).to_owned();
            battle::get_mut(ship).next.image.frame = value;

            // Exactly enough energy is consumed, switching to Y form.
            observation::enable();
            observation::drain();
            fixture.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "mmrnmhrm"), 1);
            assert_eq!(battle::get(ship).next.image.frames, fixture.frame_set(2));
            assert_ne!(battle::get(ship).state_flags & CHANGING as u16, 0);
            assert_eq!(fixture.ship_mut().special_counter, 0);

            // Cooldown and appearing state preserve the selected form and energy.
            fixture.ship_mut().special_counter = 1;
            battle::get_mut(ship).next.image.frames = fixture.frame_set(2);
            assert_eq!(fixture.descriptor_mut().ship_info.energy_level, 0);
            fixture.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "mmrnmhrm"), 0);
            assert_eq!(battle::get(ship).next.image.frames, fixture.frame_set(2));
            fixture.ship_mut().special_counter = 0;
            battle::get_mut(ship).state_flags |= APPEARING as u16;
            fixture.preprocess();
            assert_eq!(special_hooks::count(&observation::drain(), "mmrnmhrm"), 0);
            assert_eq!(battle::get(ship).next.image.frames, fixture.frame_set(2));
            fixture.uninit();
        }
    })
}

#[test]
fn missile_mode_returns_two_missiles_with_c_boundary_values() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let ship = fixture.id;
            battle::get_mut(ship).next.location = Point { x: 1200, y: -900 };
            battle::get_mut(ship).next.image.frames = fixture.frame_set(2);
            let slots = fixture.weapon(2);
            let handles = slots
                .into_iter()
                .map(|slot| slot.expect("allocated weapon"))
                .collect::<Vec<_>>();
            // Launch positions and the missile's C-defined life/damage fields are
            // asserted after the linked queue has allocated the handles.
            for handle in handles {
                let missile = handle;
                assert_eq!(battle::get(missile).life, 40);
                assert_eq!(battle::get(missile).hits, 1);
                assert_eq!(battle::get(missile).mass, 1);
                assert_eq!(battle::get(missile).thrust, 0);
                assert_ne!(battle::get(missile).state_flags & IGNORE_SIMILAR as u16, 0);
                assert_eq!(battle::get(missile).turn, 5);
                battle_context::dispose(handle);
            }
            fixture.uninit();
        }
    })
}

#[test]
fn transforming_swaps_characteristics_and_each_ship_owns_its_alternate_form() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut first_fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let mut second_fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            first_fixture.descriptor_mut().install_sound_fixture(4);
            second_fixture.descriptor_mut().install_sound_fixture(4);
            let sound_index = |fixture: &mut Ship| {
                fixture
                    .descriptor_mut()
                    .ship_data
                    .ship_sounds
                    .map(|sounds| sounds.index())
            };
            let first_descriptor = std::ptr::from_ref(&*first_fixture.descriptor_mut());
            let second_descriptor = std::ptr::from_ref(&*second_fixture.descriptor_mut());
            assert_ne!(first_descriptor, second_descriptor);
            let ship = first_fixture.id;
            battle::get_mut(ship).current.image.frames = first_fixture.frame_set(0);
            battle::get_mut(ship).next.image.frames = first_fixture.frame_set(2);
            first_fixture.ship_mut().cur_status_flags =
                (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            first_fixture.ship_mut().weapon_counter = 7;
            first_fixture.postprocess();
            assert_eq!(
                first_fixture.descriptor_mut().characteristics.max_thrust,
                50
            );
            assert_eq!(
                first_fixture
                    .descriptor_mut()
                    .characteristics
                    .thrust_increment,
                10
            );
            assert_eq!(
                first_fixture
                    .descriptor_mut()
                    .characteristics
                    .energy_regeneration,
                1
            );
            assert_eq!(first_fixture.descriptor_mut().characteristics.turn_wait, 14);
            assert_eq!(
                first_fixture.descriptor_mut().characteristics.weapon_wait,
                20
            );
            assert_eq!(
                first_fixture.descriptor_mut().cyborg_control.weapon_range,
                3999
            );
            assert_ne!(
                first_fixture.descriptor_mut().ship_info.ship_flags & SEEKING_WEAPON as u16,
                0
            );
            assert_eq!(
                first_fixture.descriptor_mut().ship_info.ship_flags & IMMEDIATE_WEAPON as u16,
                0
            );
            assert_eq!(first_fixture.ship_mut().weapon_counter, 0);
            assert_eq!(
                first_fixture.ship_mut().cur_status_flags
                    & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                0
            );
            assert_eq!(
                second_fixture.descriptor_mut().characteristics.max_thrust,
                20
            );
            assert_eq!(alternate(&second_fixture.descriptor_mut()).max_thrust, 50);
            assert_eq!(sound_index(&mut first_fixture), Some(2));
            assert_eq!(sound_index(&mut second_fixture), Some(0));

            first_fixture.descriptor_mut().characteristics.turn_wait = 17;
            battle::get_mut(ship).current.image.frames = first_fixture.frame_set(2);
            battle::get_mut(ship).next.image.frames = first_fixture.frame_set(0);
            first_fixture.ship_mut().cur_status_flags = SHIP_AT_MAX_SPEED as u16;
            first_fixture.postprocess();
            assert_eq!(
                first_fixture.descriptor_mut().characteristics.max_thrust,
                20
            );
            assert_eq!(first_fixture.descriptor_mut().characteristics.turn_wait, 2);
            assert_eq!(alternate(&first_fixture.descriptor_mut()).turn_wait, 17);
            assert_eq!(
                first_fixture.descriptor_mut().cyborg_control.weapon_range,
                200
            );
            assert_eq!(
                first_fixture.descriptor_mut().ship_info.ship_flags & SEEKING_WEAPON as u16,
                0
            );
            assert_ne!(
                first_fixture.descriptor_mut().ship_info.ship_flags & IMMEDIATE_WEAPON as u16,
                0
            );
            assert_eq!(
                first_fixture.ship_mut().cur_status_flags
                    & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
            );
            first_fixture.uninit();
            assert_eq!(sound_index(&mut first_fixture), Some(0));
            second_fixture.uninit();
            assert!(matches!(
                first_fixture.descriptor_mut().race_state,
                RaceState::None
            ));
            assert!(matches!(
                second_fixture.descriptor_mut().race_state,
                RaceState::None
            ));
        }
    })
}

#[test]
fn xwing_launches_two_lasers_and_preserves_friendly_ships() {
    battle_context::run(|_| {
        let _battle = Battlefield::new(8);
        {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let ship = fixture.id;
            battle::get_mut(ship).next.image.frames = fixture.frame_set(0);
            battle::get_mut(ship).next.location = Point { x: 1000, y: 1000 };
            let slots = fixture.weapon(2);
            let handles = slots
                .into_iter()
                .map(|slot| slot.expect("allocated weapon"))
                .collect::<Vec<_>>();
            let first = handles[0];
            let second = handles[1];
            assert_eq!(
                battle::get(first).current.location.x + battle::get(second).current.location.x,
                2000
            );
            assert_eq!(
                battle::get(first).current.location.y,
                battle::get(second).current.location.y
            );
            let target_fixture = Object::new(&_battle);
            let target = target_fixture.id;
            battle::get_mut(target).state_flags = PLAYER_SHIP as u16;
            battle::get_mut(target).player_nr = battle::get(first).player_nr;
            battle::get_mut(target).hits = 20;
            let mut point = Point { x: 0, y: 0 };
            let mut other_point = point;
            hooks::invoke_collision(
                battle::get(first).collision_func.unwrap(),
                first,
                &mut point,
                target,
                &mut other_point,
            );
            assert_eq!(battle::get(target).hits, 20);
            assert_eq!(battle::get(first).hits, 1);
            for handle in handles {
                battle_context::dispose(handle);
            }
            fixture.uninit();
        }
    })
}

fn alternate(desc: &RaceDescriptor) -> CharacteristicRecord {
    let RaceState::Mmrnmhrm { alternate } = desc.race_state else {
        panic!("Mmrnmhrm alternate characteristics missing");
    };
    alternate
}

#[path = "support/hooks.rs"]
mod hooks;
