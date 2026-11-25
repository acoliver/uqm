//! String table loaders: text (`STRTAB`), binary (`BINTAB`) and conversation
//! (`CONVERSATION`) resources.
//!
//! Text files hold `#(NAME) [clip]` header lines, each followed by the lines
//! of that string. A conversation resource path is
//! `phrases[:clip_dir[:timestamps]]`; the optional timestamp file carries one
//! `#(NAME) t1,t2,...` line per header, in the same order.

use std::ffi::CStr;

use super::conversation::ConversationTable;
use super::owned_table::{LoadedBinaryTable, LoadedTextTable};
use crate::host::{FileStream, Files};
use crate::io::Dir;
use crate::log_abi::log_message;
use crate::logging::LogLevel;
use crate::resource::FileError;
use std::io::Read;
use std::path::Path;

const MAX_STRINGS: usize = 2048;
/// `CurrentLine` size in the original loader; longer lines are split.
const LINE_SIZE: usize = 1024;

/// Line-oriented view of a stream with `fgets` semantics.
trait LineSource {
    /// Replaces `line` with the next line (up to `LINE_SIZE - 1` bytes, newline
    /// included, cut at an embedded NUL). Returns false at end of file.
    fn read_line(&mut self, line: &mut Vec<u8>) -> std::io::Result<bool>;
    fn tell(&self) -> i32;
}

struct FileLines<'a>(&'a mut FileStream);

