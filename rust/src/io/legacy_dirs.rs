//! Unix directory/path C ABI from `libs/file/dirs.c`. Expansion is lexical:
//! it does not resolve symlinks, and backing out past the first component fails.

use std::ffi::{c_char, c_int, CStr, CString};

const EP_HOME: c_int = 1;
const EP_ABSOLUTE: c_int = 2;
const EP_ENVVARS: c_int = 4;
const EP_DOTS: c_int = 8;
const EP_SLASHES: c_int = 16;
const EP_SINGLESEP: c_int = 32;

fn set_errno(error: c_int) {
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = error;
    }
    #[cfg(target_os = "linux")]
    unsafe {
        *libc::__errno_location() = error;
    }
}

/// # Safety
/// `dir` must be a NUL-terminated path.
pub unsafe extern "C" fn create_directory(dir: *const c_char, mode: c_int) -> c_int {
    libc::mkdir(dir, mode as libc::mode_t)
}

/// # Safety
/// `path` must be a NUL-terminated path. Existing final entries, including
/// files, succeed as they did in C; an intermediate file fails on the next stat.
pub unsafe extern "C" fn mkdirhier(path: *const c_char) -> c_int {
    let path = CStr::from_ptr(path).to_bytes();
    let mut buffer = Vec::with_capacity(path.len() + 2);
    let mut start = 0;
    if path.first() == Some(&b'/') {
        buffer.push(b'/');
        start = 1;
    }
    if start == path.len() {
        return 0;
    }
    let mut creating = false;
    loop {
        let end = path[start..]
            .iter()
            .position(|&c| c == b'/')
            .map_or(path.len(), |i| start + i);
        buffer.extend_from_slice(&path[start..end]);
        buffer.push(0);
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        if !creating && libc::stat(buffer.as_ptr().cast(), stat.as_mut_ptr()) == -1 {
            let error = std::io::Error::last_os_error().raw_os_error().unwrap();
            if error != libc::ENOENT {
                crate::log_abi::log_message(
                    2,
                    &format!(
                        r#"Can't stat "{}": {}"#,
                        CStr::from_ptr(buffer.as_ptr().cast()).to_string_lossy(),
                        CStr::from_ptr(libc::strerror(error)).to_string_lossy(),
                    ),
                );
                set_errno(error);
                return -1;
            }
            creating = true;
        }
        if creating && create_directory(buffer.as_ptr().cast(), 0o777) == -1 {
            let error = std::io::Error::last_os_error().raw_os_error().unwrap();
            crate::log_abi::log_message(
                2,
                &format!(
                    "Error: Can't create {}: {}",
                    CStr::from_ptr(buffer.as_ptr().cast()).to_string_lossy(),
                    CStr::from_ptr(libc::strerror(error)).to_string_lossy(),
                ),
            );
            set_errno(error);
            return -1;
        }
        buffer.pop();
        if end == path.len() {
            return 0;
        }
        buffer.push(b'/');
        start = end + 1;
        while start < path.len() && path[start] == b'/' {
            start += 1;
        }
        if start == path.len() {
            return 0;
        }
    }
}

/// Returns the libc-owned HOME or passwd entry buffer.
pub extern "C" fn get_home_dir() -> *const c_char {
    unsafe {
        let home = libc::getenv(c"HOME".as_ptr());
        if !home.is_null() {
            return home;
        }
        let user = libc::getpwuid(libc::getuid());
        if user.is_null() {
            std::ptr::null()
        } else {
            (*user).pw_dir
        }
    }
}

fn append(dest: &mut Vec<u8>, bytes: &[u8], limit: usize) -> Result<(), c_int> {
    if dest.len() + bytes.len() >= limit {
        return Err(libc::ENAMETOOLONG);
    }
    dest.extend_from_slice(bytes);
    Ok(())
}

