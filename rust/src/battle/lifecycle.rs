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

// ---------------------------------------------------------------------------
// P12: Battle Lifecycle (battle.c, init.c)
// @plan PLAN-20260320-BATTLEPT2.P12
// @requirement REQ-BATTLE-ENTRY, REQ-SHIP-INIT, REQ-SPACE-INIT, REQ-INPUT-PROCESSING
// ---------------------------------------------------------------------------

/// Activity type constants matching C (globdata.h)
pub const SUPER_MELEE: u8 = 1;
pub const IN_ENCOUNTER: u8 = 2;
pub const IN_LAST_BATTLE: u8 = 3;

/// Number of sides in battle (always 2)
pub const NUM_SIDES: i32 = 2;

/// Reference-counted shared asset state.
///
/// C reference: init.c InitSpace/UninitSpace use `++count` / `--count`
/// to manage explosion, blast, asteroid frames loaded once and shared.
pub struct SharedAssetState {
    pub ref_count: u32,
    pub loaded: bool,
}

impl SharedAssetState {
    pub fn new() -> Self {
        Self {
            ref_count: 0,
            loaded: false,
        }
    }

    /// Increment reference count. Returns true if this is the first reference
    /// (assets need loading).
    pub fn acquire(&mut self) -> bool {
        self.ref_count += 1;
        if !self.loaded {
            self.loaded = true;
            true // caller should load assets
        } else {
            false // assets already loaded
        }
    }

    /// Decrement reference count. Returns true if count reaches zero
    /// (assets should be freed).
    pub fn release(&mut self) -> bool {
        if self.ref_count == 0 {
            return false;
        }
        self.ref_count -= 1;
        if self.ref_count == 0 {
            self.loaded = false;
            true // caller should free assets
        } else {
            false // still referenced
        }
    }
}

impl Default for SharedAssetState {
    fn default() -> Self {
        Self::new()
    }
}

/// Battle sequence state tracking.
///
/// Tracks where we are in the Battle() entry/exit sequence.
/// C reference: battle.c:396-516 Battle()
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleSequenceState {
    /// Not started
    Idle,
    /// RNG seeded, music loading
    Initializing,
    /// Ships initialized, ready for selection
    ShipsReady,
    /// Ship selection in progress
    Selecting,
    /// Main battle loop active
    InBattle,
    /// AbortBattle cleanup
    Aborting,
    /// Normal exit cleanup
    Finishing,
}

/// Map a single abstract battle input to ship input flags.
///
/// C reference: battle.c ProcessInput:144-226
///
/// Maps BATTLE_LEFT/RIGHT/THRUST/WEAPON/SPECIAL to corresponding
/// ship input constants (LEFT/RIGHT/THRUST/WEAPON/SPECIAL).
pub fn map_battle_input(input: BattleInputState) -> u32 {
    let mut flags = 0u32;
    if input.0 & BattleInputState::LEFT.0 != 0 {
        flags |= super::ship_runtime::LEFT;
    }
    if input.0 & BattleInputState::RIGHT.0 != 0 {
        flags |= super::ship_runtime::RIGHT;
    }
    if input.0 & BattleInputState::THRUST.0 != 0 {
        flags |= super::ship_runtime::THRUST;
    }
    if input.0 & BattleInputState::WEAPON.0 != 0 {
        flags |= super::ship_runtime::WEAPON;
    }
    if input.0 & BattleInputState::SPECIAL.0 != 0 {
        flags |= super::ship_runtime::SPECIAL;
    }
    flags
}

/// Check if battle input contains escape request.
pub fn has_escape_input(input: BattleInputState) -> bool {
    input.0 & BattleInputState::ESCAPE.0 != 0
}

/// Determine the music type for the current battle context.
///
/// C reference: battle.c BattleSong:234-249
///
/// Returns which music resource to load/play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleMusicType {
    HyperSpace,
    QuasiSpace,
    Normal,
}

/// Select battle music type based on current location.
pub fn select_battle_music(in_hyperspace: bool, in_quasispace: bool) -> BattleMusicType {
    if in_hyperspace {
        BattleMusicType::HyperSpace
    } else if in_quasispace {
        BattleMusicType::QuasiSpace
    } else {
        BattleMusicType::Normal
    }
}

