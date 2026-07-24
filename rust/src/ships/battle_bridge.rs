//! Battle engine FFI bridge for ship behaviors
//!
//! Provides Rust wrappers around C battle engine functions that ships need
//! to call during combat (element manipulation, weapon creation, sound,
//! coordinate math, AI framework).
//!
//! The C wrapper functions live in `sc2/src/uqm/rust_bridge_ships.c` and
//! expose macros as real function symbols Rust can link against.

#[cfg(not(test))]
use std::os::raw::c_int;
use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// Opaque C handle types
// ---------------------------------------------------------------------------

/// HELEMENT — handle to an element in the display queue.
/// C: `typedef HLINK HELEMENT` — pointer-sized.
pub type HElement = usize;

/// FRAME — pointer to a frame descriptor.
pub type Frame = *mut c_void;

/// STARSHIP pointer (opaque to Rust, manipulated through C bridge).
pub type StarShipPtr = *mut c_void;

/// ELEMENT pointer (opaque to Rust for most ship operations).
pub type ElementPtr = *mut c_void;

// ---------------------------------------------------------------------------
// repr(C) structs matching C layout
// ---------------------------------------------------------------------------

/// Matches C `MISSILE_BLOCK` in weapon.h.
///
/// Field types: COORD=i16, ELEMENT_FLAGS=u16, SIZE=i16, COUNT=u16,
/// FRAME*=pointer, fn ptr=pointer, SIZE=i16.
#[repr(C)]
#[derive(Debug)]
pub struct MissileBlock {
    pub cx: i16,
    pub cy: i16,
    pub flags: u16,
    pub sender: i16,
    pub pixoffs: i16,
    pub speed: i16,
    pub hit_points: i16,
    pub damage: i16,
    pub face: u16,
    pub index: u16,
    pub life: u16,
    pub farray: *mut Frame,
    pub preprocess_func: Option<unsafe extern "C" fn(ElementPtr)>,
    pub blast_offs: i16,
}

/// Matches C `LASER_BLOCK` in weapon.h.
///
/// Field types: COORD=i16, ELEMENT_FLAGS=u16, SIZE=i16, COUNT=u16,
/// Color = { r:u8, g:u8, b:u8, a:u8 }.
#[repr(C)]
#[derive(Debug)]
pub struct LaserBlock {
    pub cx: i16,
    pub cy: i16,
    pub ex: i16,
    pub ey: i16,
    pub flags: u16,
    pub sender: i16,
    pub pixoffs: i16,
    pub face: u16,
    pub color: Color,
}

/// Matches C `Color` in gfxlib.h.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Movement state for AI (matches C MOVEMENT_STATE enum).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementState {
    Pursue = 0,
    Avoid = 1,
    Entice = 2,
    NoMovement = 3,
}

/// Matches C `EVALUATE_DESC` in races.h.
///
/// Fields: ELEMENT* ObjectPtr, COUNT facing, COUNT which_turn, MOVEMENT_STATE MoveState.
#[repr(C)]
#[derive(Debug)]
pub struct EvaluateDesc {
    pub object_ptr: ElementPtr,
    pub facing: u16,
    pub which_turn: u16,
    pub move_state: MovementState,
}

/// Indices into ObjectsOfConcern array (matches C defines in intel.h).
pub const ENEMY_SHIP_INDEX: usize = 0;
pub const ENEMY_WEAPON_INDEX: usize = 1;
pub const CREW_OBJECT_INDEX: usize = 2;
pub const ENEMY_CREW_INDEX: usize = 3;
pub const NUM_EVALUATE_DESCS: usize = 4;

// ---------------------------------------------------------------------------
// C bridge extern declarations
// ---------------------------------------------------------------------------

