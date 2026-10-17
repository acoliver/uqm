//! Start saves: the committed save games that later scenarios begin from.
//!
//! `real-sol-starbase-alliance` plays the allied Starbase chain once and saves
//! to slot 0. `extract-save` copies that save out of a passing run's retained
//! config and writes its provenance, which documents how the save was made. A
//! scenario declaring `start_save` is admitted only when the save is a regular
//! file next to the script.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uqm_rust::automation::native_window::NATIVE_ACCEPTANCE_SCHEMA;
use uqm_rust::automation::{parse_script, Action, NativeRetainedInput, StartSave};

use crate::ci::authority::Authority;
use crate::native_suite::{ScenarioState, SuiteManifest};

/// The scenario that plays the chain and produces the save.
pub const PRODUCER_SCENARIO: &str = "real-sol-starbase-alliance";
const PRODUCER_SCRIPT: &str = "rust/scripts/real-sol-starbase-alliance.json";
const PROVENANCE_SCHEMA: &str = "uqm-start-save-provenance-v1";
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
    /// The `assert_ship_resources` step that ran last before the save.
    pub asserted_before_save: serde_json::Value,
}

/// Where the provenance of `save` lives: next to it, `<stem>.provenance.json`.
pub fn provenance_path(save: &Path) -> PathBuf {
    save.with_extension("provenance.json")
}

/// The last resource assertion in the producing script. The save is taken
/// after it, so these are the values a scenario sees right after loading.
fn asserted_before_save(script_bytes: &[u8], path: &Path) -> Result<serde_json::Value, String> {
    let document = parse_script(script_bytes, path).map_err(|error| error.to_string())?;
    document
        .steps
        .iter()
        .rev()
        .find(|action| matches!(action, Action::AssertShipResources(_)))
        .map(|action| serde_json::to_value(action).map_err(|error| error.to_string()))
        .transpose()?
        .ok_or_else(|| format!("{PRODUCER_SCRIPT} asserts no ship resources before its save"))
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
}

#[derive(Deserialize)]
struct RecordedScript {
    sha256: String,
}

/// A scenario of a finalized, passing suite bundle that ran exactly the pinned
/// alliance script, with every artifact read here checked against the digests
/// the bundle recorded for it.
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
    let bytes = crate::read_regular_file_nofollow_bounded(path, limit)?;
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

