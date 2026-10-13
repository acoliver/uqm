//! Behaviour fixtures traced by hand from `sc2/src/libs/resource/stringbank.c`
//! and its two production callers (`setupmenu.c`, `confirm.c`). The C source
//! is not compiled or run; each expectation follows from the C statements
//! named next to it.

use std::ffi::CString;
use std::mem::{offset_of, size_of};

use super::*;

pub(super) fn cstring(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("fixture bytes contain no NUL")
}

pub(super) fn filled(length: usize) -> CString {
    cstring(&vec![b'q'; length])
}

/// Bytes in use per chunk, in chain order.
pub(super) fn chunk_lens(bank: &StringBank) -> Vec<usize> {
    bank.chunks().map(StringBank::used_len).collect()
}

/// (chunk index, byte offset) of a string handed out by `bank`.
pub(super) fn locate(bank: &StringBank, s: &CStr) -> (usize, usize) {
    let at = s.as_ptr() as usize;
    bank.chunks()
        .enumerate()
        .find_map(|(index, chunk)| {
            let base = chunk.data.as_ptr() as usize;
            (base..base + STRBANK_CHUNK_SIZE)
                .contains(&at)
                .then(|| (index, at - base))
        })
        .expect("string lives in the bank")
}

fn delimiter(byte: u8) -> NonZeroU8 {
    NonZeroU8::new(byte).unwrap()
}

fn capacity(slots: usize) -> NonZeroUsize {
    NonZeroUsize::new(slots).unwrap()
}

fn split(bank: &StringBank, text: &[u8], byte: u8, slots: usize) -> Vec<Option<Vec<u8>>> {
    let text = cstring(text);
    bank.split_string(&text, delimiter(byte), capacity(slots))
        .map(|piece| piece.map(|piece| piece.to_bytes().to_vec()))
        .collect()
}

fn pieces(items: &[&[u8]]) -> Vec<Option<Vec<u8>>> {
    items.iter().map(|item| Some(item.to_vec())).collect()
}

#[test]
fn chunk_layout_matches_the_c_struct() {
    assert_eq!(size_of::<StringBank>(), 1024);
    assert_eq!(
        STRBANK_CHUNK_SIZE,
        1024 - size_of::<*const ()>() - size_of::<c_int>()
    );
    assert_eq!(offset_of!(StringBank, data), 0);
    assert_eq!(offset_of!(StringBank, len), STRBANK_CHUNK_SIZE);
    assert_eq!(offset_of!(StringBank, next), 1024 - size_of::<*const ()>());
    #[cfg(target_pointer_width = "64")]
    assert_eq!(STRBANK_CHUNK_SIZE, 1012);
}

#[test]
fn a_new_bank_is_one_empty_chunk() {
    let bank = StringBank::create();
    assert_eq!(chunk_lens(&bank), [0]);
}

#[test]
fn add_string_packs_entries_back_to_back_with_their_terminators() {
    // stringbank.c AddString: result = data + len; len += strlen + 1.
    let bank = StringBank::create();
    let abc = bank.add_string(&cstring(b"abc")).unwrap();
    let empty = bank.add_string(&cstring(b"")).unwrap();
    let x = bank.add_string(&cstring(b"x")).unwrap();
    assert_eq!(abc.to_bytes(), b"abc");
    assert_eq!(empty.to_bytes(), b"");
    assert_eq!(x.to_bytes(), b"x");
    assert_eq!(
        [locate(&bank, abc), locate(&bank, empty), locate(&bank, x)],
        [(0, 0), (0, 4), (0, 5)]
    );
    assert_eq!(chunk_lens(&bank), [7]);
}

#[test]
fn add_string_keeps_duplicates_apart() {
    let bank = StringBank::create();
    let first = bank.add_string(&cstring(b"dup")).unwrap();
    let second = bank.add_string(&cstring(b"dup")).unwrap();
    assert_ne!(first.as_ptr(), second.as_ptr());
    assert_eq!(chunk_lens(&bank), [8]);
}

