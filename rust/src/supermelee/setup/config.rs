// SuperMelee Config Persistence — melee.cfg load/save
// @plan PLAN-20260314-SUPERMELEE.P06
// @requirement persistence, startup sanitization

use crate::supermelee::error::SuperMeleeError;
use crate::supermelee::setup::persistence::{
    deserialize_team, serialize_team, MELEE_TEAM_SERIAL_SIZE,
};
use crate::supermelee::setup::team::MeleeSetup;
use crate::supermelee::types::{PlayerControl, NUM_SIDES};
use std::io::{self, Read};
use std::path::Path;

// ---------------------------------------------------------------------------
// melee.cfg format
// ---------------------------------------------------------------------------

/// On-disk size of `melee.cfg`: for each side, 1 control byte + team payload.
const MELEE_CFG_SIZE: usize = (1 + MELEE_TEAM_SERIAL_SIZE) * NUM_SIDES;

/// Standard AI rating bit from C (`STANDARD_RATING = 1 << 4 = 16`).
const STANDARD_RATING: u8 = 1 << 4;

// ---------------------------------------------------------------------------
// Config load result classification
// ---------------------------------------------------------------------------

/// Result of attempting to load `melee.cfg`.
#[derive(Debug)]
pub enum ConfigLoadResult {
    /// Config loaded successfully — setup and controls populated.
    Ok,
    /// Config file missing or unreadable — caller should apply built-in fallback.
    Missing,
    /// Config file exists but is malformed — caller should apply built-in fallback.
    Invalid(String),
}

// ---------------------------------------------------------------------------
// Load
// ---------------------------------------------------------------------------

/// Loads `melee.cfg` from `config_dir`, populating `setup`.
///
/// Matches `LoadMeleeConfig()` in `melee.c`:
/// - Validates file size = `(1 + MeleeTeam_serialSize) * NUM_SIDES`
/// - Reads per-side: 1 byte control mode + team payload
/// - Sanitizes `NETWORK_CONTROL` → `HUMAN_CONTROL | STANDARD_RATING`
pub fn load_melee_config(config_dir: &Path, setup: &mut MeleeSetup) -> ConfigLoadResult {
    let path = config_dir.join("melee.cfg");

    let data = match std::fs::read(&path) {
        Ok(d) => d,
        Err(_) => return ConfigLoadResult::Missing,
    };

    if data.len() != MELEE_CFG_SIZE {
        return ConfigLoadResult::Invalid(format!(
            "melee.cfg size mismatch: got {} bytes, expected {}",
            data.len(),
            MELEE_CFG_SIZE,
        ));
    }

    let mut cursor = io::Cursor::new(&data);

    for side in 0..NUM_SIDES {
        // Read control byte
        let mut ctrl_byte = [0u8; 1];
        if cursor.read_exact(&mut ctrl_byte).is_err() {
            return ConfigLoadResult::Invalid("truncated control byte".to_string());
        }
        let mut control = PlayerControl(ctrl_byte[0]);

        // Sanitize: do not allow netplay mode at startup (matches C)
        if control.contains(PlayerControl::NETWORK_CONTROL) {
            control = PlayerControl(PlayerControl::HUMAN_CONTROL.0 | STANDARD_RATING);
        }

        setup.player_control[side] = control;

        // Read team payload
        match deserialize_team(&mut cursor) {
            Ok(team) => {
                if setup.replace_team(side, &team).is_err() {
                    return ConfigLoadResult::Invalid(format!(
                        "failed to apply team for side {}",
                        side
                    ));
                }
            }
            Err(e) => {
                return ConfigLoadResult::Invalid(format!(
                    "failed to deserialize team for side {}: {}",
                    side, e
                ));
            }
        }
    }

    ConfigLoadResult::Ok
}

// ---------------------------------------------------------------------------
// Save
// ---------------------------------------------------------------------------

