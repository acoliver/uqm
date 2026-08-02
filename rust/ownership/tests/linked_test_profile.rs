//! S3 (issue 23) linked-test profile validation tests.
//!
//! The linked-test profile must be a separately named all-feature profile
//! that is a strict superset of the accepted production profile, adds DEBUG,
//! and includes the debug-process Cargo feature. It must NOT alter the
//! accepted S1/S2 production profile.

use std::path::PathBuf;

use uqm_ownership::{
    validate_linked_test_profile, Manifest, LINKED_TEST_PROFILE_ID, PRODUCTION_PROFILE_ID,
};

fn manifest() -> Manifest {
    Manifest::from_file(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native-provider-manifest.json"),
    )
    .unwrap()
}

#[test]
fn linked_test_profile_is_declared_and_separately_named() {
    let manifest = manifest();
    let linked = manifest
        .linked_test_profile
        .as_ref()
        .expect("linked_test_profile must be declared in the manifest");
    assert_eq!(linked.id, LINKED_TEST_PROFILE_ID);
    assert_ne!(linked.id, manifest.accepted_production_profile.id);
    assert_eq!(
        manifest.accepted_production_profile.id,
        PRODUCTION_PROFILE_ID
    );
}

#[test]
fn linked_test_profile_includes_all_production_defines() {
    let manifest = manifest();
    let linked = manifest.linked_test_profile.as_ref().unwrap();
    let production = &manifest.accepted_production_profile;
    validate_linked_test_profile(linked, production).unwrap();
}

#[test]
fn linked_test_profile_has_debug_define() {
    let manifest = manifest();
    let linked = manifest.linked_test_profile.as_ref().unwrap();
    assert!(linked.defines.iter().any(|d| d.name == "DEBUG"));
}

#[test]
fn linked_test_profile_includes_debug_process_cargo_feature() {
    let manifest = manifest();
    let linked = manifest.linked_test_profile.as_ref().unwrap();
    assert!(linked.cargo_features.contains(&"debug-process".to_string()));
}

#[test]
fn linked_test_profile_is_superset_of_production_features() {
    let manifest = manifest();
    let linked = manifest.linked_test_profile.as_ref().unwrap();
    let production = &manifest.accepted_production_profile;
    for feature in &production.cargo_features {
        assert!(
            linked.cargo_features.contains(feature),
            "linked_test_profile must include production feature '{feature}'"
        );
    }
}

#[test]
fn linked_test_profile_compile_flags_match_production() {
    let manifest = manifest();
    let linked = manifest.linked_test_profile.as_ref().unwrap();
    let production = &manifest.accepted_production_profile;
    assert_eq!(linked.compile_flags, production.compile_flags);
}

#[test]
fn manifest_validates_with_linked_test_profile() {
    let manifest = manifest();
    manifest.validate_self().unwrap();
}

#[test]
fn wrong_id_rejected() {
    let manifest = manifest();
    let production = &manifest.accepted_production_profile;
    let mut bad = manifest.linked_test_profile.as_ref().unwrap().clone();
    bad.id = "wrong".into();
    assert!(validate_linked_test_profile(&bad, production).is_err());
}

#[test]
fn missing_debug_rejected() {
    let manifest = manifest();
    let production = &manifest.accepted_production_profile;
    let mut bad = manifest.linked_test_profile.as_ref().unwrap().clone();
    bad.defines.retain(|d| d.name != "DEBUG");
    assert!(validate_linked_test_profile(&bad, production).is_err());
}

#[test]
fn missing_debug_process_feature_rejected() {
    let manifest = manifest();
    let production = &manifest.accepted_production_profile;
    let mut bad = manifest.linked_test_profile.as_ref().unwrap().clone();
    bad.cargo_features.retain(|f| f != "debug-process");
    assert!(validate_linked_test_profile(&bad, production).is_err());
}

#[test]
fn missing_production_define_rejected() {
    let manifest = manifest();
    let production = &manifest.accepted_production_profile;
    let mut bad = manifest.linked_test_profile.as_ref().unwrap().clone();
    // Remove a production define that's not DEBUG
    let prod_define = production
        .defines
        .iter()
        .find(|d| d.name != "DEBUG")
        .unwrap()
        .clone();
    bad.defines.retain(|d| d.name != prod_define.name);
    assert!(validate_linked_test_profile(&bad, production).is_err());
}

#[test]
fn mismatched_compile_flags_rejected() {
    let manifest = manifest();
    let production = &manifest.accepted_production_profile;
    let mut bad = manifest.linked_test_profile.as_ref().unwrap().clone();
    bad.compile_flags.push("-O0".into());
    assert!(validate_linked_test_profile(&bad, production).is_err());
}

#[test]
fn duplicate_define_rejected() {
    let manifest = manifest();
    let production = &manifest.accepted_production_profile;
    let mut bad = manifest.linked_test_profile.as_ref().unwrap().clone();
    let debug = bad
        .defines
        .iter()
        .find(|d| d.name == "DEBUG")
        .unwrap()
        .clone();
    bad.defines.push(debug);
    assert!(validate_linked_test_profile(&bad, production).is_err());
}
