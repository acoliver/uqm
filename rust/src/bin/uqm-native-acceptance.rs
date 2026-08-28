use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(test)]
use std::time::Duration;
#[cfg(test)]
use uqm_rust::automation::native_window::NativeInventoryLimits;
use uqm_rust::automation::native_window::{
    native_window_trace_semantic_snapshot, validate_native_linked_build_semantics,
    NativeAcceptanceFailureContract, NativeAcceptancePolicy, NativeWindowRuntimeContract,
};
use uqm_rust::automation::{
    capture_native_window, native_acceptance_failure_inventory, native_acceptance_inventory,
    observe_native_window, parse_script, validate_native_acceptance_bundle,
    validate_native_acceptance_failure_bundle, validate_native_acceptance_setup_failure_bundle,
    validate_script, ChildSession, ChildSessionConfig, ChildSessionError, ChildSessionFailure,
    ChildSessionReceipt, NativeAcceptanceFailureManifest, NativeAcceptanceManifest,
    NativeAcceptanceSetupFailureContract, NativeAcceptanceSetupFailureManifest,
    NativeChildCleanupReceipt, NativeExecutionIdentity, NativeLinkedBuildReceipt,
    NativeProcessIdentity, NativeRetainedInput, NativeScreenshot, NativeScreenshotStage,
    NativeWindowAck, NativeWindowAckPublisher, NativeWindowBinding, NativeWindowBounds,
    NativeWindowChildState, NativeWindowConfigFile, NativeWindowObservation,
    NativeWindowObserverError, NativeWindowProof, NativeWindowPublication, NativeWindowStateReader,
    ObservedNativeWindow, ProcessIdentity, TraceRecord, NATIVE_ACCEPTANCE_FAILURE_SCHEMA,
    NATIVE_ACCEPTANCE_SCHEMA, NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA,
    NATIVE_LINKED_BUILD_RECEIPT_SCHEMA, NATIVE_WINDOW_ACK_SCHEMA, NATIVE_WINDOW_CONFIG_SCHEMA,
};
use uqm_rust::mainloop::options::{DEFAULT_RESOLUTION_HEIGHT, DEFAULT_RESOLUTION_WIDTH};

const NATIVE_OBSERVER_HELPER_SCHEMA: &str = "uqm-native-observer-helper-v1";
const MAX_CONTENT_ARCHIVE_ENTRIES: usize = 100_000;
const MAX_CONTENT_ARCHIVE_NAME_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CONTENT_ARCHIVE_ENTRY_NAME_BYTES: usize = 4096;
const MAX_NATIVE_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeObserverHelperResponse {
    schema: String,
    result: NativeObserverHelperResult,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum NativeObserverHelperResult {
    Observed(Result<ObservedNativeWindow, NativeWindowObserverError>),
    Captured(Result<(), NativeWindowObserverError>),
}

struct BoundedNativeObserver {
    executable: PathBuf,
    executable_digest: String,
    scratch_root: PathBuf,
    scratch_directory: std::os::fd::OwnedFd,
    scratch_identity: (u64, u64),
    members: std::collections::BTreeSet<std::ffi::OsString>,
    cleaned: bool,
    sequence: u64,
    contract: NativeWindowRuntimeContract,
}

impl BoundedNativeObserver {
    fn spawn(nonce: &str, contract: NativeWindowRuntimeContract) -> Result<Self, String> {
        if !contract.has_valid_deadline_order() {
            return Err("native observer runtime contract has invalid deadline order".to_string());
        }
        let scratch_root = std::env::temp_dir().join(format!("uqm-native-observer-{nonce}"));
        fs::create_dir(&scratch_root).map_err(|error| {
            format!(
                "create native observer scratch {}: {error}",
                scratch_root.display()
            )
        })?;
        let scratch_directory = open_directory_path_nofollow(&scratch_root)?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt as _;
        let scratch_metadata = fs::File::from(duplicate_fd(&scratch_directory)?)
            .metadata()
            .map_err(|error| {
                format!(
                    "inspect native observer scratch {}: {error}",
                    scratch_root.display()
                )
            })?;
        let scratch_identity = (scratch_metadata.dev(), scratch_metadata.ino());
        let executable = std::env::current_exe()
            .map_err(|error| format!("resolve native observer helper executable: {error}"))?;
        let executable_digest = sha256_hex(
            &read_bounded_regular(&executable, contract.inventory_limits.member_bytes)
                .map_err(|error| format!("read native observer helper executable: {error}"))?,
        );
        Ok(Self {
            executable,
            executable_digest,
            scratch_root,
            scratch_directory,
            scratch_identity,
            members: std::collections::BTreeSet::new(),
            cleaned: false,
            sequence: 0,
            contract,
        })
    }

    fn observe(
        &mut self,
        pid: u32,
        expected_window_id: Option<u64>,
    ) -> Result<ObservedNativeWindow, NativeWindowObserverError> {
        let response_sequence = self.sequence;
        let response_path = self.next_path("observe", "json")?;
        let mut command = Command::new(&self.executable);
        command
            .arg("observer-helper")
            .arg("observe")
            .arg(pid.to_string())
            .arg(
                expected_window_id
                    .map_or_else(|| "any".to_string(), |window_id| window_id.to_string()),
            )
            .arg(&response_path);
        self.run_helper(command, "observation", response_sequence)?;
        let response: NativeObserverHelperResponse =
            self.read_json_member(&response_path, self.contract.observer_response_budget_bytes)?;
        self.remove_path(&response_path)?;
        if response.schema != NATIVE_OBSERVER_HELPER_SCHEMA {
            return Err(NativeWindowObserverError::Os(
                "native observer helper response schema mismatch".to_string(),
            ));
        }
        match response.result {
            NativeObserverHelperResult::Observed(result) => result,
            NativeObserverHelperResult::Captured(_) => Err(NativeWindowObserverError::Os(
                "native observer helper returned capture evidence for observation".to_string(),
            )),
        }
    }

    fn capture(&mut self, window_id: u64) -> Result<Vec<u8>, NativeWindowObserverError> {
        let capture_path = self.next_path("capture", "png")?;
        let response_sequence = self.sequence;
        let response_path = self.next_path("capture", "json")?;
        let mut command = Command::new(&self.executable);
        command
            .arg("observer-helper")
            .arg("capture")
            .arg(window_id.to_string())
            .arg(&capture_path)
            .arg(serde_json::to_string(&self.contract).map_err(|error| {
                NativeWindowObserverError::Os(format!(
                    "serialize native capture runtime contract: {error}"
                ))
            })?)
            .arg(&response_path);
        self.run_helper(command, "capture", response_sequence)?;
        let response: NativeObserverHelperResponse =
            self.read_json_member(&response_path, self.contract.observer_response_budget_bytes)?;
        self.remove_path(&response_path)?;
        if response.schema != NATIVE_OBSERVER_HELPER_SCHEMA {
            return Err(NativeWindowObserverError::Os(
                "native capture helper response schema mismatch".to_string(),
            ));
        }
        match response.result {
            NativeObserverHelperResult::Captured(Ok(())) => {}
            NativeObserverHelperResult::Captured(Err(error)) => return Err(error),
            NativeObserverHelperResult::Observed(_) => {
                return Err(NativeWindowObserverError::Os(
                    "native capture helper returned observation evidence".to_string(),
                ));
            }
        }
        let bytes = self.read_member(&capture_path, self.contract.capture_budget_bytes)?;
        self.remove_path(&capture_path)?;
        Ok(bytes)
    }

    fn next_path(
        &mut self,
        operation: &str,
        extension: &str,
    ) -> Result<PathBuf, NativeWindowObserverError> {
        let sequence = self.sequence;
        self.sequence = self.sequence.checked_add(1).ok_or_else(|| {
            NativeWindowObserverError::Os("native observer sequence overflow".to_string())
        })?;
        let filename = std::ffi::OsString::from(format!("{operation}-{sequence}.{extension}"));
        if !self.members.insert(filename.clone()) {
            return Err(NativeWindowObserverError::Os(
                "native observer scratch member was reused".to_string(),
            ));
        }
        Ok(self.scratch_root.join(filename))
    }

    fn run_helper(
        &mut self,
        command: Command,
        operation: &str,
        sequence: u64,
    ) -> Result<(), NativeWindowObserverError> {
        let stdout_name = std::ffi::OsString::from(format!("{operation}-{sequence}.stdout.log"));
        let stderr_name = std::ffi::OsString::from(format!("{operation}-{sequence}.stderr.log"));
        if !self.members.insert(stdout_name.clone()) || !self.members.insert(stderr_name.clone()) {
            return Err(NativeWindowObserverError::Os(
                "native observer log member was reused".to_string(),
            ));
        }
        let stdout_log = self.scratch_root.join(&stdout_name);
        let stderr_log = self.scratch_root.join(&stderr_name);
        let session = ChildSession::spawn(
            command,
            ChildSessionConfig {
                stdout_log: stdout_log.clone(),
                stderr_log: stderr_log.clone(),
                stdout_budget: self.contract.observer_response_budget_bytes,
                stderr_budget: self.contract.observer_response_budget_bytes,
                timeout: self.contract.observer_timeout(),
                grace: self.contract.observer_kill_grace(),
                executable_digest: self.executable_digest.clone(),
            },
        )
        .map_err(|error| {
            NativeWindowObserverError::Os(format!("spawn native {operation} helper: {error}"))
        })?;
        let outcome = session.finish();
        let cleanup = self
            .remove_member(&stdout_name)
            .and_then(|()| self.remove_member(&stderr_name));
        if let Err(error) = cleanup {
            return Err(NativeWindowObserverError::Os(format!(
                "clean native {operation} helper logs: {error}"
            )));
        }
        match outcome {
            Ok(receipt) if receipt.exit_code == Some(0) && receipt.signal.is_none() => Ok(()),
            Ok(receipt) => Err(NativeWindowObserverError::Os(format!(
                "native {operation} helper exited with code {:?}, signal {:?}",
                receipt.exit_code, receipt.signal
            ))),
            Err(failure) => match failure.error {
                ChildSessionError::BudgetExceeded { stream } => {
                    Err(NativeWindowObserverError::OutputLimit {
                        stream: format!("{stream:?}").to_ascii_lowercase(),
                        limit_bytes: self.contract.observer_response_budget_bytes,
                    })
                }
                _ => Err(NativeWindowObserverError::Os(format!(
                    "supervise native {operation} helper: {failure}"
                ))),
            },
        }
    }

    fn verify_visible_root(&self) -> Result<(), NativeWindowObserverError> {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt as _;
        let metadata = fs::symlink_metadata(&self.scratch_root).map_err(|error| {
            NativeWindowObserverError::Os(format!(
                "inspect native observer scratch root {}: {error}",
                self.scratch_root.display()
            ))
        })?;
        if !metadata.is_dir() || (metadata.dev(), metadata.ino()) != self.scratch_identity {
            return Err(NativeWindowObserverError::Os(
                "native observer scratch root identity changed".to_string(),
            ));
        }
        Ok(())
    }

    fn member_name<'a>(
        &self,
        path: &'a Path,
    ) -> Result<&'a std::ffi::OsStr, NativeWindowObserverError> {
        if path.parent() != Some(self.scratch_root.as_path()) {
            return Err(NativeWindowObserverError::Os(
                "native observer member escaped its scratch root".to_string(),
            ));
        }
        path.file_name().ok_or_else(|| {
            NativeWindowObserverError::Os("native observer member has no filename".to_string())
        })
    }

    fn read_member(&self, path: &Path, budget: u64) -> Result<Vec<u8>, NativeWindowObserverError> {
        self.verify_visible_root()?;
        read_bounded_regular_at(&self.scratch_directory, self.member_name(path)?, budget)
    }

    fn read_json_member<T: serde::de::DeserializeOwned>(
        &self,
        path: &Path,
        budget: u64,
    ) -> Result<T, NativeWindowObserverError> {
        let bytes = self.read_member(path, budget)?;
        serde_json::from_slice(&bytes).map_err(|error| {
            NativeWindowObserverError::Os(format!("parse native observer helper response: {error}"))
        })
    }

    fn remove_path(&mut self, path: &Path) -> Result<(), NativeWindowObserverError> {
        let name = self.member_name(path)?.to_os_string();
        self.remove_member(&name)
    }

    fn remove_member(&mut self, name: &std::ffi::OsStr) -> Result<(), NativeWindowObserverError> {
        use std::os::fd::AsRawFd as _;
        use std::os::unix::ffi::OsStrExt as _;
        let name_c = std::ffi::CString::new(name.as_bytes()).map_err(|_| {
            NativeWindowObserverError::Os("native observer member contains NUL".to_string())
        })?;
        if unsafe { libc::unlinkat(self.scratch_directory.as_raw_fd(), name_c.as_ptr(), 0) } != 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::NotFound {
                return Err(NativeWindowObserverError::Os(format!(
                    "remove native observer member {}: {error}",
                    name.to_string_lossy()
                )));
            }
        }
        self.members.remove(name);
        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), NativeWindowObserverError> {
        if self.cleaned {
            return Ok(());
        }
        for member in self.members.clone() {
            self.remove_member(&member)?;
        }
        self.verify_visible_root()?;
        fs::remove_dir(&self.scratch_root).map_err(|error| {
            NativeWindowObserverError::Os(format!(
                "remove native observer scratch {}: {error}",
                self.scratch_root.display()
            ))
        })?;
        self.cleaned = true;
        Ok(())
    }

    fn finish(mut self) -> Result<(), NativeWindowObserverError> {
        self.cleanup()
    }
}

impl Drop for BoundedNativeObserver {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn read_bounded_regular(path: &Path, budget: u64) -> Result<Vec<u8>, NativeWindowObserverError> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    let file = options.open(path).map_err(|error| {
        NativeWindowObserverError::Os(format!("open native observer output: {error}"))
    })?;
    read_bounded_open_file(file, budget)
}

fn read_bounded_regular_at(
    parent: &std::os::fd::OwnedFd,
    name: &std::ffi::OsStr,
    budget: u64,
) -> Result<Vec<u8>, NativeWindowObserverError> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::ffi::OsStrExt as _;

    let name_c = std::ffi::CString::new(name.as_bytes()).map_err(|_| {
        NativeWindowObserverError::Os("native observer member contains NUL".to_string())
    })?;
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name_c.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(NativeWindowObserverError::Os(format!(
            "open native observer output {}: {}",
            name.to_string_lossy(),
            io::Error::last_os_error()
        )));
    }
    let file = fs::File::from(unsafe { std::os::fd::OwnedFd::from_raw_fd(descriptor) });
    read_bounded_open_file(file, budget)
}

fn read_bounded_open_file(
    file: fs::File,
    budget: u64,
) -> Result<Vec<u8>, NativeWindowObserverError> {
    let metadata = file.metadata().map_err(|error| {
        NativeWindowObserverError::Os(format!("inspect native observer output: {error}"))
    })?;
    if !metadata.is_file() || metadata.len() > budget {
        return Err(NativeWindowObserverError::Os(
            "native observer output violates its bounded regular-file contract".to_string(),
        ));
    }
    let read_limit = budget.checked_add(1).ok_or_else(|| {
        NativeWindowObserverError::Os("native observer output budget overflowed".to_string())
    })?;
    let mut bytes = Vec::new();
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            NativeWindowObserverError::Os(format!("read native observer output: {error}"))
        })?;
    if (bytes.len() as u64) > budget || (bytes.len() as u64) != metadata.len() {
        return Err(NativeWindowObserverError::Os(
            "native observer output changed length or exceeded its byte budget".to_string(),
        ));
    }
    Ok(bytes)
}

fn main() {
    if let Err(error) = entry() {
        eprintln!("uqm-native-acceptance: {error}");
        std::process::exit(1);
    }
}

fn entry() -> Result<(), String> {
    entry_with_arguments(&std::env::args().collect::<Vec<_>>())
}

