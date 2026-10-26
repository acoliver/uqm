//! String table loaders: text (`STRTAB`), binary (`BINTAB`) and conversation
//! (`CONVERSATION`) resources.
//!
//! Text files hold `#(NAME) [clip]` header lines, each followed by the lines
//! of that string. A conversation resource path is
//! `phrases[:clip_dir[:timestamps]]`; the optional timestamp file carries one
//! `#(NAME) t1,t2,...` line per header, in the same order.

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr::null_mut;

use crate::collections::hash_table::{string_add, string_new};
use crate::io::uio_bridge::{uio_Stream, uio_fclose, uio_fgets, uio_fopen, uio_ftell};
use crate::log_abi::{log_message, LOG_WARNING};
use crate::memory::{rust_hfree, rust_hmalloc};
use crate::resource::ffi_bridge::{
    res_CloseResFile, res_OpenResFile, GetResourceData, LengthResFile,
};
use crate::resource::ffi_types::ResourceData;
use crate::strings::string_table::{
    AllocStringTable, StringTable, StringTableEntry, HAS_NAMEINDEX, HAS_SOUND_CLIPS, HAS_TIMESTAMP,
};

#[cfg(not(test))]
use crate::mainloop::options_abi::contentDir;
#[cfg(test)]
#[allow(non_upper_case_globals)]
static mut contentDir: *mut c_void = null_mut();

const MAX_STRINGS: usize = 2048;
const LOG_INFO: i32 = 4;
/// `CurrentLine` size in the original loader; longer lines are split.
const LINE_SIZE: usize = 1024;

/// Line-oriented view of a stream with `fgets` semantics.
trait LineSource {
    /// Replaces `line` with the next line (up to `LINE_SIZE - 1` bytes, newline
    /// included, cut at an embedded NUL). Returns false at end of file.
    fn read_line(&mut self, line: &mut Vec<u8>) -> bool;
    fn tell(&self) -> i32;
}

struct UioLines(*mut uio_Stream);

impl LineSource for UioLines {
    fn read_line(&mut self, line: &mut Vec<u8>) -> bool {
        let mut buf = [0 as c_char; LINE_SIZE];
        // SAFETY: self.0 is an open stream and buf is LINE_SIZE bytes.
        let got = unsafe { uio_fgets(buf.as_mut_ptr(), LINE_SIZE as i32, self.0) };
        if got.is_null() {
            return false;
        }
        // SAFETY: uio_fgets NUL-terminates buf on success.
        let text = unsafe { CStr::from_ptr(buf.as_ptr()) };
        line.clear();
        line.extend_from_slice(text.to_bytes());
        true
    }

