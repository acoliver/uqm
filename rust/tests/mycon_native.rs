//! Mycon native entry point and callbacks linked against the engine archive.

use std::{ffi::CStr, mem, ptr};
use uqm_rust::ships::native::{abi::*, element};

static ENGINE: std::sync::Mutex<()> = std::sync::Mutex::new(());

unsafe extern "C" {
    fn init_mycon() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
}

#[test]
fn descriptor_and_plasmoid_callbacks_match_mycon_c_data() {
    let _engine = ENGINE.lock().unwrap();
    unsafe {
        let d = *init_mycon();
        assert_eq!(d.ship_info.ship_flags, (FIRES_FORE | SEEKING_WEAPON) as u16);
        assert_eq!(d.ship_info.ship_cost, 21);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (20, 20));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (40, 40));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.mycon.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.mycon.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.mycon.meleeicons"
        );
        assert_eq!(d.fleet.strength, 194);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (6392, 2200));
        assert_eq!(
            (
                d.characteristics.max_thrust,
                d.characteristics.thrust_increment
            ),
            (27, 9)
        );
        assert_eq!(
            (
                d.characteristics.weapon_energy_cost,
                d.characteristics.special_energy_cost
            ),
            (20, 40)
        );
        assert_eq!(
            (
                d.characteristics.energy_wait,
                d.characteristics.turn_wait,
                d.characteristics.thrust_wait,
                d.characteristics.weapon_wait,
                d.characteristics.special_wait,
                d.characteristics.ship_mass
            ),
            (4, 6, 6, 5, 0, 7)
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_rsc[0]),
            c"ship.mycon.graphics.podship.large"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.weapon_rsc[2]),
            c"ship.mycon.graphics.plasma.small"
        );
        assert!(d.ship_data.special_rsc.iter().all(|r| r.is_null()));
        assert_eq!(d.cyborg_control.WeaponRange, 3200);
        assert!(d.cyborg_control.intelligence_func.is_some());
        assert!(d.postprocess_func.is_some());
        assert!(d.init_weapon_func.is_some());
        assert!(d.preprocess_func.is_none());
    }
}

#[test]
fn plasma_launch_and_postprocess_boundaries_match_c() {
    let _engine = ENGINE.lock().unwrap();
    unsafe {
        assert_ne!(
            InitQueue(&raw mut disp_q, 8, mem::size_of::<ELEMENT>() as u16),
            0
        );
        InitDisplayList();
        let mut desc = Box::new(*init_mycon());
        let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
        ss.RaceDescPtr = &raw mut *desc;
        ss.playerNr = 0;
        let mut ship: Box<ELEMENT> = Box::new(mem::zeroed());
        ship.pParent = (&raw mut *ss).cast();
        ship.playerNr = 0;
        ship.next.location = POINT { x: 100, y: 200 };
        let mut handles = [ptr::null_mut(); 1];
        assert_eq!(
            desc.init_weapon_func.unwrap()(&raw mut *ship, handles.as_mut_ptr()),
            1
        );
        assert!(!handles[0].is_null());
        let plasma = element(handles[0]);
        assert_eq!((*plasma).__bindgen_anon_1.life_span, 143);
        assert_eq!((*plasma).__bindgen_anon_2.hit_points, 10);
        assert_eq!((*plasma).__bindgen_anon_3.mass_points, 10);
        assert_eq!((*plasma).__bindgen_anon_4.turn_wait, 3);
        assert_eq!((*plasma).__bindgen_anon_5.blast_offset, 0);
        (*plasma).next = (*plasma).current;
        (*plasma).__bindgen_anon_1.life_span = 100;
        (*plasma).preprocess_func.unwrap()(plasma);
        assert_eq!((*plasma).__bindgen_anon_2.hit_points, 7);
        assert_eq!((*plasma).__bindgen_anon_3.mass_points, 7);
        assert_eq!((*plasma).__bindgen_anon_4.turn_wait, 2);
        (*plasma).__bindgen_anon_2.hit_points = 3;
        (*plasma).preprocess_func.unwrap()(plasma);
        assert_eq!((*plasma).__bindgen_anon_1.life_span, 39);
        assert_eq!((*plasma).__bindgen_anon_3.mass_points, 3);
        assert_eq!((*plasma).__bindgen_anon_4.turn_wait, 1);
        ship.__bindgen_anon_2.crew_level = 17;
        ss.cur_status_flags = SPECIAL as u16;
        desc.postprocess_func.unwrap()(&raw mut *ship);
        assert_eq!(ship.__bindgen_anon_2.crew_level, 20);
        assert_eq!(desc.ship_info.energy_level, 0);
        assert_eq!(ss.special_counter, 0);

        ship.__bindgen_anon_2.crew_level = 10;
        desc.ship_info.energy_level = 39;
        desc.postprocess_func.unwrap()(&raw mut *ship);
        assert_eq!(ship.__bindgen_anon_2.crew_level, 10);
        assert_eq!(desc.ship_info.energy_level, 39);
        FreeElement(handles[0]);
        UninitQueue(&raw mut disp_q);
    }
}
