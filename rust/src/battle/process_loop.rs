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

// ---------------------------------------------------------------------------
// P04: ProcessCollisions (process.c:362-628)
// ---------------------------------------------------------------------------

/// Time value type matching C's TIME_VALUE (unsigned 16-bit).
pub type TimeValue = u16;

/// Maximum time value for collision checks.
pub const MAX_TIME_VALUE: TimeValue = u16::MAX;

/// Sentinel value returned by DrawablesIntersect when elements are
/// stuck overlapping at maximum time (not a valid collision time).
const STUCK_SENTINEL: TimeValue = 1;

/// Bridge to C's DrawablesIntersect. Returns 0 (no hit),
/// 1 (stuck sentinel), or >1 (collision time).
///
/// In production this calls the C function via FFI.
/// For testing, this can be overridden via the `collision_test_fn` field
/// on CollisionContext.
type DrawablesIntersectFn = unsafe fn(a: &Element, b: &Element, min_time: TimeValue) -> TimeValue;

/// Bridge to C's do_damage function.
type DoDamageFn = unsafe fn(element: *mut Element, damage: u16);

/// Bridge to C's collide (elastic collision) function.
type CollideFn = unsafe fn(a: *mut Element, b: *mut Element);

/// Context for collision processing, carrying function pointers
/// to bridge functions and the display list.
pub struct CollisionContext<'a> {
    pub display_list: &'a mut DisplayList,
    pub drawables_intersect: DrawablesIntersectFn,
    pub do_damage: DoDamageFn,
    pub collide: CollideFn,
    pub process_flags: ElementFlags,
}

/// Recursive collision detection and dispatch.
///
/// C reference: `ProcessCollisions()` (process.c:362-628)
///
/// Walks successor elements from `succ_handle`, checking each against
/// `element_ptr` for collisions. Handles:
/// - APPEARING+FINITE_LIFE prefilter (skip DrawablesIntersect)
/// - Stuck overlap sentinel (frame normalization, APPEARING destruction)
/// - COLLISION flag alternate intersection check
/// - Recursive earlier-time collision detection
/// - PLAYER_SHIP-aware dispatch ordering
/// - Post-collision position snapping (conditional on newly-set COLLISION)
/// - Post-bounce elastic collision + recursive recheck from head
///
/// Returns the element's COLLISION flag state.
///
/// # Safety
/// - All element pointers must be valid
/// - Callback function pointers must be valid C-ABI
/// - DisplayList must contain all referenced elements
pub unsafe fn process_collisions(
    ctx: &mut CollisionContext<'_>,
    succ_handle: Option<ElementHandle>,
    element_ptr: *mut Element,
    min_time: TimeValue,
) -> bool {
    let mut current_succ = succ_handle;

    while let Some(test_handle) = current_succ {
        let test_ptr: *mut Element = match ctx.display_list.get_mut(test_handle) {
            Some(e) => e as *mut Element,
            None => break,
        };
        let test_elem = &mut *test_ptr;

        // PreProcess unprocessed elements encountered during walk
        if !test_elem.state_flags.intersects(ctx.process_flags) {
            pre_process(test_handle, ctx.display_list);
        }

        // Advance to next successor before any modifications
        current_succ = ctx.display_list.next(test_handle);

        let element = &mut *element_ptr;
        let test_elem = &mut *test_ptr;

        // Skip self-collision
        if std::ptr::eq(element_ptr, test_ptr) {
            continue;
        }

        // Check collision eligibility (Phase 1 collision_possible)
        if !test_elem.collision_possible(element) {
            continue;
        }

        let state_flags = element.state_flags;
        let test_state_flags = test_elem.state_flags;

        // APPEARING+FINITE_LIFE prefilter (process.c:389-394)
        let time_val = if (state_flags | test_state_flags).contains(ElementFlags::FINITE_LIFE)
            && ((state_flags.contains(ElementFlags::APPEARING) && element.life_span > 1)
                || (test_state_flags.contains(ElementFlags::APPEARING) && test_elem.life_span > 1))
        {
            0
        } else {
            process_collision_loop(
                ctx,
                element_ptr,
                test_ptr,
                min_time,
                state_flags,
                test_state_flags,
            )
        };

        if time_val > 0
            && dispatch_collision(
                ctx,
                element_ptr,
                test_ptr,
                test_handle,
                time_val,
                state_flags,
                test_state_flags,
            )
        {
            return true;
        }
    }

    let element = &*element_ptr;
    element.state_flags.contains(ElementFlags::COLLISION)
}