    fn tell(&self) -> i32 {
        // SAFETY: self.0 is an open stream. Offsets are compared as C ints.
        unsafe { uio_ftell(self.0) as i32 }
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
        let len = self.lens.last_mut().expect("append before first header");
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
            *self.bytes.last_mut().expect("pool not empty") = 0;
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
) -> usize {
    let start = src.tell();
    let mut line = Vec::with_capacity(LINE_SIZE);
    while src.read_line(&mut line) {
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
    strings.lens.len()
}

/// Copies `data` into a fresh game-allocator buffer; empty data stays NULL.
unsafe fn set_entry(entry: *mut StringTableEntry, data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let buf = rust_hmalloc(data.len()).cast::<u8>();
    std::ptr::copy_nonoverlapping(data.as_ptr(), buf, data.len());
    (*entry).data = buf.cast();
    (*entry).length = data.len() as i32;
}

/// Fills `count` entries starting at `first` from the pool.
unsafe fn copy_pool(table: *mut StringTable, first: usize, count: usize, pool: &Pool) {
    let mut offset = 0;
    for (i, &len) in pool.lens[..count].iter().enumerate() {
        let len = len as usize;
        set_entry(
            (*table).strings.add(first + i),
            &pool.bytes[offset..offset + len],
        );
        offset += len;
    }
}

/// Text strings, nothing else.
unsafe fn load_plain_table(src: &mut impl LineSource, length: u32) -> *mut StringTable {
    let mut strings = Pool::default();
    let count = read_strings(src, length, &mut strings, |_, _| {});
    if count == 0 {
        return null_mut();
    }
    let table = AllocStringTable(count as i32, 0);
    copy_pool(table, 0, count, &strings);
    table
}

/// # Safety
/// `fp` must be an open UIO stream.
pub unsafe extern "C" fn _get_string_data(fp: *mut c_void, length: u32) -> *mut c_void {
    load_plain_table(&mut UioLines(fp.cast()), length).cast()
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
        if self.timestamp_src.is_some() {
            self.read_timestamp(name);
        }
        match next_clip_token(rest) {
            Some(token) => {
                let dir = self.clip_dir.as_deref().unwrap_or(&[]);
                self.clips.push_terminated(&[dir, token]);
            }
            None => self.clips.push_empty(),
        }
    }

    fn read_timestamp(&mut self, name: &[u8]) {
        let mut line = Vec::new();
        let src = self.timestamp_src.as_mut().expect("timestamp source");
        let read_ok = src.read_line(&mut line);
        match read_ok.then(|| timestamp_value(&line, name)).flatten() {
            Some([]) => self.timestamps.push_empty(),
            Some(value) => self.timestamps.push_terminated(&[value]),
            None => {
                log_message(
                    LOG_WARNING,
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
/// timestamps when any exist. Returns NULL when the file holds no strings.
unsafe fn load_conversation_table<T: LineSource>(
    src: &mut impl LineSource,
    length: u32,
    clip_dir: Option<Vec<u8>>,
    timestamp_src: Option<T>,
) -> *mut StringTable {
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
    });
    if count == 0 {
        return null_mut();
    }

    let has_clips = !extra.clips.bytes.is_empty();
    let has_timestamps = !extra.timestamps.bytes.is_empty();
    let mut flags = HAS_NAMEINDEX;
    if has_clips {
        flags |= HAS_SOUND_CLIPS;
    }
    if has_timestamps {
        flags |= HAS_TIMESTAMP;
    }

    let table = AllocStringTable(count as i32, i32::from(flags));
    copy_pool(table, 0, count, &strings);
    copy_pool(table, count, count, &extra.names);
    let mut first = 2 * count;
    if has_clips {
        copy_pool(table, first, count, &extra.clips);
        first += count;
    }
    if has_timestamps {
        copy_pool(table, first, count, &extra.timestamps);
    }

    let index = string_new(0, 0.85, 0.9);
    let names = (*table).strings.add(count);
    for i in 0..count {
        let entry = names.add(i);
        string_add(index, (*entry).data, entry.cast());
    }
    (*table).name_index = index.cast();
    table
}

/// Loads a `phrases[:clip_dir[:timestamps]]` conversation resource into
/// `resdata->ptr` (NULL on failure).
///
/// # Safety
/// `path` must be NUL-terminated and `resdata` writable.
pub unsafe extern "C" fn _get_conversation_data(path: *const c_char, resdata: *mut ResourceData) {
    let full = CStr::from_ptr(path).to_bytes();
    let paths = &full[..full.len().min(LINE_SIZE - 1)];
    let (phrases, clip_dir, ts_path) = split_conversation_path(paths);

    let phrases_c = CString::new(phrases).expect("path has no NUL");
    let fp = res_OpenResFile(contentDir.cast(), phrases_c.as_ptr(), c"rb".as_ptr());
    if fp.is_null() {
        log_message(
            LOG_WARNING,
            &format!("Warning: Can't open '{}'", String::from_utf8_lossy(phrases)),
        );
        (*resdata).ptr = null_mut();
        return;
    }

    let data_len = LengthResFile(fp) as u32;
    log_message(
        LOG_INFO,
        &format!(
            "\t'{}' -- conversation phrases -- {} bytes",
            String::from_utf8_lossy(phrases),
            data_len
        ),
    );
    match clip_dir {
        Some(dir) => log_message(
            LOG_INFO,
            &format!(
                "\t'{}' -- voice clip directory",
                String::from_utf8_lossy(dir)
            ),
        ),
        None => log_message(LOG_INFO, "\tNo associated voice clips"),
    }
    match ts_path {
        Some(ts) => log_message(
            LOG_INFO,
            &format!("\t'{}' -- timestamps", String::from_utf8_lossy(ts)),
        ),
        None => log_message(LOG_INFO, "\tNo associated timestamp file"),
    }

    if data_len == 0 {
        log_message(
            LOG_WARNING,
            &format!(
                "Warning: Trying to load empty file '{}'.",
                String::from_utf8_lossy(full)
            ),
        );
        res_CloseResFile(fp);
        (*resdata).ptr = null_mut();
        return;
    }

    let timestamp_fp = ts_path.map(|ts| {
        let ts_c = CString::new(ts).expect("path has no NUL");
        uio_fopen(contentDir.cast(), ts_c.as_ptr(), c"rb".as_ptr())
    });
    let timestamp_src = timestamp_fp.filter(|p| !p.is_null()).map(OwnedUio);
    let clip_dir = clip_dir.map(<[u8]>::to_vec);

    let table =
        load_conversation_table(&mut UioLines(fp.cast()), data_len, clip_dir, timestamp_src);
    res_CloseResFile(fp);
    (*resdata).ptr = table.cast();
}

/// Timestamp stream that closes itself when dropped (including when the
/// loader disables timestamps midway).
struct OwnedUio(*mut uio_Stream);

impl LineSource for OwnedUio {
    fn read_line(&mut self, line: &mut Vec<u8>) -> bool {
        UioLines(self.0).read_line(line)
    }

    fn tell(&self) -> i32 {
        UioLines(self.0).tell()
    }
}

impl Drop for OwnedUio {
    fn drop(&mut self) {
        // SAFETY: the stream was opened by uio_fopen and is closed once.
        unsafe { uio_fclose(self.0) };
    }
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

/// Big-endian binary table: `[count, extra_words, lens[count], ...,
/// string data]`, with the data starting after `extra_words` more words.
///
/// # Safety
/// `fp` must be an open UIO stream.
pub unsafe extern "C" fn _get_binary_table_data(fp: *mut c_void, length: u32) -> *mut c_void {
    let file = GetResourceData(fp, length);
    if file.is_null() {
        return null_mut();
    }
    let table = table_from_binary(file.cast());
    rust_hfree(file);
    table.cast()
}

/// Decodes the in-memory image of a binary table file.
///
/// # Safety
/// `file` must hold a well-formed table image.
unsafe fn table_from_binary(file: *const u8) -> *mut StringTable {
    let words = file.cast::<u32>();
    let word = |i: usize| u32::from_be(words.add(i).read_unaligned());

    let table = AllocStringTable(word(0) as i32, 0);
    let size = (*table).size as usize;
    let extra_words = word(1) as usize;
    let mut data = words.add(2 + size + extra_words).cast::<u8>();
    for i in 0..size {
        let len = word(2 + i) as usize;
        set_entry(
            (*table).strings.add(i),
            std::slice::from_raw_parts(data, len),
        );
        data = data.add(len);
    }
    table
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
        fn read_line(&mut self, line: &mut Vec<u8>) -> bool {
            if self.pos >= self.data.len() {
                return false;
            }
            let rest = &self.data[self.pos..];
            let mut take = rest.len().min(LINE_SIZE - 1);
            if let Some(nl) = rest[..take].iter().position(|&b| b == b'\n') {
                take = nl + 1;
            }
            line.clear();
            line.extend_from_slice(&rest[..take]);
            self.pos += take;
            true
        }

        fn tell(&self) -> i32 {
            self.pos as i32
        }
    }

    unsafe fn entry_text(entry: *mut StringTableEntry) -> Option<Vec<u8>> {
        if (*entry).data.is_null() {
            return None;
        }
        Some(
            std::slice::from_raw_parts((*entry).data.cast::<u8>(), (*entry).length as usize)
                .to_vec(),
        )
    }

    unsafe fn plain(text: &[u8]) -> *mut StringTable {
        load_plain_table(&mut Memory::new(text), text.len() as u32)
    }

    #[test]
    fn plain_table_joins_lines_and_trims_trailing_newlines() {
        unsafe {
            let table = plain(b"junk\n#(A)\nhello\nworld\n\n\n#(B)\nsecond\n#(C)\n");
            assert_eq!((*table).size, 3);
            let a = entry_text((*table).strings).unwrap();
            assert_eq!(a, b"hello\nworld\0");
            assert_eq!(entry_text((*table).strings.add(1)).unwrap(), b"second\0");
            assert_eq!(entry_text((*table).strings.add(2)), None);
        }
    }

    #[test]
    fn plain_table_without_headers_is_null() {
        unsafe {
            assert!(plain(b"no headers\nat all\n").is_null());
            assert!(plain(b"").is_null());
        }
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
        unsafe {
            let text = b"#(HELLO) h.ogg\nHi there\n#(BYE) b.ogg\nGoodbye\n";
            let stamps = b"#(HELLO) 10,20\n#(BYE) 30\n";
            let table = load_conversation_table(
                &mut Memory::new(text),
                text.len() as u32,
                Some(b"voice/".to_vec()),
                Some(Memory::new(stamps)),
            );
            let flags = (*table).flags;
            assert_eq!(flags, HAS_NAMEINDEX | HAS_SOUND_CLIPS | HAS_TIMESTAMP);
            let s = |slot: usize| entry_text((*table).strings.add(slot)).unwrap();
            assert_eq!(s(0), b"Hi there\0");
            assert_eq!(s(2), b"HELLO\0");
            assert_eq!(s(3), b"BYE\0");
            assert_eq!(s(4), b"voice/h.ogg\0");
            assert_eq!(s(5), b"voice/b.ogg\0");
            assert_eq!(s(6), b"10,20\n\0");
            assert_eq!(s(7), b"30\n\0");
            assert!(!(*table).name_index.is_null());
        }
    }

    #[test]
    fn bad_timestamp_line_disables_timestamps() {
        unsafe {
            let text = b"#(A) a.ogg\nx\n#(B)\ny\n";
            let table = load_conversation_table(
                &mut Memory::new(text),
                text.len() as u32,
                None,
                Some(Memory::new(b"#(A) 1\nnot a header\n")),
            );
            assert_eq!((*table).flags, HAS_NAMEINDEX | HAS_SOUND_CLIPS);
            assert_eq!(entry_text((*table).strings.add(4)).unwrap(), b"a.ogg\0");
            assert_eq!(entry_text((*table).strings.add(5)), None);
        }
    }

    #[test]
    fn string_limit_stops_reading() {
        let mut text = Vec::new();
        for i in 0..MAX_STRINGS + 5 {
            text.extend_from_slice(format!("#(S{i})\nv\n").as_bytes());
        }
        unsafe {
            let table = plain(&text);
            assert_eq!((*table).size as usize, MAX_STRINGS);
        }
    }

    #[test]
    fn binary_table_reads_lengths_and_payload() {
        // count=2, extra=1 word, lens 3 and 2, one extra word, then data.
        let mut file = Vec::new();
        for w in [2u32, 1, 3, 2, 0xdead_beef] {
            file.extend_from_slice(&w.to_be_bytes());
        }
        file.extend_from_slice(b"abcde");
        unsafe {
            let table = table_from_binary(file.as_ptr());
            assert_eq!(entry_text((*table).strings).unwrap(), b"abc");
            assert_eq!(entry_text((*table).strings.add(1)).unwrap(), b"de");
        }
    }
}
