use std::env;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use uqm_ownership::{ValidateOptions, Validator, DISPLIST_OBJECT};

fn main() {
    generate_state_bindings(Path::new("../sc2/src/uqm/globdata.h"));

    cc::Build::new()
        .warnings(true)
        .file("src/io/uio_vfprintf_helper.c")
        .cpp(false)
        .compile("uio_vfprintf_helper");
    println!("cargo:rerun-if-changed=src/io/uio_vfprintf_helper.c");

    cc::Build::new()
        .warnings(true)
        .file("src/mainloop/rust_test_bridge.c")
        .cpp(false)
        .compile("uqm_test_bridge");
    println!("cargo:rerun-if-changed=src/mainloop/rust_test_bridge.c");

    if env::var_os("CARGO_FEATURE_LINKED_C_ARCHIVE").is_some() {
        if let Err(error) = link_c_objects() {
            fail(format!(
                "S1 ownership/strict-link validation failed: {error}"
            ));
        }
    }
    compile_p00_harness();
}

fn fail(message: impl Display) -> ! {
    eprintln!("{message}");
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

/// Build a production archive exclusively from exact manifest paths.
fn link_c_objects() -> Result<(), String> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map_err(|error| format!("CARGO_MANIFEST_DIR is unavailable: {error}"))?;
    let rust_root = PathBuf::from(&manifest_dir);
    let repo_root = rust_root
        .parent()
        .ok_or_else(|| "Rust crate has no repository parent".to_string())?
        .to_path_buf();
    let manifest_path = rust_root.join("ownership/native-provider-manifest.json");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let validator =
        Validator::from_manifest_file(&manifest_path).map_err(|error| error.to_string())?;
    validator
        .validate(&ValidateOptions::production(repo_root.clone()))
        .map_err(|error| error.to_string())?;
    let manifest = validator.manifest();

    let build_vars_path = repo_root.join("sc2/build.vars");
    let build_vars = fs::read_to_string(&build_vars_path)
        .map_err(|error| format!("cannot read {}: {error}", build_vars_path.display()))?;
    let cflags_base = build_vars
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("uqm_CFLAGS='")
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .ok_or_else(|| "uqm_CFLAGS is absent from sc2/build.vars".to_string())?;
    let sc2_dir = repo_root.join("sc2");
    let sc2_include = format!("-I{} -I{}/src", sc2_dir.display(), sc2_dir.display());
    let cflags = format!(
        "{} -DRUST_OWNS_MAIN -DUSE_RUST_MAINLOOP=1 -w -c",
        cflags_base
            .replace("-W -Wall", "")
            .replace("-I\x22.\x22", &sc2_include)
            .replace("-I.", &sc2_include),
    );
    let config_path = repo_root.join("sc2/config_unix.h");
    let config = fs::read_to_string(&config_path)
        .map_err(|error| format!("cannot read {}: {error}", config_path.display()))?;
    let active_features: Vec<_> = manifest
        .accepted_production_profile
        .cargo_features
        .iter()
        .filter(|feature| {
            let variable = format!("CARGO_FEATURE_{}", feature.to_ascii_uppercase());
            env::var_os(variable.replace('-', "_")).is_some()
        })
        .map(String::as_str)
        .collect();
    validator
        .validate_production_profile(&active_features, cflags_base, &config, &cflags)
        .map_err(|error| error.to_string())?;

    let out_dir = env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "OUT_DIR is unavailable".to_string())?;

    let mut archive_inputs = Vec::new();
    let mut canonical_inputs = Vec::new();
    for object in manifest.included_objects() {
        let path = repo_root.join(&object.path);
        archive_inputs.push(path);
        canonical_inputs.push(object.path.clone());
    }
    for object in &manifest.recompiled_objects {
        let source = repo_root.join(&object.repo_relative_path);
        let output = out_dir.join(&object.object_output);
        compile_c_file(&source, &output, &cflags)?;
        archive_inputs.push(output);
        canonical_inputs.push(object.repo_relative_path.clone());
    }
    archive_inputs.sort();
    canonical_inputs.sort();

    if canonical_inputs.iter().any(|path| path == DISPLIST_OBJECT) {
        return Err(format!(
            "excluded duplicate provider reached archive membership: {DISPLIST_OBJECT}"
        ));
    }

    let sidecar = out_dir.join("uqm-c-objects.manifest");
    fs::write(&sidecar, format!("{}\n", canonical_inputs.join("\n")))
        .map_err(|error| format!("cannot write {}: {error}", sidecar.display()))?;
    validator
        .validate(&ValidateOptions {
            repo_root: repo_root.clone(),
            check_disk_objects: false,
            check_archive: true,
            archive_path: Some(sidecar),
            check_strict_link: true,
        })
        .map_err(|error| error.to_string())?;

    let archive_path = out_dir.join("libuqm_c.a");
    match fs::remove_file(&archive_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot remove stale {}: {error}",
                archive_path.display()
            ))
        }
    }
    let status = std::process::Command::new("ar")
        .arg("rcs")
        .arg(&archive_path)
        .args(&archive_inputs)
        .status()
        .map_err(|error| format!("cannot execute ar: {error}"))?;
    if !status.success() {
        return Err(format!(
            "ar failed while creating {}",
            archive_path.display()
        ));
    }
    validator
        .validate_archive_file(&archive_path)
        .map_err(|error| error.to_string())?;
    let report = validator.generate_report();
    let report_path = out_dir.join("provider-report.json");
    fs::write(
        &report_path,
        format!("{}\n", report.to_json().map_err(|error| error.to_string())?),
    )
    .map_err(|error| format!("cannot write {}: {error}", report_path.display()))?;

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    let target = env::var("CARGO_CFG_TARGET_OS")
        .map_err(|error| format!("CARGO_CFG_TARGET_OS is unavailable: {error}"))?;
    match target.as_str() {
        "macos" => println!(
            "cargo:rustc-link-arg-bin=uqm=-Wl,-force_load,{}",
            archive_path.display()
        ),
        "linux" => {
            println!("cargo:rustc-link-arg-bin=uqm=-Wl,--whole-archive");
            println!("cargo:rustc-link-arg-bin=uqm={}", archive_path.display());
            println!("cargo:rustc-link-arg-bin=uqm=-Wl,--no-whole-archive");
        }
        unsupported => return Err(format!("unsupported strict-link target OS: {unsupported}")),
    }

    for library in ["png16", "z", "m", "SDL2", "lzma", "bz2"] {
        println!("cargo:rustc-link-arg=-l{library}");
    }
    if target == "macos" {
        println!("cargo:rustc-link-arg=-lobjc");
        for framework in ["Cocoa", "CoreAudio", "AudioToolbox", "CoreFoundation"] {
            println!("cargo:rustc-link-arg=-framework");
            println!("cargo:rustc-link-arg={framework}");
        }
        for path in [
            "/opt/homebrew/lib",
            "/opt/homebrew/opt/libpng/lib",
            "/opt/homebrew/opt/SDL2/lib",
        ] {
            println!("cargo:rustc-link-search=native={path}");
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../sc2/build.vars");
    println!("cargo:rerun-if-changed=../sc2/config_unix.h");
    for object in &manifest.objects {
        println!("cargo:rerun-if-changed=../{}", object.path);
    }
    for object in &manifest.recompiled_objects {
        println!("cargo:rerun-if-changed=../{}", object.repo_relative_path);
    }
    Ok(())
}

fn compile_c_file(source: &Path, output: &Path, cflags: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("cc");
    for token in shell_tokenize(cflags) {
        command.arg(token);
    }
    let status = command
        .arg("-o")
        .arg(output)
        .arg(source)
        .status()
        .map_err(|error| format!("cannot compile {}: {error}", source.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("C compiler rejected {}", source.display()))
    }
}

fn shell_tokenize(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for character in value.chars() {
        match character {
            '"' => in_quotes = !in_quotes,
            character if character.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            character => current.push(character),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Compile P00 harness C sources.
///
/// SDL surface accessors are auto-linked via cc::Build::compile so they're
/// available to lib and test targets. The harness entry and menu binding
/// accessor are compiled as object files only — they reference production
/// symbols from libuqm_c.a and are linked by the probe script with the
/// correct force-load ordering per §8.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8
fn compile_p00_harness() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    // Read SDL2 include path from build.vars
    let build_vars_path = Path::new(&manifest_dir).join("../sc2/build.vars");
    let build_vars = fs::read_to_string(&build_vars_path).unwrap_or_default();

    // Extract SDL2 include path from CFLAGS (e.g. -I/opt/homebrew/include/SDL2)
    let sdl2_inc = build_vars
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("uqm_CFLAGS=") {
                let rest = trimmed
                    .strip_prefix("uqm_CFLAGS='")
                    .and_then(|r| r.strip_suffix("'"))?;
                for token in rest.split_whitespace() {
                    if token.starts_with("-I") && token.contains("SDL2") {
                        return Some(token[2..].to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| "/opt/homebrew/include/SDL2".to_string());

    let sc2_dir = Path::new(&manifest_dir).join("../sc2");
    let harness_dir = Path::new(&manifest_dir).join("harness");
    let out_dir = PathBuf::from(require_some(env::var_os("OUT_DIR"), "OUT_DIR not set"));

    // SDL surface accessors — auto-linked into all targets (no production symbol refs)
    cc::Build::new()
        .warnings(true)
        .file("harness/sdl_surface_accessors.c")
        .include(&harness_dir)
        .include(&sc2_dir)
        .include(&sdl2_inc)
        .cpp(false)
        .compile("p00_sdl_accessors");

    // Harness entry — compiled as object only (references production symbols)
    let harness_obj = out_dir.join("p00_harness.o");
    compile_harness_c(
        &harness_dir.join("p00_harness.c"),
        &harness_obj,
        &format!("-I{} -w -c", harness_dir.display()),
    );

    // Menu binding accessor — compiled as object only (references production symbols)
    let menu_accessor_obj = out_dir.join("menu_binding_accessor.o");
    compile_harness_c(
        &harness_dir.join("menu_binding_accessor.c"),
        &menu_accessor_obj,
        &format!(
            "-I{}/src -I{} -I{} -w -c",
            sc2_dir.display(),
            sc2_dir.display(),
            sdl2_inc
        ),
    );

    // Menu binding probe — compiled as a separate object only.
    // It defines its own main() so it must NOT be in the shared harness
    // archive (which the P00 link-map probe links with an inline main()).
    // The probe script links this object directly.
    let menu_probe_obj = out_dir.join("menu_binding_probe.o");
    compile_harness_c(
        &harness_dir.join("menu_binding_probe.c"),
        &menu_probe_obj,
        &format!("-I{} -w -c", harness_dir.display()),
    );

    // Archive harness + menu accessor (NOT the probe) for probe script use.
    // The harness archive must not contain any main() symbol.
    let harness_archive = out_dir.join("libp00_harness_shim.a");
    let _ = fs::remove_file(&harness_archive);

    let ar_status = std::process::Command::new("ar")
        .arg("rcs")
        .arg(&harness_archive)
        .arg(&harness_obj)
        .arg(&menu_accessor_obj)
        .status();

    if !ar_status.map(|s| s.success()).unwrap_or(false) {
        panic!("P00: Failed to create harness archive");
    }

    // Rerun-if-changed for all harness sources
    println!("cargo:rerun-if-changed=harness/sdl_surface_accessors.c");
    println!("cargo:rerun-if-changed=harness/sdl_surface_accessors.h");
    println!("cargo:rerun-if-changed=harness/menu_binding_accessor.c");
    println!("cargo:rerun-if-changed=harness/menu_binding_accessor.h");
    println!("cargo:rerun-if-changed=harness/menu_binding_probe.c");
    println!("cargo:rerun-if-changed=harness/p00_harness.c");
    println!("cargo:rerun-if-changed=harness/p00_harness.h");
}

/// Compile a single harness C source file to an object file.
fn compile_harness_c(source: &Path, output: &Path, cflags: &str) {
    let mut cmd = std::process::Command::new("cc");
    for token in shell_tokenize(cflags) {
        cmd.arg(&token);
    }
    cmd.arg("-o").arg(output).arg(source);

    let status = cmd.status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        panic!("P00: Failed to compile {}", source.display());
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
