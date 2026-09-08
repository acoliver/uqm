//! Source-only admission for the native autoplay suite.

use crate::ci::authority::{Authority, PinnedScript};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uqm_rust::automation::{parse_script, validate_script, Budgets};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptAdmission {
    pub script: PinnedScript,
    pub budgets: Budgets,
    /// The script's own resolved seed. Recorded, never chosen here.
    pub seed: u32,
    pub action_count: usize,
    pub capture_count: usize,
    pub outer_child_timeout_ms: u64,
    pub outer_child_kill_grace_ms: u64,
}

pub fn admit_script(
    root: &Path,
    pinned: &PinnedScript,
    authority: &Authority,
) -> Result<ScriptAdmission, String> {
    let path = root.join(&pinned.path);
    let bytes = crate::read_regular_file_nofollow_bounded(&path, pinned.byte_length)?;
    if bytes.len() as u64 != pinned.byte_length || crate::hex_sha256(&bytes) != pinned.sha256 {
        return Err(format!(
            "native acceptance script {} differs from machine authority",
            pinned.path
        ));
    }
    let document = parse_script(&bytes, &path).map_err(|error| error.to_string())?;
    let script = validate_script(document, &path).map_err(|error| error.to_string())?;
    let runtime = authority.native_runtime_contract();
    if !runtime.has_valid_deadline_order() {
        return Err("native acceptance runtime has invalid deadline ordering".into());
    }
    // The script watchdog and ChildSession supervisor run concurrently. Neither
    // promises a minimum runtime; expiry of either fails the entire scenario.
    // Observer operations occur inside the child's lifetime, not after it.
    Ok(ScriptAdmission {
        script: pinned.clone(),
        budgets: script.budgets(),
        seed: script.seed(),
        action_count: script.steps().len(),
        capture_count: script
            .steps()
            .iter()
            .filter(|action| matches!(action, uqm_rust::automation::Action::Capture(_)))
            .count(),
        outer_child_timeout_ms: runtime.outer_child_timeout_ms,
        outer_child_kill_grace_ms: runtime.outer_child_kill_grace_ms,
    })
}

/// Inspect the entire requested set before building or spawning any game.
pub fn preflight(
    root: &Path,
    selected: &[PinnedScript],
    authority: &Authority,
) -> Result<Vec<ScriptAdmission>, String> {
    let mut admissions = Vec::new();
    let mut failures = Vec::new();
    for pinned in selected {
        match admit_script(root, pinned, authority) {
            Ok(admission) => admissions.push(admission),
            Err(error) => failures.push(format!("{}: {error}", pinned.path)),
        }
    }
    if selected.is_empty() {
        failures.push("native suite selection is empty".into());
    }
    if !failures.is_empty() {
        return Err(format!(
            "native suite source preflight failed before build/launch:\n{}",
            failures.join("\n")
        ));
    }
    Ok(admissions)
}

