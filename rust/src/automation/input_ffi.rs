//! C-facing ABI shells for input automation.
//!
//! These are the `extern "C"` exports called from `gameinp.c::DoInput`.
//! Each follows the execution-contract §3 shell order:
//! 1. ABI entry counter (saturating)
//! 2. Acquire-load activation (inactive → neutral fast path)
//! 3. Active gate entry
//! 4. Depth/reentry guard
//! 5. Terminal mirror check
//! 6. Pure transition under mutex
//! 7. Unlock before external work
//! 8. External effects (setter/getter)
//! 9. Ordered publish/cancel
//! 10. Validated commit
//! 11. Conservative fallback on error/panic
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
//! @requirement REQ-INJECT-001..007, REQ-FFI-001

use crate::automation::coordinator::Coordinator;
use crate::automation::runtime::RuntimeModel;
#[cfg(not(test))]
use crate::c_bindings::controller_input::{CurrentInputState, PulsedInputState};
#[cfg(not(test))]
use crate::c_bindings::global_state::{PLANET_DESC, SOLARSYS_STATE};

// ===========================================================================
//  C global input state — FFI access to ImmediateInputState.menu[]
// ===========================================================================

/// Snapshot of real interplanetary navigation state.
#[repr(C)]
#[derive(Default)]
pub(crate) struct NavigationSnapshot {
    pub(crate) active: i32,
    pub(crate) in_ip_flight: i32,
    pub(crate) in_orbit: i32,
    pub(crate) wait_intersect: u16,
    pub(crate) inner_planet: i32,
    pub(crate) orbital_moon: i32,
    pub(crate) orbital_data_index: i32,
    pub(crate) target_data_index: i32,
    pub(crate) ship_x: i32,
    pub(crate) ship_y: i32,
    pub(crate) ship_facing: i32,
    pub(crate) target_x: i32,
    pub(crate) target_y: i32,
    pub(crate) velocity_x: i32,
    pub(crate) velocity_y: i32,
    pub(crate) view_center_x: i32,
    pub(crate) view_center_y: i32,
}

#[cfg(not(test))]
extern "C" {
    fn c_automation_set_immediate_menu_key(index: i32, value: i32) -> i32;
    fn c_automation_set_immediate_player_key(index: i32, value: i32) -> i32;
    #[link_name = "GlobData"]
    static mut GLOB_DATA: crate::comm::locdata::CGlobData;
    #[allow(non_snake_case)]
    fn GetFrameIndex(frame: *mut std::ffi::c_void) -> u16;
    /// Live orbit predicate: `playerInSolarSystem() && pSolarSysState->InOrbit`.
    #[allow(non_snake_case)]
    fn playerInPlanetOrbit() -> bool;
    #[link_name = "ScreenWidth"]
    static SCREEN_WIDTH: std::ffi::c_int;
    #[link_name = "ScreenHeight"]
    static SCREEN_HEIGHT: std::ffi::c_int;
}
/// The interplanetary view geometry derived from the live screen size.
///
/// These mirror the C macros in `units.h` and `planets.h`, which are all
/// functions of the runtime `ScreenWidth`/`ScreenHeight` globals rather than
/// compile-time constants. Hardcoding them desynchronises every
/// display/location conversion whenever the game is not at one specific
/// resolution.
///
/// C: `sc2/src/uqm/units.h`, `sc2/src/uqm/planets/planets.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ViewGeometry {
    pub(crate) half_width: i32,
    pub(crate) half_height: i32,
    pub(crate) display_to_loc: i32,
}

impl ViewGeometry {
    /// Compute the geometry for a given screen size.
    ///
    /// C: `SPACE_WIDTH = SCREEN_WIDTH - STATUS_WIDTH - SAFE_X * 2`,
    /// `SIS_SCREEN_WIDTH = SPACE_WIDTH - 14`,
    /// `SIS_SCREEN_HEIGHT = SCREEN_HEIGHT - SAFE_Y * 2 - 13`,
    /// `DISPLAY_FACTOR = (SIS_SCREEN_WIDTH >> 1) - 8`,
    /// `DISPLAY_TO_LOC = DISPLAY_FACTOR >> 1`.
    /// `SAFE_X` and `SAFE_Y` are both 0.
    pub(crate) const fn from_screen(screen_width: i32, screen_height: i32) -> Self {
        const STATUS_WIDTH: i32 = 64;
        let sis_screen_width = screen_width - STATUS_WIDTH - 14;
        let sis_screen_height = screen_height - 13;
        let display_factor = (sis_screen_width >> 1) - 8;
        Self {
            half_width: sis_screen_width >> 1,
            half_height: sis_screen_height >> 1,
            display_to_loc: display_factor >> 1,
        }
    }

    /// Read the geometry from the live C screen-size globals.
    #[cfg(not(test))]
    fn current() -> Self {
        let (width, height) = screen_size();
        Self::from_screen(width, height)
    }
}

