//! Live Umgah callbacks, private cone state, collision cleanup and AI RNG order.
#![cfg(feature = "linked_c_archive")]

use std::{
    ffi::{c_void, CStr},
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{
    abi::*, cosine, element, facing_to_angle, frame_index, put_element, sine,
};

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    fn init_umgah() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> i32;
    fn UninitQueue(queue: *mut QUEUE) -> i32;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    static mut DisplayArray: [PRIMITIVE; 330];
}

static ENGINE: Mutex<()> = Mutex::new(());

struct Battle {
    _guard: MutexGuard<'static, ()>,
    display: *mut PRIMITIVE,
    width: i32,
    height: i32,
}

impl Battle {
    fn new(capacity: COUNT) -> Self {
        let guard = ENGINE.lock().unwrap();
        unsafe {
            let display = GlobData.Game_state.DisplayArray;
            let width = ScreenWidth;
            let height = ScreenHeight;
            assert_ne!(
                InitQueue(&raw mut disp_q, capacity, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                _guard: guard,
                display,
                width,
                height,
            }
        }
    }
}

impl Drop for Battle {
    fn drop(&mut self) {
        unsafe {
            UninitQueue(&raw mut disp_q);
            GlobData.Game_state.DisplayArray = self.display;
            ScreenWidth = self.width;
            ScreenHeight = self.height;
        }
    }
}

// FRAME_DESC and DRAWABLE_DESC layouts from drawable.h.
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
        }
    }

    fn frame(&self, index: usize) -> FRAME {
        (&raw const self.frames[index]).cast_mut().cast()
    }
}

struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    handle: HELEMENT,
    _ship_frames: FrameSet,
    cone_frames: FrameSet,
}

impl Ship {
    unsafe fn new(player: SIZE) -> Self {
        // GetCodeResData also copies the init result before installing resources.
        let mut desc = Box::new(*init_umgah());
        let ship_frames = FrameSet::new(16);
        let cone_frames = FrameSet::new(48);
        desc.ship_data.ship = [ship_frames.frame(0); 3];
        desc.ship_data.special = [cone_frames.frame(0); 3];
        let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
        ss.RaceDescPtr = &raw mut *desc;
        ss.playerNr = player;
        let handle = AllocElement();
        assert!(!handle.is_null());
        let e = element(handle);
        (*e).playerNr = player;
        (*e).pParent = (&raw mut *ss).cast();
        (*e).state_flags = PLAYER_SHIP as u16;
        (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
        (*e).__bindgen_anon_2.crew_level = 10;
        (*e).__bindgen_anon_3.mass_points = 1;
        (*e).current.location = POINT { x: 4000, y: 4000 };
        (*e).current.image.frame = ship_frames.frame(0);
        (*e).current.image.farray = (&raw mut desc.ship_data.ship).cast();
        (*e).next = (*e).current;
        (*prim(e)).Type = gfx_object_STAMP_PRIM as u8;
        ss.hShip = handle;
        put_element(handle);
        Self {
            desc,
            ss,
            handle,
            _ship_frames: ship_frames,
            cone_frames,
        }
    }

    fn ptr(&self) -> *mut ELEMENT {
        self.handle.cast()
    }

    unsafe fn fire(&mut self) -> HELEMENT {
        let mut h = ptr::null_mut();
        assert_eq!(self.desc.init_weapon_func.unwrap()(self.ptr(), &mut h), 1);
        if !h.is_null() {
            // ship_postprocess attaches ownership after the initializer returns.
            assert!((*element(h)).pParent.is_null());
            (*element(h)).pParent = (&raw mut *self.ss).cast();
        }
        h
    }

    unsafe fn preprocess(&self) {
        self.desc.preprocess_func.unwrap()(self.ptr());
    }
    unsafe fn postprocess(&self) {
        self.desc.postprocess_func.unwrap()(self.ptr());
    }
}

impl Drop for Ship {
    fn drop(&mut self) {
        unsafe {
            self.desc.uninit_func.unwrap()(&raw mut *self.desc);
        }
    }
}

unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
    (&raw mut DisplayArray)
        .cast::<PRIMITIVE>()
        .add((*e).PrimIndex as usize)
}

unsafe fn queued() -> Vec<HELEMENT> {
    let mut result = Vec::new();
    let mut h = disp_q.head;
    while !h.is_null() {
        result.push(h);
        assert!(result.len() <= 64, "display queue cycle");
        h = (*element(h)).succ;
    }
    result
}

