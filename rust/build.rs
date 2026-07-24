use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    generate_state_bindings(Path::new("../sc2/src/uqm/globdata.h"));

    // Compile internal helper for uio_vfprintf (not an exported UIO symbol)
    cc::Build::new()
        .warnings(true)
        .file("src/io/uio_vfprintf_helper.c")
        .cpp(false)
        .compile("uio_vfprintf_helper");

    println!("cargo:rerun-if-changed=src/io/uio_vfprintf_helper.c");

    // Compile the main loop test bridge shim.
    cc::Build::new()
        .warnings(true)
        .file("src/mainloop/rust_test_bridge.c")
        .cpp(false)
        .compile("uqm_test_bridge");
    println!("cargo:rerun-if-changed=src/mainloop/rust_test_bridge.c");

    // ===========================================================
    // P06: Link C object files into the Rust binary.
    // ===========================================================
    // The C build system (sc2/build.sh) compiles all C sources into
    // .o files in sc2/obj/release/. We archive them into a static
    // library and link it into the Rust binary target.
    //
    // This makes the Rust binary the process entry point (main.rs)
    // while still using all the C game code.
    //
    // @plan PLAN-20260707-BINARY-INVERSION.P06
    link_c_objects();

    // ===========================================================
    // P00: Compile harness C sources for linked-harness feasibility.
    // ===========================================================
    // @plan PLAN-20260723-RUNTIME-AUTOMATION.P00 §8
    compile_p00_harness();
}

