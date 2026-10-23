//! Debug cheats, universe inspection and graphics-context diagnostics.
#![allow(non_snake_case)]
#![cfg(feature = "debug-process")]

use crate::navigation::abi::*;
use std::ffi::{c_char, c_int};

unsafe fn clear_escort_queue(queue: *mut crate::collections::queue::Queue) {
    let mut handle = (*queue).head;
    while !handle.is_null() {
        let next = (*handle.cast::<SHIP_FRAGMENT>()).succ;
        crate::collections::queue::RemoveQueue(queue, handle);
        crate::collections::queue::FreeLink(queue, handle);
        handle = next;
    }
}

fn hsvaToRgba(hue: f64, sat: f64, val: f64, alpha: BYTE) -> Color {
    let hi = (hue / 60.0) as c_int as u32;
    let f = hue / 60.0 - (hue / 60.0) as c_int as f64;
    let p = val * (1.0 - sat);
    let q = val * (1.0 - f * sat);
    let t = val * (1.0 - (1.0 - f * sat));
    let pb = (p * 255.0 + 0.5) as BYTE;
    let qb = (q * 255.0 + 0.5) as BYTE;
    let tb = (t * 255.0 + 0.5) as BYTE;
    let vb = (val * 255.0 + 0.5) as BYTE;
    assert!((0.0..360.0).contains(&hue));
    assert!((0.0..=1.0).contains(&sat));
    assert!((0.0..=1.0).contains(&val));
    assert!(hi < 6);
    let (r, g, b) = match hi {
        0 => (vb, tb, pb),
        1 => (qb, vb, pb),
        2 => (pb, vb, tb),
        3 => (pb, qb, vb),
        4 => (tb, pb, vb),
        5 => (vb, pb, qb),
        _ => (0, 0, 0),
    };
    Color { r, g, b, a: alpha }
}
pub fn eventName(value: BYTE) -> *const c_char {
    match value as u32 {
        ARILOU_ENTRANCE_EVENT => c"ARILOU_ENTRANCE_EVENT".as_ptr(),
        ARILOU_EXIT_EVENT => c"ARILOU_EXIT_EVENT".as_ptr(),
        HYPERSPACE_ENCOUNTER_EVENT => c"HYPERSPACE_ENCOUNTER_EVENT".as_ptr(),
        KOHR_AH_VICTORIOUS_EVENT => c"KOHR_AH_VICTORIOUS_EVENT".as_ptr(),
        ADVANCE_PKUNK_MISSION => c"ADVANCE_PKUNK_MISSION".as_ptr(),
        ADVANCE_THRADD_MISSION => c"ADVANCE_THRADD_MISSION".as_ptr(),
        ZOQFOT_DISTRESS_EVENT => c"ZOQFOT_DISTRESS".as_ptr(),
        ZOQFOT_DEATH_EVENT => c"ZOQFOT_DEATH_EVENT".as_ptr(),
        SHOFIXTI_RETURN_EVENT => c"SHOFIXTI_RETURN_EVENT".as_ptr(),
        ADVANCE_UTWIG_SUPOX_MISSION => c"ADVANCE_UTWIG_SUPOX_MISSION".as_ptr(),
        KOHR_AH_GENOCIDE_EVENT => c"KOHR_AH_GENOCIDE_EVENT".as_ptr(),
        SPATHI_SHIELD_EVENT => c"SPATHI_SHIELD_EVENT".as_ptr(),
        ADVANCE_ILWRATH_MISSION => c"ADVANCE_ILWRATH_MISSION".as_ptr(),
        ADVANCE_MYCON_MISSION => c"ADVANCE_MYCON_MISSION".as_ptr(),
        ARILOU_UMGAH_CHECK => c"ARILOU_UMGAH_CHECK".as_ptr(),
        YEHAT_REBEL_EVENT => c"YEHAT_REBEL_EVENT".as_ptr(),
        SLYLANDRO_RAMP_UP => c"SLYLANDRO_RAMP_UP".as_ptr(),
        SLYLANDRO_RAMP_DOWN => c"SLYLANDRO_RAMP_DOWN".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn bodyColorString(value: BYTE) -> *const c_char {
    match value as u32 {
        BLUE_BODY => c"blue".as_ptr(),
        GREEN_BODY => c"green".as_ptr(),
        ORANGE_BODY => c"orange".as_ptr(),
        RED_BODY => c"red".as_ptr(),
        WHITE_BODY => c"white".as_ptr(),
        YELLOW_BODY => c"yellow".as_ptr(),
        CYAN_BODY => c"cyan".as_ptr(),
        PURPLE_BODY => c"purple".as_ptr(),
        VIOLET_BODY => c"violet".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn starTypeString(value: BYTE) -> *const c_char {
    match value as u32 {
        DWARF_STAR => c"dwarf".as_ptr(),
        GIANT_STAR => c"giant".as_ptr(),
        SUPER_GIANT_STAR => c"super giant".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn starPresenceString(value: BYTE) -> *const c_char {
    match value as u32 {
        0 => c"".as_ptr(),
        SOL_DEFINED => c"Sol".as_ptr(),
        SHOFIXTI_DEFINED => c"Shofixti male".as_ptr(),
        MAIDENS_DEFINED => c"Shofixti maidens".as_ptr(),
        START_COLONY_DEFINED => c"Starting colony".as_ptr(),
        SPATHI_DEFINED => c"Spathi home".as_ptr(),
        ZOQFOT_DEFINED => c"Zoq-Fot-Pik home".as_ptr(),
        MELNORME0_DEFINED => c"Melnorme trader #0".as_ptr(),
        MELNORME1_DEFINED => c"Melnorme trader #1".as_ptr(),
        MELNORME2_DEFINED => c"Melnorme trader #2".as_ptr(),
        MELNORME3_DEFINED => c"Melnorme trader #3".as_ptr(),
        MELNORME4_DEFINED => c"Melnorme trader #4".as_ptr(),
        MELNORME5_DEFINED => c"Melnorme trader #5".as_ptr(),
        MELNORME6_DEFINED => c"Melnorme trader #6".as_ptr(),
        MELNORME7_DEFINED => c"Melnorme trader #7".as_ptr(),
        MELNORME8_DEFINED => c"Melnorme trader #8".as_ptr(),
        TALKING_PET_DEFINED => c"Talking Pet".as_ptr(),
        CHMMR_DEFINED => c"Chmmr home".as_ptr(),
        SYREEN_DEFINED => c"Syreen home".as_ptr(),
        BURVIXESE_DEFINED => c"Burvixese ruins".as_ptr(),
        SLYLANDRO_DEFINED => c"Slylandro home".as_ptr(),
        DRUUGE_DEFINED => c"Druuge home".as_ptr(),
        BOMB_DEFINED => c"Precursor bomb".as_ptr(),
        AQUA_HELIX_DEFINED => c"Aqua Helix".as_ptr(),
        SUN_DEVICE_DEFINED => c"Sun Device".as_ptr(),
        TAALO_PROTECTOR_DEFINED => c"Taalo Shield".as_ptr(),
        SHIP_VAULT_DEFINED => c"Syreen ship vault".as_ptr(),
        URQUAN_WRECK_DEFINED => c"Ur-Quan ship wreck".as_ptr(),
        VUX_BEAST_DEFINED => c"Zex' beauty".as_ptr(),
        SAMATRA_DEFINED => c"Sa-Matra".as_ptr(),
        ZOQ_SCOUT_DEFINED => c"Zoq-Fot-Pik scout".as_ptr(),
        MYCON_DEFINED => c"Mycon home".as_ptr(),
        EGG_CASE0_DEFINED => c"Mycon egg shell #0".as_ptr(),
        EGG_CASE1_DEFINED => c"Mycon egg shell #1".as_ptr(),
        EGG_CASE2_DEFINED => c"Mycon egg shell #2".as_ptr(),
        PKUNK_DEFINED => c"Pkunk home".as_ptr(),
        UTWIG_DEFINED => c"Utwig home".as_ptr(),
        SUPOX_DEFINED => c"Supox home".as_ptr(),
        YEHAT_DEFINED => c"Yehat home".as_ptr(),
        VUX_DEFINED => c"Vux home".as_ptr(),
        ORZ_DEFINED => c"Orz home".as_ptr(),
        THRADD_DEFINED => c"Thraddash home".as_ptr(),
        RAINBOW_DEFINED => c"Rainbow world".as_ptr(),
        ILWRATH_DEFINED => c"Ilwrath home".as_ptr(),
        ANDROSYNTH_DEFINED => c"Androsynth ruins".as_ptr(),
        MYCON_TRAP_DEFINED => c"Mycon trap".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn worldSizeString(value: BYTE) -> *const c_char {
    match value as u32 {
        SMALL_ROCKY_WORLD => c"small rocky world".as_ptr(),
        LARGE_ROCKY_WORLD => c"large rocky world".as_ptr(),
        GAS_GIANT => c"gas giant".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn worldGenAlgoString(value: BYTE) -> *const c_char {
    match value as u32 {
        TOPO_ALGO => c"TOPO_ALGO".as_ptr(),
        CRATERED_ALGO => c"CRATERED_ALGO".as_ptr(),
        GAS_GIANT_ALGO => c"GAS_GIANT_ALGO".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn atmosphereString(value: BYTE) -> *const c_char {
    match value as u32 {
        LIGHT => c"thin".as_ptr(),
        MEDIUM => c"normal".as_ptr(),
        HEAVY => c"thick".as_ptr(),
        _ => c"super thick".as_ptr(),
    }
}

pub fn densityString(value: BYTE) -> *const c_char {
    match value as u32 {
        GAS_DENSITY => c"gaseous".as_ptr(),
        LIGHT_DENSITY => c"light".as_ptr(),
        LOW_DENSITY => c"low".as_ptr(),
        NORMAL_DENSITY => c"normal".as_ptr(),
        HIGH_DENSITY => c"high".as_ptr(),
        SUPER_DENSITY => c"super high".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn depositQualityString(value: BYTE) -> *const c_char {
    match value as u32 {
        LIGHT => c"low".as_ptr(),
        MEDIUM => c"medium".as_ptr(),
        HEAVY => c"high".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

type SystemVisitor<T> = unsafe fn(*const STAR_DESC, *const SOLARSYS_STATE, &mut T);
type WorldVisitor<T> = unsafe fn(*const PLANET_DESC, &mut T);

pub struct UniverseRecurseArg<'a, T> {
    pub systemFuncPre: Option<SystemVisitor<T>>,
    pub systemFuncPost: Option<SystemVisitor<T>>,
    pub planetFuncPre: Option<WorldVisitor<T>>,
    pub planetFuncPost: Option<WorldVisitor<T>>,
    pub moonFunc: Option<WorldVisitor<T>>,
    pub arg: &'a mut T,
}

#[cfg(not(test))]
mod runtime {
    use super::*;
    use crate::navigation::globals::GlobData;
    use crate::planets::tables::{PLANET_ARRAY as planet_array, STARMAP_ARRAY as starmap_array};
    use crate::state::game_state_keys::{get_game_state, set_game_state};
    use libc::FILE;
    use std::{mem, ptr};
    extern "C" {
        #[cfg(target_os = "macos")]
        #[link_name = "__stderrp"]
        static mut error_stream: *mut FILE;
        #[cfg(not(target_os = "macos"))]
        #[link_name = "stderr"]
        static mut error_stream: *mut FILE;
        #[cfg(target_os = "macos")]
        #[link_name = "__stdoutp"]
        static mut output_stream: *mut FILE;
        #[cfg(not(target_os = "macos"))]
        #[link_name = "stdout"]
        static mut output_stream: *mut FILE;
    }

    unsafe fn callGenerateForScanType(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        node: COUNT,
        scan_type: BYTE,
        info: *mut NODE_INFO,
    ) -> COUNT {
        crate::planets::scan::callGenerateForScanType(
            system.cast(),
            world.cast(),
            node,
            scan_type,
            info.cast(),
        )
    }

    const INFINITE_RADIUS: COUNT = COUNT::MAX;

    pub static mut INSTANT_MOVE: BOOLEAN = 0;
    pub static mut DISABLE_INTERACTIVITY: BOOLEAN = 0;
    pub static mut DEBUG_HOOK: Option<unsafe fn()> = None;

    /// # Safety
    /// Must run synchronously on the game thread with initialized game state.
    pub unsafe fn debugKeyPressedSynchronous() {
        equipShip();
        giveDevices();
        clearEscorts();
        for race in [
            ARILOU_SHIP,
            PKUNK_SHIP,
            VUX_SHIP,
            YEHAT_SHIP,
            MELNORME_SHIP,
            DRUUGE_SHIP,
            ILWRATH_SHIP,
            MYCON_SHIP,
            SLYLANDRO_SHIP,
            UMGAH_SHIP,
            URQUAN_SHIP,
            BLACK_URQUAN_SHIP,
        ] {
            AddEscortShips(race as COUNT, 1);
        }
        resetCrewBattle();
        resetEnergyBattle();
        INSTANT_MOVE = BOOLEAN::from(INSTANT_MOVE == 0);
        showSpheres();
        activateAllShips();
    }

    // The asynchronous C debug-key hook contains no enabled operations.
    pub fn debugKeyPressed() {}

    /// # Safety
    /// Must run on the game thread. The game clock and event queue must be live.
    pub unsafe fn forwardToNextEvent(skipHEE: BOOLEAN) {
        if GameClockRunning() == 0 {
            return;
        }
        LockGameClock();
        let mut done = skipHEE == 0;
        loop {
            let handle = GlobData.Game_state.GameClock.event_q.head;
            if handle.is_null() {
                return;
            }
            let event = &*handle.cast::<EVENT>();
            if event.func_index as u32 != HYPERSPACE_ENCOUNTER_EVENT {
                done = true;
            }
            let (year, month, day) = (
                event.year_index,
                event.month_index as COUNT,
                event.day_index as COUNT,
            );
            loop {
                let clock = &*ptr::addr_of!(GlobData.Game_state.GameClock);
                if clock.year_index > year
                    || (clock.year_index == year
                        && (clock.month_index as COUNT > month
                            || (clock.month_index as COUNT == month
                                && clock.day_index as COUNT >= day)))
                {
                    break;
                }
                MoveGameClockDays(1);
            }
            if done {
                break;
            }
        }
        UnlockGameClock();
    }
    /// # Safety
    /// `out` must be an open C stream and `event` a readable event record.
    pub unsafe fn dumpEvent(out: *mut FILE, event: *const EVENT) {
        libc::fprintf(
            out,
            c"%4u/%02u/%02u: %s\n".as_ptr(),
            (*event).year_index as u32,
            (*event).month_index as u32,
            (*event).day_index as u32,
            eventName((*event).func_index),
        );
    }
    /// # Safety
    /// Must run with a live game clock and an open output stream.
    pub unsafe fn dumpEvents(out: *mut FILE) {
        LockGameClock();
        crate::collections::queue::for_all_links(
            ptr::addr_of_mut!(GlobData.Game_state.GameClock.event_q).cast(),
            |event| dumpEvent(out, ptr::from_mut(event).cast()),
        );
        UnlockGameClock();
    }

    /// # Safety
    /// Must run on the game thread with initialized SIS and optional solar system.
    pub unsafe fn equipShip() {
        if GlobData.Game_state.CurrentActivity as u8 as u32 == SUPER_MELEE {
            return;
        }
        for i in 0..NUM_DRIVE_SLOTS as usize {
            GlobData.SIS_state.DriveSlots[i] = FUSION_THRUSTER as BYTE;
        }
        for i in 0..NUM_JET_SLOTS as usize {
            GlobData.SIS_state.JetSlots[i] = TURNING_JETS as BYTE;
        }
        set_game_state(
            "LANDER_SHIELDS",
            (1 << EARTHQUAKE_DISASTER)
                | (1 << BIOLOGICAL_DISASTER)
                | (1 << LIGHTNING_DISASTER)
                | (1 << LAVASPOT_DISASTER),
        );
        set_game_state("IMPROVED_LANDER_SPEED", 1);
        set_game_state("IMPROVED_LANDER_CARGO", 1);
        set_game_state("IMPROVED_LANDER_SHOT", 1);
        let mut i;
        if get_game_state("CHMMR_BOMB_STATE") < 2 {
            i = 0;
            for module in [
                HIGHEFF_FUELSYS,
                HIGHEFF_FUELSYS,
                CREW_POD,
                CREW_POD,
                CREW_POD,
                CREW_POD,
                CREW_POD,
                STORAGE_BAY,
                SHIVA_FURNACE,
                SHIVA_FURNACE,
                DYNAMO_UNIT,
                TRACKING_SYSTEM,
                TRACKING_SYSTEM,
                SHIVA_FURNACE,
                CANNON_WEAPON,
                CANNON_WEAPON,
            ] {
                GlobData.SIS_state.ModuleSlots[i] = module as BYTE;
                i += 1;
            }
            GlobData.SIS_state.NumLanders = MAX_LANDERS as BYTE;
        } else {
            i = NUM_BOMB_MODULES as usize;
            for module in [
                HIGHEFF_FUELSYS,
                CREW_POD,
                SHIVA_FURNACE,
                SHIVA_FURNACE,
                CANNON_WEAPON,
                SHIVA_FURNACE,
            ] {
                GlobData.SIS_state.ModuleSlots[i] = module as BYTE;
                i += 1;
            }
        }
        assert!(i <= NUM_MODULE_SLOTS as usize);
        GlobData.SIS_state.FuelOnBoard = FUEL_RESERVE as _;
        GlobData.SIS_state.CrewEnlisted = 0;
        for i in 0..NUM_MODULE_SLOTS as usize {
            match GlobData.SIS_state.ModuleSlots[i] as u32 {
                CREW_POD => {
                    GlobData.SIS_state.CrewEnlisted = GlobData
                        .SIS_state
                        .CrewEnlisted
                        .wrapping_add(CREW_POD_CAPACITY as _)
                }
                FUEL_TANK => {
                    GlobData.SIS_state.FuelOnBoard = GlobData
                        .SIS_state
                        .FuelOnBoard
                        .wrapping_add(FUEL_TANK_CAPACITY as _)
                }
                HIGHEFF_FUELSYS => {
                    GlobData.SIS_state.FuelOnBoard = GlobData
                        .SIS_state
                        .FuelOnBoard
                        .wrapping_add(HEFUEL_TANK_CAPACITY as _)
                }
                _ => {}
            }
        }
        if !pSolarSysState.is_null() {
            (*pSolarSysState).max_ship_speed = (5 * IP_SHIP_THRUST_INCREMENT) as BYTE;
            for i in 0..NUM_DRIVE_SLOTS as usize {
                // Preserve the original C subscript: DriveSlots[i == FUSION_THRUSTER].
                if GlobData.SIS_state.DriveSlots[usize::from(i == FUSION_THRUSTER as usize)] != 0 {
                    (*pSolarSysState).max_ship_speed = (*pSolarSysState)
                        .max_ship_speed
                        .wrapping_add(IP_SHIP_THRUST_INCREMENT as BYTE);
                }
            }
            (*pSolarSysState).turn_wait = IP_SHIP_TURN_WAIT as BYTE;
            for i in 0..NUM_JET_SLOTS as usize {
                if GlobData.SIS_state.JetSlots[i] == TURNING_JETS as BYTE {
                    (*pSolarSysState).turn_wait = (*pSolarSysState)
                        .turn_wait
                        .wrapping_sub(IP_SHIP_TURN_DECREMENT as BYTE);
                }
            }
        }
        if inHQSpace() != 0 || GlobData.Game_state.CurrentActivity as u8 as u32 == IN_INTERPLANETARY
        {
            DeltaSISGauges(
                UNDEFINED_DELTA as _,
                UNDEFINED_DELTA as _,
                UNDEFINED_DELTA as _,
            );
        }
    }

    /// # Safety
    /// Must run with the game-state singleton initialized.
    pub unsafe fn giveDevices() {
        for key in [
            "ROSY_SPHERE_ON_SHIP",
            "ARTIFACT_2_ON_SHIP",
            "ARTIFACT_3_ON_SHIP",
            "SUN_DEVICE_ON_SHIP",
            "UTWIG_BOMB_ON_SHIP",
            "ULTRON_CONDITION",
            "MAIDENS_ON_SHIP",
            "TALKING_PET_ON_SHIP",
            "AQUA_HELIX_ON_SHIP",
            "CLEAR_SPINDLE_ON_SHIP",
            "UMGAH_BROADCASTERS_ON_SHIP",
            "TAALO_PROTECTOR_ON_SHIP",
            "EGG_CASE0_ON_SHIP",
            "EGG_CASE1_ON_SHIP",
            "EGG_CASE2_ON_SHIP",
            "SYREEN_SHUTTLE_ON_SHIP",
            "VUX_BEAST_ON_SHIP",
            "PORTAL_SPAWNER_ON_SHIP",
            "PORTAL_KEY_ON_SHIP",
            "BURV_BROADCASTERS_ON_SHIP",
            "MOONBASE_ON_SHIP",
            "DESTRUCT_CODE_ON_SHIP",
        ] {
            set_game_state(key, 1);
        }
    }
    /// # Safety
    /// Must run on the game thread with the escort queue initialized.
    pub unsafe fn clearEscorts() {
        let queue = ptr::addr_of_mut!(GlobData.Game_state.built_ship_q);
        clear_escort_queue(queue.cast());
        DeltaSISGauges(
            UNDEFINED_DELTA as _,
            UNDEFINED_DELTA as _,
            UNDEFINED_DELTA as _,
        );
    }

    // These are the units.h macros, evaluated with the live logical screen size.
    unsafe fn log_units() -> (i32, i32) {
        (
            ((((ScreenWidth - STATUS_WIDTH as i32 - 2 * SAFE_X as i32) << ONE_SHIFT)
                << MAX_REDUCTION)
                >> 4)
                * SECTOR_WIDTH as i32,
            ((((ScreenHeight - 2 * SAFE_Y as i32) << ONE_SHIFT) << MAX_REDUCTION) >> 4)
                * SECTOR_HEIGHT as i32,
        )
    }
    /// # Safety
    /// Must run on the game thread in HyperSpace or QuasiSpace.
    pub unsafe fn doInstantMove() {
        let (log_x, log_y) = log_units();
        let universe_x = ((MAX_X_UNIVERSE as i32 + 1) >> 4) * 10;
        let universe_y = (MAX_Y_UNIVERSE as i32 + 1) >> 4;
        let game = ptr::addr_of_mut!(GlobData.Game_state);
        let sis = ptr::addr_of_mut!(GlobData.SIS_state);
        if (*game).autopilot.x == -1 || (*game).autopilot.y == -1 {
            (*game).autopilot.x =
                (((*sis).log_x.wrapping_mul(universe_x) + (log_x >> 1)) / log_x) as COORD;
            (*game).autopilot.y = (MAX_Y_UNIVERSE as i32
                - (((*sis).log_y.wrapping_mul(universe_y) + (log_y >> 1)) / log_y))
                as COORD;
        } else {
            (*sis).log_x = ((*game).autopilot.x as i32 * log_x + (universe_x >> 1)) / universe_x;
            (*sis).log_y = ((MAX_Y_UNIVERSE as i32 - (*game).autopilot.y as i32) * log_y
                + (universe_y >> 1))
                / universe_y;
        }
        if get_game_state("ARILOU_SPACE_SIDE") <= 1 {
            CurStarDescPtr = FindStar(ptr::null_mut(), ptr::addr_of_mut!((*game).autopilot), 0, 0);
            if !CurStarDescPtr.is_null() {
                set_game_state("USED_BROADCASTER", 0);
                (*game).CurrentActivity &= !(IN_BATTLE as ACTIVITY);
                (*game).ShipFacing = 0;
                (*game).ip_planet = 0;
                (*game).in_orbit = 0;
                (*game).CurrentActivity |= START_INTERPLANETARY as ACTIVITY;
            }
        }
        (*game).autopilot.x = -1;
        (*game).autopilot.y = -1;
    }
    /// # Safety
    /// Must run on the game thread with the race queue initialized.
    pub unsafe fn showSpheres() {
        let mut handle = GlobData.Game_state.avail_race_q.head;
        while !handle.is_null() {
            let fleet = handle.cast::<FLEET_INFO>();
            let next = (*fleet).succ;
            if (*fleet).actual_strength != INFINITE_RADIUS as COUNT
                && (*fleet).known_strength != (*fleet).actual_strength
            {
                (*fleet).known_strength = (*fleet).actual_strength;
                (*fleet).known_loc = (*fleet).loc;
            }
            handle = next;
        }
    }
    /// # Safety
    /// Must run on the game thread with the race queue initialized.
    pub unsafe fn activateAllShips() {
        let mut handle = GlobData.Game_state.avail_race_q.head;
        while !handle.is_null() {
            let fleet = handle.cast::<FLEET_INFO>();
            let next = (*fleet).succ;
            if !(*fleet).icons.is_null() {
                (*fleet).allied_state = GOOD_GUY as _;
            }
            handle = next;
        }
    }

    /// # Safety
    /// Must run on the game thread. The callback must accept every star and its context.
    pub unsafe fn forAllStars<T>(callback: unsafe fn(*mut STAR_DESC, &mut T), arg: &mut T) {
        for i in 0..NUM_SOLAR_SYSTEMS as usize {
            callback(
                ptr::addr_of_mut!(starmap_array).cast::<STAR_DESC>().add(i),
                arg,
            );
        }
    }
    /// # Safety
    /// `system` and `star` must be the active system. Callback and context must be valid.
    pub unsafe fn forAllPlanets<T>(
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        callback: unsafe fn(*mut STAR_DESC, *mut SOLARSYS_STATE, *mut PLANET_DESC, &mut T),
        arg: &mut T,
    ) {
        assert!(CurStarDescPtr == star);
        assert!(pSolarSysState == system);
        for i in 0..(*system).SunDesc[0].NumPlanets as usize {
            callback(
                star,
                system,
                ptr::addr_of_mut!((*system).PlanetDesc[i]),
                arg,
            );
        }
    }
    /// # Safety
    /// The planet and system must be active and their generated moon records valid.
    pub unsafe fn forAllMoons<T>(
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        callback: unsafe fn(
            *mut STAR_DESC,
            *mut SOLARSYS_STATE,
            *mut PLANET_DESC,
            *mut PLANET_DESC,
            &mut T,
        ),
        arg: &mut T,
    ) {
        assert!(pSolarSysState == system);
        for i in 0..(*planet).NumPlanets as usize {
            callback(
                star,
                system,
                planet,
                ptr::addr_of_mut!((*system).MoonDesc[i]),
                arg,
            );
        }
    }
    /// # Safety
    /// Must run on the game thread. All supplied callbacks and their context must
    /// remain valid throughout universe generation, which can change game state.
    pub unsafe fn UniverseRecurse<T>(arg: &mut UniverseRecurseArg<'_, T>) {
        if arg.systemFuncPre.is_none()
            && arg.systemFuncPost.is_none()
            && arg.planetFuncPre.is_none()
            && arg.planetFuncPost.is_none()
            && arg.moonFunc.is_none()
        {
            return;
        }
        LockGameClock();
        let saved_activity = GlobData.Game_state.CurrentActivity;
        DISABLE_INTERACTIVITY = 1;
        forAllStars(starRecurse, arg);
        DISABLE_INTERACTIVITY = 0;
        GlobData.Game_state.CurrentActivity = saved_activity;
        UnlockGameClock();
    }
    unsafe fn starRecurse<T>(star: *mut STAR_DESC, recurse: &mut UniverseRecurseArg<'_, T>) {
        let old_system = pSolarSysState;
        let old_star = CurStarDescPtr;
        CurStarDescPtr = star;
        (*SysGenRNG).reseed(GetRandomSeedForStar(star));
        let mut system: SOLARSYS_STATE = mem::zeroed();
        system.SunDesc[0].pPrevDesc = ptr::null_mut();
        system.SunDesc[0].rand_seed = (*SysGenRNG).next_random();
        system.SunDesc[0].data_index = (((*star).Type as u32
            & ((0xff << STAR_TYPE_SHIFT) & !(0xff << STAR_COLOR_SHIFT)))
            >> STAR_TYPE_SHIFT) as BYTE;
        system.SunDesc[0].location = POINT { x: 0, y: 0 };
        system.genFuncs = getGenerateFunctions((*star).Index);
        pSolarSysState = &mut system;
        ((*system.genFuncs).generatePlanets.unwrap())(&mut system);
        if let Some(cb) = recurse.systemFuncPre {
            cb(star, &system, recurse.arg);
        }
        if recurse.planetFuncPre.is_some()
            || recurse.planetFuncPost.is_some()
            || recurse.moonFunc.is_some()
        {
            forAllPlanets(star, &mut system, planetRecurse, recurse);
        }
        if let Some(cb) = recurse.systemFuncPost {
            cb(star, &system, recurse.arg);
        }
        pSolarSysState = old_system;
        CurStarDescPtr = old_star;
    }
    unsafe fn planetRecurse<T>(
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        recurse: &mut UniverseRecurseArg<'_, T>,
    ) {
        assert!(CurStarDescPtr == star);
        assert!(pSolarSysState == system);
        (*planet).pPrevDesc = ptr::addr_of_mut!((*system).SunDesc[0]);
        if let Some(cb) = recurse.planetFuncPre {
            (*system).pOrbitalDesc = planet;
            DoPlanetaryAnalysis(ptr::addr_of_mut!((*system).SysInfo), planet);
            ((*(*system).genFuncs).generateOrbital.unwrap())(system, planet);
            cb(planet, recurse.arg);
        }
        if recurse.moonFunc.is_some() {
            (*SysGenRNG).reseed((*planet).rand_seed);
            ((*(*system).genFuncs).generateMoons.unwrap())(system, planet);
            forAllMoons(star, system, planet, moonRecurse, recurse);
        }
        if let Some(cb) = recurse.planetFuncPost {
            (*system).pOrbitalDesc = planet;
            DoPlanetaryAnalysis(ptr::addr_of_mut!((*system).SysInfo), planet);
            ((*(*system).genFuncs).generateOrbital.unwrap())(system, planet);
            cb(planet, recurse.arg);
        }
    }
    unsafe fn moonRecurse<T>(
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        moon: *mut PLANET_DESC,
        recurse: &mut UniverseRecurseArg<'_, T>,
    ) {
        assert!(CurStarDescPtr == star);
        assert!(pSolarSysState == system);
        (*moon).pPrevDesc = planet;
        if let Some(cb) = recurse.moonFunc {
            (*system).pOrbitalDesc = moon;
            if (*moon).data_index != HIERARCHY_STARBASE as BYTE
                && (*moon).data_index != SA_MATRA as BYTE
            {
                DoPlanetaryAnalysis(ptr::addr_of_mut!((*system).SysInfo), moon);
            }
            ((*(*system).genFuncs).generateOrbital.unwrap())(system, moon);
            cb(moon, recurse.arg);
        }
    }

    struct DumpUniverseArg {
        out: *mut FILE,
    }
    /// # Safety
    /// Must run on the game thread with an open output stream.
    pub unsafe fn dumpUniverse(out: *mut FILE) {
        let mut context = DumpUniverseArg { out };
        let mut recurse = UniverseRecurseArg {
            systemFuncPre: Some(dumpSystemCallback),
            systemFuncPost: None,
            planetFuncPre: Some(dumpPlanetCallback),
            planetFuncPost: None,
            moonFunc: Some(dumpMoonCallback),
            arg: &mut context,
        };
        UniverseRecurse(&mut recurse);
    }
    unsafe fn write_generation_file(
        name: *const c_char,
        callback: unsafe fn(*mut FILE),
        message: *const c_char,
    ) {
        let out = libc::fopen(name, c"w".as_ptr());
        if out.is_null() {
            let error = std::io::Error::last_os_error().raw_os_error().unwrap();
            libc::fprintf(
                error_stream,
                c"Error: Could not open file '%s' for writing: %s\n".as_ptr(),
                name,
                libc::strerror(error),
            );
            return;
        }
        callback(out);
        libc::fclose(out);
        libc::fprintf(output_stream, c"%s".as_ptr(), message);
    }
    /// # Safety
    /// Must run on the game thread; generation can change game state.
    pub unsafe fn dumpUniverseToFile() {
        write_generation_file(
            c"PlanetInfo".as_ptr(),
            dumpUniverse,
            c"*** Star dump complete. The game may be in an undefined state.\n".as_ptr(),
        );
    }
    unsafe fn dumpSystemCallback(
        star: *const STAR_DESC,
        system: *const SOLARSYS_STATE,
        arg: &mut DumpUniverseArg,
    ) {
        dumpSystem(arg.out, star, system);
    }
    /// # Safety
    /// Star and system records must be readable, and `out` an open stream.
    pub unsafe fn dumpSystem(
        out: *mut FILE,
        star: *const STAR_DESC,
        _system: *const SOLARSYS_STATE,
    ) {
        let mut name = [0 as c_char; 256];
        let mut buffer = [0 as c_char; 40];
        GetClusterName(star, name.as_mut_ptr());
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            c"%s %s".as_ptr(),
            bodyColorString(
                (((*star).Type as u32 & ((0xff << STAR_COLOR_SHIFT) as BYTE as u32))
                    >> STAR_COLOR_SHIFT) as BYTE,
            ),
            starTypeString(
                (((*star).Type as u32
                    & (((0xff << STAR_TYPE_SHIFT) & !(0xff << STAR_COLOR_SHIFT)) as BYTE as u32))
                    >> STAR_TYPE_SHIFT) as BYTE,
            ),
        );
        libc::fprintf(
            out,
            c"%-22s  (%3d.%1d, %3d.%1d) %-19s  %s\n".as_ptr(),
            name.as_ptr(),
            (*star).star_pt.x as c_int / 10,
            (*star).star_pt.x as c_int % 10,
            (*star).star_pt.y as c_int / 10,
            (*star).star_pt.y as c_int % 10,
            buffer.as_ptr(),
            starPresenceString((*star).Index),
        );
    }
    unsafe fn dumpPlanetCallback(planet: *const PLANET_DESC, arg: &mut DumpUniverseArg) {
        dumpPlanet(arg.out, planet);
    }
    /// # Safety
    /// Planet must belong to the active system, and `out` be an open stream.
    pub unsafe fn dumpPlanet(out: *mut FILE, planet: *const PLANET_DESC) {
        ((*(*pSolarSysState).genFuncs).generateName.unwrap())(pSolarSysState, planet);
        libc::fprintf(
            out,
            c"- %-37s  %s\n".as_ptr(),
            ptr::addr_of!(GlobData.SIS_state.PlanetName).cast::<c_char>(),
            planetTypeString(((*planet).data_index & !(PLANET_SHIELDED as BYTE)) as c_int),
        );
        dumpWorld(out, planet);
    }
    unsafe fn dumpMoonCallback(moon: *const PLANET_DESC, arg: &mut DumpUniverseArg) {
        dumpMoon(arg.out, moon);
    }
    /// # Safety
    /// Moon must belong to the active system, and `out` be an open stream.
    pub unsafe fn dumpMoon(out: *mut FILE, moon: *const PLANET_DESC) {
        let kind = if (*moon).data_index == HIERARCHY_STARBASE as BYTE {
            c"StarBase".as_ptr()
        } else if (*moon).data_index == SA_MATRA as BYTE {
            c"Sa-Matra".as_ptr()
        } else {
            planetTypeString(((*moon).data_index & !(PLANET_SHIELDED as BYTE)) as c_int)
        };
        libc::fprintf(
            out,
            c"  - Moon %-30c  %s\n".as_ptr(),
            b'a' as c_int + moon.offset_from(ptr::addr_of!((*pSolarSysState).MoonDesc[0])) as c_int,
            kind,
        );
        dumpWorld(out, moon);
    }
    unsafe fn dumpWorld(out: *mut FILE, world: *const PLANET_DESC) {
        if (*world).data_index == HIERARCHY_STARBASE as BYTE {
            return;
        }
        if (*world).data_index == SA_MATRA as BYTE {
            return;
        }
        let info = &(*pSolarSysState).SysInfo.PlanetInfo;
        libc::fprintf(
            out,
            c"          AxialTilt:  %d\n".as_ptr(),
            info.AxialTilt as c_int,
        );
        libc::fprintf(
            out,
            c"          Tectonics:  %d\n".as_ptr(),
            info.Tectonics as c_int,
        );
        libc::fprintf(
            out,
            c"          Weather:    %d\n".as_ptr(),
            info.Weather as c_int,
        );
        libc::fprintf(
            out,
            c"          Density:    %d\n".as_ptr(),
            info.PlanetDensity as c_int,
        );
        libc::fprintf(
            out,
            c"          Radius:     %d\n".as_ptr(),
            info.PlanetRadius as c_int,
        );
        libc::fprintf(
            out,
            c"          Gravity:    %d\n".as_ptr(),
            info.SurfaceGravity as c_int,
        );
        libc::fprintf(
            out,
            c"          Temp:       %d\n".as_ptr(),
            info.SurfaceTemperature as c_int,
        );
        libc::fprintf(
            out,
            c"          Day:        %d\n".as_ptr(),
            info.RotationPeriod as c_int,
        );
        libc::fprintf(
            out,
            c"          Atmosphere: %d\n".as_ptr(),
            info.AtmoDensity as c_int,
        );
        libc::fprintf(
            out,
            c"          LifeChance: %d\n".as_ptr(),
            info.LifeChance as c_int,
        );
        libc::fprintf(
            out,
            c"          DistToSun:  %d\n".as_ptr(),
            info.PlanetToSunDist as c_int,
        );
        if (*world).data_index & PLANET_SHIELDED as BYTE != 0 {
            return;
        }
        libc::fprintf(
            out,
            c"          Bio: %4d    Min: %4d\n".as_ptr(),
            calculateBioValue(pSolarSysState, world) as c_int,
            calculateMineralValue(pSolarSysState, world) as c_int,
        );
    }
    pub fn uqm_debug_creature_bio_value(kind: BYTE) -> c_int {
        crate::planet_side::creatures::rust_creature_bio_value(kind)
    }
    /// # Safety
    /// World must be the active orbital descriptor and its generation functions live.
    pub unsafe fn calculateBioValue(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
    ) -> COUNT {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = callGenerateForScanType(
            system,
            world,
            COUNT::MAX,
            PlanetScanTypes_BIOLOGICAL_SCAN as BYTE,
            ptr::null_mut(),
        );
        let mut result: COUNT = 0;
        for i in 0..count {
            let mut info: NODE_INFO = mem::zeroed();
            callGenerateForScanType(
                system,
                world,
                i,
                PlanetScanTypes_BIOLOGICAL_SCAN as BYTE,
                &mut info,
            );
            let value = uqm_debug_creature_bio_value(info.type_ as BYTE);
            if value < 0 {
                libc::abort();
            }
            result = result.wrapping_add((BIO_CREDIT_VALUE as c_int * value) as COUNT);
        }
        result
    }
    /// # Safety
    /// World must be active, and `bio` writable for all creature-type counters.
    pub unsafe fn generateBioIndex(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        bio: *mut COUNT,
    ) {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = callGenerateForScanType(
            system,
            world,
            COUNT::MAX,
            PlanetScanTypes_BIOLOGICAL_SCAN as BYTE,
            ptr::null_mut(),
        );
        for i in 0..(NUM_CREATURE_TYPES + NUM_SPECIAL_CREATURE_TYPES) as usize {
            *bio.add(i) = 0;
        }
        for i in 0..count {
            let mut info: NODE_INFO = mem::zeroed();
            callGenerateForScanType(
                system,
                world,
                i,
                PlanetScanTypes_BIOLOGICAL_SCAN as BYTE,
                &mut info,
            );
            let entry = bio.add(info.type_ as usize);
            *entry = (*entry).wrapping_add(1);
        }
    }
    unsafe fn element_category(kind: usize) -> usize {
        (*Elements.add(kind) & 7) as usize
    }
    /// # Safety
    /// World must be the active orbital descriptor and its generation functions live.
    pub unsafe fn calculateMineralValue(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
    ) -> COUNT {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = callGenerateForScanType(
            system,
            world,
            COUNT::MAX,
            PlanetScanTypes_MINERAL_SCAN as BYTE,
            ptr::null_mut(),
        );
        let mut result: COUNT = 0;
        for i in 0..count {
            let mut info: NODE_INFO = mem::zeroed();
            callGenerateForScanType(
                system,
                world,
                i,
                PlanetScanTypes_MINERAL_SCAN as BYTE,
                &mut info,
            );
            result = result.wrapping_add(
                ((info.density >> 8) as u32
                    * GlobData.Game_state.ElementWorth[element_category(info.type_ as usize)]
                        as u32) as COUNT,
            );
        }
        result
    }
    /// # Safety
    /// World must be active, and `minerals` writable for all element-category counters.
    pub unsafe fn generateMineralIndex(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        minerals: *mut COUNT,
    ) {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = callGenerateForScanType(
            system,
            world,
            COUNT::MAX,
            PlanetScanTypes_MINERAL_SCAN as BYTE,
            ptr::null_mut(),
        );
        for i in 0..NUM_ELEMENT_CATEGORIES as usize {
            *minerals.add(i) = 0;
        }
        for i in 0..count {
            let mut info: NODE_INFO = mem::zeroed();
            callGenerateForScanType(
                system,
                world,
                i,
                PlanetScanTypes_MINERAL_SCAN as BYTE,
                &mut info,
            );
            let entry = minerals.add(element_category(info.type_ as usize));
            *entry = (*entry).wrapping_add(info.density >> 8);
        }
    }
    struct TallyResourcesArg {
        out: *mut FILE,
        mineralCount: COUNT,
        bioCount: COUNT,
    }
    /// # Safety
    /// Must run on the game thread with an open output stream.
    pub unsafe fn tallyResources(out: *mut FILE) {
        let mut context = TallyResourcesArg {
            out,
            mineralCount: 0,
            bioCount: 0,
        };
        let mut recurse = UniverseRecurseArg {
            systemFuncPre: Some(tallySystemPreCallback),
            systemFuncPost: Some(tallySystemPostCallback),
            planetFuncPre: Some(tallyPlanetCallback),
            planetFuncPost: None,
            moonFunc: Some(tallyMoonCallback),
            arg: &mut context,
        };
        UniverseRecurse(&mut recurse);
    }
    /// # Safety
    /// Must run on the game thread; generation can change game state.
    pub unsafe fn tallyResourcesToFile() {
        write_generation_file(
            c"ResourceTally".as_ptr(),
            tallyResources,
            c"*** Resource tally complete. The game may be in an undefined state.\n".as_ptr(),
        );
    }
    unsafe fn tallySystemPreCallback(
        _: *const STAR_DESC,
        _: *const SOLARSYS_STATE,
        arg: &mut TallyResourcesArg,
    ) {
        let context = arg;
        context.mineralCount = 0;
        context.bioCount = 0;
    }
    unsafe fn tallySystemPostCallback(
        star: *const STAR_DESC,
        _: *const SOLARSYS_STATE,
        arg: &mut TallyResourcesArg,
    ) {
        let context = arg;
        let mut name = [0 as c_char; 256];
        GetClusterName(star, name.as_mut_ptr());
        libc::fprintf(
            context.out,
            c"%s\t%d\t%d\n".as_ptr(),
            name.as_ptr(),
            context.mineralCount as c_int,
            context.bioCount as c_int,
        );
    }
    unsafe fn tallyPlanetCallback(world: *const PLANET_DESC, arg: &mut TallyResourcesArg) {
        tallyResourcesWorld(arg, world);
    }
    unsafe fn tallyMoonCallback(world: *const PLANET_DESC, arg: &mut TallyResourcesArg) {
        tallyResourcesWorld(arg, world);
    }
    unsafe fn tallyResourcesWorld(arg: &mut TallyResourcesArg, world: *const PLANET_DESC) {
        if (*world).data_index == HIERARCHY_STARBASE as BYTE {
            return;
        }
        if (*world).data_index == SA_MATRA as BYTE {
            return;
        }
        arg.bioCount = arg
            .bioCount
            .wrapping_add(calculateBioValue(pSolarSysState, world));
        arg.mineralCount = arg
            .mineralCount
            .wrapping_add(calculateMineralValue(pSolarSysState, world));
    }

    /// # Safety
    /// The callback must accept every immutable planet-type record and its context.
    pub unsafe fn forAllPlanetTypes<T>(
        callback: unsafe fn(c_int, *const PlanetFrame, &mut T),
        arg: &mut T,
    ) {
        for i in 0..NUMBER_OF_PLANET_TYPES as usize {
            callback(
                i as c_int,
                ptr::addr_of!(planet_array).cast::<PlanetFrame>().add(i),
                arg,
            );
        }
    }
    struct DumpPlanetTypesArg {
        out: *mut FILE,
    }
    /// # Safety
    /// `out` must be an open output stream, with game strings initialized.
    pub unsafe fn dumpPlanetTypes(out: *mut FILE) {
        let mut context = DumpPlanetTypesArg { out };
        forAllPlanetTypes(dumpPlanetTypeCallback, &mut context);
    }
    unsafe fn dumpPlanetTypeCallback(
        index: c_int,
        planet: *const PlanetFrame,
        arg: &mut DumpPlanetTypesArg,
    ) {
        dumpPlanetType(arg.out, index, planet);
    }
    unsafe fn game_string(index: usize) -> *const c_char {
        GetStringAddress(SetAbsStringTableIndex(GameStrings, index as COUNT))
    }
    /// # Safety
    /// Planet type must be readable, game strings initialized, and stream open.
    pub unsafe fn dumpPlanetType(out: *mut FILE, index: c_int, planet: *const PlanetFrame) {
        libc::fprintf(out,c"%s\n\tType: %s\n\tColor: %s\n\tSurface generation algoritm: %s\n\tTectonics: %s\n\tAtmosphere: %s\n\tDensity: %s\n\tElements:\n".as_ptr(),
        planetTypeString(index),worldSizeString((*planet).Type & PLAN_SIZE_MASK as BYTE),
        bodyColorString((*planet).Type >> 4),worldGenAlgoString((*planet).Type & PLAN_ALGO_MASK as BYTE),
        tectonicsString((*planet).BaseTectonics),atmosphereString((*planet).AtmoAndDensity >> 4),densityString((*planet).AtmoAndDensity & 15));
        for i in 0..NUM_USEFUL_ELEMENTS as usize {
            let entry = &(*planet).UsefulElements[i];
            if entry.Density == 0 {
                continue;
            }
            let quality = entry.Density >> 4;
            libc::fprintf(
                out,
                c"\t\t0 to %d %s-quality (+%d) deposits of %s (%s)\n".as_ptr(),
                (entry.Density & 15) as c_int,
                depositQualityString(quality),
                quality as c_int * 5,
                game_string(ELEMENTS_STRING_BASE as usize + entry.ElementType as usize),
                game_string(
                    CARGO_STRING_BASE as usize + 2 + element_category(entry.ElementType as usize),
                ),
            );
        }
        libc::fprintf(out, c"\n".as_ptr());
    }
    /// # Safety
    /// Game strings must be initialized. Returned storage is shared and non-reentrant.
    pub unsafe fn planetTypeString(index: c_int) -> *const c_char {
        static mut BUFFER: [c_char; 40] = [0; 40];
        let buffer = ptr::addr_of_mut!(BUFFER).cast();
        if index >= FIRST_GAS_GIANT as c_int {
            libc::snprintf(
                buffer,
                40,
                c"%s".as_ptr(),
                game_string(SCAN_STRING_BASE as usize + 4 + 51),
            );
        } else {
            libc::snprintf(
                buffer,
                40,
                c"%s %s".as_ptr(),
                game_string((SCAN_STRING_BASE as c_int + 4 + index) as usize),
                game_string(SCAN_STRING_BASE as usize + 4 + 50),
            );
        }
        buffer
    }
    /// # Safety
    /// Returned fallback-number storage is shared and must not be accessed concurrently.
    pub unsafe fn tectonicsString(value: BYTE) -> *const c_char {
        static mut BUFFER: [c_char; 5] = [0; 5];
        match value as u32 {
            NO_TECTONICS => c"none".as_ptr(),
            LOW_TECTONICS => c"low".as_ptr(),
            MED_TECTONICS => c"medium".as_ptr(),
            HIGH_TECTONICS => c"high".as_ptr(),
            SUPER_TECTONICS => c"super".as_ptr(),
            _ => {
                let buffer = ptr::addr_of_mut!(BUFFER).cast();
                libc::snprintf(buffer, 5, c"%d".as_ptr(), value as c_int);
                buffer
            }
        }
    }

    /// # Safety
    /// Element queue must be initialized and accessed on the game thread.
    pub unsafe fn findPlayerShip(player: SIZE) -> *mut STARSHIP {
        let mut handle = disp_q.head;
        while !handle.is_null() {
            let element = handle.cast::<ELEMENT>();
            let next = (*element).succ;
            if (*element).state_flags & PLAYER_SHIP as ELEMENT_FLAGS != 0
                && (*element).playerNr == player
            {
                return (*element).pParent.cast();
            }
            handle = next;
        }
        ptr::null_mut()
    }
    /// # Safety
    /// Must run on the game thread with battle and status context initialized.
    pub unsafe fn resetCrewBattle() {
        if GlobData.Game_state.CurrentActivity & IN_BATTLE as ACTIVITY == 0 || inHQSpace() != 0 {
            return;
        }
        let ship = findPlayerShip(RPG_PLAYER_NUM as SIZE);
        if ship.is_null() || (*ship).RaceDescPtr.is_null() {
            return;
        }
        let info = &(*(*ship).RaceDescPtr).ship_info;
        let delta = (info.max_crew as c_int - info.crew_level as c_int) as COUNT;
        let old = SetContext(StatusContext);
        DeltaCrew((*ship).hShip.cast(), delta as SIZE);
        SetContext(old);
    }
    /// # Safety
    /// Must run on the game thread with battle and status context initialized.
    pub unsafe fn resetEnergyBattle() {
        if GlobData.Game_state.CurrentActivity & IN_BATTLE as ACTIVITY == 0 || inHQSpace() != 0 {
            return;
        }
        let ship = findPlayerShip(RPG_PLAYER_NUM as SIZE);
        if ship.is_null() || (*ship).RaceDescPtr.is_null() {
            return;
        }
        let info = &(*(*ship).RaceDescPtr).ship_info;
        let delta = (info.max_energy as c_int - info.energy_level as c_int) as COUNT;
        let old = SetContext(StatusContext);
        DeltaEnergy((*ship).hShip.cast(), delta as SIZE);
        SetContext(old);
    }

    /// # Safety
    /// Game strings must be initialized and `out` an open C stream.
    pub unsafe fn dumpStrings(out: *mut FILE) {
        macro_rules! category {
            ($name:literal,$base:ident,$count:ident) => {
                (
                    concat!($name, "\0").as_ptr().cast::<c_char>(),
                    $base as usize,
                    $count as usize,
                )
            };
        }
        let categories = [
            (c"0".as_ptr(), 0, 0),
            category!("STAR_STRING_BASE", STAR_STRING_BASE, STAR_STRING_COUNT),
            category!(
                "DEVICE_STRING_BASE",
                DEVICE_STRING_BASE,
                DEVICE_STRING_COUNT
            ),
            category!("CARGO_STRING_BASE", CARGO_STRING_BASE, CARGO_STRING_COUNT),
            category!(
                "ELEMENTS_STRING_BASE",
                ELEMENTS_STRING_BASE,
                ELEMENTS_STRING_COUNT
            ),
            category!("SCAN_STRING_BASE", SCAN_STRING_BASE, SCAN_STRING_COUNT),
            category!("STAR_NUMBER_BASE", STAR_NUMBER_BASE, STAR_NUMBER_COUNT),
            category!(
                "PLANET_NUMBER_BASE",
                PLANET_NUMBER_BASE,
                PLANET_NUMBER_COUNT
            ),
            category!(
                "MONTHS_STRING_BASE",
                MONTHS_STRING_BASE,
                MONTHS_STRING_COUNT
            ),
            category!(
                "FEEDBACK_STRING_BASE",
                FEEDBACK_STRING_BASE,
                FEEDBACK_STRING_COUNT
            ),
            category!(
                "STARBASE_STRING_BASE",
                STARBASE_STRING_BASE,
                STARBASE_STRING_COUNT
            ),
            category!(
                "ENCOUNTER_STRING_BASE",
                ENCOUNTER_STRING_BASE,
                ENCOUNTER_STRING_COUNT
            ),
            category!(
                "NAVIGATION_STRING_BASE",
                NAVIGATION_STRING_BASE,
                NAVIGATION_STRING_COUNT
            ),
            category!(
                "NAMING_STRING_BASE",
                NAMING_STRING_BASE,
                NAMING_STRING_COUNT
            ),
            category!("MELEE_STRING_BASE", MELEE_STRING_BASE, MELEE_STRING_COUNT),
            category!(
                "SAVEGAME_STRING_BASE",
                SAVEGAME_STRING_BASE,
                SAVEGAME_STRING_COUNT
            ),
            category!(
                "OPTION_STRING_BASE",
                OPTION_STRING_BASE,
                OPTION_STRING_COUNT
            ),
            category!(
                "QUITMENU_STRING_BASE",
                QUITMENU_STRING_BASE,
                QUITMENU_STRING_COUNT
            ),
            category!(
                "STATUS_STRING_BASE",
                STATUS_STRING_BASE,
                STATUS_STRING_COUNT
            ),
            category!(
                "FLAGSHIP_STRING_BASE",
                FLAGSHIP_STRING_BASE,
                FLAGSHIP_STRING_COUNT
            ),
            category!(
                "ORBITSCAN_STRING_BASE",
                ORBITSCAN_STRING_BASE,
                ORBITSCAN_STRING_COUNT
            ),
            category!(
                "MAINMENU_STRING_BASE",
                MAINMENU_STRING_BASE,
                MAINMENU_STRING_COUNT
            ),
            category!(
                "NETMELEE_STRING_BASE",
                NETMELEE_STRING_BASE,
                NETMELEE_STRING_COUNT
            ),
            (
                c"GAMESTR_COUNT".as_ptr(),
                GAMESTR_COUNT as usize,
                usize::MAX,
            ),
        ];
        let num_strings = GetStringTableCount(GameStrings) as usize;
        for i in 0..categories.len() - 1 {
            if categories[i].1 + categories[i].2 != categories[i + 1].1 {
                libc::fprintf(
                    error_stream,
                    c"Error: String category list in dumpStrings() is not up to date.\n".as_ptr(),
                );
                return;
            }
        }
        if GAMESTR_COUNT as usize != num_strings {
            libc::fprintf(
                error_stream,
                c"Warning: GAMESTR_COUNT is %d, but GameStrings contains %d strings.\n".as_ptr(),
                GAMESTR_COUNT as c_int,
                num_strings as c_int,
            );
        }
        let mut category = 0;
        for i in 0..num_strings {
            while category < categories.len() && i >= (*categories.as_ptr().add(category + 1)).1 {
                category += 1;
            }
            libc::fprintf(
                out,
                c"[ %s + %d ]  %s\n".as_ptr(),
                categories[category].0,
                (i - categories[category].1) as c_int,
                game_string(i),
            );
        }
    }

    unsafe fn isContextVisible(context: CONTEXT) -> bool {
        let old = SetContext(context);
        let frame = GetContextFGFrame();
        SetContext(old);
        frame == Screen
    }
    unsafe fn countVisibleContexts() -> usize {
        let mut count = 0;
        let mut context = GetFirstContext();
        while !context.is_null() {
            if isContextVisible(context) {
                count += 1;
            }
            context = GetNextContext(context);
        }
        count
    }
    unsafe fn drawContext(context: CONTEXT, hue: f64) {
        let _draw_frame = GetContextFGFrame();
        let rectangle_color = hsvaToRgba(hue, 1.0, 0.5, 100);
        let line_color = hsvaToRgba(hue, 1.0, 1.0, 90);
        let text_color = line_color;
        let old_context = SetContext(context);
        let mut rect: RECT = mem::zeroed();
        let _have_clipping_rect = GetContextClipRect(&mut rect);
        SetContext(old_context);
        let p1 = rect.corner;
        let p2 = POINT {
            x: (rect.corner.x as c_int + rect.extent.width as c_int - 1) as COORD,
            y: rect.corner.y,
        };
        let p3 = POINT {
            x: rect.corner.x,
            y: (rect.corner.y as c_int + rect.extent.height as c_int - 1) as COORD,
        };
        let p4 = POINT { x: p2.x, y: p3.y };
        let old_color = SetContextForeGroundColor(rectangle_color);
        DrawFilledRectangle(&mut rect);
        SetContextForeGroundColor(line_color);
        for (first, second) in [(p1, p2), (p2, p4), (p1, p3), (p3, p4), (p1, p4), (p2, p3)] {
            DrawLine(&mut LINE { first, second });
        }
        let old_font = SetContextFont(TinyFont);
        SetContextForeGroundColor(text_color);
        let old_mode = SetContextDrawMode(DrawMode {
            kind: DrawKind_DRAW_ALPHA as BYTE,
            factor: text_color.a as SWORD,
        });
        let mut text: TEXT = mem::zeroed();
        text.baseline.x = ((p1.x as c_int + (p2.x as c_int + 1)) / 2) as COORD;
        text.baseline.y = (p1.y as c_int + 8) as COORD;
        text.pStr = GetContextName(context);
        text.align = TEXT_ALIGN_ALIGN_CENTER;
        text.CharCount = COUNT::MAX;
        font_DrawText(&mut text);
        SetContextDrawMode(old_mode);
        SetContextForeGroundColor(old_color);
        SetContextFont(old_font);
    }
    unsafe fn describeContext(out: *mut FILE, context: CONTEXT) {
        let old = SetContext(context);
        let mut rect: RECT = mem::zeroed();
        GetContextClipRect(&mut rect);
        libc::fprintf(
            out,
            c"Context '%s':\n\tClipRect = (%d, %d)-(%d, %d)  (%d x %d)\n".as_ptr(),
            GetContextName(context),
            rect.corner.x as c_int,
            rect.corner.y as c_int,
            rect.corner.x as c_int + rect.extent.width as c_int,
            rect.corner.y as c_int + rect.extent.height as c_int,
            rect.extent.width as c_int,
            rect.extent.height as c_int,
        );
        SetContext(old);
    }
    #[repr(C)]
    struct WaitState {
        InputFunc: Option<unsafe fn(*mut WaitState) -> BOOLEAN>,
    }
    unsafe fn waitForKey(_: *mut WaitState) -> BOOLEAN {
        if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0
            || PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0
        {
            return 0;
        }
        SleepThread(ONE_SECOND / 20);
        1
    }
    unsafe fn getScreen() -> FRAME {
        let old = SetContext(ScreenContext);
        let rectangle = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: ScreenWidth as _,
                height: ScreenHeight as _,
            },
        };
        let frame = CaptureDrawable(LoadDisplayPixmap(&rectangle, ptr::null_mut()));
        SetContext(old);
        frame
    }
    unsafe fn putScreen(frame: FRAME) {
        let old = SetContext(ScreenContext);
        DrawStamp(&mut STAMP {
            origin: POINT { x: 0, y: 0 },
            frame,
        });
        SetContext(old);
    }
    /// # Safety
    /// Must run on the main thread with live graphics and input systems.
    pub unsafe fn debugContexts() {
        static mut IN_DEBUG_CONTEXTS: bool = false;
        if ptr::read_volatile(ptr::addr_of!(IN_DEBUG_CONTEXTS)) {
            return;
        }
        ptr::write_volatile(ptr::addr_of_mut!(IN_DEBUG_CONTEXTS), true);
        let count = countVisibleContexts();
        if count != 0 {
            let saved = getScreen();
            FlushGraphics();
            let debug_context = CreateContextAux(c"debugDrawContext".as_ptr());
            let debug_frame = CaptureDrawable(CloneFrame(saved));
            let old = SetContext(debug_context);
            SetContextFGFrame(debug_frame);
            let increment = 360.0 / count as f64;
            let mut visible = 0;
            let mut context = GetFirstContext();
            while !context.is_null() {
                if context != debug_context {
                    if isContextVisible(context) {
                        drawContext(context, visible as f64 * increment);
                        visible += 1;
                    }
                    describeContext(error_stream, context);
                }
                context = GetNextContext(context);
            }
            putScreen(debug_frame);
            let mut state = WaitState {
                InputFunc: Some(waitForKey),
            };
            DoInput(ptr::addr_of_mut!(state).cast(), 1);
            SetContext(old);
            DestroyContext(debug_context);
            DestroyDrawable(ReleaseDrawable(debug_frame));
            putScreen(saved);
            DestroyDrawable(ReleaseDrawable(saved));
        }
        ptr::write_volatile(ptr::addr_of_mut!(IN_DEBUG_CONTEXTS), false);
    }
}

#[cfg(not(test))]
pub use runtime::{
    activateAllShips, calculateBioValue, calculateMineralValue, clearEscorts, debugContexts,
    debugKeyPressed, debugKeyPressedSynchronous, doInstantMove, dumpEvent, dumpEvents, dumpMoon,
    dumpPlanet, dumpPlanetType, dumpPlanetTypes, dumpStrings, dumpSystem, dumpUniverse,
    dumpUniverseToFile, equipShip, findPlayerShip, forAllMoons, forAllPlanetTypes, forAllPlanets,
    forAllStars, forwardToNextEvent, generateBioIndex, generateMineralIndex, giveDevices,
    planetTypeString, resetCrewBattle, resetEnergyBattle, showSpheres, tallyResources,
    tallyResourcesToFile, tectonicsString, uqm_debug_creature_bio_value, UniverseRecurse,
    DEBUG_HOOK, DISABLE_INTERACTIVITY, INSTANT_MOVE,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::queue::{AllocLink, InitQueue, PutQueue, Queue, UninitQueue};

    #[test]
    fn traversal_callbacks_borrow_typed_context_without_an_abi_cast() {
        fn system(_: *const STAR_DESC, _: *const SOLARSYS_STATE, events: &mut Vec<&str>) {
            events.push("system");
        }
        fn world(_: *const PLANET_DESC, events: &mut Vec<&str>) {
            events.push("world");
        }
        let mut events = Vec::new();
        let visitor = UniverseRecurseArg {
            systemFuncPre: Some(system),
            systemFuncPost: None,
            planetFuncPre: Some(world),
            planetFuncPost: None,
            moonFunc: None,
            arg: &mut events,
        };
        // These callbacks do not inspect the records; only their typed context.
        unsafe {
            visitor.systemFuncPre.unwrap()(std::ptr::null(), std::ptr::null(), visitor.arg);
            visitor.planetFuncPre.unwrap()(std::ptr::null(), visitor.arg);
        }
        assert_eq!(events, ["system", "world"]);
        let _: fn(u8) -> *const c_char = eventName;
    }

    #[test]
    fn clearing_escort_queue_returns_every_fragment_slot_to_pool() {
        unsafe {
            let mut queue = std::mem::MaybeUninit::<Queue>::zeroed().assume_init();
            assert!(InitQueue(
                &mut queue,
                3,
                std::mem::size_of::<SHIP_FRAGMENT>() as u16,
            ));
            let mut original = Vec::new();
            for _ in 0..3 {
                let slot = AllocLink(&mut queue);
                assert!(!slot.is_null());
                PutQueue(&mut queue, slot);
                original.push(slot);
            }

            clear_escort_queue(&mut queue);
            assert!(queue.head.is_null());
            assert!(queue.tail.is_null());

            let reusable: Vec<_> = (0..3).map(|_| AllocLink(&mut queue)).collect();
            assert!(reusable.iter().all(|slot| original.contains(slot)));
            assert!(AllocLink(&mut queue).is_null());
            assert!(UninitQueue(&mut queue));
        }
    }

    #[test]
    fn debug_names_use_header_indices_and_unknown_fallbacks() {
        unsafe {
            assert_eq!(
                std::ffi::CStr::from_ptr(eventName(HYPERSPACE_ENCOUNTER_EVENT as BYTE)),
                c"HYPERSPACE_ENCOUNTER_EVENT"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(starPresenceString(MELNORME8_DEFINED as BYTE)),
                c"Melnorme trader #8"
            );
            assert_eq!(std::ffi::CStr::from_ptr(starPresenceString(0)), c"");
            assert_eq!(
                std::ffi::CStr::from_ptr(bodyColorString(CYAN_BODY as BYTE)),
                c"cyan"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(worldGenAlgoString(CRATERED_ALGO as BYTE)),
                c"CRATERED_ALGO"
            );
            assert_eq!(std::ffi::CStr::from_ptr(eventName(BYTE::MAX)), c"???");
            assert_eq!(
                std::ffi::CStr::from_ptr(atmosphereString(BYTE::MAX)),
                c"super thick"
            );
        }
    }
    #[test]
    fn hsv_sectors_preserve_c_rounding_and_alpha() {
        let cases = [
            (0.0, (255, 0, 0)),
            (60.0, (255, 255, 0)),
            (120.0, (0, 255, 0)),
            (180.0, (0, 255, 255)),
            (240.0, (0, 0, 255)),
            (300.0, (255, 0, 255)),
        ];
        for (hue, (r, g, b)) in cases {
            let color = hsvaToRgba(hue, 1.0, 1.0, 90);
            assert_eq!((color.r, color.g, color.b, color.a), (r, g, b, 90));
        }
        // The C t expression intentionally differs from the usual HSV formula.
        let color = hsvaToRgba(30.0, 0.5, 1.0, 100);
        assert_eq!((color.r, color.g, color.b, color.a), (255, 64, 128, 100));
    }
}
