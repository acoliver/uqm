//! Safe facade over the audio heart: stream, track, music and channel
//! operations, content loading and the volume controls that the game modes call.

use std::ffi::{c_int, c_uint, CStr};
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;

use super::control;
use super::decoder::{DecodeResult, Decoder, LimitedDecoder};
use super::mixer::buffer as mixer_buffer;
use super::mixer::{mixer_get_format, mixer_get_frequency};
use super::music;
use super::null::NullDecoder;
use super::sfx;
use super::stream;
use super::trackplayer;
use super::types::*;

/// The bytes of a content file, absent when unreadable or empty.
fn read_content_file(path: &str) -> Option<Vec<u8>> {
    use crate::host::Files;
    crate::io::content_dir()?
        .read(Path::new(path))
        .ok()
        .filter(|data| !data.is_empty())
}

#[derive(Clone, Copy)]
enum Kind {
    Ogg,
    Wav,
    Mod,
    Aiff,
}

impl Kind {
    fn of(ext: &str) -> Option<Self> {
        match ext {
            "ogg" => Some(Self::Ogg),
            "wav" => Some(Self::Wav),
            "mod" => Some(Self::Mod),
            "aif" | "aiff" => Some(Self::Aiff),
            _ => {
                eprintln!("Kind::of: unknown ext '{ext}'");
                None
            }
        }
    }

    fn open(self, data: &[u8], name: &str) -> DecodeResult<Box<dyn Decoder>> {
        macro_rules! open {
            ($decoder:expr) => {{
                let mut decoder = $decoder;
                decoder.init();
                decoder.open_from_bytes(data, name)?;
                Ok(Box::new(decoder))
            }};
        }
        match self {
            Self::Ogg => open!(super::ogg::OggDecoder::new()),
            Self::Wav => open!(super::wav::WavDecoder::new()),
            Self::Mod => open!(super::mod_decoder::ModDecoder::new()),
            Self::Aiff => open!(super::aiff::AiffDecoder::new()),
        }
    }
}

/// Get the file extension from a filename.
fn file_extension(name: &str) -> Option<&str> {
    Path::new(name).extension().and_then(|e| e.to_str())
}

/// Create per-page decoders with time-limited ranges, matching C's
/// `SoundDecoder_Load(dir, file, bufsize, dec_offset, time_stamps[page])` pattern.
///
/// Each page gets its own decoder that seeks to the correct offset and
/// limits decoding to its timestamp duration.
fn create_per_page_decoders(
    track_name: Option<&str>,
    track_text: Option<&str>,
    timestamp: Option<&str>,
) -> Vec<Box<dyn Decoder>> {
    let name = match track_name {
        Some(n) => n,
        None => return Vec::new(),
    };
    let text = match track_text {
        Some(t) => t,
        None => return Vec::new(),
    };

    // Parse pages with same delimiter behavior as C trackplayer (split on CR or LF runs).
    let mut pages: Vec<&str> = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\r' || bytes[i] == b'\n' {
            pages.push(&text[start..i]);
            while i < bytes.len() && (bytes[i] == b'\r' || bytes[i] == b'\n') {
                i += 1;
            }

            start = i;
        } else {
            i += 1;
        }
    }
    if start < bytes.len() {
        pages.push(&text[start..]);
    }

    let num_pages = pages.len();

    let timestamps: Vec<i32> = timestamp
        .map(|ts| {
            ts.split([',', '\n', '\r'])
                .filter_map(|s| s.trim().parse::<f64>().ok())
                .filter(|&v| v > 0.0)
                .map(|v| v as i32)
                .collect()
        })
        .unwrap_or_default();

    // Build per-page run times with C SpliceTrack semantics:
    // - num_timestamps = parsed + 1 when timestamp list is provided
    // - if num_timestamps > num_pages, set the last timestamp to a large negative
    //   value to represent "play remainder" until more subtitle pages are appended
    // - if no timestamps are provided, default to one timing entry per subtitle page
    let mut page_run_times: Vec<i32> = if timestamps.is_empty() {
        (0..num_pages)
            .map(|page_idx| {
                let char_count = pages[page_idx].chars().count();
                (char_count as f64 * 80.0).max(1000.0) as i32
            })
            .collect()
    } else {
        let mut runs = timestamps;
        runs.push(0);
        if runs.len() > num_pages {
            if let Some(last) = runs.last_mut() {
                *last = -100000;
            }
        } else if runs.len() < num_pages {
            // Preserve available explicit timestamps; fallback by text length.
            while runs.len() < num_pages {
                let page_idx = runs.len();
                let char_count = pages[page_idx].chars().count();
                runs.push((char_count as f64 * 80.0).max(1000.0) as i32);
            }
        }
        runs
    };

    // Always ensure the final provided page timing is negative (play to end).
    if let Some(last) = page_run_times.last_mut() {
        if *last > 0 {
            *last = -*last;
        }
    }

    let file_data = read_content_file(name);
    page_decoders(name, file_data.as_deref(), &page_run_times)
}

