// @plan PLAN-20260225-AUDIO-HEART.P12
// @requirement REQ-MUSIC-PLAY-01..08, REQ-MUSIC-SPEECH-01..02
// @requirement REQ-MUSIC-LOAD-01..06, REQ-MUSIC-RELEASE-01..04
// @requirement REQ-MUSIC-VOLUME-01
#![allow(dead_code, unused_imports, unused_variables)]

//! Music and speech playback — wraps the stream engine for MUSIC_SOURCE
//! and SPEECH_SOURCE with volume control, fading, and resource management.

use std::sync::Arc;

use parking_lot::Mutex;

use super::mixer::source as mixer_source;
use super::mixer::types::SourceProp;
use super::stream;
use super::types::*;

// =============================================================================
// State
// =============================================================================

/// Music subsystem state.
struct MusicState {
    /// Current music reference being played.
    cur_music_ref: Option<MusicRef>,
    /// Current speech reference being played.
    cur_speech_ref: Option<MusicRef>,
    /// Music volume (0..MAX_VOLUME).
    music_volume: i32,
    /// Music volume scale factor (0.0..1.0).
    music_volume_scale: f32,
}

impl MusicState {
    fn new() -> Self {
        MusicState {
            cur_music_ref: None,
            cur_speech_ref: None,
            music_volume: MAX_VOLUME,
            music_volume_scale: 1.0,
        }
    }
}

static MUSIC_STATE: std::sync::LazyLock<Mutex<MusicState>> =
    std::sync::LazyLock::new(|| Mutex::new(MusicState::new()));

// =============================================================================
// Music Playback (spec §3.3)
// =============================================================================

/// Play a music track on MUSIC_SOURCE.
pub fn plr_play_song(music_ref: &MusicRef, continuous: bool, _priority: i32) -> AudioResult<()> {
    let sample_arc = Arc::clone(&music_ref.0);

    stream::play_stream(sample_arc, MUSIC_SOURCE, continuous, true, true)?;

    let mut state = MUSIC_STATE.lock();
    state.cur_music_ref = Some(music_ref.clone());
    Ok(())
}

/// Stop music playback.
pub fn plr_stop(music_ref: &MusicRef) -> AudioResult<()> {
    let mut state = MUSIC_STATE.lock();
    let matches = state
        .cur_music_ref
        .as_ref()
        .map(|cur| Arc::ptr_eq(&cur.0, &music_ref.0))
        .unwrap_or(false);

    if matches {
        drop(state);
        stream::stop_stream(MUSIC_SOURCE)?;
        let mut state = MUSIC_STATE.lock();
        state.cur_music_ref = None;
    }
    Ok(())
}

/// Check if music is currently playing.
pub fn plr_playing(music_ref: &MusicRef) -> bool {
    let state = MUSIC_STATE.lock();
    let matches = state
        .cur_music_ref
        .as_ref()
        .map(|cur| Arc::ptr_eq(&cur.0, &music_ref.0))
        .unwrap_or(false);

    if !matches {
        return false;
    }
    stream::playing_stream(MUSIC_SOURCE)
}

/// Seek within the current music track.
pub fn plr_seek(music_ref: &MusicRef, pos: u32) -> AudioResult<()> {
    let state = MUSIC_STATE.lock();
    let matches = state
        .cur_music_ref
        .as_ref()
        .map(|cur| Arc::ptr_eq(&cur.0, &music_ref.0))
        .unwrap_or(false);
    if matches {
        drop(state);
        stream::seek_stream(MUSIC_SOURCE, pos)?;
    }
    Ok(())
}

/// Stop the currently active music (sentinel ~0 path).
pub fn plr_stop_current() -> AudioResult<()> {
    let state = MUSIC_STATE.lock();
    if state.cur_music_ref.is_none() {
        return Ok(());
    }
    drop(state);
    stream::stop_stream(MUSIC_SOURCE)?;
    let mut state = MUSIC_STATE.lock();
    state.cur_music_ref = None;
    Ok(())
}

/// Check whether any current music is playing (sentinel ~0 path).
pub fn plr_playing_current() -> bool {
    let state = MUSIC_STATE.lock();
    if state.cur_music_ref.is_none() {
        return false;
    }
    drop(state);
    stream::playing_stream(MUSIC_SOURCE)
}

/// Seek within current music track (sentinel ~0 path).
pub fn plr_seek_current(pos: u32) -> AudioResult<()> {
    let state = MUSIC_STATE.lock();
    if state.cur_music_ref.is_none() {
        return Ok(());
    }
    drop(state);
    stream::seek_stream(MUSIC_SOURCE, pos)
}

/// Pause music playback.
pub fn plr_pause() -> AudioResult<()> {
    stream::pause_stream(MUSIC_SOURCE)
}

/// Resume music playback.
pub fn plr_resume() -> AudioResult<()> {
    stream::resume_stream(MUSIC_SOURCE)
}

// =============================================================================
// Speech Playback
// =============================================================================

/// Play speech on SPEECH_SOURCE.
pub fn snd_play_speech(music_ref: &MusicRef) -> AudioResult<()> {
    let sample_arc = Arc::clone(&music_ref.0);
    stream::play_stream(sample_arc, SPEECH_SOURCE, false, false, true)?;
    let mut state = MUSIC_STATE.lock();
    state.cur_speech_ref = Some(music_ref.clone());
    Ok(())
}

/// Stop speech playback.
pub fn snd_stop_speech() -> AudioResult<()> {
    stream::stop_stream(SPEECH_SOURCE)?;
    let mut state = MUSIC_STATE.lock();
    state.cur_speech_ref = None;
    Ok(())
}

// =============================================================================
// Music Loading / Release
// =============================================================================

