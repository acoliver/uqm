//! The mode, observation and legal-action table.
//!
//! One authority answers three questions for both a human at the keyboard and
//! an automation script: what mode is the game in, which observation proves
//! it, and which inputs are legal there.
//!
//! Mode is derived from the game's own activity word rather than from screen
//! coordinates or timing, so the answer means the same thing on every platform
//! and at every window size. An activity word that does not name exactly one
//! mode is reported as ambiguous and never guessed at: acting on a guess is
//! how an automated run ends up asserting something it never established.

use crate::automation::script::{MenuKey, PlayerKey};

/// The low byte of the activity word, which names what the game is doing.
///
/// Values mirror the `ACTIVITY` enum in `sc2/src/uqm/globdata.h`. They are
/// states, not flags: exactly one is current.
mod activity_state {
    pub const SUPER_MELEE: u16 = 0;
    pub const IN_LAST_BATTLE: u16 = 1;
    pub const IN_ENCOUNTER: u16 = 2;
    pub const IN_HYPERSPACE: u16 = 3;
    pub const IN_INTERPLANETARY: u16 = 4;
    // States 6 through 8, IN_QUASISPACE, IN_PLANET_ORBIT and IN_STARBASE, are
    // documented in globdata.h as used only when displaying save game
    // summaries. A live run never reports them, so deriving orbit or starbase
    // from the activity word would produce a table that reads as complete and
    // never fires. Those modes take an explicit observation instead.
}

/// The high byte of the activity word, which carries independent flags.
mod activity_flag {
    pub const IN_BATTLE: u16 = 1 << 9;
    pub const CHECK_LOAD: u16 = 1 << 12;
    pub const CHECK_ABORT: u16 = 1 << 14;
}

/// The mask selecting the state nibble of the activity word.
const STATE_MASK: u16 = 0x00ff;

/// A gameplay mode, as observed rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GameMode {
    /// The main menu, which the game runs under the Super Melee activity.
    MainMenu,
    /// Starbase interior.
    Starbase,
    /// An alien communication exchange.
    Communication,
    /// The hyperspace or quasispace starmap.
    Starmap,
    /// Interplanetary flight inside a solar system.
    Solar,
    /// Planet orbit, from which a lander is dispatched.
    Orbit,
    /// Surface operations after landing.
    PlanetSide,
    /// Ship-to-ship combat.
    Battle,
    /// The save or load picker.
    SaveLoad,
    /// The confirmed quit path.
    Quit,
}

impl<'de> serde::Deserialize<'de> for GameMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::from_name(&name).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "unknown mode '{name}'; expected one of: {}",
                Self::ALL
                    .iter()
                    .map(|mode| mode.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
    }
}

impl GameMode {
    /// Every mode, so a caller can prove it handled the whole table.
    pub const ALL: [Self; 10] = [
        Self::MainMenu,
        Self::Starbase,
        Self::Communication,
        Self::Starmap,
        Self::Solar,
        Self::Orbit,
        Self::PlanetSide,
        Self::Battle,
        Self::SaveLoad,
        Self::Quit,
    ];

