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
    battle_context::run(|_| {
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
            crate::battle_context::with(|cx| {
                uqm_rust::state::ffi::rust_init_game_state(&mut cx.world)
            });
            crate::battle_context::with(|cx| cx.world.data.flagship.module_slots = [255; 16]);
            crate::battle_context::with(|cx| cx.world.data.flagship.drive_slots = [255; 11]);
            crate::battle_context::with(|cx| cx.world.data.flagship.jet_slots = [255; 8]);
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
                let first =
                    crate::battle_context::with(|cx| RaceDesc::load(&mut cx.world, species, false))
                        .unwrap();
                let second =
                    crate::battle_context::with(|cx| RaceDesc::load(&mut cx.world, species, false))
                        .unwrap();
                assert_ne!(first.address(), second.address(), "{key:?}");
                let original = first.info().ship_cost;
                // SAFETY: this fixture exclusively owns the live descriptor; only its scalar cost changes.
                (*(first.address() as *mut uqm_rust::engine::battle::record::RaceDescriptor))
                    .ship_info
                    .ship_cost = 254;
                assert_eq!(second.info().ship_cost, original);
                let loaded =
                    crate::battle_context::with(|cx| RaceDesc::load(&mut cx.world, species, false))
                        .unwrap();
                assert_eq!(loaded.info().ship_cost, original);
                assert!(loaded.race().is_some());
                assert_eq!(loaded.callbacks(), first.callbacks());
                crate::battle_context::with(|cx| first.unload(&mut cx.world, false));
                crate::battle_context::with(|cx| second.unload(&mut cx.world, false));
                crate::battle_context::with(|cx| loaded.unload(&mut cx.world, false));
            }
            assert!(crate::battle_context::with(|cx| RaceDesc::load(
                &mut cx.world,
                SPECIES_ID_NUM_SPECIES_ID,
                false
            ))
            .is_none());
            assert!(crate::battle_context::with(|cx| RaceDesc::load(
                &mut cx.world,
                u32::MAX,
                false
            ))
            .is_none());
            // No graphics are registered in the fixture, so required battle art fails.
            assert!(crate::battle_context::with(|cx| RaceDesc::load(
                &mut cx.world,
                SPECIES_ID_ARILOU_ID,
                true
            ))
            .is_none());
            let mut loaded = crate::battle_context::with(|cx| {
                RaceDesc::load(&mut cx.world, SPECIES_ID_ARILOU_ID, false)
            })
            .unwrap();
            assert!(loaded.race().is_some());
            loaded.set_race(&TEARDOWN_RACE);
            uqm_rust::collections::observation::enable();
            uqm_rust::collections::observation::drain();
            crate::battle_context::with(|cx| loaded.unload(&mut cx.world, true));
            assert_eq!(UNINIT.load(Ordering::SeqCst), 1);
            assert!(uqm_rust::collections::observation::drain()
                .iter()
                .any(|event| matches!(
                    event,
                    uqm_rust::collections::observation::Event::ResourcesFree { .. }
                )));
            let mut probe = crate::battle_context::with(|cx| {
                RaceDesc::load(&mut cx.world, SPECIES_ID_UR_QUAN_PROBE_ID, false)
            })
            .unwrap();
            probe.set_race(&TEARDOWN_RACE);
            // The shared callback fixture expects this scalar and must run before observation.
            (*(probe.address() as *mut uqm_rust::engine::battle::record::RaceDescriptor))
                .ship_info
                .ship_cost = 16;
            let descriptor = probe.address();
            uqm_rust::collections::observation::drain();
            crate::battle_context::with(|cx| {
                let world = &mut cx.world;
                let assets = probe.into_campaign_probe(world);
                world.retain_campaign_probe(assets);
                world.reset_game_data();
            });
            assert_eq!(UNINIT.load(Ordering::SeqCst), 2);
            assert!(
                matches!(uqm_rust::collections::observation::drain().as_slice(),
                [uqm_rust::collections::observation::Event::ResourcesFree {
                    descriptor: address, icons: false, battle: false
                }] if *address == descriptor)
            );
            let decoy =
                uqm_rust::graphics::drawable_abi::LoadedDrawable::create(2, 0, 0, 1).unwrap();
            let decoy_frames = decoy.lease();
            crate::battle_context::with(|cx| {
                let id = cx.world.avail_races().alloc().unwrap();
                let mut fleets = cx.world.avail_races();
                fleets.get_mut(id).icons = decoy_frames.frame(0).unwrap();
                fleets.get_mut(id).melee_icon = decoy_frames.frame(0).unwrap();
                fleets.put(id);
            });
            uqm_rust::collections::observation::drain();
            crate::battle_context::with(|cx| {
                uqm_rust::navigation::globals::uninit_game_structures(cx);
                assert_eq!(cx.world.avail_races_ref().count(), 0);
                assert_eq!(cx.world.encounters_ref().count(), 0);
                assert_eq!(cx.world.ip_groups_ref().count(), 0);
                assert_eq!(cx.world.npc_built_ships_ref().count(), 0);
                assert_eq!(cx.world.built_ships_ref().count(), 0);
                cx.world.release_campaign_probe();
            });
            assert_eq!(UNINIT.load(Ordering::SeqCst), 2);
            assert!(uqm_rust::collections::observation::drain().is_empty());
            assert!(decoy_frames.frame(0).is_some());
            drop(decoy);
            ffi_bridge::UninitResourceSystem();
            assert_eq!(uio_bridge::uio_closeDir(dir), 0);
            uio_bridge::uio_closeRepository(repository);
            uio_bridge::uio_unInit();
        }
    })
}
