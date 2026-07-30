use std::collections::BTreeSet;
use std::env;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use uqm_ownership::{
    apply_toolchain_environment, canonical_absolute, canonical_build_environment,
    discover_package_identities, load_native_dependencies, load_native_inputs,
    parse_dependency_file, reject_ambient_build_flags, resolve_toolchain, target_key,
    validate_native_authority, validate_observed_dependencies, write_build_evidence,
    NativeBuildEvidence, NativeCompileProfile, NativeInputManifest, PreprocessorDefine,
    ToolchainIdentity, ValidateOptions, Validator, BUILD_EVIDENCE_FILE, BUILD_EVIDENCE_SCHEMA,
    DEPENDENCY_FLAGS, DISPLIST_OBJECT, REPOSITORY_INCLUDE_ROOTS,
};

const DEPENDENCY_AUTHORITY: &str = "build/native-dependencies.json";
const INPUT_AUTHORITY: &str = "build/native-inputs.json";
const PROVIDER_AUTHORITY: &str = "ownership/native-provider-manifest.json";
const SDL_PACKAGE: [&str; 1] = ["sdl2"];
const COMMON_PRODUCTION_PACKAGES: [&str; 2] = ["libpng", "liblzma"];
const MACOS_PRODUCTION_PACKAGES: [&str; 3] = ["libpng", "liblzma", "bzip2"];

fn main() {
    if let Err(error) = run() {
        fail(error);
    }
}

fn run() -> Result<(), String> {
    let target = env_value("TARGET")?;
    let toolchain = resolve_toolchain(Path::new("."), &target)?;
    apply_toolchain_environment(&toolchain);
    generate_state_bindings(Path::new("../sc2/src/uqm/globdata.h"));
    generate_hash_abi_bindings()?;
    compile_local_helpers();
    let mut packages = discover_packages(&SDL_PACKAGE)?;
    let target_os = env_value("CARGO_CFG_TARGET_OS")?;
    if env::var_os("CARGO_FEATURE_LINKED_C_ARCHIVE").is_some() {
        reject_ambient_build_flags()?;
        validate_toolchain_marker(&toolchain)?;
        let production_packages = if target_os == "macos" {
            &MACOS_PRODUCTION_PACKAGES[..]
        } else {
            println!("cargo:rustc-link-lib=bz2");
            &COMMON_PRODUCTION_PACKAGES[..]
        };
        packages.extend(discover_packages(production_packages)?);
        link_c_objects(&packages, &toolchain)?;
    } else {
        emit_package_links(&packages, &target_os);
    }
    compile_p00_harness(&toolchain)?;
    Ok(())
}

fn validate_toolchain_marker(actual: &ToolchainIdentity) -> Result<(), String> {
    println!("cargo:rerun-if-env-changed=UQM_CANONICAL_TOOLCHAIN");
    let marker = env::var("UQM_CANONICAL_TOOLCHAIN").map_err(|_| {
        "linked production builds require canonical xtask toolchain resolution".to_string()
    })?;
    let expected: ToolchainIdentity = serde_json::from_str(&marker)
        .map_err(|error| format!("invalid UQM_CANONICAL_TOOLCHAIN marker: {error}"))?;
    if &expected != actual {
        return Err("build.rs effective toolchain differs from canonical xtask selection".into());
    }
    Ok(())
}
fn fail(message: impl Display) -> ! {
    eprintln!("S1 ownership/strict-link validation failed: {message}");
    std::process::exit(1)
}

fn require<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => fail(format!("{context}: {error}")),
    }
}

fn require_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => fail(context),
    }
}

fn compile_local_helpers() {
    for (source, library) in [
        ("src/io/uio_vfprintf_helper.c", "uio_vfprintf_helper"),
        ("src/mainloop/rust_test_bridge.c", "uqm_test_bridge"),
    ] {
        cc::Build::new()
            .warnings(true)
            .file(source)
            .cpp(false)
            .compile(library);
        println!("cargo:rerun-if-changed={source}");
    }
}

fn authority_paths(rust_root: &Path) -> [PathBuf; 3] {
    [
        rust_root.join(INPUT_AUTHORITY),
        rust_root.join(DEPENDENCY_AUTHORITY),
        rust_root.join(PROVIDER_AUTHORITY),
    ]
}

