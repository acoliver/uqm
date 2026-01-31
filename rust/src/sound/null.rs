//! Null (silent) decoder implementation
//!
//! Provides a decoder that produces silence for a specified duration.
//! Used as a fallback when audio files are missing or unsupported.

use std::path::Path;

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};

/// Null decoder that produces silence
///
/// This decoder doesn't read any actual audio data. Instead, it produces
/// silence (zero-filled buffers) for a configurable duration. This is used
/// when audio files are missing but the game needs to continue with timing.
pub struct NullDecoder {
    /// Sample frequency in Hz
    frequency: u32,
    /// Audio format
    format: AudioFormat,
    /// Total length in seconds
    length: f32,
    /// Current PCM position
    current_pcm: u32,
    /// Maximum PCM samples (based on length and frequency)
    max_pcm: u32,
    /// Last error code
    last_error: i32,
    /// Whether the decoder is initialized
    initialized: bool,
    /// Stored formats for reference
    formats: Option<DecoderFormats>,
}

impl NullDecoder {
    /// Create a new null decoder
    pub fn new() -> Self {
        Self {
            frequency: 11025,
            format: AudioFormat::Mono16,
            length: 0.0,
            current_pcm: 0,
            max_pcm: 0,
            last_error: 0,
            initialized: false,
            formats: None,
        }
    }

    /// Create a null decoder with a specific duration
    pub fn with_duration(duration_seconds: f32) -> Self {
        let mut decoder = Self::new();
        decoder.length = duration_seconds;
        decoder.max_pcm = (duration_seconds * decoder.frequency as f32) as u32;
        decoder
    }

    /// Set the duration in seconds
    pub fn set_duration(&mut self, duration_seconds: f32) {
        self.length = duration_seconds;
        self.max_pcm = (duration_seconds * self.frequency as f32) as u32;
    }
}

impl Default for NullDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundDecoder for NullDecoder {
    fn name(&self) -> &'static str {
        "Null"
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

    fn open(&mut self, _path: &Path) -> DecodeResult<()> {
        // Null decoder doesn't actually open files
        self.frequency = 11025;
        self.format = AudioFormat::Mono16;
        self.current_pcm = 0;
        Ok(())
    }

    fn open_from_bytes(&mut self, _data: &[u8], _name: &str) -> DecodeResult<()> {
        // Null decoder doesn't actually open data
        self.frequency = 11025;
        self.format = AudioFormat::Mono16;
        self.current_pcm = 0;
        Ok(())
    }

    fn close(&mut self) {
        self.current_pcm = 0;
    }

    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        if !self.initialized {
            return Err(DecodeError::NotInitialized);
        }

        let bytes_per_sample = self.format.bytes_per_sample();
        let samples_requested = buf.len() / bytes_per_sample;
        let samples_remaining = self.max_pcm.saturating_sub(self.current_pcm) as usize;
        let samples_to_decode = samples_requested.min(samples_remaining);
        let bytes_to_decode = samples_to_decode * bytes_per_sample;

        if bytes_to_decode == 0 {
            return Err(DecodeError::EndOfFile);
        }

        // Fill buffer with silence (zeros)
        buf[..bytes_to_decode].fill(0);
        self.current_pcm += samples_to_decode as u32;

        Ok(bytes_to_decode)
    }

    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        let actual_pos = pcm_pos.min(self.max_pcm);
        self.current_pcm = actual_pos;
        Ok(actual_pos)
    }

    fn get_frame(&self) -> u32 {
        0 // Null decoder has only one "frame"
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
        true
    }

    fn needs_swap(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_decoder_new() {
        let decoder = NullDecoder::new();
        assert_eq!(decoder.name(), "Null");
        assert_eq!(decoder.frequency(), 11025);
        assert_eq!(decoder.format(), AudioFormat::Mono16);
        assert_eq!(decoder.length(), 0.0);
        assert!(decoder.is_null());
        assert!(!decoder.needs_swap());
    }

    #[test]
    fn test_null_decoder_with_duration() {
        let decoder = NullDecoder::with_duration(5.0);
        assert_eq!(decoder.length(), 5.0);
        assert_eq!(decoder.max_pcm, (5.0 * 11025.0) as u32);
    }

    #[test]
    fn test_null_decoder_init_term() {
        let mut decoder = NullDecoder::new();
        assert!(decoder.init());
        assert!(decoder.initialized);
        decoder.term();
        assert!(!decoder.initialized);
    }

    #[test]
    fn test_null_decoder_decode_not_initialized() {
        let mut decoder = NullDecoder::new();
        let mut buf = [0u8; 1024];
        let result = decoder.decode(&mut buf);
        assert!(matches!(result, Err(DecodeError::NotInitialized)));
    }

    #[test]
    fn test_null_decoder_decode_silence() {
        let mut decoder = NullDecoder::with_duration(1.0);
        decoder.init();

        let mut buf = [0xFFu8; 1024];
        let result = decoder.decode(&mut buf);
        assert!(result.is_ok());

        let bytes_decoded = result.unwrap();
        assert!(bytes_decoded > 0);

        // Verify buffer is filled with zeros (silence)
        for byte in &buf[..bytes_decoded] {
            assert_eq!(*byte, 0);
        }
    }

    #[test]
    fn test_null_decoder_decode_eof() {
        let mut decoder = NullDecoder::with_duration(0.001); // Very short
        decoder.init();

        let mut buf = [0u8; 1024];
        
        // First decode should work
        let result = decoder.decode(&mut buf);
        assert!(result.is_ok());

        // Decode until EOF
        loop {
            let result = decoder.decode(&mut buf);
            if matches!(result, Err(DecodeError::EndOfFile)) {
                break;
            }
        }
    }

    #[test]
    fn test_null_decoder_seek() {
        let mut decoder = NullDecoder::with_duration(10.0);
        decoder.init();

        // Seek to middle
        let result = decoder.seek(5000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5000);

        // Seek past end should clamp
        let result = decoder.seek(u32::MAX);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), decoder.max_pcm);
    }

    #[test]
    fn test_null_decoder_get_error() {
        let mut decoder = NullDecoder::new();
        decoder.last_error = 42;
        assert_eq!(decoder.get_error(), 42);
        assert_eq!(decoder.get_error(), 0); // Should be cleared
    }

    #[test]
    fn test_null_decoder_init_module() {
        let mut decoder = NullDecoder::new();
        let formats = DecoderFormats::default();
        assert!(decoder.init_module(0, &formats));
        assert!(decoder.formats.is_some());
        decoder.term_module();
        assert!(decoder.formats.is_none());
    }

    #[test]
    fn test_null_decoder_open() {
        let mut decoder = NullDecoder::new();
        decoder.init();
        let result = decoder.open(Path::new("/nonexistent/file.ogg"));
        assert!(result.is_ok()); // Null decoder doesn't care about the file
    }

    #[test]
    fn test_null_decoder_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<NullDecoder>();
    }
}
