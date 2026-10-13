//! `expandPath`: the string expansions of `dirs.c`, over byte strings.

use std::ffi::c_int;
use std::io;

/// Expand a leading `~` to the home directory.
pub const EP_HOME: c_int = 1;
/// Make a relative path absolute with the working directory.
pub const EP_ABSOLUTE: c_int = 2;
/// Expand `$NAME` and `${NAME}`.
pub const EP_ENVVARS: c_int = 4;
/// Process `.` and `..` components.
pub const EP_DOTS: c_int = 8;
/// Treat `\` as a separator and replace it by `/`.
pub const EP_SLASHES: c_int = 16;
/// Collapse runs of `/`.
pub const EP_SINGLESEP: c_int = 32;
/// `EP_ALL_SYSTEM` on Unix: everything except `EP_SLASHES`.
pub const EP_ALL_SYSTEM: c_int = EP_HOME | EP_ABSOLUTE | EP_ENVVARS | EP_DOTS | EP_SINGLESEP;

/// What `expandPath` reads from its surroundings.
pub trait Environment {
    /// `getenv(name)`; raw bytes.
    fn var(&self, name: &[u8]) -> Option<Vec<u8>>;
    /// `getHomeDir()`; `None` when there is neither `HOME` nor a passwd entry.
    fn home_dir(&self) -> Option<Vec<u8>>;
    /// `getcwd`.
    fn current_dir(&self) -> io::Result<Vec<u8>>;
}

fn errno(code: c_int) -> io::Error {
    io::Error::from_raw_os_error(code)
}

/// C's `CHECKLEN`: `prefix + needed` bytes plus the NUL must fit in `len`.
fn check_room(prefix: usize, needed: usize, len: usize) -> io::Result<()> {
    if prefix + needed >= len {
        Err(errno(libc::ENAMETOOLONG))
    } else {
        Ok(())
    }
}

/// Expands `src` as `expandPath(dest, len, src, what)` does and returns the
/// bytes that would be written before the terminating NUL. On success the
/// result is shorter than `len`, so it and its NUL fit the caller's buffer.
///
/// Errors carry the errno C sets: `ENAMETOOLONG` when the result does not fit,
/// `EINVAL` for `~` not followed by `/`, an unterminated `${` or a `..` with
/// nothing to remove, `ENOENT` for `~` without a home directory, and
/// whatever `current_dir` reports (`ERANGE` when `len` cannot hold it).
pub fn expand_path(
    env: &dyn Environment,
    src: &[u8],
    len: usize,
    what: c_int,
) -> io::Result<Vec<u8>> {
    let expanded;
    let mut rest = if what & EP_ENVVARS != 0 {
        expanded = expand_env_vars(env, src, len)?;
        &expanded[..]
    } else {
        src
    };
    let mut dest = Vec::new();
    let slashes = what & EP_SLASHES != 0;
    let mut absolute = what & EP_ABSOLUTE != 0;

    if what & EP_HOME != 0 && rest.first() == Some(&b'~') {
        if rest.get(1) != Some(&b'/') {
            return Err(errno(libc::EINVAL));
        }
        let home = env.home_dir().ok_or_else(|| errno(libc::ENOENT))?;
        if absolute {
            push_absolute_prefix(env, &mut dest, len, &home, slashes)?;
            // The part after the '~' is not made absolute again.
            absolute = false;
        }
        check_room(dest.len(), home.len(), len)?;
        dest.extend_from_slice(&home);
        rest = &rest[1..];
    }
    if absolute {
        push_absolute_prefix(env, &mut dest, len, rest, slashes)?;
    }
    check_room(dest.len(), rest.len(), len)?;
    dest.extend_from_slice(rest);

    if slashes {
        dest.iter_mut()
            .filter(|byte| **byte == b'\\')
            .for_each(|byte| *byte = b'/');
    }
    if what & EP_DOTS != 0 {
        dest = remove_dot_components(&dest)?;
    }
    if what & EP_SINGLESEP != 0 {
        dest.dedup_by(|current, previous| *current == b'/' && *previous == b'/');
    }
    Ok(dest)
}

