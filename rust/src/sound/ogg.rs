//! Ogg Vorbis decoder implementation
//!
//! Uses the `lewton` crate for pure Rust Ogg Vorbis decoding.

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use lewton::inside_ogg::OggStreamReader;

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};

/// Calculate the duration of an Ogg Vorbis file by finding the last granule position
/// and the sample rate from the headers.
fn calculate_ogg_duration<R: Read + Seek>(data: &mut R, sample_rate: u32) -> f32 {
    // Seek to end to find the last Ogg page with granule position
    if data.seek(SeekFrom::End(0)).is_err() {
        return 0.0;
    }

    let file_size = match data.stream_position() {
        Ok(pos) => pos,
        Err(_) => return 0.0,
    };

    // Search backwards for "OggS" page marker in the last 64KB
    let search_size = std::cmp::min(65536, file_size) as usize;
    let search_start = file_size - search_size as u64;

    if data.seek(SeekFrom::Start(search_start)).is_err() {
        return 0.0;
    }

    let mut buffer = vec![0u8; search_size];
    if data.read_exact(&mut buffer).is_err() {
        return 0.0;
    }

    // Search backwards for last "OggS" marker
    let mut last_granule: Option<u64> = None;
    for i in (0..buffer.len().saturating_sub(27)).rev() {
        if buffer[i..].starts_with(b"OggS") {
            // Found an Ogg page header
            // Granule position is at offset 6-13 (8 bytes, little-endian)
            if i + 14 <= buffer.len() {
                let granule = u64::from_le_bytes([
                    buffer[i + 6],
                    buffer[i + 7],
                    buffer[i + 8],
                    buffer[i + 9],
                    buffer[i + 10],
                    buffer[i + 11],
                    buffer[i + 12],
                    buffer[i + 13],
                ]);
                // Granule position of -1 (0xFFFFFFFFFFFFFFFF) means "no granule"
                if granule != u64::MAX {
                    last_granule = Some(granule);
                    break;
                }
            }
        }
    }

    // Reset to beginning for normal reading
    let _ = data.seek(SeekFrom::Start(0));

    match last_granule {
        Some(granule) if sample_rate > 0 => granule as f32 / sample_rate as f32,
        _ => 0.0,
    }
}

/// Ogg Vorbis decoder using lewton
pub struct OggDecoder {
    /// Sample frequency in Hz
    frequency: u32,
    /// Audio format (mono/stereo, 8/16 bit)
    format: AudioFormat,
    /// Total length in seconds
    length: f32,
    /// Last error code
    last_error: i32,
    /// Whether the decoder is initialized
    initialized: bool,
    /// Stored formats for reference
    formats: Option<DecoderFormats>,
    /// The underlying Ogg stream reader (from file)
    reader_file: Option<OggStreamReader<BufReader<File>>>,
    /// The underlying Ogg stream reader (from bytes)
    reader_bytes: Option<OggStreamReader<Cursor<Vec<u8>>>>,
    /// Original data for seeking (bytes mode only)
    original_data: Option<Vec<u8>>,
    /// Decoded sample buffer (interleaved i16 samples)
    sample_buffer: Vec<i16>,
    /// Current position in sample buffer
    buffer_pos: usize,
    /// Current PCM sample position
    current_pcm: u64,
    /// Total PCM samples
    total_pcm: u64,
}

impl OggDecoder {
    /// Create a new Ogg Vorbis decoder
    pub fn new() -> Self {
        Self {
            frequency: 44100,
            format: AudioFormat::Stereo16,
            length: 0.0,
            last_error: 0,
            initialized: false,
            formats: None,
            reader_file: None,
            reader_bytes: None,
            original_data: None,
            sample_buffer: Vec::new(),
            buffer_pos: 0,
            current_pcm: 0,
            total_pcm: 0,
        }
    }

    /// Decode the next packet and fill the sample buffer
    fn decode_next_packet(&mut self) -> DecodeResult<bool> {
        // Try to read from file reader first, then bytes reader
        let packet = if let Some(ref mut reader) = self.reader_file {
            match reader.read_dec_packet_itl() {
                Ok(Some(samples)) => Some(samples),
                Ok(None) => None,
                Err(e) => {
                    self.last_error = -1;
                    return Err(DecodeError::DecoderError(format!(
                        "Ogg decode error: {:?}",
                        e
                    )));
                }
            }
        } else if let Some(ref mut reader) = self.reader_bytes {
            match reader.read_dec_packet_itl() {
                Ok(Some(samples)) => Some(samples),
                Ok(None) => None,
                Err(e) => {
                    self.last_error = -1;
                    return Err(DecodeError::DecoderError(format!(
                        "Ogg decode error: {:?}",
                        e
                    )));
                }
            }
        } else {
            return Err(DecodeError::NotInitialized);
        };

        match packet {
            Some(samples) => {
                self.sample_buffer = samples;
                self.buffer_pos = 0;
                Ok(true)
            }
            None => Ok(false), // End of stream
        }
    }
}