fn velocity_fields(v: &VELOCITY_DESC) -> [i32; 9] {
    [
        v.TravelAngle.into(),
        v.vector.width.into(),
        v.vector.height.into(),
        v.fract.width.into(),
        v.fract.height.into(),
        v.error.width.into(),
        v.error.height.into(),
        v.incr.width.into(),
        v.incr.height.into(),
    ]
}

unsafe fn velocity(e: *mut ELEMENT) -> (SIZE, SIZE) {
    let (mut x, mut y) = (0, 0);
    GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut x, &mut y);
    (x, y)
}

unsafe fn draws(seed: DWORD, n: usize) -> Vec<DWORD> {
    TFB_SeedRandom(seed);
    (0..n).map(|_| TFB_Random()).collect()
}

unsafe fn seed_where(accept: impl Fn(&[DWORD]) -> bool) -> DWORD {
    (1..100_000).find(|&seed| accept(&draws(seed, 3))).unwrap()
}

unsafe fn with_rng<T>(seed: DWORD, used: usize, action: impl FnOnce() -> T) -> T {
    let sequence = draws(seed, used + 1);
    TFB_SeedRandom(seed);
    let result = action();
    assert_eq!(
        TFB_Random(),
        sequence[used],
        "RNG consumption: {used}, seed {seed}"
    );
    result
}

fn concerns() -> [EVALUATE_DESC; 4] {
    [EVALUATE_DESC {
        ObjectPtr: ptr::null_mut(),
        facing: 0,
        which_turn: 0,
        MoveState: MOVEMENT_STATE_NO_MOVEMENT,
    }; 4]
}

unsafe fn enemy_concern(enemy: &Ship, turn: COUNT) -> [EVALUATE_DESC; 4] {
    let mut c = concerns();
    c[0].ObjectPtr = enemy.ptr();
    c[0].which_turn = turn;
    c[0].MoveState = MOVEMENT_STATE_PURSUE;
    c
}

unsafe fn think(ship: &mut Ship, c: &mut [EVALUATE_DESC; 4], input: u8, count: COUNT) -> u8 {
    ship.ss.ship_input_state = input;
    ship.desc.cyborg_control.intelligence_func.unwrap()(ship.ptr(), c.as_mut_ptr(), count);
    ship.ss.ship_input_state
}

#[test]
fn descriptor_matches_all_c_initializer_fields_and_resources() {
    let _battle = Battle::new(8);
    unsafe {
        let d = *init_umgah();
        assert_eq!(
            d.ship_info.ship_flags,
            (FIRES_FORE | IMMEDIATE_WEAPON) as u16
        );
        assert_eq!(d.ship_info.ship_cost, 7);
        assert_eq!((d.ship_info.crew_level, d.ship_info.max_crew), (10, 10));
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (30, 30));
        assert_eq!(
            CStr::from_ptr(d.ship_info.race_strings_rsc),
            c"ship.umgah.text"
        );
        assert_eq!(CStr::from_ptr(d.ship_info.icons_rsc), c"ship.umgah.icons");
        assert_eq!(
            CStr::from_ptr(d.ship_info.melee_icon_rsc),
            c"ship.umgah.meleeicons"
        );
        assert!(
            d.ship_info.race_strings.is_null()
                && d.ship_info.icons.is_null()
                && d.ship_info.melee_icon.is_null()
        );
        assert_eq!(d.fleet.strength, 150);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (1798, 6000));
        let c = d.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (18, 6, 30)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (0, 1, 150)
        );
        assert_eq!(
            (
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (4, 3, 0, 2, 1)
        );
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                CStr::from_ptr(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.umgah.graphics.drone.{size}")
            );
            assert_eq!(
                CStr::from_ptr(d.ship_data.special_rsc[i]).to_str().unwrap(),
                format!("ship.umgah.graphics.cone.{size}")
            );
            assert!(
                d.ship_data.ship[i].is_null()
                    && d.ship_data.weapon[i].is_null()
                    && d.ship_data.special[i].is_null()
            );
        }
        assert_eq!(
            CStr::from_ptr(d.ship_data.weapon_rsc[0]),
            c"ship.umgah.graphics.spritz"
        );
        assert!(d.ship_data.weapon_rsc[1].is_null() && d.ship_data.weapon_rsc[2].is_null());
        let captain = d.ship_data.captain_control;
        assert_eq!(
            CStr::from_ptr(captain.captain_rsc),
            c"ship.umgah.graphics.captain"
        );
        assert!(
            captain.background.is_null()
                && captain.turn.is_null()
                && captain.thrust.is_null()
                && captain.weapon.is_null()
                && captain.special.is_null()
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.victory_ditty_rsc),
            c"ship.umgah.ditty"
        );
        assert_eq!(
            CStr::from_ptr(d.ship_data.ship_sounds_rsc),
            c"ship.umgah.sounds"
        );
        assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
        assert_eq!(
            (
                d.cyborg_control.ManeuverabilityIndex,
                d.cyborg_control.WeaponRange
            ),
            (0, 16000)
        );
        assert!(
            d.preprocess_func.is_some()
                && d.postprocess_func.is_some()
                && d.init_weapon_func.is_some()
                && d.uninit_func.is_some()
                && d.cyborg_control.intelligence_func.is_some()
        );
        assert!(d.data.is_null() && d.CodeRef.is_null());
    }
}

