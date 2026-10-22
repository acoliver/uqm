//! Extract a newly written slot from a passing native scenario.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uqm_rust::automation::native_window::NATIVE_ACCEPTANCE_SCHEMA;
use uqm_rust::automation::{parse_script, validate_script, NativeRetainedInput, StartSave};

use crate::native_suite::{ScenarioState, SuiteManifest};

const PROVENANCE_SCHEMA: &str = "uqm-start-save-provenance-v2";
const SAVE_LIMIT: u64 = 4 * 1024 * 1024;
const TRACE_LIMIT: u64 = 64 * 1024 * 1024;
const ACCEPTANCE_LIMIT: u64 = 16 * 1024 * 1024;
const MANIFEST_LIMIT: u64 = 1024 * 1024;
const SUITE_MANIFEST: &str = "suite-manifest.json";
const SUITE_SCHEMA: &str = "uqm-native-suite-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveIdentity {
    pub path: String,
    pub slot: u8,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioIdentity {
    pub name: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub schema: String,
    pub save: SaveIdentity,
    pub scenario: ScenarioIdentity,
    pub producing_commit: String,
    pub run_id: String,
    pub terminal_reason: String,
    pub seed: u32,
    pub parent_save: Option<SaveIdentity>,
    /// Values decoded from this particular output slot, not a later script assertion.
    pub campaign_state: std::collections::BTreeMap<String, u32>,
}

/// Where the provenance of `save` lives: next to it, `<stem>.provenance.json`.
pub fn provenance_path(save: &Path) -> PathBuf {
    save.with_extension("provenance.json")
}

/// What a suite bundle's scenario records about itself that the extraction
/// relies on. Unrelated fields of the acceptance manifest are not read.
#[derive(Deserialize)]
struct RecordedRun {
    schema: String,
    script: RecordedScript,
    retained_files: Vec<NativeRetainedInput>,
    trace_path: String,
    trace_sha256: String,
    passed: bool,
}

#[derive(Deserialize)]
struct RecordedScript {
    sha256: String,
}

/// One selected scenario from a finalized, passing native suite.
struct ProducingRun {
    scenario_dir: PathBuf,
    run_id: String,
    recorded: RecordedRun,
}

