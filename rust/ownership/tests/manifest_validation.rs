use std::path::PathBuf;

use uqm_ownership::{
    ArchiveDecision, DiagnosticCode, Manifest, ValidateOptions, Validator, DISPLIST_OBJECT,
    EXPECTED_ASSESSMENT_COMMIT, EXPECTED_LEDGER_GIST_REVISION, EXPECTED_LEDGER_RAW_URL,
    EXPECTED_LEDGER_SHA256, EXPECTED_OBJECT_COUNT, QUEUE_RUST_PROVIDER, QUEUE_SYMBOLS,
};

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native-provider-manifest.json")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn manifest() -> Manifest {
    Manifest::from_file(&manifest_path()).unwrap()
}

#[test]
fn checked_in_manifest_has_exact_v3_identity_and_inventory() {
    let manifest = manifest();
    manifest.validate_self().unwrap();
    assert_eq!(
        manifest.generated_from_ledger.assessment_commit,
        EXPECTED_ASSESSMENT_COMMIT
    );
    assert_eq!(
        manifest.generated_from_ledger.raw_url,
        EXPECTED_LEDGER_RAW_URL
    );
    assert_eq!(
        manifest.generated_from_ledger.gist_revision,
        EXPECTED_LEDGER_GIST_REVISION
    );
    assert_eq!(
        manifest.generated_from_ledger.sha256,
        EXPECTED_LEDGER_SHA256
    );
    assert_eq!(manifest.objects.len(), EXPECTED_OBJECT_COUNT);
}

#[test]
fn displist_is_rejected_and_queue_contract_has_one_rust_provider() {
    let manifest = manifest();
    let displist = manifest
        .objects
        .iter()
        .find(|object| object.path == DISPLIST_OBJECT)
        .unwrap();
    assert_eq!(
        displist.archive_decision,
        ArchiveDecision::ExcludeDuplicateProvider
    );
    assert_eq!(displist.provider.path, QUEUE_RUST_PROVIDER);
    let mut symbols: Vec<_> = manifest
        .symbol_contracts
        .iter()
        .map(|contract| contract.symbol.as_str())
        .collect();
    symbols.sort_unstable();
    assert_eq!(symbols, QUEUE_SYMBOLS);
    assert!(manifest.symbol_contracts.iter().all(|contract| {
        contract.active_provider.path == QUEUE_RUST_PROVIDER
            && contract
                .excluded_providers
                .iter()
                .any(|provider| provider.path == DISPLIST_OBJECT)
    }));
}

#[test]
fn strict_link_policy_and_reports_are_deterministic() {
    let validator = Validator::new(manifest());
    let options = ValidateOptions {
        repo_root: repo_root(),
        check_disk_objects: false,
        check_archive: false,
        archive_path: None,
        check_strict_link: true,
    };
    let first = validator.validate(&options).unwrap().to_json().unwrap();
    let second = validator.validate(&options).unwrap().to_json().unwrap();
    assert_eq!(first, second);
    let build_script = std::fs::read_to_string(repo_root().join("rust/build.rs")).unwrap();
    for line in build_script
        .lines()
        .filter(|line| line.contains("cargo:rustc-link-arg"))
    {
        assert!(!line.contains("-undefined,dynamic_lookup"));
        assert!(!line.contains("-flat_namespace"));
    }
}

#[test]
fn unreadable_manifest_is_reported_as_io_error() {
    let error =
        Manifest::from_file(&repo_root().join("missing-ownership-manifest.json")).unwrap_err();
    assert_eq!(error.diagnostics[0].code, DiagnosticCode::IoError);
}

#[test]
fn production_archive_is_explicitly_feature_gated() {
    let root = repo_root();
    let build_script = std::fs::read_to_string(root.join("rust/build.rs")).unwrap();
    let verifier =
        std::fs::read_to_string(root.join("rust/ownership/verify-production.sh")).unwrap();

    assert!(build_script.contains("env::var_os(\"CARGO_FEATURE_LINKED_C_ARCHIVE\")"));
    assert!(verifier.contains("audio_heart,linked_c_archive"));
}
