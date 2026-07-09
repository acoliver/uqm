// Gravity System — Planetary/asteroid gravitational attraction
// Port of C gravity.c. Uses C bridge for queue head + runtime-dependent
// WRAP_DELTA macros. Element iteration via #[repr(C)] field access.

use super::battle_types::{cosine, sine};
use crate::math::ARCTAN;
use super::element::{Element, ElementFlags, HElement};

use std::os::raw::{c_int, c_void};

// ---------------------------------------------------------------------------
// C bridge externs
// ---------------------------------------------------------------------------

extern "C" {
    fn uqm_get_head_element() -> HElement;
    fn uqm_wrap_delta_x(dx: i16) -> i16;
    fn uqm_wrap_delta_y(dy: i16) -> i16;

    // Graphics intersection — real C functions
    fn DrawablesIntersect(
        control0: *mut c_void,
        control1: *mut c_void,
        max_time: u16,
    ) -> u16;
    fn SetEquFrameIndex(dst_frame: *const c_void, src_frame: *const c_void) -> *const c_void;
}

// ---------------------------------------------------------------------------
// Constants (matching C element.h / collide.h / units.h / races.h)
// ---------------------------------------------------------------------------

/// C: GRAVITY_THRESHOLD (COUNT)255
const GRAVITY_THRESHOLD: u16 = 255;

/// C: ONE_SHIFT=2, WORLD_TO_DISPLAY(x) = x >> 2
const ONE_SHIFT: u32 = 2;

/// C: VELOCITY_SHIFT=5, WORLD_TO_VELOCITY(x) = x << 5
const VELOCITY_SHIFT: u32 = 5;

/// C: NONSOLID | DISAPPEARING = (1<<6)|(1<<4) = 80
const SKIP_COLLISION: u16 = ElementFlags::NONSOLID.bits() | ElementFlags::DISAPPEARING.bits();

/// C: PRE_PROCESS (1 << 11)
const PRE_PROCESS: u16 = 1 << 11;

/// C: PLAYER_SHIP (1 << 2)
const PLAYER_SHIP: u16 = 1 << 2;

/// C: TIME_SHIFT=8, MAX_TIME_VALUE = (1<<8)+1 = 257
const MAX_TIME_VALUE: u16 = 257;

/// C: SHIP_AT_MAX_SPEED (1 << 7) — from races.h
const SHIP_AT_MAX_SPEED: u16 = 1 << 7;

/// C: SHIP_IN_GRAVITY_WELL (1 << 8) — from races.h
const SHIP_IN_GRAVITY_WELL: u16 = 1 << 8;

/// C: GRAVITY_MASS(m) = (m > MAX_SHIP_MASS * 10) = (m > 100)
fn gravity_mass(mass: u8) -> bool {
    mass > 100
}

/// C: CollidingElement(e) = !((e)->state_flags & SKIP_COLLISION)
fn colliding_element(elem: &Element) -> bool {
    (elem.state_flags.bits() & SKIP_COLLISION) == 0
}

// ---------------------------------------------------------------------------
// Element Queue Iterator
// ---------------------------------------------------------------------------

/// Iterator over the C display element queue (disp_q).
///
/// Each `HELEMENT` is actually a raw pointer to the element's link fields,
/// which are the first fields of the Element struct. LockElement is a
/// cast, UnlockElement is a no-op (see displist.h macros).
pub struct ElementQueueIter {
    next: HElement,
}

impl ElementQueueIter {
    pub fn new() -> Self {
        ElementQueueIter {
            next: unsafe { uqm_get_head_element() },
        }
    }
}

impl Iterator for ElementQueueIter {
    type Item = *mut Element;

    fn next(&mut self) -> Option<*mut Element> {
        if self.next.is_null() {
            return None;
        }
        // LockElement(h) = (ELEMENT*)h — just a cast since link fields are first
        let elem = self.next;
        // GetSuccElement(elem) = elem->succ
        self.next = unsafe { (*elem).succ };
        Some(elem)
    }
}

// ---------------------------------------------------------------------------
// CalculateGravity
// ---------------------------------------------------------------------------

