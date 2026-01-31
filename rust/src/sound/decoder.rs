//! Sound decoder trait definition
//!
//! Defines the `SoundDecoder` trait that all audio decoders must implement.
//! This mirrors the C `TFB_SoundDecoderFuncs` vtable pattern.

use std::path::Path;

use super::formats::{AudioFormat, DecoderFormats};

/// Error type for decoder operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// File not found
    NotFound(String),
    /// Invalid or corrupted audio data
    InvalidData(String),
    /// Unsupported audio format
    UnsupportedFormat(String),
    /// I/O error
    IoError(String),
    /// Decoder not initialized
    NotInitialized,
    /// End of file reached
    EndOfFile,
    /// Seek failed
    SeekFailed(String),
    /// Generic decoder error
    DecoderError(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::NotFound(s) => write!(f, "File not found: {}", s),
            DecodeError::InvalidData(s) => write!(f, "Invalid audio data: {}", s),
            DecodeError::UnsupportedFormat(s) => write!(f, "Unsupported format: {}", s),
            DecodeError::IoError(s) => write!(f, "I/O error: {}", s),
            DecodeError::NotInitialized => write!(f, "Decoder not initialized"),
            DecodeError::EndOfFile => write!(f, "End of file"),
            DecodeError::SeekFailed(s) => write!(f, "Seek failed: {}", s),
            DecodeError::DecoderError(s) => write!(f, "Decoder error: {}", s),
        }
    }
}

impl std::error::Error for DecodeError {}

/// Result type for decoder operations
pub type DecodeResult<T> = Result<T, DecodeError>;

/// Sound decoder trait
///
/// All audio decoders must implement this trait. The interface mirrors the
/// C `TFB_SoundDecoderFuncs` vtable for compatibility.
pub trait SoundDecoder: Send {
    /// Returns the decoder name (e.g., "Ogg Vorbis", "WAV")
    fn name(&self) -> &'static str;

    /// Initialize the decoder module with format settings
    ///
    /// Called once when the decoder type is registered.
    fn init_module(&mut self, flags: i32, formats: &DecoderFormats) -> bool;

    /// Terminate the decoder module
    ///
    /// Called once when the decoder type is unregistered.
    fn term_module(&mut self);

    /// Returns the last error code, clearing it
    fn get_error(&mut self) -> i32;

    /// Initialize a decoder instance
    fn init(&mut self) -> bool;

    /// Terminate a decoder instance
    fn term(&mut self);

    /// Open an audio file for decoding
    ///
    /// # Arguments
    /// * `path` - Path to the audio file
    ///
    /// # Returns
    /// `true` if the file was opened successfully
    fn open(&mut self, path: &Path) -> DecodeResult<()>;

    /// Open an audio file from raw bytes
    ///
    /// # Arguments
    /// * `data` - Raw audio file data
    /// * `name` - Name for logging purposes
    ///
    /// # Returns
    /// `true` if the data was opened successfully
    fn open_from_bytes(&mut self, data: &[u8], name: &str) -> DecodeResult<()>;

    /// Close the currently open audio file
    fn close(&mut self);

    /// Decode audio data into the provided buffer
    ///
    /// # Arguments
    /// * `buf` - Buffer to write decoded PCM data into
    ///
    /// # Returns
    /// Number of bytes decoded, or negative error code
    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize>;

    /// Seek to a specific PCM sample position
    ///
    /// # Arguments
    /// * `pcm_pos` - Target sample position
    ///
    /// # Returns
    /// Actual position after seeking
    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32>;

    /// Get the current frame number (for formats with frames)
    fn get_frame(&self) -> u32;

    // Accessor methods for decoder state

    /// Returns the sample frequency in Hz
    fn frequency(&self) -> u32;

    /// Returns the audio format
    fn format(&self) -> AudioFormat;

    /// Returns the total length in seconds
    fn length(&self) -> f32;

    /// Returns true if this is a null/silent decoder
    fn is_null(&self) -> bool;

    /// Returns true if byte swapping is needed
    fn needs_swap(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_error_display() {
        let err = DecodeError::NotFound("test.ogg".to_string());
        assert_eq!(format!("{}", err), "File not found: test.ogg");

        let err = DecodeError::EndOfFile;
        assert_eq!(format!("{}", err), "End of file");
    }

    #[test]
    fn test_decode_error_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<DecodeError>();
    }
}
