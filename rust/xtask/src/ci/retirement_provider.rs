//! Promotion of source evidence using the actual production compiler outputs.

#[cfg(test)]
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use uqm_ownership::{
    Manifest, NativeBuildEvidence, ProductionArtifacts, SymbolContract, Validator,
};

use super::authority::Authority;
use super::delta::SourceRetirements;
use super::exec::run_captured_with_bound_environment;
use super::{bounded_io, evidence::hex_sha256, retirement};

/// This capability is never deserialized. Only successful compiler and archive
/// measurements below can create it; it is consumed in the controller process.
pub(crate) struct VerifiedRetirements {
    contracts: Vec<SymbolContract>,
    rust_archive_sha256: String,
}

impl VerifiedRetirements {
    pub(crate) fn validator(
        self,
        manifest: Manifest,
        artifacts: &ProductionArtifacts,
        limit: u64,
    ) -> Result<Validator, String> {
        if hex_sha256(&bounded_io::read_regular_nofollow(
            &artifacts.rust_archive,
            limit,
        )?) != self.rust_archive_sha256
        {
            return Err("compiled retirement archive changed before ownership measurement".into());
        }
        Validator::new(manifest)
            .with_verified_retirements(self.contracts)
            .map_err(|error| error.to_string())
    }
}

fn tool(
    root: &Path,
    authority: &Authority,
    executable: &str,
    args: &[String],
) -> Result<Vec<u8>, String> {
    let captured = run_captured_with_bound_environment(
        root,
        executable,
        args,
        authority.supervision.builtin_limits(),
        false,
        |_| Ok(Vec::new()),
    );
    if !captured.succeeded() {
        return Err(captured.failure_detail("retirement compiler provenance"));
    }
    Ok(captured.stdout)
}

#[cfg(test)]
fn members(
    root: &Path,
    authority: &Authority,
    ar: &str,
    archive: &Path,
) -> Result<BTreeSet<String>, String> {
    let bytes = tool(
        root,
        authority,
        ar,
        &["t".into(), archive.display().to_string()],
    )?;
    let names = std::str::from_utf8(&bytes).map_err(|_| "archive member names are not UTF-8")?;
    let mut members = BTreeSet::new();
    for name in names.lines() {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || !members.insert(name.into())
        {
            return Err("ambiguous or nonlocal compiler archive member".into());
        }
    }
    Ok(members)
}

fn archive_members(bytes: &[u8]) -> Result<std::collections::BTreeMap<String, &[u8]>, String> {
    if !bytes.starts_with(b"!<arch>\n") {
        return Err("compiler archive has invalid magic".into());
    }
    let mut offset = 8usize;
    let mut result = std::collections::BTreeMap::new();
    let mut long_names = &[][..];
    while offset < bytes.len() {
        let header = bytes
            .get(offset..offset.checked_add(60).ok_or("archive offset overflow")?)
            .ok_or("truncated archive header")?;
        if &header[58..] != b"`\n" {
            return Err("invalid archive member header".into());
        }
        let size = std::str::from_utf8(&header[48..58])
            .map_err(|_| "invalid archive member size")?
            .trim()
            .parse::<usize>()
            .map_err(|_| "invalid archive member size")?;
        offset = offset.checked_add(60).ok_or("archive offset overflow")?;
        let mut data = bytes
            .get(offset..offset.checked_add(size).ok_or("archive size overflow")?)
            .ok_or("truncated archive member")?;
        let raw = std::str::from_utf8(&header[..16])
            .map_err(|_| "invalid archive member name")?
            .trim();
        let name = if let Some(length) = raw.strip_prefix("#1/") {
            let length = length
                .parse::<usize>()
                .map_err(|_| "invalid BSD archive name length")?;
            let spelling = data.get(..length).ok_or("truncated BSD archive name")?;
            data = &data[length..];
            std::str::from_utf8(spelling)
                .map_err(|_| "invalid BSD archive name")?
                .trim_end_matches('\0')
                .to_string()
        } else if raw == "//" {
            long_names = data;
            "//".into()
        } else if let Some(index) = raw.strip_prefix('/').filter(|index| !index.is_empty()) {
            let index = index
                .parse::<usize>()
                .map_err(|_| "invalid GNU archive name offset")?;
            let spelling = long_names
                .get(index..)
                .and_then(|rest| rest.split(|byte| *byte == b'\n').next())
                .ok_or("missing GNU archive name")?;
            std::str::from_utf8(spelling)
                .map_err(|_| "invalid GNU archive name")?
                .trim_end_matches('/')
                .to_string()
        } else {
            raw.trim_end_matches('/').to_string()
        };
        if (name == "lib.rmeta" || name.ends_with(".o"))
            && (name.contains('/') || name.contains('\\') || result.insert(name, data).is_some())
        {
            return Err("ambiguous compiler archive member".into());
        }
        offset = offset
            .checked_add(size)
            .and_then(|value| value.checked_add(size % 2))
            .ok_or("archive offset overflow")?;
    }
    if offset != bytes.len() {
        return Err("truncated archive padding".into());
    }
    Ok(result)
}

