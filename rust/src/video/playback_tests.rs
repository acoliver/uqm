//! Decoding and playback state of the shipped logo movie, using the real
//! .hdr/.tbl/.frm/.duk files from the content tree. Frames are rendered with
//! no destination surface, because writing pixels goes through the legacy C
//! canvas lock; the decode, clock and state handling are real.

use std::path::{Path, PathBuf};
use std::ptr;
use std::time::{Duration, Instant};

use super::{blit_frame_to_sdl_surface, PlaybackState, VideoPlayer};
use crate::graphics::ffi::{SDL_Rect, SDL_Surface};
use crate::video::decoder::DukVideoDecoder;
use crate::video::{VideoError, VideoFrame, DUCK_FPS};

const LOGO_FRAMES: u32 = 231;

fn logo_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content/addons/3dovideo/logo")
}

fn logo() -> DukVideoDecoder {
    DukVideoDecoder::open(&logo_dir(), "logo").expect("shipped logo movie opens")
}

fn player() -> VideoPlayer {
    VideoPlayer::new(logo())
}

#[test]
fn the_logo_movie_reports_its_frame_count_duration_and_size() {
    let decoder = logo();
    assert_eq!(decoder.frame_count(), LOGO_FRAMES);
    assert!((decoder.duration() - LOGO_FRAMES as f32 / DUCK_FPS).abs() < 1e-4);
    assert!(decoder.width() > 0 && decoder.height() > 0);
    assert_eq!(decoder.width() % 4, 0, "block aligned width");
    assert_eq!(decoder.height() % 4, 0, "block aligned height");
}

#[test]
fn every_logo_frame_decodes_to_a_full_picture_and_one_past_the_end_is_eof() {
    let mut decoder = logo();
    let (width, height) = (decoder.width(), decoder.height());
    let mut first_pixels = None;
    let mut changed = false;
    for index in 0..LOGO_FRAMES {
        let frame = decoder
            .decode_frame(index)
            .unwrap_or_else(|e| panic!("frame {index}: {e}"));
        assert_eq!((frame.width, frame.height), (width, height));
        assert_eq!(frame.data.len(), (width * height) as usize);
        assert_eq!(decoder.current_frame(), index);
        match &first_pixels {
            None => first_pixels = Some(frame.data.clone()),
            Some(first) => changed |= *first != frame.data,
        }
    }
    assert!(changed, "the logo animates");
    assert!(matches!(
        decoder.decode_frame(LOGO_FRAMES),
        Err(VideoError::Eof)
    ));
}

#[test]
fn decoding_the_same_frame_twice_in_sequence_gives_the_same_picture() {
    let mut decoder = logo();
    let a = decoder.decode_frame(0).unwrap();
    let b = decoder.decode_frame(0).unwrap();
    assert_eq!(a.data, b.data);
}

#[test]
fn a_new_player_is_stopped_at_the_start() {
    let player = player();
    assert_eq!(player.state(), PlaybackState::Stopped);
    assert!(!player.playing());
    assert_eq!(player.position_ms(), 0);
    assert!(!player.direct_window_mode());
}

#[test]
fn play_pause_resume_and_stop_walk_through_the_states() {
    let mut player = player();
    player.play();
    assert_eq!(player.state(), PlaybackState::Playing);
    assert!(player.playing());

    player.pause();
    assert_eq!(player.state(), PlaybackState::Paused);
    assert!(!player.playing());
    player.pause();
    assert_eq!(
        player.state(),
        PlaybackState::Paused,
        "pausing again changes nothing"
    );

    player.play();
    assert_eq!(player.state(), PlaybackState::Playing);
    player.play();
    assert_eq!(
        player.state(),
        PlaybackState::Playing,
        "play while playing is a no-op"
    );

    player.stop();
    assert_eq!(player.state(), PlaybackState::Stopped);
    assert_eq!(player.position_ms(), 0);
}

#[test]
fn pausing_a_stopped_player_does_nothing() {
    let mut player = player();
    player.pause();
    assert_eq!(player.state(), PlaybackState::Stopped);
}

#[test]
fn direct_window_mode_is_a_plain_switch() {
    let mut player = player();
    player.set_direct_window_mode(true);
    assert!(player.direct_window_mode());
    player.set_direct_window_mode(false);
    assert!(!player.direct_window_mode());
}

fn started_ago(seconds: u64) -> VideoPlayer {
    let mut player = player();
    player.play();
    player.start_time = Some(Instant::now() - Duration::from_secs(seconds));
    player
}

