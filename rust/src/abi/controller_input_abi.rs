// Shared engine records used by the Rust runtime.

pub const KEY_UP: _bindgen_ty_1 = 0;
pub const KEY_DOWN: _bindgen_ty_1 = 1;
pub const KEY_LEFT: _bindgen_ty_1 = 2;
pub const KEY_RIGHT: _bindgen_ty_1 = 3;
pub const KEY_WEAPON: _bindgen_ty_1 = 4;
pub const KEY_SPECIAL: _bindgen_ty_1 = 5;
pub const KEY_ESCAPE: _bindgen_ty_1 = 6;
pub const NUM_KEYS: _bindgen_ty_1 = 7;
pub type _bindgen_ty_1 = ::std::os::raw::c_uint;
pub const KEY_PAUSE: _bindgen_ty_2 = 0;
pub const KEY_EXIT: _bindgen_ty_2 = 1;
pub const KEY_ABORT: _bindgen_ty_2 = 2;
pub const KEY_DEBUG: _bindgen_ty_2 = 3;
pub const KEY_FULLSCREEN: _bindgen_ty_2 = 4;
pub const KEY_MENU_UP: _bindgen_ty_2 = 5;
pub const KEY_MENU_DOWN: _bindgen_ty_2 = 6;
pub const KEY_MENU_LEFT: _bindgen_ty_2 = 7;
pub const KEY_MENU_RIGHT: _bindgen_ty_2 = 8;
pub const KEY_MENU_SELECT: _bindgen_ty_2 = 9;
pub const KEY_MENU_CANCEL: _bindgen_ty_2 = 10;
pub const KEY_MENU_SPECIAL: _bindgen_ty_2 = 11;
pub const KEY_MENU_PAGE_UP: _bindgen_ty_2 = 12;
pub const KEY_MENU_PAGE_DOWN: _bindgen_ty_2 = 13;
pub const KEY_MENU_HOME: _bindgen_ty_2 = 14;
pub const KEY_MENU_END: _bindgen_ty_2 = 15;
pub const KEY_MENU_ZOOM_IN: _bindgen_ty_2 = 16;
pub const KEY_MENU_ZOOM_OUT: _bindgen_ty_2 = 17;
pub const KEY_MENU_DELETE: _bindgen_ty_2 = 18;
pub const KEY_MENU_BACKSPACE: _bindgen_ty_2 = 19;
pub const KEY_MENU_EDIT_CANCEL: _bindgen_ty_2 = 20;
pub const KEY_MENU_SEARCH: _bindgen_ty_2 = 21;
pub const KEY_MENU_NEXT: _bindgen_ty_2 = 22;
pub const KEY_MENU_ANY: _bindgen_ty_2 = 23;
pub const NUM_MENU_KEYS: _bindgen_ty_2 = 24;
pub type _bindgen_ty_2 = ::std::os::raw::c_uint;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_KB_1: CONTROL_TEMPLATE = 0;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_KB_2: CONTROL_TEMPLATE = 1;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_KB_3: CONTROL_TEMPLATE = 2;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_JOY_1: CONTROL_TEMPLATE = 3;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_JOY_2: CONTROL_TEMPLATE = 4;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_JOY_3: CONTROL_TEMPLATE = 5;
pub const CONTROL_TEMPLATE_NUM_TEMPLATES: CONTROL_TEMPLATE = 6;
pub type CONTROL_TEMPLATE = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _controller_input_state {
    pub key: [[::std::os::raw::c_int; 7usize]; 6usize],
    pub menu: [::std::os::raw::c_int; 24usize],
}
pub type CONTROLLER_INPUT_STATE = _controller_input_state;
extern "C" {
    pub static mut CurrentInputState: CONTROLLER_INPUT_STATE;
}
extern "C" {
    pub static mut PulsedInputState: CONTROLLER_INPUT_STATE;
}