impl LineSource for FileLines<'_> {
    fn read_line(&mut self, line: &mut Vec<u8>) -> std::io::Result<bool> {
        match self.0.line(LINE_SIZE)? {
            Some(text) => {
                *line = text;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn tell(&self) -> i32 {
        self.0.position() as i32
    }
}

impl LineSource for FileStream {
    fn read_line(&mut self, line: &mut Vec<u8>) -> std::io::Result<bool> {
        FileLines(self).read_line(line)
    }
    fn tell(&self) -> i32 {
        self.position() as i32
    }
}

/// Concatenated NUL-terminated strings and the byte length of each.
#[derive(Default)]
struct Pool {
    bytes: Vec<u8>,
    lens: Vec<u32>,
}

impl Pool {
    fn begin_string(&mut self) {
        self.lens.push(0);
    }

    /// Appends text to the current string, joining it to the previous line.
    fn append_line(&mut self, line: &[u8]) {
        let Some(len) = self.lens.last_mut() else {
            panic!("string table text appended before its first header");
        };
        if *len != 0 {
            self.bytes.pop();
            *len -= 1;
        }
        self.bytes.extend_from_slice(line);
        self.bytes.push(0);
        *len += line.len() as u32 + 1;
    }

    /// Drops trailing CR/LF characters of the current string.
    fn trim_last(&mut self) {
        let Some(len) = self.lens.last_mut() else {
            return;
        };
        while *len > 1 && matches!(self.bytes[self.bytes.len() - 2], b'\n' | b'\r') {
            *len -= 1;
            self.bytes.pop();
            // `len > 1` left at least the NUL-replaced byte in the pool.
            let last = self.bytes.len() - 1;
            self.bytes[last] = 0;
        }
    }

    /// Adds a whole string (plus NUL) as the next entry.
    fn push_terminated(&mut self, parts: &[&[u8]]) {
        let start = self.bytes.len();
        for part in parts {
            self.bytes.extend_from_slice(part);
        }
        self.bytes.push(0);
        self.lens.push((self.bytes.len() - start) as u32);
    }

    fn push_empty(&mut self) {
        self.lens.push(0);
    }

    /// The first `count` strings, each with its NUL; an empty string has none.
    fn into_entries(self, count: usize) -> Vec<Vec<u8>> {
        let mut offset = 0;
        self.lens[..count]
            .iter()
            .map(|&len| {
                let end = offset + len as usize;
                let entry = self.bytes[offset..end].to_vec();
                offset = end;
                entry
            })
            .collect()
    }
}

/// Splits `#(NAME) rest` like `strtok(line + 1, "()")`: the name, and the
/// text after the delimiter that ended it. `None` when no name is present.
fn parse_header(line: &[u8]) -> Option<(&[u8], &[u8])> {
    let is_paren = |b: &u8| matches!(b, b'(' | b')');
    let body = &line[1..];
    let body = &body[body.iter().position(|b| !is_paren(b))?..];
    match body.iter().position(is_paren) {
        Some(end) => Some((&body[..end], &body[end + 1..])),
        None => Some((body, &[])),
    }
}

/// First token of `text` split on whitespace and `)`, like
/// `strtok(NULL, " \t\r\n)")`.
fn next_clip_token(text: &[u8]) -> Option<&[u8]> {
    let is_sep = |b: &u8| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b')');
    let text = &text[text.iter().position(|b| !is_sep(b))?..];
    let end = text.iter().position(is_sep).unwrap_or(text.len());
    Some(&text[..end])
}

/// Reads `length` bytes worth of string text into `strings`, calling
/// `on_header` with each header's name and trailing text. Returns the number
/// of strings.
fn read_strings(
    src: &mut impl LineSource,
    length: u32,
    strings: &mut Pool,
    mut on_header: impl FnMut(&[u8], &[u8]),
) -> std::io::Result<usize> {
    let start = src.tell();
    let mut line = Vec::with_capacity(LINE_SIZE);
    while src.read_line(&mut line)? {
        if strings.lens.len() >= MAX_STRINGS {
            break;
        }
        if line.first() == Some(&b'#') {
            if let Some((name, rest)) = parse_header(&line) {
                strings.trim_last();
                strings.begin_string();
                on_header(name, rest);
            }
        } else if !strings.lens.is_empty() {
            strings.append_line(&line);
        }
        if src.tell().wrapping_sub(start) >= length as i32 {
            break;
        }
    }
    strings.trim_last();
    Ok(strings.lens.len())
}

/// Text strings, nothing else.
fn load_plain_table(
    src: &mut impl LineSource,
    length: u32,
) -> std::io::Result<Option<LoadedTextTable>> {
    let mut strings = Pool::default();
    let count = read_strings(src, length, &mut strings, |_, _| {})?;
    Ok((count != 0).then(|| LoadedTextTable::from_strings(strings.into_entries(count))))
}

/// Opens `path` in `dir` with its length.
fn open_with_length(dir: &Dir, path: &[u8]) -> Result<(FileStream, u32), FileError> {
    let name = std::str::from_utf8(path)
        .map_err(|error| FileError::at(String::from_utf8_lossy(path).as_ref(), error))?;
    let file = dir
        .open_read(Path::new(name))
        .map_err(|error| FileError::at(name, error))?;
    let length = file.len().map_err(|error| FileError::at(name, error))? as u32;
    Ok((file, length))
}

fn open_resource(dir: &Dir, path: &[u8]) -> Result<(FileStream, u32), FileError> {
    let (file, length) = open_with_length(dir, path)?;
    if length == 0 {
        return Err(FileError::invalid(
            String::from_utf8_lossy(path).as_ref(),
            "zero-length resource",
        ));
    }
    Ok((file, length))
}

pub(super) fn load_text_table(dir: &Dir, path: &CStr) -> Result<LoadedTextTable, FileError> {
    let (mut file, length) = open_resource(dir, path.to_bytes())?;
    let name = path.to_string_lossy();
    load_plain_table(&mut FileLines(&mut file), length)
        .map_err(|error| FileError::at(name.as_ref(), error))?
        .ok_or_else(|| FileError::invalid(name.as_ref(), "text table has no strings"))
}

/// Name, clip and timestamp data gathered next to the conversation strings.
struct Companions<T: LineSource> {
    names: Pool,
    clips: Pool,
    timestamps: Pool,
    /// Timestamp file; dropped for good at the first malformed line.
    timestamp_src: Option<T>,
    clip_dir: Option<Vec<u8>>,
}

impl<T: LineSource> Companions<T> {
    fn on_header(&mut self, name: &[u8], rest: &[u8]) {
        self.names.push_terminated(&[name]);
        if let Some(src) = self.timestamp_src.as_mut() {
            let mut line = Vec::new();
            let read = src
                .read_line(&mut line)
                .unwrap_or_else(|error| {
                    log::warn!("Disabling timestamps after read failure: {error}");
                    false
                })
                .then_some(line);
            self.record_timestamp(name, read);
        }
        match next_clip_token(rest) {
            Some(token) => {
                let dir = self.clip_dir.as_deref().unwrap_or(&[]);
                self.clips.push_terminated(&[dir, token]);
            }
            None => self.clips.push_empty(),
        }
    }

    /// Record the timestamps of `name` from its line in the timestamp
    /// source; `None` when the line could not be read.
    fn record_timestamp(&mut self, name: &[u8], line: Option<Vec<u8>>) {
        match line.as_deref().and_then(|line| timestamp_value(line, name)) {
            Some([]) => self.timestamps.push_empty(),
            Some(value) => self.timestamps.push_terminated(&[value]),
            None => {
                log_message(
                    LogLevel::Warning,
                    &format!(
                        "Invalid timestamp data for '{}'.  Disabling timestamps",
                        String::from_utf8_lossy(name)
                    ),
                );
                self.timestamp_src = None;
                self.timestamps = Pool::default();
            }
        }
    }
}

/// Timestamp text of a `#(NAME) t1,t2` line, or `None` if the line is not a
/// header for `name`.
fn timestamp_value<'a>(line: &'a [u8], name: &[u8]) -> Option<&'a [u8]> {
    if line.first() != Some(&b'#') {
        return None;
    }
    let at = line.windows(name.len()).position(|w| w == name)?;
    let after = line.get(at + name.len() + 1..).unwrap_or(&[]);
    let skip = after
        .iter()
        .take_while(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .count();
    Some(&after[skip..])
}

