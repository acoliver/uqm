//! Production draw queue ordering, batching and no-swap flush contract.
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
};
use uqm_rust::graphics::{
    dcqueue_abi::*,
    gfx_common_abi::{BatchGraphics, GetBatchDepth, UnbatchGraphics},
    tfb_draw_abi::{TFB_DrawScreen_Callback, TFB_DrawScreen_WaitForSignal},
};
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
        assert_eq!(GetBatchDepth(), 0);
        UnbatchGraphics();
        assert_eq!(GetBatchDepth(), 0);
        let mut expected = [0usize, 1];
        BatchGraphics();
        TFB_DrawScreen_Callback(Some(ordered), ptr::addr_of_mut!(expected[0]).cast());
        BatchGraphics();
        TFB_DrawScreen_Callback(Some(ordered), ptr::addr_of_mut!(expected[1]).cast());
        assert_eq!(GetBatchDepth(), 2);
        assert_eq!((*ptr::addr_of!(DrawCommandQueue)).Size, 0);
        UnbatchGraphics();
        assert_eq!((*ptr::addr_of!(DrawCommandQueue)).Size, 0);
        TFB_DrawScreen_WaitForSignal();
        assert_eq!(GetBatchDepth(), 1);
        assert_eq!(NEXT.load(Ordering::SeqCst), 2);
        assert_eq!((*ptr::addr_of!(DrawCommandQueue)).FullSize, 0);
        assert_eq!(ptr::addr_of!(RENDERED_FRAMES).read(), 1);
        TFB_BatchReset();
        assert_eq!(GetBatchDepth(), 0);
        UnbatchGraphics();
        assert_eq!(GetBatchDepth(), 0);
        Uninit_DrawCommandQueue();
    }
}
