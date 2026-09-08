//! Retained-input replay of a recorded native gameplay run.
//!
//! Offline validation of a retained bundle proves that the bundle is
//! internally consistent. It does not start the game, so it cannot say whether
//! the same inputs still produce the same run. This module does the other
//! thing: it reads the exact retained source, executable, content package,
//! script bytes, seed, runtime contract and acceptance policy out of a prior
//! suite, stages them, and launches the real window again from those bytes.
//!
//! Nothing here treats validation as replay. `identity` reads a bundle,
//! `bind` refuses a replay whose inputs differ from the recorded ones by even
//! one field, and `replay` executes the scenarios through the same suite
//! launcher the ordinary `native-test` route uses. The comparison afterwards
//! is over inputs and satisfied obligations only; frame counts, pixels and
//! wall-clock timing are not compared and the output says so.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ci::authority::{Authority, PinnedScript};
use crate::ci::evidence::EvidenceSnapshot;
use crate::native_suite;
use uqm_rust::automation::native_checkpoint::NativeCheckpointPlan;
use uqm_rust::automation::native_window::{NativeAcceptancePolicy, NativeWindowRuntimeContract};
use uqm_rust::automation::{
    parse_script, validate_script, NativeAcceptanceManifest, NativeLinkedBuildReceipt,
    NativeRetainedInput, NativeScreenshotStage,
};

pub(crate) const REPLAY_IDENTITY_SCHEMA: &str = "uqm-native-replay-identity-v1";
pub(crate) const REPLAY_OUTCOME_SCHEMA: &str = "uqm-native-replay-outcome-v1";

const LINKED_BUILD_RECEIPT: &str = "inputs/linked-build/linked-build-receipt.json";
const LINKED_BUILD_PREFIX: &str = "inputs/linked-build/";

/// What a replay must reproduce exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplayIdentity {
    pub schema: String,
    pub selection: Vec<PinnedScript>,
    pub executable: NativeRetainedInput,
    pub content_package: NativeRetainedInput,
    pub content_version: NativeRetainedInput,
    pub linked_source_sha: String,
    pub linked_build_receipt: NativeRetainedInput,
    pub runtime_contract: NativeWindowRuntimeContract,
    pub acceptance_policy: NativeAcceptancePolicy,
    pub scenarios: Vec<ScenarioIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScenarioIdentity {
    pub script: PinnedScript,
    pub scenario_name: String,
    pub seed: u32,
    pub source_sha256: String,
    pub resolved_sha256: String,
    pub checkpoints: Vec<String>,
    /// The fresh empty profile the run started from.
    pub initial_config: NativeRetainedInput,
}

/// What a replay is allowed to be judged on.
///
/// Frame counts, image bytes and elapsed time belong to one execution of a
/// real window and are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplayOutcome {
    pub schema: String,
    pub scenarios: Vec<ScenarioOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ScenarioOutcome {
    pub script: String,
    pub window_passed: bool,
    pub satisfied_checkpoints: Vec<usize>,
    pub stages: Vec<String>,
    pub child_exit_code: Option<i32>,
    pub child_signal: Option<i32>,
}

pub(crate) struct ReplayRequest<'a> {
    pub prior: &'a Path,
    pub output: &'a Path,
    /// The scenario selection the operator asked for, if any. A replay proves
    /// the recorded run, so a selection that is not the recorded one is
    /// refused rather than silently narrowed.
    pub scenarios: Option<&'a str>,
    /// The seed the operator expects. Never applied: scripts carry their own
    /// seed and a replay that rewrote it would not be a replay.
    pub seed: Option<u32>,
}

