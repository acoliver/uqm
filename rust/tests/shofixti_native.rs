//! Exercise Shofixti's live callbacks against the linked engine.
use uqm_rust::ships::races::{intruder_preprocess, marine_collision};
#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "support/battle.rs"]
mod shared;

use std::{
    ffi::{c_void, CStr},
    mem, ptr,
};
use uqm_rust::ships::native::{
    abi::*, cosine, element, facing_to_angle, frame_index, put_element, sine,
};

use uqm_rust::supermelee::native::battle_picker::live::{selectShipComputer, selectShipHuman};

use uqm_rust::battle::process_native::FreeElement;
use uqm_rust::engine::battle::legacy::DisplayArray;
use uqm_rust::graphics::context_abi::_GraphicsStatusFlags;
use uqm_rust::graphics::native_canvas::{
    lifecycle::TFB_DrawCanvas_New_Paletted, pixels::TFB_DrawCanvas_SetPixelIndexes,
};
use uqm_rust::graphics::tfb_draw_abi::{TFB_DrawImage_Delete, TFB_DrawImage_New};

struct Battle {
    inner: shared::Battle,
    globals: GLOBDATA,
    screen: (i32, i32),
    graphics_flags: BYTE,
    counters: [BYTE; 2],
}
impl Battle {
    unsafe fn new(capacity: COUNT) -> Self {
        let globals = GlobData;
        let screen = (ScreenWidth, ScreenHeight);
        let graphics_flags = _GraphicsStatusFlags;
        let counters = uqm_rust::app::bridge::battle(|b| b.counters);
        let inner = shared::Battle::new(capacity);
        GlobData.Game_state.CurrentActivity = IN_ENCOUNTER as _;
        uqm_rust::state::ffi::rust_init_game_state();
        set_recruited(true);
        ScreenWidth = 320;
        ScreenHeight = 240;
        _GraphicsStatusFlags &= !4;
        uqm_rust::app::bridge::battle(|b| b.counters = [10, 10]);
        ResetWinnerStarShip();
        Self {
            inner,
            globals,
            screen,
            graphics_flags,
            counters,
        }
    }
}
impl Drop for Battle {
    fn drop(&mut self) {
        unsafe {
            ResetWinnerStarShip();
            GlobData = self.globals;
            (ScreenWidth, ScreenHeight) = self.screen;
            _GraphicsStatusFlags = self.graphics_flags;
            uqm_rust::app::bridge::battle(|b| b.counters = self.counters);
        }
    }
}

unsafe fn set_recruited(value: bool) {
    uqm_rust::state::ffi::rust_set_game_state_bits(
        SHOFIXTI_RECRUITED as i32,
        END_SHOFIXTI_RECRUITED as i32,
        value as BYTE,
    );
}

#[repr(C)]
struct FrameDesc {
    kind: u16,
    index: u16,
    hot_spot: [i16; 2],
    bounds: [i16; 2],
    image: *mut c_void,
    parent: *mut DrawableDesc,
}
#[repr(C)]
struct DrawableDesc {
    flags: u8,
    max_index: u16,
    frame: *mut FrameDesc,
}
struct FrameSet {
    _drawable: Box<DrawableDesc>,
    frames: Vec<FrameDesc>,
    image: *mut c_void,
}
impl FrameSet {
    fn new(count: u16) -> Self {
        let mut drawable = Box::new(DrawableDesc {
            flags: 0,
            max_index: count - 1,
            frame: ptr::null_mut(),
        });
        let parent = &raw mut *drawable;
        let mut frames: Vec<_> = (0..count)
            .map(|index| FrameDesc {
                kind: 0,
                index,
                hot_spot: [0, 0],
                bounds: [4, 4],
                image: ptr::null_mut(),
                parent,
            })
            .collect();
        drawable.frame = frames.as_mut_ptr();
        Self {
            _drawable: drawable,
            frames,
            image: ptr::null_mut(),
        }
    }
    fn frame(&self, index: usize) -> FRAME {
        (&raw const self.frames[index]).cast_mut().cast()
    }
    unsafe fn make_opaque(&mut self) {
        let canvas = TFB_DrawCanvas_New_Paletted(4, 4, ptr::null_mut(), -1);
        assert!(!canvas.is_null());
        assert_ne!(
            TFB_DrawCanvas_SetPixelIndexes(canvas, [1u8; 16].as_ptr(), 4, 4),
            0
        );
        self.image = TFB_DrawImage_New(canvas).cast();
        assert!(!self.image.is_null());
        for frame in &mut self.frames {
            frame.image = self.image;
        }
    }
}
impl Drop for FrameSet {
    fn drop(&mut self) {
        if !self.image.is_null() {
            unsafe {
                TFB_DrawImage_Delete(self.image.cast());
            }
        }
    }
}
use shared::PoolShip;
struct Ship<'a> {
    inner: shared::Ship<'a>,
    handle: HELEMENT,
    ship_frames: FrameSet,
    weapon_frames: FrameSet,
    special_frames: FrameSet,
    captain: FrameSet,
}
impl<'a> Ship<'a> {
    unsafe fn new(battle: &'a Battle, player: SIZE) -> Self {
        let mut inner = shared::Ship::new(&battle.inner, SPECIES_ID_SHOFIXTI_ID, player);
        let handle = inner.handle;
        let ship_frames = FrameSet::new(16);
        let weapon_frames = FrameSet::new(16);
        let desc = inner.descriptor_mut();
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.weapon = [weapon_frames.frame(0); 3];
        let special_frames = FrameSet::new(10);
        let captain = FrameSet::new(20);
        desc.ship_data.special = [special_frames.frame(0); 3];
        desc.ship_data.captain_control.special = captain.frame(14);
        let frames = (&raw mut desc.ship_data.ship).cast();
        let el = inner.element_mut();
        el.current.image.farray = frames;
        el.current.image.frame = ship_frames.frame(0);
        el.next = el.current;
        el.IntersectControl.IntersectStamp.frame = ship_frames.frame(0);
        el.IntersectControl.EndPoint = POINT { x: 1000, y: 1000 };
        el.IntersectControl.IntersectStamp.origin = POINT { x: 1000, y: 1000 };
        Self {
            inner,
            handle,
            ship_frames,
            weapon_frames,
            special_frames,
            captain,
        }
    }