    /// The stable script-facing name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MainMenu => "main_menu",
            Self::Starbase => "starbase",
            Self::Communication => "communication",
            Self::Starmap => "starmap",
            Self::Solar => "solar",
            Self::Orbit => "orbit",
            Self::PlanetSide => "planet_side",
            Self::Battle => "battle",
            Self::SaveLoad => "save_load",
            Self::Quit => "quit",
        }
    }

    /// Parse a script-facing name. Unknown names are rejected, not guessed.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }

    /// The observation that establishes this mode, named for a proof bundle.
    #[must_use]
    pub const fn observation(self) -> &'static str {
        match self {
            Self::MainMenu => "activity.state == super_melee",
            Self::Starbase => "observation.docked_at_starbase",
            Self::Communication => "activity.state == in_encounter",
            Self::Starmap => "activity.state == in_hyperspace",
            Self::Solar => "activity.state == in_interplanetary",
            Self::Orbit => "observation.in_planet_orbit && no lander deployed",
            Self::PlanetSide => "observation.in_planet_orbit && lander deployed",
            Self::Battle => "activity.flag & in_battle, outside hyperspace",
            Self::SaveLoad => "activity.flag & check_load",
            Self::Quit => "activity.flag & check_abort",
        }
    }

    /// Whether this mode is driven by menu keys rather than piloting keys.
    #[must_use]
    pub const fn is_menu_driven(self) -> bool {
        matches!(
            self,
            Self::MainMenu | Self::Starbase | Self::Communication | Self::SaveLoad | Self::Quit
        )
    }

    /// The menu keys that are legal in this mode.
    #[must_use]
    pub fn legal_menu_keys(self) -> &'static [MenuKey] {
        const FULL: [MenuKey; 6] = [
            MenuKey::Up,
            MenuKey::Down,
            MenuKey::Left,
            MenuKey::Right,
            MenuKey::Select,
            MenuKey::Cancel,
        ];
        const VERTICAL: [MenuKey; 4] =
            [MenuKey::Up, MenuKey::Down, MenuKey::Select, MenuKey::Cancel];
        const CONFIRM: [MenuKey; 3] = [MenuKey::Left, MenuKey::Right, MenuKey::Select];
        match self {
            // A conversation offers a vertical list of responses.
            Self::Communication => &VERTICAL,
            // The quit prompt is a yes/no confirmation, not a list.
            Self::Quit => &CONFIRM,
            Self::MainMenu | Self::Starbase | Self::SaveLoad => &FULL,
            // Piloting modes still accept cancel, which opens their menu.
            Self::Starmap | Self::Solar | Self::Orbit | Self::PlanetSide | Self::Battle => {
                const ESCAPE_ONLY: [MenuKey; 1] = [MenuKey::Cancel];
                &ESCAPE_ONLY
            }
        }
    }

    /// The piloting keys that are legal in this mode.
    #[must_use]
    pub fn legal_player_keys(self) -> &'static [PlayerKey] {
        const PILOT: [PlayerKey; 5] = [
            PlayerKey::Thrust,
            PlayerKey::Down,
            PlayerKey::Left,
            PlayerKey::Right,
            PlayerKey::Escape,
        ];
        const COMBAT: [PlayerKey; 7] = [
            PlayerKey::Thrust,
            PlayerKey::Down,
            PlayerKey::Left,
            PlayerKey::Right,
            PlayerKey::Weapon,
            PlayerKey::Special,
            PlayerKey::Escape,
        ];
        match self {
            Self::Battle => &COMBAT,
            // A lander thrusts and steers but has no weapon loadout.
            Self::PlanetSide | Self::Starmap | Self::Solar | Self::Orbit => &PILOT,
            // A menu accepts no piloting input at all.
            Self::MainMenu | Self::Starbase | Self::Communication | Self::SaveLoad | Self::Quit => {
                &[]
            }
        }
    }

    /// Whether a menu key is legal here.
    #[must_use]
    pub fn permits_menu_key(self, key: MenuKey) -> bool {
        self.legal_menu_keys().contains(&key)
    }

    /// Whether a piloting key is legal here.
    #[must_use]
    pub fn permits_player_key(self, key: PlayerKey) -> bool {
        self.legal_player_keys().contains(&key)
    }
}

/// The result of asking what mode the game is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeObservation {
    /// Exactly one mode is consistent with the observation.
    Determined(GameMode),
    /// The observation names no mode, or more than one.
    ///
    /// Carries the activity word and the candidates, because a caller that
    /// fails on ambiguity has to be able to say what was ambiguous.
    Ambiguous {
        activity: u16,
        candidates: Vec<GameMode>,
    },
}

impl ModeObservation {
    /// The mode, or an error naming the ambiguity.
    ///
    /// # Errors
    ///
    /// Returns the ambiguity description when the observation did not name
    /// exactly one mode.
    pub fn require(self) -> Result<GameMode, String> {
        match self {
            Self::Determined(mode) => Ok(mode),
            Self::Ambiguous {
                activity,
                candidates,
            } => {
                let names: Vec<&str> = candidates.iter().map(|mode| mode.name()).collect();
                Err(format!(
                    "ambiguous mode for activity 0x{activity:04x}: {}",
                    if names.is_empty() {
                        "no mode matches".to_string()
                    } else {
                        names.join(", ")
                    }
                ))
            }
        }
    }
}

