//! `mkdirhier` and `createDirectory` from `dirs.c`.

use std::ffi::OsStr;
use std::fs::DirBuilder;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

/// Mode `mkdirhier` asks for; the process umask narrows it.
const NEW_DIRECTORY_MODE: u32 = 0o777;

/// `createDirectory`: `mkdir(path, mode)`, nothing more. `mode` goes through
/// the same narrowing to `mode_t` that C's implicit conversion applies.
pub fn create_directory(path: &Path, mode: u32) -> io::Result<()> {
    DirBuilder::new().mode(mode).create(path)
}

fn path_of(bytes: &[u8]) -> &Path {
    Path::new(OsStr::from_bytes(bytes))
}

/// `mkdirhier`: creates every missing component of `path`.
///
/// Follows the C walk: components are examined from the front, the first one
/// that does not exist (`stat` fails with `ENOENT`) starts the creation run,
/// and every component from there on is made with `mkdir(…, 0777)`. Runs of
/// `/` collapse between components; a leading run keeps two slashes after an
/// empty first component, as in C. Directories made before a failure stay.
///
/// An existing component that is not a directory is `ENOTDIR`, also for the
/// final one. C only gets that error for a component with something below it
/// (its next `stat` fails) and returns success for a regular file at the end;
/// refusing it is the requested "error on file collision" behaviour.
pub fn mkdirhier(path: &[u8]) -> io::Result<()> {
    let mut buf = Vec::with_capacity(path.len() + 1);
    let mut start = 0;
    if path.first() == Some(&b'/') {
        buf.push(b'/');
        start = 1;
    }
    if start == path.len() {
        return Ok(());
    }

    let mut end = component_end(path, start);
    // Walk the components that already exist.
    loop {
        buf.extend_from_slice(&path[start..end]);
        match check_existing(&buf) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(libc::ENOENT) => break,
            Err(error) => return Err(error),
        }
        if end == path.len() {
            return Ok(());
        }
        buf.push(b'/');
        start = skip_slashes(path, end + 1);
        if start == path.len() {
            return Ok(());
        }
        end = component_end(path, start);
    }

    // Create the components that are left.
    loop {
        if let Err(error) = create_directory(path_of(&buf), NEW_DIRECTORY_MODE) {
            crate::log_error!("Error: Can't create {}: {error}", path_of(&buf).display());
            return Err(error);
        }
        if end == path.len() {
            return Ok(());
        }
        buf.push(b'/');
        start = skip_slashes(path, end + 1);
        if start == path.len() {
            return Ok(());
        }
        end = component_end(path, start);
        buf.extend_from_slice(&path[start..end]);
    }
}

/// `stat` the component: `Ok` if it exists as a directory, `ENOENT` if it
/// does not exist, any other error as the OS reports it.
fn check_existing(buf: &[u8]) -> io::Result<()> {
    match std::fs::metadata(path_of(buf)) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => {
            let error = io::Error::from_raw_os_error(libc::ENOTDIR);
            crate::log_error!("Can't use \"{}\": {error}", path_of(buf).display());
            Err(error)
        }
        Err(error) if error.raw_os_error() == Some(libc::ENOENT) => Err(error),
        Err(error) => {
            crate::log_error!("Can't stat \"{}\": {error}", path_of(buf).display());
            Err(error)
        }
    }
}

/// Index of the `/` ending the component that starts at `start`, or the end of `path`.
fn component_end(path: &[u8], start: usize) -> usize {
    path[start..]
        .iter()
        .position(|byte| *byte == b'/')
        .map_or(path.len(), |slash| start + slash)
}

fn skip_slashes(path: &[u8], from: usize) -> usize {
    from + path[from..]
        .iter()
        .take_while(|byte| **byte == b'/')
        .count()
}
