//!
//! Phase 2: Graphics subsystem core functionality.

pub mod gfx_common;

pub use gfx_common::{
    global_state, init_global_state, FrameRateState, GfxDriver, GfxFlags, GraphicsState,
    RedrawMode, ScaleConfig, ScaleMode, ScreenDimensions,
};