/// The suite root and the scenario directory (relative to it) named by
/// `--artifacts`, which is either the suite root or one `scenarios/NNNN` dir.
fn locate_suite(artifacts: &Path) -> Result<(PathBuf, Option<String>), String> {
    let artifacts = std::fs::canonicalize(artifacts)
        .map_err(|error| format!("resolve {}: {error}", artifacts.display()))?;
    if artifacts.join(SUITE_MANIFEST).is_file() {
        return Ok((artifacts, None));
    }
    let scenarios = artifacts
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == "scenarios"));
    let suite = scenarios.and_then(Path::parent);
    match (suite, artifacts.file_name()) {
        (Some(suite), Some(name)) if suite.join(SUITE_MANIFEST).is_file() => Ok((
            suite.to_path_buf(),
            Some(format!("scenarios/{}", name.to_string_lossy())),
        )),
        _ => Err(format!(
            "{} is neither a native suite bundle nor a scenario directory of one \
             (no {SUITE_MANIFEST} at or two levels above it)",
            artifacts.display()
        )),
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, limit: u64) -> Result<T, String> {
    let bytes = crate::read_regular_file_bounded(path, limit)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn require_digest(
    what: &str,
    bytes: &[u8],
    sha256: &str,
    byte_length: Option<u64>,
) -> Result<(), String> {
    if crate::hex_sha256(bytes) != sha256 || byte_length.is_some_and(|n| n != bytes.len() as u64) {
        return Err(format!("{what} differs from the digest the run recorded"));
    }
    Ok(())
}

/// Establish that `artifacts` contains the requested passing scenario.
fn verify_run(artifacts: &Path, scenario: &str) -> Result<ProducingRun, String> {
    let (suite, requested) = locate_suite(artifacts)?;
    let manifest: SuiteManifest = read_json(&suite.join(SUITE_MANIFEST), MANIFEST_LIMIT)?;
    if manifest.schema != SUITE_SCHEMA || !manifest.finalized || !manifest.passed {
        return Err(format!(
            "{} is not a finalized passing native suite",
            suite.display()
        ));
    }
    let request = crate::read_regular_file_bounded(
        &suite.join(&manifest.request.relative_path),
        MANIFEST_LIMIT,
    )?;
    require_digest(
        "suite request",
        &request,
        &manifest.request.sha256,
        Some(manifest.request.byte_length),
    )?;
    let script_path = format!("rust/scripts/{scenario}.json");
    let mut rows = manifest
        .scenarios
        .iter()
        .filter(|row| row.script.path == script_path);
    let row = rows
        .next()
        .ok_or_else(|| format!("suite did not run {script_path}"))?;
    if rows.next().is_some() {
        return Err(format!("suite has more than one {scenario} run"));
    }
    if requested.is_some_and(|directory| directory != row.directory) {
        return Err(format!(
            "{} is not the {scenario} scenario of its suite ({})",
            artifacts.display(),
            row.directory
        ));
    }
    let ScenarioState::Completed { manifest: record } = &row.state else {
        return Err(format!("the {scenario} scenario did not complete"));
    };
    let scenario_dir = suite.join(&row.directory);
    let acceptance_bytes = crate::read_regular_file_bounded(
        &scenario_dir.join(
            record
                .relative_path
                .strip_prefix(&format!("{}/", row.directory))
                .ok_or("scenario manifest is recorded outside its scenario directory")?,
        ),
        ACCEPTANCE_LIMIT,
    )?;
    require_digest(
        "native-acceptance.json",
        &acceptance_bytes,
        &record.sha256,
        Some(record.byte_length),
    )?;
    let recorded: RecordedRun = serde_json::from_slice(&acceptance_bytes)
        .map_err(|error| format!("parse native-acceptance.json: {error}"))?;
    if recorded.schema != NATIVE_ACCEPTANCE_SCHEMA {
        return Err(format!(
            "native-acceptance.json has schema {}, expected {NATIVE_ACCEPTANCE_SCHEMA}",
            recorded.schema
        ));
    }
    if !recorded.passed {
        return Err(format!("the {scenario} scenario did not pass"));
    }
    let run_id = format!(
        "{}/{}",
        suite
            .file_name()
            .ok_or("suite directory has no name")?
            .to_string_lossy(),
        row.directory
    );
    Ok(ProducingRun {
        scenario_dir,
        run_id,
        recorded,
    })
}

/// The `terminal_reason` of the run's last trace record, with the trace itself
/// checked against the digest the run recorded.
fn terminal_reason(run: &ProducingRun) -> Result<String, String> {
    let trace = crate::read_regular_file_bounded(
        &run.scenario_dir.join(&run.recorded.trace_path),
        TRACE_LIMIT,
    )?;
    require_digest("run trace", &trace, &run.recorded.trace_sha256, None)?;
    let last = trace
        .split(|byte| *byte == b'\n')
        .rfind(|line| !line.is_empty())
        .ok_or("run trace is empty")?;
    let record: serde_json::Value = serde_json::from_slice(last)
        .map_err(|error| format!("parse last trace record: {error}"))?;
    record["terminal_reason"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "run trace ends without a terminal_reason".to_string())
}

/// Copy a newly written slot out of a passing selected run and record its state.
///
/// `artifacts` is the suite bundle root or its `scenarios/NNNN` directory.
pub fn extract_save(
    root: &Path,
    artifacts: &Path,
    scenario: &str,
    slot: u8,
    out: &Path,
) -> Result<(), String> {
    if scenario.is_empty()
        || !scenario
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("--scenario must be one scenario name".into());
    }
    if slot >= 50 {
        return Err("--slot must be in 0..50".into());
    }
    let run = verify_run(artifacts, scenario)?;
    let reason = terminal_reason(&run)?;
    if reason != "success" {
        return Err(format!(
            "{} ended with terminal_reason {reason}, not success; refusing to extract a save",
            run.run_id
        ));
    }
    let script_path = format!("rust/scripts/{scenario}.json");
    let script_bytes = crate::read_regular_file_bounded(&root.join(&script_path), SAVE_LIMIT)?;
    let script_sha = crate::hex_sha256(&script_bytes);
    if script_sha != run.recorded.script.sha256 {
        return Err(format!(
            "{script_path} in this checkout ({script_sha}) is not the script the run played ({})",
            run.recorded.script.sha256
        ));
    }
    let document =
        parse_script(&script_bytes, Path::new(&script_path)).map_err(|error| error.to_string())?;
    let script =
        validate_script(document, Path::new(&script_path)).map_err(|error| error.to_string())?;
    let slot_file = format!("uqmsave.{slot:02}");
    let save_relative = format!("config-final/save/{slot_file}");
    let save_bytes =
        crate::read_regular_file_bounded(&run.scenario_dir.join(&save_relative), SAVE_LIMIT)?;
    let save_identity = recorded_file(&run, &save_relative, &save_bytes)?;
    let parent_save = script.start_save().map(|parent| -> Result<SaveIdentity, String> {
        let bytes = crate::read_regular_file_bounded(&run.scenario_dir.join("inputs/start-save.sav"), SAVE_LIMIT)?;
        let retained = recorded_file(&run, "inputs/start-save.sav", &bytes)?;
        if slot == parent.slot && bytes == save_bytes {
            return Err("selected slot is unchanged from the installed parent save; no new save was produced".into());
        }
        Ok(SaveIdentity {
            path: parent.path.clone(), slot: parent.slot,
            sha256: retained.sha256.clone(), byte_length: retained.byte_length,
        })
    }).transpose()?;
    let campaign_state = saved_campaign_state(&save_bytes)?;
    let commit = producing_commit(&run)?;
    let relative_out = out
        .strip_prefix(root)
        .map_err(|_| format!("--output {} must be inside the repository", out.display()))?
        .to_string_lossy()
        .into_owned();
    let provenance = Provenance {
        schema: PROVENANCE_SCHEMA.into(),
        save: SaveIdentity {
            path: relative_out,
            slot,
            sha256: save_identity.sha256.clone(),
            byte_length: save_identity.byte_length,
        },
        scenario: ScenarioIdentity {
            name: scenario.into(),
            path: script_path,
            sha256: script_sha,
        },
        producing_commit: commit,
        run_id: run.run_id,
        terminal_reason: reason,
        seed: script.seed(),
        parent_save,
        campaign_state,
    };
    std::fs::write(out, &save_bytes)
        .map_err(|error| format!("write {}: {error}", out.display()))?;
    let mut text = serde_json::to_string_pretty(&provenance).map_err(|error| error.to_string())?;
    text.push('\n');
    let provenance_file = provenance_path(out);
    std::fs::write(&provenance_file, text)
        .map_err(|error| format!("write {}: {error}", provenance_file.display()))?;
    println!("saved {} and {}", out.display(), provenance_file.display());
    Ok(())
}

