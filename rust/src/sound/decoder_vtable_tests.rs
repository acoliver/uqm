//! The WAV and AIFF decoder vtables driven the way the C sound layer drives
//! them: module init, per-decoder init, open from a uio directory, decode,
//! seek and teardown, on real files behind a real uio mount.

use std::ffi::{c_void, CString};
use std::ptr;

use serial_test::serial;
use tempfile::TempDir;

use super::aiff_ffi::rust_aifa_DecoderVtbl;
use super::ffi::{uio_DirHandle, TFB_DecoderFormats, TFB_SoundDecoder, TFB_SoundDecoderFuncs};
use super::wav_ffi::rust_wav_DecoderVtbl;
use crate::io::uio_bridge::{
    uio_closeDir, uio_closeRepository, uio_init, uio_mountDir, uio_openDir, uio_openRepository,
    uio_unInit,
};

const MONO8: u32 = 101;
const STEREO8: u32 = 102;
const MONO16: u32 = 103;
const STEREO16: u32 = 104;
const UIO_FSTYPE_STDIO: i32 = 1;
const UIO_MOUNT_TOP: i32 = 1 << 2;

fn formats() -> TFB_DecoderFormats {
    TFB_DecoderFormats {
        big_endian: false,
        want_big_endian: false,
        mono8: MONO8,
        stereo8: STEREO8,
        mono16: MONO16,
        stereo16: STEREO16,
    }
}

/// A uio repository whose root is a temporary directory.
struct Directory {
    files: TempDir,
    repository: *mut crate::io::uio_bridge::uio_Repository,
    handle: *mut crate::io::uio_bridge::uio_DirHandle,
}

impl Directory {
    fn new() -> Self {
        let files = TempDir::new().unwrap();
        unsafe {
            uio_init();
            let repository = uio_openRepository(0);
            let root = CString::new("/").unwrap();
            let host = CString::new(files.path().to_str().unwrap()).unwrap();
            let mount = uio_mountDir(
                repository,
                root.as_ptr(),
                UIO_FSTYPE_STDIO,
                ptr::null_mut(),
                ptr::null(),
                host.as_ptr(),
                ptr::null_mut(),
                UIO_MOUNT_TOP,
                ptr::null_mut(),
            );
            assert!(!mount.is_null());
            let handle = uio_openDir(repository, root.as_ptr(), 0);
            assert!(!handle.is_null());
            Directory {
                files,
                repository,
                handle,
            }
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.files.path().join(name), bytes).unwrap();
    }

    fn handle(&self) -> *mut uio_DirHandle {
        self.handle as *mut uio_DirHandle
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        unsafe {
            uio_closeDir(self.handle);
            uio_closeRepository(self.repository);
            uio_unInit();
        }
    }
}

/// Zeroed storage big enough for the decoder's wrapper struct.
struct DecoderStorage {
    words: Vec<u64>,
}

impl DecoderStorage {
    fn new(funcs: &TFB_SoundDecoderFuncs) -> Self {
        let bytes = (funcs.GetStructSize)() as usize;
        Self {
            words: vec![0u64; bytes.div_ceil(8)],
        }
    }

    fn decoder(&mut self) -> *mut TFB_SoundDecoder {
        self.words.as_mut_ptr() as *mut TFB_SoundDecoder
    }
}

fn pcm_bytes(frames: usize, channels: usize, bytes_per_sample: usize) -> Vec<u8> {
    (0..frames * channels * bytes_per_sample)
        .map(|i| (i % 251) as u8)
        .collect()
}

fn wav_file(channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let block_align = channels * bits / 8;
    let mut file = Vec::new();
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    file.extend_from_slice(b"WAVEfmt ");
    file.extend_from_slice(&16u32.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&channels.to_le_bytes());
    file.extend_from_slice(&rate.to_le_bytes());
    file.extend_from_slice(&(rate * block_align as u32).to_le_bytes());
    file.extend_from_slice(&block_align.to_le_bytes());
    file.extend_from_slice(&bits.to_le_bytes());
    file.extend_from_slice(b"data");
    file.extend_from_slice(&(data.len() as u32).to_le_bytes());
    file.extend_from_slice(data);
    file
}

/// 22050 Hz as an IEEE 80-bit extended float, as AIFF stores sample rates.
const RATE_22050_EXTENDED: [u8; 10] = [0x40, 0x0D, 0xAC, 0x44, 0, 0, 0, 0, 0, 0];

