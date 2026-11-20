use super::picker::{PickerAction, PickerState, MAX_SAVED_GAMES};
use crate::engine::save::summary_record::SummaryDesc;

#[test]
fn independent_pickers_use_prepared_summaries_and_remember_only_the_supplied_slot() {
    let mut summaries = [SummaryDesc::default(); MAX_SAVED_GAMES];
    summaries[7].year_index = 2155;
    let mut loading = PickerState::new(summaries, false, 7);
    let saving = PickerState::new([SummaryDesc::default(); MAX_SAVED_GAMES], true, 12);
    assert_eq!(loading.action(false, false, true), PickerAction::Select);
    assert_eq!(saving.action(false, false, true), PickerAction::Select);
    assert_eq!(loading.slot, 7);
    loading.navigate(false, true, false, false);
    assert_eq!(loading.slot, 12);
    assert_eq!(loading.action(false, false, true), PickerAction::EmptySlot);
    assert_eq!(saving.slot, 12);
    assert!(!loading.success);
    assert!(!saving.success);
}

#[test]
fn prepared_picker_preserves_cancel_abort_and_current_page_priority() {
    let mut state = PickerState::new([SummaryDesc::default(); MAX_SAVED_GAMES], false, 49);
    assert_eq!(state.action(true, true, true), PickerAction::Abort);
    assert_eq!(state.action(false, true, true), PickerAction::Cancel);
    assert!(!state.navigate(false, false, false, false));
    assert!(state.navigate(false, true, true, true));
    assert_eq!(state.slot, 0);
    assert!(state.navigate(true, false, false, false));
    assert_eq!(state.slot, 49);
}
