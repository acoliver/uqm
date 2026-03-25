//! Process Loop — Types + Orchestration Logic
//!
//! Type definitions, constants, and per-element orchestration for the battle
//! process loop. Phase 1 defined types; Phase 2/3 (P03+) adds the behavioral
//! logic that C's `PreProcess`, `PostProcess`, and related functions implement.
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

use super::display_list::{DisplayList, ElementHandle};
use super::element::{Element, ElementFlags};

// ---------------------------------------------------------------------------
// Process Loop Error Type
// ---------------------------------------------------------------------------

/// Errors that can occur during process loop operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessError {
    /// Display list pool is exhausted
    PoolExhausted,
    /// Display primitive free list is exhausted
    PrimExhausted,
    /// Element handle is invalid or stale
    InvalidHandle,
    /// Element not found in display list
    NotInList,
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::PoolExhausted => write!(f, "display list pool exhausted"),
            ProcessError::PrimExhausted => write!(f, "display primitive free list exhausted"),
            ProcessError::InvalidHandle => write!(f, "invalid element handle"),
            ProcessError::NotInList => write!(f, "element not in display list"),
        }
    }
}

impl std::error::Error for ProcessError {}

// ---------------------------------------------------------------------------
// P03: Element Allocation/Deallocation (process.c:76-114)
// ---------------------------------------------------------------------------

/// Allocate a new element from the display list pool.
///
/// C reference: `AllocElement()` (process.c:76-99)
///
/// Allocates an element from the pool, zeros it, and allocates a display
/// primitive index. Returns a handle to the new element.
pub fn alloc_element(display_list: &mut DisplayList) -> Result<ElementHandle, ProcessError> {
    let handle = display_list.alloc().ok_or(ProcessError::PoolExhausted)?;

    // Element is zeroed by DisplayList::alloc (Element::new())
    // Display prim allocation is deferred to c_bridge in P06;
    // for now the prim_index field is set to 0 (a valid sentinel).

    Ok(handle)
}

/// Free an element back to the display list pool.
///
/// C reference: `FreeElement()` (process.c:101-114)
///
/// Frees the display primitive and returns the element to the pool.
pub fn free_element(
    display_list: &mut DisplayList,
    handle: ElementHandle,
) -> Result<(), ProcessError> {
    // Display prim deallocation deferred to c_bridge in P06.
    if display_list.free(handle) {
        Ok(())
    } else {
        Err(ProcessError::InvalidHandle)
    }
}

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
// P03: Untarget (process.c:60-87)
// ---------------------------------------------------------------------------

/// Clear all hTarget references pointing to the given element.
///
/// C reference: `Untarget()` (process.c:60-87)
///
/// Walks the display list and clears `h_target` on any element whose
/// `h_target` points to the element being removed.
pub fn untarget(display_list: &mut DisplayList, target_ptr: *mut Element) {
    if target_ptr.is_null() {
        return;
    }

    // Collect handles that need updating (can't mutate during iteration)
    let handles_to_clear: Vec<ElementHandle> = display_list
        .iter()
        .filter_map(|(handle, elem)| {
            if elem.h_target == target_ptr {
                Some(handle)
            } else {
                None
            }
        })
        .collect();

    for handle in handles_to_clear {
        if let Some(elem) = display_list.get_mut(handle) {
            elem.h_target = std::ptr::null_mut();
        }
    }
}

/// Remove an element from the display list.
///
/// C reference: `RemoveElement()` (process.c:206-225)
///
/// Unlinks the element from the list, clears targeting references,
/// and frees it.
pub fn remove_element(
    display_list: &mut DisplayList,
    handle: ElementHandle,
) -> Result<(), ProcessError> {
    // Get the raw element pointer for untarget before removal
    let elem_ptr = display_list
        .get_mut(handle)
        .map(|e| e as *mut Element)
        .ok_or(ProcessError::InvalidHandle)?;

    // Clear all hTarget references to this element
    untarget(display_list, elem_ptr);

    // Remove from the linked list
    if !display_list.remove(handle) {
        return Err(ProcessError::NotInList);
    }

    // Free the element back to pool
    free_element(display_list, handle)
}

// ---------------------------------------------------------------------------
// P03: PreProcess (process.c:128-186)
// ---------------------------------------------------------------------------