fn link_c_objects(packages: &NativePackages, toolchain: &ToolchainIdentity) -> Result<(), String> {
    let rust_root = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR")
            .map_err(|error| format!("CARGO_MANIFEST_DIR is unavailable: {error}"))?,
    );
    let repo_root = rust_root
        .parent()
        .ok_or_else(|| "Rust crate has no repository parent".to_string())?
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize repository root: {error}"))?;
    let [input_path, dependency_path, provider_path] = authority_paths(&rust_root);
    for path in [&input_path, &dependency_path, &provider_path] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let validator =
        Validator::from_manifest_file(&provider_path).map_err(|error| error.to_string())?;
    validator
        .validate(&ValidateOptions::production(repo_root.clone()))
        .map_err(|error| error.to_string())?;
    let inputs = load_native_inputs(&input_path)?;
    let dependencies = load_native_dependencies(&dependency_path)?;
    validate_native_authority(&repo_root, &inputs, &dependencies, validator.manifest())?;
    validate_active_profile(&inputs, &validator)?;
    let target_os = env_value("CARGO_CFG_TARGET_OS")?;
    let target_arch = env_value("CARGO_CFG_TARGET_ARCH")?;
    let target = target_key(&target_os, &target_arch)?;
    validate_package_defines(
        &target,
        &packages.defines,
        &inputs.production_profile.defines,
    )?;

    for dependency in &dependencies.dependencies {
        println!("cargo:rerun-if-changed=../{}", dependency.path);
    }
    let out_dir = output_directory()?;
    let object_dir = out_dir.join("native");
    fs::create_dir_all(&object_dir)
        .map_err(|error| format!("cannot create {}: {error}", object_dir.display()))?;
    let profile = native_compile_profile(&repo_root, &target, toolchain, &inputs, packages)?;
    let evidence = native_build_evidence(&repo_root, &target, toolchain, &inputs, &profile)?;
    write_build_evidence(&out_dir.join(BUILD_EVIDENCE_FILE), &evidence)?;
    let observed = compile_native_inputs(&inputs, &repo_root, &object_dir, &profile)?;
    if let Ok(capture_path) = env::var("UQM_DEPENDENCY_CAPTURE") {
        write_dependency_capture(Path::new(&capture_path), &target, &observed, &inputs)?;
    } else {
        validate_observed_dependencies(&observed, &inputs, &dependencies, &target)?;
    }
    let archive_context = ArchiveContext {
        root: &repo_root,
        out_dir: &out_dir,
        object_dir: &object_dir,
        target_os: &target_os,
        packages,
        toolchain,
    };
    create_and_validate_archive(&inputs, &validator, &archive_context)
}

fn validate_active_profile(
    inputs: &NativeInputManifest,
    validator: &Validator,
) -> Result<(), String> {
    let mut actual_features: Vec<String> = env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter_map(|name| name.strip_prefix("CARGO_FEATURE_").map(str::to_string))
        .map(|name| name.to_ascii_lowercase())
        .collect();
    actual_features.sort();
    let mut actual = inputs.production_profile.clone();
    actual.cargo_features = actual_features;
    if actual != validator.manifest().accepted_production_profile {
        return Err(format!(
            "actual production profile differs: expected {:?}, got {:?}",
            validator.manifest().accepted_production_profile,
            actual
        ));
    }
    validator
        .validate_production_profile(&actual)
        .map_err(|error| error.to_string())
}

fn validate_package_defines(
    target: &str,
    discovered: &BTreeSet<(String, Option<String>)>,
    authoritative: &[PreprocessorDefine],
) -> Result<(), String> {
    let mut expected: BTreeSet<_> = authoritative
        .iter()
        .map(|define| (define.name.clone(), define.value.clone()))
        .collect();
    if target.contains("linux") {
        expected.insert(("_REENTRANT".into(), None));
    }
    if let Some(unknown) = discovered.difference(&expected).next() {
        return Err(format!(
            "pkg-config preprocessor define differs from authoritative profile: {unknown:?}"
        ));
    }
    Ok(())
}

