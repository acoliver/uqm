//! WAV decoder implementation
//!
//! Decodes WAV (RIFF WAVE) audio files. Supports:
//! - 8-bit and 16-bit PCM
//! - Mono and stereo
//! - Any sample rate
//!
//! Based on the C implementation in `sc2/src/libs/sound/decoders/wav.c`.

use std::io::{Cursor, Read, Seek, SeekFrom};

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};

// WAV format constants (little-endian IDs)
const RIFF_ID: u32 = 0x46464952; // "RIFF"
const WAVE_ID: u32 = 0x45564157; // "WAVE"
const FMT_ID: u32 = 0x20746d66; // "fmt "
const DATA_ID: u32 = 0x61746164; // "data"

// WAV format codes
const WAVE_FORMAT_PCM: u16 = 1;

/// WAV file header
#[derive(Debug, Default)]
struct WavFileHeader {
    id: u32,     // "RIFF"
    size: u32,   // File size - 8
    format: u32, // "WAVE"
}

/// WAV format chunk
#[derive(Debug, Default)]
struct WavFormatHeader {
    format: u16,          // 1 = PCM
    channels: u16,        // 1 = mono, 2 = stereo
    sample_rate: u32,     // Samples per second
    byte_rate: u32,       // bytes per second
    block_align: u16,     // bytes per sample frame
    bits_per_sample: u16, // 8 or 16
}

/// WAV chunk header
#[derive(Debug, Default)]
struct WavChunkHeader {
    id: u32,
    size: u32,
}

/// WAV decoder
pub struct WavDecoder {
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
    /// Audio data buffer
    data: Vec<u8>,
    /// Current read position in data
    data_pos: usize,
    /// Format header info
    fmt_header: WavFormatHeader,
    /// Whether we need to swap bytes (for big-endian output)
    need_swap: bool,
}

impl WavDecoder {
    /// Create a new WAV decoder
    pub fn new() -> Self {
        Self {
            frequency: 22050,
            format: AudioFormat::Stereo16,
            length: 0.0,
            last_error: 0,
            initialized: false,
            formats: None,
            data: Vec::new(),
            data_pos: 0,
            fmt_header: WavFormatHeader::default(),
            need_swap: false,
        }
    }