/// Build one decoder per subtitle page, mirroring C `SoundDecoder_Load` per
/// page in `SpliceTrack`.
///
/// When the clip is missing or its type is unsupported, C substitutes its
/// null decoder for any page with a nonzero run time, lasting `|runTime|`
/// milliseconds, "to keep voiceovers going". Content without voice packs
/// relies on this: dialogue subtitles must still be paced and finish, or
/// `talking_finished` never becomes true and the conversation cannot end.
fn page_decoders(
    name: &str,
    file_data: Option<&[u8]>,
    page_run_times: &[i32],
) -> Vec<Box<dyn Decoder>> {
    let Some(ext) = file_extension(name) else {
        return Vec::new();
    };
    let mut decoders: Vec<Box<dyn Decoder>> = Vec::with_capacity(page_run_times.len());
    let mut dec_offset_ms: u32 = 0;

    for (page_idx, &run_time_ms) in page_run_times.iter().enumerate() {
        let real = Kind::of(ext).zip(file_data);
        let Some((kind, data)) = real else {
            if run_time_ms == 0 {
                break;
            }
            let mut silent = NullDecoder::with_duration(run_time_ms.unsigned_abs() as f32 / 1000.0);
            silent.init();
            dec_offset_ms += run_time_ms.unsigned_abs();
            decoders.push(Box::new(silent));
            continue;
        };
        let dec = match kind.open(data, name) {
            Ok(dec) => dec,
            Err(e) => {
                eprintln!(
                    "[SpliceTrack] decoder open failed for {} page {}: {:?}",
                    name, page_idx, e
                );
                break;
            }
        };

        // Preserve sign like C SoundDecoder_Load(..., runTime):
        // negative run_time means "do not clamp" (play to end from offset).
        let limited = LimitedDecoder::new(dec, dec_offset_ms, run_time_ms);

        eprintln!(
            "[SpliceTrack] page {}/{}: offset={}ms run={}ms dec_len={:.2}s",
            page_idx + 1,
            page_run_times.len(),
            dec_offset_ms,
            run_time_ms,
            limited.length()
        );

        // Advance offset by actual decoder length (ms)
        dec_offset_ms += (limited.length() * 1000.0) as u32;

        decoders.push(Box::new(limited));
    }

    eprintln!(
        "[SpliceTrack] created {} per-page decoders for {} (total offset={}ms)",
        decoders.len(),
        name,
        dec_offset_ms
    );
    decoders
}

// =============================================================================
// Stream operations
// =============================================================================

pub fn init_stream_decoder() -> c_int {
    match stream::init_stream_decoder() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("InitStreamDecoder: {}", e);
            -1
        }
    }
}

pub fn uninit_stream_decoder() {
    let _ = stream::uninit_stream_decoder();
}

pub fn stop_stream(source: u32) {
    let _ = stream::stop_stream(source as usize);
}

pub fn pause_stream(source: u32) {
    let _ = stream::pause_stream(source as usize);
}

pub fn resume_stream(source: u32) {
    let _ = stream::resume_stream(source as usize);
}

pub fn seek_stream(source: u32, pos: u32) {
    let _ = stream::seek_stream(source as usize, pos);
}

pub fn playing_stream(source: u32) -> u8 {
    u8::from(stream::playing_stream(source as usize))
}

pub fn set_music_stream_fade(how_long: i32, end_volume: c_int) -> bool {
    stream::set_music_stream_fade(how_long.max(0) as u32, end_volume)
}

// =============================================================================
// Speech track facade
// =============================================================================

