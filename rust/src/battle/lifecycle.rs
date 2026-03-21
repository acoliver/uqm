//! Battle Lifecycle Types (P11)
//!
//! Type definitions and constants for battle entry, frame processing,
//! input handling, and teardown sequences.
//! This is a type-only module — no orchestration logic.
//!
//! The battle loop (DoBattle, Battle) stays in C for Phase 1.

/// Battle frame rate constant (frames per second)
pub const BATTLE_FRAME_RATE: u32 = 24; // ONE_SECOND / 24 from battle.h

/// Frame processing stages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameStage {
    /// Input processing stage
    Input = 0,
    /// Batch processing stage
    Batch = 1,
    /// Simulation stage
    Simulate = 2,
    /// Rendering stage
    Render = 3,
    /// Exit check stage
    ExitCheck = 4,
}

/// Input state mapping for battle controls
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleInputState(pub u32);

impl BattleInputState {
    pub const NONE: Self = Self(0);
    pub const LEFT: Self = Self(1 << 0);
    pub const RIGHT: Self = Self(1 << 1);
    pub const THRUST: Self = Self(1 << 2);
    pub const WEAPON: Self = Self(1 << 3);
    pub const SPECIAL: Self = Self(1 << 4);
    pub const ESCAPE: Self = Self(1 << 5);
}

/// Teardown sequence stages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TeardownStage {
    /// Stop audio
    StopAudio = 0,
    /// Free assets
    FreeAssets = 1,
    /// Count crew
    CountCrew = 2,
    /// Writeback ship state
    Writeback = 3,
    /// Clear activity flag
    ClearActivity = 4,
}

/// InitShips return type
///
/// Returns SIZE (i16, signed). Negative values indicate hyperspace exit.
/// The C function `InitShips()` at init.c returns i16, and `Battle()` at
/// battle.c:515 tests `num_ships < 0` for hyperspace detection.
pub type InitShipsResult = i16;

/// Minimum number of ships for battle
pub const MIN_SHIPS_FOR_BATTLE: i16 = 1;

/// Hyperspace exit indicator (negative return)
pub const HYPERSPACE_EXIT: i16 = -1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battle_frame_rate() {
        assert_eq!(BATTLE_FRAME_RATE, 24);
    }

    #[test]
    fn test_frame_stage_variants() {
        assert_eq!(FrameStage::Input as u8, 0);
        assert_eq!(FrameStage::Batch as u8, 1);
        assert_eq!(FrameStage::Simulate as u8, 2);
        assert_eq!(FrameStage::Render as u8, 3);
        assert_eq!(FrameStage::ExitCheck as u8, 4);
    }

    #[test]
    fn test_battle_input_state_constants() {
        assert_eq!(BattleInputState::NONE.0, 0);
        assert_eq!(BattleInputState::LEFT.0, 1);
        assert_eq!(BattleInputState::RIGHT.0, 2);
        assert_eq!(BattleInputState::THRUST.0, 4);
        assert_eq!(BattleInputState::WEAPON.0, 8);
        assert_eq!(BattleInputState::SPECIAL.0, 16);
        assert_eq!(BattleInputState::ESCAPE.0, 32);
    }

    #[test]
    fn test_teardown_stage_variants() {
        assert_eq!(TeardownStage::StopAudio as u8, 0);
        assert_eq!(TeardownStage::FreeAssets as u8, 1);
        assert_eq!(TeardownStage::CountCrew as u8, 2);
        assert_eq!(TeardownStage::Writeback as u8, 3);
        assert_eq!(TeardownStage::ClearActivity as u8, 4);
    }

    #[test]
    fn test_init_ships_result_type() {
        let normal_result: InitShipsResult = 2;
        let hyperspace_exit: InitShipsResult = HYPERSPACE_EXIT;

        assert!(normal_result > 0);
        assert!(hyperspace_exit < 0);
        assert_eq!(hyperspace_exit, -1);
    }

    #[test]
    fn test_min_ships_constant() {
        assert_eq!(MIN_SHIPS_FOR_BATTLE, 1);
    }
}
