//! Assertion ratchet for `rust/scripts/*.json`: a scenario may gain checks
//! and tighten them, but losing or loosening one needs an explicit waiver
//! trailer (`Weakens-assertion: <reason>`) on a commit in `base..head`.
//!
//! The comparison is a pure function over two parsed [`ScenarioSet`]s; the git
//! layer only fetches the sets and the waiver trailers.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use super::authority::Authority;
use super::delta::{commit_waivers, supervised_git, Waiver};

const SCRIPT_DIR: &str = "rust/scripts/";
const GATES_PATH: &str = "rust/ci/gates.json";
const WAIVER_KEY: &str = "Weakens-assertion";

/// A timeout or tick budget may grow by up to this factor without counting as
/// a weakening. Anything beyond it is.
const BUDGET_GROWTH_LIMIT: u64 = 2;

/// Actions whose success is itself a check. A script that loses one of these
/// steps stops proving something it proved before.
///
/// Every action in `Action` (rust/src/automation/script.rs) is classified in
/// [`ActionClass`]; an action missing from that table is treated as an
/// assertion so a new action fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionClass {
    Assertion,
    Plain,
}

/// Actions that only drive input or time and carry no check of their own.
const PLAIN_ACTIONS: &[&str] = &[
    "wait_input_ticks",
    "wait_presentations",
    "set_menu_key",
    "tap_menu_key",
    "set_player_key",
    "tap_player_key",
    "turn_flagship",
    "select_planet_menu",
    "record_hyperspace_position",
    "record_saved_hyperspace_position",
    "setup_planet_side_collision_fixture",
    "finish",
];

pub fn classify_action(action: &str) -> ActionClass {
    if PLAIN_ACTIONS.contains(&action) {
        ActionClass::Plain
    } else {
        ActionClass::Assertion
    }
}

/// Scenario scripts and the per-scenario gate entries at one revision.
#[derive(Debug, Default, Clone)]
pub struct ScenarioSet {
    /// Script path to parsed script.
    pub scripts: BTreeMap<String, Value>,
    /// Scenario path listed in `native_acceptance.scenario_scripts` to its
    /// `outer_child_timeout_ms`, when it has one.
    pub gate_entries: BTreeMap<String, Option<u64>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Weakening {
    pub scenario: String,
    pub step: String,
    pub change: String,
}

#[derive(Debug, Serialize)]
pub struct RatchetReport {
    pub weakenings: Vec<Weakening>,
    pub waivers: Vec<Waiver>,
    pub passed: bool,
}

impl RatchetReport {
    pub fn new(weakenings: Vec<Weakening>, waivers: Vec<Waiver>) -> Self {
        let passed = weakenings.is_empty() || !waivers.is_empty();
        Self {
            weakenings,
            waivers,
            passed,
        }
    }
}

/// (action, label) of a step.
type StepKey = (String, String);
/// Base and head steps that share a [`StepKey`].
type StepPair<'a> = (Vec<&'a Value>, Vec<&'a Value>);

/// How a field of an assertion-class step is allowed to change.
enum FieldRule {
    /// Raising is allowed, lowering is a weakening (`minimum*`, `count`).
    Floor,
    /// Raising by more than [`BUDGET_GROWTH_LIMIT`] is a weakening
    /// (`max_ticks`, `*timeout*`).
    Budget,
    /// `false` to `true` is allowed, `true` to `false` is a weakening.
    Requirement,
    /// Any change is a weakening (`equals`, `mode`, `encounter`, ...).
    Exact,
}

fn field_rule(key: &str) -> FieldRule {
    if key.starts_with("minimum") || key == "count" {
        FieldRule::Floor
    } else if key == "max_ticks" || key.contains("timeout") {
        FieldRule::Budget
    } else if key == "expect_change" {
        FieldRule::Requirement
    } else {
        FieldRule::Exact
    }
}

/// Compare every scenario at `base` with its counterpart at `head`.
pub fn compare(base: &ScenarioSet, head: &ScenarioSet) -> Vec<Weakening> {
    let mut found = Vec::new();
    for (path, base_script) in &base.scripts {
        match head.scripts.get(path) {
            None => found.push(weakening(path, "(file)", "scenario script removed")),
            Some(head_script) => compare_script(path, base_script, head_script, &mut found),
        }
    }
    for (path, base_timeout) in &base.gate_entries {
        match head.gate_entries.get(path) {
            None => found.push(weakening(
                path,
                "(gates.json)",
                "scenario removed from native_acceptance.scenario_scripts",
            )),
            Some(head_timeout) => {
                compare_outer_timeout(path, *base_timeout, *head_timeout, &mut found)
            }
        }
    }
    found
}

fn weakening(scenario: &str, step: &str, change: impl Into<String>) -> Weakening {
    Weakening {
        scenario: scenario.into(),
        step: step.into(),
        change: change.into(),
    }
}

fn compare_outer_timeout(
    path: &str,
    base: Option<u64>,
    head: Option<u64>,
    found: &mut Vec<Weakening>,
) {
    let Some(base) = base else { return };
    match head {
        Some(head) if head <= base => {}
        Some(head) => found.push(weakening(
            path,
            "(gates.json)",
            format!("outer_child_timeout_ms raised {base} -> {head}"),
        )),
        None => found.push(weakening(
            path,
            "(gates.json)",
            format!("outer_child_timeout_ms {base} removed"),
        )),
    }
}

fn compare_script(path: &str, base: &Value, head: &Value, found: &mut Vec<Weakening>) {
    compare_budgets(path, base, head, found);
    let base_steps = checking_steps(base);
    let head_steps = checking_steps(head);

    // Steps are matched as a multiset keyed by (action, label), so reordering
    // and added steps never register. Identical steps cancel first; whatever
    // remains in a bucket is paired in document order and compared field by
    // field, and an unpaired base step was removed.
    let mut buckets: BTreeMap<StepKey, StepPair> = BTreeMap::new();
    for step in base_steps {
        buckets.entry(step_key(step)).or_default().0.push(step);
    }
    for step in head_steps {
        buckets.entry(step_key(step)).or_default().1.push(step);
    }
    for ((action, label), (mut base_rest, mut head_rest)) in buckets {
        base_rest.retain(
            |step| match head_rest.iter().position(|other| other == step) {
                Some(index) => {
                    head_rest.remove(index);
                    false
                }
                None => true,
            },
        );
        let name = if label.is_empty() {
            action
        } else {
            format!("{action} \"{label}\"")
        };
        for (index, base_step) in base_rest.iter().enumerate() {
            match head_rest.get(index) {
                None => found.push(weakening(
                    path,
                    &name,
                    format!("{} removed", describe_step(base_step)),
                )),
                Some(head_step) => compare_step_fields(path, &name, base_step, head_step, found),
            }
        }
    }
}

fn checking_steps(script: &Value) -> Vec<&Value> {
    script
        .get("steps")
        .and_then(Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .filter(|step| {
                    classify_action(step.get("action").and_then(Value::as_str).unwrap_or(""))
                        == ActionClass::Assertion
                })
                .collect()
        })
        .unwrap_or_default()
}

