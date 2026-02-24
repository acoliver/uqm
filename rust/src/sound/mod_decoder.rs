//! MOD/tracker music decoder implementation
//!
//! Uses the `mod_player` crate for pure Rust MOD file playback.
//! Supports Amiga ProTracker MOD format files.

use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};

/// MOD decoder using mod_player crate
pub struct ModDecoder {
    /// Sample frequency in Hz (always 44100 for mod_player)
    frequency: u32,
    /// Audio format (always Stereo16)
    format: AudioFormat,
    /// Total length in seconds
    length: f32,
    /// Last error code
    last_error: i32,
    /// Whether the decoder is initialized
    initialized: bool,
    /// Stored formats for reference
    formats: Option<DecoderFormats>,
    /// The loaded song
    song: Option<mod_player::Song>,
    /// The player state
    player_state: Option<mod_player::PlayerState>,
    /// Current PCM sample position
    current_pcm: u64,
    /// Total PCM samples (estimated)
    total_pcm: u64,
}

impl ModDecoder {
    /// Create a new MOD decoder
    pub fn new() -> Self {
        Self {
            frequency: 44100,
            format: AudioFormat::Stereo16,
            length: 0.0,
            last_error: 0,
            initialized: false,
            formats: None,
            song: None,
            player_state: None,
            current_pcm: 0,
            total_pcm: 0,
        }
    }

    /// Load a MOD file from bytes
    fn load_from_bytes(&mut self, data: &[u8]) -> DecodeResult<()> {
        // mod_player::read_mod expects a file path, but we can use read_mod_from_bytes
        // Actually, let's check the API...
        // The mod_player crate has read_mod_file which takes a path
        // We need to write to a temp file or find another way

        // For now, let's try parsing directly - mod_player has internal parsing
        // Looking at the mod_player source, it reads the file directly
        // We'll need to write a temp file approach or fork the crate

        // Actually, let's use a cursor approach - mod_player::Song can be created
        // by parsing the MOD format ourselves, but that's complex.

        // Simpler approach: write to temp file, load, delete
        use std::io::Write;
        let temp_path = std::env::temp_dir().join(format!("uqm_mod_{}.mod", std::process::id()));

        {
            let mut file = std::fs::File::create(&temp_path)
                .map_err(|e| DecodeError::IoError(e.to_string()))?;
            file.write_all(data)
                .map_err(|e| DecodeError::IoError(e.to_string()))?;
        }

        let result = self.load_from_path(&temp_path);

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        result
    }

    /// Load a MOD file from a path
    fn load_from_path(&mut self, path: &Path) -> DecodeResult<()> {
        let path_str = path.to_string_lossy();

        let song = mod_player::read_mod_file(&path_str);

        // Initialize player state
        let player_state = mod_player::PlayerState::new(song.format.num_channels, self.frequency);

        // Estimate song length based on actual song structure
        // num_used_patterns = number of positions in the song order table
        // Each pattern has 64 rows
        // Default tempo: 125 BPM, 6 ticks per row
        // Time per row = 60 / BPM * rows_per_beat = 60 / 125 * (1/4) = 0.12 seconds per beat
        // But actually: at 125 BPM, 6 ticks/row, tick time = 2.5ms / tick_constant
        // Amiga timing: row_time = (2500 / bpm) * ticks_per_row ms
        // Default: (2500 / 125) * 6 = 120ms per row = 0.12 seconds
        let num_positions = song.num_used_patterns as usize;
        let rows_per_pattern = 64;
        let total_rows = num_positions * rows_per_pattern;
        // More accurate: 2500ms per beat at 125 BPM with 6 ticks = 120ms per row
        let ms_per_row = 120.0; // Default timing, actual varies with tempo changes
        self.length = (total_rows as f32) * ms_per_row / 1000.0;
        self.total_pcm = (self.length * self.frequency as f32) as u64;

        self.song = Some(song);
        self.player_state = Some(player_state);
        self.current_pcm = 0;

        Ok(())
    }
}