#[test]
fn per_init_descriptor_copies_isolate_energy_resources_animation_and_custom_data() {
    let _battle = Battle::new(8);
    unsafe {
        let mut first = Ship::new(0);
        first.ss.ShipFacing = 5;
        let h = first.fire();
        let data = first.desc.data;
        assert!(!data.is_null());
        ((*element(h)).preprocess_func.unwrap())(element(h));
        assert_eq!(frame_index(first.desc.ship_data.special[0]), 21);
        first.desc.ship_info.energy_level = 1;
        first.desc.characteristics.special_wait = 77;
        let mut second = Ship::new(1);
        assert_ne!(&raw mut *first.desc, &raw mut *second.desc);
        assert!(second.desc.data.is_null());
        assert_eq!(second.desc.ship_info.energy_level, 30);
        assert_eq!(second.desc.characteristics.special_wait, 2);
        assert_eq!(frame_index(second.desc.ship_data.special[0]), 0);
        second.ss.ShipFacing = 9;
        let other = second.fire();
        assert_ne!(first.desc.data, second.desc.data);
        assert_eq!(first.desc.data, data);
        assert_eq!(frame_index(first.desc.ship_data.special[0]), 21);
        first.desc.uninit_func.unwrap()(&raw mut *first.desc);
        assert!(first.desc.data.is_null());
        assert!(!second.desc.data.is_null());
        ((*element(other)).preprocess_func.unwrap())(element(other));
        assert_eq!(frame_index(second.desc.ship_data.special[0]), 25);
        FreeElement(h);
        FreeElement(other);
        let template = init_umgah();
        (*template).ship_info.energy_level = 0;
        (*template).characteristics.special_wait = 0;
        (*template).preprocess_func = None;
        let fresh = init_umgah();
        assert_eq!((*fresh).ship_info.energy_level, 30);
        assert_eq!((*fresh).characteristics.special_wait, 2);
        assert!((*fresh).preprocess_func.is_some());
        assert!((*fresh).data.is_null());
    }
}

#[test]
fn cone_initialization_fixes_intersection_and_does_not_inherit_velocity() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(1);
        ship.ss.ShipFacing = 5;
        (*ship.ptr()).next.location = POINT { x: -9, y: 13 };
        SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 321, -654);
        let h = with_rng(8, 0, || ship.fire());
        assert!(!h.is_null());
        let cone = element(h);
        assert_eq!((*cone).state_flags, (FINITE_LIFE | IGNORE_SIMILAR) as u16);
        assert_eq!((*cone).__bindgen_anon_1.life_span, 1);
        assert_eq!((*cone).__bindgen_anon_2.hit_points, 100);
        assert_eq!((*cone).__bindgen_anon_3.mass_points, 1);
        assert_eq!((*cone).__bindgen_anon_5.blast_offset, 0);
        assert_eq!((*cone).playerNr, 1);
        assert_eq!(velocity_fields(&(*cone).velocity), [0; 9]);
        assert_eq!(
            ((*cone).current.location.x, (*cone).current.location.y),
            (-9, 13)
        );
        assert_eq!(((*cone).next.location.x, (*cone).next.location.y), (-9, 13));
        assert_eq!((*cone).current.image.frame, ship.cone_frames.frame(5));
        assert_eq!((*cone).next.image.frame, (*cone).current.image.frame);
        assert_eq!(
            (*cone).next.image.farray,
            (&raw mut ship.desc.ship_data.special).cast()
        );
        let i = (*cone).IntersectControl;
        assert_eq!(
            (i.IntersectStamp.origin.x, i.IntersectStamp.origin.y),
            (-3, 3)
        );
        assert_eq!((i.EndPoint.x, i.EndPoint.y), (-3, 3));
        assert_eq!(i.IntersectStamp.frame, ship.cone_frames.frame(5));
        assert_eq!((*prim(cone)).Type, gfx_object_STAMP_PRIM as u8);
        assert!((*cone).preprocess_func.is_some() && (*cone).collision_func.is_some());
        assert!((*cone).postprocess_func.is_none() && (*cone).death_func.is_none());
        assert_eq!(queued(), vec![ship.handle]);
        FreeElement(h);
    }
}