fn expand_bytes(
    src: &[u8],
    limit: usize,
    mut what: c_int,
    env: impl Fn(&[u8]) -> Option<Vec<u8>>,
    home: impl Fn() -> Option<Vec<u8>>,
    cwd: impl Fn() -> Result<Vec<u8>, c_int>,
) -> Result<Vec<u8>, c_int> {
    let mut expanded = Vec::new();
    if what & EP_ENVVARS != 0 {
        let mut i = 0;
        while i < src.len() {
            if src[i] != b'$' {
                append(&mut expanded, &src[i..i + 1], limit)?;
                i += 1;
                continue;
            }
            i += 1;
            let (name, end) = if src.get(i) == Some(&b'{') {
                i += 1;
                let end = src[i..]
                    .iter()
                    .position(|&c| c == b'}')
                    .ok_or(libc::EINVAL)?
                    + i;
                (&src[i..end], end + 1)
            } else {
                let start = i;
                while i < src.len() && (src[i].is_ascii_alphanumeric() || src[i] == b'_') {
                    i += 1;
                }
                (&src[start..i], i)
            };
            if let Some(value) = env(name) {
                append(&mut expanded, &value, limit)?;
            }
            i = end;
        }
    } else {
        expanded.extend_from_slice(src);
    }
    let mut src = expanded.as_slice();
    let mut dest = Vec::new();
    let slashes = what & EP_SLASHES != 0;
    let absolute_prefix = |path: &[u8]| -> Result<Vec<u8>, c_int> {
        if path.first() == Some(&b'/') || (slashes && path.first() == Some(&b'\\')) {
            return Ok(Vec::new());
        }
        let mut prefix = cwd()?;
        // getcwd fails with ERANGE when its NUL-terminated result won't fit.
        if prefix.len() >= limit {
            return Err(libc::ERANGE);
        }
        if prefix.is_empty() {
            return Err(libc::ENOENT);
        }
        if prefix.last() != Some(&b'/') {
            prefix.push(b'/');
        }
        Ok(prefix)
    };
    if what & EP_HOME != 0 && src.first() == Some(&b'~') {
        if src.get(1) != Some(&b'/') {
            return Err(libc::EINVAL);
        }
        let home = home().ok_or(libc::ENOENT)?;
        if what & EP_ABSOLUTE != 0 {
            dest = absolute_prefix(&home)?;
            what &= !EP_ABSOLUTE;
        }
        append(&mut dest, &home, limit)?;
        src = &src[1..];
    }
    if what & EP_ABSOLUTE != 0 {
        dest = absolute_prefix(src)?;
    }
    append(&mut dest, src, limit)?;
    if what & EP_SLASHES != 0 {
        for byte in &mut dest {
            if *byte == b'\\' {
                *byte = b'/';
            }
        }
    }
    if what & EP_DOTS != 0 {
        let path_start = usize::from(dest.first() == Some(&b'/'));
        let mut output = dest[..path_start].to_vec();
        let mut parts = dest[path_start..].split(|&c| c == b'/').peekable();
        while let Some(part) = parts.next() {
            let slash = parts.peek().is_some();
            match part {
                b"." => {}
                b".." => {
                    let previous = (path_start..output.len().saturating_sub(1))
                        .rev()
                        .find(|&j| output[j] == b'/');
                    if let Some(previous) = previous {
                        output.truncate(previous + usize::from(slash));
                    } else if output.len() == path_start {
                        return Err(libc::EINVAL);
                    } else {
                        output.truncate(path_start);
                    }
                }
                _ => {
                    output.extend_from_slice(part);
                    if slash {
                        output.push(b'/');
                    }
                }
            }
        }
        dest = output;
    }
    if what & EP_SINGLESEP != 0 {
        let mut previous = 0;
        dest.retain(|&c| {
            let keep = c != b'/' || previous != b'/';
            previous = c;
            keep
        });
    }
    Ok(dest)
}

/// # Safety
/// `src` is NUL-terminated; `dest` points to `len` writable bytes. Environment
/// changes must be serialized with this call, as with the original libc API.
pub unsafe extern "C" fn expand_path(
    dest: *mut c_char,
    len: usize,
    src: *const c_char,
    what: c_int,
) -> c_int {
    let result = expand_bytes(
        CStr::from_ptr(src).to_bytes(),
        len,
        what,
        |name| {
            let name = CString::new(name).unwrap();
            let value = libc::getenv(name.as_ptr());
            (!value.is_null()).then(|| CStr::from_ptr(value).to_bytes().to_vec())
        },
        || {
            let home = get_home_dir();
            (!home.is_null()).then(|| CStr::from_ptr(home).to_bytes().to_vec())
        },
        || {
            let mut buffer = vec![0u8; len];
            if libc::getcwd(buffer.as_mut_ptr().cast(), len).is_null() {
                Err(std::io::Error::last_os_error().raw_os_error().unwrap())
            } else {
                Ok(CStr::from_ptr(buffer.as_ptr().cast()).to_bytes().to_vec())
            }
        },
    );
    match result {
        Ok(bytes) => {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), dest, bytes.len());
            *dest.add(bytes.len()) = 0;
            0
        }
        Err(error) => {
            set_errno(error);
            -1
        }
    }
}

#[cfg(test)]
#[path = "legacy_dirs_tests.rs"]
mod tests;
