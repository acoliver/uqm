//! Descriptor-bound native suite inputs and pre-write evidence accounting.

use super::native_window::{NativeInventoryLimits, NativeRetainedInput};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Aggregate allowance shared by all writers in one controller.
#[derive(Clone)]
pub struct ArtifactBudget(Arc<Mutex<BudgetState>>);

struct BudgetState {
    limits: NativeInventoryLimits,
    bytes: u64,
    members: u64,
    paths: u64,
    claims: BTreeSet<String>,
    directories: BTreeSet<String>,
}

/// A claim stays consumed after an I/O failure: partial writes still use storage.
pub struct WriteReservation {
    bytes: u64,
}

impl ArtifactBudget {
    /// Reserve diagnostic space separately before admitting ordinary writes.
    pub fn new(limits: NativeInventoryLimits) -> Self {
        Self(Arc::new(Mutex::new(BudgetState {
            limits,
            bytes: 0,
            members: 0,
            paths: 0,
            claims: BTreeSet::new(),
            directories: BTreeSet::new(),
        })))
    }

    /// Capacity not already claimed, including diagnostic reservations.
    pub fn remaining(&self) -> Result<NativeInventoryLimits, String> {
        let state = self.0.lock().map_err(|_| "artifact budget lock poisoned")?;
        Ok(NativeInventoryLimits {
            aggregate_bytes: state.limits.aggregate_bytes - state.bytes,
            member_count: state.limits.member_count
                - u32::try_from(state.members).map_err(|_| "artifact count overflow")?,
            aggregate_path_bytes: state.limits.aggregate_path_bytes - state.paths,
            ..state.limits
        })
    }

    /// Claim a unique output and its full bounded size before opening it.
    pub fn reserve(&self, path: &str, bytes: u64) -> Result<WriteReservation, String> {
        self.claim(path, bytes, false)
    }

    /// Charge directories, including empty ones, before creating them.
    /// Existing directory claims are shared by their contained writers.
    pub fn reserve_directory(&self, path: &str) -> Result<(), String> {
        self.claim(path, 0, true).map(|_| ())
    }

    fn claim(&self, path: &str, bytes: u64, directory: bool) -> Result<WriteReservation, String> {
        validate_relative(path)?;
        let mut state = self.0.lock().map_err(|_| "artifact budget lock poisoned")?;
        if directory && state.directories.contains(path) {
            return Ok(WriteReservation { bytes: 0 });
        }
        let total = state
            .bytes
            .checked_add(bytes)
            .ok_or("artifact byte overflow")?;
        let mut directories = Vec::new();
        let mut parent = Path::new(path).parent();
        while let Some(value) = parent.filter(|value| !value.as_os_str().is_empty()) {
            let name = value.to_str().ok_or("non-UTF8 artifact parent")?;
            if state.claims.contains(name) {
                return Err("artifact parent was claimed as a file".into());
            }
            if !state.directories.contains(name) {
                directories.push(name.to_string());
            }
            parent = value.parent();
        }
        let members = state
            .members
            .checked_add(1 + directories.len() as u64)
            .ok_or("artifact count overflow")?;
        let added_paths = directories
            .iter()
            .try_fold(path.len() as u64, |sum, name| {
                sum.checked_add(name.len() as u64)
            })
            .ok_or("artifact path overflow")?;
        let paths = state
            .paths
            .checked_add(added_paths)
            .ok_or("artifact path overflow")?;
        if state.claims.contains(path) || state.directories.contains(path) {
            return Err(format!("artifact already claimed: {path}"));
        }
        if bytes > state.limits.member_bytes
            || total > state.limits.aggregate_bytes
            || members > u64::from(state.limits.member_count)
            || path.len() as u64 > u64::from(state.limits.path_bytes)
            || paths > state.limits.aggregate_path_bytes
        {
            return Err(format!("artifact reservation exceeds suite budget: {path}"));
        }
        state.bytes = total;
        state.members = members;
        state.paths = paths;
        if directory {
            state.directories.insert(path.to_string());
        } else {
            state.claims.insert(path.to_string());
        }
        state.directories.extend(directories);
        Ok(WriteReservation { bytes })
    }
}

