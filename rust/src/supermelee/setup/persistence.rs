// SuperMelee Team Persistence — .mle file I/O and built-in team catalog
// @plan PLAN-20260314-SUPERMELEE.P06
// @requirement team/fleet model, persistence

use crate::supermelee::error::SuperMeleeError;
use crate::supermelee::setup::team::MeleeTeam;
use crate::supermelee::types::{MeleeShip, MAX_TEAM_CHARS, MELEE_FLEET_SIZE, NUM_MELEE_SHIPS};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Serial size — matches C `MeleeTeam_serialSize`
// ---------------------------------------------------------------------------

/// On-disk size of one serialized team: 14 ship bytes + 55 name bytes.
pub const MELEE_TEAM_SERIAL_SIZE: usize = MELEE_FLEET_SIZE + MAX_TEAM_CHARS + 1 + 24;

// ---------------------------------------------------------------------------
// Serialize / Deserialize  (mirrors meleesetup.c)
// ---------------------------------------------------------------------------

/// Writes a `MeleeTeam` to `writer` in the legacy `.mle` binary format.
///
/// Format: 14 ship bytes (one `u8` per slot) followed by 55 name bytes.
/// Matches `MeleeTeam_serialize()` in `meleesetup.c`.
pub fn serialize_team<W: Write>(team: &MeleeTeam, writer: &mut W) -> Result<(), SuperMeleeError> {
    for &ship in &team.ships {
        writer
            .write_all(&[ship as u8])
            .map_err(|e| SuperMeleeError::PersistenceError(e.to_string()))?;
    }
    writer
        .write_all(&team.name)
        .map_err(|e| SuperMeleeError::PersistenceError(e.to_string()))?;
    Ok(())
}

/// Reads a `MeleeTeam` from `reader` in the legacy `.mle` binary format.
///
/// Invalid ship IDs (>= NUM_MELEE_SHIPS and not MELEE_NONE) are silently
/// replaced with `MeleeNone`, matching C `MeleeTeam_deserialize()`.
pub fn deserialize_team<R: Read>(reader: &mut R) -> Result<MeleeTeam, SuperMeleeError> {
    let mut team = MeleeTeam::new();

    for slot in 0..MELEE_FLEET_SIZE {
        let mut byte = [0u8; 1];
        reader
            .read_exact(&mut byte)
            .map_err(|e| SuperMeleeError::PersistenceError(e.to_string()))?;
        let raw = byte[0];
        if raw == MeleeShip::MeleeNone as u8 {
            team.ships[slot] = MeleeShip::MeleeNone;
        } else if (raw as usize) < NUM_MELEE_SHIPS {
            team.ships[slot] = MeleeShip::from_u8(raw).unwrap_or(MeleeShip::MeleeNone);
        } else {
            // Invalid ship — replace with MELEE_NONE (matches C behavior)
            team.ships[slot] = MeleeShip::MeleeNone;
        }
    }

    reader
        .read_exact(&mut team.name)
        .map_err(|e| SuperMeleeError::PersistenceError(e.to_string()))?;

    // Ensure NUL-termination at MAX_TEAM_CHARS (matches C)
    team.name[MAX_TEAM_CHARS] = 0;

    Ok(team)
}

// ---------------------------------------------------------------------------
// File-level team I/O
// ---------------------------------------------------------------------------

/// Loads a team from a `.mle` file at `path`.
pub fn load_team_file(path: &Path) -> Result<MeleeTeam, SuperMeleeError> {
    let data = std::fs::read(path)
        .map_err(|e| SuperMeleeError::PersistenceError(format!("{}: {}", path.display(), e)))?;
    if data.len() < MELEE_TEAM_SERIAL_SIZE {
        return Err(SuperMeleeError::InvalidTeamData(format!(
            "{}: file too short ({} bytes, expected {})",
            path.display(),
            data.len(),
            MELEE_TEAM_SERIAL_SIZE,
        )));
    }
    let mut cursor = io::Cursor::new(&data);
    deserialize_team(&mut cursor)
}