fn same_compilation(archive: &[u8], metadata: &[u8]) -> Result<(), String> {
    let compiled = archive_members(metadata)?;
    if !compiled.contains_key("lib.rmeta") {
        return Err("provider metadata archive has no Rust compiler metadata".into());
    }
    let production = archive_members(archive)?;
    let mut count = 0;
    for (name, bytes) in compiled.iter().filter(|(name, _)| name.ends_with(".o")) {
        count += 1;
        if production.get(name) != Some(bytes) {
            return Err(format!(
                "provider metadata and production archive disagree on {name}"
            ));
        }
    }
    if count == 0 {
        return Err("provider metadata archive has no compiled objects".into());
    }
    Ok(())
}

pub(crate) fn verify_compiled(
    root: &Path,
    authority: &Authority,
    sources: &SourceRetirements,
    artifacts: &ProductionArtifacts,
    native: &NativeBuildEvidence,
) -> Result<VerifiedRetirements, String> {
    let limit = authority.supervision.executable_member_limit_bytes;
    let archive_bytes = bounded_io::read_regular_nofollow(&artifacts.rust_archive, limit)?;
    if sources.entries.is_empty() {
        return Ok(VerifiedRetirements {
            contracts: Vec::new(),
            rust_archive_sha256: hex_sha256(&archive_bytes),
        });
    }
    if crate::git_text(root, &["rev-parse", "HEAD"], "retirement head")? != sources.head_sha
        || sources.base_sha.is_empty()
        || crate::tracked_worktree_dirty(root)?
    {
        return Err("retirement source binding changed before compiler verification".into());
    }
    // The sibling rlib is the metadata output of the same rustc invocation.
    // Equality of every object member binds its type information to the actual
    // static archive, rather than trusting a filename or a head JSON receipt.
    let metadata = artifacts.rust_archive.with_extension("rlib");
    let metadata_bytes = bounded_io::read_regular_nofollow(&metadata, limit)?;
    same_compilation(&archive_bytes, &metadata_bytes)?;
    let basename = artifacts
        .rust_archive
        .file_stem()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("lib"))
        .ok_or("unsupported Rust production archive name")?;
    let deps = artifacts
        .rust_archive
        .parent()
        .ok_or("archive has no parent")?;
    let depfile = deps.join(format!("{basename}.d"));
    let depbytes = bounded_io::read_regular_nofollow(
        &depfile,
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let deptext =
        std::str::from_utf8(&depbytes).map_err(|_| "Rust compiler dep-info is not UTF-8")?;
    let mut arguments = native.compile_profile.ordered_defines.clone();
    for include in &native.compile_profile.ordered_include_roots {
        arguments.extend(["-I".into(), include.clone()]);
    }
    arguments.extend(native.compile_profile.ordered_compile_flags.iter().cloned());
    arguments.extend(["-target".into(), native.toolchain.target.clone()]);
    if cfg!(target_os = "macos") {
        let sdk = tool(
            root,
            authority,
            "/usr/bin/xcrun",
            &["--show-sdk-path".into()],
        )?;
        let sdk = std::str::from_utf8(&sdk)
            .map_err(|_| "compiler SDK path is not UTF-8")?
            .trim();
        if sdk.lines().count() != 1 || !Path::new(sdk).is_absolute() {
            return Err("compiler SDK query must return one absolute path".into());
        }
        arguments.extend(["-isysroot".into(), sdk.into()]);
    }
    let directory = tempfile::Builder::new()
        .prefix("retirement-probe-")
        .tempdir_in(root.join("rust/target"))
        .map_err(|error| error.to_string())?;
    let mut probe = String::from("use uqm_rust::c_bindings::global_state as c;\n");
    let mut contracts = Vec::new();
    let mut c_probe = String::new();
    for entry in &sources.entries {
        let source = &entry.contract.active_provider.path;
        let relative = source
            .strip_prefix("rust/")
            .ok_or("Rust provider is outside crate")?;
        if !deptext.lines().any(|line| line == format!("{relative}:")) {
            return Err(format!(
                "actual compiler dep-info does not include {source}"
            ));
        }
        let rust_bytes = bounded_io::read_regular_nofollow(
            &root.join(source),
            authority.actions.evidence_snapshot_member_limit_bytes,
        )?;
        let rust = std::str::from_utf8(&rust_bytes).map_err(|_| "Rust provider is not UTF-8")?;
        let c = retirement::c_function(
            root,
            &entry.source,
            &entry.bytes,
            &entry.contract.symbol,
            &arguments,
        )?;
        let function = retirement::rust_export(rust, &entry.contract.symbol, &c)?;
        c_probe.push_str(&format!("#include \"{}\"\n_Static_assert(__builtin_types_compatible_p(__typeof__(&{}), {}), \"original ABI\");\n{}\n", c.header.display(), entry.contract.symbol, c.c_pointer_type, c.c_checks.iter().cloned().collect::<Vec<_>>().join("\n")));

        let module = source
            .strip_prefix("rust/src/")
            .and_then(|path| path.strip_suffix(".rs"))
            .ok_or("unsupported provider module")?
            .replace('/', "::");
        let signature = format!(
            "unsafe extern \"C\" fn({}){}",
            c.arguments.join(", "),
            if c.result == "()" {
                String::new()
            } else {
                format!(" -> {}", c.result)
            }
        );
        probe.push_str(&format!("const _: {signature} = uqm_rust::{module}::{function};\nconst _: {signature} = c::{};\nconst _: () = {{ {} }};\n", entry.contract.symbol, c.machine_checks.into_iter().collect::<Vec<_>>().join("\n")));
        contracts.push(entry.contract.clone());
    }
    let c_path = directory.path().join("probe.c");
    fs::write(&c_path, &c_probe).map_err(|error| error.to_string())?;
    let mut cc_args = native.compile_profile.compiler_argv(
        &c_path,
        &directory.path().join("probe.o"),
        &directory.path().join("probe.d"),
    );
    cc_args.remove(0);
    tool(root, authority, &native.toolchain.cc.executable, &cc_args)?;
    let path = directory.path().join("probe.rs");
    fs::write(&path, &probe).map_err(|error| error.to_string())?;
    let args = vec![
        "--edition=2021".into(),
        "--crate-type=lib".into(),
        "--emit=metadata".into(),
        "--target".into(),
        native.toolchain.target.clone(),
        "--extern".into(),
        format!("uqm_rust={}", metadata.display()),
        "-L".into(),
        format!("dependency={}", deps.display()),
        path.display().to_string(),
        "-o".into(),
        directory.path().join("probe.rmeta").display().to_string(),
    ];
    tool(root, authority, &native.toolchain.rustc.executable, &args)?;
    if metadata_bytes != bounded_io::read_regular_nofollow(&metadata, limit)?
        || archive_bytes != bounded_io::read_regular_nofollow(&artifacts.rust_archive, limit)?
    {
        return Err("compiled provider changed during ABI verification".into());
    }
    println!(
        "retirement compiler evidence: {}",
        serde_json::json!({
            "version": 1, "base_sha": sources.base_sha, "head_sha": sources.head_sha,
            "rust_archive_sha256": hex_sha256(&archive_bytes), "rust_metadata_sha256": hex_sha256(&metadata_bytes),
            "rust_depinfo_sha256": hex_sha256(&depbytes), "compiler_probe_sha256": hex_sha256(probe.as_bytes()),
            "compiler_probe": probe, "c_compiler_probe": c_probe, "c_probe_argv": cc_args, "toolchain": native.toolchain, "compile_profile": native.compile_profile,
            "sources": sources.entries.iter().map(|entry| serde_json::json!({"symbol": entry.contract.symbol,
                "base_source": entry.source, "base_source_sha256": hex_sha256(&entry.bytes), "rust_provider": entry.contract.active_provider.path})).collect::<Vec<_>>()
        })
    );
    println!(
        "compiled retirement ABI probes passed: base={} head={} contracts={} archive={}",
        sources.base_sha,
        sources.head_sha,
        contracts.len(),
        hex_sha256(&archive_bytes)
    );
    Ok(VerifiedRetirements {
        contracts,
        rust_archive_sha256: hex_sha256(&archive_bytes),
    })
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    version: u32,
    authority_sha256: String,
    base_sha: String,
    head_sha: String,
}

