use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::quality::baseline::Baseline;

const NATIVE: &[&str] = &[
    "crates/engine/src/graphics/sdl/opengl.rs",
    "crates/engine/src/graphics/sdl/sdl2.rs",
    "crates/engine/src/graphics/ffi_video_macos.rs",
];

pub fn read(document: &toml_edit::DocumentMut) -> Result<BTreeMap<String, String>, String> {
    let Some(item) = document.get("coverage_debt") else {
        return Ok(BTreeMap::new());
    };
    item.as_table()
        .ok_or("invalid coverage_debt section")?
        .iter()
        .map(|(path, unit)| {
            Ok((
                path.into(),
                unit.as_str().ok_or("coverage debt needs a unit")?.into(),
            ))
        })
        .collect()
}

pub fn write(document: &mut toml_edit::DocumentMut, debt: &BTreeMap<String, String>) {
    if !debt.is_empty() {
        let mut table = toml_edit::Table::new();
        for (path, unit) in debt {
            table.insert(path, toml_edit::value(unit));
        }
        document.insert("coverage_debt", toml_edit::Item::Table(table));
    }
}

fn deleted_paths(spec: &str, unit: &str) -> BTreeSet<String> {
    let mut in_unit = false;
    let mut in_deletes = false;
    let mut paths = BTreeSet::new();
    for line in spec.lines() {
        if line.starts_with("### U") {
            in_unit = line.split_whitespace().nth(1) == Some(unit);
            in_deletes = false;
        }
        if !in_unit {
            continue;
        }
        if line.starts_with("**Deletes**") {
            in_deletes = true;
        } else if line.starts_with("**") {
            in_deletes = false;
        }
        if in_deletes {
            for word in line.split(|c: char| c.is_whitespace() || c == '`' || c == '(' || c == ')')
            {
                expand_path(word.trim_end_matches([',', ';']), &mut paths);
            }
        }
    }
    paths
}

fn expand_path(word: &str, paths: &mut BTreeSet<String>) {
    if let Some((prefix, rest)) = word.split_once('{') {
        if let Some((choices, suffix)) = rest.split_once('}') {
            for choice in choices.split(',') {
                expand_path(&format!("{prefix}{choice}{suffix}"), paths);
            }
        }
    } else if word.ends_with(".rs") && word.contains('/') {
        paths.insert(word.trim_start_matches("rust/").into());
    }
}

fn validate(
    current: &BTreeMap<String, String>,
    before: &BTreeMap<String, String>,
    moved: &BTreeSet<String>,
    existing: &BTreeSet<String>,
    spec: &str,
) -> Result<(), String> {
    for (path, unit) in current {
        if !existing.contains(path) {
            return Err(format!("remove coverage debt for absent file {path}"));
        }
        if before.get(path) != Some(unit) && (before.contains_key(path) || !moved.contains(path)) {
            return Err(format!(
                "coverage debt can only be added for a moved file: {path}"
            ));
        }
        if unit == "native" {
            if !NATIVE.contains(&path.as_str()) {
                return Err(format!("{path} is not a native-window backend"));
            }
        } else {
            let original = path.strip_prefix("crates/engine/src/").unwrap_or(path);
            if !deleted_paths(spec, unit)
                .iter()
                .any(|deleted| deleted == path || deleted == original)
            {
                return Err(format!("{unit} Deletes does not list {path}"));
            }
        }
    }
    Ok(())
}

fn moved_files(root: &Path, base: &str) -> Result<BTreeSet<String>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "diff",
            "--name-status",
            "--find-renames=50%",
            "-z",
            base,
            "--",
            "rust",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git diff: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let mut fields = text.split_terminator('\0');
    let mut moved = BTreeSet::new();
    while let Some(status) = fields.next() {
        fields.next().ok_or("missing diff source path")?;
        if status.starts_with('R') {
            let target = fields.next().ok_or("missing diff move target")?;
            if let Some(path) = target.strip_prefix("rust/") {
                moved.insert(path.into());
            }
        }
    }
    Ok(moved)
}

pub fn exemptions(
    root: &Path,
    base: Option<&str>,
    current: &Baseline,
) -> Result<BTreeSet<String>, String> {
    if current.coverage_debt.is_empty() {
        return Ok(BTreeSet::new());
    }
    let base = base.unwrap_or("HEAD");
    let before = crate::quality::baseline::baseline_at_ref(root, base)?.unwrap_or_default();
    let moved = moved_files(root, base)?;
    let existing = current
        .coverage_debt
        .keys()
        .filter(|path| root.join("rust").join(path).is_file())
        .cloned()
        .collect();
    let spec = fs::read_to_string(root.join("dev-docs/design/idiomatic-rust/work-items.md"))
        .map_err(|e| e.to_string())?;
    validate(
        &current.coverage_debt,
        &before.coverage_debt,
        &moved,
        &existing,
        &spec,
    )?;
    Ok(current.coverage_debt.keys().cloned().collect())
}

pub fn edit(root: &Path, args: &[String]) -> Result<(), String> {
    let file = root.join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&file)?;
    match args {
        [operation, path] if operation == "remove" => {
            baseline.coverage_debt.remove(path).ok_or("coverage debt entry not found")?;
        }
        [path, unit, option, base] if option == "--base" => {
            baseline.coverage_debt.insert(path.clone(), unit.clone());
            exemptions(root, Some(base), &baseline)?;
        }
        _ => return Err("usage: coverage debt <path> <unit|native> --base <ref>, or coverage debt remove <path>".into()),
    }
    baseline.write(&file)
}