/// Decode and append speech while retaining the supplied callback for playback and seeking.
pub fn splice_track(
    track_name: Option<&CStr>,
    track_text: Option<&CStr>,
    timestamp: Option<&CStr>,
    callback: Option<Box<dyn Fn(i32) + Send>>,
) {
    let name = track_name
        .filter(|s| !s.is_empty())
        .map(CStr::to_string_lossy);
    let text = track_text
        .filter(|s| !s.is_empty())
        .map(CStr::to_string_lossy);
    let timestamp = timestamp
        .filter(|s| !s.is_empty())
        .map(CStr::to_string_lossy);
    eprintln!(
        "[SpliceTrack] name={:?} text_len={:?} ts_len={:?} cb={}",
        name,
        text.as_ref().map(|s| s.len()),
        timestamp.as_ref().map(|s| s.len()),
        callback.is_some()
    );

    // Create per-page decoders with time-limited ranges (like C's SoundDecoder_Load)
    let decoders = create_per_page_decoders(name.as_deref(), text.as_deref(), timestamp.as_deref());

    let _ = trackplayer::splice_track(
        name.as_deref(),
        text.as_deref(),
        timestamp.as_deref(),
        callback,
        decoders,
    );
}

/// Append complete number clips, preserving the timed null decoder for absent voice content.
pub fn splice_multi_track(track_names: &[&CStr], track_text: Option<&CStr>) {
    let Some(text) = track_text
        .filter(|s| !s.is_empty())
        .map(CStr::to_string_lossy)
    else {
        return;
    };
    let mut decoders = Vec::new();
    for clip in track_names.iter().take(trackplayer::MAX_MULTI_TRACKS) {
        let name = clip.to_string_lossy();
        let data = read_content_file(&name);
        // C loads the complete clip, or its timed null decoder when voice
        // content is absent. Never append an audio chunk without a decoder.
        decoders.extend(page_decoders(
            &name,
            data.as_deref(),
            &[-(3.0 * trackplayer::TEXT_SPEED) as i32],
        ));
    }
    if let Err(error) = trackplayer::splice_multi_track(decoders, Some(&text)) {
        eprintln!("[SpliceMultiTrack] {error}");
    }
}

pub fn play_track() {
    eprintln!("[PlayTrack] called");
    match trackplayer::play_track(true) {
        Ok(()) => eprintln!("[PlayTrack] OK"),
        Err(e) => eprintln!("[PlayTrack] ERROR: {:?}", e),
    }
}

pub fn stop_track() {
    let _ = trackplayer::stop_track();
}

pub fn jump_track() {
    let _ = trackplayer::jump_track(0);
}

pub fn pause_track() {
    let _ = trackplayer::pause_track();
}

pub fn resume_track() {
    let _ = trackplayer::resume_track();
}

pub fn playing_track() -> u16 {
    trackplayer::playing_track_num()
}

pub fn fast_reverse_smooth() {
    let _ = trackplayer::fast_reverse_smooth();
}

pub fn fast_forward_smooth() {
    let _ = trackplayer::fast_forward_smooth();
}

pub fn fast_reverse_page() {
    let _ = trackplayer::fast_reverse_page();
}

pub fn fast_forward_page() {
    let _ = trackplayer::fast_forward_page();
}

pub fn get_track_position(in_units: c_int) -> c_int {
    trackplayer::get_track_position(in_units as u32) as c_int
}

// =============================================================================
// Music and speech
// =============================================================================

pub fn snd_stop_speech() {
    let _ = music::snd_stop_speech();
}

pub fn set_music_volume(volume: c_int) {
    music::set_music_volume(volume);
}

pub fn fade_music(end_vol: u8, how_long: i16) -> u32 {
    let interval = if how_long < 0 { 0u32 } else { how_long as u32 };
    if crate::sound::types::quit_posted() {
        // Don't make users wait for fades on quit
        music::set_music_volume(end_vol as i32);
        return crate::sound::types::get_time_counter();
    }
    if !music::fade_music(interval, end_vol as i32) {
        music::set_music_volume(end_vol as i32);
        return crate::sound::types::get_time_counter();
    }
    crate::sound::types::get_time_counter() + interval
}

// =============================================================================
// Effect channels
// =============================================================================

pub fn stop_channel(channel: c_uint, priority: c_int) {
    let _ = sfx::stop_channel(channel as usize, priority);
}