/// Determine player input processing order.
///
/// C reference: battle.c GetPlayerOrder:357-372
///
/// Returns (first_player, second_player). In normal battles,
/// player 0 goes first. In network play, may be reversed.
pub fn get_player_order(is_network: bool, local_player: u8) -> (u8, u8) {
    if is_network && local_player == 1 {
        (1, 0) // network: local player goes first
    } else {
        (0, 1) // default order
    }
}

/// Check if battle should use instant victory (skip combat).
///
/// C reference: battle.c:418-424
pub fn check_instant_victory(instant_victory_flag: bool) -> Option<[i32; 2]> {
    if instant_victory_flag {
        Some([1, 0]) // battle_counter: player 0 wins immediately
    } else {
        None
    }
}

/// Compute initial battle_counter values from fleet sizes.
///
/// C reference: battle.c:431-432
pub fn compute_battle_counters(fleet_size_0: i32, fleet_size_1: i32) -> [i32; 2] {
    [fleet_size_0, fleet_size_1]
}

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

    // ---- P12: Battle Lifecycle tests ----

    #[test]
    fn test_shared_asset_acquire_release() {
        let mut state = SharedAssetState::new();
        assert!(!state.loaded);
        assert_eq!(state.ref_count, 0);

        // First acquire loads
        assert!(state.acquire());
        assert!(state.loaded);
        assert_eq!(state.ref_count, 1);

        // Second acquire doesn't reload
        assert!(!state.acquire());
        assert_eq!(state.ref_count, 2);

        // First release doesn't free
        assert!(!state.release());
        assert_eq!(state.ref_count, 1);
        assert!(state.loaded);

        // Second release frees
        assert!(state.release());
        assert_eq!(state.ref_count, 0);
        assert!(!state.loaded);
    }

    #[test]
    fn test_shared_asset_release_at_zero() {
        let mut state = SharedAssetState::new();
        assert!(!state.release()); // no-op at zero
    }

    #[test]
    fn test_map_battle_input() {
        let input = BattleInputState(
            BattleInputState::LEFT.0 | BattleInputState::THRUST.0 | BattleInputState::WEAPON.0,
        );
        let flags = map_battle_input(input);
        use super::super::ship_runtime::{LEFT, RIGHT, THRUST, WEAPON};
        assert!(flags & (LEFT as u32) != 0);
        assert!(flags & (THRUST as u32) != 0);
        assert!(flags & (WEAPON as u32) != 0);
        assert!(flags & (RIGHT as u32) == 0);
    }

    #[test]
    fn test_has_escape_input() {
        assert!(!has_escape_input(BattleInputState::NONE));
        assert!(has_escape_input(BattleInputState::ESCAPE));
        assert!(has_escape_input(BattleInputState(
            BattleInputState::LEFT.0 | BattleInputState::ESCAPE.0
        )));
    }

    #[test]
    fn test_select_battle_music() {
        assert_eq!(
            select_battle_music(true, false),
            BattleMusicType::HyperSpace
        );
        assert_eq!(
            select_battle_music(false, true),
            BattleMusicType::QuasiSpace
        );
        assert_eq!(select_battle_music(false, false), BattleMusicType::Normal);
    }

    #[test]
    fn test_get_player_order() {
        assert_eq!(get_player_order(false, 0), (0, 1));
        assert_eq!(get_player_order(true, 0), (0, 1));
        assert_eq!(get_player_order(true, 1), (1, 0));
    }

    #[test]
    fn test_check_instant_victory() {
        assert!(check_instant_victory(false).is_none());
        assert_eq!(check_instant_victory(true), Some([1, 0]));
    }

    #[test]
    fn test_compute_battle_counters() {
        assert_eq!(compute_battle_counters(3, 5), [3, 5]);
    }

    #[test]
    fn test_battle_sequence_states() {
        // Verify all states exist and are distinct
        let states = [
            BattleSequenceState::Idle,
            BattleSequenceState::Initializing,
            BattleSequenceState::ShipsReady,
            BattleSequenceState::Selecting,
            BattleSequenceState::InBattle,
            BattleSequenceState::Aborting,
            BattleSequenceState::Finishing,
        ];
        assert_eq!(states.len(), 7);
        assert_ne!(states[0], states[1]);
    }

    #[test]
    fn test_activity_constants() {
        assert_eq!(SUPER_MELEE, 1);
        assert_eq!(IN_ENCOUNTER, 2);
        assert_eq!(IN_LAST_BATTLE, 3);
    }
}