/// The live `ScreenWidth` and `ScreenHeight`.
#[cfg(not(test))]
pub(crate) fn screen_size() -> (i32, i32) {
    // SAFETY: `ScreenWidth`/`ScreenHeight` are set once during graphics
    // init and are read-only thereafter; this runs on the game thread.
    unsafe {
        (
            std::ptr::addr_of!(SCREEN_WIDTH).read(),
            std::ptr::addr_of!(SCREEN_HEIGHT).read(),
        )
    }
}

/// Without the linked game the screen is the baseline size.
#[cfg(test)]
pub(crate) fn screen_size() -> (i32, i32) {
    (DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT)
}

/// The game's baseline screen size, used only when the C globals are absent.
#[cfg(test)]
const DEFAULT_SCREEN_WIDTH: i32 = 320;
/// The game's baseline screen size, used only when the C globals are absent.
#[cfg(test)]
const DEFAULT_SCREEN_HEIGHT: i32 = 240;

/// Read the target-independent flight state of the live solar system.
///
/// # Safety
/// Must run on the game thread while `solar_system` is the live
/// `pSolarSysState`, as the automation input hook does.
#[cfg(not(test))]
unsafe fn read_flight_state(solar_system: &SOLARSYS_STATE, snapshot: &mut NavigationSnapshot) {
    snapshot.active = 1;
    snapshot.in_ip_flight = i32::from(solar_system.InIpFlight != 0);
    // Ask the game for live orbit state rather than inferring it. The
    // authoritative flag is `SOLARSYS_STATE::InOrbit`, which the exported
    // predicate reads.
    // `GLOBAL(in_orbit)` is not usable here: it only encodes orbit state
    // for save/load. `!InIpFlight` is not usable either: the game clears
    // `InIpFlight` when entering an encounter as well as an orbit, so it
    // would report orbit during encounters.
    snapshot.in_orbit = i32::from(playerInPlanetOrbit());
    snapshot.wait_intersect = solar_system.WaitIntersect;
    let game_state = std::ptr::addr_of!(GLOB_DATA.game_state);
    let ship_stamp = std::ptr::addr_of!((*game_state).ship_stamp).read();
    snapshot.ship_x = i32::from(ship_stamp.origin.x);
    snapshot.ship_y = i32::from(ship_stamp.origin.y);
    if !ship_stamp.frame.is_null() {
        snapshot.ship_facing = i32::from(GetFrameIndex(ship_stamp.frame));
    }
    let velocity = std::ptr::addr_of!((*game_state).velocity).read();
    (snapshot.velocity_x, snapshot.velocity_y) = velocity.get_current_components();

    if !solar_system.pOrbitalDesc.is_null() {
        snapshot.orbital_data_index = i32::from((*solar_system.pOrbitalDesc).data_index);
        let planet_start = solar_system.PlanetDesc.as_ptr() as usize;
        let moon_start = solar_system.MoonDesc.as_ptr() as usize;
        let desc_size = std::mem::size_of::<PLANET_DESC>();
        let orbital_address = solar_system.pOrbitalDesc as usize;
        if orbital_address >= moon_start
            && orbital_address < moon_start + solar_system.MoonDesc.len() * desc_size
        {
            snapshot.orbital_moon = ((orbital_address - moon_start) / desc_size) as i32;
        }
        let sun = solar_system.SunDesc.as_ptr();
        let orbital_planet =
            if std::ptr::eq((*solar_system.pOrbitalDesc).pPrevDesc.cast_const(), sun) {
                solar_system.pOrbitalDesc
            } else {
                (*solar_system.pOrbitalDesc).pPrevDesc
            } as usize;
        if orbital_planet >= planet_start {
            snapshot.inner_planet = ((orbital_planet - planet_start) / desc_size) as i32;
        }
    }
}

/// The flagship's authoritative logical coordinates, `GLOBAL_SIS(log_x/log_y)`.
///
/// Hyperspace flight moves exactly these fields (`hyper.c`), so they are the
/// position a movement assertion must read.
#[cfg(not(test))]
pub(crate) fn hyperspace_position() -> Option<(i32, i32)> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, so `GlobData` cannot change during this read.
    unsafe {
        let sis_state = std::ptr::addr_of!(GLOB_DATA.sis_state);
        Some((
            std::ptr::addr_of!((*sis_state).log_x).read(),
            std::ptr::addr_of!((*sis_state).log_y).read(),
        ))
    }
}

/// Without the linked game there is no position to read.
#[cfg(test)]
pub(crate) fn hyperspace_position() -> Option<(i32, i32)> {
    None
}

/// The hyperspace flagship's own `ShipFacing`, read from the player's
/// `STARSHIP` as `SaveSisHyperState` finds it. `None` when no flagship is in
/// flight or its queues are inconsistent. Nothing is written.
#[cfg(not(test))]
pub(crate) fn hyperspace_facing() -> Option<u8> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, after `InitKernel` and `InitSpace` set up the queues.
    let facing = unsafe { crate::savegame::runtime::live::live_flagship_facing() };
    facing
        .ok()
        .flatten()
        .and_then(|facing| u8::try_from(facing).ok())
}

