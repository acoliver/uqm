//! Stable values shared by save files, input, rendering, ships and netplay.

use std::mem::{align_of, offset_of, size_of};
use uqm_rust::c_bindings::{controller_input as input, global_state as state};
use uqm_rust::navigation::abi as nav;
use uqm_rust::ships::native::abi as ship;
use uqm_rust::state::game_state::{GAME_STATE_FIELDS, NUM_GAME_STATE_BITS, NUM_GAME_STATE_BYTES};

#[test]
fn story_state_size_and_endpoints_remain_stable() {
    assert_eq!(NUM_GAME_STATE_BITS, 1238);
    assert_eq!(NUM_GAME_STATE_BYTES, 155);
    assert_eq!(nav::NUM_GAME_STATE_BITS as usize, NUM_GAME_STATE_BITS);
    assert_eq!(ship::NUM_GAME_STATE_BITS as usize, NUM_GAME_STATE_BITS);
    assert_eq!(GAME_STATE_FIELDS.first(), Some(&("SHOFIXTI_VISITS", 0, 2)));
    assert_eq!(
        GAME_STATE_FIELDS.last(),
        Some(&("SAMATRA_GRPOFFS3", 1230, 1237))
    );
}

#[test]
fn controller_keys_and_template_numbers_remain_stable() {
    assert_eq!(input::NUM_KEYS, 7);
    assert_eq!(input::NUM_MENU_KEYS, 24);
    assert_eq!(input::CONTROL_TEMPLATE_NUM_TEMPLATES, 6);
    assert_eq!(input::KEY_ESCAPE, 6);
    assert_eq!(input::KEY_MENU_SELECT, 9);
    assert_eq!(input::KEY_MENU_CANCEL, 10);
    assert_eq!(input::KEY_MENU_ANY, 23);
    assert_eq!(size_of::<input::CONTROLLER_INPUT_STATE>(), 264);
    assert_eq!(offset_of!(input::CONTROLLER_INPUT_STATE, menu), 168);
    assert_eq!(size_of::<input::CONTROL_TEMPLATE>(), 4);
}

#[test]
fn engine_scalar_widths_and_species_ids_remain_stable() {
    assert_eq!(size_of::<state::COUNT>(), 2);
    assert_eq!(size_of::<state::BOOLEAN>(), 4);
    assert_eq!(size_of::<ship::SPECIES_ID>(), 4);
    assert_eq!(align_of::<ship::SPECIES_ID>(), 4);
    assert_eq!(ship::HUMAN_SHIP, 2);
    assert_eq!(ship::SAMATRA_SHIP, 23);
    assert_eq!(nav::PLANET_LANDER, 0);
    assert_eq!(nav::NUM_PURCHASE_MODULES, 14);
    assert_eq!(nav::BOMB_MODULE_0, 14);
    assert_eq!(nav::NUM_MODULES, 20);
}

#[test]
fn every_story_field_starts_at_its_rust_abi_enum_position() {
    // The schema macro emits compile-time checks for every entry, including
    // fields that cross bytes and the high bytes of serialized group offsets.
    assert_eq!(nav::SHOFIXTI_STACK3, 8);
    assert_eq!(nav::SAMATRA_GRPOFFS3, 1230);
}