fn recorded_file<'a>(
    run: &'a ProducingRun,
    relative: &str,
    bytes: &[u8],
) -> Result<&'a NativeRetainedInput, String> {
    let retained = run
        .recorded
        .retained_files
        .iter()
        .find(|file| file.relative_path == relative)
        .ok_or_else(|| format!("the run did not retain {relative}"))?;
    require_digest(
        relative,
        bytes,
        &retained.sha256,
        Some(retained.byte_length),
    )?;
    Ok(retained)
}

fn saved_campaign_state(bytes: &[u8]) -> Result<std::collections::BTreeMap<String, u32>, String> {
    use uqm_rust::savegame::{
        decode_save,
        runtime::star::{resolve_star, LoadedStar, CATALOG_LEN},
    };
    use uqm_rust::state::{game_state::get_state_bits_raw, game_state_keys::bit_range};
    let save = decode_save(bytes).map_err(|error| format!("decode produced save: {error}"))?;
    let story = |name: &str| {
        let (start, end) = bit_range(name).expect("known story key");
        u32::from(get_state_bits_raw(
            &save.game_state,
            start as usize,
            end as usize,
        ))
    };
    // The catalog is immutable game data; this process never runs the game.
    let catalog = unsafe {
        std::slice::from_raw_parts(
            std::ptr::addr_of!(uqm_rust::planets::tables::STARMAP_ARRAY).cast(),
            CATALOG_LEN,
        )
    };
    let star = resolve_star(catalog, story("ARILOU_SPACE_SIDE") as u8, &save.star)
        .map_err(|error| format!("resolve produced star: {error}"))?;
    let mut state = std::collections::BTreeMap::from([
        ("activity".into(), u32::from(save.global.current_activity)),
        ("sis:fuel".into(), save.summary.sis.fuel_on_board),
        ("sis:ru".into(), save.summary.sis.res_units),
        ("sis:crew".into(), u32::from(save.summary.sis.crew_enlisted)),
        (
            "sis:landers".into(),
            u32::from(save.summary.sis.num_landers),
        ),
        ("date:day".into(), u32::from(save.global.clock.day_index)),
        (
            "date:month".into(),
            u32::from(save.global.clock.month_index),
        ),
        ("date:year".into(), u32::from(save.global.clock.year_index)),
    ]);
    if let LoadedStar::Catalog(index) = star {
        state.insert("star:current".into(), index as u32);
    }
    for name in [
        "RADIOACTIVES_PROVIDED",
        "MOONBASE_DESTROYED",
        "STARBASE_AVAILABLE",
        "PROBE_ILWRATH_ENCOUNTER",
    ] {
        state.insert(format!("story:{name}"), story(name));
    }
    Ok(state)
}

