//! Exercise the Melnorme descriptor and callbacks against the linked battle engine.

use std::{
    ffi::{c_void, CStr},
    mem, ptr,
    sync::{Mutex, MutexGuard},
};
use uqm_rust::ships::native::{
    abi::*, cosine, element, facing_to_angle, frame_index, put_element, sine,
};

unsafe extern "C" {
    fn init_melnorme() -> *mut RACE_DESC;
    fn InitQueue(queue: *mut QUEUE, count: COUNT, size: u16) -> BOOLEAN;
    fn UninitQueue(queue: *mut QUEUE) -> BOOLEAN;
    fn InitDisplayList();
    fn FreeElement(handle: HELEMENT);
    fn RemoveElement(handle: HLINK);
    static mut DisplayArray: [PRIMITIVE; 330];
}

static ENGINE: Mutex<()> = Mutex::new(());

struct Battlefield {
    _guard: MutexGuard<'static, ()>,
    screen: (i32, i32),
}

impl Battlefield {
    fn new() -> Self {
        let guard = ENGINE.lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            assert_ne!(
                InitQueue(&raw mut disp_q, 32, mem::size_of::<ELEMENT>() as u16),
                0
            );
            InitDisplayList();
            GlobData.Game_state.DisplayArray = (&raw mut DisplayArray).cast();
            let screen = (ScreenWidth, ScreenHeight);
            ScreenWidth = 320;
            ScreenHeight = 240;
            Self {
                _guard: guard,
                screen,
            }
        }
    }
}

impl Drop for Battlefield {
    fn drop(&mut self) {
        unsafe {
            GlobData.Game_state.DisplayArray = ptr::null_mut();
            ScreenWidth = self.screen.0;
            ScreenHeight = self.screen.1;
            UninitQueue(&raw mut disp_q);
        }
    }
}

// These layouts are from libs/graphics/drawable.h. Bindgen exposes FRAME as opaque.
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
                bounds: [22, 18],
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

    fn frame(&mut self, index: usize) -> FRAME {
        (&raw mut self.frames[index]).cast()
    }
}

struct Ship {
    desc: Box<RACE_DESC>,
    ss: Box<STARSHIP>,
    el: *mut ELEMENT,
    _ship_frames: FrameSet,
    weapon_frames: FrameSet,
    special_frames: FrameSet,
}

impl Ship {
    fn new(player: SIZE) -> Self {
        unsafe {
            let mut desc = Box::new(*init_melnorme());
            let mut ship_frames = FrameSet::new(16);
            let mut weapon_frames = FrameSet::new(26);
            let mut special_frames = FrameSet::new(16);
            desc.ship_data.ship = [ship_frames.frame(0); 3];
            desc.ship_data.weapon = [weapon_frames.frame(0); 3];
            desc.ship_data.special = [special_frames.frame(0); 3];
            let mut ss: Box<STARSHIP> = Box::new(mem::zeroed());
            ss.RaceDescPtr = &raw mut *desc;
            ss.playerNr = player;
            let handle = AllocElement();
            assert!(!handle.is_null());
            ss.hShip = handle;
            let el = element(handle);
            (*el).pParent = (&raw mut *ss).cast();
            (*el).playerNr = player;
            (*el).state_flags = PLAYER_SHIP as u16;
            (*el).__bindgen_anon_1.life_span = 1;
            (*el).__bindgen_anon_2.crew_level = 20;
            (*el).current.location = POINT { x: 2000, y: 2000 };
            (*el).current.image.farray = (&raw mut desc.ship_data.ship).cast();
            (*el).current.image.frame = ship_frames.frame(0);
            (*el).next = (*el).current;
            (*el).IntersectControl.IntersectStamp.frame = ship_frames.frame(0);
            (*prim(el)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            put_element(handle);
            Self {
                desc,
                ss,
                el,
                _ship_frames: ship_frames,
                weapon_frames,
                special_frames,
            }
        }
    }

    unsafe fn pump(&mut self) -> *mut ELEMENT {
        let mut handle = ptr::null_mut();
        assert_eq!(self.desc.init_weapon_func.unwrap()(self.el, &mut handle), 1);
        assert!(!handle.is_null());
        let e = element(handle);
        // The generic ship firing path attaches the parent and prepares next.
        (*e).pParent = (&raw mut *self.ss).cast();
        (*e).next = (*e).current;
        put_element(handle);
        e
    }

    unsafe fn confusion(&mut self) -> *mut ELEMENT {
        self.desc.ship_info.energy_level = 42;
        self.ss.cur_status_flags = SPECIAL as u16;
        self.ss.special_counter = 0;
        let old_tail = disp_q.tail;
        self.desc.postprocess_func.unwrap()(self.el);
        let handle = disp_q.tail;
        assert_ne!(handle, old_tail);
        let e = element(handle);
        (*e).next = (*e).current;
        e
    }
}

impl Drop for Ship {
    fn drop(&mut self) {
        unsafe {
            RemoveElement(self.ss.hShip);
            FreeElement(self.ss.hShip);
        }
    }
}

unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
    GlobData
        .Game_state
        .DisplayArray
        .add(usize::from((*e).PrimIndex))
}

unsafe fn remove(e: *mut ELEMENT) {
    RemoveElement(e.cast());
    FreeElement(e.cast());
}

unsafe fn queue() -> Vec<HELEMENT> {
    let mut handles = Vec::new();
    let mut h = disp_q.head;
    while !h.is_null() {
        handles.push(h);
        h = (*element(h)).succ;
    }
    handles
}

