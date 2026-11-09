//! Headless calls through Sol's actual exported table and drawable ABI.

use super::Harness;
use uqm_rust::battle::gendef::SOL_DEFINED;

use uqm_rust::planets::c_api::{self, CapturedSurface};
use uqm_rust::planets::generate::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;
use uqm_rust::planets::generate::{generator, SystemGenerator};

pub fn check() {
    let table = generator(SOL_DEFINED);
    let mut h = Harness::new();
    h.set_star(0, 0);
    check_initialized_identity(&mut h);
    h.seed(104729);
    let system = &raw mut *h.state;
    unsafe {
        let world = &raw mut (*system).sys.planets[0];
        assert!(!crate::invoke_generation(&mut *system, |view, host| table
            .pickup_energy(
                view,
                uqm_rust::planets::generate::special::Body::Planet(0),
                u16::MAX,
                host
            )));
        assert!(crate::invoke_generation(&mut *system, |view, host| table
            .pickup_minerals(
                view,
                uqm_rust::planets::generate::special::Body::Planet(0),
                u16::MAX,
                host
            )));
        assert!(crate::invoke_generation(&mut *system, |view, host| table
            .pickup_life(
                view,
                uqm_rust::planets::generate::special::Body::Planet(0),
                u16::MAX,
                host
            )));
        // No resource/report infrastructure is needed for the no-scene pickup.
        assert_eq!(h.rng_position(), 104729);
    }
    check_bomb_queue(&mut h);
    super::sol_npcs::check(&mut h);
    check_drawable_ownership(&mut h);
    println!("Sol traits: initialized dispatch identity, inherited defaults, callable pickups and bomb queue reset");
}

fn check_initialized_identity(h: &mut Harness) {
    h.star_index = SOL_DEFINED;
    h.set_star(0, 0);
    h.star[0].index = uqm_rust::battle::gendef::SHOFIXTI_DEFINED;
    uqm_rust::engine::World::current()
        .write(|world| world.set_catalog(std::borrow::Cow::Owned(*h.star)));
    // SAFETY: the serialized harness retains its catalog, RNG and solar records;
    // the callback releases all generated references before returning.
    let generated = unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let mut io = uqm_rust::engine::observe::Io {
                observer: &mut observer,
            };
            uqm_rust::planets::solarsys::site::dispatch::invoke(
                cx,
                &mut io,
                &mut h.state,
                |generator, system, host| {
                    assert_eq!(system.star_index(), SOL_DEFINED);
                    generator.generate_planets(system, host)
                },
            )
            .0
        })
    };
    assert!(generated);
    assert_eq!(
        h.state.sys.sun.num_planets, 9,
        "the initialized Sol generator must survive a later star-index change"
    );
    h.star_index = 0;
    h.set_star(0, 0);
}

fn check_bomb_queue(h: &mut Harness) {
    // This uses the real live queue adapter, not a recording FleetGroups host.
    let old_bomb = uqm_rust::engine::World::current()
        .read(|world| (world).story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE));
    let old_probe = uqm_rust::engine::World::current()
        .read(|world| (world).story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED));
    uqm_rust::engine::World::current()
        .write(|world| (world).set_story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 3));
    uqm_rust::engine::World::current().write(|world| {
        (world).set_story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED, 1)
    });
    unsafe {
        let saved = uqm_rust::engine::World::current().write(|world| {
            let saved = (
                std::mem::take(&mut world.queues),
                world.data.game.battle_group_ref,
            );
            world.data.game.battle_group_ref = 0x12345678;
            saved
        });
        assert!(crate::invoke_generation(&mut *h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.reinit_npcs(system, host)
        }));
        uqm_rust::engine::World::current().write(|world| {
            assert_eq!(world.data.game.battle_group_ref, 0);
            assert!(world.ip_groups_ref().head().is_none());
            assert!(world.npc_built_ships_ref().head().is_none());
            world.queues = saved.0;
            world.data.game.battle_group_ref = saved.1;
        });
    }
    uqm_rust::engine::World::current().write(|world| {
        (world).set_story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, old_bomb)
    });
    uqm_rust::engine::World::current().write(|world| {
        (world).set_story_state(
            uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED,
            old_probe,
        )
    });
}

fn allocations() -> i32 {
    unsafe { sdl2::sys::SDL_GetNumAllocations() }
}

