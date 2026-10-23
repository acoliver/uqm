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

#[cfg(any(not(test), feature = "linked_c_archive"))]
#[allow(non_snake_case)]
mod engine {
    use super::*;
    use crate::battle::battle_native::engine::{
        activity, connection, player_input, primitive, rgb15, set_activity,
    };
    use crate::battle::battle_native::{battleFrameCount, battle_counter, BattleSong};
    use crate::battle::encount_native::{FleetIsInfinite, UpdateShipFragCrew};
    use crate::ships::native::{
        cosine, facing_to_angle, init_intersect_points, put_element, sine, starship,
    };
    use abi::*;
    use std::ffi::c_void;
    use std::{mem, ptr};

    static mut DITTY_IS_PLAYING: BOOLEAN = 0;
    static mut WINNER_STAR_SHIP: *mut STARSHIP = ptr::null_mut();

    // netmisc.h: the common state-data prefix is a release callback.
    #[repr(C)]
    struct BattleStateData {
        releaseFunction: NetConnectionStateData_ReleaseFunction,
        meleeState: *mut c_void,
        battleState: *mut battlestate_struct,
        getMeleeState: *mut getmelee_struct,
        endFrameCount: BattleFrameCounter,
    }

    extern "C" {
        fn Netplay_localReady(
            conn: *mut NetConnection,
            callback: Option<unsafe extern "C" fn(*mut NetConnection, *mut c_void)>,
            arg: *mut c_void,
            notify: bool,
        ) -> bool;
        fn Netplay_isLocalReady(conn: *const NetConnection) -> bool;
        fn Netplay_Notify_frameCount(conn: *mut NetConnection, count: BattleFrameCounter);
        fn flushPacketQueue(conn: *mut NetConnection);
        fn crew_preprocess(e: *mut ELEMENT);
        fn animation_preprocess(e: *mut ELEMENT);
        fn PostProcessStatus(e: *mut ELEMENT);
        fn PreProcessStatus(e: *mut ELEMENT);
        fn ship_preprocess(e: *mut ELEMENT);
        fn ship_postprocess(e: *mut ELEMENT);
        static mut explosion: [FRAME; 3];
    }

