//! The scenario, domain and assertion matrix.
//!
//! This is the published answer to "what does the autonomous suite actually
//! cover". It is code rather than a document because a document drifts: a
//! scenario can be added, renamed or emptied of assertions without anyone
//! noticing, and the matrix would still read as complete.
//!
//! The tests here hold three properties that a prose table cannot:
//!
//! - every gameplay domain is covered by at least one scenario,
//! - every scenario file on disk is classified, so a new one cannot be
//!   silently uncovered,
//! - every scenario either carries an assertion or is explicitly recorded as a
//!   fixture that proves something other than gameplay.

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

/// The complete matrix.
///
/// Adding a scenario file without adding a row here fails
/// [`tests::every_scenario_on_disk_is_classified`], which is the point.
pub const MATRIX: &[ScenarioRow] = &[
    row("main-menu-v1", &[Domain::BootMenu]),
    row("mode-main-menu-v1", &[Domain::BootMenu]),
    row("state-sync-v1", &[Domain::BootMenu]),
    row("quit-v1", &[Domain::Quit]),
    row("repro-game-menu-initial", &[Domain::BootMenu]),
    row("starbase-visit-v1", &[Domain::Starbase]),
    row("real-sol-probe-to-starbase", &[Domain::Starbase]),
    row(
        "real-sol-starbase-to-mars",
        &[Domain::Starbase, Domain::NavigationOrbitScan],
    ),
    row("comm-encounter-v1", &[Domain::CommunicationChoices]),
    row("repro-comm-menu-initial", &[Domain::CommunicationChoices]),
    row("repro-sol-probe-comm-ui", &[Domain::CommunicationChoices]),
    row("sol-probe-scene-v1", &[Domain::CommunicationChoices]),
    row(
        "real-sol-commander-completion",
        &[Domain::CommunicationChoices],
    ),
    row(
        "real-sol-probe-to-commander-responses",
        &[Domain::CommunicationChoices],
    ),
    row(
        "real-sol-probe-completion",
        &[Domain::CommunicationChoices, Domain::NewGame],
    ),
    row(
        "real-sol-probe-held-seek-completion",
        &[Domain::CommunicationChoices],
    ),
    row("probe-encounter-v1", &[Domain::NewGame]),
    row("probe-encounter-v2", &[Domain::NewGame]),
    row("probe-encounter-v3", &[Domain::NewGame]),
    row("probe-encounter-v4", &[Domain::NewGame]),
    row("probe-encounter-v5", &[Domain::NewGame]),
    row("probe-encounter-v6", &[Domain::NewGame]),
    row("probe-encounter-v7", &[Domain::NewGame]),
    row("explore-planet-v1", &[Domain::NavigationOrbitScan]),
    row("real-sol-autopilot", &[Domain::NavigationOrbitScan]),
    row("real-sol-earth-moon-base", &[Domain::NavigationOrbitScan]),
    row(
        "real-sol-luna-moonbase-departure",
        &[Domain::NavigationOrbitScan],
    ),
    row(
        "real-sol-planetside-collision-proof",
        &[Domain::PlanetSideResults],
    ),
    row(
        "real-sol-rust-planetside-smoke",
        &[Domain::PlanetSideResults],
    ),
    row("battle-v1", &[Domain::BattleOutcomes]),
    row("load-save-v1", &[Domain::SaveLoad]),
    journey(
        "linked-playable-v1",
        &[
            Domain::BootMenu,
            Domain::NewGame,
            Domain::CommunicationChoices,
            Domain::NavigationOrbitScan,
            Domain::BattleOutcomes,
        ],
    ),
    fixture("hard-hang", FixtureKind::HangFixture),
    fixture("inactive-smoke", FixtureKind::IdleFixture),
    fixture("watchdog-v1", FixtureKind::SupervisionFixture),
    fixture("probe-encounter-wait", FixtureKind::IdleFixture),
];

const fn row(scenario: &'static str, domains: &'static [Domain]) -> ScenarioRow {
    ScenarioRow {
        scenario,
        domains,
        fixture: None,
        composed_journey: false,
    }
}

const fn journey(scenario: &'static str, domains: &'static [Domain]) -> ScenarioRow {
    ScenarioRow {
        scenario,
        domains,
        fixture: None,
        composed_journey: true,
    }
}

const fn fixture(scenario: &'static str, kind: FixtureKind) -> ScenarioRow {
    ScenarioRow {
        scenario,
        domains: &[],
        fixture: Some(kind),
        composed_journey: false,
    }
}

