//! Real dedicated-identity regressions for protected retirement inputs.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::{authority::Authority, delta, evidence::hex_sha256, exec, retirement_provider};

pub(crate) const CHECK_COMMAND: &str = "__ci-retirement-containment-check";

pub(crate) fn run_check(root: &Path, arguments: &[String]) -> Result<(), String> {
    let [path, digest, expected_uid] = arguments else {
        return Err(
            "retirement containment check requires witness, digest and contained UID".into(),
        );
    };
    let expected_uid = expected_uid
        .parse::<u32>()
        .map_err(|error| error.to_string())?;
    let actual_uid = unsafe { libc::geteuid() };
    let issuer = fs::metadata(std::env::current_exe().map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())?
        .uid();
    if actual_uid != expected_uid || issuer == actual_uid {
        return Err(
            "retirement containment check did not cross the controller UID boundary".into(),
        );
    }
    let sources = retirement_provider::read_witness(root, &[path.clone(), digest.clone()])?;
    if !sources.entries.is_empty() {
        return Err("containment regression must use an unchanged source binding".into());
    }
    println!(
        "{}",
        serde_json::json!({"schema": "uqm.retirement-containment.v1", "controller_uid": issuer, "contained_uid": actual_uid, "witness": "verified"})
    );
    Ok(())
}

struct Probe<'a> {
    root: &'a Path,
    directory: &'a Path,
    controller: PathBuf,
    path: PathBuf,
    digest: String,
    uid: u32,
    authority: Authority,
    binding: Vec<(String, String)>,
}

