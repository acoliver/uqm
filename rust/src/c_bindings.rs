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
    include!("abi/controller_input_abi.rs");
}

/// Rust-owned game, clock, velocity and event ABI schemas.
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
    include!("abi/global_state_abi.rs");
}

/// The live solar-system state exported by `planets::solarsys` as `pSolarSysState`.
///
/// Every Rust reader of this global goes through [`current`], so the symbol is
/// declared once with the whole bindgen layout of `SOLARSYS_STATE`. The solar
/// system lifecycle owns its stack storage; other modules only borrow it, and
/// no field is reached through an offset written by hand.
///
/// The declaration is available to the running game and integration tests;
/// isolated unit tests do not own a live solar system.
#[cfg(not(test))]
pub mod solar_system {
    use super::global_state::SOLARSYS_STATE;

    extern "C" {
        #[link_name = "pSolarSysState"]
        static mut P_SOLAR_SYSTEM_STATE: *mut SOLARSYS_STATE;
    }

    /// The live `pSolarSysState`; NULL outside a solar system.
    ///
    /// # Safety
    /// Game thread only: the lifecycle replaces the pointer while entering and leaving a
    /// solar system. The caller may dereference the result only while it knows
    /// the state is not being torn down.
    pub unsafe fn current() -> *mut SOLARSYS_STATE {
        std::ptr::addr_of!(P_SOLAR_SYSTEM_STATE).read()
    }
}

/// The C-owned ship and display queues of the running battle or flight:
/// `race_q` (setup.c, one `STARSHIP` pool per side) and `disp_q` (setup.c, the
/// `ELEMENT` pool of everything on screen).
///
/// Both are `QUEUE_TABLE` pools that C initialises with `InitQueue`; Rust only
/// borrows them, through [`player_ships`] and [`display_list`], so each symbol
/// is declared once, here. What the slots hold is checked by the reader, which
/// walks the queue against its pool before it follows a link.
///
/// The declaration is available to the running game and integration tests.
#[cfg(not(test))]
pub mod live_queues {
    use crate::battle::battle_controls::NUM_PLAYERS;
    use crate::comm::locdata::CQueue;

    extern "C" {
        #[link_name = "race_q"]
        static mut RACE_Q: [CQueue; NUM_PLAYERS];
        #[link_name = "disp_q"]
        static mut DISP_Q: CQueue;
    }

    /// `&race_q[player]`, the `STARSHIP` queue of one side (`RPG_PLAYER_NUM` is
    /// the human player's).
    ///
    /// # Safety
    /// Game thread only. The caller may read the queue only while C is not
    /// changing it.
    pub unsafe fn player_ships(player: usize) -> *const CQueue {
        assert!(player < NUM_PLAYERS, "race_q has no side {player}");
        std::ptr::addr_of!(RACE_Q).cast::<CQueue>().add(player)
    }

    /// `&disp_q`, the queue of every live `ELEMENT`.
    ///
    /// # Safety
    /// Game thread only. The caller may read the queue only while C is not
    /// changing it.
    pub unsafe fn display_list() -> *const CQueue {
        std::ptr::addr_of!(DISP_Q)
    }
}

/// Fail the build when a hand-written `#[repr(C)]` mirror disagrees with the
/// Rust-owned ABI schema: total size, alignment, and for each
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
