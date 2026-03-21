//! Ship Runtime Types (P12)
//!
//! Type definitions and constants for ship per-frame processing pipeline,
//! spawn positioning, energy/crew management, and weapon firing.
//! This is a type-only module — no orchestration logic.
//!
//! The ship runtime pipeline (ship_preprocess, ship_postprocess) stays
//! in C for Phase 1.

/// Ship per-frame pipeline stages (7 stages total)
///
/// Exact order from ship.c ship_preprocess():
/// 1. Input processing
/// 2. APPEARING flag handling (first-frame initialization)
/// 3. Energy regeneration
/// 4. Race-specific preprocess
/// 5. Turn processing
/// 6. Thrust processing
/// 7. Status display update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ShipPipelineStage {
    /// Process input state
    Input = 0,
    /// Handle APPEARING flag (first frame only)
    Appearing = 1,
    /// Regenerate energy
    Energy = 2,
    /// Race-specific preprocess callback
    Preprocess = 3,
    /// Process turn input
    Turn = 4,
    /// Process thrust input
    Thrust = 5,
    /// Update status display
    Status = 6,
}

/// Spawn position types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpawnPositionType {
    /// Random position avoiding gravity wells
    Random = 0,
    /// Center position (Sa-Matra)
    Center = 1,
    /// HyperSpace position (flagship)
    HyperSpace = 2,
}

/// Maximum crew size constant
pub const MAX_CREW_SIZE: i16 = 42;

/// Maximum energy size constant
pub const MAX_ENERGY_SIZE: i16 = 42;

/// Maximum allowed speed constant (from ship.c)
/// Used for gravity well limit checks
pub const MAX_ALLOWED_SPEED: i32 = 18 << 2; // WORLD_TO_VELOCITY(DISPLAY_TO_WORLD(18))

/// Maximum allowed speed squared (for velocity checks without sqrt)
pub const MAX_ALLOWED_SPEED_SQR: u32 = (MAX_ALLOWED_SPEED * MAX_ALLOWED_SPEED) as u32;

/// Weapon firing types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WeaponFiringType {
    /// Primary weapon
    Primary = 0,
    /// Secondary weapon
    Secondary = 1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ship_pipeline_stage_count() {
        // Verify we have exactly 7 stages
        let stages = [
            ShipPipelineStage::Input,
            ShipPipelineStage::Appearing,
            ShipPipelineStage::Energy,
            ShipPipelineStage::Preprocess,
            ShipPipelineStage::Turn,
            ShipPipelineStage::Thrust,
            ShipPipelineStage::Status,
        ];
        assert_eq!(stages.len(), 7);
    }

    #[test]
    fn test_ship_pipeline_stage_order() {
        assert_eq!(ShipPipelineStage::Input as u8, 0);
        assert_eq!(ShipPipelineStage::Appearing as u8, 1);
        assert_eq!(ShipPipelineStage::Energy as u8, 2);
        assert_eq!(ShipPipelineStage::Preprocess as u8, 3);
        assert_eq!(ShipPipelineStage::Turn as u8, 4);
        assert_eq!(ShipPipelineStage::Thrust as u8, 5);
        assert_eq!(ShipPipelineStage::Status as u8, 6);
    }

    #[test]
    fn test_spawn_position_variants() {
        assert_eq!(SpawnPositionType::Random as u8, 0);
        assert_eq!(SpawnPositionType::Center as u8, 1);
        assert_eq!(SpawnPositionType::HyperSpace as u8, 2);
    }

    #[test]
    fn test_crew_energy_constants() {
        assert_eq!(MAX_CREW_SIZE, 42);
        assert_eq!(MAX_ENERGY_SIZE, 42);
    }

    #[test]
    fn test_max_allowed_speed_constants() {
        assert_eq!(MAX_ALLOWED_SPEED, 72); // 18 << 2
        assert_eq!(MAX_ALLOWED_SPEED_SQR, 5184); // 72 * 72
    }

    #[test]
    fn test_weapon_firing_type_variants() {
        assert_eq!(WeaponFiringType::Primary as u8, 0);
        assert_eq!(WeaponFiringType::Secondary as u8, 1);
    }
}