fn aiff_file(channels: u16, frames: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let mut chunks = Vec::new();
    chunks.extend_from_slice(b"COMM");
    chunks.extend_from_slice(&18u32.to_be_bytes());
    chunks.extend_from_slice(&channels.to_be_bytes());
    chunks.extend_from_slice(&frames.to_be_bytes());
    chunks.extend_from_slice(&bits.to_be_bytes());
    chunks.extend_from_slice(&RATE_22050_EXTENDED);
    chunks.extend_from_slice(b"SSND");
    chunks.extend_from_slice(&(8 + data.len() as u32).to_be_bytes());
    chunks.extend_from_slice(&0u32.to_be_bytes());
    chunks.extend_from_slice(&0u32.to_be_bytes());
    chunks.extend_from_slice(data);
    let mut file = Vec::new();
    file.extend_from_slice(b"FORM");
    file.extend_from_slice(&(4 + chunks.len() as u32).to_be_bytes());
    file.extend_from_slice(b"AIFF");
    file.extend_from_slice(&chunks);
    file
}

fn open_file(
    funcs: &TFB_SoundDecoderFuncs,
    storage: &mut DecoderStorage,
    directory: &Directory,
    name: &str,
) -> i32 {
    let name = CString::new(name).unwrap();
    (funcs.Open)(storage.decoder(), directory.handle(), name.as_ptr())
}

fn decode_all(funcs: &TFB_SoundDecoderFuncs, storage: &mut DecoderStorage) -> Vec<u8> {
    let mut out = Vec::new();
    let mut chunk = [0u8; 64];
    loop {
        let n = (funcs.Decode)(
            storage.decoder(),
            chunk.as_mut_ptr() as *mut c_void,
            chunk.len() as i32,
        );
        if n <= 0 {
            break;
        }
        out.extend_from_slice(&chunk[..n as usize]);
    }
    out
}

/// One full decoder lifecycle against a mono 16-bit file of 100 frames.
fn play_a_mono16_file(funcs: &TFB_SoundDecoderFuncs, file_name: &str, file: Vec<u8>, pcm: &[u8]) {
    let directory = Directory::new();
    directory.write(file_name, &file);
    let fmts = formats();
    assert_eq!((funcs.InitModule)(0, &fmts), 1);
    let mut storage = DecoderStorage::new(funcs);

    assert_eq!((funcs.Init)(storage.decoder()), 1);
    assert_eq!(open_file(funcs, &mut storage, &directory, file_name), 1);

    let decoder = unsafe { &*storage.decoder() };
    assert_eq!(decoder.frequency, 22050);
    assert_eq!(decoder.format, MONO16);
    assert!(!decoder.is_null);
    assert!(
        (decoder.length - 100.0 / 22050.0).abs() < 1e-6,
        "length {}",
        decoder.length
    );

    let decoded = decode_all(funcs, &mut storage);
    assert_eq!(decoded.len(), pcm.len());
    assert_eq!(decoded.as_slice(), pcm);

    assert_eq!((funcs.Seek)(storage.decoder(), 40), 40);
    let after_seek = decode_all(funcs, &mut storage);
    assert_eq!(
        after_seek.as_slice(),
        &pcm[80..],
        "40 frames of 2 bytes skipped"
    );
    assert_eq!(
        (funcs.Seek)(storage.decoder(), 100_000),
        100,
        "seeking past the end clamps to the length"
    );

    (funcs.Close)(storage.decoder());
    (funcs.Term)(storage.decoder());
    (funcs.TermModule)();
}

#[test]
#[serial]
fn wav_decoder_plays_a_file_from_open_to_term() {
    let pcm = pcm_bytes(100, 1, 2);
    let file = wav_file(1, 22050, 16, &pcm);

    play_a_mono16_file(&rust_wav_DecoderVtbl, "clip.wav", file, &pcm);
}

#[test]
#[serial]
fn aiff_decoder_plays_a_file_from_open_to_term() {
    let pcm = pcm_bytes(100, 1, 2);
    let file = aiff_file(1, 100, 16, &pcm);

    play_a_mono16_file(&rust_aifa_DecoderVtbl, "clip.aif", file, &pcm);
}

