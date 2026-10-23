mod native_ship_common;
use native_ship_common::*;
use uqm_rust::{
    ships::native::{abi::*, element, frame_index},
    state::game_state_keys::set_game_state,
};
unsafe extern "C" {
    fn init_samatra() -> *mut RACE_DESC;
}

unsafe fn boss() -> Ship {
    let mut ship = Ship::new(init_samatra, NPC_PLAYER_NUM as SIZE);
    ship.ship_frames = FrameSet::new(32);
    ship.weapon_frames = FrameSet::new(32);
    ship.special_frames = FrameSet::new(32);
    ship.desc.ship_data.ship = [ship.ship_frames.frame(0); 3];
    ship.desc.ship_data.weapon = [ship.weapon_frames.frame(0); 3];
    ship.desc.ship_data.special = [ship.special_frames.frame(0); 3];
    (*ship.ptr()).current.image.frame = ship.ship_frames.frame(0);
    (*ship.ptr()).next = (*ship.ptr()).current;
    (*ship.ptr()).state_flags |= APPEARING as u16;
    with_rng(7, 8, || (ship.desc.preprocess_func.unwrap())(ship.ptr()));
    ship
}
#[test]
fn generators_spawn_at_exact_offsets_and_decrement_once_before_gate_opens() {
    let _battle = Battle::new(64);
    unsafe {
        let ship = boss();
        assert_eq!(ship.desc.characteristics.max_thrust, 8);
        assert_eq!((ship.ss.weapon_counter, ship.ss.special_counter), (120, 36));
        let all = queued();
        assert_eq!(all.len(), 11);
        let generators: Vec<_> = all
            .iter()
            .copied()
            .filter(|h| (*element(*h)).death_func.is_some())
            .collect();
        let mut positions: Vec<_> = generators
            .iter()
            .map(|h| {
                let e = element(*h);
                assert_eq!((*e).__bindgen_anon_2.hit_points, 15);
                ((*e).current.location.x, (*e).current.location.y)
            })
            .collect();
        positions.sort();
        let mut expected: Vec<_> = [
            (-136, -35),
            (-47, -70),
            (35, -67),
            (118, -42),
            (115, 46),
            (64, 79),
            (-96, 76),
            (-145, 47),
        ]
        .iter()
        .map(|(x, y)| ((4096 + x * 4) & !15, (3840 + y * 4) & !15))
        .collect();
        expected.sort();
        assert_eq!(positions, expected);
        let gate = element(
            *all.iter()
                .find(|h| (*element(**h)).state_flags & FINITE_LIFE as u16 != 0)
                .unwrap(),
        );
        for (i, h) in generators.iter().enumerate() {
            ((*gate).preprocess_func.unwrap())(gate);
            assert_eq!((*prim(gate)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
            let g = element(*h);
            ((*g).death_func.unwrap())(g);
            assert_eq!(ship.desc.characteristics.max_thrust, 7 - i as u16);
            assert_eq!(frame_index((*g).current.image.frame), 9);
            ((*g).death_func.unwrap())(g);
            assert_eq!(ship.desc.characteristics.max_thrust, 7 - i as u16);
        }
        ((*gate).preprocess_func.unwrap())(gate);
        assert!((*gate).preprocess_func.is_none());
        assert_eq!((*gate).state_flags & FINITE_LIFE as u16, 0);
        assert_eq!(
            (*prim(gate)).Type,
            gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM
        );
        assert_eq!((*gate).__bindgen_anon_1.life_span, 2);
    }
}
#[test]
fn only_player_ship_bomb_carrier_through_open_gate_finishes_battle() {
    let _battle = Battle::new(64);
    unsafe {
        uqm_rust::state::ffi::rust_init_game_state();
        let mut ship = boss();
        let gate = element(
            *queued()
                .iter()
                .find(|h| (*element(**h)).state_flags & FINITE_LIFE as u16 != 0)
                .unwrap(),
        );
        let player = Ship::new(init_samatra, RPG_PLAYER_NUM as SIZE);
        let mut point = POINT { x: 0, y: 0 };
        let collide = (*gate).collision_func.unwrap();
        for generators in [8, 1, 0] {
            ship.desc.characteristics.max_thrust = generators;
            if generators == 0 {
                ((*gate).preprocess_func.unwrap())(gate);
            }
            for bomb in [0, 1] {
                for is_ship in [false, true] {
                    for owner in [RPG_PLAYER_NUM, NPC_PLAYER_NUM] {
                        GlobData.Game_state.CurrentActivity = IN_BATTLE as u16;
                        set_game_state("BOMB_CARRIER", bomb);
                        (*player.ptr()).playerNr = owner as SIZE;
                        (*player.ptr()).state_flags = if is_ship {
                            PLAYER_SHIP as u16
                        } else {
                            FINITE_LIFE as u16
                        };
                        collide(gate, &mut point, player.ptr(), &mut point);
                        assert_eq!(
                            GlobData.Game_state.CurrentActivity & IN_BATTLE as u16 == 0,
                            generators == 0 && bomb == 1 && is_ship && owner == RPG_PLAYER_NUM
                        );
                    }
                }
            }
        }
        set_game_state("BOMB_CARRIER", 0);
    }
}
#[test]
fn postprocess_spawns_comets_and_sentinels_only_while_generators_live() {
    let _battle = Battle::new(64);
    unsafe {
        let mut ship = boss();
        ship.ss.weapon_counter = 0;
        ship.ss.special_counter = 0;
        with_rng(5, 1, || (ship.desc.postprocess_func.unwrap())(ship.ptr()));
        assert_eq!(queued().len(), 13);
        assert_eq!((ship.ss.weapon_counter, ship.ss.special_counter), (240, 72));
        let comet = element(
            *queued()
                .iter()
                .find(|h| frame_index((*element(**h)).current.image.frame) == 24)
                .unwrap(),
        );
        (*comet).next = (*comet).current;
        with_rng(5, 0, || ((*comet).preprocess_func.unwrap())(comet));
        assert_eq!(ship.desc.characteristics.weapon_wait, 1);
        assert_eq!(queued().len(), 14);
        assert_ne!((*comet).state_flags & NONSOLID as u16, 0);
        let sentinel = element(
            *queued()
                .iter()
                .find(|h| (*element(**h)).__bindgen_anon_2.hit_points == 10)
                .unwrap(),
        );
        (*sentinel).next = (*sentinel).current;
        ((*sentinel).preprocess_func.unwrap())(sentinel);
        assert_eq!(ship.desc.characteristics.special_wait, 1);
        assert_eq!((*sentinel).__bindgen_anon_1.life_span, 3);
        ship.desc.characteristics.max_thrust = 0;
        ship.ss.weapon_counter = 0;
        ship.ss.special_counter = 0;
        (ship.desc.postprocess_func.unwrap())(ship.ptr());
        assert_eq!(queued().len(), 14);
    }
}

#[test]
fn sentinel_recoil_stuns_live_player_caps_velocity_and_preserves_sentinel() {
    let _battle = Battle::new(64);
    unsafe {
        let mut ship = boss();
        ship.ss.special_counter = 0;
        (ship.desc.postprocess_func.unwrap())(ship.ptr());
        let sentinel = element(
            *queued()
                .iter()
                .find(|h| (*element(**h)).__bindgen_anon_2.hit_points == 10)
                .unwrap(),
        );
        let mut player = Ship::new(init_samatra, RPG_PLAYER_NUM as SIZE);
        player.desc.ship_info.crew_level = 20;
        (*player.ptr()).__bindgen_anon_2.crew_level = 20;
        (*player.ptr()).__bindgen_anon_3.mass_points = 10;
        player.ss.cur_status_flags = (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16;
        SetVelocityComponents(&raw mut (*player.ptr()).velocity, 5000, 0);
        SetVelocityVector(&raw mut (*sentinel).velocity, 32, 4);
        let mut p = POINT { x: 0, y: 0 };
        ((*sentinel).collision_func.unwrap())(sentinel, &mut p, player.ptr(), &mut p);
        assert_eq!((*sentinel).__bindgen_anon_2.hit_points, 10);
        assert_eq!((*sentinel).__bindgen_anon_1.life_span, 2);
        assert_eq!((*sentinel).__bindgen_anon_4.turn_wait, 12);
        assert_ne!((*sentinel).state_flags & DEFY_PHYSICS as u16, 0);
        assert_eq!((*sentinel).state_flags & DISAPPEARING as u16, 0);
        assert_eq!(
            player.ss.cur_status_flags & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16,
            0
        );
        assert_eq!((*player.ptr()).__bindgen_anon_4.turn_wait, 1);
        assert_eq!((*player.ptr()).__bindgen_anon_5.thrust_wait, 3);
        let mut dx = 0;
        let mut dy = 0;
        GetCurrentVelocityComponents(&raw mut (*player.ptr()).velocity, &mut dx, &mut dy);
        assert_eq!((dx, dy), (5120, 0));
        assert_eq!(velocity(&(*sentinel).velocity), [0; 9]);
    }
}
