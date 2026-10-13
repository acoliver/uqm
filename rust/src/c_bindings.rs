/* FFI Bindings for Phase 0 */

// Re-export libc types for convenience
pub use libc::{c_char, c_int};

#[allow(
    clippy::all,
    dead_code,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
pub mod controller_input {
    include!(concat!(env!("OUT_DIR"), "/controller_input_abi.rs"));
}

/// Bindgen's reading of `globdata.h`, `clock.h`, `velocity.h` and `gameev.h`.
///
/// Only layout is taken from here. The Rust mirrors in `comm::locdata` and
/// `time::clock_bridge` check themselves against it at compile time.
#[allow(
    clippy::all,
    dead_code,
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals
)]
pub mod global_state {
    include!(concat!(env!("OUT_DIR"), "/global_state_abi.rs"));
}

/// Fail the build when a hand-written `#[repr(C)]` mirror disagrees with the
/// bindgen reading of the same C struct: total size, alignment, and for each
/// listed field its offset and size.
///
/// Every field of the mirror that code reads or writes must be listed, so a
/// wrong type or a moved field is a compile error and not silent corruption.
macro_rules! assert_c_layout {
    ($mirror:ty, $c:ty { $($field:ident : $c_field:ident),* $(,)? }) => {
        const _: () = {
            const fn size_of_pointee<F>(_: *const F) -> usize {
                ::core::mem::size_of::<F>()
            }
            assert!(::core::mem::size_of::<$mirror>() == ::core::mem::size_of::<$c>());
            assert!(::core::mem::align_of::<$mirror>() == ::core::mem::align_of::<$c>());
            $(
                assert!(
                    ::core::mem::offset_of!($mirror, $field)
                        == ::core::mem::offset_of!($c, $c_field)
                );
                let mirror = ::core::mem::MaybeUninit::<$mirror>::uninit();
                let c = ::core::mem::MaybeUninit::<$c>::uninit();
                // SAFETY: only the address of a field is computed; nothing is read.
                assert!(
                    size_of_pointee(unsafe { ::core::ptr::addr_of!((*mirror.as_ptr()).$field) })
                        == size_of_pointee(unsafe {
                            ::core::ptr::addr_of!((*c.as_ptr()).$c_field)
                        })
                );
            )*
        };
    };
}
pub(crate) use assert_c_layout;

extern "C" {
    static mut PlayerControls:
        [controller_input::CONTROL_TEMPLATE; crate::battle::battle_controls::NUM_PLAYERS];
}

/// Read one control-template selector without creating a reference to the C global.
///
/// # Safety
///
/// The caller must run on the game thread at a point where C cannot mutate
/// `PlayerControls` concurrently.
pub unsafe fn player_control_template(player: usize) -> Option<controller_input::CONTROL_TEMPLATE> {
    if player >= crate::battle::battle_controls::NUM_PLAYERS {
        return None;
    }
    let controls = std::ptr::addr_of!(PlayerControls).cast::<controller_input::CONTROL_TEMPLATE>();
    Some(unsafe { controls.add(player).read_volatile() })
}