    /// Read a little-endian u16
    fn read_le_u16(cursor: &mut Cursor<&[u8]>) -> DecodeResult<u16> {
        let mut buf = [0u8; 2];
        cursor
            .read_exact(&mut buf)
            .map_err(|e| DecodeError::InvalidData(format!("Failed to read u16: {}", e)))?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Read a little-endian u32
    fn read_le_u32(cursor: &mut Cursor<&[u8]>) -> DecodeResult<u32> {
        let mut buf = [0u8; 4];
        cursor
            .read_exact(&mut buf)
            .map_err(|e| DecodeError::InvalidData(format!("Failed to read u32: {}", e)))?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Parse WAV file header
    fn parse_file_header(cursor: &mut Cursor<&[u8]>) -> DecodeResult<WavFileHeader> {
        let id = Self::read_le_u32(cursor)?;
        let size = Self::read_le_u32(cursor)?;
        let format = Self::read_le_u32(cursor)?;

        if id != RIFF_ID {
            return Err(DecodeError::InvalidData("Not a RIFF file".to_string()));
        }
        if format != WAVE_ID {
            return Err(DecodeError::InvalidData("Not a WAVE file".to_string()));
        }

        Ok(WavFileHeader { id, size, format })
    }

    /// Parse chunk header
    fn parse_chunk_header(cursor: &mut Cursor<&[u8]>) -> DecodeResult<WavChunkHeader> {
        let id = Self::read_le_u32(cursor)?;
        let size = Self::read_le_u32(cursor)?;
        Ok(WavChunkHeader { id, size })
    }

    /// Parse format chunk
    fn parse_format_header(cursor: &mut Cursor<&[u8]>, size: u32) -> DecodeResult<WavFormatHeader> {
        if size < 16 {
            return Err(DecodeError::InvalidData(
                "Format chunk too small".to_string(),
            ));
        }

        let format = Self::read_le_u16(cursor)?;
        let channels = Self::read_le_u16(cursor)?;
        let sample_rate = Self::read_le_u32(cursor)?;
        let byte_rate = Self::read_le_u32(cursor)?;
        let block_align = Self::read_le_u16(cursor)?;
        let bits_per_sample = Self::read_le_u16(cursor)?;

        // Skip any extra format bytes
        if size > 16 {
            cursor
                .seek(SeekFrom::Current((size - 16) as i64))
                .map_err(|e| {
                    DecodeError::InvalidData(format!("Failed to skip format bytes: {}", e))
                })?;
        }

        if format != WAVE_FORMAT_PCM {
            return Err(DecodeError::InvalidData(format!(
                "Unsupported WAV format: {} (only PCM supported)",
                format
            )));
        }

        if channels != 1 && channels != 2 {
            return Err(DecodeError::InvalidData(format!(
                "Unsupported channel count: {}",
                channels
            )));
        }

        if bits_per_sample != 8 && bits_per_sample != 16 {
            return Err(DecodeError::InvalidData(format!(
                "Unsupported bits per sample: {}",
                bits_per_sample
            )));
        }

        Ok(WavFormatHeader {
            format,
            channels,
            sample_rate,
            byte_rate,
            block_align,
            bits_per_sample,
        })
    }

    /// Determine AudioFormat from WAV header
    fn audio_format_from_header(header: &WavFormatHeader) -> AudioFormat {
        match (header.channels, header.bits_per_sample) {
            (1, 8) => AudioFormat::Mono8,
            (2, 8) => AudioFormat::Stereo8,
            (1, 16) => AudioFormat::Mono16,
            (2, 16) => AudioFormat::Stereo16,
            _ => AudioFormat::Stereo16, // fallback
        }
    }
}

impl Default for WavDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundDecoder for WavDecoder {
    fn name(&self) -> &'static str {
        "Wave"
    }

    fn init_module(&mut self, _flags: i32, formats: &DecoderFormats) -> bool {
        self.formats = Some(*formats);
        // WAV is little-endian, so we need swap if output wants big-endian
        self.need_swap = formats.want_big_endian;
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
        self.data.clear();
        self.data_pos = 0;
        true
    }

    fn term(&mut self) {
        self.close();
        self.initialized = false;
    }

    fn open(&mut self, path: &std::path::Path) -> DecodeResult<()> {
        let data = std::fs::read(path)
            .map_err(|e| DecodeError::NotFound(format!("{}: {}", path.display(), e)))?;

        self.open_from_bytes(&data, path.to_str().unwrap_or("unknown"))
    }

    fn open_from_bytes(&mut self, data: &[u8], _name: &str) -> DecodeResult<()> {
        let mut cursor = Cursor::new(data);

        // Parse file header
        let _file_header = Self::parse_file_header(&mut cursor)?;

        // Find and parse chunks
        let mut fmt_found = false;
        let mut data_offset = 0usize;
        let mut data_size = 0u32;

        while (cursor.position() as usize) < data.len() {
            let chunk = Self::parse_chunk_header(&mut cursor)?;

            match chunk.id {
                FMT_ID => {
                    self.fmt_header = Self::parse_format_header(&mut cursor, chunk.size)?;
                    fmt_found = true;
                }
                DATA_ID => {
                    data_offset = cursor.position() as usize;
                    data_size = chunk.size;
                    // Don't read past the data chunk
                    break;
                }
                _ => {
                    // Skip unknown chunk
                    cursor
                        .seek(SeekFrom::Current(chunk.size as i64))
                        .map_err(|e| {
                            DecodeError::InvalidData(format!("Failed to skip chunk: {}", e))
                        })?;
                }
            }
        }

        if !fmt_found {
            return Err(DecodeError::InvalidData(
                "No format chunk found".to_string(),
            ));
        }

        if data_offset == 0 || data_size == 0 {
            return Err(DecodeError::InvalidData("No data chunk found".to_string()));
        }

        // Copy audio data
        let end_offset = (data_offset + data_size as usize).min(data.len());
        self.data = data[data_offset..end_offset].to_vec();
        self.data_pos = 0;

        // Set format info
        self.frequency = self.fmt_header.sample_rate;
        self.format = Self::audio_format_from_header(&self.fmt_header);

        // Calculate length
        let bytes_per_sample = self.fmt_header.bits_per_sample as u32 / 8;
        let total_samples = data_size / (bytes_per_sample * self.fmt_header.channels as u32);
        self.length = total_samples as f32 / self.frequency as f32;

        Ok(())
    }

    fn close(&mut self) {
        self.data.clear();
        self.data_pos = 0;
    }

    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        if !self.initialized {
            return Err(DecodeError::NotInitialized);
        }

        if self.data.is_empty() {
            return Err(DecodeError::NotInitialized);
        }

        if self.data_pos >= self.data.len() {
            return Err(DecodeError::EndOfFile);
        }

        // Calculate how much to copy
        let available = self.data.len() - self.data_pos;
        let to_copy = buf.len().min(available);

        // Copy data
        buf[..to_copy].copy_from_slice(&self.data[self.data_pos..self.data_pos + to_copy]);
        self.data_pos += to_copy;

        // Swap bytes if needed (16-bit samples, big-endian output)
        if self.need_swap && self.fmt_header.bits_per_sample == 16 {
            for i in (0..to_copy).step_by(2) {
                if i + 1 < to_copy {
                    buf.swap(i, i + 1);
                }
            }
        }

        Ok(to_copy)
    }

    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        if !self.initialized {
            return Err(DecodeError::NotInitialized);
        }

