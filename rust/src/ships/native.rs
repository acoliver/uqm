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
    /// `element` must be live, with a valid display primitive.
    pub unsafe fn object_cloaked(element: *mut abi::ELEMENT) -> bool {
        rust_bridge_OBJECT_CLOAKED(element.cast()) != 0
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use engine::{frame_index, object_cloaked, put_element};

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