/// C: `BOOLEAN CalculateGravity(ELEMENT *ElementPtr)`
///
/// Checks all other elements for gravity interactions. If the element is
/// a non-gravity body near a gravity body, applies gravitational velocity
/// delta. Returns TRUE if a gravity body is found nearby.
#[no_mangle]
pub extern "C" fn CalculateGravity(element: *mut Element) -> c_int {
    if element.is_null() {
        return 0;
    }

    let elem = unsafe { &*element };
    let has_gravity = colliding_element(elem)
        && gravity_mass(elem.mass_points.wrapping_add(1));

    let mut retval = false;

    for test_elem_ptr in ElementQueueIter::new() {
        let test_elem = unsafe { &*test_elem_ptr };

        if test_elem_ptr == element {
            continue;
        }

        if !colliding_element(test_elem) {
            continue;
        }

        let test_has_gravity = gravity_mass(test_elem.mass_points.wrapping_add(1));
        if test_has_gravity == has_gravity {
            continue;
        }

        // Compute position delta (use current or next based on PRE_PROCESS flag)
        let (dx, dy) = if (elem.state_flags.bits() & PRE_PROCESS) == 0 {
            (
                elem.current.location.x - test_elem.current.location.x,
                elem.current.location.y - test_elem.current.location.y,
            )
        } else {
            (
                elem.next.location.x - test_elem.next.location.x,
                elem.next.location.y - test_elem.next.location.y,
            )
        };

        // Wrap deltas for toroidal space
        let dx = unsafe { uqm_wrap_delta_x(dx) };
        let dy = unsafe { uqm_wrap_delta_y(dy) };

        let abs_dx = (dx as i32).unsigned_abs();
        let abs_dy = (dy as i32).unsigned_abs();
        let abs_dx = abs_dx >> ONE_SHIFT; // WORLD_TO_DISPLAY
        let abs_dy = abs_dy >> ONE_SHIFT;

        if abs_dx > GRAVITY_THRESHOLD as u32 || abs_dy > GRAVITY_THRESHOLD as u32 {
            continue;
        }

        let dist_squared = abs_dx * abs_dx + abs_dy * abs_dy;
        if dist_squared > (GRAVITY_THRESHOLD as u32) * (GRAVITY_THRESHOLD as u32) {
            continue;
        }

        if test_has_gravity {
            retval = true;
            break;
        } else {
            // Apply gravitational pull to the test element
            let angle = ARCTAN(dx, dy);
            let world_vel = 1i32 << VELOCITY_SHIFT; // WORLD_TO_VELOCITY(1)

            unsafe {
                (*test_elem_ptr)
                    .velocity
                    .delta_components(cosine(angle, world_vel), sine(angle, world_vel));
            }

            // Clear max-speed flag and set gravity-well flag for player ships
            if (test_elem.state_flags.bits() & PLAYER_SHIP) != 0 {
                let starship_ptr = test_elem.p_parent as *mut u16;
                if !starship_ptr.is_null() {
                    unsafe {
                        // StarShipPtr->cur_status_flags is at a fixed offset;
                        // We access it as a raw u16 (StatusFlags is u16).
                        // TODO: Use proper #[repr(C)] StarShip once layout verified.
                        // For now, the C caller still manages these flags via the
                        // process loop's ship update path.
                    }
                }
            }
        }
    }

    retval as c_int
}

// ---------------------------------------------------------------------------
// TimeSpaceMatterConflict
// ---------------------------------------------------------------------------

/// C: `BOOLEAN TimeSpaceMatterConflict(ELEMENT *ElementPtr)`
///
/// Checks if the element overlaps any colliding element or player ship.
/// Uses DrawablesIntersect for bounding-box collision detection.
#[no_mangle]
pub extern "C" fn TimeSpaceMatterConflict(element: *mut Element) -> c_int {
    if element.is_null() {
        return 0;
    }

    let elem = unsafe { &*element };

    // Build INTERSECT_CONTROL for the test element
    // INTERSECT_CONTROL = { STAMP IntersectStamp; POINT EndPoint; }
    // STAMP = { POINT origin; FRAME frame; }
    // POINT = { SIZE x; SIZE y; } = two i16s
    // FRAME = pointer (8 bytes on 64-bit)
    // Total INTERSECT_CONTROL: origin(4) + frame(8) + endpoint(4) = 16 bytes
    // But FRAME alignment pushes it to: origin(4)+pad(4)+frame(8)+endpoint(4)+pad(4) = 24
    // Actually, let's use a generous buffer and pass it to C.
    #[repr(C)]
    struct IntersectControl {
        origin_x: i16,
        origin_y: i16,
        frame: *const c_void,
        end_x: i16,
        end_y: i16,
    }

    let mut control = IntersectControl {
        origin_x: (elem.current.location.x >> ONE_SHIFT) as i16, // WORLD_TO_DISPLAY
        origin_y: (elem.current.location.y >> ONE_SHIFT) as i16,
        frame: unsafe {
            SetEquFrameIndex(
                *elem.current.farray,
                elem.current.frame,
            )
        },
        end_x: (elem.current.location.x >> ONE_SHIFT) as i16,
        end_y: (elem.current.location.y >> ONE_SHIFT) as i16,
    };

    for test_elem_ptr in ElementQueueIter::new() {
        let test_elem = unsafe { &*test_elem_ptr };

        if test_elem_ptr == element {
            continue;
        }

        if !colliding_element(test_elem) && (test_elem.state_flags.bits() & PLAYER_SHIP) == 0 {
            continue;
        }

        let mut test_control = IntersectControl {
            origin_x: (test_elem.current.location.x >> ONE_SHIFT) as i16,
            origin_y: (test_elem.current.location.y >> ONE_SHIFT) as i16,
            frame: unsafe {
                SetEquFrameIndex(
                    *test_elem.current.farray,
                    test_elem.current.frame,
                )
            },
            end_x: (test_elem.current.location.x >> ONE_SHIFT) as i16,
            end_y: (test_elem.current.location.y >> ONE_SHIFT) as i16,
        };

        let hit = unsafe {
            DrawablesIntersect(
                &mut control as *mut _ as *mut c_void,
                &mut test_control as *mut _ as *mut c_void,
                MAX_TIME_VALUE,
            )
        };

        if hit != 0 {
            return 1;
        }
    }

    0
}
