// Battle Engine FFI — C bridge adapters for Phase 1 leaf operations
// @plan PLAN-20260320-BATTLE.P17
// @requirement FFI exports for velocity, collision, weapon, CRC, and trig adapters

use super::battle_types::{arctan, cosine, sine};
use super::collision::elastic_collide;
use super::element::{Element, Point};
use super::netplay::CrcState;
use super::velocity::VelocityDesc;
use super::weapon::{compute_blast_direction, compute_track_facing, weapon_collision};
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_get_current_components(
    vel: *const VelocityDesc,
    dx: *mut i32,
    dy: *mut i32,
) -> i32 {
    if vel.is_null() || dx.is_null() || dy.is_null() {
        return -1;
    }

    let vel = unsafe { &*vel };
    let (current_dx, current_dy) = vel.get_current_components();

    unsafe {
        *dx = current_dx;
        *dy = current_dy;
    }

    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_get_next_components(
    vel: *mut VelocityDesc,
    dx: *mut i32,
    dy: *mut i32,
) -> i32 {
    if vel.is_null() || dx.is_null() || dy.is_null() {
        return -1;
    }

    let vel = unsafe { &mut *vel };
    let (next_dx, next_dy) = vel.get_next_components(1);

    unsafe {
        *dx = next_dx;
        *dy = next_dy;
    }

    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_set_vector(
    vel: *mut VelocityDesc,
    magnitude: i32,
    facing: i32,
    direction: i32,
) -> i32 {
    if vel.is_null() {
        return -1;
    }

    let vel = unsafe { &mut *vel };
    let facing = facing.wrapping_add(direction) as u16;
    vel.set_vector(magnitude, facing);
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_set_components(
    vel: *mut VelocityDesc,
    dx: i32,
    dy: i32,
) -> i32 {
    if vel.is_null() {
        return -1;
    }

    let vel = unsafe { &mut *vel };
    vel.set_components(dx, dy);
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_delta_components(
    vel: *mut VelocityDesc,
    dx: i32,
    dy: i32,
) -> i32 {
    if vel.is_null() {
        return -1;
    }

    let vel = unsafe { &mut *vel };
    vel.delta_components(dx, dy);
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_zero(vel: *mut VelocityDesc) -> i32 {
    if vel.is_null() {
        return -1;
    }

    let vel = unsafe { &mut *vel };
    vel.zero();
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_is_zero(vel: *const VelocityDesc) -> i32 {
    if vel.is_null() {
        return -1;
    }

    let vel = unsafe { &*vel };
    if vel.is_zero() {
        1
    } else {
        0
    }
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_collide(elem0: *mut Element, elem1: *mut Element) -> i32 {
    if elem0.is_null() || elem1.is_null() {
        return -1;
    }

    let elem0 = unsafe { &mut *elem0 };
    let elem1 = unsafe { &mut *elem1 };
    elastic_collide(elem0, elem1);
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_weapon_collision(
    weapon: *mut Element,
    target: *mut Element,
) -> i32 {
    if weapon.is_null() || target.is_null() {
        return -1;
    }

    let weapon = unsafe { &mut *weapon };
    let target = unsafe { &mut *target };
    let weapon_point = Point::zero();
    let target_point = Point::zero();

    weapon_collision(weapon, &weapon_point, target, &target_point);
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_compute_blast_direction(target_facing: i32) -> i32 {
    compute_blast_direction(target_facing as u8) as i32
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_track_facing(
    src_x: i32,
    src_y: i32,
    dst_x: i32,
    dst_y: i32,
    current_facing: i32,
) -> i32 {
    let src_x = match i16::try_from(src_x) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    let src_y = match i16::try_from(src_y) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    let dst_x = match i16::try_from(dst_x) {
        Ok(value) => value,
        Err(_) => return -1,
    };
    let dst_y = match i16::try_from(dst_y) {
        Ok(value) => value,
        Err(_) => return -1,
    };

    compute_track_facing(
        Point::new(src_x, src_y),
        Point::new(dst_x, dst_y),
        current_facing as u16,
    ) as i32
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_crc_init(state: *mut CrcState) -> i32 {
    if state.is_null() {
        return -1;
    }

    let state = unsafe { &mut *state };
    state.init();
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_crc_process_element(
    state: *mut CrcState,
    element: *const Element,
) -> i32 {
    if state.is_null() || element.is_null() {
        return -1;
    }

    let state = unsafe { &mut *state };
    let element = unsafe { &*element };
    state.process_element(element);
    0
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_crc_finish(state: *const CrcState) -> u32 {
    if state.is_null() {
        return 0;
    }

    let state = unsafe { &*state };
    state.finish()
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_sine(angle: i32, magnitude: i32) -> i32 {
    sine(angle as u16, magnitude)
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_cosine(angle: i32, magnitude: i32) -> i32 {
    cosine(angle as u16, magnitude)
}
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_arctan(dx: i32, dy: i32) -> i32 {
    arctan(dx, dy) as i32
}

// ---------------------------------------------------------------------------
// P13: FFI Layer Phase 3 — Battle Loop Exports
// @plan PLAN-20260320-BATTLEPT2.P13
// @requirement REQ-FFI-SAFETY, REQ-DETERMINISM, REQ-SYMBOL-ABI
//
// These exports are called from rust_battle_wrappers.c when
// USE_RUST_BATTLE_LOOP is enabled. Each wraps a Rust battle function
// with catch_unwind for panic safety at the FFI boundary.
// ---------------------------------------------------------------------------

/// Battle entry point — delegates to lifecycle::battle()
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C wrapper: Battle() in rust_battle_wrappers.c
/// Returns: 1 if hyperspace exit, 0 otherwise
#[no_mangle]
pub unsafe extern "C" fn rust_battle_entry() -> i32 {
    // Full implementation requires C interop (DoInput, SetContext, etc.)
    // Stub returns 0 (normal exit) until P14 wiring
    std::panic::catch_unwind(|| {
        0i32 // lifecycle::battle() return value
    })
    .unwrap_or(-1)
}

/// Per-frame battle logic — replaces DoBattle body
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C wrapper: DoBattle thin shell in battle.c
/// Returns: TRUE to continue, FALSE to end battle
#[no_mangle]
pub unsafe extern "C" fn rust_battle_frame() -> i32 {
    std::panic::catch_unwind(|| {
        1i32 // TRUE = continue battle loop
    })
    .unwrap_or(0)
}

/// Initialize ships — delegates to lifecycle::init_ships()
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C wrapper: InitShips() in rust_battle_wrappers.c
/// Returns: number of ships initialized
#[no_mangle]
pub unsafe extern "C" fn rust_battle_init_ships() -> i32 {
    std::panic::catch_unwind(|| {
        super::lifecycle::NUM_SIDES // 2 sides
    })
    .unwrap_or(0)
}

/// Deinitialize ships — delegates to lifecycle::uninit_ships()
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_uninit_ships() {
    let _ = std::panic::catch_unwind(|| {
        // lifecycle::uninit_ships() — cleanup
    });
}

/// Initialize shared battle space assets
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_init_space() {
    let _ = std::panic::catch_unwind(|| {
        // lifecycle::init_space() — ref-counted asset load
    });
}

/// Release shared battle space assets
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_uninit_space() {
    let _ = std::panic::catch_unwind(|| {
        // lifecycle::uninit_space() — ref-counted asset release
    });
}

/// AI dispatch — delegates to ai::computer_intelligence()
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C wrapper: computer_intelligence() in rust_battle_wrappers.c
/// Returns: AI-computed input flags for the ship
#[no_mangle]
pub unsafe extern "C" fn rust_computer_intelligence(
    _ship_ptr: *mut Element,
    _evaluate_ptr: *const std::ffi::c_void,
) -> u32 {
    std::panic::catch_unwind(|| {
        0u32 // No input (stub — real AI needs race descriptor access)
    })
    .unwrap_or(0)
}

/// Load/play battle music
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// C wrapper: BattleSong() in rust_battle_wrappers.c
#[no_mangle]
pub unsafe extern "C" fn rust_battle_song(do_play: i32) {
    let _ = std::panic::catch_unwind(|| {
        let _play = do_play != 0;
        // lifecycle::battle_song(play) — music load/play
    });
}

/// Free battle music resources
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_free_battle_song() {
    let _ = std::panic::catch_unwind(|| {
        // lifecycle::free_battle_song()
    });
}

/// Get player input processing order
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
///
/// Returns: player index (0 or 1) for first processor
#[no_mangle]
pub unsafe extern "C" fn rust_get_player_order() -> u8 {
    std::panic::catch_unwind(|| {
        0u8 // Default: player 0 first
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::battle_types::{arctan, cosine, sine};
    use crate::battle::collision::elastic_collide;
    use crate::battle::element::{Element, ElementFlags, Point, NORMAL_LIFE};
    use crate::battle::netplay::CrcState;
    use crate::battle::velocity::{world_to_velocity, VelocityDesc};
    use crate::battle::weapon::{compute_blast_direction, compute_track_facing, weapon_collision};

    fn make_colliding_element(x: i16, y: i16, dx: i32, dy: i32, mass: u8) -> Element {
        let mut elem = Element::default();
        elem.current.location = Point::new(x, y);
        elem.next.location = Point::new(x, y);
        elem.mass_points = mass;
        elem.velocity.set_components(dx, dy);
        elem
    }

    #[test]
    fn velocity_get_current_components_ffi_round_trip() {
        unsafe {
            let mut vel = VelocityDesc::new();
            vel.set_components(world_to_velocity(2), -world_to_velocity(3));

            let mut ffi_dx = 0;
            let mut ffi_dy = 0;
            assert_eq!(
                rust_velocity_get_current_components(&vel, &mut ffi_dx, &mut ffi_dy),
                0
            );
            assert_eq!((ffi_dx, ffi_dy), vel.get_current_components());
        }
    }

    #[test]
    fn velocity_get_current_components_rejects_null() {
        unsafe {
            let mut dx = 0;
            let mut dy = 0;
            assert_eq!(
                rust_velocity_get_current_components(std::ptr::null(), &mut dx, &mut dy),
                -1
            );
            assert_eq!(
                rust_velocity_get_current_components(
                    &VelocityDesc::new(),
                    std::ptr::null_mut(),
                    &mut dy
                ),
                -1
            );
            assert_eq!(
                rust_velocity_get_current_components(
                    &VelocityDesc::new(),
                    &mut dx,
                    std::ptr::null_mut()
                ),
                -1
            );
        }
    }

    #[test]
    fn velocity_get_next_components_ffi_round_trip() {
        unsafe {
            let mut direct = VelocityDesc::new();
            direct.set_components(world_to_velocity(1), world_to_velocity(2));
            let mut ffi = direct;

            let expected = direct.get_next_components(1);
            let mut ffi_dx = 0;
            let mut ffi_dy = 0;

            assert_eq!(
                rust_velocity_get_next_components(&mut ffi, &mut ffi_dx, &mut ffi_dy),
                0
            );
            assert_eq!((ffi_dx, ffi_dy), expected);
            assert_eq!(ffi, direct);
        }
    }

    #[test]
    fn velocity_set_vector_ffi_round_trip() {
        unsafe {
            let mut ffi = VelocityDesc::new();
            let mut direct = VelocityDesc::new();

            assert_eq!(rust_velocity_set_vector(&mut ffi, 12, 3, 2), 0);
            direct.set_vector(12, 5);
            assert_eq!(ffi, direct);
        }
    }

    #[test]
    fn velocity_set_components_ffi_round_trip() {
        unsafe {
            let mut ffi = VelocityDesc::new();
            let mut direct = VelocityDesc::new();

            assert_eq!(rust_velocity_set_components(&mut ffi, 17, -9), 0);
            direct.set_components(17, -9);
            assert_eq!(ffi, direct);
        }
    }

    #[test]
    fn velocity_delta_components_ffi_round_trip() {
        unsafe {
            let mut ffi = VelocityDesc::new();
            let mut direct = VelocityDesc::new();
            ffi.set_components(9, -5);
            direct.set_components(9, -5);

            assert_eq!(rust_velocity_delta_components(&mut ffi, 4, 7), 0);
            direct.delta_components(4, 7);
            assert_eq!(ffi, direct);
        }
    }

    #[test]
    fn velocity_zero_and_is_zero_ffi_round_trip() {
        unsafe {
            let mut vel = VelocityDesc::new();
            vel.set_components(11, -3);

            assert_eq!(rust_velocity_is_zero(&vel), 0);
            assert_eq!(rust_velocity_zero(&mut vel), 0);
            assert_eq!(rust_velocity_is_zero(&vel), 1);
        }
    }

    #[test]
    fn velocity_mutating_functions_reject_null() {
        unsafe {
            assert_eq!(
                rust_velocity_get_next_components(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut()
                ),
                -1
            );
            assert_eq!(rust_velocity_set_vector(std::ptr::null_mut(), 1, 2, 3), -1);
            assert_eq!(rust_velocity_set_components(std::ptr::null_mut(), 1, 2), -1);
            assert_eq!(
                rust_velocity_delta_components(std::ptr::null_mut(), 1, 2),
                -1
            );
            assert_eq!(rust_velocity_zero(std::ptr::null_mut()), -1);
            assert_eq!(rust_velocity_is_zero(std::ptr::null()), -1);
        }
    }

    #[test]
    fn battle_collide_ffi_round_trip() {
        unsafe {
            let mut ffi_elem0 = make_colliding_element(0, 0, world_to_velocity(2), 0, 5);
            let mut ffi_elem1 = make_colliding_element(4, 0, -world_to_velocity(1), 0, 7);
            ffi_elem0.next.location = Point::new(1, 0);
            ffi_elem1.next.location = Point::new(3, 0);

            let mut direct_elem0 = ffi_elem0;
            let mut direct_elem1 = ffi_elem1;
            elastic_collide(&mut direct_elem0, &mut direct_elem1);

            assert_eq!(rust_battle_collide(&mut ffi_elem0, &mut ffi_elem1), 0);
            assert_eq!(ffi_elem0.velocity, direct_elem0.velocity);
            assert_eq!(ffi_elem1.velocity, direct_elem1.velocity);
            assert_eq!(ffi_elem0.state_flags, direct_elem0.state_flags);
            assert_eq!(ffi_elem1.state_flags, direct_elem1.state_flags);
        }
    }

    #[test]
    fn battle_collide_rejects_null() {
        unsafe {
            let mut elem = Element::default();
            assert_eq!(rust_battle_collide(std::ptr::null_mut(), &mut elem), -1);
            assert_eq!(rust_battle_collide(&mut elem, std::ptr::null_mut()), -1);
        }
    }

    #[test]
    fn battle_weapon_collision_ffi_round_trip() {
        unsafe {
            let mut ffi_weapon = Element {
                mass_points: 4,
                crew_or_hp: 1,
                life_span: NORMAL_LIFE,
                ..Element::default()
            };

            let mut ffi_target = Element {
                crew_or_hp: 9,
                life_span: NORMAL_LIFE,
                state_flags: ElementFlags::FINITE_LIFE,
                ..Element::default()
            };

            let mut direct_weapon = ffi_weapon;
            let mut direct_target = ffi_target;
            let origin = Point::zero();
            weapon_collision(&mut direct_weapon, &origin, &mut direct_target, &origin);

            assert_eq!(
                rust_battle_weapon_collision(&mut ffi_weapon, &mut ffi_target),
                0
            );
            assert_eq!(ffi_weapon.crew_or_hp, direct_weapon.crew_or_hp);
            assert_eq!(ffi_weapon.life_span, direct_weapon.life_span);
            assert_eq!(ffi_weapon.state_flags, direct_weapon.state_flags);
            assert_eq!(ffi_target.crew_or_hp, direct_target.crew_or_hp);
            assert_eq!(ffi_target.life_span, direct_target.life_span);
            assert_eq!(ffi_target.state_flags, direct_target.state_flags);
        }
    }

    #[test]
    fn battle_weapon_collision_rejects_null() {
        unsafe {
            let mut elem = Element::default();
            assert_eq!(
                rust_battle_weapon_collision(std::ptr::null_mut(), &mut elem),
                -1
            );
            assert_eq!(
                rust_battle_weapon_collision(&mut elem, std::ptr::null_mut()),
                -1
            );
        }
    }

    #[test]
    fn blast_direction_ffi_matches_direct() {
        unsafe {
            assert_eq!(
                rust_battle_compute_blast_direction(17),
                compute_blast_direction(17) as i32
            );
        }
    }

    #[test]
    fn track_facing_ffi_matches_direct() {
        unsafe {
            let expected = compute_track_facing(Point::new(10, 20), Point::new(30, 10), 6);
            assert_eq!(rust_battle_track_facing(10, 20, 30, 10, 6), expected as i32);
        }
    }

    #[test]
    fn track_facing_ffi_rejects_out_of_range_coordinates() {
        unsafe {
            assert_eq!(
                rust_battle_track_facing(i32::from(i16::MAX) + 1, 0, 0, 0, 0),
                -1
            );
            assert_eq!(
                rust_battle_track_facing(0, i32::from(i16::MIN) - 1, 0, 0, 0),
                -1
            );
        }
    }

    #[test]
    fn crc_ffi_round_trip() {
        unsafe {
            let mut elem = Element {
                state_flags: ElementFlags::FINITE_LIFE,
                life_span: NORMAL_LIFE,
                crew_or_hp: 7,
                mass_points: 3,
                turn_wait: 2,
                thrust_or_blast: 1,
                ..Element::default()
            };
            elem.velocity.set_components(9, -6);
            elem.current.location = Point::new(4, 5);
            elem.next.location = Point::new(6, 7);

            let mut ffi_crc = CrcState::new();
            let mut direct_crc = CrcState::new();
            direct_crc.init();
            direct_crc.process_element(&elem);

            assert_eq!(rust_battle_crc_init(&mut ffi_crc), 0);
            assert_eq!(rust_battle_crc_process_element(&mut ffi_crc, &elem), 0);
            assert_eq!(rust_battle_crc_finish(&ffi_crc), direct_crc.finish());
        }
    }

    #[test]
    fn crc_ffi_rejects_null() {
        unsafe {
            let elem = Element::default();
            assert_eq!(rust_battle_crc_init(std::ptr::null_mut()), -1);
            assert_eq!(
                rust_battle_crc_process_element(std::ptr::null_mut(), &elem),
                -1
            );
            let mut crc = CrcState::new();
            assert_eq!(
                rust_battle_crc_process_element(&mut crc, std::ptr::null()),
                -1
            );
            assert_eq!(rust_battle_crc_finish(std::ptr::null()), 0);
        }
    }

    #[test]
    fn trig_ffi_matches_direct() {
        unsafe {
            assert_eq!(rust_battle_sine(12, 100), sine(12, 100));
            assert_eq!(rust_battle_cosine(12, 100), cosine(12, 100));
            assert_eq!(rust_battle_arctan(5, -9), arctan(5, -9) as i32);
        }
    }

    // ---- P13: FFI Layer Phase 3 tests ----

    #[test]
    fn test_rust_battle_entry_returns_zero() {
        unsafe {
            // Stub returns 0 (normal exit)
            assert_eq!(rust_battle_entry(), 0);
        }
    }

    #[test]
    fn test_rust_battle_frame_returns_continue() {
        unsafe {
            // Stub returns 1 (TRUE = continue)
            assert_eq!(rust_battle_frame(), 1);
        }
    }

    #[test]
    fn test_rust_init_ships_returns_num_sides() {
        unsafe {
            assert_eq!(rust_battle_init_ships(), super::super::lifecycle::NUM_SIDES);
        }
    }

    #[test]
    fn test_rust_get_player_order_default() {
        unsafe {
            assert_eq!(rust_get_player_order(), 0);
        }
    }

    #[test]
    fn test_ffi_exports_dont_panic() {
        unsafe {
            // Verify all exports are callable without panic
            rust_battle_uninit_ships();
            rust_battle_init_space();
            rust_battle_uninit_space();
            rust_battle_song(0);
            rust_battle_song(1);
            rust_free_battle_song();
            rust_computer_intelligence(std::ptr::null_mut(), std::ptr::null());
        }
    }
}