    unsafe fn fire(&mut self, out: &mut [HELEMENT]) -> COUNT {
        uqm_rust::app::bridge::enter(|cx| {
            let origin = cx.weapon_origin(self.inner.id);
            let weapons = cx.spawn_weapons(self.inner.ship_id, origin);
            for (slot, weapon) in out.iter_mut().zip(&weapons) {
                *slot = weapon.map_or(ptr::null_mut(), |id| cx.legacy_element(id).cast());
            }
            weapons.len() as COUNT
        })
    }
    fn ptr(&self) -> *mut ELEMENT {
        self.handle.cast()
    }
    unsafe fn ai(&mut self, concerns: &mut [EVALUATE_DESC; 5]) {
        let mut subject = *self.ptr();
        self.inner.intelligence(&mut subject, &mut concerns[..4], 0);
    }
    unsafe fn arm(&mut self) -> ElementProcessFunc {
        self.inner
            .descriptor_mut()
            .ship_data
            .captain_control
            .special = self.captain.frame(16);
        self.inner.ship_mut().old_status_flags = 0;
        self.inner.ship_mut().cur_status_flags = SPECIAL as u16;
        let old = disp_q.tail;
        self.inner.postprocess();
        if old == disp_q.tail {
            None
        } else {
            (*element(disp_q.tail)).death_func
        }
    }
}
unsafe fn core(callback: uqm_rust::engine::battle::ElementFn, element: *mut ELEMENT) {
    let id = shared::id(element);
    uqm_rust::app::bridge::enter(|cx| callback(cx, id));
}

unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
    (&raw mut DisplayArray)
        .cast::<PRIMITIVE>()
        .add((*e).PrimIndex as usize)
}
fn rgba(c: Color) -> (u8, u8, u8, u8) {
    (c.r, c.g, c.b, c.a)
}
unsafe fn with_rng<T>(seed: DWORD, draws: usize, action: impl FnOnce() -> T) -> T {
    TFB_SeedRandom(seed);
    for _ in 0..draws {
        TFB_Random();
    }
    let expected = TFB_Random();
    TFB_SeedRandom(seed);
    let result = action();
    assert_eq!(
        TFB_Random(),
        expected,
        "seed {seed}: expected {draws} draws"
    );
    result
}
unsafe fn object<'a>(
    battle: &'a Battle,
    flags: u16,
    mass: BYTE,
    hp: COUNT,
    dx: i32,
    dy: i32,
) -> shared::Object<'a> {
    let mut object = shared::Object::new(&battle.inner);
    let e = object.ptr();
    (*e).state_flags = flags;
    (*e).playerNr = -1;
    (*e).__bindgen_anon_1.life_span = 50;
    (*e).__bindgen_anon_2.hit_points = hp;
    (*e).__bindgen_anon_3.mass_points = mass;
    (*e).next.location = POINT {
        x: (4000 + dx) as COORD,
        y: (4000 + dy) as COORD,
    };
    put_element(e.cast());
    object
}