    fn same_callback(a: ElementProcessFunc, b: unsafe extern "C" fn(*mut ELEMENT)) -> bool {
        a.map(|f| f as usize) == Some(b as usize)
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

    unsafe fn PlayDitty(ship: *mut STARSHIP) {
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

    unsafe extern "C" fn readyToEnd2Callback(conn: *mut NetConnection, _arg: *mut c_void) {
        NetConnection_setState(conn, NetState_NetState_endingBattle2);
    }

    unsafe extern "C" fn readyToEndCallback(conn: *mut NetConnection, _arg: *mut c_void) {
        let data = NetConnection_getStateData(conn).cast::<BattleStateData>();
        NetConnection_setState(conn, NetState_NetState_endingBattle);
        let target = battleFrameCount.wrapping_add(1);
        if target > (*data).endFrameCount {
            (*data).endFrameCount = target;
        }
        Netplay_Notify_frameCount(conn, target);
        flushPacketQueue(conn);
        Netplay_localReady(conn, Some(readyToEnd2Callback), ptr::null_mut(), false);
    }

    unsafe fn readyForBattleEndPlayer(conn: *mut NetConnection) -> bool {
        let data = NetConnection_getStateData(conn).cast::<BattleStateData>();
        let state = NetConnection_getState(conn);
        if state == NetState_NetState_interBattle || state == NetState_NetState_inSetup {
            return true;
        }
        if state == NetState_NetState_inBattle {
            if Netplay_isLocalReady(conn) {
                return false;
            }
            Netplay_localReady(conn, Some(readyToEndCallback), ptr::null_mut(), true);
            if NetConnection_getState(conn) == NetState_NetState_inBattle {
                return false;
            }
        }
        let state = NetConnection_getState(conn);
        assert!(
            state == NetState_NetState_endingBattle || state == NetState_NetState_endingBattle2
        );
        if battleFrameCount < (*data).endFrameCount {
            return false;
        }
        if NetConnection_getState(conn) == NetState_NetState_endingBattle {
            waitReady(conn);
            assert_eq!(
                NetConnection_getState(conn),
                NetState_NetState_endingBattle2
            );
            if battleFrameCount < (*data).endFrameCount {
                return false;
            }
        }
        negotiateReady(conn, true, NetState_NetState_interBattle);
        true
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
        readyForBattleEndPlayer(connection((*context).playerNr as usize))
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

    unsafe extern "C" fn preprocess_dead_ship(_ship: *mut ELEMENT) {
        ProcessSound(usize::MAX as SOUND, ptr::null_mut());
    }

    /// # Safety
    /// `dead` must be a live dying ship element with an attached starship; its display queue and audio state must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn cleanup_dead_ship(dead: *mut ELEMENT) {
        ProcessSound(usize::MAX as SOUND, ptr::null_mut());
        let dead_ship = starship(dead);
        (*dead_ship).crew_level = (*(*dead_ship).RaceDescPtr).ship_info.crew_level;
        let mut music_started = false;
        let mut h = disp_q.head;
        while !h.is_null() {
            let e = h.cast::<ELEMENT>();
            h = (*e).succ;
            let ship = starship(e);
            if ship == dead_ship {
                (*e).pParent = ptr::null_mut();
                if (*e).state_flags & CREW_OBJECT as u16 == 0
                    || !same_callback((*e).preprocess_func, crew_preprocess)
                {
                    (*primitive(e)).Type = gfx_object_NUM_PRIMS as u8;
                    (*e).__bindgen_anon_1.life_span = 0;
                    (*e).state_flags = (NONSOLID | DISAPPEARING | FINITE_LIFE) as u16;
                    (*e).preprocess_func = None;
                    (*e).postprocess_func = None;
                    (*e).death_func = None;
                    (*e).collision_func = None;
                }
            }
            if !ship.is_null() && (*ship).cur_status_flags & PLAY_VICTORY_DITTY as u16 != 0 {
                music_started = true;
                PlayDitty(ship);
                (*ship).cur_status_flags &= !(PLAY_VICTORY_DITTY as u16);
            }
        }
        (*dead).__bindgen_anon_1.life_span = cleanup_lifetime(
            minimum_ditty_frames(ONE_SECOND),
            music_started,
            dead_ship == WINNER_STAR_SHIP,
        );
        (*dead).death_func = Some(new_ship);
        (*dead).preprocess_func = Some(preprocess_dead_ship);
        (*dead).state_flags &= !(DISAPPEARING as u16);
        (*dead).pParent = dead_ship.cast();
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

    /// # Safety
    /// `dead` must be a live ship awaiting replacement with a valid starship and initialized battle, fleet, and network state.
    #[no_mangle]
    pub unsafe extern "C" fn new_ship(dead: *mut ELEMENT) {
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
        initBattleStateDataConnections();
        if !negotiateReadyConnections(true, NetState_NetState_interBattle) {
            set_activity(activity() & !(IN_BATTLE as u16));
            BatchGraphics();
            return;
        }
        if FleetIsInfinite((*ship).playerNr as COUNT) == 0 {
            UpdateShipFragCrew(ship);
            (*ship).SpeciesID = SPECIES_ID_NO_ID;
        }
        if GetNextStarShip(ship, (*ship).playerNr as COUNT) != 0 {
            if !negotiateReadyConnections(true, NetState_NetState_inBattle) {
                set_activity(activity() & !(IN_BATTLE as u16));
                BatchGraphics();
                return;
            }
            if restart {
                BattleSong(1);
            }
        } else if battle_counter[0] == 0 || battle_counter[1] == 0 {
            set_activity(activity() & !(IN_BATTLE as u16));
        } else {
            set_activity(activity() | CHECK_ABORT as u16);
        }
        BatchGraphics();
    }

    unsafe extern "C" fn explosion_preprocess(ship: *mut ELEMENT) {
        let tick = ((NUM_EXPLOSION_FRAMES * 3) as u16)
            .wrapping_sub((*ship).__bindgen_anon_1.life_span) as u8;
        let step = explosion_step(tick);
        if step.clear_preprocess {
            (*ship).preprocess_func = None;
        }
        if step.hide_primitive {
            (*primitive(ship)).Type = gfx_object_NUM_PRIMS as u8;
            (*ship).state_flags |= CHANGING as u16;
        }
        for _ in 0..step.fragments {
            let h = AllocElement();
            if h.is_null() {
                continue;
            }
            put_element(h);
            let e = h.cast::<ELEMENT>();
            (*e).playerNr = NEUTRAL_PLAYER_NUM as i16;
            (*e).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            (*e).__bindgen_anon_1.life_span = 9;
            (*primitive(e)).Type = gfx_object_STAMP_PRIM as u8;
            (*e).current.image.farray = (&raw mut explosion).cast();
            (*e).current.image.frame = explosion[0];
            let (angle, dist) = explosion_position_random(TFB_Random());
            (*e).current.location.x = (*ship)
                .current
                .location
                .x
                .wrapping_add(cosine(angle as i32, dist as i32));
            (*e).current.location.y = (*ship)
                .current
                .location
                .y
                .wrapping_add(sine(angle as i32, dist as i32));
            (*e).preprocess_func = Some(animation_preprocess);
            let (angle, dist) = explosion_velocity_random(TFB_Random());
            SetVelocityComponents(
                &mut (*e).velocity,
                cosine(angle as i32, dist as i32),
                sine(angle as i32, dist as i32),
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
        let owner = starship(dead);
        assert!(!owner.is_null());
        if (*dead).__bindgen_anon_3.mass_points as u32 <= MAX_SHIP_MASS {
            assert!((*owner).playerNr >= 0);
            let count = &raw mut battle_counter[(*owner).playerNr as usize];
            *count = (*count).wrapping_sub(1);
        }
        if activity() as u8 == SUPER_MELEE as u8 {
            MeleeShipDeath(owner);
        }
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
        (*ship).preprocess_func = Some(explosion_preprocess);
        (*ship).postprocess_func = Some(PostProcessStatus);
        (*ship).death_func = Some(cleanup_dead_ship);
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
    /// # Safety
    /// `ship` must be a live ship with valid battle, audio, display, and ownership state on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn ship_death(ship: *mut ELEMENT) {
        let owner = starship(ship);
        StopAllBattleMusic();
        (*owner).cur_status_flags &= !(PLAY_VICTORY_DITTY as u16);
        StartShipExplosion(ship, true);
        SetWinnerStarShip(FindAliveStarShip(ship));
        RecordShipDeath(ship);
    }

    unsafe extern "C" fn cycle_ion_trail(e: *mut ELEMENT) {
        assert_eq!((*e).state_flags & PLAYER_SHIP as u16, 0);
        (*e).colorCycleIndex = (*e).colorCycleIndex.wrapping_add(1);
        if (*e).colorCycleIndex as usize != ION_COLORS.len() {
            (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_5.thrust_wait as u16;
            let (r, g, b) = ION_COLORS[(*e).colorCycleIndex as usize];
            (*primitive(e)).color = rgb15(r, g, b);
            (*e).state_flags &= !(DISAPPEARING as u16);
            (*e).state_flags |= CHANGING as u16;
        }
    }

    /// # Safety
    /// `ship` must be a live player ship with a valid owner, race frames, primitive, and display queue.
    #[no_mangle]
    pub unsafe extern "C" fn spawn_ion_trail(ship: *mut ELEMENT) {
        assert_ne!((*ship).state_flags & PLAYER_SHIP as u16, 0);
        let h = AllocElement();
        if h.is_null() {
            return;
        }
        let owner = starship(ship);
        let angle = facing_to_angle((*owner).ShipFacing as i32) + HALF_CIRCLE as i32;
        let mut rect: RECT = mem::zeroed();
        GetFrameRect((*(*owner).RaceDescPtr).ship_data.ship[0], &mut rect);
        rect.extent.height = ((rect.extent.height as i32 + rect.corner.y as i32) << ONE_SHIFT) as _;
        InsertQueue(&raw mut disp_q, h, disp_q.head);
        let e = h.cast::<ELEMENT>();
        (*e).playerNr = NEUTRAL_PLAYER_NUM as i16;
        (*e).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
        (*e).__bindgen_anon_5.thrust_wait = 1;
        (*e).__bindgen_anon_1.life_span = 1;
        (*primitive(e)).Type = gfx_object_POINT_PRIM as u8;
        (*primitive(e)).color = rgb15(31, 21, 0);
        (*e).colorCycleIndex = 0;
        (*e).current.image.frame = DecFrameIndex(stars_in_space);
        (*e).current.image.farray = &raw mut stars_in_space;
        (*e).current.location = (*ship).current.location;
        (*e).current.location.x = (*e)
            .current
            .location
            .x
            .wrapping_add(cosine(angle, rect.extent.height as i32));
        (*e).current.location.y = (*e)
            .current
            .location
            .y
            .wrapping_add(sine(angle, rect.extent.height as i32));
        (*e).death_func = Some(cycle_ion_trail);
        (*e).pParent = owner.cast();
        (*e).next = (*e).current;
        (*e).__bindgen_anon_1.life_span = (*e).__bindgen_anon_1.life_span.wrapping_sub(1);
        (*e).state_flags |= PRE_PROCESS as u16;
    }

    /// # Safety
    /// `e` must be a live ship or transition image whose owner and original ship remain valid on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn ship_transition(e: *mut ELEMENT) {
        if (*e).state_flags & PLAYER_SHIP as u16 != 0 {
            if (*e).state_flags & APPEARING as u16 != 0 {
                (*e).__bindgen_anon_1.life_span = HYPERJUMP_LIFE as u16;
                (*e).preprocess_func = Some(ship_transition);
                (*e).postprocess_func = None;
                (*primitive(e)).Type = gfx_object_NUM_PRIMS as u8;
                (*e).state_flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
            } else if (*e).__bindgen_anon_1.life_span < HYPERJUMP_LIFE as u16 {
                if (*e).__bindgen_anon_1.life_span == NORMAL_LIFE as u16
                    && (*e).__bindgen_anon_2.crew_level != 0
                {
                    let frame =
                        SetEquFrameIndex(*(*e).current.image.farray, (*e).current.image.frame);
                    (*e).current.image.frame = frame;
                    (*e).next.image.frame = frame;
                    (*primitive(e)).Type = gfx_object_STAMP_PRIM as u8;
                    init_intersect_points(e);
                    (*e).IntersectControl.IntersectStamp.frame =
                        SetEquFrameIndex(*(*e).next.image.farray, (*e).next.image.frame);
                    (*e).velocity = mem::zeroed();
                    (*e).state_flags &= !((NONSOLID | FINITE_LIFE) as u16);
                    (*e).state_flags |= CHANGING as u16;
                    (*e).preprocess_func = Some(ship_preprocess);
                    (*e).postprocess_func = Some(ship_postprocess);
                }
                return;
            }
        }
        let owner = starship(e);
        let original = (*owner).hShip.cast::<ELEMENT>();
        if (*original).state_flags & NONSOLID as u16 == 0 {
            (*e).preprocess_func = None;
        } else {
            let h = AllocElement();
            if !h.is_null() {
                put_element(h);
                let angle = facing_to_angle((*owner).ShipFacing as i32);
                let image = h.cast::<ELEMENT>();
                (*image).playerNr = NEUTRAL_PLAYER_NUM as i16;
                (*image).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
                (*image).__bindgen_anon_5.thrust_wait = 1;
                (*image).__bindgen_anon_1.life_span = 1;
                (*primitive(image)).Type = gfx_object_STAMPFILL_PRIM as u8;
                (*primitive(image)).color = rgb15(31, 21, 0);
                (*image).colorCycleIndex = 0;
                (*image).current.image = (*e).current.image;
                (*image).current.location = (*e).current.location;
                let x = cosine(angle, 40 << ONE_SHIFT) as i32;
                let y = sine(angle, 40 << ONE_SHIFT) as i32;
                if (*e).state_flags & PLAYER_SHIP as u16 == 0 {
                    (*image).current.location.x = ((*image).current.location.x as i32 + x) as i16;
                    (*image).current.location.y = ((*image).current.location.y as i32 + y) as i16;
                    (*e).preprocess_func = None;
                } else if (*e).__bindgen_anon_2.crew_level != 0 {
                    let life = (*e).__bindgen_anon_1.life_span as i32 - 1;
                    (*image).current.location.x =
                        ((*image).current.location.x as i32 - x * life) as i16;
                    (*image).current.location.y =
                        ((*image).current.location.y as i32 - y * life) as i16;
                    (*image).current.location.x = wrap_coord(
                        (*image).current.location.x,
                        (ScreenWidth - 64) << (ONE_SHIFT + MAX_REDUCTION),
                    );
                    (*image).current.location.y = wrap_coord(
                        (*image).current.location.y,
                        ScreenHeight << (ONE_SHIFT + MAX_REDUCTION),
                    );
                }
                (*image).preprocess_func = Some(ship_transition);
                (*image).death_func = Some(cycle_ion_trail);
                (*image).pParent = owner.cast();
            }
        }
    }

    /// # Safety
    /// `e` must be a live fleeing ship with valid color-cycle state, owner, and display primitive.
    #[no_mangle]
    pub unsafe extern "C" fn flee_preprocess(e: *mut ELEMENT) {
        (*e).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_4.turn_wait.wrapping_sub(1);
        if (*e).__bindgen_anon_4.turn_wait == 0 {
            let (index, thrust, wait) =
                flee_cycle((*e).colorCycleIndex, (*e).__bindgen_anon_5.thrust_wait);
            (*e).colorCycleIndex = index;
            let (r, g, b) = FLEE_COLORS[index as usize];
            (*primitive(e)).color = rgb15(r, g, b);
            (*e).__bindgen_anon_5.thrust_wait = thrust;
            (*e).__bindgen_anon_4.turn_wait = wait;
            if wait == 0 {
                (*e).death_func = Some(cleanup_dead_ship);
                (*e).__bindgen_anon_2.crew_level = 0;
                (*e).__bindgen_anon_1.life_span = (HYPERJUMP_LIFE + 1) as u16;
                (*e).preprocess_func = Some(ship_transition);
                (*e).postprocess_func = None;
                (*primitive(e)).Type = gfx_object_NUM_PRIMS as u8;
                (*e).state_flags |= (NONSOLID | FINITE_LIFE | CHANGING) as u16;
            }
        }
        let ship = starship(e);
        (*ship).cur_status_flags &= !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16);
        PreProcessStatus(e);
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
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