/// Re-execute a recorded native run from its retained inputs.
pub(crate) fn replay(root: &Path, request: &ReplayRequest<'_>) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err(
            "native gameplay replay drives a real macOS window; this host cannot run it".into(),
        );
    }
    let authority = crate::ci::authority::load_authority(root)?;
    crate::ci::authority::validate_authority(&authority)?;
    let (recorded, recorded_outcome) = read_bundle(request.prior, &authority)?;
    check_requested_selection(&recorded, request)?;
    let members = shared_members(request.prior)?;
    let staged = stage(&members, &recorded, &authority)?;

    let selection = recorded.selection.clone();
    let requested = selection
        .iter()
        .map(|pinned| pinned.path.clone())
        .collect::<Vec<_>>()
        .join(",");
    let mut suite = native_suite::SuiteAccounting::create_with_limits(
        request.output,
        &selection,
        false,
        authority.native_runtime_contract().inventory_limits,
    )?;
    println!(
        "retained-input native gameplay replay: launching {} scenario(s) recorded in {}",
        selection.len(),
        request.prior.display()
    );
    let result = crate::execute_native_scenarios(
        &crate::NativeScenarioLaunch {
            source: &staged.source(),
            controller: &staged.controller,
            linked_executable: &staged.executable(),
            content_root: &staged.content(),
            evidence_root: request.output,
            linked_build_proof: &staged.linked,
            requested: Some(&requested),
        },
        &selection,
        &authority,
        &mut suite,
    );
    native_suite::conclude(&mut suite, result, native_suite::SuitePhase::Execute)?;

    let (produced, produced_outcome) = read_bundle(request.output, &authority)?;
    bind(&recorded, &produced)?;
    compare_outcome(&recorded_outcome, &produced_outcome)?;
    println!("replayed\t{}", request.prior.display());
    println!("evidence\t{}", request.output.display());
    println!("input_identity\tmatched retained source, executable, content, scripts and seeds");
    println!(
        "semantic_outcome\tmatched satisfied checkpoint obligations \
         (frame counts, pixels and timing are not compared)"
    );
    Ok(())
}

fn check_requested_selection(
    recorded: &ReplayIdentity,
    request: &ReplayRequest<'_>,
) -> Result<(), String> {
    if let Some(seed) = request.seed {
        let differing: Vec<&str> = recorded
            .scenarios
            .iter()
            .filter(|scenario| scenario.seed != seed)
            .map(|scenario| scenario.script.path.as_str())
            .collect();
        if !differing.is_empty() {
            return Err(format!(
                "--seed {seed} does not match the recorded seed of {}; a replay reproduces the \
                 script's own seed and never rewrites it",
                differing.join(", ")
            ));
        }
    }
    let Some(requested) = request.scenarios else {
        return Ok(());
    };
    let requested: Vec<String> = requested
        .split([',', '\n', ' '])
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            if name.contains('/') {
                name.to_string()
            } else {
                format!("rust/scripts/{name}.json")
            }
        })
        .collect();
    let recorded_paths: Vec<String> = recorded
        .selection
        .iter()
        .map(|pinned| pinned.path.clone())
        .collect();
    if requested != recorded_paths {
        return Err(format!(
            "requested selection {} is not the recorded selection {}; a replay reproduces the \
             recorded run, so narrowing or reordering it is refused",
            requested.join(" "),
            recorded_paths.join(" ")
        ));
    }
    Ok(())
}