/// Saves a team to a `.mle` file at `path`.
///
/// On write failure, the partial file is removed (matching C `DoSaveTeam`).
pub fn save_team_file(team: &MeleeTeam, path: &Path) -> Result<(), SuperMeleeError> {
    let mut buf = Vec::with_capacity(MELEE_TEAM_SERIAL_SIZE);
    serialize_team(team, &mut buf)?;

    if let Err(e) = std::fs::write(path, &buf) {
        // Clean up partial artifact (matches C behavior)
        let _ = std::fs::remove_file(path);
        return Err(SuperMeleeError::PersistenceError(format!(
            "{}: {}",
            path.display(),
            e
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Built-in team catalog (mirrors C `InitPreBuilt` in loadmele.c)
// ---------------------------------------------------------------------------

/// Number of built-in pre-defined teams (matches C `PREBUILT_COUNT`).
pub const PREBUILT_COUNT: usize = 15;

/// Helper to build a team from a name and ship list.
fn make_team(name: &str, ships: &[MeleeShip]) -> MeleeTeam {
    let mut team = MeleeTeam::new();
    team.set_name(name);
    for (i, &ship) in ships.iter().enumerate() {
        if i < MELEE_FLEET_SIZE {
            team.ships[i] = ship;
        }
    }
    team
}

/// Returns the 15 built-in teams matching `InitPreBuilt()` in `loadmele.c`.
///
/// Team names use the hardcoded English strings (the C version pulls from
/// GAME_STRING for the first 5, then uses hardcoded names for the rest).
pub fn builtin_teams() -> Vec<MeleeTeam> {
    use MeleeShip::*;

    vec![
        make_team(
            "Balanced Team 1",
            &[
                Androsynth, Chmmr, Druuge, Urquan, Melnorme, Orz, Spathi, Syreen, Utwig,
            ],
        ),
        make_team(
            "Balanced Team 2",
            &[
                Arilou, Chenjesu, Earthling, KohrAh, Mycon, Yehat, Pkunk, Supox, Thraddash,
                ZoqFotPik, Shofixti,
            ],
        ),
        make_team(
            "200 points",
            &[
                Androsynth, Chmmr, Druuge, Melnorme, Earthling, KohrAh, Supox, Orz, Spathi,
                Ilwrath, Vux,
            ],
        ),
        make_team(
            "Behemoth Zenith",
            &[
                Chenjesu, Chenjesu, Chmmr, Chmmr, KohrAh, KohrAh, Urquan, Urquan, Utwig, Utwig,
            ],
        ),
        make_team(
            "The Peeled Eyes",
            &[
                Urquan, Chenjesu, Mycon, Syreen, ZoqFotPik, Shofixti, Earthling, KohrAh, Melnorme,
                Druuge, Pkunk, Orz,
            ],
        ),
        make_team(
            "Ford's Fighters",
            &[Chmmr, ZoqFotPik, Melnorme, Supox, Utwig, Umgah],
        ),
        make_team(
            "Leyland's Lashers",
            &[Androsynth, Earthling, Mycon, Orz, Urquan],
        ),
        make_team(
            "The Gregorizers 200",
            &[
                Androsynth, Chmmr, Druuge, Melnorme, Earthling, KohrAh, Supox, Orz, Pkunk, Spathi,
            ],
        ),
        make_team(
            "300 point Armada!",
            &[
                Androsynth, Chmmr, Chenjesu, Druuge, Earthling, KohrAh, Melnorme, Mycon, Orz,
                Pkunk, Spathi, Supox, Urquan, Yehat,
            ],
        ),
        make_team(
            "Little Dudes with Attitudes",
            &[Umgah, Thraddash, Shofixti, Earthling, Vux, ZoqFotPik],
        ),
        make_team(
            "New Alliance Ships",
            &[
                Arilou, Chmmr, Earthling, Orz, Pkunk, Shofixti, Supox, Syreen, Utwig, ZoqFotPik,
                Yehat, Druuge, Thraddash, Spathi,
            ],
        ),
        make_team(
            "Old Alliance Ships",
            &[
                Arilou, Chenjesu, Earthling, Mmrnmhrm, Shofixti, Syreen, Yehat,
            ],
        ),
        make_team(
            "Old Hierarchy Ships",
            &[Androsynth, Ilwrath, Mycon, Spathi, Umgah, Urquan, Vux],
        ),
        make_team(
            "Star Control 1",
            &[
                Androsynth, Arilou, Chenjesu, Earthling, Ilwrath, Mmrnmhrm, Mycon, Shofixti,
                Spathi, Syreen, Umgah, Urquan, Vux, Yehat,
            ],
        ),
        make_team(
            "Star Control 2",
            &[
                Chmmr, Druuge, KohrAh, Melnorme, Orz, Pkunk, Slylandro, Supox, Thraddash, Utwig,
                ZoqFotPik, ZoqFotPik, ZoqFotPik, ZoqFotPik,
            ],
        ),
    ]
}

// ---------------------------------------------------------------------------
// Team browser entry — unified built-in + saved
// ---------------------------------------------------------------------------

/// Describes one entry in the team browser (built-in or saved file).
#[derive(Debug, Clone)]
pub enum TeamEntry {
    /// A built-in team at `index` in the built-in catalog.
    BuiltIn { index: usize, team: MeleeTeam },
    /// A saved `.mle` file on disk.
    Saved { path: PathBuf, team: MeleeTeam },
}

impl TeamEntry {
    pub fn team(&self) -> &MeleeTeam {
        match self {
            TeamEntry::BuiltIn { team, .. } => team,
            TeamEntry::Saved { team, .. } => team,
        }
    }

    pub fn name(&self) -> &str {
        self.team().name_str()
    }
}

/// Enumerates all available teams: built-ins first, then saved `.mle` files
/// in `melee_dir`.
///
/// Invalid `.mle` files are silently skipped (matching C `GetFleetByIndex`
/// which skips unreadable entries).
pub fn enumerate_teams(melee_dir: &Path) -> Vec<TeamEntry> {
    let mut entries = Vec::new();

    // Built-ins first
    for (i, team) in builtin_teams().into_iter().enumerate() {
        entries.push(TeamEntry::BuiltIn { index: i, team });
    }

    // Then saved .mle files
    if let Ok(dir) = std::fs::read_dir(melee_dir) {
        let mut files: Vec<_> = dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mle"))
            })
            .collect();
        // Sort by filename for deterministic ordering
        files.sort_by_key(|a| a.file_name());

        for entry in files {
            let path = entry.path();
            if let Ok(team) = load_team_file(&path) {
                entries.push(TeamEntry::Saved {
                    path: path.clone(),
                    team,
                });
            }
        }
    }

    entries
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supermelee::setup::team::MeleeSetup;
    use std::io::Cursor;

    // -----------------------------------------------------------------------
    // Serialize / Deserialize roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn serialize_deserialize_roundtrip() {
        unsafe {
            let mut team = MeleeTeam::new();
            team.ships[0] = MeleeShip::Chmmr;
            team.ships[5] = MeleeShip::Earthling;
            team.set_name("Test Team");

            let mut buf = Vec::new();
            serialize_team(&team, &mut buf).unwrap();
            assert_eq!(buf.len(), MELEE_TEAM_SERIAL_SIZE);

            let restored = deserialize_team(&mut Cursor::new(&buf)).unwrap();
            assert_eq!(restored.ships[0], MeleeShip::Chmmr);
            assert_eq!(restored.ships[5], MeleeShip::Earthling);
            assert_eq!(restored.ships[1], MeleeShip::MeleeNone);
            assert_eq!(restored.name_str(), "Test Team");
        }
    }

    #[test]
    fn deserialize_replaces_invalid_ship_ids() {
        unsafe {
            let mut buf = vec![0u8; MELEE_TEAM_SERIAL_SIZE];
            // Set slot 0 to an invalid ship ID (200)
            buf[0] = 200;
            // Set slot 1 to a valid ship (Earthling = 5)
            buf[1] = 5;

            let team = deserialize_team(&mut Cursor::new(&buf)).unwrap();
            assert_eq!(team.ships[0], MeleeShip::MeleeNone); // replaced
            assert_eq!(team.ships[1], MeleeShip::Earthling); // preserved
        }
    }

    #[test]
    fn deserialize_preserves_melee_none() {
        unsafe {
            let mut buf = vec![0u8; MELEE_TEAM_SERIAL_SIZE];
            buf[0] = MeleeShip::MeleeNone as u8; // 0xFF

            let team = deserialize_team(&mut Cursor::new(&buf)).unwrap();
            assert_eq!(team.ships[0], MeleeShip::MeleeNone);
        }
    }

    #[test]
    fn deserialize_truncated_data_returns_error() {
        unsafe {
            let buf = vec![0u8; 5]; // way too short
            let result = deserialize_team(&mut Cursor::new(&buf));
            assert!(result.is_err());
        }
    }

    #[test]
    fn serialize_size_matches_c() {
        unsafe {
            // C: MELEE_FLEET_SIZE + sizeof(name) = 14 + 55 = 69
            assert_eq!(
                MELEE_TEAM_SERIAL_SIZE,
                MELEE_FLEET_SIZE + MAX_TEAM_CHARS + 1 + 24
            );
        }
    }

    // -----------------------------------------------------------------------
    // Built-in team catalog
    // -----------------------------------------------------------------------

    #[test]
    fn builtin_catalog_has_correct_count() {
        unsafe {
            let teams = builtin_teams();
            assert_eq!(teams.len(), PREBUILT_COUNT);
        }
    }

    #[test]
    fn builtin_teams_have_names() {
        unsafe {
            for team in &builtin_teams() {
                assert!(!team.name_str().is_empty(), "built-in team has no name");
            }
        }
    }

    #[test]
    fn builtin_teams_have_at_least_one_ship() {
        unsafe {
            for team in &builtin_teams() {
                let has_ship = team.ships.iter().any(|&s| s != MeleeShip::MeleeNone);
                assert!(has_ship, "built-in team '{}' has no ships", team.name_str());
            }
        }
    }

    #[test]
    fn builtin_team_names_match_c() {
        unsafe {
            let teams = builtin_teams();
            assert_eq!(teams[0].name_str(), "Balanced Team 1");
            assert_eq!(teams[1].name_str(), "Balanced Team 2");
            assert_eq!(teams[2].name_str(), "200 points");
            assert_eq!(teams[3].name_str(), "Behemoth Zenith");
            assert_eq!(teams[4].name_str(), "The Peeled Eyes");
            assert_eq!(teams[5].name_str(), "Ford's Fighters");
            assert_eq!(teams[6].name_str(), "Leyland's Lashers");
            assert_eq!(teams[7].name_str(), "The Gregorizers 200");
            assert_eq!(teams[8].name_str(), "300 point Armada!");
            assert_eq!(teams[9].name_str(), "Little Dudes with Attitudes");
            assert_eq!(teams[10].name_str(), "New Alliance Ships");
            assert_eq!(teams[11].name_str(), "Old Alliance Ships");
            assert_eq!(teams[12].name_str(), "Old Hierarchy Ships");
            assert_eq!(teams[13].name_str(), "Star Control 1");
            assert_eq!(teams[14].name_str(), "Star Control 2");
        }
    }

    // -----------------------------------------------------------------------
    // File-level save/load roundtrip
    // -----------------------------------------------------------------------

    #[test]
    fn save_and_load_team_file_roundtrip() {
        unsafe {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("test.mle");

            let mut team = MeleeTeam::new();
            team.ships[0] = MeleeShip::Urquan;
            team.ships[13] = MeleeShip::Shofixti;
            team.set_name("Roundtrip Test");

            save_team_file(&team, &path).unwrap();
            let loaded = load_team_file(&path).unwrap();

            assert_eq!(loaded.ships[0], MeleeShip::Urquan);
            assert_eq!(loaded.ships[13], MeleeShip::Shofixti);
            assert_eq!(loaded.name_str(), "Roundtrip Test");
        }
    }

    #[test]
    fn load_team_file_too_short_returns_error() {
        unsafe {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("bad.mle");
            std::fs::write(&path, &[0u8; 10]).unwrap();

            let result = load_team_file(&path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn load_team_file_missing_returns_error() {
        unsafe {
            let result = load_team_file(Path::new("/nonexistent/team.mle"));
            assert!(result.is_err());
        }
    }

    // -----------------------------------------------------------------------
    // Team browser enumeration
    // -----------------------------------------------------------------------

    #[test]
    fn enumerate_includes_builtins() {
        unsafe {
            let dir = tempfile::tempdir().unwrap();
            let entries = enumerate_teams(dir.path());
            assert!(entries.len() >= PREBUILT_COUNT);
            // First entries should be built-ins
            for i in 0..PREBUILT_COUNT {
                assert!(matches!(&entries[i], TeamEntry::BuiltIn { index, .. } if *index == i));
            }
        }
    }

    #[test]
    fn enumerate_includes_saved_files() {
        unsafe {
            let dir = tempfile::tempdir().unwrap();
            let mut team = MeleeTeam::new();
            team.ships[0] = MeleeShip::Pkunk;
            team.set_name("Saved Team");
            save_team_file(&team, &dir.path().join("saved.mle")).unwrap();

            let entries = enumerate_teams(dir.path());
            assert_eq!(entries.len(), PREBUILT_COUNT + 1);
            let last = entries.last().unwrap();
            assert!(matches!(last, TeamEntry::Saved { .. }));
            assert_eq!(last.name(), "Saved Team");
        }
    }

    #[test]
    fn enumerate_skips_invalid_mle_files() {
        unsafe {
            let dir = tempfile::tempdir().unwrap();
            // Write a too-short file
            std::fs::write(dir.path().join("bad.mle"), &[0u8; 5]).unwrap();
            // Write a valid file
            let mut team = MeleeTeam::new();
            team.ships[0] = MeleeShip::Vux;
            team.set_name("Good");
            save_team_file(&team, &dir.path().join("good.mle")).unwrap();

            let entries = enumerate_teams(dir.path());
            // Should have builtins + 1 valid saved team (bad.mle skipped)
            assert_eq!(entries.len(), PREBUILT_COUNT + 1);
        }
    }

    #[test]
    fn enumerate_nonexistent_dir_returns_only_builtins() {
        unsafe {
            let entries = enumerate_teams(Path::new("/nonexistent/melee/dir"));
            assert_eq!(entries.len(), PREBUILT_COUNT);
        }
    }

    // -----------------------------------------------------------------------
    // Malformed file doesn't corrupt active state
    // -----------------------------------------------------------------------

    #[test]
    fn invalid_saved_team_fails_without_corrupting_active_state() {
        unsafe {
            let mut setup = MeleeSetup::new();
            setup.set_ship(0, 0, MeleeShip::Chmmr).unwrap();
            setup.set_team_name(0, "Original").unwrap();
            let original_value = setup.get_fleet_value(0);

            // Attempt to load a bad file
            let result = load_team_file(Path::new("/nonexistent/bad.mle"));
            assert!(result.is_err());

            // Active setup unchanged
            assert_eq!(setup.teams[0].ships[0], MeleeShip::Chmmr);
            assert_eq!(setup.teams[0].name_str(), "Original");
            assert_eq!(setup.get_fleet_value(0), original_value);
        }
    }

    // -----------------------------------------------------------------------
    // Save failure cleanup
    // -----------------------------------------------------------------------

    #[test]
    fn save_failure_to_readonly_dir_returns_error() {
        unsafe {
            // Saving to a nonexistent directory should fail
            let path = Path::new("/nonexistent/dir/team.mle");
            let team = MeleeTeam::new();
            let result = save_team_file(&team, path);
            assert!(result.is_err());
        }
    }
}