fn step_key(step: &Value) -> (String, String) {
    let text = |key: &str| {
        step.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    (text("action"), text("label"))
}

fn describe_step(step: &Value) -> String {
    serde_json::to_string(step).unwrap_or_else(|_| "<unprintable step>".into())
}

fn compare_budgets(path: &str, base: &Value, head: &Value, found: &mut Vec<Weakening>) {
    let Some(base_budgets) = base.get("budgets").and_then(Value::as_object) else {
        return;
    };
    for (key, base_value) in base_budgets {
        let head_value = head.get("budgets").and_then(|budgets| budgets.get(key));
        if let Some(change) = loosened(&FieldRule::Budget, base_value, head_value) {
            found.push(weakening(path, &format!("budgets.{key}"), change));
        }
    }
}

fn compare_step_fields(
    path: &str,
    name: &str,
    base: &Value,
    head: &Value,
    found: &mut Vec<Weakening>,
) {
    let Some(base_fields) = base.as_object() else {
        return;
    };
    for (key, base_value) in base_fields {
        let rule = field_rule(key);
        if let Some(change) = loosened(&rule, base_value, head.get(key)) {
            found.push(weakening(path, name, format!("{key}: {change}")));
        }
    }
}

/// Describe how `head` loosens `base` under `rule`, or `None` when it does
/// not.
fn loosened(rule: &FieldRule, base: &Value, head: Option<&Value>) -> Option<String> {
    let Some(head) = head else {
        return Some(format!("{base} removed"));
    };
    match (rule, base.as_u64(), head.as_u64()) {
        (FieldRule::Floor, Some(old), Some(new)) => {
            (new < old).then(|| format!("minimum lowered {old} -> {new}"))
        }
        (FieldRule::Budget, Some(old), Some(new)) => (new
            > old.saturating_mul(BUDGET_GROWTH_LIMIT))
        .then(|| format!("budget raised {old} -> {new} (more than {BUDGET_GROWTH_LIMIT}x)")),
        (FieldRule::Requirement, _, _) => (base.as_bool() == Some(true)
            && head.as_bool() != Some(true))
        .then(|| format!("requirement relaxed {base} -> {head}")),
        _ => (base != head).then(|| format!("changed {base} -> {head}")),
    }
}

pub fn measure(
    root: &Path,
    authority: &Authority,
    base_sha: &str,
    head_sha: &str,
) -> Result<RatchetReport, String> {
    let base = load_set(root, authority, base_sha)?;
    let head = load_set(root, authority, head_sha)?;
    let weakenings = compare(&base, &head);
    let waivers = if weakenings.is_empty() {
        Vec::new()
    } else {
        commit_waivers(root, authority, base_sha, head_sha, WAIVER_KEY)?
    };
    Ok(RatchetReport::new(weakenings, waivers))
}

fn load_set(root: &Path, authority: &Authority, commit: &str) -> Result<ScenarioSet, String> {
    let names = supervised_git(
        root,
        authority,
        &[
            "ls-tree".into(),
            "--name-only".into(),
            "-z".into(),
            commit.into(),
            SCRIPT_DIR.into(),
        ],
        &format!("list scenario scripts at {commit}"),
        &[0],
    )?;
    let mut set = ScenarioSet::default();
    for name in String::from_utf8_lossy(&names.stdout)
        .split('\0')
        .filter(|name| name.ends_with(".json"))
    {
        set.scripts
            .insert(name.to_string(), git_value(root, authority, commit, name)?);
    }
    let gates = git_value(root, authority, commit, GATES_PATH)?;
    let entries = gates
        .pointer("/native_acceptance/scenario_scripts")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!("{GATES_PATH} at {commit} has no native_acceptance.scenario_scripts")
        })?;
    let listed: BTreeSet<_> = entries
        .iter()
        .filter_map(|entry| entry.get("path")?.as_str())
        .collect();
    for path in listed {
        let timeout = entries
            .iter()
            .find(|entry| entry.get("path").and_then(Value::as_str) == Some(path))
            .and_then(|entry| entry.get("outer_child_timeout_ms")?.as_u64());
        set.gate_entries.insert(path.to_string(), timeout);
    }
    Ok(set)
}

