//! Process Loop — view/zoom types and per-element state steps
//!
//! The camera and zoom types and arithmetic, the per-element state commit
//! (`setup_element`, `post_process`) and the collision time type of the battle
//! process loop. It owns no element storage: the display element queue is the
//! C `disp_q`, whose only implementation is `collections::queue::Queue`. A
//! second pooled list for the battle elements was removed by #37.
//!
//! # C Reference
//! `sc2/src/uqm/process.c` — functions ported here are annotated with C line numbers.

/// View state for camera/zoom system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ViewState {
    /// View is stable, no scrolling or zoom change
    Stable = 0,
    /// View is scrolling but zoom unchanged
    Scroll = 1,
    /// Zoom level is changing
    Change = 2,
}

/// Zoom mode (step vs continuous)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZoomMode {
    /// Fixed-step zoom style (original)
    Step = 0,
    /// Continuous zoom style (smooth)
    Continuous = 1,
}

// Zoom constants from units.h and process.c
pub const ZOOM_SHIFT: u32 = 8;
pub const MAX_REDUCTION: u32 = 3;
pub const MAX_VIS_REDUCTION: u32 = 2;
pub const REDUCTION_SHIFT: u32 = 1;
pub const NUM_VIEWS: usize = (MAX_VIS_REDUCTION + 1) as usize;
pub const MAX_ZOOM_OUT: i32 = 1 << (ZOOM_SHIFT + MAX_REDUCTION - 1);

// Hysteresis thresholds (from process.c HYSTERESIS_X/Y macros)
// These prevent oscillation at zoom boundaries
pub const HYSTERESIS_X: i32 = 24 << 2; // DISPLAY_TO_WORLD(24)
pub const HYSTERESIS_Y: i32 = 20 << 2; // DISPLAY_TO_WORLD(20)

// Zoom jump constant for continuous mode
pub const ZOOM_JUMP: i32 = (1 << ZOOM_SHIFT) >> 3;

// Camera clamping constant for single-ship mode (from process.c)
pub const ORG_JUMP_X: i32 = 4; // DISPLAY_ALIGN(LOG_SPACE_WIDTH / 75)
pub const ORG_JUMP_Y: i32 = 4; // DISPLAY_ALIGN(LOG_SPACE_HEIGHT / 75)

use super::element::{Element, ElementFlags};

// ---------------------------------------------------------------------------
// P03: Element Setup (process.c:116-126)
// ---------------------------------------------------------------------------

/// Initialize element state after allocation.
///
/// C reference: `SetUpElement()` (process.c:116-126)
///
/// Copies current state to next, and if collidable, initializes
/// intersection start/end points and frame.
pub fn setup_element(element: &mut Element) {
    element.next = element.current;
    if element.is_collidable() {
        init_intersect_start_point(element);
        init_intersect_end_point(element);
        init_intersect_frame(element);
    }
}

// ---------------------------------------------------------------------------
// P03: PostProcess (process.c:188-204)
// ---------------------------------------------------------------------------

/// Per-element postprocess pass.
///
/// C reference: `PostProcess()` (process.c:188-204)
///
/// 1. Invoke postprocess callback
/// 2. Commit state (next → current)
/// 3. Reinitialize intersection data if collidable
/// 4. Flag transitions (set POST_PROCESS, clear PRE_PROCESS|CHANGING|APPEARING)
///
/// # Safety
/// Same safety requirements as pre_process.
pub unsafe fn post_process(element: &mut Element) {
    // Step 1: Postprocess callback
    if let Some(postprocess_func) = element.postprocess_func {
        postprocess_func(element as *mut Element);
    }

    // Step 2: Commit state
    element.current = element.next;

    // Step 3: Reinitialize intersection data
    if element.is_collidable() {
        init_intersect_start_point(element);
        init_intersect_end_point(element);
    }

    // Step 4: Flag transitions
    element.state_flags = (element.state_flags
        & !(ElementFlags::PRE_PROCESS | ElementFlags::CHANGING | ElementFlags::APPEARING))
        | ElementFlags::POST_PROCESS;
}

// ---------------------------------------------------------------------------
// Intersection Initialization Helpers
// ---------------------------------------------------------------------------

/// Initialize intersection start point from current state.
///
/// In C, this reads from the element's current image/location to set
/// the intersection control's start point.
fn init_intersect_start_point(element: &mut Element) {
    element.intersect_control.intersect_stamp.origin = element.current.location;
}

