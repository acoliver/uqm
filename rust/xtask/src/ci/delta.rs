//! Base-to-head measurement of S4's zero-native ownership boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::authority::Authority;
use super::exec::{run_captured_with_limits, Captured};
use crate::hex_sha256;

const SCHEMA: &str = "uqm-s4-zero-native-delta-v1";
const LEDGER_PATH: &str = "rust/ci/native-ownership-ledger-v7.json";
const PROVIDER_PATH: &str = "rust/ownership/native-provider-manifest.json";
const NATIVE_INPUTS_PATH: &str = "rust/build/native-inputs.json";

#[derive(Debug, Serialize)]
pub struct DeltaReport {
    pub schema: String,
    pub base_sha: String,
    pub head_sha: String,
    pub categories: BTreeMap<String, DeltaCategory>,
    pub transitional_native_inputs: TransitionalNativeInputs,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct TransitionalNativeInputs {
    pub base_count: usize,
    pub head_count: usize,
    pub maximum_count: u32,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct DeltaCategory {
    pub measured_delta: usize,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Ledger {
    tracked_native_sources_and_build_inputs: Vec<PathRecord>,
    transitional_flags_and_features: Vec<FlagRecord>,
    rust_internal_ffi_files: Vec<PathRecord>,
}

#[derive(Debug, Deserialize)]
struct PathRecord {
    path: String,
}

#[derive(Debug, Deserialize)]
struct FlagRecord {
    flag: String,
}

pub fn measure(root: &Path, authority: &Authority, head_sha: &str) -> Result<DeltaReport, String> {
    let base_sha = env::var("UQM_CI_BASE_SHA")
        .map_err(|_| "source.base_sha: UQM_CI_BASE_SHA is required".to_string())?;
    measure_between(root, authority, &base_sha, head_sha)
}

fn measure_between(
    root: &Path,
    authority: &Authority,
    base_sha: &str,
    head_sha: &str,
) -> Result<DeltaReport, String> {
    validate_sha(base_sha, "UQM_CI_BASE_SHA")?;
    require_ancestor(root, authority, base_sha, head_sha)?;

    let ledger_bytes = super::bounded_io::read_regular_nofollow(
        &root.join(LEDGER_PATH),
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    if hex_sha256(&ledger_bytes) != authority.ledger_identity.sha256 {
        return Err("vendored V7 ledger hash differs from authority".into());
    }
    let ledger: Ledger = serde_json::from_slice(&ledger_bytes)
        .map_err(|error| format!("cannot parse {LEDGER_PATH}: {error}"))?;
    if ledger.tracked_native_sources_and_build_inputs.len() != 913
        || ledger.rust_internal_ffi_files.len() != 124
        || ledger.transitional_flags_and_features.len() != 48
    {
        return Err(
            "vendored V7 ledger inventory counts differ from the expected V7 contract".into(),
        );
    }
    let base_tree = tree(root, authority, base_sha)?;
    let head_tree = tree(root, authority, head_sha)?;

    let all_paths = base_tree.keys().chain(head_tree.keys());
    let sources = path_delta(
        ledger
            .tracked_native_sources_and_build_inputs
            .iter()
            .map(|record| record.path.as_str())
            .chain(
                all_paths
                    .clone()
                    .filter(|path| is_native_source(path))
                    .map(String::as_str),
            ),
        &base_tree,
        &head_tree,
    );
    // `bridges` measures expansion of the Rust/native boundary, not edits to
    // an implementation that already belongs to that boundary. Content
    // changes remain covered by the ordinary source, test, and ownership
    // gates; adding or removing a bridge path is the zero-delta concern here.
    let bridges = path_membership_delta(
        ledger
            .rust_internal_ffi_files
            .iter()
            .map(|record| record.path.as_str())
            .chain(
                all_paths
                    .clone()
                    .filter(|path| is_bridge(path))
                    .map(String::as_str),
            ),
        &base_tree,
        &head_tree,
    );
    let generated_binding_paths = all_paths
        .filter(|path| is_generated_binding(path))
        .collect::<BTreeSet<_>>();
    if generated_binding_paths.is_empty() {
        return Err("generated-binding inventory is empty".into());
    }
    let generated_bindings = path_delta(
        generated_binding_paths.iter().map(|path| path.as_str()),
        &base_tree,
        &head_tree,
    );
    let base_flags = flag_projection(root, authority, base_sha, &ledger)?;
    let head_flags = flag_projection(root, authority, head_sha, &ledger)?;
    let flags = projection_delta(&base_flags, &head_flags);

    let base_manifest = git_json(root, authority, base_sha, PROVIDER_PATH)?;
    let head_manifest_bytes = super::bounded_io::read_regular_nofollow(
        &root.join(PROVIDER_PATH),
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let head_manifest: Value = serde_json::from_slice(&head_manifest_bytes)
        .map_err(|error| format!("cannot parse {PROVIDER_PATH}: {error}"))?;
    let providers = value_delta(
        provider_projection(&base_manifest),
        provider_projection(&head_manifest),
        PROVIDER_PATH,
    );
    let objects = value_delta(
        object_projection(&base_manifest),
        object_projection(&head_manifest),
        PROVIDER_PATH,
    );
    let symbols = value_delta(
        base_manifest.get("symbol_contracts").cloned(),
        head_manifest.get("symbol_contracts").cloned(),
        PROVIDER_PATH,
    );

    let categories = BTreeMap::from([
        ("tracked_sources".into(), category(sources)),
        ("providers".into(), category(providers)),
        ("objects".into(), category(objects)),
        ("internal_symbols".into(), category(symbols)),
        ("bridges".into(), category(bridges)),
        ("generated_bindings".into(), category(generated_bindings)),
        ("transitional_flags".into(), category(flags)),
    ]);
    let base_input_count =
        native_input_count(&git_json(root, authority, base_sha, NATIVE_INPUTS_PATH)?)?;
    let head_input_count =
        native_input_count(&git_json(root, authority, head_sha, NATIVE_INPUTS_PATH)?)?;
    let transitional_native_inputs = transitional_native_inputs(
        base_input_count,
        head_input_count,
        authority
            .zero_native_delta
            .maximum_transitional_native_inputs,
    );
    let passed = categories
        .values()
        .all(|category| category.measured_delta == 0)
        && transitional_native_inputs.passed;
    Ok(DeltaReport {
        schema: SCHEMA.into(),
        base_sha: base_sha.into(),
        head_sha: head_sha.into(),
        categories,
        transitional_native_inputs,
        passed,
    })
}

fn category(changed_paths: Vec<String>) -> DeltaCategory {
    DeltaCategory {
        measured_delta: changed_paths.len(),
        changed_paths,
    }
}

fn native_input_count(manifest: &Value) -> Result<usize, String> {
    manifest
        .get("inputs")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| format!("{NATIVE_INPUTS_PATH} has no inputs array"))
}

fn transitional_native_inputs(
    base_count: usize,
    head_count: usize,
    maximum_count: u32,
) -> TransitionalNativeInputs {
    TransitionalNativeInputs {
        base_count,
        head_count,
        maximum_count,
        passed: head_count <= base_count && head_count <= maximum_count as usize,
    }
}

fn is_native_source(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    [
        ".c", ".h", ".cc", ".hh", ".cpp", ".hpp", ".cxx", ".hxx", ".m", ".mm", ".s", ".asm",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
        || file == "makefile"
        || file.starts_with("makefile.")
        || lower.ends_with(".mk")
}

fn is_bridge(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with("_ffi.rs")
        || lower.ends_with("_bridge.rs")
        || lower.contains("/ffi/")
        || lower.contains("/bridge/")
}

fn is_generated_binding(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with("_bindings.rs")
        || lower.ends_with("/bindings.rs")
        || lower.ends_with("_generated.rs")
        || lower.ends_with("/generated.rs")
}

fn path_delta<'a>(
    paths: impl Iterator<Item = &'a str>,
    base: &BTreeMap<String, String>,
    head: &BTreeMap<String, String>,
) -> Vec<String> {
    paths
        .filter(|path| base.get(*path) != head.get(*path))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn path_membership_delta<'a>(
    paths: impl Iterator<Item = &'a str>,
    base: &BTreeMap<String, String>,
    head: &BTreeMap<String, String>,
) -> Vec<String> {
    paths
        .filter(|path| base.contains_key(*path) != head.contains_key(*path))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn value_delta(base: Option<Value>, head: Option<Value>, path: &str) -> Vec<String> {
    if base == head {
        Vec::new()
    } else {
        vec![path.into()]
    }
}

fn provider_projection(manifest: &Value) -> Option<Value> {
    let objects = manifest.get("objects")?.as_array()?;
    let recompiled = manifest.get("recompiled_objects")?.as_array()?;
    Some(serde_json::json!({
        "objects": objects.iter().map(|object| serde_json::json!({
            "path": object.get("path"),
            "provider": object.get("provider"),
            "archive_decision": object.get("archive_decision"),
        })).collect::<Vec<_>>(),
        "recompiled": recompiled.iter().map(|object| serde_json::json!({
            "canonical_source": object.get("canonical_source"),
            "provider": object.get("provider"),
            "owner": object.get("owner"),
        })).collect::<Vec<_>>(),
    }))
}

fn object_projection(manifest: &Value) -> Option<Value> {
    Some(serde_json::json!({
        "objects": manifest.get("objects")?,
        "recompiled_objects": manifest.get("recompiled_objects")?,
    }))
}

fn supervised_git(
    root: &Path,
    authority: &Authority,
    arguments: &[String],
    label: &str,
    accepted_exit_codes: &[i32],
) -> Result<Captured, String> {
    let arguments: Vec<String> = [
        "-c".to_string(),
        format!("safe.directory={}", root.display()),
    ]
    .into_iter()
    .chain(arguments.iter().cloned())
    .collect();
    let captured = run_captured_with_limits(
        root,
        "git",
        &arguments,
        &[],
        authority.supervision.builtin_limits(),
    );
    if !captured.completed_under_supervision()
        || captured.signal.is_some()
        || !captured
            .exit_code
            .is_some_and(|code| accepted_exit_codes.contains(&code))
    {
        return Err(captured.failure_detail(label));
    }
    Ok(captured)
}

fn normal_git_path(bytes: &[u8]) -> Result<String, String> {
    let path = std::str::from_utf8(bytes)
        .map_err(|error| format!("git tree path is not UTF-8: {error}"))?;
    if path.is_empty()
        || Path::new(path)
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "git tree path is not a normal relative path: {path:?}"
        ));
    }
    Ok(path.to_string())
}

fn parse_tree(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let mut entries = BTreeMap::new();
    for row in bytes.split(|byte| *byte == 0).filter(|row| !row.is_empty()) {
        let tab = row
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| "malformed NUL-delimited git tree row".to_string())?;
        let metadata = std::str::from_utf8(&row[..tab])
            .map_err(|error| format!("git tree metadata is not UTF-8: {error}"))?;
        let path = normal_git_path(&row[tab + 1..])?;
        let object = metadata
            .split_whitespace()
            .nth(2)
            .ok_or_else(|| format!("malformed git tree metadata: {metadata}"))?;
        if entries.insert(path.clone(), object.into()).is_some() {
            return Err(format!("duplicate git tree path: {path:?}"));
        }
    }
    Ok(entries)
}

fn tree(
    root: &Path,
    authority: &Authority,
    commit: &str,
) -> Result<BTreeMap<String, String>, String> {
    let output = supervised_git(
        root,
        authority,
        &["ls-tree".into(), "-rz".into(), commit.into()],
        &format!("inspect tree {commit}"),
        &[0],
    )?;
    parse_tree(&output.stdout)
}

fn git_json(root: &Path, authority: &Authority, commit: &str, path: &str) -> Result<Value, String> {
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

fn parse_grep_row<'a>(row: &'a str, prefix: &str) -> Option<(&'a str, &'a str)> {
    let row = row.strip_prefix(prefix)?;
    for (separator, _) in row.match_indices(':') {
        let remainder = &row[separator + 1..];
        let Some(line_end) = remainder.find(':') else {
            continue;
        };
        let line = &remainder[..line_end];
        if !line.is_empty() && line.bytes().all(|byte| byte.is_ascii_digit()) {
            return Some((&row[..separator], &remainder[line_end + 1..]));
        }
    }
    None
}

fn flag_projection(
    root: &Path,
    authority: &Authority,
    commit: &str,
    ledger: &Ledger,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let grep_arguments = |mode: &str, paths: &[String]| {
        let mut arguments = vec!["grep".into(), "-I".into(), mode.into()];
        for record in &ledger.transitional_flags_and_features {
            arguments.extend(["-e".into(), record.flag.clone()]);
        }
        arguments.extend([commit.into(), "--".into()]);
        arguments.extend(paths.iter().cloned());
        arguments
    };
    let roots = ["rust".into(), "rast".into(), "sc2".into()];
    let listed = supervised_git(
        root,
        authority,
        &grep_arguments("-l", &roots),
        "locate transitional flag definition files",
        &[0, 1],
    )?;
    let prefix = format!("{commit}:");
    let listed = std::str::from_utf8(&listed.stdout)
        .map_err(|error| format!("transitional flag file list is not UTF-8: {error}"))?;
    let paths = listed
        .lines()
        .filter_map(|row| row.strip_prefix(&prefix))
        .filter(|path| is_flag_definition_path(path))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(BTreeMap::new());
    }
    let output = supervised_git(
        root,
        authority,
        &grep_arguments("-n", &paths),
        "read transitional flag definitions",
        &[0],
    )?;
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|error| format!("transitional flag grep output is not UTF-8: {error}"))?;
    let mut projection = BTreeMap::<String, Vec<String>>::new();
    for row in text.lines() {
        let (path, content) = parse_grep_row(row, &prefix)
            .ok_or_else(|| format!("malformed transitional flag grep row: {row}"))?;
        projection
            .entry(path.into())
            .or_default()
            .push(content.trim().to_string());
    }
    for lines in projection.values_mut() {
        lines.sort();
        lines.dedup();
    }
    Ok(projection)
}

