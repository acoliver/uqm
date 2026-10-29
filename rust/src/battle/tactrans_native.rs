//! Ship death, replacement, warp transitions, and battle-end synchronization.

use crate::ships::native::abi;

pub const fn minimum_ditty_frames(ticks_per_second: u32) -> u16 {
    (ticks_per_second * 3 / (ticks_per_second / 24)) as u16
}

pub const fn cleanup_lifetime(minimum: u16, music_started: bool, winner: bool) -> u16 {
    (if winner {
        minimum.wrapping_add(1)
    } else if music_started {
        minimum
    } else {
        1
    })
    .wrapping_add(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExplosionStep {
    pub fragments: u8,
    pub hide_primitive: bool,
    pub clear_preprocess: bool,
}

pub const fn explosion_step(tick: u8) -> ExplosionStep {
    ExplosionStep {
        fragments: match tick {
            25 | 0 | 1 | 2 | 20 | 21 | 22 | 23 | 24 => 1,
            3 | 4 | 5 | 18 | 19 => 2,
            _ => 3,
        },
        hide_primitive: tick == 15,
        clear_preprocess: tick == 25,
    }
}

fn explosion_position_random(random: u32) -> (u16, u16) {
    let angle = (random >> 16) as u8 as u16;
    let mut distance = ((random as u8 % 8) as u16) << abi::ONE_SHIFT;
    if ((random >> 8) as u8) < (256 / 3) as u8 {
        distance += 8 << abi::ONE_SHIFT;
    }
    (angle, distance)
}

fn explosion_velocity_random(random: u32) -> (u16, u16) {
    (
        random as u8 as u16,
        (((random >> 8) as u8 % 5) as u16) << (abi::ONE_SHIFT + abi::VELOCITY_SHIFT),
    )
}

fn wrap_coord(value: i16, width: i32) -> i16 {
    let v = value as i32;
    (if v < 0 {
        v + width
    } else if v >= width {
        v - width
    } else {
        v
    }) as u16 as i16
}

const ION_COLORS: [(u8, u8, u8); 12] = [
    (31, 21, 0),
    (31, 17, 0),
    (31, 14, 0),
    (31, 10, 0),
    (31, 7, 0),
    (31, 3, 0),
    (31, 0, 0),
    (27, 0, 0),
    (23, 0, 0),
    (19, 0, 0),
    (15, 0, 0),
    (11, 0, 0),
];
const FLEE_COLORS: [(u8, u8, u8); 20] = [
    (10, 0, 0),
    (14, 0, 0),
    (19, 0, 0),
    (23, 0, 0),
    (27, 0, 0),
    (31, 0, 0),
    (31, 4, 4),
    (31, 10, 10),
    (31, 15, 15),
    (31, 19, 19),
    (31, 25, 25),
    (31, 19, 19),
    (31, 15, 15),
    (31, 10, 10),
    (31, 4, 4),
    (31, 0, 0),
    (27, 0, 0),
    (23, 0, 0),
    (19, 0, 0),
    (14, 0, 0),
];

/// One cycle step, retaining the BYTE decrements and half-speed rounding.
pub fn flee_cycle(index: u8, thrust: u8) -> (u8, u8, u8) {
    let next = index.wrapping_add(1);
    let next = if next == FLEE_COLORS.len() as u8 {
        0
    } else {
        next
    };
    let thrust = if next == 0 {
        thrust.wrapping_sub(1)
    } else {
        thrust
    };
    let wait = if thrust != 0 {
        ((thrust - 1) >> 1) + 1
    } else if next != 10 {
        1
    } else {
        0
    };
    (next, thrust, wait)
}

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    use super::*;
    use crate::battle::battle_native::engine::{activity, player_input, rgb15, set_activity};
    use crate::battle::battle_native::BattleSong;
    use crate::battle::encount_native::{FleetIsInfinite, UpdateShipFragCrew};
    use crate::engine::battle::views::SharedFrames;
    use crate::engine::battle::{Cx, ElementFn, ElementId, Parent};
    use crate::ships::native::{cosine, facing_to_angle, sine, starship};
    use abi::*;
    use std::{mem, ptr};

    static mut DITTY_IS_PLAYING: BOOLEAN = 0;
    pub(crate) static mut WINNER_STAR_SHIP: *mut STARSHIP = ptr::null_mut();

    use crate::battle::world_elements::engine::crew_preprocess;

    use crate::battle::ship_native::engine::animation_preprocess;

    use crate::battle::status_native::engine::PostProcessStatus;

    use crate::battle::ship_native::engine::ship_preprocess;

    use crate::battle::ship_native::engine::ship_postprocess;

    extern "C" {}

    fn same_callback(a: ElementProcessFunc, b: ElementFn) -> bool {
        a.map(|f| f.address()) == Some(b as usize)
    }

    /// # Safety
    /// `test` must identify a live starship; the display queue and attached ships must be valid on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn OpponentAlive(test: *mut STARSHIP) -> BOOLEAN {
        let mut h = disp_q.head;
        while !h.is_null() {
            let e = h.cast::<ELEMENT>();
            h = (*e).succ;
            let ship = starship(e);
            if !ship.is_null() && ship != test && (*(*ship).RaceDescPtr).ship_info.crew_level == 0 {
                return 0;
            }
        }
        1
    }

    pub(crate) unsafe fn PlayDitty(ship: *mut STARSHIP) {
        PlayMusic((*(*ship).RaceDescPtr).ship_data.victory_ditty, 0, 3);
        DITTY_IS_PLAYING = 1;
    }

    /// # Safety
    /// The game thread must own initialized audio and battle music state.
    #[no_mangle]
    pub unsafe extern "C" fn StopDitty() {
        if DITTY_IS_PLAYING != 0 {
            StopMusic();
        }
        DITTY_IS_PLAYING = 0;
    }

    unsafe fn DittyPlaying() -> bool {
        if DITTY_IS_PLAYING == 0 {
            return false;
        }
        DITTY_IS_PLAYING = PLRPlaying(usize::MAX as MUSIC_REF);
        DITTY_IS_PLAYING != 0
    }

    /// # Safety
    /// The game thread must exclusively own battle winner state.
    #[no_mangle]
    pub unsafe extern "C" fn ResetWinnerStarShip() {
        WINNER_STAR_SHIP = ptr::null_mut();
    }

    /// # Safety
    /// The caller must run on the game thread as part of battle-end input processing.
    #[no_mangle]
    pub unsafe extern "C" fn battleEndReadyHuman(_context: *mut HumanInputContext) -> bool {
        true
    }
    /// # Safety
    /// The caller must run on the game thread as part of battle-end input processing.
    #[no_mangle]
    pub unsafe extern "C" fn battleEndReadyComputer(_context: *mut ComputerInputContext) -> bool {
        true
    }
    /// # Safety
    /// `context` and its player connection must be live on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn battleEndReadyNetwork(context: *mut NetworkInputContext) -> bool {
        (*context)
            .net
            .as_ref()
            .unwrap_or_else(|| panic!("network input has no session"))
            .end_ready((*context).playerNr as usize)
    }

    unsafe fn readyForBattleEnd() -> bool {
        if DittyPlaying() {
            return false;
        }
        for side in 0..NUM_PLAYERS as usize {
            let ctx = player_input(side);
            if !((*(*ctx).handlers).battleEndReady.unwrap())(ctx) {
                return false;
            }
        }
        true
    }

    pub(crate) fn preprocess_dead_ship(cx: &mut Cx<'_>, _ship: ElementId) {
        cx.silence_battle_sounds();
    }

    /// # Safety
    /// `dead` must be a live dying ship element with an attached starship; its display queue and audio state must be initialized.
    pub fn cleanup_dead_ship(cx: &mut Cx<'_>, dead: ElementId) {
        cx.silence_battle_sounds();
        let dead_ship = cx.owner(dead);
        cx.ship_state_mut(dead_ship).crew = cx.desc(dead_ship).ship_info.crew_level;
        let mut music_started = false;
        let mut next = cx.elements().head();
        while let Some(e) = next {
            next = cx.elements().succ(e);
            let parent = cx.parent(e);
            if parent == Parent::Ship(dead_ship) {
                cx.set_parent(e, None);
                if cx.element(e).flags & CREW_OBJECT as u16 == 0
                    || !same_callback(cx.element(e).pre, crew_preprocess)
                {
                    cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as u8;
                    cx.element_mut(e).set_life_span(0);
                    cx.element_mut(e).flags = (NONSOLID | DISAPPEARING | FINITE_LIFE) as u16;
                    cx.element_mut(e).pre = None;
                    cx.element_mut(e).post = None;
                    cx.element_mut(e).death = None;
                    cx.element_mut(e).collide = None;
                }
            }
            if let Parent::Ship(ship) = parent {
                if cx.ship(ship).cur_status_flags & PLAY_VICTORY_DITTY as u16 != 0 {
                    music_started = true;
                    cx.play_ditty(ship);
                    cx.ship_state_mut(ship).status &= !(PLAY_VICTORY_DITTY as u16);
                }
            }
        }
        let life = cleanup_lifetime(
            minimum_ditty_frames(ONE_SECOND),
            music_started,
            Some(dead_ship) == cx.winner(),
        );
        cx.element_mut(dead).set_life_span(life);
        cx.element_mut(dead).death = Some(ElementHook::Native(new_ship));
        cx.element_mut(dead).pre = Some(ElementHook::Native(preprocess_dead_ship));
        cx.element_mut(dead).flags &= !(DISAPPEARING as u16);
        cx.set_parent(dead, Some(dead_ship));
    }

    unsafe fn setMinShipLifeSpan(ship: *mut ELEMENT, life: COUNT) {
        if same_callback((*ship).death_func, new_ship) {
            assert_ne!((*ship).state_flags & FINITE_LIFE as u16, 0);
            assert_eq!((*ship).state_flags & DISAPPEARING as u16, 0);
            if (*ship).__bindgen_anon_1.life_span < life {
                (*ship).__bindgen_anon_1.life_span = life;
            }
        }
    }

    unsafe fn setMinStarShipLifeSpan(ship: *mut STARSHIP, life: COUNT) {
        setMinShipLifeSpan((*ship).hShip.cast(), life);
    }

    unsafe fn checkOtherShipLifeSpan(dead: *mut ELEMENT) {
        let owner = starship(dead);
        if !WINNER_STAR_SHIP.is_null()
            && owner != WINNER_STAR_SHIP
            && (*(*WINNER_STAR_SHIP).RaceDescPtr).ship_info.crew_level == 0
        {
            setMinStarShipLifeSpan(
                WINNER_STAR_SHIP,
                (*dead).__bindgen_anon_1.life_span.wrapping_add(1),
            );
        } else if WINNER_STAR_SHIP.is_null() {
            let mut h = disp_q.head;
            while !h.is_null() {
                let e = h.cast::<ELEMENT>();
                h = (*e).succ;
                let ship = starship(e);
                if !ship.is_null() && e != dead && (*(*ship).RaceDescPtr).ship_info.crew_level == 0
                {
                    setMinShipLifeSpan(e, (*dead).__bindgen_anon_1.life_span);
                }
            }
        }
    }

    pub fn new_ship(cx: &mut Cx<'_>, dead: ElementId) {
        cx.replace_dead_ship(dead);
    }

    /// Run the replacement transition with an explicit selection/spawn boundary.
    ///
    /// # Safety
    /// As for `new_ship`. The picker must return the next-ship result for the
    /// supplied player and must not retain the outgoing ship pointer.
    pub unsafe fn new_ship_with_picker(
        cx: &mut Cx<'_>,
        dead: *mut ELEMENT,
        pick: impl FnOnce(*mut STARSHIP, COUNT) -> BOOLEAN,
    ) {
        let net = cx.b.net.clone();
        let ship = starship(dead);
        if !readyForBattleEnd() {
            (*dead).state_flags &= !(DISAPPEARING as u16);
            (*dead).__bindgen_anon_1.life_span = (*dead).__bindgen_anon_1.life_span.wrapping_add(1);
            checkOtherShipLifeSpan(dead);
            return;
        }
        WINNER_STAR_SHIP = ptr::null_mut();
        StopDitty();
        StopMusic();
        StopSound();
        (*dead).pParent = ptr::null_mut();
        let restart = OpponentAlive(ship) != 0;
        free_ship((*ship).RaceDescPtr, 1, 1);
        (*ship).RaceDescPtr = ptr::null_mut();
        UnbatchGraphics();
        if let Some(net) = &net {
            net.init_end_frames();
        }
        if net
            .as_ref()
            .is_some_and(|net| !net.negotiate(true, NetState_NetState_interBattle))
        {
            set_activity(activity() & !(IN_BATTLE as u16));
            BatchGraphics();
            return;
        }
        if FleetIsInfinite((*ship).playerNr as COUNT) == 0 {
            UpdateShipFragCrew(ship);
            (*ship).SpeciesID = SPECIES_ID_NO_ID;
        }
        if crate::app::bridge::publish(cx, || pick(ship, (*ship).playerNr as COUNT)) != 0 {
            if net
                .as_ref()
                .is_some_and(|net| !net.negotiate(true, NetState_NetState_inBattle))
            {
                set_activity(activity() & !(IN_BATTLE as u16));
                BatchGraphics();
                return;
            }
            if restart {
                BattleSong(1);
            }
        } else if cx.b.counters.contains(&0) {
            set_activity(activity() & !(IN_BATTLE as u16));
        } else {
            set_activity(activity() | CHECK_ABORT as u16);
        }
        BatchGraphics();
    }

    pub(crate) fn explosion_preprocess(cx: &mut Cx<'_>, ship: ElementId) {
        let tick =
            ((NUM_EXPLOSION_FRAMES * 3) as u16).wrapping_sub(cx.element(ship).life_span()) as u8;
        let step = explosion_step(tick);
        if step.clear_preprocess {
            cx.element_mut(ship).pre = None;
        }
        if step.hide_primitive {
            cx.primitive_style_mut(ship).kind = gfx_object_NUM_PRIMS as u8;
            cx.element_mut(ship).flags |= CHANGING as u16;
        }
        for _ in 0..step.fragments {
            let Some(e) = cx.alloc_element() else {
                continue;
            };
            cx.put(e);
            cx.element_mut(e).player = NEUTRAL_PLAYER_NUM as i16;
            cx.element_mut(e).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            cx.element_mut(e).set_life_span(9);
            cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as u8;
            let array = cx.shared_frame_set(SharedFrames::Explosion);
            cx.set_frames(e, false, array);
            let frame = cx.first_shared_frame(SharedFrames::Explosion);
            cx.set_frame(e, false, frame);
            let (angle, dist) = explosion_position_random(cx.random());
            let location = cx.element(ship).current.location;
            cx.element_mut(e).current.location.x =
                location.x.wrapping_add(cosine(angle as i32, dist as i32));
            cx.element_mut(e).current.location.y =
                location.y.wrapping_add(sine(angle as i32, dist as i32));
            cx.element_mut(e).pre = Some(ElementHook::Native(animation_preprocess));
            let (angle, dist) = explosion_velocity_random(cx.random());
            cx.element_mut(e).velocity.set_components(
                i32::from(cosine(angle as i32, dist as i32)),
                i32::from(sine(angle as i32, dist as i32)),
            );
        }
    }

    /// # Safety
    /// The game thread must own initialized audio and battle music state.
    #[no_mangle]
    pub unsafe extern "C" fn StopAllBattleMusic() {
        StopDitty();
        StopMusic();
    }

    /// # Safety
    /// `dead` must identify the excluded element; all display queue ship attachments must be valid on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn FindAliveStarShip(dead: *mut ELEMENT) -> *mut STARSHIP {
        let mut alive = ptr::null_mut();
        let mut h = disp_q.head;
        while !h.is_null() {
            let e = h.cast::<ELEMENT>();
            if (*e).state_flags & PLAYER_SHIP as u16 != 0
                && e != dead
                && (*e).__bindgen_anon_3.mass_points as u32 <= MAX_SHIP_MASS + 1
            {
                alive = starship(e);
                assert!(!alive.is_null());
                if (*(*alive).RaceDescPtr).ship_info.crew_level == 0
                    && (*e).__bindgen_anon_3.mass_points as u32 != MAX_SHIP_MASS + 1
                {
                    alive = ptr::null_mut();
                }
                break;
            }
            h = (*e).succ;
        }
        alive
    }

    /// # Safety
    /// The caller must access winner state only on the game thread and not retain the result past ship teardown.
    #[no_mangle]
    pub unsafe extern "C" fn GetWinnerStarShip() -> *mut STARSHIP {
        WINNER_STAR_SHIP
    }
    /// # Safety
    /// A non-null `winner` must be a live starship that remains live until winner state is reset.
    #[no_mangle]
    pub unsafe extern "C" fn SetWinnerStarShip(winner: *mut STARSHIP) {
        if winner.is_null() {
            return;
        }
        (*winner).cur_status_flags |= PLAY_VICTORY_DITTY as u16;
        if WINNER_STAR_SHIP.is_null() {
            WINNER_STAR_SHIP = winner;
        }
    }
    /// # Safety
    /// `dead` must be a live ship with a valid owner, player index, and battle counter on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn RecordShipDeath(dead: *mut ELEMENT) {
        // SAFETY: the legacy ship element is checked before entering native behavior.
        crate::app::bridge::enter(|cx| {
            let id = cx
                .elements()
                .id(dead.cast())
                .unwrap_or_else(|| panic!("{}", "null dead ship"));
            cx.record_ship_death(id);
        });
    }

    /// # Safety
    /// `ship` must be a live ship with a valid owner, race descriptor, primitive, and initialized sound resources.
    #[no_mangle]
    pub unsafe extern "C" fn StartShipExplosion(ship: *mut ELEMENT, play_sound: bool) {
        let owner = starship(ship);
        (*ship).velocity = mem::zeroed();
        DeltaEnergy(
            ship,
            -((*(*owner).RaceDescPtr).ship_info.energy_level as i16),
        );
        (*ship).__bindgen_anon_1.life_span = (NUM_EXPLOSION_FRAMES * 3) as u16;
        (*ship).state_flags &= !(DISAPPEARING as u16);
        (*ship).state_flags |= (FINITE_LIFE | NONSOLID) as u16;
        (*ship).preprocess_func = Some(ElementHook::Native(explosion_preprocess));
        (*ship).postprocess_func = Some(ElementHook::Native(PostProcessStatus));
        (*ship).death_func = Some(ElementHook::Native(cleanup_dead_ship));
        (*ship).hTarget = ptr::null_mut();
        if play_sound {
            PlaySound(
                SetAbsStringTableIndex(GameSounds, 1),
                CalcSoundPosition(ship),
                ship,
                (GAME_SOUND_PRIORITY + 1) as u8,
            );
        }
    }
    pub fn ship_death(cx: &mut Cx<'_>, ship: ElementId) {
        let owner = cx.owner(ship);
        cx.stop_battle_music();
        cx.ship_state_mut(owner).status &= !(PLAY_VICTORY_DITTY as u16);
        cx.start_ship_explosion(ship, true);
        cx.select_winner(ship);
        cx.record_ship_death(ship);
    }

    pub(crate) fn cycle_ion_trail(cx: &mut Cx<'_>, e: ElementId) {
        assert_eq!(cx.element(e).flags & PLAYER_SHIP as u16, 0);
        cx.element_mut(e).color_cycle = cx.element(e).color_cycle.wrapping_add(1);
        if cx.element(e).color_cycle as usize != ION_COLORS.len() {
            let life = cx.element(e).thrust_wait() as u16;
            cx.element_mut(e).set_life_span(life);
            let (r, g, b) = ION_COLORS[cx.element(e).color_cycle as usize];
            cx.primitive_style_mut(e).color = rgb15(r, g, b);
            cx.element_mut(e).flags &= !(DISAPPEARING as u16);
            cx.element_mut(e).flags |= CHANGING as u16;
        }
    }

    pub fn ship_transition(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).flags & PLAYER_SHIP as u16 != 0 {
            if cx.element(e).flags & APPEARING as u16 != 0 {
                cx.element_mut(e).set_life_span(HYPERJUMP_LIFE as u16);
                cx.element_mut(e).pre = Some(ElementHook::Native(ship_transition));
                cx.element_mut(e).post = None;
                cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as u8;
                cx.element_mut(e).flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
            } else if cx.element(e).life_span() < HYPERJUMP_LIFE as u16 {
                if cx.element(e).life_span() == NORMAL_LIFE as u16
                    && cx.element(e).hit_points() != 0
                {
                    let frame = cx.equivalent_frame(
                        cx.element_frame_set_first(e, false),
                        cx.element(e).current.image.frame,
                    );
                    cx.set_frame(e, false, frame);
                    cx.set_frame(e, true, frame);
                    cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as u8;
                    cx.init_intersect(e);
                    let frame = cx.equivalent_frame(
                        cx.element_frame_set_first(e, true),
                        cx.element(e).next.image.frame,
                    );
                    cx.set_intersect_frame(e, frame);
                    cx.element_mut(e).velocity.zero();
                    cx.element_mut(e).flags &= !((NONSOLID | FINITE_LIFE) as u16);
                    cx.element_mut(e).flags |= CHANGING as u16;
                    cx.element_mut(e).pre = Some(ElementHook::Native(ship_preprocess));
                    cx.element_mut(e).post = Some(ElementHook::Native(ship_postprocess));
                }
                return;
            }
        }
        let owner = cx.owner(e);
        let original = cx.ship_element(owner);
        if cx.element(original).flags & NONSOLID as u16 == 0 {
            cx.element_mut(e).pre = None;
        } else if let Some(image) = cx.alloc_element() {
            cx.put(image);
            let angle = facing_to_angle(cx.ship(owner).ShipFacing as i32);
            cx.element_mut(image).player = NEUTRAL_PLAYER_NUM as i16;
            cx.element_mut(image).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            cx.element_mut(image).set_thrust_wait(1);
            cx.element_mut(image).set_life_span(1);
            cx.primitive_style_mut(image).kind = gfx_object_STAMPFILL_PRIM as u8;
            cx.primitive_style_mut(image).color = rgb15(31, 21, 0);
            cx.element_mut(image).color_cycle = 0;
            let current_image = cx.element(e).current.image;
            cx.copy_image_to_current(image, current_image);
            cx.element_mut(image).current.location = cx.element(e).current.location;
            let x = cosine(angle, 40 << ONE_SHIFT) as i32;
            let y = sine(angle, 40 << ONE_SHIFT) as i32;
            if cx.element(e).flags & PLAYER_SHIP as u16 == 0 {
                cx.element_mut(image).current.location.x =
                    (cx.element(image).current.location.x as i32 + x) as i16;
                cx.element_mut(image).current.location.y =
                    (cx.element(image).current.location.y as i32 + y) as i16;
                cx.element_mut(e).pre = None;
            } else if cx.element(e).hit_points() != 0 {
                let life = cx.element(e).life_span() as i32 - 1;
                let (w, h) = cx.screen_dimensions();
                cx.element_mut(image).current.location.x =
                    (cx.element(image).current.location.x as i32 - x * life) as i16;
                cx.element_mut(image).current.location.y =
                    (cx.element(image).current.location.y as i32 - y * life) as i16;
                cx.element_mut(image).current.location.x = wrap_coord(
                    cx.element(image).current.location.x,
                    (w - 64) << (ONE_SHIFT + MAX_REDUCTION),
                );
                cx.element_mut(image).current.location.y = wrap_coord(
                    cx.element(image).current.location.y,
                    h << (ONE_SHIFT + MAX_REDUCTION),
                );
            }
            cx.element_mut(image).pre = Some(ElementHook::Native(ship_transition));
            cx.element_mut(image).death = Some(ElementHook::Native(cycle_ion_trail));
            cx.set_parent(image, Some(owner));
        }
    }

    pub fn flee_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let wait = cx.element(e).turn_wait().wrapping_sub(1);
        cx.element_mut(e).set_turn_wait(wait);
        if wait == 0 {
            let (index, thrust, wait) =
                flee_cycle(cx.element(e).color_cycle, cx.element(e).thrust_wait());
            cx.element_mut(e).color_cycle = index;
            let (r, g, b) = FLEE_COLORS[index as usize];
            cx.primitive_style_mut(e).color = rgb15(r, g, b);
            cx.element_mut(e).set_thrust_wait(thrust);
            cx.element_mut(e).set_turn_wait(wait);
            if wait == 0 {
                cx.element_mut(e).death = Some(ElementHook::Native(cleanup_dead_ship));
                cx.element_mut(e).set_hit_points(0);
                cx.element_mut(e).set_life_span((HYPERJUMP_LIFE + 1) as u16);
                cx.element_mut(e).pre = Some(ElementHook::Native(ship_transition));
                cx.element_mut(e).post = None;
                cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as u8;
                cx.element_mut(e).flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
            }
        }
        let ship = cx.owner(e);
        cx.ship_state_mut(ship).status &= !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16);
        cx.preprocess_status(e);
    }
}

