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

use super::{campaign_state::CampaignView, ui_observation::UiObservation};
use crate::automation::coordinator::AutomationObserver;
#[cfg(not(test))]
use crate::c_bindings::global_state::SOLARSYS_STATE;
use crate::engine::input::InputControls;

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
    pub(crate) moon_obstacles: [(i32, i32); 4],
    pub(crate) moon_obstacle_count: usize,
}

impl NavigationSnapshot {
    pub(crate) fn moon_observation(
        &self,
        planet: u8,
        moon: u8,
    ) -> super::navigation::NavigationObservation {
        super::navigation::NavigationObservation {
            active: self.active != 0,
            inner_planet: u8::try_from(self.inner_planet).ok(),
            in_orbit: self.in_orbit != 0,
            ship_x: self.ship_x,
            ship_y: self.ship_y,
            ship_facing: u8::try_from(self.ship_facing).unwrap_or(0),
            target_x: self.target_x,
            target_y: self.target_y,
            velocity_x: self.velocity_x,
            velocity_y: self.velocity_y,
            view_center_x: self.view_center_x,
            view_center_y: self.view_center_y,
            // The collision guard still holds this moon, so the ship is sitting in
            // it without having entered orbit.
            target_guard_active: self.wait_intersect
                == crate::automation::navigation::moon_wait_intersect(planet, moon),
        }
    }
}