impl WriteReservation {
    /// A reserved writer cannot enlarge the reservation after publication starts.
    pub fn write(self, mut file: File, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() as u64 != self.bytes {
            return Err("artifact write differs from reservation".into());
        }
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())
    }
}

/// Suite descriptor identity and the exact logical members used by one scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedInputs {
    pub schema: String,
    pub descriptor: NativeRetainedInput,
    pub members: BTreeMap<String, NativeRetainedInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SharedDescriptor {
    schema: String,
    members: BTreeMap<String, NativeRetainedInput>,
}

/// Holds the directory descriptor throughout publication and resolution.
/// Kernel isolation of the candidate is required in addition to this binding.
pub struct SharedSnapshot {
    root: PathBuf,
    directory: File,
    budget: ArtifactBudget,
    members: BTreeMap<String, NativeRetainedInput>,
    objects: BTreeMap<String, NativeRetainedInput>,
    sealed: Option<NativeRetainedInput>,
}

impl SharedSnapshot {
    /// The root must be newly claimed by the controller, never supplied by a candidate.
    pub fn create(root: &Path, budget: ArtifactBudget) -> Result<Self, String> {
        budget.reserve_directory("shared/objects")?;
        fs::create_dir(root).map_err(|error| format!("claim shared snapshot: {error}"))?;
        let directory = open_directory(root).map_err(|error| error.to_string())?;
        fs::create_dir(root.join("objects")).map_err(|error| error.to_string())?;
        Ok(Self {
            root: root.to_path_buf(),
            directory,
            budget,
            members: BTreeMap::new(),
            objects: BTreeMap::new(),
            sealed: None,
        })
    }

    /// Snapshot exact bytes once. Reusing a logical name is a collision, even for equal bytes.
    pub fn insert(&mut self, logical: &str, bytes: &[u8]) -> Result<(), String> {
        validate_relative(logical)?;
        if self.sealed.is_some() || self.members.contains_key(logical) {
            return Err(format!(
                "shared snapshot is sealed or member already claimed: {logical}"
            ));
        }
        let digest = digest(bytes);
        let relative = format!("objects/{digest}");
        let object = NativeRetainedInput {
            relative_path: relative.clone(),
            byte_length: bytes.len() as u64,
            sha256: digest.clone(),
        };
        if let Some(existing) = self.objects.get(&digest) {
            if existing != &object || self.read_object(existing)? != bytes {
                return Err("shared object changed before reuse".into());
            }
        } else {
            let reservation = self
                .budget
                .reserve(&format!("shared/{relative}"), bytes.len() as u64)?;
            let file =
                create_relative(&self.directory, &relative).map_err(|error| error.to_string())?;
            reservation.write(file, bytes)?;
            self.objects.insert(digest, object.clone());
        }
        self.members.insert(logical.to_string(), object);
        Ok(())
    }

    /// Publish the immutable descriptor only after every object is durable.
    pub fn seal(&mut self) -> Result<NativeRetainedInput, String> {
        if self.sealed.is_some() || self.members.is_empty() {
            return Err("shared snapshot must be nonempty and sealed exactly once".into());
        }
        let bytes = serde_json::to_vec(&SharedDescriptor {
            schema: "uqm-native-shared-descriptor-v1".into(),
            members: self.members.clone(),
        })
        .map_err(|error| error.to_string())?;
        let reservation = self
            .budget
            .reserve("shared/descriptor.json", bytes.len() as u64)?;
        reservation.write(
            create_relative(&self.directory, "descriptor.json")
                .map_err(|error| error.to_string())?,
            &bytes,
        )?;
        let descriptor = identity("descriptor.json", &bytes);
        self.sealed = Some(descriptor.clone());
        Ok(descriptor)
    }

