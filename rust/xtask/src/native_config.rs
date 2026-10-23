//! What a native acceptance run needs to know before it starts: the scenario
//! inventory, the runtime limits every scenario runs under, and the content
//! package it plays against.
//!
//! Scenarios are whatever `rust/scripts/*.json` holds, so adding or editing
//! one needs no registration step.

use std::path::Path;

use serde::{Deserialize, Serialize};
use uqm_rust::automation::native_window::{
    NativeAcceptancePolicy, NativeInventoryLimits, NativeWindowBounds, NativeWindowRuntimeContract,
};
use uqm_rust::automation::{parse_script, validate_script};

pub const SCRIPTS_DIR: &str = "rust/scripts";

/// The scenario the single-script acceptance proves when no suite is selected.
pub const DEFAULT_SCENARIO: &str = "rust/scripts/linked-playable-v1.json";

pub const PLATFORM: &str = "macos";

pub const CONTENT_FILENAME: &str = "uqm-0.8.0-content.uqm";
pub const CONTENT_VERSION: &str = "0.8.0";
pub const CONTENT_BYTE_LENGTH: u64 = 11_547_353;
pub const CONTENT_SHA256: &str = "77d75ac25e6fb755a33c4ba3b38a7b7bc41fcbc02896891b0cc9ac9214b72eef";

/// Upper bound on the size of a scenario script read from the tree.
pub const SCRIPT_LIMIT_BYTES: u64 = 1024 * 1024;

/// Upper bound on the game executable the runner will copy into its bundle.
pub const EXECUTABLE_LIMIT_BYTES: u64 = 256 * 1024 * 1024;

/// A scenario whose declared wall budget outlasts the default outer deadline
/// runs under this one instead.
const LONG_SCRIPT_OUTER_CHILD_TIMEOUT_MS: u64 = 900_000;

pub const ACCEPTANCE_POLICY: NativeAcceptancePolicy = NativeAcceptancePolicy {
    stable_presentation_floor: 120,
    playable_presentation_floor: 300,
    battle_frame_floor: 300,
};

const RUNTIME_CONTRACT: NativeWindowRuntimeContract = NativeWindowRuntimeContract {
    capture_timeout_ms: 30_000,
    capture_kill_grace_ms: 5_000,
    observer_timeout_ms: 40_000,
    observer_kill_grace_ms: 5_000,
    acknowledgement_timeout_ms: 95_000,
    outer_child_timeout_ms: 300_000,
    outer_child_kill_grace_ms: 5_000,
    child_stdout_budget_bytes: 16 * 1024 * 1024,
    child_stderr_budget_bytes: 16 * 1024 * 1024,
    observer_response_budget_bytes: 64 * 1024,
    capture_budget_bytes: 64 * 1024 * 1024,
    content_expansion_budget_bytes: 32 * 1024 * 1024,
    inventory_limits: NativeInventoryLimits {
        member_count: 10_000,
        member_bytes: 64 * 1024 * 1024,
        aggregate_bytes: 1024 * 1024 * 1024,
        path_bytes: 4096,
        aggregate_path_bytes: 8 * 1024 * 1024,
    },
    expected_client_bounds: NativeWindowBounds {
        x: 80,
        y: 80,
        width: 1280,
        height: 960,
    },
};

/// One scenario script the suite may run.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub path: String,
    /// Present only on scenarios that outlast the default outer deadline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outer_child_timeout_ms: Option<u64>,
}

/// The runtime contract shared by every scenario.
#[must_use]
pub fn runtime_contract() -> NativeWindowRuntimeContract {
    RUNTIME_CONTRACT
}

impl Scenario {
    /// The runtime contract this scenario runs under.
    #[must_use]
    pub fn runtime_contract(&self) -> NativeWindowRuntimeContract {
        let mut contract = RUNTIME_CONTRACT;
        if let Some(timeout) = self.outer_child_timeout_ms {
            contract.outer_child_timeout_ms = timeout;
        }
        contract
    }
}