#[cfg(not(test))]
extern "C" {
    // Element operations (wrappers for C macros)
    fn AllocElement() -> HElement;
    fn FreeElement(h: HElement);
    fn rust_bridge_PutElement(h: HElement);
    fn rust_bridge_InsertElement(h: HElement, after: HElement);
    fn rust_bridge_GetHeadElement() -> HElement;
    fn rust_bridge_GetTailElement() -> HElement;
    fn rust_bridge_LockElement(h: HElement, ppe: *mut ElementPtr);
    fn rust_bridge_UnlockElement(h: HElement);
    fn rust_bridge_GetPredElement(e: ElementPtr) -> HElement;
    fn rust_bridge_GetSuccElement(e: ElementPtr) -> HElement;
    fn rust_bridge_GetFrameIndex(f: Frame) -> u16;

    // Weapon creation
    fn rust_bridge_initialize_missile(block: *const MissileBlock) -> HElement;
    fn rust_bridge_initialize_laser(block: *const LaserBlock) -> HElement;

    // AI
    fn rust_bridge_ship_intelligence(ship: ElementPtr, objects: *mut EvaluateDesc, count: u16);

    // Sound
    fn rust_bridge_ProcessSound(sound: usize, source: ElementPtr);
    fn rust_bridge_SetAbsSoundIndex(sounds: usize, index: u16) -> usize;

    // Coordinate conversion
    fn rust_bridge_DISPLAY_TO_WORLD(x: i32) -> i32;
    fn rust_bridge_WORLD_TO_DISPLAY(x: i32) -> i32;
    fn rust_bridge_NORMALIZE_FACING(f: u16) -> u16;
    fn rust_bridge_FACING_TO_ANGLE(f: u16) -> u16;
    fn rust_bridge_SINE(angle: u16, magnitude: i16) -> i32;
    fn rust_bridge_COSINE(angle: u16, magnitude: i16) -> i32;
    fn rust_bridge_ARCTAN(dx: i32, dy: i32) -> u16;
    fn rust_bridge_WRAP_X(x: i32) -> i32;
    fn rust_bridge_WRAP_Y(y: i32) -> i32;

    // Element state checks
    fn rust_bridge_CollidingElement(e: ElementPtr) -> c_int;
    fn rust_bridge_OBJECT_CLOAKED(e: ElementPtr) -> c_int;

    // Energy/crew
    fn rust_bridge_DeltaEnergy(e: ElementPtr, delta: i16) -> c_int;
    fn rust_bridge_DeltaCrew(e: ElementPtr, delta: i16) -> c_int;

    // Starship association
    fn rust_bridge_GetElementStarShip(e: ElementPtr, ss: *mut StarShipPtr);
    fn rust_bridge_SetElementStarShip(e: ElementPtr, ss: StarShipPtr);

    // Misc ship helpers
    fn rust_bridge_TrackShip(e: ElementPtr, pfacing: *mut u16) -> i16;
    fn rust_bridge_Untarget(e: ElementPtr);
    #[expect(
        dead_code,
        reason = "transitional FFI binding not yet wired into Rust weapon-collision path"
    )]
    fn rust_bridge_weapon_collision(
        e0: ElementPtr,
        p0: *mut c_void,
        e1: ElementPtr,
        p1: *mut c_void,
    ) -> HElement;

    // RNG
    fn TFB_Random() -> u32;

    // Frame operations (real C functions)
    fn SetAbsFrameIndex(frame: Frame, index: c_int) -> Frame;
    fn IncFrameIndex(frame: Frame) -> Frame;
    fn GetFrameCount(frame: Frame) -> c_int;

    // Velocity
    fn SetVelocityVector(vel: *mut c_void, magnitude: c_int, facing: c_int, direction: c_int);
}

// ---------------------------------------------------------------------------
// Safe Rust wrappers (production)
// ---------------------------------------------------------------------------

#[cfg(not(test))]
pub mod bridge {
    use super::*;

    pub fn alloc_element() -> Option<HElement> {
        let h = unsafe { AllocElement() };
        if h == 0 {
            None
        } else {
            Some(h)
        }
    }

    pub fn free_element(h: HElement) {
        if h != 0 {
            unsafe { FreeElement(h) }
        }
    }

    pub fn put_element(h: HElement) {
        if h != 0 {
            unsafe { rust_bridge_PutElement(h) }
        }
    }

    pub fn insert_element(h: HElement, after: HElement) {
        if h != 0 {
            unsafe { rust_bridge_InsertElement(h, after) }
        }
    }

    pub fn get_head_element() -> HElement {
        unsafe { rust_bridge_GetHeadElement() }
    }

