//! Strict native-window acceptance state shared by runtime collection and detached replay.

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

#[cfg(target_os = "macos")]
use super::child_session::{ChildSession, ChildSessionConfig, ChildSessionError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const NATIVE_WINDOW_CONFIG_SCHEMA: &str = "uqm-native-window-config-v1";
pub const NATIVE_WINDOW_STATE_SCHEMA: &str = "uqm-native-window-state-v1";
pub const NATIVE_WINDOW_ACK_SCHEMA: &str = "uqm-native-window-ack-v1";
const NATIVE_CONTROL_FILE_MAX_BYTES: u64 = 65_536;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowConfigFile {
    pub schema: String,
    pub nonce: String,
    pub client_bounds: NativeWindowBounds,
    pub runtime_contract: NativeWindowRuntimeContract,
    pub acceptance_policy: NativeAcceptancePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAcceptancePolicy {
    pub stable_presentation_floor: u64,
    pub playable_presentation_floor: u64,
    pub battle_frame_floor: u64,
}

impl NativeAcceptancePolicy {
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.stable_presentation_floor > 0
            && self.playable_presentation_floor > self.stable_presentation_floor
            && self.battle_frame_floor > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeInventoryLimits {
    pub member_count: u32,
    pub member_bytes: u64,
    pub aggregate_bytes: u64,
    pub path_bytes: u32,
    pub aggregate_path_bytes: u64,
}

impl NativeInventoryLimits {
    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.member_count > 0
            && self.member_bytes > 0
            && self.aggregate_bytes >= self.member_bytes
            && self.path_bytes > 0
            && self.aggregate_path_bytes >= self.path_bytes as u64
    }
}

/// Runtime bounds shared by the game child and its native-window observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowRuntimeContract {
    pub capture_timeout_ms: u64,
    pub capture_kill_grace_ms: u64,
    pub observer_timeout_ms: u64,
    pub observer_kill_grace_ms: u64,
    pub acknowledgement_timeout_ms: u64,
    pub outer_child_timeout_ms: u64,
    pub outer_child_kill_grace_ms: u64,
    pub child_stdout_budget_bytes: u64,
    pub child_stderr_budget_bytes: u64,
    pub observer_response_budget_bytes: u64,
    pub capture_budget_bytes: u64,
    pub content_expansion_budget_bytes: u64,
    pub inventory_limits: NativeInventoryLimits,
    pub expected_client_bounds: NativeWindowBounds,
}

impl NativeWindowRuntimeContract {
    #[must_use]
    pub const fn capture_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.capture_timeout_ms)
    }

    #[must_use]
    pub const fn capture_kill_grace(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.capture_kill_grace_ms)
    }

    #[must_use]
    pub const fn observer_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.observer_timeout_ms)
    }

    #[must_use]
    pub const fn observer_kill_grace(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.observer_kill_grace_ms)
    }

    #[must_use]
    pub const fn acknowledgement_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.acknowledgement_timeout_ms)
    }

    #[must_use]
    pub const fn outer_child_timeout(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.outer_child_timeout_ms)
    }

    #[must_use]
    pub const fn outer_child_kill_grace(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.outer_child_kill_grace_ms)
    }

    #[must_use]
    pub const fn has_valid_deadline_order(self) -> bool {
        let operation_bound = self
            .observer_timeout_ms
            .saturating_add(self.observer_kill_grace_ms);
        self.capture_timeout_ms > 0
            && self.capture_kill_grace_ms > 0
            && self.observer_timeout_ms
                > self
                    .capture_timeout_ms
                    .saturating_add(self.capture_kill_grace_ms)
            && self.acknowledgement_timeout_ms > operation_bound.saturating_add(operation_bound)
            && self.outer_child_timeout_ms > self.acknowledgement_timeout_ms
            && self.outer_child_kill_grace_ms > 0
            && self.child_stdout_budget_bytes > 0
            && self.child_stderr_budget_bytes > 0
            && self.observer_response_budget_bytes > 0
            && self.capture_budget_bytes > 0
            && self.content_expansion_budget_bytes > 0
            && self.inventory_limits.is_valid()
            && self.expected_client_bounds.width > 0
            && self.expected_client_bounds.height > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowSemanticSnapshot {
    pub trace_record_count: u64,
    pub accepted_player_inputs: u64,
    pub verified_battle_frames: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowChildState {
    pub schema: String,
    pub nonce: String,
    pub pid: u32,
    pub sdl_window_id: u32,
    pub committed_presentation: u64,
    pub shown: bool,
    pub requested_client_bounds: NativeWindowBounds,
    pub client_bounds: NativeWindowBounds,
    pub semantic: NativeWindowSemanticSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedNativeWindow {
    pub window_id: u64,
    pub os_bounds: NativeWindowBounds,
    pub visible: bool,
    pub minimized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeWindowObserverError {
    UnsupportedPlatform,
    Os(String),
    OutputLimit { stream: String, limit_bytes: u64 },
    NotFound { pid: u32 },
    Ambiguous { pid: u32, count: usize },
}

impl std::fmt::Display for NativeWindowObserverError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("native-window observation is unsupported on this platform")
            }
            Self::Os(detail) => write!(formatter, "native-window OS observation failed: {detail}"),
            Self::OutputLimit {
                stream,
                limit_bytes,
            } => write!(
                formatter,
                "native-window {stream} output exceeded {limit_bytes} bytes"
            ),
            Self::NotFound { pid } => write!(
                formatter,
                "no native window belongs to exact child PID {pid}"
            ),
            Self::Ambiguous { pid, count } => write!(
                formatter,
                "{count} native windows belong to exact child PID {pid}"
            ),
        }
    }
}

