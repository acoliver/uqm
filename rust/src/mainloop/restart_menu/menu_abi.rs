//! Restart-menu state operations formerly supplied by rust_bridge_restart.c.
use crate::engine::battle::schema::FlashContext;
use crate::engine::Kernel;
use crate::engine::StoryKey;
use crate::engine::World;
use crate::ships::native::abi::*;
use std::ptr;

macro_rules! operation {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block) => {
        /// # Safety
        /// Menu and resource pointers obey the game-thread provider's lifetime contract.
        pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? $body
    };
}
operation!(uqm_get_utwig_bomb_on_ship() -> u8 { World::current().read(|world| super::super::state_abi::game_byte(world, StoryKey::UTWIG_BOMB_ON_SHIP)) });
operation!(uqm_set_utwig_bomb_on_ship(value: u8) { World::current().write(|world| super::super::state_abi::set_game_byte(world, StoryKey::UTWIG_BOMB_ON_SHIP, value)); });
operation!(uqm_get_utwig_bomb() -> u8 { World::current().read(|world| super::super::state_abi::game_byte(world, StoryKey::UTWIG_BOMB)) });
pub fn uqm_set_player_control(kernel: &mut Kernel, player: u8, control: u16) {
    kernel.player_control[player as usize] = control as u8;
}
operation!(uqm_get_mouse_button_down() -> u32 { crate::input::sdl_input::MouseButtonDown as u32 });
pub use crate::time::counter_abi::GetTimeCounter as uqm_get_time_counter;
operation!(uqm_sleep_thread_until(time: u32) { crate::threading::pacing::linked::sleep_thread_until(time); });
operation!(uqm_sleep_thread(time: u32) { crate::threading::pacing::linked::sleep_thread(time); });
operation!(uqm_reinit_race_queues() {
    for side in 0..2 { crate::engine::battle::link_queue::ships(side).reinit(); }
});
operation!(uqm_assign_star_planet_globals() { crate::planets::starmap::star_array = ptr::addr_of_mut!(crate::planets::tables::STARMAP_ARRAY).cast(); });
operation!(uqm_do_popup_window_msg(runtime: &crate::engine::input::InputSession, strings: STRING, id: u16) {
    let string = crate::strings::string_table::SetAbsStringTableIndex(strings, (MAINMENU_STRING_BASE as u16).wrapping_add(id));
    crate::ui::confirm::do_popup_window(runtime, crate::strings::string_table::GetStringAddress(string));
});
operation!(uqm_get_menu_cur_frame(menu: *mut MENU_STATE) -> FRAME { (*menu).CurFrame });
operation!(uqm_set_menu_flash_context(menu: *mut MENU_STATE, context: *mut FlashContext) { (*menu).flashContext = context; });
operation!(uqm_set_menu_initialized(menu: *mut MENU_STATE, value: i16) { (*menu).Initialized = value; });
operation!(uqm_set_menu_cur_state(menu: *mut MENU_STATE, state: u8) { (*menu).CurState = state; });
operation!(uqm_set_menu_cur_frame(menu: *mut MENU_STATE, frame: FRAME) { (*menu).CurFrame = frame; });
operation!(uqm_set_menu_h_music(menu: *mut MENU_STATE, music: MUSIC_REF) { (*menu).hMusic = music; });
operation!(uqm_create_menu_state() -> *mut MENU_STATE {
    let menu: Box<MENU_STATE> = Box::new(std::mem::zeroed());
    Box::into_raw(menu)
});
operation!(uqm_destroy_menu_state(menu: *mut MENU_STATE) { if !menu.is_null() { drop(Box::from_raw(menu)); } });
