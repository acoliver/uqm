use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::process::Command;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    schema: u32,
    files: BTreeMap<String, Identity>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    bytes: u64,
    sha256: String,
}

pub(super) fn create(directory: &Path) -> Result<()> {
    let inventory = measure(directory)?;
    write_json(&directory.join(INVENTORY), &inventory)
}

pub(super) fn verify(
    root: &Path,
    directory: &Path,
    manifest: &manifest::Manifest,
    entry: Option<&str>,
) -> Result<()> {
    let stored: Inventory = json(&directory.join(INVENTORY))?;
    let measured = measure(directory)?;
    if stored != measured {
        return Err(format!(
            "SHA256 inventory mismatch: {}",
            directory.display()
        ));
    }
    let expected = manifest.expected_files(root, entry)?;
    let actual: BTreeSet<_> = stored.files.keys().cloned().collect();
    if actual != expected {
        return Err(format!(
            "golden registry/manifest mismatch; missing={:?}, unregistered={:?}",
            expected.difference(&actual).collect::<Vec<_>>(),
            actual.difference(&expected).collect::<Vec<_>>()
        ));
    }
    if entry.is_none() {
        verify_counts(root, directory, manifest)?;
    }
    Ok(())
}

fn verify_counts(root: &Path, directory: &Path, manifest: &manifest::Manifest) -> Result<()> {
    let bytes = read(&directory.join("parity/counts.toml"))?;
    let rows = counts::decode(std::str::from_utf8(&bytes).map_err(|error| error.to_string())?)?;
    let expected = script_names(root)?;
    let actual: BTreeSet<_> = rows.keys().cloned().collect();
    if expected != actual {
        return Err(format!(
            "counts inventory differs from every scripts JSON; missing={:?}, extra={:?}",
            expected.difference(&actual).collect::<Vec<_>>(),
            actual.difference(&expected).collect::<Vec<_>>()
        ));
    }
    for (name, row) in rows {
        let mut classified = row.clone();
        counts::classify(&mut classified, manifest.expectations.get(&name))?;
        if classified.classification != row.classification || classified.issue != row.issue {
            return Err(format!(
                "counts {name}: classification differs from manifest"
            ));
        }
    }
    Ok(())
}

fn measure(directory: &Path) -> Result<Inventory> {
    let mut files = BTreeMap::new();
    collect(directory, directory, &mut files)?;
    Ok(Inventory { schema: 1, files })
}

fn collect(root: &Path, directory: &Path, files: &mut BTreeMap<String, Identity>) -> Result<()> {
    for entry in
        fs::read_dir(directory).map_err(|error| format!("read {}: {error}", directory.display()))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            collect(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_str()
                .ok_or("invalid inventory path")?
                .replace('\\', "/");
            if relative != INVENTORY {
                files.insert(relative, identity(&path)?);
            }
        } else {
            return Err(format!(
                "inventory rejects nonregular file {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn identity(path: &Path) -> Result<Identity> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut bytes = 0;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    Ok(Identity {
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

pub(super) fn protect(
    root: &Path,
    manifest: &manifest::Manifest,
    options: &options::Options,
) -> Result<()> {
    if let Some(base) = &options.base {
        protect_base(root, base, options.allow_golden_update)?;
    }
    verify(root, &root.join(GOLDEN), manifest, None)
}

fn git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output.stdout)
}

pub(super) fn protect_base(root: &Path, base: &str, allow: bool) -> Result<()> {
    let reference = format!("{base}^{{commit}}");
    let commit = git(root, &["rev-parse", "--verify", &reference])?;
    let commit = std::str::from_utf8(&commit)
        .map_err(|error| error.to_string())?
        .trim();
    let bytes = git(root, &["ls-tree", "-r", "-z", commit, "--", GOLDEN])?;
    for item in bytes
        .split(|byte| *byte == 0)
        .filter(|item| !item.is_empty())
    {
        let text = std::str::from_utf8(item).map_err(|error| error.to_string())?;
        let (metadata, path) = text.split_once('\t').ok_or("invalid git tree entry")?;
        if !metadata.starts_with("100644 blob ") && !metadata.starts_with("100755 blob ") {
            return Err(format!("nonregular base golden: {path}"));
        }
        let actual = fs::symlink_metadata(root.join(path));
        let original = git(root, &["show", &format!("{commit}:{path}")])?;
        let unchanged = actual.is_ok_and(|metadata| metadata.is_file())
            && read(&root.join(path)).is_ok_and(|bytes| bytes == original);
        if !unchanged && !allow {
            return Err(format!("existing golden modified/deleted: {path}; requires explicit --allow-golden-update (CI requires owner-applied parity-golden-update)"));
        }
    }
    Ok(())
}
