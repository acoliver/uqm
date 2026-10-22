//! Live engine ABI for race ports. Unlike `types::RaceDesc`, these records can
//! be returned directly to the engine's `init_<race>` callers.

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    clippy::all,
    clashing_extern_declarations,
    reason = "generated C pointer types share ABI with existing opaque-pointer imports"
)]
pub mod abi {
    include!(concat!(env!("OUT_DIR"), "/ship_abi.rs"));
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod engine {
    use super::abi;
    use std::ffi::{c_int, c_void};

    // Keep opaque-pointer declarations identical to existing engine imports.
    extern "C" {
        fn PutQueue(queue: *mut c_void, handle: *mut c_void);
        fn GetFrameIndex(frame: *mut c_void) -> u16;
        fn GetFrameCount(frame: *mut c_void) -> u16;
        fn SetRelFrameIndex(frame: *mut c_void, offset: i16) -> *mut c_void;
        fn rust_bridge_OBJECT_CLOAKED(element: *mut c_void) -> c_int;
    }

    /// # Safety
    /// `handle` must be an allocated element not currently linked in `disp_q`.
    pub unsafe fn put_element(handle: abi::HELEMENT) {
        PutQueue((&raw mut abi::disp_q).cast(), handle);
    }

    /// # Safety
    /// `frame` must be null or a live engine frame.
    pub unsafe fn frame_index(frame: abi::FRAME) -> u16 {
        GetFrameIndex(frame.cast())
    }
    /// # Safety
    /// `frame` must be a live engine frame.
    pub unsafe fn frame_count(frame: abi::FRAME) -> u16 {
        GetFrameCount(frame.cast())
    }

    /// # Safety
    /// `frame` must be null or a live engine frame.
    pub unsafe fn rel_frame_index(frame: abi::FRAME, offset: i16) -> abi::FRAME {
        SetRelFrameIndex(frame.cast(), offset).cast()
    }

    /// # Safety
    /// `element` must be live, with a valid display primitive.
    pub unsafe fn object_cloaked(element: *mut abi::ELEMENT) -> bool {
        rust_bridge_OBJECT_CLOAKED(element.cast()) != 0
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use engine::{frame_count, frame_index, object_cloaked, put_element, rel_frame_index};

/// Queue-table handles are the element addresses, not separate allocations.
///
/// # Safety
/// `handle` must be a live element handle belonging to `disp_q`.
pub unsafe fn element(handle: abi::HELEMENT) -> *mut abi::ELEMENT {
    handle.cast()
}

/// # Safety
/// `element` must be a live ship or weapon with its owning STARSHIP attached.
pub unsafe fn starship(element: *mut abi::ELEMENT) -> *mut abi::STARSHIP {
    (*element).pParent.cast()
}

/// C `ANGLE_TO_FACING`, before the caller applies facing normalization.
pub const fn angle_to_facing(angle: i32) -> i32 {
    (angle + (1 << (abi::CIRCLE_SHIFT - abi::FACING_SHIFT - 1)))
        >> (abi::CIRCLE_SHIFT - abi::FACING_SHIFT)
}

/// C `FACING_TO_ANGLE`.
pub const fn facing_to_angle(facing: i32) -> i32 {
    facing << (abi::CIRCLE_SHIFT - abi::FACING_SHIFT)
}

/// C `NORMALIZE_ANGLE`.
pub const fn normalize_angle(angle: i32) -> i32 {
    angle & (abi::FULL_CIRCLE as i32 - 1)
}

/// C `NORMALIZE_FACING`.
pub const fn normalize_facing(facing: i32) -> u16 {
    (facing & ((1 << abi::FACING_SHIFT) - 1)) as u16
}

/// C `SINE`, including its signed long product and SIZE truncation.
///
/// # Safety
/// The engine sine table must have been initialized.
pub unsafe fn sine(angle: i32, magnitude: i32) -> i16 {
    let value = *(&raw const abi::sinetab)
        .cast::<abi::SIZE>()
        .add(normalize_angle(angle) as usize);
    ((i64::from(value) * i64::from(magnitude)) >> abi::SIN_SHIFT) as i16
}

/// C `COSINE`.
///
/// # Safety
/// The engine sine table must have been initialized.
pub unsafe fn cosine(angle: i32, magnitude: i32) -> i16 {
    sine(angle + abi::QUADRANT as i32, magnitude)
}

/// C WRAP_DELTA, after assignment to SIZE.
pub const fn wrap_delta(delta: i16, extent: i32) -> i16 {
    if (delta as i32).abs() <= extent >> 1 {
        delta
    } else if delta < 0 {
        (extent + delta as i32) as i16
    } else {
        (delta as i32 - extent) as i16
    }
}

/// Initialize collision endpoints using C WORLD_TO_DISPLAY truncation.
///
/// # Safety
/// `e` must point to a live engine element.
pub unsafe fn init_intersect_points(e: *mut abi::ELEMENT) {
    (*e).IntersectControl.IntersectStamp.origin.x = (*e).current.location.x >> abi::ONE_SHIFT;
    (*e).IntersectControl.IntersectStamp.origin.y = (*e).current.location.y >> abi::ONE_SHIFT;
    (*e).IntersectControl.EndPoint.x = (*e).next.location.x >> abi::ONE_SHIFT;
    (*e).IntersectControl.EndPoint.y = (*e).next.location.y >> abi::ONE_SHIFT;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_angle_rounding_and_wrap_boundaries() {
        assert_eq!(angle_to_facing(1), 0);
        assert_eq!(angle_to_facing(2), 1);
        assert_eq!(angle_to_facing(-3), -1);
        assert_eq!(facing_to_angle(15), 60);
        assert_eq!(normalize_facing(-1), 15);
        assert_eq!(normalize_facing(16), 0);
        assert_eq!(normalize_angle(-1), 63);
        assert_eq!(normalize_angle(64), 0);
    }

    #[test]
    fn wrap_delta_preserves_half_extent_and_adjusts_once() {
        for (delta, expected) in [
            (-4097, 4095),
            (-4096, -4096),
            (4096, 4096),
            (4097, -4095),
            (20000, 11808),
            (i16::MIN, -24576),
        ] {
            assert_eq!(wrap_delta(delta, 8192), expected);
        }
    }
}