    /// Bind the selected members to the sealed suite descriptor.
    pub fn references(&self, names: &[String]) -> Result<SharedInputs, String> {
        self.verify()?;
        let descriptor = self.sealed.clone().ok_or("shared snapshot is not sealed")?;
        let mut members = BTreeMap::new();
        for name in names {
            let member = self.members.get(name).ok_or("unknown shared member")?;
            if members.insert(name.clone(), member.clone()).is_some() {
                return Err("duplicate shared member reference".into());
            }
        }
        Ok(SharedInputs {
            schema: "uqm-native-shared-inputs-v1".into(),
            descriptor,
            members,
        })
    }

    /// Revalidate through the held descriptor, including visible root identity.
    pub fn verify(&self) -> Result<(), String> {
        same_directory(&self.directory, &self.root)?;
        for member in self.objects.values() {
            self.read_object(member)?;
        }
        if let Some(descriptor) = &self.sealed {
            self.read_object(descriptor)?;
        }
        Ok(())
    }

    fn read_object(&self, object: &NativeRetainedInput) -> Result<Vec<u8>, String> {
        read_identity(&self.directory, object)
    }
}

impl SharedInputs {
    /// Open references only in the suite's scenarios/NNNN layout. No supplied traversal path is used.
    pub fn open(root: &Path, limit: u64) -> Result<Option<Self>, String> {
        let directory = open_directory(root).map_err(|error| error.to_string())?;
        let bytes = match read_relative(&directory, "shared-inputs.json", limit) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let inputs: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        inputs.validate(root)?;
        Ok(Some(inputs))
    }

    fn shared_directory(root: &Path) -> Result<(PathBuf, File), String> {
        let current;
        let root = if root == Path::new(".") {
            current = std::env::current_dir().map_err(|error| error.to_string())?;
            current.as_path()
        } else {
            root
        };
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("missing scenario name")?;
        let parent = root.parent().ok_or("missing scenarios parent")?;
        if name.len() != 4
            || !name.bytes().all(|byte| byte.is_ascii_digit())
            || parent.file_name().and_then(|name| name.to_str()) != Some("scenarios")
        {
            return Err("shared inputs require suite scenarios/NNNN layout".into());
        }
        // Bind each traversed component without following symbolic links.
        let suite = parent.parent().ok_or("missing suite root")?;
        let directory = open_directory(suite).map_err(|error| error.to_string())?;
        let scenarios =
            open_relative(&directory, "scenarios", true).map_err(|error| error.to_string())?;
        let scenario = open_relative(&scenarios, name, true).map_err(|error| error.to_string())?;
        same_directory(&scenario, root)?;
        let shared =
            open_relative(&directory, "shared", true).map_err(|error| error.to_string())?;
        Ok((suite.join("shared"), shared))
    }

    /// Reject descriptor/member substitution before any bytes reach a consumer.
    pub fn validate(&self, root: &Path) -> Result<(), String> {
        if self.schema != "uqm-native-shared-inputs-v1"
            || self.members.is_empty()
            || self.descriptor.relative_path != "descriptor.json"
            || self.descriptor.byte_length > 1024 * 1024
        {
            return Err("invalid shared input reference".into());
        }
        let (_, directory) = Self::shared_directory(root)?;
        let bytes = read_identity(&directory, &self.descriptor)?;
        let descriptor: SharedDescriptor =
            serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if descriptor.schema != "uqm-native-shared-descriptor-v1" {
            return Err("invalid shared descriptor schema".into());
        }
        for (name, member) in &self.members {
            validate_relative(name)?;
            if !name.starts_with("inputs/")
                || descriptor.members.get(name) != Some(member)
                || member.sha256.len() != 64
                || !member
                    .sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || member.relative_path != format!("objects/{}", member.sha256)
            {
                return Err("shared member does not belong to bound descriptor".into());
            }
        }
        Ok(())
    }

