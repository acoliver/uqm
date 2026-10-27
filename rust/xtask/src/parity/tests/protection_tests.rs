#[test]
fn initial_complete_golden_creation_needs_no_update_flag() {
    let directory = temporary();
    let root = directory.path();
    write(&root.join("seed.txt"), b"base without goldens").unwrap();
    commit_base(root);
    let manifest = fixture(root);
    let options = options::Options {
        base: Some("HEAD".into()),
        ..Default::default()
    };
    inventory::protect(root, &manifest, &options).unwrap();
    let source = root.join(GOLDEN).join("parity/comm/parity.trace");
    let renamed = root.join(GOLDEN).join("parity/comm/renamed.trace");
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "goldens fixture",
        ],
    );
    fs::rename(&source, &renamed).unwrap();
    assert!(inventory::protect_base(root, "HEAD", false).is_err());
}

use super::*;
use std::process::Command;

fn fixture(root: &Path) -> manifest::Manifest {
    let mut manifest = manifest::parse(&corpus_text()).unwrap();
    manifest.expectations.clear();
    for index in 0..73 {
        write(
            &root.join(format!("rust/scripts/saves/save-{index}.sav")),
            b"fixture",
        )
        .unwrap();
    }
    write(
        &root.join("rust/tests/fixtures/savegame/legacy-save.00"),
        b"fixture",
    )
    .unwrap();
    write(&root.join("rust/scripts/scenario.json"), b"{}").unwrap();
    let golden = root.join(GOLDEN);
    for path in manifest.expected_files(root, None).unwrap() {
        write(&golden.join(path), b"test fixture").unwrap();
    }
    let rows = BTreeMap::from([("scenario".into(), counts_tests::success_counts())]);
    write(
        &golden.join("parity/counts.toml"),
        counts::encode(&rows).unwrap().as_bytes(),
    )
    .unwrap();
    inventory::create(&golden).unwrap();
    manifest
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit_base(root: &Path) {
    git(root, &["init", "--quiet"]);
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
}

#[test]
fn inventory_covers_trace_fields_summaries_counts_and_all_74_save_pairs() {
    let directory = temporary();
    let root = directory.path();
    let manifest = fixture(root);
    let expected = manifest.expected_files(root, None).unwrap();
    assert_eq!(expected.len(), 20 * 3 + 1 + 74 * 2);
    inventory::verify(root, &root.join(GOLDEN), &manifest, None).unwrap();
    let value: serde_json::Value = json(&root.join(GOLDEN).join(INVENTORY)).unwrap();
    assert_eq!(value["files"].as_object().unwrap().len(), 209);
    assert_eq!(
        value["files"]["parity/comm/parity.trace"]["sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
}

#[test]
fn new_or_missing_goldens_require_exact_manifest_inventory_entries() {
    let directory = temporary();
    let root = directory.path();
    let manifest = fixture(root);
    let golden = root.join(GOLDEN);
    write(&golden.join("saves/new.capture"), b"not registered").unwrap();
    assert!(inventory::verify(root, &golden, &manifest, None)
        .unwrap_err()
        .contains("SHA256"));
    inventory::create(&golden).unwrap();
    assert!(inventory::verify(root, &golden, &manifest, None)
        .unwrap_err()
        .contains("unregistered"));
    fs::remove_file(golden.join("saves/new.capture")).unwrap();
    fs::remove_file(golden.join("saves/legacy-save.00.capture.json")).unwrap();
    inventory::create(&golden).unwrap();
    assert!(inventory::verify(root, &golden, &manifest, None)
        .unwrap_err()
        .contains("missing"));
    fs::remove_file(golden.join(INVENTORY)).unwrap();
    assert!(inventory::verify(root, &golden, &manifest, None).is_err());
}

#[test]
fn base_goldens_and_manifest_cannot_be_modified_deleted_or_renamed_without_explicit_flag() {
    let directory = temporary();
    let root = directory.path();
    let manifest = fixture(root);
    commit_base(root);
    inventory::protect_base(root, "HEAD", false).unwrap();
    let trace = root.join(GOLDEN).join("parity/comm/parity.trace");
    write(&trace, b"modified").unwrap();
    assert!(inventory::protect_base(root, "HEAD", false)
        .unwrap_err()
        .contains("existing golden"));
    inventory::protect_base(root, "HEAD", true).unwrap();
    fs::remove_file(&trace).unwrap();
    assert!(inventory::protect_base(root, "HEAD", false).is_err());
    inventory::protect_base(root, "HEAD", true).unwrap();
    write(&trace, b"test fixture").unwrap();
    inventory::create(&root.join(GOLDEN)).unwrap();
    inventory::protect_base(root, "HEAD", false).unwrap();
    write(&root.join(GOLDEN).join(INVENTORY), b"{}\n").unwrap();
    assert!(inventory::protect_base(root, "HEAD", false).is_err());
    let options = options::Options {
        base: Some("HEAD".into()),
        allow_golden_update: true,
        ..Default::default()
    };
    assert!(
        inventory::protect(root, &manifest, &options).is_err(),
        "allow update must not bypass manifest validity"
    );
    assert!(inventory::protect_base(root, "missing-ref", true).is_err());
}

#[test]
fn counts_inventory_must_match_every_script_even_with_fresh_hashes() {
    let directory = temporary();
    let root = directory.path();
    let manifest = fixture(root);
    write(&root.join("rust/scripts/new-script.json"), b"{}").unwrap();
    assert!(inventory::verify(root, &root.join(GOLDEN), &manifest, None)
        .unwrap_err()
        .contains("counts inventory"));
}

#[test]
fn recordings_never_write_goldens_and_reject_symlink_output() {
    let directory = temporary();
    let root = directory.path();
    fs::create_dir_all(root.join(GOLDEN)).unwrap();
    for path in [
        root.join(GOLDEN),
        root.join(GOLDEN).join("../golden/parity/out"),
        root.join("rust/tests"),
    ] {
        assert!(reject_golden_output(root, &path).is_err());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join(GOLDEN), root.join("alias")).unwrap();
        assert!(reject_golden_output(root, &root.join("alias/out")).is_err());
    }
}