fn projection_delta(
    base: &BTreeMap<String, Vec<String>>,
    head: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    base.keys()
        .chain(head.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| base.get(*path) != head.get(*path))
        .cloned()
        .collect()
}

fn is_flag_definition_path(path: &str) -> bool {
    if path.starts_with("rust/ci/")
        || path.starts_with("rust/xtask/")
        || path.starts_with("rust/target/")
    {
        return false;
    }
    path.starts_with("rust/")
        || path.starts_with("rast/src/")
        || path.starts_with("sc2/src/")
        || path == "sc2/build.vars.in"
        || path.starts_with("sc2/Make")
}

fn require_ancestor(
    root: &Path,
    authority: &Authority,
    base: &str,
    head: &str,
) -> Result<(), String> {
    let captured = supervised_git(
        root,
        authority,
        &[
            "merge-base".into(),
            "--is-ancestor".into(),
            base.into(),
            head.into(),
        ],
        "validate base-to-head ancestry",
        &[0, 1],
    )?;
    if captured.exit_code == Some(0) {
        Ok(())
    } else {
        Err(format!(
            "source.base_sha: {base} is not an ancestor of {head}"
        ))
    }
}

fn validate_sha(value: &str, name: &str) -> Result<(), String> {
    if value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(format!(
            "{name} must be exactly 40 lowercase hexadecimal characters"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_tree_parser_preserves_paths_that_git_would_quote() {
        let hash = "a".repeat(40);
        let path = "rust/src/file with \"quotes\".rs";
        let row = format!("100644 blob {hash}\t{path}\0");
        let parsed = parse_tree(row.as_bytes()).unwrap();
        assert_eq!(parsed.get(path), Some(&hash));
    }

    #[test]
    fn nul_tree_parser_rejects_non_normal_and_non_utf8_paths() {
        let hash = "a".repeat(40);
        assert!(parse_tree(format!("100644 blob {hash}\t../escape\0").as_bytes()).is_err());
        let mut non_utf8 = format!("100644 blob {hash}\tbad-").into_bytes();
        non_utf8.extend([0xff, 0]);
        assert!(parse_tree(&non_utf8).is_err());
    }

    #[test]
    fn classifiers_cover_native_bridges_and_generated_bindings() {
        assert!(is_native_source("sc2/src/native.c"));
        assert!(is_native_source("sc2/src/Makefile"));
        assert!(is_bridge("rust/src/io/uio_bridge.rs"));
        assert!(is_bridge("rust/src/automation/input_ffi.rs"));
        assert!(is_generated_binding("rust/src/c_bindings.rs"));
        assert!(!is_generated_binding("rust/harness/menu_binding_probe.c"));
    }

    #[test]
    fn transitional_flag_projection_excludes_control_plane_and_docs() {
        assert!(is_flag_definition_path("rust/Cargo.toml"));
        assert!(is_flag_definition_path("sc2/src/config_unix.h"));
        assert!(!is_flag_definition_path("rust/xtask/src/main.rs"));
        assert!(!is_flag_definition_path("rust/ci/gates.json"));
        assert!(!is_flag_definition_path("dev-docs/rust/ci-gates.md"));
    }

    #[test]
    fn added_native_source_is_a_nonzero_measured_delta() {
        let base: BTreeMap<String, String> = BTreeMap::new();
        let head: BTreeMap<String, String> =
            BTreeMap::from([("sc2/src/new_provider.c".into(), "object-id".into())]);
        let changed = path_delta(
            base.keys()
                .chain(head.keys())
                .filter(|path| is_native_source(path))
                .map(String::as_str),
            &base,
            &head,
        );
        assert_eq!(changed, ["sc2/src/new_provider.c"]);
    }

    #[test]
    fn existing_bridge_implementation_edits_do_not_expand_the_bridge_surface() {
        let path = "rust/src/automation/input_ffi.rs";
        let base: BTreeMap<String, String> =
            BTreeMap::from([(path.into(), "old-object-id".into())]);
        let head: BTreeMap<String, String> =
            BTreeMap::from([(path.into(), "new-object-id".into())]);
        let changed = path_membership_delta(
            base.keys()
                .chain(head.keys())
                .filter(|path| is_bridge(path))
                .map(String::as_str),
            &base,
            &head,
        );
        assert!(changed.is_empty());
    }

    #[test]
    fn added_and_removed_bridge_paths_change_the_bridge_surface() {
        let existing = "rust/src/automation/input_ffi.rs";
        let added = "rust/src/automation/new_bridge.rs";
        let removed = "rust/src/legacy/ffi/old.rs";
        let base: BTreeMap<String, String> = BTreeMap::from([
            (existing.into(), "base-existing".into()),
            (removed.into(), "base-removed".into()),
        ]);
        let head: BTreeMap<String, String> = BTreeMap::from([
            (existing.into(), "head-existing".into()),
            (added.into(), "head-added".into()),
        ]);
        let changed = path_membership_delta(
            base.keys()
                .chain(head.keys())
                .filter(|path| is_bridge(path))
                .map(String::as_str),
            &base,
            &head,
        );
        assert_eq!(changed, [added, removed]);
    }

    #[test]
    fn transitional_native_inputs_may_decrease_but_not_increase_or_exceed_the_ceiling() {
        assert!(transitional_native_inputs(321, 320, 321).passed);
        assert!(!transitional_native_inputs(320, 321, 321).passed);
        assert!(!transitional_native_inputs(322, 322, 321).passed);
    }

    #[test]
    fn identical_commit_has_zero_delta_with_nonempty_inventories() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf();
        let authority = super::super::authority::load_authority(&root).unwrap();
        let head = crate::git_text(&root, &["rev-parse", "HEAD"], "read test HEAD").unwrap();
        let report = measure_between(&root, &authority, &head, &head).unwrap();
        assert!(report.passed, "{report:#?}");
        assert_eq!(report.categories.len(), 7);
        assert!(report
            .categories
            .values()
            .all(|category| category.measured_delta == 0));
        assert_eq!(
            report.transitional_native_inputs.base_count,
            report.transitional_native_inputs.head_count
        );
        assert_eq!(report.transitional_native_inputs.head_count, 321);
        assert_eq!(report.transitional_native_inputs.maximum_count, 321);
        assert!(report.transitional_native_inputs.passed);
    }
}