/// Read a retained suite's replayable input identity and comparable outcome.
pub(crate) fn read_bundle(
    bundle: &Path,
    authority: &Authority,
) -> Result<(ReplayIdentity, ReplayOutcome), String> {
    let snapshot = EvidenceSnapshot::open(bundle)
        .map_err(|error| format!("open native suite {}: {error}", bundle.display()))?;
    let selection: Vec<PinnedScript> = serde_json::from_slice(
        snapshot
            .read("suite-request.json")
            .map_err(|error| format!("read suite-request.json: {error}"))?,
    )
    .map_err(|error| format!("parse suite-request.json: {error}"))?;
    for pinned in &selection {
        if !authority
            .native_acceptance
            .scenario_scripts
            .contains(pinned)
        {
            return Err(format!(
                "retained scenario {} is not pinned by the authority, so it cannot be replayed",
                pinned.path
            ));
        }
    }
    let manifest = native_suite::validate_accounting(bundle, &selection)?;
    if !manifest.passed {
        return Err(format!(
            "{} did not complete its selection; a replay needs a recorded run whose scenarios all \
             completed",
            bundle.display()
        ));
    }
    let members = shared_members(bundle)?;
    let package = format!(
        "inputs/content/packages/{}",
        authority.native_acceptance.content_filename
    );
    let receipt: NativeLinkedBuildReceipt =
        serde_json::from_slice(member(&members, LINKED_BUILD_RECEIPT)?)
            .map_err(|error| format!("parse retained linked-build receipt: {error}"))?;
    let mut scenarios = Vec::with_capacity(manifest.scenarios.len());
    let mut outcomes = Vec::with_capacity(manifest.scenarios.len());
    let mut contracts: Option<(NativeWindowRuntimeContract, NativeAcceptancePolicy)> = None;
    for row in &manifest.scenarios {
        let proof: NativeAcceptanceManifest = serde_json::from_slice(
            snapshot
                .read(&format!("{}/native-acceptance.json", row.directory))
                .map_err(|error| format!("read {} proof: {error}", row.directory))?,
        )
        .map_err(|error| format!("parse {} proof: {error}", row.directory))?;
        if proof.runtime_contract != authority.native_runtime_contract()
            || proof.acceptance_policy != authority.native_acceptance.acceptance_policy
        {
            return Err(format!(
                "retained scenario {} used a runtime contract or acceptance policy the authority \
                 does not declare",
                row.script.path
            ));
        }
        if let Some(established) = &contracts {
            if *established != (proof.runtime_contract, proof.acceptance_policy) {
                return Err("retained scenarios disagree on runtime contract or policy".into());
            }
        }
        contracts = Some((proof.runtime_contract, proof.acceptance_policy));
        scenarios.push(scenario_identity(&snapshot, &members, row)?);
        outcomes.push(scenario_outcome(&row.script.path, &proof));
    }
    let (runtime_contract, acceptance_policy) =
        contracts.ok_or("retained suite has no scenarios to replay")?;
    Ok((
        ReplayIdentity {
            schema: REPLAY_IDENTITY_SCHEMA.to_string(),
            selection,
            executable: identity(&members, "inputs/uqm")?,
            content_package: identity(&members, &package)?,
            content_version: identity(&members, "inputs/content/version")?,
            linked_source_sha: receipt.source_sha,
            linked_build_receipt: identity(&members, LINKED_BUILD_RECEIPT)?,
            runtime_contract,
            acceptance_policy,
            scenarios,
        },
        ReplayOutcome {
            schema: REPLAY_OUTCOME_SCHEMA.to_string(),
            scenarios: outcomes,
        },
    ))
}

fn scenario_identity(
    snapshot: &EvidenceSnapshot,
    members: &BTreeMap<String, Vec<u8>>,
    row: &native_suite::ScenarioAccounting,
) -> Result<ScenarioIdentity, String> {
    let filename = script_filename(&row.script)?;
    let bytes = member(members, &format!("inputs/{filename}"))?;
    if bytes.len() as u64 != row.script.byte_length || crate::hex_sha256(bytes) != row.script.sha256
    {
        return Err(format!(
            "retained script for {} differs from the pin the suite recorded",
            row.script.path
        ));
    }
    let path = Path::new(&row.script.path);
    let document = parse_script(bytes, path).map_err(|error| error.to_string())?;
    let script = validate_script(document, path).map_err(|error| error.to_string())?;
    let plan = NativeCheckpointPlan::derive(bytes, &script)?;
    let initial = format!("{}/config-initial.json", row.directory);
    let initial_bytes = snapshot
        .read(&initial)
        .map_err(|error| format!("read {initial}: {error}"))?;
    let initial_config: serde_json::Value = serde_json::from_slice(initial_bytes)
        .map_err(|error| format!("parse {initial}: {error}"))?;
    if initial_config["schema"] != "uqm-native-initial-config-v1"
        || initial_config["files"] != serde_json::json!([])
    {
        return Err(format!(
            "{initial} is not the fresh empty profile a replay can reproduce"
        ));
    }
    Ok(ScenarioIdentity {
        script: row.script.clone(),
        scenario_name: script.name().to_string(),
        seed: script.seed(),
        source_sha256: plan.source_sha256,
        resolved_sha256: plan.resolved_sha256,
        checkpoints: plan
            .checkpoints
            .into_iter()
            .map(|checkpoint| checkpoint.id)
            .collect(),
        initial_config: crate::retained_identity("config-initial.json", initial_bytes),
    })
}

