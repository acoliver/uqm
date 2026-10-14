//! OS adapter for the native candidate's filesystem and process boundary.

use std::path::Path;
use std::process::Command;

/// Restrict only the candidate child. The controller and observer keep their existing identity.
/// There is no same-UID permission-based substitute when the OS refuses this profile.
#[cfg(target_os = "macos")]
pub fn restrict_candidate(
    command: &mut Command,
    config: &Path,
    automation: &Path,
    shared: Option<&Path>,
) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let quote = |path: &Path| -> Result<String, String> {
        let path = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
        serde_json::to_string(path.to_str().ok_or("sandbox path is not UTF-8")?)
            .map_err(|error| error.to_string())
    };
    let acknowledgement = std::fs::canonicalize(automation)
        .map_err(|error| error.to_string())?
        .join("native-window-ack.json");
    let acknowledgement = serde_json::to_string(
        acknowledgement
            .to_str()
            .ok_or("sandbox acknowledgement path is not UTF-8")?,
    )
    .map_err(|error| error.to_string())?;
    let config = quote(config)?;
    let automation = quote(automation)?;
    let executable = quote(Path::new(command.get_program()))?;
    let shared_rule = shared
        .map(|path| quote(path).map(|path| format!("(deny file-read* (subpath {path}))")))
        .transpose()?
        .unwrap_or_default();
    let profile = format!("(version 1) (allow default)
        (deny file-write* (require-all (require-not (subpath {config})) (require-not (subpath {automation})) (require-not (literal \"/dev/null\"))))
        (deny file-write* (literal {acknowledgement}))
        (deny process-fork)
        (deny process-exec (require-not (literal {executable})))
        (deny signal (require-not (target self)))
        (deny mach-priv-task-port)
        {shared_rule}");
    let profile = std::ffi::CString::new(profile).map_err(|error| error.to_string())?;
    // SAFETY: only the forked child installs the profile before exec. The CString
    // is allocated before fork and remains alive throughout sandbox_init.
    unsafe {
        command.pre_exec(move || {
            let mut error = std::ptr::null_mut();
            if sandbox_init(profile.as_ptr(), 0, &mut error) != 0 {
                if !error.is_null() {
                    sandbox_free_error(error);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "native candidate sandbox installation failed",
                ));
            }
            Ok(())
        });
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[link(name = "sandbox")]
extern "C" {
    fn sandbox_init(
        profile: *const libc::c_char,
        flags: u64,
        error: *mut *mut libc::c_char,
    ) -> libc::c_int;
    fn sandbox_free_error(error: *mut libc::c_char);
}

/// The native observer is macOS-only; unsupported hosts cannot launch unprotected candidates.
#[cfg(not(target_os = "macos"))]
pub fn restrict_candidate(
    _command: &mut Command,
    _config: &Path,
    _automation: &Path,
    _shared: Option<&Path>,
) -> Result<(), String> {
    Err("native candidate isolation requires macOS".into())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn isolated_candidate_process() {
        let Ok(root) = std::env::var("UQM_ISOLATION_TEST_ROOT") else {
            return;
        };
        use std::os::unix::fs::PermissionsExt;
        let root = Path::new(&root);
        let protected = root.join("shared/input");
        assert!(fs::write(&protected, b"tamper").is_err());
        assert!(fs::set_permissions(&protected, fs::Permissions::from_mode(0o777)).is_err());
        assert!(fs::rename(&protected, root.join("config/stolen")).is_err());
        assert!(fs::hard_link(&protected, root.join("config/alias")).is_err());
        assert!(
            fs::hard_link(root.join("inputs/uqm"), root.join("config/readable-alias")).is_err()
        );
        assert!(fs::read(&protected).is_err());
        std::os::unix::fs::symlink(&protected, root.join("config/link")).unwrap();
        assert!(fs::write(root.join("config/link"), b"tamper").is_err());
        assert!(fs::remove_dir_all(root.join("shared")).is_err());
        fs::write(root.join("config/uqm.cfg"), b"volume=17\n").unwrap();
        fs::write(root.join("automation/trace"), b"trace\n").unwrap();
        let acknowledgement = root.join("automation/native-window-ack.json");
        assert!(fs::write(&acknowledgement, b"forged").is_err());
        assert!(fs::rename(root.join("automation/trace"), &acknowledgement).is_err());
        assert!(fs::remove_file(&acknowledgement).is_err());
    }
    #[test]
    fn kernel_refuses_same_uid_candidate_mutation_and_alias_attacks() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        for directory in ["shared", "config", "automation", "inputs"] {
            fs::create_dir(root.join(directory)).unwrap();
        }
        fs::write(root.join("shared/input"), b"original").unwrap();
        fs::write(
            root.join("automation/native-window-ack.json"),
            b"controller",
        )
        .unwrap();
        fs::hard_link(root.join("shared/input"), root.join("inputs/uqm")).unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "automation::native_isolation::tests::isolated_candidate_process",
                "--nocapture",
            ])
            .env("UQM_ISOLATION_TEST_ROOT", root);
        restrict_candidate(
            &mut command,
            &root.join("config"),
            &root.join("automation"),
            Some(&root.join("shared")),
        )
        .unwrap();
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read(root.join("shared/input")).unwrap(), b"original");
        assert_eq!(
            fs::read(root.join("config/uqm.cfg")).unwrap(),
            b"volume=17\n"
        );
    }
}