/// Accept only the complete event-required suite, with one shared build and policy.
pub fn validate_success(
    root: &Path,
    selected: &[PinnedScript],
    authority: &Authority,
    source_sha: &str,
    policy: &[u8],
) -> Result<(), String> {
    let manifest = validate_accounting(root, selected)?;
    if !manifest.passed {
        return Err("native suite did not complete the required selection".into());
    }
    let snapshot = crate::ci::evidence::EvidenceSnapshot::open(root).map_err(|e| e.to_string())?;
    let mut shared_identity = None;
    for row in &manifest.scenarios {
        let proof: uqm_rust::automation::NativeAcceptanceManifest = serde_json::from_slice(
            snapshot
                .read(&format!("{}/native-acceptance.json", row.directory))
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        validate_success_policy(&proof, &row.script, authority)?;
        let temporary = materialize_member(&snapshot, &row.directory)?;
        let scenario = temporary.path().join(&row.directory);
        let shared =
            uqm_rust::automation::native_artifacts::SharedInputs::open(&scenario, 1024 * 1024)?
                .ok_or("S4 success requires protected suite shared descriptors")?;
        let identity = (shared.descriptor.clone(), proof.executable.clone());
        if shared_identity
            .as_ref()
            .is_some_and(|prior| prior != &identity)
        {
            return Err("native scenarios disagree on shared descriptor or executable".into());
        }
        shared_identity = Some(identity);
        let limit = authority.actions.evidence_snapshot_member_limit_bytes;
        let receipt = shared.read(
            &scenario,
            "inputs/linked-build/linked-build-receipt.json",
            limit,
        )?;
        let nested_policy = shared.read(&scenario, "inputs/linked-build/gates.json", limit)?;
        let contracts = crate::ci::evidence::linked_outer_correlation_contracts(
            Some(&receipt),
            Some(&nested_policy),
            Some(policy),
            source_sha,
        );
        if !contracts.is_empty() {
            return Err(contracts.join(", "));
        }
    }
    Ok(())
}

fn validate_success_policy(
    proof: &uqm_rust::automation::NativeAcceptanceManifest,
    script: &PinnedScript,
    authority: &Authority,
) -> Result<(), String> {
    validate_script_identity(script, &proof.script)?;
    let content = &authority.native_acceptance;
    crate::ci::run::validate_native_runtime_authority(authority, proof.runtime_contract)
        .map_err(|error| error.to_string())?;
    if proof.acceptance_policy != content.acceptance_policy {
        return Err("native suite runtime or acceptance policy differs from authority".into());
    }
    if proof.content_package.relative_path
        != format!("inputs/content/packages/{}", content.content_filename)
        || proof.content_package.byte_length != content.content_byte_length
        || proof.content_package.sha256 != content.content_sha256
    {
        return Err("native suite content package differs from authority".into());
    }
    let version = format!("{}\n", content.content_version);
    if !proof.retained_files.contains(&crate::retained_identity(
        "inputs/content/version",
        version.as_bytes(),
    )) {
        return Err("native suite content version differs from authority".into());
    }
    Ok(())
}

/// Validate a failed suite for diagnostic transport. Its recorded selection must
/// be pinned, but this does not establish that it was the event-required selection.
pub fn validate_failure_diagnostics(root: &Path, authority: &Authority) -> Result<(), String> {
    let snapshot =
        crate::ci::evidence::EvidenceSnapshot::open(root).map_err(|error| error.to_string())?;
    let selected: Vec<PinnedScript> = serde_json::from_slice(
        snapshot
            .read("suite-request.json")
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if selected.iter().any(|script| {
        !authority
            .native_acceptance
            .scenario_scripts
            .contains(script)
    }) {
        return Err("native failure suite contains an unpinned script".into());
    }
    let manifest = snapshot.scoped(|| validate_accounting(root, &selected))?;
    if manifest.passed {
        return Err("a successful native suite cannot be published as failure diagnostics".into());
    }
    for row in &manifest.scenarios {
        let path = format!("{}/native-acceptance-failure.json", row.directory);
        match snapshot.read(&path) {
            Ok(bytes) => validate_member_failure(&snapshot, row, bytes, authority)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MemberFailure {
    Runtime(Box<uqm_rust::automation::NativeAcceptanceFailureManifest>),
    Setup(Box<uqm_rust::automation::NativeAcceptanceSetupFailureManifest>),
}

fn materialize_member(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
    directory: &str,
) -> Result<tempfile::TempDir, String> {
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let publisher = crate::ci::evidence::EvidencePublisher::open(temporary.path())
        .map_err(|error| error.to_string())?;
    let prefix = format!("{directory}/");
    for file in snapshot.files() {
        if file.relative_path.starts_with(&prefix) || file.relative_path.starts_with("shared/") {
            publisher
                .create(&file.relative_path, &file.bytes)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(temporary)
}

fn validate_member_failure(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
    row: &ScenarioAccounting,
    bytes: &[u8],
    authority: &Authority,
) -> Result<(), String> {
    if !matches!(row.state, ScenarioState::Failed { .. }) {
        return Err("native child failure belongs to a nonfailed scenario".into());
    }
    let failure: MemberFailure =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let temporary = materialize_member(snapshot, &row.directory)?;
    let (runtime, policy) = match &failure {
        MemberFailure::Runtime(failure) => (failure.runtime_contract, failure.acceptance_policy),
        MemberFailure::Setup(failure) => (failure.runtime_contract, failure.acceptance_policy),
    };
    if runtime != authority.native_runtime_contract()
        || policy != authority.native_acceptance.acceptance_policy
    {
        return Err(
            "native failure suite runtime or acceptance policy differs from authority".into(),
        );
    }
    match failure {
        MemberFailure::Runtime(failure) => {
            validate_script_identity(&row.script, &failure.script)?;
            let expected = &authority.native_acceptance;
            if failure.content_package.relative_path
                != format!("inputs/content/packages/{}", expected.content_filename)
                || failure.content_package.byte_length != expected.content_byte_length
                || failure.content_package.sha256 != expected.content_sha256
            {
                return Err("native failure suite content differs from authority".into());
            }
            let version = format!("{}\n", expected.content_version);
            if !failure.retained_files.contains(&crate::retained_identity(
                "inputs/content/version",
                version.as_bytes(),
            )) {
                return Err("native failure suite content version differs from authority".into());
            }
            uqm_rust::automation::validate_native_acceptance_failure_bundle(
                &temporary.path().join(&row.directory),
                &failure,
            )
            .map_err(|error| format!("native suite child failure: {error:?}"))
        }
        MemberFailure::Setup(failure) => {
            uqm_rust::automation::validate_native_acceptance_setup_failure_bundle(
                &temporary.path().join(&row.directory),
                &failure,
            )
            .map_err(|error| format!("native suite child setup failure: {error:?}"))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ImageOrigin {
    OsWindowOriginal,
    DerivedClient,
    RendererReadback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GalleryImage {
    scenario: usize,
    origin: ImageOrigin,
    artifact: uqm_rust::automation::NativeRetainedInput,
}

fn gallery_images(
    manifest: &SuiteManifest,
    files: &[uqm_rust::automation::NativeRetainedInput],
) -> Vec<GalleryImage> {
    let mut images = Vec::new();
    for (scenario, row) in manifest.scenarios.iter().enumerate() {
        let native = format!("{}/screenshots/", row.directory);
        let renderer = format!("{}/automation/captures/", row.directory);
        for artifact in files
            .iter()
            .filter(|file| file.relative_path.ends_with(".png"))
        {
            let origin = if artifact.relative_path.starts_with(&native) {
                if artifact.relative_path.ends_with(".os.png") {
                    ImageOrigin::OsWindowOriginal
                } else {
                    ImageOrigin::DerivedClient
                }
            } else if artifact.relative_path.starts_with(&renderer) {
                ImageOrigin::RendererReadback
            } else {
                continue;
            };
            images.push(GalleryImage {
                scenario,
                origin,
                artifact: artifact.clone(),
            });
        }
    }
    images
}

fn gallery_html(manifest: &SuiteManifest, images: &[GalleryImage]) -> Result<Vec<u8>, String> {
    use std::fmt::Write as _;
    let mut html = String::from("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Native autoplay suite</title><h1>Native autoplay suite</h1><p>Original OS images, derived client crops and supplementary renderer readbacks are identified separately. An image in a failed suite is diagnostic evidence, not an accepted checkpoint.</p><ol>");
    for (index, row) in manifest.scenarios.iter().enumerate() {
        let state = match row.state {
            ScenarioState::NotRun => "not run",
            ScenarioState::Attempted => "attempted",
            ScenarioState::Completed { .. } => "completed",
            ScenarioState::Failed { .. } => "failed",
        };
        write!(
            html,
            "<li>{}: {} ({} ms)",
            html_escape(&row.script.path),
            state,
            row.elapsed_ms
        )
        .map_err(|error| error.to_string())?;
        for image in images.iter().filter(|image| image.scenario == index) {
            let path = html_escape(&image.artifact.relative_path);
            write!(html, "<figure><a href=\"{path}\"><img loading=\"lazy\" width=\"640\" src=\"{path}\" alt=\"{path}\"></a><figcaption>{:?}: {} bytes; SHA-256 {}</figcaption></figure>", image.origin, image.artifact.byte_length, image.artifact.sha256).map_err(|error| error.to_string())?;
        }
        html.push_str("</li>");
    }
    html.push_str("</ol></html>\n");
    Ok(html.into_bytes())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn validate_script_identity(
    expected: &PinnedScript,
    actual: &uqm_rust::automation::NativeRetainedInput,
) -> Result<(), String> {
    let filename = Path::new(&expected.path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("script has no filename")?;
    if actual.relative_path != format!("inputs/{filename}")
        || actual.byte_length != expected.byte_length
        || actual.sha256 != expected.sha256
    {
        return Err("native scenario proof used a different selected script".into());
    }
    Ok(())
}

fn validate_journal(events: &[SuiteManifest]) -> Result<(), String> {
    let first = events.first().ok_or("native suite journal is empty")?;
    if first.sequence != 0
        || first.finalized
        || first.passed
        || first.first_failure.is_some()
        || first
            .scenarios
            .iter()
            .any(|row| row.state != ScenarioState::NotRun || row.elapsed_ms != 0)
    {
        return Err("native suite journal must begin with the whole unattempted selection".into());
    }
    for pair in events.windows(2) {
        validate_transition(&pair[0], &pair[1])?;
    }
    Ok(())
}

fn validate_transition(previous: &SuiteManifest, next: &SuiteManifest) -> Result<(), String> {
    if previous.finalized
        || next.sequence != previous.sequence + 1
        || next.elapsed_ms < previous.elapsed_ms
        || next.schema != previous.schema
        || next.request != previous.request
        || next.scenarios.len() != previous.scenarios.len()
        || previous
            .first_failure
            .as_ref()
            .is_some_and(|failure| next.first_failure.as_ref() != Some(failure))
    {
        return Err("native suite journal rewrites an established event".into());
    }
    let mut changed = Vec::new();
    for (index, (before, after)) in previous.scenarios.iter().zip(&next.scenarios).enumerate() {
        if before.script != after.script
            || before.directory != after.directory
            || after.elapsed_ms < before.elapsed_ms
        {
            return Err("native suite journal rewrites scenario identity or duration".into());
        }
        if before != after {
            changed.push(index);
        }
    }
    if next.finalized {
        if !changed.is_empty() || previous.first_failure != next.first_failure {
            return Err("native suite finalization cannot rewrite execution".into());
        }
        return validate_outcome(next);
    }
    if next.passed {
        return Err("unfinished native suite cannot pass".into());
    }
    let valid = match changed.as_slice() {
        [] => {
            previous.first_failure.is_none()
                && next
                    .first_failure
                    .as_ref()
                    .is_some_and(|failure| failure.scenario.is_none() && !failure.detail.is_empty())
                && next
                    .scenarios
                    .iter()
                    .all(|row| !matches!(row.state, ScenarioState::Attempted))
        }
        [index] if previous.first_failure.is_none() => {
            let before = &previous.scenarios[*index];
            let after = &next.scenarios[*index];
            match (&before.state, &after.state) {
                (ScenarioState::NotRun, ScenarioState::Attempted) => {
                    next.first_failure.is_none()
                        && after.elapsed_ms == 0
                        && previous.scenarios[..*index]
                            .iter()
                            .all(|row| matches!(row.state, ScenarioState::Completed { .. }))
                        && previous.scenarios[*index..]
                            .iter()
                            .all(|row| row.state == ScenarioState::NotRun)
                }
                (ScenarioState::Attempted, ScenarioState::Completed { .. }) => {
                    next.first_failure.is_none()
                }
                (ScenarioState::Attempted, ScenarioState::Failed { detail }) => {
                    next.first_failure.as_ref().is_some_and(|failure| {
                        failure.scenario == Some(*index)
                            && failure.detail == *detail
                            && !detail.is_empty()
                    })
                }
                _ => false,
            }
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("native suite journal contains an invalid execution transition".into())
    }
}

#[cfg(test)]
mod journal_regressions {
    use super::*;

    fn test_authority() -> Authority {
        serde_json::from_slice(include_bytes!("../../ci/gates.json")).unwrap()
    }

    fn test_limits() -> uqm_rust::automation::native_window::NativeInventoryLimits {
        test_authority().native_runtime_contract().inventory_limits
    }

    fn pinned(name: &str) -> PinnedScript {
        PinnedScript {
            path: format!("rust/scripts/{name}.json"),
            sha256: "a".repeat(64),
            byte_length: 12,
        }
    }

    fn refused_claim(root: &Path, selected: &[PinnedScript], authority: &Authority) -> String {
        match from_request(Some(root), selected, authority) {
            Ok(_) => panic!("a root another run already claimed must not be claimed again"),
            Err(error) => error,
        }
    }

    /// An interrupted run leaves a journal that no reader accepts, and the run
    /// that finds it must not simply begin again over the top of it.
    #[test]
    fn an_interrupted_journal_is_finalized_instead_of_restarted() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a"), pinned("b")];
        {
            let mut interrupted = SuiteAccounting::create(&root, &selected, false).unwrap();
            interrupted.begin(0).unwrap();
            // The process stops here: no failure recorded, nothing finalized.
        }
        assert!(!root.join("suite-manifest.json").exists());
        let interrupted_status = retained_status(&root).unwrap().unwrap();
        assert!(!interrupted_status.finalized);
        assert!(
            validate_accounting(&root, &selected).is_err(),
            "an unfinished journal is not a bundle"
        );

        let recovered = recover_interrupted(&root, &selected, test_limits()).unwrap();
        assert!(recovered.finalized && !recovered.passed);
        let failure = recovered.first_failure.clone().unwrap();
        assert_eq!(failure.phase, SuitePhase::Execute);
        assert_eq!(failure.scenario, Some(0));
        assert!(failure.detail.contains("interrupted"), "{}", failure.detail);
        assert!(
            matches!(recovered.scenarios[0].state, ScenarioState::Failed { .. })
                && recovered.scenarios[1].state == ScenarioState::NotRun
        );
        // The attempt's duration was measured by a process that is gone, so the
        // recovery keeps what the journal recorded rather than inventing one.
        assert_eq!(
            recovered.scenarios[0].elapsed_ms,
            interrupted_status.scenarios[0].elapsed_ms
        );
        assert!(recovered.elapsed_ms >= interrupted_status.elapsed_ms);
        assert_eq!(
            recovered,
            validate_accounting(&root, &selected).unwrap(),
            "the recovered bundle must validate as the failure bundle it is"
        );
        assert!(
            recover_interrupted(&root, &selected, test_limits()).is_err(),
            "a finalized journal is not recoverable a second time"
        );
    }

    /// Recovery keeps the interruption attached to the run that was actually
    /// interrupted, so a journal cannot be adopted by a different request.
    #[test]
    fn recovery_refuses_a_journal_requested_for_another_selection() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        {
            let _interrupted = SuiteAccounting::create(&root, &selected, false).unwrap();
        }
        let error = recover_interrupted(&root, &[pinned("b")], test_limits()).unwrap_err();
        assert!(error.contains("different selection"), "{error}");
        // Nothing was published over the retained journal.
        assert!(!root.join("suite-manifest.json").exists());
        assert!(!retained_status(&root).unwrap().unwrap().finalized);
    }

    /// A run interrupted before any scenario was attempted still publishes a
    /// bundle that says so, rather than one that omits the selection.
    #[test]
    fn a_journal_interrupted_before_execution_names_the_phase_it_reached() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        {
            let _interrupted = SuiteAccounting::create(&root, &selected, false).unwrap();
        }
        let recovered = recover_interrupted(&root, &selected, test_limits()).unwrap();
        let failure = recovered.first_failure.clone().unwrap();
        assert_eq!(failure.phase, SuitePhase::Preflight);
        assert_eq!(failure.scenario, None);
        assert_eq!(recovered.scenarios[0].state, ScenarioState::NotRun);
        validate_accounting(&root, &selected).unwrap();
    }

    /// The claim path is where a restart would actually happen, so it is the
    /// path that has to refuse one.
    #[test]
    fn claiming_an_occupied_root_recovers_it_and_reports_the_interruption() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        {
            let mut interrupted = SuiteAccounting::create(&root, &selected, false).unwrap();
            interrupted.begin(0).unwrap();
        }
        let authority = test_authority();
        let error = refused_claim(&root, &selected, &authority);
        assert!(error.contains("recovered as a failure bundle"), "{error}");
        let manifest = validate_accounting(&root, &selected).unwrap();
        assert!(manifest.finalized && !manifest.passed);

        // The recovered bundle is now final, so a later claim refuses it
        // outright instead of recovering it again.
        let error = refused_claim(&root, &selected, &authority);
        assert!(error.contains("already holds a finalized run"), "{error}");
        assert_eq!(manifest, validate_accounting(&root, &selected).unwrap());
    }

    #[test]
    fn scenario_reservations_exhaust_before_launch_and_keep_failure_index() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![PinnedScript {
            path: "rust/scripts/a.json".into(),
            sha256: "a".repeat(64),
            byte_length: 12,
        }];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        assert!(suite.begin(0).is_err());
        let budget = suite.shared_budget();
        let remaining = budget.remaining().unwrap();
        let mut bytes = remaining.aggregate_bytes;
        let mut count = 0;
        while bytes > 0 {
            let length = bytes.min(remaining.member_bytes);
            budget
                .reserve(&format!("unwritten-{count}"), length)
                .unwrap();
            bytes -= length;
            count += 1;
        }
        assert!(suite.scenario_allowance(0).is_err());
        assert!(!suite.directory(0).unwrap().exists());
        conclude(
            &mut suite,
            Err("exhausted before launch".into()),
            SuitePhase::Execute,
        )
        .unwrap_err();
        let manifest = validate_accounting(&root, &selected).unwrap();
        assert!(!manifest.passed);
        assert_eq!(
            manifest.first_failure.unwrap().detail,
            "exhausted before launch"
        );
    }

    #[test]
    fn materialized_scenario_resolves_shared_objects_without_original_tree() {
        use uqm_rust::automation::native_artifacts::{SharedInputs, SharedSnapshot};
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![PinnedScript {
            path: "rust/scripts/a.json".into(),
            sha256: "a".repeat(64),
            byte_length: 12,
        }];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        let mut shared =
            SharedSnapshot::create(&root.join("shared"), suite.shared_budget()).unwrap();
        shared.insert("inputs/uqm", b"retained executable").unwrap();
        shared.seal().unwrap();
        let references = shared.references(&["inputs/uqm".into()]).unwrap();
        suite
            .publisher
            .create(
                "scenarios/0000/shared-inputs.json",
                &serde_json::to_vec(&references).unwrap(),
            )
            .unwrap();
        suite
            .publisher
            .create("scenarios/0000/stdout.log", b"retained log")
            .unwrap();
        suite
            .publisher
            .create("scenarios/0001/foreign.log", b"not selected")
            .unwrap();
        let snapshot = crate::ci::evidence::EvidenceSnapshot::open(&root).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        let retained = materialize_member(&snapshot, "scenarios/0000").unwrap();
        let scenario = retained.path().join("scenarios/0000");
        let references = SharedInputs::open(&scenario, 1024 * 1024).unwrap().unwrap();
        assert_eq!(
            references.read(&scenario, "inputs/uqm", 1024).unwrap(),
            b"retained executable"
        );
        assert_eq!(
            std::fs::read(scenario.join("stdout.log")).unwrap(),
            b"retained log"
        );
        assert!(!retained.path().join("scenarios/0001").exists());
        let object = references.path(&scenario, "inputs/uqm", 1024).unwrap();
        std::fs::write(object, b"substituted").unwrap();
        assert!(references.read(&scenario, "inputs/uqm", 1024).is_err());
    }

    #[test]
    fn scenario_accounting_charges_empty_directories_and_refuses_links_and_reuse() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![PinnedScript {
            path: "rust/scripts/a.json".into(),
            sha256: "a".repeat(64),
            byte_length: 12,
        }];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        let scenario = suite.directory(0).unwrap();
        std::fs::create_dir_all(scenario.join("empty/nested")).unwrap();
        std::fs::write(scenario.join("payload"), b"123").unwrap();
        let before = suite.budget.remaining().unwrap();
        suite.account_scenario(0).unwrap();
        let after = suite.budget.remaining().unwrap();
        assert_eq!(before.member_count - after.member_count, 3);
        assert_eq!(before.aggregate_bytes - after.aggregate_bytes, 3);
        assert!(suite.account_scenario(0).is_err());
        std::fs::remove_file(scenario.join("payload")).unwrap();
        std::os::unix::fs::symlink(temporary.path(), scenario.join("escape")).unwrap();
        assert!(suite
            .account_scenario(0)
            .unwrap_err()
            .contains("nonregular"));
    }

    #[test]
    fn not_run_rows_cannot_hide_reindexed_scenario_outputs() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![PinnedScript {
            path: "rust/scripts/a.json".into(),
            sha256: "a".repeat(64),
            byte_length: 12,
        }];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite
            .fail(SuitePhase::Preflight, None, "bad pin".into())
            .unwrap();
        suite.finish().unwrap();
        suite
            .publisher
            .create("scenarios/0000/native-acceptance.json", b"{}")
            .unwrap();
        let index = SuiteIndex {
            schema: "uqm-native-suite-index-v1".into(),
            passed: false,
            files: inventory(&root).unwrap(),
        };
        suite
            .publisher
            .replace(
                "suite-index.json",
                &serde_json::to_vec_pretty(&index).unwrap(),
            )
            .unwrap();
        assert!(validate_accounting(&root, &selected).is_err());
    }

    #[test]
    fn rewritten_attempt_history_is_rejected_even_with_updated_index_hashes() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![PinnedScript {
            path: "rust/scripts/a.json".into(),
            sha256: "a".repeat(64),
            byte_length: 12,
        }];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        suite
            .fail(SuitePhase::Execute, Some(0), "failure".into())
            .unwrap();
        suite.finish().unwrap();
        let event_path = "suite-events/000001.json";
        let mut event: SuiteManifest = serde_json::from_slice(
            &crate::ci::evidence::read_regular_relative(&root, event_path).unwrap(),
        )
        .unwrap();
        event.scenarios[0].state = ScenarioState::NotRun;
        suite
            .publisher
            .replace(event_path, &serde_json::to_vec_pretty(&event).unwrap())
            .unwrap();
        let index = SuiteIndex {
            schema: "uqm-native-suite-index-v1".into(),
            passed: false,
            files: inventory(&root).unwrap(),
        };
        suite
            .publisher
            .replace(
                "suite-index.json",
                &serde_json::to_vec_pretty(&index).unwrap(),
            )
            .unwrap();
        assert!(validate_accounting(&root, &selected).is_err());
    }
}

/// The fault and teardown matrix the acceptance contract names.
///
/// A crash, a timeout, a signal, an output-limit kill and an escaped
/// descendant each have to leave diagnostics in the retained bundle and each
/// has to say what became of the process state. These tests drive the
/// controller-side receipt directly with synthetic supervision rows; they are
/// fixtures for the accounting, not evidence that a game ran.
#[cfg(test)]
mod teardown_regressions {
    use super::*;

    fn pinned(name: &str) -> PinnedScript {
        PinnedScript {
            path: format!("rust/scripts/{name}.json"),
            sha256: "a".repeat(64),
            byte_length: 12,
        }
    }

    /// A supervision row whose process group was verified empty and whose
    /// pipes drained, carrying the fault the caller names.
    fn clean_teardown(class: FaultClass) -> ControllerSupervision {
        ControllerSupervision {
            class,
            detail: format!("{class:?} observed by the controller"),
            exit_code: match class {
                FaultClass::Exit => Some(9),
                FaultClass::Completed => Some(0),
                _ => None,
            },
            signal: (class == FaultClass::Signal).then_some(11),
            timed_out: class == FaultClass::Timeout,
            termination_reason: match class {
                FaultClass::Timeout => "timeout".into(),
                FaultClass::OutputLimit => "output-limit".into(),
                FaultClass::EscapedDescendants => "descendant-cleanup".into(),
                _ => "none".into(),
            },
            termination_signal: "none".into(),
            process_group_cleanup: "verified-empty".into(),
            pipe_cleanup: "complete".into(),
            descendant_survivors: (class == FaultClass::EscapedDescendants)
                .then(|| "pid 4242 uqm".to_string()),
            stdout_bytes_seen: 12,
            stderr_bytes_seen: 5,
            stdout_log: None,
            stderr_log: None,
        }
    }

    fn finalized_failure(
        root: &Path,
        selected: &[PinnedScript],
        supervision: ControllerSupervision,
        stdout: &[u8],
        stderr: &[u8],
    ) -> SuiteManifest {
        let mut suite = SuiteAccounting::create(root, selected, false).unwrap();
        suite.begin(0).unwrap();
        suite
            .record_supervision(0, supervision, stdout, stderr)
            .unwrap();
        suite
            .fail(
                SuitePhase::Execute,
                Some(0),
                "child did not complete".into(),
            )
            .unwrap();
        suite.finish().unwrap();
        validate_accounting(root, selected).unwrap()
    }

    fn published_teardown(root: &Path, manifest: &SuiteManifest) -> SuiteTeardown {
        read_teardown(root, manifest).unwrap()
    }

    #[test]
    fn every_child_fault_retains_diagnostics_and_states_its_process_state() {
        for class in [
            FaultClass::Exit,
            FaultClass::Signal,
            FaultClass::Timeout,
            FaultClass::OutputLimit,
            FaultClass::EscapedDescendants,
            FaultClass::LaunchFailed,
            FaultClass::SupervisionFailed,
        ] {
            let temporary = tempfile::tempdir().unwrap();
            let root = temporary.path().join("suite");
            let selected = vec![pinned("a")];
            let manifest = finalized_failure(
                &root,
                &selected,
                clean_teardown(class),
                b"child stdout",
                b"boom!",
            );
            let teardown = published_teardown(&root, &manifest);
            let row = teardown.scenarios[0].supervision.clone().unwrap();

            assert_eq!(row.class, class, "the receipt must keep the observed fault");
            let stdout = row
                .stdout_log
                .as_ref()
                .expect("a faulted child retains its stdout");
            let stderr = row
                .stderr_log
                .as_ref()
                .expect("a faulted child retains its stderr");
            assert_eq!(
                std::fs::read(root.join(&stdout.relative_path)).unwrap(),
                b"child stdout"
            );
            assert_eq!(
                std::fs::read(root.join(&stderr.relative_path)).unwrap(),
                b"boom!"
            );
            assert!(
                !stdout.relative_path.starts_with("scenarios/"),
                "controller output must not join the child's own bundle inventory"
            );

            // An escaped descendant is the only one of these that leaves state
            // behind. A crash, a timeout, a signal or an output-limit kill is a
            // failed run on a clean host.
            let expected_clear = class != FaultClass::EscapedDescendants;
            assert_eq!(
                teardown.process_state_clear, expected_clear,
                "{class:?} reported the wrong process state"
            );
            assert_eq!(row.process_state_clear(), expected_clear);
        }
    }

    #[test]
    fn a_bounded_fault_log_says_how_much_it_dropped() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let oversize = vec![b'x'; (FAULT_LOG_LIMIT as usize) + 4096];
        let mut supervision = clean_teardown(FaultClass::Exit);
        supervision.stdout_bytes_seen = oversize.len() as u64;
        let manifest = finalized_failure(&root, &selected, supervision, &oversize, b"");
        let row = published_teardown(&root, &manifest).scenarios[0]
            .supervision
            .clone()
            .unwrap();

        let stdout = row.stdout_log.unwrap();
        assert_eq!(stdout.byte_length, FAULT_LOG_LIMIT);
        assert_eq!(
            row.stdout_bytes_seen,
            oversize.len() as u64,
            "the receipt keeps the full observed length beside the retained one"
        );
        assert!(
            row.stderr_log.is_none(),
            "an empty stream is not retained as an empty file"
        );
    }

    #[test]
    fn a_receipt_cannot_claim_a_process_state_its_rows_do_not_establish() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let manifest = finalized_failure(
            &root,
            &selected,
            clean_teardown(FaultClass::EscapedDescendants),
            b"out",
            b"err",
        );
        let mut teardown = published_teardown(&root, &manifest);
        assert!(!teardown.process_state_clear);

        teardown.process_state_clear = true;
        republish(&root, &teardown);

        let error = validate_accounting(&root, &selected).unwrap_err();
        assert!(
            error.contains("its own rows do not establish"),
            "a forged clean-teardown claim must be refused: {error}"
        );
    }

    #[test]
    fn a_receipt_cannot_hide_the_descendant_that_made_it_dirty() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let manifest = finalized_failure(
            &root,
            &selected,
            clean_teardown(FaultClass::EscapedDescendants),
            b"out",
            b"err",
        );
        let mut teardown = published_teardown(&root, &manifest);
        teardown.scenarios[0]
            .supervision
            .as_mut()
            .unwrap()
            .descendant_survivors = None;
        republish(&root, &teardown);

        let error = validate_accounting(&root, &selected).unwrap_err();
        assert!(
            error.contains("contradicts the fault class"),
            "an escaped-descendant row without survivors is not that class: {error}"
        );
    }

    #[test]
    fn an_altered_fault_log_is_refused_by_its_recorded_identity() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let manifest = finalized_failure(
            &root,
            &selected,
            clean_teardown(FaultClass::Timeout),
            b"original",
            b"err",
        );
        let teardown = published_teardown(&root, &manifest);
        let log = teardown.scenarios[0]
            .supervision
            .clone()
            .unwrap()
            .stdout_log
            .unwrap();

        std::fs::write(root.join(&log.relative_path), b"rewritten").unwrap();

        let error = validate_accounting(&root, &selected).unwrap_err();
        assert!(
            error.contains("complete inventory mismatch")
                || error.contains("differs from the retained bytes"),
            "{error}"
        );
    }

    #[test]
    fn a_scenario_that_never_ran_cannot_claim_a_supervised_child() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a"), pinned("b")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        suite
            .record_supervision(0, clean_teardown(FaultClass::Exit), b"out", b"err")
            .unwrap();
        suite
            .fail(SuitePhase::Execute, Some(0), "child exited 9".into())
            .unwrap();
        suite.finish().unwrap();
        let manifest = validate_accounting(&root, &selected).unwrap();

        let mut teardown = published_teardown(&root, &manifest);
        assert_eq!(teardown.scenarios[1].supervision, None);
        teardown.scenarios[1].supervision = Some(clean_teardown(FaultClass::Completed));
        republish(&root, &teardown);

        let error = validate_accounting(&root, &selected).unwrap_err();
        assert!(
            error.contains("scenario that never ran"),
            "an unattempted row cannot acquire a child: {error}"
        );
    }

    #[test]
    fn supervision_belongs_to_the_live_attempt_and_is_recorded_once() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a"), pinned("b")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();

        assert!(
            suite
                .record_supervision(0, clean_teardown(FaultClass::Exit), b"", b"")
                .is_err(),
            "a child cannot be recorded before its attempt begins"
        );
        suite.begin(0).unwrap();
        assert!(suite
            .record_supervision(1, clean_teardown(FaultClass::Exit), b"", b"")
            .is_err());
        suite
            .record_supervision(0, clean_teardown(FaultClass::Exit), b"", b"")
            .unwrap();
        assert!(
            suite
                .record_supervision(0, clean_teardown(FaultClass::Exit), b"", b"")
                .is_err(),
            "a second receipt for one attempt would overwrite the first"
        );
    }

    #[test]
    fn an_internally_contradictory_receipt_is_refused_at_the_source() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        let mut supervision = clean_teardown(FaultClass::Timeout);
        supervision.timed_out = false;

        let error = suite
            .record_supervision(0, supervision, b"", b"")
            .unwrap_err();

        assert!(error.contains("contradicts the fault class"), "{error}");
    }

    #[test]
    fn a_preflight_failure_leaves_a_clean_receipt_with_no_children() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite
            .fail(SuitePhase::Preflight, None, "bad pin".into())
            .unwrap();
        suite.finish().unwrap();
        let manifest = validate_accounting(&root, &selected).unwrap();

        let teardown = published_teardown(&root, &manifest);
        assert_eq!(teardown.scenarios[0].supervision, None);
        assert!(
            teardown.process_state_clear,
            "a run that launched nothing left nothing behind"
        );
    }

    #[test]
    fn an_attempt_that_never_reached_a_child_says_so_rather_than_going_silent() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        suite
            .record_supervision(
                0,
                ControllerSupervision::not_launched("evidence capacity exhausted".into()),
                b"",
                b"",
            )
            .unwrap();
        suite
            .fail(
                SuitePhase::Execute,
                Some(0),
                "evidence capacity exhausted".into(),
            )
            .unwrap();
        suite.finish().unwrap();
        let manifest = validate_accounting(&root, &selected).unwrap();

        let teardown = published_teardown(&root, &manifest);
        let row = teardown.scenarios[0].supervision.clone().unwrap();
        assert_eq!(row.class, FaultClass::NotLaunched);
        assert!(
            teardown.process_state_clear,
            "no child existed, so no process state was created"
        );
    }

    /// Recovery finalizes a journal whose children were supervised by a
    /// process that no longer exists. It must not sign for their teardown.
    #[test]
    fn recovery_never_claims_a_teardown_it_did_not_observe() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        {
            let mut interrupted = SuiteAccounting::create(&root, &selected, false).unwrap();
            interrupted.begin(0).unwrap();
        }
        let limits: uqm_rust::automation::native_window::NativeInventoryLimits =
            serde_json::from_slice::<Authority>(include_bytes!("../../ci/gates.json"))
                .unwrap()
                .native_runtime_contract()
                .inventory_limits;

        let recovered = recover_interrupted(&root, &selected, limits).unwrap();

        let teardown = published_teardown(&root, &recovered);
        assert_eq!(teardown.scenarios[0].supervision, None);
        assert!(
            !teardown.process_state_clear,
            "an interrupted attempt's process state was never observed and is not clear"
        );
        assert!(teardown.scope.contains("no longer exists"));
    }

    /// Storage runs out exactly when a run is going wrong, so the space to
    /// say so is claimed before any scenario is allowed to spend the budget.
    #[test]
    fn an_exhausted_artifact_budget_still_publishes_its_teardown_receipt() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        let budget = suite.shared_budget();
        let remaining = budget.remaining().unwrap();
        let mut bytes = remaining.aggregate_bytes;
        let mut claim = 0;
        while bytes > 0 {
            let length = bytes.min(remaining.member_bytes);
            budget.reserve(&format!("exhaust-{claim}"), length).unwrap();
            bytes -= length;
            claim += 1;
        }
        assert!(
            suite.scenario_allowance(0).is_err(),
            "the budget has to actually be exhausted for this to prove anything"
        );

        suite
            .record_supervision(
                0,
                ControllerSupervision::not_launched("evidence capacity exhausted".into()),
                b"",
                b"",
            )
            .unwrap();
        suite
            .fail(
                SuitePhase::Execute,
                Some(0),
                "evidence capacity exhausted".into(),
            )
            .unwrap();
        suite.finish().unwrap();

        let manifest = validate_accounting(&root, &selected).unwrap();
        let teardown = published_teardown(&root, &manifest);
        assert_eq!(
            teardown.scenarios[0].supervision.as_ref().unwrap().class,
            FaultClass::NotLaunched
        );
        assert!(teardown.process_state_clear);
    }

    #[test]
    fn a_finalized_bundle_without_a_teardown_receipt_is_not_a_bundle() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        let selected = vec![pinned("a")];
        let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
        suite
            .fail(SuitePhase::Preflight, None, "bad pin".into())
            .unwrap();
        suite.finish().unwrap();
        validate_accounting(&root, &selected).unwrap();

        std::fs::remove_file(root.join(TEARDOWN_RELATIVE)).unwrap();

        assert!(
            validate_accounting(&root, &selected).is_err(),
            "a bundle that dropped its teardown receipt must not validate"
        );
    }

    /// Replace the published receipt and the index that covers it, so the
    /// mutation is exercised against the teardown rules rather than being
    /// rejected earlier as an inventory drift.
    fn republish(root: &Path, teardown: &SuiteTeardown) {
        let publisher = crate::ci::evidence::EvidencePublisher::open(root).unwrap();
        publisher
            .replace(
                TEARDOWN_RELATIVE,
                &serde_json::to_vec_pretty(teardown).unwrap(),
            )
            .unwrap();
        let manifest: SuiteManifest = serde_json::from_slice(
            &crate::ci::evidence::read_regular_relative(root, "suite-manifest.json").unwrap(),
        )
        .unwrap();
        let index = SuiteIndex {
            schema: "uqm-native-suite-index-v1".into(),
            passed: manifest.passed,
            files: inventory(root).unwrap(),
        };
        publisher
            .replace(
                "suite-index.json",
                &serde_json::to_vec_pretty(&index).unwrap(),
            )
            .unwrap();
    }
}

