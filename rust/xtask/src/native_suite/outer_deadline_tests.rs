//! Regressions for the user-approved long outer child deadline.
//!
//! Approval, verbatim: "yeah you can switch them to 15m". It covers pinned
//! scripts whose declared wall budget is longer than the 300 s default. A
//! wall budget is a failure ceiling, so a 900 s budget is covered as well.

use super::*;
use crate::ci::authority::load_authority;
use uqm_rust::automation::native_window::NativeWindowRuntimeContract;

/// Pinned scripts whose declared wall budget is above 300 s.
pub(crate) const APPROVED_LONG_SCRIPTS: [&str; 16] = [
    "rust/scripts/explore-planet-v1.json",
    "rust/scripts/probe-encounter-v2.json",
    "rust/scripts/probe-encounter-v3.json",
    "rust/scripts/probe-encounter-v4.json",
    "rust/scripts/probe-encounter-v5.json",
    "rust/scripts/probe-encounter-v6.json",
    "rust/scripts/probe-encounter-v7.json",
    "rust/scripts/real-sol-commander-completion.json",
    "rust/scripts/real-sol-earth-moon-base.json",
    "rust/scripts/real-sol-luna-moonbase-departure.json",
    "rust/scripts/real-sol-planetside-collision-proof.json",
    "rust/scripts/real-sol-probe-to-commander-responses.json",
    "rust/scripts/real-sol-probe-to-starbase.json",
    "rust/scripts/real-sol-rust-planetside-smoke.json",
    "rust/scripts/real-sol-starbase-to-mars.json",
    "rust/scripts/starbase-visit-v1.json",
];

/// Pinned scripts that declare a 900 s wall budget.
const WALL_900_SCRIPTS: [&str; 3] = [
    "rust/scripts/real-sol-earth-moon-base.json",
    "rust/scripts/real-sol-luna-moonbase-departure.json",
    "rust/scripts/real-sol-starbase-to-mars.json",
];

const DEFAULT_OUTER_MS: u64 = 300_000;
const LONG_OUTER_MS: u64 = 900_000;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn authority() -> Authority {
    load_authority(repository_root()).unwrap()
}

fn pin<'a>(authority: &'a Authority, path: &str) -> &'a PinnedScript {
    authority
        .native_acceptance
        .scenario_scripts
        .iter()
        .find(|entry| entry.path == path)
        .unwrap()
}

fn as_json(contract: NativeWindowRuntimeContract) -> serde_json::Value {
    serde_json::to_value(contract).unwrap()
}

#[test]
fn only_the_approved_long_scripts_carry_the_override() {
    let authority = authority();
    let mut carrying: Vec<&str> = authority
        .native_acceptance
        .scenario_scripts
        .iter()
        .filter(|entry| entry.outer_child_timeout_ms.is_some())
        .map(|entry| entry.path.as_str())
        .collect();
    carrying.sort_unstable();
    let mut approved = APPROVED_LONG_SCRIPTS.to_vec();
    approved.sort_unstable();
    assert_eq!(carrying, approved);
    for path in approved {
        assert_eq!(
            pin(&authority, path).outer_child_timeout_ms,
            Some(LONG_OUTER_MS)
        );
    }
}

#[test]
fn every_pin_runs_under_the_deadline_its_wall_budget_class_names() {
    let root = repository_root();
    let authority = authority();
    let admitted = preflight(
        root,
        &authority.native_acceptance.scenario_scripts,
        &authority,
    )
    .unwrap();
    let mut longer = Vec::new();
    for admission in &admitted {
        let path = admission.script.path.as_str();
        let wall = admission.budgets.max_wallclock_seconds;
        if wall > 300 {
            longer.push(path);
        }
        let expected = if wall > 300 {
            LONG_OUTER_MS
        } else {
            DEFAULT_OUTER_MS
        };
        assert_eq!(admission.outer_child_timeout_ms, expected, "{path}");
        assert_eq!(
            APPROVED_LONG_SCRIPTS.contains(&path),
            expected == LONG_OUTER_MS,
            "{path}"
        );
    }
    let mut expected_longer = APPROVED_LONG_SCRIPTS.to_vec();
    expected_longer.sort_unstable();
    longer.sort_unstable();
    assert_eq!(longer, expected_longer);
    assert_eq!(longer.len(), 16);
    for path in WALL_900_SCRIPTS {
        let admission = admitted.iter().find(|a| a.script.path == path).unwrap();
        assert_eq!(admission.budgets.max_wallclock_seconds, 900);
        assert_eq!(admission.outer_child_timeout_ms, LONG_OUTER_MS);
    }
}

