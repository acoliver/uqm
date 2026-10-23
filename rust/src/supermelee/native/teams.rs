//! Team ownership and persistence from meleesetup.c.
#![allow(non_snake_case)]
use crate::io::uio_bridge::{uio_fread, uio_fwrite};
use crate::memory::{rust_hfree, rust_hmalloc};
use crate::ships::native::abi::{MeleeSetup, MeleeShip, MeleeTeam};
use crate::supermelee::setup::ffi::{
    rust_supermelee_fleet_value, rust_supermelee_team_deserialize, rust_supermelee_team_serialize,
};
use std::{ffi::c_char, mem, ptr};

#[no_mangle]
pub static MeleeTeam_serialSize: usize = 69;

fn encode(team: &MeleeTeam) -> [u8; 69] {
    let ships = team.ships.map(|ship| ship as u8);
    let mut bytes = [0; 69];
    unsafe {
        assert_eq!(
            rust_supermelee_team_serialize(
                ships.as_ptr(),
                team.name.as_ptr().cast(),
                bytes.as_mut_ptr(),
                bytes.len()
            ),
            0
        );
    }
    bytes
}
fn decode(team: &mut MeleeTeam, bytes: &[u8; 69]) {
    let mut ships = [0; 14];
    unsafe {
        assert_eq!(
            rust_supermelee_team_deserialize(
                bytes.as_ptr(),
                bytes.len(),
                ships.as_mut_ptr(),
                team.name.as_mut_ptr().cast()
            ),
            0
        );
    }
    team.ships = ships.map(u32::from);
    team.name[30] = 0;
}