pub fn entry_with_arguments(arguments: &[String]) -> Result<(), String> {
    match arguments {
        [_, helper, operation, pid, expected_window_id, response]
            if helper == "observer-helper" && operation == "observe" =>
        {
            let pid = pid
                .parse::<u32>()
                .map_err(|error| format!("parse observer target PID: {error}"))?;
            let expected_window_id = match expected_window_id.as_str() {
                "any" => None,
                value => Some(
                    value
                        .parse::<u64>()
                        .map_err(|error| format!("parse expected native window identity: {error}"))?,
                ),
            };
            write_observer_helper_response(
                Path::new(response),
                NativeObserverHelperResult::Observed(observe_native_window(
                    pid,
                    expected_window_id,
                )),
            )
        }
        [_, helper, operation, window_id, capture, runtime_contract, response]
            if helper == "observer-helper" && operation == "capture" =>
        {
            let window_id = window_id
                .parse::<u64>()
                .map_err(|error| format!("parse capture window identity: {error}"))?;
            let runtime_contract: NativeWindowRuntimeContract =
                serde_json::from_str(runtime_contract)
                    .map_err(|error| format!("parse native capture runtime contract: {error}"))?;
            if !runtime_contract.has_valid_deadline_order() {
                return Err("native capture runtime contract has invalid deadline order".to_string());
            }
            write_observer_helper_response(
                Path::new(response),
                NativeObserverHelperResult::Captured(capture_native_window(
                    window_id,
                    Path::new(capture),
                    runtime_contract,
                )),
            )
        }
        [_, command, executable, content, script, evidence, executable_length, executable_sha256, runtime_contract, acceptance_policy, linked_build_proof, linked_build_member_limit]
            if command == "run" =>
        {
            let executable_length = executable_length
                .parse::<u64>()
                .map_err(|error| format!("parse expected executable length: {error}"))?;
            let linked_build_member_limit = linked_build_member_limit
                .parse::<u64>()
                .map_err(|error| format!("parse linked-build member limit: {error}"))?;
            if linked_build_member_limit == 0 {
                return Err("linked-build member limit must be nonzero".to_string());
            }
            let runtime_contract: NativeWindowRuntimeContract =
                serde_json::from_str(runtime_contract)
                    .map_err(|error| format!("parse native runtime contract: {error}"))?;
            if !runtime_contract.has_valid_deadline_order() {
                return Err("native runtime contract has invalid deadline order".to_string());
            }
            let acceptance_policy: NativeAcceptancePolicy = serde_json::from_str(acceptance_policy)
                .map_err(|error| format!("parse native acceptance policy: {error}"))?;
            if !acceptance_policy.is_valid() {
                return Err("native acceptance policy is invalid".to_string());
            }
            let evidence_root = Path::new(evidence);
            run(RunInputs {
                executable: Path::new(executable),
                content: Path::new(content),
                script: Path::new(script),
                root: evidence_root,
                expected_executable_length: executable_length,
                expected_executable_sha256: executable_sha256,
                runtime_contract,
                acceptance_policy,
                linked_build_proof: Path::new(linked_build_proof),
                linked_build_member_limit,
            })?;
            println!("{}", evidence_root.join("native-acceptance.json").display());
            Ok(())
        }
        [_, command, manifest] if command == "validate-internal-consistency" => {
            let path = Path::new(manifest);
            let root = path
                .parent()
                .ok_or_else(|| "manifest has no parent directory".to_string())?;
            let bytes = read_regular_file_nofollow_bounded(path, MAX_NATIVE_MANIFEST_BYTES)?;
            let manifest: NativeAcceptanceManifest = serde_json::from_slice(&bytes)
                .map_err(|error| format!("parse manifest: {error}"))?;
            validate_native_acceptance_bundle(root, &manifest)
                .map_err(|error| format!("validate manifest: {error:?}"))
        }
        _ => Err("usage: uqm-native-acceptance <run EXECUTABLE CONTENT SCRIPT EVIDENCE_ROOT EXECUTABLE_LENGTH EXECUTABLE_SHA256 RUNTIME_CONTRACT ACCEPTANCE_POLICY LINKED_BUILD_PROOF LINKED_BUILD_MEMBER_LIMIT|validate-internal-consistency MANIFEST>".to_string()),
    }
}

fn write_observer_helper_response(
    path: &Path,
    result: NativeObserverHelperResult,
) -> Result<(), String> {
    write_json_atomic(
        path,
        &NativeObserverHelperResponse {
            schema: NATIVE_OBSERVER_HELPER_SCHEMA.to_string(),
            result,
        },
    )
}
fn bind_provisional_window(
    provisional_window_id: &mut Option<u64>,
    observed: &ObservedNativeWindow,
    client_bounds: NativeWindowBounds,
) -> Result<bool, String> {
    match *provisional_window_id {
        Some(window_id) if window_id != observed.window_id => {
            return Err("native window identity changed before its geometry converged".to_string());
        }
        Some(_) => {}
        None => *provisional_window_id = Some(observed.window_id),
    }
    Ok(observed.os_bounds.contains(client_bounds))
}

fn normalize_native_capture(
    bytes: &[u8],
    os_bounds: NativeWindowBounds,
    client_bounds: NativeWindowBounds,
    byte_budget: u64,
) -> Result<Vec<u8>, String> {
    use image::GenericImageView as _;

    if !os_bounds.contains(client_bounds) {
        return Err("native capture bounds do not contain the client bounds".to_string());
    }
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|error| format!("decode native screenshot: {error}"))?;
    let (width, height) = image.dimensions();
    if os_bounds.width == 0
        || os_bounds.height == 0
        || width % os_bounds.width != 0
        || height % os_bounds.height != 0
    {
        return Err("native screenshot dimensions do not match the observed OS bounds".to_string());
    }
    let scale = width / os_bounds.width;
    if scale == 0 || height / os_bounds.height != scale {
        return Err("native screenshot pixel density is inconsistent".to_string());
    }
    let relative_x = u32::try_from(i64::from(client_bounds.x) - i64::from(os_bounds.x))
        .map_err(|_| "native screenshot client x offset is negative".to_string())?;
    let relative_y = u32::try_from(i64::from(client_bounds.y) - i64::from(os_bounds.y))
        .map_err(|_| "native screenshot client y offset is negative".to_string())?;
    let crop_x = relative_x
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop x overflowed".to_string())?;
    let crop_y = relative_y
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop y overflowed".to_string())?;
    let crop_width = client_bounds
        .width
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop width overflowed".to_string())?;
    let crop_height = client_bounds
        .height
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop height overflowed".to_string())?;
    if crop_x
        .checked_add(crop_width)
        .is_none_or(|right| right > width)
        || crop_y
            .checked_add(crop_height)
            .is_none_or(|bottom| bottom > height)
    {
        return Err("native screenshot crop exceeds captured pixels".to_string());
    }
    let cropped = image.crop_imm(crop_x, crop_y, crop_width, crop_height);
    let normalized = if scale == 1 {
        cropped
    } else {
        cropped.resize_exact(
            client_bounds.width,
            client_bounds.height,
            image::imageops::FilterType::Lanczos3,
        )
    };
    let mut encoded = std::io::Cursor::new(Vec::new());
    normalized
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|error| format!("encode normalized native screenshot: {error}"))?;
    let encoded = encoded.into_inner();
    if encoded.is_empty() || encoded.len() as u64 > byte_budget {
        return Err("normalized native screenshot exceeds its byte budget".to_string());
    }
    Ok(encoded)
}

fn validate_playable_screenshot_difference(stable: &[u8], playable: &[u8]) -> Result<(), String> {
    let stable = image::load_from_memory(stable)
        .map_err(|error| format!("decode stable native screenshot: {error}"))?
        .to_rgba8();
    let playable = image::load_from_memory(playable)
        .map_err(|error| format!("decode playable native screenshot: {error}"))?
        .to_rgba8();
    if stable.dimensions() != playable.dimensions() {
        return Err("stable and playable native screenshots have different dimensions".to_string());
    }
    let pixel_count = u64::from(stable.width()) * u64::from(stable.height());
    if pixel_count == 0 {
        return Err("native screenshots contain no pixels".to_string());
    }
    let materially_changed = stable
        .pixels()
        .zip(playable.pixels())
        .filter(|(before, after)| {
            before.0[..3]
                .iter()
                .zip(&after.0[..3])
                .any(|(left, right)| left.abs_diff(*right) >= 24)
        })
        .count() as u64;
    let minimum_changed = pixel_count.div_ceil(100);
    if materially_changed < minimum_changed {
        return Err(format!(
            "playable screenshot differs materially at only {materially_changed}/{pixel_count} pixels; expected at least {minimum_changed}"
        ));
    }
    Ok(())
}

struct NativeObservationSession {
    backing_verification: Result<(), String>,
    state_reader: NativeWindowStateReader,
    ack_publisher: NativeWindowAckPublisher,

    screenshots: PathBuf,
    evidence_root: PathBuf,
    nonce: String,
    requested_bounds: NativeWindowBounds,
    acceptance_policy: NativeAcceptancePolicy,
    proof: NativeWindowProof,
    last_presentation: u64,
    provisional_window_id: Option<u64>,
    first_visible_presentation: Option<u64>,
    recorded_input_events: u64,
    recorded_battle_frames: u64,
    playable_captured: bool,
    publications: Vec<NativeWindowPublication>,
}

impl NativeObservationSession {
    fn acknowledge_publication(&mut self, state: &NativeWindowChildState) -> io::Result<()> {
        self.ack_publisher
            .acknowledge(&self.nonce, state.committed_presentation)
            .map_err(io::Error::other)?;
        self.publications.push(NativeWindowPublication {
            acknowledgement: NativeWindowAck {
                schema: NATIVE_WINDOW_ACK_SCHEMA.to_string(),
                nonce: self.nonce.clone(),
                committed_presentation: state.committed_presentation,
            },
            state: state.clone(),
        });
        self.last_presentation = state.committed_presentation;
        Ok(())
    }

    fn observe(
        &mut self,
        identity: &ProcessIdentity,
        observer: &mut BoundedNativeObserver,
    ) -> io::Result<()> {
        if let Err(error) = &self.backing_verification {
            return Err(io::Error::other(error.clone()));
        }
        let Some(state) = self
            .state_reader
            .read_if_present(&self.nonce, identity.pid, self.requested_bounds)
            .map_err(io::Error::other)?
        else {
            return Ok(());
        };
        if state.committed_presentation == self.last_presentation {
            return Ok(());
        }
        if self.last_presentation.checked_add(1) != Some(state.committed_presentation) {
            return Err(io::Error::other(format!(
                "native presentation sequence skipped from {} to {}",
                self.last_presentation, state.committed_presentation
            )));
        }
        if let Some(first_visible) = self.first_visible_presentation {
            let post_visible = state
                .committed_presentation
                .checked_sub(first_visible)
                .ok_or_else(|| io::Error::other("presentation regressed before visibility"))?;
            let needs_os_observation = post_visible
                == self.acceptance_policy.stable_presentation_floor
                || (post_visible >= self.acceptance_policy.playable_presentation_floor
                    && !self.playable_captured
                    && state.semantic.accepted_player_inputs > 0
                    && state.semantic.verified_battle_frames
                        >= self.acceptance_policy.battle_frame_floor);
            if !needs_os_observation {
                if state.semantic.accepted_player_inputs < self.recorded_input_events
                    || state.semantic.verified_battle_frames < self.recorded_battle_frames
                {
                    return Err(io::Error::other(
                        "native semantic snapshot counters regressed",
                    ));
                }
                self.recorded_input_events = state.semantic.accepted_player_inputs;
                self.recorded_battle_frames = state.semantic.verified_battle_frames;
                return self.acknowledge_publication(&state);
            }
        }
        let expected_window_id = self.proof.bound_window_id().or(self.provisional_window_id);
        let observed = match observer.observe(identity.pid, expected_window_id) {
            Ok(observed) => observed,
            Err(NativeWindowObserverError::NotFound { .. })
                if self.first_visible_presentation.is_none() =>
            {
                return Ok(());
            }
            Err(error) => return Err(io::Error::other(error)),
        };
        if !observed.visible || observed.minimized {
            if self.first_visible_presentation.is_some() {
                return Err(io::Error::other(
                    "native window lost visibility after the first visible presentation",
                ));
            }
            return self.acknowledge_publication(&state);
        }
        if self.first_visible_presentation.is_none()
            && !bind_provisional_window(
                &mut self.provisional_window_id,
                &observed,
                state.client_bounds,
            )
            .map_err(io::Error::other)?
        {
            return self.acknowledge_publication(&state);
        }
        let binding = NativeWindowBinding {
            process: native_identity(identity, &self.nonce),
            window_id: observed.window_id,
            client_bounds: state.client_bounds,
            os_bounds: observed.os_bounds,
        };
        if state.semantic.accepted_player_inputs < self.recorded_input_events
            || state.semantic.verified_battle_frames < self.recorded_battle_frames
        {
            return Err(io::Error::other(
                "native semantic snapshot counters regressed",
            ));
        }
        self.recorded_input_events = state.semantic.accepted_player_inputs;
        self.recorded_battle_frames = state.semantic.verified_battle_frames;
        self.proof
            .observe_visible(NativeWindowObservation {
                binding: binding.clone(),
                committed_presentation: state.committed_presentation,
                visible: observed.visible,
                minimized: observed.minimized,
                semantic: state.semantic,
            })
            .map_err(|error| {
                io::Error::other(format!("record native-window observation: {error:?}"))
            })?;
        let first = *self
            .first_visible_presentation
            .get_or_insert(state.committed_presentation);
        let post_visible = state
            .committed_presentation
            .checked_sub(first)
            .ok_or_else(|| io::Error::other("presentation regressed before visibility"))?;
        let stage = if post_visible == self.acceptance_policy.stable_presentation_floor {
            Some(NativeScreenshotStage::Stable)
        } else if post_visible >= self.acceptance_policy.playable_presentation_floor
            && !self.playable_captured
            && self.recorded_input_events > 0
            && self.recorded_battle_frames >= self.acceptance_policy.battle_frame_floor
        {
            Some(NativeScreenshotStage::Playable)
        } else {
            None
        };
        if let Some(stage) = stage {
            self.capture_stage(
                observer,
                &observed,
                binding,
                state.committed_presentation,
                state.semantic,
                stage,
            )?;
        }
        self.acknowledge_publication(&state)
    }

    fn validate_post_capture_window(
        before: &ObservedNativeWindow,
        after: &ObservedNativeWindow,
    ) -> Result<(), String> {
        if !after.visible || after.minimized {
            return Err("native window lost compositor visibility during capture".to_string());
        }
        if after.window_id != before.window_id {
            return Err("native window identity changed during capture".to_string());
        }
        if after.os_bounds != before.os_bounds {
            return Err("native window bounds changed during capture".to_string());
        }
        Ok(())
    }