#[test]
fn a_pin_changes_only_the_outer_deadline_and_keeps_the_ordering() {
    let authority = authority();
    let base = authority.native_runtime_contract();
    assert_eq!(base.outer_child_timeout_ms, DEFAULT_OUTER_MS);
    for entry in &authority.native_acceptance.scenario_scripts {
        let contract = authority.native_runtime_contract_for(entry).unwrap();
        let mut expected = as_json(base);
        let long = entry.outer_child_timeout_ms.is_some();
        if long {
            expected["outer_child_timeout_ms"] = LONG_OUTER_MS.into();
        }
        assert_eq!(as_json(contract), expected, "{}", entry.path);
        assert!(contract.has_valid_deadline_order(), "{}", entry.path);
        assert_eq!(contract.acknowledgement_timeout_ms, 95_000);
        assert_eq!(contract.outer_child_kill_grace_ms, 5_000);
        assert!(contract.acknowledgement_timeout() < contract.outer_child_timeout());
    }
}

#[test]
fn a_pin_the_authority_does_not_declare_cannot_choose_a_deadline() {
    let authority = authority();
    let mut missing = pin(&authority, APPROVED_LONG_SCRIPTS[1]).clone();
    missing.outer_child_timeout_ms = None;
    assert!(authority.native_runtime_contract_for(&missing).is_err());

    let mut invented = pin(&authority, "rust/scripts/main-menu-v1.json").clone();
    invented.outer_child_timeout_ms = Some(LONG_OUTER_MS);
    assert!(authority.native_runtime_contract_for(&invented).is_err());

    let mut raised = pin(&authority, APPROVED_LONG_SCRIPTS[1]).clone();
    raised.outer_child_timeout_ms = Some(LONG_OUTER_MS * 2);
    assert!(authority.native_runtime_contract_for(&raised).is_err());
}

/// A pin for a copy of `source_path` whose wall budget is `wall_seconds`, with
/// the digest and length recomputed from the edited bytes. The hash is never
/// the reason a fixture is refused.
struct RehashedScript {
    root: tempfile::TempDir,
    pin: PinnedScript,
}

fn rehashed_script(
    source_path: &str,
    wall_seconds: u64,
    override_ms: Option<u64>,
) -> RehashedScript {
    let text = std::fs::read_to_string(repository_root().join(source_path)).unwrap();
    let marker = "\"max_wallclock_seconds\": ";
    let start = text.find(marker).unwrap() + marker.len();
    let end = start + text[start..].find(|c: char| !c.is_ascii_digit()).unwrap();
    let edited = format!("{}{wall_seconds}{}", &text[..start], &text[end..]);
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(source_path);
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, edited.as_bytes()).unwrap();
    let pin = PinnedScript {
        path: source_path.to_string(),
        sha256: crate::hex_sha256(edited.as_bytes()),
        byte_length: edited.len() as u64,
        outer_child_timeout_ms: override_ms,
    };
    RehashedScript { root, pin }
}

fn admit_rehashed(fixture: &RehashedScript) -> Result<ScriptAdmission, String> {
    let mut authority = authority();
    let slot = authority
        .native_acceptance
        .scenario_scripts
        .iter_mut()
        .find(|entry| entry.path == fixture.pin.path)
        .unwrap();
    *slot = fixture.pin.clone();
    admit_script(fixture.root.path(), &fixture.pin, &authority)
}

#[test]
fn rehashed_scripts_admit_the_pinned_class_at_its_budget_edges() {
    for wall in [899, 900] {
        let long = rehashed_script(
            "rust/scripts/probe-encounter-v3.json",
            wall,
            Some(LONG_OUTER_MS),
        );
        assert_eq!(
            admit_rehashed(&long).unwrap().outer_child_timeout_ms,
            LONG_OUTER_MS
        );
    }

    let promoted = rehashed_script("rust/scripts/main-menu-v1.json", 301, Some(LONG_OUTER_MS));
    assert_eq!(
        admit_rehashed(&promoted).unwrap().outer_child_timeout_ms,
        LONG_OUTER_MS
    );

    let short = rehashed_script("rust/scripts/main-menu-v1.json", 300, None);
    assert_eq!(
        admit_rehashed(&short).unwrap().outer_child_timeout_ms,
        DEFAULT_OUTER_MS
    );
}

#[test]
fn rehashed_scripts_with_the_wrong_class_are_refused_for_the_class_not_the_hash() {
    let covered = rehashed_script(
        "rust/scripts/probe-encounter-v3.json",
        200,
        Some(LONG_OUTER_MS),
    );
    let error = admit_rehashed(&covered).unwrap_err();
    assert!(error.contains("fits the 300000 ms default"), "{error}");

    let exactly_default = rehashed_script(
        "rust/scripts/probe-encounter-v3.json",
        300,
        Some(LONG_OUTER_MS),
    );
    let error = admit_rehashed(&exactly_default).unwrap_err();
    assert!(error.contains("fits the 300000 ms default"), "{error}");

    let missing = rehashed_script("rust/scripts/probe-encounter-v3.json", 720, None);
    let error = admit_rehashed(&missing).unwrap_err();
    assert!(error.contains("needs the approved 900000 ms"), "{error}");

    let grown = rehashed_script("rust/scripts/main-menu-v1.json", 420, None);
    let error = admit_rehashed(&grown).unwrap_err();
    assert!(error.contains("needs the approved 900000 ms"), "{error}");
}