#[test]
fn complete_descriptor_and_damaged_tanaka_katana_variant_are_fresh_each_call() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        for (activity, recruited, damaged) in [
            (0, false, false),
            (0x102, false, true),
            (2, true, false),
            (2, false, true),
            (0, false, false),
        ] {
            GlobData.Game_state.CurrentActivity = activity;
            set_recruited(recruited);
            let mut original = shared::descriptor(SPECIES_ID_SHOFIXTI_ID);
            let p = &raw mut original;
            let d = *p;
            assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
            assert_eq!(d.ship_info.ship_cost, 5);
            assert_eq!(
                (
                    d.ship_info.crew_level,
                    d.ship_info.max_crew,
                    d.ship_info.energy_level,
                    d.ship_info.max_energy
                ),
                (6, 6, 4, 4)
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.race_strings_rsc),
                c"ship.shofixti.text"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.icons_rsc),
                c"ship.shofixti.icons"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_info.melee_icon_rsc),
                c"ship.shofixti.meleeicons"
            );
            assert!(
                d.ship_info.race_strings.is_null()
                    && d.ship_info.icons.is_null()
                    && d.ship_info.melee_icon.is_null()
            );
            assert_eq!(
                (d.fleet.strength, d.fleet.known_loc.x, d.fleet.known_loc.y),
                (0, 0, 0)
            );
            let c = d.characteristics;
            assert_eq!(
                (
                    c.max_thrust,
                    c.thrust_increment,
                    c.turn_wait,
                    c.thrust_wait,
                    c.weapon_wait
                ),
                if damaged {
                    (8, 4, 4, 3, 10)
                } else {
                    (35, 5, 1, 0, 3)
                }
            );
            assert_eq!(
                (
                    c.energy_regeneration,
                    c.energy_wait,
                    c.weapon_energy_cost,
                    c.special_energy_cost,
                    c.special_wait,
                    c.ship_mass
                ),
                (1, 9, 1, 0, 0, 1)
            );
            for (i, size) in ["large", "medium", "small"].iter().enumerate() {
                let scout = if damaged { "oldscout" } else { "scout" };
                assert_eq!(
                    CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                    format!("ship.shofixti.graphics.{scout}.{size}")
                );
                assert_eq!(
                    CStr::from_ptr(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                    format!("ship.shofixti.graphics.missile.{size}")
                );
                if damaged {
                    assert!(d.ship_data.special_rsc[i].is_null());
                } else {
                    assert_eq!(
                        CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                        format!("ship.shofixti.graphics.destruct.{size}")
                    );
                }
                assert!(
                    d.ship_data.ship[i].is_null()
                        && d.ship_data.weapon[i].is_null()
                        && d.ship_data.special[i].is_null()
                );
            }
            assert_eq!(
                CStr::from_ptr(d.ship_data.captain_control.captain_rsc),
                if damaged {
                    c"ship.shofixti.graphics.oldcaptain"
                } else {
                    c"ship.shofixti.graphics.captain"
                }
            );
            let cap = d.ship_data.captain_control;
            assert!(
                cap.background.is_null()
                    && cap.turn.is_null()
                    && cap.thrust.is_null()
                    && cap.weapon.is_null()
                    && cap.special.is_null()
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.victory_ditty_rsc),
                c"ship.shofixti.ditty"
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_sounds_rsc),
                c"ship.shofixti.sounds"
            );
            assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
            assert_eq!(
                (
                    d.cyborg_control.ManeuverabilityIndex,
                    d.cyborg_control.WeaponRange
                ),
                (0, 960)
            );
            assert!(d.callbacks()[4] != 0 && d.callbacks()[2] != 0 && d.callbacks()[3] != 0);
            assert!(
                d.callbacks()[1] == 0
                    && d.callbacks()[0] == 0
                    && d.data.is_null()
                    && d.code_ref.is_null()
            );
            (*p).characteristics.max_thrust = 999;
            (*p).ship_info.crew_level = 0;
            (*p).postprocess_func = None;
            let fresh = shared::descriptor(SPECIES_ID_SHOFIXTI_ID);
            assert_eq!(fresh.ship_info.crew_level, 6);
            assert_ne!(fresh.callbacks()[2], 0);
        }
    })
}

#[test]
fn darts_use_next_location_exact_facing_offset_speed_and_failure_count() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(2);
        let mut ship = Ship::new(&_battle, 1);
        for facing in [0, 4, 7, 15, u16::MAX] {
            ship.inner.ship_mut().ShipFacing = facing;
            (*ship.ptr()).next.location = POINT {
                x: i16::MAX - 5,
                y: i16::MIN + 5,
            };
            let mut out = ptr::null_mut();
            with_rng(31, 0, || {
                assert_eq!(ship.fire(std::slice::from_mut(&mut out)), 1)
            });
            assert!(!out.is_null());
            let dart = element(out);
            assert_eq!(
                (*dart).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!(
                (
                    (*dart).__bindgen_anon_1.life_span,
                    (*dart).__bindgen_anon_2.hit_points,
                    (*dart).__bindgen_anon_3.mass_points,
                    (*dart).__bindgen_anon_5.blast_offset
                ),
                (10, 1, 1, 1)
            );
            assert_eq!((*dart).playerNr, 1);
            assert_eq!(frame_index((*dart).current.image.frame), facing % 16);
            assert_eq!(
                (*dart).current.image.frame,
                ship.weapon_frames.frame((facing % 16) as usize)
            );
            assert_eq!(
                (*dart).current.image.farray,
                (&raw mut ship.inner.descriptor_mut().ship_data.weapon).cast()
            );
            assert!(
                (*dart).preprocess_func.is_none()
                    && (*dart).postprocess_func.is_none()
                    && (*dart).death_func.is_none()
                    && (*dart).collision_func.is_some()
            );
            assert!((*dart).pParent.is_null());
            let angle = facing_to_angle(facing as i32) as COUNT;
            let vx = cosine(angle as i32, 96 << VELOCITY_SHIFT);
            let vy = sine(angle as i32, 96 << VELOCITY_SHIFT);
            let (mut x, mut y) = (0, 0);
            GetCurrentVelocityComponents(&raw mut (*dart).velocity, &mut x, &mut y);
            assert_eq!((x, y), (vx, vy));
            let origin_x =
                ((i32::from(i16::MAX) - 5) + i32::from(cosine(angle as i32, 60))) as COORD;
            let origin_y = ((i32::from(i16::MIN) + 5) + i32::from(sine(angle as i32, 60))) as COORD;
            assert_eq!(
                (*dart).current.location.x,
                origin_x.wrapping_sub(vx >> VELOCITY_SHIFT)
            );
            assert_eq!(
                (*dart).current.location.y,
                origin_y.wrapping_sub(vy >> VELOCITY_SHIFT)
            );
            let mut failure = 1usize as HELEMENT;
            with_rng(31, 0, || {
                assert_eq!(ship.fire(std::slice::from_mut(&mut failure)), 1)
            });
            assert!(failure.is_null());
            FreeElement(out);
        }
    })
}

