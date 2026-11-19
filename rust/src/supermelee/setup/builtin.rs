//! The fifteen built-in Super Melee teams (`loadmele.c` `InitPreBuilt`).

use crate::supermelee::setup::team::MeleeTeam;
use crate::supermelee::types::MeleeShip;

/// Number of built-in pre-defined teams (matches C `PREBUILT_COUNT`).
pub const PREBUILT_COUNT: usize = 15;

/// Helper to build a team from a name and ship list.
fn make_team(name: &str, ships: &[MeleeShip]) -> MeleeTeam {
    let mut team = MeleeTeam::new();
    team.set_name(name);
    for (slot, &ship) in team.ships.iter_mut().zip(ships) {
        *slot = ship;
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

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Built-in team catalog
    // -----------------------------------------------------------------------

    #[test]
    fn builtin_catalog_has_correct_count() {
        let teams = builtin_teams();
        assert_eq!(teams.len(), PREBUILT_COUNT);
    }

    #[test]
    fn builtin_teams_have_names() {
        for team in &builtin_teams() {
            assert!(!team.name_str().is_empty(), "built-in team has no name");
        }
    }

    #[test]
    fn builtin_teams_have_at_least_one_ship() {
        for team in &builtin_teams() {
            let has_ship = team.ships.iter().any(|&s| s != MeleeShip::MeleeNone);
            assert!(has_ship, "built-in team '{}' has no ships", team.name_str());
        }
    }

    #[test]
    fn builtin_team_names_match_c() {
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