#[cfg(not(test))]
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explosion_random_words_keep_byte_selection_and_threshold() {
        for low in 0u32..=65535 {
            let random = 0xabfe0000 | low;
            let position = explosion_position_random(random);
            let expected_distance = ((low & 0xff) % 8) * 4 + if (low >> 8) < 85 { 32 } else { 0 };
            assert_eq!(position, (254, expected_distance as u16));
            assert_eq!(
                explosion_velocity_random(random),
                ((low & 255) as u16, (((low >> 8) % 5) * 128) as u16)
            );
        }
        assert_eq!(explosion_position_random(0xff3f5407), (63, 60));
        assert_eq!(explosion_position_random(0xff3f5507), (63, 28));
    }

    #[test]
    fn warp_coordinates_adjust_once_after_coord_truncation() {
        for value in i16::MIN..=i16::MAX {
            let promoted = value as i32;
            let expected = if promoted < 0 {
                promoted + 8192
            } else if promoted >= 8192 {
                promoted - 8192
            } else {
                promoted
            };
            assert_eq!(wrap_coord(value, 8192), expected as u16 as i16);
        }
        assert_eq!(wrap_coord(20000, 8192), 11808);
        assert_eq!(wrap_coord(-8192, 8192), 0);
    }

    #[test]
    fn byte_counters_wrap_before_flee_wait_rounding() {
        assert_eq!(flee_cycle(19, 0), (0, 255, 128));
        assert_eq!(flee_cycle(9, 0), (10, 0, 0));
        assert_eq!(flee_cycle(0, 4), (1, 4, 2));
        assert_eq!(cleanup_lifetime(u16::MAX, true, false), 0);
        assert_eq!(cleanup_lifetime(u16::MAX, false, true), 1);
        assert_eq!(ION_COLORS.len(), 12);
        assert_eq!(FLEE_COLORS[10], (31, 25, 25));
        assert_eq!(FLEE_COLORS[14], (31, 4, 4));
    }

    #[test]
    fn ditty_frames_and_winner_order_match_c() {
        let min = minimum_ditty_frames(840);
        assert_eq!(min, 72);
        assert_eq!(cleanup_lifetime(min, false, false), 2);
        assert_eq!(cleanup_lifetime(min, true, false), 73);
        assert_eq!(cleanup_lifetime(min, false, true), 74);
    }
    #[test]
    fn explosion_schedule_includes_switch_fallthrough() {
        let expected = [
            1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 1, 1, 1, 1, 1, 1,
        ];
        for (tick, fragments) in expected.into_iter().enumerate() {
            let step = explosion_step(tick as u8);
            assert_eq!(step.fragments, fragments);
            assert_eq!(step.hide_primitive, tick == 15);
            assert_eq!(step.clear_preprocess, tick == 25);
        }
    }
    #[test]
    fn fleeing_finishes_at_brightest_color_after_four_complete_cycles() {
        let (mut index, mut thrust) = (0, 4);
        for step in 1..=90 {
            let (i, t, w) = flee_cycle(index, thrust);
            index = i;
            thrust = t;
            assert_eq!(w == 0, step == 90);
        }
        assert_eq!((index, thrust), (10, 0));
    }
}
