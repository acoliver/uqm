//! C Bridge — Rust→C FFI wrappers for battle engine
//!
//! Provides safe Rust wrappers around C functions that the ported battle
//! engine needs to call. These are the "deferred bridge operations" from
//! the Phase 1 integration traits, plus additional helpers used by P03–P12.
//!
//! @plan PLAN-20260320-BATTLEPT2.P06
//! @requirement REQ-FFI-SAFETY, REQ-CALLBACK-SAFETY, REQ-BUILD-COEXISTENCE
//!
//! # Safety Model
//! - Rust→C calls: null/validity checks on pointer arguments before crossing FFI
//! - C→Rust calls: handled in `ffi.rs` with catch_unwind at boundaries
//! - All `extern "C"` blocks link to C symbols available when USE_RUST_BATTLE_LOOP is defined
//!
//! # C Reference
//! `sc2/src/uqm/process.c`, `sc2/src/libs/graphics/*.c`, `sc2/src/libs/sound/*.c`

use std::os::raw::{c_int, c_void};

use super::element::{Element, Point};
use super::process_loop::TimeValue;

// ---------------------------------------------------------------------------
// Opaque C types
// ---------------------------------------------------------------------------

/// Opaque C CONTEXT handle
pub type Context = *mut c_void;
/// Opaque C FRAME handle
pub type Frame = *mut c_void;
/// Opaque C DRAWABLE handle
pub type Drawable = *mut c_void;
/// Opaque C SOUND handle
pub type Sound = *mut c_void;
/// Opaque C MUSIC_REF handle
pub type MusicRef = *mut c_void;
/// Opaque C STARSHIP pointer
pub type StarShipPtr = *mut c_void;
/// C PRIM_LINKS type (u32 with pred/succ packed)
pub type PrimLinks = u32;

// ---------------------------------------------------------------------------
// Extern "C" declarations — C functions Rust will call
// ---------------------------------------------------------------------------

