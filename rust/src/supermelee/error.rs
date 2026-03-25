// SuperMelee Error Types
// @plan PLAN-SUPERMELEE.P03, P04, P05

use thiserror::Error;

/// All errors that can arise within the SuperMelee subsystem.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum SuperMeleeError {
    /// A raw ship identifier byte does not correspond to any known ship.
    #[error("invalid ship id: {0}")]
    InvalidShipId(u8),

    /// Loaded or provided team data is structurally invalid.
    #[error("invalid team data: {0}")]
    InvalidTeamData(String),

    /// A persistence (serialize/deserialize) operation failed.
    #[error("persistence error: {0}")]
    PersistenceError(String),

    /// A configuration value is missing or malformed.
    #[error("config error: {0}")]
    ConfigError(String),

    /// A ship-selection operation is out of range or otherwise illegal.
    #[error("selection error: {0}")]
    SelectionError(String),

    /// Handing off to the battle engine failed.
    #[error("battle handoff error: {0}")]
    BattleHandoffError(String),

    /// Netplay validation detected an inconsistency between peers.
    #[error("netplay validation error: {0}")]
    NetplayValidationError(String),
}

// ===========================================================================
// Tests  (P04 — verified by P05)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ship_id_constructs_and_displays() {
        let err = SuperMeleeError::InvalidShipId(42);
        assert_eq!(format!("{}", err), "invalid ship id: 42");
    }

    #[test]
    fn invalid_team_data_constructs_and_displays() {
        let err = SuperMeleeError::InvalidTeamData("bad fleet".into());
        assert_eq!(format!("{}", err), "invalid team data: bad fleet");
    }

    #[test]
    fn persistence_error_constructs_and_displays() {
        let err = SuperMeleeError::PersistenceError("eof".into());
        assert_eq!(format!("{}", err), "persistence error: eof");
    }

    #[test]
    fn config_error_constructs_and_displays() {
        let err = SuperMeleeError::ConfigError("missing key".into());
        assert_eq!(format!("{}", err), "config error: missing key");
    }

    #[test]
    fn selection_error_constructs_and_displays() {
        let err = SuperMeleeError::SelectionError("slot out of range".into());
        assert_eq!(format!("{}", err), "selection error: slot out of range");
    }

    #[test]
    fn battle_handoff_error_constructs_and_displays() {
        let err = SuperMeleeError::BattleHandoffError("no combatant".into());
        assert_eq!(format!("{}", err), "battle handoff error: no combatant");
    }

    #[test]
    fn netplay_validation_error_constructs_and_displays() {
        let err = SuperMeleeError::NetplayValidationError("fleet mismatch".into());
        assert_eq!(
            format!("{}", err),
            "netplay validation error: fleet mismatch"
        );
    }

    #[test]
    fn errors_are_clone_and_eq() {
        let err = SuperMeleeError::InvalidShipId(7);
        assert_eq!(err.clone(), err);
    }
}