/// Claim a suite root the caller named rather than one the environment did.
///
/// The CI route binds the root through the environment; the runner command
/// binds it through `--artifacts`. Both reach the same accounting, so a locally
/// produced bundle has the layout the offline validator already understands.
///
/// A root that already holds a journal belongs to a run that got there first.
/// Starting again over it would either lose that run's evidence or, with the
/// root refused for being non-empty, leave an unreadable half-bundle behind and
/// say nothing about why. So an unfinished journal is finalized into the
/// failure bundle its own run never reached, and this returns that outcome
/// instead of a fresh suite.
pub fn from_request(
    evidence_root: Option<&Path>,
    selected: &[PinnedScript],
    authority: &Authority,
) -> Result<Option<SuiteAccounting>, String> {
    let Some(root) = evidence_root else {
        return Ok(None);
    };
    let limits = authority.native_runtime_contract().inventory_limits;
    if let Some(status) = retained_status(root)? {
        if status.finalized {
            return Err(format!(
                "native suite root {} already holds a finalized run; a new run needs its own root",
                root.display()
            ));
        }
        let recovered = recover_interrupted(root, selected, limits)?;
        return Err(format!(
            "native suite root {} held an interrupted run, recovered as a failure bundle rather than restarted: {}",
            root.display(),
            recovered
                .first_failure
                .map_or_else(|| "no recorded failure".into(), |failure| failure.detail)
        ));
    }
    SuiteAccounting::create_with_limits(
        root,
        selected,
        matches!(
            std::env::var("UQM_CI_NATIVE_ACCEPTANCE_PRECREATED_ROOT").as_deref(),
            Ok("1")
        ),
        limits,
    )
    .map(Some)
}

