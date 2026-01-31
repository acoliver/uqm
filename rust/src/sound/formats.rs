//! Audio format definitions for sound decoders
//!
//! Matches the C `TFB_DecoderFormats` structure from `decoder.h`.

/// Audio sample format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AudioFormat {
    /// 8-bit mono (1 byte per sample)
    Mono8 = 0x1100,
    /// 16-bit mono (2 bytes per sample)
    Mono16 = 0x1101,
    /// 8-bit stereo (2 bytes per sample)
    Stereo8 = 0x1102,
    /// 16-bit stereo (4 bytes per sample)
    Stereo16 = 0x1103,
}

impl AudioFormat {
    /// Returns the number of bytes per sample
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            AudioFormat::Mono8 => 1,
            AudioFormat::Stereo8 => 2,
            AudioFormat::Mono16 => 2,
            AudioFormat::Stereo16 => 4,
        }
    }

    /// Returns the number of channels
    pub fn channels(&self) -> usize {
        match self {
            AudioFormat::Mono8 | AudioFormat::Mono16 => 1,
            AudioFormat::Stereo8 | AudioFormat::Stereo16 => 2,
        }
    }

    /// Returns true if this is a 16-bit format
    pub fn is_16bit(&self) -> bool {
        matches!(self, AudioFormat::Mono16 | AudioFormat::Stereo16)
    }

    /// Returns true if this is a stereo format
    pub fn is_stereo(&self) -> bool {
        matches!(self, AudioFormat::Stereo8 | AudioFormat::Stereo16)
    }
}

impl Default for AudioFormat {
    fn default() -> Self {
        AudioFormat::Stereo16
    }
}

/// Decoder format configuration
///
/// Matches C `TFB_DecoderFormats` structure. Specifies which audio formats
/// the decoder should use for output.
/// 
/// IMPORTANT: Field order MUST match C struct:
///   bool big_endian; bool want_big_endian;
///   uint32 mono8; uint32 stereo8; uint32 mono16; uint32 stereo16;
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DecoderFormats {
    /// Whether the audio is big-endian
    pub big_endian: bool,
    /// Whether the decoder should output big-endian samples
    pub want_big_endian: bool,
    /// Format code for mono 8-bit audio
    pub mono8: u32,
    /// Format code for stereo 8-bit audio
    pub stereo8: u32,
    /// Format code for mono 16-bit audio
    pub mono16: u32,
    /// Format code for stereo 16-bit audio
    pub stereo16: u32,
}

impl DecoderFormats {
    /// Create a new DecoderFormats with standard OpenAL format codes
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the format code for a given AudioFormat
    pub fn format_code(&self, format: AudioFormat) -> u32 {
        match format {
            AudioFormat::Mono8 => self.mono8,
            AudioFormat::Mono16 => self.mono16,
            AudioFormat::Stereo8 => self.stereo8,
            AudioFormat::Stereo16 => self.stereo16,
        }
    }

    /// Get the AudioFormat for a given format code
    pub fn audio_format(&self, code: u32) -> Option<AudioFormat> {
        if code == self.mono8 {
            Some(AudioFormat::Mono8)
        } else if code == self.mono16 {
            Some(AudioFormat::Mono16)
        } else if code == self.stereo8 {
            Some(AudioFormat::Stereo8)
        } else if code == self.stereo16 {
            Some(AudioFormat::Stereo16)
        } else {
            None
        }
    }
}

impl Default for DecoderFormats {
    fn default() -> Self {
        Self {
            big_endian: false,
            want_big_endian: false,
            mono8: AudioFormat::Mono8 as u32,
            stereo8: AudioFormat::Stereo8 as u32,
            mono16: AudioFormat::Mono16 as u32,
            stereo16: AudioFormat::Stereo16 as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_bytes_per_sample() {
        assert_eq!(AudioFormat::Mono8.bytes_per_sample(), 1);
        assert_eq!(AudioFormat::Mono16.bytes_per_sample(), 2);
        assert_eq!(AudioFormat::Stereo8.bytes_per_sample(), 2);
        assert_eq!(AudioFormat::Stereo16.bytes_per_sample(), 4);
    }

    #[test]
    fn test_audio_format_channels() {
        assert_eq!(AudioFormat::Mono8.channels(), 1);
        assert_eq!(AudioFormat::Mono16.channels(), 1);
        assert_eq!(AudioFormat::Stereo8.channels(), 2);
        assert_eq!(AudioFormat::Stereo16.channels(), 2);
    }

    #[test]
    fn test_audio_format_is_16bit() {
        assert!(!AudioFormat::Mono8.is_16bit());
        assert!(AudioFormat::Mono16.is_16bit());
        assert!(!AudioFormat::Stereo8.is_16bit());
        assert!(AudioFormat::Stereo16.is_16bit());
    }

    #[test]
    fn test_audio_format_is_stereo() {
        assert!(!AudioFormat::Mono8.is_stereo());
        assert!(!AudioFormat::Mono16.is_stereo());
        assert!(AudioFormat::Stereo8.is_stereo());
        assert!(AudioFormat::Stereo16.is_stereo());
    }

    #[test]
    fn test_decoder_formats_default() {
        let formats = DecoderFormats::default();
        assert_eq!(formats.mono8, AudioFormat::Mono8 as u32);
        assert_eq!(formats.mono16, AudioFormat::Mono16 as u32);
        assert_eq!(formats.stereo8, AudioFormat::Stereo8 as u32);
        assert_eq!(formats.stereo16, AudioFormat::Stereo16 as u32);
        assert!(!formats.want_big_endian);
    }

    #[test]
    fn test_decoder_formats_format_code() {
        let formats = DecoderFormats::default();
        assert_eq!(formats.format_code(AudioFormat::Mono8), AudioFormat::Mono8 as u32);
        assert_eq!(formats.format_code(AudioFormat::Stereo16), AudioFormat::Stereo16 as u32);
    }

    #[test]
    fn test_decoder_formats_audio_format() {
        let formats = DecoderFormats::default();
        assert_eq!(formats.audio_format(AudioFormat::Mono8 as u32), Some(AudioFormat::Mono8));
        assert_eq!(formats.audio_format(AudioFormat::Stereo16 as u32), Some(AudioFormat::Stereo16));
        assert_eq!(formats.audio_format(0x9999), None);
    }
}
