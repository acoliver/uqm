//! MOD/tracker music decoder implementation
//!
//! Parses and plays Amiga ProTracker MOD data in memory, including archive resources.
//! Byte-based opening performs no filesystem writes.

use std::path::Path;

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};

/// In-memory ProTracker MOD decoder
pub struct ModDecoder {
    /// Sample frequency in Hz
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
    song: Option<super::tracker::Song>,
    /// The player state
    player_state: Option<super::tracker::Player>,
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

    fn load_from_bytes(&mut self, data: &[u8]) -> DecodeResult<()> {
        let song = super::tracker::Song::parse(data)?;
        let player = super::tracker::Player::new(song.channels, self.frequency);
        self.length = song.estimated_length();
        self.total_pcm = (self.length * self.frequency as f32) as u64;
        self.song = Some(song);
        self.player_state = Some(player);
        self.current_pcm = 0;
        Ok(())
    }

    fn load_from_path(&mut self, path: &Path) -> DecodeResult<()> {
        let data = std::fs::read(path).map_err(|error| DecodeError::IoError(error.to_string()))?;
        self.load_from_bytes(&data)
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
        if player_state.ended {
            return Err(DecodeError::EndOfFile);
        }

        let mut bytes_written = 0;
        let bytes_per_sample = 4; // 2 channels * 2 bytes per sample (16-bit stereo)

        while bytes_written + bytes_per_sample <= buf.len() {
            // Get next sample (returns f32 left/right)
            let (left, right) = player_state.next_sample(song);

            // Check for end AFTER getting sample
            // song_has_ended means the song truly ended (no more patterns)
            // has_looped means it wrapped around - for non-looping playback we treat this as end
            if player_state.ended {
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
                self.player_state =
                    Some(super::tracker::Player::new(song.channels, self.frequency));
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

    fn audible_module() -> Vec<u8> {
        let mut data = vec![0; 1084 + 1024 + 32];
        data[42..44].copy_from_slice(&16_u16.to_be_bytes());
        data[45] = 64;
        data[48..50].copy_from_slice(&16_u16.to_be_bytes());
        data[950] = 1;
        data[1080..1084].copy_from_slice(b"M.K.");
        data[1084..1088].copy_from_slice(&[0x01, 0xac, 0x10, 0]);
        for (index, sample) in data[2108..].iter_mut().enumerate() {
            *sample = if index < 16 { 100 } else { 156 };
        }
        data
    }

    #[test]
    fn module_bytes_decode_without_filesystem_writes() {
        #[cfg(target_os = "macos")]
        if std::env::var_os("UQM_MOD_MEMORY_CHILD").is_none() {
            let status = std::process::Command::new("/usr/bin/sandbox-exec")
                .args(["-p", "(version 1)(allow default)(deny file-write*)"])
                .arg(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sound::mod_decoder::tests::module_bytes_decode_without_filesystem_writes",
                    "--nocapture",
                ])
                .env("UQM_MOD_MEMORY_CHILD", "1")
                .status()
                .unwrap();
            assert!(status.success());
            return;
        }
        let mut decoder = ModDecoder::new();
        assert!(decoder.init());
        decoder
            .open_from_bytes(&audible_module(), "avatar.mod")
            .unwrap();
        let mut pcm = [0; 4096];
        assert_eq!(decoder.decode(&mut pcm).unwrap(), pcm.len());
        assert!(pcm.iter().any(|byte| *byte != 0));
        decoder.seek(0).unwrap();
        let mut repeated = [0; 4096];
        decoder.decode(&mut repeated).unwrap();
        assert_eq!(pcm, repeated);
    }

    #[test]
    fn truncated_module_returns_typed_error() {
        let mut decoder = ModDecoder::new();
        assert!(matches!(
            decoder.open_from_bytes(b"short", "short.mod"),
            Err(DecodeError::InvalidData(_))
        ));
    }

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