#[test]
fn glory_ai_toggles_the_switch_and_checks_all_deterministic_threat_branches() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship = Ship::new(&_battle, 0);
        let enemy = Ship::new(&_battle, 1);
        let mut concerns: [EVALUATE_DESC; 5] = mem::zeroed();
        for (remaining, enemy_turn, immediate, crew, special_available, start, counter, expected) in [
            (6, 5, false, 6, true, false, 0, true),
            (6, 5, false, 6, true, true, 0, false),
            (6, 5, false, 6, true, true, 1, true),
            (6, 5, false, 6, true, false, 1, false),
            (5, 4, false, 6, true, false, 0, true),
            (5, 5, false, 6, true, false, 0, false),
            (5, 5, true, 1, true, false, 0, true),
            (5, 5, true, 2, true, false, 0, false),
            (6, 4, true, 1, false, false, 0, false),
        ] {
            ship.inner
                .descriptor_mut()
                .ship_data
                .captain_control
                .special = ship.captain.frame(20 - remaining);
            ship.inner.descriptor_mut().ship_data.special[0] = if special_available {
                ship.special_frames.frame(0)
            } else {
                ptr::null_mut()
            };
            ship.inner.ship_mut().ship_input_state =
                WEAPON as u8 | if start { SPECIAL as u8 } else { 0 };
            ship.inner.ship_mut().special_counter = counter;
            (*ship.ptr()).__bindgen_anon_2.crew_level = crew;
            (*enemy.ptr()).current.location = POINT { x: 6000, y: 6000 };
            (*enemy.ptr()).__bindgen_anon_3.mass_points = 0;
            concerns[0].ObjectPtr = enemy.ptr();
            concerns[0].which_turn = enemy_turn;
            concerns[2].ObjectPtr = if immediate {
                enemy.ptr()
            } else {
                ptr::null_mut()
            };
            with_rng(88, 0, || ship.ai(&mut concerns));
            assert_eq!(
                ship.inner.ship_mut().ship_input_state & WEAPON as u8,
                0,
                "shared AI runs even during special cooldown"
            );
            assert_eq!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8 != 0, expected, "remaining {remaining}, turn {enemy_turn}, immediate {immediate}, crew {crew}, start {start}, counter {counter}");
        }
    })
}

#[test]
fn glory_ai_intercept_damage_and_random_parity_short_circuit_exactly() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship = Ship::new(&_battle, 0);
        let mut threat = Ship::new(&_battle, 1);
        ship.ship_frames.make_opaque();
        threat.ship_frames.make_opaque();
        _GraphicsStatusFlags |= 4;
        ship.inner
            .descriptor_mut()
            .ship_data
            .captain_control
            .special = ship.captain.frame(15);
        (*threat.ptr()).state_flags = 0;
        assert_ne!(
            PlotIntercept(threat.ptr(), ship.ptr(), 2, 0),
            0,
            "opaque overlapping frames must intercept"
        );
        let mut concerns: [EVALUATE_DESC; 5] = mem::zeroed();
        concerns[2].ObjectPtr = threat.ptr();
        for seed in 1..32 {
            TFB_SeedRandom(seed);
            let parity = TFB_Random() & 1 != 0;
            (*threat.ptr()).__bindgen_anon_3.mass_points = 6;
            ship.inner.ship_mut().ship_input_state = 0;
            with_rng(seed, 1, || ship.ai(&mut concerns));
            assert_eq!(
                ship.inner.ship_mut().ship_input_state & SPECIAL as u8 != 0,
                parity
            );
            (*threat.ptr()).__bindgen_anon_3.mass_points = 5;
            ship.inner.ship_mut().ship_input_state = 0;
            with_rng(seed, 0, || ship.ai(&mut concerns));
            assert_eq!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8, 0);
        }
        (*threat.ptr()).current.location.x += 500;
        (*threat.ptr()).__bindgen_anon_3.mass_points = 255;
        assert_eq!(PlotIntercept(threat.ptr(), ship.ptr(), 2, 0), 0);
        with_rng(55, 0, || ship.ai(&mut concerns));
        assert_eq!(ship.inner.ship_mut().ship_input_state & SPECIAL as u8, 0);
    })
}