    fn capture_stage(
        &mut self,
        observer: &mut BoundedNativeObserver,
        observed: &ObservedNativeWindow,
        binding: NativeWindowBinding,
        committed_presentation: u64,
        semantic: uqm_rust::automation::NativeWindowSemanticSnapshot,
        stage: NativeScreenshotStage,
    ) -> io::Result<()> {
        let filename = match stage {
            NativeScreenshotStage::Stable => "stable.png",
            NativeScreenshotStage::Playable => "playable.png",
        };
        let path = self.screenshots.join(filename);
        let capture_budget = observer.contract.capture_budget_bytes;
        let captured = observer
            .capture(observed.window_id)
            .map_err(io::Error::other)?;
        let bytes = normalize_native_capture(
            &captured,
            observed.os_bounds,
            binding.client_bounds,
            capture_budget,
        )
        .map_err(io::Error::other)?;
        if stage == NativeScreenshotStage::Playable {
            let stable_path = self.screenshots.join("stable.png");
            let stable = read_regular_file_nofollow_bounded(&stable_path, capture_budget)
                .map_err(io::Error::other)?;
            validate_playable_screenshot_difference(&stable, &bytes).map_err(io::Error::other)?;
        }
        let post_capture = observer
            .observe(binding.process.pid, Some(observed.window_id))
            .map_err(io::Error::other)?;
        Self::validate_post_capture_window(observed, &post_capture).map_err(io::Error::other)?;
        let post_capture_observation = NativeWindowObservation {
            binding: NativeWindowBinding {
                process: binding.process.clone(),
                window_id: post_capture.window_id,
                client_bounds: binding.client_bounds,
                os_bounds: post_capture.os_bounds,
            },
            committed_presentation,
            visible: post_capture.visible,
            minimized: post_capture.minimized,
            semantic,
        };
        write_bytes_atomic_noclobber(&path, &bytes).map_err(io::Error::other)?;
        let screenshot_input =
            retained_input(&self.evidence_root, &path).map_err(io::Error::other)?;
        self.proof
            .record_screenshot(NativeScreenshot {
                stage,
                binding,
                post_capture_observation,
                committed_presentation,
                input_events: self.recorded_input_events,
                trace_record_count: semantic.trace_record_count,
                battle_frames: self.recorded_battle_frames,
                relative_path: screenshot_input.relative_path,
                byte_length: screenshot_input.byte_length,
                sha256: screenshot_input.sha256,
            })
            .map_err(|error| io::Error::other(format!("record native screenshot: {error:?}")))?;
        self.playable_captured = stage == NativeScreenshotStage::Playable;
        Ok(())
    }
}

fn validate_run_paths(executable: &Path, content: &Path, script: &Path) -> Result<PathBuf, String> {
    for (label, path) in [
        ("executable", executable),
        ("content", content),
        ("script", script),
    ] {
        if !path.is_absolute() {
            return Err(format!("{label} path must be absolute: {}", path.display()));
        }
    }
    for (label, path) in [("executable", executable), ("script", script)] {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("inspect {label} {}: {error}", path.display()))?;
        if !metadata.file_type().is_file() {
            return Err(format!("{label} is not a regular file: {}", path.display()));
        }
    }
    let content_metadata = fs::symlink_metadata(content)
        .map_err(|error| format!("inspect content {}: {error}", content.display()))?;
    if !content_metadata.file_type().is_dir() {
        return Err(format!("content is not a directory: {}", content.display()));
    }
    fs::canonicalize(content)
        .map_err(|error| format!("canonicalize content {}: {error}", content.display()))
}

#[derive(Clone, Copy)]
struct ChildTerminalOutcome {
    exit_code: Option<i32>,
    signal: Option<i32>,
    term_sent: bool,
    kill_sent: bool,
}

fn runtime_failure(
    session_error: Option<(&ChildSessionError, String)>,
    child: ChildTerminalOutcome,
    observer_result: Result<(), String>,
    config_root_removed: bool,
    materialized_content_removed: bool,
) -> Result<Option<(NativeAcceptanceFailureContract, String)>, String> {
    let clean_child = clean_child_outcome(
        child.exit_code,
        child.signal,
        child.term_sent,
        child.kill_sent,
    );
    let observer_cleanup_error = observer_result.err();
    if let Some((error_kind, mut error)) = session_error {
        if let Some(cleanup) = observer_cleanup_error {
            error.push_str(&format!("; observer cleanup failed: {cleanup}"));
        }
        return match error_kind {
            ChildSessionError::Observer(_) => {
                Ok(Some((NativeAcceptanceFailureContract::Observer, error)))
            }
            _ => Ok(Some((
                NativeAcceptanceFailureContract::ChildSupervision,
                error,
            ))),
        };
    }
    if let Some((contract, mut error)) = child_exit_failure(child.exit_code, child.signal) {
        if let Some(cleanup) = observer_cleanup_error {
            error.push_str(&format!("; observer cleanup failed: {cleanup}"));
        }
        return Ok(Some((contract, error)));
    }
    if let Some(error) = observer_cleanup_error {
        return Ok(Some((NativeAcceptanceFailureContract::Observer, error)));
    }
    if !config_root_removed {
        return if clean_child {
            Ok(Some((
                NativeAcceptanceFailureContract::ConfigCleanup,
                "isolated config root was not removed".to_string(),
            )))
        } else {
            Err("config cleanup failure cannot produce a clean failure receipt".to_string())
        };
    }
    if !materialized_content_removed {
        return if clean_child {
            Ok(Some((
                NativeAcceptanceFailureContract::MaterializedContentCleanup,
                "materialized content was not removed".to_string(),
            )))
        } else {
            Err("content cleanup failure cannot produce a clean failure receipt".to_string())
        };
    }
    Ok(None)
}

fn clean_child_outcome(
    exit_code: Option<i32>,
    signal: Option<i32>,
    term_sent: bool,
    kill_sent: bool,
) -> bool {
    matches!(
        (exit_code, signal, term_sent, kill_sent),
        (Some(0), None, false, false)
    )
}

#[cfg(target_os = "macos")]
fn launchd_manager_value(subcommand: &str) -> Result<String, String> {
    let output = Command::new("/bin/launchctl")
        .arg(subcommand)
        .env_clear()
        .output()
        .map_err(|error| format!("cannot query launchd {subcommand}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "launchctl {subcommand} failed with status {}",
            output.status
        ));
    }
    std::str::from_utf8(&output.stdout)
        .map(str::trim)
        .map(str::to_string)
        .map_err(|error| format!("launchd {subcommand} is not UTF-8: {error}"))
}

#[cfg(target_os = "macos")]
fn native_execution_identity() -> Result<NativeExecutionIdentity, String> {
    let launchd_manager_uid = launchd_manager_value("manageruid")?
        .parse::<u32>()
        .map_err(|error| format!("launchd manager UID is invalid: {error}"))?;
    // SAFETY: getuid and geteuid have no preconditions.
    Ok(NativeExecutionIdentity {
        real_uid: unsafe { libc::getuid() },
        effective_uid: unsafe { libc::geteuid() },
        launchd_manager_uid,
        launchd_manager_name: launchd_manager_value("managername")?,
    })
}

#[cfg(not(target_os = "macos"))]
fn native_execution_identity() -> Result<NativeExecutionIdentity, String> {
    Err("native execution identity requires the macOS Aqua session".to_string())
}

fn finish_observer(observer: BoundedNativeObserver) -> Result<(), String> {
    observer
        .finish()
        .map_err(|error| format!("finish native observer: {error}"))
}

struct AcceptanceManifestInputs<'a> {
    command: &'a [String],
    executable: &'a NativeRetainedInput,
    script: &'a NativeRetainedInput,
    content_package: &'a NativeRetainedInput,
    runtime_contract: NativeWindowRuntimeContract,
    acceptance_policy: NativeAcceptancePolicy,
    child: &'a NativeChildCleanupReceipt,
}

fn build_acceptance_manifest(
    root: &Path,
    automation: &Path,
    observation: NativeObservationSession,
    inputs: AcceptanceManifestInputs<'_>,
) -> Result<NativeAcceptanceManifest, String> {
    let trace_path = automation.join("trace.jsonl");
    let (input_events, battle_frames) = trace_semantics(
        &trace_path,
        inputs.runtime_contract.inventory_limits.member_bytes,
    )?;
    if input_events != observation.recorded_input_events
        || battle_frames != observation.recorded_battle_frames
    {
        return Err("final semantic evidence contradicts the final observed snapshot".to_string());
    }
    let trace_input = retained_input(root, &trace_path)?;
    let manifest = NativeAcceptanceManifest {
        schema: NATIVE_ACCEPTANCE_SCHEMA.to_string(),
        command: inputs.command.to_vec(),
        environment: BTreeMap::from([("SDL_AUDIODRIVER".to_string(), "dummy".to_string())]),
        execution_identity: native_execution_identity()?,
        executable: inputs.executable.clone(),
        script: inputs.script.clone(),
        content_package: inputs.content_package.clone(),
        runtime_contract: inputs.runtime_contract,
        acceptance_policy: inputs.acceptance_policy,
        retained_files: native_acceptance_inventory(root, inputs.runtime_contract.inventory_limits)
            .map_err(|error| format!("inventory native acceptance evidence: {error:?}"))?,
        trace_path: trace_input.relative_path,
        trace_byte_length: trace_input.byte_length,
        trace_sha256: trace_input.sha256,
        child: inputs.child.clone(),
        window: observation
            .proof
            .finish()
            .map_err(|error| format!("finish native-window proof: {error:?}"))?,
        publications: observation.publications,
        passed: true,
    };
    validate_native_acceptance_bundle(root, &manifest)
        .map_err(|error| format!("self-validate native acceptance: {error:?}"))?;
    Ok(manifest)
}

struct RunInputs<'a> {
    executable: &'a Path,
    content: &'a Path,
    script: &'a Path,
    root: &'a Path,
    expected_executable_length: u64,
    expected_executable_sha256: &'a str,
    runtime_contract: NativeWindowRuntimeContract,
    acceptance_policy: NativeAcceptancePolicy,
    linked_build_proof: &'a Path,
    linked_build_member_limit: u64,
}
struct FinalizeRun<'a> {
    root: &'a Path,
    automation: &'a Path,
    config_root: &'a Path,
    command: &'a [String],
    executable: &'a NativeRetainedInput,
    script: &'a NativeRetainedInput,
    content_package: &'a NativeRetainedInput,
    runtime_contract: NativeWindowRuntimeContract,
    acceptance_policy: NativeAcceptancePolicy,
    observation: NativeObservationSession,
    materialized_content: MaterializedContent,
    session_result: Result<ChildSessionReceipt, ChildSessionFailure>,
    observer_result: Result<(), String>,
}

fn setup_failure_contract(child_spawn_attempted: bool) -> NativeAcceptanceSetupFailureContract {
    if child_spawn_attempted {
        NativeAcceptanceSetupFailureContract::ChildSpawn
    } else {
        NativeAcceptanceSetupFailureContract::Preparation
    }
}