fn scenario_for(root: &Path, path: String) -> Result<Scenario, String> {
    let full = root.join(&path);
    let bytes = std::fs::read(&full).map_err(|error| format!("read {path}: {error}"))?;
    let document = parse_script(&bytes, &full).map_err(|error| format!("{path}: {error}"))?;
    let script = validate_script(document, &full).map_err(|error| format!("{path}: {error}"))?;
    let wall_ms = script
        .budgets()
        .max_wallclock_seconds
        .checked_mul(1000)
        .ok_or_else(|| format!("{path}: wall budget overflows milliseconds"))?;
    let outer_child_timeout_ms = if wall_ms > RUNTIME_CONTRACT.outer_child_timeout_ms {
        // Let the child report its own deadline and finish a pending observation.
        let deadline = wall_ms
            .checked_add(RUNTIME_CONTRACT.acknowledgement_timeout_ms)
            .and_then(|value| value.checked_add(RUNTIME_CONTRACT.outer_child_kill_grace_ms))
            .ok_or_else(|| format!("{path}: outer deadline overflows milliseconds"))?;
        Some(deadline.max(LONG_SCRIPT_OUTER_CHILD_TIMEOUT_MS))
    } else {
        None
    };
    Ok(Scenario {
        path,
        outer_child_timeout_ms,
    })
}

/// Every scenario under `rust/scripts`, in name order.
pub fn scenarios(root: &Path) -> Result<Vec<Scenario>, String> {
    let directory = root.join(SCRIPTS_DIR);
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(&directory)
        .map_err(|error| format!("read {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|name| name.ends_with(".json")) else {
            continue;
        };
        paths.push(format!("{SCRIPTS_DIR}/{name}"));
    }
    paths.sort();
    paths
        .into_iter()
        .map(|path| scenario_for(root, path))
        .collect()
}

/// The scenarios a run executes: those named in `requested` (names or paths,
/// separated by commas, spaces or newlines), or the default scenario alone.
pub fn select(root: &Path, requested: Option<&str>) -> Result<Vec<Scenario>, String> {
    let Some(requested) = requested else {
        return Ok(vec![scenario_for(root, DEFAULT_SCENARIO.to_string())?]);
    };
    let mut selected: Vec<Scenario> = Vec::new();
    for name in requested
        .split([',', '\n', ' '])
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let path = if name.contains('/') {
            name.to_string()
        } else {
            format!("{SCRIPTS_DIR}/{name}.json")
        };
        if !root.join(&path).is_file() {
            return Err(format!("scenario {name} does not exist at {path}"));
        }
        if !selected.iter().any(|existing| existing.path == path) {
            selected.push(scenario_for(root, path)?);
        }
    }
    if selected.is_empty() {
        return Err("the scenario selection is empty".to_string());
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("xtask lives two levels below the repository root")
            .to_path_buf()
    }

    #[test]
    fn every_script_in_the_directory_is_a_scenario() {
        let all = scenarios(&root()).expect("scripts parse");
        assert!(all.iter().any(|scenario| scenario.path == DEFAULT_SCENARIO));
        assert!(all
            .iter()
            .all(|scenario| scenario.path.starts_with("rust/scripts/")));
    }

    #[test]
    fn only_scenarios_past_the_default_deadline_get_the_long_one() {
        let all = scenarios(&root()).expect("scripts parse");
        let battle = all
            .iter()
            .find(|scenario| scenario.path.ends_with("/battle-v1.json"))
            .expect("battle-v1 exists");
        assert_eq!(battle.outer_child_timeout_ms, None);
        let long = all
            .iter()
            .find(|scenario| scenario.path.ends_with("/comm-encounter-v1.json"))
            .expect("comm-encounter-v1 exists");
        assert_eq!(
            long.runtime_contract().outer_child_timeout_ms,
            LONG_SCRIPT_OUTER_CHILD_TIMEOUT_MS
        );
        assert!(long.runtime_contract().has_valid_deadline_order());
    }

    #[test]
    fn outer_deadline_covers_the_scripts_declared_wall_budget() {
        let scenario = scenario_for(
            &root(),
            "rust/scripts/campaign-s07-taalo-protected.json".into(),
        )
        .expect("campaign scenario exists");
        let bytes = std::fs::read(root().join(&scenario.path)).unwrap();
        let document = parse_script(&bytes, Path::new(&scenario.path)).unwrap();
        let script = validate_script(document, Path::new(&scenario.path)).unwrap();
        assert!(
            scenario.runtime_contract().outer_child_timeout_ms
                > script.budgets().max_wallclock_seconds * 1000
        );
    }

    #[test]
    fn selection_accepts_names_and_paths_and_rejects_unknown() {
        let root = root();
        let selected = select(
            &root,
            Some("battle-v1, rust/scripts/battle-v1.json main-menu-v1"),
        )
        .expect("known scenarios select");
        assert_eq!(selected.len(), 2);
        assert!(select(&root, Some("no-such-scenario")).is_err());
        assert!(select(&root, Some(" ,")).is_err());
        assert_eq!(select(&root, None).expect("default").len(), 1);
    }
}
