//! Restart-menu state operations formerly supplied by rust_bridge_restart.c.
use crate::game_init::kernel_abi;
use crate::ships::native::abi::*;
use std::{ffi::c_void, ptr};

macro_rules! operation {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block) => {
        /// # Safety
        /// Menu and resource pointers obey the game-thread provider's lifetime contract.
        pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? $body
    };
}
operation!(uqm_get_utwig_bomb_on_ship() -> u8 { super::super::state_abi::game_byte("UTWIG_BOMB_ON_SHIP") });
operation!(uqm_set_utwig_bomb_on_ship(value: u8) { super::super::state_abi::set_game_byte("UTWIG_BOMB_ON_SHIP", value); });
operation!(uqm_get_utwig_bomb() -> u8 { super::super::state_abi::game_byte("UTWIG_BOMB") });
operation!(uqm_set_player_control(player: u8, control: u16) { kernel_abi::PlayerControl[player as usize] = control as u8; });
operation!(uqm_get_pulsed_menu_key(index: u8) -> u32 { super::super::input_abi::PulsedInputState.menu[index as usize] as u32 });
operation!(uqm_get_mouse_button_down() -> u32 { crate::input::sdl_input::MouseButtonDown as u32 });
pub use crate::time::counter_abi::GetTimeCounter as uqm_get_time_counter;
operation!(uqm_sleep_thread_until(time: u32) { crate::threading::pacing::linked::sleep_thread_until(time); });
operation!(uqm_sleep_thread(time: u32) { crate::threading::pacing::linked::sleep_thread(time); });
operation!(uqm_set_game_paused(value: u32) { ptr::addr_of_mut!(super::super::input_abi::GamePaused).write_volatile(value); });
operation!(uqm_reinit_race_queues() {
    let queues = ptr::addr_of_mut!(kernel_abi::race_q).cast::<crate::collections::queue::Queue>();
    crate::collections::queue::ReinitQueue(queues); crate::collections::queue::ReinitQueue(queues.add(1));
});
operation!(uqm_assign_star_planet_globals() { crate::planets::starmap::star_array = ptr::addr_of_mut!(crate::planets::tables::STARMAP_ARRAY).cast(); });
operation!(uqm_do_popup_window_msg(id: u16) {
    let string = crate::strings::string_table::SetAbsStringTableIndex(kernel_abi::GameStrings.cast(), (MAINMENU_STRING_BASE as u16).wrapping_add(id));
    crate::ui::confirm::DoPopupWindow(crate::strings::string_table::GetStringAddress(string));
});
unsafe extern "C" fn restart_input(menu: *mut MENU_STATE) -> u32 {
    super::do_restart::rust_do_restart_frame(menu.cast()) as u32
}
operation!(uqm_set_rust_input_func(menu: *mut MENU_STATE) { (*menu).InputFunc = Some(restart_input); });
operation!(uqm_get_menu_cur_frame(menu: *mut MENU_STATE) -> FRAME { (*menu).CurFrame });
operation!(uqm_get_menu_priv_data(menu: *mut MENU_STATE) -> *mut c_void { (*menu).privData });
operation!(uqm_set_menu_flash_context(menu: *mut MENU_STATE, context: *mut FlashContext) { (*menu).flashContext = context; });
operation!(uqm_set_menu_initialized(menu: *mut MENU_STATE, value: i16) { (*menu).Initialized = value; });
operation!(uqm_set_menu_cur_state(menu: *mut MENU_STATE, state: u8) { (*menu).CurState = state; });
operation!(uqm_set_menu_cur_frame(menu: *mut MENU_STATE, frame: FRAME) { (*menu).CurFrame = frame; });
operation!(uqm_set_menu_h_music(menu: *mut MENU_STATE, music: MUSIC_REF) { (*menu).hMusic = music; });
operation!(uqm_create_menu_state() -> *mut MENU_STATE {
    let mut menu: Box<MENU_STATE> = Box::new(std::mem::zeroed());
    menu.InputFunc = Some(restart_input);
    Box::into_raw(menu)
});
operation!(uqm_set_menu_priv_data(menu: *mut MENU_STATE, data: *mut c_void) { if !menu.is_null() { (*menu).privData = data; } });
operation!(uqm_destroy_menu_state(menu: *mut MENU_STATE) { if !menu.is_null() { drop(Box::from_raw(menu)); } });