fn scenario_outcome(script: &str, proof: &NativeAcceptanceManifest) -> ScenarioOutcome {
    let mut satisfied: Vec<usize> = proof
        .window
        .screenshots
        .iter()
        .filter_map(|shot| match shot.stage {
            NativeScreenshotStage::Checkpoint { action_index } => Some(action_index),
            _ => None,
        })
        .collect();
    satisfied.sort_unstable();
    let mut stages: Vec<String> = proof
        .window
        .screenshots
        .iter()
        .filter_map(|shot| match shot.stage {
            NativeScreenshotStage::Stable => Some("stable".to_string()),
            NativeScreenshotStage::Playable => Some("playable".to_string()),
            NativeScreenshotStage::Checkpoint { .. } => None,
        })
        .collect();
    stages.sort();
    ScenarioOutcome {
        script: script.to_string(),
        window_passed: proof.window.passed,
        satisfied_checkpoints: satisfied,
        stages,
        child_exit_code: proof.child.exit_code,
        child_signal: proof.child.signal,
    }
}

/// Refuse a replay whose inputs are not the recorded ones, naming the field.
pub(crate) fn bind(recorded: &ReplayIdentity, produced: &ReplayIdentity) -> Result<(), String> {
    let differences = [
        difference("selection", &recorded.selection, &produced.selection),
        difference("executable", &recorded.executable, &produced.executable),
        difference(
            "content package",
            &recorded.content_package,
            &produced.content_package,
        ),
        difference(
            "content version",
            &recorded.content_version,
            &produced.content_version,
        ),
        difference(
            "linked build source",
            &recorded.linked_source_sha,
            &produced.linked_source_sha,
        ),
        difference(
            "linked build receipt",
            &recorded.linked_build_receipt,
            &produced.linked_build_receipt,
        ),
        difference(
            "runtime contract",
            &recorded.runtime_contract,
            &produced.runtime_contract,
        ),
        difference(
            "acceptance policy",
            &recorded.acceptance_policy,
            &produced.acceptance_policy,
        ),
    ];
    let mut failures: Vec<String> = differences.into_iter().flatten().collect();
    failures.extend(scenario_differences(recorded, produced));
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "replay inputs differ from the recorded run: {}",
            failures.join("; ")
        ))
    }
}

fn scenario_differences(recorded: &ReplayIdentity, produced: &ReplayIdentity) -> Vec<String> {
    if recorded.scenarios.len() != produced.scenarios.len() {
        return vec![format!(
            "scenario count {} became {}",
            recorded.scenarios.len(),
            produced.scenarios.len()
        )];
    }
    recorded
        .scenarios
        .iter()
        .zip(&produced.scenarios)
        .filter(|(before, after)| before != after)
        .map(|(before, after)| {
            format!(
                "scenario {} identity changed (seed {} -> {}, resolved {} -> {})",
                before.script.path,
                before.seed,
                after.seed,
                before.resolved_sha256,
                after.resolved_sha256
            )
        })
        .collect()
}

fn difference<T: PartialEq + std::fmt::Debug>(
    field: &str,
    recorded: &T,
    produced: &T,
) -> Option<String> {
    (recorded != produced).then(|| format!("{field} {recorded:?} became {produced:?}"))
}

fn compare_outcome(recorded: &ReplayOutcome, produced: &ReplayOutcome) -> Result<(), String> {
    if recorded == produced {
        return Ok(());
    }
    Err(format!(
        "replay inputs matched but the semantic outcome differs: recorded {}; replayed {}",
        serde_json::to_string(recorded).unwrap_or_else(|error| error.to_string()),
        serde_json::to_string(produced).unwrap_or_else(|error| error.to_string())
    ))
}

fn shared_members(bundle: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let snapshot = EvidenceSnapshot::open(bundle)
        .map_err(|error| format!("open native suite {}: {error}", bundle.display()))?;
    let members = native_suite::shared_members(&snapshot)?;
    if members.is_empty() {
        return Err(format!(
            "{} has no retained shared inputs, so there is nothing to replay",
            bundle.display()
        ));
    }
    Ok(members)
}