/// Inner while loop for DrawablesIntersect + stuck overlap resolution.
/// Returns the final time_val (0 = no collision, >0 = collision time).
///
/// C reference: process.c:397-516
unsafe fn process_collision_loop(
    ctx: &mut CollisionContext<'_>,
    element_ptr: *mut Element,
    test_ptr: *mut Element,
    min_time: TimeValue,
    state_flags: ElementFlags,
    test_state_flags: ElementFlags,
) -> TimeValue {
    let element = &mut *element_ptr;
    let test_elem = &mut *test_ptr;

    let mut time_val = (ctx.drawables_intersect)(element, test_elem, min_time);

    // Stuck overlap while loop (process.c:397-516)
    while time_val == STUCK_SENTINEL
        && !(state_flags | test_state_flags).contains(ElementFlags::FINITE_LIFE)
    {
        #[cfg(feature = "debug-process")]
        tracing::debug!("BAD NEWS: stuck overlap between elements");

        let element = &mut *element_ptr;
        let test_elem = &mut *test_ptr;

        // Alternate intersection check when COLLISION already set
        if state_flags.contains(ElementFlags::COLLISION) {
            init_intersect_end_point(test_elem);
            test_elem.intersect_control.intersect_stamp.origin =
                test_elem.intersect_control.end_point;
            time_val = (ctx.drawables_intersect)(element, test_elem, 1);
            init_intersect_start_point(test_elem);
        }

        if time_val == STUCK_SENTINEL {
            let cur_frame = element.current.frame;
            let next_frame = element.next.frame;
            let test_cur_frame = test_elem.current.frame;
            let test_next_frame = test_elem.next.frame;

            if next_frame == cur_frame && test_next_frame == test_cur_frame {
                // Identical frames — destroy APPEARING elements
                if test_state_flags.contains(ElementFlags::APPEARING) {
                    (ctx.do_damage)(test_ptr, test_elem.crew_or_hp);
                    let test_elem = &mut *test_ptr;
                    if !test_elem.p_parent.is_null() {
                        untarget(ctx.display_list, test_ptr);
                    }
                    let test_elem = &mut *test_ptr;
                    test_elem
                        .state_flags
                        .insert(ElementFlags::COLLISION | ElementFlags::DISAPPEARING);
                    if let Some(death_func) = test_elem.death_func {
                        death_func(test_ptr);
                    }
                }
                if state_flags.contains(ElementFlags::APPEARING) {
                    let element = &mut *element_ptr;
                    (ctx.do_damage)(element_ptr, element.crew_or_hp);
                    let element = &mut *element_ptr;
                    if !element.p_parent.is_null() {
                        untarget(ctx.display_list, element_ptr);
                    }
                    let element = &mut *element_ptr;
                    element
                        .state_flags
                        .insert(ElementFlags::COLLISION | ElementFlags::DISAPPEARING);
                    if let Some(death_func) = element.death_func {
                        death_func(element_ptr);
                    }
                    return 0; // C returns COLLISION here
                }

                time_val = 0;
            } else {
                // Differing frames — normalize (process.c:454-505)
                normalize_stuck_frames(element_ptr, test_ptr);
            }
        }

        if time_val == 0 {
            let element = &mut *element_ptr;
            let test_elem = &mut *test_ptr;
            init_intersect_end_point(element);
            init_intersect_end_point(test_elem);
            break;
        }

        // Re-check DrawablesIntersect after normalization
        let element = &mut *element_ptr;
        let test_elem = &mut *test_ptr;
        time_val = (ctx.drawables_intersect)(element, test_elem, min_time);
    }

    time_val
}

/// Frame normalization for stuck overlap resolution.
///
/// C reference: process.c:454-505
///
/// When frames differ between current and next, try to normalize
/// them to resolve the overlap. Also reinitializes intersection
/// data for both elements.
unsafe fn normalize_stuck_frames(element_ptr: *mut Element, test_ptr: *mut Element) {
    let element = &mut *element_ptr;
    let test_elem = &mut *test_ptr;

    // Normalize element's frame
    let cur_frame = element.current.frame;
    let next_frame = element.next.frame;
    if next_frame != cur_frame {
        // In C this uses GetFrameIndex/SetEquFrameIndex for frame index comparison.
        // For Rust, we copy current image to next and clamp life_span.
        element.next.frame = cur_frame;
        if element.life_span > super::element::NORMAL_LIFE {
            element.life_span = super::element::NORMAL_LIFE;
        }
    }

    // Normalize test element's frame
    let test_cur = test_elem.current.frame;
    let test_next = test_elem.next.frame;
    if test_next != test_cur {
        test_elem.next.frame = test_cur;
        if test_elem.life_span > super::element::NORMAL_LIFE {
            test_elem.life_span = super::element::NORMAL_LIFE;
        }
    }

    // Reinitialize intersection data for both
    init_intersect_start_point(element);
    init_intersect_end_point(element);
    init_intersect_frame(element);

    init_intersect_start_point(test_elem);
    init_intersect_end_point(test_elem);
    init_intersect_frame(test_elem);

    // Note: PLAYER_SHIP ShipFacing update is deferred to ship_runtime (P07)
    // because we don't have StarShip access here yet.
}

