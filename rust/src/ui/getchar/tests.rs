use super::*;

#[test]
fn utf8_edits_keep_character_cursor_and_bounded_terminated_storage() {
    let mut buffer = *b"a\xc3\xa9\0\0\0\0\0";
    let mut editor = Editor::new(&mut buffer, 7, 99);
    assert_eq!(editor.view().cursor, 2);
    assert!(editor.insert('界'));
    assert_eq!(editor.view().text.to_bytes(), "aé界".as_bytes());
    assert!(!editor.insert('x'));
    editor.move_left();
    assert!(editor.delete());
    assert_eq!(editor.view().text.to_bytes(), "aé".as_bytes());
    assert!(editor.backspace());
    assert_eq!(editor.view().text, c"a");
    assert_eq!(editor.view().cursor, 1);
}

#[test]
fn rejected_edit_restores_text_and_cursor() {
    let mut buffer = *b"ab\0\0\0\0\0\0";
    let mut editor = Editor::new(&mut buffer, 8, 0);
    editor.save();
    editor.move_right();
    assert!(editor.insert('é'));
    editor.restore();
    assert_eq!(editor.view().text, c"ab");
    assert_eq!(editor.view().cursor, 0);
}

#[test]
fn joystick_wraps_alphabet_and_changes_case_without_advancing_cursor() {
    let mut buffer = [0; 8];
    let mut editor = Editor::new(&mut buffer, 8, 0);
    let alphabet = Alphabet::new(c"AÉZ", c"AÉ", c"aé");
    assert_eq!(editor.joystick(&alphabet, false, true, false), Ok(true));
    assert_eq!(editor.view().text, c"É");
    assert_eq!(editor.view().cursor, 0);
    assert!(editor.view().joystick);
    assert_eq!(editor.joystick(&alphabet, false, false, true), Ok(true));
    assert_eq!(editor.view().text, c"é");
    assert_eq!(editor.joystick(&alphabet, true, false, false), Ok(true));
    assert_eq!(editor.view().text, c"a");
    assert_eq!(editor.joystick(&alphabet, true, false, false), Ok(true));
    assert_eq!(editor.view().text, c"Z");
}

#[test]
fn insertion_capacity_includes_the_terminator_and_storage_requires_one() {
    let mut buffer = *b"ab\0extra";
    let mut editor = Editor::new(&mut buffer, 3, 2);
    assert!(!editor.insert('c'));
    assert_eq!(editor.view().text, c"ab");
    assert!(std::panic::catch_unwind(|| {
        let mut buffer = *b"abcd";
        Editor::new(&mut buffer, 3, 0);
    })
    .is_err());
}

#[test]
fn insertion_limit_preserves_longer_initial_text_and_allows_deletion_and_rollback() {
    let mut buffer = *b"abcd\0\0";
    let mut editor = Editor::new(&mut buffer, 3, 4);
    assert_eq!(editor.view().text, c"abcd");
    editor.save();
    assert!(!editor.insert('é'));
    assert!(editor.backspace());
    assert_eq!(editor.view().text, c"abc");
    assert!(!editor.insert('x'));
    assert!(editor.backspace());
    assert!(!editor.insert('x'));
    assert!(editor.backspace());
    assert!(!editor.insert('é'));
    assert!(editor.insert('x'));
    assert_eq!(editor.view().text, c"ax");
    editor.restore();
    assert_eq!(editor.view().text, c"abcd");
    assert_eq!(editor.view().cursor, 4);
}

#[test]
fn joystick_distinguishes_a_full_append_from_an_unchanged_existing_character() {
    let alphabet = Alphabet::new(c"AÉZ", c"AÉ", c"aé");
    let mut buffer = *b"a\0";
    let mut editor = Editor::new(&mut buffer, 2, 1);
    assert_eq!(editor.joystick(&alphabet, false, true, false), Err(()));
    assert_eq!(editor.view().text, c"a");
    editor.move_left();
    assert_eq!(editor.joystick(&alphabet, false, true, false), Ok(false));
    assert_eq!(editor.view().text, c"a");
}

#[test]
fn cursor_keys_use_character_boundaries_and_keep_their_original_precedence() {
    let mut buffer = [0; 16];
    buffer[..7].copy_from_slice("aé界\0".as_bytes());
    let mut editor = Editor::new(&mut buffer, 16, 0);
    let mut press = |key| {
        let mut pulse = InputState::default();
        pulse.menu[key as usize] = 1;
        editor.cursor_keys(&pulse)
    };
    assert!(press(KEY_MENU_RIGHT));
    assert!(press(KEY_MENU_END));
    assert!(press(KEY_MENU_LEFT));
    assert!(press(KEY_MENU_BACKSPACE));
    assert!(press(KEY_MENU_DELETE));
    assert!(press(KEY_MENU_HOME));
    assert!(!press(KEY_MENU_LEFT));
    assert!(!press(KEY_MENU_BACKSPACE));
    assert_eq!(editor.view().text, c"a");
    assert_eq!(editor.view().cursor, 0);
    let mut pulse = InputState::default();
    pulse.menu[KEY_MENU_DELETE as usize] = 1;
    pulse.menu[KEY_MENU_RIGHT as usize] = 1;
    assert!(editor.cursor_keys(&pulse));
    assert_eq!(editor.view().text, c"");
    assert_eq!(editor.view().cursor, 0);
    assert!(!editor.cursor_keys(&pulse));
}
