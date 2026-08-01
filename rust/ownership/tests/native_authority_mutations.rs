use std::path::PathBuf;

use uqm_ownership::{
    load_native_dependencies, load_native_inputs, validate_native_authority, Manifest,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn authorities() -> (
    uqm_ownership::NativeInputManifest,
    uqm_ownership::NativeDependencyManifest,
    Manifest,
) {
    let root = root();
    (
        load_native_inputs(&root.join("rust/build/native-inputs.json")).unwrap(),
        load_native_dependencies(&root.join("rust/build/native-dependencies.json")).unwrap(),
        Manifest::from_file(&root.join("rust/ownership/native-provider-manifest.json")).unwrap(),
    )
}

#[test]
fn checked_in_native_authorities_are_exactly_bound() {
    let (inputs, dependencies, providers) = authorities();
    validate_native_authority(&root(), &inputs, &dependencies, &providers).unwrap();
    assert_eq!(inputs.inputs.len(), 321);
    assert!(dependencies
        .dependencies
        .iter()
        .any(|item| item.path == "sc2/config_unix.h"));
}

#[test]
fn source_hash_owner_command_profile_and_provider_substitutions_fail() {
    for mutation in 0..6 {
        let (mut inputs, dependencies, providers) = authorities();
        if mutation == 0 {
            inputs.inputs[0].source = inputs.inputs[1].source.clone();
        } else if mutation == 1 {
            inputs.inputs[0].source_sha256 = "0".repeat(64);
        } else if mutation == 2 {
            inputs.inputs[0].owner = "SUBSTITUTED".into();
        } else if mutation == 3 {
            inputs.inputs[0].producing_command = "cc substituted".into();
        } else if mutation == 4 {
            inputs.inputs[0].production_profile = "debug".into();
        } else {
            assert_ne!(inputs.inputs[0].provider, inputs.inputs[1].provider);
            inputs.inputs[0].provider = inputs.inputs[1].provider.clone();
        }
        assert!(validate_native_authority(&root(), &inputs, &dependencies, &providers).is_err());
    }
}

#[test]
fn recompiled_identity_substitutions_fail() {
    for mutation in 0..5 {
        let (inputs, dependencies, mut providers) = authorities();
        let declaration = &mut providers.recompiled_objects[0];
        if mutation == 0 {
            let replacement = inputs
                .inputs
                .iter()
                .find(|input| input.source != declaration.canonical_source)
                .unwrap();
            declaration.canonical_source.clone_from(&replacement.source);
            declaration
                .source_sha256
                .clone_from(&replacement.source_sha256);
        } else if mutation == 1 {
            declaration.source_sha256 = "0".repeat(64);
        } else if mutation == 2 {
            declaration.owner = "SUBSTITUTED".into();
        } else if mutation == 3 {
            declaration.producing_command = "cc substituted".into();
        } else {
            declaration.production_profile = "debug".into();
        }
        assert!(validate_native_authority(&root(), &inputs, &dependencies, &providers).is_err());
    }
}

#[test]
fn dependency_hash_and_path_drift_fail() {
    let (inputs, mut dependencies, providers) = authorities();
    dependencies.dependencies[0].sha256 = "0".repeat(64);
    assert!(validate_native_authority(&root(), &inputs, &dependencies, &providers).is_err());

    let (inputs, mut dependencies, providers) = authorities();
    assert_ne!(
        dependencies.dependencies[0].path,
        dependencies.dependencies[1].path
    );
    dependencies.dependencies[0].path = dependencies.dependencies[1].path.clone();
    dependencies.dependencies[0].sha256 = dependencies.dependencies[1].sha256.clone();
    assert!(validate_native_authority(&root(), &inputs, &dependencies, &providers).is_err());
}

#[test]
fn observed_dependencies_reject_both_missing_and_stale_declarations() {
    let (inputs, dependencies, _) = authorities();
    let target = "macos-aarch64";
    let observed: std::collections::BTreeSet<_> = inputs
        .inputs
        .iter()
        .map(|input| input.source.clone())
        .chain(
            dependencies
                .dependencies
                .iter()
                .filter(|dependency| dependency.is_active_for(target))
                .map(|dependency| dependency.path.clone()),
        )
        .collect();
    uqm_ownership::validate_observed_dependencies(&observed, &inputs, &dependencies, target)
        .unwrap();

    let mut missing = dependencies.clone();
    let removed = missing.dependencies.remove(0);
    assert!(removed.is_active_for(target));
    assert!(
        uqm_ownership::validate_observed_dependencies(&observed, &inputs, &missing, target)
            .unwrap_err()
            .contains("undeclared_observed")
    );

    let mut stale_observed = observed;
    stale_observed.remove(&removed.path);
    assert!(uqm_ownership::validate_observed_dependencies(
        &stale_observed,
        &inputs,
        &dependencies,
        target,
    )
    .unwrap_err()
    .contains("declared_but_unobserved"));
}
