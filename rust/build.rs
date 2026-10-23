use std::collections::BTreeSet;
use std::env;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// One C source per line, relative to the repository root. Deleting a ported
/// C file means deleting it and its line here.
const C_SOURCE_LIST: &str = "build/c-sources.txt";
const SDL_PACKAGE: [&str; 1] = ["sdl2"];
const REPOSITORY_INCLUDE_ROOTS: [&str; 3] = ["sc2", "sc2/src", "sc2/src/libs/uio"];
const COMPILE_FLAGS: [&str; 3] = ["-std=gnu99", "-O2", "-fPIC"];

/// Preprocessor defines every C object is compiled with.
const C_DEFINES: [&str; 36] = [
    "GFXMODULE_SDL",
    "HAVE_JOYSTICK",
    "HAVE_ZIP=1",
    "NETPLAY=NETPLAY_FULL",
    "OVCODEC_NONE",
    "RUST_OWNS_MAIN",
    "SDL_DIR=SDL2",
    "THREADLIB_SDL",
    "USE_INTERNAL_MIKMOD",
    "USE_PLATFORM_ACCEL",
    "USE_RUST_AIFF",
    "USE_RUST_AUDIO",
    "USE_RUST_AUDIO_HEART",
    "USE_RUST_BRIDGE",
    "USE_RUST_CLOCK",
    "USE_RUST_COMM",
    "USE_RUST_DUKAUD",
    "USE_RUST_FILE",
    "USE_RUST_GFX",
    "USE_RUST_INPUT",
    "USE_RUST_MAINLOOP=1",
    "USE_RUST_MEM",
    "USE_RUST_MIXER",
    "USE_RUST_MOD",
    "USE_RUST_OGG",
    "USE_RUST_RESOURCE",
    "USE_RUST_RESTART",
    "USE_RUST_SHIPS",
    "USE_RUST_STATE",
    "USE_RUST_SUPERMELEE",
    "USE_RUST_THREADS",
    "USE_RUST_UIO",
    "USE_RUST_VIDEO",
    "USE_RUST_VIDPLAYER",
    "USE_RUST_WAV",
    "_THREAD_SAFE",
];

/// Libraries and frameworks the game links besides SDL, per target OS.
struct PlatformLinks {
    packages: &'static [&'static str],
    libraries: &'static [&'static str],
    frameworks: &'static [&'static str],
}

fn platform_links(target_os: &str) -> Result<PlatformLinks, String> {
    match target_os {
        "macos" => Ok(PlatformLinks {
            packages: &["liblzma", "bzip2"],
            libraries: &["sandbox", "z", "m", "objc"],
            frameworks: &["Cocoa", "CoreAudio", "AudioToolbox", "CoreFoundation"],
        }),
        "linux" => Ok(PlatformLinks {
            packages: &["liblzma"],
            libraries: &["z", "m", "bz2"],
            frameworks: &[],
        }),
        other => Err(format!("unsupported target OS: {other}")),
    }
}

fn main() {
    if let Err(error) = run() {
        fail(error);
    }
}

fn run() -> Result<(), String> {
    let mut packages = discover_packages(&SDL_PACKAGE)?;
    let target_os = env_value("CARGO_CFG_TARGET_OS")?;
    let links = platform_links(&target_os)?;
    if env::var_os("CARGO_FEATURE_LINKED_C_ARCHIVE").is_some() {
        packages.extend(discover_packages(links.packages)?);
        build_c_archive(&target_os, &packages)?;
    } else {
        emit_package_links(&packages);
    }
    emit_platform_links(&links);
    Ok(())
}

fn fail(message: impl Display) -> ! {
    eprintln!("build failed: {message}");
    std::process::exit(1)
}

struct NativePackages {
    include_paths: BTreeSet<PathBuf>,
    link_paths: BTreeSet<PathBuf>,
    link_files: BTreeSet<PathBuf>,
    libraries: BTreeSet<String>,
    framework_paths: BTreeSet<PathBuf>,
    frameworks: BTreeSet<String>,
}