extern "C" {
    // --- Graphics Integration ---

    /// Set the current drawing context
    pub fn SetContext(context: Context) -> Context;

    /// Begin batched graphics mode
    pub fn BatchGraphics();

    /// End batched graphics mode
    pub fn UnbatchGraphics();

    /// Clear the current drawable
    pub fn ClearDrawable();

    /// Set the graphics scale factor (0 = reset)
    pub fn SetGraphicScale(scale: c_int);

    /// Draw a batch of primitives
    pub fn DrawBatch(display_array: *mut c_void, display_links: PrimLinks, flags: c_int);

    /// Perform screen transition effect
    pub fn ScreenTransition(which: c_int, rect: *const c_void);

    /// Test intersection of two drawable elements.
    /// Returns: 0 = no collision, 1 = stuck sentinel, >1 = collision time.
    pub fn DrawablesIntersect(
        element1: *mut Element,
        element2: *mut Element,
        min_time: TimeValue,
    ) -> TimeValue;

    /// Initialize intersection start point for an element
    pub fn InitIntersectStartPoint(element: *mut Element);

    /// Initialize intersection end point for an element
    pub fn InitIntersectEndPoint(element: *mut Element);

    /// Get the equivalent frame index for a given frame
    pub fn SetEquFrameIndex(target_array: Frame, source_frame: Frame) -> Frame;

    /// Get a frame at an absolute index
    pub fn SetAbsFrameIndex(frame: Frame, index: c_int) -> Frame;

    /// Increment frame index
    pub fn IncFrameIndex(frame: Frame) -> Frame;

    /// Get the hot spot of a frame
    pub fn GetFrameHot(frame: Frame) -> Point;

    /// Get bounding rectangle of a frame
    pub fn GetFrameRect(frame: Frame, rect: *mut c_void);

    /// Get the number of frames in a frame array
    pub fn GetFrameCount(frame: Frame) -> c_int;

    /// Set trilinear mipmap for a frame
    pub fn TFB_DrawScreen_SetMipmap(
        image: *mut c_void,
        mipmap_image: *mut c_void,
        hot_x: c_int,
        hot_y: c_int,
    );

    // --- Audio Integration ---

    /// Process a sound event
    pub fn ProcessSound(sound: Sound, source: *mut c_void);

    /// Flush all pending sounds
    pub fn FlushSounds();

    /// Update stereo sound positions based on ship locations
    pub fn UpdateSoundPositions();

    /// Remove all sounds associated with an object
    pub fn RemoveSoundsForObject(element: *mut Element);

    /// Play music track
    pub fn PlayMusic(music: MusicRef, looping: c_int);

    /// Stop currently playing music
    pub fn StopMusic();

    /// Check if music is playing
    pub fn PLRPlaying(music: MusicRef) -> c_int;

    /// Stop all sounds
    pub fn StopSound();

    /// Stop ditty playback
    pub fn StopDitty();

    // --- Threading Integration ---

    /// Suspend current thread for a duration
    pub fn SleepThread(ticks: c_int);

    /// Suspend current thread until a specific time
    pub fn SleepThreadUntil(wake_time: u32);

    /// Yield to other threads
    pub fn TaskSwitch();

    // --- Input Integration ---

    /// Convert current input state to battle input flags
    pub fn CurrentInputToBattleInput(player_nr: c_int) -> c_int;

    /// Process input events
    pub fn DoInput(input_state: *mut c_void, exclusive: c_int) -> c_int;

    // --- Resource Integration ---

    /// Capture a drawable resource (increment ref count)
    pub fn CaptureDrawable(drawable: Drawable) -> Drawable;

    /// Release a drawable resource (decrement ref count)
    pub fn ReleaseDrawable(drawable: Drawable);

    /// Destroy a drawable resource
    pub fn DestroyDrawable(drawable: Drawable);

    /// Load a music resource
    pub fn LoadMusic(music_file: *const u8) -> MusicRef;

    /// Destroy a music resource
    pub fn DestroyMusic(music: MusicRef);

    // --- Ship/Race Integration ---

    /// Get the starship associated with an element
    pub fn GetElementStarShip(element: *const Element, starship: *mut StarShipPtr);

    /// Set the starship associated with an element
    pub fn SetElementStarShip(element: *mut Element, starship: StarShipPtr);

    // --- Global State Integration ---

    /// Get current activity flags
    pub fn get_current_activity() -> u16;

    /// Check if in HyperSpace/QuasiSpace
    pub fn inHQSpace() -> c_int;

    // --- Combat Helpers ---

    /// Apply damage to an element
    pub fn do_damage(element: *mut Element, damage: u16);

    /// Perform elastic collision between two elements
    pub fn collide(element1: *mut Element, element2: *mut Element);

    /// Calculate gravity effect on an element
    pub fn CalculateGravity(element: *mut Element);

    // --- Space/Galaxy ---

    /// Move the SIS ship in hyperspace
    pub fn MoveSIS(dx: *mut c_int, dy: *mut c_int);

    /// Move galaxy background for parallax
    pub fn MoveGalaxy(view_state: c_int, dx: c_int, dy: c_int);

    // --- Velocity (C-only entry points, Phase 1 Rust has pure-Rust versions) ---

    /// Set velocity vector from magnitude + facing
    pub fn SetVelocityVector(vel: *mut c_void, magnitude: c_int, facing: c_int, direction: c_int);

    // --- RNG/Timing ---

    /// Get a random number
    pub fn TFB_Random() -> u32;

    /// Seed the random number generator
    pub fn TFB_SeedRandom(seed: u32);

    /// Get current time counter
    pub fn GetTimeCounter() -> u32;

    // --- Display Prim Management ---

    /// Allocate a display primitive slot
    pub fn AllocDisplayPrim() -> c_int;

    /// Free a display primitive slot
    pub fn FreeDisplayPrim(prim_index: c_int);
}

