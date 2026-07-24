// SuperMelee FFI Exports — Rust entry points callable from C
// @plan PLAN-20260314-SUPERMELEE.P11
//
// These functions are exported with C ABI so the C game code can call
// into the Rust SuperMelee implementation. They follow the project
// pattern: `#[no_mangle] pub unsafe extern "C" fn rust_supermelee_*`.

use crate::supermelee::setup::persistence::{
    deserialize_team, serialize_team, MELEE_TEAM_SERIAL_SIZE,
};
use crate::supermelee::setup::team::{ship_cost, MeleeTeam};
use crate::supermelee::types::{MeleeShip, MELEE_FLEET_SIZE};
use std::io::Cursor;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Team serialization (replaces MeleeTeam_serialize/deserialize in C)
// ---------------------------------------------------------------------------

/// Serialize a MeleeTeam to a buffer.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// Returns 0 on success, -1 on failure.
/// Buffer must be at least MELEE_TEAM_SERIAL_SIZE bytes.
#[no_mangle]
pub unsafe extern "C" fn rust_supermelee_team_serialize(
    ships: *const u8,
    name: *const u8,
    out_buf: *mut u8,
    buf_len: usize,
) -> c_int {
    if ships.is_null() || name.is_null() || out_buf.is_null() {
        return -1;
    }
    if buf_len < MELEE_TEAM_SERIAL_SIZE {
        return -1;
    }

    let mut team = MeleeTeam::new();

    // Copy ships
    let ships_slice = unsafe { std::slice::from_raw_parts(ships, MELEE_FLEET_SIZE) };
    for (i, &raw) in ships_slice.iter().enumerate() {
        team.ships[i] = MeleeShip::from_u8(raw).unwrap_or(MeleeShip::MeleeNone);
    }

    // Copy name
    let name_slice = unsafe { std::slice::from_raw_parts(name, team.name.len()) };
    team.name[..name_slice.len()].copy_from_slice(name_slice);

    let mut buf = Vec::with_capacity(MELEE_TEAM_SERIAL_SIZE);
    match serialize_team(&team, &mut buf) {
        Ok(()) => {
            let out = unsafe { std::slice::from_raw_parts_mut(out_buf, buf_len) };
            out[..buf.len()].copy_from_slice(&buf);
            0
        }
        Err(_) => -1,
    }
}

/// Deserialize a MeleeTeam from a buffer.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rust_supermelee_team_deserialize(
    in_buf: *const u8,
    buf_len: usize,
    out_ships: *mut u8,
    out_name: *mut u8,
) -> c_int {
    if in_buf.is_null() || out_ships.is_null() || out_name.is_null() {
        return -1;
    }
    if buf_len < MELEE_TEAM_SERIAL_SIZE {
        return -1;
    }

    let data = unsafe { std::slice::from_raw_parts(in_buf, buf_len) };
    let mut cursor = Cursor::new(data);

    match deserialize_team(&mut cursor) {
        Ok(team) => {
            let ships_out = unsafe { std::slice::from_raw_parts_mut(out_ships, MELEE_FLEET_SIZE) };
            for (i, &ship) in team.ships.iter().enumerate() {
                ships_out[i] = ship as u8;
            }
            let name_out = unsafe { std::slice::from_raw_parts_mut(out_name, team.name.len()) };
            name_out.copy_from_slice(&team.name);
            0
        }
        Err(_) => -1,
    }
}

// ---------------------------------------------------------------------------
// Ship cost lookup
// ---------------------------------------------------------------------------

/// Returns the fleet-point cost of a ship by its raw ID.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// Returns 0 for invalid/sentinel values.
#[no_mangle]
pub unsafe extern "C" fn rust_supermelee_ship_cost(ship_id: u8) -> u16 {
    match MeleeShip::from_u8(ship_id) {
        Some(ship) => ship_cost(ship),
        None => 0,
    }
}

// ---------------------------------------------------------------------------
// Fleet value computation
// ---------------------------------------------------------------------------

