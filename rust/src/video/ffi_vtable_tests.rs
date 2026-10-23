//! The DukVid decoder vtable and the Rust video player entry points driven the
//! way the C video layer drives them: the shipped logo movie behind a real uio
//! mount, frames delivered through caller-supplied canvas callbacks.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

use serial_test::serial;

use super::decoder::DukVideoDecoder;
use super::ffi::{
    rust_get_video_position, rust_play_video, rust_play_video_direct_window, rust_stop_video,
    rust_video_playing, TFB_PixelFormat, TFB_VideoDecoder, TFB_VideoDecoderFuncs,
    RUST_DUKV_DECODER_VTBL,
};
use super::DUCK_FPS;
use crate::io::ffi::uio_DirHandle;
use crate::sound::decoder_vtable_tests::Directory;

/// The sound tests' uio handle type is a distinct declaration of the same C
/// handle the video layer takes.
trait IoHandle {
    fn io_handle(&self) -> *mut uio_DirHandle;
}

impl IoHandle for Directory {
    fn io_handle(&self) -> *mut uio_DirHandle {
        self.handle() as *mut uio_DirHandle
    }
}

const LOGO_FRAMES: u32 = 231;

fn logo_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content/addons/3dovideo/logo")
}

fn funcs() -> &'static TFB_VideoDecoderFuncs {
    &RUST_DUKV_DECODER_VTBL
}

fn logo_directory() -> Directory {
    let directory = Directory::new();
    for ext in ["duk", "frm", "hdr", "tbl"] {
        let bytes = std::fs::read(logo_dir().join(format!("logo.{ext}"))).unwrap();
        directory.write(&format!("logo.{ext}"), &bytes);
    }
    directory
}

/// Channels laid out as the decoder's own RGBA words: the canvas then equals
/// the decoded picture exactly.
const NATIVE_RGBA: TFB_PixelFormat = TFB_PixelFormat {
    BitsPerPixel: 32,
    BytesPerPixel: 4,
    Rmask: 0xff,
    Gmask: 0xff00,
    Bmask: 0xff0000,
    Amask: 0xff00_0000,
    Rshift: 0,
    Gshift: 8,
    Bshift: 16,
    Ashift: 24,
    Rloss: 0,
    Gloss: 0,
    Bloss: 0,
    Aloss: 0,
};

const RGB565: TFB_PixelFormat = TFB_PixelFormat {
    BitsPerPixel: 16,
    BytesPerPixel: 2,
    Rmask: 0xf800,
    Gmask: 0x07e0,
    Bmask: 0x001f,
    Amask: 0,
    Rshift: 11,
    Gshift: 5,
    Bshift: 0,
    Ashift: 0,
    Rloss: 3,
    Gloss: 2,
    Bloss: 3,
    Aloss: 8,
};

/// What the callbacks saw; reached through the decoder's `data` pointer.
struct Canvas {
    pixels: Vec<u32>,
    row_bytes: usize,
    refuse_rows: bool,
    begin: u32,
    end: u32,
    timers: Vec<u32>,
    order: Vec<&'static str>,
}

unsafe fn canvas<'a>(decoder: *mut TFB_VideoDecoder) -> &'a mut Canvas {
    &mut *((*decoder).data as *mut Canvas)
}

unsafe extern "C" fn begin_frame(decoder: *mut TFB_VideoDecoder) {
    let c = canvas(decoder);
    c.begin += 1;
    c.order.push("begin");
}

unsafe extern "C" fn end_frame(decoder: *mut TFB_VideoDecoder) {
    let c = canvas(decoder);
    c.end += 1;
    c.order.push("end");
}

unsafe extern "C" fn canvas_line(decoder: *mut TFB_VideoDecoder, y: u32) -> *mut c_void {
    let c = canvas(decoder);
    if c.refuse_rows {
        return ptr::null_mut();
    }
    c.order.push("line");
    (c.pixels.as_mut_ptr() as *mut u8).add(y as usize * c.row_bytes) as *mut c_void
}

unsafe extern "C" fn set_timer(decoder: *mut TFB_VideoDecoder, ms: u32) -> bool {
    canvas(decoder).timers.push(ms);
    true
}