/// Without the linked game there is no facing to read.
#[cfg(test)]
pub(crate) fn hyperspace_facing() -> Option<u8> {
    None
}

/// The flagship's authoritative `FuelOnBoard` and `ResUnits`, in that order.
///
/// `DeltaSISGauges` changes exactly these `SIS_STATE` fields, so a reward or
/// a purchase is visible here only after the game applied it.
#[cfg(not(test))]
pub(crate) fn ship_resources() -> Option<(u32, u32)> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, so `GlobData` cannot change during this read.
    unsafe {
        let sis_state = std::ptr::addr_of!(GLOB_DATA.sis_state);
        Some((
            std::ptr::addr_of!((*sis_state).fuel_on_board).read(),
            std::ptr::addr_of!((*sis_state).res_units).read(),
        ))
    }
}

/// Without the linked game there is no flagship state to read.
#[cfg(test)]
pub(crate) fn ship_resources() -> Option<(u32, u32)> {
    None
}

/// The flagship's `CrewEnlisted` and `NumLanders`, in that order.
#[cfg(not(test))]
pub(crate) fn sis_crew_and_landers() -> Option<(u16, u8)> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, so `GlobData` cannot change during this read.
    unsafe {
        let sis_state = std::ptr::addr_of!(GLOB_DATA.sis_state);
        Some((
            std::ptr::addr_of!((*sis_state).crew_enlisted).read(),
            std::ptr::addr_of!((*sis_state).num_landers).read(),
        ))
    }
}

/// Without the linked game there is no flagship state to read.
#[cfg(test)]
pub(crate) fn sis_crew_and_landers() -> Option<(u16, u8)> {
    None
}

type SisFittings = ([u8; 11], [u8; 8], [u8; 16], u16);

#[cfg(not(test))]
pub(crate) fn sis_fittings() -> Option<SisFittings> {
    unsafe {
        let sis = std::ptr::addr_of!(GLOB_DATA.sis_state).read();
        Some((
            sis.drive_slots,
            sis.jet_slots,
            sis.module_slots,
            sis.total_element_mass,
        ))
    }
}
#[cfg(test)]
pub(crate) fn sis_fittings() -> Option<SisFittings> {
    None
}

/// The game clock's `(day, month, year)` indices.
#[cfg(not(test))]
pub(crate) fn game_date() -> Option<(u8, u8, u16)> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, so `GlobData` cannot change during this read.
    unsafe {
        let clock = std::ptr::addr_of!(GLOB_DATA.game_state.game_clock);
        Some((
            std::ptr::addr_of!((*clock).day_index).read(),
            std::ptr::addr_of!((*clock).month_index).read(),
            std::ptr::addr_of!((*clock).year_index).read(),
        ))
    }
}

/// Without the linked game there is no clock to read.
#[cfg(test)]
pub(crate) fn game_date() -> Option<(u8, u8, u16)> {
    None
}

/// The flagship's authoritative `ElementAmounts`, indexed by element category.
///
/// The lander hold is added to these when it returns to the ship, so they
/// change only after the trip. Nothing is written.
#[cfg(not(test))]
pub(crate) fn ship_element_amounts() -> Option<[u16; 8]> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, so `GlobData` cannot change during this read.
    unsafe {
        let sis_state = std::ptr::addr_of!(GLOB_DATA.sis_state);
        Some(std::ptr::addr_of!((*sis_state).element_amounts).read())
    }
}

/// Without the linked game there is no flagship state to read.
#[cfg(test)]
pub(crate) fn ship_element_amounts() -> Option<[u16; 8]> {
    None
}

/// The ships left on each side of the live or last battle, in player order.
///
/// `tactrans.c` decrements these when a ship dies. Nothing is written.
#[cfg(not(test))]
pub(crate) fn battle_fleets() -> Option<(u8, u8)> {
    // SAFETY: the production game invokes the automation hooks synchronously
    // on the game thread, so the counters cannot change during this read.
    let counters = unsafe { crate::battle::ffi::live_counters::battle_counters() };
    Some((counters[0], counters[1]))
}

/// Without the linked game there are no fleets to read.
#[cfg(test)]
pub(crate) fn battle_fleets() -> Option<(u8, u8)> {
    None
}

/// Sample the live solar system for flying out of it, without a target.
///
/// `active` is 0 whenever no solar system is live, which is how a departure
/// observes that production has torn interplanetary flight down.
#[cfg(not(test))]
pub(crate) fn departure_snapshot() -> NavigationSnapshot {
    let geometry = ViewGeometry::current();
    let mut snapshot = NavigationSnapshot {
        inner_planet: -1,
        orbital_moon: -1,
        orbital_data_index: -1,
        target_data_index: -1,
        view_center_x: geometry.half_width,
        view_center_y: geometry.half_height,
        ..NavigationSnapshot::default()
    };
    // SAFETY: the production game invokes this hook synchronously on the game
    // thread, so the solar-system pointer and its state cannot change here.
    unsafe {
        if let Some(solar_system) = crate::c_bindings::solar_system::current().as_ref() {
            read_flight_state(solar_system, &mut snapshot);
        }
    }
    snapshot
}

