use super::*;
use std::fs;
use std::process::Command;

const SOURCE: &str = "sc2/src/retired.c";
const HEADER: &str = "sc2/src/api.h";
const RUST_PATH: &str = "rust/src/replacement.rs";
const C_SOURCE: &str = "#include \"api.h\"\nvoid Existing(BYTE value) { (void)value; }\n";
const C_HEADER: &str = "#include <unistd.h>\ntypedef unsigned char BYTE;\nvoid Existing(BYTE);\n";
const RUST_SOURCE: &str = r#"
use crate::c_bindings::{global_state as c};
const _: unsafe extern "C" fn(c::BYTE) = c::Existing;
const _: unsafe extern "C" fn(c::BYTE) = replacement;
#[export_name = "Existing"]
pub unsafe extern "C" fn replacement(value: c::BYTE) { let _ = value; }
"#;

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn write(root: &Path, path: &str, value: impl AsRef<[u8]>) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value).unwrap();
}

fn profile() -> Value {
    serde_json::json!({"id":"production", "cargo_features":["audio_heart", "linked_c_archive"], "defines":[], "compile_flags":["-std=gnu99", "-O2", "-fPIC"]})
}

fn inputs(active: bool) -> Value {
    serde_json::json!({"production_profile":profile(), "inputs": if active { vec![serde_json::json!({
        "source":SOURCE, "source_sha256":hex_sha256(C_SOURCE.as_bytes()),
        "provider":"native_object:native/retired.c.o"
    })] } else { vec![] }})
}

fn manifest(active: bool) -> Value {
    serde_json::json!({
        "objects": if active { vec![serde_json::json!({
            "path":"native/retired.c.o", "issue":"DOMAIN", "canonical_source":SOURCE,
            "provider":"native_object:native/retired.c.o", "archive_decision":"include"
        })] } else { vec![] },
        "recompiled_objects":[], "symbol_contracts": if active { vec![] } else { vec![serde_json::json!({
            "symbol":"Existing", "canonical_owner":"DOMAIN/#123",
            "active_provider":{"kind":"rust_source", "path":RUST_PATH},
            "excluded_providers":[{"kind":"native_object", "path":"native/retired.c.o"}]
        })] }
    })
}

struct Fixture {
    root: tempfile::TempDir,
    base: String,
    base_manifest: Value,
    head_manifest: Value,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), &["init", "-q"]);
        git(root.path(), &["config", "user.name", "Fixture"]);
        git(
            root.path(),
            &["config", "user.email", "fixture@example.invalid"],
        );
        write(root.path(), SOURCE, C_SOURCE);
        write(root.path(), HEADER, C_HEADER);
        let base_manifest = manifest(true);
        write(
            root.path(),
            PROVIDER_PATH,
            serde_json::to_vec(&base_manifest).unwrap(),
        );
        write(
            root.path(),
            NATIVE_INPUTS_PATH,
            serde_json::to_vec(&inputs(true)).unwrap(),
        );
        git(root.path(), &["add", "."]);
        git(root.path(), &["commit", "-qm", "base"]);
        let base = git(root.path(), &["rev-parse", "HEAD"]);
        fs::remove_file(root.path().join(SOURCE)).unwrap();
        write(root.path(), RUST_PATH, RUST_SOURCE);
        write(
            root.path(),
            NATIVE_INPUTS_PATH,
            serde_json::to_vec(&inputs(false)).unwrap(),
        );
        Self {
            root,
            base,
            base_manifest,
            head_manifest: manifest(false),
        }
    }

    fn result(&self) -> Vec<PathVerdict> {
        write(
            self.root.path(),
            PROVIDER_PATH,
            serde_json::to_vec(&self.head_manifest).unwrap(),
        );
        git(self.root.path(), &["add", "."]);
        git(self.root.path(), &["commit", "-qm", "head"]);
        let head = git(self.root.path(), &["rev-parse", "HEAD"]);
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let authority = super::super::authority::load_authority(repository).unwrap();
        let base_tree = tree(self.root.path(), &authority, &self.base).unwrap();
        let head_tree = tree(self.root.path(), &authority, &head).unwrap();
        RetirementContext {
            root: self.root.path(),
            authority: &authority,
            base_sha: &self.base,
            head_sha: &head,
            base_tree: &base_tree,
            head_tree: &head_tree,
            base_manifest: &self.base_manifest,
            head_manifest: &self.head_manifest,
        }
        .symbol_verdicts()
        .unwrap()
    }
}

#[test]
fn new_measurement_matches_original_definition_header_owner_and_retired_compilation_input() {
    let verdicts = Fixture::new().result();
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0].verdict, Verdict::Pass, "{verdicts:?}");
    assert!(verdicts[0]
        .reason
        .contains("original external C definition and unchanged header verified"));
}

