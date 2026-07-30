use std::fs;
use std::path::PathBuf;

use proptest::prelude::*;
use uqm_ownership::{
    DiagnosticCode, ExternalImport, Manifest, ObjectProvider, ObservedSymbol, ProductionNm,
    ProviderKind, SymbolState, ValidateOptions, Validator, DISPLIST_OBJECT, QUEUE_RUST_PROVIDER,
    QUEUE_SYMBOLS,
};

fn manifest() -> Manifest {
    Manifest::from_file(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native-provider-manifest.json"),
    )
    .unwrap()
}

fn has(error: &uqm_ownership::OwnershipError, code: DiagnosticCode) -> bool {
    error
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

fn valid_symbols() -> Vec<ObservedSymbol> {
    QUEUE_SYMBOLS
        .iter()
        .map(|symbol| ObservedSymbol {
            symbol: (*symbol).to_string(),
            provider: QUEUE_RUST_PROVIDER.to_string(),
            state: SymbolState::DefinedInternal,
        })
        .collect()
}
fn valid_nm() -> ProductionNm {
    let definitions = QUEUE_SYMBOLS
        .iter()
        .map(|symbol| format!("libuqm_rust.a(queue.o): 00000000 T _{symbol}"))
        .collect::<Vec<_>>()
        .join("\n");
    ProductionNm {
        rust_archive: definitions.clone(),
        c_archive: QUEUE_SYMBOLS
            .iter()
            .map(|symbol| format!("libuqm_c.a(caller.o): U _{symbol}"))
            .collect::<Vec<_>>()
            .join("\n"),
        executable: definitions.replace("libuqm_rust.a(queue.o)", "uqm"),
        executable_details: String::new(),
    }
}

#[test]
fn duplicate_missing_unassigned_dynamic_and_external_symbols_fail() {
    let mut symbols = valid_symbols();
    symbols.push(symbols[0].clone());
    assert!(has(
        &Validator::new(manifest())
            .validate_symbols(&symbols)
            .unwrap_err(),
        DiagnosticCode::DuplicateProvider
    ));

    let mut symbols = valid_symbols();
    symbols.pop();
    assert!(has(
        &Validator::new(manifest())
            .validate_symbols(&symbols)
            .unwrap_err(),
        DiagnosticCode::MissingProvider
    ));

    let mut symbols = valid_symbols();
    symbols.push(ObservedSymbol {
        symbol: "UnownedInternal".into(),
        provider: "rust/src/lib.rs".into(),
        state: SymbolState::DefinedInternal,
    });
    assert!(has(
        &Validator::new(manifest())
            .validate_symbols(&symbols)
            .unwrap_err(),
        DiagnosticCode::UnassignedObject
    ));

    let mut symbols = valid_symbols();
    symbols[0].state = SymbolState::DynamicInternal;
    assert!(has(
        &Validator::new(manifest())
            .validate_symbols(&symbols)
            .unwrap_err(),
        DiagnosticCode::DynamicUnresolvedSymbol
    ));

    let mut symbols = valid_symbols();
    symbols[0].state = SymbolState::UnresolvedInternal;
    assert!(has(
        &Validator::new(manifest())
            .validate_symbols(&symbols)
            .unwrap_err(),
        DiagnosticCode::DynamicUnresolvedSymbol
    ));

    let mut symbols = valid_symbols();
    symbols.push(ObservedSymbol {
        symbol: "malloc".into(),
        provider: "libSystem".into(),
        state: SymbolState::ExternalImport,
    });
    assert!(has(
        &Validator::new(manifest())
            .validate_symbols(&symbols)
            .unwrap_err(),
        DiagnosticCode::UnassignedObject
    ));

    let mut allowlisted = manifest();
    allowlisted.external_imports.push(ExternalImport {
        symbol: "malloc".into(),
        provider: "libSystem".into(),
    });
    Validator::new(allowlisted)
        .validate_symbols(&symbols)
        .unwrap();
}

#[test]
fn actual_nm_observations_reject_duplicate_missing_wrong_and_unresolved_providers() {
    let validator = Validator::new(manifest());
    validator.validate_production_nm(&valid_nm()).unwrap();

    let mut duplicate = valid_nm();
    duplicate
        .rust_archive
        .push_str("\nlibuqm_rust.a(other.o): 00000000 T _AllocLink");
    assert!(has(
        &validator.validate_production_nm(&duplicate).unwrap_err(),
        DiagnosticCode::DuplicateProvider
    ));

    let mut missing = valid_nm();
    missing.rust_archive = missing
        .rust_archive
        .lines()
        .filter(|line| !line.ends_with("_ForAllLinks"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(has(
        &validator.validate_production_nm(&missing).unwrap_err(),
        DiagnosticCode::MissingProvider
    ));

    let mut wrong = valid_nm();
    wrong.rust_archive = wrong
        .rust_archive
        .lines()
        .filter(|line| !line.ends_with("_InitQueue"))
        .collect::<Vec<_>>()
        .join("\n");
    wrong
        .c_archive
        .push_str("\nlibuqm_c.a(displist.o): 00000000 T _InitQueue");
    assert!(has(
        &validator.validate_production_nm(&wrong).unwrap_err(),
        DiagnosticCode::UnassignedObject
    ));

    let mut unresolved = valid_nm();
    unresolved.executable.push_str("\nuqm: U _RemoveQueue");
    assert!(has(
        &validator.validate_production_nm(&unresolved).unwrap_err(),
        DiagnosticCode::DynamicUnresolvedSymbol
    ));

    let mut dynamic = valid_nm();
    dynamic.executable_details =
        "uqm: (undefined) external _PutQueue (dynamically looked up)".into();
    assert!(has(
        &validator.validate_production_nm(&dynamic).unwrap_err(),
        DiagnosticCode::DynamicUnresolvedSymbol
    ));
}

#[test]
fn recompiled_declarations_are_a_duplicate_free_archive_bijection() {
    let original = manifest();

    let mut extra = original.clone();
    let mut extra_entry = extra.recompiled_objects[0].clone();
    extra_entry.repo_relative_path = "sc2/src/extra.c".into();
    extra_entry.object_output = "extra.o".into();
    extra.recompiled_objects.push(extra_entry);
    assert!(extra.validate_self().is_err());

    let mut missing = original.clone();
    missing.recompiled_objects.pop();
    assert!(missing.validate_self().is_err());

    let mut duplicate_source = original.clone();
    duplicate_source
        .recompiled_objects
        .push(duplicate_source.recompiled_objects[0].clone());
    assert!(has(
        &duplicate_source.validate_self().unwrap_err(),
        DiagnosticCode::DuplicateObject
    ));

    let mut duplicate_output = original.clone();
    duplicate_output.recompiled_objects[1].object_output =
        duplicate_output.recompiled_objects[0].object_output.clone();
    assert!(has(
        &duplicate_output.validate_self().unwrap_err(),
        DiagnosticCode::DuplicateObject
    ));

    let mut basename_collision = original;
    let included_basename = PathBuf::from(
        &basename_collision
            .objects
            .iter()
            .find(|object| object.archive_decision == uqm_ownership::ArchiveDecision::Include)
            .unwrap()
            .path,
    )
    .file_name()
    .unwrap()
    .to_string_lossy()
    .into_owned();
    basename_collision.recompiled_objects[0].object_output = included_basename;
    assert!(has(
        &basename_collision.validate_self().unwrap_err(),
        DiagnosticCode::DuplicateObject
    ));
}

#[test]
fn missing_rust_provider_paths_and_duplicate_sidecar_lines_fail() {
    let mut missing_provider = manifest();
    missing_provider
        .objects
        .iter_mut()
        .find(|object| object.provider.kind == ProviderKind::RustSource)
        .unwrap()
        .provider
        .path = "rust/src/does-not-exist.rs".into();
    let error = Validator::new(missing_provider)
        .validate(&ValidateOptions {
            repo_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            check_disk_objects: false,
            check_archive: false,
            archive_path: None,
            check_strict_link: false,
        })
        .unwrap_err();
    assert!(has(&error, DiagnosticCode::MissingProvider));

    let value = manifest();
    let lines = value
        .included_objects()
        .into_iter()
        .map(|object| object.path.as_str())
        .chain(
            value
                .recompiled_objects
                .iter()
                .map(|object| object.repo_relative_path.as_str()),
        )
        .collect::<Vec<_>>();
    let directory = tempfile::tempdir().unwrap();
    let sidecar = directory.path().join("uqm-c-objects.manifest");
    fs::write(&sidecar, format!("{}\n{}\n", lines.join("\n"), lines[0])).unwrap();
    let error = Validator::new(value)
        .validate(&ValidateOptions {
            repo_root: PathBuf::new(),
            check_disk_objects: false,
            check_archive: true,
            archive_path: Some(sidecar),
            check_strict_link: false,
        })
        .unwrap_err();
    assert!(has(&error, DiagnosticCode::DuplicateObject));
}

#[test]
fn production_profile_rejects_feature_and_c_flag_drift() {
    let validator = Validator::new(manifest());
    let valid_features = ["audio_heart", "linked_c_archive"];
    let build_vars = "-DUSE_RUST_BRIDGE -DUSE_RUST_AUDIO_HEART -DRUST_OWNS_MAIN";
    let config = "#define USE_RUST_BRIDGE\n#define USE_RUST_AUDIO_HEART\n";
    let recompile = format!("{build_vars} -DUSE_RUST_MAINLOOP=1");
    validator
        .validate_production_profile(&valid_features, build_vars, config, &recompile)
        .unwrap();

    assert!(validator
        .validate_production_profile(&["linked_c_archive"], build_vars, config, &recompile)
        .is_err());
    assert!(validator
        .validate_production_profile(
            &valid_features,
            "-DUSE_RUST_BRIDGE -DRUST_OWNS_MAIN",
            config,
            &recompile,
        )
        .is_err());
    assert!(validator
        .validate_production_profile(
            &valid_features,
            build_vars,
            config,
            "-DUSE_RUST_BRIDGE -DUSE_RUST_AUDIO_HEART -DRUST_OWNS_MAIN -DUSE_RUST_MAINLOOP=0",
        )
        .is_err());
}

#[test]
fn ledger_projection_authenticates_owner_and_canonical_source() {
    let mut owner = manifest();
    owner.objects[0].issue = "OTHER".into();
    assert!(has(
        &owner.validate_self().unwrap_err(),
        DiagnosticCode::ManifestDrift
    ));

    let mut source = manifest();
    let object = source
        .objects
        .iter_mut()
        .find(|object| object.canonical_source.is_some())
        .unwrap();
    object.canonical_source = Some("sc2/src/wrong.c".into());
    assert!(has(
        &source.validate_self().unwrap_err(),
        DiagnosticCode::ManifestDrift
    ));
}

#[test]
fn duplicate_missing_stale_unassigned_and_drift_object_mutations_fail() {
    let mut duplicate = manifest();
    duplicate.objects.push(duplicate.objects[0].clone());
    assert!(has(
        &duplicate.validate_self().unwrap_err(),
        DiagnosticCode::DuplicateObject
    ));

    let mut unassigned = manifest();
    let included = unassigned
        .objects
        .iter_mut()
        .find(|object| object.archive_decision == uqm_ownership::ArchiveDecision::Include)
        .unwrap();
    included.provider = ObjectProvider::new(ProviderKind::RustSource, "rust/src/lib.rs");
    assert!(has(
        &unassigned.validate_self().unwrap_err(),
        DiagnosticCode::UnassignedObject
    ));

    let root = tempfile::tempdir().unwrap();
    let object_root = root.path().join("sc2/obj/release");
    fs::create_dir_all(&object_root).unwrap();
    fs::write(object_root.join("unknown.c.o"), b"unknown").unwrap();
    let error = Validator::new(manifest())
        .validate(&ValidateOptions {
            repo_root: root.path().to_path_buf(),
            check_disk_objects: true,
            check_archive: false,
            archive_path: None,
            check_strict_link: false,
        })
        .unwrap_err();
    assert!(has(&error, DiagnosticCode::MissingFromManifest));
    assert!(has(&error, DiagnosticCode::MissingFromDisk));

    let mut stale = manifest();
    stale
        .objects
        .retain(|object| object.path.ends_with("unknown.c.o"));
    stale.objects = vec![manifest().objects[0].clone()];
    let relative = stale.objects[0]
        .path
        .strip_prefix("sc2/obj/release/")
        .unwrap();
    let target = object_root.join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, b"stale").unwrap();
    let error = Validator::new(stale)
        .validate(&ValidateOptions {
            repo_root: root.path().to_path_buf(),
            check_disk_objects: true,
            check_archive: false,
            archive_path: None,
            check_strict_link: false,
        })
        .unwrap_err();
    assert!(has(&error, DiagnosticCode::StaleObject));
}

#[test]
fn identity_traversal_collision_displist_and_archive_mutations_fail() {
    let mut malformed = manifest();
    malformed.generated_from_ledger.sha256 = "0".repeat(64);
    assert!(has(
        &malformed.validate_self().unwrap_err(),
        DiagnosticCode::MalformedManifest
    ));

    let mut traversal = manifest();
    traversal.objects[0].path = "sc2/obj/release/../escape.c.o".into();
    assert!(has(
        &traversal.validate_self().unwrap_err(),
        DiagnosticCode::PathViolation
    ));

    let mut collision = manifest();
    let source = collision
        .objects
        .iter()
        .find(|object| object.archive_decision == uqm_ownership::ArchiveDecision::Include)
        .unwrap()
        .clone();
    let mut object = source.clone();
    object.path = format!(
        "sc2/obj/release/collision/{}",
        PathBuf::from(&source.path)
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
    object.provider = ObjectProvider::new(ProviderKind::NativeObject, object.path.clone());
    collision.objects.push(object);
    assert!(has(
        &collision.validate_self().unwrap_err(),
        DiagnosticCode::DuplicateObject
    ));

    let mut displist = manifest();
    let object = displist
        .objects
        .iter_mut()
        .find(|object| object.path == DISPLIST_OBJECT)
        .unwrap();
    object.archive_decision = uqm_ownership::ArchiveDecision::Include;
    object.provider = ObjectProvider::new(ProviderKind::NativeObject, DISPLIST_OBJECT);
    assert!(has(
        &displist.validate_self().unwrap_err(),
        DiagnosticCode::DuplicateProvider
    ));

    let sidecar_dir = tempfile::tempdir().unwrap();
    let sidecar = sidecar_dir.path().join("uqm-c-objects.manifest");
    fs::write(&sidecar, format!("{DISPLIST_OBJECT}\n")).unwrap();
    let error = Validator::new(manifest())
        .validate(&ValidateOptions {
            repo_root: PathBuf::new(),
            check_disk_objects: false,
            check_archive: true,
            archive_path: Some(sidecar),
            check_strict_link: false,
        })
        .unwrap_err();
    assert!(has(&error, DiagnosticCode::ExcludedObjectInArchive));
    assert!(has(&error, DiagnosticCode::MissingProvider));
}

#[test]
fn ordering_and_reentry_are_byte_deterministic() {
    let validator = Validator::new(manifest());
    let options = ValidateOptions::manifest_only();
    let first = validator.validate(&options).unwrap().to_json().unwrap();
    for _ in 0..5 {
        assert_eq!(
            validator.validate(&options).unwrap().to_json().unwrap(),
            first
        );
    }
}

proptest! {
    #[test]
    fn path_traversal_components_always_fail(depth in 1usize..8) {
        let mut value = manifest();
        value.objects[0].path = format!("sc2/obj/release/{}/escape.c.o", "../".repeat(depth));
        prop_assert!(has(&value.validate_self().unwrap_err(), DiagnosticCode::PathViolation));
    }
}