/// Builds the conversation table: strings, names (always), then clips and
/// timestamps when any exist. `None` when the file holds no strings.
fn load_conversation_table<T: LineSource>(
    src: &mut impl LineSource,
    length: u32,
    clip_dir: Option<Vec<u8>>,
    timestamp_src: Option<T>,
) -> std::io::Result<Option<ConversationTable>> {
    let mut strings = Pool::default();
    let mut extra = Companions {
        names: Pool::default(),
        clips: Pool::default(),
        timestamps: Pool::default(),
        timestamp_src,
        clip_dir,
    };
    let count = read_strings(src, length, &mut strings, |name, rest| {
        extra.on_header(name, rest)
    })?;
    if count == 0 {
        return Ok(None);
    }
    let column = |pool: Pool| (!pool.bytes.is_empty()).then(|| pool.into_entries(count));
    Ok(Some(ConversationTable {
        strings: LoadedTextTable::from_strings(strings.into_entries(count)),
        names: extra.names.into_entries(count),
        clips: column(extra.clips),
        timestamps: column(extra.timestamps),
    }))
}

/// Loads a `phrases[:clip_dir[:timestamps]]` conversation resource.
pub(super) fn load_conversation(
    content: &Dir,
    path: &CStr,
) -> Result<ConversationTable, FileError> {
    let full = path.to_bytes();
    let paths = &full[..full.len().min(LINE_SIZE - 1)];
    let (phrases, clip_dir, ts_path) = split_conversation_path(paths);
    let (mut file, data_len) = open_with_length(content, phrases)?;
    log_message(
        LogLevel::Info,
        &format!(
            "\t'{}' -- conversation phrases -- {} bytes",
            String::from_utf8_lossy(phrases),
            data_len
        ),
    );
    match clip_dir {
        Some(dir) => log_message(
            LogLevel::Info,
            &format!(
                "\t'{}' -- voice clip directory",
                String::from_utf8_lossy(dir)
            ),
        ),
        None => log_message(LogLevel::Info, "\tNo associated voice clips"),
    }
    match ts_path {
        Some(ts) => log_message(
            LogLevel::Info,
            &format!("\t'{}' -- timestamps", String::from_utf8_lossy(ts)),
        ),
        None => log_message(LogLevel::Info, "\tNo associated timestamp file"),
    }
    if data_len == 0 {
        log_message(
            LogLevel::Warning,
            &format!(
                "Warning: Trying to load empty file '{}'.",
                String::from_utf8_lossy(full)
            ),
        );
        return Err(FileError::invalid(
            path.to_string_lossy().as_ref(),
            "empty conversation",
        ));
    }
    let timestamp_src = ts_path.and_then(|ts| match open_with_length(content, ts) {
        Ok((file, _)) => Some(file),
        Err(error) => {
            log::warn!("No conversation timestamps: {error}");
            None
        }
    });
    load_conversation_table(
        &mut FileLines(&mut file),
        data_len,
        clip_dir.map(<[u8]>::to_vec),
        timestamp_src,
    )
    .map_err(|error| FileError::at(String::from_utf8_lossy(phrases).as_ref(), error))?
    .ok_or_else(|| {
        FileError::invalid(
            String::from_utf8_lossy(phrases).as_ref(),
            "conversation has no strings",
        )
    })
}

