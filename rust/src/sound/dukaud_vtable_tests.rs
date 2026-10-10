//! The DukAud decoder vtable driven the way the C sound layer drives it,
//! on synthetic .duk/.frm pairs behind a real uio mount.

use std::ffi::c_void;
use std::ptr;

use serial_test::serial;

use super::decoder_vtable_tests::{
    decode_all, formats, open_file, DecoderStorage, Directory, STEREO16,
};
use super::dukaud_ffi::rust_duka_DecoderVtbl;
use super::ffi::TFB_SoundDecoderFuncs;

const MAGIC: u16 = 0xf77f;
const FRAMES: usize = 3;
const BYTES_PER_FRAME: usize = 8;

/// ADPCM payload bytes of frame `frame`; each differs so replays are checkable.
fn adpcm(frame: usize) -> Vec<u8> {
    (0..BYTES_PER_FRAME)
        .map(|i| (0x17u8).wrapping_mul((frame * BYTES_PER_FRAME + i + 1) as u8))
        .collect()
}

fn duk_and_frm(frames: usize) -> (Vec<u8>, Vec<u8>) {
    let audio_size = 10 + BYTES_PER_FRAME;
    let mut duk = Vec::new();
    let mut frm = Vec::new();
    for frame in 0..frames {
        frm.extend_from_slice(&(duk.len() as u32).to_be_bytes());
        duk.extend_from_slice(&(audio_size as u32).to_be_bytes());
        duk.extend_from_slice(&0u32.to_be_bytes());
        duk.extend_from_slice(&MAGIC.to_be_bytes());
        duk.extend_from_slice(&(BYTES_PER_FRAME as u16).to_be_bytes());
        duk.extend_from_slice(&1u16.to_be_bytes());
        duk.extend_from_slice(&(frame as u16 + 3).to_be_bytes());
        duk.extend_from_slice(&(frame as u16 + 5).to_be_bytes());
        duk.extend(adpcm(frame));
    }
    (duk, frm)
}

fn funcs() -> &'static TFB_SoundDecoderFuncs {
    &rust_duka_DecoderVtbl
}

struct Session {
    directory: Directory,
    storage: DecoderStorage,
}

