use uqm_rust::ships::{load_ship, unload_ship};
#[path = "support/battle_context.rs"]
mod battle_context;

#[path = "support/battle.rs"]
mod battle;
use battle::Battle;
use std::{
    ffi::CString,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use uqm_rust::{io::uio_bridge, resource::ffi_bridge, ships::native::abi::*};

unsafe extern "C" {
    fn log_init(max: i32);

}
static UNINIT: AtomicUsize = AtomicUsize::new(0);
fn uninit(
    _: &mut uqm_rust::engine::battle::World,
    stats: &mut uqm_rust::engine::battle::views::RaceStats,
) {
    assert_eq!(stats.ship_info.ship_cost, 16);
    assert!(!uqm_rust::collections::observation::drain()
        .iter()
        .any(|event| matches!(
            event,
            uqm_rust::collections::observation::Event::ResourcesFree { .. }
        )));
    UNINIT.fetch_add(1, Ordering::SeqCst);
}
static TEARDOWN_RACE: uqm_rust::engine::battle::Race = uqm_rust::engine::battle::Race {
    descriptor: |_| {
        uqm_rust::engine::battle::template::RaceTemplate::default().descriptor(&TEARDOWN_RACE)
    },
    preprocess: None,
    postprocess: None,
    init_weapon: None,
    intelligence: None,
    uninit: Some(uninit),
};

#[test]
fn code_resources_and_species_loader_preserve_mapping_copy_ownership_and_failures() {
    battle_context::run(|| {
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
                let species = i as u32 + 1;
                let mut first = load_ship(species, 0).unwrap();
                let second = load_ship(species, 0).unwrap();
                assert_ne!(
                    std::ptr::from_ref(&*first),
                    std::ptr::from_ref(&*second),
                    "{key:?}"
                );
                let original = first.ship_info.ship_cost;
                first.ship_info.ship_cost = 254;
                assert_eq!(second.ship_info.ship_cost, original);
                let loaded = load_ship(species, 0).unwrap();
                assert_eq!(loaded.ship_info.ship_cost, original);
                assert!(loaded.race.is_some());
                assert_eq!(loaded.callbacks(), first.callbacks());
                unload_ship(first, 1, 0);
                unload_ship(second, 1, 0);
                unload_ship(loaded, 1, 0);
            }
            assert!(load_ship(SPECIES_ID_NUM_SPECIES_ID, 0).is_none());
            assert!(load_ship(u32::MAX, 0).is_none());
            // No graphics are registered in the fixture, so required battle art fails.
            assert!(load_ship(SPECIES_ID_ARILOU_ID, 1).is_none());
            let mut loaded = load_ship(SPECIES_ID_ARILOU_ID, 0).unwrap();
            assert!(loaded.race.is_some());
            loaded.race = Some(&TEARDOWN_RACE);
            uqm_rust::collections::observation::enable();
            uqm_rust::collections::observation::drain();
            unload_ship(loaded, 1, 1);
            assert_eq!(UNINIT.load(Ordering::SeqCst), 1);
            assert!(uqm_rust::collections::observation::drain()
                .iter()
                .any(|event| matches!(
                    event,
                    uqm_rust::collections::observation::Event::ResourcesFree { .. }
                )));
            ffi_bridge::UninitResourceSystem();
            assert_eq!(uio_bridge::uio_closeDir(dir), 0);
            uio_bridge::uio_closeRepository(repository);
            uio_bridge::uio_unInit();
        }
    })
}