#[test]
fn held_cone_advances_by_16_preserves_phase_and_resets_only_when_facing_changes() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 5;
        for index in [5, 21, 37, 5] {
            let h = with_rng(3, 0, || ship.fire());
            let cone = element(h);
            let data = ship.desc.data;
            assert!(!data.is_null());
            assert_eq!(*data.cast::<u16>(), 5);
            assert_eq!(frame_index((*cone).current.image.frame), index);
            let old_frame = (*cone).next.image.frame;
            with_rng(3, 0, || ((*cone).preprocess_func.unwrap())(cone));
            assert_eq!(
                frame_index(ship.desc.ship_data.special[0]),
                (index + 16) % 48
            );
            assert_eq!((*cone).next.image.frame, old_frame);
            assert_eq!(
                (*cone).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            let next = ship.fire();
            assert_eq!(ship.desc.data, data, "holding facing keeps the allocation");
            assert_eq!(
                frame_index((*element(next)).current.image.frame),
                (index + 16) % 48
            );
            FreeElement(next);
            FreeElement(h);
        }
        ship.ss.ShipFacing = 9;
        let h = ship.fire();
        assert_eq!(*ship.desc.data.cast::<u16>(), 9);
        assert_eq!(frame_index((*element(h)).current.image.frame), 9);
        assert_eq!(
            (*element(h)).IntersectControl.IntersectStamp.frame,
            ship.cone_frames.frame(9)
        );
        FreeElement(h);
    }
}

#[test]
fn appearance_and_uninit_free_only_the_instance_custom_data_and_are_repeatable() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.fire();
        FreeElement(h);
        assert!(!ship.desc.data.is_null());
        SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 123, -456);
        let before = velocity_fields(&(*ship.ptr()).velocity);
        ship.ss.cur_status_flags = SPECIAL as u16;
        ship.ss.special_counter = 9;
        (*ship.ptr()).state_flags |= APPEARING as u16;
        with_rng(4, 0, || ship.preprocess());
        assert!(ship.desc.data.is_null());
        assert_eq!(ship.desc.ship_info.energy_level, 30);
        assert_eq!(ship.ss.special_counter, 9);
        assert_eq!(velocity_fields(&(*ship.ptr()).velocity), before);
        ship.preprocess();
        let h = ship.fire();
        FreeElement(h);
        assert!(!ship.desc.data.is_null());
        let frame = ship.desc.ship_data.special[0];
        ship.desc.uninit_func.unwrap()(&raw mut *ship.desc);
        ship.desc.uninit_func.unwrap()(&raw mut *ship.desc);
        assert!(ship.desc.data.is_null());
        assert_eq!(ship.desc.ship_data.special[0], frame);
    }
}

#[test]
fn failed_cone_allocation_still_updates_private_facing_and_returns_one_slot() {
    let _battle = Battle::new(1);
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.ShipFacing = 15;
        assert!(with_rng(5, 0, || ship.fire()).is_null());
        assert!(!ship.desc.data.is_null());
        assert_eq!(*ship.desc.data.cast::<u16>(), 15);
        assert_eq!(frame_index(ship.desc.ship_data.special[0]), 15);
        assert_eq!(ship.desc.ship_info.energy_level, 30);
        ship.ss.ShipFacing = 4;
        assert!(ship.fire().is_null());
        assert_eq!(*ship.desc.data.cast::<u16>(), 4);
        assert_eq!(frame_index(ship.desc.ship_data.special[0]), 4);
        assert_eq!(queued(), vec![ship.handle]);
    }
}

unsafe fn target(finite: bool) -> ELEMENT {
    let mut e: ELEMENT = mem::zeroed();
    e.playerNr = 1;
    e.state_flags = if finite { FINITE_LIFE as u16 } else { 0 };
    e.__bindgen_anon_1.life_span = 2;
    e.__bindgen_anon_2.hit_points = 3;
    e.__bindgen_anon_3.mass_points = 1;
    e
}

