use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::quality::tool::{self, Options};

const DEBT: &[&str] = &[
    "unsafe_op_in_unsafe_fn",
    "clippy::undocumented_unsafe_blocks",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::mem_forget",
];

#[derive(Default)]
struct Diagnostics {
    counts: BTreeMap<String, u64>,
    seen: BTreeSet<String>,
    errors: Vec<String>,
}
impl Diagnostics {
    fn message(
        &mut self,
        root: &Path,
        message: &Value,
        identities: &BTreeMap<String, String>,
    ) -> Result<(), String> {
        if message["reason"] != "compiler-message" {
            return Ok(());
        }
        let diagnostic = &message["message"];
        let level = diagnostic["level"]
            .as_str()
            .ok_or("diagnostic missing level")?;
        if !["error", "warning"].contains(&level) {
            return Ok(());
        }
        let raw_code = diagnostic["code"]["code"].as_str().unwrap_or("");
        // rustc reports this lint with its error code even at warning level.
        let code = if level == "warning" && raw_code == "E0133" {
            "unsafe_op_in_unsafe_fn"
        } else {
            raw_code
        };
        let rendered = diagnostic["rendered"]
            .as_str()
            .unwrap_or("unrendered diagnostic");
        if level != "warning" || !DEBT.contains(&code) {
            self.errors.push(rendered.into());
            return Ok(());
        }
        let spans = diagnostic["spans"]
            .as_array()
            .ok_or("diagnostic missing spans")?;
        let primary = spans
            .iter()
            .find(|s| s["is_primary"] == true)
            .ok_or("debt diagnostic missing primary span")?;
        let filename = primary["file_name"]
            .as_str()
            .ok_or("primary span missing filename")?;
        let Some(identity) = tool::file_identity(root, filename, identities)? else {
            return Err(format!("debt diagnostic outside workspace: {filename}"));
        };
        let event = format!(
            "{code}|{filename}|{}:{}|{}",
            primary["line_start"], primary["column_start"], diagnostic["message"]
        );
        if self.seen.insert(event) {
            *self.counts.entry(format!("{code}|{identity}")).or_default() += 1;
        }
        Ok(())
    }
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let options = Options::parse(args)?;
    let identities = tool::identities(root)?;
    let mut command = Command::new(crate::quality::cargo());
    command.current_dir(root.join("rust")).args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--no-default-features",
        "--features",
        "audio_heart,debug-process,native",
        "--message-format=json",
        "--",
        "-D",
        "warnings",
    ]);
    for lint in DEBT {
        command.args(["--force-warn", lint]);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("launch clippy: {e}"))?;
    let stdout = child.stdout.take().ok_or("clippy stdout missing")?;
    let mut diagnostics = Diagnostics::default();
    let parsed = (|| {
        for line in BufReader::new(stdout).lines() {
            let line = line.map_err(|e| e.to_string())?;
            let message: Value =
                serde_json::from_str(&line).map_err(|e| format!("clippy JSON: {e}"))?;
            diagnostics.message(root, &message, &identities)?;
        }
        Ok::<_, String>(())
    })();
    // Always reap the compiler, including malformed output and broken pipes.
    let status = child.wait().map_err(|e| e.to_string())?;
    parsed?;
    if !status.success() || !diagnostics.errors.is_empty() {
        return Err(format!(
            "clippy deny set: {} findings ({status})\n{}",
            diagnostics.errors.len(),
            diagnostics.errors.join("\n")
        ));
    }
    println!("clippy deny set: 0 findings");
    for lint in DEBT {
        let total: u64 = diagnostics
            .counts
            .iter()
            .filter(|(k, _)| k.starts_with(&format!("{lint}|")))
            .map(|(_, v)| v)
            .sum();
        println!("{lint}: {total}");
    }
    tool::record(root, &options, diagnostics.counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message(level: &str, code: &str) -> Value {
        serde_json::json!({"reason":"compiler-message", "message":{"level":level,"code":{"code":code},"message":"debt", "rendered":"diagnostic", "spans":[{"is_primary":true,"file_name":"src/savegame/encode.rs","line_start":44,"column_start":1}]}})
    }
    #[test]
    fn only_debt_warnings_are_baselined_and_duplicate_targets_count_once() {
        let mut d = Diagnostics::default();
        let ids = [(
            "src/savegame/encode.rs".into(),
            "uqm::savegame::encode::@file".into(),
        )]
        .into();
        for _ in 0..2 {
            d.message(
                Path::new("/repo"),
                &message("warning", "clippy::unwrap_used"),
                &ids,
            )
            .unwrap();
        }
        assert_eq!(d.counts.values().sum::<u64>(), 1);
        assert!(d
            .counts
            .contains_key("clippy::unwrap_used|uqm::savegame::encode::@file"));
        d.message(Path::new("/repo"), &message("warning", "clippy::all"), &ids)
            .unwrap();
        d.message(
            Path::new("/repo"),
            &message("error", "clippy::unwrap_used"),
            &ids,
        )
        .unwrap();
        assert_eq!(d.errors.len(), 2);
    }
    #[test]
    fn unknown_warning_and_rust_error_fail_even_without_a_lint_code() {
        let mut d = Diagnostics::default();
        d.message(
            Path::new("/repo"),
            &message("warning", ""),
            &BTreeMap::new(),
        )
        .unwrap();
        d.message(
            Path::new("/repo"),
            &message("error", "E0133"),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(d.errors.len(), 2);
    }
    #[test]
    fn rustc_unsafe_operation_warning_uses_error_code_but_errors_still_fail() {
        let mut d = Diagnostics::default();
        d.message(
            Path::new("/repo"),
            &message("warning", "E0133"),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(d.counts.values().sum::<u64>(), 1);
        assert!(d
            .counts
            .keys()
            .all(|k| k.starts_with("unsafe_op_in_unsafe_fn|")));
        d.message(
            Path::new("/repo"),
            &message("error", "E0133"),
            &BTreeMap::new(),
        )
        .unwrap();
        d.message(
            Path::new("/repo"),
            &message("failure-note", ""),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(d.errors.len(), 1);
    }

    #[test]
    fn force_warn_exposes_debt_under_a_broad_source_allow() {
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/clippy-fixtures");
        std::fs::create_dir_all(&target).unwrap();
        let fixture = tempfile::tempdir_in(target).unwrap();
        std::fs::create_dir(fixture.path().join("src")).unwrap();
        std::fs::write(
            fixture.path().join("Cargo.toml"),
            "[package]\nname=\"debt-fixture\"\nversion=\"0.0.0\"\nedition=\"2021\"\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(fixture.path().join("src/main.rs"), "#![allow(clippy::all)]\nfn value(x: Option<u8>) -> u8 { x.unwrap() }\nfn main() { value(Some(1)); std::mem::forget(String::from(\"owned\")); }\n").unwrap();
        let mut command = Command::new(crate::quality::cargo());
        command.current_dir(fixture.path()).args([
            "clippy",
            "--offline",
            "--message-format=json",
            "--",
            "-D",
            "warnings",
        ]);
        for lint in DEBT {
            command.args(["--force-warn", lint]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let warnings: BTreeSet<String> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .filter(|message| message["message"]["level"] == "warning")
            .map(|message| message["message"]["code"]["code"].as_str().unwrap().into())
            .collect();
        assert_eq!(
            warnings,
            ["clippy::unwrap_used".into(), "clippy::mem_forget".into()].into(),
            "source allow must not hide debt"
        );
    }

    #[test]
    fn mem_forget_is_counted_and_an_additional_hit_fails() {
        let mut diagnostics = Diagnostics::default();
        let root = Path::new("/repo");
        let ids = [(
            "src/savegame/encode.rs".into(),
            "uqm::savegame::encode::@file".into(),
        )]
        .into();
        let mut hit = message("warning", "clippy::mem_forget");
        diagnostics.message(root, &hit, &ids).unwrap();
        let key = "clippy::mem_forget|uqm::savegame::encode::@file";
        assert!(diagnostics.errors.is_empty());
        assert_eq!(diagnostics.counts.get(key), Some(&1));
        let baseline = diagnostics.counts.clone();
        assert!(tool::compare(&baseline, &BTreeMap::new(), &BTreeMap::new()).is_err());
        tool::compare(&diagnostics.counts, &baseline, &BTreeMap::new()).unwrap();

        hit["message"]["spans"][0]["line_start"] = 45.into();
        diagnostics.message(root, &hit, &ids).unwrap();
        assert_eq!(diagnostics.counts.get(key), Some(&2));
        let error = tool::compare(&diagnostics.counts, &baseline, &BTreeMap::new()).unwrap_err();
        assert!(error.contains(&format!("{key}: 1 -> 2")), "{error}");
    }

    #[test]
    fn new_debt_and_spent_headroom_fail() {
        let baseline = [("unwrap|uqm::file".into(), 2)].into();
        let measured = [("unwrap|uqm::file".into(), 3)].into();
        assert!(tool::compare(&measured, &baseline, &BTreeMap::new()).is_err());
        assert!(tool::compare(&measured, &BTreeMap::new(), &BTreeMap::new()).is_err());
    }
}