pub(super) fn witness_digest(
    base_sha: &str,
    head_sha: &str,
    authority_sha256: &str,
) -> Result<String, String> {
    let witness = Witness {
        version: 1,
        base_sha: base_sha.into(),
        head_sha: head_sha.into(),
        authority_sha256: authority_sha256.into(),
    };
    Ok(hex_sha256(
        &serde_json::to_vec(&witness).map_err(|error| error.to_string())?,
    ))
}

pub(crate) fn stage_witness(
    directory: &Path,
    root: &Path,
    sources: &SourceRetirements,
) -> Result<Vec<String>, String> {
    if sources.entries.is_empty() {
        return Ok(Vec::new());
    }
    let witness = Witness {
        version: 1,
        authority_sha256: hex_sha256(&super::authority::load_policy_bytes(root)?),
        base_sha: sources.base_sha.clone(),
        head_sha: sources.head_sha.clone(),
    };
    let bytes = serde_json::to_vec(&witness).map_err(|error| error.to_string())?;
    let path = directory.join("retirement-witness.json");
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o440)
        .open(&path)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    Ok(vec![path.display().to_string(), hex_sha256(&bytes)])
}

pub(crate) fn stage_direct_witness(
    parent: &Path,
    root: &Path,
    sources: &SourceRetirements,
    limit: u64,
) -> Result<(tempfile::TempDir, Vec<String>), String> {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::Builder::new()
        .prefix("trusted-controller-")
        .tempdir_in(parent)
        .map_err(|error| error.to_string())?;
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o750))
        .map_err(|error| error.to_string())?;
    let controller = directory.path().join("uqm-base-controller");
    bounded_io::copy_regular_nofollow(
        &std::env::current_exe().map_err(|error| error.to_string())?,
        &controller,
        limit,
    )?;
    fs::set_permissions(&controller, fs::Permissions::from_mode(0o550))
        .map_err(|error| error.to_string())?;
    let arguments = stage_witness(directory.path(), root, sources)?;
    Ok((directory, arguments))
}