#[cfg(test)]
mod tests {
    use super::*;
    const FILE: &str = "crates/engine/src/graphics/widgets_abi.rs";
    const SPEC: &str = "### U7-02 Assets\n**Touches**: graphics/widgets_abi.rs\n**Deletes**\n- graphics/{font_abi,widgets_abi}.rs\n**Acceptance**\n";

    #[test]
    fn deleting_unit_must_list_the_file_in_deletes_not_touches() {
        let debt = BTreeMap::from([(FILE.into(), "U7-02".into())]);
        let paths = BTreeSet::from([FILE.into()]);
        assert!(validate(&debt, &BTreeMap::new(), &paths, &paths, SPEC).is_ok());
        let wrong = SPEC.replace("font_abi,widgets_abi", "font_abi,frame_abi");
        assert!(validate(&debt, &BTreeMap::new(), &paths, &paths, &wrong)
            .unwrap_err()
            .contains("Deletes does not list"));
        let wrong_unit = BTreeMap::from([(FILE.into(), "U7-01".into())]);
        assert!(validate(&wrong_unit, &BTreeMap::new(), &paths, &paths, SPEC).is_err());
    }

    #[test]
    fn entries_are_shrink_only_except_for_new_moves_and_native_is_restricted() {
        let debt = BTreeMap::from([(FILE.into(), "U7-02".into())]);
        let paths = BTreeSet::from([FILE.into()]);
        assert!(validate(&debt, &BTreeMap::new(), &BTreeSet::new(), &paths, SPEC).is_err());
        assert!(validate(&debt, &debt, &BTreeSet::new(), &paths, SPEC).is_ok());
        assert!(validate(&debt, &debt, &BTreeSet::new(), &BTreeSet::new(), SPEC).is_err());
        assert!(validate(&BTreeMap::new(), &debt, &paths, &paths, SPEC).is_ok());
        let native = BTreeMap::from([(FILE.into(), "native".into())]);
        assert!(validate(&native, &BTreeMap::new(), &paths, &paths, SPEC).is_err());
        for file in NATIVE {
            let native = BTreeMap::from([(file.to_string(), "native".into())]);
            let paths = BTreeSet::from([file.to_string()]);
            assert!(validate(&native, &BTreeMap::new(), &paths, &paths, SPEC).is_ok());
        }
    }

    #[test]
    fn baseline_round_trip_preserves_coverage_debt() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("baseline.toml");
        let baseline = Baseline {
            coverage_debt: BTreeMap::from([(FILE.into(), "U7-02".into())]),
            ..Baseline::default()
        };
        baseline.write(&file).unwrap();
        assert_eq!(Baseline::read(&file).unwrap(), baseline);
    }
}

#[cfg(test)]
mod command_tests {
    use super::*;

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn command_requires_a_real_move_and_retained_debt_needs_no_new_move() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let file = "crates/engine/src/graphics/widgets_abi.rs";
        let old = root.join("rust/src/graphics/widgets_abi.rs");
        let new = root.join("rust").join(file);
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::create_dir_all(new.parent().unwrap()).unwrap();
        fs::create_dir_all(root.join("rust/xtask")).unwrap();
        fs::create_dir_all(root.join("dev-docs/design/idiomatic-rust")).unwrap();
        fs::write(&old, "pub fn widget() {}\n").unwrap();
        fs::write(
            root.join("dev-docs/design/idiomatic-rust/work-items.md"),
            "### U7-02 Assets\n**Deletes**\n- graphics/widgets_abi.rs\n**Acceptance**\n",
        )
        .unwrap();
        let baseline = root.join("rust/xtask/quality-baseline.toml");
        Baseline::default().write(&baseline).unwrap();
        git(root, &["init", "-q"]);
        git(root, &["add", "."]);
        git(
            root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-qm",
                "base",
            ],
        );
        fs::rename(old, &new).unwrap();
        git(root, &["add", "."]);
        let args = [file.into(), "U7-02".into(), "--base".into(), "HEAD".into()];
        edit(root, &args).unwrap();
        let debt = Baseline::read(&baseline).unwrap();
        assert_eq!(
            exemptions(root, Some("HEAD"), &debt).unwrap(),
            BTreeSet::from([file.into()])
        );
        git(root, &["add", "."]);
        git(
            root,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-qm",
                "move",
            ],
        );
        assert_eq!(exemptions(root, None, &debt).unwrap().len(), 1);
        let other = "crates/engine/src/graphics/font_abi.rs";
        fs::write(root.join("rust").join(other), "pub fn font() {}\n").unwrap();
        git(root, &["add", "."]);
        let args = [other.into(), "U7-02".into(), "--base".into(), "HEAD".into()];
        assert!(edit(root, &args).unwrap_err().contains("moved file"));
        assert_eq!(Baseline::read(&baseline).unwrap(), debt);
        fs::remove_file(new).unwrap();
        assert!(exemptions(root, None, &debt)
            .unwrap_err()
            .contains("absent file"));
        edit(root, &["remove".into(), file.into()]).unwrap();
        assert!(Baseline::read(&baseline).unwrap().coverage_debt.is_empty());
        assert!(edit(root, &["remove".into(), file.into()]).is_err());
        assert!(edit(root, &[]).is_err());
    }
}