#[test]
fn both_switch_edges_advance_and_only_the_third_remaining_frame_detonates() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(16);
        let mut ship = Ship::new(&_battle, 0);

        for (old, current, index) in [
            (0, 0, 14),
            (0, SPECIAL as u16, 15),
            (SPECIAL as u16, SPECIAL as u16, 15),
            (SPECIAL as u16, 0, 16),
            (0, SPECIAL as u16, 17),
        ] {
            ship.inner.ship_mut().old_status_flags = old;
            ship.inner.ship_mut().cur_status_flags = current;
            with_rng(11, 0, || ship.inner.postprocess());
            assert_eq!(
                frame_index(
                    ship.inner
                        .descriptor_mut()
                        .ship_data
                        .captain_control
                        .special
                ),
                index
            );
            assert_eq!(
                (*ship.ptr()).__bindgen_anon_2.crew_level,
                if index == 17 { 0 } else { 6 }
            );
        }
        let deferred = element(disp_q.tail);
        assert_ne!(deferred, ship.ptr());
        assert_eq!(
            (*deferred).state_flags,
            (APPEARING | NONSOLID | FINITE_LIFE) as u16
        );
        assert_eq!((*deferred).__bindgen_anon_1.life_span, 0);
        assert_eq!((*deferred).pParent, (*ship.ptr()).pParent);
        assert_eq!(
            ((*deferred).next.location.x, (*deferred).next.location.y),
            (4000, 4000)
        );
        assert!((*deferred).hTarget.is_null() && (*deferred).death_func.is_some());
        assert_eq!(ship.inner.descriptor_mut().ship_info.crew_level, 0);
        assert_eq!((*ship.ptr()).__bindgen_anon_1.life_span, 0);
        assert_ne!((*ship.ptr()).state_flags & NONSOLID as u16, 0);
        assert!((*ship.ptr()).death_func.is_some());
    })
}

#[test]
fn deferred_glory_damage_obeys_solidity_marine_identity_radius_and_gravity() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(64);
        let mut ship = Ship::new(&_battle, 0);
        let mut enemy = Ship::new(&_battle, 1);
        enemy.inner.descriptor_mut().ship_info.max_crew = 50;
        enemy.inner.descriptor_mut().ship_info.crew_level = 50;
        (*enemy.ptr()).__bindgen_anon_2.crew_level = 50;
        let mut objects = Vec::new();
        for (flags, mass, dx, dy, expected) in [
            (0, 1, 0, 0, 11),
            (COLLISION, 1, 0, 0, 11),
            (NONSOLID, 1, 0, 0, 30),
            (DISAPPEARING, 1, 0, 0, 30),
            (0, 100, 0, 0, 11),
            (0, 101, 0, 0, 30),
            (0, 1, 180 * 4, 0, 29),
            (0, 1, -(180 * 4 + 3), 0, 29),
            (0, 1, 181 * 4, 0, 30),
            (0, 1, 127 * 4, -127 * 4, 29),
            (0, 1, 128 * 4, 128 * 4, 30),
        ] {
            objects.push((object(&_battle, flags as u16, mass, 30, dx, dy), expected));
        }
        let mut marine_owner = object(&_battle, (NONSOLID | DISAPPEARING) as u16, 1, 1, 0, 0);
        let marine = marine_owner.ptr();
        (*marine).preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Legacy(
            intruder_preprocess,
        ));
        (*marine).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Legacy(
            marine_collision,
        ));
        let mut preprocess_only_owner = object(&_battle, NONSOLID as u16, 1, 1, 0, 0);
        let preprocess_only = preprocess_only_owner.ptr();
        (*preprocess_only).preprocess_func = Some(uqm_rust::engine::battle::ElementHook::Legacy(
            intruder_preprocess,
        ));
        let mut collision_only_owner = object(&_battle, NONSOLID as u16, 1, 1, 0, 0);
        let collision_only = collision_only_owner.ptr();
        (*collision_only).collision_func = Some(uqm_rust::engine::battle::CollisionHook::Legacy(
            marine_collision,
        ));
        let mut fragile_owner = object(&_battle, 0, 1, 19, 0, 0);
        let fragile = fragile_owner.ptr();
        let kill = ship.arm().unwrap();
        let deferred = element(disp_q.tail);
        assert_eq!(
            (*enemy.ptr()).__bindgen_anon_2.crew_level,
            50,
            "damage must be deferred"
        );
        with_rng(2, 0, || {
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(kill, deferred)
        });
        assert_eq!((*enemy.ptr()).__bindgen_anon_2.crew_level, 31);
        assert_eq!(enemy.inner.descriptor_mut().ship_info.crew_level, 31);
        for (object, hp) in &mut objects {
            let obj = object.ptr();
            assert_eq!((*obj).__bindgen_anon_2.hit_points, *hp);
            assert_eq!((*obj).__bindgen_anon_1.life_span, 50);
        }
        for obj in [marine, fragile] {
            assert_eq!((*obj).__bindgen_anon_2.hit_points, 0);
            assert_eq!((*obj).__bindgen_anon_1.life_span, 0);
        }
        for obj in [preprocess_only, collision_only] {
            assert_eq!((*obj).__bindgen_anon_2.hit_points, 1);
        }
        enemy.inner.descriptor_mut().ship_info.crew_level = 1;
        (*enemy.ptr()).__bindgen_anon_2.crew_level = 1;
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(kill, deferred);
        assert_eq!((*enemy.ptr()).__bindgen_anon_1.life_span, 0);
        assert_eq!(enemy.inner.descriptor_mut().ship_info.crew_level, 0);
    })
}