/// Splits `phrases[:clip_dir[:timestamps]]`.
fn split_conversation_path(paths: &[u8]) -> (&[u8], Option<&[u8]>, Option<&[u8]>) {
    let Some(first) = paths.iter().position(|&b| b == b':') else {
        return (paths, None, None);
    };
    let rest = &paths[first + 1..];
    match rest.iter().position(|&b| b == b':') {
        Some(second) => (
            &paths[..first],
            Some(&rest[..second]),
            Some(&rest[second + 1..]),
        ),
        None => (&paths[..first], Some(rest), None),
    }
}

/// A `BINTAB` file in the content directory: a `0xFFFFFFFF` marker (an
/// uncompressed resource), then a big-endian binary table: `[count,
/// extra_words, lens[count], ..., string data]`, with the data starting after
/// `extra_words` more words.
pub(super) fn load_binary_table(
    content: &Dir,
    path: &CStr,
) -> Result<LoadedBinaryTable, FileError> {
    let (mut file, length) = open_resource(content, path.to_bytes())?;
    let mut marker = [0; 4];
    file.read_exact(&mut marker)
        .map_err(|error| FileError::at(path.to_string_lossy().as_ref(), error))?;
    if u32::from_ne_bytes(marker) != u32::MAX {
        log::warn!("LZ-compressed binary data not supported");
        return Err(FileError::invalid(
            path.to_string_lossy().as_ref(),
            "LZ-compressed binary data not supported",
        ));
    }
    let data_len = (length as usize).saturating_sub(4);
    if data_len == 0 {
        return Err(FileError::invalid(
            path.to_string_lossy().as_ref(),
            "empty binary table",
        ));
    }
    let mut bytes = vec![0; data_len];
    file.read_exact(&mut bytes)
        .map_err(|error| FileError::at(path.to_string_lossy().as_ref(), error))?;
    Ok(table_from_binary(&bytes))
}