unsafe fn exhaust_queue() -> Vec<HELEMENT> {
    let mut handles = Vec::new();
    loop {
        let h = AllocElement();
        if h.is_null() {
            return handles;
        }
        handles.push(h);
    }
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

unsafe fn velocity_for(speed: i32, facing: u16) -> VELOCITY_DESC {
    let angle = facing_to_angle(i32::from(facing)) as COUNT;
    let mut v = mem::zeroed();
    SetVelocityComponents(
        &mut v,
        cosine(i32::from(angle), speed << VELOCITY_SHIFT),
        sine(i32::from(angle), speed << VELOCITY_SHIFT),
    );
    v
}

unsafe fn assert_rng_after(seed: u32, draws: usize) {
    let after = TFB_Random();
    TFB_SeedRandom(seed);
    for _ in 0..draws {
        TFB_Random();
    }
    assert_eq!(after, TFB_Random(), "expected {draws} random draws");
}

unsafe fn collide(pulse: *mut ELEMENT, target: *mut ELEMENT) {
    (*pulse).collision_func.unwrap()(pulse, ptr::null_mut(), target, ptr::null_mut());
}

#[test]
fn descriptor_matches_all_c_resources_values_and_empty_runtime_fields() {
    let _battle = Battlefield::new();
    unsafe {
        let p = init_melnorme();
        assert_eq!(p, init_melnorme());
        let d = *p;
        assert_eq!(d.ship_info.ship_flags, FIRES_FORE as u16);
        assert_eq!(
            (
                d.ship_info.ship_cost,
                d.ship_info.crew_level,
                d.ship_info.max_crew
            ),
            (18, 20, 20)
        );
        assert_eq!((d.ship_info.energy_level, d.ship_info.max_energy), (42, 42));
        let s = |p| CStr::from_ptr(p);
        assert_eq!(s(d.ship_info.race_strings_rsc), c"ship.melnorme.text");
        assert_eq!(s(d.ship_info.icons_rsc), c"ship.melnorme.icons");
        assert_eq!(s(d.ship_info.melee_icon_rsc), c"ship.melnorme.meleeicons");
        assert!(d.ship_info.race_strings.is_null());
        assert!(d.ship_info.icons.is_null() && d.ship_info.melee_icon.is_null());
        assert_eq!(d.fleet.strength, u16::MAX);
        assert_eq!((d.fleet.known_loc.x, d.fleet.known_loc.y), (4999, 4999));
        let c = d.characteristics;
        assert_eq!(
            (c.max_thrust, c.thrust_increment, c.energy_regeneration),
            (36, 6, 1)
        );
        assert_eq!(
            (c.weapon_energy_cost, c.special_energy_cost, c.energy_wait),
            (5, 20, 4)
        );
        assert_eq!(
            (
                c.turn_wait,
                c.thrust_wait,
                c.weapon_wait,
                c.special_wait,
                c.ship_mass
            ),
            (4, 4, 1, 20, 7)
        );
        for (i, size) in ["large", "medium", "small"].iter().enumerate() {
            assert_eq!(
                s(d.ship_data.ship_rsc[i]).to_str().unwrap(),
                format!("ship.melnorme.graphics.trader.{size}")
            );
            assert_eq!(
                s(d.ship_data.weapon_rsc[i]).to_str().unwrap(),
                format!("ship.melnorme.graphics.pumpup.{size}")
            );
            assert_eq!(
                s(d.ship_data.special_rsc[i]).to_str().unwrap(),
                format!("ship.melnorme.graphics.confuse.{size}")
            );
        }
        let captain = d.ship_data.captain_control;
        assert_eq!(s(captain.captain_rsc), c"ship.melnorme.graphics.captain");
        assert!(captain.background.is_null() && captain.turn.is_null() && captain.thrust.is_null());
        assert!(captain.weapon.is_null() && captain.special.is_null());
        assert_eq!(s(d.ship_data.victory_ditty_rsc), c"ship.melnorme.ditty");
        assert_eq!(s(d.ship_data.ship_sounds_rsc), c"ship.melnorme.sounds");
        assert!(d
            .ship_data
            .ship
            .iter()
            .chain(d.ship_data.weapon.iter())
            .chain(d.ship_data.special.iter())
            .all(|f| f.is_null()));
        assert!(d.ship_data.victory_ditty.is_null() && d.ship_data.ship_sounds.is_null());
        assert_eq!(
            (
                d.cyborg_control.ManeuverabilityIndex,
                d.cyborg_control.WeaponRange
            ),
            (0, 1800)
        );
        assert!(
            d.postprocess_func.is_some()
                && d.init_weapon_func.is_some()
                && d.cyborg_control.intelligence_func.is_some()
        );
        assert!(d.preprocess_func.is_none() && d.uninit_func.is_none());
        assert!(d.data.is_null() && d.CodeRef.is_null());
        // init_melnorme re-registers callbacks, without resetting loaded data.
        let mut loaded = FrameSet::new(26);
        let frame = loaded.frame(0);
        (*p).ship_info.energy_level = 37;
        (*p).ship_data.weapon[0] = frame;
        (*p).postprocess_func = None;
        (*p).init_weapon_func = None;
        (*p).cyborg_control.intelligence_func = None;
        assert_eq!(init_melnorme(), p);
        assert!((*p).postprocess_func.is_some() && (*p).init_weapon_func.is_some());
        assert!((*p).cyborg_control.intelligence_func.is_some());
        let preserved = ((*p).ship_info.energy_level, (*p).ship_data.weapon[0]);
        *p = d;
        assert_eq!(preserved, (37, frame));
    }
}

#[test]
fn initial_pump_uses_the_c_missile_block_at_all_facings_and_coordinate_limits() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        for facing in 0..16 {
            ship.ss.ShipFacing = facing;
            (*ship.el).next.location = POINT {
                x: i16::MAX,
                y: i16::MIN,
            };
            let mut h = ptr::null_mut();
            TFB_SeedRandom(32);
            assert_eq!(ship.desc.init_weapon_func.unwrap()(ship.el, &mut h), 1);
            assert_rng_after(32, 0);
            let e = element(h);
            assert!(!e.is_null());
            assert_eq!((*e).__bindgen_anon_1.life_span, 2);
            assert_eq!((*e).__bindgen_anon_2.hit_points, 2);
            assert_eq!((*e).__bindgen_anon_3.mass_points, 2);
            assert_eq!((*e).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*e).__bindgen_anon_5.thrust_wait, 72);
            assert_eq!(
                (*e).state_flags,
                (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
            );
            assert_eq!((*e).playerNr, 0);
            assert_eq!(
                (*e).current.image.farray,
                (&raw mut ship.desc.ship_data.weapon).cast()
            );
            assert_eq!(frame_index((*e).current.image.frame), 0);
            assert!((*e).preprocess_func.is_none());
            assert!((*e).postprocess_func.is_some() && (*e).collision_func.is_some());
            assert!(
                (*e).pParent.is_null(),
                "the firing path, not the initializer, attaches the ship"
            );
            assert_eq!(
                velocity_fields(&(*e).velocity),
                velocity_fields(&velocity_for(96, facing))
            );
            // pixoffs and speed are both DISPLAY_TO_WORLD(24), but the two
            // separate SIZE truncations still matter at diagonal facings.
            let a = facing_to_angle(facing.into());
            let dx = cosine(a, 96 << VELOCITY_SHIFT);
            let dy = sine(a, 96 << VELOCITY_SHIFT);
            let x = i16::MAX
                .wrapping_add(cosine(a, 96))
                .wrapping_sub(dx >> VELOCITY_SHIFT);
            let y = i16::MIN
                .wrapping_add(sine(a, 96))
                .wrapping_sub(dy >> VELOCITY_SHIFT);
            assert_eq!(((*e).current.location.x, (*e).current.location.y), (x, y));
            assert_eq!(((*e).next.location.x, (*e).next.location.y), (0, 0));
            assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
            FreeElement(h);
        }
    }
}

