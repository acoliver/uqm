use super::*;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

// Captured from the original C encoder before its deletion. The input below
// crosses the dictionary ring and the Huffman frequency reconstruction limit.
const LEGACY: &[u8] = include_bytes!("../../../tests/fixtures/lzh/legacy.lzh");

fn legacy_input() -> Vec<u8> {
    let mut bytes = vec![b' '; 8192];
    bytes.extend(b"Star Control II legacy compression\0".repeat(1000));
    let mut x = 123456789u32;
    for _ in 0..60000 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        bytes.push((x >> 24) as u8);
    }
    bytes
}

fn encode(input: &[u8], chunk: usize) -> Vec<u8> {
    let mut encoder = Encoder::new(Cursor::new(Vec::new())).unwrap();
    for part in input.chunks(chunk) {
        encoder.write_all(part).unwrap();
    }
    encoder.finish().unwrap().into_inner()
}

fn decode(input: &[u8], chunk: usize) -> Vec<u8> {
    let mut decoder = Decoder::new(input).unwrap();
    let mut output = vec![0; decoder.len() as usize];
    for part in output.chunks_mut(chunk) {
        decoder.read_exact(part).unwrap();
    }
    assert_eq!(decoder.read(&mut [0]).unwrap(), 0);
    output
}

#[test]
fn c_encoder_fixture_is_byte_identical_and_decodes_across_read_boundaries() {
    let input = legacy_input();
    for chunk in [1, 15, 16, 17, 257, 4096, 65535] {
        assert_eq!(encode(&input, chunk), LEGACY, "write chunk {chunk}");
        assert_eq!(decode(LEGACY, chunk), input, "read chunk {chunk}");
    }
}

#[test]
fn c_encoder_short_stream_vectors() {
    let vectors: Vec<(Vec<u8>, Vec<u8>)> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/lzh/short-streams.json"
    ))
    .unwrap();
    for (input, expected) in vectors {
        for chunk in [1, 15, 16, 17, 256] {
            assert_eq!(
                encode(&input, chunk),
                expected,
                "length {} chunk {chunk}",
                input.len()
            );
            assert_eq!(decode(&expected, chunk), input);
        }
    }
}

#[test]
fn round_trips_dictionary_and_lookahead_boundaries() {
    for len in [0, 1, 2, 15, 16, 17, 4095, 4096, 4097, 65535] {
        let input: Vec<_> = (0..len).map(|i| (i * 17) as u8).collect();
        let packed = encode(&input, 257);
        assert_eq!(decode(&packed, 13), input);
    }
}

#[test]
fn eof_zero_length_reads_and_invalid_headers() {
    let input = vec![b'a'; 37];
    let packed = encode(&input, 7);
    let mut decoder = Decoder::new(packed.as_slice()).unwrap();
    let mut output = [0; 40];
    assert_eq!(decoder.read(&mut []).unwrap(), 0);
    assert_eq!(decoder.read(&mut output[..36]).unwrap(), 36);
    assert_eq!(output[..36], input[..36]);
    assert_eq!(decoder.read(&mut output[..2]).unwrap(), 1);
    assert_eq!(output[0], b'a');
    assert_eq!(decoder.read(&mut output).unwrap(), 0);
    assert!(
        matches!(Decoder::new([255; 4].as_slice()), Err(error) if error.kind() == io::ErrorKind::InvalidData)
    );
    assert!(
        matches!(Decoder::new([0; 3].as_slice()), Err(error) if error.kind() == io::ErrorKind::UnexpectedEof)
    );
}

#[test]
fn stream_header_is_rewritten_at_its_original_offset() {
    let input = legacy_input();
    let mut stream = Cursor::new(Vec::new());
    stream.write_all(b"prefix").unwrap();
    let mut encoder = Encoder::new(stream).unwrap();
    for part in input.chunks(4096) {
        encoder.write_all(part).unwrap();
    }
    let mut stream = encoder.finish().unwrap();
    assert_eq!(stream.position(), 10);
    assert_eq!(stream.get_ref(), &[b"prefix".as_slice(), LEGACY].concat());
    stream.seek(SeekFrom::Start(6)).unwrap();
    let mut decoder = Decoder::new(stream).unwrap();
    let mut output = vec![0; input.len()];
    for part in output.chunks_mut(113) {
        decoder.read_exact(part).unwrap();
    }
    assert_eq!(output, input);
}

#[test]
fn decoders_do_not_share_state() {
    let first = encode(b"first stream", 1);
    let second = encode(b"second stream", 2);
    let mut a = Decoder::new(first.as_slice()).unwrap();
    let mut b = Decoder::new(second.as_slice()).unwrap();
    for (&left, &right) in b"first stream".iter().zip(b"second stream") {
        let mut byte = [0];
        a.read_exact(&mut byte).unwrap();
        assert_eq!(byte[0], left);
        b.read_exact(&mut byte).unwrap();
        assert_eq!(byte[0], right);
    }
}

struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::ErrorKind::PermissionDenied.into())
    }
}
impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::WriteZero.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}
impl Seek for Broken {
    fn seek(&mut self, _: SeekFrom) -> io::Result<u64> {
        Err(io::ErrorKind::NotSeekable.into())
    }
}

#[test]
fn stream_errors_are_reported() {
    assert!(
        matches!(Decoder::new(Broken), Err(error) if error.kind() == io::ErrorKind::PermissionDenied)
    );
    assert!(matches!(Encoder::new(Broken), Err(error) if error.kind() == io::ErrorKind::WriteZero));
    let header = Cursor::new(1u32.to_le_bytes());
    let mut decoder = Decoder::new(header.chain(Broken)).unwrap();
    assert_eq!(
        decoder.read(&mut [0]).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
}

struct FailingWriter {
    inner: Cursor<Vec<u8>>,
    fail_seek: bool,
}
impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}
impl Seek for FailingWriter {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        if self.fail_seek {
            Err(io::ErrorKind::NotSeekable.into())
        } else {
            self.inner.seek(position)
        }
    }
}

#[test]
fn finishing_reports_header_rewrite_and_flush_errors() {
    for fail_seek in [false, true] {
        let mut encoder = Encoder::new(FailingWriter {
            inner: Cursor::new(Vec::new()),
            fail_seek,
        })
        .unwrap();
        encoder.write_all(b"compressed payload").unwrap();
        let error = encoder.finish().err().unwrap();
        assert_eq!(
            error.kind(),
            if fail_seek {
                io::ErrorKind::NotSeekable
            } else {
                io::ErrorKind::BrokenPipe
            }
        );
    }
}
