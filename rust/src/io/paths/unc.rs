//! `uio_skipUNCServerShare` and `uio_getUNCServerShare`: recognising the
//! `\\server\share` prefix of a Windows UNC path in a byte string.
//!
//! Both are compiled into `paths.c` on every platform, and the one native
//! caller (`uioutils.c`) calls the skip function everywhere, so both are
//! ported without a platform gate. The two differ in which bytes end the
//! server and share parts: [`skip`] accepts `\` or `/` unconditionally, as
//! the C does; [`get`] uses `isPathDelimiter`, which is `/` alone on the
//! supported platforms.

use super::components::SEPARATOR;

const BACKSLASH: u8 = b'\\';

/// A recognised UNC prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerShare {
    /// `\\server\share`, with a backslash between the two names.
    pub name: Vec<u8>,
    /// Bytes of the input the prefix and the separators after it use.
    pub consumed: usize,
}

/// `uio_skipUNCServerShare`: the length of the `\\server\share` prefix of
/// `path`, or 0 when it has none. Trailing separators are not skipped.
/// `path` is the bytes before the terminating NUL.
pub fn skip(path: &[u8]) -> usize {
    if !path.starts_with(&[BACKSLASH, BACKSLASH]) {
        return 0;
    }
    let is_end = |b: &u8| *b == BACKSLASH || *b == SEPARATOR;
    let Some(server_len) = path[2..].iter().position(is_end) else {
        return 0;
    };
    // Past the separator that ends the server part, then the share part.
    let share_start = 2 + server_len + 1;
    let share_len = path[share_start..]
        .iter()
        .position(is_end)
        .unwrap_or(path.len() - share_start);
    share_start + share_len
}

/// `uio_getUNCServerShare`: the `\\server\share` prefix of `path` and how many
/// bytes to skip to reach what follows it (including any separators), or
/// `None` when `path` is not a UNC path. The server name must not be empty.
///
/// The C original never terminates on a non-empty share (its loop advances
/// the server end instead of the share end) and overwrites the first byte of
/// the server name; this follows the documented behaviour.
pub fn get(path: &[u8]) -> Option<ServerShare> {
    if !path.starts_with(&[BACKSLASH, BACKSLASH]) {
        return None;
    }
    let server = &path[2..];
    let server_len = server.iter().position(|&b| b == SEPARATOR)?;
    if server_len == 0 {
        return None;
    }
    let share_start = 2 + server_len + 1;
    let share_len = path[share_start..]
        .iter()
        .position(|&b| b == SEPARATOR)
        .unwrap_or(path.len() - share_start);
    let share_end = share_start + share_len;
    let trailing = path[share_end..]
        .iter()
        .take_while(|&&b| b == SEPARATOR)
        .count();

    let mut name = Vec::with_capacity(server_len + share_len + 3);
    name.extend_from_slice(&[BACKSLASH, BACKSLASH]);
    name.extend_from_slice(&server[..server_len]);
    name.push(BACKSLASH);
    name.extend_from_slice(&path[share_start..share_end]);
    Some(ServerShare {
        name,
        consumed: share_end + trailing,
    })
}