    /// Read the descriptor-bound object within the consumer's independent bound.
    pub fn read(&self, root: &Path, logical: &str, limit: u64) -> Result<Vec<u8>, String> {
        let (_, directory) = Self::shared_directory(root)?;
        let member = self.members.get(logical).ok_or("unknown shared input")?;
        if member.byte_length > limit {
            return Err("shared member exceeds consumer bound".into());
        }
        read_identity(&directory, member)
    }

    /// Obtain a launch path after descriptor validation. Candidate isolation keeps it immutable.
    pub fn path(&self, root: &Path, logical: &str, limit: u64) -> Result<PathBuf, String> {
        self.validate(root)?;
        self.read(root, logical, limit)?;
        let (shared, _) = Self::shared_directory(root)?;
        Ok(shared.join(
            &self
                .members
                .get(logical)
                .ok_or("unknown shared input")?
                .relative_path,
        ))
    }

    /// Create a no-clobber runtime alias between bound directory descriptors.
    pub fn link_runtime(&self, root: &Path, logical: &str, limit: u64) -> Result<(), String> {
        use std::os::fd::AsRawFd;
        self.validate(root)?;
        self.read(root, logical, limit)?;
        let (_, shared) = Self::shared_directory(root)?;
        let scenario = open_directory(root).map_err(|error| error.to_string())?;
        let member = self.members.get(logical).ok_or("unknown shared input")?;
        let (source_parent, source_name) =
            relative_parent(&shared, &member.relative_path).map_err(|error| error.to_string())?;
        let (target_parent, target_name) =
            relative_parent(&scenario, logical).map_err(|error| error.to_string())?;
        let source = std::ffi::CString::new(source_name).map_err(|error| error.to_string())?;
        let target = std::ffi::CString::new(target_name).map_err(|error| error.to_string())?;
        // SAFETY: both directories and C strings remain owned across linkat; flags do not follow symlinks.
        if unsafe {
            libc::linkat(
                source_parent.as_raw_fd(),
                source.as_ptr(),
                target_parent.as_raw_fd(),
                target.as_ptr(),
                0,
            )
        } != 0
        {
            return Err(format!(
                "claim runtime alias: {}",
                io::Error::last_os_error()
            ));
        }
        if read_identity(
            &scenario,
            &NativeRetainedInput {
                relative_path: logical.into(),
                ..member.clone()
            },
        )? != self.read(root, logical, limit)?
        {
            return Err("runtime alias differs from shared object".into());
        }
        Ok(())
    }

    /// Virtual inventory preserves existing exact-build logical names without copying archives.
    pub fn inventory(&self) -> Vec<NativeRetainedInput> {
        self.members
            .iter()
            .map(|(name, member)| NativeRetainedInput {
                relative_path: name.clone(),
                byte_length: member.byte_length,
                sha256: member.sha256.clone(),
            })
            .collect()
    }
}

