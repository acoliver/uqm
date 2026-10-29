//! Exercise the Mmrnmhrm callbacks through the linked engine ABI.

#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;
use battle::*;

use std::ffi::CStr;
use uqm_rust::engine::battle::RaceState;
use uqm_rust::ships::native::abi::*;

use battle::Battle as Battlefield;

#[test]
fn descriptor_exposes_xwing_defaults_and_both_form_resources() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
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
                CStr::from_ptr(desc.ship_info.race_strings_rsc),
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
                CStr::from_ptr(desc.ship_data.ship_rsc[0]),
                c"ship.mmrnmhrm.graphics.xform.large"
            );
            assert_eq!(
                CStr::from_ptr(desc.ship_data.weapon_rsc[0]),
                c"ship.mmrnmhrm.graphics.torpedo.large"
            );
            assert_eq!(
                CStr::from_ptr(desc.ship_data.special_rsc[0]),
                c"ship.mmrnmhrm.graphics.ywing.large"
            );
            assert_eq!(desc.cyborg_control.WeaponRange, 200);
            assert!(desc.callbacks()[1] != 0);
            assert!(desc.callbacks()[2] != 0);
            assert!(desc.callbacks()[3] != 0);
            assert!(desc.callbacks()[4] != 0);
            assert!(desc.callbacks()[0] != 0);
            fixture.uninit();
        }
    })
}

#[test]
fn special_transform_checks_appearance_energy_and_cooldown() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let ship = fixture.ptr();
            fixture.ship_mut().cur_status_flags = SPECIAL as u16;
            (*ship).next.image.farray = fixture.descriptor_mut().ship_data.ship.as_mut_ptr();
            (*ship).next.image.frame = fixture.descriptor_mut().ship_data.ship[0];

            // Exactly enough energy is consumed, switching to Y form.
            fixture.preprocess();
            assert_eq!(
                (*ship).next.image.farray,
                fixture.descriptor_mut().ship_data.special.as_mut_ptr()
            );
            assert_ne!((*ship).state_flags & CHANGING as u16, 0);
            assert_eq!(fixture.ship_mut().special_counter, 0);

            // Cooldown and appearing state preserve the selected form and energy.
            fixture.ship_mut().special_counter = 1;
            (*ship).next.image.farray = fixture.descriptor_mut().ship_data.special.as_mut_ptr();
            assert_eq!(fixture.descriptor_mut().ship_info.energy_level, 0);
            fixture.preprocess();
            assert_eq!(
                (*ship).next.image.farray,
                fixture.descriptor_mut().ship_data.special.as_mut_ptr()
            );
            fixture.ship_mut().special_counter = 0;
            (*ship).state_flags |= APPEARING as u16;
            fixture.preprocess();
            assert_eq!(
                (*ship).next.image.farray,
                fixture.descriptor_mut().ship_data.special.as_mut_ptr()
            );
            fixture.uninit();
        }
    })
}

#[test]
fn missile_mode_returns_two_missiles_with_c_boundary_values() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let ship = fixture.ptr();
            (*ship).next.location = POINT { x: 1200, y: -900 };
            (*ship).next.image.farray = fixture.descriptor_mut().ship_data.special.as_mut_ptr();
            let slots = fixture.weapon(2);
            let handles = slots
                .into_iter()
                .map(|slot| fixture.weapon_record(slot).cast::<std::ffi::c_void>())
                .collect::<Vec<_>>();
            assert!(!handles[0].is_null());
            assert!(!handles[1].is_null());
            // Launch positions and the missile's C-defined life/damage fields are
            // asserted after the linked queue has allocated the handles.
            for handle in handles {
                let missile = handle.cast::<ELEMENT>();
                assert_eq!((*missile).__bindgen_anon_1.life_span, 40);
                assert_eq!((*missile).__bindgen_anon_2.hit_points, 1);
                assert_eq!((*missile).__bindgen_anon_3.mass_points, 1);
                assert_eq!((*missile).__bindgen_anon_5.blast_offset, 0);
                assert_ne!((*missile).state_flags & IGNORE_SIMILAR as u16, 0);
                assert_eq!((*missile).__bindgen_anon_4.turn_wait, 5);
                FreeElement(handle);
            }
            fixture.uninit();
        }
    })
}