#[test]
fn collision_removes_and_frees_the_blast_but_does_not_restore_cone_life_or_solidity() {
    let _battle = Battle::new(3);
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.fire();
        let cone = element(h);
        let mut hit = target(false);
        let mut a = POINT { x: 10, y: 20 };
        let mut b = POINT { x: 30, y: 40 };
        let before = queued();
        with_rng(1, 0, || {
            ((*cone).collision_func.unwrap())(cone, &mut a, &mut hit, &mut b)
        });
        assert_eq!(queued(), before, "the generic collision blast was unlinked");
        assert_eq!(
            (*cone).state_flags,
            (FINITE_LIFE | IGNORE_SIMILAR | COLLISION | NONSOLID) as u16
        );
        assert_eq!((*cone).__bindgen_anon_1.life_span, 0);
        assert_eq!((*cone).__bindgen_anon_2.hit_points, 0);
        assert_eq!(hit.__bindgen_anon_2.hit_points, 3);
        let freed_blast = AllocElement();
        assert!(
            !freed_blast.is_null(),
            "blast element returned to the queue free list"
        );
        assert!(AllocElement().is_null());
        FreeElement(freed_blast);
        FreeElement(h);
    }
}

#[test]
fn collision_preserves_disappearing_when_blast_allocation_fails() {
    let _battle = Battle::new(2);
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.fire();
        let cone = element(h);
        let mut hit = target(false);
        let mut p = POINT { x: 0, y: 0 };
        ((*cone).collision_func.unwrap())(cone, &mut p, &mut hit, ptr::null_mut());
        assert_eq!(
            (*cone).state_flags & (DISAPPEARING | COLLISION | NONSOLID) as u16,
            (DISAPPEARING | COLLISION | NONSOLID) as u16
        );
        assert_eq!((*cone).__bindgen_anon_1.life_span, 0);
        assert_eq!(queued(), vec![ship.handle]);
        FreeElement(h);
    }
}

#[test]
fn finite_weapon_hit_damages_once_without_a_blast_and_does_not_clear_disappearing() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let h = ship.fire();
        let cone = element(h);
        let mut hit = target(true);
        (*cone).state_flags |= DISAPPEARING as u16;
        let collide = (*cone).collision_func.unwrap();
        with_rng(7, 0, || {
            collide(cone, ptr::null_mut(), &mut hit, ptr::null_mut())
        });
        assert_eq!(hit.__bindgen_anon_2.hit_points, 2);
        assert_eq!((*cone).__bindgen_anon_2.hit_points, 100);
        assert_eq!((*cone).__bindgen_anon_1.life_span, 1);
        assert_ne!((*cone).state_flags & COLLISION as u16, 0);
        assert_ne!((*cone).state_flags & DISAPPEARING as u16, 0);
        collide(cone, ptr::null_mut(), &mut hit, ptr::null_mut());
        assert_eq!(hit.__bindgen_anon_2.hit_points, 2);
        assert_eq!(queued(), vec![ship.handle]);
        FreeElement(h);
    }
}

#[test]
fn retropropulsion_adds_the_exact_reverse_vector_for_every_facing_then_stops_all_velocity_fields() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        for facing in 0..16 {
            ship.ss.ShipFacing = facing;
            ship.ss.cur_status_flags = (SPECIAL
                | SHIP_AT_MAX_SPEED
                | SHIP_BEYOND_MAX_SPEED
                | SHIP_IN_GRAVITY_WELL
                | WEAPON) as u16;
            ship.ss.special_counter = 17;
            ship.desc.ship_info.energy_level = 30;
            SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 123, -456);
            let mut expected = (*ship.ptr()).velocity;
            let a = facing_to_angle(i32::from(facing)) + 32;
            DeltaVelocityComponents(&mut expected, cosine(a, 160 * 32), sine(a, 160 * 32));
            with_rng(8, 0, || ship.preprocess());
            assert_eq!(
                velocity_fields(&(*ship.ptr()).velocity),
                velocity_fields(&expected)
            );
            assert_eq!(ship.desc.ship_info.energy_level, 29);
            assert_eq!(ship.ss.energy_counter, 150);
            assert_eq!(
                ship.ss.special_counter, 2,
                "special counter does not gate retropropulsion"
            );
            assert_eq!(
                ship.ss.cur_status_flags,
                (SPECIAL | SHIP_IN_GRAVITY_WELL | WEAPON) as u16
            );
            with_rng(8, 0, || ship.postprocess());
            assert_eq!(ship.ss.special_counter, 0);
            assert_eq!(velocity_fields(&(*ship.ptr()).velocity), [0; 9]);
        }
        ship.ss.ShipFacing = 0;
        ship.ss.cur_status_flags = SPECIAL as u16;
        ship.preprocess();
        assert_eq!(velocity(ship.ptr()), (0, 160 * 32));
    }
}