impl NativePackages {
    fn extend(&mut self, other: Self) {
        self.include_paths.extend(other.include_paths);
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
            .map_err(|error| format!("pkg-config discovery failed for {package}: {error}"))?;
        result.include_paths.extend(library.include_paths);
        result.link_paths.extend(library.link_paths);
        result.link_files.extend(library.link_files);
        result.libraries.extend(library.libs);
        result.framework_paths.extend(library.framework_paths);
        result.frameworks.extend(library.frameworks);
    }
    Ok(result)
}

/// Compile every listed C source and archive the objects into `libuqm_c.a`.
fn build_c_archive(target_os: &str, packages: &NativePackages) -> Result<(), String> {
    let rust_root = PathBuf::from(env_value("CARGO_MANIFEST_DIR")?);
    let repo_root = rust_root
        .parent()
        .ok_or_else(|| "Rust crate has no repository parent".to_string())?
        .to_path_buf();
    let list_path = rust_root.join(C_SOURCE_LIST);
    println!("cargo:rerun-if-changed={}", list_path.display());
    // Headers are not tracked individually; any change under sc2/src rebuilds.
    println!(
        "cargo:rerun-if-changed={}",
        repo_root.join("sc2/src").display()
    );
    let list = fs::read_to_string(&list_path)
        .map_err(|error| format!("cannot read {}: {error}", list_path.display()))?;
    let sources: Vec<&str> = list
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    if sources.is_empty() {
        emit_package_links(packages);
        return Ok(());
    }

    let out_dir = output_directory()?;
    let object_dir = out_dir.join("native");
    fs::create_dir_all(&object_dir)
        .map_err(|error| format!("cannot create {}: {error}", object_dir.display()))?;

    let compiler = env::var("CC").unwrap_or_else(|_| "cc".into());
    let mut common: Vec<String> = C_DEFINES
        .iter()
        .map(|define| format!("-D{define}"))
        .collect();
    if env::var_os("CARGO_FEATURE_DEBUG_PROCESS").is_some() {
        common.push("-DDEBUG".into());
    }
    for include in &packages.include_paths {
        common.extend(["-I".into(), include.to_string_lossy().into_owned()]);
    }
    for root in REPOSITORY_INCLUDE_ROOTS {
        common.extend([
            "-I".into(),
            repo_root.join(root).to_string_lossy().into_owned(),
        ]);
    }
    common.extend(COMPILE_FLAGS.map(String::from));

    let mut objects = Vec::with_capacity(sources.len());
    for source in &sources {
        let object = object_dir.join(format!("{}.o", source.replace('/', "__")));
        run_status(
            Command::new(&compiler)
                .args(&common)
                .arg("-c")
                .arg(repo_root.join(source))
                .arg("-o")
                .arg(&object),
            &format!("compile {source}"),
        )?;
        objects.push(object);
    }

    let archive = out_dir.join("libuqm_c.a");
    match fs::remove_file(&archive) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "cannot remove stale {}: {error}",
                archive.display()
            ))
        }
    }
    run_status(
        Command::new(env::var("AR").unwrap_or_else(|_| "ar".into()))
            .arg("rcs")
            .arg(&archive)
            .args(&objects),
        "archive creation",
    )?;
    emit_archive_link(target_os, &out_dir, &archive)?;
    emit_package_links(packages);
    Ok(())
}

fn emit_archive_link(target_os: &str, out_dir: &Path, archive: &Path) -> Result<(), String> {
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    match target_os {
        "macos" => {
            // The library records no #[link] for the C archive so pure test
            // builds stay C-free; the game binary force-loads the archive
            // itself to resolve the C registration symbols.
            println!(
                "cargo:rustc-link-arg-bin=uqm=-Wl,-force_load,{}",
                archive.display()
            );
        }
        "linux" => {
            println!("cargo:rustc-link-lib=static:+whole-archive,-bundle=uqm_c");
        }
        other => return Err(format!("unsupported target OS: {other}")),
    }
    Ok(())
}

fn emit_package_links(packages: &NativePackages) {
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
    for framework in &packages.frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

fn emit_platform_links(links: &PlatformLinks) {
    for library in links.libraries {
        println!("cargo:rustc-link-lib={library}");
    }
    for framework in links.frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
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