#[test]
fn appearing_charge_only_zeros_the_entire_velocity_record() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let e = ship.pump();
        let before = queue();
        (*e).__bindgen_anon_4.turn_wait = 0x82;
        (*e).__bindgen_anon_5.thrust_wait = 13;
        ship.ss.energy_counter = 19;
        TFB_SeedRandom(7);
        (*e).postprocess_func.unwrap()(e);
        assert_eq!(velocity_fields(&(*e).velocity), [0; 9]);
        assert_eq!((*e).__bindgen_anon_4.turn_wait, 0x82);
        assert_eq!((*e).__bindgen_anon_5.thrust_wait, 13);
        assert_eq!(ship.ss.energy_counter, 19);
        assert_eq!(queue(), before);
        assert_eq!((*prim(e)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
        assert_rng_after(7, 0);
    }
}

#[test]
fn held_charge_replaces_the_element_and_uses_the_old_level_for_damage() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        for level in 0..4u8 {
            for reverse in [0, 0x80] {
                let e = ship.pump();
                (*e).state_flags &= !(APPEARING as u16);
                (*e).__bindgen_anon_4.turn_wait = reverse | level;
                (*e).__bindgen_anon_5.thrust_wait = 1;
                (*e).current.image.frame = ship.weapon_frames.frame(usize::from(level) * 5 + 2);
                ship.ss.cur_status_flags = WEAPON as u16;
                ship.ss.old_status_flags = WEAPON as u16;
                ship.ss.energy_counter = 0;
                TFB_SeedRandom(12);
                (*e).postprocess_func.unwrap()(e);
                let p = element(disp_q.tail);
                assert_ne!(p, e);
                let next_level = (level + 1).min(3);
                assert_eq!((*p).__bindgen_anon_4.turn_wait, reverse | next_level);
                assert_eq!((*p).__bindgen_anon_5.thrust_wait, 72);
                assert_eq!(
                    frame_index((*p).current.image.frame),
                    u16::from(next_level) * 5 + 2
                );
                assert_eq!((*p).__bindgen_anon_2.hit_points, 2u16 << level);
                assert_eq!((*p).__bindgen_anon_3.mass_points, 2u8 << level);
                assert_eq!((*p).__bindgen_anon_1.life_span, 2);
                assert_eq!(ship.ss.weapon_counter, 1);
                assert_eq!(ship.ss.energy_counter, 4);
                assert_eq!((*p).pParent, (&raw mut *ship.ss).cast());
                assert!((*p).preprocess_func.is_none() && (*p).postprocess_func.is_some());
                assert_eq!((*prim(e)).Type, gfx_object_NUM_PRIMS as GRAPHICS_PRIM);
                assert_ne!((*e).state_flags & NONSOLID as u16, 0);
                assert_eq!(
                    velocity_fields(&(*p).velocity),
                    velocity_fields(&velocity_for(96, ship.ss.ShipFacing))
                );
                assert_rng_after(12, 0);
                remove(p);
                remove(e);
            }
        }
    }
}

#[test]
fn charge_wait_wraps_as_a_byte_and_hit_points_truncate_after_the_c_assignment_chain() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let e = ship.pump();
        (*e).state_flags &= !(APPEARING as u16);
        (*e).__bindgen_anon_4.turn_wait = 8;
        (*e).__bindgen_anon_5.thrust_wait = 0;
        (*e).current.image.frame = ship.weapon_frames.frame(0);
        ship.ss.cur_status_flags = WEAPON as u16;
        ship.ss.old_status_flags = WEAPON as u16;
        (*e).postprocess_func.unwrap()(e);
        let p = element(disp_q.tail);
        assert_eq!((*p).__bindgen_anon_5.thrust_wait, 255);
        assert_eq!((*p).__bindgen_anon_4.turn_wait, 8);
        assert_eq!(frame_index((*p).current.image.frame), 1);
        assert_eq!((*p).__bindgen_anon_2.hit_points, 512);
        assert_eq!((*p).__bindgen_anon_3.mass_points, 0);
    }
}