#[test]
fn retropropulsion_gates_on_appearance_thrust_wait_key_and_energy() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        for (appearing, wait, key, energy, low) in [
            (true, 0, true, 30, false),
            (false, 1, true, 30, false),
            (false, 255, true, 0, false),
            (false, 0, false, 30, false),
            (false, 0, true, 0, true),
        ] {
            (*ship.ptr()).state_flags =
                (PLAYER_SHIP | if appearing { APPEARING } else { 0 }) as u16;
            (*ship.ptr()).__bindgen_anon_5.thrust_wait = wait;
            ship.ss.cur_status_flags = if key { SPECIAL as u16 } else { 0 };
            ship.ss.special_counter = 0;
            ship.ss.energy_counter = 19;
            ship.desc.ship_info.energy_level = energy;
            SetVelocityComponents(&raw mut (*ship.ptr()).velocity, 333, -555);
            let before = velocity_fields(&(*ship.ptr()).velocity);
            with_rng(4, 0, || ship.preprocess());
            assert_eq!(velocity_fields(&(*ship.ptr()).velocity), before);
            assert_eq!(ship.desc.ship_info.energy_level, energy);
            assert_eq!(ship.ss.special_counter, 0);
            assert_eq!(ship.ss.energy_counter, 19);
            assert_eq!(ship.ss.cur_status_flags & (1 << 5) != 0, low);
            ship.postprocess();
            assert_eq!(
                velocity_fields(&(*ship.ptr()).velocity),
                before,
                "counter zero coasts unchanged"
            );
        }
        (*ship.ptr()).state_flags = PLAYER_SHIP as u16;
        (*ship.ptr()).__bindgen_anon_5.thrust_wait = 0;
        ship.desc.ship_info.energy_level = 1;
        ship.preprocess();
        assert_eq!(ship.desc.ship_info.energy_level, 0);
        assert_eq!(ship.ss.special_counter, 2);
        assert_eq!(ship.ss.cur_status_flags & (1 << 5), 0);
    }
}

#[test]
fn intelligence_filters_enticing_weapons_before_the_shared_pass() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let threat = Ship::new(1);
        for (turn, old, flags, keep, movement) in [
            (4, 0, FINITE_LIFE, false, MOVEMENT_STATE_ENTICE),
            (3, SPECIAL, FINITE_LIFE, false, MOVEMENT_STATE_ENTICE),
            (3, 0, FINITE_LIFE, true, MOVEMENT_STATE_AVOID),
            (3, 0, FINITE_LIFE | CREW_OBJECT, true, MOVEMENT_STATE_PURSUE),
            (1, 0, 0, true, MOVEMENT_STATE_PURSUE),
        ] {
            ship.ss.old_status_flags = old as u16;
            (*threat.ptr()).state_flags = flags as u16;
            let mut c = concerns();
            let w = &mut c[ENEMY_WEAPON_INDEX as usize];
            w.ObjectPtr = threat.ptr();
            w.which_turn = turn;
            w.MoveState = MOVEMENT_STATE_ENTICE;
            with_rng(7, 0, || think(&mut ship, &mut c, SPECIAL as u8, 0));
            assert_eq!(!c[ENEMY_WEAPON_INDEX as usize].ObjectPtr.is_null(), keep);
            assert_eq!(c[ENEMY_WEAPON_INDEX as usize].MoveState, movement);
        }
        let mut c = concerns();
        c[ENEMY_WEAPON_INDEX as usize].ObjectPtr = threat.ptr();
        c[ENEMY_WEAPON_INDEX as usize].MoveState = MOVEMENT_STATE_AVOID;
        with_rng(7, 0, || think(&mut ship, &mut c, 0, 0));
        assert_eq!(c[ENEMY_WEAPON_INDEX as usize].ObjectPtr, threat.ptr());
        assert_eq!(
            c[ENEMY_WEAPON_INDEX as usize].MoveState,
            MOVEMENT_STATE_AVOID
        );
    }
}