struct NativePackages {
    include_paths: BTreeSet<PathBuf>,
    defines: BTreeSet<(String, Option<String>)>,
    link_paths: BTreeSet<PathBuf>,
    link_files: BTreeSet<PathBuf>,
    libraries: BTreeSet<String>,
    framework_paths: BTreeSet<PathBuf>,
    frameworks: BTreeSet<String>,
}

impl NativePackages {
    fn extend(&mut self, other: Self) {
        self.include_paths.extend(other.include_paths);
        self.defines.extend(other.defines);
        self.link_paths.extend(other.link_paths);
        self.link_files.extend(other.link_files);
        self.libraries.extend(other.libraries);
        self.framework_paths.extend(other.framework_paths);
        self.frameworks.extend(other.frameworks);
    }
}

fn discover_packages(packages: &[&str]) -> Result<NativePackages, String> {
    let mut result = NativePackages {
        include_paths: BTreeSet::new(),
        defines: BTreeSet::new(),
        link_paths: BTreeSet::new(),
        link_files: BTreeSet::new(),
        libraries: BTreeSet::new(),
        framework_paths: BTreeSet::new(),
        frameworks: BTreeSet::new(),
    };
    for package in packages {
        let library = pkg_config::Config::new()
            .cargo_metadata(false)
            .probe(package)
            .map_err(|error| {
                format!("structured pkg-config discovery failed for {package}: {error}")
            })?;
        if library.ld_args.iter().any(|args| !args.is_empty()) {
            return Err(format!(
                "pkg-config returned unsupported raw linker arguments for {package}"
            ));
        }
        result.include_paths.extend(library.include_paths);
        result.defines.extend(library.defines);
        result.link_paths.extend(library.link_paths);
        result.link_files.extend(library.link_files);
        result.libraries.extend(library.libs);
        result.framework_paths.extend(library.framework_paths);
        result.frameworks.extend(library.frameworks);
    }
    Ok(result)
}

fn native_compile_profile(
    root: &Path,
    target: &str,
    toolchain: &ToolchainIdentity,
    inputs: &NativeInputManifest,
    packages: &NativePackages,
) -> Result<NativeCompileProfile, String> {
    let mut ordered_defines = vec![
        format!("-DUQM_BUILD_DATE=\"{}\"", build_date()?),
        "-D__DATE__=UQM_BUILD_DATE".into(),
        "-D__TIME__=\"00:00:00\"".into(),
    ];
    ordered_defines.extend(
        inputs
            .production_profile
            .defines
            .iter()
            .map(PreprocessorDefine::compiler_argument),
    );
    let mut ordered_include_roots: Vec<_> = packages
        .include_paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    ordered_include_roots.extend(
        REPOSITORY_INCLUDE_ROOTS
            .iter()
            .map(|path| root.join(path).to_string_lossy().into_owned()),
    );
    let mut command_template = vec![toolchain.cc.executable.clone()];
    command_template.extend(ordered_defines.iter().cloned());
    for include in &ordered_include_roots {
        command_template.extend(["-I".into(), include.clone()]);
    }
    command_template.extend(inputs.production_profile.compile_flags.iter().cloned());
    command_template.extend(DEPENDENCY_FLAGS.iter().map(|value| (*value).into()));
    command_template.extend([
        "-c".into(),
        "<canonical-source>".into(),
        "-o".into(),
        "<object-output>".into(),
    ]);
    Ok(NativeCompileProfile {
        target: target.into(),
        compiler: toolchain.cc.executable.clone(),
        ordered_defines,
        ordered_include_roots,
        ordered_compile_flags: inputs.production_profile.compile_flags.clone(),
        dependency_flags: DEPENDENCY_FLAGS
            .iter()
            .map(|value| (*value).into())
            .collect(),
        command_template,
    })
}