#[test]
fn failed_deferred_allocation_still_kills_the_ship() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(1);
        let mut ship = Ship::new(&_battle, 0);
        assert!(with_rng(4, 0, || ship.arm()).is_none());
        assert_eq!({ disp_q.tail }, ship.handle);
        assert_eq!((*ship.ptr()).__bindgen_anon_1.life_span, 0);
        assert_eq!((*ship.ptr()).__bindgen_anon_2.crew_level, 0);
        assert_eq!(ship.inner.descriptor_mut().ship_info.crew_level, 0);
        assert!((*ship.ptr()).death_func.is_some());
    })
}

#[test]
fn glory_death_selects_the_winner_preserves_prior_winners_and_counts_the_death() {
    battle_context::run(|| unsafe {
        for (other_crew, other_mass, prior, expected_self, ditty) in [
            (6, 1, 0, false, true),
            (0, 1, 0, true, true),
            (0, 11, 0, false, true),
            (6, 12, 0, true, true),
            (6, 1, 1, true, false),
            (6, 1, 2, false, true),
        ] {
            let _battle = Battle::new(8);
            let mut ship = Ship::new(&_battle, 0);
            let mut other = Ship::new(&_battle, 1);
            other.inner.descriptor_mut().ship_info.crew_level = other_crew;
            (*other.ptr()).__bindgen_anon_3.mass_points = other_mass;
            if prior == 1 {
                SetWinnerStarShip(&raw mut *ship.inner.ship_mut());
            }
            if prior == 2 {
                SetWinnerStarShip(&raw mut *other.inner.ship_mut());
            }
            ship.arm();
            let death = (*ship.ptr()).death_func.unwrap();
            with_rng(13, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    death,
                    ship.ptr(),
                )
            });
            let expected = if expected_self {
                &raw mut *ship.inner.ship_mut()
            } else {
                &raw mut *other.inner.ship_mut()
            };
            assert_eq!(GetWinnerStarShip(), expected);
            assert_eq!((*expected).cur_status_flags & 0x200 != 0, ditty);
            assert_eq!(uqm_rust::app::bridge::battle(|b| b.counters[0]), 9);
            assert_eq!((*ship.ptr()).__bindgen_anon_1.life_span, 36);
            assert_eq!(ship.inner.descriptor_mut().ship_info.energy_level, 0);
            assert_eq!((*ship.ptr()).velocity.travel_angle, 0);
            assert!(
                (*ship.ptr()).preprocess_func.is_some()
                    && (*ship.ptr()).postprocess_func.is_some()
                    && (*ship.ptr()).death_func.is_some()
            );
        }
    })
}