fn witness_bytes(
    path: &Path,
    expected_sha256: &str,
    controller: &Path,
    limit: u64,
) -> Result<Vec<u8>, String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let directory = controller
        .parent()
        .ok_or("controller has no staging directory")?;
    if path != directory.join("retirement-witness.json")
        || controller.file_name().and_then(|name| name.to_str()) != Some("uqm-base-controller")
    {
        return Err(
            "retirement witness is not a direct member of the staged controller directory".into(),
        );
    }
    for (entry, mode, is_directory) in [
        (directory, 0o750, true),
        (path, 0o440, false),
        (controller, 0o550, false),
    ] {
        let metadata = fs::symlink_metadata(entry).map_err(|error| error.to_string())?;
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o7777 != mode
            || (is_directory && !metadata.is_dir())
            || (!is_directory && !metadata.is_file())
        {
            return Err(
                "retirement witness or staging directory has invalid ownership, mode or file kind"
                    .into(),
            );
        }
    }
    let bytes = bounded_io::read_regular_nofollow(path, limit)?;
    if hex_sha256(&bytes) != expected_sha256 {
        return Err("retirement witness hash differs from the controller argument".into());
    }
    Ok(bytes)
}

pub(crate) fn read_witness(root: &Path, arguments: &[String]) -> Result<SourceRetirements, String> {
    let [path, expected_sha256] = arguments else {
        return Err("retirement witness requires its exact path and SHA-256".into());
    };
    let authority = super::load_authority(root)?;
    let controller = std::env::current_exe().map_err(|error| error.to_string())?;
    let bytes = witness_bytes(
        Path::new(path),
        expected_sha256,
        &controller,
        authority.actions.evidence_snapshot_member_limit_bytes,
    )?;
    let witness: Witness = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if witness.version != 1
        || witness.authority_sha256 != hex_sha256(&super::authority::load_policy_bytes(root)?)
        || witness.head_sha
            != crate::git_text(root, &["rev-parse", "HEAD"], "retirement witness head")?
    {
        return Err("retirement witness version, authority or source binding differs".into());
    }
    let report =
        super::delta::measure_between(root, &authority, &witness.base_sha, &witness.head_sha)?;
    super::delta::source_retirements(root, &authority, &report)
}