/// Whether a lander is currently deployed on a planet surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfacePresence {
    /// No lander is deployed.
    InOrbit,
    /// A lander is on the surface.
    OnSurface,
}

/// Everything the mode derivation is allowed to look at.
///
/// The activity word alone cannot name every mode, because the states for
/// orbit and starbase are reserved for save summaries and never appear in a
/// live run. Those arrive as explicit observations rather than being inferred,
/// so a caller can see exactly what each answer rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observations {
    /// The live activity word.
    pub activity: u16,
    /// Whether a lander is deployed, which separates orbit from the surface.
    pub surface: SurfacePresence,
    /// Whether the ship is docked, which the activity word cannot report.
    pub docked_at_starbase: bool,
    /// Whether the ship is in planet orbit, which the activity word cannot
    /// report either.
    pub in_planet_orbit: bool,
}

impl Observations {
    /// An observation carrying only the activity word.
    #[must_use]
    pub const fn from_activity(activity: u16) -> Self {
        Self {
            activity,
            surface: SurfacePresence::InOrbit,
            docked_at_starbase: false,
            in_planet_orbit: false,
        }
    }
}

/// Derive the mode from everything observed.
///
/// Precedence is deliberate and ordered from most modal to least. A quit
/// prompt covers a load picker, a load picker covers whatever raised it, and
/// combat covers the state byte, because `IN_BATTLE` is raised while the state
/// byte still reads whatever the player was doing beforehand. Getting this
/// order wrong is how a real battle at activity 0x0200 reads as a main menu.
#[must_use]
pub fn observe(observations: Observations) -> ModeObservation {
    let activity = observations.activity;

    if activity & activity_flag::CHECK_ABORT != 0 && activity & activity_flag::CHECK_LOAD != 0 {
        return ModeObservation::Ambiguous {
            activity,
            candidates: vec![GameMode::Quit, GameMode::SaveLoad],
        };
    }
    if activity & activity_flag::CHECK_ABORT != 0 {
        return ModeObservation::Determined(GameMode::Quit);
    }
    if activity & activity_flag::CHECK_LOAD != 0 {
        return ModeObservation::Determined(GameMode::SaveLoad);
    }

    // IN_BATTLE is also raised while flying hyperspace, where it means travel
    // rather than combat, so hyperspace is excluded before trusting the flag.
    let state = activity & STATE_MASK;
    if activity & activity_flag::IN_BATTLE != 0 && state != activity_state::IN_HYPERSPACE {
        return ModeObservation::Determined(GameMode::Battle);
    }
    if state == activity_state::IN_LAST_BATTLE {
        return ModeObservation::Determined(GameMode::Battle);
    }

    // Neither of these is derivable from the activity word in a live run.
    if observations.docked_at_starbase {
        return ModeObservation::Determined(GameMode::Starbase);
    }
    if observations.in_planet_orbit {
        return ModeObservation::Determined(match observations.surface {
            SurfacePresence::OnSurface => GameMode::PlanetSide,
            SurfacePresence::InOrbit => GameMode::Orbit,
        });
    }

    match state {
        activity_state::SUPER_MELEE => ModeObservation::Determined(GameMode::MainMenu),
        activity_state::IN_ENCOUNTER => ModeObservation::Determined(GameMode::Communication),
        activity_state::IN_HYPERSPACE => ModeObservation::Determined(GameMode::Starmap),
        activity_state::IN_INTERPLANETARY => ModeObservation::Determined(GameMode::Solar),
        _ => ModeObservation::Ambiguous {
            activity,
            candidates: Vec::new(),
        },
    }
}