#[test]
fn glory_flash_switch_and_explosion_color_sequence_use_twelve_frame_ship_death() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(8);
        let mut ship = Ship::new(&_battle, 0);
        ship.arm();
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
            (*ship.ptr()).death_func.unwrap(),
            ship.ptr(),
        );
        let flash = (*ship.ptr()).preprocess_func.unwrap();
        for (life, color) in [
            (36, (255, 255, 255, 255)),
            (35, (255, 255, 82, 255)),
            (34, (255, 255, 255, 255)),
        ] {
            (*ship.ptr()).__bindgen_anon_1.life_span = life;
            with_rng(3, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    flash,
                    ship.ptr(),
                )
            });
            assert_eq!((*prim(ship.ptr())).Type, gfx_object_STAMPFILL_PRIM as u8);
            assert_eq!(rgba((*prim(ship.ptr())).color), color);
            assert_ne!((*ship.ptr()).state_flags & CHANGING as u16, 0);
        }
        (*ship.ptr()).__bindgen_anon_1.life_span = 33;
        with_rng(3, 0, || {
            uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                flash,
                ship.ptr(),
            )
        });
        assert!((*ship.ptr()).preprocess_func.is_none());
        assert_eq!((*prim(ship.ptr())).Type, gfx_object_NUM_PRIMS as u8);
        let explosion = element(disp_q.tail);
        assert_ne!(explosion, ship.ptr());
        assert_eq!(
            (*explosion).state_flags,
            (APPEARING | FINITE_LIFE | NONSOLID) as u16
        );
        assert_eq!((*explosion).playerNr, -1);
        assert_eq!((*explosion).__bindgen_anon_1.life_span, 10);
        assert_eq!((*explosion).__bindgen_anon_2.hit_points, 0);
        assert_eq!((*explosion).__bindgen_anon_3.mass_points, 0);
        assert_eq!((*explosion).pParent, (*ship.ptr()).pParent);
        assert_eq!(
            (*explosion).current.image.frame,
            ship.special_frames.frame(0)
        );
        assert_eq!(
            (*explosion).current.image.farray,
            (&raw mut ship.inner.descriptor_mut().ship_data.special).cast()
        );
        assert_eq!(
            (
                (*explosion).current.location.x,
                (*explosion).current.location.y
            ),
            (4000, 4000)
        );
        assert_eq!((*prim(explosion)).Type, gfx_object_STAMPFILL_PRIM as u8);
        assert_eq!(rgba((*prim(explosion)).color), (255, 255, 255, 255));
        assert!((*explosion).postprocess_func.is_none() && (*explosion).death_func.is_none());
        let animate = (*explosion).preprocess_func.unwrap();
        for color in [
            (255, 255, 82, 255),
            (255, 82, 82, 255),
            (165, 82, 0, 255),
            (165, 0, 0, 255),
            (165, 0, 0, 255),
        ] {
            (*explosion).next.image.frame = ship.special_frames.frame(7);
            with_rng(3, 0, || {
                uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                    animate, explosion,
                )
            });
            assert_eq!(
                frame_index((*explosion).next.image.frame),
                1,
                "advance current, not next frame"
            );
            assert_eq!(rgba((*prim(explosion)).color), color);
        }
        (*prim(explosion)).color = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 0,
        };
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(animate, explosion);
        assert_eq!(
            rgba((*prim(explosion)).color),
            (255, 255, 255, 0),
            "sameColor includes alpha"
        );
    })
}

#[test]
fn failed_visual_explosion_allocation_still_hides_the_ship() {
    battle_context::run(|| unsafe {
        let _battle = Battle::new(1);
        let mut ship = Ship::new(&_battle, 0);
        ship.arm();
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
            (*ship.ptr()).death_func.unwrap(),
            ship.ptr(),
        );
        (*ship.ptr()).__bindgen_anon_1.life_span = 33;
        uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
            (*ship.ptr()).preprocess_func.unwrap(),
            ship.ptr(),
        );
        assert_eq!({ disp_q.tail }, ship.handle);
        assert_eq!((*prim(ship.ptr())).Type, gfx_object_NUM_PRIMS as u8);
        assert!((*ship.ptr()).preprocess_func.is_none());
    })
}