#[cfg(not(test))]
extern "C" {

    /// Live orbit predicate: `playerInSolarSystem() && pSolarSysState->InOrbit`.
    #[allow(non_snake_case)]
    fn playerInPlanetOrbit() -> bool;
    #[link_name = "ScreenWidth"]
    static mut SCREEN_WIDTH: i32;
    #[link_name = "ScreenHeight"]
    static mut SCREEN_HEIGHT: i32;
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
unsafe fn read_flight_state(
    world: &CampaignView,
    solar_system: &SOLARSYS_STATE,
    snapshot: &mut NavigationSnapshot,
) {
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
    snapshot.ship_x = i32::from(world.ship_position.0);
    snapshot.ship_y = i32::from(world.ship_position.1);
    snapshot.ship_facing = world.ship_facing;
    (snapshot.velocity_x, snapshot.velocity_y) = world.velocity;

    if !solar_system.pOrbitalDesc.is_null() {
        snapshot.orbital_data_index = i32::from((*solar_system.pOrbitalDesc).data_index);
        let planet_start = solar_system.PlanetDesc.as_ptr() as usize;
        let moon_start = solar_system.MoonDesc.as_ptr() as usize;
        let desc_size = std::mem::size_of::<crate::engine::world::planet::PlanetDesc>();
        let orbital_address = solar_system.pOrbitalDesc as usize;
        if orbital_address >= moon_start
            && orbital_address < moon_start + solar_system.MoonDesc.len() * desc_size
        {
            snapshot.orbital_moon = ((orbital_address - moon_start) / desc_size) as i32;
        }
        let sun = solar_system.SunDesc.as_ptr();
        let orbital_planet =
            if std::ptr::eq((*solar_system.pOrbitalDesc).prev_desc.cast_const(), sun) {
                solar_system.pOrbitalDesc
            } else {
                (*solar_system.pOrbitalDesc).prev_desc
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
pub(crate) fn hyperspace_position(world: &CampaignView) -> Option<(i32, i32)> {
    Some((world.flagship.log_x, world.flagship.log_y))
}

/// Without the linked game there is no position to read.
#[cfg(test)]
pub(crate) fn hyperspace_position(_world: &CampaignView) -> Option<(i32, i32)> {
    None
}

/// The flight velocity published by `sis_hyper_postprocess` has stopped.
/// Unavailable outside hyperspace flight or while a conversation is active.
#[cfg(not(test))]
pub(crate) fn hyperspace_stopped(world: &CampaignView, ui: &UiObservation) -> Option<bool> {
    if crate::mainloop::types::ActivityValue(world.activity).kind()
        != crate::mainloop::types::ActivityKind::InHyperspace
        || ui.conversation_snapshot().active
    {
        return None;
    }
    // SAFETY: the input hook runs synchronously on the game thread.
    let velocity = world.velocity;
    Some(velocity == (0, 0))
}

#[cfg(test)]
pub(crate) fn hyperspace_stopped(_world: &CampaignView, _ui: &UiObservation) -> Option<bool> {
    None
}

/// The flagship's authoritative `FuelOnBoard` and `ResUnits`, in that order.
///
/// `DeltaSISGauges` changes exactly these `SIS_STATE` fields, so a reward or
/// a purchase is visible here only after the game applied it.
#[cfg(not(test))]
pub(crate) fn ship_resources(world: &CampaignView) -> Option<(u32, u32)> {
    Some((world.flagship.fuel_on_board, world.flagship.res_units))
}

/// Without the linked game there is no flagship state to read.
#[cfg(test)]
pub(crate) fn ship_resources(_world: &CampaignView) -> Option<(u32, u32)> {
    None
}

/// The flagship's `CrewEnlisted` and `NumLanders`, in that order.
#[cfg(not(test))]
pub(crate) fn sis_crew_and_landers(world: &CampaignView) -> Option<(u16, u8)> {
    Some((world.flagship.crew_enlisted, world.flagship.num_landers))
}

/// Without the linked game there is no flagship state to read.
#[cfg(test)]
pub(crate) fn sis_crew_and_landers(_world: &CampaignView) -> Option<(u16, u8)> {
    None
}

/// The flagship's authoritative `ElementAmounts`, indexed by element category.
///
/// The lander hold is added to these when it returns to the ship, so they
/// change only after the trip. Nothing is written.
#[cfg(not(test))]
pub(crate) fn ship_element_amounts(world: &CampaignView) -> Option<[u16; 8]> {
    Some(world.flagship.element_amounts)
}

/// Without the linked game there is no flagship state to read.
#[cfg(test)]
pub(crate) fn ship_element_amounts(_world: &CampaignView) -> Option<[u16; 8]> {
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
pub(crate) fn departure_snapshot(world: &CampaignView) -> NavigationSnapshot {
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
            read_flight_state(world, solar_system, &mut snapshot);
        }
    }
    snapshot
}

#[cfg(test)]
pub(crate) fn departure_snapshot(world: &CampaignView) -> NavigationSnapshot {
    navigation_snapshot(world, 0, None)
}

/// Copy a live group's target geometry without moving it or invoking contact.
#[cfg(not(test))]
pub(crate) fn contact_snapshot(
    world: &CampaignView,
    race: u8,
) -> Option<(NavigationSnapshot, bool)> {
    use crate::c_bindings::global_state::POINT;
    extern "C" {
        fn locationToDisplay(location: POINT, radius: i16) -> POINT;
    }
    unsafe {
        let solar = crate::c_bindings::solar_system::current().as_ref()?;
        let (_, system, location) = world
            .groups
            .iter()
            .copied()
            .find(|(id, _, _)| *id == race)?;
        let mut snapshot = if system == 0 {
            departure_snapshot(world)
        } else {
            navigation_snapshot(world, i32::from(system) - 1, None)
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
                    x: location.0,
                    y: location.1,
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
pub(crate) fn contact_snapshot(
    _world: &CampaignView,
    _race: u8,
) -> Option<(NavigationSnapshot, bool)> {
    None
}

#[cfg(not(test))]
pub(crate) fn contact_race(world: &CampaignView) -> Option<u8> {
    world.contact_race
}

#[cfg(test)]
pub(crate) fn contact_race(_world: &CampaignView) -> Option<u8> {
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
    world: &CampaignView,
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

    // SAFETY: this synchronous snapshot borrows the current solar system until
    // return, without callbacks or any mutation of its planet arrays.
    let solar = unsafe { crate::c_bindings::solar_system::current().as_ref() };
    let Some(solar_system) = solar else {
        return snapshot;
    };
    if target_planet < 0 || target_planet >= i32::from(solar_system.SunDesc[0].num_planets) {
        return snapshot;
    }
    // SAFETY: the live synchronous solar borrow also keeps its global flight state live.
    unsafe { read_flight_state(world, solar_system, &mut snapshot) };

    if let Some(moon) = target_moon {
        if snapshot.inner_planet == target_planet
            && moon >= 0
            && moon < i32::from(solar_system.PlanetDesc[target_planet as usize].num_planets)
        {
            for (index, body) in solar_system.MoonDesc
                [..usize::from(solar_system.PlanetDesc[target_planet as usize].num_planets)]
                .iter()
                .enumerate()
            {
                if index != moon as usize {
                    snapshot.moon_obstacles[snapshot.moon_obstacle_count] = (
                        i32::from(body.image.origin.x),
                        i32::from(body.image.origin.y),
                    );
                    snapshot.moon_obstacle_count += 1;
                }
            }
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
    snapshot
}

#[cfg(test)]
pub(crate) fn navigation_snapshot(
    _world: &CampaignView,
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

pub(crate) fn graphics_options(
    renderer: Option<std::ffi::CString>,
    observer: Option<std::sync::Arc<AutomationObserver>>,
) -> crate::graphics::sdl::lifecycle_abi::GraphicsOptions {
    let presented_owner = observer.clone();
    crate::graphics::sdl::lifecycle_abi::GraphicsOptions {
        renderer,
        #[cfg(feature = "debug-process")]
        native_position: crate::automation::native_window::active_native_window_config()
            .map(|config| (config.client_bounds.x, config.client_bounds.y)),
        #[cfg(not(feature = "debug-process"))]
        native_position: None,
        present_hook: Some(std::rc::Rc::new(move |presented| {
            let Some(observer) = &presented_owner else {
                return 0;
            };
            if !presented {
                return i32::from(observer.runtime.mirror.is_terminal());
            }
            let mut owner = super::observer::Handle(observer.clone());
            let io = crate::engine::observe::Io {
                observer: &mut owner,
            };
            let mut frame = crate::engine::observe::Presented {
                frame: crate::graphics::ffi::take_presented_frame(),
                stop: false,
            };
            io.observer.presented(&mut frame);
            i32::from(frame.stop)
        })),
        readback_hook: Some(std::rc::Rc::new(move || {
            observer
                .as_ref()
                .map(|owner| owner.runtime.mirror.capture_generation())
        })),
    }
}
// ===========================================================================
//  Service hook: called before UpdateInputState in DoInput
// ===========================================================================

pub(crate) fn service_input(
    observer: &AutomationObserver,
    capture: &mut dyn FnMut() -> CampaignView,
    button_wait: bool,
    setup: &mut super::input_admission::Target<'_>,
    sink: &mut dyn crate::engine::observe::Observer<Option<CampaignView>>,
) -> i32 {
    // Step 1: ABI entry (saturating).
    let rt = &observer.runtime;
    rt.record_abi_entry();

    // Step 2: Acquire-load activation.
    if !rt.mirror.is_active() {
        return 0; // Inactive fast path: no stop.
    }

    // Step 2b: Check terminal — if already terminal, return stop to
    // break the DoInput loop. This is the key mechanism that makes
    // DoInput break out when the scheduler finishes.

    if rt.mirror.is_terminal() {
        // Re-assert CHECK_ABORT every frame. Game logic (handle_select)
        // can overwrite CurrentActivity, clearing our CHECK_ABORT. By
        // re-asserting on every DoInput call, we ensure the activity
        // state machine's should_continue() check sees CHECK_ABORT and
        // exits the inner loop.
        #[cfg(not(test))]
        {
            observer.activity.raise_flags(0x4000);
        }
        return 1; // Terminal: stop the DoInput loop.
    }

    // Step 4: Feed the input callback to the coordinator (scheduler+watchdog).
    let stop = observer.process_input_admission(capture, button_wait, setup, sink);
    if stop {
        return 1;
    }

    0
}

// ===========================================================================
//  Observation hook: called after UpdateInputState in DoInput
// ===========================================================================

pub(crate) fn after_input_update(observer: &AutomationObserver, activity: u16) -> i32 {
    // Step 1: ABI entry (saturating).
    let rt = &observer.runtime;
    rt.record_abi_entry();

    // Step 2: Acquire-load activation.
    if !rt.mirror.is_active() {
        return 0; // Inactive fast path: no stop.
    }

    // Step 2b: Check terminal — if terminal, return stop to break DoInput.
    if rt.mirror.is_terminal() {
        return 1; // Terminal: stop the DoInput loop.
    }

    // Step 3: Commit the scripted player-input observation at the exact
    // production boundary that follows UpdateInputState.
    observer.parity_input(activity);
    observer.process_player_input_observation().into()
}

/// Set an externally supplied menu key, preserving the bounds/normalization contract.
pub fn rust_automation_set_immediate_menu_key(
    input: &InputControls,
    index: i32,
    value: i32,
) -> i32 {
    let Some(index) = menu_key_slot(index) else {
        return -1;
    };
    input.set_menu_key(index, value != 0);
    0
}

pub fn rust_automation_set_immediate_player_key(
    input: &InputControls,
    index: i32,
    value: i32,
) -> i32 {
    let Ok(index) = usize::try_from(index) else {
        return -1;
    };
    if index >= crate::engine::input::frames::FLIGHT_KEYS {
        return -1;
    }
    input.set_player_key(0, index, value != 0);
    0
}

pub(crate) fn discard_menu_pulse(input: &InputControls, index: i32) -> bool {
    let Some(index) = menu_key_slot(index) else {
        return false;
    };
    input.clear_menu_pulse(index);
    true
}

fn menu_key_slot(index: i32) -> Option<usize> {
    usize::try_from(index)
        .ok()
        .filter(|slot| *slot < crate::engine::input::frames::MENU_KEYS)
}

pub(crate) fn inject_menu_key(input: &InputControls, index: i32, value: i32) {
    if rust_automation_set_immediate_menu_key(input, index, value) != 0 {
        input.reject_injection();
    }
}

pub(crate) fn inject_player_key(input: &InputControls, index: i32, value: i32) {
    if rust_automation_set_immediate_player_key(input, index, value) != 0 {
        input.reject_injection();
    }
}

pub(crate) fn current_inputs(input: &InputControls) -> crate::engine::input::frames::InputState {
    input.current()
}

pub fn rust_automation_get_current_menu_key(input: &InputControls, index: i32) -> i32 {
    menu_key_slot(index).map_or(-1, |index| input.current().menu[index])
}

pub fn rust_automation_get_pulsed_menu_key(input: &InputControls, index: i32) -> i32 {
    menu_key_slot(index).map_or(-1, |index| input.pulsed().menu[index])
}

// ===========================================================================
//  Unit tests
// ===========================================================================

/// Read-only interstellar observations, copied at the automation game-thread boundary.
#[cfg(not(test))]
pub(crate) fn star_flight_observation(
    world: &CampaignView,
    target: super::star_flight::StarTarget,
    ui: &UiObservation,
) -> super::star_flight::Observation {
    extern "C" {

        fn universeToDispx(x: std::ffi::c_long) -> i16;
        fn universeToDispy(y: std::ffi::c_long) -> i16;
    }
    // SAFETY: the coordinator is called synchronously on the game thread.
    unsafe {
        let current_star = world.current_star.map(|id| id.0 as u16);
        let cursor = crate::planets::starmap_cursor::cursor();
        let (tx, ty) = target.coordinates();
        let cursor_delta = cursor.map_or((0, 0), |(x, y)| {
            (
                universeToDispx(tx.into()) - universeToDispx(x.into()),
                universeToDispy(ty.into()) - universeToDispy(y.into()),
            )
        });
        let autopilot = world.autopilot;
        super::star_flight::Observation {
            activity: world.activity,
            current_star,
            space_side: world
                .story_value(crate::engine::StoryKey::ARILOU_SPACE_SIDE)
                .expect("initialized state"),
            conversation_active: ui.conversation_snapshot().active,
            portal_open: world.story_value(crate::engine::StoryKey::ARILOU_SPACE) == Some(1),
            solar_active: departure_snapshot(world).active != 0,
            cursor,
            cursor_delta,
            autopilot: (autopilot.0 >= 0 && autopilot.1 >= 0).then_some(autopilot),
            position: hyperspace_position(world),
            fuel: ship_resources(world).map(|(fuel, _)| fuel),
            instant_move: {
                #[cfg(feature = "debug-process")]
                {
                    crate::front_end::uqmdebug::INSTANT_MOVE != 0
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
    _world: &CampaignView,
    _target: super::star_flight::StarTarget,
    _ui: &UiObservation,
) -> super::star_flight::Observation {
    super::star_flight::Observation::default()
}

/// Player-selected combat settings: enabled flag and speed selector.
#[cfg(not(test))]
pub(crate) fn cyborg_settings(world: &CampaignView) -> Option<(bool, u8)> {
    let flags = world.combat_flags;
    Some((flags & (1 << 5) != 0, flags >> 6))
}

#[cfg(test)]
pub(crate) fn cyborg_settings(_world: &CampaignView) -> Option<(bool, u8)> {
    None
}

#[cfg(not(test))]
pub(crate) fn escort_roster(world: &CampaignView) -> Option<Vec<(u8, u16)>> {
    world.story_value(crate::engine::StoryKey::STARBASE_AVAILABLE)?;
    Some(world.escorts.clone())
}

#[cfg(test)]
pub(crate) fn escort_roster(_world: &CampaignView) -> Option<Vec<(u8, u16)>> {
    None
}

pub(crate) fn fleet_fight_observation(
    world: &CampaignView,
    ui: &UiObservation,
) -> super::fleet_fight::Observation {
    let (cyborg, speed) = cyborg_settings(world).unwrap_or((false, 0));
    let (player, enemy) = battle_fleets().unwrap_or((0, 0));
    #[cfg(not(test))]
    let picker = {
        let (generation, active) = ui.ship_picker_observation();
        active.then(|| super::fleet_fight::Picker {
            generation,
            cursor: ui.ship_picker_cursor(),
            escorts: world
                .player_ships
                .iter()
                .take(world.player_ships.len().saturating_sub(1))
                .filter(|ship| ship.species != super::super::pickship::NO_ID)
                .map(|ship| ship.index)
                .collect(),
        })
    };
    #[cfg(test)]
    let picker = None;
    super::fleet_fight::Observation {
        activity: world.activity,
        cyborg,
        speed,
        counters: [player, enemy],
        flagship_crew: sis_crew_and_landers(world).map(|(crew, _)| crew),
        picker,
        encounter_menu: ui.encounter_menu_active(),
        escape_generation: ui.ship_escape_generation(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod graphics_hook_tests {
    #[test]
    fn options_install_both_live_observers() {
        let options = super::graphics_options(None, None);
        assert!(options.present_hook.is_some());
        assert!(options.readback_hook.is_some());
        assert!(options.renderer.is_none());
    }
}