    pub fn get_tail_element() -> HElement {
        unsafe { rust_bridge_GetTailElement() }
    }

    /// Lock an element handle, returning an opaque pointer.
    ///
    /// # Safety
    /// The returned pointer is only valid until `unlock_element` is called.
    pub unsafe fn lock_element(h: HElement) -> ElementPtr {
        let mut ptr: ElementPtr = std::ptr::null_mut();
        rust_bridge_LockElement(h, &mut ptr);
        ptr
    }

    pub fn unlock_element(h: HElement) {
        if h != 0 {
            unsafe { rust_bridge_UnlockElement(h) }
        }
    }

    #[allow(
        clippy::missing_safety_doc,
        reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
    )]
    pub unsafe fn get_pred_element(e: ElementPtr) -> HElement {
        rust_bridge_GetPredElement(e)
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_succ_element(e: ElementPtr) -> HElement {
        rust_bridge_GetSuccElement(e)
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_frame_index(f: Frame) -> u16 {
        rust_bridge_GetFrameIndex(f)
    }

    pub fn create_missile(block: &MissileBlock) -> Option<HElement> {
        let h = unsafe { rust_bridge_initialize_missile(block as *const MissileBlock) };
        if h == 0 {
            None
        } else {
            Some(h)
        }
    }

    pub fn create_laser(block: &LaserBlock) -> Option<HElement> {
        let h = unsafe { rust_bridge_initialize_laser(block as *const LaserBlock) };
        if h == 0 {
            None
        } else {
            Some(h)
        }
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn call_ship_intelligence(
        ship: ElementPtr,
        objects: &mut [EvaluateDesc; NUM_EVALUATE_DESCS],
        count: u16,
    ) {
        rust_bridge_ship_intelligence(ship, objects.as_mut_ptr(), count);
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn process_sound(sound: usize, source: ElementPtr) {
        rust_bridge_ProcessSound(sound, source);
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_abs_sound_index(sounds: usize, index: u16) -> usize {
        rust_bridge_SetAbsSoundIndex(sounds, index)
    }

    pub fn display_to_world(x: i32) -> i32 {
        unsafe { rust_bridge_DISPLAY_TO_WORLD(x) }
    }

    pub fn world_to_display(x: i32) -> i32 {
        unsafe { rust_bridge_WORLD_TO_DISPLAY(x) }
    }

    pub fn normalize_facing(f: u16) -> u16 {
        unsafe { rust_bridge_NORMALIZE_FACING(f) }
    }

    pub fn facing_to_angle(f: u16) -> u16 {
        unsafe { rust_bridge_FACING_TO_ANGLE(f) }
    }

    pub fn sine(angle: u16, magnitude: i16) -> i32 {
        unsafe { rust_bridge_SINE(angle, magnitude) }
    }

    pub fn cosine(angle: u16, magnitude: i16) -> i32 {
        unsafe { rust_bridge_COSINE(angle, magnitude) }
    }

    pub fn arctan(dx: i32, dy: i32) -> u16 {
        unsafe { rust_bridge_ARCTAN(dx, dy) }
    }

    pub fn wrap_x(x: i32) -> i32 {
        unsafe { rust_bridge_WRAP_X(x) }
    }

    pub fn wrap_y(y: i32) -> i32 {
        unsafe { rust_bridge_WRAP_Y(y) }
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn colliding_element(e: ElementPtr) -> bool {
        rust_bridge_CollidingElement(e) != 0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn object_cloaked(e: ElementPtr) -> bool {
        rust_bridge_OBJECT_CLOAKED(e) != 0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn delta_energy(e: ElementPtr, delta: i16) -> bool {
        rust_bridge_DeltaEnergy(e, delta) != 0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn delta_crew(e: ElementPtr, delta: i16) -> bool {
        rust_bridge_DeltaCrew(e, delta) != 0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_element_starship(e: ElementPtr) -> StarShipPtr {
        let mut ss: StarShipPtr = std::ptr::null_mut();
        rust_bridge_GetElementStarShip(e, &mut ss);
        ss
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_element_starship(e: ElementPtr, ss: StarShipPtr) {
        rust_bridge_SetElementStarShip(e, ss);
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn track_ship(e: ElementPtr, facing: &mut u16) -> i16 {
        rust_bridge_TrackShip(e, facing as *mut u16)
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn untarget(e: ElementPtr) {
        rust_bridge_Untarget(e);
    }

    pub fn random() -> u32 {
        unsafe { TFB_Random() }
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_abs_frame_index(frame: Frame, index: i32) -> Frame {
        SetAbsFrameIndex(frame, index as c_int)
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn inc_frame_index(frame: Frame) -> Frame {
        IncFrameIndex(frame)
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_frame_count(frame: Frame) -> i32 {
        GetFrameCount(frame) as i32
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_velocity_vector(
        vel: *mut c_void,
        magnitude: i32,
        facing: i32,
        direction: i32,
    ) {
        SetVelocityVector(vel, magnitude as c_int, facing as c_int, direction as c_int);
    }
}

// ---------------------------------------------------------------------------
// Test mocks (only compiled for cargo test)
// ---------------------------------------------------------------------------

#[cfg(test)]
pub mod bridge {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

    fn mock_handle() -> HElement {
        NEXT_HANDLE.fetch_add(1, Ordering::Relaxed) as HElement
    }

    pub fn alloc_element() -> Option<HElement> {
        Some(mock_handle())
    }

    pub fn free_element(_h: HElement) {}

    pub fn put_element(_h: HElement) {}

    pub fn insert_element(_h: HElement, _after: HElement) {}

    pub fn get_head_element() -> HElement {
        0
    }

    pub fn get_tail_element() -> HElement {
        0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn lock_element(_h: HElement) -> ElementPtr {
        std::ptr::null_mut()
    }

    pub fn unlock_element(_h: HElement) {}
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_pred_element(_e: ElementPtr) -> HElement {
        0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_succ_element(_e: ElementPtr) -> HElement {
        0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_frame_index(_f: Frame) -> u16 {
        0
    }

    pub fn create_missile(_block: &MissileBlock) -> Option<HElement> {
        Some(mock_handle())
    }

    pub fn create_laser(_block: &LaserBlock) -> Option<HElement> {
        Some(mock_handle())
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn call_ship_intelligence(
        _ship: ElementPtr,
        _objects: &mut [EvaluateDesc; NUM_EVALUATE_DESCS],
        _count: u16,
    ) {
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn process_sound(_sound: usize, _source: ElementPtr) {}
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_abs_sound_index(sounds: usize, _index: u16) -> usize {
        sounds
    }

    pub fn display_to_world(x: i32) -> i32 {
        x << 1
    }

    pub fn world_to_display(x: i32) -> i32 {
        x >> 1
    }

    pub fn normalize_facing(f: u16) -> u16 {
        f & 0x0F
    }

    pub fn facing_to_angle(f: u16) -> u16 {
        f * (256 / 16)
    }

    pub fn sine(_angle: u16, _magnitude: i16) -> i32 {
        0
    }

    pub fn cosine(_angle: u16, _magnitude: i16) -> i32 {
        0
    }

    pub fn arctan(_dx: i32, _dy: i32) -> u16 {
        0
    }

    pub fn wrap_x(x: i32) -> i32 {
        x
    }

    pub fn wrap_y(y: i32) -> i32 {
        y
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn colliding_element(_e: ElementPtr) -> bool {
        false
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn object_cloaked(_e: ElementPtr) -> bool {
        false
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn delta_energy(_e: ElementPtr, _delta: i16) -> bool {
        true
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn delta_crew(_e: ElementPtr, _delta: i16) -> bool {
        true
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_element_starship(_e: ElementPtr) -> StarShipPtr {
        std::ptr::null_mut()
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_element_starship(_e: ElementPtr, _ss: StarShipPtr) {}
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn track_ship(_e: ElementPtr, _facing: &mut u16) -> i16 {
        0
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn untarget(_e: ElementPtr) {}

    pub fn random() -> u32 {
        42
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_abs_frame_index(_frame: Frame, _index: i32) -> Frame {
        std::ptr::null_mut()
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn inc_frame_index(_frame: Frame) -> Frame {
        std::ptr::null_mut()
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn get_frame_count(_frame: Frame) -> i32 {
        1
    }
    /// # Safety
    ///
    /// This is an FFI function called from C. The caller must ensure pointers are valid.
    pub unsafe fn set_velocity_vector(
        _vel: *mut c_void,
        _magnitude: i32,
        _facing: i32,
        _direction: i32,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Element state flags (matching C defines in element.h)
// ---------------------------------------------------------------------------

pub mod element_flags {
    pub const APPEARING: u16 = 1 << 0;
    pub const DISAPPEARING: u16 = 1 << 1;
    pub const CHANGING: u16 = 1 << 2;
    pub const NONSOLID: u16 = 1 << 3;
    pub const COLLISION: u16 = 1 << 4;
    pub const IGNORE_SIMILAR: u16 = 1 << 5;
    pub const PLAYER_SHIP: u16 = 1 << 6;
    pub const FINITE_LIFE: u16 = 1 << 7;
    pub const CREW_OBJECT: u16 = 1 << 8;
    pub const BACKGROUND_OBJECT: u16 = 1 << 9;
    pub const DEFY_PHYSICS: u16 = 1 << 14;
    pub const GOOD_GUY: u16 = 1 << 15;
}

/// Ship input status flags (matching C defines).
pub mod status_flags {
    pub const THRUST: u32 = 1 << 0;
    pub const LEFT: u32 = 1 << 1;
    pub const RIGHT: u32 = 1 << 2;
    pub const WEAPON: u32 = 1 << 3;
    pub const SPECIAL: u32 = 1 << 4;
    pub const DOWN: u32 = 1 << 5;
    pub const SHIP_AT_MAX_SPEED: u32 = 1 << 6;
    pub const SHIP_BEYOND_MAX_SPEED: u32 = 1 << 7;
    pub const SHIP_IN_GRAVITY_WELL: u32 = 1 << 8;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_element_returns_nonzero() {
        let h = bridge::alloc_element();
        assert!(h.is_some());
        assert_ne!(h.unwrap(), 0);
    }

    #[test]
    fn create_missile_returns_handle() {
        let block = MissileBlock {
            cx: 100,
            cy: 200,
            flags: 0,
            sender: 0,
            pixoffs: 10,
            speed: 40,
            hit_points: 1,
            damage: 4,
            face: 0,
            index: 0,
            life: 60,
            farray: std::ptr::null_mut(),
            preprocess_func: None,
            blast_offs: 8,
        };
        let h = bridge::create_missile(&block);
        assert!(h.is_some());
    }

    #[test]
    fn create_laser_returns_handle() {
        let block = LaserBlock {
            cx: 100,
            cy: 200,
            ex: 50,
            ey: -30,
            flags: 0,
            sender: 0,
            pixoffs: 0,
            face: 0,
            color: Color {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF,
                a: 0xFF,
            },
        };
        let h = bridge::create_laser(&block);
        assert!(h.is_some());
    }

    #[test]
    fn coordinate_conversions() {
        assert_eq!(bridge::display_to_world(100), 200);
        assert_eq!(bridge::world_to_display(200), 100);
        assert_eq!(bridge::normalize_facing(16), 0);
        assert_eq!(bridge::normalize_facing(5), 5);
        assert_eq!(bridge::normalize_facing(0x1F), 0x0F);
    }

    #[test]
    fn element_flag_values() {
        assert_eq!(element_flags::APPEARING, 1);
        assert_eq!(element_flags::PLAYER_SHIP, 64);
        assert_eq!(element_flags::IGNORE_SIMILAR, 32);
    }

    #[test]
    fn status_flag_values() {
        assert_eq!(status_flags::THRUST, 1);
        assert_eq!(status_flags::WEAPON, 8);
        assert_eq!(status_flags::SPECIAL, 16);
    }

    #[test]
    fn evaluate_desc_indices() {
        assert_eq!(ENEMY_SHIP_INDEX, 0);
        assert_eq!(ENEMY_WEAPON_INDEX, 1);
        assert_eq!(NUM_EVALUATE_DESCS, 4);
    }

    #[test]
    fn random_returns_value() {
        let r = bridge::random();
        assert_eq!(r, 42); // mock returns 42
    }
}