/// Collision dispatch — recursive earlier-time check + handler invocation.
///
/// C reference: process.c:519-620
///
/// Returns true if COLLISION flag was set (caller should return).
unsafe fn dispatch_collision(
    ctx: &mut CollisionContext<'_>,
    element_ptr: *mut Element,
    test_ptr: *mut Element,
    test_handle: ElementHandle,
    time_val: TimeValue,
    pre_state_flags: ElementFlags,
    pre_test_state_flags: ElementFlags,
) -> bool {
    let element = &mut *element_ptr;
    let test_elem = &mut *test_ptr;

    #[cfg(feature = "debug-process")]
    tracing::debug!("Collision candidate at time {}", time_val);

    // Save collision points
    let save_pt = element.intersect_control.end_point;
    let test_save_pt = test_elem.intersect_control.end_point;

    // Reinitialize end points
    init_intersect_end_point(element);
    init_intersect_end_point(test_elem);

    // Recursive earlier-time checks (process.c:531-540)
    let should_dispatch = time_val == STUCK_SENTINEL || {
        let element = &*element_ptr;
        let test_elem = &*test_ptr;

        // Check if element has earlier collision with something else
        let elem_earlier = element.state_flags.contains(ElementFlags::COLLISION) || {
            let succ = ctx.display_list.next(test_handle);
            !process_collisions(ctx, succ, element_ptr, time_val - 1)
        };

        // Check if test element has earlier collision
        elem_earlier && {
            test_elem.state_flags.contains(ElementFlags::COLLISION) || {
                // APPEARING elements check from head; others from element's successor
                let start = if test_elem.state_flags.contains(ElementFlags::APPEARING) {
                    ctx.display_list.head()
                } else {
                    // Find element_ptr's handle and get its successor
                    find_handle_for_ptr(ctx.display_list, element_ptr)
                        .and_then(|h| ctx.display_list.next(h))
                };
                !process_collisions(ctx, start, test_ptr, time_val - 1)
            }
        }
    };

    if !should_dispatch {
        return false;
    }

    // Re-read flags after recursive calls
    let element = &mut *element_ptr;
    let test_elem = &mut *test_ptr;
    let _state_flags = element.state_flags;
    let test_state_flags = test_elem.state_flags;

    #[cfg(feature = "debug-process")]
    tracing::debug!("PROCESSING collision at time {}", time_val);

    // Dispatch collision handlers in PLAYER_SHIP-aware order
    // collision_func signature: (self, &self_save_pt, other, &other_save_pt)
    let save_pt_ref = &save_pt as *const super::element::Point;
    let test_save_pt_ref = &test_save_pt as *const super::element::Point;

    if test_state_flags.contains(ElementFlags::PLAYER_SHIP) {
        if let Some(collision_func) = test_elem.collision_func {
            collision_func(test_ptr, test_save_pt_ref, element_ptr, save_pt_ref);
        }
        let element = &mut *element_ptr;
        if let Some(collision_func) = element.collision_func {
            collision_func(element_ptr, save_pt_ref, test_ptr, test_save_pt_ref);
        }
    } else {
        if let Some(collision_func) = element.collision_func {
            collision_func(element_ptr, save_pt_ref, test_ptr, test_save_pt_ref);
        }
        let test_elem = &mut *test_ptr;
        if let Some(collision_func) = test_elem.collision_func {
            collision_func(test_ptr, test_save_pt_ref, element_ptr, save_pt_ref);
        }
    }

    // Post-collision position snapping (process.c:572-610)
    let element = &mut *element_ptr;
    let test_elem = &mut *test_ptr;

    // Test element snapping (conditional on COLLISION being newly set)
    if test_elem.state_flags.contains(ElementFlags::COLLISION)
        && !pre_test_state_flags.contains(ElementFlags::COLLISION)
    {
        test_elem.intersect_control.intersect_stamp.origin = test_save_pt;
        test_elem.next.location.x =
            super::battle_types::display_to_world(test_save_pt.x as i32) as i16;
        test_elem.next.location.y =
            super::battle_types::display_to_world(test_save_pt.y as i32) as i16;
        init_intersect_end_point(test_elem);
    }

    // Element snapping (conditional on COLLISION being newly set)
    if element.state_flags.contains(ElementFlags::COLLISION) {
        if !pre_state_flags.contains(ElementFlags::COLLISION) {
            element.intersect_control.intersect_stamp.origin = save_pt;
            element.next.location.x =
                super::battle_types::display_to_world(save_pt.x as i32) as i16;
            element.next.location.y =
                super::battle_types::display_to_world(save_pt.y as i32) as i16;
            init_intersect_end_point(element);

            // Post-bounce elastic collision for non-FINITE_LIFE pairs
            if !pre_state_flags.contains(ElementFlags::FINITE_LIFE)
                && !pre_test_state_flags.contains(ElementFlags::FINITE_LIFE)
            {
                (ctx.collide)(element_ptr, test_ptr);

                // Re-check from head for both elements
                let head = ctx.display_list.head();
                process_collisions(ctx, head, element_ptr, MAX_TIME_VALUE);
                let head = ctx.display_list.head();
                process_collisions(ctx, head, test_ptr, MAX_TIME_VALUE);
            }
        }
        return true;
    }

    // If element is no longer collidable, set COLLISION and return
    let element = &*element_ptr;
    if !element.is_collidable() {
        let element = &mut *element_ptr;
        element.state_flags.insert(ElementFlags::COLLISION);
        return true;
    }

    false
}