impl Probe<'_> {
    fn check(&self, digest: &str) -> exec::Captured {
        exec::run_captured_with_bound_environment(
            self.root,
            &self.controller.display().to_string(),
            &[
                CHECK_COMMAND.into(),
                self.path.display().to_string(),
                digest.into(),
                self.uid.to_string(),
            ],
            self.authority.supervision.builtin_limits(),
            true,
            |_| Ok(self.binding.clone()),
        )
    }

    fn positive(&self) -> Result<(), String> {
        let captured = self.check(&self.digest);
        if !captured.succeeded() {
            return Err(format!(
                "{}: {}",
                captured.failure_detail("cross-UID retirement witness"),
                String::from_utf8_lossy(&captured.stderr)
            ));
        }
        let receipt: serde_json::Value =
            serde_json::from_slice(&captured.stdout).map_err(|error| error.to_string())?;
        if receipt["schema"] != "uqm.retirement-containment.v1"
            || receipt["controller_uid"] != unsafe { libc::geteuid() }
            || receipt["contained_uid"] != self.uid
            || receipt["witness"] != "verified"
        {
            return Err("cross-UID retirement witness receipt differs".into());
        }
        println!("retirement containment proof: {receipt}");
        Ok(())
    }

    fn deny(&self, label: &str, digest: &str, expected: &str) -> Result<(), String> {
        let captured = self.check(digest);
        if !captured.completed_under_supervision()
            || captured.exit_code != Some(1)
            || !String::from_utf8_lossy(&captured.stderr).contains(expected)
        {
            return Err(format!(
                "retirement negative {label}: {}: {}",
                captured.failure_detail(label),
                String::from_utf8_lossy(&captured.stderr)
            ));
        }
        println!("retirement containment negative: {label}: rejected");
        Ok(())
    }

    fn filesystem_negatives(&self) -> Result<(), String> {
        self.deny("wrong digest", &"0".repeat(64), "hash differs")?;
        for (path, mode, label) in [
            (self.path.as_path(), 0o640, "writable witness"),
            (self.directory, 0o770, "writable staging"),
            (self.controller.as_path(), 0o750, "writable controller"),
        ] {
            let original = fs::symlink_metadata(path)
                .map_err(|error| error.to_string())?
                .permissions();
            fs::set_permissions(path, fs::Permissions::from_mode(mode))
                .map_err(|error| error.to_string())?;
            let result = self.deny(label, &self.digest, "invalid ownership, mode or file kind");
            fs::set_permissions(path, original).map_err(|error| error.to_string())?;
            result?;
        }
        fs::remove_file(&self.path).map_err(|error| error.to_string())?;
        self.deny("missing witness", &self.digest, "No such file")?;
        std::os::unix::fs::symlink(&self.controller, &self.path)
            .map_err(|error| error.to_string())?;
        self.deny(
            "symlink witness",
            &self.digest,
            "invalid ownership, mode or file kind",
        )?;
        fs::remove_file(&self.path).map_err(|error| error.to_string())?;
        Ok(())
    }

    fn binding_negatives(&self, original: &[u8]) -> Result<(), String> {
        for field in ["authority_sha256", "head_sha", "base_sha"] {
            let mut changed: serde_json::Value =
                serde_json::from_slice(original).map_err(|error| error.to_string())?;
            changed[field] = "0"
                .repeat(if field == "authority_sha256" { 64 } else { 40 })
                .into();
            let bytes = serde_json::to_vec(&changed).map_err(|error| error.to_string())?;
            self.write_witness(&bytes)?;
            self.deny("changed bytes", &self.digest, "hash differs")?;
            self.deny(
                field,
                &hex_sha256(&bytes),
                if field == "base_sha" {
                    "validate base-to-head ancestry"
                } else {
                    "binding differs"
                },
            )?;
            fs::remove_file(&self.path).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn write_witness(&self, bytes: &[u8]) -> Result<(), String> {
        fs::write(&self.path, bytes).map_err(|error| error.to_string())?;
        fs::set_permissions(&self.path, fs::Permissions::from_mode(0o440))
            .map_err(|error| error.to_string())
    }

    fn candidate_owned_negative(&self, home: &Path, original: &[u8]) -> Result<(), String> {
        let candidate = tempfile::Builder::new()
            .prefix("candidate-witness-")
            .tempdir_in(home)
            .map_err(|error| error.to_string())?;
        fs::set_permissions(candidate.path(), fs::Permissions::from_mode(0o770))
            .map_err(|error| error.to_string())?;
        let path = candidate.path().join("witness");
        self.write_witness(original)?;
        let forged = exec::run_captured_with_bound_environment(
            self.root,
            "/bin/sh",
            &[
                "-c".into(),
                "cp \"$1\" \"$2\" && chmod 0440 \"$2\"".into(),
                "candidate-witness".into(),
                self.path.display().to_string(),
                path.display().to_string(),
            ],
            self.authority.supervision.builtin_limits(),
            true,
            |_| Ok(Vec::new()),
        );
        if !forged.succeeded()
            || fs::symlink_metadata(&path)
                .map_err(|error| error.to_string())?
                .uid()
                != self.uid
        {
            return Err(format!(
                "candidate ownership fixture failed: {}",
                String::from_utf8_lossy(&forged.stderr)
            ));
        }
        fs::remove_file(&self.path).map_err(|error| error.to_string())?;
        fs::rename(path, &self.path).map_err(|error| error.to_string())?;
        self.deny(
            "candidate-owned witness",
            &self.digest,
            "invalid ownership, mode or file kind",
        )
    }
}

fn containment_staging_parent(root: &Path, authority: &Path) -> Result<PathBuf, String> {
    // This preflight runs before the gate step sets its staging-root variable.
    // The separately bound policy already lives below the protected shared root.
    let parent = authority
        .parent()
        .ok_or("containment authority has no parent")?;
    super::run::checked_staging_parent(root, Some(parent.as_os_str().to_owned()), true)
        .map_err(|error| error.to_string())
}

pub(super) fn verify(root: &Path, home: &Path) -> Result<(), String> {
    let authority = super::load_authority(root)?;
    let authority_path =
        std::env::var("UQM_CI_AUTHORITY_PATH").map_err(|error| error.to_string())?;
    let parent = containment_staging_parent(root, Path::new(&authority_path))?;
    let uid = std::env::var(exec::DEDICATED_CONTAINMENT_UID_ENV)
        .map_err(|error| error.to_string())?
        .parse::<u32>()
        .map_err(|error| error.to_string())?;
    let head = crate::git_text(root, &["rev-parse", "HEAD"], "containment regression head")?;
    let sources = delta::SourceRetirements {
        base_sha: head.clone(),
        head_sha: head,
        entries: Vec::new(),
    };
    let (directory, arguments) = retirement_provider::stage_direct_witness(
        &parent,
        root,
        &sources,
        authority.supervision.executable_member_limit_bytes,
    )?;
    let probe = Probe {
        root,
        directory: directory.path(),
        controller: directory.path().join("uqm-base-controller"),
        path: PathBuf::from(&arguments[0]),
        digest: arguments[1].clone(),
        uid,
        authority,
        binding: vec![
            ("UQM_CI_SOURCE_ROOT".into(), root.display().to_string()),
            ("UQM_CI_AUTHORITY_PATH".into(), authority_path),
        ],
    };
    let original = fs::read(&probe.path).map_err(|error| error.to_string())?;
    probe.positive()?;
    probe.filesystem_negatives()?;
    probe.binding_negatives(&original)?;
    probe.candidate_owned_negative(home, &original)?;
    verify_projection(root, &probe.authority, &parent)
}

fn verify_projection(root: &Path, authority: &Authority, parent: &Path) -> Result<(), String> {
    let diff = delta::projection_diff_in(
        root,
        authority,
        parent,
        &["#define OLD 1".into()],
        &["#define NEW 1".into()],
    )?;
    if !diff.contains("-#define OLD 1") || !diff.contains("+#define NEW 1") {
        return Err("cross-UID projection lost its textual hunk".into());
    }
    let private = tempfile::Builder::new()
        .prefix("unreadable-projection-")
        .tempdir_in(parent)
        .map_err(|error| error.to_string())?;
    let paths = [private.path().join("base"), private.path().join("head")];
    for file in &paths {
        fs::write(file, b"unreadable\n").map_err(|error| error.to_string())?;
    }
    let error = delta::projection_diff_paths(root, authority, &paths)
        .err()
        .ok_or("contained Git read a controller-private directory")?;
    if !error.contains("Could not access") && !error.contains("Permission denied") {
        return Err(format!("projection read error lost Git stderr: {error}"));
    }
    println!("projection containment proof: textual diff passed; private-parent rejected: {error}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn containment_preflight_uses_the_protected_authority_parent_before_gate_environment() {
        let source = tempfile::tempdir().unwrap();
        let shared = tempfile::tempdir().unwrap();
        fs::set_permissions(shared.path(), fs::Permissions::from_mode(0o750)).unwrap();
        let authority = shared.path().join("authority.json");
        assert_eq!(
            containment_staging_parent(source.path(), &authority).unwrap(),
            shared.path()
        );
        fs::set_permissions(shared.path(), fs::Permissions::from_mode(0o770)).unwrap();
        assert!(containment_staging_parent(source.path(), &authority).is_err());
        fs::set_permissions(shared.path(), fs::Permissions::from_mode(0o750)).unwrap();
        let link = source.path().join("authority-parent");
        std::os::unix::fs::symlink(shared.path(), &link).unwrap();
        assert!(containment_staging_parent(source.path(), &link.join("authority.json")).is_err());
        assert!(
            containment_staging_parent(source.path(), Path::new("relative/authority.json"))
                .is_err()
        );
    }

    #[test]
    fn containment_receipt_cannot_be_claimed_by_same_uid_execution() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let error = run_check(
            root,
            &[
                "unused".into(),
                "0".repeat(64),
                unsafe { libc::geteuid() }.to_string(),
            ],
        )
        .unwrap_err();
        assert!(error.contains("did not cross the controller UID boundary"));
        assert!(run_check(root, &[]).is_err());
        assert!(run_check(root, &["unused".into(), "0".repeat(64), "not-a-uid".into()]).is_err());
    }
}