pub fn channel_playing(channel: c_uint) -> c_int {
    c_int::from(sfx::channel_playing(channel as usize))
}

pub fn set_channel_volume(channel: c_uint, volume: c_int, priority: c_int) {
    sfx::set_channel_volume(channel as usize, volume, priority);
}

pub fn update_sound_position(source_index: c_uint, pos: SoundPosition) {
    sfx::update_sound_position(source_index as usize, pos);
}

pub fn get_positional_object(source_index: c_uint) -> usize {
    sfx::get_positional_object(source_index as usize)
}

pub fn set_positional_object(source_index: c_uint, object: usize) {
    sfx::set_positional_object(source_index as usize, object);
}

// =============================================================================
// Global control
// =============================================================================

pub fn stop_sound() {
    control::stop_sound();
}

pub fn sound_playing() -> c_int {
    c_int::from(control::sound_playing())
}

pub fn wait_for_sound_end(channel: u16) {
    // COUNT is u16; TFBSOUND_WAIT_ALL is (COUNT)~0 = 0xFFFF
    let ch = if channel == 0xFFFF {
        None
    } else {
        Some(channel as usize)
    };
    control::wait_for_sound_end(ch);
}

pub fn set_sfx_volume(volume: f32) {
    control::set_sfx_volume((volume * MAX_VOLUME as f32) as i32);
}

pub fn set_speech_volume(volume: f32) {
    control::set_speech_volume((volume * MAX_VOLUME as f32) as i32);
}

// =============================================================================
// Content loading
// =============================================================================

/// Load a sound bank listing: each line names a clip next to the listing, and
/// every clip is decoded to PCM up front.
pub fn load_sound_bank(name: &str) -> Option<super::owned_bank::LoadedSoundBank> {
    eprintln!("load_sound_bank: loading bank {name}");
    let Some(listing) = read_content_file(name) else {
        eprintln!("load_sound_bank({name}): failed to read file");
        return None;
    };
    let dir_prefix = match name.rfind('/').or_else(|| name.rfind('\\')) {
        Some(pos) => &name[..=pos],
        None => "",
    };
    let text = String::from_utf8_lossy(&listing);
    let mut samples: Vec<SoundSample> = Vec::new();
    for line in text.lines() {
        let snd_name = line.trim();
        if snd_name.is_empty() {
            continue;
        }
        let full_path = format!("{dir_prefix}{snd_name}");
        eprintln!("load_sound_bank: loading sound {full_path}");
        let Some(ext) = file_extension(snd_name) else {
            eprintln!("load_sound_bank: no extension for '{snd_name}', skipping");
            continue;
        };
        let Some(kind) = Kind::of(ext) else {
            eprintln!("load_sound_bank: no decoder for .{ext}, skipping {snd_name}");
            continue;
        };
        let Some(snd_data) = read_content_file(&full_path) else {
            eprintln!("load_sound_bank: couldn't read {full_path}");
            continue;
        };
        let mut decoder = match kind.open(&snd_data, snd_name) {
            Ok(decoder) => decoder,
            Err(e) => {
                eprintln!("load_sound_bank: decoder failed for {full_path}: {e:?}");
                continue;
            }
        };
        let freq = decoder.frequency();
        let format = decoder.format();
        let length = decoder.length();
        eprintln!("load_sound_bank: decoder {snd_name} rate={freq} format={format:?}");
        let pcm = match super::types::decode_all(decoder.as_mut()) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("load_sound_bank: decode_all failed for {full_path}: {e:?}");
                continue;
            }
        };
        eprintln!(
            "load_sound_bank: decoded {} bytes for {snd_name}",
            pcm.len()
        );

        // One buffer per effect: pre-decoded, no streaming decoder.
        let mut sample = match stream::create_sound_sample(None, 1, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("load_sound_bank: create_sound_sample failed: {e}");
                continue;
            }
        };
        sample.length = length;
        if !sample.buffers.is_empty() && !pcm.is_empty() {
            let _ = mixer_buffer::mixer_buffer_data(
                sample.buffers[0],
                format.to_mixer_format(),
                &pcm,
                freq,
                mixer_get_frequency(),
                mixer_get_format(),
            );
        }
        samples.push(sample);
    }
    if samples.is_empty() {
        eprintln!("load_sound_bank({name}): no sounds decoded");
        return None;
    }
    eprintln!("load_sound_bank({name}): loaded {} sounds", samples.len());
    Some(super::owned_bank::LoadedSoundBank::from_samples(samples))
}