/// The journal state a previous run left in `root`, if it left one.
///
/// The status document is published with the suite's first transition, before
/// any child runs, so its presence is what separates a root some run already
/// claimed from a root that merely has something else in it.
fn retained_status(root: &Path) -> Result<Option<SuiteManifest>, String> {
    let bytes = match crate::ci::evidence::read_regular_relative(root, "suite-status.json") {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read retained suite status: {error}")),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("parse retained suite status: {error}"))
}

/// Finalize the journal an interrupted run left behind.
///
/// An interrupted run publishes its last transition and then stops, so its
/// journal is intact but carries no manifest, index or catalog and no offline
/// reader will accept it. Recovery rebinds that journal to the same selection,
/// records the interruption against whichever row was active, and publishes the
/// failure bundle the run never reached. The interrupted attempt keeps the
/// duration the journal already recorded: it was measured by a process that no
/// longer exists, and this one cannot measure it after the fact.
pub fn recover_interrupted(
    root: &Path,
    selected: &[PinnedScript],
    limits: uqm_rust::automation::native_window::NativeInventoryLimits,
) -> Result<SuiteManifest, String> {
    let status = retained_status(root)?
        .ok_or_else(|| format!("native suite root {} holds no journal", root.display()))?;
    if status.finalized {
        return Err(format!(
            "native suite journal in {} is already final",
            root.display()
        ));
    }
    if status.schema != "uqm-native-suite-v1" {
        return Err(format!(
            "retained suite journal schema {:?} is not uqm-native-suite-v1",
            status.schema
        ));
    }
    let request = crate::ci::evidence::read_regular_relative(root, "suite-request.json")
        .map_err(|error| format!("read retained suite request: {error}"))?;
    let expected = serde_json::to_vec_pretty(selected).map_err(|error| error.to_string())?;
    if request != expected
        || status.request != crate::retained_identity("suite-request.json", &request)
        || status
            .scenarios
            .iter()
            .map(|row| row.script.clone())
            .collect::<Vec<_>>()
            != selected
    {
        return Err(format!(
            "native suite journal in {} was requested for a different selection",
            root.display()
        ));
    }
    let budget = uqm_rust::automation::native_artifacts::ArtifactBudget::new(limits);
    budget.reserve("suite-request.json", request.len() as u64)?;
    budget.reserve_directory("scenarios")?;
    let diagnostics =
        reserve_suite_diagnostics(&budget, limits, status.scenarios.len(), request.len())?;
    let active = status
        .scenarios
        .iter()
        .position(|row| matches!(row.state, ScenarioState::Attempted));
    let carried_ms = status.elapsed_ms;
    let sequence = status
        .sequence
        .checked_add(1)
        .ok_or("retained suite sequence overflow")?;
    // Recovery supervised none of these children. Every slot stays empty, so
    // the published receipt cannot claim a teardown this process never saw.
    let teardown = vec![None; status.scenarios.len()];
    let mut suite = SuiteAccounting {
        root: root.to_path_buf(),
        publisher: crate::ci::evidence::EvidencePublisher::open(root)
            .map_err(|error| error.to_string())?,
        manifest: SuiteManifest { sequence, ..status },
        started: std::time::Instant::now(),
        carried_ms,
        member_started: None,
        budget,
        diagnostics,
        teardown,
    };
    // Whatever the interrupted run retained is charged before anything else is
    // published, so recovery cannot exceed the budget that run was held to.
    for index in 0..suite.manifest.scenarios.len() {
        suite.account_scenario(index)?;
    }
    let (phase, detail) = match active {
        Some(index) => (
            SuitePhase::Execute,
            format!(
                "native suite run was interrupted while scenario {index} was attempted; the attempt's duration was never measured"
            ),
        ),
        None => (
            SuitePhase::Preflight,
            "native suite run was interrupted before any scenario was attempted".to_string(),
        ),
    };
    let retained_elapsed = active.map(|index| suite.manifest.scenarios[index].elapsed_ms);
    suite.record_failure(phase, active, detail, retained_elapsed)?;
    suite.finish()?;
    validate_accounting(root, selected)
}

/// The shared store's descriptor, read directly by offline consumers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedDescriptorDocument {
    schema: String,
    members: std::collections::BTreeMap<String, uqm_rust::automation::NativeRetainedInput>,
}