#[cfg(test)]
pub(crate) fn departure_snapshot() -> NavigationSnapshot {
    navigation_snapshot(0, None)
}

/// Copy a live group's target geometry without moving it or invoking contact.
#[cfg(not(test))]
pub(crate) fn contact_snapshot(race: u8) -> Option<(NavigationSnapshot, bool)> {
    use crate::c_bindings::global_state::POINT;
    use crate::savegame::runtime::groups::CIpGroup;
    use crate::savegame::runtime::queues::{walk, QueueKind};
    extern "C" {
        fn locationToDisplay(location: POINT, radius: i16) -> POINT;
    }
    unsafe {
        let solar = crate::c_bindings::solar_system::current().as_ref()?;
        let queue = &*std::ptr::addr_of!(GLOB_DATA.game_state.ip_group_q);
        let group = walk::<CIpGroup>(queue, QueueKind::IpGroups)
            .expect("live IP group queue")
            .into_iter()
            .find(|group| (**group).race_id == race && (**group).in_system != 0)?;
        let system = (*group).sys_loc;
        let mut snapshot = if system == 0 {
            departure_snapshot()
        } else {
            navigation_snapshot(i32::from(system) - 1, None)
        };
        let same_view = (snapshot.inner_planet + 1) as u8 == system;
        if same_view {
            let radius = if system == 0 {
                solar.SunDesc[0].radius
            } else {
                128 << 6
            };
            let point = locationToDisplay(
                POINT {
                    x: (*group).loc.x,
                    y: (*group).loc.y,
                },
                radius,
            );
            snapshot.target_x = i32::from(point.x);
            snapshot.target_y = i32::from(point.y);
            // The contact controller pursues a ship in this view, not its planet.
            snapshot.inner_planet = -1;
        }
        Some((snapshot, same_view))
    }
}

#[cfg(test)]
pub(crate) fn contact_snapshot(_race: u8) -> Option<(NavigationSnapshot, bool)> {
    None
}

#[cfg(not(test))]
pub(crate) fn contact_race() -> Option<u8> {
    unsafe {
        std::ptr::addr_of!(GLOB_DATA.game_state.npc_built_ship_q.head)
            .read()
            .cast::<crate::comm::dispatch::CShipFragment>()
            .as_ref()
            .map(|ship| ship.race_id)
    }
}

#[cfg(test)]
pub(crate) fn contact_race() -> Option<u8> {
    None
}

/// Where a planet is on the display right now.
///
/// In the outer system `PLANET_DESC::image.origin` is the planet's display
/// position, converted the way `displayToLocation` followed by
/// `locationToDisplay` would round it. Inside the planet's own inner system
/// that field is not updated (`ValidateOrbit` only writes it for the outer
/// view and for moons, and `EnterPlanetOrbit` documents it as "for the outer
/// view"), while the planet is drawn at the view centre. Steering at the stale
/// origin sends the flagship out of the inner system instead of into the
/// planet.
///
/// C integer division truncates toward zero, as Rust's does.
fn planet_display_target(
    inside_target_inner_system: bool,
    outer_origin: (i32, i32),
    radius: i32,
    geometry: &ViewGeometry,
) -> (i32, i32) {
    if inside_target_inner_system {
        return (geometry.half_width, geometry.half_height);
    }
    if radius == 0 {
        return (0, 0);
    }
    let location_x = (outer_origin.0 - geometry.half_width) * radius / geometry.display_to_loc;
    let location_y = (outer_origin.1 - geometry.half_height) * radius / geometry.display_to_loc;
    (
        geometry.half_width + location_x * geometry.display_to_loc / radius,
        geometry.half_height + location_y * geometry.display_to_loc / radius,
    )
}

