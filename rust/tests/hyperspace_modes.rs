use uqm_rust::{engine::Game, navigation::hyper_data};

#[test]
fn resource_teardown_uses_only_the_supplied_kernel() {
    let mut game = Game::new();
    hyper_data::free_hyper_data(&mut game.kernel);
    assert!(game.kernel.hyperstars.iter().all(|frame| frame.is_null()));
    assert!(game.kernel.cmaps.iter().all(Option::is_none));
    uqm_rust::engine::battle::hyperspace::free_hyperspace(&mut game.kernel);
    assert!(game.kernel.stars_in_space.is_null());
}

#[test]
fn encounter_generation_keeps_forced_melnorme_draws_and_one_shot_suppression() {
    use uqm_rust::{
        comm::dispatch::ONE_SHOT_ENCOUNTER,
        engine::{
            species::ship::MELNORME_SHIP, world::activity::activity_flags::IN_BATTLE,
            world::story::StoryState, StoryKey,
        },
        navigation::{hyper_units::Geometry, hyperspace::check_hyperspace_encounter},
    };
    let mut game = Game::new();
    game.world.story = Some(StoryState::new());
    game.world.set_activity(IN_BATTLE);
    let geometry = Geometry::new((320, 240));
    game.world.data.flagship.log_x = geometry.log_x(1752);
    game.world.data.flagship.log_y = geometry.log_y(1450);
    game.world.data.flagship.fuel_on_board = 100;
    for race in 0..24 {
        let mut fleets = game.world.avail_races();
        let id = fleets.alloc().unwrap();
        if race == MELNORME_SHIP {
            fleets.get_mut(id).actual_strength = u16::MAX;
        }
        fleets.put(id);
    }
    game.world.set_story_state(StoryKey::USED_BROADCASTER, 1);
    let mut draws = 0;
    let mut random = || {
        draws += 1;
        99
    };
    check_hyperspace_encounter(&mut game.world, geometry, &mut random);
    assert_eq!(game.world.story_state(StoryKey::USED_BROADCASTER), 0);
    assert_eq!(game.world.story_state(StoryKey::BROADCASTER_RESPONSE), 1);
    let queue = game.world.encounters_ref();
    assert_eq!(queue.count(), 1);
    let encounter = *queue.get(queue.head().unwrap());
    assert_eq!(encounter.race_id, MELNORME_SHIP);
    assert_eq!(encounter.flags, ONE_SHOT_ENCOUNTER);
    assert_eq!(encounter.radius, 20000);
    game.world.set_story_state(StoryKey::USED_BROADCASTER, 1);
    check_hyperspace_encounter(&mut game.world, geometry, &mut random);
    assert_eq!(game.world.encounters_ref().count(), 1);
    assert_eq!(
        draws, 2,
        "duplicate suppression does not skip the original random draw"
    );
    {
        let mut queue = game.world.encounters();
        let first = queue.head().unwrap();
        queue.get_mut(first).race_id = 0;
        while let Some(id) = queue.alloc() {
            queue.get_mut(id).race_id = 0;
            queue.put(id);
        }
    }
    game.world.set_story_state(StoryKey::USED_BROADCASTER, 1);
    game.world
        .set_story_state(StoryKey::BROADCASTER_RESPONSE, 0);
    let mut full_pool_draws = 0;
    check_hyperspace_encounter(&mut game.world, geometry, &mut || {
        full_pool_draws += 1;
        99
    });
    assert_eq!(full_pool_draws, 1, "pool exhaustion follows the RNG draw");
    assert_eq!(game.world.encounters_ref().count(), 16);
    assert!(game
        .world
        .encounters_ref()
        .find(|encounter| encounter.race_id != 0)
        .is_none());
    assert_eq!(game.world.story_state(StoryKey::USED_BROADCASTER), 0);
    assert_eq!(game.world.story_state(StoryKey::BROADCASTER_RESPONSE), 1);
}