/// Establish that `artifacts` is the passing run of the pinned alliance script.
fn verify_run(authority: &Authority, artifacts: &Path) -> Result<ProducingRun, String> {
    let pinned = authority
        .native_acceptance
        .scenario_scripts
        .iter()
        .find(|pin| pin.path == PRODUCER_SCRIPT)
        .ok_or("authority does not pin the alliance script")?;
    let (suite, requested) = locate_suite(artifacts)?;
    let manifest: SuiteManifest = read_json(&suite.join(SUITE_MANIFEST), MANIFEST_LIMIT)?;
    if manifest.schema != SUITE_SCHEMA || !manifest.finalized || !manifest.passed {
        return Err(format!(
            "{} is not a finalized passing native suite",
            suite.display()
        ));
    }
    let request = crate::read_regular_file_nofollow_bounded(
        &suite.join(&manifest.request.relative_path),
        MANIFEST_LIMIT,
    )?;
    require_digest(
        "suite request",
        &request,
        &manifest.request.sha256,
        Some(manifest.request.byte_length),
    )?;
    let [row] = manifest.scenarios.as_slice() else {
        return Err(format!(
            "{} ran {} scenarios; extract-save needs a bundle of exactly the {PRODUCER_SCENARIO} run",
            suite.display(),
            manifest.scenarios.len()
        ));
    };
    if row.script != *pinned {
        return Err(format!(
            "the suite ran {}, not {PRODUCER_SCRIPT}",
            row.script.path
        ));
    }
    if requested.is_some_and(|directory| directory != row.directory) {
        return Err(format!(
            "{} is not the {PRODUCER_SCENARIO} scenario of its suite ({})",
            artifacts.display(),
            row.directory
        ));
    }
    let ScenarioState::Completed { manifest: record } = &row.state else {
        return Err(format!("the {PRODUCER_SCENARIO} scenario did not complete"));
    };
    let scenario_dir = suite.join(&row.directory);
    let acceptance_bytes = crate::read_regular_file_nofollow_bounded(
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
    let trace = crate::read_regular_file_nofollow_bounded(
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

/// Copy the slot's save out of a passing alliance run and write its provenance.
///
/// `artifacts` is the suite bundle root or its `scenarios/NNNN` directory.
pub fn extract_save(
    root: &Path,
    authority: &Authority,
    artifacts: &Path,
    slot: u8,
    out: &Path,
) -> Result<(), String> {
    let run = verify_run(authority, artifacts)?;
    let reason = terminal_reason(&run)?;
    if reason != "success" {
        return Err(format!(
            "{} ended with terminal_reason {reason}, not success; refusing to extract a save",
            run.run_id
        ));
    }
    let script_bytes =
        crate::read_regular_file_nofollow_bounded(&root.join(PRODUCER_SCRIPT), SAVE_LIMIT)?;
    let script_sha = crate::hex_sha256(&script_bytes);
    if script_sha != run.recorded.script.sha256 {
        return Err(format!(
            "{PRODUCER_SCRIPT} in this checkout ({script_sha}) is not the script the run played ({})",
            run.recorded.script.sha256
        ));
    }
    let asserted = asserted_before_save(&script_bytes, Path::new(PRODUCER_SCRIPT))?;
    let slot_file = format!("uqmsave.{slot:02}");
    let save_relative = format!("config-final/save/{slot_file}");
    let save_bytes = crate::read_regular_file_nofollow_bounded(
        &run.scenario_dir.join(&save_relative),
        SAVE_LIMIT,
    )?;
    let retained = run
        .recorded
        .retained_files
        .iter()
        .find(|file| file.relative_path == save_relative)
        .ok_or_else(|| format!("the run did not retain {save_relative}"))?;
    require_digest(
        &save_relative,
        &save_bytes,
        &retained.sha256,
        Some(retained.byte_length),
    )?;
    let commit = git_head(root)?;
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
            sha256: crate::hex_sha256(&save_bytes),
            byte_length: save_bytes.len() as u64,
        },
        scenario: ScenarioIdentity {
            name: PRODUCER_SCENARIO.into(),
            path: PRODUCER_SCRIPT.into(),
            sha256: script_sha,
        },
        producing_commit: commit,
        run_id: run.run_id,
        terminal_reason: reason,
        asserted_before_save: asserted,
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

fn git_head(root: &Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("run git rev-parse HEAD: {error}"))?;
    if !output.status.success() {
        return Err("git rev-parse HEAD failed".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    crate::read_regular_file_nofollow_bounded(&root.join(&relative), SAVE_LIMIT).map_err(
        |error| {
            format!(
                "script {script_path} starts from {}: {error}",
                relative.display()
            )
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci::authority::load_authority;

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
        let authority = load_authority(root).unwrap();
        for pin in &authority.native_acceptance.scenario_scripts {
            let path = root.join(&pin.path);
            let bytes = std::fs::read(&path).unwrap();
            let document = parse_script(&bytes, &path).unwrap();
            if let Some(start_save) = document.start_save {
                verify_start_save(root, &pin.path, &start_save)
                    .unwrap_or_else(|error| panic!("{}: {error}", pin.path));
            }
        }
    }

    const SAVE_BYTES: &[u8] = b"allied starbase save";

    /// A suite bundle laid out like a real run of the alliance script.
    struct Bundle {
        directory: tempfile::TempDir,
        authority: Authority,
    }

    impl Bundle {
        fn new(terminal_reason: &str) -> Self {
            let authority = load_authority(repository_root()).unwrap();
            let pinned = authority
                .native_acceptance
                .scenario_scripts
                .iter()
                .find(|pin| pin.path == PRODUCER_SCRIPT)
                .unwrap()
                .clone();
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
            let acceptance = serde_json::to_vec_pretty(&serde_json::json!({
                "schema": NATIVE_ACCEPTANCE_SCHEMA,
                "script": {
                    "relative_path": "inputs/real-sol-starbase-alliance.json",
                    "byte_length": script_bytes.len(),
                    "sha256": crate::hex_sha256(&script_bytes),
                },
                "retained_files": [
                    {
                        "relative_path": "config-final/save/uqmsave.00",
                        "byte_length": SAVE_BYTES.len(),
                        "sha256": crate::hex_sha256(SAVE_BYTES),
                    },
                ],
                "trace_path": "automation/trace.jsonl",
                "trace_sha256": crate::hex_sha256(trace.as_bytes()),
            }))
            .unwrap();
            std::fs::write(scenario.join("native-acceptance.json"), &acceptance).unwrap();
            let request = serde_json::to_vec_pretty(&[&pinned]).unwrap();
            std::fs::write(suite.join("suite-request.json"), &request).unwrap();
            let bundle = Self {
                directory,
                authority,
            };
            bundle.write_manifest(|_| {});
            bundle
        }

        fn suite(&self) -> PathBuf {
            self.directory.path().join("art-real-sol-starbase-alliance")
        }

        fn scenario(&self) -> PathBuf {
            self.suite().join("scenarios/0000")
        }

        /// Rewrite `suite-manifest.json` for the bundle as it is on disk,
        /// after `edit` has changed the manifest's content.
        fn write_manifest(&self, edit: impl FnOnce(&mut serde_json::Value)) {
            let pinned = self
                .authority
                .native_acceptance
                .scenario_scripts
                .iter()
                .find(|pin| pin.path == PRODUCER_SCRIPT)
                .unwrap();
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
        let from_suite = verify_run(&bundle.authority, &bundle.suite()).unwrap();
        let from_scenario = verify_run(&bundle.authority, &bundle.scenario()).unwrap();
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
        let run = verify_run(&bundle.authority, &bundle.scenario()).unwrap();
        assert_eq!(terminal_reason(&run).unwrap(), "battle_lost");
        let out = bundle.directory.path().join("out.sav");
        let error = extract_save(
            bundle.directory.path(),
            &bundle.authority,
            &bundle.scenario(),
            0,
            &out,
        )
        .unwrap_err();
        assert!(error.contains("not success"), "{error}");
        assert!(!out.exists());
    }

    #[test]
    fn a_run_of_another_script_than_the_alliance_script_is_refused() {
        let bundle = Bundle::new("success");
        bundle.write_manifest(|manifest| {
            manifest["scenarios"][0]["script"]["path"] = "rust/scripts/other.json".into();
        });
        let error = verify_run(&bundle.authority, &bundle.scenario())
            .err()
            .unwrap();
        assert!(
            error.contains("not rust/scripts/real-sol-starbase-alliance.json"),
            "{error}"
        );
    }

    #[test]
    fn a_bundle_of_anything_but_one_alliance_scenario_is_refused() {
        let bundle = Bundle::new("success");
        bundle.write_manifest(|manifest| {
            let row = manifest["scenarios"][0].clone();
            manifest["scenarios"].as_array_mut().unwrap().push(row);
        });
        let error = verify_run(&bundle.authority, &bundle.suite())
            .err()
            .unwrap();
        assert!(error.contains("exactly"), "{error}");

        let bundle = Bundle::new("success");
        std::fs::create_dir_all(bundle.suite().join("scenarios/0001")).unwrap();
        let error = verify_run(&bundle.authority, &bundle.suite().join("scenarios/0001"))
            .err()
            .unwrap();
        assert!(error.contains("not the"), "{error}");

        let bundle = Bundle::new("success");
        bundle.write_manifest(|manifest| manifest["passed"] = false.into());
        let error = verify_run(&bundle.authority, &bundle.suite())
            .err()
            .unwrap();
        assert!(error.contains("finalized passing"), "{error}");
    }

    #[test]
    fn a_directory_outside_a_suite_bundle_is_refused() {
        let bundle = Bundle::new("success");
        let error = verify_run(&bundle.authority, &bundle.scenario().join("automation"))
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
        let run = verify_run(&bundle.authority, &bundle.scenario()).unwrap();
        assert!(terminal_reason(&run).unwrap_err().contains("run trace"));

        let bundle = Bundle::new("success");
        std::fs::write(
            bundle.scenario().join("native-acceptance.json"),
            b"{\"schema\":\"tampered\"}",
        )
        .unwrap();
        let error = verify_run(&bundle.authority, &bundle.scenario())
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
            &bundle.authority,
            &bundle.scenario(),
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
