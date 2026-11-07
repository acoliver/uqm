use super::*;
use std::{
    ffi::{CStr, CString},
    ptr,
};
use uqm_rust::{
    graphics::{
        dcqueue_abi::TFB_FlushGraphics,
        drawable_abi::*,
        native_abi::*,
        sdl::lifecycle_abi::{GraphicsOptions, TFB_InitGraphics, TFB_UninitGraphics},
    },
    io::uio_bridge as uio,
    planets::generate::{
        andro::AndrosynthSite, burv::BurvScenes, slylandro::HomeSite, spa::SpathiSite,
        syreen::StarbaseSite, thrad::ThraddashSite, vault::VaultSite, wreck::WreckSite,
        zoq::ZoqFotPikSite,
    },
};

const GRAPHICS: &[&str] = &[
    "ruins",
    "burvixcaster",
    "umgahcaster",
    "syreenvault",
    "urquanwreck",
    "taalodevice",
    "aquahelix",
    "utwigbomb",
    "sundevice",
    "eggcase",
    "maidens",
    "landerfonteffect",
];
const REPORTS: &[&str] = &[
    "ruins",
    "burvixeseruins",
    "burvixcaster",
    "umgahcaster",
    "syreenvault",
    "urquanwreck",
    "androsynthruins",
    "taalodevice",
    "aquahelix",
    "utwigbomb",
    "sundevice",
    "eggcase",
    "maidens",
    "vuxbeast",
    "spindle",
    "ultron",
    "sphere",
];

fn check_scene(
    game: &mut Game,
    index: u8,
    mask: Option<&str>,
    report: &str,
    first: Option<u16>,
    load: impl FnOnce(&mut Site<'_, '_, '_>, &mut SolarSys<'_>),
) {
    with_site(game, index, |host, system| {
        load(host, system);
        system.scene(|info, frame| {
            assert!(!info.lander_font.is_null());
            assert!(!info.lander_font_eff.is_null());
            if let Some(mask) = mask {
                assert!(!frame.is_null(), "missing mask {mask}");
                // SAFETY: the fixture owns this loaded mask until cleanup below.
                let pixel = unsafe { GetFramePixel(*frame, POINT { x: 0, y: 0 }) };
                let palette_index = GRAPHICS
                    .iter()
                    .position(|key| *key == mask)
                    .unwrap_or_else(|| panic!("unknown scene fixture mask {mask}"));
                let expected = u8::try_from(palette_index)
                    .unwrap_or_else(|error| panic!("mask fixture palette index: {error}"))
                    + 1;
                assert_eq!(
                    (pixel.r, pixel.g, pixel.b),
                    (expected, 19, 39),
                    "mask {mask}"
                );
            } else {
                assert!(frame.is_null());
            }
            use uqm_rust::strings::string_table::*;
            if let Some(first) = first {
                let table = info.discovery_string.cast();
                assert!(!info.discovery_string.is_null(), "missing report {report}");
                // SAFETY: the fixture retains the loaded report and its selected index.
                assert_eq!(unsafe { GetStringTableIndex(table) }, first);
                // SAFETY: the loaded report owns this terminated string until cleanup.
                let text = unsafe { CStr::from_ptr(GetStringAddress(table)) }
                    .to_str()
                    .unwrap_or_else(|error| panic!("report {report} is not UTF-8: {error}"));
                assert_eq!(text.trim_end_matches('\n'), format!("{report}-{first}"));
            } else {
                assert!(info.discovery_string.is_null());
            }
            // SAFETY: both font allocations belong exclusively to this fixture.
            unsafe {
                uqm_rust::graphics::font_abi::DestroyFont(info.lander_font.cast());
                DestroyDrawable(uqm_rust::graphics::pixmap_abi::ReleaseDrawable(
                    info.lander_font_eff.cast(),
                ));
            }
            info.lander_font = ptr::null_mut();
            info.lander_font_eff = ptr::null_mut();
            if !frame.is_null() {
                // SAFETY: this is the mask allocation checked above, released exactly once.
                unsafe { DestroyDrawable(uqm_rust::graphics::pixmap_abi::ReleaseDrawable(*frame)) };
                *frame = ptr::null_mut();
            }
            if !info.discovery_string.is_null() {
                // SAFETY: the fixture owns the loaded report table, released exactly once.
                unsafe { DestroyStringTable(ReleaseStringTable(info.discovery_string.cast())) };
                info.discovery_string = ptr::null_mut();
            }
        });
    });
}