/// Every logical shared member of a retained suite, verified against the
/// sealed descriptor.
///
/// `SharedInputs` resolves the members of one launched scenario. An offline
/// reader also needs the members of scenarios that never ran: their script
/// bytes are what says which obligations the suite still owed, and a replay
/// needs every input regardless of which scenario consumed it.
pub fn shared_members(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
    let bytes = match snapshot.read("shared/descriptor.json") {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(std::collections::BTreeMap::new())
        }
        Err(error) => return Err(format!("read shared/descriptor.json: {error}")),
    };
    if bytes.len() as u64 > SHARED_DESCRIPTOR_LIMIT {
        return Err("shared descriptor exceeds its retained bound".into());
    }
    let descriptor: SharedDescriptorDocument = serde_json::from_slice(bytes)
        .map_err(|error| format!("parse shared/descriptor.json: {error}"))?;
    if descriptor.schema != "uqm-native-shared-descriptor-v1" {
        return Err(format!(
            "shared descriptor schema {:?} is not uqm-native-shared-descriptor-v1",
            descriptor.schema
        ));
    }
    let mut members = std::collections::BTreeMap::new();
    for (logical, member) in &descriptor.members {
        let object = format!("shared/{}", member.relative_path);
        let bytes = snapshot
            .read(&object)
            .map_err(|error| format!("read shared object {object}: {error}"))?;
        if bytes.len() as u64 != member.byte_length || crate::hex_sha256(bytes) != member.sha256 {
            return Err(format!(
                "shared object {object} differs from its descriptor"
            ));
        }
        members.insert(logical.clone(), bytes.to_vec());
    }
    Ok(members)
}

/// The shared store's own descriptor bound, mirrored for offline readers.
const SHARED_DESCRIPTOR_LIMIT: u64 = 1024 * 1024;

/// Preserve the execution error even if publication also fails.
pub fn conclude(
    suite: &mut SuiteAccounting,
    result: Result<(), String>,
    phase: SuitePhase,
) -> Result<(), String> {
    let publication = (|| {
        if let Err(detail) = &result {
            let active = suite
                .manifest
                .scenarios
                .iter()
                .position(|row| matches!(row.state, ScenarioState::Attempted));
            suite.fail(phase, active, detail.clone())?;
        }
        suite.finish()?;
        let selected = suite
            .manifest
            .scenarios
            .iter()
            .map(|row| row.script.clone())
            .collect::<Vec<_>>();
        validate_accounting(&suite.root, &selected)?;
        println!(
            "native suite index: {}",
            suite.root.join("suite-index.json").display()
        );
        Ok(())
    })();
    match (result, publication) {
        (Err(original), Err(publication)) => {
            Err(format!("{original}; suite publication: {publication}"))
        }
        (Err(original), Ok(())) => Err(original),
        (Ok(()), publication) => publication,
    }
}