/// Observe the live game.
///
/// Every field comes from a signal the game already publishes, so the table is
/// the same authority a human and a script both see rather than a parallel
/// model that can drift.
///
/// Starbase has no activity state of its own in a live run: `IN_STARBASE` is
/// reserved for save summaries, and docking presents the commander as a scene.
/// The scene is therefore the observation, which is why this is read here
/// rather than inferred from the activity word.
#[must_use]
pub fn live() -> Observations {
    use crate::automation::scenario::{active_scene, AutomationScene};
    use crate::automation::ui_observation::{planet_menu_phase, PlanetMenuPhase};

    let phase = planet_menu_phase();
    Observations {
        activity: crate::mainloop::ffi::get_current_activity().0,
        surface: if phase == PlanetMenuPhase::LandingSite {
            SurfacePresence::OnSurface
        } else {
            SurfacePresence::InOrbit
        },
        docked_at_starbase: active_scene() == Some(AutomationScene::StarbaseCommander),
        in_planet_orbit: phase != PlanetMenuPhase::Inactive,
    }
}

/// Observe the live game and require an unambiguous mode.
///
/// # Errors
///
/// Returns the ambiguity description when the observation does not name
/// exactly one mode.
pub fn require_live() -> Result<GameMode, String> {
    observe(live()).require()
}

/// One published row of the mode table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeRow {
    pub mode: &'static str,
    pub observation: &'static str,
    pub menu_keys: Vec<&'static str>,
    pub player_keys: Vec<&'static str>,
}