#[test]
fn transforming_swaps_characteristics_and_each_ship_owns_its_alternate_form() {
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut first_fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let mut second_fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            use uqm_rust::strings::string_table::*;
            let sounds = AllocStringTable(4, 0);
            first_fixture.descriptor_mut().ship_data.ship_sounds =
                CaptureStringTable(sounds).cast();
            second_fixture.descriptor_mut().ship_data.ship_sounds =
                CaptureStringTable(sounds).cast();
            assert_ne!(
                first_fixture.descriptor_mut() as *mut RACE_DESC,
                second_fixture.descriptor_mut() as *mut RACE_DESC
            );
            let ship = first_fixture.ptr();
            (*ship).current.image.farray =
                first_fixture.descriptor_mut().ship_data.ship.as_mut_ptr();
            (*ship).next.image.farray = first_fixture
                .descriptor_mut()
                .ship_data
                .special
                .as_mut_ptr();
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
                first_fixture.descriptor_mut().cyborg_control.WeaponRange,
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
            assert_eq!(alternate(second_fixture.descriptor_mut()).max_thrust, 50);
            assert_eq!(
                GetStringTableIndex(first_fixture.descriptor_mut().ship_data.ship_sounds.cast()),
                2
            );
            assert_eq!(
                GetStringTableIndex(second_fixture.descriptor_mut().ship_data.ship_sounds.cast()),
                0
            );

            first_fixture.descriptor_mut().characteristics.turn_wait = 17;
            (*ship).current.image.farray = first_fixture
                .descriptor_mut()
                .ship_data
                .special
                .as_mut_ptr();
            (*ship).next.image.farray = first_fixture.descriptor_mut().ship_data.ship.as_mut_ptr();
            first_fixture.ship_mut().cur_status_flags = SHIP_AT_MAX_SPEED as u16;
            first_fixture.postprocess();
            assert_eq!(
                first_fixture.descriptor_mut().characteristics.max_thrust,
                20
            );
            assert_eq!(first_fixture.descriptor_mut().characteristics.turn_wait, 2);
            assert_eq!(alternate(first_fixture.descriptor_mut()).turn_wait, 17);
            assert_eq!(
                first_fixture.descriptor_mut().cyborg_control.WeaponRange,
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
            assert_eq!(
                GetStringTableIndex(first_fixture.descriptor_mut().ship_data.ship_sounds.cast()),
                0
            );
            first_fixture.descriptor_mut().ship_data.ship_sounds = std::ptr::null_mut();
            second_fixture.descriptor_mut().ship_data.ship_sounds = std::ptr::null_mut();
            FreeStringTable(sounds);
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
    battle_context::run(|| {
        let _battle = Battlefield::new(8);
        unsafe {
            let mut fixture = Ship::bare(&_battle, SPECIES_ID_MMRNMHRM_ID, 0);
            let ship = fixture.ptr();
            (*ship).next.image.farray = fixture.descriptor_mut().ship_data.ship.as_mut_ptr();
            (*ship).next.location = POINT { x: 1000, y: 1000 };
            let slots = fixture.weapon(2);
            let handles = slots
                .into_iter()
                .map(|slot| fixture.weapon_record(slot).cast::<std::ffi::c_void>())
                .collect::<Vec<_>>();
            let first = handles[0].cast::<ELEMENT>();
            let second = handles[1].cast::<ELEMENT>();
            assert!(!first.is_null() && !second.is_null());
            assert_eq!(
                (*first).current.location.x + (*second).current.location.x,
                2000
            );
            assert_eq!((*first).current.location.y, (*second).current.location.y);
            let mut target_fixture = Object::new(&_battle);
            let target = target_fixture.ptr();
            (*target).state_flags = PLAYER_SHIP as u16;
            (*target).playerNr = (*first).playerNr;
            (*target).__bindgen_anon_2.crew_level = 20;
            let mut point = POINT { x: 0, y: 0 };
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                (*first).collision_func.unwrap(),
                first,
                &mut point,
                target,
                &mut point,
                uqm_rust::battle::process_native::invoke_collision::CollisionCall::Hook,
            );
            assert_eq!((*target).__bindgen_anon_2.crew_level, 20);
            assert_eq!((*first).__bindgen_anon_2.hit_points, 1);
            for handle in handles {
                FreeElement(handle);
            }
            fixture.uninit();
        }
    })
}

fn alternate(desc: &RACE_DESC) -> CHARACTERISTIC_STUFF {
    let RaceState::Mmrnmhrm { alternate } = desc.race_state else {
        panic!("Mmrnmhrm alternate characteristics missing");
    };
    alternate
}