#[test]
fn short_range_ai_resets_special_wait_and_fires_only_before_turn_16() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        for reason in 0..3 {
            for turn in [15, 16] {
                ship.ss.special_counter = if reason == 0 { 1 } else { 0 };
                let mut c = enemy_concern(&enemy, turn);
                if reason == 1 {
                    c[GRAVITY_MASS_INDEX as usize].ObjectPtr = enemy.ptr();
                }
                if reason == 2 {
                    c[0].ObjectPtr = ptr::null_mut();
                }
                let input = with_rng(4, 0, || {
                    think(
                        &mut ship,
                        &mut c,
                        (SPECIAL | THRUST | LEFT | WEAPON) as u8,
                        0,
                    )
                });
                assert_eq!(
                    input,
                    (THRUST | LEFT | if turn < 16 { WEAPON } else { 0 }) as u8
                );
                assert_eq!(ship.desc.cyborg_control.WeaponRange, 200);
                assert_eq!(ship.desc.characteristics.special_wait, 255);
            }
        }
    }
}

#[test]
fn rearward_zip_uses_one_draw_with_both_random_turn_directions() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        (*enemy.ptr()).next.location = POINT { x: 4000, y: 4400 };
        for odd in [false, true] {
            let seed = seed_where(|d| (d[0] & 1 != 0) == odd);
            let mut c = enemy_concern(&enemy, 8);
            let input = with_rng(seed, 1, || think(&mut ship, &mut c, THRUST as u8, 0));
            assert_eq!(input, (SPECIAL | if odd { LEFT } else { RIGHT }) as u8);
            assert_eq!(ship.desc.characteristics.special_wait, 8);
            assert_eq!(ship.desc.cyborg_control.WeaponRange, 32000);
        }
    }
}

#[test]
fn ai_rng_follows_shared_steering_and_uses_old_turn_flags_for_alignment() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        (*enemy.ptr()).current.location = POINT { x: 4000, y: 4400 };
        (*enemy.ptr()).next = (*enemy.ptr()).current;
        ship.ss.weapon_counter = 1;
        for first_odd in [false, true] {
            let seed = seed_where(|d| (d[0] & 1 != 0) == first_odd && (d[1] & 1 != 0) != first_odd);
            ship.ss.ShipFacing = 0;
            (*ship.ptr()).current.image.frame = ship._ship_frames.frame(0);
            (*ship.ptr()).next = (*ship.ptr()).current;
            let mut c = enemy_concern(&enemy, 8);
            let input = with_rng(seed, 2, || think(&mut ship, &mut c, 0, 1));
            assert_eq!(ship.ss.ShipFacing, if first_odd { 15 } else { 1 });
            assert_eq!(
                input,
                (SPECIAL | LEFT | RIGHT) as u8,
                "second draw is Umgah's turn, after the shared turn"
            );
            assert_eq!(ship.desc.characteristics.special_wait, 8);
        }
        ship.ss.ShipFacing = 0;
        ship.ss.old_status_flags = LEFT as u16;
        let mut c = enemy_concern(&enemy, 8);
        assert_eq!(
            with_rng(8, 0, || think(&mut ship, &mut c, THRUST as u8, 0)),
            SPECIAL as u8
        );
    }
}

#[test]
fn rear_zip_alignment_and_randomness_change_at_turns_8_and_16() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        (*enemy.ptr()).next.location = POINT { x: 4000, y: 4400 };
        for (turn, wait, old, boost, used) in [
            (8, 0, 0, true, 1),
            (9, 0, 0, true, 0),
            (8, 1, 0, true, 0),
            (8, 0, LEFT, true, 0),
            (15, 1, 0, true, 0),
            (16, 1, 0, false, 0),
            (16, 0, LEFT, false, 0),
            (16, 0, 0, true, 0),
        ] {
            ship.ss.old_status_flags = old as u16;
            (*ship.ptr()).__bindgen_anon_4.turn_wait = wait;
            let mut c = enemy_concern(&enemy, turn);
            let input = with_rng(99, used, || think(&mut ship, &mut c, THRUST as u8, 0));
            assert_eq!(
                input & SPECIAL as u8 != 0,
                boost,
                "turn {turn}, wait {wait}, old {old}"
            );
            assert_eq!(input & THRUST as u8, 0);
            assert_eq!(
                ship.desc.characteristics.special_wait,
                if boost { turn as u8 } else { 255 }
            );
        }
    }
}

#[test]
fn ai_energy_test_is_strict_and_uses_world_to_turn_shift() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        (*enemy.ptr()).next.location = POINT { x: 4000, y: 4400 };
        (*ship.ptr()).__bindgen_anon_4.turn_wait = 1;
        for (energy, turn, boost) in [
            (0, 0, false),
            (1, 1, true),
            (1, 2, false),
            (2, 4, true),
            (2, 5, false),
            (3, 6, true),
            (3, 7, false),
        ] {
            ship.desc.ship_info.energy_level = energy;
            let mut c = enemy_concern(&enemy, turn);
            let input = with_rng(6, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
            assert_eq!(input & SPECIAL as u8 != 0, boost);
            assert_eq!(
                ship.desc.cyborg_control.WeaponRange,
                if boost { 32000 } else { 200 }
            );
            assert_eq!(input & THRUST as u8 != 0, !boost);
        }
    }
}

