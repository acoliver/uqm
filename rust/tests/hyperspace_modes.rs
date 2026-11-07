use uqm_rust::{engine::Game, navigation::hyper_data};

#[test]
fn resource_teardown_uses_only_the_supplied_kernel() {
    let mut game = Game::new();
    hyper_data::free_hyper_data(&mut game.kernel);
    assert!(game.kernel.hyperstars.iter().all(|frame| frame.is_null()));
    assert!(game.kernel.cmaps.iter().all(|map| map.is_null()));
    hyper_data::free_hyperspace(&mut game.kernel);
    assert!(game.kernel.stars_in_space.is_null());
}

#[test]
fn encounter_generation_keeps_forced_melnorme_draws_and_one_shot_suppression() {
    use uqm_rust::{
        engine::{world::story::GameState, StoryKey},
        navigation::{abi::*, hyper_units::Geometry, hyperspace::check_hyperspace_encounter},
    };
    let mut game = Game::new();
    game.world.story = Some(GameState::new());
    game.world.set_activity(IN_BATTLE as u16);
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
    assert_eq!(encounter.race_id, MELNORME_SHIP as u8);
    assert_eq!(encounter.flags, ONE_SHOT_ENCOUNTER as u8);
    assert_eq!(encounter.radius, 20000);
    game.world.set_story_state(StoryKey::USED_BROADCASTER, 1);
    check_hyperspace_encounter(&mut game.world, geometry, &mut random);
    assert_eq!(game.world.encounters_ref().count(), 1);
    assert_eq!(
        draws, 2,
        "duplicate suppression does not skip the original random draw"
    );
}

#[test]
#[serial_test::serial]
fn owned_entry_keeps_display_order_resource_swap_and_quasispace_state() {
    use uqm_rust::{
        engine::{
            battle::{Battle, Cx},
            world::story::GameState,
            StoryKey,
        },
        graphics::{
            context_abi::{CreateContextAux, DestroyContext},
            drawable_abi::{AllocDrawable, DestroyDrawable},
            pixmap_abi::{CaptureDrawable, ReleaseDrawable},
        },
        navigation::{
            abi::{CHECK_LOAD, POINT},
            hyper_data::{load_hyperspace, EntryDisplay},
        },
    };
    for activity in [0, CHECK_LOAD as u16, CHECK_LOAD as u16 | 3] {
        for side in [0, 3] {
            let mut game = Game::new();
            game.world.story = Some(GameState::new());
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
                value if value == CHECK_LOAD as u16 => vec![EntryDisplay::DrawFrame],
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
            hyper_data::free_hyperspace(cx.kernel);
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
    use uqm_rust::{
        engine::battle::{
            views::{FrameSelection, Frames, SharedFrames},
            Battle, Cx, ElementHook,
        },
        graphics::{
            drawable_abi::{AllocDrawable, DestroyDrawable},
            pixmap_abi::CaptureDrawable,
        },
        navigation::{
            abi::*,
            hyper_transition::{hyper_transition, save_sis_hyper_state},
        },
    };
    let mut game = Game::new();
    game.world.data.game.ship_facing = 1;
    game.kernel.reset_element_capacity(1);
    game.kernel.reset_ship_capacity(0, 1);
    let ship = game.kernel.ships(0).build(SPECIES_ID_SIS_SHIP_ID).unwrap();
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
    let element = cx.alloc_element().unwrap();
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
    cx.element_mut(element).flags = (APPEARING | PLAYER_SHIP) as u16;
    assert!(hyper_transition(&mut cx, element));
    assert!(cx.element(element).post.is_none());
    assert_eq!(
        cx.frame_number(cx.element(element).next.image.frame.view()),
        31
    );
    cx.element_mut(element).flags &= !(APPEARING as u16);
    for _ in 0..16 {
        cx.copy_next_to_current(element);
        assert!(hyper_transition(&mut cx, element));
    }
    assert_eq!(cx.element(element).pre.unwrap().address(), pre.address());
    assert_eq!(cx.element(element).post.unwrap().address(), post.address());
    assert_eq!(cx.element(element).flags & NONSOLID as u16, 0);
    save_sis_hyper_state(&mut cx);
    assert_eq!(cx.world.data.game.ship_facing, 1);
    cx.set_ship_element(ship, None);
    cx.dispose(element);
    // SAFETY: no element retains an animation view after disposal.
    unsafe {
        DestroyDrawable(drawable.cast());
    }
}
