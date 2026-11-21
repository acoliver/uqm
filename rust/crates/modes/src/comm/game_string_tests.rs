//! Conversation screens name their fixed text by game-string index; these
//! tests resolve those indices against the shipped string table.

use uqm_engine::{state::game_state::StoryState, Game, StoryKey};

fn shipped_game_string(index: usize) -> &'static str {
    include_str!("../../../../../sc2/content/base/gamestrings.txt")
        .split("#(")
        .skip(1)
        .nth(index)
        .and_then(|entry| entry.lines().nth(1))
        .unwrap_or_else(|| panic!("game string {index} is missing"))
        .trim()
}

#[test]
fn allied_starbase_conversation_is_titled_starbase_and_starbase_commander() {
    let mut game = Game::new();
    game.world.story = Some(StoryState::new());
    game.world
        .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0xff);
    game.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);

    let title = super::hail::starbase_title(&game.world).expect("allied starbase title");
    assert_eq!(shipped_game_string(title), "Starbase");
    assert_eq!(shipped_game_string(title + 1), "Starbase Commander");

    game.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 0);
    assert_eq!(super::hail::starbase_title(&game.world), None);
}

#[test]
fn player_feedback_uses_the_response_header_and_more_prompt() {
    let base = usize::from(super::sis_graphics::FEEDBACK_STRING_BASE);
    assert_eq!(shipped_game_string(base), "(In response to your statement)");
    assert_eq!(shipped_game_string(base + 1), "MORE");
}