fn producing_commit(run: &ProducingRun) -> Result<String, String> {
    const RECEIPT: &str = "inputs/linked-build/linked-build-receipt.json";
    let bytes = match uqm_rust::automation::native_artifacts::SharedInputs::open(
        &run.scenario_dir,
        MANIFEST_LIMIT,
    )? {
        Some(shared) => shared.read(&run.scenario_dir, RECEIPT, MANIFEST_LIMIT)?,
        None => {
            let bytes =
                crate::read_regular_file_bounded(&run.scenario_dir.join(RECEIPT), MANIFEST_LIMIT)?;
            recorded_file(run, RECEIPT, &bytes)?;
            bytes
        }
    };
    #[derive(Deserialize)]
    struct Source {
        source_sha: String,
    }
    let source: Source = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if source.source_sha.len() != 40
        || !source
            .source_sha
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("invalid producer source commit in linked build receipt".into());
    }
    Ok(source.source_sha)
}

/// Admit the save a script starts with: it must be a regular file next to
/// the script.
pub fn verify_start_save(
    root: &Path,
    script_path: &str,
    start_save: &StartSave,
) -> Result<(), String> {
    let relative = Path::new(script_path)
        .parent()
        .ok_or("script path has no directory")?
        .join(&start_save.path);
    let full = root.join(&relative);
    if !full.exists() {
        return Err(format!(
            "start save missing: {} (script {script_path} starts from it); regenerate with \
             `xtask native save-extract --scenario <producer> --artifacts <passing bundle> --slot <N> --output {}`",
            relative.display(),
            relative.display()
        ));
    }
    crate::read_regular_file_bounded(&full, SAVE_LIMIT).map_err(|error| {
        format!(
            "script {script_path} starts from {}: {error}",
            relative.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_config::scenarios;

    fn repository_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
    }

    /// Every script that starts from a save must have that save present.
    #[test]
    fn every_start_save_is_present() {
        let root = repository_root();
        for pin in &scenarios(root).unwrap() {
            let path = root.join(&pin.path);
            let bytes = std::fs::read(&path).unwrap();
            let document = parse_script(&bytes, &path).unwrap();
            let Some(start_save) = document.start_save else {
                continue;
            };
            verify_start_save(root, &pin.path, &start_save)
                .unwrap_or_else(|error| panic!("{}: {error}", pin.path));
        }
    }

    const PRODUCER_SCENARIO: &str = "real-sol-starbase-alliance";
    const PRODUCER_SCRIPT: &str = "rust/scripts/real-sol-starbase-alliance.json";
    const SAVE_BYTES: &[u8] = include_bytes!("../../scripts/saves/allied-starbase.sav");

    fn producer_scenario() -> crate::native_config::Scenario {
        crate::native_config::scenarios(repository_root())
            .unwrap()
            .into_iter()
            .find(|scenario| scenario.path == PRODUCER_SCRIPT)
            .unwrap()
    }

    /// A suite bundle laid out like a real run of the alliance script.
    struct Bundle {
        directory: tempfile::TempDir,
    }

    impl Bundle {
        fn new(terminal_reason: &str) -> Self {
            let pinned = producer_scenario();
            let script_bytes = std::fs::read(repository_root().join(PRODUCER_SCRIPT)).unwrap();
            let directory = tempfile::tempdir().unwrap();
            let suite = directory.path().join("art-real-sol-starbase-alliance");
            let scenario = suite.join("scenarios/0000");
            std::fs::create_dir_all(scenario.join("automation")).unwrap();
            std::fs::create_dir_all(scenario.join("config-final/save")).unwrap();
            let trace =
                format!("{{\"event\":\"start\"}}\n{{\"terminal_reason\":\"{terminal_reason}\"}}\n");
            std::fs::write(scenario.join("automation/trace.jsonl"), &trace).unwrap();
            std::fs::write(scenario.join("config-final/save/uqmsave.00"), SAVE_BYTES).unwrap();
            std::fs::create_dir_all(scenario.join("inputs/linked-build")).unwrap();
            let receipt = br#"{"source_sha":"0123456789012345678901234567890123456789"}"#;
            std::fs::write(
                scenario.join("inputs/linked-build/linked-build-receipt.json"),
                receipt,
            )
            .unwrap();
            let acceptance = serde_json::to_vec_pretty(&serde_json::json!({
                "schema": NATIVE_ACCEPTANCE_SCHEMA,
                "passed": true,
                "script": {
                    "relative_path": "inputs/real-sol-starbase-alliance.json",
                    "byte_length": script_bytes.len(),
                    "sha256": crate::hex_sha256(&script_bytes),
                },
                "retained_files": [
                    crate::retained_identity("inputs/linked-build/linked-build-receipt.json", receipt),
                    crate::retained_identity("config-final/save/uqmsave.00", SAVE_BYTES),
                ],
                "trace_path": "automation/trace.jsonl",
                "trace_sha256": crate::hex_sha256(trace.as_bytes()),
            }))
            .unwrap();
            std::fs::write(scenario.join("native-acceptance.json"), &acceptance).unwrap();
            let request = serde_json::to_vec_pretty(&[&pinned]).unwrap();
            std::fs::write(suite.join("suite-request.json"), &request).unwrap();
            let bundle = Self { directory };
            bundle.write_manifest(|_| {});
            bundle
        }

        fn suite(&self) -> PathBuf {
            self.directory.path().join("art-real-sol-starbase-alliance")
        }

        fn scenario(&self) -> PathBuf {
            self.suite().join("scenarios/0000")
        }

        fn use_script(&self, name: &str) {
            let path = format!("rust/scripts/{name}.json");
            let script = std::fs::read(repository_root().join(&path)).unwrap();
            let acceptance_path = self.scenario().join("native-acceptance.json");
            let mut acceptance: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&acceptance_path).unwrap()).unwrap();
            acceptance["script"]["sha256"] = crate::hex_sha256(&script).into();
            let parent =
                std::fs::read(repository_root().join("rust/scripts/saves/allied-starbase.sav"))
                    .unwrap();
            std::fs::write(self.scenario().join("inputs/start-save.sav"), &parent).unwrap();
            acceptance["retained_files"].as_array_mut().unwrap().push(
                serde_json::to_value(crate::retained_identity("inputs/start-save.sav", &parent))
                    .unwrap(),
            );
            std::fs::write(&acceptance_path, serde_json::to_vec(&acceptance).unwrap()).unwrap();
            self.write_manifest(|manifest| {
                manifest["scenarios"][0]["script"]["path"] = path.into()
            });
        }

        /// Rewrite `suite-manifest.json` for the bundle as it is on disk,
        /// after `edit` has changed the manifest's content.
        fn write_manifest(&self, edit: impl FnOnce(&mut serde_json::Value)) {
            let pinned = producer_scenario();
            let request = std::fs::read(self.suite().join("suite-request.json")).unwrap();
            let acceptance = std::fs::read(self.scenario().join("native-acceptance.json")).unwrap();
            let mut manifest = serde_json::json!({
                "schema": SUITE_SCHEMA,
                "sequence": 3,
                "request": crate::retained_identity("suite-request.json", &request),
                "scenarios": [{
                    "script": pinned,
                    "directory": "scenarios/0000",
                    "state": {
                        "state": "completed",
                        "manifest": crate::retained_identity(
                            "scenarios/0000/native-acceptance.json",
                            &acceptance,
                        ),
                    },
                    "elapsed_ms": 1,
                }],
                "first_failure": null,
                "elapsed_ms": 2,
                "finalized": true,
                "passed": true,
            });
            edit(&mut manifest);
            std::fs::write(
                self.suite().join("suite-manifest.json"),
                serde_json::to_vec_pretty(&manifest).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn the_run_is_identified_from_the_suite_bundle_given_either_directory() {
        let bundle = Bundle::new("success");
        let from_suite = verify_run(&bundle.suite(), PRODUCER_SCENARIO).unwrap();
        let from_scenario = verify_run(&bundle.scenario(), PRODUCER_SCENARIO).unwrap();
        assert_eq!(
            from_suite.run_id,
            "art-real-sol-starbase-alliance/scenarios/0000"
        );
        assert_eq!(from_scenario.run_id, from_suite.run_id);
        assert_eq!(terminal_reason(&from_suite).unwrap(), "success");
    }

    #[test]
    fn a_run_that_did_not_end_in_success_is_refused() {
        let bundle = Bundle::new("battle_lost");
        let run = verify_run(&bundle.scenario(), PRODUCER_SCENARIO).unwrap();
        assert_eq!(terminal_reason(&run).unwrap(), "battle_lost");
        let out = bundle.directory.path().join("out.sav");
        let error = extract_save(
            bundle.directory.path(),
            &bundle.scenario(),
            PRODUCER_SCENARIO,
            0,
            &out,
        )
        .unwrap_err();
        assert!(error.contains("not success"), "{error}");
        assert!(!out.exists());
    }

    #[test]
    fn a_run_of_another_script_is_refused() {
        let bundle = Bundle::new("success");
        bundle.write_manifest(|manifest| {
            manifest["scenarios"][0]["script"]["path"] = "rust/scripts/other.json".into();
        });
        let error = verify_run(&bundle.scenario(), PRODUCER_SCENARIO)
            .err()
            .unwrap();
        assert!(error.contains("suite did not run"), "{error}");
    }

    #[test]
    fn a_bundle_of_anything_but_one_alliance_scenario_is_refused() {
        let bundle = Bundle::new("success");
        bundle.write_manifest(|manifest| {
            let row = manifest["scenarios"][0].clone();
            manifest["scenarios"].as_array_mut().unwrap().push(row);
        });
        let error = verify_run(&bundle.suite(), PRODUCER_SCENARIO)
            .err()
            .unwrap();
        assert!(error.contains("more than one"), "{error}");

        let bundle = Bundle::new("success");
        std::fs::create_dir_all(bundle.suite().join("scenarios/0001")).unwrap();
        let error = verify_run(&bundle.suite().join("scenarios/0001"), PRODUCER_SCENARIO)
            .err()
            .unwrap();
        assert!(error.contains("not the"), "{error}");

        let bundle = Bundle::new("success");
        bundle.write_manifest(|manifest| manifest["passed"] = false.into());
        let error = verify_run(&bundle.suite(), PRODUCER_SCENARIO)
            .err()
            .unwrap();
        assert!(error.contains("finalized passing"), "{error}");
    }

    #[test]
    fn a_directory_outside_a_suite_bundle_is_refused() {
        let bundle = Bundle::new("success");
        let error = verify_run(&bundle.scenario().join("automation"), PRODUCER_SCENARIO)
            .err()
            .unwrap();
        assert!(error.contains("neither a native suite"), "{error}");
    }

    #[test]
    fn an_artifact_that_differs_from_its_recorded_digest_is_refused() {
        let bundle = Bundle::new("success");
        std::fs::write(
            bundle.scenario().join("automation/trace.jsonl"),
            "{\"terminal_reason\":\"success\"}\n",
        )
        .unwrap();
        let run = verify_run(&bundle.scenario(), PRODUCER_SCENARIO).unwrap();
        assert!(terminal_reason(&run).unwrap_err().contains("run trace"));

        let bundle = Bundle::new("success");
        std::fs::write(
            bundle.scenario().join("native-acceptance.json"),
            b"{\"schema\":\"tampered\"}",
        )
        .unwrap();
        let error = verify_run(&bundle.scenario(), PRODUCER_SCENARIO)
            .err()
            .unwrap();
        assert!(error.contains("native-acceptance.json"), "{error}");
    }

    #[test]
    fn extraction_copies_the_recorded_save_and_writes_its_provenance() {
        let bundle = Bundle::new("success");
        let repository = tempfile::tempdir().unwrap();
        let script = repository.path().join(PRODUCER_SCRIPT);
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::copy(repository_root().join(PRODUCER_SCRIPT), &script).unwrap();
        for args in [
            &["init", "--quiet"][..],
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                "base",
            ][..],
        ] {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(repository.path())
                .status()
                .unwrap();
            assert!(status.success());
        }
        let out = repository.path().join("saves/allied-starbase.sav");
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();

        extract_save(
            repository.path(),
            &bundle.scenario(),
            PRODUCER_SCENARIO,
            0,
            &out,
        )
        .unwrap();

        assert_eq!(std::fs::read(&out).unwrap(), SAVE_BYTES);
        let provenance: Provenance =
            serde_json::from_slice(&std::fs::read(provenance_path(&out)).unwrap()).unwrap();
        assert_eq!(provenance.save.sha256, crate::hex_sha256(SAVE_BYTES));
        assert_eq!(provenance.save.path, "saves/allied-starbase.sav");
        assert_eq!(provenance.terminal_reason, "success");
        assert_eq!(
            provenance.run_id,
            "art-real-sol-starbase-alliance/scenarios/0000"
        );
        assert_eq!(
            provenance.scenario.sha256,
            crate::hex_sha256(&std::fs::read(repository_root().join(PRODUCER_SCRIPT)).unwrap())
        );
    }

    #[test]
    fn any_selected_scenario_can_produce_but_an_unchanged_parent_slot_is_refused() {
        let bundle = Bundle::new("success");
        bundle.use_script("fly-to-alpha-centauri-v1");
        assert!(verify_run(&bundle.suite(), "fly-to-alpha-centauri-v1").is_ok());
        let out = repository_root().join("tmp/unwritten-test.sav");
        let error = extract_save(
            repository_root(),
            &bundle.suite(),
            "fly-to-alpha-centauri-v1",
            0,
            &out,
        )
        .unwrap_err();
        assert!(error.contains("unchanged"), "{error}");
        let error = extract_save(
            repository_root(),
            &bundle.suite(),
            "fly-to-alpha-centauri-v1",
            1,
            &out,
        )
        .unwrap_err();
        assert!(error.contains("uqmsave.01"), "{error}");
        // A different valid slot needs no capture-label convention or resource assertion heuristic.
        let different = include_bytes!("../../scripts/saves/alliance-after-mercury.sav");
        std::fs::write(
            bundle.scenario().join("config-final/save/uqmsave.00"),
            different,
        )
        .unwrap();
        let acceptance_path = bundle.scenario().join("native-acceptance.json");
        let mut acceptance: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&acceptance_path).unwrap()).unwrap();
        acceptance["retained_files"][1] = serde_json::to_value(crate::retained_identity(
            "config-final/save/uqmsave.00",
            different,
        ))
        .unwrap();
        std::fs::write(&acceptance_path, serde_json::to_vec(&acceptance).unwrap()).unwrap();
        bundle.write_manifest(|manifest| {
            manifest["scenarios"][0]["script"]["path"] =
                "rust/scripts/fly-to-alpha-centauri-v1.json".into()
        });
        let output_dir = repository_root().join("tmp");
        std::fs::create_dir_all(&output_dir).unwrap();
        let output = tempfile::tempdir_in(output_dir).unwrap();
        let out = output.path().join("produced.sav");
        extract_save(
            repository_root(),
            &bundle.suite(),
            "fly-to-alpha-centauri-v1",
            0,
            &out,
        )
        .unwrap();
        let provenance: Provenance = read_json(&provenance_path(&out), MANIFEST_LIMIT).unwrap();
        assert_eq!(provenance.scenario.name, "fly-to-alpha-centauri-v1");
        assert_eq!(
            provenance.parent_save.unwrap().sha256,
            crate::hex_sha256(SAVE_BYTES)
        );
        assert_eq!(provenance.campaign_state["sis:fuel"], 926);
        assert_eq!(
            provenance.producing_commit,
            "0123456789012345678901234567890123456789"
        );
    }

    #[test]
    fn snapshots_describe_each_saved_file_not_the_scripts_last_assertion() {
        for (bytes, fuel) in [
            (SAVE_BYTES, 896),
            (
                include_bytes!("../../scripts/saves/alliance-after-mercury.sav").as_slice(),
                926,
            ),
            (
                include_bytes!("../../scripts/saves/alliance-after-luna.sav").as_slice(),
                896,
            ),
        ] {
            let state = saved_campaign_state(bytes).unwrap();
            assert_eq!(state["star:current"], 82);
            assert_eq!(state["sis:fuel"], fuel);
        }
    }

    #[test]
    fn a_script_whose_save_is_missing_is_refused() {
        let start_save = StartSave {
            path: "saves/missing.sav".into(),
            slot: 0,
        };
        let error =
            verify_start_save(repository_root(), "rust/scripts/x.json", &start_save).unwrap_err();
        assert!(error.contains("starts from"), "{error}");
    }
}