#[test]
fn a_chunk_takes_a_string_only_while_it_needs_strictly_less_than_the_remaining_bytes() {
    // AddString tests `len < remaining`, so an exact fit moves on.
    let bank = StringBank::create();
    bank.add_string(&filled(1000)).unwrap();
    assert_eq!(chunk_lens(&bank), [1001]);
    // 11 bytes remain: a 10-byte string needs 11, which is not < 11.
    let spilled = bank.add_string(&filled(10)).unwrap();
    assert_eq!(locate(&bank, spilled), (1, 0));
    assert_eq!(chunk_lens(&bank), [1001, 11]);
    // A 9-byte string needs 10 < 11 and goes back into the first chunk.
    let returned = bank.add_string(&filled(9)).unwrap();
    assert_eq!(locate(&bank, returned), (0, 1001));
    assert_eq!(chunk_lens(&bank), [1011, 11]);
    // One byte remains in chunk 0, so even "" (needs 1) is not < 1.
    let empty = bank.add_string(&filled(0)).unwrap();
    assert_eq!(locate(&bank, empty), (1, 11));
}

#[test]
fn a_string_filling_a_whole_chunk_gets_a_fresh_chunk_of_its_own() {
    // Needs 1012 = STRBANK_CHUNK_SIZE; 1012 < 1012 is false even for the
    // empty first chunk, so add_chunk() runs and the first chunk stays empty.
    let bank = StringBank::create();
    let full = bank.add_string(&filled(STRBANK_CHUNK_SIZE - 1)).unwrap();
    assert_eq!(locate(&bank, full), (1, 0));
    assert_eq!(chunk_lens(&bank), [0, STRBANK_CHUNK_SIZE]);
}

#[test]
fn the_longest_string_that_still_fits_beside_its_terminator_uses_the_first_chunk() {
    let bank = StringBank::create();
    let long = bank.add_string(&filled(STRBANK_CHUNK_SIZE - 2)).unwrap();
    assert_eq!(locate(&bank, long), (0, 0));
    assert_eq!(chunk_lens(&bank), [STRBANK_CHUNK_SIZE - 1]);
}

#[test]
fn a_string_too_long_for_any_chunk_is_refused_without_touching_the_bank() {
    let bank = StringBank::create();
    assert!(bank.add_string(&filled(STRBANK_CHUNK_SIZE)).is_none());
    assert!(bank
        .add_or_find_string(&filled(STRBANK_CHUNK_SIZE))
        .is_none());
    assert!(bank.add_string(&filled(STRBANK_CHUNK_SIZE + 500)).is_none());
    assert_eq!(chunk_lens(&bank), [0]);
}

#[test]
fn add_or_find_returns_the_existing_entry() {
    let bank = StringBank::create();
    let first = bank.add_or_find_string(&cstring(b"Graphics")).unwrap();
    let other = bank.add_or_find_string(&cstring(b"Audio")).unwrap();
    let again = bank.add_or_find_string(&cstring(b"Graphics")).unwrap();
    assert_eq!(first.as_ptr(), again.as_ptr());
    assert_ne!(first.as_ptr(), other.as_ptr());
    assert_eq!(chunk_lens(&bank), [9 + 6]);
}

#[test]
fn add_or_find_compares_whole_entries_not_prefixes_or_suffixes() {
    let bank = StringBank::create();
    let abc = bank.add_string(&cstring(b"abc")).unwrap();
    for near in [&b"ab"[..], b"bc", b"abcd", b"xabc", b"b", b"c"] {
        let added = bank.add_or_find_string(&cstring(near)).unwrap();
        assert_ne!(added.as_ptr(), abc.as_ptr(), "{near:?}");
        assert_eq!(added.to_bytes(), near);
    }
}

#[test]
fn add_or_find_matches_the_empty_string() {
    let bank = StringBank::create();
    bank.add_string(&cstring(b"a")).unwrap();
    let empty = bank.add_string(&cstring(b"")).unwrap();
    let found = bank.add_or_find_string(&cstring(b"")).unwrap();
    assert_eq!(found.as_ptr(), empty.as_ptr());
}