/// Initialize intersection end point from next state.
///
/// In C, this sets the end point from next.location for collision
/// trajectory calculation.
fn init_intersect_end_point(element: &mut Element) {
    element.intersect_control.end_point = element.next.location;
}

/// Initialize intersection frame from current image.
///
/// In C, this sets the intersection stamp's frame from the element's
/// current frame handle.
fn init_intersect_frame(element: &mut Element) {
    element.intersect_control.intersect_stamp.frame = element.current.frame;
}

// ---------------------------------------------------------------------------
// P04: ProcessCollisions (process.c:362-628)
// ---------------------------------------------------------------------------

/// Time value type matching C's TIME_VALUE (unsigned 16-bit).
pub type TimeValue = u16;

/// Maximum time value for collision checks.
pub const MAX_TIME_VALUE: TimeValue = u16::MAX;

// ---------------------------------------------------------------------------
// P05: Zoom/Camera Calculation (process.c:206-358)
// ---------------------------------------------------------------------------

/// Coordinate conversion constants from battle_types
use super::battle_types::{
    LOG_SPACE_HEIGHT, LOG_SPACE_WIDTH, ONE_SHIFT, TRANSITION_HEIGHT, TRANSITION_WIDTH,
};

/// Wrap a world coordinate delta to shortest path.
/// C reference: WRAP_DELTA_X / WRAP_DELTA_Y macros.
pub fn wrap_delta_x(dx: i32) -> i32 {
    let half = LOG_SPACE_WIDTH / 2;
    if dx > half {
        dx - LOG_SPACE_WIDTH
    } else if dx < -half {
        dx + LOG_SPACE_WIDTH
    } else {
        dx
    }
}

/// Wrap Y coordinate delta.
pub fn wrap_delta_y(dy: i32) -> i32 {
    let half = LOG_SPACE_HEIGHT / 2;
    if dy > half {
        dy - LOG_SPACE_HEIGHT
    } else if dy < -half {
        dy + LOG_SPACE_HEIGHT
    } else {
        dy
    }
}

/// Align a coordinate to display grid (truncate lower bits).
/// C reference: DISPLAY_ALIGN macro.
pub fn display_align(x: i32) -> i32 {
    x & !((1 << ONE_SHIFT) - 1)
}

/// Calculate zoom reduction for step mode (discrete 3-level zoom).
///
/// C reference: `CalcReduction()` step path (process.c:215-248)
///
/// Returns reduction level (0 = closest, MAX_VIS_REDUCTION = farthest).
pub fn calc_reduction_step(
    dx: i32,
    dy: i32,
    current_zoom_out: i32,
    is_last_battle: bool,
    is_beyond_encounter: bool,
) -> i32 {
    if is_beyond_encounter {
        return 0;
    }

    let sdx = dx;
    let sdy = dy;
    let mut dx = dx;
    let mut dy = dy;
    let mut next_reduction = MAX_VIS_REDUCTION as i32;

    while next_reduction > 0 {
        dx <<= REDUCTION_SHIFT;
        dy <<= REDUCTION_SHIFT;
        if dx > TRANSITION_WIDTH || dy > TRANSITION_HEIGHT {
            break;
        }
        next_reduction -= REDUCTION_SHIFT as i32;
    }

    // Hysteresis check for zoom-in
    if next_reduction < current_zoom_out && current_zoom_out <= MAX_VIS_REDUCTION as i32 {
        let shift = MAX_VIS_REDUCTION as i32 - next_reduction;
        if ((sdx + HYSTERESIS_X) << shift) > TRANSITION_WIDTH
            || ((sdy + HYSTERESIS_Y) << shift) > TRANSITION_HEIGHT
        {
            next_reduction += REDUCTION_SHIFT as i32;
        }
    }

    // IN_LAST_BATTLE: minimum zoom level 1
    if next_reduction == 0 && is_last_battle {
        next_reduction += REDUCTION_SHIFT as i32;
    }

    next_reduction
}

