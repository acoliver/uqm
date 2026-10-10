//! Decoding and playback state of the shipped logo movie, using the real
//! .hdr/.tbl/.frm/.duk files from the content tree. Frame rendering is not
//! exercised here because it goes through the legacy C canvas lock.

use std::path::{Path, PathBuf};

use super::{PlaybackState, VideoPlayer};
use crate::video::decoder::DukVideoDecoder;
use crate::video::{VideoError, DUCK_FPS};

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
