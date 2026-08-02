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
use crate::automation::input::setter_set_menu_key;
use crate::automation::runtime::RuntimeModel;

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

/// The C `CONTROLLER_INPUT_STATE` struct, used only when linking against
/// the real C archive.
#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct ControllerInputState {
    key: [[i32; 7]; 6],
    menu: [i32; 24],
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct CPlanetDesc {
    _rand_seed: u32,
    data_index: u8,
    num_planets: u8,
    radius: i16,
    _location: crate::comm::locdata::CPoint,
    _temp_color: crate::comm::locdata::CColor,
    _next_index: u16,
    image: crate::comm::locdata::CStamp,
    previous: *mut CPlanetDesc,
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct CSolarSystemState {
    _input_func: *mut std::ffi::c_void,
    in_ip_flight: i32,
    wait_intersect: u16,
    _wait_intersect_pad: [u8; 2],
    sun: [CPlanetDesc; 1],
    planets: [CPlanetDesc; 16],
    moons: [CPlanetDesc; 4],
    _base: *mut CPlanetDesc,
    orbital: *mut CPlanetDesc,
    _between_orbital_and_in_orbit: [u8; 256],
    in_orbit: i32,
    _tail: [u8; 4],
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    static mut ImmediateInputState: ControllerInputState;
    static CurrentInputState: ControllerInputState;
    static PulsedInputState: ControllerInputState;
    static PlayerControls: [i32; 2];
    #[link_name = "pSolarSysState"]
    static mut P_SOLAR_SYSTEM_STATE: *mut CSolarSystemState;
    #[link_name = "GlobData"]
    static mut GLOB_DATA: crate::comm::locdata::CGlobData;
    #[allow(non_snake_case)]
    fn GetFrameIndex(frame: *mut std::ffi::c_void) -> u16;
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
    #[cfg(feature = "linked_c_archive")]
    fn current() -> Self {
        // SAFETY: `ScreenWidth`/`ScreenHeight` are set once during graphics
        // init and are read-only thereafter; this hook runs on the game thread.
        let (width, height) = unsafe {
            (
                std::ptr::addr_of!(SCREEN_WIDTH).read(),
                std::ptr::addr_of!(SCREEN_HEIGHT).read(),
            )
        };
        Self::from_screen(width, height)
    }
}

/// The game's baseline screen size, used only when the C globals are absent.
#[cfg(not(feature = "linked_c_archive"))]
const DEFAULT_SCREEN_WIDTH: i32 = 320;
/// The game's baseline screen size, used only when the C globals are absent.
#[cfg(not(feature = "linked_c_archive"))]
const DEFAULT_SCREEN_HEIGHT: i32 = 240;

#[cfg(feature = "linked_c_archive")]
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
        let solar_system_ptr = std::ptr::addr_of!(P_SOLAR_SYSTEM_STATE).read();
        let Some(solar_system) = solar_system_ptr.as_ref() else {
            return snapshot;
        };
        if target_planet < 0 || target_planet >= i32::from(solar_system.sun[0].num_planets) {
            return snapshot;
        }

        snapshot.active = 1;
        snapshot.in_ip_flight = i32::from(solar_system.in_ip_flight != 0);
        snapshot.in_orbit = i32::from(solar_system.in_orbit != 0);
        snapshot.wait_intersect = solar_system.wait_intersect;
        let game_state = std::ptr::addr_of!(GLOB_DATA.game_state);
        let ship_stamp = std::ptr::addr_of!((*game_state).ship_stamp).read();
        snapshot.ship_x = i32::from(ship_stamp.origin.x);
        snapshot.ship_y = i32::from(ship_stamp.origin.y);
        if !ship_stamp.frame.is_null() {
            snapshot.ship_facing = i32::from(GetFrameIndex(ship_stamp.frame));
        }
        let velocity = std::ptr::addr_of!((*game_state)._velocity)
            .cast::<crate::battle::velocity::VelocityDesc>()
            .read_unaligned();
        (snapshot.velocity_x, snapshot.velocity_y) = velocity.get_current_components();

        if !solar_system.orbital.is_null() {
            snapshot.orbital_data_index = i32::from((*solar_system.orbital).data_index);
            let planet_start = solar_system.planets.as_ptr() as usize;
            let moon_start = solar_system.moons.as_ptr() as usize;
            let desc_size = std::mem::size_of::<CPlanetDesc>();
            let orbital_address = solar_system.orbital as usize;
            if orbital_address >= moon_start
                && orbital_address < moon_start + solar_system.moons.len() * desc_size
            {
                snapshot.orbital_moon = ((orbital_address - moon_start) / desc_size) as i32;
            }
            let sun = solar_system.sun.as_ptr();
            let orbital_planet = if std::ptr::eq((*solar_system.orbital).previous.cast_const(), sun)
            {
                solar_system.orbital
            } else {
                (*solar_system.orbital).previous
            } as usize;
            if orbital_planet >= planet_start {
                snapshot.inner_planet = ((orbital_planet - planet_start) / desc_size) as i32;
            }
        }

        if let Some(moon) = target_moon {
            if snapshot.inner_planet == target_planet
                && moon >= 0
                && moon < i32::from(solar_system.planets[target_planet as usize].num_planets)
            {
                let target = &solar_system.moons[moon as usize];
                snapshot.target_data_index = i32::from(target.data_index);
                let target_origin = target.image.origin;
                snapshot.target_x = i32::from(target_origin.x);
                snapshot.target_y = i32::from(target_origin.y);
                return snapshot;
            }
        }

        let target_origin = solar_system.planets[target_planet as usize].image.origin;
        let radius = i32::from(solar_system.sun[0].radius);
        if radius != 0 {
            // Match displayToLocation followed by locationToDisplay exactly.
            // C integer division truncates toward zero, as Rust's does.
            let location_x = (i32::from(target_origin.x) - geometry.half_width) * radius
                / geometry.display_to_loc;
            let location_y = (i32::from(target_origin.y) - geometry.half_height) * radius
                / geometry.display_to_loc;
            snapshot.target_x = geometry.half_width + location_x * geometry.display_to_loc / radius;
            snapshot.target_y =
                geometry.half_height + location_y * geometry.display_to_loc / radius;
        }
    }
    snapshot
}

