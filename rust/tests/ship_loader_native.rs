use uqm_rust::engine::battle::RaceDesc;
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
            uqm_rust::engine::World::current().write(uqm_rust::state::ffi::rust_init_game_state);
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .module_slots = [255; 16];
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .drive_slots = [255; 11];
            (**uqm_rust::engine::WorldData::current())
                .flagship
                .jet_slots = [255; 8];
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
                let first = uqm_rust::engine::World::current()
                    .write(|world| RaceDesc::load(world, species, false))
                    .unwrap();
                let second = uqm_rust::engine::World::current()
                    .write(|world| RaceDesc::load(world, species, false))
                    .unwrap();
                assert_ne!(first.address(), second.address(), "{key:?}");
                let original = first.info().ship_cost;
                // SAFETY: this fixture exclusively owns the live descriptor; only its scalar cost changes.
                (*(first.address() as *mut uqm_rust::engine::battle::record::RaceDescriptor))
                    .ship_info
                    .ship_cost = 254;
                assert_eq!(second.info().ship_cost, original);
                let loaded = uqm_rust::engine::World::current()
                    .write(|world| RaceDesc::load(world, species, false))
                    .unwrap();
                assert_eq!(loaded.info().ship_cost, original);
                assert!(loaded.race().is_some());
                assert_eq!(loaded.callbacks(), first.callbacks());
                uqm_rust::engine::World::current().write(|world| first.unload(world, true, false));
                uqm_rust::engine::World::current().write(|world| second.unload(world, true, false));
                uqm_rust::engine::World::current().write(|world| loaded.unload(world, true, false));
            }
            assert!(uqm_rust::engine::World::current()
                .write(|world| RaceDesc::load(world, SPECIES_ID_NUM_SPECIES_ID, false))
                .is_none());
            assert!(uqm_rust::engine::World::current()
                .write(|world| RaceDesc::load(world, u32::MAX, false))
                .is_none());
            // No graphics are registered in the fixture, so required battle art fails.
            assert!(uqm_rust::engine::World::current()
                .write(|world| RaceDesc::load(world, SPECIES_ID_ARILOU_ID, true))
                .is_none());
            let mut loaded = uqm_rust::engine::World::current()
                .write(|world| RaceDesc::load(world, SPECIES_ID_ARILOU_ID, false))
                .unwrap();
            assert!(loaded.race().is_some());
            loaded.set_race(&TEARDOWN_RACE);
            uqm_rust::collections::observation::enable();
            uqm_rust::collections::observation::drain();
            uqm_rust::engine::World::current().write(|world| loaded.unload(world, true, true));
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