#[cfg(not(test))]
pub(crate) fn navigation_snapshot(
    target_planet: i32,
    target_moon: Option<i32>,
) -> NavigationSnapshot {
    let geometry = ViewGeometry::current();
    let mut snapshot = NavigationSnapshot {
        inner_planet: -1,
        orbital_moon: -1,
        orbital_data_index: -1,
        target_data_index: -1,
        view_center_x: geometry.half_width,
        view_center_y: geometry.half_height,
        ..NavigationSnapshot::default()
    };

    unsafe {
        // The production game invokes this hook synchronously on the game
        // thread, so these mutable C globals cannot change during a snapshot.
        let solar_system_ptr = crate::c_bindings::solar_system::current();
        let Some(solar_system) = solar_system_ptr.as_ref() else {
            return snapshot;
        };
        if target_planet < 0 || target_planet >= i32::from(solar_system.SunDesc[0].NumPlanets) {
            return snapshot;
        }
        read_flight_state(solar_system, &mut snapshot);

        if let Some(moon) = target_moon {
            if snapshot.inner_planet == target_planet
                && moon >= 0
                && moon < i32::from(solar_system.PlanetDesc[target_planet as usize].NumPlanets)
            {
                let target = &solar_system.MoonDesc[moon as usize];
                snapshot.target_data_index = i32::from(target.data_index);
                let target_origin = target.image.origin;
                snapshot.target_x = i32::from(target_origin.x);
                snapshot.target_y = i32::from(target_origin.y);
                return snapshot;
            }
        }

        let target_origin = solar_system.PlanetDesc[target_planet as usize].image.origin;
        let (target_x, target_y) = planet_display_target(
            snapshot.inner_planet == target_planet,
            (i32::from(target_origin.x), i32::from(target_origin.y)),
            i32::from(solar_system.SunDesc[0].radius),
            &geometry,
        );
        snapshot.target_x = target_x;
        snapshot.target_y = target_y;
    }
    snapshot
}

#[cfg(test)]
pub(crate) fn navigation_snapshot(
    _target_planet: i32,
    _target_moon: Option<i32>,
) -> NavigationSnapshot {
    // `active` stays 0, so no controls are derived from this snapshot. The
    // view centre is still reported honestly so that a caller which reads it
    // without checking `active` cannot silently steer against (0, 0).
    let geometry = ViewGeometry::from_screen(DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT);
    NavigationSnapshot {
        view_center_x: geometry.half_width,
        view_center_y: geometry.half_height,
        ..NavigationSnapshot::default()
    }
}

/// The global runtime model for automation.
///
/// In production this is initialized by `setup_automation()`. In tests and
/// inactive mode it stays `None` (inactive fast path).
static AUTOMATION_RT: std::sync::OnceLock<RuntimeModel> = std::sync::OnceLock::new();

/// Initialize the automation runtime model. Called by `setup_automation()`.
pub fn init_automation_runtime() {
    let _ = AUTOMATION_RT.get_or_init(RuntimeModel::new);
}

/// Check if automation is active.
fn is_automation_active() -> bool {
    if let Some(rt) = AUTOMATION_RT.get() {
        rt.mirror.is_active()
    } else {
        false
    }
}

/// Get the runtime model if it exists.
fn with_runtime() -> Option<&'static RuntimeModel> {
    AUTOMATION_RT.get()
}

/// Get the runtime model if it exists (public for coordinator).
pub fn get_runtime() -> Option<&'static RuntimeModel> {
    AUTOMATION_RT.get()
}

// ===========================================================================
//  Service hook: called before UpdateInputState in DoInput
// ===========================================================================

/// C-callable automation service hook for `DoInput`.
///
/// Called after both pumps (TFB_ProcessEvents + TaskSwitch) and before
/// the sole `UpdateInputState`. Returns 1 (stop) if automation wants to
/// stop, 0 (continue) otherwise.
///
/// In inactive mode: returns 0 (no stop) via fast path.
/// In active mode: follows the full ABI shell.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-001, REQ-INJECT-002
pub extern "C" fn rust_automation_service_do_input() -> i32 {
    // Step 1: ABI entry (saturating).
    if let Some(rt) = with_runtime() {
        rt.record_abi_entry();
    } else {
        return 0; // No runtime → inactive fast path.
    }

    // Step 2: Acquire-load activation.
    if !is_automation_active() {
        return 0; // Inactive fast path: no stop.
    }

    // Step 2b: Check terminal — if already terminal, return stop to
    // break the DoInput loop. This is the key mechanism that makes
    // DoInput break out when the scheduler finishes.
    let rt = match with_runtime() {
        Some(rt) => rt,
        None => return 0,
    };

    if rt.mirror.is_terminal() {
        // Re-assert CHECK_ABORT every frame. Game logic (handle_select)
        // can overwrite CurrentActivity, clearing our CHECK_ABORT. By
        // re-asserting on every DoInput call, we ensure the activity
        // state machine's should_continue() check sees CHECK_ABORT and
        // exits the inner loop.
        #[cfg(not(test))]
        unsafe {
            crate::mainloop::c_extern::set_current_activity(
                crate::mainloop::c_extern::get_current_activity() | 0x4000,
            );
        }
        return 1; // Terminal: stop the DoInput loop.
    }

    // Step 4: Feed the input callback to the coordinator (scheduler+watchdog).
    if Coordinator::is_active() && Coordinator::process_input() {
        return 1; // Stop requested by scheduler or watchdog.
    }

    0
}

// ===========================================================================
//  Observation hook: called after UpdateInputState in DoInput
// ===========================================================================

