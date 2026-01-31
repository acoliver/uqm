// types.rs - Core types and enums for the audio mixer

//! Core types and enumerations for the OpenAL-like audio mixer.
//!
//! This module defines the fundamental types used throughout the mixer
//! system, including error codes, audio formats, and state enumerations.

/// Magic number for mixer buffers (MIXB in little-endian)
pub const MIXER_BUF_MAGIC: u32 = 0x4258494D;

/// Magic number for mixer sources (MIXS in little-endian)
pub const MIXER_SRC_MAGIC: u32 = 0x5358494D;

/// Maximum number of sources that can be active simultaneously
pub const MAX_SOURCES: usize = 8;

/// Gain adjustment constant for volume scaling
pub const MIX_GAIN_ADJ: f32 = 255.0;

/// Maximum value for 16-bit signed integer
pub const SINT16_MAX: f32 = 32767.0;

/// Minimum value for 16-bit signed integer
pub const SINT16_MIN: f32 = -32768.0;

/// Maximum value for 8-bit signed integer
pub const SINT8_MAX: f32 = 127.0;

/// Minimum value for 8-bit signed integer
pub const SINT8_MIN: f32 = -128.0;

/// Base dummy ID for format constants
pub const MIX_FORMAT_DUMMYID: u32 = 0x00170000;

/// Mixer error codes (compatible with OpenAL error codes)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerError {
    NoError = 0,
    InvalidName = 0xA001,
    InvalidEnum = 0xA002,
    InvalidValue = 0xA003,
    InvalidOperation = 0xA004,
    OutOfMemory = 0xA005,
    DriverFailure = 0xA101,
}

impl MixerError {
    /// Convert from raw u32 error code
    pub fn from_u32(code: u32) -> Self {
        match code {
            0 => MixerError::NoError,
            0xA001 => MixerError::InvalidName,
            0xA002 => MixerError::InvalidEnum,
            0xA003 => MixerError::InvalidValue,
            0xA004 => MixerError::InvalidOperation,
            0xA005 => MixerError::OutOfMemory,
            0xA101 => MixerError::DriverFailure,
            _ => MixerError::DriverFailure,
        }
    }

    /// Convert to raw u32 error code
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

/// Audio format enumeration
///
/// Format is encoded as: bits 0-7 = bytes per sample, bits 8-15 = channels
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerFormat {
    Mono8 = 0x00170101,
    Stereo8 = 0x00170201,
    Mono16 = 0x00170102,
    Stereo16 = 0x00170202,
}

impl MixerFormat {
    /// Extract bytes per channel from format
    pub fn bytes_per_channel(self) -> u32 {
        (self as u32) & 0xFF
    }

    /// Extract number of channels from format
    pub fn channels(self) -> u32 {
        ((self as u32) >> 8) & 0xFF
    }

    /// Calculate sample size (bytes per channel * channels)
    pub fn sample_size(self) -> u32 {
        self.bytes_per_channel() * self.channels()
    }

    /// Create format from bytes per channel and channels
    pub fn make(bpc: u32, chans: u32) -> u32 {
        MIX_FORMAT_DUMMYID | (bpc & 0xFF) | ((chans & 0xFF) << 8)
    }
}

/// Source playback state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Initial = 0,
    Stopped,
    Playing,
    Paused,
}

/// Buffer state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    Initial = 0,
    Filled,
    Queued,
    Playing,
    Processed,
}

/// Mixer quality level (affects resampling algorithm)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerQuality {
    Low = 0,
    Medium = 1,
    High = 2,
}

impl MixerQuality {
    /// Default quality is Medium
    pub const DEFAULT: MixerQuality = MixerQuality::Medium;
}

/// Mixer flags
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerFlags {
    None = 0,
    FakeData = 1,
}

impl MixerFlags {
    /// Check if flag is set
    pub fn contains(&self, flag: MixerFlags) -> bool {
        (*self as u32) & (flag as u32) != 0
    }
}

/// Source properties (compatible with OpenAL)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceProp {
    Position = 0x1004,
    Looping = 0x1007,
    Buffer = 0x1009,
    Gain = 0x100A,
    SourceState = 0x1010,
    BuffersQueued = 0x1015,
    BuffersProcessed = 0x1016,
}

/// Buffer properties (compatible with OpenAL)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferProp {
    Frequency = 0x2001,
    Bits = 0x2002,
    Channels = 0x2003,
    Size = 0x2004,
    Data = 0x2005,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_extraction() {
        assert_eq!(MixerFormat::Mono8.bytes_per_channel(), 1);
        assert_eq!(MixerFormat::Mono8.channels(), 1);
        assert_eq!(MixerFormat::Mono8.sample_size(), 1);

        assert_eq!(MixerFormat::Stereo16.bytes_per_channel(), 2);
        assert_eq!(MixerFormat::Stereo16.channels(), 2);
        assert_eq!(MixerFormat::Stereo16.sample_size(), 4);
    }

    #[test]
    fn test_format_make() {
        let format = MixerFormat::make(2, 2);
        assert_eq!(format, MixerFormat::Stereo16 as u32);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(MixerError::InvalidName.to_u32(), 0xA001);
        assert_eq!(MixerError::from_u32(0xA002), MixerError::InvalidEnum);
    }

    #[test]
    fn test_flags() {
        let flags = MixerFlags::FakeData;
        assert!(flags.contains(MixerFlags::FakeData));
        assert!(!flags.contains(MixerFlags::None));
    }
}