fn native_build_evidence(
    root: &Path,
    target: &str,
    toolchain: &ToolchainIdentity,
    inputs: &NativeInputManifest,
    profile: &NativeCompileProfile,
) -> Result<NativeBuildEvidence, String> {
    let epoch = source_date_epoch()?;
    let mut active_features: Vec<_> = env::vars_os()
        .filter_map(|(name, _)| name.into_string().ok())
        .filter_map(|name| name.strip_prefix("CARGO_FEATURE_").map(str::to_string))
        .map(|name| name.to_ascii_lowercase())
        .collect();
    active_features.sort();
    if active_features != inputs.production_profile.cargo_features {
        return Err(format!(
            "active semantic Cargo features differ from exact profile: expected {:?}, got {active_features:?}",
            inputs.production_profile.cargo_features
        ));
    }
    Ok(NativeBuildEvidence {
        schema: BUILD_EVIDENCE_SCHEMA.into(),
        source_date_epoch: epoch,
        build_date: build_date()?,
        target: target.into(),
        active_features,
        toolchain: toolchain.clone(),
        packages: discover_package_identities(
            root,
            &toolchain.pkg_config,
            uqm_ownership::production_packages(target),
        )?,
        compile_profile: profile.clone(),
        build_environment: canonical_build_environment(toolchain, epoch),
    })
}

fn compile_native_inputs(
    inputs: &NativeInputManifest,
    root: &Path,
    object_dir: &Path,
    profile: &NativeCompileProfile,
) -> Result<BTreeSet<String>, String> {
    let mut observed = BTreeSet::new();
    for input in &inputs.inputs {
        let output = object_dir.join(&input.object_output);
        let depfile = output.with_extension("d");
        let source = canonical_absolute(root, &input.source)?;
        let argv = profile.compiler_argv(&source, &output, &depfile);
        let (compiler, arguments) = argv
            .split_first()
            .ok_or_else(|| "canonical compiler invocation is empty".to_string())?;
        let status = Command::new(compiler)
            .args(arguments)
            .status()
            .map_err(|error| format!("cannot compile {}: {error}", source.display()))?;
        if !status.success() {
            return Err(format!(
                "C compiler rejected canonical input {}",
                source.display()
            ));
        }
        observed.extend(parse_dependency_file(&depfile, root)?);
        println!("cargo:rerun-if-changed=../{}", input.source);
    }
    Ok(observed)
}

fn write_dependency_capture(
    path: &Path,
    target: &str,
    observed: &BTreeSet<String>,
    inputs: &NativeInputManifest,
) -> Result<(), String> {
    let sources: BTreeSet<_> = inputs
        .inputs
        .iter()
        .map(|input| input.source.as_str())
        .collect();
    let dependencies: Vec<_> = observed
        .iter()
        .filter(|path| !sources.contains(path.as_str()))
        .cloned()
        .collect();
    let mut bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "schema": "uqm-native-dependency-capture-v1",
        "target": target,
        "dependencies": dependencies,
    }))
    .map_err(|error| format!("cannot serialize dependency capture: {error}"))?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|error| {
        format!(
            "cannot write dependency capture {}: {error}",
            path.display()
        )
    })
}

struct ArchiveContext<'a> {
    root: &'a Path,
    out_dir: &'a Path,
    object_dir: &'a Path,
    target_os: &'a str,
    packages: &'a NativePackages,
    toolchain: &'a ToolchainIdentity,
}

fn create_and_validate_archive(
    inputs: &NativeInputManifest,
    validator: &Validator,
    context: &ArchiveContext<'_>,
) -> Result<(), String> {
    let canonical_inputs = archive_sidecar_inputs(inputs, validator.manifest())?;
    if canonical_inputs.iter().any(|path| path == DISPLIST_OBJECT) {
        return Err(format!(
            "excluded duplicate provider reached archive membership: {DISPLIST_OBJECT}"
        ));
    }
    let sidecar = context.out_dir.join("uqm-c-objects.manifest");
    fs::write(&sidecar, format!("{}\n", canonical_inputs.join("\n")))
        .map_err(|error| format!("cannot write {}: {error}", sidecar.display()))?;
    validator
        .validate(&ValidateOptions {
            repo_root: context.root.to_path_buf(),
            check_disk_objects: false,
            check_archive: true,
            archive_path: Some(sidecar),
            check_strict_link: true,
        })
        .map_err(|error| error.to_string())?;
    let mut objects: Vec<_> = inputs
        .inputs
        .iter()
        .map(|input| context.object_dir.join(&input.object_output))
        .collect();
    objects.sort();
    let archive = context.out_dir.join("libuqm_c.a");
    remove_if_present(&archive)?;
    run_status(
        Command::new(&context.toolchain.ar.executable)
            .env("ZERO_AR_DATE", "1")
            .arg("rcs")
            .arg(&archive)
            .args(&objects),
        "production archive creation",
    )?;
    validator
        .validate_archive_file(&archive)
        .map_err(|error| error.to_string())?;
    write_provider_report(validator, context.out_dir)?;
    emit_archive_link(context.target_os, context.out_dir, &archive)?;
    emit_package_links(context.packages, context.target_os);
    Ok(())
}