#[test]
fn add_or_find_returns_the_first_duplicate_in_chain_order() {
    let bank = StringBank::create();
    let first = bank.add_string(&cstring(b"twin")).unwrap();
    bank.add_string(&cstring(b"twin")).unwrap();
    assert_eq!(
        bank.add_or_find_string(&cstring(b"twin")).unwrap().as_ptr(),
        first.as_ptr()
    );
}

#[test]
fn add_or_find_searches_every_chunk() {
    let bank = StringBank::create();
    bank.add_string(&filled(1000)).unwrap();
    let in_second = bank.add_string(&cstring(b"second chunk")).unwrap();
    assert_eq!(locate(&bank, in_second).0, 1);
    let found = bank.add_or_find_string(&cstring(b"second chunk")).unwrap();
    assert_eq!(found.as_ptr(), in_second.as_ptr());
    assert_eq!(chunk_lens(&bank), [1001, 13]);
}

#[test]
fn a_miss_lands_in_the_first_chunk_with_room() {
    let bank = StringBank::create();
    bank.add_string(&filled(1000)).unwrap();
    bank.add_string(&filled(10)).unwrap();
    let added = bank
        .add_or_find_string(&cstring(b"fits chunk zero"))
        .unwrap();
    assert_eq!(locate(&bank, added), (1, 11));
}

#[test]
fn substring_takes_at_most_the_first_n_bytes() {
    // AddSubstring: n > strlen adds the whole string, otherwise strncpy(n).
    let bank = StringBank::create();
    let word = cstring(b"abcdef");
    let cases: [(usize, &[u8]); 8] = [
        (0, b""),
        (1, b"a"),
        (3, b"abc"),
        (5, b"abcde"),
        (6, b"abcdef"),
        (7, b"abcdef"),
        (usize::from(u16::MAX), b"abcdef"),
        (u32::MAX as usize, b"abcdef"),
    ];
    for (n, expected) in cases {
        assert_eq!(
            bank.add_substring(&word, n).unwrap().to_bytes(),
            expected,
            "{n}"
        );
    }
}

#[test]
fn substring_limits_follow_the_chunk_capacity() {
    let long = filled(STRBANK_CHUNK_SIZE + 400);
    let bank = StringBank::create();
    let widest = bank.add_substring(&long, STRBANK_CHUNK_SIZE - 1).unwrap();
    assert_eq!(widest.to_bytes().len(), STRBANK_CHUNK_SIZE - 1);
    // n >= STRBANK_CHUNK_SIZE returns NULL when n <= strlen.
    assert!(bank.add_substring(&long, STRBANK_CHUNK_SIZE).is_none());
    assert!(bank.add_substring(&long, STRBANK_CHUNK_SIZE + 1).is_none());
    assert!(bank
        .add_or_find_substring(&long, STRBANK_CHUNK_SIZE)
        .is_none());
    // n > strlen falls back to AddString, which refuses a string this long.
    let exactly_full = filled(STRBANK_CHUNK_SIZE);
    assert!(bank.add_substring(&exactly_full, usize::MAX).is_none());
    assert!(bank
        .add_or_find_substring(&exactly_full, usize::MAX)
        .is_none());
}

#[test]
fn substring_never_reads_past_the_string() {
    let bank = StringBank::create();
    let found = bank.add_substring(&cstring(b"ab"), 2).unwrap();
    assert_eq!(found.to_bytes(), b"ab");
}

#[test]
fn add_or_find_substring_reuses_an_entry_equal_to_the_prefix() {
    let bank = StringBank::create();
    let abc = bank.add_string(&cstring(b"abc")).unwrap();
    let found = bank.add_or_find_substring(&cstring(b"abcdef"), 3).unwrap();
    assert_eq!(found.as_ptr(), abc.as_ptr());
    let empty = bank.add_or_find_substring(&cstring(b"zzz"), 0).unwrap();
    assert_eq!(empty.to_bytes(), b"");
    assert_eq!(
        bank.add_or_find_substring(&cstring(b"yyy"), 0)
            .unwrap()
            .as_ptr(),
        empty.as_ptr()
    );
}

