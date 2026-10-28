//! The tracker (MOD) decoder vtable driven the way the C sound layer drives
//! it, on the reference modules checked in under tests/fixtures/tracker.

use std::ffi::c_void;
use std::path::PathBuf;
use std::ptr;

use serial_test::serial;

use super::decoder_vtable_tests::{
    decode_all, formats, open_file, DecoderStorage, Directory, STEREO16,
};
use super::ffi::TFB_SoundDecoderFuncs;
use super::mod_ffi::rust_mod_DecoderVtbl;

/// Rendered length of short-periods.mod, recorded in reference.json.
const SHORT_PERIODS_FRAMES: usize = 6174;
const FRAME_BYTES: usize = 4;

fn funcs() -> &'static TFB_SoundDecoderFuncs {
    &rust_mod_DecoderVtbl
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("UQM_RUST_ROOT"))
        .join("tests/fixtures/tracker")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {}: {e}", path.display()))
}

struct Session {
    directory: Directory,
    storage: DecoderStorage,
}

impl Session {
    fn new(files: &[(&str, Vec<u8>)]) -> Self {
        let directory = Directory::new();
        for (name, bytes) in files {
            directory.write(name, bytes);
        }
        let fmts = formats();
        assert_eq!((funcs().InitModule)(0, &fmts), 1);
        let mut storage = DecoderStorage::new(funcs());
        assert_eq!((funcs().Init)(storage.decoder()), 1);
        Session { directory, storage }
    }

    fn with_short_periods() -> Self {
        Session::new(&[("song.mod", fixture("short-periods.mod"))])
    }

    fn open(&mut self, name: &str) -> i32 {
        open_file(funcs(), &mut self.storage, &self.directory, name)
    }

    fn decode_all(&mut self) -> Vec<u8> {
        decode_all(funcs(), &mut self.storage)
    }

    fn seek(&mut self, frame: u32) -> u32 {
        (funcs().Seek)(self.storage.decoder(), frame)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        (funcs().Term)(self.storage.decoder());
        (funcs().TermModule)();
    }
}

#[test]
#[serial]
fn a_module_opens_as_stereo16_with_its_rendered_length() {
    let mut session = Session::with_short_periods();
    assert_eq!(session.open("song.mod"), 1);
    let decoder = unsafe { &*session.storage.decoder() };
    assert_eq!(decoder.frequency, 44_100);
    assert_eq!(decoder.format, STEREO16);
    assert!(!decoder.is_null);
    assert!(!decoder.need_swap);
    let expected = SHORT_PERIODS_FRAMES as f32 / 44_100.0;
    assert!(
        (decoder.length - expected).abs() < 1e-3,
        "length {} expected {}",
        decoder.length,
        expected
    );
}

#[test]
#[serial]
fn decoding_renders_the_whole_module_then_reports_the_end() {
    let mut session = Session::with_short_periods();
    session.open("song.mod");
    let pcm = session.decode_all();
    assert_eq!(pcm.len(), SHORT_PERIODS_FRAMES * FRAME_BYTES);
    assert!(pcm.iter().any(|&b| b != 0), "the module is audible");
    let mut spare = [0u8; 64];
    assert_eq!(
        (funcs().Decode)(
            session.storage.decoder(),
            spare.as_mut_ptr() as *mut c_void,
            64
        ),
        0,
        "after the last frame Decode returns 0"
    );
}

#[test]
#[serial]
fn seeking_resumes_exactly_where_uninterrupted_playback_would_be() {
    let mut session = Session::with_short_periods();
    session.open("song.mod");
    let full = session.decode_all();

    for frame in [0usize, 1, 997, 4096] {
        assert_eq!(session.seek(frame as u32), frame as u32);
        let rest = session.decode_all();
        assert_eq!(rest, full[frame * FRAME_BYTES..], "seek to frame {frame}");
    }
}

#[test]
#[serial]
fn seeking_past_the_end_stops_at_the_last_frame() {
    let mut session = Session::with_short_periods();
    session.open("song.mod");
    assert_eq!(session.seek(u32::MAX), SHORT_PERIODS_FRAMES as u32);
    assert!(session.decode_all().is_empty());
}

#[test]
#[serial]
fn the_frame_counter_counts_blocks_of_1024_frames() {
    let mut session = Session::with_short_periods();
    session.open("song.mod");
    assert_eq!((funcs().GetFrame)(session.storage.decoder()), 0);
    session.seek(3000);
    assert_eq!((funcs().GetFrame)(session.storage.decoder()), 2);
}

#[test]
#[serial]
fn opening_something_that_is_not_a_module_fails() {
    let mut session = Session::new(&[
        ("text.mod", b"this is plainly not a tracker module".to_vec()),
        ("empty.mod", Vec::new()),
    ]);
    assert_eq!(session.open("text.mod"), 0);
    assert_eq!(session.open("empty.mod"), 0);
    assert_eq!(session.open("absent.mod"), 0);
}

#[test]
#[serial]
fn close_ends_playback_and_the_module_can_be_reopened() {
    let mut session = Session::with_short_periods();
    session.open("song.mod");
    let first = session.decode_all();
    (funcs().Close)(session.storage.decoder());
    let mut buf = [0u8; 16];
    assert_eq!(
        (funcs().Decode)(
            session.storage.decoder(),
            buf.as_mut_ptr() as *mut c_void,
            16
        ),
        -1
    );
    assert_eq!(session.open("song.mod"), 1);
    assert_eq!(session.decode_all(), first);
}

#[test]
#[serial]
fn calls_with_missing_pointers_or_sizes_are_refused() {
    let mut session = Session::with_short_periods();
    session.open("song.mod");
    let name = std::ffi::CString::new("song.mod").unwrap();
    let mut buf = [0u8; 16];
    assert_eq!((funcs().Init)(ptr::null_mut()), 0);
    assert_eq!(
        (funcs().Open)(ptr::null_mut(), session.directory.handle(), name.as_ptr()),
        0
    );
    assert_eq!(
        (funcs().Open)(
            session.storage.decoder(),
            session.directory.handle(),
            ptr::null()
        ),
        0
    );
    assert_eq!(
        (funcs().Decode)(session.storage.decoder(), ptr::null_mut(), 16),
        -1
    );
    assert_eq!(
        (funcs().Decode)(
            session.storage.decoder(),
            buf.as_mut_ptr() as *mut c_void,
            0
        ),
        -1
    );
    assert_eq!((funcs().Seek)(ptr::null_mut(), 5), 0);
    assert_eq!((funcs().GetFrame)(ptr::null_mut()), 0);
    assert_eq!((funcs().GetError)(ptr::null_mut()), -1);
    (funcs().Close)(ptr::null_mut());
    (funcs().Term)(ptr::null_mut());
    assert_eq!((funcs().InitModule)(0, ptr::null()), 0);
}

#[test]
#[serial]
fn a_decoder_without_an_instance_answers_with_defaults() {
    let mut storage = DecoderStorage::new(funcs());
    let mut buf = [0u8; 16];
    assert_eq!((funcs().GetError)(storage.decoder()), -1);
    assert_eq!((funcs().GetFrame)(storage.decoder()), 0);
    assert_eq!((funcs().Seek)(storage.decoder(), 9), 0);
    assert_eq!(
        (funcs().Decode)(storage.decoder(), buf.as_mut_ptr() as *mut c_void, 16),
        -1
    );
    (funcs().Close)(storage.decoder());
    (funcs().Term)(storage.decoder());
}
