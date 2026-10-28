// Gravity System — Planetary/asteroid gravitational attraction
// Port of C gravity.c. Uses C bridge for queue head + runtime-dependent
// WRAP_DELTA macros. Element iteration via #[repr(C)] field access.

use super::battle_types::{cosine, sine};
use super::element::{Element, ElementFlags, HElement};
use crate::math::arctan;

use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// C bridge externs
// ---------------------------------------------------------------------------

use crate::graphics::geometry_ffi::DrawablesIntersect;

use crate::graphics::pixmap_abi::SetEquFrameIndex;

extern "C" {
    #[cfg(test)]
    fn uqm_get_head_element() -> *mut std::ffi::c_void;
    #[cfg(test)]
    fn uqm_wrap_delta_x(dx: i16) -> i16;
    #[cfg(test)]
    fn uqm_wrap_delta_y(dy: i16) -> i16;
}
/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
unsafe fn uqm_get_head_element() -> HElement {
    (crate::mainloop::main2_abi::uqm_get_head_element()) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
unsafe fn uqm_wrap_delta_x(dx: i16) -> i16 {
    (crate::mainloop::main2_abi::uqm_wrap_delta_x(dx as _)) as _
}

/// # Safety
/// Called on the game thread with the live resources required by the Rust provider.
#[cfg(not(test))]
unsafe fn uqm_wrap_delta_y(dy: i16) -> i16 {
    (crate::mainloop::main2_abi::uqm_wrap_delta_y(dy as _)) as _
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

// C: SHIP_AT_MAX_SPEED (1 << 7) — from races.h
// C: SHIP_IN_GRAVITY_WELL (1 << 8) — from races.h

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

impl Default for ElementQueueIter {
    fn default() -> Self {
        Self::new()
    }
}

impl ElementQueueIter {
    pub fn new() -> Self {
        ElementQueueIter {
            next: unsafe { uqm_get_head_element().cast() },
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
/// # Safety
/// A nonnull `element` must point to a live Element in the initialized
/// game-thread element queue. The caller must serialize queue access.
pub unsafe extern "C" fn CalculateGravity(element: *mut Element) -> c_int {
    if element.is_null() {
        return 0;
    }

    let elem = unsafe { &*element };
    let has_gravity = colliding_element(elem) && gravity_mass(elem.mass_points.wrapping_add(1));

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
            let angle = arctan(dx, dy);
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
                    // StarShipPtr->cur_status_flags is at a fixed offset;
                    // We access it as a raw u16 (StatusFlags is u16).
                    // TODO: Use proper #[repr(C)] StarShip once layout verified.
                    // For now, the C caller still manages these flags via the
                    // process loop's ship update path.
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
/// # Safety
/// A nonnull `element` must point to a live Element. The element queue
/// and drawable system must be initialized and accessed on the game thread.
pub unsafe extern "C" fn TimeSpaceMatterConflict(element: *mut Element) -> c_int {
    if element.is_null() {
        return 0;
    }

    let elem = unsafe { &*element };

    use crate::graphics::geometry_ffi::{IntersectControl, Point, Stamp};

    let mut elem_control = IntersectControl {
        last_time_val: 0,
        EndPoint: Point {
            x: (elem.current.location.x >> ONE_SHIFT),
            y: (elem.current.location.y >> ONE_SHIFT),
        },
        IntersectStamp: Stamp {
            origin: Point {
                x: (elem.current.location.x >> ONE_SHIFT),
                y: (elem.current.location.y >> ONE_SHIFT),
            },
            frame: unsafe {
                (SetEquFrameIndex((*elem.current.farray).cast(), elem.current.frame.cast())).cast()
            },
        },
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
            last_time_val: 0,
            EndPoint: Point {
                x: (test_elem.current.location.x >> ONE_SHIFT),
                y: (test_elem.current.location.y >> ONE_SHIFT),
            },
            IntersectStamp: Stamp {
                origin: Point {
                    x: (test_elem.current.location.x >> ONE_SHIFT),
                    y: (test_elem.current.location.y >> ONE_SHIFT),
                },
                frame: unsafe {
                    (SetEquFrameIndex(
                        (*test_elem.current.farray).cast(),
                        test_elem.current.frame.cast(),
                    ))
                    .cast()
                },
            },
        };

        let hit =
            unsafe { DrawablesIntersect(&mut elem_control, &mut test_control, MAX_TIME_VALUE) };

        if hit != 0 {
            return 1;
        }
    }

    0
}