#[test]
#[serial_test::serial]
fn owned_entry_keeps_display_order_resource_swap_and_quasispace_state() {
    use uqm_rust::{
        engine::world::activity::activity_flags::CHECK_LOAD,
        engine::{
            battle::{Battle, Cx},
            world::story::StoryState,
            StoryKey,
        },
        graphics::native_abi::POINT,
        graphics::{
            context_abi::{CreateContextAux, DestroyContext},
            drawable_abi::{AllocDrawable, DestroyDrawable},
            pixmap_abi::{CaptureDrawable, ReleaseDrawable},
        },
        navigation::hyper_data::{load_hyperspace, EntryDisplay},
    };
    for activity in [0, CHECK_LOAD, CHECK_LOAD | 3] {
        for side in [0, 3] {
            let mut game = Game::new();
            game.world.story = Some(StoryState::new());
            game.world
                .set_story_state(StoryKey::ARILOU_SPACE_SIDE, side);
            game.world.set_story_state(StoryKey::USED_BROADCASTER, 1);
            game.world
                .set_story_state(StoryKey::BROADCASTER_RESPONSE, 1);
            game.world.hyper_distance.dx = 123;
            game.world.data.game.autopilot = POINT { x: -1, y: 200 };
            game.kernel.last_activity = activity;
            // SAFETY: this serialized fixture owns every registered allocation until teardown.
            unsafe {
                game.kernel.hyperstars =
                    std::array::from_fn(|_| CaptureDrawable(AllocDrawable(1).cast()));
                game.kernel.stars_in_space = CaptureDrawable(AllocDrawable(1).cast());
                #[cfg(feature = "debug-process")]
                {
                    game.kernel.radar_context = CreateContextAux(c"hyper-entry-radar".as_ptr());
                    game.kernel.space_context = CreateContextAux(c"hyper-entry-space".as_ptr());
                }
                #[cfg(not(feature = "debug-process"))]
                {
                    game.kernel.radar_context = CreateContextAux();
                    game.kernel.space_context = CreateContextAux();
                }
            }
            let ambient = game.kernel.hyperstars[0];
            let stars = game.kernel.stars_in_space;
            let mut battle = Battle::new();
            let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
                panic!("entry RNG")
            });
            let mut display = Vec::new();
            load_hyperspace(&mut cx, |cx, operation| {
                assert_eq!(cx.kernel.stars_in_space, ambient);
                assert_eq!(cx.kernel.hyperstars[0], stars);
                assert_eq!(cx.world.hyper_distance.dx, 0);
                assert_eq!(cx.world.hyper_distance.ticks, 1);
                display.push(operation);
            });
            let mut expected = match activity {
                0 => vec![EntryDisplay::RepairBorder],
                value if value == CHECK_LOAD => vec![EntryDisplay::DrawFrame],
                _ => vec![EntryDisplay::DrawGauges, EntryDisplay::RepairBorder],
            };
            expected.extend([
                EntryDisplay::ClearMessage,
                EntryDisplay::Palette(side > 1),
                EntryDisplay::ClearRadar,
            ]);
            assert_eq!(display, expected);
            assert_eq!(cx.world.data.game.ship_stamp.origin.x, -9999);
            assert_eq!(cx.world.data.game.ship_stamp.origin.y, -9999);
            assert_eq!(
                cx.world.story_state(StoryKey::USED_BROADCASTER),
                u32::from(side == 0)
            );
            assert_eq!(
                cx.world.story_state(StoryKey::BROADCASTER_RESPONSE),
                u32::from(side == 0)
            );
            uqm_rust::engine::battle::hyperspace::free_hyperspace(cx.kernel);
            assert_eq!(cx.kernel.stars_in_space, stars);
            hyper_data::free_hyper_data(cx.kernel);
            // SAFETY: entry leases have dropped; the fixture releases its remaining context/star owners.
            unsafe {
                DestroyDrawable(ReleaseDrawable(cx.kernel.stars_in_space));
                DestroyContext(cx.kernel.radar_context);
                DestroyContext(cx.kernel.space_context);
            }
        }
    }
}