fn run(inputs: RunInputs<'_>) -> Result<(), String> {
    let RunInputs {
        executable,
        content,
        script,
        root,
        expected_executable_length,
        expected_executable_sha256,
        runtime_contract,
        acceptance_policy,
        linked_build_proof,
        linked_build_member_limit,
    } = inputs;
    if !cfg!(target_os = "macos") {
        return Err("native-window acceptance is unsupported on this platform".to_string());
    }
    if !root.is_absolute() {
        return Err("evidence root must be absolute".to_string());
    }
    let precreated_root = matches!(
        std::env::var("UQM_CI_NATIVE_ACCEPTANCE_PRECREATED_ROOT").as_deref(),
        Ok("1")
    );
    let fresh_root = create_fresh_root(root, precreated_root)?;
    let root = fresh_root.path();
    let mut child_spawned = false;
    let mut child_spawn_attempted = false;
    let result = (|| -> Result<(), String> {
        let content = validate_run_paths(executable, content, script)?;
        let executable = fs::canonicalize(executable).map_err(|error| {
            format!("canonicalize executable {}: {error}", executable.display())
        })?;
        let script = fs::canonicalize(script)
            .map_err(|error| format!("canonicalize script {}: {error}", script.display()))?;
        let content_package = find_content_package(&content)?;
        let inputs = root.join("inputs");
        let content_root = inputs.join("content");
        let content_packages = content_root.join("packages");
        let linked_build = inputs.join("linked-build");
        let automation = root.join("automation");
        let screenshots = root.join("screenshots");
        let config_root = root.join("config");
        for directory in [
            &inputs,
            &content_root,
            &content_packages,
            &linked_build,
            &automation,
            &screenshots,
            &config_root,
        ] {
            fs::create_dir_all(directory)
                .map_err(|error| format!("create {}: {error}", directory.display()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                fs::set_permissions(directory, fs::Permissions::from_mode(0o750))
                    .map_err(|error| format!("publish {}: {error}", directory.display()))?;
            }
        }

        let retained_executable = inputs.join("uqm");
        let retained_script = inputs.join("linked-playable-v1.json");
        let content_filename = content_package
            .file_name()
            .ok_or_else(|| "content package has no filename".to_string())?;
        let content_version = content_version_from_filename(content_filename)?;
        let retained_content = content_packages.join(content_filename);
        copy_executable_bounded(&executable, &retained_executable, linked_build_member_limit)?;
        copy_regular_bounded(&script, &retained_script, linked_build_member_limit)?;
        copy_regular_bounded(
            &content_package,
            &retained_content,
            linked_build_member_limit,
        )?;
        write_bytes_atomic_noclobber(
            &content_root.join("version"),
            format!("{content_version}\n").as_bytes(),
        )?;
        let executable_input = retained_input(root, &retained_executable)?;
        let script_input = retained_input(root, &retained_script)?;
        let content_input = retained_input(root, &retained_content)?;
        let script_bytes =
            read_regular_file_nofollow_bounded(&retained_script, linked_build_member_limit)?;
        retain_linked_build_proof(
            linked_build_proof,
            &linked_build,
            &executable_input,
            linked_build_member_limit,
        )?;
        let document = parse_script(&script_bytes, &retained_script)
            .map_err(|error| format!("parse script: {error}"))?;
        validate_script(document, &retained_script)
            .map_err(|error| format!("validate script: {error}"))?;

        validate_expected_executable(
            &executable_input,
            expected_executable_length,
            expected_executable_sha256,
        )?;
        let materialized_content = materialize_content_package(
            &retained_content,
            &content_root,
            runtime_contract.content_expansion_budget_bytes,
            Some(&content_input),
        )?;
        let nonce = fresh_nonce()?;
        let requested_bounds = runtime_contract.expected_client_bounds;
        if requested_bounds.width
            != u32::try_from(DEFAULT_RESOLUTION_WIDTH)
                .map_err(|_| "default resolution width is invalid".to_string())?
            || requested_bounds.height
                != u32::try_from(DEFAULT_RESOLUTION_HEIGHT)
                    .map_err(|_| "default resolution height is invalid".to_string())?
        {
            return Err("authority native bounds differ from the linked runtime resolution".into());
        }
        let proof_config = root.join("native-window-proof.json");
        write_json_atomic(
            &proof_config,
            &NativeWindowConfigFile {
                schema: NATIVE_WINDOW_CONFIG_SCHEMA.to_string(),
                nonce: nonce.clone(),
                client_bounds: requested_bounds,
                runtime_contract,
                acceptance_policy,
            },
        )?;

        let ack_publisher =
            NativeWindowAckPublisher::bind(&automation.join("native-window-ack.json"))?;
        let state_reader = NativeWindowStateReader::bind(
            &automation.join("native-window-state.json"),
            runtime_contract.observer_response_budget_bytes,
        )?;
        let mut observer = BoundedNativeObserver::spawn(&nonce, runtime_contract)?;
        let command_arguments = vec![
            retained_executable.to_string_lossy().into_owned(),
            format!("--configdir={}", config_root.display()),
            format!("--contentdir={}", content_root.display()),
            format!("--automation-script={}", retained_script.display()),
            format!("--automation-output={}", automation.display()),
            format!("--native-window-proof={}", proof_config.display()),
        ];
        let mut command = Command::new(&retained_executable);
        command
            .args(&command_arguments[1..])
            .env_clear()
            .env("SDL_AUDIODRIVER", "dummy");
        child_spawn_attempted = true;
        let session = ChildSession::spawn(
            command,
            ChildSessionConfig {
                stdout_log: root.join("stdout.log"),
                stderr_log: root.join("stderr.log"),
                stdout_budget: runtime_contract.child_stdout_budget_bytes,
                stderr_budget: runtime_contract.child_stderr_budget_bytes,
                timeout: runtime_contract.outer_child_timeout(),
                grace: runtime_contract.outer_child_kill_grace(),
                executable_digest: executable_input.sha256.clone(),
            },
        )
        .map_err(|error| format!("spawn exact linked executable: {error}"))?;
        child_spawned = true;
        let child_identity = native_identity(session.identity(), &nonce);
        let mut observation = NativeObservationSession {
            backing_verification: verify_spawned_executable(
                session.identity(),
                &retained_executable,
                &executable_input,
            ),
            state_reader,
            ack_publisher,
            screenshots,
            evidence_root: root.to_path_buf(),
            nonce: nonce.clone(),
            requested_bounds,
            acceptance_policy,
            proof: NativeWindowProof::new(child_identity, requested_bounds, acceptance_policy),
            last_presentation: 0,
            provisional_window_id: None,
            first_visible_presentation: None,
            recorded_input_events: 0,
            recorded_battle_frames: 0,
            playable_captured: false,
            publications: Vec::new(),
        };
        let session_result =
            session.finish_observing(|identity| observation.observe(identity, &mut observer));

        let observer_result = finish_observer(observer);
        finalize_run(FinalizeRun {
            root,
            automation: &automation,
            config_root: &config_root,
            command: &command_arguments,
            executable: &executable_input,
            script: &script_input,
            content_package: &content_input,
            runtime_contract,
            acceptance_policy,
            observation,
            materialized_content,
            session_result,
            observer_result,
        })
    })();
    if let Err(error) = result {
        if !child_spawned {
            if let Err(publication) = publish_setup_failure(
                root,
                expected_executable_length,
                expected_executable_sha256,
                runtime_contract,
                acceptance_policy,
                child_spawn_attempted,
                &error,
            ) {
                return Err(format!(
                    "{error}; setup failure publication failed: {publication}"
                ));
            }
        }
        return Err(error);
    }
    Ok(())
}

fn publish_setup_failure(
    root: &Path,
    expected_executable_byte_length: u64,
    expected_executable_sha256: &str,
    runtime_contract: NativeWindowRuntimeContract,
    acceptance_policy: NativeAcceptancePolicy,
    child_spawn_attempted: bool,
    error: &str,
) -> Result<(), String> {
    let config_root = root.join("config");
    if config_root.exists() {
        fs::remove_dir_all(&config_root)
            .map_err(|cleanup| format!("setup cleanup failed: {cleanup}"))?;
    }
    let retained_files =
        native_acceptance_failure_inventory(root, runtime_contract.inventory_limits)
            .map_err(|inventory| format!("inventory native setup failure: {inventory:?}"))?;
    let manifest = NativeAcceptanceSetupFailureManifest {
        schema: NATIVE_ACCEPTANCE_SETUP_FAILURE_SCHEMA.to_string(),
        command: std::env::args().collect(),
        expected_executable_byte_length,
        expected_executable_sha256: expected_executable_sha256.to_string(),
        runtime_contract,
        acceptance_policy,
        retained_files,
        failure_contract: setup_failure_contract(child_spawn_attempted),
        error: error.to_string(),
        passed: false,
    };
    validate_native_acceptance_setup_failure_bundle(root, &manifest)
        .map_err(|validation| format!("self-validate native setup failure: {validation:?}"))?;
    write_json_atomic(&root.join("native-acceptance-failure.json"), &manifest)
}

fn finalize_run(inputs: FinalizeRun<'_>) -> Result<(), String> {
    let FinalizeRun {
        root,
        automation,
        config_root,
        command,
        executable,
        script,
        content_package,
        runtime_contract,
        acceptance_policy,
        observation,
        mut materialized_content,
        session_result,
        observer_result,
    } = inputs;
    let content_cleanup_result = materialized_content.cleanup();
    let cleanup_result = fs::remove_dir_all(config_root);
    let config_root_removed = cleanup_result.is_ok() && !config_root.exists();
    let materialized_content_removed =
        content_cleanup_result.is_ok() && materialized_content.is_removed().unwrap_or(false);
    let (session_receipt, session_error) = match session_result {
        Ok(receipt) => (receipt, None),
        Err(failure) => {
            let error = format!("{failure}");
            (failure.receipt, Some((failure.error, error)))
        }
    };
    let child_cleanup = NativeChildCleanupReceipt {
        process: native_identity(&session_receipt.identity, &observation.nonce),
        exit_code: session_receipt.exit_code,
        signal: session_receipt.signal,
        term_sent: session_receipt.term_sent,
        kill_sent: session_receipt.kill_sent,
        stdout_bytes: session_receipt.stdout_bytes,
        stderr_bytes: session_receipt.stderr_bytes,
        output_drained: session_receipt.output_drained,
        initial_process_group_empty: session_receipt.orphan_check_passed,
        config_root_removed,
        materialized_content_removed,
    };
    let failure_context = FailureManifestContext {
        root,
        command,
        executable,
        script,
        content_package,
        runtime_contract,
        acceptance_policy,
        child: &child_cleanup,
    };
    let failure = runtime_failure(
        session_error
            .as_ref()
            .map(|(error_kind, error)| (error_kind, error.clone())),
        ChildTerminalOutcome {
            exit_code: session_receipt.exit_code,
            signal: session_receipt.signal,
            term_sent: session_receipt.term_sent,
            kill_sent: session_receipt.kill_sent,
        },
        observer_result,
        config_root_removed,
        materialized_content_removed,
    )?;
    if let Some((contract, error)) = failure {
        let manifest_path = publish_failure_manifest(&failure_context, contract, &error)?;
        return Err(format!("{error}; evidence: {}", manifest_path.display()));
    }
    let acceptance_result = build_acceptance_manifest(
        root,
        automation,
        observation,
        AcceptanceManifestInputs {
            command,
            executable,
            script,
            content_package,
            runtime_contract,
            acceptance_policy,
            child: &child_cleanup,
        },
    );
    let manifest = match acceptance_result {
        Ok(manifest) => manifest,
        Err(error) => {
            let manifest_path = publish_failure_manifest(
                &failure_context,
                NativeAcceptanceFailureContract::Semantic,
                &error,
            )?;
            return Err(format!("{error}; evidence: {}", manifest_path.display()));
        }
    };
    write_json_atomic(&root.join("native-acceptance.json"), &manifest)
}

fn child_exit_failure(
    exit_code: Option<i32>,
    signal: Option<i32>,
) -> Option<(NativeAcceptanceFailureContract, String)> {
    match (exit_code, signal) {
        (Some(0), None) => None,
        (Some(code), None) => Some((
            NativeAcceptanceFailureContract::ChildExit,
            format!("native child exited with status {code}"),
        )),
        (None, Some(signal)) => Some((
            NativeAcceptanceFailureContract::ChildExit,
            format!("native child terminated by signal {signal}"),
        )),
        _ => Some((
            NativeAcceptanceFailureContract::ChildSupervision,
            "native child produced a contradictory terminal state".to_string(),
        )),
    }
}

struct FailureManifestContext<'a> {
    root: &'a Path,
    command: &'a [String],
    executable: &'a NativeRetainedInput,
    script: &'a NativeRetainedInput,
    content_package: &'a NativeRetainedInput,
    runtime_contract: NativeWindowRuntimeContract,
    acceptance_policy: NativeAcceptancePolicy,
    child: &'a NativeChildCleanupReceipt,
}

fn publish_failure_manifest(
    context: &FailureManifestContext<'_>,
    contract: NativeAcceptanceFailureContract,
    error: &str,
) -> Result<PathBuf, String> {
    let manifest = NativeAcceptanceFailureManifest {
        schema: NATIVE_ACCEPTANCE_FAILURE_SCHEMA.to_string(),
        command: context.command.to_vec(),
        environment: BTreeMap::from([("SDL_AUDIODRIVER".to_string(), "dummy".to_string())]),
        executable: context.executable.clone(),
        script: context.script.clone(),
        content_package: context.content_package.clone(),
        runtime_contract: context.runtime_contract,
        acceptance_policy: context.acceptance_policy,
        retained_files: native_acceptance_failure_inventory(
            context.root,
            context.runtime_contract.inventory_limits,
        )
        .map_err(|cause| format!("inventory failed native acceptance failure: {cause:?}"))?,
        child: context.child.clone(),
        failure_contract: contract,
        error: error.to_string(),
        passed: false,
    };
    validate_native_acceptance_failure_bundle(context.root, &manifest)
        .map_err(|cause| format!("self-validate failed native acceptance: {cause:?}"))?;
    let manifest_path = context.root.join("native-acceptance-failure.json");
    write_json_atomic(&manifest_path, &manifest)?;
    Ok(manifest_path)
}

fn find_content_package(content: &Path) -> Result<PathBuf, String> {
    let mut packages = Vec::new();
    for entry in fs::read_dir(content)
        .map_err(|error| format!("read content directory {}: {error}", content.display()))?
    {
        let entry = entry.map_err(|error| format!("read content entry: {error}"))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("inspect content entry {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!(
                "content directory contains a non-regular entry: {}",
                path.display()
            ));
        }
        if path.extension().and_then(|value| value.to_str()) == Some("uqm") {
            packages.push(path);
        }
    }
    if packages.len() != 1 {
        return Err(format!(
            "content directory must contain exactly one .uqm package, found {}",
            packages.len()
        ));
    }
    Ok(packages.remove(0))
}

fn content_version_from_filename(filename: &std::ffi::OsStr) -> Result<&str, String> {
    let filename = filename
        .to_str()
        .ok_or_else(|| "content package filename is not UTF-8".to_string())?;
    let version = filename
        .strip_prefix("uqm-")
        .and_then(|value| value.strip_suffix("-content.uqm"))
        .ok_or_else(|| "content package filename does not encode its version".to_string())?;
    if version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Err("content package filename has an invalid version".to_string());
    }
    Ok(version)
}

#[derive(Clone, Copy)]
enum MaterializedEntryKind {
    File,
    Directory,
}

struct MaterializedEntry {
    relative_path: PathBuf,
    kind: MaterializedEntryKind,
}

struct MaterializedContent {
    root: std::os::fd::OwnedFd,
    entries: Vec<MaterializedEntry>,
    top_level: std::collections::BTreeSet<std::ffi::OsString>,
}

impl MaterializedContent {
    fn cleanup(&mut self) -> Result<(), String> {
        use std::os::fd::AsRawFd as _;

        while let Some(entry) = self.entries.pop() {
            let removal = (|| -> Result<(), String> {
                let (parent, filename) = open_relative_parent(&self.root, &entry.relative_path)?;
                let flags = match entry.kind {
                    MaterializedEntryKind::File => 0,
                    MaterializedEntryKind::Directory => libc::AT_REMOVEDIR,
                };
                if unsafe { libc::unlinkat(parent.as_raw_fd(), filename.as_ptr(), flags) } != 0 {
                    return Err(format!(
                        "remove materialized content {}: {}",
                        entry.relative_path.display(),
                        io::Error::last_os_error()
                    ));
                }
                Ok(())
            })();
            if let Err(error) = removal {
                self.entries.push(entry);
                return Err(error);
            }
        }
        Ok(())
    }

    fn is_removed(&self) -> Result<bool, String> {
        use std::os::fd::AsRawFd as _;
        use std::os::unix::ffi::OsStrExt as _;

        for name in &self.top_level {
            let name = std::ffi::CString::new(name.as_bytes())
                .map_err(|_| "materialized top-level name contains a NUL byte".to_string())?;
            let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
            if unsafe {
                libc::fstatat(
                    self.root.as_raw_fd(),
                    name.as_ptr(),
                    status.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } == 0
            {
                return Ok(false);
            }
            if io::Error::last_os_error().raw_os_error() != Some(libc::ENOENT) {
                return Err(format!(
                    "inspect materialized content {}: {}",
                    name.to_string_lossy(),
                    io::Error::last_os_error()
                ));
            }
        }
        Ok(true)
    }
}

impl Drop for MaterializedContent {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn materialize_content_package(
    package: &Path,
    content_root: &Path,
    budget_bytes: u64,
    expected: Option<&NativeRetainedInput>,
) -> Result<MaterializedContent, String> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut package_file = options
        .open(package)
        .map_err(|error| format!("open content package {}: {error}", package.display()))?;
    let metadata = package_file
        .metadata()
        .map_err(|error| format!("inspect content package {}: {error}", package.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "content package is not a regular file: {}",
            package.display()
        ));
    }
    if let Some(expected) = expected {
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = package_file
                .read(&mut buffer)
                .map_err(|error| format!("hash content package {}: {error}", package.display()))?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        if metadata.len() != expected.byte_length
            || format!("{:x}", digest.finalize()) != expected.sha256
        {
            return Err("retained content package changed before archive parsing".to_string());
        }
        package_file
            .rewind()
            .map_err(|error| format!("rewind content package {}: {error}", package.display()))?;
    }
    let mut archive = zip::ZipArchive::new(package_file)
        .map_err(|error| format!("parse content package {}: {error}", package.display()))?;
    let root = open_directory_path_nofollow(content_root)?;
    let mut materialized = MaterializedContent {
        root,
        entries: Vec::new(),
        top_level: std::collections::BTreeSet::new(),
    };
    let extraction = extract_archive(&mut archive, &mut materialized, budget_bytes);
    if let Err(error) = extraction {
        return match materialized.cleanup() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!(
                "{error}; partial content cleanup failed: {cleanup}"
            )),
        };
    }
    Ok(materialized)
}

fn extract_archive(
    archive: &mut zip::ZipArchive<fs::File>,
    materialized: &mut MaterializedContent,
    budget_bytes: u64,
) -> Result<(), String> {
    if archive.len() > MAX_CONTENT_ARCHIVE_ENTRIES {
        return Err(format!(
            "content package contains more than {MAX_CONTENT_ARCHIVE_ENTRIES} entries"
        ));
    }
    let mut declared_expanded = 0_u64;
    let mut written_expanded = 0_u64;
    let mut name_bytes = 0_u64;
    let mut seen = std::collections::BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("read content package entry {index}: {error}"))?;
        let entry_name_bytes = entry.name_raw().len();
        if entry_name_bytes > MAX_CONTENT_ARCHIVE_ENTRY_NAME_BYTES {
            return Err(format!(
                "content package entry {index} has an overlong name"
            ));
        }
        name_bytes = name_bytes
            .checked_add(entry_name_bytes as u64)
            .ok_or_else(|| "content package entry-name length overflowed".to_string())?;
        if name_bytes > MAX_CONTENT_ARCHIVE_NAME_BYTES {
            return Err(format!(
                "content package entry names exceed {MAX_CONTENT_ARCHIVE_NAME_BYTES} bytes"
            ));
        }
        let relative = validate_archive_entry(index, &entry)?;
        if !seen.insert(relative.clone()) {
            return Err(format!(
                "content package entry {index} duplicates an earlier path"
            ));
        }
        declared_expanded = declared_expanded
            .checked_add(entry.size())
            .ok_or_else(|| "content package expanded length overflowed".to_string())?;
        if declared_expanded > budget_bytes {
            return Err(format!(
                "content package exceeds expansion budget {budget_bytes}"
            ));
        }
        let first = relative
            .components()
            .next()
            .ok_or_else(|| format!("content package entry {index} has an empty path"))?;
        materialized
            .top_level
            .insert(first.as_os_str().to_os_string());
        if entry.is_dir() {
            ensure_directories(materialized, &relative)?;
        } else {
            let expected = entry.size();
            extract_archive_file(
                materialized,
                &relative,
                &mut entry,
                expected,
                &mut written_expanded,
                budget_bytes,
            )?;
        }
    }
    Ok(())
}

fn validate_archive_entry(index: usize, entry: &zip::read::ZipFile<'_>) -> Result<PathBuf, String> {
    let relative = entry
        .enclosed_name()
        .ok_or_else(|| format!("content package entry {index} has an unsafe path"))?
        .to_path_buf();
    let Some(std::path::Component::Normal(first)) = relative.components().next() else {
        return Err(format!(
            "content package entry {index} has an empty or unsafe path"
        ));
    };
    if first == "version" || first == "packages" {
        return Err(format!(
            "content package entry {index} conflicts with retained input layout"
        ));
    }
    if let Some(mode) = entry.unix_mode() {
        let kind = u64::from(mode) & u64::from(libc::S_IFMT);
        let expected = if entry.is_dir() {
            u64::from(libc::S_IFDIR)
        } else {
            u64::from(libc::S_IFREG)
        };
        if kind != 0 && kind != expected {
            return Err(format!(
                "content package entry {index} has an unsupported or contradictory Unix file type"
            ));
        }
    }
    Ok(relative)
}