fn decoders() -> [(&'static str, &'static TFB_SoundDecoderFuncs, &'static str); 2] {
    [
        ("wav", &rust_wav_DecoderVtbl, "Rust Wave"),
        ("aiff", &rust_aifa_DecoderVtbl, "Rust AIFF"),
    ]
}

#[test]
#[serial]
fn decoders_report_their_names_and_a_usable_struct_size() {
    for (label, funcs, name) in decoders() {
        let reported = unsafe { std::ffi::CStr::from_ptr((funcs.GetName)()) };
        assert_eq!(reported.to_str().unwrap(), name, "{label}");
        assert!(
            (funcs.GetStructSize)() as usize
                >= std::mem::size_of::<TFB_SoundDecoder>() + std::mem::size_of::<*mut c_void>(),
            "{label} wrapper holds the base struct plus its decoder pointer"
        );
    }
}

#[test]
#[serial]
fn module_init_without_formats_is_refused() {
    for (label, funcs, _) in decoders() {
        assert_eq!((funcs.InitModule)(0, ptr::null()), 0, "{label}");
    }
}

#[test]
#[serial]
fn opening_a_missing_or_malformed_file_fails_cleanly() {
    for (label, funcs, _) in decoders() {
        let directory = Directory::new();
        directory.write("garbage.bin", b"this is not an audio file at all");
        directory.write("empty.bin", b"");
        let fmts = formats();
        assert_eq!((funcs.InitModule)(0, &fmts), 1, "{label}");
        let mut storage = DecoderStorage::new(funcs);
        assert_eq!((funcs.Init)(storage.decoder()), 1, "{label}");

        assert_eq!(
            open_file(funcs, &mut storage, &directory, "absent.bin"),
            0,
            "{label}"
        );
        assert_eq!(
            open_file(funcs, &mut storage, &directory, "garbage.bin"),
            0,
            "{label}"
        );
        assert_eq!(
            open_file(funcs, &mut storage, &directory, "empty.bin"),
            0,
            "{label}"
        );

        (funcs.Term)(storage.decoder());
        (funcs.TermModule)();
    }
}

#[test]
#[serial]
fn opening_before_the_decoder_is_initialised_fails() {
    for (label, funcs, _) in decoders() {
        let directory = Directory::new();
        directory.write("clip.bin", &wav_file(1, 22050, 16, &pcm_bytes(4, 1, 2)));
        let mut storage = DecoderStorage::new(funcs);

        assert_eq!(
            open_file(funcs, &mut storage, &directory, "clip.bin"),
            0,
            "{label}"
        );
        assert_eq!((funcs.GetError)(storage.decoder()), -1, "{label}");
        assert_eq!((funcs.GetFrame)(storage.decoder()), 0, "{label}");
    }
}

#[test]
#[serial]
fn decoding_into_an_empty_buffer_produces_no_audio() {
    for (label, funcs, _) in decoders() {
        let mut storage = DecoderStorage::new(funcs);
        let mut byte = 0u8;

        let into_null = (funcs.Decode)(storage.decoder(), ptr::null_mut(), 16);
        let zero_length = (funcs.Decode)(storage.decoder(), &mut byte as *mut u8 as *mut c_void, 0);

        assert!(into_null <= 0, "{label}: {into_null}");
        assert!(zero_length <= 0, "{label}: {zero_length}");
    }
}

#[test]
#[serial]
fn decoders_without_an_instance_ignore_close_term_and_decode() {
    for (label, funcs, _) in decoders() {
        (funcs.Close)(ptr::null_mut());
        (funcs.Term)(ptr::null_mut());
        assert_eq!((funcs.Init)(ptr::null_mut()), 0, "{label}");
        let mut chunk = [0u8; 4];
        assert!(
            (funcs.Decode)(ptr::null_mut(), chunk.as_mut_ptr() as *mut c_void, 4) <= 0,
            "{label}"
        );
    }
}

#[test]
#[serial]
fn stereo_eight_bit_aiff_selects_the_stereo8_format_code() {
    let directory = Directory::new();
    let pcm = pcm_bytes(10, 2, 1);
    directory.write("clip.aif", &aiff_file(2, 10, 8, &pcm));
    let funcs = &rust_aifa_DecoderVtbl;
    let fmts = formats();
    assert_eq!((funcs.InitModule)(0, &fmts), 1);
    let mut storage = DecoderStorage::new(funcs);
    assert_eq!((funcs.Init)(storage.decoder()), 1);

    assert_eq!(open_file(funcs, &mut storage, &directory, "clip.aif"), 1);

    assert_eq!(unsafe { (*storage.decoder()).format }, STEREO8);
    let unsigned: Vec<u8> = pcm.iter().map(|sample| sample.wrapping_add(128)).collect();
    assert_eq!(
        decode_all(funcs, &mut storage),
        unsigned,
        "signed AIFF 8-bit samples are delivered as unsigned"
    );
    (funcs.Term)(storage.decoder());
    (funcs.TermModule)();
}

#[test]
#[serial]
fn aiff_refuses_to_open_once_the_module_formats_are_torn_down() {
    let directory = Directory::new();
    directory.write("clip.aif", &aiff_file(1, 4, 16, &pcm_bytes(4, 1, 2)));
    let funcs = &rust_aifa_DecoderVtbl;
    let fmts = formats();
    assert_eq!((funcs.InitModule)(0, &fmts), 1);
    let mut storage = DecoderStorage::new(funcs);
    assert_eq!((funcs.Init)(storage.decoder()), 1);
    (funcs.TermModule)();

    assert_eq!(open_file(funcs, &mut storage, &directory, "clip.aif"), 0);

    (funcs.Term)(storage.decoder());
}