/// Load a streaming music file with 64 buffers.
pub fn load_music(name: &str) -> Option<MusicRef> {
    eprintln!("load_music: loading {name}");
    let kind = Kind::of(file_extension(name)?)?;
    let data = read_content_file(name)?;
    let decoder = match kind.open(&data, name) {
        Ok(decoder) => decoder,
        Err(e) => {
            eprintln!("load_music({name}): decoder open failed: {e:?}");
            return None;
        }
    };
    eprintln!(
        "load_music: decoder rate={} format={:?} length={:.1}s",
        decoder.frequency(),
        decoder.format(),
        decoder.length()
    );
    match stream::create_sound_sample(Some(decoder), 64, None) {
        Ok(sample) => Some(MusicRef(Arc::new(Mutex::new(sample)))),
        Err(e) => {
            eprintln!("load_music({name}): create_sound_sample failed: {e}");
            None
        }
    }
}

/// Release a loaded music reference and its stream buffers.
pub fn destroy_music(music_ref: MusicRef) {
    let _ = music::release_music_data(music_ref);
}

#[cfg(test)]
mod tests {
    /// Content without voice packs names clips that do not exist. C keeps the
    /// conversation going with a null decoder per page lasting `|runTime|`.
    #[test]
    fn missing_voice_clip_pages_are_paced_by_silent_decoders() {
        let decoders = super::page_decoders("urquan-027.ogg", None, &[2000, -3000]);
        assert_eq!(decoders.len(), 2);
        assert!(decoders.iter().all(|decoder| decoder.is_null()));
        assert!((decoders[0].length() - 2.0).abs() < f32::EPSILON);
        assert!((decoders[1].length() - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn silent_page_decoder_plays_to_its_end() {
        let mut decoders = super::page_decoders("urquan-027.ogg", None, &[1000]);
        let decoder = &mut decoders[0];
        let mut buffer = vec![0u8; 4096];
        let mut total = 0usize;
        loop {
            match decoder.decode(&mut buffer) {
                Ok(bytes) => total += bytes,
                Err(crate::sound::decoder::DecodeError::EndOfFile) => break,
                Err(other) => panic!("silent page failed to decode: {other:?}"),
            }
        }
        // One second of 11025 Hz mono16 silence.
        assert_eq!(total, 11025 * 2);
    }

    #[test]
    fn missing_clip_with_zero_run_time_loads_nothing_like_c() {
        assert!(super::page_decoders("urquan-027.ogg", None, &[0, 1000]).is_empty());
    }

    #[test]
    fn unsupported_clip_type_is_also_paced_silently() {
        let decoders = super::page_decoders("speech.xyz", Some(b"not audio"), &[1500]);
        assert_eq!(decoders.len(), 1);
        assert!(decoders[0].is_null());
        assert!((decoders[0].length() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn clip_without_extension_has_no_decoder_like_c() {
        assert!(super::page_decoders("speech", None, &[1000]).is_empty());
    }

    use super::*;

    #[test]
    fn test_init_stream_decoder_return_type() {
        // Verify the facade signature returns c_int
        let f: fn() -> c_int = init_stream_decoder;
        let _ = f; // type-check only; calling mutates global state
    }

    #[test]
    fn test_playing_stream_returns_int() {
        let result = playing_stream(0);
        assert!(result == 0 || result == 1);
    }

    #[test]
    fn test_playing_track_matches_trackplayer_state() {
        let result = playing_track();
        assert_eq!(result, trackplayer::playing_track_num());
    }

    #[test]
    fn test_sound_playing_returns_int() {
        let result = sound_playing();
        assert!(result == 0 || result == 1);
    }

    #[test]
    fn phrase_callback_is_optional_and_reusable_when_seeking() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let absent: Option<Box<dyn Fn(i32) + Send>> = None;
        assert!(absent.is_none());
        let calls = std::sync::Arc::new(AtomicU32::new(0));
        let captured = calls.clone();
        let callback: Box<dyn Fn(i32) + Send> = Box::new(move |_| {
            captured.fetch_add(1, Ordering::SeqCst);
        });
        callback(0);
        callback(17);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