#[cfg(not(feature = "linked_c_archive"))]
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
#[no_mangle]
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
        #[cfg(feature = "linked_c_archive")]
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
#[no_mangle]
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

    // Step 3: Observation only — no additional scheduler processing.
    0
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

/// C-callable bounds-checked setter for `ImmediateInputState.menu[index]`.
///
/// Writes directly to the C global volatile `ImmediateInputState.menu[index]`.
/// Returns 0 on success, -1 on invalid index.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[no_mangle]
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    let _result = setter_set_menu_key(index as u8, value as u8);
    unsafe {
        ImmediateInputState.menu[index as usize] = if value != 0 { 1 } else { 0 };
    }
    0
}

/// C-callable bounds-checked setter for `ImmediateInputState.menu[index]`
/// (stub for non-linked builds — lib tests).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P06
/// @requirement REQ-INJECT-003
#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
pub extern "C" fn rust_automation_set_immediate_menu_key(index: i32, value: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    let _result = setter_set_menu_key(index as u8, value as u8);
    let _ = value;
    0
}

// ===========================================================================
//  Present hook: called from TFB_SwapBuffers
// ===========================================================================

/// Set one player-one gameplay control in `ImmediateInputState`.
#[no_mangle]
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_set_immediate_player_key(index: i32, value: i32) -> i32 {
    if !(0..7).contains(&index) {
        return -1;
    }
    unsafe {
        let template = PlayerControls[0] as usize;
        if template >= 6 {
            return -1;
        }
        ImmediateInputState.key[template][index as usize] = i32::from(value != 0);
    }
    0
}

#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
pub extern "C" fn rust_automation_set_immediate_player_key(index: i32, _value: i32) -> i32 {
    if (0..7).contains(&index) {
        0
    } else {
        -1
    }
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
    let generation = with_runtime()
        .map(|runtime| runtime.mirror.capture_generation())
        .unwrap_or(0);
    Coordinator::process_present(generation).into()
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
#[no_mangle]
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_get_current_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    unsafe { CurrentInputState.menu[index as usize] }
}

/// C-callable getter for `CurrentInputState.menu[index]` (stub for tests).
#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
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
#[no_mangle]
#[cfg(feature = "linked_c_archive")]
pub extern "C" fn rust_automation_get_pulsed_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    unsafe { PulsedInputState.menu[index as usize] }
}

/// C-callable getter for `PulsedInputState.menu[index]` (stub for tests).
#[no_mangle]
#[cfg(not(feature = "linked_c_archive"))]
pub extern "C" fn rust_automation_get_pulsed_menu_key(index: i32) -> i32 {
    if index < 0 || index >= i32::from(crate::automation::input::NUM_MENU_KEYS) {
        return -1;
    }
    0
}

// ===========================================================================
//  Unit tests
// ===========================================================================

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
}