impl Default for OggDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundDecoder for OggDecoder {
    fn name(&self) -> &'static str {
        "Ogg Vorbis"
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
        self.sample_buffer.clear();
        self.buffer_pos = 0;
        true
    }

    fn term(&mut self) {
        self.close();
        self.initialized = false;
    }

    fn open(&mut self, path: &Path) -> DecodeResult<()> {
        let file = File::open(path)
            .map_err(|e| DecodeError::NotFound(format!("{}: {}", path.display(), e)))?;

        let reader = BufReader::new(file);
        let ogg_reader = OggStreamReader::new(reader)
            .map_err(|e| DecodeError::InvalidData(format!("Failed to open Ogg stream: {:?}", e)))?;

        self.frequency = ogg_reader.ident_hdr.audio_sample_rate;
        self.format = if ogg_reader.ident_hdr.audio_channels == 1 {
            AudioFormat::Mono16
        } else {
            AudioFormat::Stereo16
        };

        // Estimate length (lewton doesn't provide total samples easily)
        // We'll calculate it as we decode or use a reasonable estimate
        self.length = 0.0; // Will be updated if seeking is supported
        self.total_pcm = 0;
        self.current_pcm = 0;
        self.sample_buffer.clear();
        self.buffer_pos = 0;

        self.reader_file = Some(ogg_reader);
        self.reader_bytes = None;

        Ok(())
    }

    fn open_from_bytes(&mut self, data: &[u8], _name: &str) -> DecodeResult<()> {
        // Store original data for seeking/rewind
        let data_vec = data.to_vec();

        // Calculate duration from the raw data before creating the reader
        let mut cursor_for_duration = Cursor::new(data_vec.clone());

        // First pass: get sample rate from headers
        let temp_cursor = Cursor::new(data_vec.clone());
        let temp_reader = OggStreamReader::new(temp_cursor).map_err(|e| {
            DecodeError::InvalidData(format!("Failed to open Ogg stream from bytes: {:?}", e))
        })?;
        let sample_rate = temp_reader.ident_hdr.audio_sample_rate;
        let channels = temp_reader.ident_hdr.audio_channels;
        drop(temp_reader);

        // Calculate duration
        let duration = calculate_ogg_duration(&mut cursor_for_duration, sample_rate);

        // Now create the actual reader
        let cursor = Cursor::new(data_vec.clone());
        let ogg_reader = OggStreamReader::new(cursor).map_err(|e| {
            DecodeError::InvalidData(format!("Failed to open Ogg stream from bytes: {:?}", e))
        })?;

        self.frequency = sample_rate;
        self.format = if channels == 1 {
            AudioFormat::Mono16
        } else {
            AudioFormat::Stereo16
        };

        self.length = duration;
        self.total_pcm = (duration * sample_rate as f32) as u64;
        self.current_pcm = 0;
        self.sample_buffer.clear();
        self.buffer_pos = 0;

        self.original_data = Some(data_vec);
        self.reader_bytes = Some(ogg_reader);
        self.reader_file = None;

        Ok(())
    }

    fn close(&mut self) {
        self.reader_file = None;
        self.reader_bytes = None;
        self.original_data = None;
        self.sample_buffer.clear();
        self.buffer_pos = 0;
        self.current_pcm = 0;
    }

    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        if !self.initialized {
            return Err(DecodeError::NotInitialized);
        }

        if self.reader_file.is_none() && self.reader_bytes.is_none() {
            return Err(DecodeError::NotInitialized);
        }

        let mut bytes_written = 0;
        let bytes_per_sample = 2; // i16

        while bytes_written < buf.len() {
            // If buffer is empty or exhausted, decode next packet
            if self.buffer_pos >= self.sample_buffer.len() {
                match self.decode_next_packet()? {
                    true => {} // Got more samples
                    false => {
                        // End of stream
                        if bytes_written == 0 {
                            return Err(DecodeError::EndOfFile);
                        }
                        break;
                    }
                }
            }

            // Copy samples from buffer to output
            let samples_available = self.sample_buffer.len() - self.buffer_pos;
            let bytes_available = samples_available * bytes_per_sample;
            let bytes_needed = buf.len() - bytes_written;
            let bytes_to_copy = bytes_available.min(bytes_needed);
            let samples_to_copy = bytes_to_copy / bytes_per_sample;

            // Convert i16 samples to bytes (little-endian)
            for i in 0..samples_to_copy {
                let sample = self.sample_buffer[self.buffer_pos + i];
                let sample_bytes = sample.to_le_bytes();
                let offset = bytes_written + i * bytes_per_sample;
                buf[offset] = sample_bytes[0];
                buf[offset + 1] = sample_bytes[1];
            }

            self.buffer_pos += samples_to_copy;
            bytes_written += samples_to_copy * bytes_per_sample;
            self.current_pcm += samples_to_copy as u64 / self.format.channels() as u64;
        }

        Ok(bytes_written)
    }

    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        // lewton doesn't support native seeking, so we implement it by:
        // 1. Rewinding to start (reopening the stream)
        // 2. Decoding and discarding samples until we reach the target position

        let target_pcm = pcm_pos as u64;

        // If seeking backwards or to start, we need to rewind first
        if target_pcm <= self.current_pcm || pcm_pos == 0 {
            self.sample_buffer.clear();
            self.buffer_pos = 0;
            self.current_pcm = 0;

            // Reopen from original data if we have it (bytes mode)
            if let Some(ref data) = self.original_data {
                let cursor = Cursor::new(data.clone());
                match OggStreamReader::new(cursor) {
                    Ok(ogg_reader) => {
                        self.reader_bytes = Some(ogg_reader);
                    }
                    Err(e) => {
                        return Err(DecodeError::SeekFailed(format!(
                            "Failed to rewind: {:?}",
                            e
                        )));
                    }
                }
            }
            // For file mode, we'd need to reopen the file - not implemented yet

            if pcm_pos == 0 {
                return Ok(0);
            }
        }

        // Now skip forward by decoding and discarding until we reach target
        let channels = self.format.channels() as u64;
        while self.current_pcm < target_pcm {
            // Decode next packet
            match self.decode_next_packet() {
                Ok(true) => {
                    // Calculate how many PCM frames are in this buffer
                    let samples_in_buffer = self.sample_buffer.len() as u64;
                    let frames_in_buffer = samples_in_buffer / channels;

                    let remaining_to_skip = target_pcm - self.current_pcm;

                    if frames_in_buffer <= remaining_to_skip {
                        // Skip entire buffer
                        self.current_pcm += frames_in_buffer;
                        self.buffer_pos = self.sample_buffer.len();
                    } else {
                        // Partial skip - position within this buffer
                        let samples_to_skip = (remaining_to_skip * channels) as usize;
                        self.buffer_pos = samples_to_skip;
                        self.current_pcm = target_pcm;
                    }
                }
                Ok(false) => {
                    // End of stream reached before target
                    return Ok(self.current_pcm as u32);
                }
                Err(e) => {
                    return Err(DecodeError::SeekFailed(format!(
                        "Seek failed during skip: {}",
                        e
                    )));
                }
            }
        }

        Ok(self.current_pcm as u32)
    }

    fn get_frame(&self) -> u32 {
        // Ogg doesn't have traditional frames like video
        // Return the current granule position approximation
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
    fn test_ogg_decoder_new() {
        let decoder = OggDecoder::new();
        assert_eq!(decoder.name(), "Ogg Vorbis");
        assert_eq!(decoder.frequency(), 44100);
        assert_eq!(decoder.format(), AudioFormat::Stereo16);
        assert!(!decoder.is_null());
        assert!(!decoder.needs_swap());
    }

    #[test]
    fn test_ogg_decoder_init_term() {
        let mut decoder = OggDecoder::new();
        assert!(decoder.init());
        assert!(decoder.initialized);
        decoder.term();
        assert!(!decoder.initialized);
    }

    #[test]
    fn test_ogg_decoder_init_module() {
        let mut decoder = OggDecoder::new();
        let formats = DecoderFormats::default();
        assert!(decoder.init_module(0, &formats));
        assert!(decoder.formats.is_some());
        decoder.term_module();
        assert!(decoder.formats.is_none());
    }

    #[test]
    fn test_ogg_decoder_decode_not_initialized() {
        let mut decoder = OggDecoder::new();
        let mut buf = [0u8; 1024];
        let result = decoder.decode(&mut buf);
        assert!(matches!(result, Err(DecodeError::NotInitialized)));
    }

    #[test]
    fn test_ogg_decoder_open_nonexistent() {
        let mut decoder = OggDecoder::new();
        decoder.init();
        let result = decoder.open(Path::new("/nonexistent/file.ogg"));
        assert!(matches!(result, Err(DecodeError::NotFound(_))));
    }

    #[test]
    fn test_ogg_decoder_get_error() {
        let mut decoder = OggDecoder::new();
        decoder.last_error = 42;
        assert_eq!(decoder.get_error(), 42);
        assert_eq!(decoder.get_error(), 0); // Should be cleared
    }

    #[test]
    fn test_ogg_decoder_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<OggDecoder>();
    }

    #[test]
    fn test_ogg_decoder_seek_to_zero() {
        let mut decoder = OggDecoder::new();
        decoder.init();
        decoder.current_pcm = 1000;
        let result = decoder.seek(0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(decoder.current_pcm, 0);
    }

    #[test]
    fn test_ogg_decoder_seek_unsupported() {
        let mut decoder = OggDecoder::new();
        decoder.init();
        let result = decoder.seek(1000);
        assert!(matches!(result, Err(DecodeError::SeekFailed(_))));
    }

    // Integration test with real file would go here
    // #[test]
    // fn test_ogg_decoder_decode_real_file() {
    //     let mut decoder = OggDecoder::new();
    //     decoder.init();
    //     decoder.open(Path::new("test_fixtures/test.ogg")).unwrap();
    //     let mut buf = [0u8; 4096];
    //     let bytes = decoder.decode(&mut buf).unwrap();
    //     assert!(bytes > 0);
    // }
}