fn git_value(
    root: &Path,
    authority: &Authority,
    commit: &str,
    path: &str,
) -> Result<Value, String> {
    let output = supervised_git(
        root,
        authority,
        &["show".into(), format!("{commit}:{path}")],
        &format!("read {path} at {commit}"),
        &[0],
    )?;
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot parse {path} at {commit}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Assertion-class actions that do not start with `assert_` or `wait_for_`.
    const CHECKING_ACTIONS: &[&str] = &[
        "fight_battle",
        "pick_battle_ship",
        "collect_planet_minerals",
        "select_communication_response",
        "capture",
        "navigate_to_planet",
        "navigate_to_moon",
        "navigate_to_orbit",
        "navigate_to_hyperspace",
        "expect_quit",
    ];

    const SCENARIO: &str = "rust/scripts/a.json";

    fn set(steps: Value, max_ticks_budget: u64) -> ScenarioSet {
        let mut set = ScenarioSet::default();
        set.scripts.insert(
            SCENARIO.into(),
            json!({"version": 1, "name": "a",
                   "budgets": {"max_input_ticks": max_ticks_budget},
                   "steps": steps}),
        );
        set
    }

    fn base_steps() -> Value {
        json!([
            {"action": "tap_menu_key", "key": "select", "hold": 2, "settle": 4, "value": 1},
            {"action": "assert_mode", "mode": "super_melee"},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 30},
            {"action": "capture", "label": "menu", "expect_change": true},
            {"action": "finish"}
        ])
    }

    fn changes(base: &ScenarioSet, head: &ScenarioSet) -> Vec<String> {
        compare(base, head)
            .into_iter()
            .map(|w| format!("{} | {}", w.step, w.change))
            .collect()
    }

    #[test]
    fn identical_sets_do_not_weaken() {
        assert!(compare(&set(base_steps(), 100), &set(base_steps(), 100)).is_empty());
    }

    #[test]
    fn removed_assertion_step_is_reported() {
        let head = json!([
            {"action": "tap_menu_key", "key": "select", "hold": 2, "settle": 4, "value": 1},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 30},
            {"action": "capture", "label": "menu", "expect_change": true},
            {"action": "finish"}
        ]);
        let found = changes(&set(base_steps(), 100), &set(head, 100));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("assert_mode | "), "{found:?}");
        assert!(found[0].contains("removed"));
    }

    #[test]
    fn reordering_adding_and_plain_edits_pass() {
        let head = json!([
            {"action": "capture", "label": "menu", "expect_change": true},
            {"action": "assert_scene", "scene": "extra"},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 31},
            {"action": "assert_mode", "mode": "super_melee"},
            {"action": "tap_menu_key", "key": "up", "hold": 9, "settle": 9, "value": 1},
            {"action": "wait_input_ticks", "count": 3},
            {"action": "finish"}
        ]);
        assert!(compare(&set(base_steps(), 100), &set(head, 100)).is_empty());
    }

    #[test]
    fn removing_a_plain_step_passes() {
        let head = json!([
            {"action": "assert_mode", "mode": "super_melee"},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 30},
            {"action": "capture", "label": "menu", "expect_change": true}
        ]);
        assert!(compare(&set(base_steps(), 100), &set(head, 100)).is_empty());
    }

    #[test]
    fn lowered_minimum_is_reported() {
        let head = json!([
            {"action": "assert_mode", "mode": "super_melee"},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 10},
            {"action": "capture", "label": "menu", "expect_change": true}
        ]);
        let found = changes(&set(base_steps(), 100), &set(head, 100));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("minimum lowered 30 -> 10"));
    }

    #[test]
    fn changed_exact_value_is_reported() {
        let head = json!([
            {"action": "assert_mode", "mode": "main_menu"},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 30},
            {"action": "capture", "label": "menu", "expect_change": true}
        ]);
        let found = changes(&set(base_steps(), 100), &set(head, 100));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("mode: changed"));
    }

    #[test]
    fn relaxed_expect_change_is_reported() {
        let head = json!([
            {"action": "assert_mode", "mode": "super_melee"},
            {"action": "wait_for_battle_frames", "max_ticks": 1000, "minimum": 30},
            {"action": "capture", "label": "menu", "expect_change": false}
        ]);
        assert_eq!(changes(&set(base_steps(), 100), &set(head, 100)).len(), 1);
    }

    #[test]
    fn step_max_ticks_may_grow_up_to_double_but_not_beyond() {
        let with_ticks =
            |ticks: u64| set(json!([{"action": "fight_battle", "max_ticks": ticks}]), 100);
        let base = with_ticks(1000);
        assert!(compare(&base, &with_ticks(1500)).is_empty());
        assert!(compare(&base, &with_ticks(2000)).is_empty());
        assert_eq!(compare(&base, &with_ticks(3000)).len(), 1);
    }

    #[test]
    fn scenario_budget_may_grow_up_to_double_but_not_beyond() {
        let base = set(base_steps(), 1000);
        assert!(compare(&base, &set(base_steps(), 1500)).is_empty());
        let found = compare(&base, &set(base_steps(), 3000));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].step, "budgets.max_input_ticks");
    }

    #[test]
    fn deleted_scenario_is_reported() {
        let found = compare(&set(base_steps(), 100), &ScenarioSet::default());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].scenario, SCENARIO);
        assert!(found[0].change.contains("removed"));
    }

    #[test]
    fn added_scenario_passes() {
        assert!(compare(&ScenarioSet::default(), &set(base_steps(), 100)).is_empty());
    }

    #[test]
    fn outer_child_timeout_may_not_be_raised_or_dropped_or_unlisted() {
        let entry = |timeout: Option<u64>| {
            let mut set = ScenarioSet::default();
            set.gate_entries.insert(SCENARIO.into(), timeout);
            set
        };
        let base = entry(Some(900_000));
        assert!(compare(&base, &entry(Some(900_000))).is_empty());
        assert!(compare(&base, &entry(Some(600_000))).is_empty());
        assert_eq!(compare(&base, &entry(Some(900_001))).len(), 1);
        assert_eq!(compare(&base, &entry(None)).len(), 1);
        assert_eq!(compare(&base, &ScenarioSet::default()).len(), 1);
        assert!(compare(&entry(None), &entry(Some(1))).is_empty());
    }

    #[test]
    fn every_action_is_classified_and_unknown_actions_are_assertions() {
        for action in CHECKING_ACTIONS {
            assert_eq!(classify_action(action), ActionClass::Assertion, "{action}");
        }
        assert_eq!(classify_action("assert_anything"), ActionClass::Assertion);
        assert_eq!(classify_action("wait_for_anything"), ActionClass::Assertion);
        assert_eq!(classify_action("brand_new_action"), ActionClass::Assertion);
        for action in PLAIN_ACTIONS {
            assert_eq!(classify_action(action), ActionClass::Plain, "{action}");
            assert!(!action.starts_with("assert_") && !action.starts_with("wait_for_"));
        }
    }

    #[test]
    fn a_waiver_lets_weakenings_pass_but_stays_reported() {
        let weak = vec![weakening(SCENARIO, "assert_mode", "removed")];
        let waiver = Waiver {
            commit: "a".repeat(40),
            reason: "why".into(),
        };
        assert!(!RatchetReport::new(weak.clone(), Vec::new()).passed);
        let waived = RatchetReport::new(weak, vec![waiver]);
        assert!(waived.passed);
        assert_eq!(waived.weakenings.len(), 1);
        assert!(RatchetReport::new(Vec::new(), Vec::new()).passed);
    }
}
