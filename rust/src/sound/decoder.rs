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

/// A wrapper decoder that limits decoding to a specific sample range.
///
/// Mirrors C's `SoundDecoder_Load(dir, file, bufsize, startTime, runTime)` behavior:
/// seeks to `start_sample` on open, returns EOF after `end_sample`.
pub struct LimitedDecoder {
    inner: Box<dyn SoundDecoder>,
    start_sample: u32,
    end_sample: u32,
    /// Bytes per sample frame (e.g., 4 for stereo16)
    bytes_per_frame: u32,
    /// Current position in samples
    current_sample: u32,
    /// Length in seconds (clamped to the limited range)
    limited_length: f32,
}

impl LimitedDecoder {
    /// Create a new LimitedDecoder.
    ///
    /// # Arguments
    /// * `inner` - The underlying decoder (already opened with data)
    /// * `start_time_ms` - Start time in milliseconds
    /// * `run_time_ms` - Maximum run time in milliseconds (0 = unlimited)
    pub fn new(
        mut inner: Box<dyn SoundDecoder>,
        start_time_ms: u32,
        run_time_ms: i32,
    ) -> Self {
        let freq = inner.frequency();
        let format = inner.format();
        let total_length = inner.length();

        let bytes_per_frame = match format {
            AudioFormat::Mono8 => 1,
            AudioFormat::Stereo8 | AudioFormat::Mono16 => 2,
            AudioFormat::Stereo16 => 4,
        };

        let start_sample = (start_time_ms as f64 / 1000.0 * freq as f64) as u32;

        // Compute limited length like C does:
        // length = total_length - startTime/1000
        // if runTime > 0 && runTime/1000 < length: length = runTime/1000
        let mut limited_length = total_length - start_time_ms as f32 / 1000.0;
        if limited_length < 0.0 {
            limited_length = 0.0;
        }
        if run_time_ms > 0 && (run_time_ms as f32 / 1000.0) < limited_length {
            limited_length = run_time_ms as f32 / 1000.0;
        }

        let end_sample = start_sample + (limited_length * freq as f32) as u32;

        // Seek to start position
        if start_sample > 0 {
            let _ = inner.seek(start_sample);
        }

        LimitedDecoder {
            inner,
            start_sample,
            end_sample,
            bytes_per_frame,
            current_sample: start_sample,
            limited_length,
        }
    }
}

impl SoundDecoder for LimitedDecoder {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn init_module(&mut self, flags: i32, formats: &DecoderFormats) -> bool {
        self.inner.init_module(flags, formats)
    }

    fn term_module(&mut self) {
        self.inner.term_module()
    }

    fn get_error(&mut self) -> i32 {
        self.inner.get_error()
    }

    fn init(&mut self) -> bool {
        self.inner.init()
    }

    fn term(&mut self) {
        self.inner.term()
    }

    fn open(&mut self, path: &Path) -> DecodeResult<()> {
        self.inner.open(path)
    }

    fn open_from_bytes(&mut self, data: &[u8], name: &str) -> DecodeResult<()> {
        self.inner.open_from_bytes(data, name)
    }

    fn close(&mut self) {
        self.inner.close()
    }

    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        if self.current_sample >= self.end_sample {
            return Err(DecodeError::EndOfFile);
        }

        // Limit buffer to remaining samples
        let remaining_samples = self.end_sample - self.current_sample;
        let remaining_bytes = remaining_samples as usize * self.bytes_per_frame as usize;
        let max_bytes = buf.len().min(remaining_bytes);

        if max_bytes == 0 {
            return Err(DecodeError::EndOfFile);
        }

        let result = self.inner.decode(&mut buf[..max_bytes]);
        if let Ok(n) = &result {
            self.current_sample += (*n as u32) / self.bytes_per_frame;
        }
        result
    }

    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        let target = self.start_sample + pcm_pos;
        let result = self.inner.seek(target)?;
        self.current_sample = result;
        Ok(result - self.start_sample)
    }

    fn get_frame(&self) -> u32 {
        self.inner.get_frame()
    }

    fn frequency(&self) -> u32 {
        self.inner.frequency()
    }

    fn format(&self) -> AudioFormat {
        self.inner.format()
    }

    fn length(&self) -> f32 {
        self.limited_length
    }

    fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    fn needs_swap(&self) -> bool {
        self.inner.needs_swap()
    }
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