/// Writes `melee.cfg` to `config_dir` from the current `setup`.
///
/// Matches `WriteMeleeConfig()` in `melee.c`:
/// - Per-side: 1 byte control mode + team payload
/// - On failure, removes the partial file
pub fn save_melee_config(config_dir: &Path, setup: &MeleeSetup) -> Result<(), SuperMeleeError> {
    let path = config_dir.join("melee.cfg");
    let mut buf = Vec::with_capacity(MELEE_CFG_SIZE);

    for side in 0..NUM_SIDES {
        buf.push(setup.player_control[side].bits());
        serialize_team(&setup.teams[side], &mut buf)?;
    }

    if let Err(e) = std::fs::write(&path, &buf) {
        let _ = std::fs::remove_file(&path);
        return Err(SuperMeleeError::PersistenceError(format!(
            "melee.cfg write failed: {}",
            e
        )));
    }

    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use crate::supermelee::types::MeleeShip;

    #[test]
    fn missing_config_returns_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut setup = MeleeSetup::new();
        let result = load_melee_config(dir.path(), &mut setup);
        assert!(matches!(result, ConfigLoadResult::Missing));
    }

    #[test]
    fn invalid_config_size_returns_invalid() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("melee.cfg"), [0u8; 10]).unwrap();
        let mut setup = MeleeSetup::new();
        let result = load_melee_config(dir.path(), &mut setup);
        assert!(matches!(result, ConfigLoadResult::Invalid(_)));
    }

    #[test]
    fn valid_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut setup = MeleeSetup::new();
        setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
        setup.set_ship(1, 0, MeleeShip::Shofixti).unwrap();
        setup.set_team_name(0, "Side A").unwrap();
        setup.set_team_name(1, "Side B").unwrap();
        setup.player_control[0] = PlayerControl::HUMAN_CONTROL;
        setup.player_control[1] = PlayerControl::COMPUTER_CONTROL;
        save_melee_config(dir.path(), &setup).unwrap();
        let mut restored = MeleeSetup::new();
        let result = load_melee_config(dir.path(), &mut restored);
        assert!(matches!(result, ConfigLoadResult::Ok));
        assert_eq!(restored.teams[0].ships[0], MeleeShip::Chmmr);
        assert_eq!(restored.teams[1].ships[0], MeleeShip::Shofixti);
        assert_eq!(restored.teams[0].name_str(), "Side A");
        assert_eq!(restored.teams[1].name_str(), "Side B");
        assert_eq!(restored.player_control[0], PlayerControl::HUMAN_CONTROL);
        assert_eq!(restored.player_control[1], PlayerControl::COMPUTER_CONTROL);
    }

    #[test]
    fn network_control_is_sanitized_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut setup = MeleeSetup::new();
        setup.player_control[0] = PlayerControl::NETWORK_CONTROL;
        setup.player_control[1] = PlayerControl::HUMAN_CONTROL;
        save_melee_config(dir.path(), &setup).unwrap();
        let mut restored = MeleeSetup::new();
        let result = load_melee_config(dir.path(), &mut restored);
        assert!(matches!(result, ConfigLoadResult::Ok));
        // Network control should be sanitized to HUMAN + STANDARD_RATING
        assert!(restored.player_control[0].contains(PlayerControl::HUMAN_CONTROL));
        assert!(!restored.player_control[0].contains(PlayerControl::NETWORK_CONTROL));
        // STANDARD_RATING bit should be set (bit 4 = 16)
        assert_ne!(restored.player_control[0].bits() & STANDARD_RATING, 0);
    }

    #[test]
    fn config_write_failure_returns_error() {
        let setup = MeleeSetup::new();
        let result = save_melee_config(Path::new("/nonexistent/dir"), &setup);
        assert!(result.is_err());
    }

    #[test]
    fn config_file_has_correct_size() {
        let dir = tempfile::tempdir().unwrap();
        let setup = MeleeSetup::new();
        save_melee_config(dir.path(), &setup).unwrap();
        let data = std::fs::read(dir.path().join("melee.cfg")).unwrap();
        assert_eq!(data.len(), MELEE_CFG_SIZE);
    }
}