#[test]
fn release_requires_both_weapon_status_words_to_hold_and_uses_full_velocity_units() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        for (current, old) in [(0, 0), (WEAPON, 0), (0, WEAPON), (WEAPON, WEAPON)] {
            for facing in 0..16 {
                let e = ship.pump();
                (*e).state_flags &= !(APPEARING as u16);
                ship.ss.cur_status_flags = current as u16;
                ship.ss.old_status_flags = old as u16;
                ship.ss.ShipFacing = facing;
                ship.ss.weapon_counter = 9;
                TFB_SeedRandom(14);
                (*e).postprocess_func.unwrap()(e);
                let p = element(disp_q.tail);
                let held = current & old & WEAPON != 0;
                assert_eq!((*p).__bindgen_anon_1.life_span, if held { 2 } else { 10 });
                assert_eq!((*p).preprocess_func.is_some(), !held);
                assert_eq!((*p).postprocess_func.is_some(), held);
                assert_eq!(ship.ss.weapon_counter, if held { 1 } else { 9 });
                assert_eq!(
                    velocity_fields(&(*p).velocity),
                    velocity_fields(&velocity_for(if held { 96 } else { 180 }, facing))
                );
                assert_rng_after(14, 0);
                remove(p);
                remove(e);
            }
        }
    }
}

#[test]
fn fired_charge_animation_bounces_at_each_five_frame_boundary_without_changing_level() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let e = ship.pump();
        (*e).state_flags &= !(APPEARING as u16);
        ship.ss.cur_status_flags = 0;
        (*e).postprocess_func.unwrap()(e);
        let p = element(disp_q.tail);
        let pre = (*p).preprocess_func.unwrap();
        for level in 0..4u8 {
            for (offset, reverse, next, next_reverse) in [
                (0, 0, 1, 0),
                (4, 0, 3, 0x80),
                (3, 0x80, 2, 0x80),
                (0, 0x80, 1, 0),
            ] {
                (*p).current.image.frame =
                    ship.weapon_frames.frame(usize::from(level) * 5 + offset);
                (*p).next.image.frame = (*p).current.image.frame;
                (*p).__bindgen_anon_4.turn_wait = level | reverse;
                (*p).__bindgen_anon_5.thrust_wait = 2;
                (*p).state_flags &= !(CHANGING as u16);
                TFB_SeedRandom(11);
                pre(p);
                assert_eq!((*p).__bindgen_anon_5.thrust_wait, 1);
                assert_eq!((*p).__bindgen_anon_4.turn_wait, level | next_reverse);
                assert_eq!(
                    frame_index((*p).next.image.frame),
                    u16::from(level) * 5 + next
                );
                assert_ne!((*p).state_flags & CHANGING as u16, 0);
                assert_rng_after(11, 0);
            }
        }
        (*p).__bindgen_anon_5.thrust_wait = 1;
        (*p).state_flags &= !(CHANGING as u16);
        let frame = (*p).next.image.frame;
        pre(p);
        assert_eq!((*p).__bindgen_anon_5.thrust_wait, 0);
        assert_eq!((*p).next.image.frame, frame);
        assert_eq!((*p).state_flags & CHANGING as u16, 0);
        pre(p);
        assert_eq!((*p).__bindgen_anon_5.thrust_wait, 255);
        assert_ne!((*p).state_flags & CHANGING as u16, 0);
    }
}

#[test]
fn pump_collision_restores_the_shared_blast_counter_and_sets_the_level_blast_animation() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        for (level, width) in [(0, 22), (1, 23), (2, 1026), (3, 22)] {
            let e = ship.pump();
            (*e).__bindgen_anon_4.turn_wait = 0x80 | level;
            (*e).__bindgen_anon_5.thrust_wait = 67;
            let index = usize::from(level) * 5;
            ship.weapon_frames.frames[index].bounds[0] = width;
            (*e).next.image.frame = ship.weapon_frames.frame(index);
            (*e).velocity.TravelAngle = 0;
            let mut obstacle: ELEMENT = mem::zeroed();
            let mut point = POINT { x: 20, y: 30 };
            let old_tail = disp_q.tail;
            TFB_SeedRandom(33);
            (*e).collision_func.unwrap()(e, &mut point, &mut obstacle, &mut point);
            assert_ne!({ disp_q.tail }, old_tail);
            let blast = element(disp_q.tail);
            assert_eq!((*e).__bindgen_anon_5.thrust_wait, 67);
            assert_eq!((*e).__bindgen_anon_2.hit_points, 0);
            assert_eq!((*blast).__bindgen_anon_1.life_span, 3 + u16::from(level));
            assert_eq!((*blast).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*blast).__bindgen_anon_5.next_turn, 0);
            assert_eq!((*blast).current.image.farray, (*e).next.image.farray);
            assert_eq!(frame_index((*blast).current.image.frame), 20);
            let offset = i32::from((width >> 1) as u8) << ONE_SHIFT;
            assert_eq!((*blast).current.location.x, 80 + cosine(0, offset));
            assert_eq!((*blast).current.location.y, 120 + sine(0, offset));
            let animate = (*blast).preprocess_func.unwrap();
            (*blast).__bindgen_anon_4.turn_wait = 1;
            (*blast).__bindgen_anon_5.next_turn = 3;
            (*blast).state_flags &= !(CHANGING as u16);
            animate(blast);
            assert_eq!((*blast).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*blast).state_flags & CHANGING as u16, 0);
            animate(blast);
            assert_eq!(frame_index((*blast).next.image.frame), 21);
            assert_eq!((*blast).__bindgen_anon_4.turn_wait, 3);
            assert_ne!((*blast).state_flags & CHANGING as u16, 0);
            assert_rng_after(33, 0);
            // Already-collided weapons do not allocate a second blast.
            let before = queue();
            (*e).collision_func.unwrap()(e, &mut point, &mut obstacle, &mut point);
            assert_eq!(queue(), before);
            assert_eq!((*e).__bindgen_anon_5.thrust_wait, 67);
            remove(blast);
            remove(e);
        }
    }
}

