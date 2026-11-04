//! Queue allocation and battle lifecycle during the owner migration.
#[cfg(test)]
use super::catalog::free_master_ship_list;
use std::panic::catch_unwind;
type CCount = u16;

/// Initializes the battle subsystem.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `COUNT rust_ships_init(void);`
///
/// Returns the number of players (2 for battle, 1 for hyperspace) on success,
/// 0 on failure.
pub unsafe extern "C" fn rust_ships_init() -> CCount {
    #[cfg(debug_assertions)]
    eprintln!("LIFECYCLE: rust_ships_init entry");

    let result = catch_unwind(|| {
        #[cfg(test)]
        {
            use crate::ships::lifecycle::init_ships;
            let activity = 2u8;
            match init_ships(activity) {
                Ok(num_players) => num_players as CCount,
                Err(_) => 0,
            }
        }

        #[cfg(not(test))]
        {
            use crate::ships::native_adapters::rust_bridge_init_battle_arena;

            // Delegate arena setup entirely to C — this calls the original
            // InitShips() body which handles InitSpace, display list, galaxy,
            // asteroids, planets, hyperspace setup, etc.
            // SAFETY: the lifecycle entry runs on the serialized game thread.
            let num_ships = unsafe { rust_bridge_init_battle_arena() };
            if num_ships <= 0 {
                #[cfg(debug_assertions)]
                eprintln!(
                    "LIFECYCLE: rust_ships_init FAILED (arena returned {})",
                    num_ships
                );
                return 0;
            }

            // Track initialization state on the Rust side
            super::lifecycle::mark_ships_initialized();

            #[cfg(debug_assertions)]
            eprintln!("LIFECYCLE: rust_ships_init exit (num_ships={})", num_ships);

            num_ships as CCount
        }
    })
    .unwrap_or_else(|_| {
        #[cfg(debug_assertions)]
        eprintln!("LIFECYCLE: rust_ships_init PANICKED");
        0
    });

    result
}

/// Uninitializes the battle subsystem.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C: `void rust_ships_uninit(void);`
///
/// Performs complete battle teardown:
/// - Stops audio (StopSound)
/// - Frees space resources (UninitSpace — explosions, blasts, asteroids)
/// - Counts floating crew in display list
/// - Distributes floating crew to survivor's descriptor
/// - Writes back descriptor crew → starship.crew_level
/// - Frees each spawned ship's descriptor via free_ship() → rust_ships_free()
/// - Clears IN_BATTLE from CurrentActivity
/// - Reinits queues for non-IN_ENCOUNTER
///
/// Idempotent: calling twice is safe (idempotence guard checks Rust + C state).
pub unsafe extern "C" fn rust_ships_uninit() {
    #[cfg(debug_assertions)]
    eprintln!("LIFECYCLE: rust_ships_uninit entry");

    let _ = catch_unwind(|| {
        #[cfg(test)]
        {
            // Test mode: defensive catalog cleanup (no C arena to tear down)
            free_master_ship_list();
            super::lifecycle::mark_ships_uninitialized();
        }

        #[cfg(not(test))]
        {
            use crate::ships::native_adapters::uninit_ships as rust_bridge_uninit_ships;

            // H2: Reconcile Rust-side state with C-side state before
            // deciding whether to skip teardown. The Rust flag alone
            // is not authoritative — it can desync from C arena state
            // in failure/partial-init paths.
            let rust_says_initialized = super::lifecycle::is_ships_initialized_for_uninit();

            if !rust_says_initialized {
                // Check C-side: does CurrentActivity still have IN_BATTLE?
                let c_activity = (crate::mainloop::ffi::get_current_activity().0 & 0xFF) as u8;
                // IN_ENCOUNTER is 2, which implies battle context may exist
                let c_might_have_arena = c_activity == 2;

                if c_might_have_arena {
                    // H2: Desync detected — Rust says uninitialized but C
                    // may still have arena resources. Proceed with teardown
                    // to prevent resource leak. C state is authoritative.
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "rust_ships_uninit: WARNING desync detected — \
                         Rust says uninitialized but C activity={:#x} \
                         suggests arena may exist. Proceeding with teardown.",
                        c_activity
                    );
                    // Fall through to teardown below
                } else {
                    // Both Rust and C agree: no arena to tear down.
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "rust_ships_uninit: idempotence guard fired \
                         (Rust=uninitialized, C activity={:#x})",
                        c_activity
                    );
                    return;
                }
            }

            #[cfg(debug_assertions)]
            eprintln!("rust_ships_uninit: beginning teardown");

            // Delegate full teardown to C helper.
            // This performs:
            //   - StopSound()
            //   - UninitSpace() (free explosion/blast/asteroid resources)
            //   - CountCrewElements() (count floating crew in display list)
            //   - Iterate display list: add floating crew to survivor
            //   - Write back descriptor crew -> StarShipPtr->crew_level
            //   - free_ship() for each spawned ship (calls rust_ships_free)
            //   - Clear IN_BATTLE from CurrentActivity
            //   - UpdateShipFragCrew() for IN_ENCOUNTER
            //   - ReinitQueue / FreeHyperspace for non-IN_ENCOUNTER
            //
            // C-side null guards (C3): The C helper guards against null
            // StarShipPtr and null descriptor on each element, so
            // partial-init states are handled safely.
            // SAFETY: the lifecycle entry exclusively owns game-thread arena teardown.
            unsafe { rust_bridge_uninit_ships() };

            // Update Rust-side state tracking
            super::lifecycle::mark_ships_uninitialized();

            #[cfg(debug_assertions)]
            {
                assert!(
                    !super::lifecycle::is_ships_initialized_for_uninit(),
                    "ships_initialized should be false after uninit"
                );
                eprintln!("rust_ships_uninit: teardown complete, state cleared");
            }
        }
    });
}