/// Create a static archive from C object files and link it into the binary.
///
/// @plan PLAN-20260707-BINARY-INVERSION.P06
fn link_c_objects() {
    // Use absolute path to be robust against CWD differences
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let obj_dir = Path::new(&manifest_dir).join("../sc2/obj/release");

    // Collect all .o files from the C build
    let mut obj_files: Vec<PathBuf> = Vec::new();
    collect_object_files(&obj_dir, &mut obj_files);
    obj_files.sort();

    if obj_files.is_empty() {
        panic!(
            "P06: No C object files found in {}. Run 'cd sc2 && ./build.sh uqm' first.",
            obj_dir.display()
        );
    }

    // Read CFLAGS from build.vars to stay in sync with the C build.
    // We extract the uqm_CFLAGS value and supplement with RUST_OWNS_MAIN.
    let build_vars_path = Path::new(&manifest_dir).join("../sc2/build.vars");
    let build_vars =
        fs::read_to_string(&build_vars_path).expect("failed to read sc2/build.vars for CFLAGS");
    let cflags_base = build_vars
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("uqm_CFLAGS='")
                .and_then(|rest| rest.strip_suffix("'"))
        })
        .expect("uqm_CFLAGS not found in build.vars")
        .to_string();

    // Build the CFLAGS for RUST_OWNS_MAIN compilation: use the C build's
    // flags plus RUST_OWNS_MAIN and USE_RUST_MAINLOOP.
    // Fix relative include paths (-I".") to resolve from sc2/ directory.
    // Remove -W -Wall (we use -w to suppress warnings from our recompiled files).
    let sc2_dir = Path::new(&manifest_dir).join("../sc2");
    let sc2_inc = format!("-I{} -I{}/src", sc2_dir.display(), sc2_dir.display());
    // Match -I"." from build.vars (\x22 = double quote)
    let dot_inc = "-I\x22.\x22";
    let cflags_common = format!(
        "{base} -DRUST_OWNS_MAIN -DUSE_RUST_MAINLOOP=1 -w -c",
        base = cflags_base
            .replace("-W -Wall", "")
            .replace(dot_inc, &sc2_inc)
            .replace("-I.", &sc2_inc),
    );

    let sc2_src = Path::new(&manifest_dir).join("../sc2/src");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));

    let uqm_obj = out_dir.join("uqm_rust_main.o");
    let gameinp_obj = out_dir.join("gameinp_rust_main.o");

    compile_c_file(&sc2_src.join("uqm.c"), &uqm_obj, &cflags_common);
    compile_c_file(&sc2_src.join("uqm/gameinp.c"), &gameinp_obj, &cflags_common);

    // Create a static archive from the object files
    // (excluding the original uqm.c.o and gameinp.c.o, which have
    // been recompiled with RUST_OWNS_MAIN above)
    let archive_path = out_dir.join("libuqm_c.a");

    // Remove old archive if it exists
    let _ = fs::remove_file(&archive_path);

    // Filter out the original uqm.c.o and gameinp.c.o — they're replaced
    // by the RUST_OWNS_MAIN versions compiled above
    let mut archive_inputs: Vec<&PathBuf> = obj_files
        .iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !matches!(
                name,
                "uqm.c.o"
                    | "gameinp.c.o"
                    | "alarm.c.o"
                    | "async.c.o"
                    | "callback.c.o"
                    | "gravity.c.o"
                    | "random.c.o"
                    | "random2.c.o"
                    | "sqrt.c.o"
                    | "velocity.c.o"
                    | "gendef.c.o"
                    | "collide.c.o"
                    | "trans.c.o"
                    | "battlecontrols.c.o"
            )
        })
        .collect();
    archive_inputs.extend([&uqm_obj, &gameinp_obj]);
    archive_inputs.sort_by_key(|path| path.display().to_string());

    let object_manifest = archive_inputs
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        out_dir.join("uqm-c-objects.manifest"),
        format!("{object_manifest}\n"),
    )
    .expect("failed to write deterministic C object manifest");

    // Use ar to create the archive (includes RUST_OWNS_MAIN versions)
    let ar_status = std::process::Command::new("ar")
        .arg("rcs")
        .arg(&archive_path)
        .args(&archive_inputs)
        .status();

    if !ar_status.map(|s| s.success()).unwrap_or(false) {
        panic!("P06: Failed to create C archive");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    // Force-load the C archive into the binary target so that all C object
    // files are included, resolving internal C-to-C reference chains (e.g.
    // Starcon2Main → audio functions) that normal archive extraction would
    // miss because no Rust code directly references the intermediate symbols.
    // cargo:rustc-link-arg-bin only applies to the named binary target,
    // so the staticlib is not affected.
    println!(
        "cargo:rustc-link-arg-bin=uqm=-Wl,-force_load,{}",
        archive_path.display()
    );
    // Allow unresolved transitional symbols (e.g. graphics_backend,
    // format_conv_surf from pure.c which is compiled out by USE_RUST_GFX).
    // These symbols are never called at runtime when the Rust graphics
    // driver is active.
    println!("cargo:rustc-link-arg-bin=uqm=-Wl,-undefined,dynamic_lookup");

    // Link external C libraries (from build.vars LDFLAGS)
    println!("cargo:rustc-link-arg=-lpng16");
    println!("cargo:rustc-link-arg=-lz");
    println!("cargo:rustc-link-arg=-lm");
    println!("cargo:rustc-link-arg=-lSDL2");
    println!("cargo:rustc-link-arg=-lobjc");
    println!("cargo:rustc-link-arg=-framework");
    println!("cargo:rustc-link-arg=Cocoa");
    println!("cargo:rustc-link-arg=-framework");
    println!("cargo:rustc-link-arg=CoreAudio");
    println!("cargo:rustc-link-arg=-framework");
    println!("cargo:rustc-link-arg=AudioToolbox");
    println!("cargo:rustc-link-arg=-framework");
    println!("cargo:rustc-link-arg=CoreFoundation");
    println!("cargo:rustc-link-arg=-llzma");
    println!("cargo:rustc-link-arg=-lbz2");

    // Add SDL2 and library search paths
    println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    println!("cargo:rustc-link-search=native=/opt/homebrew/opt/libpng/lib");
    println!("cargo:rustc-link-search=native=/opt/homebrew/opt/SDL2/lib");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../sc2/build.vars");
    println!("cargo:rerun-if-changed=../sc2/config_unix.h");
    for object in &obj_files {
        println!("cargo:rerun-if-changed={}", object.display());
    }
    println!("cargo:rerun-if-changed=../sc2/src/uqm.c");
    println!("cargo:rerun-if-changed=../sc2/src/uqm/gameinp.c");
}

/// Compile a single C source file to an object file.
/// Uses shell-like tokenization for CFLAGS to handle quoted paths.
fn compile_c_file(source: &Path, output: &Path, cflags: &str) {
    let mut cmd = std::process::Command::new("cc");
    for token in shell_tokenize(cflags) {
        cmd.arg(&token);
    }
    cmd.arg("-o").arg(output).arg(source);

    let status = cmd.status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        panic!("P06: Failed to compile {}", source.display());
    }
}

/// Simple shell-like tokenizer for CFLAGS strings.
/// Handles double-quoted arguments (e.g. -I"some path").
fn shell_tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in s.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Recursively collect all .o files from a directory.
fn collect_object_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_object_files(&path, files);
            } else if path.extension().map(|e| e == "o").unwrap_or(false) {
                files.push(path);
            }
        }
    }
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
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));

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

    let source = fs::read_to_string(globdata_path)
        .expect("failed to read sc2/src/uqm/globdata.h for Rust state bindings");

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
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
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

    fs::write(out_dir.join("state_generated.rs"), generated)
        .expect("failed to write generated Rust state bindings");
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