fn table_from_binary(file: &[u8]) -> LoadedBinaryTable {
    let word = |index: usize| {
        let offset = index * 4;
        u32::from_be_bytes([
            file[offset],
            file[offset + 1],
            file[offset + 2],
            file[offset + 3],
        ]) as usize
    };
    let size = word(0);
    let extra_words = word(1);
    let mut data = &file[(2 + size + extra_words) * 4..];
    let entries = (0..size)
        .map(|index| {
            let (entry, rest) = data.split_at(word(2 + index));
            data = rest;
            entry.to_vec()
        })
        .collect();
    LoadedBinaryTable::from_entries(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory stream with `uio_fgets` line splitting.
    struct Memory {
        data: Vec<u8>,
        pos: usize,
    }

    impl Memory {
        fn new(text: &[u8]) -> Self {
            Memory {
                data: text.to_vec(),
                pos: 0,
            }
        }
    }

    impl LineSource for Memory {
        fn read_line(&mut self, line: &mut Vec<u8>) -> std::io::Result<bool> {
            if self.pos >= self.data.len() {
                return Ok(false);
            }
            let rest = &self.data[self.pos..];
            let mut take = rest.len().min(LINE_SIZE - 1);
            if let Some(nl) = rest[..take].iter().position(|&b| b == b'\n') {
                take = nl + 1;
            }
            line.clear();
            line.extend_from_slice(&rest[..take]);
            self.pos += take;
            Ok(true)
        }

        fn tell(&self) -> i32 {
            self.pos as i32
        }
    }

    fn plain(text: &[u8]) -> Option<LoadedTextTable> {
        load_plain_table(&mut Memory::new(text), text.len() as u32).unwrap()
    }

    #[test]
    fn plain_table_joins_lines_and_trims_trailing_newlines() {
        let table = plain(b"junk\n#(A)\nhello\nworld\n\n\n#(B)\nsecond\n#(C)\n").unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(table.text(0), Some(c"hello\nworld"));
        assert_eq!(table.text(1), Some(c"second"));
        assert_eq!(table.text(2), None);
    }

    #[test]
    fn plain_table_without_headers_is_absent() {
        assert!(plain(b"no headers\nat all\n").is_none());
        assert!(plain(b"").is_none());
    }

    #[test]
    fn header_parsing_matches_strtok_rules() {
        assert_eq!(
            parse_header(b"#(NAME) clip.ogg\n"),
            Some((&b"NAME"[..], &b" clip.ogg\n"[..]))
        );
        assert_eq!(parse_header(b"#NAME"), Some((&b"NAME"[..], &b""[..])));
        assert_eq!(parse_header(b"#()"), None);
        assert_eq!(parse_header(b"#"), None);
        assert_eq!(parse_header(b"#\n"), Some((&b"\n"[..], &b""[..])));
        assert_eq!(next_clip_token(b" clip.ogg\n"), Some(&b"clip.ogg"[..]));
        assert_eq!(next_clip_token(b" \n"), None);
        assert_eq!(next_clip_token(b""), None);
    }

    #[test]
    fn conversation_path_splits_on_first_two_colons() {
        assert_eq!(
            split_conversation_path(b"a.txt"),
            (&b"a.txt"[..], None, None)
        );
        assert_eq!(
            split_conversation_path(b"a.txt:clips/"),
            (&b"a.txt"[..], Some(&b"clips/"[..]), None)
        );
        assert_eq!(
            split_conversation_path(b"a.txt:clips/:t.txt"),
            (&b"a.txt"[..], Some(&b"clips/"[..]), Some(&b"t.txt"[..]))
        );
    }

    #[test]
    fn conversation_collects_names_clips_and_timestamps() {
        let text = b"#(HELLO) h.ogg\nHi there\n#(BYE) b.ogg\nGoodbye\n";
        let stamps = b"#(HELLO) 10,20\n#(BYE) 30\n";
        let table = load_conversation_table(
            &mut Memory::new(text),
            text.len() as u32,
            Some(b"voice/".to_vec()),
            Some(Memory::new(stamps)),
        )
        .unwrap()
        .unwrap();
        let first = table.phrase(0).unwrap();
        assert_eq!(first.text, Some(c"Hi there"));
        assert_eq!(first.name, Some(c"HELLO"));
        assert_eq!(first.clip, Some(c"voice/h.ogg"));
        assert_eq!(first.timestamp, Some(c"10,20\n"));
        let second = table.phrase(1).unwrap();
        assert_eq!(second.name, Some(c"BYE"));
        assert_eq!(second.clip, Some(c"voice/b.ogg"));
        assert_eq!(second.timestamp, Some(c"30\n"));
    }

    #[test]
    fn bad_timestamp_line_disables_timestamps() {
        let text = b"#(A) a.ogg\nx\n#(B)\ny\n";
        let table = load_conversation_table(
            &mut Memory::new(text),
            text.len() as u32,
            None,
            Some(Memory::new(b"#(A) 1\nnot a header\n")),
        )
        .unwrap()
        .unwrap();
        assert!(table.timestamps.is_none());
        assert_eq!(table.phrase(0).unwrap().clip, Some(c"a.ogg"));
        assert_eq!(table.phrase(1).unwrap().clip, None);
    }

    #[test]
    fn string_limit_stops_reading() {
        let mut text = Vec::new();
        for i in 0..MAX_STRINGS + 5 {
            text.extend_from_slice(format!("#(S{i})\nv\n").as_bytes());
        }
        assert_eq!(plain(&text).unwrap().len(), MAX_STRINGS);
    }

    #[test]
    fn binary_table_reads_lengths_and_payload() {
        // count=2, extra=1 word, lens 3 and 2, one extra word, then data.
        let mut file = Vec::new();
        for w in [2u32, 1, 3, 2, 0xdead_beef] {
            file.extend_from_slice(&w.to_be_bytes());
        }
        file.extend_from_slice(b"abcde");
        let owner = table_from_binary(&file);
        assert_eq!(owner.entries(), [b"abc".to_vec(), b"de".to_vec()]);
    }

    #[test]
    #[serial_test::serial]
    fn resource_files_load_through_the_content_directory() {
        let mount = crate::io::fixture::TempMount::new();
        mount.write("text.txt", b"#(A)\nalpha\n");
        mount.write("empty.txt", b"");
        let mut binary = u32::MAX.to_ne_bytes().to_vec();
        for word in [1u32, 0, 2] {
            binary.extend_from_slice(&word.to_be_bytes());
        }
        binary.extend_from_slice(b"hi");
        mount.write("table.bin", &binary);
        mount.write("packed.bin", &[0, 0, 0, 1, 0, 0, 0, 0]);
        mount.write("talk.txt", b"#(GREET) hi.ogg\nHello\n");
        let text = load_text_table(&mount.dir(), c"text.txt").unwrap();
        assert_eq!(text.text(0), Some(c"alpha"));
        assert!(load_text_table(&mount.dir(), c"empty.txt").is_err());
        assert!(load_text_table(&mount.dir(), c"missing.txt").is_err());
        assert_eq!(
            load_binary_table(&mount.dir(), c"table.bin")
                .unwrap()
                .entries(),
            [b"hi".to_vec()]
        );
        assert!(load_binary_table(&mount.dir(), c"packed.bin").is_err());
        let talk = load_conversation(&mount.dir(), c"talk.txt:voice/").unwrap();
        let phrase = talk.phrase(0).unwrap();
        assert_eq!(
            (phrase.text, phrase.clip),
            (Some(c"Hello"), Some(c"voice/hi.ogg"))
        );
        assert!(load_conversation(&mount.dir(), c"missing.txt").is_err());
        assert!(load_conversation(&mount.dir(), c"empty.txt").is_err());
    }

    #[test]
    fn binary_table_owns_full_lengths_empty_entries_and_embedded_zeros() {
        let mut file = Vec::new();
        for word in [3u32, 0, 65537, 0, 4] {
            file.extend_from_slice(&word.to_be_bytes());
        }
        file.extend(std::iter::repeat_n(0x7b, 65537));
        file.extend_from_slice(&[1, 2, 0, 4]);
        let owner = table_from_binary(&file);
        file.fill(0);
        assert_eq!(
            owner.entries(),
            [vec![0x7b; 65537], Vec::new(), vec![1, 2, 0, 4]]
        );
        assert_eq!(owner.entry(5), Some([1, 2, 0, 4].as_slice()));
        assert_eq!(owner.entry(1), Some([].as_slice()));
        let empty = table_from_binary(&[0; 8]);
        assert!(empty.entries().is_empty());
        assert!(empty.entry(0).is_none());
    }

    #[test]
    fn binary_table_rejects_truncated_headers_and_entries() {
        for bytes in [
            vec![],
            vec![0; 7],
            vec![0, 0, 0, 1, 0, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0],
        ] {
            assert!(std::panic::catch_unwind(|| table_from_binary(&bytes)).is_err());
        }
    }
}