#[test]
fn a_stopped_player_has_nothing_to_render_and_a_paused_one_is_still_alive() {
    let mut player = player();
    assert!(!unsafe { player.process_frame(ptr::null_mut()) });
    assert_eq!(player.state(), PlaybackState::Stopped);

    player.play();
    player.pause();
    assert!(unsafe { player.process_frame(ptr::null_mut()) });
    assert_eq!(player.state(), PlaybackState::Paused);
    assert_eq!(
        player.last_rendered_frame, None,
        "a paused player draws nothing"
    );
}

#[test]
fn the_first_frame_is_rendered_at_once_and_the_next_is_scheduled() {
    let mut player = player();
    player.play();

    assert!(unsafe { player.process_frame(ptr::null_mut()) });

    assert_eq!(player.current_frame, 0);
    assert_eq!(player.last_rendered_frame, Some(0));
    assert!(player.position_ms() < 500);
    let due = player.next_frame_due.expect("the next frame is scheduled");
    assert!(due > Instant::now() - Duration::from_millis(1));
}

#[test]
fn calling_again_before_the_next_frame_is_due_renders_nothing() {
    let mut player = player();
    player.play();
    assert!(unsafe { player.process_frame(ptr::null_mut()) });
    player.next_frame_due = Some(Instant::now() + Duration::from_secs(60));
    let before = (
        player.current_frame,
        player.last_rendered_frame,
        player.pos_ms,
    );

    assert!(unsafe { player.process_frame(ptr::null_mut()) });

    assert_eq!(
        (
            player.current_frame,
            player.last_rendered_frame,
            player.pos_ms
        ),
        before
    );
}

#[test]
fn later_frames_follow_the_wall_clock() {
    let mut player = started_ago(2);

    assert!(unsafe { player.process_frame(ptr::null_mut()) });

    let expected = (2.0 * DUCK_FPS) as u32;
    assert!(
        player.current_frame >= expected && player.current_frame < expected + 24,
        "frame {} should be about two seconds in",
        player.current_frame
    );
    assert_eq!(player.last_rendered_frame, Some(player.current_frame));
    assert!(player.position_ms() >= 2000);
}

#[test]
fn running_past_the_last_frame_finishes_the_movie() {
    let mut player = started_ago(3600);

    assert!(!unsafe { player.process_frame(ptr::null_mut()) });

    assert_eq!(player.state(), PlaybackState::Finished);
    assert!(!player.playing());
    assert!(!unsafe { player.process_frame(ptr::null_mut()) });
}

#[test]
fn a_looping_movie_wraps_back_to_its_first_frame() {
    let mut player = started_ago(3600);
    player.set_loop(true);
    player.pause_duration = 5.0;

    assert!(unsafe { player.process_frame(ptr::null_mut()) });

    assert_eq!(player.state(), PlaybackState::Playing);
    assert_eq!(player.current_frame, 0);
    assert_eq!(player.last_rendered_frame, Some(0));
    assert_eq!(
        player.pause_duration, 0.0,
        "the pause bookkeeping starts over"
    );
}

#[test]
fn a_finished_movie_can_be_played_again_from_the_start() {
    let mut player = started_ago(3600);
    assert!(!unsafe { player.process_frame(ptr::null_mut()) });
    assert_eq!(player.state(), PlaybackState::Finished);

    player.play();

    assert_eq!(player.state(), PlaybackState::Playing);
    assert_eq!(player.current_frame, 0);
    assert_eq!(player.last_rendered_frame, None);
    assert!(unsafe { player.process_frame(ptr::null_mut()) });
    assert_eq!(player.last_rendered_frame, Some(0));
}

#[test]
fn direct_window_presentation_without_a_window_ends_playback() {
    let mut player = player();
    player.set_direct_window_mode(true);
    player.play();

    assert!(!unsafe { player.process_frame(ptr::null_mut()) });

    assert_eq!(player.state(), PlaybackState::Finished);
    assert_eq!(player.last_rendered_frame, None, "the frame was not shown");
}

#[test]
fn direct_window_presentation_scales_the_frame_before_presenting() {
    let mut player = player();
    player.set_direct_window_mode(true);
    player.set_xbrz_factor(2);
    player.play();

    // Presentation still has no window to go to, but the scaling stage runs
    // first and must not disturb the player.
    assert!(!unsafe { player.process_frame(ptr::null_mut()) });
    assert_eq!(player.state(), PlaybackState::Finished);
    assert_eq!(player.xbrz_factor, 2);
}