/// A decoder instance wired to a test canvas, opened on the logo movie.
struct Session {
    words: Vec<u64>,
    canvas: Box<Canvas>,
    format: Box<TFB_PixelFormat>,
    _directory: Directory,
}

impl Session {
    fn raw(&mut self) -> *mut TFB_VideoDecoder {
        self.words.as_mut_ptr() as *mut TFB_VideoDecoder
    }

    fn unopened(format: TFB_PixelFormat) -> Self {
        let bytes = (funcs().GetStructSize)() as usize;
        let mut session = Session {
            words: vec![0u64; bytes.div_ceil(8)],
            canvas: Box::new(Canvas {
                pixels: Vec::new(),
                row_bytes: 0,
                refuse_rows: false,
                begin: 0,
                end: 0,
                timers: Vec::new(),
                order: Vec::new(),
            }),
            format: Box::new(format),
            _directory: logo_directory(),
        };
        assert!((funcs().InitModule)(0));
        let decoder = session.raw();
        let fmt = &mut *session.format as *mut TFB_PixelFormat;
        assert!((funcs().Init)(decoder, fmt));
        session.install_callbacks();
        session
    }

    fn install_callbacks(&mut self) {
        let decoder = self.raw();
        let canvas = &mut *self.canvas as *mut Canvas as *mut c_void;
        unsafe {
            (*decoder).data = canvas;
            (*decoder).callbacks.BeginFrame = Some(begin_frame);
            (*decoder).callbacks.EndFrame = Some(end_frame);
            (*decoder).callbacks.GetCanvasLine = Some(canvas_line);
            (*decoder).callbacks.SetTimer = Some(set_timer);
        }
    }

    fn open(format: TFB_PixelFormat) -> Self {
        let mut session = Self::unopened(format);
        let name = CString::new("logo").unwrap();
        let decoder = session.raw();
        let dir = session._directory.io_handle();
        assert!((funcs().Open)(decoder, dir, name.as_ptr()));
        let (w, h) = unsafe { ((*decoder).w as usize, (*decoder).h as usize) };
        session.canvas.row_bytes = w * format.BytesPerPixel as usize;
        session.canvas.pixels = vec![0u32; (w * h * format.BytesPerPixel as usize).div_ceil(4)];
        session
    }

    fn decode_next(&mut self) -> i32 {
        (funcs().DecodeNext)(self.raw())
    }

    fn cur_frame(&mut self) -> u32 {
        (funcs().GetFrame)(self.raw())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let decoder = self.raw();
        (funcs().Close)(decoder);
        (funcs().TermModule)();
    }
}

fn reference_frame(index: u32) -> Vec<u32> {
    let mut decoder = DukVideoDecoder::open(&logo_dir(), "logo").unwrap();
    decoder.decode_frame(index).unwrap().data
}

#[test]
#[serial]
fn the_vtable_names_itself_and_sizes_its_wrapper() {
    let name = unsafe { CStr::from_ptr((funcs().GetName)()) };
    assert_eq!(name.to_str().unwrap(), "Rust DukVid");
    assert!(
        (funcs().GetStructSize)() as usize >= std::mem::size_of::<TFB_VideoDecoder>(),
        "the wrapper embeds the base decoder"
    );
    assert!((funcs().InitModule)(0));
    (funcs().TermModule)();
}

#[test]
#[serial]
fn null_decoders_are_refused_by_every_entry_point() {
    let null = ptr::null_mut::<TFB_VideoDecoder>();
    let name = CString::new("logo").unwrap();
    assert_eq!((funcs().GetError)(null), -1);
    assert!(!(funcs().Init)(null, ptr::null_mut()));
    assert!(!(funcs().Open)(null, ptr::null_mut(), name.as_ptr()));
    assert_eq!((funcs().DecodeNext)(null), -1);
    assert_eq!((funcs().SeekFrame)(null, 5), 0);
    assert_eq!((funcs().SeekTime)(null, 1.0), 0.0);
    assert_eq!((funcs().GetFrame)(null), 0);
    assert_eq!((funcs().GetTime)(null), 0.0);
    (funcs().Term)(null);
    (funcs().Close)(null);
}

