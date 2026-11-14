//! Byte and line operations formerly exercised through the UIO stream API.
use super::Vfs;
use crate::host::{FileStream, Files};
use std::io::{Read, Seek, SeekFrom, Write};

fn fixture(bytes: &[u8]) -> (tempfile::TempDir, Vfs, std::path::PathBuf) {
    let root = concat!(env!("UQM_RUST_ROOT"), "/../tmp");
    std::fs::create_dir_all(root).unwrap();
    let directory = tempfile::tempdir_in(root).unwrap();
    let path = directory.path().join("data");
    std::fs::write(&path, bytes).unwrap();
    (directory, Vfs::default(), path)
}

fn lines(input: &mut FileStream, capacity: usize) -> Vec<Vec<u8>> {
    let mut result = Vec::new();
    while let Some(line) = input.line(capacity).unwrap() {
        result.push(line);
    }
    result
}

#[test]
fn line_splitting_keeps_newlines_crlf_and_unterminated_tail() {
    let (_directory, files, path) = fixture(b"alpha\nbeta\r\nlast");
    let mut input = files.open_read(&path).unwrap();
    assert_eq!(
        lines(&mut input, 64),
        [b"alpha\n".to_vec(), b"beta\r\n".to_vec(), b"last".to_vec()]
    );
    assert_eq!(input.line(64).unwrap(), None);
}

#[test]
fn a_full_line_leaves_room_for_the_terminator_and_one_byte_capacity_consumes_nothing() {
    let (_directory, files, path) = fixture(b"abcdefghij\n");
    let mut input = files.open_read(&path).unwrap();
    assert_eq!(input.line(1).unwrap(), None);
    assert_eq!(input.position(), 0);
    assert_eq!(input.line(4).unwrap(), Some(b"abc".to_vec()));
    assert_eq!(input.line(4).unwrap(), Some(b"def".to_vec()));
    assert_eq!(input.line(100).unwrap(), Some(b"ghij\n".to_vec()));
}

#[test]
fn embedded_nul_truncates_text_but_the_complete_line_is_consumed() {
    let (_directory, files, path) = fixture(b"one\0hidden\ntwo\n");
    let mut input = files.open_read(&path).unwrap();
    assert_eq!(input.line(64).unwrap(), Some(b"one".to_vec()));
    assert_eq!(input.position(), 11);
    assert_eq!(input.line(64).unwrap(), Some(b"two\n".to_vec()));
}

#[test]
fn binary_reads_preserve_all_byte_values_and_report_eof() {
    let (_directory, files, path) = fixture(&[0x41, 0xff, 0, 0x7f]);
    let mut input = files.open_read(&path).unwrap();
    for expected in [0x41, 0xff, 0, 0x7f] {
        let mut byte = [0];
        assert_eq!(input.read(&mut byte).unwrap(), 1);
        assert_eq!(byte[0], expected);
    }
    assert_eq!(input.read(&mut [0]).unwrap(), 0);
    assert_eq!(input.read(&mut []).unwrap(), 0);
}

#[test]
fn seeking_can_reread_a_byte_after_eof() {
    let (_directory, files, path) = fixture(b"QRS");
    let mut input = files.open_read(&path).unwrap();
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"QRS");
    input.seek(SeekFrom::Current(-1)).unwrap();
    assert_eq!(input.line(8).unwrap(), Some(b"S".to_vec()));
    input.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(input.line(8).unwrap(), Some(b"QRS".to_vec()));
}

#[test]
fn character_and_block_writes_land_in_order_and_empty_writes_do_nothing() {
    let (_directory, files, path) = fixture(b"old;");
    let mut output = files.open_write(&path).unwrap();
    assert_eq!(output.len().unwrap(), 0);
    assert_eq!(output.write(b"").unwrap(), 0);
    output.write_all(b"H").unwrap();
    output.write_all(b"ello, ").unwrap();
    output.write_all(b"tail").unwrap();
    output.finish().unwrap();
    assert_eq!(files.read(&path).unwrap(), b"Hello, tail");
}

#[test]
fn read_only_streams_reject_writes_without_changing_the_file() {
    let (_directory, files, path) = fixture(b"data");
    let mut input = files.open_read(&path).unwrap();
    assert!(input.write(b"x").is_err());
    assert_eq!(files.read(&path).unwrap(), b"data");
}

#[test]
fn short_reads_and_all_seek_origins_preserve_the_position() {
    let (_directory, files, path) = fixture(b"0123456789");
    let mut input = files.open_read(&path).unwrap();
    let mut buf = [0; 8];
    assert_eq!(input.read(&mut buf).unwrap(), 8);
    assert_eq!(&buf, b"01234567");
    assert_eq!(input.read(&mut buf).unwrap(), 2);
    assert_eq!(&buf[..2], b"89");
    assert_eq!(input.read(&mut buf).unwrap(), 0);
    assert_eq!(input.seek(SeekFrom::Start(2)).unwrap(), 2);
    assert_eq!(input.seek(SeekFrom::Current(3)).unwrap(), 5);
    assert_eq!(input.seek(SeekFrom::End(-2)).unwrap(), 8);
    assert_eq!(input.len().unwrap(), 10);
    assert_eq!(input.position(), 8);
    assert!(input.seek(SeekFrom::Current(-9)).is_err());
    assert_eq!(input.position(), 8);
}

#[test]
fn windows_use_seek_and_take_and_leave_unread_buffer_bytes_untouched() {
    let (_directory, files, path) = fixture(b"0123456789");
    let mut input = files.open_read(&path).unwrap();
    for (offset, length, expected) in [
        (0, 10, &b"0123456789"[..]),
        (3, 4, &b"3456"[..]),
        (8, 10, &b"89"[..]),
        (10, 4, &b""[..]),
        (50, 4, &b""[..]),
    ] {
        input.seek(SeekFrom::Start(offset)).unwrap();
        let mut bytes = Vec::new();
        Read::by_ref(&mut input)
            .take(length)
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, expected);
    }
    input.seek(SeekFrom::Start(8)).unwrap();
    let mut buffer = [b'.'; 6];
    assert_eq!(input.read(&mut buffer).unwrap(), 2);
    assert_eq!(&buffer, b"89....");
    input.seek(SeekFrom::Start(3)).unwrap();
    let mut sub_block = Vec::new();
    input.take(5).read_to_end(&mut sub_block).unwrap();
    assert_eq!(sub_block, b"34567");
    assert_eq!(&sub_block[2..], b"567");
    assert_eq!(&sub_block[1..3], b"45");
}