fn member<'a>(members: &'a BTreeMap<String, Vec<u8>>, logical: &str) -> Result<&'a [u8], String> {
    members
        .get(logical)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("retained suite has no shared member {logical}"))
}

fn identity(
    members: &BTreeMap<String, Vec<u8>>,
    logical: &str,
) -> Result<NativeRetainedInput, String> {
    Ok(crate::retained_identity(logical, member(members, logical)?))
}

fn script_filename(pinned: &PinnedScript) -> Result<String, String> {
    Path::new(&pinned.path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| format!("pinned script {} has no filename", pinned.path))
}

/// The retained inputs, written back out where the suite launcher expects them.
///
/// The staging root owns the lifetime of everything the child reads, so it is
/// held here and every path is derived from it.
struct Staged {
    directory: tempfile::TempDir,
    controller: PathBuf,
    linked: crate::LinkedBuildProof,
}

impl Staged {
    /// A source root whose `rust/scripts/` holds the retained script bytes,
    /// so the existing admission still checks them against the authority pin.
    fn source(&self) -> PathBuf {
        self.directory.path().join("source")
    }

    fn content(&self) -> PathBuf {
        self.directory.path().join("content")
    }

    fn executable(&self) -> PathBuf {
        self.directory.path().join("uqm")
    }
}

fn stage(
    members: &BTreeMap<String, Vec<u8>>,
    recorded: &ReplayIdentity,
    authority: &Authority,
) -> Result<Staged, String> {
    let directory =
        tempfile::tempdir().map_err(|error| format!("claim replay staging root: {error}"))?;
    let linked =
        tempfile::tempdir().map_err(|error| format!("claim replay linked-build root: {error}"))?;
    crate::ci::exec::permit_containment_directory(linked.path())
        .map_err(|error| format!("permit replay linked-build root: {error}"))?;
    let controller =
        std::env::current_exe().map_err(|error| format!("resolve replay controller: {error}"))?;
    let staged = Staged {
        directory,
        controller,
        linked: crate::LinkedBuildProof { directory: linked },
    };
    std::fs::create_dir_all(staged.source().join("rust/scripts"))
        .map_err(|error| format!("claim replay script root: {error}"))?;
    std::fs::create_dir(staged.content())
        .map_err(|error| format!("claim replay content root: {error}"))?;
    for pinned in &recorded.selection {
        let filename = script_filename(pinned)?;
        write_new(
            &staged.source().join(&pinned.path),
            member(members, &format!("inputs/{filename}"))?,
            0o600,
        )?;
    }
    let package = &authority.native_acceptance.content_filename;
    write_new(
        &staged.content().join(package),
        member(members, &format!("inputs/content/packages/{package}"))?,
        0o600,
    )?;
    write_new(&staged.executable(), member(members, "inputs/uqm")?, 0o700)?;
    let mut staged_members = 0;
    for (logical, bytes) in members {
        let Some(name) = logical.strip_prefix(LINKED_BUILD_PREFIX) else {
            continue;
        };
        write_new(&staged.linked.directory.path().join(name), bytes, 0o640)?;
        staged_members += 1;
    }
    if staged_members == 0 {
        return Err("retained suite has no linked-build members to replay".into());
    }
    Ok(staged)
}