#[test]
fn a_900_second_budget_without_the_override_is_refused() {
    let missing = rehashed_script("rust/scripts/probe-encounter-v3.json", 900, None);
    let error = admit_rehashed(&missing).unwrap_err();
    assert!(error.contains("needs the approved 900000 ms"), "{error}");
}

#[test]
fn the_three_900_second_scripts_carry_the_override_in_the_authority() {
    let authority = authority();
    for path in WALL_900_SCRIPTS {
        assert_eq!(
            pin(&authority, path).outer_child_timeout_ms,
            Some(LONG_OUTER_MS)
        );
    }
}

#[test]
fn removing_the_override_from_any_longer_script_is_refused_at_admission() {
    let root = repository_root();
    for path in APPROVED_LONG_SCRIPTS {
        let mut authority = authority();
        let slot = authority
            .native_acceptance
            .scenario_scripts
            .iter_mut()
            .find(|entry| entry.path == path)
            .unwrap();
        slot.outer_child_timeout_ms = None;
        let stripped = slot.clone();
        let error = admit_script(root, &stripped, &authority).unwrap_err();
        assert!(
            error.contains("needs the approved 900000 ms"),
            "{path}: {error}"
        );
    }
}

#[test]
fn the_primary_native_script_keeps_the_default_deadline() {
    let authority = authority();
    let primary = pin(&authority, &authority.native_acceptance.script);
    assert_eq!(primary.outer_child_timeout_ms, None);
    assert_eq!(
        authority
            .native_runtime_contract_for(primary)
            .unwrap()
            .outer_child_timeout_ms,
        DEFAULT_OUTER_MS
    );
}

#[test]
fn child_deadline_validation_rejects_an_override_that_is_not_the_approved_value() {
    for bad in [
        0,
        1,
        299_999,
        300_000,
        899_999,
        900_001,
        1_800_000,
        u64::MAX,
    ] {
        let mut authority = authority();
        let slot = authority
            .native_acceptance
            .scenario_scripts
            .iter_mut()
            .find(|entry| entry.path == APPROVED_LONG_SCRIPTS[1])
            .unwrap();
        slot.outer_child_timeout_ms = Some(bad);
        let error = crate::ci::authority::validate_authority(&authority).unwrap_err();
        assert!(error.contains("outer_child_timeout_ms"), "{bad}: {error}");
    }
}

#[test]
fn child_deadline_validation_rejects_the_primary_script_and_broken_ordering() {
    let mut primary = authority();
    let named = primary.native_acceptance.script.clone();
    primary
        .native_acceptance
        .scenario_scripts
        .iter_mut()
        .find(|entry| entry.path == named)
        .unwrap()
        .outer_child_timeout_ms = Some(LONG_OUTER_MS);
    let error = crate::ci::authority::validate_authority(&primary).unwrap_err();
    assert!(error.contains("single-script gate"), "{error}");

    let mut unordered = authority();
    unordered
        .native_acceptance
        .runtime_contract
        .acknowledgement_timeout_ms = LONG_OUTER_MS;
    assert!(crate::ci::authority::validate_authority(&unordered).is_err());
}

#[test]
fn pin_parsing_rejects_malformed_and_unknown_fields() {
    let template = serde_json::json!({
        "path": "rust/scripts/probe-encounter-v3.json",
        "sha256": "1b40fea5dcc41a7e4c7986fb74a7a0618a7aa4a395dbfeef980d2354ee45e442",
        "byte_length": 1491,
    });
    for value in [
        serde_json::json!(-1),
        serde_json::json!("900000"),
        serde_json::json!(900000.5),
        serde_json::json!(null),
    ] {
        let mut candidate = template.clone();
        candidate["outer_child_timeout_ms"] = value.clone();
        let parsed = serde_json::from_value::<PinnedScript>(candidate);
        if value.is_null() {
            assert_eq!(parsed.unwrap().outer_child_timeout_ms, None);
        } else {
            assert!(parsed.is_err(), "{value}");
        }
    }
    let mut misspelled = template.clone();
    misspelled["outer_child_timeout_msec"] = 900_000.into();
    assert!(serde_json::from_value::<PinnedScript>(misspelled).is_err());

    let absent: PinnedScript = serde_json::from_value(template).unwrap();
    assert_eq!(absent.outer_child_timeout_ms, None);
    assert!(!serde_json::to_string(&absent)
        .unwrap()
        .contains("outer_child_timeout_ms"));
}