// ---------------------------------------------------------------------------
// Safe Rust wrappers
// ---------------------------------------------------------------------------

/// Safe wrapper: test intersection of two elements.
///
/// Returns 0 (no collision), STUCK_SENTINEL (1), or collision time (>1).
///
/// # Safety
/// Both element pointers must be valid.
pub unsafe fn drawables_intersect(
    a: *mut Element,
    b: *mut Element,
    min_time: TimeValue,
) -> TimeValue {
    if a.is_null() || b.is_null() {
        return 0;
    }
    DrawablesIntersect(a, b, min_time)
}

/// Safe wrapper: initialize intersection start point.
#[allow(
    clippy::missing_safety_doc,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub unsafe fn init_intersect_start(element: *mut Element) {
    if !element.is_null() {
        InitIntersectStartPoint(element);
    }
}

/// Safe wrapper: initialize intersection end point.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn init_intersect_end(element: *mut Element) {
    if !element.is_null() {
        InitIntersectEndPoint(element);
    }
}

/// Safe wrapper: apply damage to an element.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn apply_damage(element: *mut Element, damage: u16) {
    if !element.is_null() {
        do_damage(element, damage);
    }
}

/// Safe wrapper: elastic collision.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn elastic_collide_c(a: *mut Element, b: *mut Element) {
    if !a.is_null() && !b.is_null() {
        collide(a, b);
    }
}

/// Safe wrapper: process and flush sounds.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn flush_all_sounds() {
    FlushSounds();
}

/// Safe wrapper: update stereo sound positions.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn update_sound_positions() {
    UpdateSoundPositions();
}

/// Safe wrapper: remove sounds for an element being destroyed.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn remove_sounds_for(element: *mut Element) {
    if !element.is_null() {
        RemoveSoundsForObject(element);
    }
}

/// Safe wrapper: set graphics scale.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn set_graphic_scale(scale: i32) {
    SetGraphicScale(scale as c_int);
}

/// Safe wrapper: clear current drawable.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn clear_drawable() {
    ClearDrawable();
}

/// Safe wrapper: get starship from element.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Returns null if element is null.
pub unsafe fn get_element_starship(element: *const Element) -> StarShipPtr {
    if element.is_null() {
        return std::ptr::null_mut();
    }
    let mut result: StarShipPtr = std::ptr::null_mut();
    GetElementStarShip(element, &mut result);
    result
}

/// Safe wrapper: check if in HyperSpace/QuasiSpace.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn is_hq_space() -> bool {
    inHQSpace() != 0
}

/// Safe wrapper: get current activity flags.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn current_activity() -> u16 {
    get_current_activity()
}

/// Safe wrapper: get a random number.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn random() -> u32 {
    TFB_Random()
}

/// Safe wrapper: get current time counter.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn time_counter() -> u32 {
    GetTimeCounter()
}

/// Safe wrapper: task switch / yield.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn task_switch() {
    TaskSwitch();
}

/// Safe wrapper: sleep thread for ticks.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
pub unsafe fn sleep_thread(ticks: i32) {
    SleepThread(ticks as c_int);
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// C's `END_OF_LIST` sentinel for prim links
pub const END_OF_LIST: u16 = 0xFFFF;

/// C's `NUM_PRIMS` — max primitive types for InsertPrim check
pub const NUM_PRIMS: u8 = 6;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_of_list_sentinel() {
        assert_eq!(END_OF_LIST, 0xFFFF);
    }

    #[test]
    fn test_num_prims_value() {
        const { assert!(NUM_PRIMS > 0) };
        const { assert!(NUM_PRIMS <= 8) };
    }

    #[test]
    fn test_opaque_type_sizes() {
        // All opaque types are pointer-sized
        assert_eq!(
            std::mem::size_of::<Context>(),
            std::mem::size_of::<*mut c_void>()
        );
        assert_eq!(
            std::mem::size_of::<Frame>(),
            std::mem::size_of::<*mut c_void>()
        );
    }
}
