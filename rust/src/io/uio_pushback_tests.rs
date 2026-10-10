//! One byte of `uio_ungetc` pushback and how the other stream calls treat the
//! parked byte, checked against real files. The reference for every
//! expectation is stdio: the next read returns the parked byte first, the
//! stream position is one byte behind the file position while it is parked,
//! and a seek or a write drops it.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

use super::*;

fn fread_bytes(stream: *mut uio_Stream, len: usize) -> (Vec<u8>, usize) {
    let mut buf = vec![0u8; len];
    let items = unsafe { uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 1, len, stream) };
    buf.truncate(items);
    (buf, items)
}

#[test]
#[serial]
fn a_second_pushback_before_the_first_is_read_is_refused() {
    let dir = Dir::new(&[("t.txt", b"abc")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_ungetc(b'X' as c_int, stream), b'X' as c_int);
        assert_eq!(uio_ungetc(b'Y' as c_int, stream), -1);
        assert_eq!(uio_fgetc(stream), b'X' as c_int, "the first byte survives");
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        assert_eq!(
            uio_ungetc(b'Y' as c_int, stream),
            b'Y' as c_int,
            "the slot is free again once the byte was read"
        );
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn every_byte_value_survives_a_pushback_round_trip() {
    let dir = Dir::new(&[("t.bin", b"zz")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        for value in 0..=255u8 {
            assert_eq!(uio_ungetc(c_int::from(value), stream), c_int::from(value));
            assert_eq!(uio_fgetc(stream), c_int::from(value), "byte {value:#04x}");
        }
        assert_eq!(
            uio_fgetc(stream),
            b'z' as c_int,
            "the file was never touched"
        );
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn ungetc_converts_its_argument_to_an_unsigned_char() {
    let dir = Dir::new(&[("t.bin", b"zz")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_ungetc(0x141, stream), 0x41);
        assert_eq!(uio_fgetc(stream), 0x41);
        assert_eq!(uio_ungetc(-1, stream), -1, "only EOF itself is refused");
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fread_hands_out_the_parked_byte_before_the_file() {
    let dir = Dir::new(&[("t.bin", b"abcdef")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        let (bytes, items) = fread_bytes(stream, 5);
        assert_eq!((bytes.as_slice(), items), (&b"Zbcde"[..], 5));
        assert_eq!(uio_ftell(stream), 5);
        assert_eq!(status(stream), UIO_STREAM_STATUS_OK);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fread_of_only_the_parked_byte_does_not_touch_the_file() {
    let dir = Dir::new(&[("t.bin", b"abc")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        let (bytes, _) = fread_bytes(stream, 1);
        assert_eq!(bytes, b"Z");
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn a_short_fread_that_used_the_parked_byte_still_flags_the_end_of_the_file() {
    let dir = Dir::new(&[("t.bin", b"ab")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        assert_eq!(uio_fgetc(stream), b'b' as c_int);
        assert_eq!(uio_ungetc(b'b' as c_int, stream), b'b' as c_int);
        let (bytes, _) = fread_bytes(stream, 4);
        assert_eq!(bytes, b"b");
        assert_eq!(uio_feof(stream), 1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fgets_starts_the_line_with_the_parked_byte() {
    let dir = Dir::new(&[("t.txt", b"abc\ndef\n")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        assert_eq!(uio_ungetc(b'X' as c_int, stream), b'X' as c_int);
        assert_eq!(gets(stream, 16).as_deref(), Some("Xbc\n"));

        assert_eq!(uio_ungetc(b'\n' as c_int, stream), b'\n' as c_int);
        assert_eq!(
            gets(stream, 16).as_deref(),
            Some("\n"),
            "a parked newline ends the line by itself"
        );
        assert_eq!(gets(stream, 16).as_deref(), Some("def\n"));
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn a_parked_byte_is_not_lost_to_a_one_byte_fgets_buffer() {
    let dir = Dir::new(&[("t.txt", b"abc")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_ungetc(b'X' as c_int, stream), b'X' as c_int);
        let mut buf = [0x55 as c_char; 1];
        assert!(uio_fgets(buf.as_mut_ptr(), 1, stream).is_null());
        assert_eq!(uio_fgetc(stream), b'X' as c_int);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn ftell_and_relative_seeks_count_from_the_stream_position() {
    let dir = Dir::new(&[("t.txt", b"abcdef")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fseek(stream, 3, SEEK_SET), 0);
        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        assert_eq!(uio_ftell(stream), 2);

        assert_eq!(uio_fseek(stream, 1, SEEK_CUR), 0, "from stream position 2");
        assert_eq!(uio_ftell(stream), 3);
        assert_eq!(
            uio_fgetc(stream),
            b'd' as c_int,
            "the parked byte was dropped"
        );
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn absolute_and_end_relative_seeks_drop_the_parked_byte() {
    let dir = Dir::new(&[("t.txt", b"abcdef")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        assert_eq!(uio_fseek(stream, 1, SEEK_SET), 0);
        assert_eq!(uio_fgetc(stream), b'b' as c_int);

        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        assert_eq!(uio_fseek(stream, -1, SEEK_END), 0);
        assert_eq!(uio_fgetc(stream), b'f' as c_int);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn a_refused_seek_keeps_the_parked_byte() {
    let dir = Dir::new(&[("t.txt", b"abcdef")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        assert_eq!(uio_fseek(stream, -1, SEEK_SET), -1);
        assert_eq!(uio_fseek(stream, 0, 99), -1);
        assert_eq!(uio_fseek(stream, -2, SEEK_CUR), -1, "before the start");
        assert_eq!(uio_fgetc(stream), b'Z' as c_int);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn seeking_clears_the_end_of_file_and_error_flags() {
    let dir = Dir::new(&[("t.txt", b"a")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'a' as c_int);
        assert_eq!(uio_fgetc(stream), -1);
        assert_eq!(uio_feof(stream), 1);
        assert_eq!(uio_fseek(stream, 0, SEEK_SET), 0);
        assert_eq!(uio_feof(stream), 0);
        assert_eq!(uio_fgetc(stream), b'a' as c_int);

        (*stream).status = UIO_STREAM_STATUS_ERROR;
        assert_eq!(uio_fseek(stream, 0, SEEK_SET), 0);
        assert_eq!(uio_ferror(stream), 0, "a successful seek clears both flags");
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn pushback_at_the_end_of_the_file_clears_the_flag_and_the_end_comes_back() {
    let dir = Dir::new(&[("t.txt", b"Q")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_fgetc(stream), -1);
        assert_eq!(uio_feof(stream), 1);

        assert_eq!(uio_ungetc(b'X' as c_int, stream), b'X' as c_int);
        assert_eq!(uio_feof(stream), 0);
        assert_eq!(uio_fgetc(stream), b'X' as c_int);
        assert_eq!(
            uio_feof(stream),
            0,
            "reading the parked byte is not the end"
        );
        assert_eq!(uio_fgetc(stream), -1);
        assert_eq!(uio_feof(stream), 1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn a_refused_pushback_leaves_the_end_of_file_flag_alone() {
    let dir = Dir::new(&[("t.txt", b"Q")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_fgetc(stream), -1);
        assert_eq!(uio_ungetc(-1, stream), -1);
        assert_eq!(uio_feof(stream), 1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn a_write_only_stream_has_nothing_to_push_back_onto() {
    let dir = Dir::new(&[]);
    let stream = dir.must_open("out.txt", "w");
    unsafe {
        assert_eq!(uio_ungetc(b'X' as c_int, stream), -1);
        assert_eq!(uio_fputc(b'a' as c_int, stream), b'a' as c_int);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(dir.host("out.txt")).unwrap(), b"a");
}

#[test]
#[serial]
fn writing_after_a_pushback_lands_at_the_stream_position() {
    let dir = Dir::new(&[("rw.txt", b"0123456789")]);
    let stream = dir.must_open("rw.txt", "r+");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'0' as c_int);
        assert_eq!(uio_fgetc(stream), b'1' as c_int);
        assert_eq!(uio_ungetc(b'1' as c_int, stream), b'1' as c_int);
        assert_eq!(uio_fputc(b'X' as c_int, stream), b'X' as c_int);
        assert_eq!(uio_ftell(stream), 2);
        assert_eq!(uio_fseek(stream, 0, SEEK_SET), 0);
        assert_eq!(gets(stream, 32).as_deref(), Some("0X23456789"));
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fwrite_and_fputs_after_a_pushback_also_land_at_the_stream_position() {
    let dir = Dir::new(&[("rw.txt", b"0123456789")]);
    let stream = dir.must_open("rw.txt", "r+");
    let ab = CString::new("AB").unwrap();
    unsafe {
        assert_eq!(uio_fseek(stream, 4, SEEK_SET), 0);
        assert_eq!(uio_ungetc(b'?' as c_int, stream), b'?' as c_int);
        assert_eq!(uio_fputs(ab.as_ptr(), stream), 0);

        assert_eq!(uio_ungetc(b'?' as c_int, stream), b'?' as c_int);
        let tail = b"yz";
        assert_eq!(
            uio_fwrite(tail.as_ptr() as *const libc::c_void, 1, 2, stream),
            2
        );
        uio_fclose(stream);
    }
    assert_eq!(
        std::fs::read(dir.host("rw.txt")).unwrap(),
        b"012Ayz6789",
        "both writes started one byte before the file position"
    );
}

#[test]
#[serial]
fn reading_a_directory_reports_an_error_not_the_end_of_the_file() {
    let dir = Dir::new(&[]);
    std::fs::create_dir(dir.host("sub")).unwrap();
    let stream = dir.must_open("sub", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), -1);
        assert_eq!(uio_ferror(stream), 1);
        assert_eq!(uio_feof(stream), 0);
        uio_clearerr(stream);

        let mut line = [0 as c_char; 8];
        assert!(uio_fgets(line.as_mut_ptr(), 8, stream).is_null());
        assert_eq!(uio_ferror(stream), 1);
        uio_clearerr(stream);

        let (bytes, items) = fread_bytes(stream, 4);
        assert_eq!((bytes.len(), items), (0, 0));
        assert_eq!(uio_ferror(stream), 1);
        assert_eq!(uio_feof(stream), 0);

        assert_eq!(uio_fseek(stream, 0, SEEK_SET), 0);
        assert_eq!(uio_ferror(stream), 0);
        uio_fclose(stream);
    }
}

#[derive(Clone, Debug)]
enum Op {
    Getc,
    Ungetc(u8),
    Read(usize),
    Gets(usize),
    SeekSet(u64),
    SeekCur(i64),
    SeekEnd(i64),
    Tell,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Getc),
        3 => any::<u8>().prop_map(Op::Ungetc),
        2 => (1usize..8).prop_map(Op::Read),
        2 => (1usize..8).prop_map(Op::Gets),
        1 => (0u64..14).prop_map(Op::SeekSet),
        1 => (-6i64..6).prop_map(Op::SeekCur),
        1 => (-14i64..3).prop_map(Op::SeekEnd),
        1 => Just(Op::Tell),
    ]
}

/// stdio read semantics in the shape the stream implements them: the parked
/// byte sits in front of the file, so the stream position is one behind the
/// file position while it is parked.
struct Model {
    content: Vec<u8>,
    file_pos: u64,
    parked: Option<u8>,
    eof: bool,
}

impl Model {
    fn take(&mut self) -> Option<u8> {
        if let Some(byte) = self.parked.take() {
            return Some(byte);
        }
        let byte = self.content.get(self.file_pos as usize).copied();
        if byte.is_some() {
            self.file_pos += 1;
        }
        byte
    }

    fn read(&mut self, wanted: usize) -> Vec<u8> {
        let mut out = Vec::new();
        while out.len() < wanted {
            match self.take() {
                Some(byte) => out.push(byte),
                None => {
                    self.eof = true;
                    break;
                }
            }
        }
        out
    }

    fn gets(&mut self, size: usize) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        while out.len() + 1 < size {
            match self.take() {
                Some(byte) => {
                    out.push(byte);
                    if byte == b'\n' {
                        break;
                    }
                }
                None => {
                    self.eof = true;
                    break;
                }
            }
        }
        (!out.is_empty()).then_some(out)
    }

    fn seek(&mut self, target: Option<u64>) -> c_int {
        match target {
            Some(position) => {
                self.file_pos = position;
                self.parked = None;
                self.eof = false;
                0
            }
            None => -1,
        }
    }

    fn stream_position(&self) -> i64 {
        (self.file_pos as i64 - i64::from(self.parked.is_some())).max(0)
    }
}

fn run(content: &[u8], ops: &[Op]) {
    let dir = Dir::new(&[("model.bin", content)]);
    let stream = dir.must_open("model.bin", "r");
    let mut model = Model {
        content: content.to_vec(),
        file_pos: 0,
        parked: None,
        eof: false,
    };
    for step in ops {
        unsafe {
            match step {
                Op::Getc => {
                    let expected = model.take().map_or(-1, c_int::from);
                    if expected == -1 {
                        model.eof = true;
                    }
                    assert_eq!(uio_fgetc(stream), expected, "{step:?}");
                }
                Op::Ungetc(byte) => {
                    let expected = if model.parked.is_some() {
                        -1
                    } else {
                        model.parked = Some(*byte);
                        model.eof = false;
                        c_int::from(*byte)
                    };
                    assert_eq!(uio_ungetc(c_int::from(*byte), stream), expected, "{step:?}");
                }
                Op::Read(wanted) => {
                    let expected = model.read(*wanted);
                    let (bytes, _) = fread_bytes(stream, *wanted);
                    assert_eq!(bytes, expected, "{step:?}");
                }
                Op::Gets(size) => {
                    let expected = model.gets(*size);
                    let mut buf = vec![0x55 as c_char; *size];
                    let got = uio_fgets(buf.as_mut_ptr(), *size as c_int, stream);
                    match expected {
                        Some(line) => {
                            assert!(!got.is_null(), "{step:?}");
                            let text = CStr::from_ptr(got).to_bytes();
                            assert_eq!(text, line.as_slice(), "{step:?}");
                        }
                        None => assert!(got.is_null(), "{step:?}"),
                    }
                }
                Op::SeekSet(position) => {
                    let expected = model.seek(Some(*position));
                    assert_eq!(uio_fseek(stream, *position as c_long, SEEK_SET), expected);
                }
                Op::SeekCur(delta) => {
                    let target = u64::try_from(
                        model.file_pos as i64 - i64::from(model.parked.is_some()) + delta,
                    )
                    .ok();
                    let expected = model.seek(target);
                    assert_eq!(
                        uio_fseek(stream, *delta as c_long, SEEK_CUR),
                        expected,
                        "{step:?}"
                    );
                }
                Op::SeekEnd(delta) => {
                    let target = u64::try_from(content.len() as i64 + delta).ok();
                    let expected = model.seek(target);
                    assert_eq!(
                        uio_fseek(stream, *delta as c_long, SEEK_END),
                        expected,
                        "{step:?}"
                    );
                }
                Op::Tell => {
                    assert_eq!(
                        uio_ftell(stream) as i64,
                        model.stream_position(),
                        "{step:?}"
                    );
                }
            }
            assert_eq!(
                uio_feof(stream) != 0,
                model.eof,
                "end-of-file flag after {step:?}"
            );
        }
    }
    unsafe { uio_fclose(stream) };
}

#[test]
#[serial]
fn stream_reads_seeks_and_pushback_match_the_stdio_reference_model() {
    let strategy = (vec(any::<u8>(), 0..12), vec(op(), 1..40));
    let mut runner = TestRunner::new(Config {
        cases: 128,
        failure_persistence: None,
        ..Config::default()
    });
    runner
        .run(&strategy, |(content, ops)| {
            run(&content, &ops);
            Ok(())
        })
        .unwrap();
}
