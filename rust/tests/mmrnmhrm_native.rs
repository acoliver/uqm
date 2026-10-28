//! Exercise the Mmrnmhrm callbacks through the linked engine ABI.

#[path = "support/battle_context.rs"]
mod battle_context;

use std::{
    ffi::CStr,
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::abi::*;

unsafe extern "C" {
    fn init_mmrnmhrm() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
}

static ENGINE: Mutex<()> = Mutex::new(());

struct Battlefield {
    _engine: MutexGuard<'static, ()>,
}

impl Battlefield {
    fn new() -> Self {
        let guard = ENGINE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        unsafe {
            assert_ne!(
                InitQueue(&raw mut disp_q, 8, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
        }
        Self { _engine: guard }
    }
}

impl Drop for Battlefield {
    fn drop(&mut self) {
        unsafe {
            UninitQueue(&raw mut disp_q);
        }
    }
}

#[test]
fn descriptor_exposes_xwing_defaults_and_both_form_resources() {
    battle_context::run(|| {
        let _battle = Battlefield::new();
        unsafe {
            let mut desc = *init_mmrnmhrm();
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
            assert!(desc.preprocess_func.is_some());
            assert!(desc.postprocess_func.is_some());
            assert!(desc.init_weapon_func.is_some());
            assert!(desc.cyborg_control.intelligence_func.is_some());
            assert!(desc.uninit_func.is_some());
            desc.uninit_func.unwrap()(&mut desc);
        }
    })
}

#[test]
fn special_transform_checks_appearance_energy_and_cooldown() {
    battle_context::run(|| {
        let _battle = Battlefield::new();
        unsafe {
            let mut desc = Box::new(*init_mmrnmhrm());
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *desc;
            ss.cur_status_flags = SPECIAL as u16;
            let mut ship: Box<ELEMENT> = Box::new(mem::zeroed());
            ship.pParent = (&raw mut *ss).cast();
            ship.next.image.farray = desc.ship_data.ship.as_mut_ptr();
            ship.next.image.frame = desc.ship_data.ship[0];
            let preprocess = desc.preprocess_func.unwrap();

            // Exactly enough energy is consumed, switching to Y form.
            preprocess(&raw mut *ship);
            assert_eq!(ship.next.image.farray, desc.ship_data.special.as_mut_ptr());
            assert_ne!(ship.state_flags & CHANGING as u16, 0);
            assert_eq!(ss.special_counter, 0);

            // Cooldown and appearing state preserve the selected form and energy.
            ss.special_counter = 1;
            ship.next.image.farray = desc.ship_data.special.as_mut_ptr();
            assert_eq!(desc.ship_info.energy_level, 0);
            preprocess(&raw mut *ship);
            assert_eq!(ship.next.image.farray, desc.ship_data.special.as_mut_ptr());
            ss.special_counter = 0;
            ship.state_flags |= APPEARING as u16;
            preprocess(&raw mut *ship);
            assert_eq!(ship.next.image.farray, desc.ship_data.special.as_mut_ptr());
            desc.uninit_func.unwrap()(&raw mut *desc);
        }
    })
}

#[test]
fn missile_mode_returns_two_missiles_with_c_boundary_values() {
    battle_context::run(|| {
        let _battle = Battlefield::new();
        unsafe {
            let mut desc = Box::new(*init_mmrnmhrm());
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *desc;
            ss.ShipFacing = 0;
            let mut ship: Box<ELEMENT> = Box::new(mem::zeroed());
            ship.pParent = (&raw mut *ss).cast();
            ship.next.location = POINT { x: 1200, y: -900 };
            ship.next.image.farray = desc.ship_data.special.as_mut_ptr();
            let mut handles = [ptr::null_mut(); 2];
            let init = desc.init_weapon_func.unwrap();
            assert_eq!(init(&raw mut *ship, handles.as_mut_ptr()), 2);
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
            desc.uninit_func.unwrap()(&raw mut *desc);
        }
    })
}

#[test]
fn transforming_swaps_characteristics_and_each_ship_owns_its_alternate_form() {
    battle_context::run(|| {
        let _battle = Battlefield::new();
        unsafe {
            let mut first = Box::new(*init_mmrnmhrm());
            let mut second = Box::new(*init_mmrnmhrm());
            assert_ne!(first.data, second.data);
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *first;
            let mut ship: Box<ELEMENT> = Box::new(mem::zeroed());
            ship.pParent = (&raw mut *ss).cast();
            ship.current.image.farray = first.ship_data.ship.as_mut_ptr();
            ship.next.image.farray = first.ship_data.special.as_mut_ptr();
            ss.cur_status_flags = (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
            ss.weapon_counter = 7;
            first.postprocess_func.unwrap()(&raw mut *ship);
            assert_eq!(first.characteristics.max_thrust, 50);
            assert_eq!(first.characteristics.thrust_increment, 10);
            assert_eq!(first.characteristics.energy_regeneration, 1);
            assert_eq!(first.characteristics.turn_wait, 14);
            assert_eq!(first.characteristics.weapon_wait, 20);
            assert_eq!(first.cyborg_control.WeaponRange, 3999);
            assert_ne!(first.ship_info.ship_flags & SEEKING_WEAPON as u16, 0);
            assert_eq!(first.ship_info.ship_flags & IMMEDIATE_WEAPON as u16, 0);
            assert_eq!(ss.weapon_counter, 0);
            assert_eq!(
                ss.cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                0
            );
            assert_eq!(second.characteristics.max_thrust, 20);
            assert_eq!((*second.data.cast::<CHARACTERISTIC_STUFF>()).max_thrust, 50);

            first.characteristics.turn_wait = 17;
            ship.current.image.farray = first.ship_data.special.as_mut_ptr();
            ship.next.image.farray = first.ship_data.ship.as_mut_ptr();
            ss.cur_status_flags = SHIP_AT_MAX_SPEED as u16;
            first.postprocess_func.unwrap()(&raw mut *ship);
            assert_eq!(first.characteristics.max_thrust, 20);
            assert_eq!(first.characteristics.turn_wait, 2);
            assert_eq!((*first.data.cast::<CHARACTERISTIC_STUFF>()).turn_wait, 17);
            assert_eq!(first.cyborg_control.WeaponRange, 200);
            assert_eq!(first.ship_info.ship_flags & SEEKING_WEAPON as u16, 0);
            assert_ne!(first.ship_info.ship_flags & IMMEDIATE_WEAPON as u16, 0);
            assert_eq!(
                ss.cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
                (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16
            );
            first.uninit_func.unwrap()(&raw mut *first);
            second.uninit_func.unwrap()(&raw mut *second);
            assert!(first.data.is_null());
            assert!(second.data.is_null());
        }
    })
}

#[test]
fn xwing_launches_two_lasers_and_preserves_friendly_ships() {
    battle_context::run(|| {
        let _battle = Battlefield::new();
        unsafe {
            let mut desc = Box::new(*init_mmrnmhrm());
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *desc;
            let mut ship: Box<ELEMENT> = Box::new(mem::zeroed());
            ship.pParent = (&raw mut *ss).cast();
            ship.next.image.farray = desc.ship_data.ship.as_mut_ptr();
            ship.next.location = POINT { x: 1000, y: 1000 };
            let mut handles = [ptr::null_mut(); 2];
            assert_eq!(
                desc.init_weapon_func.unwrap()(&raw mut *ship, handles.as_mut_ptr()),
                2
            );
            let first = handles[0].cast::<ELEMENT>();
            let second = handles[1].cast::<ELEMENT>();
            assert!(!first.is_null() && !second.is_null());
            assert_eq!(
                (*first).current.location.x + (*second).current.location.x,
                2000
            );
            assert_eq!((*first).current.location.y, (*second).current.location.y);
            let mut target: ELEMENT = mem::zeroed();
            target.state_flags = PLAYER_SHIP as u16;
            target.playerNr = (*first).playerNr;
            target.__bindgen_anon_2.crew_level = 20;
            let mut point = POINT { x: 0, y: 0 };
            uqm_rust::battle::process_native::invoke_collision::invoke_legacy_collision(
                (*first).collision_func.unwrap(),
                first,
                &mut point,
                &mut target,
                &mut point,
            );
            assert_eq!(target.__bindgen_anon_2.crew_level, 20);
            assert_eq!((*first).__bindgen_anon_2.hit_points, 1);
            for handle in handles {
                FreeElement(handle);
            }
            desc.uninit_func.unwrap()(&raw mut *desc);
        }
    })
}
