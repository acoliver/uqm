//! `native smoke`: a short playability check of the game on a real window.
//!
//! The smoke set (`rust/ci/smoke.json`) is the smallest set of existing
//! scenarios that together play the main menu, flight in a solar system and in
//! hyperspace, orbit, landing and takeoff, a conversation, a battle, and a save
//! followed by a load. Each scenario runs through the same code path as
//! `native run --scenario`; this module only sequences them, keeps going after
//! a failure so the receipt is complete, and writes `smoke-receipt.json` and
//! `smoke-receipt.md`. It posts nothing anywhere.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

pub const SMOKE_SET: &str = "rust/ci/smoke.json";
const SMOKE_SCHEMA: &str = "uqm-smoke-v1";
const RECEIPT_SCHEMA: &str = "uqm-smoke-receipt-v1";

/// The functions a smoke set has to play between them.
pub const REQUIRED_FUNCTIONS: [&str; 6] = [
    "main_menu",
    "flight",
    "orbit_land_takeoff",
    "conversation",
    "battle",
    "save_load",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeSet {
    pub schema: String,
    pub scenarios: Vec<String>,
    pub covers: BTreeMap<String, Vec<String>>,
    /// Capture labels a reader should look at, per scenario.
    pub look_at: BTreeMap<String, Vec<String>>,
}

impl SmokeSet {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(SMOKE_SET);
        let bytes = std::fs::read(&path).map_err(|error| format!("read {SMOKE_SET}: {error}"))?;
        let set: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse {SMOKE_SET}: {error}"))?;
        set.check()?;
        Ok(set)
    }

    fn check(&self) -> Result<(), String> {
        if self.schema != SMOKE_SCHEMA {
            return Err(format!(
                "{SMOKE_SET} schema is {}, expected {SMOKE_SCHEMA}",
                self.schema
            ));
        }
        for table in [&self.covers, &self.look_at] {
            if let Some(stray) = table.keys().find(|name| !self.scenarios.contains(name)) {
                return Err(format!(
                    "{SMOKE_SET} describes {stray}, which is not a smoke scenario"
                ));
            }
        }
        for function in REQUIRED_FUNCTIONS {
            if !self
                .covers
                .values()
                .flatten()
                .any(|covered| covered == function)
            {
                return Err(format!("{SMOKE_SET} covers nothing for {function}"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioReceipt {
    pub scenario: String,
    pub passed: bool,
    pub terminal_reason: String,
    pub duration_ms: u64,
    pub capture_count: usize,
    pub frame_check: String,
    pub screenshots_dir: String,
    /// Original OS screenshots and cropped client images a reader should look at.
    pub look_at: Vec<LookAt>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LookAt {
    pub label: String,
    pub screenshot: String,
    pub original_os_capture: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub commit: String,
    pub tree_clean: bool,
    pub date: String,
    pub pr: Option<u32>,
    pub passed: bool,
    pub artifacts: String,
    pub scenarios: Vec<ScenarioReceipt>,
}

impl Receipt {
    pub fn to_markdown(&self) -> String {
        let mut text = format!(
            "### Smoke receipt\n\ncommit `{}` ({}), {}{}\n\n\
             | scenario | result | reason | seconds | captures | frame check |\n\
             |---|---|---|---|---|---|\n",
            self.commit,
            if self.tree_clean {
                "clean tree"
            } else {
                "DIRTY tree"
            },
            self.date,
            self.pr.map_or_else(String::new, |pr| format!(", PR #{pr}")),
        );
        for row in &self.scenarios {
            text.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} |\n",
                row.scenario,
                if row.passed { "pass" } else { "FAIL" },
                cell(&row.terminal_reason),
                row.duration_ms / 1000,
                row.capture_count,
                cell(&row.frame_check),
            ));
        }
        text.push_str(&format!(
            "\nOverall: **{}**. Evidence: `{}`\n\n\
             Frames to look at (a person or the image-checking agent; \"fixed\" needs these \
             looked at):\n\n",
            if self.passed { "pass" } else { "FAIL" },
            self.artifacts
        ));
        for row in &self.scenarios {
            for look in &row.look_at {
                text.push_str(&format!(
                    "- {} / {}: original OS `{}`; cropped client `{}`\n",
                    row.scenario, look.label, look.original_os_capture, look.screenshot
                ));
            }
        }
        text
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("run git {}: {error}", arguments.join(" ")))?;
    if !output.status.success() {
        return Err(format!("git {} failed", arguments.join(" ")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn utc_date() -> Result<String, String> {
    let output = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .map_err(|error| format!("run date: {error}"))?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Count `checkpoint-NNNN.png` window captures, not the `.os.png` originals.
fn count_captures(screenshots: &Path) -> usize {
    std::fs::read_dir(screenshots).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                name.starts_with("checkpoint-")
                    && name.ends_with(".png")
                    && !name.ends_with(".os.png")
            })
            .count()
    })
}

/// What the retained scenario directory says, read after the run.
struct Retained {
    duration_ms: Option<u64>,
    capture_count: usize,
    frame_check: String,
    look_at: Vec<LookAt>,
}

fn inspect(bundle: &Path, wanted: &[String]) -> Retained {
    let directory = bundle.join("scenarios/0000");
    let screenshots = directory.join("screenshots");
    let duration_ms = std::fs::read(bundle.join("suite-manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<crate::native_suite::SuiteManifest>(&bytes).ok())
        .and_then(|manifest| manifest.scenarios.first().map(|row| row.elapsed_ms));
    let proof = std::fs::read(directory.join("native-acceptance.json"))
        .map_err(|error| format!("no completed scenario manifest: {error}"))
        .and_then(|bytes| {
            serde_json::from_slice::<uqm_rust::automation::NativeAcceptanceManifest>(&bytes)
                .map_err(|error| format!("parse native-acceptance.json: {error}"))
        });
    let (frame_check, look_at) = match &proof {
        Ok(proof) => (
            match crate::native_frames::validate_scenario(&directory, proof) {
                Ok(()) => "passed".to_string(),
                Err(error) => error,
            },
            look_at_frames(&screenshots, proof, wanted),
        ),
        Err(error) => (format!("not run: {error}"), Vec::new()),
    };
    Retained {
        duration_ms,
        capture_count: count_captures(&screenshots),
        frame_check,
        look_at,
    }
}

fn look_at_frames(
    screenshots: &Path,
    proof: &uqm_rust::automation::NativeAcceptanceManifest,
    wanted: &[String],
) -> Vec<LookAt> {
    let Some(plan) = &proof.window.checkpoint_plan else {
        return Vec::new();
    };
    wanted
        .iter()
        .filter_map(|label| {
            plan.checkpoints.iter().find_map(|checkpoint| {
                let action = serde_json::to_value(&checkpoint.action).ok()?;
                let named = action.get("label").and_then(|value| value.as_str());
                (named == Some(label.as_str())).then(|| LookAt {
                    label: label.clone(),
                    screenshot: screenshots
                        .join(format!("checkpoint-{:04}.png", checkpoint.action_index))
                        .display()
                        .to_string(),
                    original_os_capture: screenshots
                        .join(format!("checkpoint-{:04}.os.png", checkpoint.action_index))
                        .display()
                        .to_string(),
                })
            })
        })
        .collect()
}

pub struct SmokeRequest<'a> {
    pub root: &'a Path,
    pub artifacts: PathBuf,
    pub pr: Option<u32>,
}

/// Run every smoke scenario with `run_one`, then write the receipt.
///
/// `run_one(scenario, bundle)` is the `native run --scenario` path.
pub fn run(
    request: &SmokeRequest<'_>,
    set: &SmokeSet,
    run_one: &mut dyn FnMut(&str, &Path) -> Result<(), String>,
) -> Result<(), String> {
    std::fs::create_dir_all(&request.artifacts)
        .map_err(|error| format!("create {}: {error}", request.artifacts.display()))?;
    let mut rows = Vec::new();
    for scenario in &set.scenarios {
        let bundle = request.artifacts.join(scenario);
        println!("smoke: running {scenario}");
        let started = std::time::Instant::now();
        let outcome = run_one(scenario, &bundle);
        let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let wanted = set.look_at.get(scenario).map_or(&[][..], Vec::as_slice);
        let retained = inspect(&bundle, wanted);
        let frames_ok = retained.frame_check == "passed";
        let row = ScenarioReceipt {
            scenario: scenario.clone(),
            passed: outcome.is_ok() && frames_ok,
            terminal_reason: match &outcome {
                Ok(()) => "completed".to_string(),
                Err(error) => error.clone(),
            },
            duration_ms: retained.duration_ms.unwrap_or(wall_ms),
            capture_count: retained.capture_count,
            frame_check: retained.frame_check,
            screenshots_dir: bundle
                .join("scenarios/0000/screenshots")
                .display()
                .to_string(),
            look_at: retained.look_at,
        };
        println!(
            "smoke: {scenario}: {}",
            if row.passed { "pass" } else { "FAIL" }
        );
        rows.push(row);
    }
    let receipt = Receipt {
        schema: RECEIPT_SCHEMA,
        commit: git_output(request.root, &["rev-parse", "HEAD"])?,
        tree_clean: git_output(request.root, &["status", "--porcelain"])?.is_empty(),
        date: utc_date()?,
        pr: request.pr,
        passed: rows.iter().all(|row| row.passed),
        artifacts: request.artifacts.display().to_string(),
        scenarios: rows,
    };
    let json = serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?;
    std::fs::write(request.artifacts.join("smoke-receipt.json"), json + "\n")
        .map_err(|error| format!("write smoke-receipt.json: {error}"))?;
    std::fs::write(
        request.artifacts.join("smoke-receipt.md"),
        receipt.to_markdown(),
    )
    .map_err(|error| format!("write smoke-receipt.md: {error}"))?;
    if receipt.passed {
        println!("smoke: passed; receipt in {}", receipt.artifacts);
        return Ok(());
    }
    let failed: Vec<&str> = receipt
        .scenarios
        .iter()
        .filter(|row| !row.passed)
        .map(|row| row.scenario.as_str())
        .collect();
    Err(format!(
        "smoke failed: {}; receipt in {}",
        failed.join(", "),
        receipt.artifacts
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn every_smoke_scenario_is_pinned_in_gates_and_the_six_functions_are_covered() {
        let root = repo_root();
        let set = SmokeSet::load(&root).unwrap();
        let gates: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("rust/ci/gates.json")).unwrap())
                .unwrap();
        let pinned: Vec<String> = gates["native_acceptance"]["scenario_scripts"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|script| script["path"].as_str())
            .filter_map(|path| Path::new(path).file_stem()?.to_str().map(str::to_string))
            .collect();
        for scenario in &set.scenarios {
            assert!(pinned.contains(scenario), "{scenario} is not in gates.json");
        }
        for function in REQUIRED_FUNCTIONS {
            assert!(
                set.covers
                    .values()
                    .flatten()
                    .any(|covered| covered == function),
                "{function} is not covered"
            );
        }
    }

    #[test]
    fn look_at_labels_are_captures_in_their_scripts() {
        let root = repo_root();
        let set = SmokeSet::load(&root).unwrap();
        for (scenario, labels) in &set.look_at {
            let script =
                std::fs::read_to_string(root.join(format!("rust/scripts/{scenario}.json")))
                    .unwrap();
            for label in labels {
                assert!(
                    script.contains(&format!("\"label\": \"{label}\"")),
                    "{scenario} has no capture {label}"
                );
            }
        }
    }

    #[test]
    fn a_set_missing_a_function_is_refused() {
        let mut set = SmokeSet::load(&repo_root()).unwrap();
        set.covers.remove("battle-v1");
        assert!(set.check().unwrap_err().contains("battle"));
    }

    #[test]
    fn the_run_continues_after_a_failure_and_the_receipt_says_so() {
        let temp = tempfile::tempdir().unwrap();
        let root = repo_root();
        let set = SmokeSet::load(&root).unwrap();
        let mut calls = Vec::new();
        let result = run(
            &SmokeRequest {
                root: &root,
                artifacts: temp.path().join("out"),
                pr: Some(7),
            },
            &set,
            &mut |scenario, _| {
                calls.push(scenario.to_string());
                if scenario == "load-save-v1" {
                    Err("presentation 3 was not acknowledged".into())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(calls, set.scenarios);
        assert!(result.unwrap_err().contains("load-save-v1"));
        let receipt: serde_json::Value = serde_json::from_slice(
            &std::fs::read(temp.path().join("out/smoke-receipt.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(receipt["pr"], 7);
        assert_eq!(receipt["passed"], false);
        let rows = receipt["scenarios"].as_array().unwrap();
        assert_eq!(rows.len(), set.scenarios.len());
        assert_eq!(
            rows[2]["terminal_reason"],
            "presentation 3 was not acknowledged"
        );
        let markdown = std::fs::read_to_string(temp.path().join("out/smoke-receipt.md")).unwrap();
        assert!(markdown.contains("| load-save-v1 | FAIL |"), "{markdown}");
        assert!(markdown.contains("PR #7"), "{markdown}");
    }

    #[test]
    fn a_scenario_that_ran_nothing_has_no_frame_check_to_pass() {
        let temp = tempfile::tempdir().unwrap();
        let retained = inspect(temp.path(), &[]);
        assert!(
            retained.frame_check.starts_with("not run:"),
            "{}",
            retained.frame_check
        );
        assert_eq!(retained.capture_count, 0);
    }

    #[test]
    fn table_cells_cannot_break_the_row() {
        assert_eq!(cell("a|b\nc"), "a\\|b c");
    }
}