/// Find the handle for an element given its raw pointer.
/// Needed for determining successor in recursive checks.
fn find_handle_for_ptr(display_list: &DisplayList, ptr: *mut Element) -> Option<ElementHandle> {
    for (handle, elem) in display_list.iter() {
        if std::ptr::eq(elem as *const Element, ptr as *const Element) {
            return Some(handle);
        }
    }
    None
}

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

    let view_state;
    if *current_zoom_out == next_reduction {
        view_state = if dx == 0 && dy == 0 && !is_hq_space {
            ViewState::Stable
        } else {
            ViewState::Scroll
        };
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
        view_state = ViewState::Change;
    }

    (view_state, dx, dy)
}

// ---------------------------------------------------------------------------
// P05: Queue Orchestration (process.c:629-1061)
// ---------------------------------------------------------------------------

/// Battle state holding globals needed for queue orchestration.
/// In C these are file-scope globals; in Rust they're collected here.
pub struct BattleState {
    pub display_list: DisplayList,
    pub zoom_out: i32,
    pub opt_max_zoom_out: i32,
    pub space_org: super::element::Point,
    pub zoom_mode: ZoomMode,
    pub battle_counter: [u8; 2],
    pub nth_frame: u16,
    pub is_hq_space: bool,
    pub is_last_battle: bool,
    pub is_beyond_encounter: bool,
    pub is_super_melee: bool,
    pub check_abort_or_load: bool,
    pub stereo_sfx: bool,
}

impl BattleState {
    /// Create a new battle state with default values.
    pub fn new() -> Self {
        Self {
            display_list: DisplayList::with_default_capacity(),
            zoom_out: (MAX_VIS_REDUCTION + 1) as i32,
            opt_max_zoom_out: MAX_VIS_REDUCTION as i32,
            space_org: super::element::Point::zero(),
            zoom_mode: ZoomMode::Step,
            battle_counter: [0; 2],
            nth_frame: 0,
            is_hq_space: false,
            is_last_battle: false,
            is_beyond_encounter: false,
            is_super_melee: false,
            check_abort_or_load: false,
            stereo_sfx: false,
        }
    }

    /// Initialize display list and zoom state.
    ///
    /// C reference: `InitDisplayList()` (process.c:985-1008)
    pub fn init_display_list(&mut self) {
        match self.zoom_mode {
            ZoomMode::Step => {
                self.zoom_out = (MAX_VIS_REDUCTION + 1) as i32;
                self.opt_max_zoom_out = MAX_VIS_REDUCTION as i32;
            }
            ZoomMode::Continuous => {
                self.zoom_out = MAX_ZOOM_OUT + (1 << ZOOM_SHIFT);
                self.opt_max_zoom_out = MAX_ZOOM_OUT;
            }
        }
        // Re-create the display list (empties active list, rebuilds free chain)
        self.display_list = DisplayList::with_default_capacity();
    }
}