fn extract_archive_file<R: io::Read>(
    materialized: &mut MaterializedContent,
    relative: &Path,
    input: &mut R,
    expected: u64,
    written_expanded: &mut u64,
    budget_bytes: u64,
) -> Result<(), String> {
    let (parent, filename) = ensure_parent_directories(materialized, relative)?;
    let mut output = create_file_at(&parent, &filename, relative)?;
    materialized.entries.push(MaterializedEntry {
        relative_path: relative.to_path_buf(),
        kind: MaterializedEntryKind::File,
    });
    let mut buffer = [0_u8; 64 * 1024];
    let mut written = 0_u64;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("extract content entry {}: {error}", relative.display()))?;
        if read == 0 {
            break;
        }
        written = written
            .checked_add(read as u64)
            .ok_or_else(|| "content entry written length overflowed".to_string())?;
        *written_expanded = written_expanded
            .checked_add(read as u64)
            .ok_or_else(|| "content package written length overflowed".to_string())?;
        if written > expected || *written_expanded > budget_bytes {
            return Err(format!(
                "content entry {} exceeds the authority expansion budget",
                relative.display()
            ));
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| format!("extract content entry {}: {error}", relative.display()))?;
    }
    if written != expected {
        return Err(format!(
            "content entry {} length changed during extraction",
            relative.display()
        ));
    }
    use std::os::unix::fs::PermissionsExt as _;
    output
        .set_permissions(fs::Permissions::from_mode(0o640))
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("publish content entry {}: {error}", relative.display()))
}

fn ensure_parent_directories(
    materialized: &mut MaterializedContent,
    relative: &Path,
) -> Result<(std::os::fd::OwnedFd, std::ffi::CString), String> {
    let parent = relative
        .parent()
        .ok_or_else(|| format!("content entry has no parent: {}", relative.display()))?;
    let directory = ensure_directories(materialized, parent)?;
    let filename = relative
        .file_name()
        .ok_or_else(|| format!("content entry has no filename: {}", relative.display()))?;
    Ok((directory, component_cstring(filename)?))
}

fn ensure_directories(
    materialized: &mut MaterializedContent,
    relative: &Path,
) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::AsRawFd as _;

    let mut current = duplicate_fd(&materialized.root)?;
    let mut traversed = PathBuf::new();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(format!("unsafe content directory {}", relative.display()));
        };
        let name_c = component_cstring(name)?;
        traversed.push(name);
        let created = if unsafe { libc::mkdirat(current.as_raw_fd(), name_c.as_ptr(), 0o700) } == 0
        {
            materialized.entries.push(MaterializedEntry {
                relative_path: traversed.clone(),
                kind: MaterializedEntryKind::Directory,
            });
            true
        } else if io::Error::last_os_error().raw_os_error() == Some(libc::EEXIST) {
            false
        } else {
            return Err(format!(
                "create content directory {}: {}",
                traversed.display(),
                io::Error::last_os_error()
            ));
        };
        current = open_directory_at(&current, &name_c, &traversed)?;
        if created && unsafe { libc::fchmod(current.as_raw_fd(), 0o750) } != 0 {
            return Err(format!(
                "publish content directory {}: {}",
                traversed.display(),
                io::Error::last_os_error()
            ));
        }
    }
    Ok(current)
}

fn open_directory_path_nofollow(path: &Path) -> Result<std::os::fd::OwnedFd, String> {
    let resolved;
    let path = if path.is_absolute() {
        let parent = path
            .parent()
            .ok_or_else(|| format!("content root has no parent directory: {}", path.display()))?;
        let name = path
            .file_name()
            .ok_or_else(|| format!("content root has no directory name: {}", path.display()))?;
        resolved = fs::canonicalize(parent)
            .map_err(|error| format!("resolve content-root parent {}: {error}", parent.display()))?
            .join(name);
        resolved.as_path()
    } else {
        path
    };
    let mut current = if path.is_absolute() {
        open_directory_raw(c"/", Path::new("/"))?
    } else {
        open_directory_raw(c".", Path::new("."))?
    };
    for component in path.components() {
        match component {
            std::path::Component::RootDir | std::path::Component::CurDir => continue,
            std::path::Component::Normal(name) => {
                current = open_directory_at(&current, &component_cstring(name)?, path)?;
            }
            _ => return Err(format!("unsafe content root path: {}", path.display())),
        }
    }
    Ok(current)
}

fn open_relative_parent(
    root: &std::os::fd::OwnedFd,
    relative: &Path,
) -> Result<(std::os::fd::OwnedFd, std::ffi::CString), String> {
    let mut current = duplicate_fd(root)?;
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    for component in parent.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(format!("unsafe materialized path: {}", relative.display()));
        };
        current = open_directory_at(&current, &component_cstring(name)?, parent)?;
    }
    let filename = relative
        .file_name()
        .ok_or_else(|| format!("materialized path has no filename: {}", relative.display()))?;
    Ok((current, component_cstring(filename)?))
}

fn open_directory_raw(
    path: &std::ffi::CStr,
    display: &Path,
) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::FromRawFd as _;

    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(format!(
            "open content directory {}: {}",
            display.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) })
}

fn open_directory_at(
    parent: &std::os::fd::OwnedFd,
    name: &std::ffi::CStr,
    display: &Path,
) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(format!(
            "open content directory {}: {}",
            display.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) })
}

fn create_file_at(
    parent: &std::os::fd::OwnedFd,
    name: &std::ffi::CStr,
    display: &Path,
) -> Result<fs::File, String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(format!(
            "create content entry {}: {}",
            display.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { fs::File::from_raw_fd(fd) })
}

fn duplicate_fd(fd: &std::os::fd::OwnedFd) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(format!(
            "duplicate content directory descriptor: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(duplicate) })
}

fn component_cstring(value: &std::ffi::OsStr) -> Result<std::ffi::CString, String> {
    use std::os::unix::ffi::OsStrExt as _;

    std::ffi::CString::new(value.as_bytes())
        .map_err(|_| "content path component contains a NUL byte".to_string())
}

struct FreshRoot {
    previous_directory: std::os::fd::OwnedFd,
    _directory: std::os::fd::OwnedFd,
}

impl FreshRoot {
    fn path(&self) -> &Path {
        Path::new(".")
    }
}

impl Drop for FreshRoot {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd as _;

        // The process is exiting after a production run. Tests need the restore
        // so a bound-root check cannot affect later cases in the same binary.
        if unsafe { libc::fchdir(self.previous_directory.as_raw_fd()) } != 0 {
            eprintln!(
                "uqm-native-acceptance: restore working directory: {}",
                io::Error::last_os_error()
            );
        }
    }
}

fn create_fresh_root(root: &Path, allow_precreated: bool) -> Result<FreshRoot, String> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::ffi::OsStrExt as _;

    let requested_parent = root
        .parent()
        .ok_or_else(|| format!("evidence root has no parent: {}", root.display()))?;
    let parent_path = fs::canonicalize(requested_parent).map_err(|error| {
        format!(
            "resolve evidence root parent {}: {error}",
            requested_parent.display()
        )
    })?;
    let filename_os = root
        .file_name()
        .ok_or_else(|| format!("evidence root has no filename: {}", root.display()))?;
    let bound_path = parent_path.join(filename_os);
    let parent = CString::new(parent_path.as_os_str().as_bytes())
        .map_err(|_| "evidence root parent contains a NUL byte".to_string())?;
    let filename = CString::new(filename_os.as_bytes())
        .map_err(|_| "evidence root filename contains a NUL byte".to_string())?;
    let parent_fd = unsafe {
        libc::open(
            parent.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if parent_fd < 0 {
        return Err(format!(
            "open evidence root parent: {}",
            std::io::Error::last_os_error()
        ));
    }
    let parent_fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(parent_fd) };
    let previous_directory = unsafe {
        libc::open(
            c".".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if previous_directory < 0 {
        return Err(format!(
            "bind current directory: {}",
            std::io::Error::last_os_error()
        ));
    }
    let previous_directory = unsafe { std::os::fd::OwnedFd::from_raw_fd(previous_directory) };
    let created = if unsafe { libc::mkdirat(parent_fd.as_raw_fd(), filename.as_ptr(), 0o700) } == 0
    {
        true
    } else {
        let error = std::io::Error::last_os_error();
        if !allow_precreated || error.raw_os_error() != Some(libc::EEXIST) {
            return Err(format!("create evidence root: {error}"));
        }
        false
    };
    let directory = unsafe {
        libc::openat(
            parent_fd.as_raw_fd(),
            filename.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if directory < 0 {
        let error = std::io::Error::last_os_error();
        if created {
            unsafe {
                libc::unlinkat(parent_fd.as_raw_fd(), filename.as_ptr(), libc::AT_REMOVEDIR);
            }
        }
        return Err(format!("bind evidence root: {error}"));
    }
    let directory = unsafe { std::os::fd::OwnedFd::from_raw_fd(directory) };
    if created && unsafe { libc::fchmod(directory.as_raw_fd(), 0o750) } != 0 {
        let error = std::io::Error::last_os_error();
        unsafe {
            libc::unlinkat(parent_fd.as_raw_fd(), filename.as_ptr(), libc::AT_REMOVEDIR);
        }
        return Err(format!("publish evidence root: {error}"));
    }
    if !created {
        let mut members = fs::read_dir(&bound_path)
            .map_err(|error| format!("inspect precreated evidence root: {error}"))?;
        if members.next().is_some() {
            return Err("precreated evidence root is not empty".to_string());
        }
    }
    if unsafe { libc::fchdir(directory.as_raw_fd()) } != 0 {
        let error = std::io::Error::last_os_error();
        if created {
            unsafe {
                libc::unlinkat(parent_fd.as_raw_fd(), filename.as_ptr(), libc::AT_REMOVEDIR);
            }
        }
        return Err(format!("enter bound evidence root: {error}"));
    }
    Ok(FreshRoot {
        previous_directory,
        _directory: directory,
    })
}

fn retain_linked_build_proof(
    source_root: &Path,
    destination_root: &Path,
    executable: &NativeRetainedInput,
    member_limit: u64,
) -> Result<(), String> {
    let receipt_path = source_root.join("linked-build-receipt.json");
    let receipt_bytes = read_bounded_regular(&receipt_path, member_limit)
        .map_err(|error| format!("read linked-build receipt: {error:?}"))?;
    let receipt: NativeLinkedBuildReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("parse linked-build receipt: {error}"))?;
    validate_linked_build_receipt(&receipt, executable)?;
    let cargo_messages =
        read_bounded_regular(&source_root.join("cargo-messages.jsonl"), member_limit)
            .map_err(|error| format!("read linked-build Cargo messages: {error:?}"))?;
    validate_linked_build_cargo_messages(&receipt, &cargo_messages)?;
    let provider_report =
        read_bounded_regular(&source_root.join("provider-report.json"), member_limit)
            .map_err(|error| format!("read linked-build provider report: {error:?}"))?;
    let build_evidence = read_bounded_regular(
        &source_root.join("native-build-evidence.json"),
        member_limit,
    )
    .map_err(|error| format!("read linked-build native build evidence: {error:?}"))?;
    let cargo_manifest = read_bounded_regular(&source_root.join("Cargo.toml"), member_limit)
        .map_err(|error| format!("read linked-build Cargo manifest: {error:?}"))?;
    let cargo_lock = read_bounded_regular(&source_root.join("Cargo.lock"), member_limit)
        .map_err(|error| format!("read linked-build Cargo lock: {error:?}"))?;
    let authority = read_bounded_regular(&source_root.join("gates.json"), member_limit)
        .map_err(|error| format!("read linked-build authority: {error:?}"))?;
    let canonical_toolchain =
        read_bounded_regular(&source_root.join("canonical-toolchain.json"), member_limit)
            .map_err(|error| format!("read linked-build canonical toolchain: {error:?}"))?;
    validate_native_linked_build_semantics(
        &provider_report,
        &build_evidence,
        &authority,
        &canonical_toolchain,
        &cargo_manifest,
        &cargo_lock,
    )
    .map_err(|error| format!("linked-build semantic validation failed: {error:?}"))?;
    let members = [
        ("cargo-messages.jsonl", &receipt.cargo_messages),
        ("rust-archive.a", &receipt.rust_archive),
        ("c-archive.a", &receipt.c_archive),
        ("object-sidecar.manifest", &receipt.object_sidecar),
        ("provider-report.json", &receipt.provider_report),
        ("native-build-evidence.json", &receipt.native_build_evidence),
        ("Cargo.toml", &receipt.cargo_manifest),
        ("Cargo.lock", &receipt.cargo_lock),
        ("gates.json", &receipt.authority),
        ("canonical-toolchain.json", &receipt.canonical_toolchain),
    ];
    let mut retained = Vec::new();
    let retention = (|| -> Result<(), String> {
        for (filename, expected) in members {
            if expected.byte_length > member_limit {
                return Err(format!(
                    "linked-build member {filename} exceeds authority limit"
                ));
            }
            let source = source_root.join(filename);
            let destination = destination_root.join(filename);
            copy_regular_bounded(&source, &destination, member_limit)?;
            retained.push(destination.clone());
            validate_linked_build_member(&destination, expected)?;
        }
        let retained_receipt = destination_root.join("linked-build-receipt.json");
        write_bytes_atomic_noclobber(&retained_receipt, &receipt_bytes)?;
        retained.push(retained_receipt);
        Ok(())
    })();
    if let Err(error) = retention {
        let cleanup_errors: Vec<_> = retained
            .iter()
            .rev()
            .filter_map(|path| {
                fs::remove_file(path)
                    .err()
                    .map(|cleanup| format!("{}: {cleanup}", path.display()))
            })
            .collect();
        return if cleanup_errors.is_empty() {
            Err(error)
        } else {
            Err(format!(
                "{error}; linked-build retention cleanup failed: {}",
                cleanup_errors.join(", ")
            ))
        };
    }
    Ok(())
}

fn validate_linked_build_receipt(
    receipt: &NativeLinkedBuildReceipt,
    executable: &NativeRetainedInput,
) -> Result<(), String> {
    if receipt.schema != NATIVE_LINKED_BUILD_RECEIPT_SCHEMA
        || receipt.source_sha.len() != 40
        || !receipt
            .source_sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || receipt.native_profile != "linked-test"
        || receipt.feature != "audio_heart,debug-process,linked_c_archive"
        || receipt.executable != *executable
        || !Path::new(&receipt.cargo_executable_path).is_absolute()
        || !Path::new(&receipt.cargo_rust_archive_path).is_absolute()
        || !Path::new(&receipt.cargo_out_dir).is_absolute()
    {
        return Err("linked-build receipt contract or executable identity differs".to_string());
    }
    let arguments = receipt
        .cargo_command
        .get(1..)
        .ok_or_else(|| "linked-build Cargo command is empty".to_string())?;
    let required = [
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
    let target_is_valid = arguments.len() == required.len()
        || (arguments.len() == required.len() + 2
            && arguments[required.len()] == "--target-dir"
            && Path::new(&arguments[required.len() + 1]).is_absolute());
    if !target_is_valid
        || !arguments
            .iter()
            .take(required.len())
            .map(String::as_str)
            .eq(required)
    {
        return Err("linked-build Cargo command is not the strict linked-test build".to_string());
    }
    let expected_paths = [
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
    if expected_paths
        .into_iter()
        .any(|(path, input)| input.relative_path != path || !valid_sha256(&input.sha256))
    {
        return Err("linked-build receipt has an invalid retained member identity".to_string());
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_linked_build_cargo_messages(
    receipt: &NativeLinkedBuildReceipt,
    bytes: &[u8],
) -> Result<(), String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("linked-build Cargo messages are not UTF-8: {error}"))?;
    let messages: Vec<serde_json::Value> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|error| format!("invalid linked-build Cargo message: {error}"))
        })
        .collect::<Result<_, _>>()?;
    let package_ids: std::collections::BTreeSet<_> = messages
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
        return Err("linked-build Cargo capture has no unique UQM package identity".to_string());
    }
    let package_id = package_ids.into_iter().next().expect("length checked");
    let mut executables = std::collections::BTreeSet::new();
    let mut rust_archives = std::collections::BTreeSet::new();
    let mut out_dirs = std::collections::BTreeSet::new();
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
        return Err("linked-build Cargo capture differs from the receipt".to_string());
    }
    Ok(())
}