/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_init(team: *mut MeleeTeam) {
    (*team).ships.fill(255);
    (*team).name[0] = 0;
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_uninit(_team: *mut MeleeTeam) {}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_new() -> *mut MeleeTeam {
    let team = rust_hmalloc(mem::size_of::<MeleeTeam>()).cast::<MeleeTeam>();
    team.write(mem::zeroed());
    MeleeTeam_init(team);
    team
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_delete(team: *mut MeleeTeam) {
    rust_hfree(team.cast());
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_serialize(
    team: *const MeleeTeam,
    stream: *mut crate::io::uio_bridge::uio_Stream,
) -> i32 {
    let bytes = encode(&*team);
    if uio_fwrite(bytes.as_ptr().cast(), bytes.len(), 1, stream) == 1 {
        0
    } else {
        -1
    }
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_deserialize(
    team: *mut MeleeTeam,
    stream: *mut crate::io::uio_bridge::uio_Stream,
) -> i32 {
    let mut bytes = [0; 69];
    if uio_fread(bytes.as_mut_ptr().cast(), bytes.len(), 1, stream) != 1 {
        MeleeTeam_delete(team);
        return -1;
    }
    decode(&mut *team, &bytes);
    0
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_getValue(team: *const MeleeTeam) -> u16 {
    let ships = (*team).ships.map(|ship| ship as u8);
    rust_supermelee_fleet_value(ships.as_ptr())
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_getShip(team: *const MeleeTeam, slot: u16) -> MeleeShip {
    (*team).ships[usize::from(slot)]
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_setShip(team: *mut MeleeTeam, slot: u16, ship: MeleeShip) {
    (*team).ships[usize::from(slot)] = ship;
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_getFleet(team: *const MeleeTeam) -> *const MeleeShip {
    (*team).ships.as_ptr()
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_getTeamName(team: *const MeleeTeam) -> *const c_char {
    (*team).name.as_ptr()
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_setName(team: *mut MeleeTeam, name: *const c_char) {
    libc::strncpy((*team).name.as_mut_ptr(), name, 54);
    (*team).name[54] = 0;
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeTeam_copy(copy: *mut MeleeTeam, original: *const MeleeTeam) {
    copy.write(original.read());
}

unsafe fn reset_sent(setup: *mut MeleeSetup, side: usize) {
    (*setup).sentTeams[side].ships.fill(254);
    (*setup).haveSentTeamName[side] = false;
    if cfg!(feature = "debug-process") {
        MeleeTeam_setName(&mut (*setup).sentTeams[side], c"<INVALID>".as_ptr());
    }
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_new() -> *mut MeleeSetup {
    let setup = rust_hmalloc(mem::size_of::<MeleeSetup>()).cast::<MeleeSetup>();
    if setup.is_null() {
        return setup;
    }
    setup.write(mem::zeroed());
    for side in 0..2 {
        MeleeTeam_init(&mut (*setup).teams[side]);
        (*setup).fleetValue[side] = 0;
        reset_sent(setup, side);
    }
    setup
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_delete(setup: *mut MeleeSetup) {
    rust_hfree(setup.cast());
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_resetSentTeams(setup: *mut MeleeSetup) {
    for side in 0..2 {
        reset_sent(setup, side);
    }
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_setShip(
    setup: *mut MeleeSetup,
    side: usize,
    slot: u16,
    ship: MeleeShip,
) -> bool {
    let old = MeleeTeam_getShip(&(*setup).teams[side], slot);
    if ship == old {
        return false;
    }
    if old != 255 {
        (*setup).fleetValue[side] = (*setup).fleetValue[side]
            .wrapping_sub(crate::ships::ffi::rust_ships_get_cost_by_index(old as u16));
    }
    MeleeTeam_setShip(&mut (*setup).teams[side], slot, ship);
    if ship != 255 {
        (*setup).fleetValue[side] = (*setup).fleetValue[side]
            .wrapping_add(crate::ships::ffi::rust_ships_get_cost_by_index(ship as u16));
    }
    true
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_getShip(
    setup: *const MeleeSetup,
    side: usize,
    slot: u16,
) -> MeleeShip {
    MeleeTeam_getShip(&(*setup).teams[side], slot)
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_getFleet(
    setup: *const MeleeSetup,
    side: usize,
) -> *const MeleeShip {
    MeleeTeam_getFleet(&(*setup).teams[side])
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_setTeamName(
    setup: *mut MeleeSetup,
    side: usize,
    name: *const c_char,
) -> bool {
    let team = &mut (*setup).teams[side];
    if libc::strcmp(team.name.as_ptr(), name) == 0 {
        return false;
    }
    MeleeTeam_setName(team, name);
    true
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_getTeamName(
    setup: *const MeleeSetup,
    side: usize,
) -> *const c_char {
    MeleeTeam_getTeamName(&(*setup).teams[side])
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_getFleetValue(setup: *const MeleeSetup, side: usize) -> u16 {
    (*setup).fleetValue[side]
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_deserializeTeam(
    setup: *mut MeleeSetup,
    side: usize,
    stream: *mut crate::io::uio_bridge::uio_Stream,
) -> i32 {
    let ret = MeleeTeam_deserialize(&mut (*setup).teams[side], stream);
    if ret == 0 {
        (*setup).fleetValue[side] = MeleeTeam_getValue(&(*setup).teams[side]);
    }
    ret
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_serializeTeam(
    setup: *const MeleeSetup,
    side: usize,
    stream: *mut crate::io::uio_bridge::uio_Stream,
) -> i32 {
    MeleeTeam_serialize(&(*setup).teams[side], stream)
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_getSentShip(
    setup: *const MeleeSetup,
    side: usize,
    slot: u16,
) -> MeleeShip {
    MeleeTeam_getShip(&(*setup).sentTeams[side], slot)
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_getSentTeamName(
    setup: *const MeleeSetup,
    side: usize,
) -> *const c_char {
    if !(*setup).haveSentTeamName[side] {
        ptr::null()
    } else {
        MeleeTeam_getTeamName(&(*setup).sentTeams[side])
    }
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_setSentShip(
    setup: *mut MeleeSetup,
    side: usize,
    slot: u16,
    ship: MeleeShip,
) -> bool {
    let team = &mut (*setup).sentTeams[side];
    if MeleeTeam_getShip(team, slot) == ship {
        return false;
    }
    MeleeTeam_setShip(team, slot, ship);
    true
}
/// # Safety
/// Pointers must be valid for the indicated records or streams on the game thread.
#[no_mangle]
pub unsafe extern "C" fn MeleeSetup_setSentTeamName(
    setup: *mut MeleeSetup,
    side: usize,
    name: *const c_char,
) -> bool {
    let had = (*setup).haveSentTeamName[side];
    let team = &mut (*setup).sentTeams[side];
    if name.is_null() {
        if !had {
            return false;
        }
        if cfg!(feature = "debug-process") {
            MeleeTeam_setName(team, c"<INVALID>".as_ptr());
        }
    } else {
        if had && libc::strcmp(team.name.as_ptr(), name) == 0 {
            return false;
        }
        MeleeTeam_setName(team, name);
    }
    (*setup).haveSentTeamName[side] = !name.is_null();
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[serial_test::serial]
    fn persists_a_team_in_a_real_uio_stream_and_releases_it_on_short_read() {
        use crate::io::uio_bridge::*;
        use std::ffi::CString;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/melee-team-tests");
        std::fs::create_dir_all(&root).unwrap();
        let scratch = tempfile::TempDir::new_in(root).unwrap();
        let host = CString::new(scratch.path().to_str().unwrap()).unwrap();
        unsafe {
            uio_init();
            let repo = uio_openRepository(0);
            let mount = uio_mountDir(
                repo,
                c"/".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                4,
                ptr::null_mut(),
            );
            assert!(!mount.is_null());
            let dir = uio_openDir(repo, c"/".as_ptr(), 0);
            let team = MeleeTeam_new();
            (*team).ships[0] = 3;
            MeleeTeam_setName(team, c"Test fleet".as_ptr());
            let output = uio_fopen(dir, c"fleet.mle".as_ptr(), c"wb".as_ptr());
            assert!(!output.is_null());
            assert_eq!(MeleeTeam_serialize(team, output), 0);
            uio_fclose(output);
            assert_eq!(
                std::fs::read(scratch.path().join("fleet.mle")).unwrap(),
                encode(&*team)
            );
            let input = uio_fopen(dir, c"fleet.mle".as_ptr(), c"rb".as_ptr());
            let loaded = MeleeTeam_new();
            assert_eq!(MeleeTeam_deserialize(loaded, input), 0);
            assert_eq!((*loaded).ships, (*team).ships);
            assert_eq!((*loaded).name, (*team).name);
            assert_eq!(MeleeTeam_deserialize(loaded, input), -1);
            MeleeTeam_delete(team);
            uio_fclose(input);
            uio_closeDir(dir);
            uio_closeRepository(repo);
            uio_unInit();
        }
    }
    #[test]
    fn team_wire_round_trip_packs_integer_enums_and_truncates_name() {
        unsafe {
            let team = MeleeTeam_new();
            (*team).ships[0] = 3;
            (*team).ships[13] = 24;
            MeleeTeam_setName(
                team,
                c"A fleet name that exceeds thirty characters by a bit".as_ptr(),
            );
            let bytes = encode(&*team);
            assert_eq!(bytes[0], 3);
            assert_eq!(bytes[13], 24);
            let copy = MeleeTeam_new();
            decode(&mut *copy, &bytes);
            assert_eq!((*copy).ships, (*team).ships);
            assert_eq!((*copy).name[30], 0);
            assert_eq!(MeleeTeam_getValue(copy), 30 + 6);
            MeleeTeam_delete(copy);
            MeleeTeam_delete(team);
        }
    }
    #[test]
    fn sent_name_distinguishes_unset_and_empty() {
        unsafe {
            let setup = MeleeSetup_new();
            assert_eq!(MeleeSetup_getSentShip(setup, 0, 0), 254);
            assert!(MeleeSetup_getSentTeamName(setup, 0).is_null());
            assert!(MeleeSetup_setSentTeamName(setup, 0, c"".as_ptr()));
            assert!(!MeleeSetup_getSentTeamName(setup, 0).is_null());
            assert!(!MeleeSetup_setSentTeamName(setup, 0, c"".as_ptr()));
            assert!(MeleeSetup_setSentTeamName(setup, 0, ptr::null()));
            assert!(!MeleeSetup_setSentTeamName(setup, 0, ptr::null()));
            MeleeSetup_delete(setup);
        }
    }
}