#[cfg(test)]
mod tests {
    use super::super::delta::SourceRetirement;
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    fn command(root: &Path, program: &str, arguments: &[String]) {
        let output = Command::new(program)
            .args(arguments)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{program} {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().into()
    }

    #[test]
    fn witness_requires_protected_staging_exact_hash_and_regular_file() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("staged");
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o750)).unwrap();
        let controller = directory.join("uqm-base-controller");
        fs::write(&controller, b"controller").unwrap();
        fs::set_permissions(&controller, fs::Permissions::from_mode(0o550)).unwrap();
        let path = directory.join("retirement-witness.json");
        fs::write(&path, b"witness").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o440)).unwrap();
        let hash = hex_sha256(b"witness");
        assert_eq!(
            witness_bytes(&path, &hash, &controller, 100).unwrap(),
            b"witness"
        );
        assert!(witness_bytes(&path, &"0".repeat(64), &controller, 100).is_err());
        assert!(witness_bytes(&path, &hash, &controller, 3).is_err());
        assert!(witness_bytes(
            &root.path().join("retirement-witness.json"),
            &hash,
            &controller,
            100
        )
        .is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(witness_bytes(&path, &hash, &controller, 100).is_err());
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&controller, &path).unwrap();
        assert!(witness_bytes(&path, &hash, &controller, 100).is_err());
    }

    #[test]
    fn direct_witness_controller_retains_its_protected_adjacent_path() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("rust/ci")).unwrap();
        fs::write(
            root.path().join("rust/ci/gates.json"),
            include_bytes!("../../../ci/gates.json"),
        )
        .unwrap();
        let sources = SourceRetirements {
            base_sha: "a".repeat(40),
            head_sha: "b".repeat(40),
            entries: vec![SourceRetirement {
                contract: SymbolContract {
                    symbol: "Existing".into(),
                    canonical_owner: "DOMAIN/#1".into(),
                    active_provider: uqm_ownership::ProviderRef {
                        kind: uqm_ownership::ProviderKind::RustSource,
                        path: "rust/src/provider.rs".into(),
                    },
                    excluded_providers: vec![uqm_ownership::ProviderRef {
                        kind: uqm_ownership::ProviderKind::NativeObject,
                        path: "native/retired.c.o".into(),
                    }],
                },
                source: "sc2/src/retired.c".into(),
                bytes: Vec::new(),
            }],
        };
        let (directory, arguments) =
            stage_direct_witness(root.path(), root.path(), &sources, 100_000_000).unwrap();
        let controller = directory.path().join("uqm-base-controller");
        let mut resolved =
            super::super::doctor::resolve_executable(controller.to_str().unwrap(), 100_000_000)
                .unwrap();
        resolved.execute_retained_source();
        assert_eq!(Path::new(resolved.execution_path()), controller);
        resolved.verify_unchanged().unwrap();
        witness_bytes(
            Path::new(&arguments[0]),
            &arguments[1],
            Path::new(resolved.execution_path()),
            1_000_000,
        )
        .unwrap();
    }

    #[test]
    fn actual_compiler_metadata_archive_and_added_symbol_mutations() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let authority = super::super::load_authority(repository).unwrap();
        let toolchain = crate::canonical_toolchain(repository).unwrap();
        for mutation in 0..8 {
            let root = tempfile::tempdir().unwrap();
            fs::create_dir_all(root.path().join("rust/src")).unwrap();
            fs::create_dir_all(root.path().join("rust/target/deps")).unwrap();
            fs::create_dir_all(root.path().join("sc2/src")).unwrap();
            fs::write(
                root.path().join("sc2/src/api.h"),
                "#include <stdlib.h>\ntypedef unsigned char BYTE; void Existing(BYTE);",
            )
            .unwrap();
            let provider = "use crate::c_bindings::global_state as c;\nconst _: unsafe extern \"C\" fn(c::BYTE) = c::Existing;\nconst _: unsafe extern \"C\" fn(c::BYTE) = replacement;\n#[export_name = \"Existing\"]\npub unsafe extern \"C\" fn replacement(value: c::BYTE) { let _ = value; }\n";
            fs::write(root.path().join("rust/src/replacement.rs"), provider).unwrap();
            let installed = Manifest::from_file(
                &repository.join("rust/ownership/native-provider-manifest.json"),
            )
            .unwrap();
            let originals = installed
                .symbol_contracts
                .iter()
                .map(|contract| {
                    format!(
                        "#[no_mangle] pub extern \"C\" fn {}() {{}}\n",
                        contract.symbol
                    )
                })
                .collect::<String>();
            fs::write(root.path().join("rust/src/lib.rs"), format!("pub mod c_bindings {{ pub mod global_state {{ pub type BYTE = {}; extern \"C\" {{ pub fn Existing(value: BYTE); }} }} }}\npub mod replacement;\n{originals}", if mutation == 1 { "u16" } else { "u8" })).unwrap();
            fs::write(root.path().join(".gitignore"), "rust/target\n").unwrap();
            git(root.path(), &["init", "-q"]);
            git(
                root.path(),
                &[
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "add",
                    ".",
                ],
            );
            git(
                root.path(),
                &[
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "commit",
                    "-qm",
                    "fixture",
                ],
            );
            let head = git(root.path(), &["rev-parse", "HEAD"]);
            let contract = SymbolContract {
                symbol: "Existing".into(),
                canonical_owner: "DOMAIN/#123".into(),
                active_provider: uqm_ownership::ProviderRef {
                    kind: uqm_ownership::ProviderKind::RustSource,
                    path: "rust/src/replacement.rs".into(),
                },
                excluded_providers: vec![uqm_ownership::ProviderRef {
                    kind: uqm_ownership::ProviderKind::NativeObject,
                    path: "native/retired.c.o".into(),
                }],
            };
            let sources = SourceRetirements {
                base_sha: head.clone(),
                head_sha: head,
                entries: vec![SourceRetirement {
                    contract: contract.clone(),
                    source: "sc2/src/retired.c".into(),
                    bytes: b"#include \"api.h\"\nvoid Existing(BYTE value) { (void)value; }"
                        .to_vec(),
                }],
            };
            if [6, 7].contains(&mutation) {
                fs::write(
                    root.path().join("rust/src/replacement.rs"),
                    provider.replace(
                        "#[export_name = \"Existing\"]",
                        "#[export_name = \"MissingExisting\"]",
                    ),
                )
                .unwrap();
            }
            command(
                &root.path().join("rust"),
                &toolchain.rustc.executable,
                &[
                    "--edition=2021".into(),
                    "--crate-name=uqm_rust".into(),
                    "--crate-type=rlib,staticlib".into(),
                    "--emit=link,dep-info".into(),
                    "-Cextra-filename=-fixture".into(),
                    "--out-dir=target/deps".into(),
                    "src/lib.rs".into(),
                ],
            );
            let archive = root.path().join("rust/target/deps/libuqm_rust-fixture.a");
            let empty = root.path().join("rust/target/deps/libuqm_c.a");
            fs::write(
                root.path().join("rust/target/anchor.c"),
                "void FixtureAnchor(void) {}",
            )
            .unwrap();
            command(
                root.path(),
                &toolchain.cc.executable,
                &[
                    "-c".into(),
                    "rust/target/anchor.c".into(),
                    "-o".into(),
                    "rust/target/anchor.o".into(),
                ],
            );
            command(
                root.path(),
                &toolchain.ar.executable,
                &[
                    "crs".into(),
                    empty.display().to_string(),
                    "rust/target/anchor.o".into(),
                ],
            );
            if [2, 3, 7].contains(&mutation) {
                fs::write(
                    root.path().join("rust/target/duplicate.c"),
                    "void Existing(unsigned char value) { (void)value; }",
                )
                .unwrap();
                let object = root.path().join("rust/target/duplicate.o");
                command(
                    root.path(),
                    &toolchain.cc.executable,
                    &[
                        "-c".into(),
                        "rust/target/duplicate.c".into(),
                        "-o".into(),
                        object.display().to_string(),
                    ],
                );
                command(
                    root.path(),
                    &toolchain.ar.executable,
                    &[
                        "r".into(),
                        (if mutation == 2 { &archive } else { &empty })
                            .display()
                            .to_string(),
                        object.display().to_string(),
                    ],
                );
            }
            if mutation == 4 {
                let metadata = archive.with_extension("rlib");
                fs::remove_file(metadata).unwrap();
            }
            if mutation == 5 {
                let objects = members(
                    root.path(),
                    &authority,
                    &toolchain.ar.executable,
                    &archive.with_extension("rlib"),
                )
                .unwrap();
                let member = objects
                    .iter()
                    .find(|name| name.starts_with("uqm_rust") && name.ends_with(".o"))
                    .unwrap();
                command(
                    root.path(),
                    &toolchain.ar.executable,
                    &["d".into(), archive.display().to_string(), member.clone()],
                );
            }
            let native = NativeBuildEvidence {
                schema: uqm_ownership::BUILD_EVIDENCE_SCHEMA.into(),
                source_date_epoch: 0,
                build_date: "fixture".into(),
                target: toolchain.target.clone(),
                active_features: Vec::new(),
                toolchain: toolchain.clone(),
                packages: Vec::new(),
                compile_profile: uqm_ownership::NativeCompileProfile {
                    target: toolchain.target.clone(),
                    compiler: toolchain.cc.executable.clone(),
                    ordered_defines: Vec::new(),
                    ordered_include_roots: Vec::new(),
                    ordered_compile_flags: Vec::new(),
                    dependency_flags: Vec::new(),
                    command_template: Vec::new(),
                },
                build_environment: Default::default(),
            };
            let artifacts = ProductionArtifacts {
                rust_archive: archive.clone(),
                c_archive: empty,
                executable: archive,
            };
            if [6, 7].contains(&mutation) {
                let mut manifest = Manifest::from_file(
                    &repository.join("rust/ownership/native-provider-manifest.json"),
                )
                .unwrap();
                manifest.symbol_contracts.push(contract.clone());
                let validator = Validator::new(manifest)
                    .with_verified_retirements(vec![contract.clone()])
                    .unwrap();
                let measured = validator.validate_symbol_artifacts(
                    &artifacts,
                    &uqm_ownership::ProductionToolPaths {
                        ar: toolchain.ar.executable.clone().into(),
                        nm: toolchain.nm.executable.clone().into(),
                    },
                );
                let error = measured.unwrap_err().to_string();
                assert!(
                    error.contains("Existing"),
                    "archive mutation {mutation}: {error}"
                );
                continue;
            }
            let result = verify_compiled(root.path(), &authority, &sources, &artifacts, &native);
            if [1, 4, 5].contains(&mutation) {
                assert!(result.is_err(), "compiler mutation {mutation}");
                continue;
            }
            let verified = result.unwrap_or_else(|error| panic!("mutation {mutation}: {error}"));
            let mut manifest = Manifest::from_file(
                &repository.join("rust/ownership/native-provider-manifest.json"),
            )
            .unwrap();
            manifest.symbol_contracts.push(contract);
            let validator = verified
                .validator(
                    manifest,
                    &artifacts,
                    authority.supervision.executable_member_limit_bytes,
                )
                .unwrap();
            let measured = validator.validate_symbol_artifacts(
                &artifacts,
                &uqm_ownership::ProductionToolPaths {
                    ar: toolchain.ar.executable.clone().into(),
                    nm: toolchain.nm.executable.clone().into(),
                },
            );
            if mutation == 0 {
                measured.unwrap();
            } else {
                assert!(measured.is_err(), "archive mutation {mutation}");
            }
        }
    }
}