#[test]
#[serial]
fn opening_the_logo_reports_its_geometry_and_frame_timing() {
    let mut session = Session::open(NATIVE_RGBA);
    let reference = DukVideoDecoder::open(&logo_dir(), "logo").unwrap();
    let decoder = session.raw();
    unsafe {
        assert_eq!((*decoder).w, reference.width());
        assert_eq!((*decoder).h, reference.height());
        assert_eq!((*decoder).frame_count, LOGO_FRAMES);
        assert!(((*decoder).length - LOGO_FRAMES as f32 / DUCK_FPS).abs() < 1e-4);
        assert_eq!((*decoder).interframe_wait, (1000.0 / DUCK_FPS) as u32);
        assert!(!(*decoder).audio_synced);
        assert_eq!((*decoder).pos, 0.0);
        assert_eq!((*decoder).cur_frame, 0);
        assert_eq!((*decoder).error, 0);
        assert_eq!((*decoder).dir, session._directory.io_handle());
    }
    assert_eq!((funcs().GetError)(decoder), 0);
    assert_eq!(session.cur_frame(), 0);
    assert_eq!((funcs().GetTime)(decoder), 0.0);
}

#[test]
#[serial]
fn init_remembers_the_pixel_format_it_was_given() {
    let mut session = Session::unopened(RGB565);
    let decoder = session.raw();
    let fmt = &*session.format as *const TFB_PixelFormat;
    assert_eq!(unsafe { (*decoder).format }, fmt);
    assert_eq!((funcs().GetError)(decoder), 0);
}

#[test]
#[serial]
fn init_without_a_format_leaves_the_decoder_formatless_but_usable() {
    let bytes = (funcs().GetStructSize)() as usize;
    let mut words = vec![0u64; bytes.div_ceil(8)];
    let decoder = words.as_mut_ptr() as *mut TFB_VideoDecoder;
    assert!((funcs().Init)(decoder, ptr::null_mut()));
    unsafe { assert!((*decoder).format.is_null()) };
    assert_eq!((funcs().GetError)(decoder), 0);
    (funcs().Term)(decoder);
}

#[test]
#[serial]
fn a_suffixed_name_opens_the_same_movie() {
    let mut session = Session::unopened(NATIVE_RGBA);
    let decoder = session.raw();
    let name = CString::new("logo.duk").unwrap();
    assert!((funcs().Open)(
        decoder,
        session._directory.io_handle(),
        name.as_ptr()
    ));
    unsafe { assert_eq!((*decoder).frame_count, LOGO_FRAMES) };
}

#[test]
#[serial]
fn opening_a_missing_movie_fails_and_records_the_error() {
    let mut session = Session::unopened(NATIVE_RGBA);
    let decoder = session.raw();
    let name = CString::new("nothing_here").unwrap();
    assert!(!(funcs().Open)(
        decoder,
        session._directory.io_handle(),
        name.as_ptr()
    ));
    assert_eq!((funcs().GetError)(decoder), -1);
    assert_eq!(
        (funcs().DecodeNext)(decoder),
        -1,
        "no movie, nothing to decode"
    );
}

#[test]
#[serial]
fn a_missing_companion_file_fails_the_open() {
    for missing in ["duk", "frm", "hdr", "tbl"] {
        let mut session = Session::unopened(NATIVE_RGBA);
        let decoder = session.raw();
        std::fs::remove_file(session._directory.path_of(&format!("logo.{missing}"))).unwrap();
        let name = CString::new("logo").unwrap();
        assert!(
            !(funcs().Open)(decoder, session._directory.io_handle(), name.as_ptr()),
            "without logo.{missing}"
        );
        assert_eq!((funcs().GetError)(decoder), -1, "without logo.{missing}");
    }
}

#[test]
#[serial]
fn a_corrupt_header_is_reported_as_a_bad_file() {
    let mut session = Session::unopened(NATIVE_RGBA);
    session._directory.write("logo.hdr", &[1, 2, 3]);
    let decoder = session.raw();
    let name = CString::new("logo").unwrap();
    assert!(!(funcs().Open)(
        decoder,
        session._directory.io_handle(),
        name.as_ptr()
    ));
    assert_eq!((funcs().GetError)(decoder), -2);
}

#[test]
#[serial]
fn a_frame_table_that_is_not_whole_words_is_a_bad_file() {
    let mut session = Session::unopened(NATIVE_RGBA);
    session._directory.write("logo.frm", &[0, 0, 0, 0, 0]);
    let decoder = session.raw();
    let name = CString::new("logo").unwrap();
    assert!(!(funcs().Open)(
        decoder,
        session._directory.io_handle(),
        name.as_ptr()
    ));
    assert_eq!((funcs().GetError)(decoder), -2);
}

