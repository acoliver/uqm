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

/// The C-owned live solar-system state, `pSolarSysState` (solarsys.c).
///
/// Every Rust reader of this global goes through [`current`], so the symbol is
/// declared once with the whole bindgen layout of `SOLARSYS_STATE`. C still
/// allocates, initialises and tears the state down; Rust only borrows it, and
/// no field is reached through an offset written by hand.
///
/// The declaration exists where a consumer exists: the game build, and the
/// unit-test build of the `linked_c_archive` feature. The plain unit-test
/// build has no C archive and no consumer.
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod solar_system {
    use super::global_state::SOLARSYS_STATE;

    extern "C" {
        #[link_name = "pSolarSysState"]
        static mut P_SOLAR_SYSTEM_STATE: *mut SOLARSYS_STATE;
    }

    /// The live `pSolarSysState`; NULL outside a solar system.
    ///
    /// # Safety
    /// Game thread only: C replaces the pointer while entering and leaving a
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
/// The declaration exists where a consumer exists: the game build, and the
/// unit-test build of the `linked_c_archive` feature.
#[cfg(any(not(test), feature = "linked_c_archive"))]
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

#[cfg(test)]
mod solar_system_declaration_tests {
    use super::global_state::{planet_desc, SOLARSYS_STATE};
    use std::mem::{offset_of, size_of};
    use std::path::{Path, PathBuf};

    fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("source directory is readable") {
            let path = entry.expect("directory entry is readable").path();
            if path.is_dir() {
                rust_sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    /// Two declarations of one C symbol with different types are an ABI clash
    /// that only a lint suppression can hide, so the symbol has one owner and
    /// every reader goes through it.
    #[test]
    fn p_solar_sys_state_is_declared_once_with_the_generated_type() {
        let needle = format!("link_name = \"{}\"", "pSolarSysState");
        let mut sources = Vec::new();
        rust_sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut sources,
        );
        let declaring: Vec<_> = sources
            .iter()
            .filter(|path| {
                std::fs::read_to_string(path)
                    .expect("source file is UTF-8")
                    .contains(&needle)
            })
            .collect();
        assert_eq!(
            declaring,
            [&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/c_bindings.rs")]
        );
        let text = std::fs::read_to_string(declaring[0]).expect("source file is UTF-8");
        assert_eq!(text.matches(&needle).count(), 1);
        assert!(text.contains("static mut P_SOLAR_SYSTEM_STATE: *mut SOLARSYS_STATE;"));
    }

    /// The ship and display queues follow the same rule as `pSolarSysState`:
    /// one declaration, in this file, with the pool type the readers use.
    #[test]
    fn race_q_and_disp_q_are_declared_once_with_the_queue_type() {
        let mut sources = Vec::new();
        rust_sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut sources,
        );
        let this_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/c_bindings.rs");
        for (symbol, declaration) in [
            ("race_q", "static mut RACE_Q: [CQueue; NUM_PLAYERS];"),
            ("disp_q", "static mut DISP_Q: CQueue;"),
        ] {
            let needle = format!("link_name = \"{symbol}\"");
            let declaring: Vec<_> = sources
                .iter()
                .filter(|path| {
                    std::fs::read_to_string(path)
                        .expect("source file is UTF-8")
                        .contains(&needle)
                })
                .collect();
            assert_eq!(declaring, [&this_file], "{symbol}");
            let text = std::fs::read_to_string(&this_file).expect("source file is UTF-8");
            assert_eq!(text.matches(&needle).count(), 1, "{symbol}");
            assert!(text.contains(declaration), "{symbol}");
        }
    }

    /// The prefix views that other modules used to write by hand read these
    /// offsets; the generated layout still puts every one of those fields
    /// there, so the views could be replaced by the generated type.
    #[test]
    fn generated_layout_matches_the_retired_prefix_views() {
        assert_eq!(size_of::<planet_desc>(), 48);
        assert_eq!(offset_of!(planet_desc, data_index), 4);
        assert_eq!(offset_of!(planet_desc, NumPlanets), 5);
        assert_eq!(offset_of!(planet_desc, radius), 6);
        assert_eq!(offset_of!(planet_desc, image), 24);
        assert_eq!(offset_of!(planet_desc, pPrevDesc), 40);

        assert_eq!(size_of::<SOLARSYS_STATE>(), 1304);
        assert_eq!(offset_of!(SOLARSYS_STATE, InIpFlight), 8);
        assert_eq!(offset_of!(SOLARSYS_STATE, WaitIntersect), 12);
        assert_eq!(offset_of!(SOLARSYS_STATE, SunDesc), 16);
        assert_eq!(offset_of!(SOLARSYS_STATE, PlanetDesc), 64);
        assert_eq!(offset_of!(SOLARSYS_STATE, MoonDesc), 832);
        assert_eq!(offset_of!(SOLARSYS_STATE, pBaseDesc), 1024);
        assert_eq!(offset_of!(SOLARSYS_STATE, pOrbitalDesc), 1032);
        assert_eq!(
            offset_of!(SOLARSYS_STATE, SysInfo)
                + offset_of!(super::global_state::SYSTEM_INFO, PlanetInfo)
                + offset_of!(super::global_state::PLANET_INFO, ScanRetrieveMask),
            1124
        );
        assert_eq!(offset_of!(SOLARSYS_STATE, PlanetSideFrame), 1168);
        assert_eq!(
            offset_of!(SOLARSYS_STATE, Orbit)
                + offset_of!(super::global_state::PLANET_ORBIT, TopoZoomFrame),
            1224
        );
    }
}