#[test]
fn confusion_special_obeys_energy_input_and_cooldown_boundaries() {
    let _battle = Battlefield::new();
    unsafe {
        for (energy, status, counter, fired) in [
            (19, SPECIAL, 0, false),
            (20, SPECIAL, 0, true),
            (42, SPECIAL, 1, false),
            (42, 0, 0, false),
        ] {
            let mut ship = Ship::new(1);
            ship.ss.cur_status_flags = status as u16;
            ship.ss.special_counter = counter;
            ship.desc.ship_info.energy_level = energy;
            ship.desc.characteristics.special_wait = 29;
            let before = queue();
            TFB_SeedRandom(23);
            ship.desc.postprocess_func.unwrap()(ship.el);
            assert_eq!(
                ship.desc.ship_info.energy_level,
                if fired { energy - 20 } else { energy }
            );
            assert_eq!(ship.ss.special_counter, if fired { 29 } else { counter });
            if fired {
                let e = element(disp_q.tail);
                assert_eq!((*e).__bindgen_anon_1.life_span, 20);
                assert_eq!((*e).__bindgen_anon_2.hit_points, 200);
                assert_eq!((*e).__bindgen_anon_3.mass_points, 0);
                assert_eq!((*e).__bindgen_anon_5.blast_offset, 4);
                assert_eq!(
                    (*e).state_flags,
                    (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16
                );
                assert_eq!((*e).playerNr, 1);
                assert_eq!((*e).pParent, (&raw mut *ship.ss).cast());
                assert_eq!(
                    (*e).current.image.farray,
                    (&raw mut ship.desc.ship_data.special).cast()
                );
                assert_eq!(frame_index((*e).current.image.frame), 0);
                assert_eq!(
                    velocity_fields(&(*e).velocity),
                    velocity_fields(&velocity_for(120, ship.ss.ShipFacing))
                );
                assert!((*e).preprocess_func.is_some() && (*e).collision_func.is_some());
                assert!((*e).postprocess_func.is_none());
                assert_eq!(ship.ss.energy_counter, 4);
                remove(e);
            } else {
                assert_eq!(queue(), before);
                if energy == 19 {
                    assert_ne!(
                        ship.ss.cur_status_flags & 32,
                        0,
                        "DeltaEnergy sets LOW_ON_ENERGY"
                    );
                }
            }
            assert_rng_after(23, 0);
        }
    }
}

#[test]
fn failed_initialization_returns_one_null_slot_and_special_still_spends_energy() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let exhausted = exhaust_queue();
        assert!(!exhausted.is_empty());
        let mut h = ship.ss.hShip;
        TFB_SeedRandom(18);
        assert_eq!(ship.desc.init_weapon_func.unwrap()(ship.el, &mut h), 1);
        assert!(h.is_null());
        let before = queue();
        ship.desc.ship_info.energy_level = 42;
        ship.ss.cur_status_flags = SPECIAL as u16;
        ship.desc.postprocess_func.unwrap()(ship.el);
        assert_eq!(ship.desc.ship_info.energy_level, 22);
        assert_eq!(ship.ss.special_counter, 0);
        assert_eq!(ship.ss.energy_counter, 4);
        assert_eq!(queue(), before);
        assert_rng_after(18, 0);
        for h in exhausted {
            FreeElement(h);
        }
    }
}

#[test]
fn solid_confusion_animation_wraps_in_the_first_eight_frames() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let e = ship.confusion();
        let pre = (*e).preprocess_func.unwrap();
        for (index, expected) in [(0, 1), (6, 7), (7, 0), (15, 0)] {
            (*e).current.image.frame = ship.special_frames.frame(index);
            (*e).state_flags &= !(CHANGING as u16);
            TFB_SeedRandom(17);
            pre(e);
            assert_eq!(frame_index((*e).next.image.frame), expected);
            assert_ne!((*e).state_flags & CHANGING as u16, 0);
            assert_rng_after(17, 0);
        }
    }
}

#[test]
fn confusion_collision_inserts_a_controller_at_the_head_with_exactly_one_rng_draw() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let victim = Ship::new(1);
        let mut seen = [false; 2];
        for seed in 1..33 {
            let e = ship.confusion();
            let old_head = disp_q.head;
            TFB_SeedRandom(seed);
            let bit = (TFB_Random() as u8 & 1) as usize;
            TFB_SeedRandom(seed);
            collide(e, victim.el);
            let h = disp_q.head;
            assert_ne!(h, old_head);
            let controller = element(h);
            assert_eq!((*controller).succ, old_head);
            assert_eq!((*controller).playerNr, 0);
            assert_eq!(
                (*controller).state_flags,
                (FINITE_LIFE | NONSOLID | CHANGING) as u16
            );
            assert_eq!((*controller).__bindgen_anon_1.life_span, 400);
            assert_eq!((*controller).__bindgen_anon_4.turn_wait, 1 << bit);
            assert_eq!((*controller).hTarget, victim.ss.hShip);
            assert_eq!((*controller).pParent, (&raw mut *ship.ss).cast());
            assert_eq!((*controller).current.image.farray, (*e).next.image.farray);
            assert_eq!(frame_index((*controller).current.image.frame), 8);
            assert_eq!(
                (*controller).next.image.frame,
                (*controller).current.image.frame
            );
            assert_eq!(
                (*prim(controller)).Type,
                gfx_object_NUM_PRIMS as GRAPHICS_PRIM
            );
            assert_eq!((*e).__bindgen_anon_2.hit_points, 0);
            assert_eq!((*e).__bindgen_anon_1.life_span, 0);
            assert_eq!(
                (*e).state_flags & (DISAPPEARING | COLLISION | NONSOLID) as u16,
                (DISAPPEARING | COLLISION | NONSOLID) as u16
            );
            assert_rng_after(seed, 1);
            seen[bit] = true;
            remove(controller);
            remove(e);
        }
        assert_eq!(seen, [true, true]);
    }
}