/// The complete published table, in a stable order.
#[must_use]
pub fn table() -> Vec<ModeRow> {
    GameMode::ALL
        .into_iter()
        .map(|mode| ModeRow {
            mode: mode.name(),
            observation: mode.observation(),
            menu_keys: mode
                .legal_menu_keys()
                .iter()
                .map(|key| key.name())
                .collect(),
            player_keys: mode
                .legal_player_keys()
                .iter()
                .map(|key| key.name())
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_is_published_with_an_observation() {
        let rows = table();
        assert_eq!(rows.len(), GameMode::ALL.len());
        for row in &rows {
            assert!(
                !row.observation.is_empty(),
                "{} has no observation",
                row.mode
            );
            assert!(
                !row.menu_keys.is_empty() || !row.player_keys.is_empty(),
                "{} accepts no input at all",
                row.mode
            );
        }
    }

    #[test]
    fn mode_names_round_trip_and_reject_the_unknown() {
        for mode in GameMode::ALL {
            assert_eq!(GameMode::from_name(mode.name()), Some(mode));
        }
        assert_eq!(GameMode::from_name("teleporting"), None);
    }

    #[test]
    fn the_activity_values_shipped_scripts_assert_resolve_correctly() {
        // These are the exact assertions in rust/scripts, so the table is
        // checked against what the game actually reports rather than against
        // my reading of the header.
        // battle-v1: mask 0x02ff, equals 0x0200.
        assert_eq!(
            observe(Observations::from_activity(0x0200)).require(),
            Ok(GameMode::Battle),
            "a real battle must not read as a main menu"
        );
        // comm-encounter-v1: state 4.
        assert_eq!(
            observe(Observations::from_activity(4)).require(),
            Ok(GameMode::Solar)
        );
        // explore-planet-v1 and load-save-v1: state 3.
        assert_eq!(
            observe(Observations::from_activity(3)).require(),
            Ok(GameMode::Starmap)
        );
    }

    #[test]
    fn each_live_state_determines_its_mode() {
        let cases = [
            (activity_state::SUPER_MELEE, GameMode::MainMenu),
            (activity_state::IN_ENCOUNTER, GameMode::Communication),
            (activity_state::IN_HYPERSPACE, GameMode::Starmap),
            (activity_state::IN_INTERPLANETARY, GameMode::Solar),
            (activity_state::IN_LAST_BATTLE, GameMode::Battle),
        ];
        for (activity, expected) in cases {
            assert_eq!(
                observe(Observations::from_activity(activity)).require(),
                Ok(expected),
                "activity 0x{activity:04x}"
            );
        }
    }

    #[test]
    fn orbit_and_starbase_come_from_observation_not_the_activity_word() {
        // globdata.h reserves those states for save summaries, so a live run
        // never reports them and the word alone cannot answer.
        let docked = Observations {
            docked_at_starbase: true,
            ..Observations::from_activity(activity_state::IN_INTERPLANETARY)
        };
        assert_eq!(observe(docked).require(), Ok(GameMode::Starbase));

        let orbiting = Observations {
            in_planet_orbit: true,
            ..Observations::from_activity(activity_state::IN_INTERPLANETARY)
        };
        assert_eq!(observe(orbiting).require(), Ok(GameMode::Orbit));

        let landed = Observations {
            in_planet_orbit: true,
            surface: SurfacePresence::OnSurface,
            ..Observations::from_activity(activity_state::IN_INTERPLANETARY)
        };
        assert_eq!(observe(landed).require(), Ok(GameMode::PlanetSide));
    }

    #[test]
    fn a_modal_overlay_outranks_the_state_beneath_it() {
        let load =
            Observations::from_activity(activity_state::IN_HYPERSPACE | activity_flag::CHECK_LOAD);
        assert_eq!(observe(load).require(), Ok(GameMode::SaveLoad));

        let quit = Observations::from_activity(
            activity_state::IN_INTERPLANETARY | activity_flag::CHECK_ABORT,
        );
        assert_eq!(observe(quit).require(), Ok(GameMode::Quit));
    }

    #[test]
    fn the_battle_flag_in_hyperspace_is_travel_not_combat() {
        // IN_BATTLE is raised while flying hyperspace, which is exactly the
        // overlap that makes a naive flag test wrong.
        let travelling =
            Observations::from_activity(activity_state::IN_HYPERSPACE | activity_flag::IN_BATTLE);
        assert_eq!(observe(travelling).require(), Ok(GameMode::Starmap));

        // Outside hyperspace the same flag does mean combat.
        let fighting = Observations::from_activity(
            activity_state::IN_INTERPLANETARY | activity_flag::IN_BATTLE,
        );
        assert_eq!(observe(fighting).require(), Ok(GameMode::Battle));
    }

    #[test]
    fn an_unnamed_state_is_ambiguous_rather_than_guessed() {
        // State 5 is WON_LAST_BATTLE, which names no interactive mode, so the
        // answer is that we do not know rather than a nearest match.
        let observation = observe(Observations::from_activity(5));
        assert!(matches!(observation, ModeObservation::Ambiguous { .. }));
        let error = observation.require().unwrap_err();
        assert!(error.contains("no mode matches"), "{error}");
    }

    #[test]
    fn two_raised_overlays_are_ambiguous() {
        let both =
            Observations::from_activity(activity_flag::CHECK_LOAD | activity_flag::CHECK_ABORT);
        let error = observe(both).require().unwrap_err();
        assert!(error.contains("save_load"), "{error}");
        assert!(error.contains("quit"), "{error}");
    }

    #[test]
    fn a_menu_mode_refuses_piloting_input() {
        for mode in GameMode::ALL
            .into_iter()
            .filter(|mode| mode.is_menu_driven())
        {
            assert!(
                mode.legal_player_keys().is_empty(),
                "{} must not accept piloting input",
                mode.name()
            );
            assert!(!mode.permits_player_key(PlayerKey::Thrust));
        }
    }

    #[test]
    fn only_combat_offers_weapons() {
        for mode in GameMode::ALL {
            assert_eq!(
                mode.permits_player_key(PlayerKey::Weapon),
                mode == GameMode::Battle,
                "{} weapon legality",
                mode.name()
            );
            assert_eq!(
                mode.permits_player_key(PlayerKey::Special),
                mode == GameMode::Battle,
                "{} special legality",
                mode.name()
            );
        }
    }

    #[test]
    fn every_piloting_mode_can_open_its_menu() {
        for mode in GameMode::ALL
            .into_iter()
            .filter(|mode| !mode.is_menu_driven())
        {
            assert!(
                mode.permits_menu_key(MenuKey::Cancel),
                "{} cannot open a menu",
                mode.name()
            );
        }
    }
}
