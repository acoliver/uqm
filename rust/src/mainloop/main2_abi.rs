//! Runtime geometry used by the gravity iterator.
macro_rules! operation {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block) => {
        /// # Safety
        /// Called in startup or shutdown order on the game thread with live provider handles.
        pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? $body
    };
}
operation!(uqm_get_head_element() -> *mut std::ffi::c_void { crate::game_init::kernel_abi::disp_q.head });
operation!(uqm_wrap_delta_x(delta: i16) -> i16 { crate::ships::native::wrap_delta(delta, (crate::ships::native::abi::ScreenWidth - 64) << 5) });
operation!(uqm_wrap_delta_y(delta: i16) -> i16 { crate::ships::native::wrap_delta(delta, crate::ships::native::abi::ScreenHeight << 5) });
