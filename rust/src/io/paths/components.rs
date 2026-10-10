//! The byte-string rules of `paths.c`: walking the components of a path,
//! `validPathName`, `joinPaths` and `joinPathsAbsolute`, and the POSIX
//! splitting and joining behind `decomposePath` and `composePath`.
//!
//! One authority for what a component is: the bytes between slashes.
//! Empty components are real here (`"a//b"` has three), exactly as the C
//! walkers see them; only [`decompose`] collapses runs of slashes, because
//! only `decomposePath` does. Nothing validates or converts UTF-8.
//!
//! Positions are byte offsets into the slice the caller hands in, and the
//! "end of path" position is `start == bytes.len()`. The pointer entry points
//! in [`super::c_abi`] translate their `const char *` pairs to and from these
//! offsets, so the arithmetic lives in one place.

/// The only path separator on the supported platforms. `paths.h` also accepts
/// a backslash where `BACKSLASH_IS_PATH_SEPARATOR` is defined (Windows,
/// Symbian), which is not a target of this module.
pub const SEPARATOR: u8 = b'/';

/// One component: `bytes[start..end]`. `start == end == bytes.len()` marks the
/// end of the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// Index of the first separator at or after `from`, or the length.
fn component_end(bytes: &[u8], from: usize) -> usize {
    bytes[from..]
        .iter()
        .position(|&b| b == SEPARATOR)
        .map_or(bytes.len(), |at| from + at)
}

/// `getFirstPathComponent`: the component at the start of `bytes`.
pub fn first_component(bytes: &[u8]) -> Span {
    Span::new(0, component_end(bytes, 0))
}

/// `getNextPathComponent`: the component after `current`. At the end of the
/// path the result is the empty span at the end.
///
/// # Panics
/// `current.end` is past the end of `bytes`, or is not at a separator while
/// inside the path: the C `assert`s of the original.
pub fn next_component(bytes: &[u8], current: Span) -> Span {
    assert!(
        current.end <= bytes.len(),
        "component end past the path end"
    );
    if current.end == bytes.len() {
        return Span::new(current.end, current.end);
    }
    assert_eq!(
        bytes[current.end], SEPARATOR,
        "component end is not at a separator"
    );
    let start = current.end + 1;
    Span::new(start, component_end(bytes, start))
}

/// `getLastPathComponent`: the component that ends at the end of `bytes`.
/// A trailing separator makes the last component empty.
pub fn last_component(bytes: &[u8]) -> Span {
    Span::new(component_start_before(bytes, bytes.len()), bytes.len())
}

/// `getPreviousPathComponent`: the component before the one that starts at
/// `start`; the empty span at the start when there is none.
///
/// # Panics
/// `start` is past the end of `bytes`.
pub fn previous_component(bytes: &[u8], start: usize) -> Span {
    assert!(start <= bytes.len(), "component start past the path end");
    if start == 0 {
        return Span::new(0, 0);
    }
    let end = start - 1;
    Span::new(component_start_before(bytes, end), end)
}

/// The start of the component that ends at `end`: just after the last
/// separator before it.
fn component_start_before(bytes: &[u8], end: usize) -> usize {
    bytes[..end]
        .iter()
        .rposition(|&b| b == SEPARATOR)
        .map_or(0, |at| at + 1)
}

/// Every component of `bytes` in order, as the C loops
/// `getFirstPathComponent` / `getNextPathComponent` while `start < end` see
/// them: empty components included, the empty tail after a trailing separator
/// not.
pub fn components(bytes: &[u8]) -> Components<'_> {
    Components {
        bytes,
        next: Some(first_component(bytes)),
    }
}

pub struct Components<'a> {
    bytes: &'a [u8],
    next: Option<Span>,
}

impl<'a> Iterator for Components<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        let span = self.next.take()?;
        if span.start >= self.bytes.len() {
            return None;
        }
        self.next = Some(next_component(self.bytes, span));
        Some(&self.bytes[span.start..span.end])
    }
}

/// `validPathName`: no component is empty, `.` or `..`. The empty path and a
/// trailing separator are valid; a leading separator is not.
pub fn valid_path_name(bytes: &[u8]) -> bool {
    components(bytes).all(|name| !name.is_empty() && name != b"." && name != b"..")
}

/// `joinPaths`: `first` and `second` with one separator between. Only one
/// trailing separator of `first` is absorbed. An empty `first` gives `second`
/// unchanged. `second` should not start with a separator; if it does the
/// bytes are copied as they are.
pub fn join_paths(first: &[u8], second: &[u8]) -> Vec<u8> {
    if first.is_empty() {
        return second.to_vec();
    }
    let first = first.strip_suffix(&[SEPARATOR]).unwrap_or(first);
    let mut joined = Vec::with_capacity(first.len() + second.len() + 1);
    joined.extend_from_slice(first);
    joined.push(SEPARATOR);
    joined.extend_from_slice(second);
    joined
}

/// `joinPathsAbsolute`: as [`join_paths`], with a leading separator always
/// added (also when `first` already has one).
pub fn join_paths_absolute(first: &[u8], second: &[u8]) -> Vec<u8> {
    let mut joined = Vec::with_capacity(first.len() + second.len() + 2);
    joined.push(SEPARATOR);
    if first.is_empty() {
        joined.extend_from_slice(second);
    } else {
        joined.extend_from_slice(&join_paths(first, second));
    }
    joined
}

/// The empty path, the only path `decomposePath` refuses (`ENOENT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyPath;

/// A path split by [`decompose`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decomposed<'a> {
    pub absolute: bool,
    pub names: Vec<&'a [u8]>,
}

/// `decomposePath` on a POSIX platform: leading separators make the path
/// absolute, and runs of separators count as one. `"/"` is absolute with no
/// names.
pub fn decompose(path: &[u8]) -> Result<Decomposed<'_>, EmptyPath> {
    if path.is_empty() {
        return Err(EmptyPath);
    }
    Ok(Decomposed {
        absolute: path[0] == SEPARATOR,
        names: path
            .split(|&b| b == SEPARATOR)
            .filter(|name| !name.is_empty())
            .collect(),
    })
}

/// `composePath`: the names joined by separators, with a leading one when
/// `absolute`. The result is sized from what is written; the C original
/// leaves the separators out of its length.
///
/// # Panics
/// `names` is empty (the C `assert(pathComp != NULL)`).
pub fn compose(names: &[&[u8]], absolute: bool) -> Vec<u8> {
    assert!(
        !names.is_empty(),
        "composePath needs at least one component"
    );
    let length = usize::from(absolute)
        + names.iter().map(|name| name.len()).sum::<usize>()
        + (names.len() - 1);
    let mut path = Vec::with_capacity(length);
    if absolute {
        path.push(SEPARATOR);
    }
    for (index, name) in names.iter().enumerate() {
        if index > 0 {
            path.push(SEPARATOR);
        }
        path.extend_from_slice(name);
    }
    debug_assert_eq!(path.len(), length);
    path
}
