//! Redirecting the process's standard error to a file.

/// Make the process's standard error write to a newly created file at `path`.
///
/// Both the C runtime's descriptor 2 and, on Windows, the Win32 standard-error
/// handle follow the file, so Rust, C and the OS agree on where stderr goes.
///
/// # Errors
///
/// Returns the OS error if the file cannot be created or attached.
pub fn redirect_stderr(path: &std::path::Path) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        // SAFETY: file and stderr are valid CRT descriptors.
        if unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(windows)]
    {
        use super::errno::errno;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::Console::{SetStdHandle, STD_ERROR_HANDLE};
        // SAFETY: transfer the file handle to the CRT, duplicate it onto stderr,
        // then point Rust/Win32 stderr at that same live CRT-owned handle.
        unsafe {
            let fd = libc::open_osfhandle(
                file.as_raw_handle() as isize,
                libc::O_WRONLY | libc::O_BINARY,
            );
            if fd == -1 {
                return Err(std::io::Error::from_raw_os_error(errno()));
            }
            std::mem::forget(file);
            let result = libc::dup2(fd, 2);
            libc::close(fd);
            if result == -1 {
                return Err(std::io::Error::from_raw_os_error(errno()));
            }
            if SetStdHandle(STD_ERROR_HANDLE, libc::get_osfhandle(2) as _) == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::redirect_stderr;

    #[test]
    fn a_path_that_cannot_be_created_leaves_stderr_alone() {
        let root = tempfile::tempdir().unwrap();
        let error = redirect_stderr(&root.path().join("missing/stderr.log")).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn redirected_stderr_lands_in_the_file() {
        // Redirecting the test binary's own descriptor would swallow every later
        // test's output, so a child process performs the redirect.
        let root = tempfile::tempdir().unwrap();
        let log = root.path().join("stderr.log");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "os::stderr::tests::redirect_child",
                "--nocapture",
            ])
            .env("UQM_STDERR_REDIRECT_TARGET", &log)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "redirected\n");
    }

    /// Runs only as the child of `redirected_stderr_lands_in_the_file`.
    #[test]
    fn redirect_child() {
        let Some(target) = std::env::var_os("UQM_STDERR_REDIRECT_TARGET") else {
            return;
        };
        redirect_stderr(std::path::Path::new(&target)).unwrap();
        eprintln!("redirected");
    }
}