#[test]
fn invented_symbol_changed_signature_nonretired_provider_and_unsupported_paths_fail() {
    for change in 0..10 {
        let mut fixture = Fixture::new();
        match change {
            0 => fixture.head_manifest["symbol_contracts"][0]["symbol"] = "Invented".into(),
            1 => write(
                fixture.root.path(),
                RUST_PATH,
                RUST_SOURCE.replace("value: c::BYTE", "value: u16"),
            ),
            2 => fixture.head_manifest["objects"] = fixture.base_manifest["objects"].clone(),
            3 => write(fixture.root.path(), SOURCE, C_SOURCE),
            4 => {
                fixture.head_manifest["symbol_contracts"][0]["active_provider"]["path"] =
                    "../escape.rs".into()
            }
            5 => {
                fixture.head_manifest["symbol_contracts"][0]["excluded_providers"] =
                    serde_json::json!([])
            }
            6 => {
                fixture.head_manifest["symbol_contracts"][0]["excluded_providers"][0]["path"] =
                    "native/fabricated.c.o".into()
            }
            7 => {
                fixture.head_manifest["symbol_contracts"][0]["canonical_owner"] =
                    "OTHER/#123".into()
            }
            8 => write(
                fixture.root.path(),
                NATIVE_INPUTS_PATH,
                serde_json::to_vec(&inputs(true)).unwrap(),
            ),
            9 => write(
                fixture.root.path(),
                RUST_PATH,
                RUST_SOURCE.replace(
                    "const _: unsafe extern \"C\" fn(c::BYTE) = replacement;",
                    "",
                ),
            ),
            _ => unreachable!(),
        }
        let verdicts = fixture.result();
        assert_eq!(verdicts.len(), 1);
        assert_eq!(
            verdicts[0].verdict,
            Verdict::Fail,
            "case {change}: {verdicts:?}"
        );
    }
}

#[test]
fn changed_header_and_head_only_declaration_cannot_supply_the_base_proof() {
    for header in [
        C_HEADER.replace("BYTE);", "unsigned short);"),
        format!("{C_HEADER}\nvoid Invented(BYTE);\n"),
    ] {
        let fixture = Fixture::new();
        write(fixture.root.path(), HEADER, header);
        let verdicts = fixture.result();
        assert_eq!(verdicts[0].verdict, Verdict::Fail, "{verdicts:?}");
    }
}

#[test]
fn head_json_cannot_supply_a_base_object_or_reassign_an_existing_measured_symbol() {
    let mut fixture = Fixture::new();
    fixture.base_manifest["objects"] = serde_json::json!([]);
    assert_eq!(fixture.result()[0].verdict, Verdict::Fail);
    let mut fixture = Fixture::new();
    fixture.base_manifest["symbol_contracts"] = serde_json::json!([{
        "symbol":"Existing", "canonical_owner":"OLD/#1", "active_provider":{"kind":"rust_source", "path":"rust/src/old.rs"}
    }]);
    assert_eq!(fixture.result()[0].verdict, Verdict::Fail);
}

#[test]
fn production_scripts_forward_empty_and_nonempty_arguments_under_system_bash() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = tempfile::tempdir().unwrap();
    let controller = fixture.path().join("controller");
    fs::write(
        &controller,
        "#!/bin/sh\nprintf '%s\\0' \"$@\" > \"$UQM_TEST_ARGUMENTS\"\n",
    )
    .unwrap();
    fs::set_permissions(&controller, fs::Permissions::from_mode(0o700)).unwrap();
    for script in [
        include_str!("../../../probes/run_p00_probes.sh"),
        include_str!("../../../harness/run_p00_harness.sh"),
        include_str!("../../../harness/run_menu_binding_probe.sh"),
    ] {
        let forwarding = script
            .lines()
            .filter(|line| {
                line.starts_with("RETIREMENT_ARGUMENTS=")
                    || line.contains("\"${UQM_CI_CONTROLLER_EXECUTABLE}\" __ci-production")
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(forwarding.contains("__ci-production"));
        for arguments in [
            &[][..],
            &["--witness", "path with spaces", "", "line\nbreak"][..],
        ] {
            let retained = fixture.path().join("arguments");
            let output = Command::new("/bin/bash")
                .args(["-euc", &forwarding, "--"])
                .args(arguments)
                .env("UQM_CI_CONTROLLER_EXECUTABLE", &controller)
                .env("UQM_TEST_ARGUMENTS", &retained)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "arguments {arguments:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let expected = std::iter::once("__ci-production")
                .chain(arguments.iter().copied())
                .flat_map(|argument| argument.as_bytes().iter().copied().chain([0]))
                .collect::<Vec<_>>();
            assert_eq!(fs::read(retained).unwrap(), expected);
        }
    }
}