/// C-callable automation observation hook for after `UpdateInputState`.
///
/// Reads current/pulsed menu keys via production getters, traces the
/// observation, and returns stop if the scheduler says to stop.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006, REQ-INJECT-007
pub extern "C" fn rust_automation_after_input_update() -> i32 {
    // Step 1: ABI entry (saturating).
    if let Some(rt) = with_runtime() {
        rt.record_abi_entry();
    } else {
        return 0; // No runtime → inactive fast path.
    }

    // Step 2: Acquire-load activation.
    if !is_automation_active() {
        return 0; // Inactive fast path: no stop.
    }

    let rt = match with_runtime() {
        Some(rt) => rt,
        None => return 0,
    };

    // Step 2b: Check terminal — if terminal, return stop to break DoInput.
    if rt.mirror.is_terminal() {
        return 1; // Terminal: stop the DoInput loop.
    }

    // Step 3: Commit the scripted player-input observation at the exact
    // production boundary that follows UpdateInputState.
    Coordinator::process_player_input_observation().into()
}

/// Advance active automation at non-`DoInput` main-thread boundaries.
///
/// This is used after synchronous communication returns, where there may be no
/// subsequent legacy input callback before Rust dispatch returns to the outer
/// game loop. It runs only the scheduler service step; the next production
/// input owner performs `UpdateInputState`.
#[no_mangle]
pub extern "C" fn rust_automation_service_boundary() -> i32 {
    rust_automation_service_do_input()
}
// ===========================================================================
//  Bounds-checked production setter (REQ-INJECT-003)
// ===========================================================================

/// Set `ImmediateInputState.menu[index]` through its native owner.
///
/// The C module owns the input state, so it is the single authority for
/// bounds and normalization; this forwards its verdict unchanged.
/// Returns 0 on success, -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[cfg(not(test))]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    unsafe { c_automation_set_immediate_menu_key(index, value) }
}

/// C-callable bounds-checked setter for `ImmediateInputState.menu[index]`
/// (stub for non-linked builds — lib tests).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[cfg(test)]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    use crate::automation::input::{setter_set_menu_key, SetterResult};

    let Ok(index) = u8::try_from(index) else {
        return -1;
    };
    match setter_set_menu_key(index, u8::from(value != 0)) {
        SetterResult::InvalidIndex { .. } => -1,
        SetterResult::Set { .. } | SetterResult::Cleared { .. } => 0,
    }
}

// ===========================================================================
//  Present hook: called from TFB_SwapBuffers
// ===========================================================================

/// Set one player-one gameplay control through the native owner of
/// `ImmediateInputState`, forwarding its verdict unchanged.
#[cfg(not(test))]
pub extern "C" fn rust_automation_set_immediate_player_key(index: i32, value: i32) -> i32 {
    unsafe { c_automation_set_immediate_player_key(index, value) }
}

#[cfg(test)]
pub extern "C" fn rust_automation_set_immediate_player_key(index: i32, _value: i32) -> i32 {
    if (0..7).contains(&index) {
        0
    } else {
        -1
    }
}

/// Discard the pulse the production input loop derived for a menu key.
///
/// `UpdateInputState` turns a key that stays down into a repeat pulse once
/// `MENU_REPEAT_DELAY` of wall-clock time has passed since the last pulse. A
/// scripted tap counts input updates, so on a slow host that clock fires
/// inside one tap and the game sees several presses. Returns `false` for an
/// index outside the menu key table.
#[cfg(not(test))]
pub(crate) fn discard_menu_pulse(index: i32) -> bool {
    let Some(index) = menu_key_slot(index) else {
        return false;
    };
    // SAFETY: the single game thread runs `DoInput`, between
    // `UpdateInputState` and the consumers of the pulsed state.
    unsafe {
        (*std::ptr::addr_of_mut!(PulsedInputState)).menu[index] = 0;
    }
    true
}

/// Stub for builds without the linked C archive, which have no pulsed state.
#[cfg(test)]
pub(crate) fn discard_menu_pulse(index: i32) -> bool {
    menu_key_slot(index).is_some()
}

fn menu_key_slot(index: i32) -> Option<usize> {
    usize::try_from(index)
        .ok()
        .filter(|slot| *slot < usize::from(crate::automation::input::NUM_MENU_KEYS))
}