/// Calculate zoom reduction for continuous mode (smooth zoom).
///
/// C reference: `CalcReduction()` continuous path (process.c:249-274)
///
/// Returns zoom factor in ZOOM_SHIFT fixed-point.
pub fn calc_reduction_continuous(
    dx: i32,
    dy: i32,
    is_last_battle: bool,
    is_beyond_encounter: bool,
) -> i32 {
    if is_beyond_encounter {
        return 1 << ZOOM_SHIFT;
    }

    let zoom_x = (dx * MAX_ZOOM_OUT) / (LOG_SPACE_WIDTH >> 2);
    let zoom_x = zoom_x.clamp(1 << ZOOM_SHIFT, MAX_ZOOM_OUT);

    let zoom_y = (dy * MAX_ZOOM_OUT) / (LOG_SPACE_HEIGHT >> 2);
    let zoom_y = zoom_y.clamp(1 << ZOOM_SHIFT, MAX_ZOOM_OUT);

    let mut next_reduction = zoom_x.max(zoom_y);

    if next_reduction < (2 << ZOOM_SHIFT) && is_last_battle {
        next_reduction = 2 << ZOOM_SHIFT;
    }

    next_reduction
}

/// World-to-screen coordinate conversion.
///
/// C reference: `CalcDisplayCoord()` (process.c:785-796)
pub fn calc_display_coord_step(coord: i32, origin: i32, reduction: i32) -> i32 {
    (coord - origin) >> reduction
}

/// Continuous zoom coordinate conversion.
pub fn calc_display_coord_continuous(coord: i32, origin: i32, zoom_factor: i32) -> i32 {
    ((coord - origin) << ZOOM_SHIFT) / zoom_factor
}

/// Calculate camera view state and scroll deltas.
///
/// C reference: `CalcView()` (process.c:283-358)
///
/// Returns (view_state, scroll_dx, scroll_dy).
pub fn calc_view(
    origin: &mut super::element::Point,
    next_reduction: i32,
    current_zoom_out: &mut i32,
    space_org: &mut super::element::Point,
    ships_alive: u8,
    zoom_mode: ZoomMode,
    is_hq_space: bool,
) -> (ViewState, i32, i32) {
    let mut dx = (LOG_SPACE_WIDTH / 2) - origin.x as i32;
    let mut dy = (LOG_SPACE_HEIGHT / 2) - origin.y as i32;
    dx = wrap_delta_x(dx);
    dy = wrap_delta_y(dy);

    // Single-ship clamping
    if ships_alive == 1 {
        dx = dx.clamp(-ORG_JUMP_X, ORG_JUMP_X);
        dy = dy.clamp(-ORG_JUMP_Y, ORG_JUMP_Y);
    }

    let view_state = if *current_zoom_out == next_reduction {
        if dx == 0 && dy == 0 && !is_hq_space {
            ViewState::Stable
        } else {
            ViewState::Scroll
        }
    } else {
        match zoom_mode {
            ZoomMode::Step => {
                space_org.x = (LOG_SPACE_WIDTH / 2) as i16
                    - (LOG_SPACE_WIDTH >> ((MAX_REDUCTION + 1) as i32 - next_reduction)) as i16;
                space_org.y = (LOG_SPACE_HEIGHT / 2) as i16
                    - (LOG_SPACE_HEIGHT >> ((MAX_REDUCTION + 1) as i32 - next_reduction)) as i16;
            }
            ZoomMode::Continuous => {
                let mut nr = next_reduction;
                if ships_alive == 1
                    && *current_zoom_out > nr
                    && *current_zoom_out <= MAX_ZOOM_OUT
                    && *current_zoom_out - nr > ZOOM_JUMP
                {
                    nr = *current_zoom_out - ZOOM_JUMP;
                }
                space_org.x = display_align(
                    (LOG_SPACE_WIDTH / 2) - (LOG_SPACE_WIDTH * nr / (MAX_ZOOM_OUT << 2)),
                ) as i16;
                space_org.y = display_align(
                    (LOG_SPACE_HEIGHT / 2) - (LOG_SPACE_HEIGHT * nr / (MAX_ZOOM_OUT << 2)),
                ) as i16;
            }
        }
        *current_zoom_out = next_reduction;
        ViewState::Change
    };

    (view_state, dx, dy)
}

#[cfg(test)]
mod tests {
    use super::super::element::{Element, ElementFlags, Point};
    use super::*;

    // -- Phase 1 type/constant tests (retained) --

    #[test]
    fn test_view_state_variants() {
        assert_eq!(ViewState::Stable as u8, 0);
        assert_eq!(ViewState::Scroll as u8, 1);
        assert_eq!(ViewState::Change as u8, 2);
    }

