//! Debug cheats, universe inspection and graphics-context diagnostics.
#![cfg(feature = "debug-process")]

#[cfg(not(test))]
use crate::engine::battle::Cx;
#[cfg(not(test))]
use crate::engine::{Kernel, StoryKey, WorldData};
use crate::navigation::abi::*;
use std::ffi::{c_char, c_int};

mod equipment;

fn hsva_to_rgba(hue: f64, sat: f64, val: f64, alpha: BYTE) -> Color {
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
pub fn event_name(value: BYTE) -> *const c_char {
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

pub fn body_color_string(value: BYTE) -> *const c_char {
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

pub fn star_type_string(value: BYTE) -> *const c_char {
    match value as u32 {
        DWARF_STAR => c"dwarf".as_ptr(),
        GIANT_STAR => c"giant".as_ptr(),
        SUPER_GIANT_STAR => c"super giant".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn star_presence_string(value: BYTE) -> *const c_char {
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

pub fn world_size_string(value: BYTE) -> *const c_char {
    match value as u32 {
        SMALL_ROCKY_WORLD => c"small rocky world".as_ptr(),
        LARGE_ROCKY_WORLD => c"large rocky world".as_ptr(),
        GAS_GIANT => c"gas giant".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn world_gen_algo_string(value: BYTE) -> *const c_char {
    match value as u32 {
        TOPO_ALGO => c"TOPO_ALGO".as_ptr(),
        CRATERED_ALGO => c"CRATERED_ALGO".as_ptr(),
        GAS_GIANT_ALGO => c"GAS_GIANT_ALGO".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn atmosphere_string(value: BYTE) -> *const c_char {
    match value as u32 {
        LIGHT => c"thin".as_ptr(),
        MEDIUM => c"normal".as_ptr(),
        HEAVY => c"thick".as_ptr(),
        _ => c"super thick".as_ptr(),
    }
}

pub fn density_string(value: BYTE) -> *const c_char {
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

pub fn deposit_quality_string(value: BYTE) -> *const c_char {
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
    pub system_func_pre: Option<SystemVisitor<T>>,
    pub system_func_post: Option<SystemVisitor<T>>,
    pub planet_func_pre: Option<WorldVisitor<T>>,
    pub planet_func_post: Option<WorldVisitor<T>>,
    pub moon_func: Option<WorldVisitor<T>>,
    pub arg: &'a mut T,
}

#[cfg(not(test))]
mod runtime {
    use super::*;
    use crate::engine::World;

    use crate::planets::tables::{PLANET_ARRAY as planet_array, STARMAP_ARRAY as starmap_array};
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

    unsafe fn call_generate_for_scan_type(
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

    pub static mut INSTANT_MOVE: BOOLEAN = 0;
    pub static mut DISABLE_INTERACTIVITY: BOOLEAN = 0;
    pub static mut DEBUG_HOOK: Option<unsafe fn()> = None;

    /// # Safety
    /// Must run synchronously on the game thread with initialized game state.
    pub unsafe fn debug_key_pressed_synchronous() {
        equip_ship();
        give_devices();
        clear_escorts();
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
            let added = crate::app::bridge::enter(|cx| {
                crate::engine::world::fleet::add_escort_ships(
                    cx.world,
                    race as COUNT,
                    1,
                    crate::engine::world::fleet::captain_name,
                )
            });
            if added.is_some() {
                crate::battle::build_native::refresh_escorts();
            }
        }
        reset_crew_battle();
        reset_energy_battle();
        INSTANT_MOVE = BOOLEAN::from(INSTANT_MOVE == 0);
        show_spheres();
        activate_all_ships();
    }

    // The asynchronous C debug-key hook contains no enabled operations.
    pub fn debug_key_pressed() {}

    /// # Safety
    /// Must run on the game thread. The game clock and event queue must be live.
    pub unsafe fn forward_to_next_event(cx: &mut Cx<'_>, skip_hee: BOOLEAN) {
        if crate::time::clock_bridge::rust_clock_is_running(cx.world) == 0 {
            return;
        }
        LockGameClock();
        let mut done = skip_hee == 0;
        loop {
            let (date, selector) = {
                let events = cx.world.clock.events_ref();
                let Some(id) = events.head() else {
                    return;
                };
                let event = events.get(id);
                (
                    (event.year_index, event.month_index, event.day_index),
                    event.func_index,
                )
            };
            done |= u32::from(selector) != HYPERSPACE_ENCOUNTER_EVENT;
            while {
                let clock = &cx.world.data.game.game_clock;
                (clock.year_index, clock.month_index, clock.day_index) < date
            } {
                cx.with_legacy(|| MoveGameClockDays(1));
            }
            if done {
                break;
            }
        }
        UnlockGameClock();
    }
    /// # Safety
    /// `out` must be an open C stream and `event` a readable event record.
    pub unsafe fn dump_event(out: *mut FILE, event: &crate::engine::clock::CEvent) {
        libc::fprintf(
            out,
            c"%4u/%02u/%02u: %s\n".as_ptr(),
            event.year_index as u32,
            event.month_index as u32,
            event.day_index as u32,
            event_name(event.func_index),
        );
    }
    /// # Safety
    /// Must run with a live game clock and an open output stream.
    pub unsafe fn dump_events(out: *mut FILE) {
        let world = World::current();
        LockGameClock();
        world.read(|world| {
            let events = world.clock.events_ref();
            let mut next = events.head();
            while let Some(id) = next {
                next = events.succ(id);
                dump_event(out, events.get(id));
            }
        });
        UnlockGameClock();
    }

    /// # Safety
    /// Must run on the game thread with initialized SIS and optional solar system.
    pub unsafe fn equip_ship() {
        if !World::current().write(super::equipment::equip_flagship) {
            return;
        }
        if let Some(system) = pSolarSysState.as_mut() {
            World::current().read(|world| {
                super::equipment::update_flight(&world.data.flagship, system);
            });
        }
        if World::current().read(in_hqspace) != 0
            || crate::engine::World::current().read(|world| world.activity()) as u8 as u32
                == IN_INTERPLANETARY
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
    pub fn give_devices() {
        for key in [
            StoryKey::ROSY_SPHERE_ON_SHIP,
            StoryKey::ARTIFACT_2_ON_SHIP,
            StoryKey::ARTIFACT_3_ON_SHIP,
            StoryKey::SUN_DEVICE_ON_SHIP,
            StoryKey::UTWIG_BOMB_ON_SHIP,
            StoryKey::ULTRON_CONDITION,
            StoryKey::MAIDENS_ON_SHIP,
            StoryKey::TALKING_PET_ON_SHIP,
            StoryKey::AQUA_HELIX_ON_SHIP,
            StoryKey::CLEAR_SPINDLE_ON_SHIP,
            StoryKey::UMGAH_BROADCASTERS_ON_SHIP,
            StoryKey::TAALO_PROTECTOR_ON_SHIP,
            StoryKey::EGG_CASE0_ON_SHIP,
            StoryKey::EGG_CASE1_ON_SHIP,
            StoryKey::EGG_CASE2_ON_SHIP,
            StoryKey::SYREEN_SHUTTLE_ON_SHIP,
            StoryKey::VUX_BEAST_ON_SHIP,
            StoryKey::PORTAL_SPAWNER_ON_SHIP,
            StoryKey::PORTAL_KEY_ON_SHIP,
            StoryKey::BURV_BROADCASTERS_ON_SHIP,
            StoryKey::MOONBASE_ON_SHIP,
            StoryKey::DESTRUCT_CODE_ON_SHIP,
        ] {
            World::current().write(|world| world.set_story_state(key, 1));
        }
    }
    /// # Safety
    /// Must run on the game thread with the escort queue initialized.
    pub unsafe fn clear_escorts() {
        World::current().write(equipment::clear_escorts);
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
    pub unsafe fn do_instant_move() {
        let lease = World::current();
        let world = &mut **lease;
        let (log_x, log_y) = log_units();
        let universe_x = ((MAX_X_UNIVERSE as i32 + 1) >> 4) * 10;
        let universe_y = (MAX_Y_UNIVERSE as i32 + 1) >> 4;
        let game = ptr::addr_of_mut!(world.data.game);
        let sis = ptr::addr_of_mut!(world.data.flagship);
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
        if world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1 {
            let selected =
                crate::planets::starmap::find_star_in(world, None, (*game).autopilot, 0, 0);
            world.select_star(
                selected.as_ref(),
                crate::planets::starmap::current_catalog(),
            );
            if !world
                .star_pointer(crate::planets::starmap::current_catalog())
                .is_null()
            {
                world.set_story_state(StoryKey::USED_BROADCASTER, 0);
                world.set_activity(world.activity() & !(IN_BATTLE as ACTIVITY));
                (*game).ship_facing = 0;
                (*game).ip_planet = 0;
                (*game).in_orbit = 0;
                world.set_activity(world.activity() | START_INTERPLANETARY as ACTIVITY);
            }
        }
        (*game).autopilot.x = -1;
        (*game).autopilot.y = -1;
    }
    /// # Safety
    /// Must run on the game thread with the race queue initialized.
    pub unsafe fn show_spheres() {
        World::current().write(equipment::reveal_spheres);
    }
    /// # Safety
    /// Must run on the game thread with the race queue initialized.
    pub unsafe fn activate_all_ships() {
        World::current().write(equipment::activate_fleets);
    }

    /// # Safety
    /// Must run on the game thread. The callback must accept every star and its context.
    pub unsafe fn for_all_stars<T>(
        cx: &mut Cx<'_>,
        callback: unsafe fn(&mut Cx<'_>, *mut STAR_DESC, &mut T),
        arg: &mut T,
    ) {
        for i in 0..NUM_SOLAR_SYSTEMS as usize {
            callback(
                cx,
                ptr::addr_of_mut!(starmap_array).cast::<STAR_DESC>().add(i),
                arg,
            );
        }
    }
    /// # Safety
    /// `system` and `star` must be the active system. Callback and context must be valid.
    pub unsafe fn for_all_planets<T>(
        cx: &mut Cx<'_>,
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        callback: unsafe fn(
            &mut Cx<'_>,
            *mut STAR_DESC,
            *mut SOLARSYS_STATE,
            *mut PLANET_DESC,
            &mut T,
        ),
        arg: &mut T,
    ) {
        assert!(
            cx.world
                .star_pointer(crate::planets::starmap::current_catalog())
                == star.cast()
        );
        assert!(pSolarSysState == system);
        for i in 0..(*system).SunDesc[0].NumPlanets as usize {
            callback(
                cx,
                star,
                system,
                ptr::addr_of_mut!((*system).PlanetDesc[i]),
                arg,
            );
        }
    }
    /// # Safety
    /// The planet and system must be active and their generated moon records valid.
    pub unsafe fn for_all_moons<T>(
        cx: &mut Cx<'_>,
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        callback: unsafe fn(
            &mut Cx<'_>,
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
                cx,
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
    pub unsafe fn universe_recurse<T>(cx: &mut Cx<'_>, arg: &mut UniverseRecurseArg<'_, T>) {
        if arg.system_func_pre.is_none()
            && arg.system_func_post.is_none()
            && arg.planet_func_pre.is_none()
            && arg.planet_func_post.is_none()
            && arg.moon_func.is_none()
        {
            return;
        }
        LockGameClock();
        let saved_activity = cx.world.activity();
        DISABLE_INTERACTIVITY = 1;
        for_all_stars(cx, star_recurse, arg);
        DISABLE_INTERACTIVITY = 0;
        cx.world.set_activity(saved_activity);
        UnlockGameClock();
    }
    unsafe fn star_recurse<T>(
        cx: &mut Cx<'_>,
        star: *mut STAR_DESC,
        recurse: &mut UniverseRecurseArg<'_, T>,
    ) {
        let old_system = pSolarSysState;
        let old_star = cx
            .world
            .star_pointer(crate::planets::starmap::current_catalog());
        cx.world.select_star(
            (star.cast::<crate::engine::world::CStarDesc>()).as_ref(),
            crate::planets::starmap::current_catalog(),
        );
        (*SysGenRNG).reseed(GetRandomSeedForStar(star));
        let mut system: SOLARSYS_STATE = mem::zeroed();
        system.SunDesc[0].pPrevDesc = ptr::null_mut();
        system.SunDesc[0].rand_seed = (*SysGenRNG).next_random();
        system.SunDesc[0].data_index = (((*star).Type as u32
            & ((0xff << STAR_TYPE_SHIFT) & !(0xff << STAR_COLOR_SHIFT)))
            >> STAR_TYPE_SHIFT) as BYTE;
        system.SunDesc[0].location = POINT { x: 0, y: 0 };
        system.genFuncs = (getGenerateFunctions((*star).Index)).cast();
        pSolarSysState = &mut system;
        cx.with_legacy(|| ((*system.genFuncs).generatePlanets.unwrap())(&mut system));
        if let Some(cb) = recurse.system_func_pre {
            cx.with_legacy(|| cb(star, &system, recurse.arg));
        }
        if recurse.planet_func_pre.is_some()
            || recurse.planet_func_post.is_some()
            || recurse.moon_func.is_some()
        {
            for_all_planets(cx, star, &mut system, planet_recurse, recurse);
        }
        if let Some(cb) = recurse.system_func_post {
            cx.with_legacy(|| cb(star, &system, recurse.arg));
        }
        pSolarSysState = old_system;
        cx.world.select_star(
            (old_star).as_ref(),
            crate::planets::starmap::current_catalog(),
        );
    }
    unsafe fn planet_recurse<T>(
        cx: &mut Cx<'_>,
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        recurse: &mut UniverseRecurseArg<'_, T>,
    ) {
        assert!(
            cx.world
                .star_pointer(crate::planets::starmap::current_catalog())
                == star.cast()
        );
        assert!(pSolarSysState == system);
        (*planet).pPrevDesc = ptr::addr_of_mut!((*system).SunDesc[0]);
        if let Some(cb) = recurse.planet_func_pre {
            (*system).pOrbitalDesc = planet;
            cx.with_legacy(|| DoPlanetaryAnalysis(ptr::addr_of_mut!((*system).SysInfo), planet));
            ((*(*system).genFuncs).generateOrbital.unwrap())(cx, system, planet);
            cx.with_legacy(|| cb(planet, recurse.arg));
        }
        if recurse.moon_func.is_some() {
            (*SysGenRNG).reseed((*planet).rand_seed);
            cx.with_legacy(|| ((*(*system).genFuncs).generateMoons.unwrap())(system, planet));
            for_all_moons(cx, star, system, planet, moon_recurse, recurse);
        }
        if let Some(cb) = recurse.planet_func_post {
            (*system).pOrbitalDesc = planet;
            cx.with_legacy(|| DoPlanetaryAnalysis(ptr::addr_of_mut!((*system).SysInfo), planet));
            ((*(*system).genFuncs).generateOrbital.unwrap())(cx, system, planet);
            cx.with_legacy(|| cb(planet, recurse.arg));
        }
    }
    unsafe fn moon_recurse<T>(
        cx: &mut Cx<'_>,
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        moon: *mut PLANET_DESC,
        recurse: &mut UniverseRecurseArg<'_, T>,
    ) {
        assert!(
            cx.world
                .star_pointer(crate::planets::starmap::current_catalog())
                == star.cast()
        );
        assert!(pSolarSysState == system);
        (*moon).pPrevDesc = planet;
        if let Some(cb) = recurse.moon_func {
            (*system).pOrbitalDesc = moon;
            if (*moon).data_index != HIERARCHY_STARBASE as BYTE
                && (*moon).data_index != SA_MATRA as BYTE
            {
                cx.with_legacy(|| DoPlanetaryAnalysis(ptr::addr_of_mut!((*system).SysInfo), moon));
            }
            ((*(*system).genFuncs).generateOrbital.unwrap())(cx, system, moon);
            cx.with_legacy(|| cb(moon, recurse.arg));
        }
    }

    struct DumpUniverseArg {
        out: *mut FILE,
    }
    /// # Safety
    /// Must run on the game thread with an open output stream.
    pub unsafe fn dump_universe(cx: &mut Cx<'_>, out: *mut FILE) {
        let mut context = DumpUniverseArg { out };
        let mut recurse = UniverseRecurseArg {
            system_func_pre: Some(dump_system_callback),
            system_func_post: None,
            planet_func_pre: Some(dump_planet_callback),
            planet_func_post: None,
            moon_func: Some(dump_moon_callback),
            arg: &mut context,
        };
        universe_recurse(cx, &mut recurse);
    }
    unsafe fn write_generation_file(
        name: *const c_char,
        callback: impl FnOnce(*mut FILE),
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
    pub unsafe fn dump_universe_to_file(cx: &mut Cx<'_>) {
        write_generation_file(
            c"PlanetInfo".as_ptr(),
            |out| dump_universe(cx, out),
            c"*** Star dump complete. The game may be in an undefined state.\n".as_ptr(),
        );
    }
    unsafe fn dump_system_callback(
        star: *const STAR_DESC,
        system: *const SOLARSYS_STATE,
        arg: &mut DumpUniverseArg,
    ) {
        dump_system(arg.out, star, system);
    }
    /// # Safety
    /// Star and system records must be readable, and `out` an open stream.
    pub unsafe fn dump_system(
        out: *mut FILE,
        star: *const STAR_DESC,
        _system: *const SOLARSYS_STATE,
    ) {
        let mut name = [0 as c_char; 256];
        let mut buffer = [0 as c_char; 40];
        GetClusterName((star).cast(), name.as_mut_ptr());
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            c"%s %s".as_ptr(),
            body_color_string(
                (((*star).Type as u32 & ((0xff << STAR_COLOR_SHIFT) as BYTE as u32))
                    >> STAR_COLOR_SHIFT) as BYTE,
            ),
            star_type_string(
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
            star_presence_string((*star).Index),
        );
    }
    unsafe fn dump_planet_callback(planet: *const PLANET_DESC, arg: &mut DumpUniverseArg) {
        dump_planet(arg.out, planet);
    }
    /// # Safety
    /// Planet must belong to the active system, and `out` be an open stream.
    pub unsafe fn dump_planet(out: *mut FILE, planet: *const PLANET_DESC) {
        let world = WorldData::current();
        ((*(*pSolarSysState).genFuncs).generateName.unwrap())(pSolarSysState, planet);
        libc::fprintf(
            out,
            c"- %-37s  %s\n".as_ptr(),
            ptr::addr_of!((**world).flagship.planet_name).cast::<c_char>(),
            planet_type_string(((*planet).data_index & !(PLANET_SHIELDED as BYTE)) as c_int),
        );
        dump_world(out, planet);
    }
    unsafe fn dump_moon_callback(moon: *const PLANET_DESC, arg: &mut DumpUniverseArg) {
        dump_moon(arg.out, moon);
    }
    /// # Safety
    /// Moon must belong to the active system, and `out` be an open stream.
    pub unsafe fn dump_moon(out: *mut FILE, moon: *const PLANET_DESC) {
        let kind = if (*moon).data_index == HIERARCHY_STARBASE as BYTE {
            c"StarBase".as_ptr()
        } else if (*moon).data_index == SA_MATRA as BYTE {
            c"Sa-Matra".as_ptr()
        } else {
            planet_type_string(((*moon).data_index & !(PLANET_SHIELDED as BYTE)) as c_int)
        };
        libc::fprintf(
            out,
            c"  - Moon %-30c  %s\n".as_ptr(),
            b'a' as c_int + moon.offset_from(ptr::addr_of!((*pSolarSysState).MoonDesc[0])) as c_int,
            kind,
        );
        dump_world(out, moon);
    }
    unsafe fn dump_world(out: *mut FILE, world: *const PLANET_DESC) {
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
            calculate_bio_value(pSolarSysState, world) as c_int,
            calculate_mineral_value(pSolarSysState, world) as c_int,
        );
    }
    pub fn uqm_debug_creature_bio_value(kind: BYTE) -> c_int {
        crate::planet_side::creatures::rust_creature_bio_value(kind)
    }
    /// # Safety
    /// World must be the active orbital descriptor and its generation functions live.
    pub unsafe fn calculate_bio_value(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
    ) -> COUNT {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = call_generate_for_scan_type(
            system,
            world,
            COUNT::MAX,
            PlanetScanTypes_BIOLOGICAL_SCAN as BYTE,
            ptr::null_mut(),
        );
        let mut result: COUNT = 0;
        for i in 0..count {
            let mut info: NODE_INFO = mem::zeroed();
            call_generate_for_scan_type(
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
    pub unsafe fn generate_bio_index(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        bio: *mut COUNT,
    ) {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = call_generate_for_scan_type(
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
            call_generate_for_scan_type(
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
    pub unsafe fn calculate_mineral_value(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
    ) -> COUNT {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = call_generate_for_scan_type(
            system,
            world,
            COUNT::MAX,
            PlanetScanTypes_MINERAL_SCAN as BYTE,
            ptr::null_mut(),
        );
        let mut result: COUNT = 0;
        for i in 0..count {
            let mut info: NODE_INFO = mem::zeroed();
            call_generate_for_scan_type(
                system,
                world,
                i,
                PlanetScanTypes_MINERAL_SCAN as BYTE,
                &mut info,
            );
            result = result.wrapping_add(
                ((info.density >> 8) as u32
                    * WorldData::current().read(|owner| {
                        owner.game.element_worth[element_category(info.type_ as usize)]
                    }) as u32) as COUNT,
            );
        }
        result
    }
    /// # Safety
    /// World must be active, and `minerals` writable for all element-category counters.
    pub unsafe fn generate_mineral_index(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        minerals: *mut COUNT,
    ) {
        assert_eq!((*system).pOrbitalDesc.cast_const(), world);
        let count = call_generate_for_scan_type(
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
            call_generate_for_scan_type(
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
        mineral_count: COUNT,
        bio_count: COUNT,
    }
    /// # Safety
    /// Must run on the game thread with an open output stream.
    pub unsafe fn tally_resources(cx: &mut Cx<'_>, out: *mut FILE) {
        let mut context = TallyResourcesArg {
            out,
            mineral_count: 0,
            bio_count: 0,
        };
        let mut recurse = UniverseRecurseArg {
            system_func_pre: Some(tally_system_pre_callback),
            system_func_post: Some(tally_system_post_callback),
            planet_func_pre: Some(tally_planet_callback),
            planet_func_post: None,
            moon_func: Some(tally_moon_callback),
            arg: &mut context,
        };
        universe_recurse(cx, &mut recurse);
    }
    /// # Safety
    /// Must run on the game thread; generation can change game state.
    pub unsafe fn tally_resources_to_file(cx: &mut Cx<'_>) {
        write_generation_file(
            c"ResourceTally".as_ptr(),
            |out| tally_resources(cx, out),
            c"*** Resource tally complete. The game may be in an undefined state.\n".as_ptr(),
        );
    }
    unsafe fn tally_system_pre_callback(
        _: *const STAR_DESC,
        _: *const SOLARSYS_STATE,
        arg: &mut TallyResourcesArg,
    ) {
        let context = arg;
        context.mineral_count = 0;
        context.bio_count = 0;
    }
    unsafe fn tally_system_post_callback(
        star: *const STAR_DESC,
        _: *const SOLARSYS_STATE,
        arg: &mut TallyResourcesArg,
    ) {
        let context = arg;
        let mut name = [0 as c_char; 256];
        GetClusterName((star).cast(), name.as_mut_ptr());
        libc::fprintf(
            context.out,
            c"%s\t%d\t%d\n".as_ptr(),
            name.as_ptr(),
            context.mineral_count as c_int,
            context.bio_count as c_int,
        );
    }
    unsafe fn tally_planet_callback(world: *const PLANET_DESC, arg: &mut TallyResourcesArg) {
        tally_resources_world(arg, world);
    }
    unsafe fn tally_moon_callback(world: *const PLANET_DESC, arg: &mut TallyResourcesArg) {
        tally_resources_world(arg, world);
    }
    unsafe fn tally_resources_world(arg: &mut TallyResourcesArg, world: *const PLANET_DESC) {
        if (*world).data_index == HIERARCHY_STARBASE as BYTE {
            return;
        }
        if (*world).data_index == SA_MATRA as BYTE {
            return;
        }
        arg.bio_count = arg
            .bio_count
            .wrapping_add(calculate_bio_value(pSolarSysState, world));
        arg.mineral_count = arg
            .mineral_count
            .wrapping_add(calculate_mineral_value(pSolarSysState, world));
    }

    /// # Safety
    /// The callback must accept every immutable planet-type record and its context.
    pub unsafe fn for_all_planet_types<T>(
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
    pub unsafe fn dump_planet_types(out: *mut FILE) {
        let mut context = DumpPlanetTypesArg { out };
        for_all_planet_types(dump_planet_type_callback, &mut context);
    }
    unsafe fn dump_planet_type_callback(
        index: c_int,
        planet: *const PlanetFrame,
        arg: &mut DumpPlanetTypesArg,
    ) {
        dump_planet_type(arg.out, index, planet);
    }
    unsafe fn game_string(index: usize) -> *const c_char {
        GetStringAddress(SetAbsStringTableIndex(
            Kernel::current().read(|owner| owner.game_strings),
            index as COUNT,
        ))
    }
    /// # Safety
    /// Planet type must be readable, game strings initialized, and stream open.
    pub unsafe fn dump_planet_type(out: *mut FILE, index: c_int, planet: *const PlanetFrame) {
        libc::fprintf(out,c"%s\n\tType: %s\n\tColor: %s\n\tSurface generation algoritm: %s\n\tTectonics: %s\n\tAtmosphere: %s\n\tDensity: %s\n\tElements:\n".as_ptr(),
        planet_type_string(index),world_size_string((*planet).Type & PLAN_SIZE_MASK as BYTE),
        body_color_string((*planet).Type >> 4),world_gen_algo_string((*planet).Type & PLAN_ALGO_MASK as BYTE),
        tectonics_string((*planet).BaseTectonics),atmosphere_string((*planet).AtmoAndDensity >> 4),density_string((*planet).AtmoAndDensity & 15));
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
                deposit_quality_string(quality),
                quality as c_int * 5,
                game_string(ELEMENTS_STRING_BASE as usize + entry.ElementType as usize),
                game_string(
                    CARGO_STRING_BASE as usize + 2 + element_category(entry.ElementType as usize),
                ),
            );
        }
        libc::fprintf(out, c"\n".as_ptr());
    }
    macro_rules! category {
        ($name:literal,$base:ident,$count:ident) => {
            (
                concat!($name, "\0").as_ptr().cast::<c_char>(),
                $base as usize,
                $count as usize,
            )
        };
    }
    const STRING_CATEGORIES: &[(*const c_char, usize, usize)] = &[
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

    /// # Safety
    /// Game strings must be initialized. Returned storage is shared and non-reentrant.
    pub unsafe fn planet_type_string(index: c_int) -> *const c_char {
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
    pub unsafe fn tectonics_string(value: BYTE) -> *const c_char {
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

    pub fn find_player_ship(cx: &Cx<'_>, player: SIZE) -> Option<crate::engine::battle::ShipId> {
        let mut handle = cx.elements().head();
        while let Some(element) = handle {
            let state = cx.element(element);
            if state.flags & PLAYER_SHIP as u16 != 0 && state.player == player {
                return Some(cx.owner(element));
            }
            handle = cx.elements().succ(element);
        }
        None
    }
    /// # Safety
    /// Must run on the game thread with battle and status context initialized.
    pub unsafe fn reset_crew_battle() {
        if crate::engine::World::current().read(|world| world.activity()) & IN_BATTLE as ACTIVITY
            == 0
            || World::current().read(in_hqspace) != 0
        {
            return;
        }
        crate::app::bridge::enter(|cx| {
            let Some(ship) = find_player_ship(cx, RPG_PLAYER_NUM as SIZE) else {
                return;
            };
            if !cx.has_descriptor(ship) {
                return;
            }
            let info = cx.race_stats(ship).ship_info;
            let delta = (info.max_crew as c_int - info.crew_level as c_int) as COUNT;
            let old = SetContext(cx.kernel.status_context);
            cx.delta_crew(cx.ship_element(ship), delta as SIZE);
            SetContext(old);
        });
    }
    /// # Safety
    /// Must run on the game thread with battle and status context initialized.
    pub unsafe fn reset_energy_battle() {
        if crate::engine::World::current().read(|world| world.activity()) & IN_BATTLE as ACTIVITY
            == 0
            || World::current().read(in_hqspace) != 0
        {
            return;
        }
        crate::app::bridge::enter(|cx| {
            let Some(ship) = find_player_ship(cx, RPG_PLAYER_NUM as SIZE) else {
                return;
            };
            if !cx.has_descriptor(ship) {
                return;
            }
            let info = cx.race_stats(ship).ship_info;
            let delta = (info.max_energy as c_int - info.energy_level as c_int) as COUNT;
            let old = SetContext(cx.kernel.status_context);
            cx.delta_energy(cx.ship_element(ship), delta as SIZE);
            SetContext(old);
        });
    }

    /// # Safety
    /// Game strings must be initialized and `out` an open C stream.
    pub unsafe fn dump_strings(out: *mut FILE) {
        let categories = STRING_CATEGORIES;
        let num_strings =
            GetStringTableCount(Kernel::current().read(|owner| owner.game_strings)) as usize;
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

    unsafe fn is_context_visible(context: CONTEXT) -> bool {
        let old = SetContext(context);
        let frame = GetContextFGFrame();
        SetContext(old);
        frame == Kernel::current().read(|owner| owner.screen)
    }
    unsafe fn count_visible_contexts() -> usize {
        let mut count = 0;
        let mut context = GetFirstContext();
        while !context.is_null() {
            if is_context_visible(context) {
                count += 1;
            }
            context = GetNextContext(context);
        }
        count
    }
    unsafe fn draw_context(context: CONTEXT, hue: f64) {
        let _draw_frame = GetContextFGFrame();
        let rectangle_color = hsva_to_rgba(hue, 1.0, 0.5, 100);
        let line_color = hsva_to_rgba(hue, 1.0, 1.0, 90);
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
        let old_font = SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
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
    unsafe fn describe_context(out: *mut FILE, context: CONTEXT) {
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
        input_func: Option<unsafe fn(*mut WaitState) -> BOOLEAN>,
    }
    unsafe fn wait_for_key(_: *mut WaitState) -> BOOLEAN {
        if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0
            || PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0
        {
            return 0;
        }
        SleepThread(ONE_SECOND / 20);
        1
    }
    unsafe fn get_screen() -> FRAME {
        let old = SetContext(Kernel::current().read(|owner| owner.screen_context));
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
    unsafe fn put_screen(frame: FRAME) {
        let old = SetContext(Kernel::current().read(|owner| owner.screen_context));
        DrawStamp(&mut STAMP {
            origin: POINT { x: 0, y: 0 },
            frame,
        });
        SetContext(old);
    }
    /// # Safety
    /// Must run on the main thread with live graphics and input systems.
    pub unsafe fn debug_contexts() {
        static mut IN_DEBUG_CONTEXTS: bool = false;
        if ptr::read_volatile(ptr::addr_of!(IN_DEBUG_CONTEXTS)) {
            return;
        }
        ptr::write_volatile(ptr::addr_of_mut!(IN_DEBUG_CONTEXTS), true);
        let count = count_visible_contexts();
        if count != 0 {
            let saved = get_screen();
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
                    if is_context_visible(context) {
                        draw_context(context, visible as f64 * increment);
                        visible += 1;
                    }
                    describe_context(error_stream, context);
                }
                context = GetNextContext(context);
            }
            put_screen(debug_frame);
            let mut state = WaitState {
                input_func: Some(wait_for_key),
            };
            DoInput(ptr::addr_of_mut!(state).cast(), 1);
            SetContext(old);
            DestroyContext(debug_context);
            DestroyDrawable(ReleaseDrawable(debug_frame));
            put_screen(saved);
            DestroyDrawable(ReleaseDrawable(saved));
        }
        ptr::write_volatile(ptr::addr_of_mut!(IN_DEBUG_CONTEXTS), false);
    }
}

#[cfg(not(test))]
pub use runtime::{
    activate_all_ships, calculate_bio_value, calculate_mineral_value, clear_escorts,
    debug_contexts, debug_key_pressed, debug_key_pressed_synchronous, do_instant_move, dump_event,
    dump_events, dump_moon, dump_planet, dump_planet_type, dump_planet_types, dump_strings,
    dump_system, dump_universe, dump_universe_to_file, equip_ship, find_player_ship, for_all_moons,
    for_all_planet_types, for_all_planets, for_all_stars, forward_to_next_event,
    generate_bio_index, generate_mineral_index, give_devices, planet_type_string,
    reset_crew_battle, reset_energy_battle, show_spheres, tally_resources, tally_resources_to_file,
    tectonics_string, universe_recurse, uqm_debug_creature_bio_value, DEBUG_HOOK,
    DISABLE_INTERACTIVITY, INSTANT_MOVE,
};

#[cfg(test)]
mod tests {
    use super::*;

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
            system_func_pre: Some(system),
            system_func_post: None,
            planet_func_pre: Some(world),
            planet_func_post: None,
            moon_func: None,
            arg: &mut events,
        };
        // These callbacks do not inspect the records; only their typed context.
        unsafe {
            visitor.system_func_pre.unwrap()(std::ptr::null(), std::ptr::null(), visitor.arg);
            visitor.planet_func_pre.unwrap()(std::ptr::null(), visitor.arg);
        }
        assert_eq!(events, ["system", "world"]);
        let _: fn(u8) -> *const c_char = event_name;
    }

    #[test]
    fn clearing_escort_queue_returns_every_fragment_slot_to_pool() {
        let mut world = crate::engine::World::new();
        let mut original = Vec::new();
        for _ in 0..12 {
            let mut ships = world.built_ships();
            let id = ships.alloc().unwrap();
            ships.put(id);
            original.push(id);
        }
        assert!(world.built_ships().alloc().is_none());

        equipment::clear_escorts(&mut world);
        assert!(world.built_ships_ref().head().is_none());
        assert!(world.built_ships_ref().tail().is_none());
        for id in original {
            assert!(!world.built_ships_ref().contains(id));
        }
        for _ in 0..12 {
            assert!(world.built_ships().alloc().is_some());
        }
        assert!(world.built_ships().alloc().is_none());
    }

    #[test]
    fn debug_names_use_header_indices_and_unknown_fallbacks() {
        unsafe {
            assert_eq!(
                std::ffi::CStr::from_ptr(event_name(HYPERSPACE_ENCOUNTER_EVENT as BYTE)),
                c"HYPERSPACE_ENCOUNTER_EVENT"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(star_presence_string(MELNORME8_DEFINED as BYTE)),
                c"Melnorme trader #8"
            );
            assert_eq!(std::ffi::CStr::from_ptr(star_presence_string(0)), c"");
            assert_eq!(
                std::ffi::CStr::from_ptr(body_color_string(CYAN_BODY as BYTE)),
                c"cyan"
            );
            assert_eq!(
                std::ffi::CStr::from_ptr(world_gen_algo_string(CRATERED_ALGO as BYTE)),
                c"CRATERED_ALGO"
            );
            assert_eq!(std::ffi::CStr::from_ptr(event_name(BYTE::MAX)), c"???");
            assert_eq!(
                std::ffi::CStr::from_ptr(atmosphere_string(BYTE::MAX)),
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
            let color = hsva_to_rgba(hue, 1.0, 1.0, 90);
            assert_eq!((color.r, color.g, color.b, color.a), (r, g, b, 90));
        }
        // The C t expression intentionally differs from the usual HSV formula.
        let color = hsva_to_rgba(30.0, 0.5, 1.0, 100);
        assert_eq!((color.r, color.g, color.b, color.a), (255, 64, 128, 100));
    }
}
