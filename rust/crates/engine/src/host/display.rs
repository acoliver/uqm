//! The game thread's window system and draw queue as safe calls.
//!
//! The graphics entry points these forward to are `unsafe` only because they
//! must run on the game thread, which owns SDL and the draw queue. Every
//! caller of this module is on that thread, so the preconditions hold for all
//! of them; the graphics owner replaces each body when its entry point becomes
//! safe.

use crate::graphics::cmap_abi::UninitColorMaps;
use crate::graphics::dcqueue_abi::{
    TFB_FlushGraphics, TFB_FlushGraphicsEx, TFB_PurgeDanglingGraphics,
};
use crate::graphics::sdl::lifecycle_abi::{
    GraphicsOptions, TFB_InitGraphics, TFB_ProcessEvents, TFB_SwapBuffers, TFB_UninitGraphics,
};

/// Dispatch the window system's pending events to the game.
pub fn process_events() {
    // SAFETY: game thread only, with input initialized.
    unsafe { TFB_ProcessEvents() };
}

/// Run every committed draw command and present the frame if it changed.
pub fn flush_draws() {
    // SAFETY: game thread only; it consumes the draw queue.
    unsafe { TFB_FlushGraphics() };
}

/// Run every committed draw command without presenting a frame.
///
/// Queued commands hold pointers into the resources they draw with; owners
/// call this before freeing such a resource.
pub fn drain_committed_draws() {
    // SAFETY: game thread only; it consumes the draw queue.
    unsafe { TFB_FlushGraphicsEx(1) };
}

/// Run every committed draw command, then publish exactly one frame.
///
/// `flush_draws` alone does not swap when the queue is already empty.
pub fn present_committed_draws() {
    drain_committed_draws();
    // SAFETY: game thread only, with graphics initialized.
    unsafe { TFB_SwapBuffers(0) };
}

/// Bring graphics up; `false` when the backend could not start.
pub fn init_graphics(
    driver: i32,
    flags: i32,
    options: &GraphicsOptions,
    width: i32,
    height: i32,
) -> bool {
    // SAFETY: called once on the game thread; `options` is live for the call.
    unsafe {
        TFB_InitGraphics(
            driver,
            flags,
            std::ptr::from_ref(options).cast(),
            width,
            height,
        ) == 0
    }
}

/// Shut graphics down in the order the subsystems depend on each other.
pub fn uninit_graphics() {
    // SAFETY: game thread only, after every graphics producer has stopped and
    // no queued command retains a colour map.
    unsafe {
        TFB_PurgeDanglingGraphics();
        UninitColorMaps();
        TFB_UninitGraphics();
    }
}
