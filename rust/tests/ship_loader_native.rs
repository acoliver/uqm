#![cfg(feature = "linked_c_archive")]
mod native_ship_common;
use native_ship_common::Battle;
use std::{
    ffi::{c_void, CString},
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use uqm_rust::{io::uio_bridge, resource::ffi_bridge, ships::native::abi::*};

unsafe extern "C" {
    fn log_init(max: i32);
    fn LoadCodeResInstance(res: *const std::ffi::c_char) -> *mut c_void;
    fn CaptureCodeRes(code: *mut c_void, data: *mut c_void, out: *mut *mut c_void) -> *mut c_void;
    fn ReleaseCodeRes(code: *mut c_void) -> *mut c_void;
    fn DestroyCodeRes(code: *mut c_void) -> u32;
    fn c_load_ship(species: u32, battle: u32) -> *mut RACE_DESC;
    fn load_ship(species: u32, battle: u32) -> *mut RACE_DESC;
    fn c_free_ship(desc: *mut RACE_DESC, icons: u32, battle: u32);
    fn free_ship(desc: *mut RACE_DESC, icons: u32, battle: u32);
}
static UNINIT: AtomicUsize = AtomicUsize::new(0);
unsafe extern "C" fn uninit(desc: *mut RACE_DESC) {
    assert!(!(*desc).CodeRef.is_null());
    UNINIT.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn code_resources_and_species_loader_preserve_mapping_copy_ownership_and_failures() {
    let _battle = Battle::new(16);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp/s6");
    std::fs::create_dir_all(&root).unwrap();
    let fixture = tempfile::tempdir_in(root).unwrap();
    let rmp = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content/uqm.rmp"),
    )
    .unwrap();
    let mut index = String::new();
    for line in rmp
        .lines()
        .filter(|l| l.contains(".code") && l.contains("SHIP:"))
    {
        index.push_str(line);
        index.push('\n');
    }
    index.push_str("test.invalid.code = SHIP:255\n");
    std::fs::write(fixture.path().join("ships.rmp"), &index).unwrap();
    let path = CString::new(fixture.path().to_str().unwrap()).unwrap();
    unsafe {
        log_init(100);
        uqm_rust::state::ffi::rust_init_game_state();
        GlobData.SIS_state.ModuleSlots = [255; 16];
        GlobData.SIS_state.DriveSlots = [255; 11];
        GlobData.SIS_state.JetSlots = [255; 8];
        uio_bridge::uio_init();
        let repository = uio_bridge::uio_openRepository(0);
        let mount = uio_bridge::uio_mountDir(
            repository,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            path.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let dir = uio_bridge::uio_openDir(repository, c"/".as_ptr(), 0);
        assert!(!dir.is_null());
        ffi_bridge::InitResourceSystem();
        ffi_bridge::LoadResourceIndex(dir.cast(), c"ships.rmp".as_ptr(), ptr::null());
        let keys = [
            "arilou",
            "chmmr",
            "earthling",
            "orz",
            "pkunk",
            "shofixti",
            "spathi",
            "supox",
            "thraddash",
            "utwig",
            "vux",
            "yehat",
            "melnorme",
            "druuge",
            "ilwrath",
            "mycon",
            "slylandro",
            "umgah",
            "urquan",
            "zoqfotpik",
            "syreen",
            "kohrah",
            "androsynth",
            "chenjesu",
            "mmrnmhrm",
            "flagship",
            "samatra",
            "drone",
        ];
        for (i, key) in keys.iter().enumerate() {
            let key = CString::new(format!("ship.{key}.code")).unwrap();
            let first = LoadCodeResInstance(key.as_ptr());
            assert!(!first.is_null(), "code resource {key:?}");
            let second = LoadCodeResInstance(key.as_ptr());
            assert!(!second.is_null());
            assert_ne!(
                first, second,
                "each detached code resource owns its descriptor copy"
            );
            let mut out = ptr::null_mut();
            assert_eq!(CaptureCodeRes(first, ptr::null_mut(), &mut out), first);
            let expected = out.cast::<RACE_DESC>();
            let original = (*expected).ship_info.ship_cost;
            (*expected).ship_info.ship_cost = 254;
            let mut other = ptr::null_mut();
            CaptureCodeRes(second, ptr::null_mut(), &mut other);
            assert_eq!((*other.cast::<RACE_DESC>()).ship_info.ship_cost, original);
            let loaded = c_load_ship(i as u32 + 1, 0);
            assert!(!loaded.is_null(), "species {}", i + 1);
            assert_eq!((*loaded).ship_info.ship_cost, original);
            assert!(!(*loaded).CodeRef.is_null());
            assert_eq!(
                (*loaded).preprocess_func.map(|f| f as usize),
                (*expected).preprocess_func.map(|f| f as usize)
            );
            c_free_ship(loaded, 1, 0);
            // CodeRes destruction owns only the record. Its caller owns uninit.
            for (code, desc) in [(first, expected), (second, other.cast())] {
                if let Some(uninit) = (*desc).uninit_func {
                    uninit(desc);
                }
                assert_eq!(ReleaseCodeRes(code), code);
                assert_eq!(DestroyCodeRes(code), 1);
            }
        }
        assert!(LoadCodeResInstance(c"test.invalid.code".as_ptr()).is_null());
        assert!(c_load_ship(SPECIES_ID_NUM_SPECIES_ID, 0).is_null());
        assert!(load_ship(u32::MAX, 0).is_null());
        // No graphics are registered in the fixture, so required battle art fails.
        assert!(c_load_ship(SPECIES_ID_ARILOU_ID, 1).is_null());
        let loaded = load_ship(SPECIES_ID_ARILOU_ID, 0);
        assert!(!loaded.is_null());
        (*loaded).uninit_func = Some(uninit);
        free_ship(loaded, 1, 1);
        assert_eq!(UNINIT.load(Ordering::SeqCst), 1);
        assert_eq!(DestroyCodeRes(ptr::null_mut()), 1);
        ffi_bridge::UninitResourceSystem();
        assert_eq!(uio_bridge::uio_closeDir(dir), 0);
        uio_bridge::uio_closeRepository(repository);
        uio_bridge::uio_unInit();
    }
}