/// Which domains a changed source path can affect.
///
/// The mapping is deliberately coarse. A path that maps to nothing is not
/// assumed harmless: [`select_for_changed_paths`] answers with the complete
/// required suite, because an unmapped path is a path nobody has reasoned
/// about, and guessing it is safe is how a regression ships.
#[must_use]
pub fn domains_for_path(path: &str) -> &'static [Domain] {
    // Longest-prefix first, so a specific rule wins over a general one.
    const RULES: &[(&str, &[Domain])] = &[
        ("rust/src/automation/", &[]), // harness: affects everything, handled below
        ("rust/src/comm/", &[Domain::CommunicationChoices]),
        (
            "rust/src/planets/",
            &[Domain::NavigationOrbitScan, Domain::PlanetSideResults],
        ),
        ("rust/src/battle/", &[Domain::BattleOutcomes]),
        ("rust/src/save/", &[Domain::SaveLoad]),
        ("rust/src/load/", &[Domain::SaveLoad]),
        (
            "rust/src/mainloop/restart_menu/",
            &[Domain::BootMenu, Domain::Quit],
        ),
        ("rust/src/starbase/", &[Domain::Starbase]),
        ("rust/scripts/", &[]), // scenarios themselves: run everything
    ];
    RULES
        .iter()
        .filter(|(prefix, _)| path.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map_or(&[], |(_, domains)| *domains)
}

/// Whether a path is mapped at all.
#[must_use]
pub fn path_is_mapped(path: &str) -> bool {
    !domains_for_path(path).is_empty()
}

/// The scenarios that must run for a set of changed paths.
///
/// Returns every gameplay scenario when any path is unmapped, which includes
/// harness and scenario changes because those can affect any domain.
#[must_use]
pub fn select_for_changed_paths(paths: &[String]) -> Vec<&'static str> {
    if paths.is_empty() || paths.iter().any(|path| !path_is_mapped(path)) {
        return required_suite();
    }
    let mut selected: Vec<&'static str> = Vec::new();
    for path in paths {
        for domain in domains_for_path(path) {
            for scenario in scenarios_for(*domain) {
                if !selected.contains(&scenario) {
                    selected.push(scenario);
                }
            }
        }
    }
    selected.sort_unstable();
    selected
}

/// Every scenario that proves gameplay, which is the full required suite.
#[must_use]
pub fn required_suite() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = MATRIX
        .iter()
        .filter(|row| row.fixture.is_none())
        .map(|row| row.scenario)
        .collect();
    all.sort_unstable();
    all
}