#[test]
fn confusion_reuses_the_first_same_player_nonsolid_special_even_if_it_is_a_trail() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let victim = Ship::new(1);
        let another = Ship::new(1);
        let pulse = ship.confusion();
        let farray = (&raw mut ship.desc.ship_data.special).cast();
        let mut candidates = Vec::new();
        for (player, frames, flags) in [
            (1, farray, NONSOLID),
            (0, (&raw mut ship.desc.ship_data.weapon).cast(), NONSOLID),
            (0, farray, FINITE_LIFE),
            (0, farray, FINITE_LIFE | NONSOLID | CHANGING),
            (0, farray, FINITE_LIFE | NONSOLID),
        ] {
            let h = AllocElement();
            assert!(!h.is_null());
            let e = element(h);
            (*e).playerNr = player;
            (*e).current.image.farray = frames;
            (*e).current.image.frame = ship.special_frames.frame(11);
            (*e).state_flags = flags as u16;
            (*e).__bindgen_anon_1.life_span = 1;
            (*e).hTarget = victim.ss.hShip;
            (*e).pParent = (&raw mut *ship.ss).cast();
            (*prim(e)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
            put_element(h);
            candidates.push(e);
        }
        let chosen = candidates[3];
        let exhausted = exhaust_queue();
        let before = queue();
        TFB_SeedRandom(51);
        let direction = 1 << (TFB_Random() as u8 & 1);
        TFB_SeedRandom(51);
        collide(pulse, another.el);
        assert_eq!(
            queue(),
            before,
            "reuse does not move or allocate an element"
        );
        assert_eq!((*chosen).__bindgen_anon_1.life_span, 400);
        assert_eq!((*chosen).__bindgen_anon_4.turn_wait, direction);
        assert_eq!(
            (*chosen).hTarget,
            victim.ss.hShip,
            "reuse does not retarget"
        );
        assert_eq!(frame_index((*chosen).current.image.frame), 11);
        assert!(
            (*chosen).preprocess_func.is_none(),
            "reuse does not initialize the callback"
        );
        assert_eq!((*prim(chosen)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
        for &i in &[0, 1, 2, 4] {
            assert_eq!((*candidates[i]).__bindgen_anon_1.life_span, 1);
        }
        assert_rng_after(51, 1);
        for h in exhausted {
            FreeElement(h);
        }
    }
}

#[test]
fn confusion_ignores_nonships_and_consumes_no_rng_when_controller_allocation_fails() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let victim = Ship::new(1);
        let e = ship.confusion();
        let mut obstacle: ELEMENT = mem::zeroed();
        let before = queue();
        let flags = (*e).state_flags;
        TFB_SeedRandom(62);
        collide(e, &mut obstacle);
        assert_eq!(queue(), before);
        assert_eq!((*e).state_flags, flags);
        assert_eq!((*e).__bindgen_anon_2.hit_points, 200);
        assert_rng_after(62, 0);
        let exhausted = exhaust_queue();
        TFB_SeedRandom(62);
        collide(e, victim.el);
        assert_eq!(queue(), before);
        assert_eq!((*e).__bindgen_anon_1.life_span, 0);
        assert_eq!((*e).__bindgen_anon_2.hit_points, 0);
        assert_ne!((*e).state_flags & NONSOLID as u16, 0);
        assert_rng_after(62, 0);
        for h in exhausted {
            FreeElement(h);
        }
    }
}

#[test]
fn controller_follows_target_overrides_controls_wraps_animation_and_emits_one_frame_trails() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut victim = Ship::new(1);
        let e = ship.confusion();
        collide(e, victim.el);
        let controller = element(disp_q.head);
        let pre = (*controller).preprocess_func.unwrap();
        for direction in [LEFT, RIGHT] {
            (*controller).__bindgen_anon_4.turn_wait = direction as u8;
            (*controller).current.image.frame = ship.special_frames.frame(15);
            (*victim.el).next.location = POINT { x: -123, y: 456 };
            victim.ss.ship_input_state = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL | 0x80) as u8;
            let before = queue().len();
            TFB_SeedRandom(88);
            pre(controller);
            assert_eq!(
                victim.ss.ship_input_state,
                (THRUST | WEAPON | 0x80 | direction) as u8
            );
            assert_eq!(
                ((*controller).next.location.x, (*controller).next.location.y),
                (-123, 456)
            );
            assert_eq!(frame_index((*controller).next.image.frame), 8);
            assert_eq!(queue().len(), before + 1);
            let trail = element(disp_q.tail);
            assert_eq!((*trail).playerNr, 0);
            assert_eq!(
                (*trail).state_flags,
                (FINITE_LIFE | NONSOLID | CHANGING) as u16
            );
            assert_eq!((*trail).__bindgen_anon_1.life_span, 1);
            assert_eq!((*trail).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*trail).hTarget, victim.ss.hShip);
            assert_eq!((*trail).pParent, (*controller).pParent);
            assert_eq!((*trail).current.image.frame, (*controller).next.image.frame);
            assert_eq!((*trail).next.image.frame, (*controller).next.image.frame);
            assert_eq!((*prim(trail)).Type, gfx_object_STAMP_PRIM as GRAPHICS_PRIM);
            assert_eq!((*trail).preprocess_func.unwrap() as usize, pre as usize);
            // The visible one-frame copy follows, but does not override inputs
            // or emit another copy because its turn_wait is zero.
            victim.ss.ship_input_state = SPECIAL as u8;
            (*victim.el).next.location.x = 800;
            let count = queue().len();
            pre(trail);
            assert_eq!((*trail).next.location.x, 800);
            assert_eq!(victim.ss.ship_input_state, SPECIAL as u8);
            assert_eq!(queue().len(), count);
            assert_rng_after(88, 0);
            remove(trail);
        }
        (*controller).hTarget = ptr::null_mut();
        let before = queue();
        pre(controller);
        assert_eq!((*controller).__bindgen_anon_1.life_span, 0);
        assert_ne!((*controller).state_flags & DISAPPEARING as u16, 0);
        assert_eq!(queue(), before);
    }
}

