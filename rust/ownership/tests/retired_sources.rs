use std::fs;
use std::path::PathBuf;

use uqm_ownership::{
    ArchiveDecision, DiagnosticCode, Manifest, ObjectProvider, OwnershipError, ProviderKind,
    ValidateOptions, Validator, DISPLIST_SOURCE, LIST_OBJECT, LIST_SOURCE, RETIRED_NATIVE_SOURCES,
};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest() -> Manifest {
    Manifest::from_file(&manifest_dir().join("native-provider-manifest.json")).unwrap()
}

fn disk_options(repo_root: PathBuf) -> ValidateOptions {
    ValidateOptions {
        repo_root,
        check_disk_objects: true,
        check_archive: false,
        archive_path: None,
        check_strict_link: false,
    }
}

fn has(error: &OwnershipError, code: DiagnosticCode, path: &str) -> bool {
    error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code && diagnostic.path.as_deref() == Some(path))
}

#[test]
fn checked_in_manifest_records_exactly_the_authorized_retirements() {
    let manifest = manifest();
    manifest.validate_self().unwrap();
    assert_eq!(manifest.retired_sources, RETIRED_NATIVE_SOURCES);
    let list = manifest
        .objects
        .iter()
        .find(|object| object.path == LIST_OBJECT)
        .unwrap();
    assert_eq!(list.archive_decision, ArchiveDecision::ExcludeRetired);
    assert_eq!(
        list.provider,
        ObjectProvider::new(ProviderKind::Retired, LIST_SOURCE)
    );
}

#[test]
fn checked_in_tree_has_none_of_the_retired_sources_and_every_other_source() {
    let repo_root = manifest_dir().join("../..");
    for source in RETIRED_NATIVE_SOURCES {
        assert!(!repo_root.join(source).exists(), "{source} is back");
    }
    Validator::new(manifest())
        .validate(&disk_options(repo_root))
        .unwrap();
}

#[test]
fn a_retired_source_that_reappears_on_disk_is_rejected() {
    let repo_root = tempfile::tempdir().unwrap();
    let source = repo_root.path().join(LIST_SOURCE);
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "int list;\n").unwrap();

    let error = Validator::new(manifest())
        .validate(&disk_options(repo_root.path().to_path_buf()))
        .unwrap_err();
    assert!(has(&error, DiagnosticCode::StaleObject, LIST_SOURCE));
    assert!(!has(&error, DiagnosticCode::MissingFromDisk, LIST_SOURCE));
    assert!(!has(
        &error,
        DiagnosticCode::MissingFromDisk,
        DISPLIST_SOURCE
    ));
}

#[test]
fn a_retired_object_cannot_return_to_the_archive() {
    let mut manifest = manifest();
    let object = manifest
        .objects
        .iter_mut()
        .find(|object| object.path == LIST_OBJECT)
        .unwrap();
    object.archive_decision = ArchiveDecision::Include;
    let error = manifest.validate_self().unwrap_err();
    assert!(has(&error, DiagnosticCode::UnassignedObject, LIST_OBJECT));
}

#[test]
fn the_retired_source_list_cannot_shrink_or_grow() {
    let mut shrunk = manifest();
    shrunk.retired_sources.pop();
    assert!(has(
        &shrunk.validate_self().unwrap_err(),
        DiagnosticCode::ManifestDrift,
        "retired_sources"
    ));

    let mut grown = manifest();
    grown
        .retired_sources
        .push("sc2/src/uqm/starcon.c".to_string());
    let error = grown.validate_self().unwrap_err();
    assert!(has(
        &error,
        DiagnosticCode::ManifestDrift,
        "retired_sources"
    ));
    assert!(has(
        &error,
        DiagnosticCode::UnassignedObject,
        "sc2/src/uqm/starcon.c"
    ));
}

#[test]
fn a_retired_object_must_name_a_listed_source() {
    let mut manifest = manifest();
    manifest
        .retired_sources
        .retain(|source| source != LIST_SOURCE);
    let error = manifest.validate_self().unwrap_err();
    assert!(has(&error, DiagnosticCode::UnassignedObject, LIST_OBJECT));
}
