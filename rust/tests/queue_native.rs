//! Production draw queue ordering, batching and no-swap flush contract.
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use uqm_rust::graphics::{
    dcqueue_abi::*,
    tfb_draw_abi::{TFB_DrawScreen_Callback, TFB_DrawScreen_WaitForSignal},
};
#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
extern "C" {
    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
}
static NEXT: AtomicUsize = AtomicUsize::new(0);
unsafe extern "C" fn ordered(arg: *mut c_void) {
    let expected = *arg.cast::<usize>();
    assert_eq!(NEXT.fetch_add(1, Ordering::SeqCst), expected);
}
#[test]
fn nested_batches_publish_in_order_and_wait_flushes_without_a_presenter() {
    unsafe {
        ScreenWidth = 320;
        ScreenHeight = 240;
        Init_DrawCommandQueue();
        let mut expected = [0usize, 1];
        TFB_BatchGraphics();
        TFB_DrawScreen_Callback(Some(ordered), ptr::addr_of_mut!(expected[0]).cast());
        TFB_BatchGraphics();
        TFB_DrawScreen_Callback(Some(ordered), ptr::addr_of_mut!(expected[1]).cast());
        assert_eq!(TFB_GetBatchDepth(), 2);
        assert_eq!((*ptr::addr_of!(DrawCommandQueue)).Size, 0);
        TFB_UnbatchGraphics();
        assert_eq!((*ptr::addr_of!(DrawCommandQueue)).Size, 0);
        TFB_DrawScreen_WaitForSignal();
        assert_eq!(TFB_GetBatchDepth(), 1);
        assert_eq!(NEXT.load(Ordering::SeqCst), 2);
        assert_eq!((*ptr::addr_of!(DrawCommandQueue)).FullSize, 0);
        assert_eq!(ptr::addr_of!(RenderedFrames).read(), 1);
        TFB_UnbatchGraphics();
        Uninit_DrawCommandQueue();
    }
}