#[test]
fn a_video_scaler_is_applied_on_the_surface_path() {
    let decoder = logo();
    let (w, h) = (decoder.width(), decoder.height());
    let mut player = VideoPlayer::new(decoder);
    player.set_scaler(Some(crate::video::scaler::VideoScaler::new(
        w,
        h,
        w * 2,
        h * 2,
    )));
    player.set_position(-4, 7);
    player.play();

    assert!(unsafe { player.process_frame(ptr::null_mut()) });

    assert_eq!(player.last_rendered_frame, Some(0));
    assert_eq!((player.dst_x, player.dst_y), (-4, 7));

    player.set_scaler(None);
    player.set_lanczos_scaler(None);
    assert!(player.scaler.is_none() && player.lanczos_scaler.is_none());
}

#[test]
fn the_playback_clock_depends_on_the_state() {
    let mut player = player();
    assert_eq!(player.current_time_seconds(), 0.0, "stopped");

    let start = Instant::now() - Duration::from_secs(10);
    player.state = PlaybackState::Paused;
    player.start_time = Some(start);
    player.pause_time = Some(start + Duration::from_secs(4));
    player.pause_duration = 1.5;
    assert!(
        (player.current_time_seconds() - 2.5).abs() < 1e-3,
        "paused time is the time before the pause less earlier pauses"
    );

    player.pause_time = None;
    player.current_frame = 48;
    assert!(
        (player.current_time_seconds() - 48.0 / DUCK_FPS).abs() < 1e-3,
        "a pause with no timestamps falls back to the frame position"
    );

    player.state = PlaybackState::Finished;
    assert!((player.current_time_seconds() - LOGO_FRAMES as f32 / DUCK_FPS).abs() < 1e-3);

    player.state = PlaybackState::Playing;
    player.start_time = None;
    assert_eq!(
        player.current_time_seconds(),
        0.0,
        "playing but never started"
    );
}

#[test]
fn resuming_adds_the_time_spent_paused() {
    let mut player = player();
    player.play();
    player.pause();
    player.pause_time = Some(Instant::now() - Duration::from_secs(3));

    player.play();

    assert_eq!(player.state(), PlaybackState::Playing);
    assert!(player.pause_duration >= 3.0);
    assert!(player.pause_time.is_none());
}

#[test]
fn stopping_a_rendered_movie_clears_its_progress() {
    let mut player = started_ago(2);
    assert!(unsafe { player.process_frame(ptr::null_mut()) });
    assert!(player.position_ms() >= 2000);

    player.stop();

    assert_eq!(player.state(), PlaybackState::Stopped);
    assert_eq!(player.position_ms(), 0);
    assert_eq!(player.current_frame, 0);
    assert_eq!(player.last_rendered_frame, None);
    assert!(player.next_frame_due.is_none() && player.start_time.is_none());
}

/// A memory-backed 32-bit surface that owns its pixels.
struct Canvas {
    pixels: Vec<u32>,
    surface: SDL_Surface,
}

impl Canvas {
    fn new(width: i32, height: i32, pitch_pixels: i32) -> Box<Self> {
        let mut canvas = Box::new(Canvas {
            pixels: vec![0xDEAD_BEEF; (pitch_pixels * height) as usize],
            surface: SDL_Surface {
                flags: 0,
                format: ptr::null_mut(),
                w: width,
                h: height,
                pitch: pitch_pixels * 4,
                pixels: ptr::null_mut(),
                userdata: ptr::null_mut(),
                locked: 0,
                list_blitmap: ptr::null_mut(),
                clip_rect: SDL_Rect::default(),
                map: ptr::null_mut(),
                refcount: 1,
            },
        });
        canvas.surface.pixels = canvas.pixels.as_mut_ptr() as *mut std::ffi::c_void;
        canvas
    }

    fn at(&self, x: usize, y: usize) -> u32 {
        self.pixels[y * (self.surface.pitch as usize / 4) + x]
    }
}

fn frame(width: u32, height: u32, pixel: impl Fn(u32, u32) -> u32) -> VideoFrame {
    let mut data = Vec::new();
    for y in 0..height {
        for x in 0..width {
            data.push(pixel(x, y));
        }
    }
    VideoFrame {
        width,
        height,
        data,
        timestamp: 0.0,
    }
}