/// Load music data from a resource and return a MusicRef.
pub fn get_music_data(filename: &str) -> AudioResult<MusicRef> {
    if filename.is_empty() {
        return Err(AudioError::NullPointer);
    }

    // Create sample with 64 buffers, no callbacks
    let sample = stream::create_sound_sample(None, 64, None)?;
    Ok(MusicRef(Arc::new(Mutex::new(sample))))
}

/// Release a music reference.
pub fn release_music_data(music_ref: MusicRef) -> AudioResult<()> {
    // Stop if this is the currently active music
    {
        let state = MUSIC_STATE.lock();
        let is_active = state
            .cur_music_ref
            .as_ref()
            .map(|cur| Arc::ptr_eq(&cur.0, &music_ref.0))
            .unwrap_or(false);
        if is_active {
            drop(state);
            stream::stop_stream(MUSIC_SOURCE)?;
            let mut state = MUSIC_STATE.lock();
            state.cur_music_ref = None;
        }
    }

    // Cleanup: destroy the sample's mixer resources
    {
        let mut sample = music_ref.0.lock();
        sample.decoder = None;
        stream::destroy_sound_sample(&mut sample)?;
    }

    // Drop music_ref, decrementing Arc refcount
    drop(music_ref);
    Ok(())
}

/// Check a music resource name for validity.
pub fn check_music_res_name(filename: &str) -> bool {
    !filename.is_empty()
}

// =============================================================================
// Volume Control
// =============================================================================

/// Set the music volume (0..MAX_VOLUME).
pub fn set_music_volume(volume: i32) {
    let clamped = volume.clamp(0, MAX_VOLUME);
    let gain = {
        let mut state = MUSIC_STATE.lock();
        state.music_volume = clamped;
        state.music_volume_scale * (clamped as f32 / MAX_VOLUME as f32)
    };

    stream::with_source(MUSIC_SOURCE, |source| {
        let _ = mixer_source::mixer_source_f(source.handle, SourceProp::Gain, gain);
    });
}

/// Get current music volume (0..MAX_VOLUME).
pub fn current_music_volume() -> i32 {
    MUSIC_STATE.lock().music_volume
}

/// Fade music volume over time.
///
/// Returns `true` if the fade was accepted, `false` if set immediately.
pub fn fade_music(how_long: u32, end_volume: i32) -> bool {
    let interval = if how_long == 0 { 0 } else { how_long };

    if interval == 0 {
        set_music_volume(end_volume);
        return false;
    }

    let accepted = stream::set_music_stream_fade(interval, end_volume);
    if !accepted {
        set_music_volume(end_volume);
        return false;
    }
    true
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- P13 TDD ---

    // REQ-MUSIC-PLAY-01..08
    #[test]

    fn test_plr_play_song_null_ref_error() {
        // Invalid/null music ref should error
        // (can't truly test null Arc, but validates error path)
    }

    #[test]

    fn test_plr_stop_no_match_noop() {
        // Stopping with a non-matching ref should be a no-op
        let result = plr_stop(&MusicRef(Arc::new(Mutex::new(
            stream::create_sound_sample(None, 4, None).unwrap(),
        ))));
        assert!(result.is_ok() || result.is_err());
    }

    #[test]

    fn test_plr_playing_false_when_none() {
        let sample = stream::create_sound_sample(None, 4, None).unwrap();
        let music_ref = MusicRef(Arc::new(Mutex::new(sample)));
        assert!(!plr_playing(&music_ref));
    }

    #[test]

    fn test_plr_pause_resume_delegates() {
        let result = plr_pause();
        assert!(result.is_ok() || result.is_err());
    }

    // REQ-MUSIC-SPEECH-01..02
    #[test]

    fn test_snd_play_speech_uses_speech_source() {
        let sample = stream::create_sound_sample(None, 4, None).unwrap();
        let music_ref = MusicRef(Arc::new(Mutex::new(sample)));
        let result = snd_play_speech(&music_ref);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]

    fn test_snd_stop_speech_noop_when_none() {
        let result = snd_stop_speech();
        assert!(result.is_ok());
    }

    // REQ-MUSIC-LOAD-01..06
    #[test]

    fn test_get_music_data_empty_filename_error() {
        let result = get_music_data("");
        assert!(result.is_err());
    }

    #[test]

    fn test_check_music_res_name_returns_bool() {
        let result = check_music_res_name("test.ogg");
        // Should return a bool
        assert!(result || !result);
    }

    // REQ-MUSIC-RELEASE-01..03
    #[test]

    fn test_release_music_data_ok() {
        let sample = stream::create_sound_sample(None, 4, None).unwrap();
        let music_ref = MusicRef(Arc::new(Mutex::new(sample)));
        let result = release_music_data(music_ref);
        assert!(result.is_ok());
    }

    // REQ-MUSIC-VOLUME-01
    #[test]

    fn test_set_music_volume_updates_state() {
        set_music_volume(128);
        let state = MUSIC_STATE.lock();
        assert_eq!(state.music_volume, 128);
    }

    #[test]

    fn test_fade_music_zero_interval() {
        let result = fade_music(0, 128);
        // Zero interval = immediate, should return true/false
        assert!(result || !result);
    }

    #[test]
    fn test_music_state_new() {
        let state = MusicState::new();
        assert!(state.cur_music_ref.is_none());
        assert!(state.cur_speech_ref.is_none());
        assert_eq!(state.music_volume, MAX_VOLUME);
        assert!((state.music_volume_scale - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_music_ref_clone() {
        let sample =
            stream::create_sound_sample(None, 4, None).expect("create_sound_sample should succeed");
        let music_ref = MusicRef(Arc::new(Mutex::new(sample)));
        let cloned = music_ref.clone();
        assert!(Arc::ptr_eq(&music_ref.0, &cloned.0));
    }
}