/// Per-element preprocess pass.
///
/// C reference: `PreProcess()` (process.c:128-186)
///
/// Handles the per-element preprocessing logic:
/// 1. Death check (life_span == 0 → death callback, DISAPPEARING, untarget)
/// 2. APPEARING handling (setup element; PLAYER_SHIP clears locally)
/// 3. Preprocess callback invocation
/// 4. Velocity stepping
/// 5. FINITE_LIFE decrement
/// 6. Flag transitions (set PRE_PROCESS, clear POST_PROCESS|COLLISION)
///
/// # Safety
/// This function calls C function pointers stored in the element struct.
/// The caller must ensure:
/// - The handle is valid in the display list
/// - Callback functions are valid C-ABI function pointers
/// - After callback invocation, element pointers may be invalidated
pub unsafe fn pre_process(handle: ElementHandle, display_list: &mut DisplayList) {
    // Get raw pointer — we need to bypass the borrow checker here because
    // the C design requires simultaneous access to element and display list
    // (untarget walks the list while modifying elements).
    let elem_ptr: *mut Element = match display_list.get_mut(handle) {
        Some(e) => e as *mut Element,
        None => return,
    };

    let element = &mut *elem_ptr;

    // Step 1: Death check — if life_span is already 0
    if element.life_span == 0 {
        if !element.p_parent.is_null() {
            untarget(display_list, elem_ptr);
        }

        element.state_flags.insert(ElementFlags::DISAPPEARING);
        if let Some(death_func) = element.death_func {
            death_func(elem_ptr);
        }
    }

    // Re-acquire after potential callback
    let element = &mut *elem_ptr;
    let mut state_flags = element.state_flags;

    if !state_flags.contains(ElementFlags::DISAPPEARING) {
        // Step 2: APPEARING handling
        if state_flags.contains(ElementFlags::APPEARING) {
            setup_element(element);

            if state_flags.contains(ElementFlags::PLAYER_SHIP) {
                state_flags.remove(ElementFlags::APPEARING);
            }
        }

        // Step 3: Preprocess callback (skipped if APPEARING still set)
        if element.preprocess_func.is_some() && !state_flags.contains(ElementFlags::APPEARING) {
            if let Some(preprocess_func) = element.preprocess_func {
                preprocess_func(elem_ptr);
            }

            // Re-acquire after callback
            let element = &mut *elem_ptr;
            state_flags = element.state_flags;

            if state_flags.contains(ElementFlags::CHANGING) && element.is_collidable() {
                init_intersect_frame(element);
            }
        }

        let element = &mut *elem_ptr;

        // Step 4: Velocity stepping (unless IGNORE_VELOCITY)
        if !state_flags.contains(ElementFlags::IGNORE_VELOCITY) {
            let (dx, dy) = element.velocity.get_next_components(1);
            if dx != 0 || dy != 0 {
                state_flags.insert(ElementFlags::CHANGING);
                element.next.location.x = element.next.location.x.wrapping_add(dx as i16);
                element.next.location.y = element.next.location.y.wrapping_add(dy as i16);
            }
        }

        // Update intersection end point if collidable
        if element.is_collidable() {
            init_intersect_end_point(element);
        }

        // Step 5: FINITE_LIFE decrement
        if state_flags.contains(ElementFlags::FINITE_LIFE) {
            element.life_span = element.life_span.saturating_sub(1);
        }
    }

    // Step 6: Flag transitions — set PRE_PROCESS, clear POST_PROCESS|COLLISION
    let element = &mut *elem_ptr;
    element.state_flags = (state_flags & !(ElementFlags::POST_PROCESS | ElementFlags::COLLISION))
        | ElementFlags::PRE_PROCESS;
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

#[cfg(test)]
mod tests {
    use super::super::display_list::DisplayList;
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

    // -- P03: alloc/free tests --

    #[test]
    fn test_alloc_element_success() {
        let mut dl = DisplayList::with_default_capacity();
        let handle = alloc_element(&mut dl).unwrap();
        // Verify element is zeroed
        let elem = dl.get(handle).unwrap();
        assert_eq!(elem.life_span, 0);
        assert_eq!(elem.state_flags, ElementFlags::empty());
        assert!(elem.preprocess_func.is_none());
    }

    #[test]
    fn test_alloc_free_roundtrip() {
        let mut dl = DisplayList::with_default_capacity();
        let handle = alloc_element(&mut dl).unwrap();
        assert!(free_element(&mut dl, handle).is_ok());
    }

    #[test]
    fn test_free_invalid_handle() {
        let mut dl = DisplayList::with_default_capacity();
        let handle = alloc_element(&mut dl).unwrap();
        let _ = free_element(&mut dl, handle);
        // Freeing again should fail (stale handle)
        assert_eq!(
            free_element(&mut dl, handle),
            Err(ProcessError::InvalidHandle)
        );
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut dl = DisplayList::new(2); // tiny pool
        let _h1 = alloc_element(&mut dl).unwrap();
        let _h2 = alloc_element(&mut dl).unwrap();
        assert_eq!(alloc_element(&mut dl), Err(ProcessError::PoolExhausted));
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

    // -- P03: untarget tests --

    #[test]
    fn test_untarget_clears_references() {
        let mut dl = DisplayList::with_default_capacity();
        let h1 = alloc_element(&mut dl).unwrap();
        let h2 = alloc_element(&mut dl).unwrap();
        dl.push_back(h1);
        dl.push_back(h2);

        // Point h2's h_target at h1's element
        let h1_ptr = dl.get_mut(h1).map(|e| e as *mut Element).unwrap();
        dl.get_mut(h2).unwrap().h_target = h1_ptr;

        // Untarget h1 — should clear h2's h_target
        untarget(&mut dl, h1_ptr);
        assert!(dl.get(h2).unwrap().h_target.is_null());
    }

    #[test]
    fn test_untarget_null_is_noop() {
        let mut dl = DisplayList::with_default_capacity();
        untarget(&mut dl, std::ptr::null_mut()); // should not panic
    }

    // -- P03: remove_element tests --

    #[test]
    fn test_remove_element_removes_from_list() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();
        dl.push_back(h);
        assert_eq!(dl.count(), 1);

        assert!(remove_element(&mut dl, h).is_ok());
        assert_eq!(dl.count(), 0);
    }

    // -- P03: pre_process tests --

    #[test]
    fn test_pre_process_death_on_zero_lifespan() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();
        dl.push_back(h);

        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 0;
            elem.state_flags = ElementFlags::PRE_PROCESS;
        }

        unsafe { pre_process(h, &mut dl) };

        let elem = dl.get(h).unwrap();
        assert!(elem.state_flags.contains(ElementFlags::DISAPPEARING));
    }

    #[test]
    fn test_pre_process_appearing_player_ship() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();

        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 10;
            elem.state_flags = ElementFlags::APPEARING | ElementFlags::PLAYER_SHIP;
        }

        unsafe { pre_process(h, &mut dl) };

        let elem = dl.get(h).unwrap();
        assert!(elem.state_flags.contains(ElementFlags::PRE_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::POST_PROCESS));
    }

    #[test]
    fn test_pre_process_appearing_non_player_skips_callback() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();

        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 10;
            elem.state_flags = ElementFlags::APPEARING;
        }

        unsafe { pre_process(h, &mut dl) };

        let elem = dl.get(h).unwrap();
        assert!(elem.state_flags.contains(ElementFlags::PRE_PROCESS));
    }

    #[test]
    fn test_pre_process_velocity_stepping() {
        use super::super::velocity::VELOCITY_SHIFT;
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();

        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 10;
            elem.state_flags = ElementFlags::empty();
            elem.next.location = Point::new(100, 200);
            // Use values large enough to survive fixed-point truncation
            elem.velocity
                .set_components(5 << VELOCITY_SHIFT, -(3i32 << VELOCITY_SHIFT));
        }

        unsafe { pre_process(h, &mut dl) };

        let elem = dl.get(h).unwrap();
        assert!(elem.state_flags.contains(ElementFlags::CHANGING));
    }

    #[test]
    fn test_pre_process_finite_life_decrement() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();

        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 5;
            elem.state_flags = ElementFlags::FINITE_LIFE;
        }

        unsafe { pre_process(h, &mut dl) };

        let elem = dl.get(h).unwrap();
        assert_eq!(elem.life_span, 4);
    }

    #[test]
    fn test_pre_process_ignore_velocity() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();

        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 10;
            elem.state_flags = ElementFlags::IGNORE_VELOCITY;
            elem.next.location = Point::new(100, 200);
            elem.velocity.set_components(5, -3);
        }

        unsafe { pre_process(h, &mut dl) };

        let elem = dl.get(h).unwrap();
        assert_eq!(elem.next.location, Point::new(100, 200));
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

    // -- P03: ProcessError tests --

    #[test]
    fn test_process_error_display() {
        assert_eq!(
            format!("{}", ProcessError::PoolExhausted),
            "display list pool exhausted"
        );
        assert_eq!(
            format!("{}", ProcessError::InvalidHandle),
            "invalid element handle"
        );
    }
}
