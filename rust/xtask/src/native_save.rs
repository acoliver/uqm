//! Start saves: the committed save games that later scenarios begin from.
//!
//! `real-sol-starbase-alliance` plays the allied Starbase chain once and saves
//! to slot 0. `extract-save` copies that save out of a passing run's retained
//! config and writes its provenance. A scenario declaring `start_save` is
//! admitted only when the save on disk matches the digest pinned in the
//! authority, and the provenance must name the alliance script the authority
//! pins now, so changing the chain forces the save to be regenerated.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uqm_rust::automation::{parse_script, Action, StartSave};

use crate::ci::authority::{Authority, PinnedSave};

/// The scenario that plays the chain and produces the save.
pub const PRODUCER_SCENARIO: &str = "real-sol-starbase-alliance";
const PRODUCER_SCRIPT: &str = "rust/scripts/real-sol-starbase-alliance.json";
const PROVENANCE_SCHEMA: &str = "uqm-start-save-provenance-v1";
const SAVE_LIMIT: u64 = 4 * 1024 * 1024;
const TRACE_LIMIT: u64 = 64 * 1024 * 1024;

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

fn terminal_reason(run: &Path) -> Result<String, String> {
    let trace = crate::read_regular_file_nofollow_bounded(
        &run.join("automation/trace.jsonl"),
        TRACE_LIMIT,
    )?;
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
pub fn extract_save(
    root: &Path,
    authority: &Authority,
    run: &Path,
    slot: u8,
    out: &Path,
) -> Result<(), String> {
    let reason = terminal_reason(run)?;
    if reason != "success" {
        return Err(format!(
            "{} ended with terminal_reason {reason}, not success; refusing to extract a save",
            run.display()
        ));
    }
    let script_name = format!("{PRODUCER_SCENARIO}.json");
    let script_bytes = crate::read_regular_file_nofollow_bounded(
        &run.join("inputs").join(&script_name),
        SAVE_LIMIT,
    )?;
    let script_sha = crate::hex_sha256(&script_bytes);
    let pinned = authority
        .native_acceptance
        .scenario_scripts
        .iter()
        .find(|pin| pin.path == PRODUCER_SCRIPT)
        .ok_or("authority does not pin the alliance script")?;
    if pinned.sha256 != script_sha {
        return Err(format!(
            "the run played an alliance script ({script_sha}) that is not the pinned one ({})",
            pinned.sha256
        ));
    }
    let asserted = asserted_before_save(&script_bytes, Path::new(PRODUCER_SCRIPT))?;
    let slot_file = format!("uqmsave.{slot:02}");
    let save_bytes = crate::read_regular_file_nofollow_bounded(
        &run.join("config-final/save").join(&slot_file),
        SAVE_LIMIT,
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
        run_id: run
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or("run directory has no name")?,
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
    println!(
        "pin it in rust/ci/gates.json under native_acceptance.start_saves:\n{}",
        serde_json::to_string_pretty(&PinnedSave {
            path: provenance.save.path,
            sha256: provenance.save.sha256,
            byte_length: provenance.save.byte_length,
        })
        .map_err(|error| error.to_string())?
    );
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

/// Admit the save a script starts with: it must be pinned and match its pin.
pub fn verify_start_save(
    root: &Path,
    script_path: &str,
    start_save: &StartSave,
    authority: &Authority,
) -> Result<(), String> {
    let relative = Path::new(script_path)
        .parent()
        .ok_or("script path has no directory")?
        .join(&start_save.path);
    let relative = relative.to_string_lossy();
    let pin = authority
        .native_acceptance
        .start_saves
        .iter()
        .find(|pin| pin.path == relative)
        .ok_or_else(|| {
            format!(
                "script {} starts from {relative}, which the authority does not pin",
                script_path
            )
        })?;
    let bytes = crate::read_regular_file_nofollow_bounded(&root.join(&pin.path), pin.byte_length)
        .map_err(|error| format!("start save {}: {error}", pin.path))?;
    if bytes.len() as u64 != pin.byte_length || crate::hex_sha256(&bytes) != pin.sha256 {
        return Err(format!(
            "start save {} differs from machine authority",
            pin.path
        ));
    }
    Ok(())
}

/// The provenance of every pinned save must name the alliance script the
/// authority pins now, and the save bytes it names.
pub fn check_provenance_current(root: &Path, authority: &Authority) -> Result<(), String> {
    let producer = authority
        .native_acceptance
        .scenario_scripts
        .iter()
        .find(|pin| pin.path == PRODUCER_SCRIPT)
        .ok_or("authority does not pin the alliance script")?;
    for pin in &authority.native_acceptance.start_saves {
        let provenance_file = provenance_path(&root.join(&pin.path));
        let bytes = crate::read_regular_file_nofollow_bounded(&provenance_file, SAVE_LIMIT)
            .map_err(|error| format!("provenance of {}: {error}", pin.path))?;
        let provenance: Provenance = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse provenance of {}: {error}", pin.path))?;
        if provenance.scenario.sha256 != producer.sha256 {
            return Err(format!(
                "{} was produced by {} {}, but {PRODUCER_SCRIPT} is now pinned as {}; \
                 regenerate the save",
                pin.path, provenance.scenario.name, provenance.scenario.sha256, producer.sha256
            ));
        }
        if provenance.save.sha256 != pin.sha256 || provenance.save.byte_length != pin.byte_length {
            return Err(format!(
                "provenance of {} names different save bytes",
                pin.path
            ));
        }
    }
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

    /// Every script that starts from a save must have that save pinned, present
    /// and matching, and the provenance of every pinned save must name the
    /// alliance script as pinned today. Changing the chain therefore fails here
    /// until the save is regenerated with `native extract-save`.
    #[test]
    fn every_start_save_is_pinned_and_produced_by_the_current_chain() {
        let root = repository_root();
        let authority = load_authority(root).unwrap();
        for pin in &authority.native_acceptance.scenario_scripts {
            let path = root.join(&pin.path);
            let bytes = std::fs::read(&path).unwrap();
            let document = parse_script(&bytes, &path).unwrap();
            if let Some(start_save) = document.start_save {
                verify_start_save(root, &pin.path, &start_save, &authority)
                    .unwrap_or_else(|error| panic!("{}: {error}", pin.path));
            }
        }
        check_provenance_current(root, &authority).unwrap();
    }

    #[test]
    fn a_provenance_naming_another_alliance_script_is_stale() {
        let root = tempfile::tempdir().unwrap();
        let mut authority = load_authority(repository_root()).unwrap();
        let save = b"save bytes";
        std::fs::write(root.path().join("a.sav"), save).unwrap();
        authority.native_acceptance.start_saves = vec![PinnedSave {
            path: "a.sav".into(),
            sha256: crate::hex_sha256(save),
            byte_length: save.len() as u64,
        }];
        let provenance = Provenance {
            schema: PROVENANCE_SCHEMA.into(),
            save: SaveIdentity {
                path: "a.sav".into(),
                slot: 0,
                sha256: crate::hex_sha256(save),
                byte_length: save.len() as u64,
            },
            scenario: ScenarioIdentity {
                name: PRODUCER_SCENARIO.into(),
                path: PRODUCER_SCRIPT.into(),
                sha256: "0".repeat(64),
            },
            producing_commit: "c".into(),
            run_id: "run".into(),
            terminal_reason: "success".into(),
            asserted_before_save: serde_json::Value::Null,
        };
        std::fs::write(
            provenance_path(&root.path().join("a.sav")),
            serde_json::to_vec(&provenance).unwrap(),
        )
        .unwrap();
        let error = check_provenance_current(root.path(), &authority).unwrap_err();
        assert!(error.contains("regenerate the save"), "{error}");
    }

    #[test]
    fn a_script_whose_save_is_not_pinned_is_refused() {
        let authority = load_authority(repository_root()).unwrap();
        let start_save = StartSave {
            path: "saves/missing.sav".into(),
            slot: 0,
        };
        let error = verify_start_save(
            repository_root(),
            "rust/scripts/x.json",
            &start_save,
            &authority,
        )
        .unwrap_err();
        assert!(error.contains("does not pin"), "{error}");
    }

    #[test]
    fn a_save_that_differs_from_its_pin_is_refused() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("rust/scripts/saves")).unwrap();
        std::fs::write(root.path().join("rust/scripts/saves/a.sav"), b"changed").unwrap();
        let mut authority = load_authority(repository_root()).unwrap();
        authority.native_acceptance.start_saves = vec![PinnedSave {
            path: "rust/scripts/saves/a.sav".into(),
            sha256: crate::hex_sha256(b"original"),
            byte_length: 8,
        }];
        let start_save = StartSave {
            path: "saves/a.sav".into(),
            slot: 0,
        };
        let error = verify_start_save(root.path(), "rust/scripts/x.json", &start_save, &authority)
            .unwrap_err();
        assert!(error.contains("differs"), "{error}");
    }
}