fn cargo_target_has_kind(message: &serde_json::Value, expected: &str) -> bool {
    message["target"]["kind"]
        .as_array()
        .is_some_and(|kinds| kinds.iter().any(|kind| kind == expected))
}

fn validate_linked_build_member(path: &Path, expected: &NativeRetainedInput) -> Result<(), String> {
    let (byte_length, sha256) = regular_file_identity(path)?;
    if byte_length != expected.byte_length || sha256 != expected.sha256 {
        return Err(format!(
            "retained linked-build member {} differs from its receipt",
            path.display()
        ));
    }
    Ok(())
}

fn copy_executable_bounded(source: &Path, destination: &Path, limit: u64) -> Result<(), String> {
    copy_regular_bounded(source, destination, limit)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};

        let mut options = fs::OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        let file = match options.open(destination) {
            Ok(file) => file,
            Err(error) => {
                return copy_failure_cleanup(
                    destination,
                    format!(
                        "open retained executable {} for mode binding: {error}",
                        destination.display()
                    ),
                );
            }
        };
        if let Err(error) = file.set_permissions(fs::Permissions::from_mode(0o550)) {
            drop(file);
            return copy_failure_cleanup(
                destination,
                format!(
                    "set retained executable mode on {}: {error}",
                    destination.display()
                ),
            );
        }
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                drop(file);
                return copy_failure_cleanup(
                    destination,
                    format!(
                        "inspect retained executable {} after mode binding: {error}",
                        destination.display()
                    ),
                );
            }
        };
        if !metadata.is_file() || metadata.permissions().mode() & 0o7777 != 0o550 {
            drop(file);
            return copy_failure_cleanup(
                destination,
                format!(
                    "retained executable {} does not have exact mode 0550",
                    destination.display()
                ),
            );
        }
    }
    Ok(())
}

fn copy_regular_bounded(source: &Path, destination: &Path, limit: u64) -> Result<(), String> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut input_options = fs::OpenOptions::new();
    input_options.read(true);
    #[cfg(unix)]
    input_options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let input = input_options
        .open(source)
        .map_err(|error| format!("open linked-build input {}: {error}", source.display()))?;
    let metadata = input
        .metadata()
        .map_err(|error| format!("inspect linked-build input {}: {error}", source.display()))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!(
            "linked-build input is not a bounded regular file: {}",
            source.display()
        ));
    }
    copy_bounded_reader(input, source, destination, metadata.len(), limit)
}

fn copy_bounded_reader<R: io::Read>(
    input: R,
    source: &Path,
    destination: &Path,
    expected_length: u64,
    limit: u64,
) -> Result<(), String> {
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| {
            format!(
                "create linked-build input {}: {error}",
                destination.display()
            )
        })?;
    let copied = match io::copy(&mut input.take(limit.saturating_add(1)), &mut output) {
        Ok(copied) => copied,
        Err(error) => {
            drop(output);
            return copy_failure_cleanup(
                destination,
                format!("copy linked-build input {}: {error}", source.display()),
            );
        }
    };
    if copied > limit || copied != expected_length {
        drop(output);
        return copy_failure_cleanup(
            destination,
            format!(
                "linked-build input {} changed length or exceeded its limit",
                source.display()
            ),
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if let Err(error) = output.set_permissions(fs::Permissions::from_mode(0o640)) {
            drop(output);
            return copy_failure_cleanup(
                destination,
                format!(
                    "publish linked-build input {}: {error}",
                    destination.display()
                ),
            );
        }
    }
    if let Err(error) = output.sync_all() {
        drop(output);
        return copy_failure_cleanup(
            destination,
            format!("sync linked-build input {}: {error}", destination.display()),
        );
    }
    Ok(())
}

fn copy_failure_cleanup(destination: &Path, failure: String) -> Result<(), String> {
    match fs::remove_file(destination) {
        Ok(()) => Err(failure),
        Err(error) => Err(format!(
            "{failure}; remove partial linked-build input {}: {error}",
            destination.display()
        )),
    }
}

fn read_regular_file_nofollow_bounded(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options
        .open(path)
        .map_err(|error| format!("open regular file {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspect opened file {}: {error}", path.display()))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!(
            "path is not a bounded regular file: {}",
            path.display()
        ));
    }
    read_bounded_reader(file, metadata.len(), limit, path)
}

