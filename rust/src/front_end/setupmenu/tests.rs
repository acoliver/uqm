use super::*;
mod native;
mod options;

#[test]
fn choice_and_volume_fields_round_trip_without_the_edit_template_choice() {
    let choices = std::array::from_fn(|index| index % 3);
    let sliders = [25, 50, 75, 42];
    let options = GlobalOptions::from_widgets(&choices, sliders);
    let mut actual = options.choices();
    actual[20] = choices[20];
    assert_eq!(actual, choices);
    assert_eq!(options.sliders(), sliders);
}

#[test]
fn setup_focus_skips_labels_and_keeps_binding_column_only_between_controls() {
    let mut menu = MenuState::new([8; 24]);
    menu.enter(7, true);
    assert_eq!(menu.focus[7], 0);
    assert_eq!(menu.event(Event::Down), Action::None);
    assert_eq!(menu.focus[7], 2);
    menu.event(Event::Down);
    menu.event(Event::Right);
    assert_eq!((menu.focus[7], menu.binding_column), (3, 1));
    menu.event(Event::Down);
    assert_eq!((menu.focus[7], menu.binding_column), (4, 1));
    menu.event(Event::Up);
    menu.event(Event::Up);
    menu.event(Event::Down);
    assert_eq!(menu.binding_column, 0);
}

#[test]
fn cancel_returns_to_parent_and_root_cancel_applies_instead_of_reverting() {
    let mut menu = MenuState::new([8; 24]);
    menu.enter(1, true);
    assert_eq!(menu.event(Event::Cancel), Action::Enter(0, false));
    menu.enter(0, false);
    assert_eq!(menu.event(Event::Cancel), Action::Finish);
}

#[test]
fn independent_sessions_reset_focus_and_distinguish_highlight_from_selection() {
    let mut first = MenuState::new([8; 24]);
    first.enter(1, true);
    first.highlight[0] = 3;
    assert_eq!(first.choices[0], 0);
    assert_eq!(first.event(Event::Select), Action::ChoiceChanged(0));
    assert_eq!(first.choices[0], 3);
    let second = MenuState::new([8; 24]);
    assert_eq!(
        (second.choices[0], second.highlight[0], second.focus[1]),
        (0, 0, 0)
    );
}

#[test]
fn setup_string_splitting_preserves_empty_last_line_and_final_unsplit_remainder() {
    assert_eq!(
        split_lines(c"a\nb\n", 100),
        [c"a".to_owned(), c"b".to_owned(), c"".to_owned()]
    );
    assert_eq!(
        split_lines(c"a\nb\nc", 2),
        [c"a".to_owned(), c"b\nc".to_owned()]
    );
}

#[test]
fn every_binding_row_routes_select_delete_and_keeps_both_columns_in_bounds() {
    let mut menu = MenuState::new([7; 24]);
    menu.enter(7, true);
    for control in 0..7 {
        menu.focus[7] = control + 3;
        for column in 0..2 {
            menu.binding_column = column;
            assert_eq!(menu.event(Event::Select), Action::Rebind(control));
            assert_eq!(menu.event(Event::Delete), Action::Clear(control));
            for _ in 0..4 {
                menu.event(Event::Right);
                assert!(menu.binding_column < 2);
            }
            for _ in 0..4 {
                menu.event(Event::Left);
                assert!(menu.binding_column < 2);
            }
        }
    }
}