fn write_new(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("claim {}: {error}", parent.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("stage {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("write {}: {error}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        file.set_permissions(std::fs::Permissions::from_mode(mode))
            .map_err(|error| format!("seal {}: {error}", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    file.sync_all()
        .map_err(|error| format!("sync {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pinned(path: &str) -> PinnedScript {
        PinnedScript {
            path: path.into(),
            sha256: "a".repeat(64),
            byte_length: 4,
        }
    }

    fn scenario(seed: u32) -> ScenarioIdentity {
        ScenarioIdentity {
            script: pinned("rust/scripts/main-menu-v1.json"),
            scenario_name: "main-menu-v1".into(),
            seed,
            source_sha256: "b".repeat(64),
            resolved_sha256: "c".repeat(64),
            checkpoints: vec!["native:ccc:0".into()],
            initial_config: crate::retained_identity("config-initial.json", b"{}"),
        }
    }

    fn identity_fixture() -> ReplayIdentity {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../ci/gates.json")).unwrap();
        ReplayIdentity {
            schema: REPLAY_IDENTITY_SCHEMA.into(),
            selection: vec![pinned("rust/scripts/main-menu-v1.json")],
            executable: crate::retained_identity("inputs/uqm", b"executable"),
            content_package: crate::retained_identity("inputs/content/packages/x", b"content"),
            content_version: crate::retained_identity("inputs/content/version", b"0.8.0\n"),
            linked_source_sha: "d".repeat(40),
            linked_build_receipt: crate::retained_identity(LINKED_BUILD_RECEIPT, b"{}"),
            runtime_contract: authority.native_runtime_contract(),
            acceptance_policy: authority.native_acceptance.acceptance_policy,
            scenarios: vec![scenario(7)],
        }
    }

    #[test]
    fn matching_identities_bind() {
        assert!(bind(&identity_fixture(), &identity_fixture()).is_ok());
    }

    #[test]
    fn a_substituted_executable_is_refused_by_name() {
        let recorded = identity_fixture();
        let mut produced = identity_fixture();
        produced.executable = crate::retained_identity("inputs/uqm", b"a different binary");

        let error = bind(&recorded, &produced).unwrap_err();

        assert!(error.contains("executable"), "{error}");
        assert!(error.contains("became"), "{error}");
    }

    #[test]
    fn a_changed_seed_or_resolved_scenario_is_refused() {
        let mutations: [fn(&mut ReplayIdentity); 3] = [
            |identity| identity.scenarios[0].seed = 8,
            |identity| identity.scenarios[0].resolved_sha256 = "e".repeat(64),
            |identity| identity.scenarios[0].checkpoints.clear(),
        ];
        for mutate in mutations {
            let mut produced = identity_fixture();
            mutate(&mut produced);
            let error = bind(&identity_fixture(), &produced).unwrap_err();
            assert!(error.contains("identity changed"), "{error}");
        }
    }

    /// A named field and the mutation that changes it.
    type FieldMutation = (&'static str, fn(&mut ReplayIdentity));

    #[test]
    fn changed_content_source_or_policy_is_refused() {
        let cases: [FieldMutation; 4] = [
            ("content package", |identity| {
                identity.content_package =
                    crate::retained_identity("inputs/content/packages/x", b"other");
            }),
            ("content version", |identity| {
                identity.content_version =
                    crate::retained_identity("inputs/content/version", b"0.7.0\n");
            }),
            ("linked build source", |identity| {
                identity.linked_source_sha = "f".repeat(40);
            }),
            ("acceptance policy", |identity| {
                identity.acceptance_policy.battle_frame_floor += 1;
            }),
        ];
        for (field, mutate) in cases {
            let mut produced = identity_fixture();
            mutate(&mut produced);
            let error = bind(&identity_fixture(), &produced).unwrap_err();
            assert!(error.contains(field), "{field} was not named: {error}");
        }
    }

    #[test]
    fn every_differing_field_is_reported_not_only_the_first() {
        let mut produced = identity_fixture();
        produced.executable = crate::retained_identity("inputs/uqm", b"other");
        produced.linked_source_sha = "0".repeat(40);

        let error = bind(&identity_fixture(), &produced).unwrap_err();

        assert!(error.contains("executable"), "{error}");
        assert!(error.contains("linked build source"), "{error}");
    }

    #[test]
    fn a_selection_that_is_not_the_recorded_one_is_refused() {
        let recorded = identity_fixture();
        let refused = check_requested_selection(
            &recorded,
            &ReplayRequest {
                prior: Path::new("/prior"),
                output: Path::new("/output"),
                scenarios: Some("battle-v1"),
                seed: None,
            },
        )
        .unwrap_err();
        assert!(
            refused.contains("is not the recorded selection"),
            "{refused}"
        );

        assert!(check_requested_selection(
            &recorded,
            &ReplayRequest {
                prior: Path::new("/prior"),
                output: Path::new("/output"),
                scenarios: Some("main-menu-v1"),
                seed: None,
            },
        )
        .is_ok());
    }

    #[test]
    fn a_seed_the_script_does_not_declare_is_refused_rather_than_applied() {
        let recorded = identity_fixture();
        let error = check_requested_selection(
            &recorded,
            &ReplayRequest {
                prior: Path::new("/prior"),
                output: Path::new("/output"),
                scenarios: None,
                seed: Some(8),
            },
        )
        .unwrap_err();

        assert!(error.contains("never rewrites it"), "{error}");
        assert!(check_requested_selection(
            &recorded,
            &ReplayRequest {
                prior: Path::new("/prior"),
                output: Path::new("/output"),
                scenarios: None,
                seed: Some(7),
            },
        )
        .is_ok());
    }

    #[test]
    fn a_differing_semantic_outcome_is_reported_separately_from_inputs() {
        let recorded = ReplayOutcome {
            schema: REPLAY_OUTCOME_SCHEMA.into(),
            scenarios: vec![ScenarioOutcome {
                script: "rust/scripts/main-menu-v1.json".into(),
                window_passed: true,
                satisfied_checkpoints: vec![0, 1],
                stages: vec!["stable".into()],
                child_exit_code: Some(0),
                child_signal: None,
            }],
        };
        let mut produced = recorded.clone();
        produced.scenarios[0].satisfied_checkpoints = vec![0];

        assert!(compare_outcome(&recorded, &recorded).is_ok());
        let error = compare_outcome(&recorded, &produced).unwrap_err();
        assert!(error.contains("semantic outcome differs"), "{error}");
        assert!(
            error.contains("pixels") || error.contains("inputs matched"),
            "the message must not read as an input mismatch: {error}"
        );
    }

    #[test]
    fn staging_refuses_a_bundle_without_linked_build_members() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../ci/gates.json")).unwrap();
        let mut recorded = identity_fixture();
        recorded.selection = vec![pinned("rust/scripts/main-menu-v1.json")];
        let members = BTreeMap::from([
            ("inputs/uqm".to_string(), b"executable".to_vec()),
            ("inputs/main-menu-v1.json".to_string(), b"{}".to_vec()),
            (
                format!(
                    "inputs/content/packages/{}",
                    authority.native_acceptance.content_filename
                ),
                b"content".to_vec(),
            ),
        ]);

        let Err(error) = stage(&members, &recorded, &authority) else {
            panic!("a bundle with no linked build cannot be staged for replay");
        };

        assert!(error.contains("linked-build members"), "{error}");
    }

    #[test]
    fn staging_writes_the_retained_inputs_where_the_launcher_expects_them() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../ci/gates.json")).unwrap();
        let recorded = identity_fixture();
        let members = BTreeMap::from([
            ("inputs/uqm".to_string(), b"executable".to_vec()),
            ("inputs/main-menu-v1.json".to_string(), b"{}".to_vec()),
            (
                format!(
                    "inputs/content/packages/{}",
                    authority.native_acceptance.content_filename
                ),
                b"content".to_vec(),
            ),
            (
                LINKED_BUILD_RECEIPT.to_string(),
                b"{\"schema\":\"x\"}".to_vec(),
            ),
        ]);

        let staged = stage(&members, &recorded, &authority).unwrap();

        assert_eq!(
            std::fs::read(staged.source().join("rust/scripts/main-menu-v1.json")).unwrap(),
            b"{}"
        );
        assert_eq!(std::fs::read(staged.executable()).unwrap(), b"executable");
        assert_eq!(
            std::fs::read(
                staged
                    .content()
                    .join(&authority.native_acceptance.content_filename)
            )
            .unwrap(),
            b"content"
        );
        assert!(staged
            .linked
            .directory
            .path()
            .join("linked-build-receipt.json")
            .is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                std::fs::metadata(staged.executable())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700,
                "the retained binary must be launchable"
            );
        }
    }

    #[test]
    fn a_missing_shared_member_names_what_the_replay_cannot_reproduce() {
        let members = BTreeMap::from([("inputs/uqm".to_string(), b"executable".to_vec())]);
        let error = identity(&members, "inputs/content/version").unwrap_err();
        assert!(error.contains("inputs/content/version"), "{error}");
    }
}
