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
}

/// Create a static archive from C object files and link it into the binary.
///
/// @plan PLAN-20260707-BINARY-INVERSION.P06
fn link_c_objects() {
    // Use absolute path to be robust against CWD differences
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    let obj_dir = Path::new(&manifest_dir)
        .join("../sc2/obj/release");

    // Collect all .o files from the C build
    let mut obj_files: Vec<PathBuf> = Vec::new();
    collect_object_files(&obj_dir, &mut obj_files);

    if obj_files.is_empty() {
        panic!("P06: No C object files found in {}. Run 'cd sc2 && ./build.sh uqm' first.", obj_dir.display());
    }

    // Read CFLAGS from build.vars to stay in sync with the C build.
    // We extract the uqm_CFLAGS value and supplement with RUST_OWNS_MAIN.
    let build_vars_path = Path::new(&manifest_dir).join("../sc2/build.vars");
    let build_vars = fs::read_to_string(&build_vars_path)
        .expect("failed to read sc2/build.vars for CFLAGS");
    let cflags_base = build_vars
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix("uqm_CFLAGS='")
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

    compile_c_file(
        &sc2_src.join("uqm.c"),
        &uqm_obj,
        &cflags_common,
    );
    compile_c_file(
        &sc2_src.join("uqm/gameinp.c"),
        &gameinp_obj,
        &cflags_common,
    );

    // Create a static archive from the object files
    // (excluding the original uqm.c.o and gameinp.c.o, which have
    // been recompiled with RUST_OWNS_MAIN above)
    let archive_path = out_dir.join("libuqm_c.a");

    // Remove old archive if it exists
    let _ = fs::remove_file(&archive_path);

    // Filter out the original uqm.c.o and gameinp.c.o — they're replaced
    // by the RUST_OWNS_MAIN versions compiled above
    let filtered_files: Vec<&PathBuf> = obj_files
        .iter()
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name != "uqm.c.o" && name != "gameinp.c.o"
        })
        .collect();

    // Use ar to create the archive (includes RUST_OWNS_MAIN versions)
    let ar_status = std::process::Command::new("ar")
        .arg("rcs")
        .arg(&archive_path)
        .args(&filtered_files)
        .arg(&uqm_obj)
        .arg(&gameinp_obj)
        .status();

    if !ar_status.map(|s| s.success()).unwrap_or(false) {
        panic!("P06: Failed to create C archive");
    }

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    // Only link the C archive into the binary, NOT the staticlib.
    // cargo:rustc-link-arg-bin only applies to the named binary target.
    // This prevents the staticlib from bundling C objects (which would
    // duplicate when the C binary also links the same .o files).
    println!("cargo:rustc-link-arg-bin=uqm=-luqm_c");

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

    println!("cargo:rerun-if-changed=../sc2/obj/release");
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
