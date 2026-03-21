// Element Type — Core Battle Entity
// @plan PLAN-20260320-BATTLE.P04
// @requirement REQ-BAT-001 through REQ-BAT-042 — Element type, flags, constants

use bitflags::bitflags;

// ---------------------------------------------------------------------------
// Element State Flags
// ---------------------------------------------------------------------------

bitflags! {
    /// Element state flags matching C ELEMENT_FLAGS from element.h
    /// These flags control element lifecycle, collision behavior, and processing.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ElementFlags: u16 {
        const PLAYER_SHIP       = 1 << 2;  // REQ-BAT-014
        const APPEARING         = 1 << 3;  // REQ-BAT-007
        const DISAPPEARING      = 1 << 4;  // REQ-BAT-008
        const CHANGING          = 1 << 5;  // REQ-BAT-015
        const NONSOLID          = 1 << 6;  // REQ-BAT-010
        const COLLISION         = 1 << 7;  // REQ-BAT-009
        const IGNORE_SIMILAR    = 1 << 8;  // REQ-BAT-011
        const DEFY_PHYSICS      = 1 << 9;  // REQ-BAT-016
        const FINITE_LIFE       = 1 << 10; // REQ-BAT-012
        const PRE_PROCESS       = 1 << 11; // REQ-BAT-017
        const POST_PROCESS      = 1 << 12; // REQ-BAT-018
        const IGNORE_VELOCITY   = 1 << 13; // REQ-BAT-019
        const CREW_OBJECT       = 1 << 14; // REQ-BAT-020
        const BACKGROUND_OBJECT = 1 << 15; // REQ-BAT-013
    }
}

// ---------------------------------------------------------------------------
// Visual State Types
// ---------------------------------------------------------------------------

/// Point in 2D space (display coordinates)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    pub const fn new(x: i16, y: i16) -> Self {
        Point { x, y }
    }

    pub const fn zero() -> Self {
        Point { x: 0, y: 0 }
    }
}

/// Extent (width and height) - imported from velocity module
pub use super::velocity::Extent;

/// Frame handle (opaque pointer to graphics frame)
pub type FrameHandle = *mut std::ffi::c_void;

/// Stamp (visual representation with position and frame)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Stamp {
    pub origin: Point,
    pub frame: FrameHandle,
}

/// Intersection control for collision detection
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IntersectControl {
    pub intersect_stamp: Stamp,
    pub end_point: Point,
}

// ---------------------------------------------------------------------------
// Element Visual State
// ---------------------------------------------------------------------------

/// Visual state (current or next frame)
/// Matches C's STATE struct from element.h
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElementVisualState {
    pub location: Point,
    pub frame: FrameHandle,
    pub farray: *mut FrameHandle, // Pointer to frame array
}