#[test]
fn ai_uses_enemy_maneuverability_and_inclusive_16_through_24_range() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        (*enemy.ptr()).next.location = POINT { x: 4000, y: 3600 };
        for (turn, maneuver, long) in [
            (6, 25, false),
            (7, 25, true),
            (7, 26, false),
            (15, 26, false),
            (16, 26, true),
            (24, 26, true),
            (25, 26, false),
            (25, 25, true),
        ] {
            enemy.desc.cyborg_control.ManeuverabilityIndex = maneuver;
            let mut c = enemy_concern(&enemy, turn);
            let input = with_rng(9, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
            assert_eq!(
                ship.desc.cyborg_control.WeaponRange,
                if long { 32000 } else { 200 }
            );
            assert_eq!(input & THRUST as u8 != 0, !long);
            assert_eq!(input & WEAPON as u8 != 0, turn < 16);
            assert_eq!(input & SPECIAL as u8, 0);
            assert_eq!(ship.desc.characteristics.special_wait, 255);
        }
    }
}

#[test]
fn active_zip_continues_on_nonincreasing_turns_then_exits_with_thrust_and_no_steering() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        (*enemy.ptr()).next.location = POINT { x: 4000, y: 3600 };
        ship.ss.old_status_flags = SPECIAL as u16;
        for (turn, previous, continuing) in [(9, 10, true), (10, 10, true), (11, 10, false)] {
            ship.desc.characteristics.special_wait = previous;
            let mut c = enemy_concern(&enemy, turn);
            let input = with_rng(3, 0, || {
                think(
                    &mut ship,
                    &mut c,
                    (SPECIAL | LEFT | RIGHT | THRUST) as u8,
                    0,
                )
            });
            assert_eq!(
                input,
                if continuing {
                    (SPECIAL | LEFT | RIGHT | WEAPON) as u8
                } else {
                    (THRUST | WEAPON) as u8
                }
            );
            assert_eq!(
                ship.desc.characteristics.special_wait,
                if continuing { turn as u8 } else { 255 }
            );
        }
        ship.desc.ship_info.energy_level = 255;
        for turn in [255, 256, u16::MAX] {
            ship.desc.characteristics.special_wait = 255;
            let mut c = enemy_concern(&enemy, turn);
            let input = with_rng(3, 0, || think(&mut ship, &mut c, THRUST as u8, 0));
            assert_eq!(input, SPECIAL as u8);
            assert_eq!(
                ship.desc.characteristics.special_wait, 255,
                "turn estimate saturates to a byte"
            );
        }
    }
}

#[test]
fn rear_arc_uses_unwrapped_next_location_difference_and_inclusive_angle_bounds() {
    let _battle = Battle::new(8);
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        (*ship.ptr()).__bindgen_anon_4.turn_wait = 1;
        for facing in 0..16 {
            ship.ss.ShipFacing = facing;
            for angle in 0..64 {
                let dx = cosine(angle, 1000);
                let dy = sine(angle, 1000);
                (*enemy.ptr()).next.location = POINT {
                    x: 4000 + dx,
                    y: 4000 + dy,
                };
                let direction = ARCTAN(dx, dy);
                let rear = ((i32::from(direction) - (i32::from(facing) * 4 + 32) + 10) & 63) <= 20;
                let mut c = enemy_concern(&enemy, 7);
                let input = with_rng(11, 0, || think(&mut ship, &mut c, 0, 0));
                assert_eq!(
                    input & SPECIAL as u8 != 0,
                    rear,
                    "facing {facing}, angle {direction}"
                );
                assert_eq!(input & WEAPON as u8 != 0, !rear);
            }
        }
        ship.ss.ShipFacing = 0;
        (*ship.ptr()).next.location = POINT {
            x: 4000,
            y: i16::MAX - 1,
        };
        (*enemy.ptr()).next.location = POINT {
            x: 4000,
            y: i16::MIN + 1,
        };
        let mut c = enemy_concern(&enemy, 7);
        assert_eq!(
            with_rng(1, 0, || think(&mut ship, &mut c, 0, 0)),
            SPECIAL as u8
        );
    }
}