#[test]
#[serial]
fn a_truncated_vector_table_is_a_bad_file() {
    let mut session = Session::unopened(NATIVE_RGBA);
    session._directory.write("logo.tbl", &[0u8; 16]);
    let decoder = session.raw();
    let name = CString::new("logo").unwrap();
    assert!(!(funcs().Open)(
        decoder,
        session._directory.io_handle(),
        name.as_ptr()
    ));
    assert_eq!((funcs().GetError)(decoder), -2);
}

#[test]
#[serial]
fn a_name_that_is_not_utf8_is_refused() {
    let mut session = Session::unopened(NATIVE_RGBA);
    let decoder = session.raw();
    let bad = [0xffu8, 0xfe, 0];
    assert!(!(funcs().Open)(
        decoder,
        session._directory.io_handle(),
        bad.as_ptr() as *const c_char
    ));
    assert_eq!((funcs().GetError)(decoder), -1);
    assert!(!(funcs().Open)(
        decoder,
        session._directory.io_handle(),
        ptr::null()
    ));
}

#[test]
#[serial]
fn decoding_without_canvas_callbacks_still_advances_the_clock() {
    let mut session = Session::open(NATIVE_RGBA);
    let decoder = session.raw();
    unsafe {
        (*decoder).callbacks.GetCanvasLine = None;
        (*decoder).callbacks.BeginFrame = None;
        (*decoder).callbacks.EndFrame = None;
        (*decoder).callbacks.SetTimer = None;
    }
    assert_eq!(session.decode_next(), 1);
    assert_eq!(session.cur_frame(), 1);
    assert!(
        session.canvas.pixels.iter().all(|&p| p == 0),
        "nothing was drawn"
    );
}

#[test]
#[serial]
fn a_decoded_frame_lands_on_the_canvas_pixel_for_pixel() {
    let mut session = Session::open(NATIVE_RGBA);
    assert_eq!(session.decode_next(), 1);
    assert_eq!(session.canvas.pixels, reference_frame(0));
    (funcs().SeekFrame)(session.raw(), 150);
    assert_eq!(session.decode_next(), 1);
    let reference = reference_frame(150);
    assert_eq!(session.canvas.pixels, reference);
    let distinct: std::collections::HashSet<u32> = reference.iter().copied().collect();
    assert!(distinct.len() > 1, "the picture is not a flat colour");
}

#[test]
#[serial]
fn each_frame_is_bracketed_by_begin_and_end_and_followed_by_the_timer() {
    let mut session = Session::open(NATIVE_RGBA);
    let height = unsafe { (*session.raw()).h } as usize;
    assert_eq!(session.decode_next(), 1);
    assert_eq!(session.decode_next(), 1);
    assert_eq!((session.canvas.begin, session.canvas.end), (2, 2));
    assert_eq!(session.canvas.order.first(), Some(&"begin"));
    assert_eq!(session.canvas.order[1 + height], "end");
    assert_eq!(session.canvas.order[2 + height], "begin");
    assert_eq!(session.canvas.order.len(), 2 * (height + 2));
    let wait = (1000.0 / DUCK_FPS) as u32;
    assert_eq!(session.canvas.timers, vec![wait, wait]);
}

#[test]
#[serial]
fn decoding_advances_the_frame_counter_and_clock() {
    let mut session = Session::open(NATIVE_RGBA);
    for expected in 1..=5u32 {
        assert_eq!(session.decode_next(), 1);
        assert_eq!(session.cur_frame(), expected);
    }
    // The clock reads the start of the last frame shown.
    let time = (funcs().GetTime)(session.raw());
    assert!((time - 4.0 / DUCK_FPS).abs() < 1e-5, "time was {time}");
}

#[test]
#[serial]
fn successive_frames_differ_on_the_canvas() {
    let mut session = Session::open(NATIVE_RGBA);
    assert_eq!(session.decode_next(), 1);
    let first = session.canvas.pixels.clone();
    for _ in 0..150 {
        assert_eq!(session.decode_next(), 1);
    }
    assert_eq!(session.canvas.pixels, reference_frame(150));
    assert_ne!(session.canvas.pixels, first);
}