impl Default for BattleState {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-process the display queue: iterate head-to-tail, preprocess each
/// element, run collision detection, track ship positions for camera.
///
/// C reference: `PreProcessQueue()` (process.c:629-746)
///
/// Returns (view_state, scroll_x, scroll_y).
///
/// # Safety
/// Calls through element callback function pointers.
pub unsafe fn pre_process_queue(
    state: &mut BattleState,
    collision_ctx_fn: DrawablesIntersectFn,
    do_damage_fn: DoDamageFn,
    collide_fn: CollideFn,
) -> (ViewState, i32, i32) {
    let sides_active = (if state.battle_counter[0] > 0 { 1u8 } else { 0 })
        + (if state.battle_counter[1] > 0 { 1 } else { 0 });

    let (mut min_reduction, mut max_reduction) = match state.zoom_mode {
        ZoomMode::Step => {
            let v = (MAX_VIS_REDUCTION + 1) as i32;
            (v, v)
        }
        ZoomMode::Continuous => {
            let v = MAX_ZOOM_OUT + (1 << ZOOM_SHIFT);
            (v, v)
        }
    };

    let mut origin =
        super::element::Point::new((LOG_SPACE_WIDTH / 2) as i16, (LOG_SPACE_HEIGHT / 2) as i16);
    let mut ships_alive: u8 = 0;

    let mut current = state.display_list.head();
    while let Some(handle) = current {
        let elem_ptr: *mut Element = match state.display_list.get_mut(handle) {
            Some(e) => e as *mut Element,
            None => break,
        };
        let element = &mut *elem_ptr;

        if !element.state_flags.contains(ElementFlags::PRE_PROCESS) {
            pre_process(handle, &mut state.display_list);
        }

        let next = state.display_list.next(handle);
        let element = &*elem_ptr;

        // Run collision detection against successors
        if element.is_collidable() && !element.state_flags.contains(ElementFlags::COLLISION) {
            let mut ctx = CollisionContext {
                display_list: &mut state.display_list,
                drawables_intersect: collision_ctx_fn,
                do_damage: do_damage_fn,
                collide: collide_fn,
                process_flags: ElementFlags::PRE_PROCESS,
            };
            process_collisions(&mut ctx, next, elem_ptr, MAX_TIME_VALUE);
        }

        // Track player ship positions for camera
        let element = &*elem_ptr;
        if element.state_flags.contains(ElementFlags::PLAYER_SHIP) {
            ships_alive += 1;

            if max_reduction > state.opt_max_zoom_out && min_reduction > state.opt_max_zoom_out {
                origin.x = display_align(element.next.location.x as i32) as i16;
                origin.y = display_align(element.next.location.y as i32) as i16;
            }

            let dx_raw = display_align(element.next.location.x as i32) - origin.x as i32;
            let dx_wrapped = wrap_delta_x(dx_raw);
            let dy_raw = display_align(element.next.location.y as i32) - origin.y as i32;
            let dy_wrapped = wrap_delta_y(dy_raw);

            if sides_active <= 2 || element.player_nr == 0 {
                origin.x = display_align(origin.x as i32 + (dx_wrapped >> 1)) as i16;
                origin.y = display_align(origin.y as i32 + (dy_wrapped >> 1)) as i16;

                let adx = dx_wrapped.abs();
                let ady = dy_wrapped.abs();
                max_reduction = match state.zoom_mode {
                    ZoomMode::Step => calc_reduction_step(
                        adx,
                        ady,
                        state.zoom_out,
                        state.is_last_battle,
                        state.is_beyond_encounter,
                    ),
                    ZoomMode::Continuous => calc_reduction_continuous(
                        adx,
                        ady,
                        state.is_last_battle,
                        state.is_beyond_encounter,
                    ),
                };
            } else if max_reduction > state.opt_max_zoom_out
                && min_reduction <= state.opt_max_zoom_out
            {
                origin.x = display_align(origin.x as i32 + (dx_wrapped >> 1)) as i16;
                origin.y = display_align(origin.y as i32 + (dy_wrapped >> 1)) as i16;

                let adx = dx_wrapped.abs();
                let ady = dy_wrapped.abs();
                min_reduction = match state.zoom_mode {
                    ZoomMode::Step => calc_reduction_step(
                        adx,
                        ady,
                        state.zoom_out,
                        state.is_last_battle,
                        state.is_beyond_encounter,
                    ),
                    ZoomMode::Continuous => calc_reduction_continuous(
                        adx,
                        ady,
                        state.is_last_battle,
                        state.is_beyond_encounter,
                    ),
                };
            } else {
                let adx = dx_wrapped.abs();
                let ady = dy_wrapped.abs();
                let reduction = match state.zoom_mode {
                    ZoomMode::Step => calc_reduction_step(
                        adx << 1,
                        ady << 1,
                        state.zoom_out,
                        state.is_last_battle,
                        state.is_beyond_encounter,
                    ),
                    ZoomMode::Continuous => calc_reduction_continuous(
                        adx << 1,
                        ady << 1,
                        state.is_last_battle,
                        state.is_beyond_encounter,
                    ),
                };
                if min_reduction > state.opt_max_zoom_out || reduction < min_reduction {
                    min_reduction = reduction;
                }
            }
        }

        current = next;
    }

    // Finalize reduction (process.c:732-740)
    if (min_reduction > state.opt_max_zoom_out || min_reduction <= max_reduction)
        && {
            min_reduction = max_reduction;
            min_reduction > state.opt_max_zoom_out
        }
        && {
            min_reduction = state.zoom_out;
            min_reduction > state.opt_max_zoom_out
        }
    {
        min_reduction = match state.zoom_mode {
            ZoomMode::Step => 0,
            ZoomMode::Continuous => 1 << ZOOM_SHIFT,
        };
    }

    calc_view(
        &mut origin,
        min_reduction,
        &mut state.zoom_out,
        &mut state.space_org,
        ships_alive,
        state.zoom_mode,
        state.is_hq_space,
    )
}

/// Post-process the display queue: handle newly-added elements, apply
/// scroll offsets, remove DISAPPEARING elements, convert coordinates.
///
/// C reference: `PostProcessQueue()` (process.c:798-983)
///
/// # Safety
/// Calls through element callback function pointers.
pub unsafe fn post_process_queue(
    state: &mut BattleState,
    view_state: ViewState,
    mut scroll_x: i32,
    mut scroll_y: i32,
) {
    let _reduction = match state.zoom_mode {
        ZoomMode::Step => state.zoom_out + ONE_SHIFT as i32,
        ZoomMode::Continuous => state.zoom_out << ONE_SHIFT,
    };

    let mut current = state.display_list.head();
    while let Some(handle) = current {
        let elem_ptr: *mut Element = match state.display_list.get_mut(handle) {
            Some(e) => e as *mut Element,
            None => break,
        };
        let element = &*elem_ptr;
        let state_flags = element.state_flags;

        let (delta_x, delta_y);

        if state_flags.contains(ElementFlags::PRE_PROCESS) {
            // Asymmetric DEFY_PHYSICS clearing
            let element = &mut *elem_ptr;
            element.asymmetric_defy_physics_clear();

            if state_flags.contains(ElementFlags::POST_PROCESS) {
                delta_x = 0;
                delta_y = 0;
            } else {
                delta_x = scroll_x;
                delta_y = scroll_y;
            }
        } else {
            // Newly-added element cascading (process.c:843-871)
            let mut post_handle = Some(handle);
            while let Some(ph) = post_handle {
                let pp: *mut Element = match state.display_list.get_mut(ph) {
                    Some(e) => e as *mut Element,
                    None => break,
                };
                let post_elem = &*pp;

                if !post_elem.state_flags.contains(ElementFlags::PRE_PROCESS) {
                    pre_process(ph, &mut state.display_list);
                }

                let next_post = state.display_list.next(ph);

                let post_elem = &*pp;
                if post_elem.is_collidable()
                    && !post_elem.state_flags.contains(ElementFlags::COLLISION)
                {
                    // Collisions vs entire list from head — uses PRE_PROCESS|POST_PROCESS flags
                    // (Deferred to P06 bridge: need collision_ctx for full integration)
                }

                post_handle = next_post;
            }

            scroll_x = 0;
            scroll_y = 0;
            delta_x = 0;
            delta_y = 0;
        }

        // Check for DISAPPEARING after cascading
        let element = &*elem_ptr;
        let state_flags = element.state_flags;

        if state_flags.contains(ElementFlags::DISAPPEARING) {
            let next = state.display_list.next(handle);
            // Remove and free element
            if state.display_list.remove(handle) {
                let _ = state.display_list.free(handle);
            }
            current = next;
            continue;
        }

        // Coordinate conversion for surviving elements
        // (World→screen coordinate conversion, zoom sprite selection,
        //  and InsertPrim are C-side rendering operations deferred to P06 bridge)
        if view_state != ViewState::Stable
            || state_flags.intersects(ElementFlags::APPEARING | ElementFlags::CHANGING)
        {
            let element = &mut *elem_ptr;
            let next_x = super::battle_types::wrap_x(
                element.next.location.x as i32 + delta_x,
                LOG_SPACE_WIDTH as u32,
            );
            let next_y = super::battle_types::wrap_y(
                element.next.location.y as i32 + delta_y,
                LOG_SPACE_HEIGHT as u32,
            );

            // Store wrapped+scrolled coordinates back
            element.next.location.x = next_x as i16;
            element.next.location.y = next_y as i16;
        }

        // PostProcess the element
        let element = &mut *elem_ptr;
        post_process(element);

        let next = state.display_list.next(handle);
        current = next;
    }
}

/// Top-level frame dispatch.
///
/// C reference: `RedrawQueue()` (process.c:1012-1061)
///
/// Executes the full frame: simulation (always) + rendering (conditionally).
///
/// # Safety
/// Calls through element callback function pointers and C bridge functions.
pub unsafe fn redraw_queue(
    state: &mut BattleState,
    clear: bool,
    collision_ctx_fn: DrawablesIntersectFn,
    do_damage_fn: DoDamageFn,
    collide_fn: CollideFn,
) {
    // 1. PreProcessQueue
    let (view_state, scroll_x, scroll_y) =
        pre_process_queue(state, collision_ctx_fn, do_damage_fn, collide_fn);

    // 2. PostProcessQueue
    post_process_queue(state, view_state, scroll_x, scroll_y);

    // 3. Sound position updates (deferred to P06 bridge: UpdateSoundPositions)

    // 4. Rendering (deferred to P06 bridge: SetContext, DrawBatch, etc.)
    // The simulation always executes; rendering is conditionally skipped
    // based on nth_frame skip counter, CHECK_ABORT/CHECK_LOAD state.
    let _ = clear; // Used by C's ClearDrawable in render path
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
        unsafe {
            let mut dl = DisplayList::with_default_capacity();
            let h = alloc_element(&mut dl).unwrap();
            dl.push_back(h);
            {
                let elem = dl.get_mut(h).unwrap();
                elem.life_span = 0;
                elem.state_flags = ElementFlags::PRE_PROCESS;
            }
            pre_process(h, &mut dl);
            let elem = dl.get(h).unwrap();
            assert!(elem.state_flags.contains(ElementFlags::DISAPPEARING));
        }
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

    // -- P04: ProcessCollisions tests --

    /// No-op DrawablesIntersect that returns 0 (no collision)
    unsafe fn intersect_never(_a: &Element, _b: &Element, _min: TimeValue) -> TimeValue {
        0
    }

    /// No-op do_damage
    unsafe fn damage_noop(_e: *mut Element, _d: u16) {}

    /// No-op collide
    unsafe fn collide_noop(_a: *mut Element, _b: *mut Element) {}

    fn make_collision_ctx(dl: &mut DisplayList) -> CollisionContext<'_> {
        CollisionContext {
            display_list: dl,
            drawables_intersect: intersect_never,
            do_damage: damage_noop,
            collide: collide_noop,
            process_flags: ElementFlags::PRE_PROCESS,
        }
    }