fn archive_sidecar_inputs(
    inputs: &NativeInputManifest,
    providers: &uqm_ownership::Manifest,
) -> Result<Vec<String>, String> {
    let recompiled: BTreeSet<_> = providers
        .recompiled_objects
        .iter()
        .map(|item| item.object_output.as_str())
        .collect();
    let mut paths = Vec::with_capacity(inputs.inputs.len());
    for input in &inputs.inputs {
        if recompiled.contains(input.object_output.as_str()) {
            paths.push(input.source.clone());
        } else {
            paths.push(format!("native/{}", input.object_output));
        }
    }
    paths.sort();
    Ok(paths)
}

fn write_provider_report(validator: &Validator, out_dir: &Path) -> Result<(), String> {
    let path = out_dir.join("provider-report.json");
    let report = validator
        .generate_report()
        .to_json()
        .map_err(|error| error.to_string())?;
    fs::write(&path, format!("{report}\n"))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn emit_archive_link(target_os: &str, out_dir: &Path, archive: &Path) -> Result<(), String> {
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    match target_os {
        "macos" => {
            println!(
                "cargo:rustc-link-arg-bin=uqm=-Wl,-force_load,{}",
                archive.display()
            );
        }
        "linux" => {
            println!("cargo:rustc-link-arg-bin=uqm=-Wl,--whole-archive");
            println!("cargo:rustc-link-arg-bin=uqm={}", archive.display());
            println!("cargo:rustc-link-arg-bin=uqm=-Wl,--no-whole-archive");
        }
        other => return Err(format!("unsupported validated target OS: {other}")),
    }
    Ok(())
}

fn emit_package_links(packages: &NativePackages, target_os: &str) {
    for path in &packages.link_paths {
        println!("cargo:rustc-link-search=native={}", path.display());
    }
    for path in &packages.framework_paths {
        println!("cargo:rustc-link-search=framework={}", path.display());
    }
    for path in &packages.link_files {
        println!("cargo:rustc-link-arg-bin=uqm={}", path.display());
    }
    for library in &packages.libraries {
        println!("cargo:rustc-link-lib={library}");
    }
    for library in ["z", "m"] {
        println!("cargo:rustc-link-lib={library}");
    }
    for framework in &packages.frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=objc");
        for framework in ["Cocoa", "CoreAudio", "AudioToolbox", "CoreFoundation"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}

fn generate_hash_abi_bindings() -> Result<(), String> {
    let out_dir = output_directory()?;
    let char_header = "../sc2/src/libs/uio/charhashtable.h";
    let string_header = "../sc2/src/libs/strings/stringhashtable.h";
    for header in [
        char_header,
        string_header,
        "../sc2/src/libs/uio/hashtable.h",
    ] {
        println!("cargo:rerun-if-changed={header}");
    }
    let bindings = bindgen::Builder::default()
        .header(char_header)
        .header(string_header)
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .allowlist_type("(Char|String)HashTable_.*")
        .allowlist_function("(Char|String)HashTable_.*")
        .generate_comments(false)
        .layout_tests(false)
        .generate()
        .map_err(|error| format!("cannot generate retained hash-table ABI bindings: {error}"))?;
    let generated = canonicalize_hash_abi_names(bindings.to_string());
    fs::write(out_dir.join("hash_table_abi.rs"), generated)
        .map_err(|error| format!("cannot write generated hash-table ABI bindings: {error}"))
}

fn canonicalize_hash_abi_names(mut bindings: String) -> String {
    let replacements = [
        ("CharHashTable_", "CharHashTable"),
        ("StringHashTable_", "StringHashTable"),
        ("STRING_TABLE_ENTRY_DESC", "StringTableEntryDesc"),
        ("string_table_entry", "StringTableEntry"),
        ("uio_bool", "UioBool"),
        ("uio_uint32", "UioUint32"),
        ("hashFunction", "hash_function"),
        ("equalFunction", "equal_function"),
        ("copyFunction", "copy_function"),
        ("freeKeyFunction", "free_key_function"),
        ("freeValueFunction", "free_value_function"),
        ("minFillQuotient", "min_fill_quotient"),
        ("maxFillQuotient", "max_fill_quotient"),
        ("initialSize", "initial_size"),
        ("minSize", "min_size"),
        ("maxSize", "max_size"),
        ("hashMask", "hash_mask"),
        ("numEntries", "num_entries"),
        ("numCollisions", "num_collisions"),
        ("hashTable", "hash_table"),
        ("bucketNr", "bucket_nr"),
    ];
    for (original, canonical) in replacements {
        bindings = bindings.replace(original, canonical);
    }
    for family in ["CharHashTable", "StringHashTable"] {
        for function in [
            "newHashTable",
            "add",
            "remove",
            "find",
            "count",
            "deleteHashTable",
            "getIterator",
            "iteratorDone",
            "iteratorKey",
            "iteratorValue",
            "iteratorNext",
            "freeIterator",
        ] {
            let declaration = format!("    pub fn {family}{function}(");
            let linked = format!("    #[link_name = \"{family}_{function}\"]\n{declaration}");
            bindings = bindings.replace(&declaration, &linked);
        }
    }
    bindings
}

fn compile_p00_harness(toolchain: &ToolchainIdentity) -> Result<(), String> {
    let manifest_dir = PathBuf::from(env_value("CARGO_MANIFEST_DIR")?);
    let sdl2_includes = env::var_os("DEP_SDL2_INCLUDE")
        .ok_or_else(|| "SDL2 dependency did not publish DEP_SDL2_INCLUDE".to_string())?;
    let sdl2_includes: Vec<PathBuf> = env::split_paths(&sdl2_includes).collect();
    if sdl2_includes.is_empty() {
        return Err("DEP_SDL2_INCLUDE did not contain an SDL2 include directory".into());
    }
    let sc2_dir = manifest_dir.join("../sc2");
    let harness_dir = manifest_dir.join("harness");
    let out_dir = output_directory()?;
    cc::Build::new()
        .warnings(true)
        .file(harness_dir.join("sdl_surface_accessors.c"))
        .include(&harness_dir)
        .include(&sc2_dir)
        .includes(&sdl2_includes)
        .cpp(false)
        .compile("p00_sdl_accessors");
    let harness_obj = out_dir.join("p00_harness.o");
    compile_harness_c(
        &harness_dir.join("p00_harness.c"),
        &harness_obj,
        std::slice::from_ref(&harness_dir),
        toolchain,
    )?;
    let menu_accessor = out_dir.join("menu_binding_accessor.o");
    let mut menu_includes = vec![sc2_dir.join("src"), sc2_dir.clone()];
    menu_includes.extend(sdl2_includes);
    compile_harness_c(
        &harness_dir.join("menu_binding_accessor.c"),
        &menu_accessor,
        &menu_includes,
        toolchain,
    )?;
    let menu_probe = out_dir.join("menu_binding_probe.o");
    compile_harness_c(
        &harness_dir.join("menu_binding_probe.c"),
        &menu_probe,
        std::slice::from_ref(&harness_dir),
        toolchain,
    )?;
    let archive = out_dir.join("libp00_harness_shim.a");
    remove_if_present(&archive)?;
    run_status(
        Command::new(&toolchain.ar.executable)
            .arg("rcs")
            .arg(&archive)
            .args([&harness_obj, &menu_accessor]),
        "P00 harness archive",
    )?;
    for source in [
        "harness/sdl_surface_accessors.c",
        "harness/sdl_surface_accessors.h",
        "harness/menu_binding_accessor.c",
        "harness/menu_binding_accessor.h",
        "harness/menu_binding_probe.c",
        "harness/p00_harness.c",
        "harness/p00_harness.h",
    ] {
        println!("cargo:rerun-if-changed={source}");
    }
    Ok(())
}

fn compile_harness_c(
    source: &Path,
    output: &Path,
    includes: &[PathBuf],
    toolchain: &ToolchainIdentity,
) -> Result<(), String> {
    let mut command = Command::new(&toolchain.cc.executable);
    for include in includes {
        command.arg("-I").arg(include);
    }
    run_status(
        command.arg("-c").arg(source).arg("-o").arg(output),
        &format!("P00 compile {}", source.display()),
    )
}

fn run_status(command: &mut Command, label: &str) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("cannot execute {label}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with {status}"))
    }
}