impl ElementVisualState {
    pub const fn new() -> Self {
        ElementVisualState {
            location: Point::zero(),
            frame: std::ptr::null_mut(),
            farray: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// VelocityDesc Forward Declaration
// ---------------------------------------------------------------------------

// VelocityDesc is defined in velocity.rs; we'll import it there
// This is just a forward declaration for Element struct
use super::velocity::VelocityDesc;

// ---------------------------------------------------------------------------
// Element Callbacks
// ---------------------------------------------------------------------------

/// Element processing function (preprocess, postprocess, death)
pub type ElementProcessFunc = Option<unsafe extern "C" fn(*mut Element)>;

/// Element collision function
pub type ElementCollisionFunc =
    Option<unsafe extern "C" fn(*mut Element, *const Point, *mut Element, *const Point)>;

// ---------------------------------------------------------------------------
// Link Types (for doubly-linked display list)
// ---------------------------------------------------------------------------

/// Element handle (opaque pointer for display list)
pub type HElement = *mut Element;

// ---------------------------------------------------------------------------
// Element Struct
// ---------------------------------------------------------------------------

/// ELEMENT — The central battle entity type
/// Matches C's ELEMENT struct from element.h exactly
/// #[repr(C)] ensures binary compatibility with C code
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Element {
    // -- Link fields (must be first for QUEUE compatibility) --
    pub pred: HElement,
    pub succ: HElement,

    // -- Callbacks --
    pub preprocess_func: ElementProcessFunc,
    pub postprocess_func: ElementProcessFunc,
    pub collision_func: ElementCollisionFunc,
    pub death_func: ElementProcessFunc,

    // -- Owner identity --
    pub player_nr: i16, // -1=neutral, 0=bottom/human, 1=top/NPC

    // -- State flags --
    pub state_flags: ElementFlags,

    // -- Union: life_span / scan_node --
    // In C this is a union; we use life_span for battle
    pub life_span: u16,

    // -- Union: crew_level / hit_points / facing / cycle --
    // Ships use crew_level; weapons use hit_points
    // We store as u16 and interpret based on context
    pub crew_or_hp: u16,

    // -- Mass --
    pub mass_points: u8,

    // -- Union: turn_wait / sys_loc --
    pub turn_wait: u8,

    // -- Union: thrust_wait / blast_offset / next_turn --
    pub thrust_or_blast: u8,

    // -- Color cycling --
    pub color_cycle_index: u8,

    // -- Velocity --
    pub velocity: VelocityDesc,

    // -- Collision control --
    pub intersect_control: IntersectControl,

    // -- Display primitive index --
    pub prim_index: u16,

    // -- Current and next state --
    pub current: ElementVisualState,
    pub next: ElementVisualState,

    // -- Parent reference (pointer to StarShip or other owner) --
    pub p_parent: *mut std::ffi::c_void,

    // -- Tracking target reference --
    pub h_target: HElement,
}

impl Element {
    /// Creates a new zeroed Element
    /// All fields are initialized to zero/null
    pub fn new() -> Self {
        Element {
            pred: std::ptr::null_mut(),
            succ: std::ptr::null_mut(),
            preprocess_func: None,
            postprocess_func: None,
            collision_func: None,
            death_func: None,
            player_nr: -1, // NEUTRAL_PLAYER_NUM
            state_flags: ElementFlags::empty(),
            life_span: 0,
            crew_or_hp: 0,
            mass_points: 0,
            turn_wait: 0,
            thrust_or_blast: 0,
            color_cycle_index: 0,
            velocity: VelocityDesc::new(),
            intersect_control: IntersectControl {
                intersect_stamp: Stamp {
                    origin: Point::zero(),
                    frame: std::ptr::null_mut(),
                },
                end_point: Point::zero(),
            },
            prim_index: 0,
            current: ElementVisualState::new(),
            next: ElementVisualState::new(),
            p_parent: std::ptr::null_mut(),
            h_target: std::ptr::null_mut(),
        }
    }

    /// Tests if this element belongs to the same player as another
    pub fn is_same_player(&self, other: &Element) -> bool {
        self.player_nr == other.player_nr
    }

    /// Tests if this element is eligible for collision detection
    /// Ineligible if NONSOLID or DISAPPEARING
    pub fn is_collidable(&self) -> bool {
        !self
            .state_flags
            .intersects(ElementFlags::NONSOLID | ElementFlags::DISAPPEARING)
    }

    /// Tests if collision is possible between this element and another
    /// Implements C's CollisionPossible macro from collide.h
    pub fn collision_possible(&self, other: &Element) -> bool {
        // First element must be collidable
        if !self.is_collidable() {
            return false;
        }

        // Both elements must not both have COLLISION flag set
        if self.state_flags.contains(ElementFlags::COLLISION)
            && other.state_flags.contains(ElementFlags::COLLISION)
        {
            return false;
        }

        // If IGNORE_SIMILAR, they must have different parents
        if self.state_flags.contains(ElementFlags::IGNORE_SIMILAR)
            && other.state_flags.contains(ElementFlags::IGNORE_SIMILAR)
            && self.p_parent == other.p_parent
        {
            return false;
        }

        // At least one must have non-zero mass
        if self.mass_points == 0 && other.mass_points == 0 {
            return false;
        }

        true
    }

    /// REQ-BAT-035: PreProcess completion flag transitions
    /// Sets PRE_PROCESS, clears POST_PROCESS and COLLISION
    pub fn mark_preprocessed(&mut self) {
        self.state_flags.insert(ElementFlags::PRE_PROCESS);
        self.state_flags
            .remove(ElementFlags::POST_PROCESS | ElementFlags::COLLISION);
    }

    /// REQ-BAT-036: PostProcess completion flag transitions
    /// Sets POST_PROCESS, clears PRE_PROCESS, CHANGING, APPEARING
    pub fn mark_postprocessed(&mut self) {
        self.state_flags.insert(ElementFlags::POST_PROCESS);
        self.state_flags
            .remove(ElementFlags::PRE_PROCESS | ElementFlags::CHANGING | ElementFlags::APPEARING);
    }

    /// REQ-BAT-037: PostProcessQueue entry (no COLLISION) — clear DEFY_PHYSICS
    pub fn clear_defy_physics_if_no_collision(&mut self) {
        if !self.state_flags.contains(ElementFlags::COLLISION) {
            self.state_flags.remove(ElementFlags::DEFY_PHYSICS);
        }
    }

    /// REQ-BAT-038: PostProcessQueue entry (COLLISION set) — clear COLLISION, retain DEFY_PHYSICS
    pub fn clear_collision_retain_defy_physics(&mut self) {
        if self.state_flags.contains(ElementFlags::COLLISION) {
            self.state_flags.remove(ElementFlags::COLLISION);
            // DEFY_PHYSICS is explicitly retained
        }
    }
}

impl Default for Element {
    fn default() -> Self {
        Element::new()
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// REQ-BAT-039: Standard persistent element life span
pub const NORMAL_LIFE: u16 = 1;

/// REQ-BAT-040: Maximum crew and energy capacity
pub const MAX_CREW_SIZE: u8 = 42;
pub const MAX_ENERGY_SIZE: u8 = 42;

/// REQ-BAT-041: Maximum ship mass
pub const MAX_SHIP_MASS: u8 = 10;

/// REQ-BAT-041: Gravity mass threshold (mass_points >= 100)
/// GRAVITY_MASS(m) = (m > MAX_SHIP_MASS * 10)
pub const fn gravity_mass(mass_points: u8) -> bool {
    (mass_points as u16) > (MAX_SHIP_MASS as u16) * 10
}

/// REQ-BAT-042: Gravity pull distance threshold (display coordinates)
pub const GRAVITY_THRESHOLD: u8 = 255;

/// Neutral player number
pub const NEUTRAL_PLAYER_NUM: i16 = -1;

// For backward compatibility with code that uses raw flag bits
pub const PLAYER_SHIP: u16 = ElementFlags::PLAYER_SHIP.bits();
pub const APPEARING: u16 = ElementFlags::APPEARING.bits();
pub const DISAPPEARING: u16 = ElementFlags::DISAPPEARING.bits();
pub const CHANGING: u16 = ElementFlags::CHANGING.bits();
pub const NONSOLID: u16 = ElementFlags::NONSOLID.bits();
pub const COLLISION_FLAG: u16 = ElementFlags::COLLISION.bits();
pub const IGNORE_SIMILAR: u16 = ElementFlags::IGNORE_SIMILAR.bits();
pub const DEFY_PHYSICS: u16 = ElementFlags::DEFY_PHYSICS.bits();
pub const FINITE_LIFE: u16 = ElementFlags::FINITE_LIFE.bits();
pub const PRE_PROCESS: u16 = ElementFlags::PRE_PROCESS.bits();
pub const POST_PROCESS: u16 = ElementFlags::POST_PROCESS.bits();
pub const IGNORE_VELOCITY: u16 = ElementFlags::IGNORE_VELOCITY.bits();
pub const CREW_OBJECT: u16 = ElementFlags::CREW_OBJECT.bits();
pub const BACKGROUND_OBJECT: u16 = ElementFlags::BACKGROUND_OBJECT.bits();

// ---------------------------------------------------------------------------
// Additional Methods (P05)
// ---------------------------------------------------------------------------

impl Element {
    /// Copies next state → current state
    /// Used at end of each frame to commit the next frame state
    pub fn commit_state(&mut self) {
        self.current = self.next;
    }

    /// Safe accessor for stamp visual (when using STAMP primitive)
    pub fn get_stamp(&self) -> Option<Stamp> {
        // In the C code, the stamp is constructed from current.location + current.image.frame
        if !self.current.frame.is_null() {
            Some(Stamp {
                origin: self.current.location,
                frame: self.current.frame,
            })
        } else {
            None
        }
    }

    /// Safe accessor for next stamp visual
    pub fn get_next_stamp(&self) -> Option<Stamp> {
        if !self.next.frame.is_null() {
            Some(Stamp {
                origin: self.next.location,
                frame: self.next.frame,
            })
        } else {
            None
        }
    }

    /// Safe accessor for line endpoints (when using LINE primitive)
    pub fn get_line_endpoints(&self) -> (Point, Point) {
        (
            self.intersect_control.intersect_stamp.origin,
            self.intersect_control.end_point,
        )
    }

    /// Safe accessor for point (when using POINT primitive)
    pub fn get_point(&self) -> Point {
        self.current.location
    }

    /// Flag transition helpers

    pub fn set_appearing(&mut self) {
        self.state_flags.insert(ElementFlags::APPEARING);
    }

    pub fn clear_appearing(&mut self) {
        self.state_flags.remove(ElementFlags::APPEARING);
    }

    pub fn set_disappearing(&mut self) {
        self.state_flags.insert(ElementFlags::DISAPPEARING);
    }

    pub fn set_collision(&mut self) {
        self.state_flags.insert(ElementFlags::COLLISION);
    }

    pub fn clear_collision(&mut self) {
        self.state_flags.remove(ElementFlags::COLLISION);
    }

    pub fn is_player_ship(&self) -> bool {
        self.state_flags.contains(ElementFlags::PLAYER_SHIP)
    }

    pub fn has_finite_life(&self) -> bool {
        self.state_flags.contains(ElementFlags::FINITE_LIFE)
    }

    pub fn is_crew_object(&self) -> bool {
        self.state_flags.contains(ElementFlags::CREW_OBJECT)
    }

    pub fn is_background_object(&self) -> bool {
        self.state_flags.contains(ElementFlags::BACKGROUND_OBJECT)
    }

    pub fn clear_defy_physics(&mut self) {
        self.state_flags.remove(ElementFlags::DEFY_PHYSICS);
    }

    /// Asymmetric DEFY_PHYSICS clearing as per PostProcessQueue pattern (process.c:827)
    /// If COLLISION is set: clear COLLISION but keep DEFY_PHYSICS
    /// If COLLISION is NOT set: clear DEFY_PHYSICS
    pub fn asymmetric_defy_physics_clear(&mut self) {
        if self.state_flags.contains(ElementFlags::COLLISION) {
            self.state_flags.remove(ElementFlags::COLLISION);
            // DEFY_PHYSICS is explicitly retained
        } else {
            self.state_flags.remove(ElementFlags::DEFY_PHYSICS);
        }
    }

    /// Lifecycle helpers

    /// Decrements life_span, returns true if still alive
    /// Returns false if life_span reaches 0
    pub fn decrement_life(&mut self) -> bool {
        if self.life_span > 0 {
            self.life_span -= 1;
            self.life_span > 0
        } else {
            false
        }
    }

    /// Tests if element is alive
    /// An element is alive if either:
    /// - life_span > 0, OR
    /// - FINITE_LIFE flag is not set (infinite life)
    pub fn is_alive(&self) -> bool {
        self.life_span > 0 || !self.state_flags.contains(ElementFlags::FINITE_LIFE)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Size Assertions (critical for FFI) --

    #[test]
    fn element_flags_is_u16() {
        assert_eq!(std::mem::size_of::<ElementFlags>(), 2);
    }

    #[test]
    fn point_size() {
        assert_eq!(std::mem::size_of::<Point>(), 4); // 2 × i16
    }

    #[test]
    fn extent_size() {
        assert_eq!(std::mem::size_of::<Extent>(), 4); // 2 × i16
    }

    // -- Flag Bit Positions --

    #[test]
    fn element_flags_bit_positions() {
        assert_eq!(ElementFlags::PLAYER_SHIP.bits(), 1 << 2);
        assert_eq!(ElementFlags::APPEARING.bits(), 1 << 3);
        assert_eq!(ElementFlags::DISAPPEARING.bits(), 1 << 4);
        assert_eq!(ElementFlags::CHANGING.bits(), 1 << 5);
        assert_eq!(ElementFlags::NONSOLID.bits(), 1 << 6);
        assert_eq!(ElementFlags::COLLISION.bits(), 1 << 7);
        assert_eq!(ElementFlags::IGNORE_SIMILAR.bits(), 1 << 8);
        assert_eq!(ElementFlags::DEFY_PHYSICS.bits(), 1 << 9);
        assert_eq!(ElementFlags::FINITE_LIFE.bits(), 1 << 10);
        assert_eq!(ElementFlags::PRE_PROCESS.bits(), 1 << 11);
        assert_eq!(ElementFlags::POST_PROCESS.bits(), 1 << 12);
        assert_eq!(ElementFlags::IGNORE_VELOCITY.bits(), 1 << 13);
        assert_eq!(ElementFlags::CREW_OBJECT.bits(), 1 << 14);
        assert_eq!(ElementFlags::BACKGROUND_OBJECT.bits(), 1 << 15);
    }

    // -- Point/Extent Construction --

    #[test]
    fn point_construction() {
        let p = Point::new(100, 200);
        assert_eq!(p.x, 100);
        assert_eq!(p.y, 200);

        let z = Point::zero();
        assert_eq!(z.x, 0);
        assert_eq!(z.y, 0);
    }

    #[test]
    fn extent_construction() {
        let e = Extent::new(320, 240);
        assert_eq!(e.width, 320);
        assert_eq!(e.height, 240);

        let z = Extent::zero();
        assert_eq!(z.width, 0);
        assert_eq!(z.height, 0);
    }

    // -- Collision Eligibility --

    #[test]
    fn is_collidable_normal() {
        let mut elem = Element::new();
        assert!(elem.is_collidable()); // Default: no flags set

        elem.state_flags.insert(ElementFlags::NONSOLID);
        assert!(!elem.is_collidable()); // NONSOLID excludes

        elem.state_flags.remove(ElementFlags::NONSOLID);
        elem.state_flags.insert(ElementFlags::DISAPPEARING);
        assert!(!elem.is_collidable()); // DISAPPEARING excludes
    }

    #[test]
    fn collision_possible_both_have_collision_flag() {
        let mut e0 = Element::new();
        let mut e1 = Element::new();
        e0.mass_points = 1;
        e1.mass_points = 1;

        e0.state_flags.insert(ElementFlags::COLLISION);
        e1.state_flags.insert(ElementFlags::COLLISION);

        assert!(!e0.collision_possible(&e1)); // Both have COLLISION
    }

    #[test]
    fn collision_possible_ignore_similar_same_parent() {
        let mut e0 = Element::new();
        let mut e1 = Element::new();
        e0.mass_points = 1;
        e1.mass_points = 1;

        let parent_ptr = &mut 0u8 as *mut u8 as *mut std::ffi::c_void;
        e0.p_parent = parent_ptr;
        e1.p_parent = parent_ptr;

        e0.state_flags.insert(ElementFlags::IGNORE_SIMILAR);
        e1.state_flags.insert(ElementFlags::IGNORE_SIMILAR);

        assert!(!e0.collision_possible(&e1)); // Same parent, both IGNORE_SIMILAR
    }

    #[test]
    fn collision_possible_zero_mass() {
        let e0 = Element::new();
        let e1 = Element::new();
        // Both have mass_points=0 by default
        assert!(!e0.collision_possible(&e1));
    }

    #[test]
    fn collision_possible_valid() {
        let mut e0 = Element::new();
        let mut e1 = Element::new();
        e0.mass_points = 5;
        e1.mass_points = 3;

        assert!(e0.collision_possible(&e1)); // Valid collision
    }

    // -- Lifecycle Flag Transitions --

    #[test]
    fn mark_preprocessed_transitions() {
        let mut elem = Element::new();
        elem.state_flags
            .insert(ElementFlags::POST_PROCESS | ElementFlags::COLLISION);

        elem.mark_preprocessed();

        assert!(elem.state_flags.contains(ElementFlags::PRE_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::POST_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::COLLISION));
    }

    #[test]
    fn mark_postprocessed_transitions() {
        let mut elem = Element::new();
        elem.state_flags
            .insert(ElementFlags::PRE_PROCESS | ElementFlags::CHANGING | ElementFlags::APPEARING);

        elem.mark_postprocessed();

        assert!(elem.state_flags.contains(ElementFlags::POST_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::PRE_PROCESS));
        assert!(!elem.state_flags.contains(ElementFlags::CHANGING));
        assert!(!elem.state_flags.contains(ElementFlags::APPEARING));
    }

    #[test]
    fn clear_defy_physics_if_no_collision() {
        let mut elem = Element::new();
        elem.state_flags.insert(ElementFlags::DEFY_PHYSICS);

        elem.clear_defy_physics_if_no_collision();

        assert!(!elem.state_flags.contains(ElementFlags::DEFY_PHYSICS));
    }

    #[test]
    fn clear_collision_retain_defy_physics() {
        let mut elem = Element::new();
        elem.state_flags
            .insert(ElementFlags::COLLISION | ElementFlags::DEFY_PHYSICS);

        elem.clear_collision_retain_defy_physics();

        assert!(!elem.state_flags.contains(ElementFlags::COLLISION));
        assert!(elem.state_flags.contains(ElementFlags::DEFY_PHYSICS));
    }

    // -- Constants --

    #[test]
    fn constants_match_c() {
        assert_eq!(NORMAL_LIFE, 1);
        assert_eq!(MAX_CREW_SIZE, 42);
        assert_eq!(MAX_ENERGY_SIZE, 42);
        assert_eq!(MAX_SHIP_MASS, 10);
        assert_eq!(GRAVITY_THRESHOLD, 255);
        assert_eq!(NEUTRAL_PLAYER_NUM, -1);
    }

    #[test]
    fn gravity_mass_threshold() {
        assert!(!gravity_mass(0));
        assert!(!gravity_mass(MAX_SHIP_MASS));
        assert!(!gravity_mass(MAX_SHIP_MASS * 10)); // 100 is NOT gravity mass
        assert!(gravity_mass(MAX_SHIP_MASS * 10 + 1)); // 101 IS gravity mass
        assert!(gravity_mass(255));
    }

    // -- Element Construction --

    #[test]
    fn element_new_default_values() {
        let elem = Element::new();
        assert_eq!(elem.player_nr, NEUTRAL_PLAYER_NUM);
        assert_eq!(elem.state_flags, ElementFlags::empty());
        assert_eq!(elem.life_span, 0);
        assert_eq!(elem.mass_points, 0);
        assert!(elem.pred.is_null());
        assert!(elem.succ.is_null());
    }

    #[test]
    fn element_is_same_player() {
        let mut e0 = Element::new();
        let mut e1 = Element::new();
        let mut e2 = Element::new();

        e0.player_nr = 0;
        e1.player_nr = 0;
        e2.player_nr = 1;

        assert!(e0.is_same_player(&e1));
        assert!(!e0.is_same_player(&e2));
    }

    // -- P05 Tests: Element Methods & Lifecycle --

    #[test]
    fn test_commit_state_copies_next_to_current() {
        let mut elem = Element::new();

        // Set different values for next vs current
        elem.current.location = Point::new(100, 200);
        elem.next.location = Point::new(300, 400);

        // Commit should copy next → current
        elem.commit_state();

        assert_eq!(elem.current.location.x, 300);
        assert_eq!(elem.current.location.y, 400);
        assert_eq!(elem.next.location.x, 300);
        assert_eq!(elem.next.location.y, 400);
    }

    #[test]
    fn test_is_collidable_returns_false_for_nonsolid() {
        let mut elem = Element::new();
        elem.state_flags.insert(ElementFlags::NONSOLID);
        assert!(!elem.is_collidable());
    }

    #[test]
    fn test_is_collidable_returns_false_for_disappearing() {
        let mut elem = Element::new();
        elem.state_flags.insert(ElementFlags::DISAPPEARING);
        assert!(!elem.is_collidable());
    }

    #[test]
    fn test_collision_possible_enforces_ignore_similar_same_parent() {
        let mut e0 = Element::new();
        let mut e1 = Element::new();

        // Both need mass
        e0.mass_points = 5;
        e1.mass_points = 3;

        // Set same parent
        let parent_ptr = &mut 0u8 as *mut u8 as *mut std::ffi::c_void;
        e0.p_parent = parent_ptr;
        e1.p_parent = parent_ptr;

        // Both set IGNORE_SIMILAR
        e0.state_flags.insert(ElementFlags::IGNORE_SIMILAR);
        e1.state_flags.insert(ElementFlags::IGNORE_SIMILAR);

        // Should return false due to same parent + both IGNORE_SIMILAR
        assert!(!e0.collision_possible(&e1));
    }

    #[test]
    fn test_collision_possible_different_parent_ok() {
        let mut e0 = Element::new();
        let mut e1 = Element::new();

        e0.mass_points = 5;
        e1.mass_points = 3;

        // Different parents
        let parent0 = &mut 0u8 as *mut u8 as *mut std::ffi::c_void;
        let parent1 = &mut 1u8 as *mut u8 as *mut std::ffi::c_void;
        e0.p_parent = parent0;
        e1.p_parent = parent1;

        e0.state_flags.insert(ElementFlags::IGNORE_SIMILAR);
        e1.state_flags.insert(ElementFlags::IGNORE_SIMILAR);

        // Should return true since parents differ
        assert!(e0.collision_possible(&e1));
    }

    #[test]
    fn test_asymmetric_defy_physics_clear_collision_set() {
        let mut elem = Element::new();
        elem.state_flags
            .insert(ElementFlags::COLLISION | ElementFlags::DEFY_PHYSICS);

        elem.asymmetric_defy_physics_clear();

        // COLLISION should be cleared
        assert!(!elem.state_flags.contains(ElementFlags::COLLISION));
        // DEFY_PHYSICS should be retained
        assert!(elem.state_flags.contains(ElementFlags::DEFY_PHYSICS));
    }

    #[test]
    fn test_asymmetric_defy_physics_clear_collision_not_set() {
        let mut elem = Element::new();
        elem.state_flags.insert(ElementFlags::DEFY_PHYSICS);
        // COLLISION is NOT set

        elem.asymmetric_defy_physics_clear();

        // DEFY_PHYSICS should be cleared
        assert!(!elem.state_flags.contains(ElementFlags::DEFY_PHYSICS));
    }

    #[test]
    fn test_decrement_life_zero_life() {
        let mut elem = Element::new();
        elem.life_span = 0;

        assert!(!elem.decrement_life());
        assert_eq!(elem.life_span, 0);
    }

    #[test]
    fn test_decrement_life_one_life() {
        let mut elem = Element::new();
        elem.life_span = 1;

        assert!(!elem.decrement_life());
        assert_eq!(elem.life_span, 0);
    }

    #[test]
    fn test_decrement_life_normal_life() {
        let mut elem = Element::new();
        elem.life_span = NORMAL_LIFE;

        assert!(!elem.decrement_life());
        assert_eq!(elem.life_span, 0);
    }

    #[test]
    fn test_decrement_life_multiple() {
        let mut elem = Element::new();
        elem.life_span = 5;

        assert!(elem.decrement_life());
        assert_eq!(elem.life_span, 4);
        assert!(elem.decrement_life());
        assert_eq!(elem.life_span, 3);
    }

    #[test]
    fn test_is_alive_with_positive_life() {
        let mut elem = Element::new();
        elem.life_span = 10;
        elem.state_flags.insert(ElementFlags::FINITE_LIFE);

        assert!(elem.is_alive());
    }

    #[test]
    fn test_is_alive_with_zero_life_and_finite_life() {
        let mut elem = Element::new();
        elem.life_span = 0;
        elem.state_flags.insert(ElementFlags::FINITE_LIFE);

        assert!(!elem.is_alive());
    }

    #[test]
    fn test_is_alive_with_zero_life_no_finite_life() {
        let mut elem = Element::new();
        elem.life_span = 0;
        // FINITE_LIFE is NOT set

        assert!(elem.is_alive()); // Infinite life
    }

    #[test]
    fn test_flag_transition_helpers() {
        let mut elem = Element::new();

        // Test appearing
        elem.set_appearing();
        assert!(elem.state_flags.contains(ElementFlags::APPEARING));
        elem.clear_appearing();
        assert!(!elem.state_flags.contains(ElementFlags::APPEARING));

        // Test disappearing
        elem.set_disappearing();
        assert!(elem.state_flags.contains(ElementFlags::DISAPPEARING));

        // Test collision
        elem.set_collision();
        assert!(elem.state_flags.contains(ElementFlags::COLLISION));
        elem.clear_collision();
        assert!(!elem.state_flags.contains(ElementFlags::COLLISION));

        // Test clear_defy_physics
        elem.state_flags.insert(ElementFlags::DEFY_PHYSICS);
        elem.clear_defy_physics();
        assert!(!elem.state_flags.contains(ElementFlags::DEFY_PHYSICS));
    }

    #[test]
    fn test_flag_query_helpers() {
        let mut elem = Element::new();

        assert!(!elem.is_player_ship());
        elem.state_flags.insert(ElementFlags::PLAYER_SHIP);
        assert!(elem.is_player_ship());

        assert!(!elem.has_finite_life());
        elem.state_flags.insert(ElementFlags::FINITE_LIFE);
        assert!(elem.has_finite_life());

        assert!(!elem.is_crew_object());
        elem.state_flags.insert(ElementFlags::CREW_OBJECT);
        assert!(elem.is_crew_object());

        assert!(!elem.is_background_object());
        elem.state_flags.insert(ElementFlags::BACKGROUND_OBJECT);
        assert!(elem.is_background_object());
    }

    #[test]
    fn test_safe_accessors() {
        let mut elem = Element::new();

        // Test point accessor
        elem.current.location = Point::new(123, 456);
        let point = elem.get_point();
        assert_eq!(point.x, 123);
        assert_eq!(point.y, 456);

        // Test line endpoints accessor
        elem.intersect_control.intersect_stamp.origin = Point::new(10, 20);
        elem.intersect_control.end_point = Point::new(30, 40);
        let (start, end) = elem.get_line_endpoints();
        assert_eq!(start.x, 10);
        assert_eq!(start.y, 20);
        assert_eq!(end.x, 30);
        assert_eq!(end.y, 40);
    }
}