/// Set when the native owner refuses an automation input write.
static INJECTION_REJECTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Write a menu key, recording any rejection by the native owner.
pub(crate) fn inject_menu_key(index: i32, value: i32) {
    if rust_automation_set_immediate_menu_key(index, value) != 0 {
        INJECTION_REJECTED.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Write a player control, recording any rejection by the native owner.
pub(crate) fn inject_player_key(index: i32, value: i32) {
    if rust_automation_set_immediate_player_key(index, value) != 0 {
        INJECTION_REJECTED.store(true, std::sync::atomic::Ordering::Release);
    }
}

/// True once the native owner has refused an automation input write.
///
/// Typed key domains make this unreachable today; it becomes reachable only
/// if those domains and the native key tables diverge, and the run must fail
/// rather than continue against input that never landed.
pub(crate) fn injection_rejected() -> bool {
    INJECTION_REJECTED.load(std::sync::atomic::Ordering::Acquire)
}

/// Query whether automation has reached a terminal state before rendering.
#[no_mangle]
pub extern "C" fn rust_automation_present_callback() -> i32 {
    with_runtime()
        .is_some_and(|runtime| runtime.mirror.is_terminal())
        .into()
}

/// Commit one frame after the graphics backend has actually presented it.
#[no_mangle]
pub extern "C" fn rust_automation_presented_frame() -> i32 {
    if !Coordinator::is_active() {
        return 0;
    }
    let frame = crate::graphics::ffi::take_presented_frame();
    Coordinator::process_present(frame).into()
}

// ===========================================================================
//  Production getters (REQ-INJECT-006)
// ===========================================================================

/// C-callable getter for `CurrentInputState.menu[index]`.
///
/// Returns the value (0 or 1) or -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[cfg(not(test))]
pub extern "C" fn rust_automation_get_current_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    unsafe { CurrentInputState.menu[index as usize] }
}

/// C-callable getter for `CurrentInputState.menu[index]` (stub for tests).
#[cfg(test)]
pub extern "C" fn rust_automation_get_current_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    0
}

/// C-callable getter for `PulsedInputState.menu[index]`.
///
/// Returns the value (0 or 1) or -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-006
#[cfg(not(test))]
pub extern "C" fn rust_automation_get_pulsed_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    unsafe { PulsedInputState.menu[index as usize] }
}

/// C-callable getter for `PulsedInputState.menu[index]` (stub for tests).
#[cfg(test)]
pub extern "C" fn rust_automation_get_pulsed_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    0
}

// ===========================================================================
//  Unit tests
// ===========================================================================

/// Read-only interstellar observations, copied at the automation game-thread boundary.
#[cfg(not(test))]
pub(crate) fn star_flight_observation(
    target: super::star_flight::StarTarget,
) -> super::star_flight::Observation {
    use crate::savegame::runtime::star::{CStarDesc, NUM_SOLAR_SYSTEMS};
    extern "C" {
        static CurStarDescPtr: *const CStarDesc;
        fn universeToDispx(x: std::ffi::c_long) -> i16;
        fn universeToDispy(y: std::ffi::c_long) -> i16;
        #[cfg(feature = "debug-process")]
        static instantMove: i32;
    }
    // SAFETY: the coordinator is called synchronously on the game thread.
    unsafe {
        let current = std::ptr::addr_of!(CurStarDescPtr).read() as usize;
        let base = std::ptr::addr_of!(crate::planets::tables::STARMAP_ARRAY) as usize;
        let offset = current.wrapping_sub(base);
        let entry_size = std::mem::size_of::<CStarDesc>();
        let current_star = (offset < NUM_SOLAR_SYSTEMS * entry_size
            && offset.is_multiple_of(entry_size))
        .then_some((offset / entry_size) as u16);
        let cursor = crate::planets::starmap_cursor::cursor();
        let (tx, ty) = target.coordinates();
        let cursor_delta = cursor.map_or((0, 0), |(x, y)| {
            (
                universeToDispx(tx.into()) - universeToDispx(x.into()),
                universeToDispy(ty.into()) - universeToDispy(y.into()),
            )
        });
        let autopilot = std::ptr::addr_of!(GLOB_DATA.game_state.autopilot).read();
        super::star_flight::Observation {
            activity: crate::mainloop::ffi::get_current_activity().0,
            current_star,
            solar_active: departure_snapshot().active != 0,
            cursor,
            cursor_delta,
            autopilot: (autopilot.x >= 0 && autopilot.y >= 0).then_some((autopilot.x, autopilot.y)),
            position: hyperspace_position(),
            fuel: ship_resources().map(|(fuel, _)| fuel),
            instant_move: {
                #[cfg(feature = "debug-process")]
                {
                    instantMove != 0
                }
                #[cfg(not(feature = "debug-process"))]
                {
                    false
                }
            },
        }
    }
}

#[cfg(test)]
pub(crate) fn star_flight_observation(
    _target: super::star_flight::StarTarget,
) -> super::star_flight::Observation {
    super::star_flight::Observation::default()
}

/// Player-selected combat settings: enabled flag and speed selector.
#[cfg(not(test))]
pub(crate) fn cyborg_settings() -> Option<(bool, u8)> {
    // Game thread, read only while the input callback owns the live game.
    let flags = unsafe { std::ptr::addr_of!(GLOB_DATA.game_state.glob_flags).read() };
    Some((flags & (1 << 5) != 0, flags >> 6))
}

#[cfg(test)]
pub(crate) fn cyborg_settings() -> Option<(bool, u8)> {
    None
}

#[cfg(not(test))]
pub(crate) fn escort_roster() -> Option<Vec<(u8, u16)>> {
    crate::state::ffi::named_state_value("STARBASE_AVAILABLE")?;
    use crate::comm::dispatch::CShipFragment;
    use crate::savegame::runtime::queues::{walk, QueueKind};
    unsafe {
        let queue = &*std::ptr::addr_of!(GLOB_DATA.game_state.built_ship_q);
        Some(
            walk::<CShipFragment>(queue, QueueKind::BuiltShips)
                .expect("live escort queue")
                .into_iter()
                .map(|ship| ((*ship).race_id, (*ship).crew_level))
                .collect(),
        )
    }
}