fn check_drawable_ownership(h: &mut Harness) {
    let format =
        sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBA8888).unwrap();
    // Production keeps its non-alpha screen separate from the alpha conversion
    // surface. Sharing the latter here converts RGB resource art into RGBA.
    let screen =
        sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBX8888).unwrap();
    let old_format = unsafe { std::ptr::addr_of!(c_api::FORMAT_CONV_SURF).read() };
    let old_screen = unsafe { std::ptr::addr_of!(c_api::SDL_SCREEN).read() };
    unsafe {
        std::ptr::addr_of_mut!(c_api::FORMAT_CONV_SURF).write(format.raw().cast());
        std::ptr::addr_of_mut!(c_api::SDL_SCREEN).write(screen.raw().cast());
        c_api::Init_DrawCommandQueue();
    }
    // Warm SDL's format cache before measuring resource-owned allocations.
    unsafe {
        drop(CapturedSurface::pixmap(2, 2, 1));
        c_api::TFB_FlushGraphicsEx(1);
    }
    let baseline = allocations();
    assert!(baseline >= 0);
    let map_width = MAP_WIDTH as i16;
    let map_height = MAP_HEIGHT as i16;
    for (width, height) in [(map_width, map_height), (map_width / 2, map_height / 2)] {
        let definition = unsafe { CapturedSurface::pixmap(width, height, 2) };
        let frame = definition.as_frame();
        assert!(!frame.is_null());
        let created = allocations();
        assert!(created > baseline);
        unsafe {
            assert_eq!(
                ((*frame).Bounds.width, (*frame).Bounds.height),
                (width, height)
            );
            assert!(!(*frame).image.is_null());
        }
        let retained = if (width, height) == (map_width, map_height) {
            definition
        } else {
            let scaled =
                unsafe { definition.rescaled(i32::from(map_width), i32::from(map_height)) };
            assert!(!scaled.as_frame().is_null());
            assert_ne!(scaled.as_frame(), frame);
            let both = allocations();
            assert!(both > created);
            drop(scaled);
            unsafe { c_api::TFB_FlushGraphicsEx(1) };
            assert_eq!(allocations(), created);
            let normalized = unsafe { definition.into_definition(map_width, map_height) }.unwrap();
            assert_ne!(normalized.as_frame(), frame);
            unsafe {
                c_api::TFB_FlushGraphicsEx(1);
                let elevation = normalized.as_frame().add(1);
                assert_eq!((*elevation).Index, 1);
                assert_eq!((*elevation).parent, (*normalized.as_frame()).parent);
                assert_eq!(
                    ((*elevation).Bounds.width, (*elevation).Bounds.height),
                    (map_width, map_height)
                );
                assert!(!(*elevation).image.is_null());
            }
            assert_eq!(allocations(), created);
            normalized
        };
        let live = retained.as_frame();
        unsafe {
            assert_eq!(
                ((*live).Bounds.width, (*live).Bounds.height),
                (map_width, map_height)
            );
        }
        let owned = retained.into_drawable().unwrap();
        assert_eq!(owned.lease().frame(0), Some(live));
        drop(owned);
        // SAFETY: all surface owners have dropped; drain their deferred image releases.
        unsafe {
            c_api::TFB_FlushGraphicsEx(1);
        }
        assert_eq!(
            allocations(),
            baseline,
            "source/rescale owners leaked SDL allocations"
        );
        println!("Sol drawable ownership: {width}x{height} two-frame allocation -> retained {map_width}x{map_height} -> owned release; SDL allocations {created} -> {baseline}");
    }
    // DEBUG's early-return LoadPlanet path must release an unconsumed non-null
    // definition as well, rather than proving only a missing-resource branch.
    let saved_debug = unsafe { c_api::DISABLE_INTERACTIVITY };
    unsafe {
        c_api::DISABLE_INTERACTIVITY = 1;
        uqm_rust::app::bridge::enter(|cx| {
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let game = std::cell::RefCell::new(cx.reborrow());
            let io = std::cell::RefCell::new(uqm_rust::engine::observe::Io {
                observer: &mut observer,
            });
            let definition = game
                .borrow_mut()
                .with_legacy(|| CapturedSurface::pixmap(map_width / 2, map_height / 2, 2));
            let mut pickup = uqm_rust::planet_side::special_effects::PickupContext::NoLander;
            let mut site = uqm_rust::planets::solarsys::site::Site::new(
                &game,
                &io,
                h.state.star(),
                h.state.sys.star_index(),
                h.state.in_orbit(),
                &mut pickup,
                None,
            );
            site.invoke(&mut h.state.sys, |_, system, _| {
                assert!(c_api::load_planet(
                    &game,
                    &io,
                    system,
                    (
                        uqm_rust::engine::StarId(0),
                        uqm_rust::engine::world::generation::Body::Planet(0)
                    ),
                    false,
                    definition.into_drawable(),
                    Default::default(),
                )
                .is_none());
            });
        });
        c_api::TFB_FlushGraphicsEx(1);
        c_api::DISABLE_INTERACTIVITY = saved_debug;
    }
    assert_eq!(allocations(), baseline);
    super::sol_resources::check(h);
    unsafe {
        c_api::Uninit_DrawCommandQueue();
        std::ptr::addr_of_mut!(c_api::FORMAT_CONV_SURF).write(old_format);
        std::ptr::addr_of_mut!(c_api::SDL_SCREEN).write(old_screen);
    }
}