        let bytes_per_sample = self.fmt_header.bits_per_sample as usize / 8;
        let bytes_per_frame = bytes_per_sample * self.fmt_header.channels as usize;
        let byte_pos = pcm_pos as usize * bytes_per_frame;

        if byte_pos >= self.data.len() {
            self.data_pos = self.data.len();
            return Ok((self.data.len() / bytes_per_frame) as u32);
        }

        self.data_pos = byte_pos;
        Ok(pcm_pos)
    }

    fn get_frame(&self) -> u32 {
        let bytes_per_sample = self.fmt_header.bits_per_sample as usize / 8;
        let bytes_per_frame = bytes_per_sample * self.fmt_header.channels as usize;
        if bytes_per_frame > 0 {
            (self.data_pos / bytes_per_frame) as u32
        } else {
            0
        }
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
        self.need_swap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_decoder_new() {
        let decoder = WavDecoder::new();
        assert_eq!(decoder.name(), "Wave");
        assert_eq!(decoder.frequency(), 22050);
        assert!(!decoder.is_null());
    }

    #[test]
    fn test_wav_decoder_init_term() {
        let mut decoder = WavDecoder::new();
        assert!(decoder.init());
        assert!(decoder.initialized);
        decoder.term();
        assert!(!decoder.initialized);
    }

    #[test]
    fn test_wav_decoder_init_module() {
        let mut decoder = WavDecoder::new();
        let formats = DecoderFormats::default();
        assert!(decoder.init_module(0, &formats));
        assert!(decoder.formats.is_some());
        decoder.term_module();
        assert!(decoder.formats.is_none());
    }

    #[test]
    fn test_wav_decoder_decode_not_initialized() {
        let mut decoder = WavDecoder::new();
        let mut buf = [0u8; 1024];
        let result = decoder.decode(&mut buf);
        assert!(matches!(result, Err(DecodeError::NotInitialized)));
    }

    #[test]
    fn test_wav_audio_format_from_header() {
        let mut header = WavFormatHeader::default();

        header.channels = 1;
        header.bits_per_sample = 8;
        assert_eq!(
            WavDecoder::audio_format_from_header(&header),
            AudioFormat::Mono8
        );

        header.channels = 2;
        header.bits_per_sample = 8;
        assert_eq!(
            WavDecoder::audio_format_from_header(&header),
            AudioFormat::Stereo8
        );

        header.channels = 1;
        header.bits_per_sample = 16;
        assert_eq!(
            WavDecoder::audio_format_from_header(&header),
            AudioFormat::Mono16
        );

        header.channels = 2;
        header.bits_per_sample = 16;
        assert_eq!(
            WavDecoder::audio_format_from_header(&header),
            AudioFormat::Stereo16
        );
    }

    #[test]
    fn test_wav_decoder_open_from_bytes_valid() {
        // Minimal valid WAV file: 44 bytes header + some data
        let wav_data: Vec<u8> = vec![
            // RIFF header
            0x52, 0x49, 0x46, 0x46, // "RIFF"
            0x28, 0x00, 0x00, 0x00, // file size - 8 = 40
            0x57, 0x41, 0x56, 0x45, // "WAVE"
            // fmt chunk
            0x66, 0x6d, 0x74, 0x20, // "fmt "
            0x10, 0x00, 0x00, 0x00, // chunk size = 16
            0x01, 0x00, // format = PCM
            0x01, 0x00, // channels = 1
            0x22, 0x56, 0x00, 0x00, // sample rate = 22050
            0x22, 0x56, 0x00, 0x00, // byte rate = 22050
            0x01, 0x00, // block align = 1
            0x08, 0x00, // bits per sample = 8
            // data chunk
            0x64, 0x61, 0x74, 0x61, // "data"
            0x04, 0x00, 0x00, 0x00, // data size = 4
            0x80, 0x80, 0x80, 0x80, // 4 bytes of silence (8-bit)
        ];

        let mut decoder = WavDecoder::new();
        decoder.init();

        let result = decoder.open_from_bytes(&wav_data, "test.wav");
        assert!(result.is_ok(), "Failed to open WAV: {:?}", result);

        assert_eq!(decoder.frequency(), 22050);
        assert_eq!(decoder.format(), AudioFormat::Mono8);
        assert!(decoder.length() > 0.0);
    }

    #[test]
    fn test_wav_decoder_decode_valid() {
        // Minimal valid WAV file with known data
        let wav_data: Vec<u8> = vec![
            0x52, 0x49, 0x46, 0x46, 0x28, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d,
            0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x22, 0x56, 0x00, 0x00,
            0x22, 0x56, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x04, 0x00,
            0x00, 0x00, 0x10, 0x20, 0x30, 0x40,
        ];

        let mut decoder = WavDecoder::new();
        decoder.init();
        decoder.open_from_bytes(&wav_data, "test.wav").unwrap();

        let mut buf = [0u8; 16];
        let result = decoder.decode(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 4);
        assert_eq!(&buf[..4], &[0x10, 0x20, 0x30, 0x40]);
    }

    #[test]
    fn test_wav_decoder_seek() {
        let wav_data: Vec<u8> = vec![
            0x52, 0x49, 0x46, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d,
            0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x22, 0x56, 0x00, 0x00,
            0x22, 0x56, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x08, 0x00,
            0x00, 0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
        ];

        let mut decoder = WavDecoder::new();
        decoder.init();
        decoder.open_from_bytes(&wav_data, "test.wav").unwrap();

        // Seek to sample 4
        let result = decoder.seek(4);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 4);

        // Read remaining samples
        let mut buf = [0u8; 16];
        let bytes = decoder.decode(&mut buf).unwrap();
        assert_eq!(bytes, 4);
        assert_eq!(&buf[..4], &[0x50, 0x60, 0x70, 0x80]);
    }

    #[test]
    fn test_wav_decoder_invalid_riff() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let mut decoder = WavDecoder::new();
        decoder.init();
        let result = decoder.open_from_bytes(&invalid_data, "test.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_wav_decoder_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<WavDecoder>();
    }
}
