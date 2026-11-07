use uqm_rust::engine::world::state_file::FileMode;
use uqm_rust::engine::{Game, StoryKey};

#[test]
fn separate_games_own_story_state_files_and_kernel_resources() {
    let mut first = Game::new();
    first.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    let second = Game::new();
    first.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 2);
    first.world.data.flagship.crew_enlisted = 17;
    first.kernel.screen_width = 320;
    first.world.files.open(0, FileMode::Write).unwrap();
    first
        .world
        .files
        .get_file_mut(0)
        .unwrap()
        .write(b"owned")
        .unwrap();
    assert_eq!(first.world.story_state(StoryKey::ARILOU_SPACE_SIDE), 2);
    assert_eq!(second.world.story_state(StoryKey::ARILOU_SPACE_SIDE), 0);
    assert_eq!(second.world.data.flagship.crew_enlisted, 0);
    assert_eq!(second.kernel.screen_width, 0);
    assert_eq!(second.world.files.get_file(0).unwrap().length(), 0);
}

#[test]
fn typed_keys_keep_the_existing_serialized_bit_ranges() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    game.world.set_story_state(StoryKey::SHOFIXTI_RECRUITED, 1);
    game.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 3);
    game.world.set_story_state(StoryKey::SAMATRA_GRPOFFS3, 0xab);
    assert_eq!(game.world.story.as_ref().unwrap().get_state(12, 12), 1);
    assert_eq!(game.world.story.as_ref().unwrap().get_state(42, 43), 3);
    assert_eq!(
        game.world.story.as_ref().unwrap().get_state(1230, 1237),
        0xab
    );
}

#[test]
fn generated_story_keys_cover_the_existing_table_without_changing_bits() {
    let mut game = Game::new();
    game.world.story = Some(uqm_rust::state::game_state::StoryState::new());
    for &(name, start, width) in uqm_rust::state::game_state::STATE_KEYS {
        let key = StoryKey::from_name(name).unwrap();
        assert_eq!(key.start(), start as usize, "{name}");
        assert_eq!(key.end(), (start + width - 1) as usize, "{name}");
        game.world.story.as_mut().unwrap().reset();
        let expected = (1u32 << width) - 1;
        game.world.set_story_state(key, expected);
        assert_eq!(game.world.story_state(key), expected, "{name}");
        assert_eq!(
            game.world
                .story
                .as_ref()
                .unwrap()
                .get_state(key.start(), key.end()) as u32,
            expected
        );
    }
    assert!(StoryKey::from_name("NOT_A_STORY_KEY").is_none());
    assert!(StoryKey::from_name("").is_none());
}

#[test]
fn current_star_is_a_catalog_identity_and_foreign_pointers_are_rejected() {
    use uqm_rust::engine::world::{CStarDesc, CATALOG_LEN};
    let mut game = Game::new();
    let star = CStarDesc {
        star_pt: uqm_rust::graphics::native_abi::POINT { x: 0, y: 0 },
        star_type: 0,
        index: 0,
        prefix: 0,
        postfix: 0,
    };
    let catalog = Box::new([star; CATALOG_LEN]);
    game.world
        .select_star(Some(&catalog[CATALOG_LEN - 1]), Some(&catalog));
    assert_eq!(
        game.world.star_pointer(Some(&catalog)),
        std::ptr::from_ref(&catalog[CATALOG_LEN - 1]).cast_mut()
    );
    let selected = game.world.current_star;
    let foreign = star;
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        game.world.select_star(Some(&foreign), Some(&catalog));
    }));
    assert!(rejected.is_err());
    assert_eq!(game.world.current_star, selected);
    game.world.select_star(None, None);
    assert_eq!(game.world.current_star, None);
}

#[test]
#[serial_test::serial]
fn new_campaign_resets_story_without_discarding_owned_state_files() {
    let mut game = Game::new();
    game.world.files.open(0, FileMode::Write).unwrap();
    game.world
        .files
        .get_file_mut(0)
        .unwrap()
        .write(b"retained")
        .unwrap();
    // SAFETY: the local owner is published only for each synchronous entry.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::engine::World::current().write(uqm_rust::state::ffi::rust_init_game_state)
        })
    };
    game.world.set_story_state(StoryKey::SHOFIXTI_RECRUITED, 1);
    // SAFETY: no owner borrow survives the synchronous bridge call.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || {
            uqm_rust::engine::World::current().write(uqm_rust::state::ffi::rust_init_game_state)
        })
    };
    assert_eq!(game.world.story_state(StoryKey::SHOFIXTI_RECRUITED), 0);
    assert_eq!(game.world.files.get_file(0).unwrap().length(), 8);
}

#[test]
fn owned_story_operations_need_no_published_legacy_context() {
    use uqm_rust::state::ffi::*;
    let mut game = uqm_rust::engine::Game::new();
    rust_init_game_state(&mut game.world);
    rust_set_game_state_bits(&mut game.world, 0, 2, 5);
    assert_eq!(rust_get_game_state_bits(&game.world, 0, 2), 5);
    rust_set_game_state_32(&mut game.world, 32, 0x1234_abcd);
    assert_eq!(rust_get_game_state_32(&game.world, 32), 0x1234_abcd);
    rust_copy_game_state(&mut game.world, 8, 0, 2);
    assert_eq!(rust_get_game_state_bits(&game.world, 8, 10), 5);
    rust_reset_game_state(&mut game.world);
    assert_eq!(rust_get_game_state_32(&game.world, 32), 0);
}