#[test]
#[serial]
fn sixteen_bit_canvases_receive_packed_565_pixels() {
    let mut session = Session::open(RGB565);
    assert_eq!(session.decode_next(), 1);
    let reference = reference_frame(0);
    let bytes: Vec<u8> = session
        .canvas
        .pixels
        .iter()
        .flat_map(|p| p.to_ne_bytes())
        .collect();
    for (i, pixel) in reference.iter().enumerate() {
        let (r, g, b) = (pixel & 0xff, (pixel >> 8) & 0xff, (pixel >> 16) & 0xff);
        let expected = (((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)) as u16;
        let got = u16::from_ne_bytes([bytes[2 * i], bytes[2 * i + 1]]);
        assert_eq!(got, expected, "pixel {i}");
    }
}

#[test]
#[serial]
fn a_canvas_that_refuses_rows_is_skipped_without_failing_the_frame() {
    let mut session = Session::open(NATIVE_RGBA);
    session.canvas.refuse_rows = true;
    assert_eq!(session.decode_next(), 1);
    assert_eq!(session.cur_frame(), 1);
    assert!(session.canvas.pixels.iter().all(|&p| p == 0));
}

#[test]
#[serial]
fn an_unsupported_pixel_depth_draws_nothing_but_the_movie_still_advances() {
    let odd = TFB_PixelFormat {
        BytesPerPixel: 3,
        BitsPerPixel: 24,
        ..NATIVE_RGBA
    };
    let mut session = Session::open(odd);
    session.canvas.pixels = vec![0xdead_beef; session.canvas.pixels.len()];
    assert_eq!(session.decode_next(), 1);
    assert!(session.canvas.pixels.iter().all(|&p| p == 0xdead_beef));
    assert_eq!(session.cur_frame(), 1);
}

#[test]
#[serial]
fn the_movie_ends_with_zero_and_a_looping_movie_starts_over() {
    let mut session = Session::open(NATIVE_RGBA);
    assert_eq!(
        (funcs().SeekFrame)(session.raw(), LOGO_FRAMES - 1),
        LOGO_FRAMES - 1
    );
    assert_eq!(session.decode_next(), 1, "last frame");
    assert_eq!(session.cur_frame(), LOGO_FRAMES);
    assert_eq!(session.decode_next(), 0, "past the end");
    assert_eq!(session.cur_frame(), LOGO_FRAMES, "nothing advanced");

    unsafe { (*session.raw()).looping = true };
    assert_eq!(session.decode_next(), 1);
    assert_eq!(
        session.cur_frame(),
        1,
        "wrapped to the first frame and played it"
    );
    assert_eq!(session.canvas.pixels, reference_frame(0));
}

#[test]
#[serial]
fn seeking_clamps_to_the_last_frame_and_moves_the_clock() {
    let mut session = Session::open(NATIVE_RGBA);
    let decoder = session.raw();
    assert_eq!((funcs().SeekFrame)(decoder, 100), 100);
    assert_eq!(session.cur_frame(), 100);
    assert!(((funcs().GetTime)(decoder) - 100.0 / DUCK_FPS).abs() < 1e-4);
    assert_eq!((funcs().SeekFrame)(decoder, 100_000), LOGO_FRAMES - 1);
    assert_eq!(session.cur_frame(), LOGO_FRAMES - 1);
}

#[test]
#[serial]
fn seeking_by_time_lands_on_the_frame_boundary_at_or_before_it() {
    let mut session = Session::open(NATIVE_RGBA);
    let decoder = session.raw();
    let landed = (funcs().SeekTime)(decoder, 2.0);
    let frame = (2.0 * DUCK_FPS) as u32;
    assert_eq!(session.cur_frame(), frame);
    assert!((landed - frame as f32 / DUCK_FPS).abs() < 1e-5);
    let beyond = (funcs().SeekTime)(decoder, 10_000.0);
    assert!((beyond - (LOGO_FRAMES - 1) as f32 / DUCK_FPS).abs() < 1e-4);
}

#[test]
#[serial]
fn decoding_after_a_seek_shows_the_frame_that_was_asked_for() {
    let mut session = Session::open(NATIVE_RGBA);
    (funcs().SeekFrame)(session.raw(), 57);
    assert_eq!(session.decode_next(), 1);
    assert_eq!(session.canvas.pixels, reference_frame(57));
}

#[test]
#[serial]
fn closing_twice_is_harmless_and_term_module_forgets_the_format() {
    let mut session = Session::open(NATIVE_RGBA);
    let decoder = session.raw();
    (funcs().Close)(decoder);
    (funcs().Close)(decoder);
    assert_eq!(
        (funcs().DecodeNext)(decoder),
        -1,
        "a closed decoder has no movie"
    );
    (funcs().Term)(decoder);
}

/// Stops whatever the Rust player is holding when a test ends.
struct StopOnDrop;

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        unsafe { rust_stop_video() };
    }
}