#[test]
fn blitting_repacks_frame_pixels_for_the_screen_format() {
    let mut canvas = Canvas::new(6, 4, 6);
    // Frame pixels carry red in the low byte; the screen wants it on top.
    let picture = frame(2, 2, |x, y| 0x00_30_20_10 + x * 0x100_0000 + y);

    unsafe { blit_frame_to_sdl_surface(&mut canvas.surface, &picture, 1, 1) };

    assert_eq!(canvas.at(1, 1), 0x10_20_30_00, "red, green, blue move up");
    assert_eq!(canvas.at(2, 1), 0x10_20_30_00);
    assert_eq!(
        canvas.at(1, 2),
        0x11_20_30_00,
        "the low byte is red (row 1 adds 1)"
    );
    assert_eq!(
        canvas.at(0, 0),
        0xDEAD_BEEF,
        "outside the picture is untouched"
    );
    assert_eq!(canvas.at(3, 1), 0xDEAD_BEEF);
}

#[test]
fn blitting_clips_the_picture_to_the_surface() {
    let mut canvas = Canvas::new(4, 3, 4);
    let picture = frame(4, 4, |x, y| y * 16 + x + 1);

    // Starting up and to the left hides the first row and column.
    unsafe { blit_frame_to_sdl_surface(&mut canvas.surface, &picture, -1, -1) };

    assert_eq!(
        canvas.at(0, 0),
        (16 + 1 + 1) << 24,
        "source (1,1) lands at (0,0)"
    );
    assert_eq!(canvas.at(2, 1), (2 * 16 + 3 + 1) << 24);
    assert_eq!(canvas.at(3, 2), 0xDEAD_BEEF, "source (4,3) does not exist");
}

#[test]
fn blitting_past_the_edges_writes_nothing() {
    let mut canvas = Canvas::new(4, 3, 4);
    let picture = frame(2, 2, |_, _| 0x0000_00FF);

    unsafe { blit_frame_to_sdl_surface(&mut canvas.surface, &picture, 4, 0) };
    unsafe { blit_frame_to_sdl_surface(&mut canvas.surface, &picture, 0, 3) };
    unsafe { blit_frame_to_sdl_surface(&mut canvas.surface, &picture, -2, 0) };
    unsafe { blit_frame_to_sdl_surface(&mut canvas.surface, &picture, 0, -2) };

    assert!(canvas.pixels.iter().all(|&p| p == 0xDEAD_BEEF));
}

#[test]
fn blitting_refuses_surfaces_it_cannot_write() {
    let picture = frame(2, 2, |_, _| 0x0000_00FF);

    // Pitch narrower than a row of 32-bit pixels (a 16-bit style surface).
    let mut narrow = Canvas::new(4, 3, 4);
    narrow.surface.pitch = 4 * 2;
    unsafe { blit_frame_to_sdl_surface(&mut narrow.surface, &picture, 0, 0) };
    assert!(narrow.pixels.iter().all(|&p| p == 0xDEAD_BEEF));

    // No pixel storage.
    let mut hollow = Canvas::new(4, 3, 4);
    hollow.surface.pixels = ptr::null_mut();
    unsafe { blit_frame_to_sdl_surface(&mut hollow.surface, &picture, 0, 0) };
    assert!(hollow.pixels.iter().all(|&p| p == 0xDEAD_BEEF));

    // Degenerate size.
    let mut empty = Canvas::new(0, 3, 4);
    unsafe { blit_frame_to_sdl_surface(&mut empty.surface, &picture, 0, 0) };
    assert!(empty.pixels.iter().all(|&p| p == 0xDEAD_BEEF));
}

#[test]
fn a_rendered_logo_frame_lands_on_the_surface_in_screen_format() {
    let mut decoder = logo();
    let first = decoder.decode_frame(0).unwrap();
    let (w, h) = (first.width as i32, first.height as i32);
    let mut canvas = Canvas::new(w + 8, h + 8, w + 8);
    let mut player = VideoPlayer::new(decoder);
    player.set_position(4, 4);
    player.play();

    assert!(unsafe { player.process_frame(&mut canvas.surface) });

    for &(x, y) in &[
        (0usize, 0usize),
        (w as usize / 2, h as usize / 2),
        (w as usize - 1, h as usize - 1),
    ] {
        let p = first.data[y * w as usize + x];
        let expected = ((p & 0xFF) << 24) | (((p >> 8) & 0xFF) << 16) | (((p >> 16) & 0xFF) << 8);
        assert_eq!(canvas.at(x + 4, y + 4), expected, "pixel ({x}, {y})");
    }
    assert_eq!(
        canvas.at(0, 0),
        0xDEAD_BEEF,
        "the margin keeps its old contents"
    );
}
