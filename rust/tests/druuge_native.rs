#![cfg(feature = "linked_c_archive")]

use std::{
    ffi::CStr,
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{abi::*, element};

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    fn init_druuge() -> *mut RACE_DESC;
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
        let lock = ENGINE.lock().unwrap_or_else(|e| e.into_inner());
        unsafe {
            assert_ne!(
                InitQueue(&raw mut disp_q, 8, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
        }
        Self { _engine: lock }
    }
}
impl Drop for Battlefield {
    fn drop(&mut self) {
        unsafe {
            UninitQueue(&raw mut disp_q);
        }
    }
}

struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    el: Box<ELEMENT>,
}
impl Ship {
    fn new(player: SIZE) -> Self {
        unsafe {
            let mut desc = Box::new(*init_druuge());
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *desc;
            ss.playerNr = player;
            let mut el: Box<ELEMENT> = Box::new(mem::zeroed());
            el.pParent = (&raw mut *ss).cast();
            el.playerNr = player;
            el.state_flags = PLAYER_SHIP as u16;
            Self { desc, ss, el }
        }
    }
    fn el(&mut self) -> *mut ELEMENT {
        &raw mut *self.el
    }
}

#[test]
fn descriptor_matches_druuge_c_data() {
    unsafe {
        let d = *init_druuge();
        assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
        assert_eq!(
            (
                d.ship_info.ship_cost,
                d.ship_info.crew_level,
                d.ship_info.max_crew
            ),
            (17, 14, 14)
        );
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (32, 32));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.druuge.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.druuge.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.druuge.meleeicons"
        );
        assert_eq!(d.fleet.strength, 254);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (9500, 2792));
        let c = d.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (20, 2, 1)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (4, 16, 50)
        );
        assert_eq!(
            (
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (4, 1, 10, 30, 5)
        );
        assert_eq!(d.cyborg_control.WeaponRange, 2400);
        assert!(d.preprocess_func.is_some() && d.postprocess_func.is_some());
        assert!(d.init_weapon_func.is_some() && d.cyborg_control.intelligence_func.is_some());
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.druuge.graphics.mauler.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.druuge.graphics.cannon.{size}")
            );
            assert!(d.ship_data.special_rsc[i].is_null());
        }
    }
}

#[test]
fn cannon_launch_uses_mass_driver_missile_parameters() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 5;
        ship.el.next.location = POINT { x: 100, y: 200 };
        let mut out = [ptr::null_mut()];
        assert_eq!(
            ship.desc.init_weapon_func.unwrap()(ship.el(), out.as_mut_ptr()),
            1
        );
        let missile = element(out[0]);
        assert!(!missile.is_null());
        assert_eq!((*missile).__bindgen_anon_1.life_span, 20);
        assert_eq!((*missile).__bindgen_anon_2.hit_points, 4);
        assert_eq!((*missile).__bindgen_anon_3.mass_points, 6);
        assert_eq!((*missile).__bindgen_anon_5.blast_offset, 6);
        assert_ne!((*missile).state_flags & IGNORE_SIMILAR as u16, 0);
        FreeElement(out[0]);
    }
}

#[test]
fn furnace_burns_one_crew_and_adds_sixteen_energy_at_boundaries() {
    let _battle = Battlefield::new();
    let mut ship = Ship::new(0);
    unsafe {
        let preprocess = ship.desc.preprocess_func.unwrap();
        (*ship.el()).__bindgen_anon_2.crew_level = 2;
        (*ship.desc).ship_info.energy_level = 15;
        (*ship.ss).cur_status_flags = SPECIAL as u16;
        preprocess(ship.el());
        assert_eq!((*ship.el()).__bindgen_anon_2.crew_level, 1);
        assert_eq!((*ship.desc).ship_info.energy_level, 31);
        assert_eq!((*ship.ss).special_counter, 30);

        (*ship.desc).ship_info.energy_level = 32;
        (*ship.ss).cur_status_flags = SPECIAL as u16;
        (*ship.ss).special_counter = 0;
        preprocess(ship.el());
        assert_eq!((*ship.el()).__bindgen_anon_2.crew_level, 1);
        assert_eq!((*ship.ss).cur_status_flags & SPECIAL as u16, 0);
    }
}

#[test]
fn intelligence_selects_pursuit_and_furnace_for_weapon_energy_shortfall() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        (*ship.desc).ship_info.energy_level = 3;
        ship.el.__bindgen_anon_2.crew_level = 14;
        (*ship.el()).__bindgen_anon_5.thrust_wait = 0;
        let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
        concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.el();
        concerns[ENEMY_SHIP_INDEX as usize].which_turn = 10;
        let ai = ship.desc.cyborg_control.intelligence_func.unwrap();
        ai(ship.el(), concerns.as_mut_ptr(), 0);
        assert_eq!(
            concerns[ENEMY_SHIP_INDEX as usize].MoveState,
            MOVEMENT_STATE_PURSUE
        );
        assert_eq!((*ship.el()).__bindgen_anon_5.thrust_wait, 11);
        assert_ne!((*ship.ss).ship_input_state & (WEAPON | SPECIAL) as u8, 0);
        assert_eq!((*ship.ss).ship_input_state & SPECIAL as u8, SPECIAL as u8);
    }
}

#[test]
fn cannon_recoil_is_applied_only_on_the_firing_frame_and_caps_speed() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 4;
        ship.ss.cur_status_flags = (WEAPON | SHIP_AT_MAX_SPEED) as u16;
        ship.ss.weapon_counter = 9;
        ship.desc.postprocess_func.unwrap()(ship.el());
        let (mut dx, mut dy) = (0, 0);
        GetCurrentVelocityComponents(&raw mut ship.el.velocity, &mut dx, &mut dy);
        assert_eq!((dx, dy), (0, 0));
        ship.ss.weapon_counter = 10;
        ship.desc.postprocess_func.unwrap()(ship.el());
        GetCurrentVelocityComponents(&raw mut ship.el.velocity, &mut dx, &mut dy);
        assert_eq!((dx, dy), (-768, 0));
        assert_eq!(ship.ss.cur_status_flags & SHIP_AT_MAX_SPEED as u16, 0);
        for _ in 0..5 {
            ship.desc.postprocess_func.unwrap()(ship.el());
        }
        GetCurrentVelocityComponents(&raw mut ship.el.velocity, &mut dx, &mut dy);
        assert_eq!((dx, dy), (-3072, 0));
    }
}