impl Default for ModDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundDecoder for ModDecoder {
    fn name(&self) -> &'static str {
        "Rust MOD"
    }

    fn init_module(&mut self, _flags: i32, formats: &DecoderFormats) -> bool {
        self.formats = Some(*formats);
        true
    }

    fn term_module(&mut self) {
        self.formats = None;
    }

    fn get_error(&mut self) -> i32 {
        let err = self.last_error;
        self.last_error = 0;
        err
    }

    fn init(&mut self) -> bool {
        self.initialized = true;
        self.current_pcm = 0;
        true
    }

    fn term(&mut self) {
        self.close();
        self.initialized = false;
    }

    fn open(&mut self, path: &Path) -> DecodeResult<()> {
        self.load_from_path(path)
    }

    fn open_from_bytes(&mut self, data: &[u8], _name: &str) -> DecodeResult<()> {
        self.load_from_bytes(data)
    }

    fn close(&mut self) {
        self.song = None;
        self.player_state = None;
        self.current_pcm = 0;
    }

    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        if !self.initialized {
            return Err(DecodeError::NotInitialized);
        }

        let song = self.song.as_ref().ok_or(DecodeError::NotInitialized)?;
        let player_state = self
            .player_state
            .as_mut()
            .ok_or(DecodeError::NotInitialized)?;

        // Check if song has truly ended (not just looped)
        // song_has_ended means the song order list is exhausted
        if player_state.song_has_ended {
            return Err(DecodeError::EndOfFile);
        }

        let mut bytes_written = 0;
        let bytes_per_sample = 4; // 2 channels * 2 bytes per sample (16-bit stereo)

        while bytes_written + bytes_per_sample <= buf.len() {
            // Get next sample (returns f32 left/right)
            let (left, right) = mod_player::next_sample(song, player_state);

            // Check for end AFTER getting sample
            // song_has_ended means the song truly ended (no more patterns)
            // has_looped means it wrapped around - for non-looping playback we treat this as end
            if player_state.song_has_ended {
                if bytes_written == 0 {
                    return Err(DecodeError::EndOfFile);
                }
                break;
            }

            // Convert f32 [-1.0, 1.0] to i16
            let left_i16 = (left.clamp(-1.0, 1.0) * 32767.0) as i16;
            let right_i16 = (right.clamp(-1.0, 1.0) * 32767.0) as i16;

            // Write as little-endian bytes
            let left_bytes = left_i16.to_le_bytes();
            let right_bytes = right_i16.to_le_bytes();

            buf[bytes_written] = left_bytes[0];
            buf[bytes_written + 1] = left_bytes[1];
            buf[bytes_written + 2] = right_bytes[0];
            buf[bytes_written + 3] = right_bytes[1];

            bytes_written += bytes_per_sample;
            self.current_pcm += 1;
        }

        Ok(bytes_written)
    }

    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        // MOD seeking is complex - for now only support seek to 0 (restart)
        if pcm_pos == 0 {
            if let Some(ref song) = self.song {
                self.player_state = Some(mod_player::PlayerState::new(
                    song.format.num_channels,
                    self.frequency,
                ));
                self.current_pcm = 0;
                return Ok(0);
            }
        }

        Err(DecodeError::SeekFailed(
            "MOD seeking not fully supported".to_string(),
        ))
    }

    fn get_frame(&self) -> u32 {
        // Return current position as frame count
        (self.current_pcm / 1024) as u32
    }

    fn frequency(&self) -> u32 {
        self.frequency
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn length(&self) -> f32 {
        self.length
    }

    fn is_null(&self) -> bool {
        false
    }

    fn needs_swap(&self) -> bool {
        false // We output little-endian
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_decoder_new() {
        let decoder = ModDecoder::new();
        assert_eq!(decoder.name(), "Rust MOD");
        assert_eq!(decoder.frequency(), 44100);
        assert_eq!(decoder.format(), AudioFormat::Stereo16);
        assert!(!decoder.is_null());
        assert!(!decoder.needs_swap());
    }

    #[test]
    fn test_mod_decoder_init_term() {
        let mut decoder = ModDecoder::new();
        assert!(decoder.init());
        assert!(decoder.initialized);
        decoder.term();
        assert!(!decoder.initialized);
    }

    #[test]
    fn test_mod_decoder_init_module() {
        let mut decoder = ModDecoder::new();
        let formats = DecoderFormats::default();
        assert!(decoder.init_module(0, &formats));
        assert!(decoder.formats.is_some());
        decoder.term_module();
        assert!(decoder.formats.is_none());
    }

    #[test]
    fn test_mod_decoder_decode_not_initialized() {
        let mut decoder = ModDecoder::new();
        let mut buf = [0u8; 1024];
        let result = decoder.decode(&mut buf);
        assert!(matches!(result, Err(DecodeError::NotInitialized)));
    }

    #[test]
    fn test_mod_decoder_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ModDecoder>();
    }
}
