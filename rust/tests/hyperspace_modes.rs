#![forbid(unsafe_code)]
use uqm_engine::Game;
use uqm_modes::navigation::hyper_data;

#[test]
fn resource_teardown_uses_only_the_supplied_kernel() {
    let mut game = Game::new();
    hyper_data::free_hyper_data(&mut game.kernel);
    assert!(game.kernel.hyperstars.iter().all(|frame| frame.is_null()));
    assert!(game.kernel.cmaps.iter().all(Option::is_none));
    uqm_engine::battle::hyperspace::free_hyperspace(&mut game.kernel);
    assert!(game.kernel.stars_in_space.is_null());
}

#[test]
fn encounter_generation_keeps_forced_melnorme_draws_and_one_shot_suppression() {
    use uqm_engine::{
        species::ship::MELNORME_SHIP, world::activity::activity_flags::IN_BATTLE,
        world::story::StoryState, StoryKey,
    };
    use uqm_modes::comm::dispatch::ONE_SHOT_ENCOUNTER;
    use uqm_modes::navigation::{hyper_units::Geometry, hyperspace::check_hyperspace_encounter};
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
    use uqm_engine::battle::Battle;
    use uqm_engine::battle::Cx;
    use uqm_engine::graphics::{context::LoadedContext, drawable::LoadedDrawable, geometry::Point};
    use uqm_engine::world::activity::activity_flags::CHECK_LOAD;
    use uqm_engine::world::story::StoryState;
    use uqm_engine::StoryKey;
    use uqm_modes::navigation::hyper_data::load_hyperspace;
    use uqm_modes::navigation::hyper_data::EntryDisplay;
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
            game.world.data.game.autopilot = Point { x: -1, y: 200 };
            game.kernel.last_activity = activity;
            let star_frame = || {
                LoadedDrawable::create(0, 1, 1, 1)
                    .unwrap_or_else(|| panic!("star fixture"))
                    .into_frame()
            };
            game.kernel.hyperstars = std::array::from_fn(|_| star_frame());
            game.kernel.stars_in_space = star_frame();
            game.kernel.radar_context = Some(LoadedContext::create(c"hyper-entry-radar").into_id());
            game.kernel.space_context = Some(LoadedContext::create(c"hyper-entry-space").into_id());
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
            uqm_engine::battle::hyperspace::free_hyperspace(cx.kernel);
            assert_eq!(cx.kernel.stars_in_space, stars);
            hyper_data::free_hyper_data(cx.kernel);
            drop(LoadedDrawable::from_frame(cx.kernel.stars_in_space));
            for context in [cx.kernel.radar_context, cx.kernel.space_context] {
                drop(context.map(LoadedContext::from_id));
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
    use uqm_content::ships::types::SpeciesId;
    use uqm_engine::battle::views::FrameSelection;
    use uqm_engine::battle::views::Frames;
    use uqm_engine::battle::views::SharedFrames;
    use uqm_engine::battle::Battle;
    use uqm_engine::battle::Cx;
    use uqm_engine::battle::ElementHook;
    use uqm_engine::graphics::drawable::LoadedDrawable;
    use uqm_engine::world::activity::activity_flags::CHECK_LOAD;
    use uqm_engine::world::activity::ActivityKind;
    use uqm_modes::battle::element::APPEARING;
    use uqm_modes::battle::element::NONSOLID;
    use uqm_modes::battle::element::PLAYER_SHIP;
    use uqm_modes::battle::ship_native::ship_postprocess;
    use uqm_modes::battle::ship_native::ship_preprocess;
    use uqm_modes::navigation::hyper_transition::hyper_transition;
    use uqm_modes::navigation::hyper_transition::save_sis_hyper_state;
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
    let drawable = LoadedDrawable::create(0, 1, 1, 32).unwrap_or_else(|| panic!("frames fixture"));
    let base = drawable.first_frame();
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
    drop(drawable);
}

fn load_palette_fixture() -> Result<
    [Option<uqm_engine::graphics::palette_cycle::PaletteCycle>; 2],
    Box<dyn std::error::Error>,
> {
    use uqm_engine::graphics::palette_cycle::PaletteCycle;
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
    {
        let content = uqm_engine::test_support::native_session::ContentMount::new(
            &directory.path().canonicalize()?,
        );
        content.load_index("palettes.rmp");
        let palettes = [
            Some(
                PaletteCycle::load(&content.content(), c"fixture.hyper")
                    .ok()
                    .ok_or("hyperspace fixture palette failed to load")?,
            ),
            Some(
                PaletteCycle::load(&content.content(), c"fixture.quasi")
                    .ok()
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
    use uqm_engine::battle::hyperspace as resources;
    use uqm_engine::graphics::{context::LoadedContext, geometry::Color};
    use uqm_engine::test_support::native_probe;
    let mut game = Game::new();
    game.kernel.cmaps = load_palette_fixture()?;
    let select: fn(&uqm_engine::Kernel, bool) = resources::select_palette;
    let _graphics = native_probe::DrawQueue::init();
    for (side, channel) in [12u8, 90].into_iter().enumerate() {
        select(&game.kernel, side != 0);
        let palette = native_probe::color_map_palette(17);
        assert!(palette
            .iter()
            .all(|color| (color.r, color.g, color.b) == (channel, channel, channel)));
    }
    let context = LoadedContext::create(c"hyperspace status fixture");
    game.kernel.status_context = Some(context.context());
    let retained = resources::status_context(&game.kernel);
    retained.select();
    retained.set_background(Color {
        r: 12,
        g: 34,
        b: 56,
        a: 255,
    });
    let selected = retained.background(Color::default());
    assert_eq!((selected.r, selected.g, selected.b), (12, 34, 56));
    drop(retained);
    uqm_engine::graphics::menu::select(None);
    drop(context);
    game.kernel.status_context = None;
    resources::free_hyper_data(&mut game.kernel);
    assert!(game.kernel.cmaps.iter().all(Option::is_none));
    Ok(())
}