#[test]
fn glory_mutual_loss_applies_both_rosters_and_selects_both_replacements() {
    battle_context::run(|| {
        use uqm_rust::{
            battle::tactrans_native as transitions,
            pickship::ffi::get_encounter_star_ship_with_picker,
        };
        unsafe extern "C" fn ready(_: *mut InputContext) -> bool {
            true
        }
        unsafe {
            for computer in [false, true] {
                for initial in [[2, 2], [1, 2], [2, 1], [1, 1]] {
                    let _battle = Battle::new(16);
                    let saved_queues = race_q;
                    let saved_inputs = std::array::from_fn::<_, 2, _>(|side| {
                        uqm_rust::battle::battle_native::player_input(side)
                    });
                    let saved_offscreen = OffScreenContext;
                    let saved_controls = PlayerControls;
                    let saved_pulsed = PulsedInputState;
                    let mut context = mem::zeroed();
                    OffScreenContext = &mut context;
                    GlobData.Game_state.CurrentActivity = (SUPER_MELEE | IN_BATTLE) as u16;
                    uqm_rust::app::bridge::battle(|b| b.counters = initial);
                    let mut ship = Ship::new(&_battle, 0);
                    let mut other = Ship::new(&_battle, 1);
                    ship.inner.ship_mut().SpeciesID = SPECIES_ID_SHOFIXTI_ID;
                    other.inner.ship_mut().SpeciesID = SPECIES_ID_SHOFIXTI_ID;
                    let mut reserves = [PoolShip::new(0), PoolShip::new(1)];
                    let mut handlers = BattleInputHandlers {
                        frameInput: None,
                        selectShip: None,
                        battleEndReady: Some(ready),
                        deleteContext: None,
                    };
                    let mut inputs = [InputContext {
                        handlers: &mut handlers,
                        playerNr: 0,
                    }; 2];
                    inputs[1].playerNr = 1;
                    for side in 0..2 {
                        uqm_rust::battle::battle_native::set_player_input(side, &mut inputs[side]);
                        let active = if side == 0 {
                            &raw mut *ship.inner.ship_mut()
                        } else {
                            &raw mut *other.inner.ship_mut()
                        };
                        reserves[side].SpeciesID = SPECIES_ID_SHOFIXTI_ID;
                        reserves[side].index = 1;
                        reserves[side].playerNr = side as SIZE;
                        race_q[side].head = active.cast();
                        race_q[side].tail = active.cast();
                        if initial[side] > 1 {
                            (*active).succ = (&raw mut *reserves[side]).cast();
                            reserves[side].pred = active.cast();
                            race_q[side].tail = (&raw mut *reserves[side]).cast();
                        }
                    }
                    // Arm the real glory device, execute its area damage, then
                    // process both deaths within the same simulated frame.
                    let blast = ship.arm().unwrap();
                    uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                        blast,
                        element(disp_q.tail),
                    );
                    assert_eq!(
                        (
                            ship.inner.descriptor_mut().ship_info.crew_level,
                            other.inner.descriptor_mut().ship_info.crew_level
                        ),
                        (0, 0)
                    );
                    uqm_rust::battle::process_native::invoke_element::invoke_legacy_element(
                        (*ship.ptr()).death_func.unwrap(),
                        ship.ptr(),
                    );
                    core(transitions::ship_death, other.ptr());
                    let remaining = uqm_rust::app::bridge::battle(|b| b.counters);
                    assert_eq!(remaining, [initial[0] - 1, initial[1] - 1]);
                    assert_eq!(
                        (
                            ship.inner.ship_mut().SpeciesID,
                            other.inner.ship_mut().SpeciesID
                        ),
                        (SPECIES_ID_NO_ID, SPECIES_ID_NO_ID)
                    );
                    core(transitions::cleanup_dead_ship, ship.ptr());
                    core(transitions::cleanup_dead_ship, other.ptr());
                    transitions::StopDitty();
                    let expected_selection = initial == [2, 2];
                    let mut selected = [false; 2];
                    for side in 0..2 {
                        let outgoing = if side == 0 { ship.ptr() } else { other.ptr() };
                        // The fixture owns its frame buffers, rather than the
                        // resource loader used by production free_ship.
                        let desc = if side == 0 {
                            ship.inner.descriptor_mut()
                        } else {
                            other.inner.descriptor_mut()
                        };
                        desc.ship_data.ship = [ptr::null_mut(); 3];
                        desc.ship_data.weapon = [ptr::null_mut(); 3];
                        desc.ship_data.special = [ptr::null_mut(); 3];
                        uqm_rust::app::bridge::enter(|cx| {
                            transitions::new_ship_with_picker(cx, outgoing, |last, player| {
                                assert_eq!(player as usize, side);
                                let chosen = get_encounter_star_ship_with_picker(
                                    last.cast(),
                                    player,
                                    |player, out| {
                                        selected[side] = true;
                                        let mut state: GETMELEE_STATE = mem::zeroed();
                                        let p = &mut state.player[player as usize];
                                        p.selecting = 1;
                                        p.ships_left = 1;
                                        p.col = 1;
                                        // Expire the computer's display delay without a sleep.
                                        p.timeIn = u32::MAX - 419;
                                        let result = if computer && side == 1 {
                                            selectShipComputer(
                                                (&mut inputs[side] as *mut InputContext).cast(),
                                                &mut state,
                                            )
                                        } else {
                                            PlayerControls[side] = side as u32;
                                            PulsedInputState.key[side][KEY_WEAPON as usize] = 1;
                                            selectShipHuman(
                                                (&mut inputs[side] as *mut InputContext).cast(),
                                                &mut state,
                                            )
                                        };
                                        assert_eq!(result, 1);
                                        assert_eq!(state.player[side].done, 1);
                                        assert_eq!(
                                            state.player[side].hBattleShip,
                                            (&raw mut *reserves[side]).cast()
                                        );
                                        *out = state.player[side].hBattleShip;
                                        1
                                    },
                                );
                                assert_eq!(!chosen.is_null(), expected_selection);
                                (!chosen.is_null()) as BOOLEAN
                            })
                        });
                    }
                    assert_eq!(selected, [expected_selection; 2]);
                    let remaining = uqm_rust::app::bridge::battle(|b| b.counters);
                    assert_eq!(remaining, [initial[0] - 1, initial[1] - 1]);
                    assert_eq!(GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16, 0);
                    assert_eq!(
                        GlobData.Game_state.CurrentActivity & IN_BATTLE as u16 != 0,
                        expected_selection
                    );
                    race_q = saved_queues;
                    for (side, input) in saved_inputs.into_iter().enumerate() {
                        uqm_rust::battle::battle_native::set_player_input(side, input);
                    }
                    OffScreenContext = saved_offscreen;
                    PlayerControls = saved_controls;
                    PulsedInputState = saved_pulsed;
                }
            }
        }
    })
}