#[test]
#[serial_test::serial]
fn scene_capabilities_load_their_masks_report_indices_and_caller_names() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&root).unwrap();
    let directory = tempfile::tempdir_in(root.canonicalize().unwrap()).unwrap();
    std::fs::create_dir(directory.path().join("font")).unwrap();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 255, 255, 255]))
        .save(directory.path().join("font/0041.png"))
        .unwrap();
    let mut rmp = String::from("font.lander = FONTRES:font\n");
    for (i, key) in GRAPHICS.iter().enumerate() {
        image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([u8::try_from(i).unwrap() + 1, 19, 39, 255]),
        )
        .save(directory.path().join(format!("{key}.png")))
        .unwrap();
        std::fs::write(
            directory.path().join(format!("{key}.ani")),
            format!("{key}.png -1 0 0 0\n"),
        )
        .unwrap();
        rmp.push_str(&format!("graphics.{key} = GFXRES:{key}.ani\n"));
    }
    for key in REPORTS {
        std::fs::write(
            directory.path().join(format!("{key}.txt")),
            format!("#(zero)\n{key}-0\n#(one)\n{key}-1\n#(two)\n{key}-2\n"),
        )
        .unwrap();
        rmp.push_str(&format!("text.{key} = STRTAB:{key}.txt\n"));
    }
    std::fs::write(directory.path().join("fixture.rmp"), rmp).unwrap();
    let path = CString::new(directory.path().to_str().unwrap()).unwrap();
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    // SAFETY: this serialized fixture owns SDL, the mount and all its native allocations on one thread.
    unsafe {
        assert_eq!(
            TFB_InitGraphics(
                1,
                0,
                ptr::from_ref(&GraphicsOptions::default()).cast(),
                320,
                240
            ),
            0
        );
        uio::uio_init();
        let repo = uio::uio_openRepository(0);
        assert!(!uio::uio_mountDir(
            repo,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            path.as_ptr(),
            ptr::null_mut(),
            6,
            ptr::null_mut()
        )
        .is_null());
        let content = uio::uio_openDir(repo, c"/".as_ptr(), 0);
        let previous = uqm_rust::mainloop::dir_prep::contentDir;
        uqm_rust::mainloop::dir_prep::contentDir = content.cast();
        uqm_rust::resource::ffi_bridge::InitResourceSystem();
        uqm_rust::resource::ffi_bridge::LoadResourceIndex(
            content.cast(),
            c"fixture.rmp".as_ptr(),
            ptr::null(),
        );
        let mut game = campaign();
        check_scene(
            &mut game,
            BURVIXESE_DEFINED,
            Some("ruins"),
            "burvixeseruins",
            Some(0),
            |host, system| BurvScenes::load_ruins_scene(host, system),
        );
        check_scene(
            &mut game,
            BURVIXESE_DEFINED,
            Some("burvixcaster"),
            "burvixcaster",
            Some(0),
            |host, system| BurvScenes::load_broadcasters_scene(host, system),
        );
        for unshielded in [false, true] {
            check_scene(
                &mut game,
                SPATHI_DEFINED,
                Some("umgahcaster"),
                "umgahcaster",
                Some(u16::from(unshielded)),
                |h, s| SpathiSite::load_broadcasters_scene(h, s, unshielded),
            );
        }
        for first in 0..3 {
            check_scene(
                &mut game,
                SHIP_VAULT_DEFINED,
                Some("syreenvault"),
                "syreenvault",
                Some(first),
                |h, s| VaultSite::load_vault_scene(h, s, first),
            );
            check_scene(
                &mut game,
                URQUAN_WRECK_DEFINED,
                Some("urquanwreck"),
                "urquanwreck",
                Some(first),
                |h, s| WreckSite::load_wreck_scene(h, s, first),
            );
        }
        check_scene(
            &mut game,
            ANDROSYNTH_DEFINED,
            Some("ruins"),
            "androsynthruins",
            Some(2),
            |h, s| AndrosynthSite::load_ruins_story(h, s, 2),
        );
        check_scene(
            &mut game,
            ANDROSYNTH_DEFINED,
            Some("ruins"),
            "androsynthruins",
            None,
            |h, s| AndrosynthSite::load_ruins_story(h, s, 3),
        );
        check_scene(
            &mut game,
            TAALO_PROTECTOR_DEFINED,
            Some("taalodevice"),
            "taalodevice",
            Some(0),
            |host, system| OrzSite::load_taalo_device_scene(host, system),
        );
        check_scene(
            &mut game,
            AQUA_HELIX_DEFINED,
            Some("aquahelix"),
            "aquahelix",
            Some(0),
            |host, system| ThraddashSite::load_aqua_scene(host, system),
        );
        for bomb in [false, true] {
            let key = if bomb { "utwigbomb" } else { "ruins" };
            check_scene(&mut game, BOMB_DEFINED, Some(key), key, Some(0), |h, s| {
                UtwigSite::load_scene(h, s, bomb)
            });
        }
        for (scene, key) in [
            (
                uqm_rust::planets::generate::myc::Scene::SunDevice,
                "sundevice",
            ),
            (uqm_rust::planets::generate::myc::Scene::EggCase, "eggcase"),
        ] {
            check_scene(&mut game, MYCON_DEFINED, Some(key), key, Some(0), |h, s| {
                MyconSite::load_scene(h, s, scene)
            });
        }
        for (scene, key, mask) in [
            (
                uqm_rust::planets::generate::vux::Scene::Maidens,
                "maidens",
                Some("maidens"),
            ),
            (
                uqm_rust::planets::generate::vux::Scene::Beast,
                "vuxbeast",
                None,
            ),
            (
                uqm_rust::planets::generate::vux::Scene::Ruins,
                "ruins",
                Some("ruins"),
            ),
        ] {
            check_scene(&mut game, VUX_DEFINED, mask, key, Some(0), |h, s| {
                VuxSite::load_scene(h, s, scene)
            });
        }
        for (index, key) in [
            (PKUNK_DEFINED, "spindle"),
            (SUPOX_DEFINED, "ultron"),
            (DRUUGE_DEFINED, "sphere"),
            (ORZ_DEFINED, "ruins"),
        ] {
            check_scene(&mut game, index, Some("ruins"), key, Some(1), |h, s| {
                HomeContactSite::load_ruins_scene(h, s, true)
            });
        }
        check_scene(
            &mut game,
            YEHAT_DEFINED,
            Some("ruins"),
            "ruins",
            Some(0),
            |host, system| YehatSite::load_ruins_scene(host, system),
        );
        game.kernel
            .install_game_text(uqm_rust::strings::owned_table::LoadedTextTable::fixture(
                vec![b"A\0".to_vec(), b"Caller planet\0".to_vec()],
            ));
        with_site(&mut game, SOL_DEFINED, |h, system| {
            use uqm_rust::engine::world::generation::ReportDisposition;
            use uqm_rust::planets::generate::{
                default::PlanetNames, host::ReportSite, sol::orbital::Orbital,
            };
            PlanetNames::set_planet_name(h, 1);
            PlanetNames::set_battle_planet(h, 17);
            Orbital::set_current_activity(h, 4);
            assert_eq!(Orbital::current_activity(h), 4);
            Orbital::set_global_flags_and_data(h, 23);
            assert_eq!(h.flag(StoryKey::GLOBAL_FLAGS_AND_DATA), 23);
            Orbital::set_current_activity(h, 0);
            assert!(!ReportSite::report(h, system, ReportDisposition::Once));
        });
        let previous_seed = uqm_rust::math::seed_random(123);
        let mut expected_rng = uqm_rust::math::RandomContext::with_seed(123);
        with_site(&mut game, TALKING_PET_DEFINED, |host, _| {
            assert_eq!(
                UrquanEncounterSite::random(host),
                expected_rng.next_random()
            );
        });
        uqm_rust::math::seed_random(previous_seed);
        with_site(&mut game, ZOQFOT_DEFINED, |host, _| {
            ZoqFotPikSite::clone_black_urquan_into_built_queue(host);
        });
        assert_eq!(race_of_first_npc(&game), ship::BLACK_URQUAN_SHIP);
        game.world.npc_built_ships().reinit();
        assert_eq!(game.world.flag(StoryKey::BATTLE_PLANET), 17);
        assert_eq!(
            CStr::from_ptr(game.world.data.flagship.planet_name.as_ptr().cast()),
            c"Caller planet"
        );
        let old_interactivity = uqm_rust::front_end::uqmdebug::DISABLE_INTERACTIVITY;
        uqm_rust::front_end::uqmdebug::DISABLE_INTERACTIVITY = 1;
        with_site(&mut game, ORZ_DEFINED, |h, _| {
            HomeContactSite::talk(h);
            ZoqFotPikSite::talk_black_urquan(h);
            TalkingPetSite::talk_pet(h);
            UtwigSite::talk_druuge(h);
            SamatraSite::talk_rebels(h);
            StarbaseSite::talk_to_syreen(h);
            HomeSite::talk_to_slylandro(h);
            YehatSite::talk_to_yehat(h);
            ChmmrSite::talk_to_chmmr(h);
            ChmmrSite::talk_to_ilwrath(h);
        });
        uqm_rust::front_end::uqmdebug::DISABLE_INTERACTIVITY = old_interactivity;
        assert_eq!(game.world.activity(), 0);
        TFB_FlushGraphics();
        drop(game);
        uqm_rust::resource::ffi_bridge::UninitResourceSystem();
        uqm_rust::mainloop::dir_prep::contentDir = previous;
        uio::uio_closeDir(content);
        uio::uio_closeRepository(repo);
        uio::uio_unInit();
        TFB_UninitGraphics();
    }
}
