//! Process Loop Types (P10)
//!
//! Type definitions and constants for the battle process loop.
//! This is a type-only module — no orchestration logic.
//!
//! The process loop orchestration (PreProcessQueue, PostProcessQueue,
//! ProcessCollisions) stays in C for Phase 1. This module defines
//! the types and constants those systems depend on.

/// View state for camera/zoom system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ViewState {
    /// View is stable, no scrolling or zoom change
    Stable = 0,
    /// View is scrolling but zoom unchanged
    Scroll = 1,
    /// Zoom level is changing
    Change = 2,
}

/// Zoom mode (step vs continuous)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZoomMode {
    /// Fixed-step zoom style (original)
    Step = 0,
    /// Continuous zoom style (smooth)
    Continuous = 1,
}

// Zoom constants from units.h and process.c
pub const ZOOM_SHIFT: u32 = 8;
pub const MAX_REDUCTION: u32 = 3;
pub const MAX_VIS_REDUCTION: u32 = 2;
pub const REDUCTION_SHIFT: u32 = 1;
pub const NUM_VIEWS: usize = (MAX_VIS_REDUCTION + 1) as usize;
pub const MAX_ZOOM_OUT: i32 = 1 << (ZOOM_SHIFT + MAX_REDUCTION - 1);

// Hysteresis thresholds (from process.c HYSTERESIS_X/Y macros)
// These prevent oscillation at zoom boundaries
pub const HYSTERESIS_X: i32 = 24 << 2; // DISPLAY_TO_WORLD(24)
pub const HYSTERESIS_Y: i32 = 20 << 2; // DISPLAY_TO_WORLD(20)

// Zoom jump constant for continuous mode
pub const ZOOM_JUMP: i32 = (1 << ZOOM_SHIFT) >> 3;

// Camera clamping constant for single-ship mode (from process.c)
pub const ORG_JUMP_X: i32 = 4; // DISPLAY_ALIGN(LOG_SPACE_WIDTH / 75)
pub const ORG_JUMP_Y: i32 = 4; // DISPLAY_ALIGN(LOG_SPACE_HEIGHT / 75)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_state_variants() {
        assert_eq!(ViewState::Stable as u8, 0);
        assert_eq!(ViewState::Scroll as u8, 1);
        assert_eq!(ViewState::Change as u8, 2);
    }

    #[test]
    fn test_zoom_mode_variants() {
        assert_eq!(ZoomMode::Step as u8, 0);
        assert_eq!(ZoomMode::Continuous as u8, 1);
    }

    #[test]
    fn test_zoom_constants() {
        assert_eq!(ZOOM_SHIFT, 8);
        assert_eq!(MAX_REDUCTION, 3);
        assert_eq!(MAX_VIS_REDUCTION, 2);
        assert_eq!(REDUCTION_SHIFT, 1);
        assert_eq!(NUM_VIEWS, 3);
        assert_eq!(MAX_ZOOM_OUT, 1024); // 1 << (8 + 3 - 1) = 1 << 10
    }

    #[test]
    fn test_hysteresis_constants() {
        assert_eq!(HYSTERESIS_X, 96); // 24 << 2
        assert_eq!(HYSTERESIS_Y, 80); // 20 << 2
    }

    #[test]
    fn test_zoom_jump_constant() {
        assert_eq!(ZOOM_JUMP, 32); // (1 << 8) >> 3 = 256 >> 3
    }

    #[test]
    fn test_camera_clamp_constants() {
        assert_eq!(ORG_JUMP_X, 4);
        assert_eq!(ORG_JUMP_Y, 4);
    }
}