/// Computes fleet value for 14 ship slots.
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
/// `ships` must point to MELEE_FLEET_SIZE bytes.
#[no_mangle]
pub unsafe extern "C" fn rust_supermelee_fleet_value(ships: *const u8) -> u16 {
    if ships.is_null() {
        return 0;
    }
    let ships_slice = unsafe { std::slice::from_raw_parts(ships, MELEE_FLEET_SIZE) };
    ships_slice
        .iter()
        .map(|&raw| MeleeShip::from_u8(raw).map(ship_cost).unwrap_or(0))
        .sum()
}

// ---------------------------------------------------------------------------
// Team serial size constant
// ---------------------------------------------------------------------------

/// Returns the serial size of a MeleeTeam (for C callers that need it).
/// # Safety
///
/// This is an FFI function called from C. The caller must ensure pointers are valid.
#[no_mangle]
pub unsafe extern "C" fn rust_supermelee_team_serial_size() -> usize {
    MELEE_TEAM_SERIAL_SIZE
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supermelee::types::MeleeShip;

    #[test]
    fn ship_cost_ffi_matches_rust() {
        unsafe {
            assert_eq!(
                rust_supermelee_ship_cost(0),
                ship_cost(MeleeShip::Androsynth)
            );
            assert_eq!(
                rust_supermelee_ship_cost(24),
                ship_cost(MeleeShip::ZoqFotPik)
            );
            assert_eq!(rust_supermelee_ship_cost(255), 0); // MELEE_NONE
            assert_eq!(rust_supermelee_ship_cost(200), 0); // invalid
        }
    }

    #[test]
    fn fleet_value_ffi_works() {
        unsafe {
            let ships = [
                MeleeShip::Chmmr as u8,
                MeleeShip::Shofixti as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
                MeleeShip::MeleeNone as u8,
            ];
            let val = rust_supermelee_fleet_value(ships.as_ptr());
            assert_eq!(
                val,
                ship_cost(MeleeShip::Chmmr) + ship_cost(MeleeShip::Shofixti)
            );
        }
    }

    #[test]
    fn serial_size_matches() {
        unsafe {
            assert_eq!(rust_supermelee_team_serial_size(), MELEE_TEAM_SERIAL_SIZE);
        }
    }

    #[test]
    fn serialize_deserialize_roundtrip_ffi() {
        unsafe {
            let mut ships = [MeleeShip::MeleeNone as u8; MELEE_FLEET_SIZE];
            ships[0] = MeleeShip::Urquan as u8;
            ships[3] = MeleeShip::Pkunk as u8;

            let name_buf_size = crate::supermelee::types::MAX_TEAM_CHARS + 1 + 24;
            let mut name = vec![0u8; name_buf_size];
            let test_name = b"FFI Test";
            name[..test_name.len()].copy_from_slice(test_name);

            let mut serial_buf = vec![0u8; MELEE_TEAM_SERIAL_SIZE];
            let ret = rust_supermelee_team_serialize(
                ships.as_ptr(),
                name.as_ptr(),
                serial_buf.as_mut_ptr(),
                serial_buf.len(),
            );
            assert_eq!(ret, 0);

            let mut out_ships = vec![0u8; MELEE_FLEET_SIZE];
            let mut out_name = vec![0u8; name_buf_size];
            let ret = rust_supermelee_team_deserialize(
                serial_buf.as_ptr(),
                serial_buf.len(),
                out_ships.as_mut_ptr(),
                out_name.as_mut_ptr(),
            );
            assert_eq!(ret, 0);
            assert_eq!(out_ships[0], MeleeShip::Urquan as u8);
            assert_eq!(out_ships[3], MeleeShip::Pkunk as u8);
            assert_eq!(&out_name[..test_name.len()], test_name);
        }
    }

    #[test]
    fn null_pointers_return_error() {
        unsafe {
            assert_eq!(
                rust_supermelee_team_serialize(
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    0
                ),
                -1
            );
            assert_eq!(
                rust_supermelee_team_deserialize(
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut()
                ),
                -1
            );
            assert_eq!(rust_supermelee_fleet_value(std::ptr::null()), 0);
        }
    }
}