/// The scenarios covering a domain.
#[must_use]
pub fn scenarios_for(domain: Domain) -> Vec<&'static str> {
    MATRIX
        .iter()
        .filter(|row| row.domains.contains(&domain))
        .map(|row| row.scenario)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn script_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts")
    }

    fn scenarios_on_disk() -> BTreeSet<String> {
        std::fs::read_dir(script_dir())
            .expect("scripts directory")
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                    return None;
                }
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(str::to_string)
            })
            .collect()
    }

    /// Every file under a directory, following subdirectories.
    fn files_under(root: &std::path::Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let Ok(entries) = std::fs::read_dir(root) else {
            return found;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                found.extend(files_under(&path));
            } else {
                found.push(path);
            }
        }
        found
    }

    #[test]
    fn this_domain_owns_no_native_implementation() {
        // The autonomous-play domain is Rust from the start rather than a port
        // of a C provider, so its ownership outcome is a declaration that
        // nothing native remains. A declaration nobody checks decays, so this
        // checks it: if a C-family file ever appears under these roots, the
        // declaration is false and this fails rather than a reviewer having to
        // notice.
        const NATIVE: &[&str] = &["c", "h", "m", "mm", "cpp", "cc", "cxx", "hpp", "S", "s"];
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root")
            .to_path_buf();
        let roots = [
            repo.join("rust/src/automation"),
            repo.join("rust/harness"),
            repo.join(".github/workflows"),
        ];
        for root in roots {
            assert!(root.is_dir(), "{} is missing", root.display());
            for file in files_under(&root) {
                let extension = file
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                assert!(
                    !NATIVE.contains(&extension),
                    "{} is a native implementation inside a domain declared free of one",
                    file.display()
                );
            }
        }
    }

    #[test]
    fn every_domain_is_covered() {
        for domain in Domain::ALL {
            assert!(
                !scenarios_for(domain).is_empty(),
                "no scenario covers {}",
                domain.name()
            );
        }
    }

    #[test]
    fn every_scenario_on_disk_is_classified() {
        // A new scenario that nobody classified is a scenario nobody knows the
        // coverage of, which is exactly how a matrix stops being true.
        let classified: BTreeSet<String> =
            MATRIX.iter().map(|row| row.scenario.to_string()).collect();
        let on_disk = scenarios_on_disk();
        let unclassified: Vec<&String> = on_disk.difference(&classified).collect();
        assert!(
            unclassified.is_empty(),
            "scenarios on disk with no matrix row: {unclassified:?}"
        );
    }

    #[test]
    fn every_classified_scenario_exists() {
        let on_disk = scenarios_on_disk();
        let missing: Vec<&str> = MATRIX
            .iter()
            .map(|row| row.scenario)
            .filter(|scenario| !on_disk.contains(*scenario))
            .collect();
        assert!(missing.is_empty(), "matrix rows with no file: {missing:?}");
    }

    #[test]
    fn a_scenario_either_covers_a_domain_or_declares_why_not() {
        // Without this a scenario can sit in the suite proving nothing, and
        // the count of scenarios still looks healthy.
        for row in MATRIX {
            assert_ne!(
                row.domains.is_empty(),
                row.fixture.is_none(),
                "{} must cover a domain or declare a fixture kind, not both or neither",
                row.scenario
            );
        }
    }

    #[test]
    fn the_suite_has_a_composed_journey() {
        let journeys: Vec<&str> = MATRIX
            .iter()
            .filter(|row| row.composed_journey)
            .map(|row| row.scenario)
            .collect();
        assert!(!journeys.is_empty(), "no composed journey is declared");
        // A journey that covers one domain is an independent scenario wearing
        // the wrong label.
        for journey in &journeys {
            let row = MATRIX
                .iter()
                .find(|row| row.scenario == *journey)
                .expect("declared journey");
            assert!(
                row.domains.len() >= 3,
                "{journey} is labelled a composed journey but covers {} domain(s)",
                row.domains.len()
            );
        }
    }

    #[test]
    fn every_gameplay_scenario_asserts_something() {
        // A scenario with no assertion runs the game and proves nothing about
        // it. Fixtures are exempt because they prove supervision instead.
        for row in MATRIX.iter().filter(|row| row.fixture.is_none()) {
            let path = script_dir().join(format!("{}.json", row.scenario));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let document: serde_json::Value = serde_json::from_str(&text).expect("scenario parses");
            let steps = document["steps"].as_array().expect("steps array");
            let asserts = steps
                .iter()
                .filter_map(|step| step["action"].as_str())
                .filter(|action| action.starts_with("assert") || action.starts_with("wait_for"))
                .count();
            assert!(
                asserts > 0,
                "{} carries no assertion and proves nothing",
                row.scenario
            );
        }
    }

    #[test]
    fn an_unmapped_path_selects_the_complete_suite() {
        // The important half of the policy. A path nobody has classified is a
        // path nobody has reasoned about, so it must not narrow the suite.
        let selected = select_for_changed_paths(&["rust/src/nowhere/thing.rs".to_string()]);
        assert_eq!(selected, required_suite());

        // One unmapped path among mapped ones still widens to everything.
        let mixed = select_for_changed_paths(&[
            "rust/src/battle/x.rs".to_string(),
            "docs/whatever.md".to_string(),
        ]);
        assert_eq!(mixed, required_suite());
    }

    #[test]
    fn no_changed_paths_selects_the_complete_suite() {
        assert_eq!(select_for_changed_paths(&[]), required_suite());
    }

    #[test]
    fn a_mapped_path_selects_its_domains_and_nothing_else() {
        let selected = select_for_changed_paths(&["rust/src/battle/frame.rs".to_string()]);
        assert!(selected.contains(&"battle-v1"));
        assert!(
            !selected.contains(&"load-save-v1"),
            "a battle change must not drag in save/load: {selected:?}"
        );
        assert!(selected.len() < required_suite().len());
    }

    #[test]
    fn the_longest_matching_prefix_wins() {
        // rust/src/mainloop/restart_menu/ is more specific than any shorter
        // rule that might later be added above it.
        let domains = domains_for_path("rust/src/mainloop/restart_menu/input.rs");
        assert!(domains.contains(&Domain::Quit));
        assert!(domains.contains(&Domain::BootMenu));
    }

    #[test]
    fn selection_never_returns_a_fixture() {
        // Fixtures prove supervision, not gameplay, so a gameplay selection
        // that includes one is a selection that will look busier than it is.
        let fixtures: Vec<&str> = MATRIX
            .iter()
            .filter(|row| row.fixture.is_some())
            .map(|row| row.scenario)
            .collect();
        for scenario in required_suite() {
            assert!(!fixtures.contains(&scenario), "{scenario} is a fixture");
        }
    }

    #[test]
    fn every_selected_scenario_has_a_file() {
        let on_disk = scenarios_on_disk();
        for scenario in required_suite() {
            assert!(on_disk.contains(scenario), "{scenario} has no file");
        }
    }

    #[test]
    fn domain_names_are_unique_and_stable() {
        let names: BTreeSet<&str> = Domain::ALL.iter().map(|domain| domain.name()).collect();
        assert_eq!(names.len(), Domain::ALL.len());
    }
}