impl std::error::Error for NativeWindowObserverError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowAck {
    pub schema: String,
    pub nonce: String,
    pub committed_presentation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowPublication {
    pub state: NativeWindowChildState,
    pub acknowledgement: NativeWindowAck,
}

#[derive(Debug)]
struct BoundOutputDirectory {
    #[cfg(unix)]
    directory: fs::File,
    #[cfg(not(unix))]
    path: PathBuf,
}

impl BoundOutputDirectory {
    fn open(path: &Path) -> Result<Self, String> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            let directory = fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
                .open(path)
                .map_err(|error| format!("open native-window output directory: {error}"))?;
            if !directory
                .metadata()
                .map_err(|error| format!("inspect native-window output directory: {error}"))?
                .is_dir()
            {
                return Err("native-window output root is not a directory".to_string());
            }
            Ok(Self { directory })
        }
        #[cfg(not(unix))]
        {
            let metadata = fs::symlink_metadata(path)
                .map_err(|error| format!("inspect native-window output directory: {error}"))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err("native-window output root is not a real directory".to_string());
            }
            Ok(Self {
                path: path.to_path_buf(),
            })
        }
    }

    #[cfg(unix)]
    fn publish(&self, name: &std::ffi::OsStr, bytes: &[u8], label: &str) -> Result<(), String> {
        use std::ffi::CString;
        use std::io::Write as _;
        use std::os::fd::{AsRawFd as _, FromRawFd as _};
        use std::os::unix::ffi::OsStrExt as _;

        let name = CString::new(name.as_bytes())
            .map_err(|_| format!("native-window {label} filename contains NUL"))?;
        let temporary = CString::new(format!(".{}.tmp", name.to_string_lossy()))
            .map_err(|_| format!("native-window {label} temporary filename contains NUL"))?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                temporary.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                0o600,
            )
        };
        if descriptor < 0 {
            return Err(format!(
                "create native-window {label} temporary: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut file = unsafe { fs::File::from_raw_fd(descriptor) };
        if let Err(error) = file
            .write_all(bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
        {
            drop(file);
            unsafe {
                libc::unlinkat(self.directory.as_raw_fd(), temporary.as_ptr(), 0);
            }
            return Err(format!("write native-window {label}: {error}"));
        }
        drop(file);
        if unsafe {
            libc::renameat(
                self.directory.as_raw_fd(),
                temporary.as_ptr(),
                self.directory.as_raw_fd(),
                name.as_ptr(),
            )
        } != 0
        {
            let error = std::io::Error::last_os_error();
            unsafe {
                libc::unlinkat(self.directory.as_raw_fd(), temporary.as_ptr(), 0);
            }
            return Err(format!("publish native-window {label}: {error}"));
        }
        if unsafe { libc::fsync(self.directory.as_raw_fd()) } != 0 {
            return Err(format!(
                "sync native-window {label} directory: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn publish(&self, name: &std::ffi::OsStr, bytes: &[u8], label: &str) -> Result<(), String> {
        use std::io::Write as _;

        let destination = self.path.join(name);
        let temporary = destination.with_extension("json.tmp");
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| format!("create native-window {label} temporary: {error}"))?;
        if let Err(error) = file
            .write_all(bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
        {
            drop(file);
            let _ = fs::remove_file(&temporary);
            return Err(format!("write native-window {label}: {error}"));
        }
        drop(file);
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("publish native-window {label}: {error}"));
        }
        Ok(())
    }

    #[cfg(unix)]
    fn read(
        &self,
        name: &std::ffi::OsStr,
        byte_limit: u64,
        label: &str,
    ) -> Result<Vec<u8>, String> {
        use std::ffi::CString;
        use std::io::Read as _;
        use std::os::fd::{AsRawFd as _, FromRawFd as _};
        use std::os::unix::ffi::OsStrExt as _;

        let name = CString::new(name.as_bytes())
            .map_err(|_| format!("native-window {label} filename contains NUL"))?;
        let descriptor = unsafe {
            libc::openat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(format!(
                "open native-window {label}: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut file = unsafe { fs::File::from_raw_fd(descriptor) };
        if !file
            .metadata()
            .map_err(|error| format!("inspect native-window {label}: {error}"))?
            .is_file()
        {
            return Err(format!("native-window {label} is not a regular file"));
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(byte_limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| format!("read native-window {label}: {error}"))?;
        if bytes.len() as u64 > byte_limit {
            return Err(format!("native-window {label} exceeds its byte budget"));
        }
        Ok(bytes)
    }

    #[cfg(not(unix))]
    fn read(
        &self,
        name: &std::ffi::OsStr,
        byte_limit: u64,
        label: &str,
    ) -> Result<Vec<u8>, String> {
        let bytes = read_regular_nofollow_bounded(&self.path.join(name), byte_limit)
            .map_err(|error| format!("read native-window {label}: {error}"))?;
        if bytes.len() as u64 > byte_limit {
            return Err(format!("native-window {label} exceeds its byte budget"));
        }
        Ok(bytes)
    }
}

impl BoundOutputDirectory {
    #[cfg(unix)]
    fn contains(&self, name: &std::ffi::OsStr) -> Result<bool, String> {
        use std::ffi::CString;
        use std::mem::MaybeUninit;
        use std::os::fd::AsRawFd as _;
        use std::os::unix::ffi::OsStrExt as _;

        let name = CString::new(name.as_bytes())
            .map_err(|_| "native-window output filename contains NUL".to_string())?;
        let mut metadata = MaybeUninit::<libc::stat>::uninit();
        if unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                name.as_ptr(),
                metadata.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } == 0
        {
            return Ok(true);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(false)
        } else {
            Err(format!("inspect native-window output: {error}"))
        }
    }

    #[cfg(not(unix))]
    fn contains(&self, name: &std::ffi::OsStr) -> Result<bool, String> {
        match fs::symlink_metadata(self.path.join(name)) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(format!("inspect native-window output: {error}")),
        }
    }
}

#[derive(Debug)]
pub struct NativeWindowAckPublisher {
    output_directory: BoundOutputDirectory,
    filename: std::ffi::OsString,
}

impl NativeWindowAckPublisher {
    pub fn bind(path: &std::path::Path) -> Result<Self, String> {
        let parent = path
            .parent()
            .ok_or_else(|| "native-window acknowledgement has no parent directory".to_string())?;
        let filename = path
            .file_name()
            .ok_or_else(|| "native-window acknowledgement has no filename".to_string())?;
        Ok(Self {
            output_directory: BoundOutputDirectory::open(parent)?,
            filename: filename.to_os_string(),
        })
    }

    pub fn acknowledge(&self, nonce: &str, committed_presentation: u64) -> Result<(), String> {
        if !is_lower_hex(nonce, 64) || committed_presentation == 0 {
            return Err("native-window acknowledgement violates its identity contract".to_string());
        }
        let bytes = serde_json::to_vec_pretty(&NativeWindowAck {
            schema: NATIVE_WINDOW_ACK_SCHEMA.to_string(),
            nonce: nonce.to_string(),

            committed_presentation,
        })
        .map_err(|error| format!("serialize native-window acknowledgement: {error}"))?;
        self.output_directory
            .publish(&self.filename, &bytes, "acknowledgement")
    }
}

#[derive(Debug)]
pub struct NativeWindowStateReader {
    output_directory: BoundOutputDirectory,
    filename: std::ffi::OsString,
    byte_limit: u64,
}

impl NativeWindowStateReader {
    pub fn bind(path: &std::path::Path, byte_limit: u64) -> Result<Self, String> {
        if byte_limit == 0 {
            return Err("native-window state byte budget must be nonzero".to_string());
        }
        let parent = path
            .parent()
            .ok_or_else(|| "native-window state has no parent directory".to_string())?;
        let filename = path
            .file_name()
            .ok_or_else(|| "native-window state has no filename".to_string())?;
        Ok(Self {
            output_directory: BoundOutputDirectory::open(parent)?,
            filename: filename.to_os_string(),
            byte_limit,
        })
    }

    pub fn read(
        &self,
        expected_nonce: &str,
        expected_pid: u32,
        expected_bounds: NativeWindowBounds,
    ) -> Result<NativeWindowChildState, String> {
        let bytes = self
            .output_directory
            .read(&self.filename, self.byte_limit, "state")?;
        validate_native_window_state_bytes(&bytes, expected_nonce, expected_pid, expected_bounds)
    }

    pub fn read_if_present(
        &self,
        expected_nonce: &str,
        expected_pid: u32,
        expected_bounds: NativeWindowBounds,
    ) -> Result<Option<NativeWindowChildState>, String> {
        if !self.output_directory.contains(&self.filename)? {
            return Ok(None);
        }
        self.read(expected_nonce, expected_pid, expected_bounds)
            .map(Some)
    }
}

#[derive(Debug, Clone)]
pub struct ActiveNativeWindowConfig {
    pub nonce: String,
    pub client_bounds: NativeWindowBounds,
    pub state_path: std::path::PathBuf,
    pub ack_path: std::path::PathBuf,
    output_directory: std::sync::Arc<BoundOutputDirectory>,
    pub runtime_contract: NativeWindowRuntimeContract,
    pub acceptance_policy: NativeAcceptancePolicy,
}

static ACTIVE_NATIVE_WINDOW_CONFIG: std::sync::OnceLock<ActiveNativeWindowConfig> =
    std::sync::OnceLock::new();

pub const NATIVE_WINDOW_RECEIPT_SCHEMA: &str = "uqm-native-window-proof-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeProcessIdentity {
    pub pid: u32,
    pub start_time: String,
    pub executable_sha256: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl NativeWindowBounds {
    #[must_use]
    pub fn contains(self, inner: Self) -> bool {
        let left = i64::from(self.x);
        let top = i64::from(self.y);
        let right = left + i64::from(self.width);
        let bottom = top + i64::from(self.height);
        let inner_left = i64::from(inner.x);
        let inner_top = i64::from(inner.y);
        let inner_right = inner_left + i64::from(inner.width);
        let inner_bottom = inner_top + i64::from(inner.height);
        left <= inner_left && top <= inner_top && right >= inner_right && bottom >= inner_bottom
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowBinding {
    pub process: NativeProcessIdentity,
    pub window_id: u64,
    pub client_bounds: NativeWindowBounds,
    pub os_bounds: NativeWindowBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowObservation {
    pub binding: NativeWindowBinding,
    pub committed_presentation: u64,
    pub visible: bool,
    pub minimized: bool,
    pub semantic: NativeWindowSemanticSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeScreenshotStage {
    Stable,
    Playable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeScreenshot {
    pub stage: NativeScreenshotStage,
    pub binding: NativeWindowBinding,
    pub post_capture_observation: NativeWindowObservation,
    pub committed_presentation: u64,
    pub input_events: u64,
    pub trace_record_count: u64,
    pub battle_frames: u64,
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeWindowReceipt {
    pub schema: String,
    pub acceptance_policy: NativeAcceptancePolicy,
    pub binding: NativeWindowBinding,
    pub first_visible_presentation: u64,
    pub final_committed_presentation: u64,
    pub stable_presentations: u64,
    pub input_events: u64,
    pub battle_frames: u64,
    pub observations: Vec<NativeWindowObservation>,
    pub screenshots: Vec<NativeScreenshot>,
    pub passed: bool,
}

pub const NATIVE_ACCEPTANCE_SCHEMA: &str = "uqm-native-window-acceptance-v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeChildCleanupReceipt {
    pub process: NativeProcessIdentity,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub term_sent: bool,
    pub kill_sent: bool,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub output_drained: bool,
    pub initial_process_group_empty: bool,
    pub config_root_removed: bool,
    pub materialized_content_removed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRetainedInput {
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

pub const NATIVE_LINKED_BUILD_RECEIPT_SCHEMA: &str = "uqm-native-linked-build-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeLinkedBuildReceipt {
    pub schema: String,
    pub source_sha: String,
    pub cargo_command: Vec<String>,
    pub native_profile: String,
    pub feature: String,
    pub cargo_executable_path: String,
    pub cargo_rust_archive_path: String,
    pub cargo_out_dir: String,
    pub executable: NativeRetainedInput,
    pub cargo_messages: NativeRetainedInput,
    pub rust_archive: NativeRetainedInput,
    pub c_archive: NativeRetainedInput,
    pub object_sidecar: NativeRetainedInput,
    pub provider_report: NativeRetainedInput,
    pub native_build_evidence: NativeRetainedInput,
    pub cargo_manifest: NativeRetainedInput,
    pub cargo_lock: NativeRetainedInput,
    pub authority: NativeRetainedInput,
    pub canonical_toolchain: NativeRetainedInput,
}

const NATIVE_BUILD_EVIDENCE_SCHEMA: &str = "uqm-native-build-evidence-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedProviderReport {
    schema: String,
    entries: Vec<LinkedProviderEntry>,
    ledger_sha256: String,
    symbols: Vec<LinkedProviderSymbol>,
    tracked_native_file_delta: i32,
    summary: LinkedProviderSummary,
    #[serde(default)]
    diagnostics: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedProviderEntry {
    path: String,
    issue: String,
    provider: String,
    archive_decision: serde_json::Value,
    status: String,
    #[serde(default)]
    diagnostics: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedProviderSymbol {
    symbol: String,
    canonical_owner: String,
    provider_kind: serde_json::Value,
    provider_path: String,
    excluded_provider_paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedProviderSummary {
    total_objects: usize,
    included: usize,
    excluded: usize,
    duplicate_providers_excluded: usize,
    recompiled: usize,
    replaced: usize,
    violations: usize,
    passed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedNativeBuildEvidence {
    schema: String,
    source_date_epoch: u64,
    build_date: String,
    target: String,
    active_features: Vec<String>,
    toolchain: LinkedToolchainIdentity,
    packages: Vec<LinkedPackageIdentity>,
    compile_profile: LinkedCompileProfile,
    build_environment: std::collections::BTreeMap<String, String>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LinkedToolIdentity {
    executable: String,
    version: String,
    sha256: String,
    effective_args: Vec<String>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct LinkedToolchainIdentity {
    target: String,
    rustc: LinkedToolIdentity,
    cargo: LinkedToolIdentity,
    cc: LinkedToolIdentity,
    ar: LinkedToolIdentity,
    nm: LinkedToolIdentity,
    pkg_config: LinkedToolIdentity,
    linker: LinkedToolIdentity,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedPackageIdentity {
    name: String,
    version: String,
    cflags: Vec<String>,
    libs: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedCompileProfile {
    target: String,
    compiler: String,
    ordered_defines: Vec<String>,
    ordered_include_roots: Vec<String>,
    ordered_compile_flags: Vec<String>,
    dependency_flags: Vec<String>,
    command_template: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeExecutionIdentity {
    pub real_uid: u32,
    pub effective_uid: u32,
    pub launchd_manager_uid: u32,
    pub launchd_manager_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAcceptanceManifest {
    pub schema: String,
    pub command: Vec<String>,
    pub environment: std::collections::BTreeMap<String, String>,
    pub execution_identity: NativeExecutionIdentity,
    pub executable: NativeRetainedInput,
    pub script: NativeRetainedInput,
    pub content_package: NativeRetainedInput,
    pub runtime_contract: NativeWindowRuntimeContract,
    pub acceptance_policy: NativeAcceptancePolicy,
    pub retained_files: Vec<NativeRetainedInput>,
    pub trace_path: String,
    pub trace_byte_length: u64,
    pub trace_sha256: String,
    pub child: NativeChildCleanupReceipt,
    pub window: NativeWindowReceipt,
    pub publications: Vec<NativeWindowPublication>,
    pub passed: bool,
}

pub const NATIVE_ACCEPTANCE_FAILURE_SCHEMA: &str = "uqm-native-window-acceptance-failure-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeAcceptanceFailureContract {
    ChildSupervision,
    ChildExit,
    Observer,
    ConfigCleanup,
    MaterializedContentCleanup,
    Semantic,
}

pub const NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA: &str =
    "uqm-native-window-acceptance-setup-failure-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeAcceptanceSetupFailureContract {
    Preparation,
    ChildSpawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAcceptanceSetupFailureManifest {
    pub schema: String,
    pub command: Vec<String>,
    pub expected_executable_byte_length: u64,
    pub expected_executable_sha256: String,
    pub runtime_contract: NativeWindowRuntimeContract,
    pub acceptance_policy: NativeAcceptancePolicy,
    pub retained_files: Vec<NativeRetainedInput>,
    pub failure_contract: NativeAcceptanceSetupFailureContract,
    pub error: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeAcceptanceFailureManifest {
    pub schema: String,
    pub command: Vec<String>,
    pub environment: std::collections::BTreeMap<String, String>,
    pub executable: NativeRetainedInput,
    pub script: NativeRetainedInput,
    pub content_package: NativeRetainedInput,
    pub runtime_contract: NativeWindowRuntimeContract,
    pub acceptance_policy: NativeAcceptancePolicy,
    pub retained_files: Vec<NativeRetainedInput>,
    pub child: NativeChildCleanupReceipt,
    pub failure_contract: NativeAcceptanceFailureContract,
    pub error: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWindowProofError {
    InvalidIdentity,
    InvalidBounds,
    ExpectedProcessChanged,
    ExpectedClientBoundsChanged {
        expected: NativeWindowBounds,
        actual: NativeWindowBounds,
    },
    OsBoundsDoNotContainClient {
        client: NativeWindowBounds,
        os: NativeWindowBounds,
    },
    BindingChanged {
        expected_window_id: u64,
        actual_window_id: u64,
        expected_os_bounds: NativeWindowBounds,
        actual_os_bounds: NativeWindowBounds,
    },
    NotVisible,
    PresentationOrder,
    StableFloor,
    PlayableFloor,
    MissingInput,
    BattleFloor,
    ScreenshotIdentity,
    ScreenshotStage,
    Receipt,
}

impl std::fmt::Display for NativeWindowProofError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity => formatter.write_str("invalid native process identity"),
            Self::InvalidBounds => formatter.write_str("invalid native window bounds"),
            Self::ExpectedProcessChanged => {
                formatter.write_str("observed process differs from the expected child")
            }
            Self::ExpectedClientBoundsChanged { expected, actual } => write!(
                formatter,
                "observed client bounds changed from {expected:?} to {actual:?}"
            ),
            Self::OsBoundsDoNotContainClient { client, os } => write!(
                formatter,
                "native OS bounds {os:?} do not contain client bounds {client:?}"
            ),
            Self::BindingChanged {
                expected_window_id,
                actual_window_id,
                expected_os_bounds,
                actual_os_bounds,
            } => write!(
                formatter,
                "native window binding changed from id {expected_window_id} {expected_os_bounds:?} to id {actual_window_id} {actual_os_bounds:?}"
            ),
            Self::NotVisible => formatter.write_str("native window is not visible"),
            Self::PresentationOrder => {
                formatter.write_str("native presentation order is invalid")
            }
            Self::StableFloor => formatter.write_str("stable presentation floor was not met"),
            Self::PlayableFloor => formatter.write_str("playable presentation floor was not met"),
            Self::MissingInput => formatter.write_str("accepted player input is missing"),
            Self::BattleFloor => formatter.write_str("battle-frame floor was not met"),
            Self::ScreenshotIdentity => {
                formatter.write_str("native screenshot identity is invalid")
            }
            Self::ScreenshotStage => formatter.write_str("native screenshot stage is invalid"),
            Self::Receipt => formatter.write_str("native-window receipt is invalid"),
        }
    }
}

impl std::error::Error for NativeWindowProofError {}

pub struct NativeWindowProof {
    expected_process: NativeProcessIdentity,
    expected_client_bounds: NativeWindowBounds,
    acceptance_policy: NativeAcceptancePolicy,
    binding: Option<NativeWindowBinding>,
    first_visible_presentation: Option<u64>,
    last_presentation: Option<u64>,
    stable_presentations: u64,
    input_events: u64,
    battle_frames: u64,
    observations: Vec<NativeWindowObservation>,
    screenshots: Vec<NativeScreenshot>,
}

impl NativeWindowProof {
    #[must_use]
    pub fn new(
        expected_process: NativeProcessIdentity,
        expected_client_bounds: NativeWindowBounds,
        acceptance_policy: NativeAcceptancePolicy,
    ) -> Self {
        Self {
            expected_process,
            expected_client_bounds,
            acceptance_policy,
            binding: None,
            first_visible_presentation: None,
            last_presentation: None,
            stable_presentations: 0,
            input_events: 0,
            battle_frames: 0,
            observations: Vec::new(),
            screenshots: Vec::new(),
        }
    }

    #[must_use]
    pub fn bound_window_id(&self) -> Option<u64> {
        self.binding.as_ref().map(|binding| binding.window_id)
    }

    pub fn observe_visible(
        &mut self,
        observation: NativeWindowObservation,
    ) -> Result<(), NativeWindowProofError> {
        validate_process(&observation.binding.process)?;
        validate_bounds(observation.binding.client_bounds)?;
        validate_bounds(observation.binding.os_bounds)?;
        if observation.binding.process != self.expected_process {
            return Err(NativeWindowProofError::ExpectedProcessChanged);
        }
        if observation.binding.client_bounds != self.expected_client_bounds {
            return Err(NativeWindowProofError::ExpectedClientBoundsChanged {
                expected: self.expected_client_bounds,
                actual: observation.binding.client_bounds,
            });
        }
        if !observation
            .binding
            .os_bounds
            .contains(observation.binding.client_bounds)
        {
            return Err(NativeWindowProofError::OsBoundsDoNotContainClient {
                client: observation.binding.client_bounds,
                os: observation.binding.os_bounds,
            });
        }
        if !observation.visible || observation.minimized {
            return Err(NativeWindowProofError::NotVisible);
        }
        if observation.semantic.accepted_player_inputs > observation.semantic.trace_record_count
            || (observation.semantic.verified_battle_frames > 0
                && observation.semantic.trace_record_count == 0)
        {
            return Err(NativeWindowProofError::Receipt);
        }
        if let Some(binding) = &self.binding {
            if binding != &observation.binding {
                return Err(NativeWindowProofError::BindingChanged {
                    expected_window_id: binding.window_id,
                    actual_window_id: observation.binding.window_id,
                    expected_os_bounds: binding.os_bounds,
                    actual_os_bounds: observation.binding.os_bounds,
                });
            }
            let previous_presentation = self
                .last_presentation
                .ok_or(NativeWindowProofError::PresentationOrder)?;
            if observation.committed_presentation <= previous_presentation {
                return Err(NativeWindowProofError::PresentationOrder);
            }
            let previous = self
                .observations
                .last()
                .ok_or(NativeWindowProofError::Receipt)?
                .semantic;
            if observation.semantic.trace_record_count < previous.trace_record_count
                || observation.semantic.accepted_player_inputs < previous.accepted_player_inputs
                || observation.semantic.verified_battle_frames < previous.verified_battle_frames
            {
                return Err(NativeWindowProofError::Receipt);
            }
            self.stable_presentations = observation
                .committed_presentation
                .checked_sub(
                    self.first_visible_presentation
                        .ok_or(NativeWindowProofError::PresentationOrder)?,
                )
                .ok_or(NativeWindowProofError::PresentationOrder)?;
        } else {
            if observation.binding.window_id == 0 {
                return Err(NativeWindowProofError::InvalidIdentity);
            }
            self.first_visible_presentation = Some(observation.committed_presentation);
            self.binding = Some(observation.binding.clone());
        }
        self.last_presentation = Some(observation.committed_presentation);
        self.input_events = observation.semantic.accepted_player_inputs;
        self.battle_frames = observation.semantic.verified_battle_frames;
        self.observations.push(observation);
        Ok(())
    }

    pub fn record_screenshot(
        &mut self,
        screenshot: NativeScreenshot,
    ) -> Result<(), NativeWindowProofError> {
        let binding = self
            .binding
            .as_ref()
            .ok_or(NativeWindowProofError::ScreenshotIdentity)?;
        let semantic = self
            .observations
            .last()
            .ok_or(NativeWindowProofError::ScreenshotIdentity)?
            .semantic;
        if &screenshot.binding != binding
            || screenshot.post_capture_observation.binding != screenshot.binding
            || screenshot.post_capture_observation.committed_presentation
                != screenshot.committed_presentation
            || !screenshot.post_capture_observation.visible
            || screenshot.post_capture_observation.minimized
            || screenshot.post_capture_observation.semantic != semantic
            || screenshot.input_events != semantic.accepted_player_inputs
            || screenshot.trace_record_count != semantic.trace_record_count
            || screenshot.battle_frames != semantic.verified_battle_frames
            || screenshot.committed_presentation != self.last_presentation.unwrap_or_default()
            || !is_normal_relative_path(&screenshot.relative_path)
            || screenshot.byte_length == 0
            || !is_lower_hex(&screenshot.sha256, 64)
        {
            return Err(NativeWindowProofError::ScreenshotIdentity);
        }
        let stage_is_valid = match screenshot.stage {
            NativeScreenshotStage::Stable => {
                self.stable_presentations == self.acceptance_policy.stable_presentation_floor
                    && !self
                        .screenshots
                        .iter()
                        .any(|existing| existing.stage == NativeScreenshotStage::Playable)
            }
            NativeScreenshotStage::Playable => {
                let first_visible = self.first_visible_presentation.unwrap_or_default();
                let prior_eligible = self
                    .observations
                    .iter()
                    .take(self.observations.len().saturating_sub(1))
                    .any(|observation| {
                        observation
                            .committed_presentation
                            .checked_sub(first_visible)
                            .is_some_and(|post_visible| {
                                post_visible >= self.acceptance_policy.playable_presentation_floor
                                    && observation.semantic.accepted_player_inputs > 0
                                    && observation.semantic.verified_battle_frames
                                        >= self.acceptance_policy.battle_frame_floor
                            })
                    });
                self.stable_presentations >= self.acceptance_policy.playable_presentation_floor
                    && self
                        .screenshots
                        .iter()
                        .any(|existing| existing.stage == NativeScreenshotStage::Stable)
                    && screenshot.input_events > 0
                    && screenshot.battle_frames >= self.acceptance_policy.battle_frame_floor
                    && !prior_eligible
            }
        };
        if !stage_is_valid
            || self
                .screenshots
                .iter()
                .any(|existing| existing.stage == screenshot.stage)
        {
            return Err(NativeWindowProofError::ScreenshotStage);
        }
        self.screenshots.push(screenshot);
        Ok(())
    }

    pub fn finish(self) -> Result<NativeWindowReceipt, NativeWindowProofError> {
        if self.stable_presentations < self.acceptance_policy.stable_presentation_floor {
            return Err(NativeWindowProofError::StableFloor);
        }
        if self.stable_presentations < self.acceptance_policy.playable_presentation_floor {
            return Err(NativeWindowProofError::PlayableFloor);
        }
        if self.input_events == 0 {
            return Err(NativeWindowProofError::MissingInput);
        }
        if self.battle_frames < self.acceptance_policy.battle_frame_floor {
            return Err(NativeWindowProofError::BattleFloor);
        }
        for stage in [
            NativeScreenshotStage::Stable,
            NativeScreenshotStage::Playable,
        ] {
            if !self.screenshots.iter().any(|shot| shot.stage == stage) {
                return Err(NativeWindowProofError::ScreenshotStage);
            }
        }
        if self.screenshots.len() != 2 || self.screenshots[0].sha256 == self.screenshots[1].sha256 {
            return Err(NativeWindowProofError::ScreenshotStage);
        }
        Ok(NativeWindowReceipt {
            schema: NATIVE_WINDOW_RECEIPT_SCHEMA.to_string(),
            acceptance_policy: self.acceptance_policy,
            binding: self.binding.ok_or(NativeWindowProofError::NotVisible)?,
            first_visible_presentation: self
                .first_visible_presentation
                .ok_or(NativeWindowProofError::NotVisible)?,
            final_committed_presentation: self
                .last_presentation
                .ok_or(NativeWindowProofError::NotVisible)?,
            stable_presentations: self.stable_presentations,
            input_events: self.input_events,
            battle_frames: self.battle_frames,
            observations: self.observations,
            screenshots: self.screenshots,
            passed: true,
        })
    }
}

fn normalized_positive_process_start(value: &str) -> bool {
    !value.is_empty() && !value.starts_with("0") && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_process(process: &NativeProcessIdentity) -> Result<(), NativeWindowProofError> {
    if process.pid == 0
        || !normalized_positive_process_start(&process.start_time)
        || !is_lower_hex(&process.executable_sha256, 64)
        || !is_lower_hex(&process.nonce, 64)
    {
        return Err(NativeWindowProofError::InvalidIdentity);
    }
    Ok(())
}

fn validate_bounds(bounds: NativeWindowBounds) -> Result<(), NativeWindowProofError> {
    if bounds.width == 0 || bounds.height == 0 {
        return Err(NativeWindowProofError::InvalidBounds);
    }
    Ok(())
}

fn is_normal_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_absolute_path(value: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            )
        })
}
fn valid_bound_command_path(value: &str) -> bool {
    valid_absolute_path(value)
        || (!value.is_empty()
            && !value.contains('\\')
            && Path::new(value)
                .components()
                .all(|component| matches!(component, Component::CurDir | Component::Normal(_))))
}

#[cfg(unix)]
fn read_relative_regular_nofollow_bounded(
    root: &Path,
    relative: &Path,
    limit: u64,
) -> std::io::Result<Vec<u8>> {
    use std::{
        ffi::CString,
        io::Read as _,
        os::fd::{AsRawFd as _, FromRawFd as _},
        os::unix::{ffi::OsStrExt as _, fs::OpenOptionsExt as _},
    };

    fn open_at(
        directory: &fs::File,
        name: &std::ffi::OsStr,
        flags: i32,
    ) -> std::io::Result<fs::File> {
        let name = CString::new(name.as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL")
        })?;
        // SAFETY: `name` is NUL-terminated, the directory descriptor stays open for the
        // call, and a successful descriptor is immediately owned by `File`.
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `openat` returned a fresh owned descriptor.
        Ok(unsafe { fs::File::from_raw_fd(descriptor) })
    }

    let root_file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(root)?;
    if !root_file.metadata()?.is_dir() {
        return Err(std::io::Error::other("root is not a directory"));
    }
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(name) => Ok(name),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path is not normalized relative",
            )),
        })
        .collect::<std::io::Result<Vec<_>>>()?;
    let (filename, directories) = components
        .split_last()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path is empty"))?;
    let mut directory = root_file;
    for name in directories {
        directory = open_at(&directory, name, libc::O_RDONLY | libc::O_DIRECTORY)?;
        if !directory.metadata()?.is_dir() {
            return Err(std::io::Error::other("path component is not a directory"));
        }
    }
    let file = open_at(&directory, filename, libc::O_RDONLY)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(std::io::Error::other("path is not a regular file"));
    }
    if metadata.len() > limit {
        return Err(std::io::Error::other("regular file exceeds byte limit"));
    }
    let read_limit = limit
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("regular file byte limit overflowed"))?;
    let mut bytes = Vec::with_capacity(metadata.len().min(1024 * 1024) as usize);
    file.take(read_limit).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::other("regular file grew beyond byte limit"));
    }
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_relative_regular_nofollow_bounded(
    root: &Path,
    relative: &Path,
    limit: u64,
) -> std::io::Result<Vec<u8>> {
    use std::io::Read as _;

    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::other("path is not a regular file"));
    }
    if metadata.len() > limit {
        return Err(std::io::Error::other("regular file exceeds byte limit"));
    }
    let read_limit = limit
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("regular file byte limit overflowed"))?;
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(metadata.len().min(1024 * 1024) as usize);
    file.take(read_limit).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(std::io::Error::other("regular file grew beyond byte limit"));
    }
    Ok(bytes)
}

fn read_regular_nofollow_bounded(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let filename = path.file_name().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no file name")
    })?;
    read_relative_regular_nofollow_bounded(parent, Path::new(filename), limit)
}

#[cfg(target_os = "macos")]
pub fn capture_native_window(
    window_id: u64,
    path: &Path,
    contract: NativeWindowRuntimeContract,
) -> Result<(), NativeWindowObserverError> {
    if window_id == 0 || path.exists() {
        return Err(NativeWindowObserverError::Os(
            "native screenshot destination or window identity is invalid".to_string(),
        ));
    }
    let screencapture = Path::new("/usr/sbin/screencapture");
    let executable_digest = format!(
        "{:x}",
        Sha256::digest(
            &read_regular_nofollow_bounded(screencapture, contract.inventory_limits.member_bytes,)
                .map_err(|error| {
                    NativeWindowObserverError::Os(format!("read screencapture executable: {error}"))
                })?
        )
    );
    let scratch = path.with_extension("capture-supervision");
    fs::create_dir(&scratch).map_err(|error| {
        NativeWindowObserverError::Os(format!("create capture supervision directory: {error}"))
    })?;
    let mut command = std::process::Command::new(screencapture);
    command
        .arg("-x")
        .arg("-o")
        .arg(format!("-l{window_id}"))
        .arg(path);
    use std::os::unix::process::CommandExt as _;

    let capture_budget = contract.capture_budget_bytes as libc::rlim_t;
    unsafe {
        command.pre_exec(move || {
            let limit = libc::rlimit {
                rlim_cur: capture_budget,
                rlim_max: capture_budget,
            };
            if libc::setrlimit(libc::RLIMIT_FSIZE, &limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let session = ChildSession::spawn(
        command,
        ChildSessionConfig {
            stdout_log: scratch.join("stdout.log"),
            stderr_log: scratch.join("stderr.log"),
            stdout_budget: contract.observer_response_budget_bytes,
            stderr_budget: contract.observer_response_budget_bytes,
            timeout: contract.capture_timeout(),
            grace: contract.capture_kill_grace(),
            executable_digest,
        },
    );
    let outcome = match session {
        Ok(session) => session.finish(),
        Err(error) => {
            let _ = fs::remove_dir_all(&scratch);
            return Err(NativeWindowObserverError::Os(format!(
                "launch screencapture: {error}"
            )));
        }
    };
    let _ = fs::remove_dir_all(&scratch);
    match outcome {
        Ok(receipt) if receipt.exit_code == Some(0) && receipt.signal.is_none() => {}
        Ok(receipt) => {
            return Err(NativeWindowObserverError::Os(format!(
                "screencapture exited with code {:?}, signal {:?}",
                receipt.exit_code, receipt.signal
            )));
        }
        Err(failure) => {
            return match failure.error {
                ChildSessionError::BudgetExceeded { stream } => {
                    Err(NativeWindowObserverError::OutputLimit {
                        stream: format!("{stream:?}").to_ascii_lowercase(),
                        limit_bytes: contract.observer_response_budget_bytes,
                    })
                }
                _ => Err(NativeWindowObserverError::Os(format!(
                    "supervise screencapture: {failure}"
                ))),
            };
        }
    }
    let bytes = read_regular_nofollow_bounded(path, contract.capture_budget_bytes)
        .map_err(|error| NativeWindowObserverError::Os(format!("read screenshot: {error}")))?;
    decode_png_bounded(&bytes, contract.capture_budget_bytes)
}

#[cfg(any(target_os = "macos", test))]
fn decode_png_bounded(bytes: &[u8], budget: u64) -> Result<(), NativeWindowObserverError> {
    if budget == 0 || bytes.len() as u64 > budget {
        return Err(NativeWindowObserverError::OutputLimit {
            stream: "screenshot".to_string(),
            limit_bytes: budget,
        });
    }
    let dimensions =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png)
            .into_dimensions()
            .map_err(|error| {
                NativeWindowObserverError::Os(format!("inspect screenshot: {error}"))
            })?;
    let decoded_bytes = u64::from(dimensions.0)
        .checked_mul(u64::from(dimensions.1))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            NativeWindowObserverError::Os("screenshot dimensions overflowed".to_string())
        })?;
    if dimensions.0 == 0 || dimensions.1 == 0 || decoded_bytes > budget {
        return Err(NativeWindowObserverError::OutputLimit {
            stream: "screenshot-decoded".to_string(),
            limit_bytes: budget,
        });
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(dimensions.0);
    limits.max_image_height = Some(dimensions.1);
    limits.max_alloc = Some(budget);
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| NativeWindowObserverError::Os(format!("decode screenshot: {error}")))?;
    if decoded.width() != dimensions.0 || decoded.height() != dimensions.1 {
        return Err(NativeWindowObserverError::Os(
            "screenshot dimensions changed during decoding".to_string(),
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn capture_native_window(
    _window_id: u64,
    _path: &Path,
    _contract: NativeWindowRuntimeContract,
) -> Result<(), NativeWindowObserverError> {
    Err(NativeWindowObserverError::UnsupportedPlatform)
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(target_os = "macos")]
pub fn observe_native_window(
    pid: u32,
    expected_window_id: Option<u64>,
) -> Result<ObservedNativeWindow, NativeWindowObserverError> {
    darwin::observe(pid, expected_window_id)
}

#[cfg(not(target_os = "macos"))]
pub fn observe_native_window(
    _pid: u32,
    _expected_window_id: Option<u64>,
) -> Result<ObservedNativeWindow, NativeWindowObserverError> {
    Err(NativeWindowObserverError::UnsupportedPlatform)
}

#[cfg(target_os = "macos")]
mod darwin {
    use super::{NativeWindowBounds, NativeWindowObserverError, ObservedNativeWindow};
    use std::ffi::c_void;

    type CfArrayRef = *const c_void;
    type CfDictionaryRef = *const c_void;
    type CfTypeRef = *const c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGSize {
        width: f64,
        height: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFArrayGetCount(array: CfArrayRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CfArrayRef, index: isize) -> *const c_void;
        fn CFDictionaryGetValue(dictionary: CfDictionaryRef, key: *const c_void) -> *const c_void;
        fn CFNumberGetValue(number: CfTypeRef, number_type: i32, value: *mut c_void) -> u8;
        fn CFBooleanGetValue(boolean: CfTypeRef) -> u8;
        fn CFRelease(value: CfTypeRef);
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        static kCGWindowOwnerPID: *const c_void;
        static kCGWindowNumber: *const c_void;
        static kCGWindowBounds: *const c_void;
        static kCGWindowIsOnscreen: *const c_void;
        static kCGWindowLayer: *const c_void;
        fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CfArrayRef;
        fn CGRectMakeWithDictionaryRepresentation(
            dictionary: CfDictionaryRef,
            bounds: *mut CGRect,
        ) -> u8;
    }

    const CF_NUMBER_I64: i32 = 4;

    pub(super) fn observe(
        pid: u32,
        expected_window_id: Option<u64>,
    ) -> Result<ObservedNativeWindow, NativeWindowObserverError> {
        // SAFETY: CoreGraphics returns an owned immutable CFArray. Every borrowed
        // dictionary/key/value remains valid until the matching CFRelease.
        unsafe {
            let windows = CGWindowListCopyWindowInfo(0, 0);
            if windows.is_null() {
                return Err(NativeWindowObserverError::Os(
                    "CGWindowListCopyWindowInfo returned null".to_string(),
                ));
            }
            let mut matches = Vec::new();
            let count = CFArrayGetCount(windows);
            for index in 0..count {
                let dictionary = CFArrayGetValueAtIndex(windows, index) as CfDictionaryRef;
                let Some(owner) = dictionary_i64(dictionary, kCGWindowOwnerPID) else {
                    continue;
                };
                let Some(layer) = dictionary_i64(dictionary, kCGWindowLayer) else {
                    continue;
                };
                if owner != i64::from(pid) || layer != 0 {
                    continue;
                }
                let Some(window_id) = dictionary_i64(dictionary, kCGWindowNumber) else {
                    continue;
                };
                let bounds_value = CFDictionaryGetValue(dictionary, kCGWindowBounds);
                let mut bounds = CGRect {
                    origin: CGPoint { x: 0.0, y: 0.0 },
                    size: CGSize {
                        width: 0.0,
                        height: 0.0,
                    },
                };
                if bounds_value.is_null()
                    || CGRectMakeWithDictionaryRepresentation(bounds_value, &mut bounds) == 0
                    || window_id <= 0
                    || bounds.size.width < 1.0
                    || bounds.size.height < 1.0
                {
                    continue;
                }
                let on_screen = CFDictionaryGetValue(dictionary, kCGWindowIsOnscreen);
                let visible = !on_screen.is_null() && CFBooleanGetValue(on_screen) != 0;
                matches.push(ObservedNativeWindow {
                    window_id: window_id as u64,
                    os_bounds: NativeWindowBounds {
                        x: bounds.origin.x.round() as i32,
                        y: bounds.origin.y.round() as i32,
                        width: bounds.size.width.round() as u32,
                        height: bounds.size.height.round() as u32,
                    },
                    visible,
                    minimized: !visible,
                });
            }
            CFRelease(windows);
            select_observed_native_window(pid, expected_window_id, matches)
        }
    }

    pub(super) fn select_observed_native_window(
        pid: u32,
        expected_window_id: Option<u64>,
        mut matches: Vec<ObservedNativeWindow>,
    ) -> Result<ObservedNativeWindow, NativeWindowObserverError> {
        matches.retain(|window| {
            window.visible
                && !window.minimized
                && expected_window_id.is_none_or(|expected| window.window_id == expected)
        });
        matches.sort_by_key(|window| {
            (
                window.window_id,
                window.os_bounds.x,
                window.os_bounds.y,
                window.os_bounds.width,
                window.os_bounds.height,
            )
        });
        matches.dedup();
        match matches.len() {
            0 => Err(NativeWindowObserverError::NotFound { pid }),
            1 => Ok(matches[0]),
            count => Err(NativeWindowObserverError::Ambiguous { pid, count }),
        }
    }

    unsafe fn dictionary_i64(dictionary: CfDictionaryRef, key: *const c_void) -> Option<i64> {
        // SAFETY: callers provide a live CGWindow dictionary and one of its exported CFString keys.
        let value = unsafe { CFDictionaryGetValue(dictionary, key) };
        if value.is_null() {
            return None;
        }
        let mut number = 0_i64;
        // SAFETY: `number` is correctly aligned and sized for kCFNumberSInt64Type.
        (unsafe { CFNumberGetValue(value, CF_NUMBER_I64, (&mut number as *mut i64).cast()) } != 0)
            .then_some(number)
    }
}

pub fn activate_native_window_proof(config_path: &Path, output_root: &Path) -> Result<(), String> {
    let bytes = read_regular_nofollow_bounded(config_path, NATIVE_CONTROL_FILE_MAX_BYTES)
        .map_err(|error| format!("read native-window config: {error}"))?;
    let config: NativeWindowConfigFile = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse native-window config: {error}"))?;
    if config.schema != NATIVE_WINDOW_CONFIG_SCHEMA
        || !is_lower_hex(&config.nonce, 64)
        || validate_bounds(config.client_bounds).is_err()
        || !config.runtime_contract.has_valid_deadline_order()
        || !config.acceptance_policy.is_valid()
    {
        return Err("native-window config violates its schema contract".to_string());
    }
    let output_directory = std::sync::Arc::new(BoundOutputDirectory::open(output_root)?);
    ACTIVE_NATIVE_WINDOW_CONFIG
        .set(ActiveNativeWindowConfig {
            nonce: config.nonce,
            client_bounds: config.client_bounds,
            state_path: output_root.join("native-window-state.json"),
            ack_path: output_root.join("native-window-ack.json"),
            output_directory,
            runtime_contract: config.runtime_contract,
            acceptance_policy: config.acceptance_policy,
        })
        .map_err(|_| "native-window proof was activated more than once".to_string())
}

#[must_use]
pub fn active_native_window_config() -> Option<&'static ActiveNativeWindowConfig> {
    ACTIVE_NATIVE_WINDOW_CONFIG.get()
}

#[cfg(feature = "debug-process")]
pub(crate) fn publish_native_window_presentation(
    committed_presentation: u64,
) -> Result<(), String> {
    let Some(config) = active_native_window_config() else {
        return Ok(());
    };
    let window = crate::graphics::ffi::with_gfx_state(|canvas, _, _| {
        if committed_presentation == 1 {
            canvas.window_mut().set_position(
                sdl2::video::WindowPos::Positioned(config.client_bounds.x),
                sdl2::video::WindowPos::Positioned(config.client_bounds.y),
            );
            canvas.window_mut().show();
            canvas.window_mut().raise();
        }
        let (x, y) = canvas.window().position();
        let (width, height) = canvas.window().size();
        (
            canvas.window().id(),
            NativeWindowBounds {
                x,
                y,
                width,
                height,
            },
        )
    })
    .ok_or_else(|| "graphics state is unavailable for native-window proof".to_string())?;
    let (trace_record_count, accepted_player_inputs, verified_battle_frames) =
        crate::automation::Coordinator::native_window_semantic_snapshot()
            .ok_or_else(|| "native-window semantic snapshot is unavailable".to_string())?;
    publish_native_window_state(&NativeWindowChildState {
        schema: NATIVE_WINDOW_STATE_SCHEMA.to_string(),
        nonce: config.nonce.clone(),
        pid: std::process::id(),
        sdl_window_id: window.0,
        committed_presentation,
        shown: true,
        requested_client_bounds: config.client_bounds,
        client_bounds: window.1,
        semantic: NativeWindowSemanticSnapshot {
            trace_record_count,
            accepted_player_inputs,
            verified_battle_frames,
        },
    })
}

pub fn publish_native_window_state(state: &NativeWindowChildState) -> Result<(), String> {
    let config = ACTIVE_NATIVE_WINDOW_CONFIG
        .get()
        .ok_or_else(|| "native-window proof is inactive".to_string())?;
    if state.schema != NATIVE_WINDOW_STATE_SCHEMA
        || state.nonce != config.nonce
        || state.pid != std::process::id()
        || state.sdl_window_id == 0
        || state.requested_client_bounds != config.client_bounds
        || validate_bounds(state.client_bounds).is_err()
    {
        return Err("native-window child state violates its identity contract".to_string());
    }
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|error| format!("serialize native-window state: {error}"))?;
    config.output_directory.publish(
        std::ffi::OsStr::new("native-window-state.json"),
        &bytes,
        "state",
    )?;
    wait_for_native_window_ack(
        &config.output_directory,
        &config.nonce,
        state.committed_presentation,
        config.runtime_contract.acknowledgement_timeout(),
        config.runtime_contract.observer_response_budget_bytes,
    )
}

fn wait_for_native_window_ack(
    output_directory: &BoundOutputDirectory,
    nonce: &str,
    committed_presentation: u64,
    timeout: std::time::Duration,
    byte_limit: u64,
) -> Result<(), String> {
    let deadline = std::time::Instant::now()
        .checked_add(timeout)
        .ok_or_else(|| "native-window acknowledgement deadline overflowed".to_string())?;
    loop {
        if let Ok(bytes) = output_directory.read(
            std::ffi::OsStr::new("native-window-ack.json"),
            byte_limit,
            "acknowledgement",
        ) {
            if let Ok(ack) = serde_json::from_slice::<NativeWindowAck>(&bytes) {
                if ack.schema == NATIVE_WINDOW_ACK_SCHEMA
                    && ack.nonce == nonce
                    && ack.committed_presentation == committed_presentation
                {
                    return Ok(());
                }
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "native-window state presentation {committed_presentation} was not acknowledged"
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

pub fn acknowledge_native_window_state(
    path: &Path,
    nonce: &str,
    committed_presentation: u64,
) -> Result<(), String> {
    NativeWindowAckPublisher::bind(path)?.acknowledge(nonce, committed_presentation)
}

pub fn read_native_window_state(
    path: &Path,
    expected_nonce: &str,
    expected_pid: u32,
    expected_bounds: NativeWindowBounds,
) -> Result<NativeWindowChildState, String> {
    let bytes = read_regular_nofollow_bounded(path, NATIVE_CONTROL_FILE_MAX_BYTES)
        .map_err(|error| format!("read native-window state: {error}"))?;
    validate_native_window_state_bytes(&bytes, expected_nonce, expected_pid, expected_bounds)
}

fn validate_native_window_state_bytes(
    bytes: &[u8],
    expected_nonce: &str,
    expected_pid: u32,
    expected_bounds: NativeWindowBounds,
) -> Result<NativeWindowChildState, String> {
    let state: NativeWindowChildState = serde_json::from_slice(bytes)
        .map_err(|error| format!("parse native-window state: {error}"))?;
    if state.schema != NATIVE_WINDOW_STATE_SCHEMA
        || state.nonce != expected_nonce
        || state.pid != expected_pid
        || state.sdl_window_id == 0
        || state.requested_client_bounds != expected_bounds
        || validate_bounds(state.client_bounds).is_err()
    {
        return Err("native-window state violates its identity contract".to_string());
    }
    Ok(state)
}

pub fn validate_native_window_receipt(
    receipt: &NativeWindowReceipt,
) -> Result<(), NativeWindowProofError> {
    if receipt.schema != NATIVE_WINDOW_RECEIPT_SCHEMA
        || !receipt.passed
        || !receipt.acceptance_policy.is_valid()
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let mut proof = NativeWindowProof::new(
        receipt.binding.process.clone(),
        receipt.binding.client_bounds,
        receipt.acceptance_policy,
    );
    let mut recorded_screenshots = 0;
    for observation in &receipt.observations {
        proof.observe_visible(observation.clone())?;
        for screenshot in receipt.screenshots.iter().filter(|screenshot| {
            screenshot.committed_presentation == observation.committed_presentation
        }) {
            proof.record_screenshot(screenshot.clone())?;
            recorded_screenshots += 1;
        }
    }
    if recorded_screenshots != receipt.screenshots.len()
        || receipt.input_events != proof.input_events
        || receipt.battle_frames != proof.battle_frames
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let rebuilt = proof.finish()?;
    if rebuilt != *receipt {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

pub fn validate_native_window_bundle(
    root: &Path,
    receipt: &NativeWindowReceipt,
    inventory_limits: NativeInventoryLimits,
) -> Result<(), NativeWindowProofError> {
    validate_native_window_receipt(receipt)?;
    let root_metadata = fs::symlink_metadata(root).map_err(|_| NativeWindowProofError::Receipt)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(NativeWindowProofError::Receipt);
    }
    let mut first_capture: Option<(u32, u32, Vec<u8>)> = None;
    for screenshot in &receipt.screenshots {
        let relative = Path::new(&screenshot.relative_path);
        let mut current = root.to_path_buf();
        let component_count = relative.components().count();
        for (position, component) in relative.components().enumerate() {
            let Component::Normal(name) = component else {
                return Err(NativeWindowProofError::ScreenshotIdentity);
            };
            current.push(name);
            let metadata = fs::symlink_metadata(&current)
                .map_err(|_| NativeWindowProofError::ScreenshotIdentity)?;
            if metadata.file_type().is_symlink()
                || (position + 1 == component_count && !metadata.is_file())
                || (position + 1 < component_count && !metadata.is_dir())
            {
                return Err(NativeWindowProofError::ScreenshotIdentity);
            }
        }
        if screenshot.byte_length > inventory_limits.member_bytes {
            return Err(NativeWindowProofError::ScreenshotIdentity);
        }
        let bytes = read_relative_regular_nofollow_bounded(root, relative, screenshot.byte_length)
            .map_err(|_| NativeWindowProofError::ScreenshotIdentity)?;

        if bytes.len() as u64 != screenshot.byte_length
            || format!("{:x}", Sha256::digest(&bytes)) != screenshot.sha256
        {
            return Err(NativeWindowProofError::ScreenshotIdentity);
        }
        use image::ImageDecoder as _;

        let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(&bytes))
            .map_err(|_| NativeWindowProofError::ScreenshotIdentity)?;
        let (width, height) = decoder.dimensions();
        let expected = screenshot.binding.client_bounds;
        let maximum_decoded_bytes = u64::from(expected.width)
            .checked_mul(u64::from(expected.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(NativeWindowProofError::ScreenshotIdentity)?;
        if width != expected.width
            || height != expected.height
            || decoder.total_bytes() > maximum_decoded_bytes
        {
            return Err(NativeWindowProofError::ScreenshotIdentity);
        }
        let decoded = image::DynamicImage::from_decoder(decoder)
            .map_err(|_| NativeWindowProofError::ScreenshotIdentity)?;
        let normalized = decoded.to_rgba8();
        if let Some((width, height, pixels)) = &first_capture {
            if *width != normalized.width()
                || *height != normalized.height()
                || pixels.as_slice() == normalized.as_raw()
            {
                return Err(NativeWindowProofError::ScreenshotStage);
            }
        } else {
            first_capture = Some((
                normalized.width(),
                normalized.height(),
                normalized.into_raw(),
            ));
        }
    }
    Ok(())
}
struct NativeInventoryBudget {
    limits: NativeInventoryLimits,
    entries: u32,
    bytes: u64,
    path_bytes: u64,
}

impl NativeInventoryBudget {
    fn new(limits: NativeInventoryLimits) -> Result<Self, NativeWindowProofError> {
        if !limits.is_valid() {
            return Err(NativeWindowProofError::Receipt);
        }
        Ok(Self {
            limits,
            entries: 0,
            bytes: 0,
            path_bytes: 0,
        })
    }

    fn admit_name(&mut self, name_bytes: usize) -> Result<(), NativeWindowProofError> {
        if name_bytes > self.limits.path_bytes as usize {
            return Err(NativeWindowProofError::Receipt);
        }
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or(NativeWindowProofError::Receipt)?;
        if self.entries > self.limits.member_count {
            return Err(NativeWindowProofError::Receipt);
        }
        Ok(())
    }

    fn admit_path(&mut self, path_bytes: usize) -> Result<(), NativeWindowProofError> {
        let path_bytes = u64::try_from(path_bytes).map_err(|_| NativeWindowProofError::Receipt)?;
        if path_bytes > u64::from(self.limits.path_bytes) {
            return Err(NativeWindowProofError::Receipt);
        }
        self.path_bytes = self
            .path_bytes
            .checked_add(path_bytes)
            .ok_or(NativeWindowProofError::Receipt)?;
        if self.path_bytes > self.limits.aggregate_path_bytes {
            return Err(NativeWindowProofError::Receipt);
        }
        Ok(())
    }

    fn admit_file(&mut self, byte_length: u64) -> Result<(), NativeWindowProofError> {
        self.bytes = self
            .bytes
            .checked_add(byte_length)
            .ok_or(NativeWindowProofError::Receipt)?;
        if byte_length > self.limits.member_bytes || self.bytes > self.limits.aggregate_bytes {
            return Err(NativeWindowProofError::Receipt);
        }
        Ok(())
    }
}
fn digest_exact_bounded<R: std::io::Read>(
    reader: &mut R,
    expected: u64,
    limit: u64,
) -> Result<String, NativeWindowProofError> {
    if expected > limit {
        return Err(NativeWindowProofError::Receipt);
    }
    let mut digest = Sha256::new();
    let mut byte_length = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| NativeWindowProofError::Receipt)?;
        if read == 0 {
            break;
        }
        byte_length = byte_length
            .checked_add(read as u64)
            .ok_or(NativeWindowProofError::Receipt)?;
        if byte_length > expected || byte_length > limit {
            return Err(NativeWindowProofError::Receipt);
        }
        digest.update(&buffer[..read]);
    }
    if byte_length != expected {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn native_acceptance_inventory(
    root: &Path,
    limits: NativeInventoryLimits,
) -> Result<Vec<NativeRetainedInput>, NativeWindowProofError> {
    native_acceptance_inventory_excluding(root, "native-acceptance.json", limits)
}

pub fn native_acceptance_failure_inventory(
    root: &Path,
    limits: NativeInventoryLimits,
) -> Result<Vec<NativeRetainedInput>, NativeWindowProofError> {
    native_acceptance_inventory_excluding(root, "native-acceptance-failure.json", limits)
}

fn native_acceptance_inventory_excluding(
    root: &Path,
    excluded_manifest: &str,
    limits: NativeInventoryLimits,
) -> Result<Vec<NativeRetainedInput>, NativeWindowProofError> {
    let sibling_manifest = if excluded_manifest == "native-acceptance.json" {
        "native-acceptance-failure.json"
    } else {
        "native-acceptance.json"
    };
    let mut files = Vec::new();
    let mut budget = NativeInventoryBudget::new(limits)?;
    collect_inventory(
        root,
        excluded_manifest,
        sibling_manifest,
        &mut files,
        &mut budget,
    )?;
    files.sort();
    Ok(files)
}

#[cfg(unix)]
fn collect_inventory(
    root: &Path,
    excluded_manifest: &str,
    sibling_manifest: &str,
    files: &mut Vec<NativeRetainedInput>,
    budget: &mut NativeInventoryBudget,
) -> Result<(), NativeWindowProofError> {
    use std::{
        ffi::{CString, OsStr},
        os::fd::{AsRawFd as _, FromRawFd as _},
        os::unix::{
            ffi::{OsStrExt as _, OsStringExt as _},
            fs::OpenOptionsExt as _,
        },
    };

    fn open_at(directory: &fs::File, name: &OsStr) -> std::io::Result<fs::File> {
        let name = CString::new(name.as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains NUL")
        })?;
        // SAFETY: `name` and the directory descriptor remain valid for the call. A successful
        // descriptor is immediately transferred to `File`.
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: `openat` returned a fresh owned descriptor.
        Ok(unsafe { fs::File::from_raw_fd(descriptor) })
    }

    fn directory_names(
        directory: &fs::File,
        budget: &mut NativeInventoryBudget,
    ) -> Result<Vec<std::ffi::OsString>, NativeWindowProofError> {
        struct Directory(*mut libc::DIR);
        impl Drop for Directory {
            fn drop(&mut self) {
                // SAFETY: `fdopendir` returned this owned directory stream and it is closed once.
                unsafe { libc::closedir(self.0) };
            }
        }

        #[cfg(target_os = "macos")]
        fn errno_pointer() -> *mut libc::c_int {
            unsafe { libc::__error() }
        }
        #[cfg(not(target_os = "macos"))]
        fn errno_pointer() -> *mut libc::c_int {
            unsafe { libc::__errno_location() }
        }

        // `fdopendir` owns its descriptor, so duplicate the exact no-follow directory descriptor.
        // SAFETY: `directory` holds a valid descriptor for the duration of this call.
        let duplicated = unsafe { libc::dup(directory.as_raw_fd()) };
        if duplicated < 0 {
            return Err(NativeWindowProofError::Receipt);
        }
        // SAFETY: `duplicated` is an owned directory descriptor. On failure, close it below.
        let stream = unsafe { libc::fdopendir(duplicated) };
        if stream.is_null() {
            // SAFETY: `fdopendir` did not take ownership after returning null.
            unsafe { libc::close(duplicated) };
            return Err(NativeWindowProofError::Receipt);
        }
        let stream = Directory(stream);
        let mut names = Vec::new();
        loop {
            // POSIX requires clearing errno before `readdir` to distinguish EOF from failure.
            unsafe { *errno_pointer() = 0 };
            // SAFETY: the directory stream remains valid and exclusively used by this loop.
            let entry = unsafe { libc::readdir(stream.0) };
            if entry.is_null() {
                if unsafe { *errno_pointer() } != 0 {
                    return Err(NativeWindowProofError::Receipt);
                }
                break;
            }
            // SAFETY: `d_name` is NUL-terminated for the lifetime of the returned directory entry.
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            budget.admit_name(name.len())?;
            names.push(std::ffi::OsString::from_vec(name.to_vec()));
        }
        names.sort();
        Ok(names)
    }

    let directory = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(root)
        .map_err(|_| NativeWindowProofError::Receipt)?;
    let mut pending = vec![(directory, PathBuf::new())];
    while let Some((directory, relative_directory)) = pending.pop() {
        let names = directory_names(&directory, budget)?;
        for name in names {
            let mut opened =
                open_at(&directory, &name).map_err(|_| NativeWindowProofError::Receipt)?;
            let metadata = opened
                .metadata()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            let relative = relative_directory.join(&name);
            budget.admit_path(relative.as_os_str().as_bytes().len())?;
            if metadata.is_dir() {
                pending.push((opened, relative));
                continue;
            }
            if !metadata.is_file() {
                return Err(NativeWindowProofError::Receipt);
            }
            let relative_path = relative
                .to_str()
                .filter(|value| is_normal_relative_path(value))
                .ok_or(NativeWindowProofError::Receipt)?;
            budget.admit_file(metadata.len())?;
            if relative_path == excluded_manifest {
                continue;
            }
            if relative_path == sibling_manifest {
                return Err(NativeWindowProofError::Receipt);
            }
            let sha256 =
                digest_exact_bounded(&mut opened, metadata.len(), budget.limits.member_bytes)?;
            files.push(NativeRetainedInput {
                relative_path: relative_path.to_string(),
                byte_length: metadata.len(),
                sha256,
            });
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn collect_inventory(
    root: &Path,
    excluded_manifest: &str,
    sibling_manifest: &str,
    files: &mut Vec<NativeRetainedInput>,
    budget: &mut NativeInventoryBudget,
) -> Result<(), NativeWindowProofError> {
    fn collect(
        root: &Path,
        directory: &Path,
        excluded_manifest: &str,
        sibling_manifest: &str,
        files: &mut Vec<NativeRetainedInput>,
        budget: &mut NativeInventoryBudget,
    ) -> Result<(), NativeWindowProofError> {
        let mut paths = fs::read_dir(directory)
            .map_err(|_| NativeWindowProofError::Receipt)?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|_| NativeWindowProofError::Receipt)
            })
            .collect::<Result<Vec<_>, _>>()?;
        paths.sort();
        for path in paths {
            let name_bytes = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(NativeWindowProofError::Receipt)?
                .len();
            budget.admit_name(name_bytes)?;
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| NativeWindowProofError::Receipt)?;
            if metadata.file_type().is_symlink() {
                return Err(NativeWindowProofError::Receipt);
            }
            if metadata.is_dir() {
                collect(
                    root,
                    &path,
                    excluded_manifest,
                    sibling_manifest,
                    files,
                    budget,
                )?;
                continue;
            }
            if !metadata.is_file() {
                return Err(NativeWindowProofError::Receipt);
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| NativeWindowProofError::Receipt)?;
            let relative_path = relative
                .to_str()
                .filter(|value| is_normal_relative_path(value))
                .ok_or(NativeWindowProofError::Receipt)?;
            budget.admit_path(relative_path.len())?;
            budget.admit_file(metadata.len())?;
            if relative_path == excluded_manifest {
                continue;
            }
            if relative_path == sibling_manifest {
                return Err(NativeWindowProofError::Receipt);
            }
            let bytes = read_relative_regular_nofollow_bounded(root, relative, metadata.len())
                .map_err(|_| NativeWindowProofError::Receipt)?;
            if bytes.len() as u64 != metadata.len() {
                return Err(NativeWindowProofError::Receipt);
            }
            files.push(NativeRetainedInput {
                relative_path: relative_path.to_string(),
                byte_length: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            });
        }
        Ok(())
    }

    let metadata = fs::symlink_metadata(root).map_err(|_| NativeWindowProofError::Receipt)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(NativeWindowProofError::Receipt);
    }
    collect(
        root,
        root,
        excluded_manifest,
        sibling_manifest,
        files,
        budget,
    )
}

fn validate_retained_input(
    root: &Path,
    input: &NativeRetainedInput,
    maximum_byte_length: u64,
) -> Result<Vec<u8>, NativeWindowProofError> {
    if !is_normal_relative_path(&input.relative_path)
        || input.byte_length == 0
        || input.byte_length > maximum_byte_length
        || !is_lower_hex(&input.sha256, 64)
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let relative = Path::new(&input.relative_path);
    let mut current = root.to_path_buf();
    let component_count = relative.components().count();
    for (position, component) in relative.components().enumerate() {
        let Component::Normal(name) = component else {
            return Err(NativeWindowProofError::Receipt);
        };
        current.push(name);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| NativeWindowProofError::Receipt)?;
        if metadata.file_type().is_symlink()
            || (position + 1 == component_count && !metadata.is_file())
            || (position + 1 < component_count && !metadata.is_dir())
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    let bytes = read_relative_regular_nofollow_bounded(root, relative, input.byte_length)
        .map_err(|_| NativeWindowProofError::Receipt)?;
    if bytes.len() as u64 != input.byte_length
        || format!("{:x}", Sha256::digest(&bytes)) != input.sha256
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(bytes)
}

fn validate_retained_native_config(
    root: &Path,
    retained_files: &[NativeRetainedInput],
    process: &NativeProcessIdentity,
    expected_bounds: Option<NativeWindowBounds>,
    runtime_contract: NativeWindowRuntimeContract,
    acceptance_policy: NativeAcceptancePolicy,
) -> Result<(), NativeWindowProofError> {
    let input = retained_files
        .iter()
        .find(|input| input.relative_path == "native-window-proof.json")
        .ok_or(NativeWindowProofError::Receipt)?;
    let bytes =
        validate_retained_input(root, input, runtime_contract.inventory_limits.member_bytes)?;
    let config: NativeWindowConfigFile =
        serde_json::from_slice(&bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    if config.schema != NATIVE_WINDOW_CONFIG_SCHEMA
        || config.nonce != process.nonce
        || expected_bounds.is_some_and(|bounds| config.client_bounds != bounds)
        || config.runtime_contract != runtime_contract
        || config.acceptance_policy != acceptance_policy
        || !config.runtime_contract.has_valid_deadline_order()
        || !config.acceptance_policy.is_valid()
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

fn retained_content_root(
    root: &Path,
    content_package: &NativeRetainedInput,
    retained_files: &[NativeRetainedInput],
    maximum_byte_length: u64,
) -> Result<PathBuf, NativeWindowProofError> {
    let package_path = Path::new(&content_package.relative_path);
    let packages = package_path
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "packages"))
        .ok_or(NativeWindowProofError::Receipt)?;
    let content_root = packages.parent().ok_or(NativeWindowProofError::Receipt)?;
    let filename = package_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(NativeWindowProofError::Receipt)?;
    let version = filename
        .strip_prefix("uqm-")
        .and_then(|value| value.strip_suffix("-content.uqm"))
        .filter(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
        .ok_or(NativeWindowProofError::Receipt)?;
    let version_path = content_root.join("version");
    let version_path = version_path
        .to_str()
        .ok_or(NativeWindowProofError::Receipt)?;
    let version_input = retained_files
        .iter()
        .find(|input| input.relative_path == version_path)
        .ok_or(NativeWindowProofError::Receipt)?;
    if validate_retained_input(root, version_input, maximum_byte_length)?
        != format!("{version}\n").as_bytes()
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(content_root.to_path_buf())
}

fn validate_native_acceptance_failure_provenance(
    root: &Path,
    manifest: &NativeAcceptanceFailureManifest,
) -> Result<(), NativeWindowProofError> {
    let maximum_byte_length = manifest.runtime_contract.inventory_limits.member_bytes;
    let command = &manifest.command;
    if command.len() != 6
        || command[0].ends_with("/cargo")
        || command[0].ends_with("/sh")
        || command[0].ends_with("/bash")
        || manifest.environment.len() != 1
        || manifest
            .environment
            .get("SDL_AUDIODRIVER")
            .map(String::as_str)
            != Some("dummy")
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let executable_bytes =
        validate_retained_input(root, &manifest.executable, maximum_byte_length)?;
    let actual_retained_files =
        native_acceptance_failure_inventory(root, manifest.runtime_contract.inventory_limits)?;
    if actual_retained_files.is_empty() || manifest.retained_files != actual_retained_files {
        return Err(NativeWindowProofError::Receipt);
    }
    validate_retained_native_config(
        root,
        &manifest.retained_files,
        &manifest.child.process,
        None,
        manifest.runtime_contract,
        manifest.acceptance_policy,
    )?;
    if format!("{:x}", Sha256::digest(&executable_bytes))
        != manifest.child.process.executable_sha256
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let script_bytes = validate_retained_input(root, &manifest.script, maximum_byte_length)?;
    validate_retained_input(root, &manifest.content_package, maximum_byte_length)?;
    let document =
        crate::automation::script::parse_script(&script_bytes, &manifest.script.relative_path)
            .map_err(|_| NativeWindowProofError::Receipt)?;
    crate::automation::script::validate_script(document, &manifest.script.relative_path)
        .map_err(|_| NativeWindowProofError::Receipt)?;
    let content_parent = retained_content_root(
        root,
        &manifest.content_package,
        &manifest.retained_files,
        maximum_byte_length,
    )?;
    let command_paths = [
        command[0].as_str(),
        command[1]
            .strip_prefix("--configdir=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[2]
            .strip_prefix("--contentdir=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[3]
            .strip_prefix("--automation-script=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[4]
            .strip_prefix("--automation-output=")
            .ok_or(NativeWindowProofError::Receipt)?,
        command[5]
            .strip_prefix("--native-window-proof=")
            .ok_or(NativeWindowProofError::Receipt)?,
    ];
    let mut recorded_root = Path::new(command_paths[0]);
    for _ in Path::new(&manifest.executable.relative_path).components() {
        recorded_root = recorded_root
            .parent()
            .ok_or(NativeWindowProofError::Receipt)?;
    }
    if command_paths
        .iter()
        .any(|path| !valid_bound_command_path(path))
        || Path::new(command_paths[0]) != recorded_root.join(&manifest.executable.relative_path)
        || Path::new(command_paths[1]) != recorded_root.join("config")
        || Path::new(command_paths[2]) != recorded_root.join(content_parent)
        || Path::new(command_paths[3]) != recorded_root.join(&manifest.script.relative_path)
        || Path::new(command_paths[4]) != recorded_root.join("automation")
        || Path::new(command_paths[5]) != recorded_root.join("native-window-proof.json")
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

pub fn native_window_trace_semantic_snapshot(
    records: &[crate::automation::trace::TraceRecord],
) -> Result<NativeWindowSemanticSnapshot, NativeWindowProofError> {
    let mut accepted_player_inputs = 0_u64;
    let mut verified_battle_frames = 0_u64;
    let mut index = 0;
    while let Some(record) = records.get(index) {
        let Some(label) = record.label.as_deref() else {
            index += 1;
            continue;
        };
        if label.starts_with("player_input_observed:") {
            let observed = parse_player_input_observed(record, label)?;
            let accepted = records
                .get(index + 1)
                .ok_or(NativeWindowProofError::Receipt)?;
            let accepted_label = accepted
                .label
                .as_deref()
                .ok_or(NativeWindowProofError::Receipt)?;
            let (accepted_key, accepted_value) = parse_player_input(accepted, accepted_label)?;
            if observed.0 != accepted_key
                || observed.1 != accepted_value
                || observed.1 != observed.2
                || (observed.1 == 0 && observed.3 != 0)
            {
                return Err(NativeWindowProofError::Receipt);
            }
            accepted_player_inputs = accepted_player_inputs
                .checked_add(u64::from(observed.3))
                .ok_or(NativeWindowProofError::Receipt)?;
            index += 2;
            continue;
        }
        if label.starts_with("player_input") {
            return Err(NativeWindowProofError::Receipt);
        }
        if label.starts_with("battle_frames_reached:") {
            let values = label
                .strip_prefix("battle_frames_reached:count=")
                .ok_or(NativeWindowProofError::Receipt)?;
            let (actual_value, minimum_value) = values
                .split_once(":minimum=")
                .ok_or(NativeWindowProofError::Receipt)?;
            let actual = actual_value
                .parse::<u64>()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            let minimum = minimum_value
                .parse::<u64>()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            if record.kind != crate::automation::trace::RecordKind::SemanticAssertion
                || actual.to_string() != actual_value
                || minimum.to_string() != minimum_value
                || minimum == 0
                || actual < minimum
            {
                return Err(NativeWindowProofError::Receipt);
            }
            verified_battle_frames = verified_battle_frames.max(actual);
        }
        if label.starts_with("battle_frames_verified:") {
            let value = label
                .strip_prefix("battle_frames_verified:count=")
                .ok_or(NativeWindowProofError::Receipt)?;
            let parsed = value
                .parse::<u64>()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            if record.kind != crate::automation::trace::RecordKind::SemanticAssertion
                || parsed.to_string() != value
            {
                return Err(NativeWindowProofError::Receipt);
            }
            verified_battle_frames = verified_battle_frames.max(parsed);
        }
        index += 1;
    }
    Ok(NativeWindowSemanticSnapshot {
        trace_record_count: records
            .len()
            .try_into()
            .map_err(|_| NativeWindowProofError::Receipt)?,
        accepted_player_inputs,
        verified_battle_frames,
    })
}

fn parse_player_input_observed<'a>(
    record: &crate::automation::trace::TraceRecord,
    label: &'a str,
) -> Result<(&'a str, u8, u8, u8), NativeWindowProofError> {
    let fields = label
        .strip_prefix("player_input_observed:key=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (key, fields) = fields
        .split_once(":intended=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (intended, fields) = fields
        .split_once(":current=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (current, pulsed) = fields
        .split_once(":pulsed=")
        .ok_or(NativeWindowProofError::Receipt)?;
    if record.kind != crate::automation::trace::RecordKind::SemanticAssertion
        || !valid_player_key(key)
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok((
        key,
        parse_binary_field(intended)?,
        parse_binary_field(current)?,
        parse_binary_field(pulsed)?,
    ))
}

fn parse_player_input<'a>(
    record: &crate::automation::trace::TraceRecord,
    label: &'a str,
) -> Result<(&'a str, u8), NativeWindowProofError> {
    let fields = label
        .strip_prefix("player_input:key=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (key, value) = fields
        .split_once(":value=")
        .ok_or(NativeWindowProofError::Receipt)?;
    if record.kind != crate::automation::trace::RecordKind::SemanticAssertion
        || !valid_player_key(key)
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok((key, parse_binary_field(value)?))
}

fn parse_binary_field(value: &str) -> Result<u8, NativeWindowProofError> {
    match value {
        "0" => Ok(0),
        "1" => Ok(1),
        _ => Err(NativeWindowProofError::Receipt),
    }
}

fn valid_player_key(key: &str) -> bool {
    matches!(
        key,
        "Thrust" | "Down" | "Left" | "Right" | "Weapon" | "Special" | "Escape"
    )
}

pub fn validate_native_acceptance_setup_failure_bundle(
    root: &Path,
    manifest: &NativeAcceptanceSetupFailureManifest,
) -> Result<(), NativeWindowProofError> {
    if manifest.schema != NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA
        || manifest.passed
        || manifest.command.is_empty()
        || manifest.expected_executable_byte_length == 0
        || !is_lower_hex(&manifest.expected_executable_sha256, 64)
        || !manifest.runtime_contract.has_valid_deadline_order()
        || !manifest.acceptance_policy.is_valid()
        || manifest.error.trim().is_empty()
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let retained =
        native_acceptance_failure_inventory(root, manifest.runtime_contract.inventory_limits)?;
    if retained != manifest.retained_files {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

pub fn validate_native_acceptance_failure_bundle(
    root: &Path,
    manifest: &NativeAcceptanceFailureManifest,
) -> Result<(), NativeWindowProofError> {
    let child_terminal = matches!(
        (manifest.child.exit_code, manifest.child.signal),
        (Some(0..=255), None) | (None, Some(1..=i32::MAX))
    );
    let outcome_matches_contract = match manifest.failure_contract {
        NativeAcceptanceFailureContract::ChildExit => matches!(
            (manifest.child.exit_code, manifest.child.signal),
            (Some(1..=255), None) | (None, Some(1..=i32::MAX))
        ),
        NativeAcceptanceFailureContract::Observer
        | NativeAcceptanceFailureContract::ChildSupervision => child_terminal,
        NativeAcceptanceFailureContract::ConfigCleanup
        | NativeAcceptanceFailureContract::MaterializedContentCleanup
        | NativeAcceptanceFailureContract::Semantic => {
            manifest.child.exit_code == Some(0)
                && manifest.child.signal.is_none()
                && !manifest.child.term_sent
                && !manifest.child.kill_sent
        }
    };
    let config_cleanup_is_truthful = manifest.child.config_root_removed
        || (manifest.failure_contract == NativeAcceptanceFailureContract::ConfigCleanup
            && manifest
                .retained_files
                .iter()
                .any(|file| file.relative_path.starts_with("config/")));
    let content_cleanup_is_truthful = manifest.child.materialized_content_removed
        || manifest.failure_contract == NativeAcceptanceFailureContract::MaterializedContentCleanup;
    if manifest.schema != NATIVE_ACCEPTANCE_FAILURE_SCHEMA
        || manifest.passed
        || !outcome_matches_contract
        || manifest.error.is_empty()
        || !child_terminal
        || !manifest.child.output_drained
        || !manifest.child.initial_process_group_empty
        || !config_cleanup_is_truthful
        || !content_cleanup_is_truthful
        || !manifest.runtime_contract.has_valid_deadline_order()
        || !manifest.acceptance_policy.is_valid()
        || validate_process(&manifest.child.process).is_err()
    {
        return Err(NativeWindowProofError::Receipt);
    }
    validate_native_acceptance_failure_provenance(root, manifest)
}

fn recording_inventory_consistency(
    root: &Path,
    manifest: &NativeAcceptanceManifest,
) -> Result<Vec<u8>, NativeWindowProofError> {
    let maximum_byte_length = manifest.runtime_contract.inventory_limits.member_bytes;
    let executable = validate_retained_input(root, &manifest.executable, maximum_byte_length)?;
    let retained_files =
        native_acceptance_inventory(root, manifest.runtime_contract.inventory_limits)?;
    if retained_files.is_empty() || manifest.retained_files != retained_files {
        return Err(NativeWindowProofError::Receipt);
    }
    validate_retained_native_config(
        root,
        &manifest.retained_files,
        &manifest.child.process,
        Some(manifest.window.binding.client_bounds),
        manifest.runtime_contract,
        manifest.acceptance_policy,
    )?;
    if format!("{:x}", Sha256::digest(&executable)) != manifest.child.process.executable_sha256 {
        return Err(NativeWindowProofError::Receipt);
    }
    let script = validate_retained_input(root, &manifest.script, maximum_byte_length)?;
    validate_retained_input(root, &manifest.content_package, maximum_byte_length)?;
    let document = crate::automation::script::parse_script(&script, &manifest.script.relative_path)
        .map_err(|_| NativeWindowProofError::Receipt)?;
    crate::automation::script::validate_script(document, &manifest.script.relative_path)
        .map_err(|_| NativeWindowProofError::Receipt)?;
    Ok(executable)
}

fn recorded_provenance_consistency(
    root: &Path,
    manifest: &NativeAcceptanceManifest,
) -> Result<(), NativeWindowProofError> {
    let content_parent = retained_content_root(
        root,
        &manifest.content_package,
        &manifest.retained_files,
        manifest.runtime_contract.inventory_limits.member_bytes,
    )?;
    let command_paths = [
        manifest.command[0].as_str(),
        manifest.command[1]
            .strip_prefix("--configdir=")
            .ok_or(NativeWindowProofError::Receipt)?,
        manifest.command[2]
            .strip_prefix("--contentdir=")
            .ok_or(NativeWindowProofError::Receipt)?,
        manifest.command[3]
            .strip_prefix("--automation-script=")
            .ok_or(NativeWindowProofError::Receipt)?,
        manifest.command[4]
            .strip_prefix("--automation-output=")
            .ok_or(NativeWindowProofError::Receipt)?,
        manifest.command[5]
            .strip_prefix("--native-window-proof=")
            .ok_or(NativeWindowProofError::Receipt)?,
    ];
    let mut recorded_root = Path::new(command_paths[0]);
    for _ in Path::new(&manifest.executable.relative_path).components() {
        recorded_root = recorded_root
            .parent()
            .ok_or(NativeWindowProofError::Receipt)?;
    }
    if command_paths
        .iter()
        .any(|path| !valid_bound_command_path(path))
        || Path::new(command_paths[0]) != recorded_root.join(&manifest.executable.relative_path)
        || Path::new(command_paths[1]) != recorded_root.join("config")
        || Path::new(command_paths[2]) != recorded_root.join(content_parent)
        || Path::new(command_paths[3]) != recorded_root.join(&manifest.script.relative_path)
        || Path::new(command_paths[4]) != recorded_root.join("automation")
        || Path::new(command_paths[5]) != recorded_root.join("native-window-proof.json")
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

fn child_terminal_publications(
    manifest: &NativeAcceptanceManifest,
) -> Result<(), NativeWindowProofError> {
    if manifest.publications.is_empty() {
        return Err(NativeWindowProofError::Receipt);
    }
    let mut previous_publication: Option<&NativeWindowPublication> = None;
    for publication in &manifest.publications {
        let state = &publication.state;
        let acknowledgement = &publication.acknowledgement;
        if state.schema != NATIVE_WINDOW_STATE_SCHEMA
            || state.nonce != manifest.child.process.nonce
            || state.pid != manifest.child.process.pid
            || !state.shown
            || state.requested_client_bounds != manifest.window.binding.client_bounds
            || state.client_bounds.width == 0
            || state.client_bounds.height == 0
            || acknowledgement.schema != NATIVE_WINDOW_ACK_SCHEMA
            || acknowledgement.nonce != state.nonce
            || acknowledgement.committed_presentation != state.committed_presentation
        {
            return Err(NativeWindowProofError::Receipt);
        }
        if let Some(previous) = previous_publication {
            if previous.state.committed_presentation.checked_add(1)
                != Some(state.committed_presentation)
                || state.semantic.trace_record_count < previous.state.semantic.trace_record_count
                || state.semantic.accepted_player_inputs
                    < previous.state.semantic.accepted_player_inputs
                || state.semantic.verified_battle_frames
                    < previous.state.semantic.verified_battle_frames
            {
                return Err(NativeWindowProofError::Receipt);
            }
        }
        previous_publication = Some(publication);
    }
    Ok(())
}

fn publication_observation_correlation(
    manifest: &NativeAcceptanceManifest,
) -> Result<(), NativeWindowProofError> {
    for publication in &manifest.publications {
        let matching: Vec<_> = manifest
            .window
            .observations
            .iter()
            .filter(|observation| {
                publication.state.committed_presentation == observation.committed_presentation
            })
            .collect();
        if publication.state.committed_presentation < manifest.window.first_visible_presentation {
            if !matching.is_empty() {
                return Err(NativeWindowProofError::Receipt);
            }
        } else if matching.len() > 1
            || (matching.len() == 1
                && (publication.state.client_bounds != matching[0].binding.client_bounds
                    || publication.state.semantic != matching[0].semantic))
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    for observation in &manifest.window.observations {
        let matching: Vec<_> = manifest
            .publications
            .iter()
            .filter(|publication| {
                publication.state.committed_presentation == observation.committed_presentation
            })
            .collect();
        if matching.len() != 1
            || matching[0].state.client_bounds != observation.binding.client_bounds
            || matching[0].state.semantic != observation.semantic
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    for screenshot in &manifest.window.screenshots {
        let publications: Vec<_> = manifest
            .publications
            .iter()
            .filter(|publication| {
                publication.state.committed_presentation == screenshot.committed_presentation
            })
            .collect();
        if publications.len() != 1
            || publications[0].state.semantic != screenshot.post_capture_observation.semantic
            || publications[0].state.client_bounds
                != screenshot.post_capture_observation.binding.client_bounds
            || publications[0].acknowledgement.committed_presentation
                != screenshot.post_capture_observation.committed_presentation
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    let playable = manifest
        .window
        .screenshots
        .iter()
        .find(|screenshot| screenshot.stage == NativeScreenshotStage::Playable)
        .ok_or(NativeWindowProofError::Receipt)?;
    let first_eligible = manifest.publications.iter().find(|publication| {
        publication
            .state
            .committed_presentation
            .checked_sub(manifest.window.first_visible_presentation)
            .is_some_and(|post_visible| {
                post_visible >= manifest.acceptance_policy.playable_presentation_floor
                    && publication.state.semantic.accepted_player_inputs > 0
                    && publication.state.semantic.verified_battle_frames
                        >= manifest.acceptance_policy.battle_frame_floor
            })
    });
    if first_eligible.map(|publication| publication.state.committed_presentation)
        != Some(playable.committed_presentation)
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

fn trace_recording_consistency(
    root: &Path,
    manifest: &NativeAcceptanceManifest,
) -> Result<Vec<crate::automation::trace::TraceRecord>, NativeWindowProofError> {
    let bytes = read_relative_regular_nofollow_bounded(
        root,
        Path::new(&manifest.trace_path),
        manifest.runtime_contract.inventory_limits.member_bytes,
    )
    .map_err(|_| NativeWindowProofError::Receipt)?;
    if bytes.is_empty()
        || !bytes.ends_with(b"\n")
        || bytes.len() as u64 != manifest.trace_byte_length
        || format!("{:x}", Sha256::digest(&bytes)) != manifest.trace_sha256
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let mut records = Vec::new();
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let record: crate::automation::trace::TraceRecord =
            serde_json::from_slice(line).map_err(|_| NativeWindowProofError::Receipt)?;
        if record.schema != crate::automation::trace::TraceRecord::SCHEMA
            || record.sequence != records.len() as u64
        {
            return Err(NativeWindowProofError::Receipt);
        }
        records.push(record);
    }
    if records.first().map(|record| &record.kind)
        != Some(&crate::automation::trace::RecordKind::RunStart)
        || records.last().map(|record| &record.kind)
            != Some(&crate::automation::trace::RecordKind::RunEnd)
        || records
            .last()
            .and_then(|record| record.terminal_reason.as_deref())
            != Some("success")
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(records)
}

fn child_terminal_recording_state(
    root: &Path,
    manifest: &NativeAcceptanceManifest,
) -> Result<(), NativeWindowProofError> {
    let final_observation = manifest
        .window
        .observations
        .last()
        .ok_or(NativeWindowProofError::Receipt)?;
    let state = read_native_window_state(
        &root.join("automation/native-window-state.json"),
        &manifest.window.binding.process.nonce,
        manifest.window.binding.process.pid,
        manifest.window.binding.client_bounds,
    )
    .map_err(|_| NativeWindowProofError::Receipt)?;
    if state.committed_presentation < final_observation.committed_presentation
        || state.semantic.trace_record_count < final_observation.semantic.trace_record_count
        || state.semantic.accepted_player_inputs < final_observation.semantic.accepted_player_inputs
        || state.semantic.verified_battle_frames < final_observation.semantic.verified_battle_frames
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let ack_bytes = validate_retained_input(
        root,
        &NativeRetainedInput {
            relative_path: "automation/native-window-ack.json".to_string(),
            byte_length: manifest
                .retained_files
                .iter()
                .find(|file| file.relative_path == "automation/native-window-ack.json")
                .map(|file| file.byte_length)
                .ok_or(NativeWindowProofError::Receipt)?,
            sha256: manifest
                .retained_files
                .iter()
                .find(|file| file.relative_path == "automation/native-window-ack.json")
                .map(|file| file.sha256.clone())
                .ok_or(NativeWindowProofError::Receipt)?,
        },
        manifest.runtime_contract.inventory_limits.member_bytes,
    )?
    .to_owned();
    let ack: NativeWindowAck =
        serde_json::from_slice(&ack_bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    if ack.schema != NATIVE_WINDOW_ACK_SCHEMA
        || ack.nonce != manifest.window.binding.process.nonce
        || ack.committed_presentation != state.committed_presentation
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let final_publication = manifest
        .publications
        .last()
        .ok_or(NativeWindowProofError::Receipt)?;
    if final_publication.state != state || final_publication.acknowledgement != ack {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

fn valid_acceptance_command_environment(manifest: &NativeAcceptanceManifest) -> bool {
    manifest.command.len() == 6
        && !manifest.command[0].ends_with("/cargo")
        && !manifest.command[0].ends_with("/sh")
        && !manifest.command[0].ends_with("/bash")
        && manifest.environment.len() == 1
        && manifest
            .environment
            .get("SDL_AUDIODRIVER")
            .map(String::as_str)
            == Some("dummy")
}

fn valid_acceptance_child_contract(manifest: &NativeAcceptanceManifest) -> bool {
    manifest.child.exit_code == Some(0)
        && manifest.child.signal.is_none()
        && !manifest.child.term_sent
        && !manifest.child.kill_sent
        && manifest.child.output_drained
        && manifest.child.initial_process_group_empty
        && manifest.child.config_root_removed
        && manifest.child.materialized_content_removed
        && manifest.child.process == manifest.window.binding.process
}

fn validate_recorded_semantics(
    manifest: &NativeAcceptanceManifest,
    records: &[crate::automation::trace::TraceRecord],
) -> Result<(), NativeWindowProofError> {
    for publication in &manifest.publications {
        let count: usize = publication
            .state
            .semantic
            .trace_record_count
            .try_into()
            .map_err(|_| NativeWindowProofError::Receipt)?;
        if count > records.len()
            || native_window_trace_semantic_snapshot(&records[..count])?
                != publication.state.semantic
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    for observation in &manifest.window.observations {
        let count: usize = observation
            .semantic
            .trace_record_count
            .try_into()
            .map_err(|_| NativeWindowProofError::Receipt)?;
        if count > records.len()
            || native_window_trace_semantic_snapshot(&records[..count])? != observation.semantic
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    let final_semantic = native_window_trace_semantic_snapshot(records)?;
    if final_semantic.accepted_player_inputs != manifest.window.input_events
        || final_semantic.verified_battle_frames != manifest.window.battle_frames
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

fn is_normalized_absolute_path(path: &str) -> bool {
    let path = Path::new(path);
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}

fn validate_linked_build_receipt_bundle(
    root: &Path,
    manifest: &NativeAcceptanceManifest,
) -> Result<(), NativeWindowProofError> {
    let limit = manifest.runtime_contract.inventory_limits.member_bytes;
    let receipt_input = manifest
        .retained_files
        .iter()
        .find(|input| input.relative_path == "inputs/linked-build/linked-build-receipt.json")
        .ok_or(NativeWindowProofError::Receipt)?;
    let receipt_bytes = validate_retained_input(root, receipt_input, limit)?;
    let receipt: NativeLinkedBuildReceipt =
        serde_json::from_slice(&receipt_bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    if receipt.schema != NATIVE_LINKED_BUILD_RECEIPT_SCHEMA
        || !is_lower_hex(&receipt.source_sha, 40)
        || receipt.native_profile != "linked-test"
        || receipt.feature != "audio_heart,debug-process,linked_c_archive"
        || receipt.executable != manifest.executable
        || receipt
            .cargo_command
            .first()
            .is_none_or(|path| !is_normalized_absolute_path(path))
        || !is_normalized_absolute_path(&receipt.cargo_executable_path)
        || !is_normalized_absolute_path(&receipt.cargo_rust_archive_path)
        || !is_normalized_absolute_path(&receipt.cargo_out_dir)
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let required_arguments = [
        "build",
        "--locked",
        "--manifest-path",
        "rust/Cargo.toml",
        "--release",
        "--no-default-features",
        "--features",
        "audio_heart,debug-process,linked_c_archive",
        "--bin",
        "uqm",
        "--message-format=json-render-diagnostics",
    ];
    let arguments = receipt
        .cargo_command
        .get(1..)
        .ok_or(NativeWindowProofError::Receipt)?;
    if arguments != required_arguments
        && !(arguments.len() == required_arguments.len() + 2
            && arguments[..required_arguments.len()] == required_arguments
            && arguments[required_arguments.len()] == "--target-dir"
            && is_normalized_absolute_path(&arguments[required_arguments.len() + 1]))
    {
        return Err(NativeWindowProofError::Receipt);
    }
    let members = [
        ("inputs/uqm", &receipt.executable),
        (
            "inputs/linked-build/cargo-messages.jsonl",
            &receipt.cargo_messages,
        ),
        ("inputs/linked-build/rust-archive.a", &receipt.rust_archive),
        ("inputs/linked-build/c-archive.a", &receipt.c_archive),
        (
            "inputs/linked-build/object-sidecar.manifest",
            &receipt.object_sidecar,
        ),
        (
            "inputs/linked-build/provider-report.json",
            &receipt.provider_report,
        ),
        (
            "inputs/linked-build/native-build-evidence.json",
            &receipt.native_build_evidence,
        ),
        ("inputs/linked-build/Cargo.toml", &receipt.cargo_manifest),
        ("inputs/linked-build/Cargo.lock", &receipt.cargo_lock),
        ("inputs/linked-build/gates.json", &receipt.authority),
        (
            "inputs/linked-build/canonical-toolchain.json",
            &receipt.canonical_toolchain,
        ),
    ];
    for (path, expected) in members {
        if expected.relative_path != path
            || manifest
                .retained_files
                .iter()
                .find(|input| input.relative_path == path)
                != Some(expected)
        {
            return Err(NativeWindowProofError::Receipt);
        }
    }
    let cargo_messages = validate_retained_input(root, &receipt.cargo_messages, limit)?;
    for input in [
        &receipt.executable,
        &receipt.rust_archive,
        &receipt.c_archive,
        &receipt.object_sidecar,
    ] {
        validate_retained_input(root, input, limit)?;
    }
    validate_linked_cargo_messages(&receipt, &cargo_messages)?;
    let provider = validate_retained_input(root, &receipt.provider_report, limit)?;
    let evidence = validate_retained_input(root, &receipt.native_build_evidence, limit)?;
    let cargo_manifest = validate_retained_input(root, &receipt.cargo_manifest, limit)?;
    let cargo_lock = validate_retained_input(root, &receipt.cargo_lock, limit)?;
    let authority = validate_retained_input(root, &receipt.authority, limit)?;
    let canonical_toolchain = validate_retained_input(root, &receipt.canonical_toolchain, limit)?;
    validate_native_linked_build_semantics(
        &provider,
        &evidence,
        &authority,
        &canonical_toolchain,
        &cargo_manifest,
        &cargo_lock,
    )
}

fn validate_linked_cargo_messages(
    receipt: &NativeLinkedBuildReceipt,
    bytes: &[u8],
) -> Result<(), NativeWindowProofError> {
    let text = std::str::from_utf8(bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    let messages: Vec<serde_json::Value> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).map_err(|_| NativeWindowProofError::Receipt))
        .collect::<Result<_, _>>()?;
    let package_ids: BTreeSet<_> = messages
        .iter()
        .filter(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == "uqm"
                && cargo_target_has_kind(message, "bin")
                && message["executable"].as_str() == Some(receipt.cargo_executable_path.as_str())
        })
        .filter_map(|message| message["package_id"].as_str().map(str::to_string))
        .collect();
    if package_ids.len() != 1 {
        return Err(NativeWindowProofError::Receipt);
    }
    let package_id = package_ids.into_iter().next().expect("length checked");
    let mut executables = BTreeSet::new();
    let mut rust_archives = BTreeSet::new();
    let mut out_dirs = BTreeSet::new();
    let mut build_finished = Vec::new();
    for message in &messages {
        match message["reason"].as_str() {
            Some("compiler-artifact")
                if message["package_id"].as_str() == Some(package_id.as_str())
                    && message["target"]["name"] == "uqm"
                    && cargo_target_has_kind(message, "bin") =>
            {
                if let Some(path) = message["executable"].as_str() {
                    executables.insert(path.to_string());
                }
            }
            Some("compiler-artifact")
                if message["package_id"].as_str() == Some(package_id.as_str())
                    && message["target"]["name"] == "uqm_rust"
                    && cargo_target_has_kind(message, "staticlib") =>
            {
                for path in message["filenames"].as_array().into_iter().flatten() {
                    let Some(path) = path.as_str() else { continue };
                    let filename = Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default();
                    if filename.starts_with("libuqm_rust-") && filename.ends_with(".a") {
                        rust_archives.insert(path.to_string());
                    }
                }
            }
            Some("build-script-executed")
                if message["package_id"].as_str() == Some(package_id.as_str()) =>
            {
                if let Some(path) = message["out_dir"].as_str() {
                    out_dirs.insert(path.to_string());
                }
            }
            Some("build-finished") => build_finished.push(message["success"] == true),
            _ => {}
        }
    }
    if executables
        != [receipt.cargo_executable_path.clone()]
            .into_iter()
            .collect()
        || rust_archives
            != [receipt.cargo_rust_archive_path.clone()]
                .into_iter()
                .collect()
        || out_dirs != [receipt.cargo_out_dir.clone()].into_iter().collect()
        || build_finished != [true]
    {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok(())
}

fn cargo_target_has_kind(message: &serde_json::Value, expected: &str) -> bool {
    message["target"]["kind"]
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|kind| kind == expected))
}

pub fn validate_native_linked_build_semantics(
    provider_bytes: &[u8],
    evidence_bytes: &[u8],
    authority_bytes: &[u8],
    canonical_toolchain_bytes: &[u8],
    cargo_manifest_bytes: &[u8],
    cargo_lock_bytes: &[u8],
) -> Result<(), NativeWindowProofError> {
    let provider: LinkedProviderReport =
        serde_json::from_slice(provider_bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    let evidence: LinkedNativeBuildEvidence =
        serde_json::from_slice(evidence_bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    let authority: serde_json::Value =
        serde_json::from_slice(authority_bytes).map_err(|_| NativeWindowProofError::Receipt)?;
    let canonical_toolchain: LinkedToolchainIdentity =
        serde_json::from_slice(canonical_toolchain_bytes)
            .map_err(|_| NativeWindowProofError::Receipt)?;
    let authority_ledger = authority
        .pointer("/ledger_identity/sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or(NativeWindowProofError::Receipt)?;
    let rust_prefix = authority
        .pointer("/tools/rust/expected_output_prefix")
        .and_then(serde_json::Value::as_str)
        .ok_or(NativeWindowProofError::Receipt)?;
    let cargo_manifest_sha256 = authority
        .pointer("/package/cargo_manifest_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or(NativeWindowProofError::Receipt)?;
    let cargo_lock_sha256 = authority
        .pointer("/package/cargo_lock_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or(NativeWindowProofError::Receipt)?;
    if authority.get("schema").and_then(serde_json::Value::as_str) == Some("uqm-s4-ci-authority-v1")
        && provider.ledger_sha256 == authority_ledger
        && evidence.toolchain == canonical_toolchain
        && evidence.toolchain.rustc.version.starts_with(rust_prefix)
        && format!("{:x}", Sha256::digest(cargo_manifest_bytes)) == cargo_manifest_sha256
        && format!("{:x}", Sha256::digest(cargo_lock_bytes)) == cargo_lock_sha256
        && valid_linked_provider_report(&provider)
        && valid_linked_build_evidence(&evidence)
    {
        Ok(())
    } else {
        Err(NativeWindowProofError::Receipt)
    }
}

fn valid_linked_provider_report(provider: &LinkedProviderReport) -> bool {
    let summary = &provider.summary;
    provider.schema == "uqm-provider-report-v1"
        && provider.ledger_sha256.len() == 64
        && provider
            .ledger_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && provider.tracked_native_file_delta == 0
        && provider.diagnostics.is_empty()
        && summary.passed
        && summary.violations == 0
        && summary.total_objects == provider.entries.len()
        && summary.included + summary.excluded == summary.total_objects
        && summary.duplicate_providers_excluded + summary.recompiled + summary.replaced
            == summary.excluded
        && provider.entries.iter().all(|entry| {
            !entry.path.is_empty()
                && !entry.issue.is_empty()
                && !entry.provider.is_empty()
                && entry.archive_decision.is_string()
                && entry.status == "ok"
                && entry.diagnostics.is_empty()
        })
        && provider.symbols.iter().all(|symbol| {
            !symbol.symbol.is_empty()
                && !symbol.canonical_owner.is_empty()
                && symbol.provider_kind.is_string()
                && !symbol.provider_path.is_empty()
                && symbol
                    .excluded_provider_paths
                    .iter()
                    .all(|path| !path.is_empty())
        })
}

fn linked_build_targets_correlate(native_target: &str, rust_target: &str) -> bool {
    matches!(
        (native_target, rust_target),
        ("macos-aarch64", "aarch64-apple-darwin")
            | ("macos-x86_64", "x86_64-apple-darwin")
            | ("linux-aarch64", "aarch64-unknown-linux-gnu")
            | ("linux-x86_64", "x86_64-unknown-linux-gnu")
    )
}

fn valid_linked_build_evidence(evidence: &LinkedNativeBuildEvidence) -> bool {
    let expected = ["audio_heart", "debug-process", "linked_c_archive"];
    let tools = [
        &evidence.toolchain.rustc,
        &evidence.toolchain.cargo,
        &evidence.toolchain.cc,
        &evidence.toolchain.ar,
        &evidence.toolchain.nm,
        &evidence.toolchain.pkg_config,
        &evidence.toolchain.linker,
    ];
    evidence.schema == NATIVE_BUILD_EVIDENCE_SCHEMA
        && evidence.source_date_epoch > 0
        && !evidence.build_date.is_empty()
        && evidence
            .active_features
            .iter()
            .map(String::as_str)
            .eq(expected)
        && !evidence.target.is_empty()
        && linked_build_targets_correlate(&evidence.target, &evidence.toolchain.target)
        && tools.iter().all(|tool| {
            !tool.executable.is_empty()
                && !tool.version.is_empty()
                && !tool.sha256.is_empty()
                && tool.effective_args.iter().all(|arg| !arg.is_empty())
        })
        && !evidence.packages.is_empty()
        && evidence.packages.iter().all(|package| {
            !package.name.is_empty()
                && !package.version.is_empty()
                && package.cflags.iter().all(|flag| !flag.is_empty())
                && package.libs.iter().all(|flag| !flag.is_empty())
        })
        && evidence.compile_profile.target == evidence.target
        && !evidence.compile_profile.compiler.is_empty()
        && evidence
            .compile_profile
            .ordered_defines
            .iter()
            .any(|define| define == "-DDEBUG")
        && evidence
            .compile_profile
            .ordered_include_roots
            .iter()
            .all(|root| !root.is_empty())
        && evidence
            .compile_profile
            .ordered_compile_flags
            .iter()
            .all(|flag| !flag.is_empty())
        && evidence.compile_profile.dependency_flags == ["-MMD", "-MF", "<depfile>"]
        && !evidence.compile_profile.command_template.is_empty()
        && !evidence.build_environment.is_empty()
}

pub fn validate_native_acceptance_bundle(
    root: &Path,
    manifest: &NativeAcceptanceManifest,
) -> Result<(), NativeWindowProofError> {
    if manifest.schema != NATIVE_ACCEPTANCE_SCHEMA
        || !manifest.passed
        || manifest.execution_identity.real_uid == 0
        || manifest.execution_identity.real_uid != manifest.execution_identity.effective_uid
        || manifest.execution_identity.real_uid != manifest.execution_identity.launchd_manager_uid
        || manifest.execution_identity.launchd_manager_name != "Aqua"
        || !valid_acceptance_command_environment(manifest)
        || manifest.trace_path != "automation/trace.jsonl"
        || !valid_acceptance_child_contract(manifest)
        || !manifest.runtime_contract.has_valid_deadline_order()
        || !manifest.acceptance_policy.is_valid()
        || manifest.acceptance_policy != manifest.window.acceptance_policy
    {
        return Err(NativeWindowProofError::Receipt);
    }
    recording_inventory_consistency(root, manifest)?;
    validate_linked_build_receipt_bundle(root, manifest)?;
    recorded_provenance_consistency(root, manifest)?;
    validate_native_window_bundle(
        root,
        &manifest.window,
        manifest.runtime_contract.inventory_limits,
    )?;
    child_terminal_publications(manifest)?;
    publication_observation_correlation(manifest)?;
    child_terminal_recording_state(root, manifest)?;
    let records = trace_recording_consistency(root, manifest)?;
    validate_recorded_semantics(manifest, &records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_start_identity_is_normalized_positive_decimal() {
        for valid in ["1", "1234", "18446744073709551616"] {
            assert!(normalized_positive_process_start(valid));
        }
        for invalid in ["", "0", "01", "+1", "-1", "1.0", " 1"] {
            assert!(!normalized_positive_process_start(invalid));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn darwin_observer_selects_one_visible_window_among_hidden_and_duplicate_entries() {
        let visible = ObservedNativeWindow {
            window_id: 41,
            os_bounds: NativeWindowBounds {
                x: 80,
                y: 48,
                width: 1280,
                height: 992,
            },
            visible: true,
            minimized: false,
        };
        let hidden = ObservedNativeWindow {
            window_id: 42,
            visible: false,
            minimized: true,
            ..visible
        };
        assert_eq!(
            darwin::select_observed_native_window(
                7,
                None,
                vec![hidden, hidden, visible, visible, hidden]
            )
            .unwrap(),
            visible
        );

        let second_visible = ObservedNativeWindow {
            window_id: 43,
            ..visible
        };
        assert_eq!(
            darwin::select_observed_native_window(7, None, vec![visible, second_visible]),
            Err(NativeWindowObserverError::Ambiguous { pid: 7, count: 2 })
        );
        assert_eq!(
            darwin::select_observed_native_window(
                7,
                Some(second_visible.window_id),
                vec![visible, second_visible]
            )
            .unwrap(),
            second_visible
        );
        let conflicting_duplicate = ObservedNativeWindow {
            os_bounds: NativeWindowBounds {
                x: 81,
                ..visible.os_bounds
            },
            ..visible
        };
        assert_eq!(
            darwin::select_observed_native_window(
                7,
                Some(visible.window_id),
                vec![visible, conflicting_duplicate]
            ),
            Err(NativeWindowObserverError::Ambiguous { pid: 7, count: 2 })
        );
    }

    #[test]
    fn linked_native_and_rust_targets_must_describe_the_same_supported_host() {
        for (native, rust) in [
            ("macos-aarch64", "aarch64-apple-darwin"),
            ("macos-x86_64", "x86_64-apple-darwin"),
            ("linux-aarch64", "aarch64-unknown-linux-gnu"),
            ("linux-x86_64", "x86_64-unknown-linux-gnu"),
        ] {
            assert!(linked_build_targets_correlate(native, rust));
        }
        assert!(!linked_build_targets_correlate(
            "macos-aarch64",
            "x86_64-apple-darwin"
        ));
        assert!(!linked_build_targets_correlate(
            "linux-aarch64",
            "aarch64-apple-darwin"
        ));
        assert!(!linked_build_targets_correlate(
            "test-target",
            "test-target"
        ));
    }

    fn process() -> NativeProcessIdentity {
        NativeProcessIdentity {
            pid: 42,
            start_time: "1234".to_string(),
            executable_sha256: format!("{:x}", Sha256::digest(b"native-executable")),
            nonce: "b".repeat(64),
        }
    }

    fn bounds() -> NativeWindowBounds {
        NativeWindowBounds {
            x: 100,
            y: 120,
            width: 640,
            height: 480,
        }
    }

    fn policy() -> NativeAcceptancePolicy {
        NativeAcceptancePolicy {
            stable_presentation_floor: 120,
            playable_presentation_floor: 300,
            battle_frame_floor: 300,
        }
    }

    fn runtime_contract() -> NativeWindowRuntimeContract {
        NativeWindowRuntimeContract {
            capture_timeout_ms: 30_000,
            capture_kill_grace_ms: 5_000,
            observer_timeout_ms: 40_000,
            observer_kill_grace_ms: 5_000,
            acknowledgement_timeout_ms: 95_000,
            outer_child_timeout_ms: 300_000,
            outer_child_kill_grace_ms: 5_000,
            child_stdout_budget_bytes: 16_777_216,
            child_stderr_budget_bytes: 16_777_216,
            observer_response_budget_bytes: 65_536,
            capture_budget_bytes: 67_108_864,
            content_expansion_budget_bytes: 33_554_432,
            inventory_limits: NativeInventoryLimits {
                member_count: 10_000,
                member_bytes: 67_108_864,
                aggregate_bytes: 268_435_456,
                path_bytes: 4096,
                aggregate_path_bytes: 8_388_608,
            },
            expected_client_bounds: bounds(),
        }
    }
    fn observation(presentation: u64) -> NativeWindowObservation {
        NativeWindowObservation {
            binding: NativeWindowBinding {
                process: process(),
                window_id: 99,
                client_bounds: bounds(),
                os_bounds: NativeWindowBounds {
                    x: 94,
                    y: 92,
                    width: 652,
                    height: 514,
                },
            },
            committed_presentation: presentation,
            visible: true,
            minimized: false,
            semantic: NativeWindowSemanticSnapshot {
                trace_record_count: 1,
                accepted_player_inputs: 0,
                verified_battle_frames: 0,
            },
        }
    }

    fn semantic_observation(
        presentation: u64,
        trace_record_count: u64,
        accepted_player_inputs: u64,
        verified_battle_frames: u64,
    ) -> NativeWindowObservation {
        let mut observation = observation(presentation);
        observation.semantic = NativeWindowSemanticSnapshot {
            trace_record_count,
            accepted_player_inputs,
            verified_battle_frames,
        };
        observation
    }

    fn advance(proof: &mut NativeWindowProof, through: u64) {
        for presentation in 0..=through {
            proof.observe_visible(observation(presentation)).unwrap();
        }
    }

    fn screenshot(stage: NativeScreenshotStage, presentation: u64) -> NativeScreenshot {
        let input_events = u64::from(stage == NativeScreenshotStage::Playable);
        let trace_record_count = match stage {
            NativeScreenshotStage::Stable => 1,
            NativeScreenshotStage::Playable => 5,
        };
        let battle_frames = if stage == NativeScreenshotStage::Playable {
            policy().battle_frame_floor
        } else {
            0
        };
        let post_capture_observation = semantic_observation(
            presentation,
            trace_record_count,
            input_events,
            battle_frames,
        );
        NativeScreenshot {
            stage,
            binding: post_capture_observation.binding.clone(),
            post_capture_observation,
            committed_presentation: presentation,
            input_events,
            trace_record_count,
            battle_frames,
            relative_path: format!("screenshots/{presentation}.png"),
            byte_length: 128,
            sha256: match stage {
                NativeScreenshotStage::Stable => "c".repeat(64),
                NativeScreenshotStage::Playable => "d".repeat(64),
            },
        }
    }

    fn trace_record(
        sequence: u64,
        kind: crate::automation::trace::RecordKind,
        label: Option<&str>,
    ) -> crate::automation::trace::TraceRecord {
        let terminal_reason =
            (kind == crate::automation::trace::RecordKind::RunEnd).then(|| "success".to_string());
        crate::automation::trace::TraceRecord {
            schema: crate::automation::trace::TraceRecord::SCHEMA,
            run: 0,
            sequence,
            input_seen: sequence,
            present_seen: sequence,
            elapsed_ms: sequence,
            kind,
            label: label.map(str::to_string),
            from: None,
            to: None,
            terminal_reason,
            seed_application: None,
            presentation: None,
            activity: None,
        }
    }

    fn publish_rehashed_linked_mutation(
        root: &Path,
        manifest: &mut NativeAcceptanceManifest,
        receipt: &mut serde_json::Value,
        receipt_path: &Path,
        member: &str,
        receipt_field: &str,
        bytes: &[u8],
    ) {
        fs::write(root.join(member), bytes).unwrap();
        receipt[receipt_field]["byte_length"] = serde_json::json!(bytes.len() as u64);
        receipt[receipt_field]["sha256"] =
            serde_json::json!(format!("{:x}", Sha256::digest(bytes)));
        let receipt_bytes = serde_json::to_vec(receipt).unwrap();
        fs::write(receipt_path, &receipt_bytes).unwrap();
        for retained in &mut manifest.retained_files {
            if retained.relative_path == member {
                retained.byte_length = bytes.len() as u64;
                retained.sha256 = format!("{:x}", Sha256::digest(bytes));
            } else if retained.relative_path == "inputs/linked-build/linked-build-receipt.json" {
                retained.byte_length = receipt_bytes.len() as u64;
                retained.sha256 = format!("{:x}", Sha256::digest(&receipt_bytes));
            }
        }
    }

    #[test]
    fn command_paths_accept_only_absolute_or_descriptor_bound_normal_paths() {
        assert!(valid_bound_command_path("/tmp/uqm"));
        assert!(valid_bound_command_path("./inputs/uqm"));
        assert!(valid_bound_command_path("inputs/uqm"));
        assert!(!valid_bound_command_path("../uqm"));
        assert!(!valid_bound_command_path("./inputs/../uqm"));
        assert!(!valid_bound_command_path("C:\\uqm"));
        assert!(!valid_bound_command_path(""));
    }

    #[test]
    fn stable_stage_rejects_119_and_121_and_accepts_exactly_120_presentations() {
        let mut short = NativeWindowProof::new(process(), bounds(), policy());
        advance(&mut short, 119);
        assert_eq!(short.stable_presentations, 119);
        assert_eq!(
            short.record_screenshot(screenshot(NativeScreenshotStage::Stable, 119)),
            Err(NativeWindowProofError::ScreenshotStage)
        );

        let mut exact = NativeWindowProof::new(process(), bounds(), policy());
        advance(&mut exact, 120);
        assert_eq!(exact.stable_presentations, 120);
        assert!(exact
            .record_screenshot(screenshot(NativeScreenshotStage::Stable, 120))
            .is_ok());

        let mut late = NativeWindowProof::new(process(), bounds(), policy());
        advance(&mut late, 121);
        assert_eq!(late.stable_presentations, 121);
        assert_eq!(
            late.record_screenshot(screenshot(NativeScreenshotStage::Stable, 121)),
            Err(NativeWindowProofError::ScreenshotStage)
        );
    }

    /// Rewrite the bundle command as root-relative descriptors.
    fn descriptor_bound_manifest(manifest: &NativeAcceptanceManifest) -> NativeAcceptanceManifest {
        let mut descriptor_bound = manifest.clone();
        descriptor_bound.command = vec![
            "./inputs/uqm".to_string(),
            "--configdir=./config".to_string(),
            "--contentdir=./inputs/content".to_string(),
            "--automation-script=./inputs/linked-playable-v1.json".to_string(),
            "--automation-output=./automation".to_string(),
            "--native-window-proof=./native-window-proof.json".to_string(),
        ];
        descriptor_bound
    }

    /// Read the retained linked-build receipt as bytes and parsed JSON.
    fn linked_receipt_state(root: &Path) -> (PathBuf, Vec<u8>, serde_json::Value) {
        let path = root.join("inputs/linked-build/linked-build-receipt.json");
        let bytes = fs::read(&path).unwrap();
        let value = serde_json::from_slice(&bytes).unwrap();
        (path, bytes, value)
    }

    /// Build the accepted playable bundle used by the acceptance-validation tests.
    fn playable_acceptance_fixture() -> (
        tempfile::TempDir,
        NativeWindowReceipt,
        NativeAcceptanceManifest,
    ) {
        let mut proof = NativeWindowProof::new(process(), bounds(), policy());
        advance(&mut proof, 120);
        proof
            .record_screenshot(screenshot(NativeScreenshotStage::Stable, 120))
            .unwrap();
        for presentation in 121..299 {
            proof.observe_visible(observation(presentation)).unwrap();
        }
        proof
            .observe_visible(semantic_observation(299, 5, 1, 300))
            .unwrap();
        assert_eq!(proof.stable_presentations, 299);
        assert_eq!(
            proof.record_screenshot(screenshot(NativeScreenshotStage::Playable, 299)),
            Err(NativeWindowProofError::ScreenshotStage)
        );

        proof
            .observe_visible(semantic_observation(300, 5, 1, 300))
            .unwrap();
        proof
            .record_screenshot(screenshot(NativeScreenshotStage::Playable, 300))
            .unwrap();
        let mut receipt = proof.finish().unwrap();
        assert_eq!(receipt.stable_presentations, 300);
        assert!(receipt.passed);
        assert!(validate_native_window_receipt(&receipt).is_ok());

        let root = tempfile::tempdir().unwrap();
        for screenshot in &mut receipt.screenshots {
            let pixel = match screenshot.stage {
                NativeScreenshotStage::Stable => image::Rgb([0, 0, 0]),
                NativeScreenshotStage::Playable => image::Rgb([255, 255, 255]),
            };
            let image = image::RgbImage::from_pixel(bounds().width, bounds().height, pixel);
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image)
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            let bytes = bytes.into_inner();
            let path = root.path().join(&screenshot.relative_path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, &bytes).unwrap();
            screenshot.byte_length = bytes.len() as u64;
            screenshot.sha256 = format!("{:x}", Sha256::digest(&bytes));
        }
        let playable_position = receipt
            .screenshots
            .iter()
            .position(|screenshot| screenshot.stage == NativeScreenshotStage::Playable)
            .unwrap();
        let playable_path = root
            .path()
            .join(&receipt.screenshots[playable_position].relative_path);
        let original_playable = fs::read(&playable_path).unwrap();
        let equal_pixels = image::RgbaImage::from_pixel(
            bounds().width,
            bounds().height,
            image::Rgba([0, 0, 0, 255]),
        );
        let mut equal_bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(equal_pixels)
            .write_to(&mut equal_bytes, image::ImageFormat::Png)
            .unwrap();
        let equal_bytes = equal_bytes.into_inner();
        assert_ne!(original_playable, equal_bytes);
        fs::write(&playable_path, &equal_bytes).unwrap();
        let mut equal_pixel_receipt = receipt.clone();
        equal_pixel_receipt.screenshots[playable_position].byte_length = equal_bytes.len() as u64;
        equal_pixel_receipt.screenshots[playable_position].sha256 =
            format!("{:x}", Sha256::digest(&equal_bytes));
        assert_eq!(
            validate_native_window_bundle(
                root.path(),
                &equal_pixel_receipt,
                runtime_contract().inventory_limits,
            ),
            Err(NativeWindowProofError::ScreenshotStage)
        );
        fs::write(&playable_path, original_playable).unwrap();
        assert!(validate_native_window_bundle(
            root.path(),
            &receipt,
            runtime_contract().inventory_limits,
        )
        .is_ok());
        let mut altered_post_capture = receipt.clone();
        altered_post_capture.screenshots[0]
            .post_capture_observation
            .binding
            .window_id += 1;
        assert_eq!(
            validate_native_window_receipt(&altered_post_capture),
            Err(NativeWindowProofError::ScreenshotIdentity)
        );
        let mut reordered_post_capture = receipt.clone();
        reordered_post_capture.screenshots.swap(0, 1);
        assert_eq!(
            validate_native_window_receipt(&reordered_post_capture),
            Err(NativeWindowProofError::Receipt)
        );
        let trace_records = [
            trace_record(0, crate::automation::trace::RecordKind::RunStart, None),
            trace_record(
                1,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input_observed:key=Thrust:intended=1:current=1:pulsed=1"),
            ),
            trace_record(
                2,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input:key=Thrust:value=1"),
            ),
            trace_record(
                3,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("battle_frames_verified:count=300"),
            ),
            trace_record(4, crate::automation::trace::RecordKind::RunEnd, None),
        ];
        let trace_bytes = trace_records
            .iter()
            .map(|record| record.to_jsonl().unwrap())
            .collect::<String>()
            .into_bytes();
        let trace_path = root.path().join("automation/trace.jsonl");
        fs::create_dir_all(trace_path.parent().unwrap()).unwrap();
        fs::write(&trace_path, &trace_bytes).unwrap();
        let executable_bytes = b"native-executable";
        let executable_path = root.path().join("inputs/uqm");
        fs::create_dir_all(executable_path.parent().unwrap()).unwrap();
        fs::write(&executable_path, executable_bytes).unwrap();
        let executable_input = NativeRetainedInput {
            relative_path: "inputs/uqm".to_string(),
            byte_length: executable_bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(executable_bytes)),
        };
        let linked_root = root.path().join("inputs/linked-build");
        fs::create_dir_all(&linked_root).unwrap();
        let linked_members: [(&str, &[u8]); 10] = [
            (
                "cargo-messages.jsonl",
                concat!(
                    "{\"reason\":\"compiler-artifact\",\"package_id\":\"path+file:///repo/rust#uqm@0.8.0\",\"target\":{\"name\":\"uqm\",\"kind\":[\"bin\"]},\"executable\":\"/tmp/uqm\",\"filenames\":[]}\n",
                    "{\"reason\":\"compiler-artifact\",\"package_id\":\"path+file:///repo/rust#uqm@0.8.0\",\"target\":{\"name\":\"uqm_rust\",\"kind\":[\"staticlib\"]},\"executable\":null,\"filenames\":[\"/tmp/libuqm_rust-a.a\"]}\n",
                    "{\"reason\":\"build-script-executed\",\"package_id\":\"path+file:///repo/rust#uqm@0.8.0\",\"out_dir\":\"/tmp/out\"}\n",
                    "{\"reason\":\"build-finished\",\"success\":true}\n"
                )
                .as_bytes(),
            ),
            ("rust-archive.a", b"rust archive"),
            ("c-archive.a", b"c archive"),
            ("object-sidecar.manifest", b"object sidecar"),
            (
                "provider-report.json",
                br#"{"schema":"uqm-provider-report-v1","entries":[],"ledger_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","symbols":[],"tracked_native_file_delta":0,"summary":{"total_objects":0,"included":0,"excluded":0,"duplicate_providers_excluded":0,"recompiled":0,"replaced":0,"violations":0,"passed":true},"diagnostics":[]}"#,
            ),
            (
                "native-build-evidence.json",
                br#"{"schema":"uqm-native-build-evidence-v1","source_date_epoch":1,"build_date":"1970-01-01","target":"macos-aarch64","active_features":["audio_heart","debug-process","linked_c_archive"],"toolchain":{"target":"aarch64-apple-darwin","rustc":{"executable":"/rustc","version":"rustc 1.97.1 test","sha256":"aa","effective_args":[]},"cargo":{"executable":"/cargo","version":"1","sha256":"aa","effective_args":[]},"cc":{"executable":"/cc","version":"1","sha256":"aa","effective_args":[]},"ar":{"executable":"/ar","version":"1","sha256":"aa","effective_args":[]},"nm":{"executable":"/nm","version":"1","sha256":"aa","effective_args":[]},"pkg_config":{"executable":"/pkg-config","version":"1","sha256":"aa","effective_args":[]},"linker":{"executable":"/linker","version":"1","sha256":"aa","effective_args":[]}},"packages":[{"name":"sdl2","version":"1","cflags":[],"libs":[]}],"compile_profile":{"target":"macos-aarch64","compiler":"/cc","ordered_defines":["-DDEBUG"],"ordered_include_roots":[],"ordered_compile_flags":[],"dependency_flags":["-MMD","-MF","<depfile>"],"command_template":["/cc"]},"build_environment":{"SOURCE_DATE_EPOCH":"1"}}"#,
            ),
            ("Cargo.toml", b"[package]\nname = \"uqm\"\n"),
            ("Cargo.lock", b"# lock\n"),
            (
                "gates.json",
                br#"{"schema":"uqm-s4-ci-authority-v1","ledger_identity":{"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"tools":{"rust":{"expected_output_prefix":"rustc 1.97.1 "}},"package":{"cargo_manifest_sha256":"97114b6db551bf665ec805d7d066227bde6ac04d7e01c5e2b981ec6effafc345","cargo_lock_sha256":"718475cd179c5fce5f3cbaf68fb45b15017015ebeec359c040cf704e3f1c86b6"}}"#,
            ),
            (
                "canonical-toolchain.json",
                br#"{"target":"aarch64-apple-darwin","rustc":{"executable":"/rustc","version":"rustc 1.97.1 test","sha256":"aa","effective_args":[]},"cargo":{"executable":"/cargo","version":"1","sha256":"aa","effective_args":[]},"cc":{"executable":"/cc","version":"1","sha256":"aa","effective_args":[]},"ar":{"executable":"/ar","version":"1","sha256":"aa","effective_args":[]},"nm":{"executable":"/nm","version":"1","sha256":"aa","effective_args":[]},"pkg_config":{"executable":"/pkg-config","version":"1","sha256":"aa","effective_args":[]},"linker":{"executable":"/linker","version":"1","sha256":"aa","effective_args":[]}}"#,
            ),
        ];
        let linked_input = |filename: &str| {
            let bytes = linked_members
                .iter()
                .find_map(|(name, bytes)| (*name == filename).then_some(*bytes))
                .unwrap();
            NativeRetainedInput {
                relative_path: format!("inputs/linked-build/{filename}"),
                byte_length: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(bytes)),
            }
        };
        for (filename, bytes) in linked_members {
            fs::write(linked_root.join(filename), bytes).unwrap();
        }
        let linked_receipt = NativeLinkedBuildReceipt {
            schema: NATIVE_LINKED_BUILD_RECEIPT_SCHEMA.to_string(),
            source_sha: "a".repeat(40),
            cargo_command: vec![
                "/toolchain/cargo".to_string(),
                "build".to_string(),
                "--locked".to_string(),
                "--manifest-path".to_string(),
                "rust/Cargo.toml".to_string(),
                "--release".to_string(),
                "--no-default-features".to_string(),
                "--features".to_string(),
                "audio_heart,debug-process,linked_c_archive".to_string(),
                "--bin".to_string(),
                "uqm".to_string(),
                "--message-format=json-render-diagnostics".to_string(),
            ],
            native_profile: "linked-test".to_string(),
            feature: "audio_heart,debug-process,linked_c_archive".to_string(),
            cargo_executable_path: "/tmp/uqm".to_string(),
            cargo_rust_archive_path: "/tmp/libuqm_rust-a.a".to_string(),
            cargo_out_dir: "/tmp/out".to_string(),
            executable: executable_input.clone(),
            cargo_messages: linked_input("cargo-messages.jsonl"),
            rust_archive: linked_input("rust-archive.a"),
            c_archive: linked_input("c-archive.a"),
            object_sidecar: linked_input("object-sidecar.manifest"),
            provider_report: linked_input("provider-report.json"),
            native_build_evidence: linked_input("native-build-evidence.json"),
            cargo_manifest: linked_input("Cargo.toml"),
            cargo_lock: linked_input("Cargo.lock"),
            authority: linked_input("gates.json"),
            canonical_toolchain: linked_input("canonical-toolchain.json"),
        };
        fs::write(
            linked_root.join("linked-build-receipt.json"),
            serde_json::to_vec(&linked_receipt).unwrap(),
        )
        .unwrap();
        let script_bytes = include_bytes!("../../scripts/linked-playable-v1.json");
        let script_path = root.path().join("inputs/linked-playable-v1.json");
        fs::write(&script_path, script_bytes).unwrap();
        let content_bytes = b"native-content-package";
        let content_path = root
            .path()
            .join("inputs/content/packages/uqm-0.8.0-content.uqm");
        fs::create_dir_all(content_path.parent().unwrap()).unwrap();
        fs::write(&content_path, content_bytes).unwrap();
        fs::write(root.path().join("inputs/content/version"), b"0.8.0\n").unwrap();
        fs::write(
            root.path().join("native-window-proof.json"),
            serde_json::to_vec(&NativeWindowConfigFile {
                schema: NATIVE_WINDOW_CONFIG_SCHEMA.to_string(),
                nonce: receipt.binding.process.nonce.clone(),
                client_bounds: receipt.binding.client_bounds,
                runtime_contract: runtime_contract(),
                acceptance_policy: policy(),
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.path().join("automation/native-window-state.json"),
            serde_json::to_vec(&NativeWindowChildState {
                schema: NATIVE_WINDOW_STATE_SCHEMA.to_string(),
                nonce: receipt.binding.process.nonce.clone(),
                pid: receipt.binding.process.pid,
                sdl_window_id: 17,
                committed_presentation: 300,
                shown: true,
                requested_client_bounds: receipt.binding.client_bounds,
                client_bounds: receipt.binding.client_bounds,
                semantic: receipt.observations.last().unwrap().semantic,
            })
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.path().join("automation/native-window-ack.json"),
            serde_json::to_vec(&NativeWindowAck {
                schema: NATIVE_WINDOW_ACK_SCHEMA.to_string(),
                nonce: receipt.binding.process.nonce.clone(),
                committed_presentation: 300,
            })
            .unwrap(),
        )
        .unwrap();
        let manifest = NativeAcceptanceManifest {
            schema: NATIVE_ACCEPTANCE_SCHEMA.to_string(),
            command: vec![
                "/tmp/inputs/uqm".to_string(),
                "--configdir=/tmp/config".to_string(),
                "--contentdir=/tmp/inputs/content".to_string(),
                "--automation-script=/tmp/inputs/linked-playable-v1.json".to_string(),
                "--automation-output=/tmp/automation".to_string(),
                "--native-window-proof=/tmp/native-window-proof.json".to_string(),
            ],
            environment: std::collections::BTreeMap::from([(
                "SDL_AUDIODRIVER".to_string(),
                "dummy".to_string(),
            )]),
            execution_identity: NativeExecutionIdentity {
                real_uid: 501,
                effective_uid: 501,
                launchd_manager_uid: 501,
                launchd_manager_name: "Aqua".to_string(),
            },
            executable: executable_input,
            script: NativeRetainedInput {
                relative_path: "inputs/linked-playable-v1.json".to_string(),
                byte_length: script_bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(script_bytes)),
            },
            content_package: NativeRetainedInput {
                relative_path: "inputs/content/packages/uqm-0.8.0-content.uqm".to_string(),
                byte_length: content_bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(content_bytes)),
            },
            runtime_contract: runtime_contract(),
            acceptance_policy: policy(),
            retained_files: native_acceptance_inventory(
                root.path(),
                runtime_contract().inventory_limits,
            )
            .unwrap(),
            trace_path: "automation/trace.jsonl".to_string(),
            trace_byte_length: trace_bytes.len() as u64,
            trace_sha256: format!("{:x}", Sha256::digest(&trace_bytes)),
            child: NativeChildCleanupReceipt {
                process: receipt.binding.process.clone(),
                exit_code: Some(0),
                signal: None,
                term_sent: false,
                kill_sent: false,
                stdout_bytes: 0,
                stderr_bytes: 0,
                output_drained: true,
                initial_process_group_empty: true,
                config_root_removed: true,
                materialized_content_removed: true,
            },
            window: receipt.clone(),
            publications: receipt
                .observations
                .iter()
                .map(|observation| NativeWindowPublication {
                    state: NativeWindowChildState {
                        schema: NATIVE_WINDOW_STATE_SCHEMA.to_string(),
                        nonce: receipt.binding.process.nonce.clone(),
                        pid: receipt.binding.process.pid,
                        sdl_window_id: 17,
                        committed_presentation: observation.committed_presentation,
                        shown: true,
                        requested_client_bounds: receipt.binding.client_bounds,
                        client_bounds: observation.binding.client_bounds,
                        semantic: observation.semantic,
                    },
                    acknowledgement: NativeWindowAck {
                        schema: NATIVE_WINDOW_ACK_SCHEMA.to_string(),
                        nonce: receipt.binding.process.nonce.clone(),
                        committed_presentation: observation.committed_presentation,
                    },
                })
                .collect(),
            passed: true,
        };
        (root, receipt, manifest)
    }

    #[test]
    fn playable_floor_rejects_299_and_accepts_300_with_semantic_evidence() {
        let (_root, receipt, manifest) = playable_acceptance_fixture();
        assert_eq!(receipt.stable_presentations, 300);
        assert!(receipt.passed);
        assert!(manifest.passed);
    }

    #[test]
    fn linked_build_receipt_mutations_are_rejected() {
        let (root, _receipt, manifest) = playable_acceptance_fixture();
        let relocated = tempfile::tempdir().unwrap();
        for retained in &manifest.retained_files {
            let source = root.path().join(&retained.relative_path);
            let destination = relocated.path().join(&retained.relative_path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(source, destination).unwrap();
        }
        assert!(validate_native_acceptance_bundle(relocated.path(), &manifest).is_ok());
        let mut descriptor_bound = manifest.clone();
        descriptor_bound.command = vec![
            "./inputs/uqm".to_string(),
            "--configdir=./config".to_string(),
            "--contentdir=./inputs/content".to_string(),
            "--automation-script=./inputs/linked-playable-v1.json".to_string(),
            "--automation-output=./automation".to_string(),
            "--native-window-proof=./native-window-proof.json".to_string(),
        ];
        assert!(validate_native_acceptance_bundle(root.path(), &descriptor_bound).is_ok());
        let linked_receipt_path = root
            .path()
            .join("inputs/linked-build/linked-build-receipt.json");
        let linked_receipt_bytes = fs::read(&linked_receipt_path).unwrap();
        let linked_receipt_value: serde_json::Value =
            serde_json::from_slice(&linked_receipt_bytes).unwrap();
        let mutations = [
            ("schema", serde_json::json!("wrong")),
            (
                "cargo_command",
                serde_json::json!(["/toolchain/cargo", "build"]),
            ),
            ("native_profile", serde_json::json!("wrong")),
            ("feature", serde_json::json!("audio_heart")),
            ("cargo_executable_path", serde_json::json!("/tmp/other")),
            ("cargo_rust_archive_path", serde_json::json!("/tmp/other.a")),
            ("cargo_out_dir", serde_json::json!("/tmp/other-out")),
        ];
        for (field, replacement) in mutations {
            let mut changed = linked_receipt_value.clone();
            changed[field] = replacement;
            let changed_bytes = serde_json::to_vec(&changed).unwrap();
            fs::write(&linked_receipt_path, &changed_bytes).unwrap();
            let mut changed_manifest = descriptor_bound.clone();
            let retained = changed_manifest
                .retained_files
                .iter_mut()
                .find(|input| {
                    input.relative_path == "inputs/linked-build/linked-build-receipt.json"
                })
                .unwrap();
            retained.byte_length = changed_bytes.len() as u64;
            retained.sha256 = format!("{:x}", Sha256::digest(&changed_bytes));
            assert_eq!(
                validate_linked_build_receipt_bundle(root.path(), &changed_manifest),
                Err(NativeWindowProofError::Receipt)
            );
        }
        fs::write(&linked_receipt_path, &linked_receipt_bytes).unwrap();
        for field in [
            "executable",
            "cargo_messages",
            "rust_archive",
            "c_archive",
            "object_sidecar",
            "provider_report",
            "native_build_evidence",
            "cargo_manifest",
            "cargo_lock",
            "authority",
            "canonical_toolchain",
        ] {
            let mut changed = linked_receipt_value.clone();
            changed[field]["sha256"] = serde_json::json!("0".repeat(64));
            let changed_bytes = serde_json::to_vec(&changed).unwrap();
            fs::write(&linked_receipt_path, &changed_bytes).unwrap();
            let mut changed_manifest = descriptor_bound.clone();
            let retained = changed_manifest
                .retained_files
                .iter_mut()
                .find(|input| {
                    input.relative_path == "inputs/linked-build/linked-build-receipt.json"
                })
                .unwrap();
            retained.byte_length = changed_bytes.len() as u64;
            retained.sha256 = format!("{:x}", Sha256::digest(&changed_bytes));
            assert_eq!(
                validate_linked_build_receipt_bundle(root.path(), &changed_manifest),
                Err(NativeWindowProofError::Receipt),
                "accepted mutated {field} identity"
            );
        }
        fs::write(&linked_receipt_path, &linked_receipt_bytes).unwrap();
        for member in [
            "inputs/uqm",
            "inputs/linked-build/cargo-messages.jsonl",
            "inputs/linked-build/rust-archive.a",
            "inputs/linked-build/c-archive.a",
            "inputs/linked-build/object-sidecar.manifest",
            "inputs/linked-build/provider-report.json",
            "inputs/linked-build/native-build-evidence.json",
            "inputs/linked-build/Cargo.toml",
            "inputs/linked-build/Cargo.lock",
            "inputs/linked-build/gates.json",
            "inputs/linked-build/canonical-toolchain.json",
        ] {
            let path = root.path().join(member);
            let original = fs::read(&path).unwrap();
            let mut changed = original.clone();
            changed.push(b'x');
            fs::write(&path, changed).unwrap();
            assert_eq!(
                validate_linked_build_receipt_bundle(root.path(), &descriptor_bound),
                Err(NativeWindowProofError::Receipt),
                "accepted mutated retained member {member}"
            );
            fs::write(path, original).unwrap();
        }
    }

    #[test]
    fn forged_publications_and_authority_bindings_are_rejected() {
        let (root, _receipt, manifest) = playable_acceptance_fixture();
        let descriptor_bound = descriptor_bound_manifest(&manifest);
        let (linked_receipt_path, linked_receipt_bytes, linked_receipt_value) =
            linked_receipt_state(root.path());
        let mut missing_publication = manifest.clone();
        for (member, receipt_field, mutate) in [
            (
                "inputs/linked-build/provider-report.json",
                "provider_report",
                "schema",
            ),
            (
                "inputs/linked-build/native-build-evidence.json",
                "native_build_evidence",
                "schema",
            ),
        ] {
            let member_path = root.path().join(member);
            let original = fs::read(&member_path).unwrap();
            let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
            value[mutate] = serde_json::json!("wrong");
            let changed_member = serde_json::to_vec(&value).unwrap();
            fs::write(&member_path, &changed_member).unwrap();
            let mut changed_receipt = linked_receipt_value.clone();
            changed_receipt[receipt_field]["byte_length"] =
                serde_json::json!(changed_member.len() as u64);
            changed_receipt[receipt_field]["sha256"] =
                serde_json::json!(format!("{:x}", Sha256::digest(&changed_member)));
            let changed_receipt = serde_json::to_vec(&changed_receipt).unwrap();
            fs::write(&linked_receipt_path, &changed_receipt).unwrap();
            let mut changed_manifest = descriptor_bound.clone();
            for retained in &mut changed_manifest.retained_files {
                if retained.relative_path == member {
                    retained.byte_length = changed_member.len() as u64;
                    retained.sha256 = format!("{:x}", Sha256::digest(&changed_member));
                } else if retained.relative_path == "inputs/linked-build/linked-build-receipt.json"
                {
                    retained.byte_length = changed_receipt.len() as u64;
                    retained.sha256 = format!("{:x}", Sha256::digest(&changed_receipt));
                }
            }
            assert_eq!(
                validate_linked_build_receipt_bundle(root.path(), &changed_manifest),
                Err(NativeWindowProofError::Receipt),
                "accepted semantically invalid {member}"
            );
            fs::write(member_path, original).unwrap();
            fs::write(&linked_receipt_path, &linked_receipt_bytes).unwrap();
        }
        missing_publication.publications.remove(121);
        let authority_path = root.path().join("inputs/linked-build/gates.json");
        let mut forged_authority: serde_json::Value =
            serde_json::from_slice(&fs::read(&authority_path).unwrap()).unwrap();
        forged_authority["ledger_identity"]["sha256"] = serde_json::json!("b".repeat(64));
        let forged_authority = serde_json::to_vec(&forged_authority).unwrap();
        for (member, receipt_field, changed_member) in [
            (
                "inputs/linked-build/Cargo.toml",
                "cargo_manifest",
                b"[package]\nname = \"forged\"\n".to_vec(),
            ),
            (
                "inputs/linked-build/Cargo.lock",
                "cargo_lock",
                b"# forged lock\n".to_vec(),
            ),
            (
                "inputs/linked-build/gates.json",
                "authority",
                forged_authority,
            ),
        ] {
            let original_member = fs::read(root.path().join(member)).unwrap();
            let mut changed_receipt = linked_receipt_value.clone();
            let mut changed_manifest = descriptor_bound.clone();
            publish_rehashed_linked_mutation(
                root.path(),
                &mut changed_manifest,
                &mut changed_receipt,
                &linked_receipt_path,
                member,
                receipt_field,
                &changed_member,
            );
            assert_eq!(
                validate_linked_build_receipt_bundle(root.path(), &changed_manifest),
                Err(NativeWindowProofError::Receipt),
                "accepted coherently rehashed retained member {member}"
            );
            fs::write(root.path().join(member), original_member).unwrap();
            fs::write(&linked_receipt_path, &linked_receipt_bytes).unwrap();
        }

        let canonical_path = root
            .path()
            .join("inputs/linked-build/canonical-toolchain.json");
        let evidence_path = root
            .path()
            .join("inputs/linked-build/native-build-evidence.json");
        let original_canonical = fs::read(&canonical_path).unwrap();
        let original_evidence = fs::read(&evidence_path).unwrap();
        let mut forged_canonical: serde_json::Value =
            serde_json::from_slice(&original_canonical).unwrap();
        forged_canonical["rustc"]["version"] = serde_json::json!("rustc 9.99.9 forged");
        let forged_canonical = serde_json::to_vec(&forged_canonical).unwrap();
        let mut forged_evidence: serde_json::Value =
            serde_json::from_slice(&original_evidence).unwrap();
        forged_evidence["toolchain"] = serde_json::from_slice(&forged_canonical).unwrap();
        let forged_evidence = serde_json::to_vec(&forged_evidence).unwrap();
        let mut changed_receipt = linked_receipt_value.clone();
        let mut changed_manifest = descriptor_bound.clone();
        publish_rehashed_linked_mutation(
            root.path(),
            &mut changed_manifest,
            &mut changed_receipt,
            &linked_receipt_path,
            "inputs/linked-build/canonical-toolchain.json",
            "canonical_toolchain",
            &forged_canonical,
        );
        publish_rehashed_linked_mutation(
            root.path(),
            &mut changed_manifest,
            &mut changed_receipt,
            &linked_receipt_path,
            "inputs/linked-build/native-build-evidence.json",
            "native_build_evidence",
            &forged_evidence,
        );
        assert_eq!(
            validate_linked_build_receipt_bundle(root.path(), &changed_manifest),
            Err(NativeWindowProofError::Receipt),
            "accepted coherently rehashed toolchain and build evidence"
        );
        fs::write(canonical_path, original_canonical).unwrap();
        fs::write(evidence_path, original_evidence).unwrap();
        fs::write(&linked_receipt_path, &linked_receipt_bytes).unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &missing_publication),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_ack = manifest.clone();
        forged_ack.publications[120]
            .acknowledgement
            .committed_presentation += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_ack),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_publication_semantics = manifest.clone();
        forged_publication_semantics.publications[120]
            .state
            .semantic
            .trace_record_count += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_publication_semantics),
            Err(NativeWindowProofError::Receipt)
        );
        assert!(validate_native_acceptance_bundle(root.path(), &manifest).is_ok());
    }

    #[test]
    fn bundle_policy_and_configuration_mismatches_are_rejected() {
        let (root, _receipt, manifest) = playable_acceptance_fixture();
        let descriptor_bound = descriptor_bound_manifest(&manifest);
        let script_path = root.path().join("inputs/linked-playable-v1.json");
        let content_path = root
            .path()
            .join("inputs/content/packages/uqm-0.8.0-content.uqm");
        let mut sparse_observations = manifest.clone();
        let stable_presentation = sparse_observations.window.screenshots[0].committed_presentation;
        let playable_presentation =
            sparse_observations.window.screenshots[1].committed_presentation;
        let first_visible = sparse_observations.window.first_visible_presentation;
        sparse_observations
            .window
            .observations
            .retain(|observation| {
                matches!(
                    observation.committed_presentation,
                    presentation
                        if presentation == first_visible
                            || presentation == stable_presentation
                            || presentation == playable_presentation
                )
            });
        assert!(validate_native_acceptance_bundle(root.path(), &sparse_observations).is_ok());
        let mut invalid_policy = manifest.clone();
        invalid_policy.acceptance_policy.stable_presentation_floor = 0;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &invalid_policy),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_runtime = manifest.clone();
        forged_runtime.runtime_contract.capture_timeout_ms += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_runtime),
            Err(NativeWindowProofError::Receipt)
        );
        let mut mismatched_receipt_policy = manifest.clone();
        mismatched_receipt_policy
            .window
            .acceptance_policy
            .battle_frame_floor += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &mismatched_receipt_policy),
            Err(NativeWindowProofError::Receipt)
        );
        let config_path = root.path().join("native-window-proof.json");
        let config_bytes = fs::read(&config_path).unwrap();
        let mut mismatched_config: NativeWindowConfigFile =
            serde_json::from_slice(&config_bytes).unwrap();
        mismatched_config.acceptance_policy.battle_frame_floor += 1;
        fs::write(
            &config_path,
            serde_json::to_vec(&mismatched_config).unwrap(),
        )
        .unwrap();
        let mut config_bound_manifest = manifest.clone();
        config_bound_manifest.retained_files =
            native_acceptance_inventory(root.path(), runtime_contract().inventory_limits).unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &config_bound_manifest),
            Err(NativeWindowProofError::Receipt)
        );
        fs::write(&config_path, config_bytes).unwrap();

        let failure_manifest = NativeAcceptanceFailureManifest {
            schema: NATIVE_ACCEPTANCE_FAILURE_SCHEMA.to_string(),
            command: manifest.command.clone(),
            environment: manifest.environment.clone(),
            executable: manifest.executable.clone(),
            script: manifest.script.clone(),
            content_package: manifest.content_package.clone(),
            runtime_contract: runtime_contract(),
            acceptance_policy: policy(),
            retained_files: manifest.retained_files.clone(),
            child: NativeChildCleanupReceipt {
                exit_code: None,
                signal: Some(15),
                term_sent: true,
                ..manifest.child.clone()
            },
            failure_contract: NativeAcceptanceFailureContract::ChildSupervision,
            error: "native observer interrupted child supervision".to_string(),
            passed: false,
        };
        assert!(validate_native_acceptance_failure_bundle(root.path(), &failure_manifest).is_ok());
        let mut descriptor_bound_failure = failure_manifest.clone();
        descriptor_bound_failure.command = descriptor_bound.command.clone();
        assert!(
            validate_native_acceptance_failure_bundle(root.path(), &descriptor_bound_failure)
                .is_ok()
        );
        let mut failure_invalid_policy = failure_manifest.clone();
        failure_invalid_policy
            .acceptance_policy
            .playable_presentation_floor = 0;
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &failure_invalid_policy),
            Err(NativeWindowProofError::Receipt)
        );
        let mut failure_forged_runtime = failure_manifest.clone();
        failure_forged_runtime
            .runtime_contract
            .observer_response_budget_bytes += 1;
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &failure_forged_runtime),
            Err(NativeWindowProofError::Receipt)
        );
        let mut child_exit = failure_manifest.clone();
        child_exit.failure_contract = NativeAcceptanceFailureContract::ChildExit;
        assert!(validate_native_acceptance_failure_bundle(root.path(), &child_exit).is_ok());
        let mut semantic_signal = failure_manifest.clone();
        semantic_signal.failure_contract = NativeAcceptanceFailureContract::Semantic;
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &semantic_signal),
            Err(NativeWindowProofError::Receipt)
        );
        let mut observer_signal = failure_manifest.clone();
        observer_signal.failure_contract = NativeAcceptanceFailureContract::Observer;
        assert!(validate_native_acceptance_failure_bundle(root.path(), &observer_signal).is_ok());
        let mut config_cleanup_signal = failure_manifest.clone();
        config_cleanup_signal.failure_contract = NativeAcceptanceFailureContract::ConfigCleanup;
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &config_cleanup_signal),
            Err(NativeWindowProofError::Receipt)
        );
        let mut observer_successful_exit = failure_manifest.clone();
        observer_successful_exit.failure_contract = NativeAcceptanceFailureContract::Observer;
        observer_successful_exit.child.exit_code = Some(0);
        observer_successful_exit.child.signal = None;
        observer_successful_exit.child.term_sent = false;
        assert!(
            validate_native_acceptance_failure_bundle(root.path(), &observer_successful_exit)
                .is_ok()
        );
        let mut observer_after_term = observer_successful_exit.clone();
        observer_after_term.child.exit_code = None;
        observer_after_term.child.signal = Some(libc::SIGTERM);
        observer_after_term.child.term_sent = true;
        assert!(
            validate_native_acceptance_failure_bundle(root.path(), &observer_after_term).is_ok()
        );
        let mut child_exit_zero = observer_successful_exit;
        child_exit_zero.failure_contract = NativeAcceptanceFailureContract::ChildExit;
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &child_exit_zero),
            Err(NativeWindowProofError::Receipt)
        );
        fs::write(
            root.path().join("native-acceptance-failure.json"),
            serde_json::to_vec(&failure_manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &manifest),
            Err(NativeWindowProofError::Receipt)
        );
        assert!(validate_native_acceptance_failure_bundle(root.path(), &failure_manifest).is_ok());
        fs::write(
            root.path().join("native-acceptance.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &failure_manifest),
            Err(NativeWindowProofError::Receipt)
        );
        fs::remove_file(root.path().join("native-acceptance.json")).unwrap();
        fs::remove_file(root.path().join("native-acceptance-failure.json")).unwrap();
        let mut forged_failure = failure_manifest.clone();
        forged_failure.child.output_drained = false;
        assert_eq!(
            validate_native_acceptance_failure_bundle(root.path(), &forged_failure),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_failure_contract = serde_json::to_value(&failure_manifest).unwrap();
        forged_failure_contract["failure_contract"] = serde_json::json!("unknown");
        assert!(
            serde_json::from_value::<NativeAcceptanceFailureManifest>(forged_failure_contract)
                .is_err()
        );

        let mut forged_cleanup = manifest.clone();
        forged_cleanup.child.initial_process_group_empty = false;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_cleanup),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_command = manifest.clone();
        forged_command.command[0] = "/usr/bin/cargo".to_string();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_command),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_environment = manifest.clone();
        forged_environment
            .environment
            .insert("SDL_VIDEODRIVER".to_string(), "dummy".to_string());
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_environment),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_root_identity = manifest.clone();
        forged_root_identity.execution_identity.real_uid = 0;
        forged_root_identity.execution_identity.effective_uid = 0;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_root_identity),
            Err(NativeWindowProofError::Receipt)
        );
        let mut mismatched_identity = manifest.clone();
        mismatched_identity.execution_identity.effective_uid += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &mismatched_identity),
            Err(NativeWindowProofError::Receipt)
        );
        let mut mismatched_launchd_identity = manifest.clone();
        mismatched_launchd_identity
            .execution_identity
            .launchd_manager_uid += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &mismatched_launchd_identity),
            Err(NativeWindowProofError::Receipt)
        );
        let mut non_aqua_identity = manifest.clone();
        non_aqua_identity.execution_identity.launchd_manager_name = "Background".to_string();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &non_aqua_identity),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_process = manifest.clone();
        forged_process.child.process.pid += 1;
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_process),
            Err(NativeWindowProofError::Receipt)
        );
        let mut forged_trace = manifest.clone();
        forged_trace.trace_sha256 = "0".repeat(64);
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_trace),
            Err(NativeWindowProofError::Receipt)
        );
        let original_script = fs::read(&script_path).unwrap();
        fs::write(&script_path, b"{}\n").unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &manifest),
            Err(NativeWindowProofError::Receipt)
        );
        fs::write(&script_path, original_script).unwrap();
        let original_content = fs::read(&content_path).unwrap();
        let state_path = root.path().join("automation/native-window-state.json");
        let state_bytes = fs::read(&state_path).unwrap();
        let mut state: NativeWindowChildState = serde_json::from_slice(&state_bytes).unwrap();
        state.sdl_window_id = 0;
        fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
        let mut coherent_state_tamper = manifest.clone();
        coherent_state_tamper.retained_files =
            native_acceptance_inventory(root.path(), runtime_contract().inventory_limits).unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &coherent_state_tamper),
            Err(NativeWindowProofError::Receipt)
        );
        fs::write(&state_path, state_bytes).unwrap();
        let ack_path = root.path().join("automation/native-window-ack.json");
        let ack_bytes = fs::read(&ack_path).unwrap();
        let mut ack: NativeWindowAck = serde_json::from_slice(&ack_bytes).unwrap();
        ack.committed_presentation -= 1;
        fs::write(&ack_path, serde_json::to_vec(&ack).unwrap()).unwrap();
        let mut coherent_ack_tamper = manifest.clone();
        coherent_ack_tamper.retained_files =
            native_acceptance_inventory(root.path(), runtime_contract().inventory_limits).unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &coherent_ack_tamper),
            Err(NativeWindowProofError::Receipt)
        );
        fs::write(&ack_path, ack_bytes).unwrap();
        fs::write(&content_path, b"tampered-content").unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &manifest),
            Err(NativeWindowProofError::Receipt)
        );
        fs::write(&content_path, original_content).unwrap();
        let mut forged_content = manifest.clone();
        forged_content.content_package.sha256 = "0".repeat(64);
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &forged_content),
            Err(NativeWindowProofError::Receipt)
        );
        let mut omitted_inventory = manifest.clone();
        omitted_inventory.retained_files.pop();
        let mut unretained_content_route = manifest.clone();
        unretained_content_route.command[2] = "--contentdir=/tmp/unretained".to_string();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &unretained_content_route),
            Err(NativeWindowProofError::Receipt)
        );
        let mut extra_command_argument = manifest.clone();
        extra_command_argument.command.push("--forged".to_string());
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &extra_command_argument),
            Err(NativeWindowProofError::Receipt)
        );
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &omitted_inventory),
            Err(NativeWindowProofError::Receipt)
        );
        let unexpected = root.path().join("unexpected.bin");
        fs::write(&unexpected, b"unexpected").unwrap();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &manifest),
            Err(NativeWindowProofError::Receipt)
        );
        fs::remove_file(unexpected).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let backup = root.path().join("inputs/linked-playable-v1.backup");
            fs::rename(&script_path, &backup).unwrap();
            symlink(&backup, &script_path).unwrap();
            assert_eq!(
                validate_native_acceptance_bundle(root.path(), &manifest),
                Err(NativeWindowProofError::Receipt)
            );
            fs::remove_file(&script_path).unwrap();
            fs::rename(backup, &script_path).unwrap();
        }
        let mut identical_screenshots = manifest.clone();
        identical_screenshots.window.screenshots[1].sha256 =
            identical_screenshots.window.screenshots[0].sha256.clone();
        assert_eq!(
            validate_native_acceptance_bundle(root.path(), &identical_screenshots),
            Err(NativeWindowProofError::ScreenshotStage)
        );
    }

    #[test]
    fn screenshot_and_observation_identity_is_enforced() {
        let (root, receipt, _manifest) = playable_acceptance_fixture();
        let stable_position = receipt
            .screenshots
            .iter()
            .position(|screenshot| screenshot.stage == NativeScreenshotStage::Stable)
            .unwrap();
        let stable_path = root
            .path()
            .join(&receipt.screenshots[stable_position].relative_path);
        let original_stable = fs::read(&stable_path).unwrap();
        let wrong_dimensions = image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
        let mut wrong_dimension_bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(wrong_dimensions)
            .write_to(&mut wrong_dimension_bytes, image::ImageFormat::Png)
            .unwrap();
        let wrong_dimension_bytes = wrong_dimension_bytes.into_inner();
        fs::write(&stable_path, &wrong_dimension_bytes).unwrap();
        let mut wrong_dimension_receipt = receipt.clone();
        wrong_dimension_receipt.screenshots[stable_position].byte_length =
            wrong_dimension_bytes.len() as u64;
        wrong_dimension_receipt.screenshots[stable_position].sha256 =
            format!("{:x}", Sha256::digest(&wrong_dimension_bytes));
        assert_eq!(
            validate_native_window_bundle(
                root.path(),
                &wrong_dimension_receipt,
                runtime_contract().inventory_limits,
            ),
            Err(NativeWindowProofError::ScreenshotIdentity)
        );
        fs::write(&stable_path, original_stable).unwrap();

        let mut oversized_claim = receipt.clone();
        oversized_claim.screenshots[stable_position].byte_length = runtime_contract()
            .inventory_limits
            .member_bytes
            .checked_add(1)
            .unwrap();
        assert_eq!(
            validate_native_window_bundle(
                root.path(),
                &oversized_claim,
                runtime_contract().inventory_limits,
            ),
            Err(NativeWindowProofError::ScreenshotIdentity)
        );

        fs::write(
            root.path().join(&receipt.screenshots[0].relative_path),
            b"tampered",
        )
        .unwrap();
        assert_eq!(
            validate_native_window_bundle(
                root.path(),
                &receipt,
                runtime_contract().inventory_limits,
            ),
            Err(NativeWindowProofError::ScreenshotIdentity)
        );

        let mut omitted_observation = receipt.clone();
        omitted_observation.observations.pop();
        assert!(validate_native_window_receipt(&omitted_observation).is_err());

        let mut unsafe_screenshot_path = receipt.clone();
        unsafe_screenshot_path.screenshots[0].relative_path = "../stable.png".to_string();
        assert_eq!(
            validate_native_window_receipt(&unsafe_screenshot_path),
            Err(NativeWindowProofError::ScreenshotIdentity)
        );
    }

    #[test]
    fn semantic_trace_labels_require_exact_ordered_authoritative_pairs() {
        let valid = [
            trace_record(
                0,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input_observed:key=Weapon:intended=1:current=1:pulsed=1"),
            ),
            trace_record(
                1,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input:key=Weapon:value=1"),
            ),
            trace_record(
                2,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input_observed:key=Weapon:intended=0:current=0:pulsed=0"),
            ),
            trace_record(
                3,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input:key=Weapon:value=0"),
            ),
            trace_record(
                4,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("battle_frames_reached:count=23:minimum=1"),
            ),
            trace_record(
                5,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("battle_frames_verified:count=300"),
            ),
        ];
        assert_eq!(
            native_window_trace_semantic_snapshot(&valid[..5]).unwrap(),
            NativeWindowSemanticSnapshot {
                trace_record_count: 5,
                accepted_player_inputs: 1,
                verified_battle_frames: 23,
            }
        );
        assert_eq!(
            native_window_trace_semantic_snapshot(&valid).unwrap(),
            NativeWindowSemanticSnapshot {
                trace_record_count: 6,
                accepted_player_inputs: 1,
                verified_battle_frames: 300,
            }
        );
        for forged in [
            "player_input:key=Unknown:value=1",
            "player_input:key=Weapon:value=01",
            "battle_frames_reached:count=0:minimum=1",
            "battle_frames_reached:count=1:minimum=0",
            "battle_frames_reached:count=01:minimum=1",
            "battle_frames_reached:count=1:minimum=01",
            "battle_frames_reached:value=1:minimum=1",
            "battle_frames_verified:count=0300",
            "battle_frames_verified:value=300",
        ] {
            let records = [trace_record(
                0,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some(forged),
            )];
            assert_eq!(
                native_window_trace_semantic_snapshot(&records),
                Err(NativeWindowProofError::Receipt)
            );
        }
        let held = [
            trace_record(
                0,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input_observed:key=Weapon:intended=1:current=1:pulsed=0"),
            ),
            trace_record(
                1,
                crate::automation::trace::RecordKind::SemanticAssertion,
                Some("player_input:key=Weapon:value=1"),
            ),
        ];
        assert_eq!(
            native_window_trace_semantic_snapshot(&held).unwrap(),
            NativeWindowSemanticSnapshot {
                trace_record_count: 2,
                accepted_player_inputs: 0,
                verified_battle_frames: 0,
            }
        );
        for (observed, accepted) in [
            (
                "player_input_observed:key=Weapon:intended=1:current=1:pulsed=1",
                "player_input:key=Thrust:value=1",
            ),
            (
                "player_input_observed:key=Weapon:intended=0:current=0:pulsed=0",
                "player_input:key=Weapon:value=1",
            ),
        ] {
            let records = [
                trace_record(
                    0,
                    crate::automation::trace::RecordKind::SemanticAssertion,
                    Some(observed),
                ),
                trace_record(
                    1,
                    crate::automation::trace::RecordKind::SemanticAssertion,
                    Some(accepted),
                ),
            ];
            assert_eq!(
                native_window_trace_semantic_snapshot(&records),
                Err(NativeWindowProofError::Receipt)
            );
        }
        let wrong_kind = [trace_record(
            0,
            crate::automation::trace::RecordKind::Presentation,
            Some("battle_frames_verified:count=300"),
        )];
        assert_eq!(
            native_window_trace_semantic_snapshot(&wrong_kind),
            Err(NativeWindowProofError::Receipt)
        );
    }

    #[test]
    fn playable_capture_rejects_an_earlier_semantically_eligible_presentation() {
        let mut proof = NativeWindowProof::new(process(), bounds(), policy());
        advance(&mut proof, 120);
        proof
            .record_screenshot(screenshot(NativeScreenshotStage::Stable, 120))
            .unwrap();
        for presentation in 121..300 {
            proof.observe_visible(observation(presentation)).unwrap();
        }
        proof
            .observe_visible(semantic_observation(300, 5, 1, 300))
            .unwrap();
        proof
            .observe_visible(semantic_observation(301, 5, 1, 300))
            .unwrap();
        let mut late = screenshot(NativeScreenshotStage::Playable, 301);
        late.binding = observation(301).binding;
        late.committed_presentation = 301;
        assert_eq!(
            proof.record_screenshot(late),
            Err(NativeWindowProofError::ScreenshotStage)
        );
    }

    #[test]
    fn rejects_os_bounds_that_do_not_contain_the_client() {
        let mut proof = NativeWindowProof::new(process(), bounds(), policy());
        let mut transitional = observation(0);
        transitional.binding.os_bounds = NativeWindowBounds {
            x: 321,
            y: 31,
            width: 1278,
            height: 990,
        };
        assert_eq!(
            proof.observe_visible(transitional),
            Err(NativeWindowProofError::OsBoundsDoNotContainClient {
                client: bounds(),
                os: NativeWindowBounds {
                    x: 321,
                    y: 31,
                    width: 1278,
                    height: 990,
                },
            })
        );
    }

    #[test]
    fn rejects_window_identity_or_bounds_changes() {
        let mut proof = NativeWindowProof::new(process(), bounds(), policy());
        proof.observe_visible(observation(0)).unwrap();
        let mut changed = observation(1);
        let expected_window_id = changed.binding.window_id;
        let expected_os_bounds = changed.binding.os_bounds;
        changed.binding.window_id += 1;
        let actual_window_id = changed.binding.window_id;
        assert_eq!(
            proof.observe_visible(changed),
            Err(NativeWindowProofError::BindingChanged {
                expected_window_id,
                actual_window_id,
                expected_os_bounds,
                actual_os_bounds: expected_os_bounds,
            })
        );
    }

    #[test]
    fn acknowledgement_requires_exact_nonce_and_presentation() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("native-window-ack.json");
        let output_directory = BoundOutputDirectory::open(root.path()).unwrap();
        let nonce = "a".repeat(64);
        let wrong_nonce = "b".repeat(64);
        assert_eq!(
            wait_for_native_window_ack(
                &output_directory,
                &nonce,
                41,
                std::time::Duration::MAX,
                4096,
            )
            .unwrap_err(),
            "native-window acknowledgement deadline overflowed"
        );

        acknowledge_native_window_state(&path, &wrong_nonce, 41).unwrap();
        assert!(wait_for_native_window_ack(
            &output_directory,
            &nonce,
            41,
            std::time::Duration::from_millis(5),
            4096,
        )
        .is_err());

        acknowledge_native_window_state(&path, &nonce, 40).unwrap();
        assert!(wait_for_native_window_ack(
            &output_directory,
            &nonce,
            41,
            std::time::Duration::from_millis(5),
            4096,
        )
        .is_err());

        fs::write(&path, b"not-json\n").unwrap();
        let writer_path = path.clone();
        let writer_nonce = nonce.clone();
        let writer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            acknowledge_native_window_state(&writer_path, &writer_nonce, 41).unwrap();
        });
        wait_for_native_window_ack(
            &output_directory,
            &nonce,
            41,
            std::time::Duration::from_secs(60),
            4096,
        )
        .unwrap();
        writer.join().unwrap();
        assert!(!root.path().join(".native-window-ack.json.tmp").exists());
    }
    #[cfg(unix)]
    #[test]
    fn bound_output_publication_ignores_a_replacement_root_path() {
        let root = tempfile::tempdir().unwrap();
        let visible = root.path().join("output");
        let retained = root.path().join("output-retained");
        fs::create_dir(&visible).unwrap();
        let bound = BoundOutputDirectory::open(&visible).unwrap();
        fs::rename(&visible, &retained).unwrap();
        fs::create_dir(&visible).unwrap();

        bound
            .publish(std::ffi::OsStr::new("state.json"), b"bound", "state")
            .unwrap();

        assert_eq!(fs::read(retained.join("state.json")).unwrap(), b"bound\n");
        assert!(!visible.join("state.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn bound_acknowledgement_publisher_ignores_a_replacement_root_path() {
        let root = tempfile::tempdir().unwrap();
        let visible = root.path().join("output");
        let retained = root.path().join("output-retained");
        fs::create_dir(&visible).unwrap();
        let publisher =
            NativeWindowAckPublisher::bind(&visible.join("native-window-ack.json")).unwrap();
        fs::rename(&visible, &retained).unwrap();
        fs::create_dir(&visible).unwrap();
        let nonce = "a".repeat(64);

        publisher.acknowledge(&nonce, 41).unwrap();

        let acknowledgement: NativeWindowAck =
            serde_json::from_slice(&fs::read(retained.join("native-window-ack.json")).unwrap())
                .unwrap();
        assert_eq!(acknowledgement.nonce, nonce);
        assert_eq!(acknowledgement.committed_presentation, 41);
        assert!(!visible.join("native-window-ack.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn bound_state_reader_ignores_a_replacement_root_path() {
        let root = tempfile::tempdir().unwrap();
        let visible = root.path().join("output");
        let retained = root.path().join("output-retained");
        fs::create_dir(&visible).unwrap();
        let reader =
            NativeWindowStateReader::bind(&visible.join("native-window-state.json"), 4096).unwrap();
        fs::rename(&visible, &retained).unwrap();
        fs::create_dir(&visible).unwrap();
        let nonce = "a".repeat(64);
        let bounds = NativeWindowBounds {
            x: 0,
            y: 0,
            width: 1280,
            height: 960,
        };
        let retained_state = NativeWindowChildState {
            schema: NATIVE_WINDOW_STATE_SCHEMA.to_string(),
            nonce: nonce.clone(),
            pid: std::process::id(),
            sdl_window_id: 17,
            committed_presentation: 41,
            shown: true,
            requested_client_bounds: bounds,
            client_bounds: bounds,
            semantic: NativeWindowSemanticSnapshot {
                trace_record_count: 90,
                accepted_player_inputs: 12,
                verified_battle_frames: 30,
            },
        };
        fs::write(
            retained.join("native-window-state.json"),
            serde_json::to_vec(&retained_state).unwrap(),
        )
        .unwrap();
        fs::write(
            visible.join("native-window-state.json"),
            b"attacker-controlled",
        )
        .unwrap();

        let observed = reader
            .read_if_present(&nonce, std::process::id(), bounds)
            .unwrap()
            .unwrap();
        assert_eq!(observed, retained_state);
    }

    #[cfg(unix)]
    #[test]
    fn detached_reads_reject_symlinked_intermediate_components() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        fs::write(external.path().join("capture.png"), b"png").unwrap();
        symlink(external.path(), root.path().join("screenshots")).unwrap();

        assert!(read_relative_regular_nofollow_bounded(
            root.path(),
            Path::new("screenshots/capture.png"),
            1024,
        )
        .is_err());
        assert_eq!(
            native_acceptance_inventory(root.path(), runtime_contract().inventory_limits),
            Err(NativeWindowProofError::Receipt)
        );
    }

    #[test]
    fn native_inventory_enforces_each_authority_budget() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("alpha"), b"ab").unwrap();
        fs::write(root.path().join("beta"), b"cd").unwrap();
        let limits = runtime_contract().inventory_limits;
        assert_eq!(
            native_acceptance_inventory(root.path(), limits)
                .unwrap()
                .len(),
            2
        );

        assert!(native_acceptance_inventory(
            root.path(),
            NativeInventoryLimits {
                member_count: 1,
                ..limits
            },
        )
        .is_err());
        assert!(native_acceptance_inventory(
            root.path(),
            NativeInventoryLimits {
                member_bytes: 1,
                ..limits
            },
        )
        .is_err());
        assert!(native_acceptance_inventory(
            root.path(),
            NativeInventoryLimits {
                aggregate_bytes: 3,
                ..limits
            },
        )
        .is_err());
        assert!(native_acceptance_inventory(
            root.path(),
            NativeInventoryLimits {
                path_bytes: 4,
                ..limits
            },
        )
        .is_err());
        assert!(native_acceptance_inventory(
            root.path(),
            NativeInventoryLimits {
                aggregate_path_bytes: 8,
                ..limits
            },
        )
        .is_err());
    }

    #[test]
    fn retained_file_digest_rejects_deterministic_growth_and_truncation() {
        let mut grew = std::io::Cursor::new(b"abcd");
        assert_eq!(
            digest_exact_bounded(&mut grew, 3, 3),
            Err(NativeWindowProofError::Receipt)
        );
        let mut truncated = std::io::Cursor::new(b"abc");
        assert_eq!(
            digest_exact_bounded(&mut truncated, 4, 4),
            Err(NativeWindowProofError::Receipt)
        );
        let mut over_limit = std::io::Cursor::new(b"abc");
        assert_eq!(
            digest_exact_bounded(&mut over_limit, 3, 2),
            Err(NativeWindowProofError::Receipt)
        );
        let mut exact = std::io::Cursor::new(b"abc");
        assert_eq!(
            digest_exact_bounded(&mut exact, 3, 3).unwrap(),
            format!("{:x}", Sha256::digest(b"abc"))
        );
    }

    #[test]
    fn native_inventory_counts_directories_and_the_excluded_manifest() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("native-acceptance.json"), b"manifest").unwrap();
        let limits = NativeInventoryLimits {
            member_count: 1,
            ..runtime_contract().inventory_limits
        };
        assert!(native_acceptance_inventory(root.path(), limits)
            .unwrap()
            .is_empty());

        let directory = root.path().join("payload");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("value"), b"value").unwrap();
        assert!(native_acceptance_inventory(root.path(), limits).is_err());
    }

    #[test]
    fn screenshot_decoder_enforces_encoded_and_decoded_byte_budgets() {
        use image::ImageEncoder as _;

        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&[0; 40_000], 100, 100, image::ColorType::Rgba8.into())
            .unwrap();
        assert!(decode_png_bounded(&png, 100_000).is_ok());
        assert_eq!(
            decode_png_bounded(&png, png.len() as u64 - 1),
            Err(NativeWindowObserverError::OutputLimit {
                stream: "screenshot".to_string(),
                limit_bytes: png.len() as u64 - 1,
            })
        );
        assert!(png.len() < 1_024);
        assert_eq!(
            decode_png_bounded(&png, 1_024),
            Err(NativeWindowObserverError::OutputLimit {
                stream: "screenshot-decoded".to_string(),
                limit_bytes: 1_024,
            })
        );
    }

    #[test]
    fn setup_failure_envelope_rejects_invalid_contract_and_inventory() {
        let root = tempfile::tempdir().unwrap();
        let mut manifest = NativeAcceptanceSetupFailureManifest {
            schema: NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA.to_string(),
            command: vec!["uqm-native-acceptance".to_string()],
            expected_executable_byte_length: 1,
            expected_executable_sha256: "a".repeat(64),
            runtime_contract: runtime_contract(),
            acceptance_policy: policy(),
            retained_files: Vec::new(),
            failure_contract: NativeAcceptanceSetupFailureContract::Preparation,
            error: "invalid content package version".to_string(),
            passed: false,
        };
        assert!(validate_native_acceptance_setup_failure_bundle(root.path(), &manifest).is_ok());

        manifest.passed = true;
        assert_eq!(
            validate_native_acceptance_setup_failure_bundle(root.path(), &manifest),
            Err(NativeWindowProofError::Receipt)
        );
        manifest.passed = false;
        manifest.retained_files.push(NativeRetainedInput {
            relative_path: "forged".to_string(),
            byte_length: 1,
            sha256: "b".repeat(64),
        });
        assert_eq!(
            validate_native_acceptance_setup_failure_bundle(root.path(), &manifest),
            Err(NativeWindowProofError::Receipt)
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_inventory_handles_deep_trees_without_recursive_walking() {
        let root = tempfile::tempdir().unwrap();
        let mut directory = root.path().to_path_buf();
        for _ in 0..300 {
            directory.push("d");
            fs::create_dir(&directory).unwrap();
        }
        fs::write(directory.join("value"), b"value").unwrap();

        let inventory =
            native_acceptance_inventory(root.path(), runtime_contract().inventory_limits).unwrap();
        assert_eq!(inventory.len(), 1);
        assert!(inventory[0].relative_path.ends_with("/value"));
    }

    #[cfg(unix)]
    #[test]
    fn runtime_state_read_rejects_a_symlink() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let bounds = NativeWindowBounds {
            x: 1,
            y: 2,
            width: 1280,
            height: 960,
        };
        let target = root.path().join("target.json");
        fs::write(
            &target,
            serde_json::to_vec(&NativeWindowChildState {
                schema: NATIVE_WINDOW_STATE_SCHEMA.to_string(),
                nonce: "a".repeat(64),
                pid: 42,
                sdl_window_id: 7,
                committed_presentation: 1,
                shown: true,
                requested_client_bounds: bounds,
                client_bounds: bounds,
                semantic: NativeWindowSemanticSnapshot {
                    trace_record_count: 0,
                    accepted_player_inputs: 0,
                    verified_battle_frames: 0,
                },
            })
            .unwrap(),
        )
        .unwrap();
        let alias = root.path().join("state.json");
        symlink(target, &alias).unwrap();
        assert!(read_native_window_state(&alias, &"a".repeat(64), 42, bounds).is_err());
    }
}
