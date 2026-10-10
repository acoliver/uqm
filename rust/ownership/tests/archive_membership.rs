//! Archive membership validation against a real `ar` and a real archive.
//!
//! The archive is built from the exact member names the checked-in manifest
//! selects, so these tests fail if the validator stops comparing what `ar -t`
//! reports with what the manifest declares.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use uqm_ownership::{DiagnosticCode, Manifest, OwnershipError, Validator};

fn manifest() -> Manifest {
    Manifest::from_file(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native-provider-manifest.json"),
    )
    .unwrap()
}

fn ar() -> PathBuf {
    let search = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&search)
        .map(|directory| directory.join("ar"))
        .find(|candidate| candidate.is_file())
        .expect("an `ar` executable is on PATH")
}

fn expected_members(manifest: &Manifest) -> BTreeSet<String> {
    let mut members: BTreeSet<String> = manifest
        .included_objects()
        .into_iter()
        .map(|object| {
            Path::new(&object.path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    members.extend(
        manifest
            .recompiled_objects
            .iter()
            .map(|object| object.object_output.clone()),
    );
    members
}

/// Appends every member in one `ar q`, which keeps repeated names as separate
/// members instead of replacing the earlier one.
fn build_archive(directory: &Path, members: &[&str]) -> PathBuf {
    let archive = directory.join("libuqm_c.a");
    let staging = directory.join("members");
    fs::create_dir_all(&staging).unwrap();
    let sources: Vec<PathBuf> = members
        .iter()
        .map(|member| {
            let source = staging.join(member);
            fs::write(&source, format!("member {member}\n")).unwrap();
            source
        })
        .collect();
    let output = Command::new(ar())
        .arg("q")
        .arg(&archive)
        .args(&sources)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ar q failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    archive
}

fn codes(error: &OwnershipError) -> BTreeSet<&'static str> {
    error
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

fn expect(codes: &[DiagnosticCode]) -> BTreeSet<&'static str> {
    codes.iter().map(|code| code.as_str()).collect()
}

fn names_of(error: &OwnershipError, code: DiagnosticCode) -> BTreeSet<String> {
    error
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .filter_map(|diagnostic| diagnostic.path.clone())
        .collect()
}

#[test]
fn an_archive_with_exactly_the_declared_members_is_accepted() {
    let manifest = manifest();
    let expected = expected_members(&manifest);
    assert!(expected.len() > 10, "the manifest declares archive members");
    let members: Vec<&str> = expected.iter().map(String::as_str).collect();
    let directory = tempfile::tempdir().unwrap();
    let archive = build_archive(directory.path(), &members);

    Validator::new(manifest)
        .validate_archive_file(&archive, &ar())
        .unwrap();
}

#[test]
fn a_member_the_manifest_does_not_declare_is_named_as_unknown() {
    let manifest = manifest();
    let expected = expected_members(&manifest);
    let mut members: Vec<&str> = expected.iter().map(String::as_str).collect();
    members.push("stale-excluded.c.o");
    let directory = tempfile::tempdir().unwrap();
    let archive = build_archive(directory.path(), &members);

    let error = Validator::new(manifest)
        .validate_archive_file(&archive, &ar())
        .unwrap_err();
    assert_eq!(
        codes(&error),
        expect(&[DiagnosticCode::ExcludedObjectInArchive])
    );
    assert_eq!(
        names_of(&error, DiagnosticCode::ExcludedObjectInArchive),
        BTreeSet::from(["stale-excluded.c.o".to_string()])
    );
}

#[test]
fn a_declared_member_that_is_absent_is_named_as_missing() {
    let manifest = manifest();
    let expected = expected_members(&manifest);
    let omitted = expected.iter().next().unwrap().clone();
    let members: Vec<&str> = expected
        .iter()
        .filter(|name| **name != omitted)
        .map(String::as_str)
        .collect();
    let directory = tempfile::tempdir().unwrap();
    let archive = build_archive(directory.path(), &members);

    let error = Validator::new(manifest)
        .validate_archive_file(&archive, &ar())
        .unwrap_err();
    assert_eq!(codes(&error), expect(&[DiagnosticCode::MissingProvider]));
    assert_eq!(
        names_of(&error, DiagnosticCode::MissingProvider),
        BTreeSet::from([omitted])
    );
}

#[test]
fn a_member_name_that_appears_twice_is_a_duplicate_object() {
    let manifest = manifest();
    let expected = expected_members(&manifest);
    let mut members: Vec<&str> = expected.iter().map(String::as_str).collect();
    members.push(members[0]);
    let directory = tempfile::tempdir().unwrap();
    let archive = build_archive(directory.path(), &members);

    let error = Validator::new(manifest)
        .validate_archive_file(&archive, &ar())
        .unwrap_err();
    assert_eq!(codes(&error), expect(&[DiagnosticCode::DuplicateObject]));
}

#[test]
fn unknown_missing_and_duplicate_members_are_all_reported_together() {
    let manifest = manifest();
    let expected = expected_members(&manifest);
    let omitted = expected.iter().next().unwrap().clone();
    let mut members: Vec<&str> = expected
        .iter()
        .filter(|name| **name != omitted)
        .map(String::as_str)
        .collect();
    members.push("stale-excluded.c.o");
    members.push(members[0]);
    let directory = tempfile::tempdir().unwrap();
    let archive = build_archive(directory.path(), &members);

    let error = Validator::new(manifest)
        .validate_archive_file(&archive, &ar())
        .unwrap_err();
    assert_eq!(
        codes(&error),
        expect(&[
            DiagnosticCode::DuplicateObject,
            DiagnosticCode::ExcludedObjectInArchive,
            DiagnosticCode::MissingProvider,
        ])
    );
}

#[test]
fn a_file_that_is_not_an_archive_is_reported_with_the_tool_and_path() {
    let directory = tempfile::tempdir().unwrap();
    let not_an_archive = directory.path().join("libuqm_c.a");
    fs::write(&not_an_archive, b"this is not an archive").unwrap();

    let error = Validator::new(manifest())
        .validate_archive_file(&not_an_archive, &ar())
        .unwrap_err();
    let text = error.to_string();
    assert!(text.contains("libuqm_c.a"), "{text}");
    assert!(text.contains("ar"), "{text}");
}