#[cfg(test)]
pub(crate) fn escort_roster() -> Option<Vec<(u8, u16)>> {
    None
}

pub(crate) fn fleet_fight_observation() -> super::fleet_fight::Observation {
    let (cyborg, speed) = cyborg_settings().unwrap_or((false, 0));
    let (player, enemy) = battle_fleets().unwrap_or((0, 0));
    #[cfg(not(test))]
    let picker = {
        let (generation, active) = super::ui_observation::ship_picker_observation();
        active.then(|| super::fleet_fight::Picker {
            generation,
            cursor: super::ui_observation::ship_picker_cursor(),
            escorts: crate::pickship::ffi::live_escort_indices(),
        })
    };
    #[cfg(test)]
    let picker = None;
    super::fleet_fight::Observation {
        activity: crate::mainloop::ffi::get_current_activity().0,
        cyborg,
        speed,
        counters: [player, enemy],
        flagship_crew: sis_crew_and_landers().map(|(crew, _)| crew),
        picker,
        encounter_menu: super::ui_observation::encounter_menu_active(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setter_valid_index_returns_zero() {
        assert_eq!(rust_automation_set_immediate_menu_key(6, 1), 0);
    }

    #[test]
    fn setter_invalid_index_returns_negative() {
        assert_eq!(rust_automation_set_immediate_menu_key(24, 1), -1);
        assert_eq!(rust_automation_set_immediate_menu_key(-1, 1), -1);
    }

    #[test]
    fn setter_clear_returns_zero() {
        assert_eq!(rust_automation_set_immediate_menu_key(5, 0), 0);
    }

    #[test]
    fn getter_invalid_index_returns_negative() {
        assert_eq!(rust_automation_get_current_menu_key(24), -1);
        assert_eq!(rust_automation_get_current_menu_key(-1), -1);
        assert_eq!(rust_automation_get_pulsed_menu_key(24), -1);
        assert_eq!(rust_automation_get_pulsed_menu_key(-1), -1);
    }

    #[test]
    fn getter_valid_index_returns_zero() {
        assert_eq!(rust_automation_get_current_menu_key(6), 0);
        assert_eq!(rust_automation_get_pulsed_menu_key(6), 0);
    }

    #[test]
    fn service_inactive_returns_zero() {
        // Without init_automation_runtime, this should return 0 (inactive).
        assert_eq!(rust_automation_service_do_input(), 0);
    }

    #[test]
    fn observation_inactive_returns_zero() {
        assert_eq!(rust_automation_after_input_update(), 0);
    }

    /// Ground truth captured by compiling the real C macros from `units.h`
    /// and `planets.h` against each screen size. The interplanetary view
    /// centre is not a constant, so navigation must not assume one.
    #[test]
    fn view_geometry_matches_the_c_macros_at_every_supported_resolution() {
        let low = ViewGeometry::from_screen(320, 240);
        assert_eq!(low.half_width, 121);
        assert_eq!(low.half_height, 113);
        assert_eq!(low.display_to_loc, 56);

        let high = ViewGeometry::from_screen(640, 480);
        assert_eq!(high.half_width, 281);
        assert_eq!(high.half_height, 233);
        assert_eq!(high.display_to_loc, 136);
    }

    /// Earth's moon 0 (the Hierarchy Starbase) sits at display (86, 113) in
    /// the 320x240 inner view. That y-coordinate is exactly the view centre,
    /// which the previously hardcoded 91 would have mismatched by 22 pixels.
    #[test]
    fn view_geometry_centre_matches_the_observed_inner_system_layout() {
        let geometry = ViewGeometry::from_screen(320, 240);
        assert_eq!(geometry.half_height, 113);
        assert_eq!(geometry.half_width - 35, 86);
    }

    #[test]
    fn planet_target_inside_its_own_inner_system_is_the_view_centre() {
        let geometry = ViewGeometry::from_screen(320, 240);
        // Mars's stale outer-view origin must not be steered at once the ship
        // is inside Mars's inner system, where the planet is at the centre.
        assert_eq!(
            planet_display_target(true, (200, 40), 56, &geometry),
            (121, 113)
        );
    }

    #[test]
    fn planet_target_in_the_outer_system_is_the_rounded_outer_origin() {
        let geometry = ViewGeometry::from_screen(320, 240);
        // (200 - 121) * 112 / 56 = 158; 121 + 158 * 56 / 112 = 200.
        assert_eq!(
            planet_display_target(false, (200, 40), 112, &geometry),
            (200, 40)
        );
        assert_eq!(
            planet_display_target(false, (200, 40), 0, &geometry),
            (0, 0)
        );
    }
}