/// The phase that first prevented the exact requested suite from completing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuitePhase {
    Preflight,
    Build,
    Execute,
    Validate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScenarioState {
    NotRun,
    Attempted,
    Completed {
        manifest: uqm_rust::automation::NativeRetainedInput,
    },
    Failed {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioAccounting {
    pub script: PinnedScript,
    pub directory: String,
    pub state: ScenarioState,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteFailure {
    pub phase: SuitePhase,
    pub scenario: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteManifest {
    pub schema: String,
    pub sequence: u64,
    pub request: uqm_rust::automation::NativeRetainedInput,
    pub scenarios: Vec<ScenarioAccounting>,
    pub first_failure: Option<SuiteFailure>,
    pub elapsed_ms: u64,
    pub finalized: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SuiteIndex {
    schema: String,
    passed: bool,
    files: Vec<uqm_rust::automation::NativeRetainedInput>,
}
/// The suite-owned teardown receipt every finalized bundle carries.
pub const TEARDOWN_RELATIVE: &str = "suite-teardown.json";

const TEARDOWN_SCHEMA: &str = "uqm-native-suite-teardown-v1";

/// How much of a faulted child's captured output is retained per stream.
///
/// The supervisor already bounds what it reads; this bounds what the bundle
/// keeps, so 32 faulted scenarios cannot spend the whole artifact budget on
/// logs. The receipt records the full observed length beside the retained one,
/// so a truncated log is visibly truncated rather than quietly short.
const FAULT_LOG_LIMIT: u64 = 64 * 1024;

/// The marker the neutral supervision fields carry when no child was spawned.
const NOT_LAUNCHED: &str = "not-launched";

/// What the controller observed when the scenario's child returned.
///
/// This is the controller's own account of the child process, separate from
/// the manifest the child writes for itself. A child that dies before it can
/// write anything still leaves one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultClass {
    /// The attempt failed before any child process existed.
    NotLaunched,
    /// The child ran to completion under supervision and exited zero.
    Completed,
    /// The child exited nonzero.
    Exit,
    /// The child was terminated by a signal it did not choose.
    Signal,
    /// The child exceeded its authorized timeout and was terminated.
    Timeout,
    /// The child exceeded an authorized output limit and was terminated.
    OutputLimit,
    /// The child left descendants in the process group the controller owns.
    EscapedDescendants,
    /// The child could not be started at all.
    LaunchFailed,
    /// Supervision itself failed, so the child's outcome is not established.
    SupervisionFailed,
}

/// The controller's account of one supervised scenario child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerSupervision {
    pub class: FaultClass,
    /// The first contract the child broke, or a statement that it broke none.
    pub detail: String,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
    pub termination_reason: String,
    pub termination_signal: String,
    pub process_group_cleanup: String,
    pub pipe_cleanup: String,
    /// The descendants the controller found still owning the group, if any.
    pub descendant_survivors: Option<String>,
    pub stdout_bytes_seen: u64,
    pub stderr_bytes_seen: u64,
    /// Bounded retained diagnostics, present when the child did not complete.
    pub stdout_log: Option<uqm_rust::automation::NativeRetainedInput>,
    pub stderr_log: Option<uqm_rust::automation::NativeRetainedInput>,
}

impl ControllerSupervision {
    /// The neutral row for an attempt that failed before any child existed.
    pub fn not_launched(detail: String) -> Self {
        Self {
            class: FaultClass::NotLaunched,
            detail,
            exit_code: None,
            signal: None,
            timed_out: false,
            termination_reason: NOT_LAUNCHED.into(),
            termination_signal: NOT_LAUNCHED.into(),
            process_group_cleanup: NOT_LAUNCHED.into(),
            pipe_cleanup: NOT_LAUNCHED.into(),
            descendant_survivors: None,
            stdout_bytes_seen: 0,
            stderr_bytes_seen: 0,
            stdout_log: None,
            stderr_log: None,
        }
    }

    /// Whether this row leaves no process state behind.
    ///
    /// A fault is not a dirty teardown. A child that timed out, was signalled
    /// or exited nonzero still leaves the host clean when its process group was
    /// verified empty and its pipes were drained. Only a surviving descendant,
    /// an unverified group or an undrained pipe makes this false.
    #[must_use]
    pub fn process_state_clear(&self) -> bool {
        if self.class == FaultClass::NotLaunched {
            return true;
        }
        self.descendant_survivors.is_none()
            && self.pipe_cleanup == "complete"
            && matches!(
                self.process_group_cleanup.as_str(),
                "verified-empty" | "not-supported"
            )
    }

    /// Refuse a row whose fields contradict the class it declares.
    fn self_consistent(&self) -> bool {
        let neutral = self.termination_reason == NOT_LAUNCHED
            && self.termination_signal == NOT_LAUNCHED
            && self.process_group_cleanup == NOT_LAUNCHED
            && self.pipe_cleanup == NOT_LAUNCHED
            && self.exit_code.is_none()
            && self.signal.is_none()
            && !self.timed_out
            && self.descendant_survivors.is_none()
            && self.stdout_log.is_none()
            && self.stderr_log.is_none();
        if self.detail.is_empty() {
            return false;
        }
        match self.class {
            FaultClass::NotLaunched => neutral,
            FaultClass::Completed => {
                !neutral
                    && self.exit_code == Some(0)
                    && self.signal.is_none()
                    && !self.timed_out
                    && self.termination_reason == "none"
                    && self.descendant_survivors.is_none()
                    && self.stdout_log.is_none()
                    && self.stderr_log.is_none()
            }
            FaultClass::Timeout => !neutral && self.timed_out,
            FaultClass::Signal => !neutral && self.signal.is_some(),
            FaultClass::Exit => {
                !neutral && self.signal.is_none() && self.exit_code.is_some_and(|code| code != 0)
            }
            FaultClass::OutputLimit => !neutral && self.termination_reason == "output-limit",
            FaultClass::EscapedDescendants => !neutral && self.descendant_survivors.is_some(),
            FaultClass::LaunchFailed | FaultClass::SupervisionFailed => !neutral,
        }
    }
}

/// One selected scenario's teardown row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeardownRow {
    pub scenario: usize,
    pub directory: String,
    pub script: PinnedScript,
    /// Absent when this process did not supervise a child for the row, which
    /// is either because none was ever attempted or because the attempt
    /// belonged to a process that no longer exists.
    pub supervision: Option<ControllerSupervision>,
}

/// What the suite can say about the process state it leaves behind.
///
/// This receipt covers the controller's own children. It does not speak for
/// host state the controller never owned, and it says so rather than implying
/// a wider guarantee than it can support.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteTeardown {
    pub schema: String,
    /// True only when every attempted row was supervised by this process and
    /// each one left its process group verified empty with its pipes drained.
    pub process_state_clear: bool,
    pub scope: String,
    pub scenarios: Vec<TeardownRow>,
}

impl SuiteTeardown {
    const SCOPE: &'static str =
        "the controller's own scenario children: process-group emptiness, pipe drain and \
         escaped-descendant observation. It does not attest to host state this controller never \
         owned, and an attempt supervised by a process that no longer exists is never claimed as \
         clear.";

    fn build(manifest: &SuiteManifest, rows: &[Option<ControllerSupervision>]) -> Self {
        let scenarios: Vec<TeardownRow> = manifest
            .scenarios
            .iter()
            .enumerate()
            .map(|(scenario, row)| TeardownRow {
                scenario,
                directory: row.directory.clone(),
                script: row.script.clone(),
                supervision: rows.get(scenario).cloned().flatten(),
            })
            .collect();
        Self {
            schema: TEARDOWN_SCHEMA.into(),
            process_state_clear: derive_process_state_clear(manifest, &scenarios),
            scope: Self::SCOPE.into(),
            scenarios,
        }
    }
}

/// Whether the controller can honestly claim it left no process state.
///
/// An attempted row with no supervision is not clear: the process that would
/// have observed the teardown is gone, and this one cannot observe it after
/// the fact. A row that was never attempted created no process state at all.
fn derive_process_state_clear(manifest: &SuiteManifest, rows: &[TeardownRow]) -> bool {
    manifest.scenarios.len() == rows.len()
        && manifest.scenarios.iter().zip(rows).all(|(state, row)| {
            match (&state.state, &row.supervision) {
                (ScenarioState::NotRun, None) => true,
                (_, Some(supervision)) => supervision.process_state_clear(),
                (_, None) => false,
            }
        })
}

/// Read and check a finalized bundle's teardown receipt.
///
/// A receipt is refused when it covers a different selection, when a row
/// contradicts the class it declares, when a scenario the suite recorded as
/// completed carries a faulted child, when a row that was never attempted
/// claims a supervised child, when a retained log is absent or altered, or
/// when the clean-teardown claim is not the one the rows actually derive.
fn validate_teardown(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
    manifest: &SuiteManifest,
) -> Result<SuiteTeardown, String> {
    let bytes = snapshot
        .read(TEARDOWN_RELATIVE)
        .map_err(|error| format!("read {TEARDOWN_RELATIVE}: {error}"))?;
    let teardown: SuiteTeardown = serde_json::from_slice(bytes)
        .map_err(|error| format!("parse {TEARDOWN_RELATIVE}: {error}"))?;
    if teardown.schema != TEARDOWN_SCHEMA
        || teardown.scope != SuiteTeardown::SCOPE
        || teardown.scenarios.len() != manifest.scenarios.len()
    {
        return Err(
            "native suite teardown receipt does not describe this suite's selection".into(),
        );
    }
    for (index, (row, accounting)) in teardown
        .scenarios
        .iter()
        .zip(&manifest.scenarios)
        .enumerate()
    {
        if row.scenario != index
            || row.directory != accounting.directory
            || row.script != accounting.script
        {
            return Err("native suite teardown row does not match its scenario".into());
        }
        let Some(supervision) = &row.supervision else {
            continue;
        };
        if accounting.state == ScenarioState::NotRun {
            return Err(
                "native suite teardown claims a supervised child for a scenario that never ran"
                    .into(),
            );
        }
        if !supervision.self_consistent() {
            return Err(format!(
                "native suite teardown row {index} contradicts the fault class it declares"
            ));
        }
        if matches!(accounting.state, ScenarioState::Completed { .. })
            && !matches!(
                supervision.class,
                FaultClass::Completed | FaultClass::NotLaunched
            )
        {
            return Err(
                "native suite recorded a completed scenario whose child did not complete".into(),
            );
        }
        for log in [&supervision.stdout_log, &supervision.stderr_log]
            .into_iter()
            .flatten()
        {
            let retained = snapshot
                .read(&log.relative_path)
                .map_err(|error| format!("read {}: {error}", log.relative_path))?;
            if crate::retained_identity(&log.relative_path, retained) != *log {
                return Err(format!(
                    "native suite teardown log {} differs from the retained bytes",
                    log.relative_path
                ));
            }
        }
    }
    if teardown.process_state_clear != derive_process_state_clear(manifest, &teardown.scenarios) {
        return Err(
            "native suite teardown claims a process state its own rows do not establish".into(),
        );
    }
    Ok(teardown)
}

/// The checked teardown receipt of a finalized bundle, for reporting.
pub fn read_teardown(bundle: &Path, manifest: &SuiteManifest) -> Result<SuiteTeardown, String> {
    let snapshot = crate::ci::evidence::EvidenceSnapshot::open(bundle)
        .map_err(|error| format!("open native suite {}: {error}", bundle.display()))?;
    snapshot.scoped(|| validate_teardown(&snapshot, manifest))
}

/// Durable serial accounting. Every transition is published before the next child action.
pub struct SuiteAccounting {
    root: std::path::PathBuf,
    publisher: crate::ci::evidence::EvidencePublisher,
    manifest: SuiteManifest,
    started: std::time::Instant,
    /// Elapsed time this process did not measure, carried from a journal it
    /// took over. Suite duration has to keep rising across the handover, and
    /// the earlier run's clock is gone with the process that read it.
    carried_ms: u64,
    member_started: Option<std::time::Instant>,
    budget: uqm_rust::automation::native_artifacts::ArtifactBudget,
    diagnostics: std::collections::BTreeMap<String, u64>,
    /// The controller's account of each scenario child it supervised. A slot
    /// stays empty when this process never supervised a child for that row.
    teardown: Vec<Option<ControllerSupervision>>,
}

impl SuiteAccounting {
    #[cfg(test)]
    pub fn create(
        root: &Path,
        selected: &[PinnedScript],
        precreated: bool,
    ) -> Result<Self, String> {
        let authority: Authority = serde_json::from_slice(include_bytes!("../../ci/gates.json"))
            .map_err(|error| error.to_string())?;
        Self::create_with_limits(
            root,
            selected,
            precreated,
            authority.native_runtime_contract().inventory_limits,
        )
    }

    pub fn create_with_limits(
        root: &Path,
        selected: &[PinnedScript],
        precreated: bool,
        limits: uqm_rust::automation::native_window::NativeInventoryLimits,
    ) -> Result<Self, String> {
        let scenarios = selected
            .iter()
            .enumerate()
            .map(|(index, script)| ScenarioAccounting {
                script: script.clone(),
                directory: format!("scenarios/{index:04}"),
                state: ScenarioState::NotRun,
                elapsed_ms: 0,
            })
            .collect::<Vec<_>>();
        if selected.is_empty()
            || selected
                .iter()
                .map(|script| &script.path)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != selected.len()
        {
            return Err("native suite selection must be nonempty and unique".into());
        }
        match std::fs::create_dir(root) {
            Ok(()) => {}
            Err(error) if precreated && error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("create native suite root: {error}")),
        }
        let publisher = crate::ci::evidence::EvidencePublisher::open(root)
            .map_err(|error| error.to_string())?;
        if !crate::ci::evidence::regular_file_inventory(root)
            .map_err(|error| error.to_string())?
            .is_empty()
            || std::fs::read_dir(root)
                .map_err(|error| error.to_string())?
                .next()
                .is_some()
        {
            return Err("native suite root must be empty".into());
        }
        let request = serde_json::to_vec_pretty(selected).map_err(|error| error.to_string())?;
        let budget = uqm_rust::automation::native_artifacts::ArtifactBudget::new(limits);
        budget.reserve("suite-request.json", request.len() as u64)?;
        budget.reserve_directory("scenarios")?;
        let diagnostics =
            reserve_suite_diagnostics(&budget, limits, selected.len(), request.len())?;
        publisher
            .create("suite-request.json", &request)
            .map_err(|error| error.to_string())?;
        std::fs::create_dir(root.join("scenarios")).map_err(|error| error.to_string())?;
        let mut suite = Self {
            root: root.to_path_buf(),
            publisher,
            manifest: SuiteManifest {
                schema: "uqm-native-suite-v1".into(),
                sequence: 0,
                request: crate::retained_identity("suite-request.json", &request),
                scenarios,
                first_failure: None,
                elapsed_ms: 0,
                finalized: false,
                passed: false,
            },
            started: std::time::Instant::now(),
            carried_ms: 0,
            member_started: None,
            budget,
            diagnostics,
            teardown: vec![None; selected.len()],
        };
        suite.persist()?;
        Ok(suite)
    }

    pub fn shared_budget(&self) -> uqm_rust::automation::native_artifacts::ArtifactBudget {
        self.budget.clone()
    }

    /// Record what the controller observed of one scenario child.
    ///
    /// `stdout` and `stderr` are the child's captured streams. They are
    /// retained, bounded, only when the child did not complete: a successful
    /// child already published its own bundle, and a faulted one may have
    /// published nothing at all, which is exactly when its output is the only
    /// diagnostic left.
    pub fn record_supervision(
        &mut self,
        index: usize,
        mut supervision: ControllerSupervision,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Result<(), String> {
        if !matches!(
            self.manifest.scenarios.get(index).map(|row| &row.state),
            Some(ScenarioState::Attempted)
        ) {
            return Err("controller supervision belongs to the live attempt".into());
        }
        if self.teardown[index].is_some() {
            return Err("controller supervision was already recorded for this attempt".into());
        }
        if supervision.class != FaultClass::Completed
            && supervision.class != FaultClass::NotLaunched
        {
            supervision.stdout_log = self.retain_fault_log(index, "stdout", stdout)?;
            supervision.stderr_log = self.retain_fault_log(index, "stderr", stderr)?;
        }
        if !supervision.self_consistent() {
            return Err(format!(
                "controller supervision for scenario {index} contradicts the fault class it \
                 declares: {supervision:?}"
            ));
        }
        self.teardown[index] = Some(supervision);
        Ok(())
    }

    fn retain_fault_log(
        &self,
        index: usize,
        stream: &str,
        bytes: &[u8],
    ) -> Result<Option<uqm_rust::automation::NativeRetainedInput>, String> {
        if bytes.is_empty() {
            return Ok(None);
        }
        let relative = fault_log_relative(index, stream);
        let bound = usize::try_from(FAULT_LOG_LIMIT).map_err(|error| error.to_string())?;
        let retained = &bytes[..bytes.len().min(bound)];
        self.publish(&relative, retained)
            .map_err(|error| format!("publish {relative}: {error}"))?;
        Ok(Some(crate::retained_identity(&relative, retained)))
    }

    fn publish(&self, name: &str, bytes: &[u8]) -> std::io::Result<()> {
        let bound = self
            .diagnostics
            .get(name)
            .ok_or_else(|| std::io::Error::other("unreserved suite diagnostic"))?;
        if bytes.len() as u64 > *bound {
            return Err(std::io::Error::other(
                "suite diagnostic exceeded reservation",
            ));
        }
        if name == "suite-status.json" {
            self.publisher.replace(name, bytes)
        } else {
            self.publisher.create(name, bytes)
        }
    }

    pub fn scenario_allowance(
        &self,
        index: usize,
    ) -> Result<uqm_rust::automation::native_window::NativeInventoryLimits, String> {
        let row = self
            .manifest
            .scenarios
            .get(index)
            .ok_or("scenario outside selection")?;
        if !matches!(row.state, ScenarioState::Attempted) {
            return Err("allowance requires the current serial attempt".into());
        }
        let mut limits = self.budget.remaining()?;
        limits.member_bytes = limits.member_bytes.min(limits.aggregate_bytes);
        let prefix = row.directory.len() as u64 + 1;
        limits.path_bytes = limits
            .path_bytes
            .checked_sub(prefix as u32)
            .ok_or("suite path capacity exhausted")?;
        limits.aggregate_path_bytes = limits
            .aggregate_path_bytes
            .checked_sub(prefix * u64::from(limits.member_count))
            .ok_or("suite path capacity exhausted")?;
        if !limits.is_valid() {
            return Err("suite evidence capacity exhausted before launch".into());
        }
        Ok(limits)
    }

    pub fn account_scenario(&self, index: usize) -> Result<(), String> {
        let directory = self.directory(index)?;
        match std::fs::symlink_metadata(&directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Ok(metadata) if metadata.is_dir() => {}
            _ => return Err("scenario output is not a readable directory".into()),
        }
        let mut pending = vec![directory];
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).map_err(|error| error.to_string())? {
                let path = entry.map_err(|error| error.to_string())?.path();
                let metadata =
                    std::fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
                let relative = path
                    .strip_prefix(&self.root)
                    .map_err(|error| error.to_string())?
                    .to_str()
                    .ok_or("non-UTF8 scenario output")?;
                if metadata.is_dir() {
                    self.budget.reserve_directory(relative)?;
                    pending.push(path);
                } else if metadata.is_file() {
                    self.budget.reserve(relative, metadata.len())?;
                } else {
                    return Err("scenario output contains a nonregular member".into());
                }
            }
        }
        Ok(())
    }

    fn persist(&mut self) -> Result<(), String> {
        self.manifest.elapsed_ms = self
            .carried_ms
            .checked_add(millis(self.started.elapsed())?)
            .ok_or("native suite elapsed milliseconds overflow")?;
        let bytes = serde_json::to_vec_pretty(&self.manifest).map_err(|error| error.to_string())?;
        self.publish(
            &format!("suite-events/{:06}.json", self.manifest.sequence),
            &bytes,
        )
        .map_err(|error| error.to_string())?;
        self.publish("suite-status.json", &bytes)
            .map_err(|error| error.to_string())?;
        self.manifest.sequence = self
            .manifest
            .sequence
            .checked_add(1)
            .ok_or("suite sequence overflow")?;
        Ok(())
    }

    pub fn directory(&self, index: usize) -> Result<std::path::PathBuf, String> {
        self.manifest
            .scenarios
            .get(index)
            .map(|row| self.root.join(&row.directory))
            .ok_or_else(|| "native suite scenario index is outside selection".into())
    }

    pub fn begin(&mut self, index: usize) -> Result<(), String> {
        if self.manifest.finalized
            || self.manifest.first_failure.is_some()
            || self
                .manifest
                .scenarios
                .iter()
                .position(|row| !matches!(row.state, ScenarioState::Completed { .. }))
                != Some(index)
            || !matches!(self.manifest.scenarios[index].state, ScenarioState::NotRun)
        {
            return Err("native suite attempt violates serial selection order".into());
        }
        self.budget
            .reserve_directory(&self.manifest.scenarios[index].directory)?;
        self.manifest.scenarios[index].state = ScenarioState::Attempted;
        self.member_started = Some(std::time::Instant::now());
        self.persist()
    }

    pub fn complete(&mut self, index: usize) -> Result<(), String> {
        let row = self
            .manifest
            .scenarios
            .get(index)
            .ok_or("scenario is outside selection")?;
        if self.manifest.finalized
            || self.manifest.first_failure.is_some()
            || !matches!(row.state, ScenarioState::Attempted)
        {
            return Err("native suite completion requires a live attempt".into());
        }
        let relative = format!("{}/native-acceptance.json", row.directory);
        let bytes = crate::ci::evidence::read_regular_relative(&self.root, &relative)
            .map_err(|error| error.to_string())?;
        let proof: uqm_rust::automation::NativeAcceptanceManifest = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse native scenario proof: {error}"))?;
        validate_script_identity(&row.script, &proof.script)?;
        uqm_rust::automation::validate_native_acceptance_bundle(&self.directory(index)?, &proof)
            .map_err(|error| format!("validate native scenario proof: {error:?}"))?;
        self.manifest.scenarios[index].elapsed_ms = millis(
            self.member_started
                .ok_or("missing scenario start")?
                .elapsed(),
        )?;
        self.manifest.scenarios[index].state = ScenarioState::Completed {
            manifest: crate::retained_identity(&relative, &bytes),
        };
        self.member_started = None;
        self.persist()
    }

    pub fn fail(
        &mut self,
        phase: SuitePhase,
        scenario: Option<usize>,
        detail: String,
    ) -> Result<(), String> {
        self.record_failure(phase, scenario, detail, None)
    }

    /// `retained_elapsed_ms` is the duration to record for the failed row when
    /// this process did not measure the attempt. Only recovery of an
    /// interrupted journal is in that position: the attempt began in a process
    /// that no longer exists, so its duration is whatever that process recorded
    /// rather than something this one can invent.
    fn record_failure(
        &mut self,
        phase: SuitePhase,
        scenario: Option<usize>,
        detail: String,
        retained_elapsed_ms: Option<u64>,
    ) -> Result<(), String> {
        if self.manifest.finalized || self.manifest.first_failure.is_some() || detail.is_empty() {
            return Err("native suite failure must identify its first contract".into());
        }
        if let Some(index) = scenario {
            let elapsed_ms = match retained_elapsed_ms {
                Some(retained) => retained,
                None => millis(
                    self.member_started
                        .ok_or("missing scenario start")?
                        .elapsed(),
                )?,
            };
            let row = self
                .manifest
                .scenarios
                .get_mut(index)
                .ok_or("failed scenario is outside selection")?;
            if !matches!(row.state, ScenarioState::Attempted) {
                return Err("native suite failed row must have been attempted".into());
            }
            row.state = ScenarioState::Failed {
                detail: detail.clone(),
            };
            row.elapsed_ms = elapsed_ms;
            self.member_started = None;
        } else if self.member_started.is_some() {
            return Err("active attempt requires a scenario-bound failure".into());
        }
        self.manifest.first_failure = Some(SuiteFailure {
            phase,
            scenario,
            detail,
        });
        self.persist()
    }

    pub fn finish(&mut self) -> Result<(), String> {
        if self.manifest.finalized
            || self
                .manifest
                .scenarios
                .iter()
                .any(|row| matches!(row.state, ScenarioState::Attempted))
        {
            return Err("native suite cannot finalize an active attempt or finalized run".into());
        }
        self.manifest.passed = self
            .manifest
            .scenarios
            .iter()
            .all(|row| matches!(row.state, ScenarioState::Completed { .. }))
            && self.manifest.first_failure.is_none();
        if !self.manifest.passed && self.manifest.first_failure.is_none() {
            return Err("native suite cannot silently omit selected scenarios".into());
        }
        self.manifest.finalized = true;
        self.persist()?;
        // The final manifest is the final immutable event, not another transition.
        let bytes = crate::ci::evidence::read_regular_relative(&self.root, "suite-status.json")
            .map_err(|error| error.to_string())?;
        self.publish("suite-manifest.json", &bytes)
            .map_err(|error| error.to_string())?;
        // The teardown receipt is published before the index so the index
        // covers it: a bundle cannot carry a teardown claim the inventory does
        // not account for.
        self.publish(
            TEARDOWN_RELATIVE,
            &serde_json::to_vec_pretty(&SuiteTeardown::build(&self.manifest, &self.teardown))
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let images = gallery_images(&self.manifest, &inventory(&self.root)?);
        self.publish(
            "suite-gallery.json",
            &serde_json::to_vec_pretty(&images).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        self.publish(
            "suite-gallery.html",
            &gallery_html(&self.manifest, &images)?,
        )
        .map_err(|error| error.to_string())?;
        let index = SuiteIndex {
            schema: "uqm-native-suite-index-v1".into(),
            passed: self.manifest.passed,
            files: inventory(&self.root)?,
        };
        self.publish(
            "suite-index.json",
            &serde_json::to_vec_pretty(&index).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn reserve_suite_diagnostics(
    budget: &uqm_rust::automation::native_artifacts::ArtifactBudget,
    limits: uqm_rust::automation::native_window::NativeInventoryLimits,
    scenarios: usize,
    request_bytes: usize,
) -> Result<std::collections::BTreeMap<String, u64>, String> {
    let event_bound = (request_bytes as u64)
        .checked_add(scenarios as u64 * 1024 + 16384)
        .ok_or("suite diagnostic size overflow")?;
    let index_bound = limits
        .aggregate_path_bytes
        .checked_add(u64::from(limits.member_count) * 256 + 65536)
        .ok_or("suite index size overflow")?;
    let mut reserved = std::collections::BTreeMap::new();
    for sequence in 0..(scenarios * 2 + 3) {
        reserved.insert(format!("suite-events/{sequence:06}.json"), event_bound);
    }
    for name in ["suite-status.json", "suite-manifest.json"] {
        reserved.insert(name.into(), event_bound);
    }
    for name in [
        "suite-index.json",
        "suite-gallery.json",
        "suite-gallery.html",
    ] {
        reserved.insert(name.into(), index_bound);
    }
    // The teardown receipt and the faulted children's bounded logs are
    // reserved with the rest of the diagnostics, before any scenario is
    // allowed to spend the budget. A crash is exactly when the storage is
    // already under pressure, so the space to describe it is claimed first.
    reserved.insert(
        TEARDOWN_RELATIVE.into(),
        (scenarios as u64)
            .checked_mul(2048)
            .and_then(|bytes| bytes.checked_add(16384))
            .ok_or("suite teardown size overflow")?,
    );
    for scenario in 0..scenarios {
        for stream in ["stdout", "stderr"] {
            reserved.insert(fault_log_relative(scenario, stream), FAULT_LOG_LIMIT);
        }
    }
    for (name, bytes) in &reserved {
        budget.reserve(name, *bytes)?;
    }
    Ok(reserved)
}

/// Where a faulted child's bounded captured stream is retained.
///
/// This lives outside `scenarios/`, because a scenario directory's inventory
/// is bound to the manifest the child published for itself; adding controller
/// output to it would invalidate the child's own proof.
fn fault_log_relative(scenario: usize, stream: &str) -> String {
    format!("suite-faults/{scenario:04}.{stream}.log")
}

fn millis(duration: std::time::Duration) -> Result<u64, String> {
    u64::try_from(duration.as_millis())
        .map_err(|_| "native suite elapsed milliseconds overflow".into())
}

fn inventory(root: &Path) -> Result<Vec<uqm_rust::automation::NativeRetainedInput>, String> {
    Ok(crate::ci::evidence::regular_file_inventory(root)
        .map_err(|error| error.to_string())?
        .iter()
        .filter(|file| file.relative_path != "suite-index.json")
        .map(|file| crate::retained_identity(&file.relative_path, &file.bytes))
        .collect())
}

/// Validate exact selection, durable transitions and the complete bounded offline index.
/// A valid failed suite remains failed; this function does not convert diagnostics to proof.
pub fn validate_accounting(
    root: &Path,
    selected: &[PinnedScript],
) -> Result<SuiteManifest, String> {
    if selected.is_empty()
        || selected
            .iter()
            .map(|script| &script.path)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != selected.len()
    {
        return Err("native suite expected selection must be nonempty and unique".into());
    }
    let snapshot =
        crate::ci::evidence::EvidenceSnapshot::open(root).map_err(|error| error.to_string())?;
    snapshot.scoped(|| {
        let read = |path: &str| {
            crate::ci::evidence::read_regular_relative(root, path)
                .map_err(|error| error.to_string())
        };
        let manifest: SuiteManifest = serde_json::from_slice(&read("suite-manifest.json")?)
            .map_err(|error| error.to_string())?;
        let index: SuiteIndex = serde_json::from_slice(&read("suite-index.json")?)
            .map_err(|error| error.to_string())?;
        let request = read("suite-request.json")?;
        let expected_request =
            serde_json::to_vec_pretty(selected).map_err(|error| error.to_string())?;
        if manifest.schema != "uqm-native-suite-v1"
            || index.schema != "uqm-native-suite-index-v1"
            || !manifest.finalized
            || request != expected_request
            || manifest.request != crate::retained_identity("suite-request.json", &request)
            || manifest
                .scenarios
                .iter()
                .map(|row| row.script.clone())
                .collect::<Vec<_>>()
                != selected
            || index.passed != manifest.passed
            || index.files != inventory(root)?
        {
            return Err("native suite selection, schema or complete inventory mismatch".into());
        }
        validate_outcome(&manifest)?;
        let images = gallery_images(&manifest, &index.files);
        if read("suite-gallery.json")?
            != serde_json::to_vec_pretty(&images).map_err(|error| error.to_string())?
            || read("suite-gallery.html")? != gallery_html(&manifest, &images)?
        {
            return Err(
                "native suite gallery disagrees with indexed image origins or outcomes".into(),
            );
        }
        validate_teardown(&snapshot, &manifest)?;
        validate_retained_journal(&snapshot, &manifest, &index)?;
        validate_retained_scenarios(&snapshot, &manifest)?;
        Ok(manifest)
    })
}

fn validate_retained_journal(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
    manifest: &SuiteManifest,
    index: &SuiteIndex,
) -> Result<(), String> {
    let read = |path: &str| snapshot.read(path).map_err(|error| error.to_string());
    let final_bytes = read("suite-manifest.json")?;
    if read("suite-status.json")? != final_bytes
        || read(&format!("suite-events/{:06}.json", manifest.sequence))? != final_bytes
    {
        return Err("native suite final event/status disagrees with manifest".into());
    }
    let event_paths = index
        .files
        .iter()
        .filter(|file| file.relative_path.starts_with("suite-events/"))
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    if manifest.sequence >= index.files.len() as u64
        || event_paths
            != (0..=manifest.sequence)
                .map(|sequence| format!("suite-events/{sequence:06}.json"))
                .collect::<Vec<_>>()
    {
        return Err("native suite journal member set differs from its sequence".into());
    }
    let mut events = Vec::new();
    for sequence in 0..=manifest.sequence {
        let event: SuiteManifest =
            serde_json::from_slice(read(&format!("suite-events/{sequence:06}.json"))?)
                .map_err(|error| error.to_string())?;
        if event.sequence != sequence
            || event.request != manifest.request
            || event
                .scenarios
                .iter()
                .map(|row| &row.script)
                .ne(manifest.scenarios.iter().map(|row| &row.script))
        {
            return Err("native suite journal selection or sequence drift".into());
        }
        events.push(event);
    }
    validate_journal(&events)
}

fn validate_retained_scenarios(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
    manifest: &SuiteManifest,
) -> Result<(), String> {
    for file in snapshot
        .files()
        .iter()
        .filter(|file| file.relative_path.starts_with("scenarios/"))
    {
        let row = manifest
            .scenarios
            .iter()
            .find(|row| {
                file.relative_path
                    .starts_with(&format!("{}/", row.directory))
            })
            .ok_or("native suite retains output for an unselected scenario")?;
        if matches!(row.state, ScenarioState::NotRun) {
            return Err("native suite not-run scenario has retained output".into());
        }
    }
    for (number, row) in manifest.scenarios.iter().enumerate() {
        if row.directory != format!("scenarios/{number:04}") {
            return Err("native suite scenario directory does not match selection".into());
        }
        if let ScenarioState::Completed { manifest: proof } = &row.state {
            let relative = format!("{}/native-acceptance.json", row.directory);
            let bytes = snapshot
                .read(&relative)
                .map_err(|error| error.to_string())?;
            if *proof != crate::retained_identity(&relative, bytes) {
                return Err("completed native scenario manifest identity mismatch".into());
            }
            let native: uqm_rust::automation::NativeAcceptanceManifest =
                serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
            validate_script_identity(&row.script, &native.script)?;
            validate_completed_snapshot(snapshot, &row.directory)?;
        }
    }
    Ok(())
}

fn validate_completed_snapshot(
    snapshot: &crate::ci::evidence::EvidenceSnapshot,
    directory: &str,
) -> Result<(), String> {
    let temporary = materialize_member(snapshot, directory)?;
    let proof = serde_json::from_slice(
        snapshot
            .read(&format!("{directory}/native-acceptance.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    uqm_rust::automation::validate_native_acceptance_bundle(
        &temporary.path().join(directory),
        &proof,
    )
    .map_err(|error| format!("offline native scenario proof: {error:?}"))
}

fn validate_outcome(manifest: &SuiteManifest) -> Result<(), String> {
    let mut incomplete = false;
    for (number, row) in manifest.scenarios.iter().enumerate() {
        match &row.state {
            ScenarioState::Completed { .. } if !incomplete => {}
            ScenarioState::NotRun => incomplete = true,
            ScenarioState::Failed { detail }
                if !incomplete
                    && manifest.first_failure.as_ref().is_some_and(|failure| {
                        failure.scenario == Some(number) && failure.detail == *detail
                    }) =>
            {
                incomplete = true
            }
            _ => return Err("native suite has an invalid terminal selection order".into()),
        }
    }
    if manifest.passed != (!incomplete && manifest.first_failure.is_none())
        || (!manifest.passed && manifest.first_failure.is_none())
    {
        return Err("native suite outcome contradicts selection accounting".into());
    }
    if let Some(failure) = &manifest.first_failure {
        if failure.detail.is_empty()
            || failure.scenario.is_some_and(|index| {
                !manifest
                    .scenarios
                    .get(index)
                    .is_some_and(|row| matches!(row.state, ScenarioState::Failed { .. }))
            })
        {
            return Err("native suite first failure contradicts failed row".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(test)]
    mod accounting_tests {
        use super::*;

        fn selection() -> Vec<PinnedScript> {
            ["a", "b", "c"]
                .map(|id| PinnedScript {
                    path: format!("rust/scripts/{id}.json"),
                    sha256: "a".repeat(64),
                    byte_length: 12,
                })
                .to_vec()
        }

        #[test]
        fn failed_member_preserves_exact_selection_and_unattempted_suffix_offline() {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("suite");
            let selected = selection();
            let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
            suite.begin(0).unwrap();
            suite
                .fail(SuitePhase::Execute, Some(0), "observer failed".into())
                .unwrap();
            suite.finish().unwrap();
            let manifest = validate_accounting(&root, &selected).unwrap();
            assert!(matches!(
                manifest.scenarios[0].state,
                ScenarioState::Failed { .. }
            ));
            assert!(manifest.scenarios[1..]
                .iter()
                .all(|row| matches!(row.state, ScenarioState::NotRun)));
            assert!(!manifest.passed);
            assert!(validate_accounting(&root, &selected[..2]).is_err());
            std::fs::write(root.join("unindexed.txt"), b"drift").unwrap();
            assert!(validate_accounting(&root, &selected).is_err());
        }

        #[test]
        fn preflight_failure_is_durable_and_reentry_cannot_overwrite_it() {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("suite");
            let selected = selection();
            let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
            suite
                .fail(SuitePhase::Preflight, None, "digest mismatch".into())
                .unwrap();
            suite.finish().unwrap();
            let before = std::fs::read(root.join("suite-manifest.json")).unwrap();
            assert!(SuiteAccounting::create(&root, &selected, true).is_err());
            assert_eq!(
                before,
                std::fs::read(root.join("suite-manifest.json")).unwrap()
            );
            let manifest = validate_accounting(&root, &selected).unwrap();
            assert!(manifest
                .scenarios
                .iter()
                .all(|row| matches!(row.state, ScenarioState::NotRun)));
        }

        #[test]
        fn unfinished_attempt_cannot_be_reported_as_complete_or_skipped() {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("suite");
            let selected = selection();
            let mut suite = SuiteAccounting::create(&root, &selected, false).unwrap();
            assert!(suite.begin(1).is_err());
            suite.begin(0).unwrap();
            assert!(suite.finish().is_err());
            assert!(
                suite.complete(0).is_err(),
                "child exit alone cannot complete a row"
            );
        }
    }

    #[test]
    fn all_pinned_script_watchdogs_are_admitted_without_rewriting_actions() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let authority = crate::ci::authority::load_authority(root).unwrap();
        let admitted = preflight(
            root,
            &authority.native_acceptance.scenario_scripts,
            &authority,
        )
        .unwrap();
        assert_eq!(admitted.len(), 36);
        assert_eq!(
            admitted
                .iter()
                .filter(|entry| entry.budgets.max_wallclock_seconds >= 300)
                .count(),
            15
        );
        assert!(admitted
            .iter()
            .all(|entry| entry.outer_child_timeout_ms == 300_000));
        for admission in admitted {
            let path = root.join(&admission.script.path);
            let bytes = std::fs::read(&path).unwrap();
            let script = validate_script(parse_script(&bytes, &path).unwrap(), &path).unwrap();
            assert_eq!(admission.budgets, script.budgets());
            assert_eq!(admission.action_count, script.steps().len());
        }
    }

    #[test]
    fn preflight_reports_every_bad_script_and_never_accepts_an_empty_selection() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let authority = crate::ci::authority::load_authority(root).unwrap();
        let mut scripts = authority.native_acceptance.scenario_scripts[..2].to_vec();
        for script in &mut scripts {
            script.sha256 = "0".repeat(64);
        }
        let error = preflight(root, &scripts, &authority).unwrap_err();
        for script in scripts {
            assert!(error.contains(&script.path));
        }
        assert!(preflight(root, &[], &authority)
            .unwrap_err()
            .contains("empty"));
    }
}