#[test]
fn strings_are_bytes_and_come_back_unchanged() {
    let bank = StringBank::create();
    let samples: [&[u8]; 5] = [
        "Für Elise – 日本語 \u{1F680}".as_bytes(),
        b"\xff\xfe\x80 not utf-8",
        b"\xc3\x28 broken pair",
        b"\xe2\x82",
        b"\x01\x7f\r\t",
    ];
    for sample in samples {
        let stored = bank.add_or_find_string(&cstring(sample)).unwrap();
        assert_eq!(stored.to_bytes(), sample);
        let again = bank.add_or_find_string(&cstring(sample)).unwrap();
        assert_eq!(again.as_ptr(), stored.as_ptr());
    }
}

#[test]
fn a_substring_may_cut_inside_a_multibyte_character() {
    let bank = StringBank::create();
    let cut = bank.add_substring(&cstring("é".as_bytes()), 1).unwrap();
    assert_eq!(cut.to_bytes(), [0xC3]);
}

#[test]
fn handed_out_strings_stay_put_and_intact_while_the_bank_grows() {
    let bank = StringBank::create();
    let mut kept = Vec::new();
    for index in 0..300usize {
        let mut raw = format!("entry-{index:04}-").into_bytes();
        raw.resize(450 + index % 7, b'a' + (index % 26) as u8);
        let stored = bank.add_string(&cstring(&raw)).unwrap();
        kept.push((stored.as_ptr(), raw));
    }
    assert!(chunk_lens(&bank).len() > 100);
    for (pointer, raw) in &kept {
        let stored = unsafe { CStr::from_ptr(*pointer) };
        assert_eq!(stored.to_bytes(), raw.as_slice());
    }
    for (pointer, raw) in &kept {
        let found = bank.add_or_find_string(&cstring(raw)).unwrap();
        assert_eq!(found.as_ptr(), *pointer);
    }
}

#[test]
fn dropping_a_very_long_chain_does_not_recurse() {
    let mut head = StringBank::create();
    for _ in 0..60_000 {
        let previous_head = StringBank::create();
        assert!(previous_head.next.set(head).is_ok());
        head = previous_head;
    }
    drop(head);
}

#[test]
fn split_breaks_on_every_delimiter_when_capacity_allows() {
    let bank = StringBank::create();
    assert_eq!(
        split(&bank, b"Graphics\nAudio\nControls", b'\n', 100),
        pieces(&[b"Graphics", b"Audio", b"Controls"])
    );
}

#[test]
fn split_stops_at_capacity_and_keeps_the_remainder_in_the_last_piece() {
    // SplitString loops i < n-1 and files the rest of the string last.
    let bank = StringBank::create();
    assert_eq!(
        split(&bank, b"a\nb\nc\nd", b'\n', 2),
        pieces(&[b"a", b"b\nc\nd"])
    );
    assert_eq!(
        split(&bank, b"a\nb\nc\nd", b'\n', 3),
        pieces(&[b"a", b"b", b"c\nd"])
    );
    assert_eq!(
        split(&bank, b"a\nb\nc\nd", b'\n', 4),
        pieces(&[b"a", b"b", b"c", b"d"])
    );
    assert_eq!(
        split(&bank, b"a\nb\nc\nd", b'\n', 5),
        pieces(&[b"a", b"b", b"c", b"d"])
    );
}

#[test]
fn split_with_one_slot_files_the_whole_string() {
    let bank = StringBank::create();
    assert_eq!(split(&bank, b"a\nb", b'\n', 1), pieces(&[b"a\nb"]));
}

#[test]
fn split_without_a_delimiter_returns_the_string_as_one_piece() {
    let bank = StringBank::create();
    assert_eq!(
        split(&bank, b"one line", b'\n', 100),
        pieces(&[b"one line"])
    );
}

#[test]
fn split_of_the_empty_string_is_one_empty_piece() {
    let bank = StringBank::create();
    assert_eq!(split(&bank, b"", b'\n', 100), pieces(&[b""]));
}