#[test]
#[serial]
fn the_player_entry_points_start_report_and_stop_a_movie() {
    let _stop = StopOnDrop;
    let directory = logo_directory();
    let name = CString::new("logo").unwrap();
    unsafe {
        assert!(!rust_video_playing());
        assert_eq!(rust_get_video_position(), 0);
        assert!(rust_play_video(
            directory.io_handle(),
            name.as_ptr(),
            4,
            8,
            false
        ));
        assert!(rust_video_playing());
        assert_eq!(rust_get_video_position(), 0);
        rust_stop_video();
        assert!(!rust_video_playing());
        assert_eq!(rust_get_video_position(), 0);
    }
}

#[test]
#[serial]
fn the_player_accepts_a_suffixed_name_and_replaces_the_current_movie() {
    let _stop = StopOnDrop;
    let directory = logo_directory();
    let plain = CString::new("logo").unwrap();
    let suffixed = CString::new("logo.duk").unwrap();
    unsafe {
        assert!(rust_play_video(
            directory.io_handle(),
            plain.as_ptr(),
            0,
            0,
            true
        ));
        assert!(rust_play_video(
            directory.io_handle(),
            suffixed.as_ptr(),
            0,
            0,
            false
        ));
        assert!(rust_video_playing());
    }
}

#[test]
#[serial]
fn the_player_refuses_missing_files_bad_names_and_corrupt_movies() {
    let _stop = StopOnDrop;
    let directory = logo_directory();
    let missing = CString::new("absent").unwrap();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        assert!(!rust_play_video(ptr::null_mut(), ptr::null(), 0, 0, false));
        assert!(!rust_play_video(
            directory.io_handle(),
            bad.as_ptr() as *const c_char,
            0,
            0,
            false
        ));
        assert!(!rust_play_video(
            directory.io_handle(),
            missing.as_ptr(),
            0,
            0,
            false
        ));
        assert!(!rust_video_playing());
        // Two mounts at "/" would shadow each other.
        drop(directory);

        for ext in ["duk", "frm", "hdr", "tbl"] {
            let dir = logo_directory();
            std::fs::remove_file(dir.path_of(&format!("logo.{ext}"))).unwrap();
            let name = CString::new("logo").unwrap();
            assert!(
                !rust_play_video(dir.io_handle(), name.as_ptr(), 0, 0, false),
                "without logo.{ext}"
            );
        }

        let dir = logo_directory();
        dir.write("logo.hdr", &[9, 9, 9]);
        let name = CString::new("logo").unwrap();
        assert!(!rust_play_video(
            dir.io_handle(),
            name.as_ptr(),
            0,
            0,
            false
        ));
        assert!(!rust_video_playing());
    }
}

#[test]
#[serial]
fn the_direct_window_player_refuses_requests_it_cannot_serve() {
    let _stop = StopOnDrop;
    let directory = logo_directory();
    let name = CString::new("logo").unwrap();
    let bad = [0xffu8, 0xfe, 0];
    unsafe {
        assert!(!rust_play_video_direct_window(
            directory.io_handle(),
            ptr::null(),
            640,
            480,
            false
        ));
        assert!(!rust_play_video_direct_window(
            directory.io_handle(),
            bad.as_ptr() as *const c_char,
            640,
            480,
            false
        ));
        // Without an initialised Rust graphics state there is no window.
        assert!(!rust_play_video_direct_window(
            directory.io_handle(),
            name.as_ptr(),
            640,
            480,
            false
        ));
        assert!(!rust_video_playing());
    }
}