#[test]
fn controller_still_overrides_controls_when_trail_allocation_fails() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut victim = Ship::new(1);
        let pulse = ship.confusion();
        collide(pulse, victim.el);
        let controller = element(disp_q.head);
        (*controller).__bindgen_anon_4.turn_wait = RIGHT as u8;
        (*controller).current.image.frame = ship.special_frames.frame(8);
        let exhausted = exhaust_queue();
        let before = queue();
        victim.ss.ship_input_state = (LEFT | SPECIAL | WEAPON) as u8;
        TFB_SeedRandom(9);
        (*controller).preprocess_func.unwrap()(controller);
        assert_eq!(victim.ss.ship_input_state, (RIGHT | WEAPON) as u8);
        assert_eq!(frame_index((*controller).next.image.frame), 9);
        assert_eq!(queue(), before);
        assert_rng_after(9, 0);
        for h in exhausted {
            FreeElement(h);
        }
    }
}

#[test]
fn ai_without_targets_preserves_c_toggle_thresholds_counters_and_rng_order() {
    let _battle = Battlefield::new();
    unsafe {
        for (energy, special_counter, weapon_counter, initial, expected) in [
            (19, 0, 0, 0, 0),
            (20, 0, 0, 0, 0),
            (24, 0, 0, 0, 0),
            (25, 0, 0, 0, WEAPON),
            (0, 1, 0, 0, WEAPON),
            (19, 0, 1, 0, WEAPON),
            (24, 0, 1, 0, WEAPON),
            (42, 0, 2, 0, 0),
            (19, 0, 2, WEAPON, WEAPON),
            (20, 0, 2, WEAPON, WEAPON | SPECIAL),
            (42, 1, 2, SPECIAL | LEFT, LEFT),
        ] {
            let mut ship = Ship::new(0);
            ship.desc.ship_info.energy_level = energy;
            ship.ss.special_counter = special_counter;
            ship.ss.weapon_counter = weapon_counter;
            ship.ss.ship_input_state = initial as u8;
            (*ship.el).__bindgen_anon_4.turn_wait = 255;
            (*ship.el).__bindgen_anon_5.thrust_wait = 255;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            let init = ship.desc.init_weapon_func.unwrap() as usize;
            let before = queue();
            TFB_SeedRandom(77);
            ship.desc.cyborg_control.intelligence_func.unwrap()(ship.el, concerns.as_mut_ptr(), 0);
            assert_eq!(
                ship.ss.ship_input_state, expected as u8,
                "energy {energy}, special {special_counter}, weapon {weapon_counter}"
            );
            assert_eq!(ship.ss.weapon_counter, weapon_counter);
            assert_eq!(ship.desc.init_weapon_func.unwrap() as usize, init);
            assert_eq!((*ship.el).__bindgen_anon_4.turn_wait, 255);
            assert_eq!((*ship.el).__bindgen_anon_5.thrust_wait, 255);
            assert_eq!(queue(), before);
            assert_rng_after(77, 0);
        }
    }
}

#[test]
fn ai_enemy_move_state_matches_energy_held_weapon_and_immediate_weapon_rules() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let mut enemy = Ship::new(1);
        ship.ss.special_counter = 1;
        for (energy, held, immediate, expected) in [
            (24, false, false, MOVEMENT_STATE_ENTICE),
            (24, false, true, MOVEMENT_STATE_ENTICE),
            (25, false, false, MOVEMENT_STATE_PURSUE),
            (24, true, false, MOVEMENT_STATE_PURSUE),
            (25, false, true, MOVEMENT_STATE_AVOID),
            (24, true, true, MOVEMENT_STATE_AVOID),
        ] {
            ship.desc.ship_info.energy_level = energy;
            ship.ss.old_status_flags = if held { WEAPON as u16 } else { 0 };
            enemy.desc.ship_info.ship_flags = if immediate {
                IMMEDIATE_WEAPON as u16
            } else {
                0
            };
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.el;
            concerns[ENEMY_SHIP_INDEX as usize].MoveState = MOVEMENT_STATE_AVOID;
            ship.desc.cyborg_control.intelligence_func.unwrap()(ship.el, concerns.as_mut_ptr(), 0);
            assert_eq!(concerns[ENEMY_SHIP_INDEX as usize].MoveState, expected);
        }
    }
}