fn read_bounded_reader<R: io::Read>(
    input: R,
    expected_length: u64,
    limit: u64,
    path: &Path,
) -> Result<Vec<u8>, String> {
    if expected_length > limit {
        return Err(format!(
            "regular file exceeds its limit: {}",
            path.display()
        ));
    }
    let read_limit = limit
        .checked_add(1)
        .ok_or_else(|| format!("regular-file limit overflow for {}", path.display()))?;
    let mut bytes = Vec::new();
    input
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read regular file {}: {error}", path.display()))?;
    if (bytes.len() as u64) != expected_length || (bytes.len() as u64) > limit {
        return Err(format!(
            "regular file changed length or exceeded its limit: {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn validate_expected_executable(
    retained: &NativeRetainedInput,
    expected_length: u64,
    expected_sha256: &str,
) -> Result<(), String> {
    if retained.byte_length != expected_length || retained.sha256 != expected_sha256 {
        return Err(
            "retained executable differs from the linked build output selected by xtask"
                .to_string(),
        );
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn regular_file_identity(path: &Path) -> Result<(u64, String), String> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut file = options
        .open(path)
        .map_err(|error| format!("open regular file {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspect opened file {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("path is not a regular file: {}", path.display()));
    }
    let mut digest = Sha256::new();
    let mut byte_length = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read regular file {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        byte_length = byte_length
            .checked_add(count as u64)
            .ok_or_else(|| format!("regular file length overflow: {}", path.display()))?;
        digest.update(&buffer[..count]);
    }
    if byte_length != metadata.len() {
        return Err(format!(
            "regular file changed while reading: {}",
            path.display()
        ));
    }
    Ok((byte_length, format!("{:x}", digest.finalize())))
}

#[cfg(target_os = "macos")]
fn verify_spawned_executable(
    identity: &ProcessIdentity,
    expected_path: &Path,
    expected: &NativeRetainedInput,
) -> Result<(), String> {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt as _};

    unsafe extern "C" {
        fn proc_pidpath(
            pid: libc::c_int,
            buffer: *mut libc::c_void,
            buffer_size: u32,
        ) -> libc::c_int;
    }

    let pid = i32::try_from(identity.pid).map_err(|_| "child PID exceeds i32".to_string())?;
    let mut bytes = vec![0_u8; 4096];
    // SAFETY: the buffer is writable for its declared length and `pid` identifies the
    // already-launched child owned by `ChildSession`.
    let count = unsafe { proc_pidpath(pid, bytes.as_mut_ptr().cast(), bytes.len() as u32) };
    if count <= 0 {
        return Err(format!(
            "cannot resolve executable backing child PID {}",
            identity.pid
        ));
    }
    bytes.truncate(count as usize);
    let backing_path = PathBuf::from(OsString::from_vec(bytes));
    let expected_canonical = expected_path
        .canonicalize()
        .map_err(|error| format!("canonicalize retained executable: {error}"))?;
    let backing_canonical = backing_path
        .canonicalize()
        .map_err(|error| format!("canonicalize child executable backing path: {error}"))?;
    if backing_canonical != expected_canonical {
        return Err(format!(
            "child executable backing path mismatch: expected {}, got {}",
            expected_canonical.display(),
            backing_canonical.display()
        ));
    }
    let (byte_length, sha256) = regular_file_identity(&backing_canonical)?;
    if byte_length != expected.byte_length || sha256 != expected.sha256 {
        return Err(
            "child executable backing identity contradicts retained executable".to_string(),
        );
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn verify_spawned_executable(
    _identity: &ProcessIdentity,
    _expected_path: &Path,
    _expected: &NativeRetainedInput,
) -> Result<(), String> {
    Err("native executable backing verification is supported only on macOS".to_string())
}

fn retained_input(root: &Path, path: &Path) -> Result<NativeRetainedInput, String> {
    let (byte_length, sha256) = relative_regular_file_identity(path)?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("retained input escaped evidence root: {}", path.display()))?;
    Ok(NativeRetainedInput {
        relative_path: relative.to_string_lossy().into_owned(),
        byte_length,
        sha256,
    })
}

fn native_identity(identity: &ProcessIdentity, nonce: &str) -> NativeProcessIdentity {
    NativeProcessIdentity {
        pid: identity.pid,
        start_time: identity.start_time.clone(),
        executable_sha256: identity.executable_digest.clone(),
        nonce: nonce.to_string(),
    }
}

fn fresh_nonce() -> Result<String, String> {
    use std::io::Read as _;
    let mut random = [0_u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut random))
        .map_err(|error| format!("read OS randomness: {error}"))?;
    Ok(random.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn trace_semantics(path: &Path, limit: u64) -> Result<(u64, u64), String> {
    let bytes = read_regular_file_nofollow_bounded(path, limit)?;
    if bytes.is_empty() || !bytes.ends_with(b"\n") {
        return Err("trace must be nonempty and newline terminated".to_string());
    }
    let text = std::str::from_utf8(&bytes).map_err(|error| format!("trace UTF-8: {error}"))?;
    let records = text
        .lines()
        .enumerate()
        .map(|(sequence, line)| {
            let record: TraceRecord = serde_json::from_str(line)
                .map_err(|error| format!("parse trace record: {error}"))?;
            if record.schema != TraceRecord::SCHEMA || record.sequence != sequence as u64 {
                return Err("trace sequence/schema mismatch".to_string());
            }
            Ok(record)
        })
        .collect::<Result<Vec<_>, String>>()?;
    let semantic = native_window_trace_semantic_snapshot(&records)
        .map_err(|error| format!("validate native trace semantics: {error:?}"))?;
    Ok((
        semantic.accepted_player_inputs,
        semantic.verified_battle_frames,
    ))
}

#[cfg(unix)]
fn opened_parent(path: &Path) -> Result<(std::os::fd::OwnedFd, std::ffi::CString), String> {
    use std::os::fd::FromRawFd as _;
    use std::os::unix::ffi::OsStrExt as _;

    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    let filename = path
        .file_name()
        .ok_or_else(|| format!("path has no filename: {}", path.display()))?;
    let parent = std::ffi::CString::new(parent.as_os_str().as_bytes())
        .map_err(|_| format!("path parent contains NUL: {}", path.display()))?;
    let filename = std::ffi::CString::new(filename.as_bytes())
        .map_err(|_| format!("path filename contains NUL: {}", path.display()))?;
    let descriptor = unsafe {
        libc::open(
            parent.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "open parent directory {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok((
        unsafe { std::os::fd::OwnedFd::from_raw_fd(descriptor) },
        filename,
    ))
}

#[cfg(unix)]
fn relative_regular_file_identity(path: &Path) -> Result<(u64, String), String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let (parent, filename) = opened_parent(path)?;
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            filename.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "open retained input {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    let mut file = fs::File::from(unsafe { std::os::fd::OwnedFd::from_raw_fd(descriptor) });
    let metadata = file
        .metadata()
        .map_err(|error| format!("inspect retained input {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("retained input is not regular: {}", path.display()));
    }
    let mut digest = Sha256::new();
    let mut byte_length = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read retained input {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        byte_length = byte_length
            .checked_add(count as u64)
            .ok_or_else(|| format!("retained input length overflow: {}", path.display()))?;
        digest.update(&buffer[..count]);
    }
    if byte_length != metadata.len() {
        return Err(format!(
            "retained input changed while reading: {}",
            path.display()
        ));
    }
    Ok((byte_length, format!("{:x}", digest.finalize())))
}

#[cfg(not(unix))]
fn relative_regular_file_identity(path: &Path) -> Result<(u64, String), String> {
    regular_file_identity(path)
}

#[cfg(unix)]
fn write_bytes_atomic_noclobber(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::ffi::OsStringExt as _;
    use std::os::unix::fs::PermissionsExt as _;

    let (parent, filename) = opened_parent(path)?;
    let temporary_name = path
        .file_name()
        .ok_or_else(|| format!("path has no filename: {}", path.display()))?
        .to_os_string();
    let mut temporary_name = temporary_name.into_vec();
    temporary_name.extend_from_slice(b".tmp");
    let temporary_name = std::ffi::CString::new(temporary_name)
        .map_err(|_| format!("temporary path contains NUL: {}", path.display()))?;
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            temporary_name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "create temporary for {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    let mut file = fs::File::from(unsafe { std::os::fd::OwnedFd::from_raw_fd(descriptor) });
    use std::io::Write as _;
    if let Err(error) = file
        .write_all(bytes)
        .and_then(|()| file.set_permissions(fs::Permissions::from_mode(0o640)))
        .and_then(|()| file.sync_all())
    {
        drop(file);
        unsafe { libc::unlinkat(parent.as_raw_fd(), temporary_name.as_ptr(), 0) };
        return Err(format!("write temporary for {}: {error}", path.display()));
    }
    drop(file);
    if unsafe {
        libc::linkat(
            parent.as_raw_fd(),
            temporary_name.as_ptr(),
            parent.as_raw_fd(),
            filename.as_ptr(),
            0,
        )
    } != 0
    {
        let error = std::io::Error::last_os_error();
        unsafe { libc::unlinkat(parent.as_raw_fd(), temporary_name.as_ptr(), 0) };
        return Err(format!("publish {}: {error}", path.display()));
    }
    if unsafe { libc::unlinkat(parent.as_raw_fd(), temporary_name.as_ptr(), 0) } != 0 {
        return Err(format!(
            "remove temporary for {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    if unsafe { libc::fsync(parent.as_raw_fd()) } != 0 {
        return Err(format!(
            "sync parent for {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn write_bytes_atomic_noclobber(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("create {}: {error}", path.display()))?;
    use std::io::Write as _;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("write {}: {error}", path.display()))
}

fn write_json_atomic(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    bytes.push(b'\n');
    write_bytes_atomic_noclobber(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    static CWD_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn runtime_contract() -> NativeWindowRuntimeContract {
        NativeWindowRuntimeContract {
            capture_timeout_ms: 30_000,
            capture_kill_grace_ms: 5_000,
            observer_timeout_ms: 40_000,
            observer_kill_grace_ms: 5_000,
            acknowledgement_timeout_ms: 95_000,
            outer_child_timeout_ms: 300_000,
            outer_child_kill_grace_ms: 5_000,
            child_stdout_budget_bytes: 16 * 1024 * 1024,
            child_stderr_budget_bytes: 16 * 1024 * 1024,
            observer_response_budget_bytes: 64 * 1024,
            capture_budget_bytes: 64 * 1024 * 1024,
            content_expansion_budget_bytes: 32 * 1024 * 1024,
            inventory_limits: NativeInventoryLimits {
                member_count: 10_000,
                member_bytes: 64 * 1024 * 1024,
                aggregate_bytes: 256 * 1024 * 1024,
                path_bytes: 4096,
                aggregate_path_bytes: 8 * 1024 * 1024,
            },
            expected_client_bounds: NativeWindowBounds {
                x: 80,
                y: 80,
                width: 1280,
                height: 960,
            },
        }
    }

    fn observer_for_helper_test(
        scratch_root: &Path,
        executable: PathBuf,
        contract: NativeWindowRuntimeContract,
    ) -> BoundedNativeObserver {
        use std::os::unix::fs::MetadataExt as _;
        let metadata = fs::symlink_metadata(scratch_root).unwrap();
        BoundedNativeObserver {
            executable: executable.clone(),
            executable_digest: sha256_hex(
                &read_regular_file_nofollow_bounded(&executable, 64 * 1024 * 1024).unwrap(),
            ),
            scratch_root: scratch_root.to_path_buf(),
            scratch_directory: open_directory_path_nofollow(scratch_root).unwrap(),
            scratch_identity: (metadata.dev(), metadata.ino()),
            members: std::collections::BTreeSet::new(),
            cleaned: false,
            sequence: 0,
            contract,
        }
    }

    fn acceptance_policy() -> NativeAcceptancePolicy {
        NativeAcceptancePolicy {
            stable_presentation_floor: 120,
            playable_presentation_floor: 300,
            battle_frame_floor: 300,
        }
    }

    fn linked_build_receipt_for_cargo_test() -> NativeLinkedBuildReceipt {
        fn input(path: &str) -> NativeRetainedInput {
            NativeRetainedInput {
                relative_path: path.to_string(),
                byte_length: 1,
                sha256: "a".repeat(64),
            }
        }
        NativeLinkedBuildReceipt {
            schema: NATIVE_LINKED_BUILD_RECEIPT_SCHEMA.to_string(),
            source_sha: "a".repeat(40),
            cargo_command: Vec::new(),
            native_profile: "linked-test".to_string(),
            feature: "audio_heart,debug-process,linked_c_archive".to_string(),
            cargo_executable_path: "/target/release/uqm".to_string(),
            cargo_rust_archive_path: "/target/release/deps/libuqm_rust-test.a".to_string(),
            cargo_out_dir: "/target/release/build/uqm-test/out".to_string(),
            executable: input("inputs/uqm"),
            cargo_messages: input("inputs/linked-build/cargo-messages.jsonl"),
            rust_archive: input("inputs/linked-build/rust-archive.a"),
            c_archive: input("inputs/linked-build/c-archive.a"),
            object_sidecar: input("inputs/linked-build/object-sidecar.manifest"),
            provider_report: input("inputs/linked-build/provider-report.json"),
            native_build_evidence: input("inputs/linked-build/native-build-evidence.json"),
            cargo_manifest: input("inputs/linked-build/Cargo.toml"),
            cargo_lock: input("inputs/linked-build/Cargo.lock"),
            authority: input("inputs/linked-build/gates.json"),
            canonical_toolchain: input("inputs/linked-build/canonical-toolchain.json"),
        }
    }

    fn cargo_messages_for_test(package_id: &str, archive_package_id: &str) -> Vec<u8> {
        [
            serde_json::json!({
                "reason": "compiler-artifact",
                "package_id": package_id,
                "target": {"name": "uqm", "kind": ["bin"]},
                "executable": "/target/release/uqm",
                "filenames": []
            }),
            serde_json::json!({
                "reason": "compiler-artifact",
                "package_id": archive_package_id,
                "target": {"name": "uqm_rust", "kind": ["staticlib"]},
                "executable": null,
                "filenames": ["/target/release/deps/libuqm_rust-test.a"]
            }),
            serde_json::json!({
                "reason": "build-script-executed",
                "package_id": package_id,
                "out_dir": "/target/release/build/uqm-test/out"
            }),
            serde_json::json!({"reason": "build-finished", "success": true}),
        ]
        .into_iter()
        .map(|message| format!("{message}\n"))
        .collect::<String>()
        .into_bytes()
    }

    #[test]
    fn linked_build_cargo_replay_rejects_cross_package_artifacts_and_duplicate_completion() {
        let receipt = linked_build_receipt_for_cargo_test();
        let package_id = "path+file:///checkout/rust#uqm@0.8.0";
        let valid = cargo_messages_for_test(package_id, package_id);
        validate_linked_build_cargo_messages(&receipt, &valid).unwrap();

        let mismatched = cargo_messages_for_test(package_id, "registry+test#uqm@0.8.0");
        assert!(validate_linked_build_cargo_messages(&receipt, &mismatched).is_err());

        let mut duplicate_completion = valid;
        duplicate_completion
            .extend_from_slice(b"{\"reason\":\"build-finished\",\"success\":true}\n");
        assert!(validate_linked_build_cargo_messages(&receipt, &duplicate_completion).is_err());
    }

    #[test]
    fn runner_requires_an_absolute_evidence_root_before_inspecting_inputs() {
        let digest = "0".repeat(64);
        let error = run(RunInputs {
            executable: Path::new("missing-executable"),
            content: Path::new("missing-content"),
            script: Path::new("missing-script"),
            root: Path::new("relative-evidence"),
            expected_executable_length: 1,
            expected_executable_sha256: &digest,
            runtime_contract: runtime_contract(),
            acceptance_policy: acceptance_policy(),
            linked_build_proof: Path::new("missing-linked-build-proof"),
            linked_build_member_limit: 1,
        })
        .unwrap_err();
        assert_eq!(error, "evidence root must be absolute");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn runner_rejects_every_relative_runtime_input_before_metadata_access() {
        for (label, executable, content, script) in [
            (
                "executable",
                Path::new("relative-executable"),
                Path::new("/missing-content"),
                Path::new("/missing-script"),
            ),
            (
                "content",
                Path::new("/missing-executable"),
                Path::new("relative-content"),
                Path::new("/missing-script"),
            ),
            (
                "script",
                Path::new("/missing-executable"),
                Path::new("/missing-content"),
                Path::new("relative-script"),
            ),
        ] {
            let error = validate_run_paths(executable, content, script).unwrap_err();
            assert!(error.starts_with(&format!("{label} path must be absolute:")));
        }
    }
    #[test]
    fn pre_spawn_failure_publishes_a_typed_self_validating_envelope() {
        let parent = tempfile::tempdir().unwrap();
        let _cwd_guard = CWD_TEST_LOCK.lock().unwrap();
        let content = parent.path().join("content");
        let script = parent.path().join("script.json");
        let evidence = parent.path().join("evidence");
        fs::create_dir(&content).unwrap();
        fs::write(content.join("bad.uqm"), b"not reached").unwrap();
        fs::write(&script, b"not reached").unwrap();
        let digest = "0".repeat(64);

        let run_error = run(RunInputs {
            executable: Path::new("/usr/bin/true"),
            content: &content,
            script: &script,
            root: &evidence,
            expected_executable_length: 1,
            expected_executable_sha256: &digest,
            runtime_contract: runtime_contract(),
            acceptance_policy: acceptance_policy(),
            linked_build_proof: parent.path(),
            linked_build_member_limit: 1,
        })
        .unwrap_err();
        let bytes = fs::read(evidence.join("native-acceptance-failure.json"))
            .unwrap_or_else(|error| panic!("{run_error}; read setup failure manifest: {error}"));
        let manifest: NativeAcceptanceSetupFailureManifest =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            manifest.failure_contract,
            NativeAcceptanceSetupFailureContract::Preparation
        );
        assert_eq!(manifest.error, run_error);
        assert_ne!(manifest.error, "evidence root must be absolute");
        validate_native_acceptance_setup_failure_bundle(&evidence, &manifest).unwrap();
    }

    #[test]
    fn typed_observer_failure_preserves_truthful_child_cleanup_receipt() {
        let root = tempfile::tempdir().unwrap();
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        let session = ChildSession::spawn(
            command,
            ChildSessionConfig {
                stdout_log: root.path().join("stdout.log"),
                stderr_log: root.path().join("stderr.log"),
                stdout_budget: 1024,
                stderr_budget: 1024,
                timeout: Duration::from_secs(30),
                grace: Duration::from_millis(10),
                executable_digest: "a".repeat(64),
            },
        )
        .unwrap();
        let failure = session
            .finish_observing(|_| Err(io::Error::other("native observation failed")))
            .unwrap_err();
        assert!(matches!(&failure.error, ChildSessionError::Observer(_)));
        let detail = format!("{failure}");

        let clean_observer = runtime_failure(
            Some((&failure.error, detail.clone())),
            ChildTerminalOutcome {
                exit_code: Some(0),
                signal: None,
                term_sent: false,
                kill_sent: false,
            },
            Err("scratch root identity changed".to_string()),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            clean_observer.as_ref().map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::Observer)
        );
        assert!(clean_observer.as_ref().is_some_and(|failure| failure
            .1
            .contains("observer cleanup failed: scratch root identity changed")));

        let receipt = &failure.receipt;
        let signaled_after_observer = runtime_failure(
            Some((&failure.error, detail)),
            ChildTerminalOutcome {
                exit_code: receipt.exit_code,
                signal: receipt.signal,
                term_sent: receipt.term_sent,
                kill_sent: receipt.kill_sent,
            },
            Ok(()),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            signaled_after_observer.as_ref().map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::Observer)
        );
        assert!(receipt.term_sent || receipt.kill_sent);
    }

    #[test]
    fn setup_failure_contract_tracks_whether_child_spawn_was_attempted() {
        assert_eq!(
            setup_failure_contract(false),
            NativeAcceptanceSetupFailureContract::Preparation
        );
        assert_eq!(
            setup_failure_contract(true),
            NativeAcceptanceSetupFailureContract::ChildSpawn
        );
    }

    #[test]
    fn actual_child_session_supervision_error_uses_supervision_contract() {
        let root = tempfile::tempdir().unwrap();
        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        let session = ChildSession::spawn(
            command,
            ChildSessionConfig {
                stdout_log: root.path().join("stdout.log"),
                stderr_log: root.path().join("stderr.log"),
                stdout_budget: 1024,
                stderr_budget: 1024,
                timeout: Duration::from_millis(20),
                grace: Duration::from_millis(10),
                executable_digest: "a".repeat(64),
            },
        )
        .unwrap();
        let failure = session.finish_observing(|_| Ok(())).unwrap_err();
        assert!(!matches!(&failure.error, ChildSessionError::Observer(_)));
        let detail = format!("{failure}");
        let receipt = &failure.receipt;
        let classified = runtime_failure(
            Some((&failure.error, detail)),
            ChildTerminalOutcome {
                exit_code: receipt.exit_code,
                signal: receipt.signal,
                term_sent: receipt.term_sent,
                kill_sent: receipt.kill_sent,
            },
            Ok(()),
            true,
            true,
        )
        .unwrap();
        assert_eq!(
            classified.as_ref().map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::ChildSupervision)
        );
    }

    #[test]
    fn observer_helper_that_never_responds_is_terminated_killed_and_reaped() {
        use std::os::unix::process::CommandExt as _;

        let mut command = Command::new("/bin/sleep");
        command.arg("30");
        unsafe {
            command.pre_exec(|| {
                libc::signal(libc::SIGTERM, libc::SIG_IGN);
                Ok(())
            });
        }
        let scratch = tempfile::tempdir().unwrap();
        let executable = PathBuf::from("/bin/sleep");
        let mut observer = observer_for_helper_test(
            scratch.path(),
            executable,
            NativeWindowRuntimeContract {
                observer_timeout_ms: 10,
                observer_kill_grace_ms: 10,
                ..runtime_contract()
            },
        );
        let started = std::time::Instant::now();
        let error = observer.run_helper(command, "test", 1).unwrap_err();

        assert!(error.to_string().contains("term=true, kill=true"));
        assert!(started.elapsed() < Duration::from_secs(2));
        observer.finish().unwrap();
        assert!(!scratch.path().exists());
    }

    #[test]
    fn observer_helper_output_flood_is_typed_and_group_cleaned() {
        let scratch = tempfile::tempdir().unwrap();
        let executable = ["/bin/bash", "/usr/bin/bash", "/bin/sh"]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| {
                fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
            })
            .expect("a regular shell executable is required for the observer flood test");
        let mut observer = observer_for_helper_test(
            scratch.path(),
            executable.clone(),
            NativeWindowRuntimeContract {
                observer_response_budget_bytes: 128,
                ..runtime_contract()
            },
        );
        let mut command = Command::new(&executable);

        command.args(["-c", "while :; do printf 0123456789abcdef; done"]);
        let error = observer.run_helper(command, "flood", 1).unwrap_err();
        assert!(matches!(
            error,
            NativeWindowObserverError::OutputLimit {
                stream,
                limit_bytes: 128
            } if stream == "stdout"
        ));
        observer.finish().unwrap();
        assert!(!scratch.path().exists());
    }

    #[test]
    fn observer_cleanup_stays_bound_when_the_visible_root_is_replaced() {
        let base = tempfile::tempdir().unwrap();
        let scratch = base.path().join("scratch");
        fs::create_dir(&scratch).unwrap();
        let executable = PathBuf::from("/bin/echo");
        let mut observer = observer_for_helper_test(&scratch, executable, runtime_contract());
        let member = observer.next_path("response", "json").unwrap();
        fs::write(&member, b"retained").unwrap();

        let retained = base.path().join("retained-scratch");
        fs::rename(&scratch, &retained).unwrap();
        fs::create_dir(&scratch).unwrap();
        fs::write(scratch.join("unrelated"), b"do not remove").unwrap();

        assert!(observer.finish().is_err());
        assert!(!retained.join("response-0.json").exists());
        assert_eq!(
            fs::read(scratch.join("unrelated")).unwrap(),
            b"do not remove"
        );
    }

    #[test]
    fn observer_member_reads_reject_symlink_replacement() {
        use std::os::unix::fs::symlink;

        let scratch = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), b"outside").unwrap();
        let executable = PathBuf::from("/bin/echo");
        let mut observer = observer_for_helper_test(scratch.path(), executable, runtime_contract());
        let member = observer.next_path("response", "json").unwrap();
        symlink(outside.path(), &member).unwrap();

        assert!(observer.read_member(&member, 1024).is_err());
        observer.finish().unwrap();
        assert_eq!(fs::read(outside.path()).unwrap(), b"outside");
    }

    #[test]
    fn fresh_root_remains_bound_when_its_path_is_replaced() {
        use std::os::unix::fs::{symlink, PermissionsExt as _};
        let _cwd_guard = CWD_TEST_LOCK.lock().unwrap();

        let parent = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let requested = parent.path().join("evidence");
        let retained = parent.path().join("retained");
        let root = create_fresh_root(&requested, false).unwrap();
        assert_eq!(
            fs::metadata(&requested).unwrap().permissions().mode() & 0o777,
            0o750
        );
        fs::rename(&requested, &retained).unwrap();
        symlink(outside.path(), &requested).unwrap();

        fs::write(root.path().join("proof"), b"bound").unwrap();

        assert_eq!(fs::read(retained.join("proof")).unwrap(), b"bound");
        assert!(!outside.path().join("proof").exists());
        fs::remove_file(requested).unwrap();
    }

    #[test]
    fn precreated_evidence_root_must_be_empty_and_remains_bound() {
        let _cwd_guard = CWD_TEST_LOCK.lock().unwrap();
        let parent = tempfile::tempdir().unwrap();
        let requested = parent.path().join("evidence");
        fs::create_dir(&requested).unwrap();

        let root = create_fresh_root(&requested, true).unwrap();
        fs::write(root.path().join("proof"), b"bound").unwrap();
        drop(root);
        assert_eq!(fs::read(requested.join("proof")).unwrap(), b"bound");
    }

    #[test]
    fn precreated_evidence_root_rejects_existing_members() {
        let _cwd_guard = CWD_TEST_LOCK.lock().unwrap();
        let parent = tempfile::tempdir().unwrap();
        let requested = parent.path().join("evidence");
        fs::create_dir(&requested).unwrap();
        fs::write(requested.join("stale"), b"stale").unwrap();

        assert!(create_fresh_root(&requested, true).is_err());
    }

    #[test]
    fn runtime_contract_leaves_acknowledgement_inside_outer_child_bound() {
        let contract = runtime_contract();
        assert!(contract.has_valid_deadline_order());
        assert!(contract.acknowledgement_timeout() < contract.outer_child_timeout());
    }

    #[test]
    fn child_terminal_state_has_a_causal_failure_contract() {
        assert!(child_exit_failure(Some(0), None).is_none());
        assert_eq!(
            child_exit_failure(Some(1), None).map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::ChildExit)
        );
        assert_eq!(
            child_exit_failure(None, Some(15)).map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::ChildExit)
        );
        assert_eq!(
            child_exit_failure(None, None).map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::ChildSupervision)
        );
        assert_eq!(
            child_exit_failure(Some(0), Some(15)).map(|failure| failure.0),
            Some(NativeAcceptanceFailureContract::ChildSupervision)
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn runner_rejects_unsupported_platform_before_inspecting_inputs() {
        let digest = "0".repeat(64);
        let error = run(RunInputs {
            executable: Path::new("missing-executable"),
            content: Path::new("missing-content"),
            script: Path::new("missing-script"),
            root: Path::new("/tmp/uqm-native-acceptance-unsupported-test"),
            expected_executable_length: 1,
            expected_executable_sha256: &digest,
            runtime_contract: runtime_contract(),
            acceptance_policy: acceptance_policy(),
            linked_build_proof: Path::new("missing-linked-build-proof"),
            linked_build_member_limit: 1,
        })
        .unwrap_err();
        assert_eq!(
            error,
            "native-window acceptance is unsupported on this platform"
        );
    }
    #[test]
    fn selected_linked_executable_identity_must_match_retained_copy() {
        let retained = NativeRetainedInput {
            relative_path: "inputs/uqm".to_string(),
            byte_length: 42,
            sha256: "a".repeat(64),
        };
        validate_expected_executable(&retained, 42, &"a".repeat(64)).unwrap();
        assert!(validate_expected_executable(&retained, 41, &"a".repeat(64)).is_err());
        assert!(validate_expected_executable(&retained, 42, &"b".repeat(64)).is_err());
    }

    #[test]
    fn fresh_nonces_are_lowercase_sha256_width_and_distinct() {
        let first = fresh_nonce().unwrap();
        let second = fresh_nonce().unwrap();
        assert_eq!(first.len(), 64);
        assert!(first
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()));
        assert_ne!(first, second);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn spawned_child_backing_matches_retained_executable_identity() {
        let root = tempfile::tempdir().unwrap();
        let executable = Path::new("/bin/sleep");
        let (byte_length, sha256) = regular_file_identity(executable).unwrap();
        let expected = NativeRetainedInput {
            relative_path: "inputs/sleep".to_string(),
            byte_length,
            sha256,
        };
        let mut command = Command::new(executable);
        command.arg("30");
        let session = ChildSession::spawn(
            command,
            ChildSessionConfig {
                stdout_log: root.path().join("stdout.log"),

                stderr_log: root.path().join("stderr.log"),
                stdout_budget: 1024,
                stderr_budget: 1024,
                timeout: Duration::from_secs(30),
                grace: Duration::from_millis(10),
                executable_digest: expected.sha256.clone(),
            },
        )
        .unwrap();
        verify_spawned_executable(session.identity(), executable, &expected).unwrap();
        let mut forged = expected.clone();
        forged.sha256 = "0".repeat(64);
        assert!(verify_spawned_executable(session.identity(), executable, &forged).is_err());
        assert!(session
            .finish_observing(|_| Err(io::Error::other("test complete")))
            .is_err());
    }

    #[test]
    fn bounded_linked_copy_rejects_oversize_without_publishing_output() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::write(&source, b"too-large").unwrap();

        assert!(copy_regular_bounded(&source, &destination, 1).is_err());
        assert!(!destination.exists());
    }

    #[cfg(unix)]
    #[test]
    fn bounded_executable_copy_is_readable_by_the_containment_group_but_not_writable() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::write(&source, b"executable").unwrap();

        copy_executable_bounded(&source, &destination, 1024).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"executable");
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
            0o550
        );
    }

    #[cfg(unix)]
    #[test]
    fn bounded_regular_copy_is_readable_by_the_containment_group_but_not_writable() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::write(&source, b"content").unwrap();

        copy_regular_bounded(&source, &destination, 1024).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"content");
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
            0o640
        );
    }

    #[test]
    fn bounded_linked_copy_refuses_to_clobber_retained_output() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::write(&source, b"replacement").unwrap();
        fs::write(&destination, b"retained").unwrap();

        assert!(copy_regular_bounded(&source, &destination, 1024).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"retained");
    }

    #[test]
    fn bounded_regular_read_rejects_oversize_growth_and_truncation() {
        let path = Path::new("fixture");
        assert!(read_bounded_reader(&b"abcd"[..], 4, 3, path).is_err());
        assert!(read_bounded_reader(&b"abcd"[..], 3, 3, path).is_err());
        assert!(read_bounded_reader(&b"abc"[..], 4, 4, path).is_err());
        assert_eq!(
            read_bounded_reader(&b"abc"[..], 3, 3, path).unwrap(),
            b"abc"
        );
    }
    #[test]
    fn bounded_linked_copy_rejects_growth_and_truncation_without_retaining_partials() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let grown = root.path().join("grown");
        let truncated = root.path().join("truncated");

        assert!(copy_bounded_reader(&b"abcd"[..], &source, &grown, 3, 3).is_err());
        assert!(!grown.exists());
        assert!(copy_bounded_reader(&b"abc"[..], &source, &truncated, 4, 4).is_err());
        assert!(!truncated.exists());
    }

    #[test]
    fn content_discovery_requires_exactly_one_regular_package() {
        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("uqm-0.8.0-content.uqm");
        fs::write(&package, b"content").unwrap();
        assert_eq!(find_content_package(root.path()).unwrap(), package);

        fs::write(root.path().join("duplicate.uqm"), b"duplicate").unwrap();
        assert!(find_content_package(root.path()).is_err());
    }

    #[test]
    fn content_materialization_removes_partial_output_after_a_conflict() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"first").unwrap();
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"second").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert!(!content_root.join("data").exists());
    }

    #[test]
    fn content_materialization_rejects_the_declared_expansion_before_writing() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"too large").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1, None).is_err());
        assert!(!content_root.join("data").exists());
    }

    #[test]
    fn content_materialization_rejects_parent_traversal() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("../escape.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"must not escape").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert!(!root.path().join("escape.txt").exists());
    }

    #[test]
    fn content_materialization_preserves_the_retained_version_marker() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("version", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"forged\n").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();
        fs::write(content_root.join("version"), b"0.8.0\n").unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert_eq!(fs::read(content_root.join("version")).unwrap(), b"0.8.0\n");
    }

    #[test]
    fn content_materialization_rejects_a_changed_retained_package_identity() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"content").unwrap();
        archive.finish().unwrap();
        let (byte_length, sha256) = regular_file_identity(&package).unwrap();
        let mut expected = NativeRetainedInput {
            relative_path: "inputs/content/packages/content.uqm".to_string(),
            byte_length,
            sha256,
        };
        expected.sha256 = "0".repeat(64);
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        assert!(
            materialize_content_package(&package, &content_root, 1024, Some(&expected)).is_err()
        );
        assert!(!content_root.join("data").exists());
    }

    #[cfg(unix)]
    #[test]
    fn content_materialization_is_readable_by_the_containment_group_but_not_writable() {
        use std::os::unix::fs::PermissionsExt as _;
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"content").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        let mut materialized =
            materialize_content_package(&package, &content_root, 1024, None).unwrap();
        assert_eq!(
            fs::metadata(content_root.join("data"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o750
        );
        assert_eq!(
            fs::metadata(content_root.join("data/value.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
        materialized.cleanup().unwrap();
    }

    #[test]
    fn content_materialization_rejects_an_oversized_archive_name() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file(
                "a".repeat(MAX_CONTENT_ARCHIVE_ENTRY_NAME_BYTES + 1),
                SimpleFileOptions::default(),
            )
            .unwrap();
        archive.write_all(b"content").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert!(fs::read_dir(&content_root).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn observer_output_rejects_symlinks_and_fifos_without_blocking() {
        use std::os::unix::fs::{symlink, OpenOptionsExt as _};

        let root = tempfile::tempdir().unwrap();
        let regular = root.path().join("regular");
        let linked = root.path().join("linked");
        let fifo = root.path().join("fifo");
        fs::write(&regular, b"outside").unwrap();
        symlink(&regular, &linked).unwrap();
        assert!(read_bounded_regular(&linked, 1024).is_err());

        let fifo_name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_name.as_ptr(), 0o600) }, 0);
        assert!(fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&fifo)
            .is_ok());
        assert!(read_bounded_regular(&fifo, 1024).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn content_materialization_rejects_archive_symlinks() {
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .add_symlink(
                "data/link",
                "../../escape.txt",
                SimpleFileOptions::default(),
            )
            .unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert!(!content_root.join("data").exists());
    }

    #[cfg(unix)]
    #[test]
    fn content_materialization_never_follows_an_existing_symlink() {
        use std::os::unix::fs::symlink;
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"must not escape").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        fs::create_dir(&content_root).unwrap();
        symlink(outside.path(), content_root.join("data")).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert!(!outside.path().join("value.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn content_materialization_rejects_a_symlink_content_root() {
        use std::os::unix::fs::symlink;
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"must not escape").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        symlink(outside.path(), &content_root).unwrap();

        assert!(materialize_content_package(&package, &content_root, 1024, None).is_err());
        assert!(!outside.path().join("data").exists());
    }

    #[cfg(unix)]
    #[test]
    fn content_cleanup_remains_bound_after_the_root_path_is_replaced() {
        use std::os::unix::fs::symlink;
        use zip::write::SimpleFileOptions;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let package = root.path().join("content.uqm");
        let file = fs::File::create(&package).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .start_file("data/value.txt", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"retained").unwrap();
        archive.finish().unwrap();
        let content_root = root.path().join("retained-content");
        let moved_root = root.path().join("retained-content-moved");
        fs::create_dir(&content_root).unwrap();
        let mut materialized =
            materialize_content_package(&package, &content_root, 1024, None).unwrap();
        fs::rename(&content_root, &moved_root).unwrap();
        symlink(outside.path(), &content_root).unwrap();

        materialized.cleanup().unwrap();
        assert!(materialized.is_removed().unwrap());
        assert!(!moved_root.join("data").exists());
        assert!(!outside.path().join("data").exists());
    }

    #[test]
    fn screenshot_publication_is_atomic_and_refuses_to_clobber() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("stable.png");
        write_bytes_atomic_noclobber(&destination, b"first").unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"first");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(
                fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
        assert!(!root.path().join("stable.png.tmp").exists());

        assert!(write_bytes_atomic_noclobber(&destination, b"second").is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"first");
        assert!(!root.path().join("stable.png.tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn screenshot_publication_rejects_a_replaced_parent_directory() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let screenshots = root.path().join("screenshots");
        let retained = root.path().join("screenshots-retained");
        fs::create_dir(&screenshots).unwrap();
        fs::rename(&screenshots, &retained).unwrap();
        symlink(outside.path(), &screenshots).unwrap();

        let destination = screenshots.join("stable.png");
        assert!(write_bytes_atomic_noclobber(&destination, b"capture").is_err());
        assert!(!outside.path().join("stable.png").exists());
        assert!(!retained.join("stable.png").exists());
    }

    #[test]
    fn native_capture_is_cropped_and_density_normalized_to_client_bounds() {
        use image::GenericImageView as _;

        let mut source = image::RgbaImage::from_pixel(6, 6, image::Rgba([1, 2, 3, 255]));
        for y in 2..6 {
            for x in 2..6 {
                source.put_pixel(x, y, image::Rgba([20, 40, 60, 255]));
            }
        }
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let os_bounds = NativeWindowBounds {
            x: 10,
            y: 20,
            width: 3,
            height: 3,
        };
        let client_bounds = NativeWindowBounds {
            x: 11,
            y: 21,
            width: 2,
            height: 2,
        };

        let normalized =
            normalize_native_capture(encoded.get_ref(), os_bounds, client_bounds, 64 * 1024)
                .unwrap();
        let decoded =
            image::load_from_memory_with_format(&normalized, image::ImageFormat::Png).unwrap();
        assert_eq!(decoded.dimensions(), (2, 2));
        assert_eq!(decoded.to_rgba8().get_pixel(0, 0).0, [20, 40, 60, 255]);
    }

    #[test]
    fn playable_screenshot_requires_a_material_scene_change() {
        fn png(image: image::RgbaImage) -> Vec<u8> {
            let mut encoded = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(image)
                .write_to(&mut encoded, image::ImageFormat::Png)
                .unwrap();
            encoded.into_inner()
        }

        let stable = image::RgbaImage::from_pixel(20, 20, image::Rgba([10, 20, 30, 255]));
        let mut noise = stable.clone();
        noise.put_pixel(0, 0, image::Rgba([255, 255, 255, 255]));
        let mut playable = stable.clone();
        for x in 0..4 {
            playable.put_pixel(x, 0, image::Rgba([200, 100, 50, 255]));
        }

        assert!(
            validate_playable_screenshot_difference(&png(stable.clone()), &png(stable)).is_err()
        );
        assert!(validate_playable_screenshot_difference(
            &png(image::RgbaImage::from_pixel(
                20,
                20,
                image::Rgba([10, 20, 30, 255]),
            )),
            &png(noise),
        )
        .is_err());
        assert!(validate_playable_screenshot_difference(
            &png(image::RgbaImage::from_pixel(
                20,
                20,
                image::Rgba([10, 20, 30, 255]),
            )),
            &png(playable),
        )
        .is_ok());
    }

    #[test]
    fn provisional_window_identity_survives_geometry_convergence() {
        let client_bounds = NativeWindowBounds {
            x: 80,
            y: 80,
            width: 1280,
            height: 960,
        };
        let mut provisional = None;
        let transitional = ObservedNativeWindow {
            window_id: 22_402,
            os_bounds: NativeWindowBounds {
                x: 321,
                y: 31,
                width: 1278,
                height: 990,
            },
            visible: true,
            minimized: false,
        };
        assert!(!bind_provisional_window(&mut provisional, &transitional, client_bounds).unwrap());
        assert_eq!(provisional, Some(transitional.window_id));

        let converged = ObservedNativeWindow {
            window_id: transitional.window_id,
            os_bounds: NativeWindowBounds {
                x: 80,
                y: 48,
                width: 1280,
                height: 992,
            },
            visible: true,
            minimized: false,
        };
        assert!(bind_provisional_window(&mut provisional, &converged, client_bounds).unwrap());

        let replacement = ObservedNativeWindow {
            window_id: converged.window_id + 1,
            ..converged
        };
        assert!(bind_provisional_window(&mut provisional, &replacement, client_bounds).is_err());
    }

    #[test]
    fn post_capture_observation_rejects_visibility_identity_and_bounds_races() {
        let before = ObservedNativeWindow {
            window_id: 41,
            os_bounds: NativeWindowBounds {
                x: 10,
                y: 20,
                width: 640,
                height: 480,
            },
            visible: true,
            minimized: false,
        };
        assert!(NativeObservationSession::validate_post_capture_window(&before, &before).is_ok());

        let mut changed = before;
        changed.minimized = true;
        assert!(NativeObservationSession::validate_post_capture_window(&before, &changed).is_err());
        changed = before;
        changed.window_id += 1;
        assert!(NativeObservationSession::validate_post_capture_window(&before, &changed).is_err());
        changed = before;
        changed.os_bounds.width += 1;
        assert!(NativeObservationSession::validate_post_capture_window(&before, &changed).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn content_discovery_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target.uqm");
        fs::write(&target, b"content").unwrap();
        symlink(&target, root.path().join("alias.uqm")).unwrap();
        assert!(find_content_package(root.path()).is_err());
    }
}