    #[test]
    fn test_process_collisions_no_elements() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();
        dl.push_back(h);
        let elem_ptr = dl.get_mut(h).map(|e| e as *mut Element).unwrap();
        let mut ctx = make_collision_ctx(&mut dl);
        unsafe {
            let result = process_collisions(&mut ctx, None, elem_ptr, MAX_TIME_VALUE);
            assert!(!result);
        }
    }

    #[test]
    fn test_process_collisions_self_skip() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();
        {
            let elem = dl.get_mut(h).unwrap();
            elem.life_span = 10;
            elem.mass_points = 5;
            elem.state_flags = ElementFlags::PRE_PROCESS;
        }
        dl.push_back(h);
        let elem_ptr = dl.get_mut(h).map(|e| e as *mut Element).unwrap();
        let mut ctx = make_collision_ctx(&mut dl);
        unsafe {
            // Passing h as both succ and element — should skip self
            let result = process_collisions(&mut ctx, Some(h), elem_ptr, MAX_TIME_VALUE);
            assert!(!result);
        }
    }

    #[test]
    fn test_process_collisions_no_collision_possible() {
        let mut dl = DisplayList::with_default_capacity();
        let h1 = alloc_element(&mut dl).unwrap();
        let h2 = alloc_element(&mut dl).unwrap();
        // Both elements have zero mass — collision_possible returns false
        {
            let e1 = dl.get_mut(h1).unwrap();
            e1.life_span = 10;
            e1.state_flags = ElementFlags::PRE_PROCESS;
        }
        {
            let e2 = dl.get_mut(h2).unwrap();
            e2.life_span = 10;
            e2.state_flags = ElementFlags::PRE_PROCESS;
        }
        dl.push_back(h1);
        dl.push_back(h2);
        let elem_ptr = dl.get_mut(h1).map(|e| e as *mut Element).unwrap();
        let mut ctx = make_collision_ctx(&mut dl);
        unsafe {
            let result = process_collisions(&mut ctx, Some(h2), elem_ptr, MAX_TIME_VALUE);
            assert!(!result);
        }
    }

    #[test]
    fn test_process_collisions_appearing_finite_life_prefilter() {
        let mut dl = DisplayList::with_default_capacity();
        let h1 = alloc_element(&mut dl).unwrap();
        let h2 = alloc_element(&mut dl).unwrap();
        {
            let e1 = dl.get_mut(h1).unwrap();
            e1.life_span = 10;
            e1.mass_points = 5;
            e1.state_flags = ElementFlags::PRE_PROCESS | ElementFlags::FINITE_LIFE;
        }
        {
            let e2 = dl.get_mut(h2).unwrap();
            e2.life_span = 5; // life_span > 1
            e2.mass_points = 5;
            e2.state_flags =
                ElementFlags::PRE_PROCESS | ElementFlags::APPEARING | ElementFlags::FINITE_LIFE;
        }
        dl.push_back(h1);
        dl.push_back(h2);
        // DrawablesIntersect should NOT be called — prefilter skips it
        unsafe fn intersect_should_not_be_called(
            _a: &Element,
            _b: &Element,
            _min: TimeValue,
        ) -> TimeValue {
            panic!("DrawablesIntersect should not be called for APPEARING+FINITE_LIFE prefilter");
        }
        let elem_ptr = dl.get_mut(h1).map(|e| e as *mut Element).unwrap();
        let mut ctx = CollisionContext {
            display_list: &mut dl,
            drawables_intersect: intersect_should_not_be_called,
            do_damage: damage_noop,
            collide: collide_noop,
            process_flags: ElementFlags::PRE_PROCESS,
        };
        unsafe {
            let result = process_collisions(&mut ctx, Some(h2), elem_ptr, MAX_TIME_VALUE);
            assert!(!result); // time_val = 0, no collision
        }
    }

    #[test]
    fn test_process_collisions_basic_hit() {
        // DrawablesIntersect returns a valid collision time > 1
        unsafe fn intersect_at_5(_a: &Element, _b: &Element, _min: TimeValue) -> TimeValue {
            5
        }
        static mut COLLISION_COUNT: u32 = 0;
        unsafe extern "C" fn counting_collision(
            _self: *mut Element,
            _self_pt: *const Point,
            _other: *mut Element,
            _other_pt: *const Point,
        ) {
            unsafe {
                COLLISION_COUNT += 1;
            }
        }
        let mut dl = DisplayList::with_default_capacity();
        let h1 = alloc_element(&mut dl).unwrap();
        let h2 = alloc_element(&mut dl).unwrap();
        {
            let e1 = dl.get_mut(h1).unwrap();
            e1.life_span = 10;
            e1.mass_points = 5;
            e1.state_flags = ElementFlags::PRE_PROCESS;
            e1.collision_func = Some(counting_collision);
        }
        {
            let e2 = dl.get_mut(h2).unwrap();
            e2.life_span = 10;
            e2.mass_points = 5;
            e2.state_flags = ElementFlags::PRE_PROCESS;
            e2.collision_func = Some(counting_collision);
        }
        dl.push_back(h1);
        dl.push_back(h2);
        unsafe { COLLISION_COUNT = 0 };
        let elem_ptr = dl.get_mut(h1).map(|e| e as *mut Element).unwrap();
        let mut ctx = CollisionContext {
            display_list: &mut dl,
            drawables_intersect: intersect_at_5,
            do_damage: damage_noop,
            collide: collide_noop,
            process_flags: ElementFlags::PRE_PROCESS,
        };
        unsafe {
            let result = process_collisions(&mut ctx, Some(h2), elem_ptr, MAX_TIME_VALUE);
            // Both collision handlers should have been called
            let count = std::ptr::addr_of_mut!(COLLISION_COUNT);
            assert_eq!(*count, 2);
            // Result depends on whether COLLISION flag got set
            // (collision_func needs to set it; our counting_collision doesn't)
            let _ = result;
        }
    }

    #[test]
    fn test_process_collisions_player_ship_dispatch_order() {
        // When test element is PLAYER_SHIP, test's collision_func is called first
        static mut CALL_ORDER: [u8; 2] = [0, 0];
        static mut CALL_IDX: usize = 0;
        unsafe extern "C" fn elem_collision(
            _self: *mut Element,
            _self_pt: *const Point,
            _other: *mut Element,
            _other_pt: *const Point,
        ) {
            CALL_ORDER[CALL_IDX] = 1;
            CALL_IDX += 1;
        }
        unsafe extern "C" fn test_collision(
            _self: *mut Element,
            _self_pt: *const Point,
            _other: *mut Element,
            _other_pt: *const Point,
        ) {
            CALL_ORDER[CALL_IDX] = 2;
            CALL_IDX += 1;
        }
        unsafe fn intersect_at_3(_a: &Element, _b: &Element, _min: TimeValue) -> TimeValue {
            3
        }
        let mut dl = DisplayList::with_default_capacity();
        let h1 = alloc_element(&mut dl).unwrap();
        let h2 = alloc_element(&mut dl).unwrap();
        {
            let e1 = dl.get_mut(h1).unwrap();
            e1.life_span = 10;
            e1.mass_points = 5;
            e1.state_flags = ElementFlags::PRE_PROCESS;
            e1.collision_func = Some(elem_collision);
        }
        {
            let e2 = dl.get_mut(h2).unwrap();
            e2.life_span = 10;
            e2.mass_points = 5;
            e2.state_flags = ElementFlags::PRE_PROCESS | ElementFlags::PLAYER_SHIP;
            e2.collision_func = Some(test_collision);
        }
        dl.push_back(h1);
        dl.push_back(h2);
        unsafe {
            CALL_IDX = 0;
            CALL_ORDER = [0, 0];
        }
        let elem_ptr = dl.get_mut(h1).map(|e| e as *mut Element).unwrap();
        let mut ctx = CollisionContext {
            display_list: &mut dl,
            drawables_intersect: intersect_at_3,
            do_damage: damage_noop,
            collide: collide_noop,
            process_flags: ElementFlags::PRE_PROCESS,
        };
        unsafe {
            let _ = process_collisions(&mut ctx, Some(h2), elem_ptr, MAX_TIME_VALUE);
            // PLAYER_SHIP test element's collision should be called FIRST
            assert_eq!(
                CALL_ORDER[0], 2,
                "test (PLAYER_SHIP) should be called first"
            );
            assert_eq!(CALL_ORDER[1], 1, "element should be called second");
        }
    }

    #[test]
    fn test_find_handle_for_ptr() {
        let mut dl = DisplayList::with_default_capacity();
        let h = alloc_element(&mut dl).unwrap();
        dl.push_back(h);
        let ptr = dl.get(h).unwrap() as *const Element as *mut Element;
        assert_eq!(find_handle_for_ptr(&dl, ptr), Some(h));
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

    #[test]
    fn test_battle_state_init_display_list_step() {
        let mut state = BattleState::new();
        state.zoom_mode = ZoomMode::Step;
        state.init_display_list();
        assert_eq!(state.zoom_out, (MAX_VIS_REDUCTION + 1) as i32);
        assert_eq!(state.opt_max_zoom_out, MAX_VIS_REDUCTION as i32);
    }

    #[test]
    fn test_battle_state_init_display_list_continuous() {
        let mut state = BattleState::new();
        state.zoom_mode = ZoomMode::Continuous;
        state.init_display_list();
        assert_eq!(state.zoom_out, MAX_ZOOM_OUT + (1 << ZOOM_SHIFT));
        assert_eq!(state.opt_max_zoom_out, MAX_ZOOM_OUT);
    }

    #[test]
    fn test_post_process_queue_removes_disappearing() {
        let mut state = BattleState::new();
        let h = state.display_list.alloc().unwrap();
        {
            let elem = state.display_list.get_mut(h).unwrap();
            elem.state_flags =
                ElementFlags::PRE_PROCESS | ElementFlags::POST_PROCESS | ElementFlags::DISAPPEARING;
        }
        state.display_list.push_back(h);
        unsafe {
            post_process_queue(&mut state, ViewState::Stable, 0, 0);
        }
        // Element should be removed
        assert!(state.display_list.head().is_none());
    }

    #[test]
    fn test_redraw_queue_basic_frame() {
        let mut state = BattleState::new();
        // Empty display list: should complete without panic
        unsafe {
            redraw_queue(
                &mut state,
                false,
                intersect_never,
                damage_noop,
                collide_noop,
            );
        }
    }
}