#[test]
fn ai_tests_the_confusion_trajectory_and_restores_waits_and_initializer() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        // The first pass has zero concerns. The special pass always evaluates
        // the enemy slot and must not move a ship whose waits were zero.
        (*ship.el).__bindgen_anon_4.turn_wait = 0;
        (*ship.el).__bindgen_anon_5.thrust_wait = 0;
        let init = ship.desc.init_weapon_func.unwrap() as usize;
        for (energy, distance, expected) in [
            (19, 320, 0),
            (20, 320, SPECIAL),
            (24, 320, SPECIAL),
            (25, 320, WEAPON | SPECIAL),
            (25, 4000, WEAPON),
        ] {
            ship.desc.ship_info.energy_level = energy;
            ship.ss.ship_input_state = 0;
            ship.ss.weapon_counter = 0;
            ship.ss.special_counter = 0;
            (*enemy.el).current.location = POINT {
                x: 2000 + cosine(0, distance),
                y: 2000 + sine(0, distance),
            };
            (*enemy.el).next = (*enemy.el).current;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.el;
            concerns[ENEMY_SHIP_INDEX as usize].MoveState = MOVEMENT_STATE_NO_MOVEMENT;
            let before = queue();
            TFB_SeedRandom(95);
            ship.desc.cyborg_control.intelligence_func.unwrap()(ship.el, concerns.as_mut_ptr(), 0);
            assert_eq!(
                ship.ss.ship_input_state, expected as u8,
                "energy {energy}, distance {distance}"
            );
            assert_eq!((*ship.el).__bindgen_anon_4.turn_wait, 0);
            assert_eq!((*ship.el).__bindgen_anon_5.thrust_wait, 0);
            assert_eq!(ship.ss.ShipFacing, 0);
            assert_eq!(ship.ss.weapon_counter, 0);
            assert_eq!(ship.desc.init_weapon_func.unwrap() as usize, init);
            assert_eq!(queue(), before, "AI test missiles are freed, not queued");
            assert_rng_after(95, 0);
        }
    }
}

#[test]
fn held_charge_uses_the_same_ping_pong_animation_as_a_released_pulse() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        ship.ss.cur_status_flags = WEAPON as u16;
        ship.ss.old_status_flags = WEAPON as u16;
        for (index, turn, expected_index, expected_turn) in [
            (0, 0, 1, 0),
            (4, 0, 3, 0x80),
            (7, 0x81, 6, 0x81),
            (5, 0x81, 6, 1),
            (14, 2, 13, 0x82),
            (15, 0x83, 16, 3),
        ] {
            let e = ship.pump();
            (*e).state_flags &= !(APPEARING as u16);
            (*e).current.image.frame = ship.weapon_frames.frame(index);
            (*e).__bindgen_anon_4.turn_wait = turn;
            (*e).__bindgen_anon_5.thrust_wait = 2;
            (*e).postprocess_func.unwrap()(e);
            let p = element(disp_q.tail);
            assert_eq!((*p).__bindgen_anon_5.thrust_wait, 1);
            assert_eq!((*p).__bindgen_anon_4.turn_wait, expected_turn);
            assert_eq!(frame_index((*p).current.image.frame), expected_index);
            assert_eq!((*p).state_flags & CHANGING as u16, 0);
            remove(p);
            remove(e);
        }
    }
}

#[test]
fn ai_first_pass_tests_the_full_speed_pump_and_toggles_an_existing_charge_to_release() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        // Blocking movement lets the real shared AI test each projectile
        // without changing the facing used by the second pass.
        (*ship.el).__bindgen_anon_4.turn_wait = 1;
        (*ship.el).__bindgen_anon_5.thrust_wait = 1;
        for (distance, counter, expected) in [
            (1400, 0, WEAPON | SPECIAL),
            (1400, 1, SPECIAL),
            (2000, 0, WEAPON | SPECIAL),
            (2000, 1, WEAPON | SPECIAL),
            (2600, 0, WEAPON),
        ] {
            ship.desc.ship_info.energy_level = 42;
            ship.ss.ship_input_state = 0;
            ship.ss.special_counter = 0;
            ship.ss.weapon_counter = counter;
            (*enemy.el).current.location = POINT {
                x: 2000 + cosine(0, distance),
                y: 2000 + sine(0, distance),
            };
            (*enemy.el).next = (*enemy.el).current;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.el;
            let before = queue();
            TFB_SeedRandom(71);
            ship.desc.cyborg_control.intelligence_func.unwrap()(ship.el, concerns.as_mut_ptr(), 1);
            assert_eq!(
                ship.ss.ship_input_state, expected as u8,
                "distance {distance}, counter {counter}"
            );
            assert_eq!(ship.ss.weapon_counter, counter);
            assert_eq!((*ship.el).__bindgen_anon_4.turn_wait, 1);
            assert_eq!((*ship.el).__bindgen_anon_5.thrust_wait, 1);
            assert_eq!(queue(), before);
            assert_rng_after(71, 0);
        }
    }
}

#[test]
fn ai_pursuit_uses_the_shared_half_turn_rng_draw_without_extra_draws() {
    let _battle = Battlefield::new();
    unsafe {
        let mut ship = Ship::new(0);
        let enemy = Ship::new(1);
        let dx = cosine(HALF_CIRCLE as i32, 400);
        let dy = sine(HALF_CIRCLE as i32, 400);
        assert_eq!(ARCTAN(dx, dy), HALF_CIRCLE as u16);
        (*enemy.el).current.location = POINT {
            x: 2000 + dx,
            y: 2000 + dy,
        };
        (*enemy.el).next = (*enemy.el).current;
        ship.ss.weapon_counter = 2;
        ship.ss.special_counter = 1;
        for seed in 1..9 {
            ship.ss.ShipFacing = 0;
            ship.ss.ship_input_state = 0;
            (*ship.el).__bindgen_anon_4.turn_wait = 0;
            (*ship.el).__bindgen_anon_5.thrust_wait = 1;
            let mut concerns: [EVALUATE_DESC; 4] = mem::zeroed();
            concerns[ENEMY_SHIP_INDEX as usize].ObjectPtr = enemy.el;
            concerns[ENEMY_SHIP_INDEX as usize].which_turn = 1;
            TFB_SeedRandom(seed);
            let left = TFB_Random() & 1 != 0;
            TFB_SeedRandom(seed);
            ship.desc.cyborg_control.intelligence_func.unwrap()(ship.el, concerns.as_mut_ptr(), 1);
            assert_eq!(
                ship.ss.ship_input_state,
                if left { LEFT } else { RIGHT } as u8
            );
            assert_eq!(ship.ss.ShipFacing, if left { 15 } else { 1 });
            assert_eq!(ship.ss.weapon_counter, 2);
            assert_rng_after(seed, 1);
        }
    }
}