    #[test]
    fn test_zoom_mode_variants() {
        assert_eq!(ZoomMode::Step as u8, 0);
        assert_eq!(ZoomMode::Continuous as u8, 1);
    }

    #[test]
    fn test_zoom_constants() {
        assert_eq!(ZOOM_SHIFT, 8);
        assert_eq!(MAX_REDUCTION, 3);
        assert_eq!(MAX_VIS_REDUCTION, 2);
        assert_eq!(REDUCTION_SHIFT, 1);
        assert_eq!(NUM_VIEWS, 3);
        assert_eq!(MAX_ZOOM_OUT, 1024);
    }

    #[test]
    fn test_hysteresis_constants() {
        assert_eq!(HYSTERESIS_X, 96);
        assert_eq!(HYSTERESIS_Y, 80);
    }

    #[test]
    fn test_zoom_jump_constant() {
        assert_eq!(ZOOM_JUMP, 32);
    }

    #[test]
    fn test_camera_clamp_constants() {
        assert_eq!(ORG_JUMP_X, 4);
        assert_eq!(ORG_JUMP_Y, 4);
    }

    // -- P03: setup_element tests --

    #[test]
    fn test_setup_element_copies_current_to_next() {
        let mut elem = Element::new();
        elem.current.location = Point::new(100, 200);
        setup_element(&mut elem);
        assert_eq!(elem.next.location, Point::new(100, 200));
    }

    #[test]
    fn test_setup_element_collidable_inits_intersect() {
        let mut elem = Element::new();
        // Make collidable (not NONSOLID, not DISAPPEARING)
        elem.state_flags = ElementFlags::empty();
        elem.current.location = Point::new(50, 75);
        setup_element(&mut elem);
        assert_eq!(
            elem.intersect_control.intersect_stamp.origin,
            Point::new(50, 75)
        );
        assert_eq!(elem.intersect_control.end_point, Point::new(50, 75));
    }

    // -- P03: post_process tests --

    #[test]
    fn test_post_process_commits_state() {
        let mut elem = Element::new();
        elem.next.location = Point::new(42, 84);
        elem.state_flags = ElementFlags::PRE_PROCESS;
        unsafe {
            post_process(&mut elem);
        }
        assert_eq!(elem.current.location, Point::new(42, 84));
    }

    #[test]
    fn test_post_process_flag_transitions() {
        let mut elem = Element::new();
        elem.state_flags =
            ElementFlags::PRE_PROCESS | ElementFlags::CHANGING | ElementFlags::APPEARING;
        unsafe {
            post_process(&mut elem);
        }
        assert!(elem.state_flags.contains(ElementFlags::POST_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::PRE_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::CHANGING));
        assert!(!elem.state_flags.contains(ElementFlags::APPEARING));
    }

    #[test]
    fn test_post_process_collidable_reinit() {
        let mut elem = Element::new();
        elem.state_flags = ElementFlags::empty(); // collidable (no NONSOLID/DISAPPEARING)
        elem.current.location = Point::new(10, 20);
        elem.next.location = Point::new(30, 40);
        unsafe {
            post_process(&mut elem);
        }
        // After commit, current == next; intersect start should be updated
        assert_eq!(elem.current.location, Point::new(30, 40));
        assert_eq!(
            elem.intersect_control.intersect_stamp.origin,
            Point::new(30, 40)
        );
    }

    #[test]
    fn test_max_time_value() {
        assert_eq!(MAX_TIME_VALUE, u16::MAX);
    }

    // -- P05: Zoom/Camera/Queue Orchestration tests --

    #[test]
    fn test_wrap_delta_x_no_wrap() {
        assert_eq!(wrap_delta_x(100), 100);
        assert_eq!(wrap_delta_x(-100), -100);
        assert_eq!(wrap_delta_x(0), 0);
    }

    #[test]
    fn test_wrap_delta_x_wraps_positive() {
        let half = LOG_SPACE_WIDTH / 2;
        assert_eq!(wrap_delta_x(half + 1), half + 1 - LOG_SPACE_WIDTH);
    }

    #[test]
    fn test_wrap_delta_x_wraps_negative() {
        let half = LOG_SPACE_WIDTH / 2;
        assert_eq!(wrap_delta_x(-half - 1), -half - 1 + LOG_SPACE_WIDTH);
    }