fn output_directory() -> Result<PathBuf, String> {
    env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "OUT_DIR is unavailable".into())
}

fn env_value(name: &str) -> Result<String, String> {
    env::var(name).map_err(|error| format!("{name} is unavailable: {error}"))
}

fn build_date() -> Result<String, String> {
    println!("cargo:rerun-if-env-changed=UQM_BUILD_DATE");
    let value = env::var("UQM_BUILD_DATE").unwrap_or_else(|_| "Jan  1 1970".to_string());
    if value.is_empty() || value.contains('"') || value.contains('\n') {
        return Err("UQM_BUILD_DATE is not a valid C string value".to_string());
    }
    Ok(value)
}

fn source_date_epoch() -> Result<u64, String> {
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    let value = env_value("SOURCE_DATE_EPOCH")?;
    value
        .parse()
        .map_err(|error| format!("invalid SOURCE_DATE_EPOCH {value:?}: {error}"))
}

fn remove_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot remove stale {}: {error}", path.display())),
    }
}

fn generate_state_bindings(globdata_path: &Path) {
    println!("cargo:rerun-if-changed={}", globdata_path.display());

    let source = require(
        fs::read_to_string(globdata_path),
        "failed to read sc2/src/uqm/globdata.h for Rust state bindings",
    );

    let mut bit = 0usize;
    let mut entries = Vec::new();

    for line in source.lines() {
        let Some((name, width)) = parse_game_state_entry(line) else {
            continue;
        };

        let start = bit;
        let end = bit + width - 1;
        entries.push((name, start, end));
        bit += width;
    }

    let num_bits = bit;
    let num_bytes = (num_bits + 7) >> 3;
    let out_dir = PathBuf::from(require_some(env::var_os("OUT_DIR"), "OUT_DIR not set"));
    let mut generated = String::new();

    generated.push_str("// @generated by rust/build.rs from sc2/src/uqm/globdata.h\n");
    generated.push_str("// Do not edit by hand.\n\n");
    generated.push_str(&format!(
        "pub const NUM_GAME_STATE_BITS: usize = {num_bits};\n"
    ));
    generated.push_str(&format!(
        "pub const NUM_GAME_STATE_BYTES: usize = {num_bytes};\n\n"
    ));
    generated.push_str("pub fn lookup_game_state_bits(name: &str) -> Option<(usize, usize)> {\n");
    generated.push_str("    match name {\n");

    for (name, start, end) in entries {
        generated.push_str(&format!("        \"{name}\" => Some(({start}, {end})),\n"));
    }

    generated.push_str("        _ => None,\n");
    generated.push_str("    }\n");
    generated.push_str("}\n");

    require(
        fs::write(out_dir.join("state_generated.rs"), generated),
        "failed to write generated Rust state bindings",
    );
}

fn parse_game_state_entry(line: &str) -> Option<(String, usize)> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || !trimmed.contains("ADD_GAME_STATE") {
        return None;
    }

    let open = trimmed.find('(')?;
    let close = trimmed[open + 1..].find(')')? + open + 1;
    let inner = &trimmed[open + 1..close];
    let mut parts = inner.split(',').map(str::trim);

    let name = parts.next()?;
    let width = parts.next()?.parse().ok()?;

    Some((name.to_string(), width))
}