#[test]
fn split_keeps_empty_pieces_around_leading_trailing_and_doubled_delimiters() {
    let bank = StringBank::create();
    assert_eq!(split(&bank, b"\n", b'\n', 100), pieces(&[b"", b""]));
    assert_eq!(split(&bank, b"a\n", b'\n', 100), pieces(&[b"a", b""]));
    assert_eq!(split(&bank, b"\na", b'\n', 100), pieces(&[b"", b"a"]));
    assert_eq!(
        split(&bank, b"a\n\nb", b'\n', 100),
        pieces(&[b"a", b"", b"b"])
    );
}

#[test]
fn split_files_equal_pieces_once() {
    let bank = StringBank::create();
    let text = cstring(b"x\ny\nx\nx");
    let found: Vec<_> = bank
        .split_string(&text, delimiter(b'\n'), capacity(30))
        .map(|piece| piece.unwrap().as_ptr())
        .collect();
    assert_eq!(found[0], found[2]);
    assert_eq!(found[0], found[3]);
    assert_ne!(found[0], found[1]);
    assert_eq!(chunk_lens(&bank), [4]);
}

#[test]
fn split_reuses_entries_added_earlier_in_the_same_bank() {
    // setupmenu.c files the title with AddOrFindString, then splits tables
    // into the same bank; a repeated label must resolve to one pointer.
    let bank = StringBank::create();
    let title = bank.add_or_find_string(&cstring(b"Setup")).unwrap();
    let text = cstring(b"Setup\nQuit");
    let found: Vec<_> = bank
        .split_string(&text, delimiter(b'\n'), capacity(100))
        .map(|piece| piece.unwrap())
        .collect();
    assert_eq!(found[0].as_ptr(), title.as_ptr());
    assert_eq!(found[1].to_bytes(), b"Quit");
}

#[test]
fn split_marks_a_piece_too_long_for_a_chunk_as_missing_and_keeps_going() {
    let bank = StringBank::create();
    let mut raw = vec![b'z'; STRBANK_CHUNK_SIZE];
    raw.extend_from_slice(b"\nshort\n");
    raw.extend(vec![b'z'; STRBANK_CHUNK_SIZE - 1]);
    raw.push(b'\n');
    let result = split(&bank, &raw, b'\n', 100);
    assert_eq!(result.len(), 4);
    assert_eq!(result[0], None);
    assert_eq!(result[1], Some(b"short".to_vec()));
    assert_eq!(result[2], Some(vec![b'z'; STRBANK_CHUNK_SIZE - 1]));
    assert_eq!(result[3], Some(b"".to_vec()));
}

#[test]
fn split_remainder_that_is_too_long_is_reported_missing() {
    let bank = StringBank::create();
    let mut raw = b"a\n".to_vec();
    raw.extend(vec![b'z'; STRBANK_CHUNK_SIZE]);
    assert_eq!(
        split(&bank, &raw, b'\n', 2),
        vec![Some(b"a".to_vec()), None]
    );
}

#[test]
fn split_works_on_any_delimiter_byte_including_a_high_one() {
    let bank = StringBank::create();
    assert_eq!(split(&bank, b"a,b;c", b',', 100), pieces(&[b"a", b"b;c"]));
    assert_eq!(
        split(&bank, "caf\u{e9}s".as_bytes(), 0xC3, 100),
        pieces(&[b"caf", b"\xa9s"])
    );
}

#[test]
fn split_ends_after_its_last_piece() {
    let bank = StringBank::create();
    let text = cstring(b"a\nb");
    let mut it = bank.split_string(&text, delimiter(b'\n'), capacity(100));
    assert!(it.next().is_some());
    assert!(it.next().is_some());
    assert!(it.next().is_none());
    assert!(it.next().is_none());
}

#[test]
fn split_pieces_outlive_the_source_text() {
    let bank = StringBank::create();
    let kept: Vec<&CStr> = {
        let text = cstring(b"left\nright");
        bank.split_string(&text, delimiter(b'\n'), capacity(2))
            .map(Option::unwrap)
            .collect()
    };
    assert_eq!(kept[0].to_bytes(), b"left");
    assert_eq!(kept[1].to_bytes(), b"right");
}