#[test]
#[serial_test::serial]
fn entry_animation_restores_the_real_flagship_hooks_and_saved_facing() {
    flagship_entry(false);
}

#[test]
#[serial_test::serial]
fn loaded_flagship_keeps_its_hooks_and_skips_the_entry_animation() {
    flagship_entry(true);
}

fn flagship_entry(loading: bool) {
    use uqm_rust::{
        battle::{
            element::{APPEARING, NONSOLID, PLAYER_SHIP},
            ship_native::{ship_postprocess, ship_preprocess},
        },
        engine::battle::{
            views::{FrameSelection, Frames, SharedFrames},
            Battle, Cx, ElementHook,
        },
        engine::world::activity::{activity_flags::CHECK_LOAD, ActivityKind},
        graphics::{
            drawable_abi::{AllocDrawable, DestroyDrawable},
            pixmap_abi::CaptureDrawable,
        },
        navigation::hyper_transition::{hyper_transition, save_sis_hyper_state},
        ships::types::SpeciesId,
    };
    const IN_HYPERSPACE: u16 = ActivityKind::InHyperspace as u16;
    let mut game = Game::new();
    game.world.data.game.ship_facing = 1;
    if loading {
        game.kernel.last_activity = CHECK_LOAD | IN_HYPERSPACE;
    }
    game.kernel.reset_element_capacity(1);
    game.kernel.reset_ship_capacity(0, 1);
    let ship = game
        .kernel
        .ships(0)
        .build(SpeciesId::SisShip as u32)
        .unwrap_or_else(|| panic!("flagship fixture ship capacity exhausted"));
    let ship = game.kernel.ships(0).ship_id(ship);
    // SAFETY: the serialized fixture owns all animation frame records and their drawable.
    let drawable = unsafe {
        let drawable = AllocDrawable(32);
        (*drawable).MaxIndex = 31;
        for index in 0..32 {
            (*(*drawable).Frame.add(index)).Index = index as u16;
        }
        drawable
    };
    // SAFETY: the drawable remains owned until the context releases its element frames.
    let base = unsafe { CaptureDrawable(drawable.cast()) };
    let mut battle = Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("transition RNG")
    });
    let element = cx
        .alloc_element()
        .unwrap_or_else(|| panic!("flagship fixture element capacity exhausted"));
    cx.set_parent(element, Some(ship));
    cx.set_ship_element(ship, Some(element));
    cx.set_single_frames(
        element,
        Frames::retain_shared(SharedFrames::Flagship, &[base]),
        FrameSelection::retain(base),
    );
    cx.set_frame(element, false, FrameSelection::retain(base));
    let pre = ElementHook::Native(ship_preprocess);
    let post = ElementHook::Native(ship_postprocess);
    cx.element_mut(element).pre = Some(pre);
    cx.element_mut(element).post = Some(post);
    cx.element_mut(element).flags = APPEARING | PLAYER_SHIP;
    cx.copy_current_to_next(element);
    assert_eq!(hyper_transition(&mut cx, element), !loading);
    if loading {
        assert_eq!(cx.kernel.last_activity, IN_HYPERSPACE);
        assert!(cx.kernel.hyperspace_ship_hooks.0.is_none());
        assert!(cx.kernel.hyperspace_ship_hooks.1.is_none());
    } else {
        assert!(cx.element(element).post.is_none());
        assert_eq!(
            cx.frame_number(cx.element(element).next.image.frame.view()),
            31
        );
        cx.element_mut(element).flags &= !APPEARING;
        for _ in 0..16 {
            cx.copy_next_to_current(element);
            assert!(hyper_transition(&mut cx, element));
        }
    }
    assert_eq!(
        cx.element(element).pre.map(ElementHook::address),
        Some(pre.address())
    );
    assert_eq!(
        cx.element(element).post.map(ElementHook::address),
        Some(post.address())
    );
    assert_eq!(cx.element(element).flags & NONSOLID, 0);
    save_sis_hyper_state(&mut cx);
    assert_eq!(cx.world.data.game.ship_facing, 1);
    cx.set_ship_element(ship, None);
    cx.dispose(element);
    // SAFETY: no element retains an animation view after disposal.
    unsafe {
        DestroyDrawable(drawable.cast());
    }
}