/// The `EP_ENVVARS` pass: `$NAME` (ASCII letters, digits, `_`) and `${NAME}`.
/// Unset names expand to nothing, values are inserted unexpanded, and the
/// scratch result is bounded by `len` exactly as C's `HMalloc (len)` buffer.
fn expand_env_vars(env: &dyn Environment, src: &[u8], len: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < src.len() {
        if src[at] != b'$' {
            check_room(out.len(), 1, len)?;
            out.push(src[at]);
            at += 1;
            continue;
        }
        let (name, next) = variable_name(src, at + 1)?;
        if let Some(value) = env.var(name) {
            check_room(out.len(), value.len(), len)?;
            out.extend_from_slice(&value);
        }
        at = next;
    }
    Ok(out)
}

/// The variable name starting at `from` (just past the `$`) and the index
/// where scanning resumes.
fn variable_name(src: &[u8], from: usize) -> io::Result<(&[u8], usize)> {
    if src.get(from) == Some(&b'{') {
        let name_start = from + 1;
        let close = src[name_start..]
            .iter()
            .position(|byte| *byte == b'}')
            .ok_or_else(|| errno(libc::EINVAL))?;
        let name_end = name_start + close;
        return Ok((&src[name_start..name_end], name_end + 1));
    }
    let name_len = src[from..]
        .iter()
        .take_while(|byte| byte.is_ascii_alphanumeric() || **byte == b'_')
        .count();
    Ok((&src[from..from + name_len], from + name_len))
}

/// `expandPathAbsolute` for Unix: a path starting with `/` (or `\` when
/// backslashes are separators) is already absolute; any other gets the
/// working directory and a joining `/` in front.
fn push_absolute_prefix(
    env: &dyn Environment,
    dest: &mut Vec<u8>,
    len: usize,
    path: &[u8],
    slashes: bool,
) -> io::Result<()> {
    if matches!(path.first(), Some(b'/')) || (slashes && path.first() == Some(&b'\\')) {
        return Ok(());
    }
    // getcwd(dest, 0) fails with EINVAL; a buffer smaller than cwd + NUL with ERANGE.
    if len == 0 {
        return Err(errno(libc::EINVAL));
    }
    let cwd = env.current_dir()?;
    if cwd.is_empty() {
        return Err(errno(libc::ENOENT));
    }
    if cwd.len() + 1 > len {
        return Err(errno(libc::ERANGE));
    }
    dest.extend_from_slice(&cwd);
    if dest.last() != Some(&b'/') {
        dest.push(b'/');
    }
    Ok(())
}

/// The `EP_DOTS` pass over a complete path. Removing `..` keeps the quirks of
/// the C loop: it only looks for the previous separator before the final
/// byte written so far, a `..` that is the last component drops the slash the
/// previous component ended with, and one with nothing before it is `EINVAL`.
fn remove_dot_components(path: &[u8]) -> io::Result<Vec<u8>> {
    let root_len = usize::from(path.first() == Some(&b'/'));
    let (root, components) = path.split_at(root_len);
    let mut out = Vec::with_capacity(components.len());
    let mut start = 0;
    loop {
        let end = components[start..]
            .iter()
            .position(|byte| *byte == b'/')
            .map_or(components.len(), |slash| start + slash);
        let component = &components[start..end];
        let has_slash = end < components.len();
        match component {
            b"." => {}
            b".." => step_out(&mut out, has_slash)?,
            _ => {
                out.extend_from_slice(component);
                if has_slash {
                    out.push(b'/');
                }
            }
        }
        if !has_slash {
            break;
        }
        start = end + 1;
    }
    Ok([root, &out].concat())
}

/// Handles one `..`: drops the last component of `out`.
fn step_out(out: &mut Vec<u8>, has_slash: bool) -> io::Result<()> {
    let searchable = out.len().saturating_sub(1);
    match out[..searchable].iter().rposition(|byte| *byte == b'/') {
        Some(slash) => {
            out.truncate(slash);
            if has_slash {
                out.push(b'/');
            }
        }
        None if out.is_empty() => return Err(errno(libc::EINVAL)),
        None => out.clear(),
    }
    Ok(())
}
