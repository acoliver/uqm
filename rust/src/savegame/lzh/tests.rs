use super::*;
use std::ffi::c_void;

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

unsafe fn encode(input: &[u8], chunk: usize) -> Vec<u8> {
    let mut output = vec![0; input.len() * 2 + 1024];
    let desc = copen(output.as_mut_ptr().cast(), 1, 1);
    assert!(!desc.is_null());
    for part in input.chunks(chunk) {
        assert_eq!(
            cwrite(part.as_ptr().cast(), 1, part.len() as u16, desc),
            part.len() as u16
        );
    }
    let len = cclose(desc) as usize;
    output.truncate(if input.is_empty() { 4 } else { len });
    output
}

unsafe fn decode(input: &[u8], chunk: usize) -> Vec<u8> {
    // The original memory API, unlike FILE_STREAM, has no EOF signal. The
    // Huffman bit reader prefetches up to two bytes past the payload.
    let mut padded = input.to_vec();
    padded.extend([0; 8]);
    let desc = copen(padded.as_mut_ptr().cast(), 1, 0);
    assert!(!desc.is_null());
    let mut len = 0;
    cfilelength(desc, &mut len);
    let mut output = vec![0; len as usize];
    for part in output.chunks_mut(chunk) {
        assert_eq!(
            cread(part.as_mut_ptr().cast(), 1, part.len() as u16, desc),
            part.len() as u16
        );
    }
    let mut extra = 0u8;
    assert_eq!(cread((&mut extra as *mut u8).cast(), 1, 1, desc), 0);
    assert_eq!(cclose(desc), len);
    output
}

#[test]
#[serial_test::serial]
fn c_encoder_fixture_is_byte_identical_and_decodes_across_read_boundaries() {
    let input = legacy_input();
    unsafe {
        for chunk in [1, 15, 16, 17, 257, 4096, 65535] {
            assert_eq!(encode(&input, chunk), LEGACY, "write chunk {chunk}");
            assert_eq!(decode(LEGACY, chunk), input, "read chunk {chunk}");
        }
    }
}

#[test]
#[serial_test::serial]
fn c_encoder_short_stream_vectors() {
    let vectors: Vec<(Vec<u8>, Vec<u8>)> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/lzh/short-streams.json"
    ))
    .unwrap();
    unsafe {
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
}

#[test]
#[serial_test::serial]
fn round_trips_dictionary_and_lookahead_boundaries() {
    unsafe {
        for len in [0, 1, 2, 15, 16, 17, 4095, 4096, 4097, 65535] {
            let input: Vec<_> = (0..len).map(|i| (i * 17) as u8).collect();
            let packed = encode(&input, 257);
            assert_eq!(decode(&packed, 13), input);
        }
    }
}

#[test]
#[serial_test::serial]
fn whole_items_eof_and_null_descriptors() {
    unsafe {
        let input = vec![b'a'; 37];
        let mut packed = encode(&input, 7);
        packed.extend([0; 8]);
        let desc = copen(packed.as_mut_ptr().cast(), 1, 0);
        let mut output = [0u8; 40];
        assert_eq!(cread(output.as_mut_ptr().cast(), 4, 10, desc), 9);
        assert_eq!(output[..36], input[..36]);
        assert_eq!(cread(output.as_mut_ptr().cast(), 2, 1, desc), 0);
        assert_eq!(cread(output.as_mut_ptr().cast(), 1, 1, desc), 1);
        assert_eq!(cclose(desc), 37);
        assert_eq!(cclose(ptr::null_mut()), 0);
        let mut len = 123;
        cfilelength(ptr::null_mut(), &mut len);
        assert_eq!(len, 0);
        assert_eq!(cread(ptr::null_mut(), 0, 0, ptr::null_mut()), 0);
        assert_eq!(cwrite(ptr::null(), 0, 0, ptr::null_mut()), 0);
        let mut invalid = [255u8; 4];
        assert!(copen(invalid.as_mut_ptr().cast(), 1, 0).is_null());
    }
}

#[test]
#[serial_test::serial]
fn file_stream_header_is_rewritten_at_its_original_offset() {
    use crate::io::uio_bridge::{
        uio_closeDir, uio_closeRepository, uio_mountDir, uio_openDir, uio_openRepository,
    };
    use crate::resource::ffi_bridge::{
        res_CloseResFile as CloseResFile, res_OpenResFile as OpenResFile, SeekResFile, TellResFile,
    };
    use std::ffi::CString;

    std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp")).unwrap();
    let temp = tempfile::Builder::new()
        .prefix("lzh-")
        .tempdir_in(concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp"))
        .unwrap();
    let host = CString::new(temp.path().to_str().unwrap()).unwrap();
    let input = legacy_input();
    unsafe {
        let repo = uio_openRepository(0);
        assert!(!uio_mountDir(
            repo,
            c"/".as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null(),
            host.as_ptr(),
            ptr::null_mut(),
            4,
            ptr::null_mut()
        )
        .is_null());
        let root = uio_openDir(repo, c"/".as_ptr(), 0);
        let stream = OpenResFile(root.cast::<c_void>(), c"save".as_ptr(), c"wb+".as_ptr());
        assert!(!stream.is_null());
        for byte in b"prefix" {
            PutResFileChar(*byte as _, stream);
        }
        let desc = copen(stream, 0, 1);
        for part in input.chunks(4096) {
            assert_eq!(
                cwrite(part.as_ptr().cast(), 1, part.len() as u16, desc),
                part.len() as u16
            );
        }
        assert_eq!(cclose(desc) as usize, LEGACY.len());
        assert_eq!(TellResFile(stream), 10);
        CloseResFile(stream);
        assert_eq!(
            std::fs::read(temp.path().join("save")).unwrap(),
            [b"prefix".as_slice(), LEGACY].concat()
        );
        let stream = OpenResFile(root.cast(), c"save".as_ptr(), c"rb".as_ptr());
        assert_eq!(SeekResFile(stream, 6, libc::SEEK_SET), 0);
        let desc = copen(stream, 0, 0);
        let mut output = vec![0u8; input.len()];
        for part in output.chunks_mut(113) {
            assert_eq!(
                cread(part.as_mut_ptr().cast(), 1, part.len() as u16, desc),
                part.len() as u16
            );
        }
        assert_eq!(output, input);
        cclose(desc);
        CloseResFile(stream);
        uio_closeDir(root);
        uio_closeRepository(repo);
    }
}