    #[test]
    fn test_wrap_delta_y_symmetry() {
        assert_eq!(wrap_delta_y(100), 100);
        let half = LOG_SPACE_HEIGHT / 2;
        assert_eq!(wrap_delta_y(half + 1), half + 1 - LOG_SPACE_HEIGHT);
    }

    #[test]
    fn test_display_align_truncates_low_bits() {
        let mask = (1 << ONE_SHIFT) - 1;
        assert_eq!(display_align(0), 0);
        assert_eq!(display_align(mask), 0);
        assert_eq!(display_align(mask + 1), (mask + 1));
        assert_eq!(display_align(0x1FF), 0x1FF & !mask);
    }

    #[test]
    fn test_calc_reduction_step_closest_zoom() {
        // Two ships very close → reduction should be 0
        let r = calc_reduction_step(0, 0, MAX_VIS_REDUCTION as i32, false, false);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_calc_reduction_step_beyond_encounter() {
        let r = calc_reduction_step(1000, 1000, 0, false, true);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_calc_reduction_step_last_battle_minimum() {
        // Should bump from 0 to REDUCTION_SHIFT in last battle
        let r = calc_reduction_step(0, 0, MAX_VIS_REDUCTION as i32, true, false);
        assert_eq!(r, REDUCTION_SHIFT as i32);
    }

    #[test]
    fn test_calc_reduction_continuous_beyond_encounter() {
        let r = calc_reduction_continuous(1000, 1000, false, true);
        assert_eq!(r, 1 << ZOOM_SHIFT);
    }

    #[test]
    fn test_calc_reduction_continuous_close_ships() {
        let r = calc_reduction_continuous(0, 0, false, false);
        assert_eq!(r, 1 << ZOOM_SHIFT); // Clamped to minimum
    }

    #[test]
    fn test_calc_reduction_continuous_last_battle_minimum() {
        let r = calc_reduction_continuous(0, 0, true, false);
        assert_eq!(r, 2 << ZOOM_SHIFT);
    }

    #[test]
    fn test_calc_display_coord_step() {
        assert_eq!(calc_display_coord_step(100, 50, 1), 25);
        assert_eq!(calc_display_coord_step(100, 100, 2), 0);
    }

    #[test]
    fn test_calc_display_coord_continuous() {
        let zoom = 1 << ZOOM_SHIFT; // 1:1 zoom
        assert_eq!(calc_display_coord_continuous(200, 100, zoom), 100);
    }

    #[test]
    fn test_calc_view_stable() {
        let mut origin = Point::new((LOG_SPACE_WIDTH / 2) as i16, (LOG_SPACE_HEIGHT / 2) as i16);
        let mut zoom_out = 5;
        let mut space_org = Point::zero();
        let (vs, dx, dy) = calc_view(
            &mut origin,
            5, // same as current zoom_out
            &mut zoom_out,
            &mut space_org,
            2,
            ZoomMode::Step,
            false,
        );
        assert_eq!(vs, ViewState::Stable);
        assert_eq!(dx, 0);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_calc_view_change_on_zoom_delta() {
        let mut origin = Point::new((LOG_SPACE_WIDTH / 2) as i16, (LOG_SPACE_HEIGHT / 2) as i16);
        let mut zoom_out = 5;
        let mut space_org = Point::zero();
        let (vs, _, _) = calc_view(
            &mut origin,
            3, // different from current zoom_out
            &mut zoom_out,
            &mut space_org,
            2,
            ZoomMode::Step,
            false,
        );
        assert_eq!(vs, ViewState::Change);
        assert_eq!(zoom_out, 3);
    }

    #[test]
    fn test_calc_view_single_ship_clamping() {
        let mut origin = Point::new(0, 0); // far from center
        let mut zoom_out = 5;
        let mut space_org = Point::zero();
        let (vs, dx, dy) = calc_view(
            &mut origin,
            5,
            &mut zoom_out,
            &mut space_org,
            1, // single ship
            ZoomMode::Step,
            false,
        );
        // dx/dy should be clamped to ORG_JUMP_X/ORG_JUMP_Y
        assert!(dx.abs() <= ORG_JUMP_X);
        assert!(dy.abs() <= ORG_JUMP_Y);
        assert_eq!(vs, ViewState::Scroll);
    }
}
