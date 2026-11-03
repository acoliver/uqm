use uqm_rust::engine::{Game, StoryKey};
use uqm_rust::state::state_file::FileMode;

#[test]
fn separate_games_own_story_state_files_and_kernel_resources() {
    let mut first = Game::new();
    first.world.story = Some(uqm_rust::state::game_state::GameState::new());
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
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
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
    game.world.story = Some(uqm_rust::state::game_state::GameState::new());
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
    // SAFETY: all fields are integers, and this test retains the whole catalog.
    let mut catalog = Box::new([unsafe { std::mem::zeroed::<CStarDesc>() }; CATALOG_LEN]);
    let base = catalog.as_mut_ptr();
    // SAFETY: the selected entry belongs to the retained catalog.
    unsafe {
        game.world.select_star(
            (base.add(CATALOG_LEN - 1)).as_ref(),
            (base as *const [uqm_rust::engine::world::CStarDesc;
                uqm_rust::engine::world::CATALOG_LEN])
                .as_ref(),
        );
        assert_eq!(
            game.world.star_pointer(
                (base as *const [uqm_rust::engine::world::CStarDesc;
                    uqm_rust::engine::world::CATALOG_LEN])
                    .as_ref()
            ),
            base.add(CATALOG_LEN - 1)
        );
    }
    let selected = game.world.current_star;
    // SAFETY: validation checks identity before any pointer dereference.
    let mut foreign: CStarDesc = unsafe { std::mem::zeroed() };
    let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        game.world.select_star(
            (&mut foreign).as_ref(),
            (base as *const [uqm_rust::engine::world::CStarDesc;
                uqm_rust::engine::world::CATALOG_LEN])
                .as_ref(),
        );
    }));
    assert!(rejected.is_err());
    assert_eq!(game.world.current_star, selected);
    // SAFETY: null clears the selection without dereferencing the catalog.
    unsafe {
        game.world.select_star(
            None,
            (base as *const [uqm_rust::engine::world::CStarDesc;
                uqm_rust::engine::world::CATALOG_LEN])
                .as_ref(),
        )
    };
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
        uqm_rust::app::bridge::with_game(&mut game, || uqm_rust::state::ffi::rust_init_game_state())
    };
    game.world.set_story_state(StoryKey::SHOFIXTI_RECRUITED, 1);
    // SAFETY: no owner borrow survives the synchronous bridge call.
    unsafe {
        uqm_rust::app::bridge::with_game(&mut game, || uqm_rust::state::ffi::rust_init_game_state())
    };
    assert_eq!(game.world.story_state(StoryKey::SHOFIXTI_RECRUITED), 0);
    assert_eq!(game.world.files.get_file(0).unwrap().length(), 8);
}