fn load_palette_fixture(
) -> Result<[Option<uqm_rust::graphics::palette_cycle::PaletteCycle>; 2], Box<dyn std::error::Error>>
{
    use std::{ffi::CString, ptr};
    use uqm_rust::{
        graphics::palette_cycle::PaletteCycle,
        resource::ffi_bridge::{InitResourceSystem, LoadResourceIndex},
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&root)?;
    let directory = tempfile::tempdir_in(root)?;
    for channel in [12u8, 90] {
        let mut bytes = Vec::new();
        for word in [u32::MAX, 1, 0, 770] {
            bytes.extend_from_slice(&word.to_be_bytes());
        }
        bytes.extend_from_slice(&[17, 17]);
        bytes.extend(std::iter::repeat_n(channel, 768));
        std::fs::write(directory.path().join(format!("{channel}.ct")), bytes)?;
    }
    std::fs::write(
        directory.path().join("palettes.rmp"),
        "fixture.hyper = BINTAB:12.ct\nfixture.quasi = BINTAB:90.ct\n",
    )?;
    let host = CString::new(
        directory
            .path()
            .canonicalize()?
            .to_str()
            .ok_or("fixture path is not UTF-8")?,
    )?;
    // SAFETY: this serialized fixture restores the content directory before closing its own mount.
    unsafe {
        let content = uqm_engine::test_support::native_session::ContentMount::new(
            &directory.path().canonicalize()?,
        );
        InitResourceSystem();
        content.load_index("palettes.rmp");
        let palettes = [
            Some(
                PaletteCycle::load(c"fixture.hyper")
                    .ok_or("hyperspace fixture palette failed to load")?,
            ),
            Some(
                PaletteCycle::load(c"fixture.quasi")
                    .ok_or("quasispace fixture palette failed to load")?,
            ),
        ];
        drop(content);
        Ok(palettes)
    }
}

#[test]
#[serial_test::serial]
fn owned_palette_selection_uses_the_supplied_tables_and_status_context(
) -> Result<(), Box<dyn std::error::Error>> {
    use uqm_rust::{
        engine::battle::hyperspace as resources,
        graphics::{cmap_abi::*, context_abi::*, native_abi::Color},
    };
    let mut game = Game::new();
    game.kernel.cmaps = load_palette_fixture()?;
    let select: fn(&uqm_rust::engine::Kernel, bool) = resources::select_palette;
    // SAFETY: the fixture owns the context and all acquired palette references.
    unsafe {
        InitColorMaps();
        for (side, channel) in [12u8, 90].into_iter().enumerate() {
            select(&game.kernel, side != 0);
            let selected = TFB_GetColorMap(17);
            assert!((0..256).all(|index| {
                let color = uqm_rust::graphics::sdl::palette::GetNativePaletteColor(
                    (*selected).palette,
                    index,
                );
                (color.r, color.g, color.b) == (channel, channel, channel)
            }));
            TFB_ReturnColorMap(selected);
        }
        #[cfg(feature = "debug-process")]
        let context = CreateContextAux(c"hyperspace status fixture".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let context = CreateContextAux();
        game.kernel.status_context = context;
        let retained = resources::status_context(&game.kernel);
        retained.select();
        retained.set_background(Color {
            r: 12,
            g: 34,
            b: 56,
            a: 255,
        });
        let selected = SetContextBackGroundColor(Color::default());
        assert_eq!((selected.r, selected.g, selected.b), (12, 34, 56));
        drop(retained);
        SetContext(std::ptr::null_mut());
        DestroyContext(context);
        game.kernel.status_context = std::ptr::null_mut();
        resources::free_hyper_data(&mut game.kernel);
        assert!(game.kernel.cmaps.iter().all(Option::is_none));
        UninitColorMaps();
    }
    Ok(())
}
