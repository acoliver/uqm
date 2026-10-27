//! Domain and proof classification for the scenario matrix.

/// A gameplay domain the suite must cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    /// Reaching the main menu from a cold start.
    BootMenu,
    /// Beginning a new game.
    NewGame,
    /// Starbase arrival and interaction.
    Starbase,
    /// Choosing among alien conversation responses.
    CommunicationChoices,
    /// Interplanetary travel, orbit and scanning.
    NavigationOrbitScan,
    /// Surface operations and their outcomes.
    PlanetSideResults,
    /// Ship combat and its outcome.
    BattleOutcomes,
    /// Saving and restoring a game.
    SaveLoad,
    /// Leaving the game through the menu.
    Quit,
}

impl Domain {
    /// Every domain the suite is required to cover.
    pub const ALL: [Self; 9] = [
        Self::BootMenu,
        Self::NewGame,
        Self::Starbase,
        Self::CommunicationChoices,
        Self::NavigationOrbitScan,
        Self::PlanetSideResults,
        Self::BattleOutcomes,
        Self::SaveLoad,
        Self::Quit,
    ];

    /// The stable published name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BootMenu => "boot_menu",
            Self::NewGame => "new_game",
            Self::Starbase => "starbase",
            Self::CommunicationChoices => "communication_choices",
            Self::NavigationOrbitScan => "navigation_orbit_scan",
            Self::PlanetSideResults => "planet_side_results",
            Self::BattleOutcomes => "battle_outcomes",
            Self::SaveLoad => "save_load",
            Self::Quit => "quit",
        }
    }
}

/// Why a scenario carries no gameplay assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureKind {
    /// Uses granted development setup and cannot prove campaign ancestry.
    DevelopmentSetup,
    /// Deliberately hangs, to prove a watchdog fires.
    HangFixture,
    /// Deliberately idles, to prove the idle path is exercised.
    IdleFixture,
    /// Exercises the supervisor rather than the game.
    SupervisionFixture,
}

/// One published row: a scenario, what it covers, and what it proves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioRow {
    /// File stem under `rust/scripts`.
    pub scenario: &'static str,
    /// Domains this scenario covers. Empty only for a declared fixture.
    pub domains: &'static [Domain],
    /// Set when the scenario proves something other than gameplay.
    pub fixture: Option<FixtureKind>,
    /// Whether this scenario is part of the composed end-to-end journey.
    pub composed_journey: bool,
}