impl Session {
    fn new(files: &[(&str, &[u8])]) -> Self {
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

    fn open(&mut self, name: &str) -> i32 {
        open_file(funcs(), &mut self.storage, &self.directory, name)
    }

    fn decode_all(&mut self) -> Vec<u8> {
        decode_all(funcs(), &mut self.storage)
    }

    fn seek(&mut self, pcm: u32) -> u32 {
        (funcs().Seek)(self.storage.decoder(), pcm)
    }

    fn frame(&mut self) -> u32 {
        (funcs().GetFrame)(self.storage.decoder())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        (funcs().Term)(self.storage.decoder());
        (funcs().TermModule)();
    }
}

fn three_frame_session() -> Session {
    let (duk, frm) = duk_and_frm(FRAMES);
    Session::new(&[("clip.duk", &duk), ("clip.frm", &frm)])
}

#[test]
#[serial]
fn the_vtable_names_itself_and_sizes_its_wrapper() {
    let name = unsafe { std::ffi::CStr::from_ptr((funcs().GetName)()) };
    assert_eq!(name.to_str().unwrap(), "Rust DukAud");
    assert!(
        (funcs().GetStructSize)() as usize
            >= std::mem::size_of::<super::ffi::TFB_SoundDecoder>()
                + std::mem::size_of::<*mut c_void>()
    );
    assert_eq!((funcs().InitModule)(0, ptr::null()), 0);
}

#[test]
#[serial]
fn opening_reports_stereo16_at_the_fixed_rate_and_the_clip_length() {
    let mut session = three_frame_session();
    assert_eq!(session.open("clip.duk"), 1);
    let decoder = unsafe { &*session.storage.decoder() };
    assert_eq!(decoder.frequency, 22_050);
    assert_eq!(decoder.format, STEREO16);
    assert!(!decoder.is_null);
    assert!(!decoder.need_swap);
    // Length counts video frames at the movie rate, not audio samples.
    let expected = FRAMES as f32 / crate::video::DUCK_FPS;
    assert!(
        (decoder.length - expected).abs() < 1e-3,
        "length {} expected {}",
        decoder.length,
        expected
    );
}

#[test]
#[serial]
fn decoding_yields_every_frames_samples_then_end_of_stream() {
    let mut session = three_frame_session();
    assert_eq!(session.open("clip.duk"), 1);
    let pcm = session.decode_all();
    // Each ADPCM byte holds two 16-bit samples.
    assert_eq!(pcm.len(), FRAMES * BYTES_PER_FRAME * 2 * 2);
    assert!(
        pcm.iter().any(|&b| b != 0),
        "non-zero ADPCM gives audible PCM"
    );
    let mut spare = [0u8; 64];
    assert_eq!(
        (funcs().Decode)(
            session.storage.decoder(),
            spare.as_mut_ptr() as *mut c_void,
            64
        ),
        0,
        "end of stream reads as zero bytes"
    );
}

#[test]
#[serial]
fn seeking_to_the_start_replays_identical_audio() {
    let mut session = three_frame_session();
    session.open("clip.duk");
    let first = session.decode_all();
    assert_eq!(session.seek(0), 0);
    assert_eq!(session.decode_all(), first);
}

#[test]
#[serial]
fn seeking_lands_on_frame_boundaries_and_ignores_targets_past_the_end() {
    let mut session = three_frame_session();
    session.open("clip.duk");
    let per_frame = BYTES_PER_FRAME as u32;
    assert_eq!(
        session.seek(per_frame + 3),
        per_frame,
        "rounds down to a frame"
    );
    let tail = session.decode_all();
    assert_eq!(tail.len(), 2 * BYTES_PER_FRAME * 4);

    assert_eq!(session.seek(0), 0);
    assert_eq!(session.seek(100 * per_frame), 0, "stays where it was");
}

#[test]
#[serial]
fn the_frame_counter_follows_playback() {
    let mut session = three_frame_session();
    session.open("clip.duk");
    assert_eq!(session.frame(), 0);
    let mut one_frame = vec![0u8; BYTES_PER_FRAME * 4];
    let n = (funcs().Decode)(
        session.storage.decoder(),
        one_frame.as_mut_ptr() as *mut c_void,
        one_frame.len() as i32,
    );
    assert_eq!(n as usize, one_frame.len());
    assert_eq!(session.frame(), 1);
    session.decode_all();
    assert_eq!(session.frame(), FRAMES as u32);
}

#[test]
#[serial]
fn opening_needs_both_the_audio_file_and_its_frame_table() {
    let (duk, frm) = duk_and_frm(FRAMES);
    let mut session = Session::new(&[("only.duk", &duk), ("table.frm", &frm)]);
    assert_eq!(session.open("only.duk"), 0, "no only.frm next to it");
    assert_eq!(session.open("table.duk"), 0, "no table.duk to read");
    assert_eq!(session.open("absent.duk"), 0);
    assert_eq!((funcs().GetError)(session.storage.decoder()), 0);
}

#[test]
#[serial]
fn opening_refuses_audio_without_the_subframe_magic() {
    let (mut duk, frm) = duk_and_frm(FRAMES);
    duk[8] = 0;
    duk[9] = 0;
    let mut session = Session::new(&[("bad.duk", &duk), ("bad.frm", &frm)]);
    assert_eq!(session.open("bad.duk"), 0);
}

#[test]
#[serial]
fn calls_with_missing_pointers_are_refused_not_followed() {
    let mut session = three_frame_session();
    session.open("clip.duk");
    let name = std::ffi::CString::new("clip.duk").unwrap();
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
        (funcs().Decode)(ptr::null_mut(), buf.as_mut_ptr() as *mut c_void, 16),
        -1
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
    assert_eq!(
        (funcs().Decode)(
            session.storage.decoder(),
            buf.as_mut_ptr() as *mut c_void,
            -4
        ),
        -1
    );
    assert_eq!((funcs().Seek)(ptr::null_mut(), 5), 0);
    assert_eq!((funcs().GetFrame)(ptr::null_mut()), 0);
    assert_eq!((funcs().GetError)(ptr::null_mut()), -1);
    (funcs().Close)(ptr::null_mut());
    (funcs().Term)(ptr::null_mut());
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

#[test]
#[serial]
fn closing_stops_audio_until_the_clip_is_opened_again() {
    let mut session = three_frame_session();
    session.open("clip.duk");
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
    assert_eq!(session.open("clip.duk"), 1);
    assert_eq!(session.decode_all(), first);
}