fn identity(path: &str, bytes: &[u8]) -> NativeRetainedInput {
    NativeRetainedInput {
        relative_path: path.into(),
        byte_length: bytes.len() as u64,
        sha256: digest(bytes),
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn validate_relative(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("artifact path must be normalized relative UTF-8".into());
    }
    Ok(())
}
fn read_identity(directory: &File, identity: &NativeRetainedInput) -> Result<Vec<u8>, String> {
    let bytes = read_relative(directory, &identity.relative_path, identity.byte_length)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 != identity.byte_length || digest(&bytes) != identity.sha256 {
        return Err(format!(
            "shared object identity changed: {}",
            identity.relative_path
        ));
    }
    Ok(bytes)
}
fn read_relative(directory: &File, relative: &str, limit: u64) -> io::Result<Vec<u8>> {
    let file = open_relative(directory, relative, false)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > limit {
        return Err(io::Error::other("shared input exceeds regular-file bound"));
    }
    let mut bytes = Vec::new();
    file.take(
        limit
            .checked_add(1)
            .ok_or_else(|| io::Error::other("read bound overflow"))?,
    )
    .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != meta.len() {
        return Err(io::Error::other("shared input changed during read"));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn open_directory(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}
#[cfg(unix)]
fn same_directory(directory: &File, path: &Path) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    let bound = directory.metadata().map_err(|error| error.to_string())?;
    let visible = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !visible.is_dir() || bound.dev() != visible.dev() || bound.ino() != visible.ino() {
        return Err("shared directory identity changed".into());
    }
    Ok(())
}
#[cfg(unix)]
fn open_relative(directory: &File, relative: &str, is_directory: bool) -> io::Result<File> {
    validate_relative(relative).map_err(io::Error::other)?;
    let mut parent = directory.try_clone()?;
    let parts: Vec<_> = relative.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        let flags = libc::O_RDONLY
            | libc::O_NONBLOCK
            | if index + 1 < parts.len() || is_directory {
                libc::O_DIRECTORY
            } else {
                0
            };
        parent = open_at(&parent, part, flags)?;
    }
    Ok(parent)
}
#[cfg(unix)]
fn relative_parent<'a>(directory: &File, relative: &'a str) -> io::Result<(File, &'a str)> {
    validate_relative(relative).map_err(io::Error::other)?;
    match relative.rsplit_once('/') {
        Some((parent, name)) => Ok((open_relative(directory, parent, true)?, name)),
        None => Ok((directory.try_clone()?, relative)),
    }
}
#[cfg(unix)]
fn create_relative(directory: &File, relative: &str) -> io::Result<File> {
    let (parent, name) = relative_parent(directory, relative)?;
    open_at(&parent, name, libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL)
}
#[cfg(unix)]
fn open_at(directory: &File, name: &str, flags: i32) -> io::Result<File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let name = std::ffi::CString::new(name).map_err(io::Error::other)?;
    // SAFETY: directory and name remain valid; a successful descriptor is immediately owned.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o500,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits(bytes: u64) -> NativeInventoryLimits {
        NativeInventoryLimits {
            member_count: 100,
            member_bytes: bytes,
            aggregate_bytes: bytes,
            path_bytes: 4096,
            aggregate_path_bytes: 8192,
        }
    }
    #[test]
    fn empty_directories_and_parent_paths_consume_exact_capacity() {
        let budget = ArtifactBudget::new(NativeInventoryLimits {
            member_count: 3,
            member_bytes: 1,
            aggregate_bytes: 1,
            path_bytes: 5,
            aggregate_path_bytes: 9,
        });
        budget.reserve_directory("a/b").unwrap();
        budget.reserve_directory("a").unwrap();
        budget.reserve("a/b/c", 1).unwrap();
        let left = budget.remaining().unwrap();
        assert_eq!(left.member_count, 0);
        assert_eq!(left.aggregate_path_bytes, 0);
        assert_eq!(left.aggregate_bytes, 0);
        assert!(budget.reserve_directory("z").is_err());
        assert!(budget.reserve("a", 0).is_err());
        assert!(budget.reserve_directory("a/b/c").is_err());
        assert!(budget.reserve("a/b/c/d", 0).is_err());
    }

    #[test]
    fn file_directory_claim_conflicts_do_not_consume_other_capacity() {
        let budget = ArtifactBudget::new(limits(8192));
        budget.reserve_directory("directory").unwrap();
        budget.reserve("file", 3).unwrap();
        let before = budget.remaining().unwrap();
        assert!(budget.reserve("directory", 0).is_err());
        assert!(budget.reserve_directory("file").is_err());
        assert!(budget.reserve("file/child", 1).is_err());
        assert_eq!(budget.remaining().unwrap(), before);
        budget.reserve("directory/child", 1).unwrap();
    }

    #[test]
    fn shared_directory_exhaustion_precedes_root_creation() {
        let temp = tempfile::tempdir().unwrap();
        let mut bound = limits(8192);
        bound.member_count = 1;
        let root = temp.path().join("shared");
        assert!(SharedSnapshot::create(&root, ArtifactBudget::new(bound)).is_err());
        assert!(!root.exists());
    }

    #[test]
    fn simultaneous_duplicate_claims_have_one_winner() {
        let budget = ArtifactBudget::new(limits(100));
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let budget = budget.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    budget.reserve("same", 50).is_ok()
                })
            })
            .collect();
        barrier.wait();
        assert_eq!(
            threads
                .into_iter()
                .map(|thread| usize::from(thread.join().unwrap()))
                .sum::<usize>(),
            1
        );
        assert!(budget.reserve("remaining", 50).is_ok());
        assert!(budget.reserve("overflow", 1).is_err());
    }
    #[test]
    fn diagnostic_reservation_survives_exhaustion_and_failed_write() {
        let budget = ArtifactBudget::new(limits(10));
        let diagnostic = budget.reserve("failure", 3).unwrap();
        let failed = budget.reserve("output", 7).unwrap();
        assert!(budget.reserve("overflow", 1).is_err());
        let temp = tempfile::tempdir().unwrap();
        assert!(failed
            .write(File::create(temp.path().join("bad")).unwrap(), b"too long")
            .is_err());
        diagnostic
            .write(File::create(temp.path().join("failure")).unwrap(), b"bad")
            .unwrap();
        assert_eq!(fs::read(temp.path().join("failure")).unwrap(), b"bad");
        assert!(budget.reserve("output", 0).is_err());
    }
    #[test]
    fn shared_reuse_mutation_and_descriptor_substitution_are_refused() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut store =
            SharedSnapshot::create(&root.join("shared"), ArtifactBudget::new(limits(8192)))
                .unwrap();
        assert!(
            SharedSnapshot::create(&root.join("shared"), ArtifactBudget::new(limits(8192)))
                .is_err()
        );
        store.insert("inputs/uqm", b"binary").unwrap();
        store.insert("inputs/alias", b"binary").unwrap();
        assert_eq!(
            fs::read_dir(root.join("shared/objects")).unwrap().count(),
            1
        );
        assert!(store.insert("inputs/uqm", b"changed").is_err());
        store.seal().unwrap();
        assert!(store.insert("inputs/new", b"new").is_err());
        let refs = store.references(&["inputs/uqm".into()]).unwrap();
        fs::create_dir_all(root.join("scenarios/0000")).unwrap();
        let scenario = root.join("scenarios/0000");
        refs.validate(&scenario).unwrap();
        let mut forged = refs.clone();
        forged.members.get_mut("inputs/uqm").unwrap().sha256 = "0".repeat(64);
        assert!(forged.validate(&scenario).is_err());
        let object = refs.path(&scenario, "inputs/uqm", 10_000).unwrap();
        fs::remove_file(&object).unwrap();
        fs::write(&object, b"mutant").unwrap();
        assert!(refs.read(&scenario, "inputs/uqm", 10_000).is_err());
        assert!(store.references(&["inputs/uqm".into()]).is_err());
    }
    #[test]
    fn descriptor_bound_store_rejects_replaced_root_and_symlink_members() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("shared");
        let mut store = SharedSnapshot::create(&root, ArtifactBudget::new(limits(8192))).unwrap();
        store.insert("inputs/uqm", b"binary").unwrap();
        store.seal().unwrap();
        fs::rename(&root, temp.path().join("old")).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(store.verify().is_err());
        let old = temp.path().join("old");
        let object = old.join(format!("objects/{}", digest(b"binary")));
        fs::remove_file(&object).unwrap();
        std::os::unix::fs::symlink(temp.path().join("outside"), object).unwrap();
        assert!(store
            .read_object(store.objects.values().next().unwrap())
            .is_err());
    }
}
