//! AIFF/AIFC decoder implementation
//!
//! Decodes AIFF and AIFF-C audio files. Supports:
//! - PCM (uncompressed) 8-bit and 16-bit
//! - SDX2 ADPCM compressed audio
//! - Mono and stereo (up to 4 channels)
//! - IEEE 754 80-bit extended precision sample rates
//!
//! Based on the C implementation in `sc2/src/libs/sound/decoders/aiffaud.c`.
//!
//! @plan PLAN-20260225-AIFF-DECODER.P03
//! @requirement REQ-FP-1, REQ-SV-1, REQ-CH-1, REQ-LF-1, REQ-EH-1

// Stub phase: constants, types, and helpers are defined but not yet used.
// These warnings will resolve as implementation phases consume them.
#![allow(dead_code, unused_imports)]

use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use super::decoder::{DecodeError, DecodeResult, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};

// AIFF format constants (big-endian IDs)
const FORM_ID: u32 = 0x464F524D; // "FORM"
const FORM_TYPE_AIFF: u32 = 0x41494646; // "AIFF"
const FORM_TYPE_AIFC: u32 = 0x41494643; // "AIFC"
const COMMON_ID: u32 = 0x434F4D4D; // "COMM"
const SOUND_DATA_ID: u32 = 0x53534E44; // "SSND"
const SDX2_COMPRESSION: u32 = 0x53445832; // "SDX2"

const AIFF_COMM_SIZE: u32 = 18;
const AIFF_EXT_COMM_SIZE: u32 = 22;
const AIFF_SSND_SIZE: u32 = 8;
const MAX_CHANNELS: usize = 4;
const MIN_SAMPLE_RATE: i32 = 300;
const MAX_SAMPLE_RATE: i32 = 96000;
const MAX_FILE_SIZE: usize = 64 * 1024 * 1024;

/// Compression type for AIFF-C files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// Uncompressed PCM (AIFF or AIFC with "NONE")
    None,
    /// SDX2 ADPCM compression
    Sdx2,
}

impl Default for CompressionType {
    fn default() -> Self {
        CompressionType::None
    }
}

/// COMM chunk data from AIFF header
#[derive(Debug, Default, Clone)]
struct CommonChunk {
    channels: u16,
    sample_frames: u32,
    sample_size: u16,
    sample_rate: i32,
    ext_type_id: u32,
}

/// SSND chunk header
#[derive(Debug, Default)]
struct SoundDataHeader {
    offset: u32,
    block_size: u32,
}

/// Generic chunk header
#[derive(Debug, Default)]
struct ChunkHeader {
    id: u32,
    size: u32,
}

/// AIFF/AIFC decoder
pub struct AiffDecoder {
    /// Sample frequency in Hz
    frequency: u32,
    /// Audio format (mono/stereo, 8/16 bit)
    format: AudioFormat,
    /// Total length in seconds
    length: f32,
    /// Whether byte swapping is needed for output
    need_swap: bool,
    /// Last error code (get-and-clear via get_error)
    last_error: i32,
    /// Stored decoder formats from init_module
    formats: Option<DecoderFormats>,
    /// Whether init() has been called
    initialized: bool,
    /// Parsed COMM chunk data
    common: CommonChunk,
    /// Detected compression type
    comp_type: CompressionType,
    /// Bits per sample from COMM chunk
    bits_per_sample: u16,
    /// Bytes per sample frame (channels * bytes_per_sample)
    block_align: u16,
    /// Bytes per PCM frame in the file (for SDX2: channels * 1)
    file_block: u16,
    /// Raw audio data (entire SSND payload loaded in memory)
    data: Vec<u8>,
    /// Current read position in data
    data_pos: usize,
    /// Total PCM frames available
    max_pcm: u32,
    /// Current PCM frame position
    cur_pcm: u32,
    /// SDX2 predictor values per channel
    prev_val: [i16; MAX_CHANNELS],
}

impl AiffDecoder {
    /// Create a new AIFF decoder with default state
    pub fn new() -> Self {
        Self {
            frequency: 0,
            format: AudioFormat::Stereo16,
            length: 0.0,
            need_swap: false,
            last_error: 0,
            formats: None,
            initialized: false,
            common: CommonChunk::default(),
            comp_type: CompressionType::None,
            bits_per_sample: 0,
            block_align: 0,
            file_block: 0,
            data: Vec::new(),
            data_pos: 0,
            max_pcm: 0,
            cur_pcm: 0,
            prev_val: [0; MAX_CHANNELS],
        }
    }
}

// --- Private helper functions (stubs for P05) ---

fn read_be_u16(_cursor: &mut Cursor<&[u8]>) -> DecodeResult<u16> {
    todo!("P05: parser impl")
}

fn read_be_u32(_cursor: &mut Cursor<&[u8]>) -> DecodeResult<u32> {
    todo!("P05: parser impl")
}

fn read_be_i16(_cursor: &mut Cursor<&[u8]>) -> DecodeResult<i16> {
    todo!("P05: parser impl")
}

fn read_be_f80(_cursor: &mut Cursor<&[u8]>) -> DecodeResult<i32> {
    todo!("P05: parser impl — IEEE 754 80-bit extended precision")
}

fn read_chunk_header(_cursor: &mut Cursor<&[u8]>) -> DecodeResult<ChunkHeader> {
    todo!("P05: parser impl")
}

impl AiffDecoder {
    fn read_common_chunk(
        &mut self,
        _cursor: &mut Cursor<&[u8]>,
        _chunk_size: u32,
    ) -> DecodeResult<CommonChunk> {
        todo!("P05: parser impl")
    }

    fn read_sound_data_header(
        &mut self,
        _cursor: &mut Cursor<&[u8]>,
    ) -> DecodeResult<SoundDataHeader> {
        todo!("P05: parser impl")
    }
}

impl SoundDecoder for AiffDecoder {
    fn name(&self) -> &'static str {
        "AIFF"
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
        let fmts = match self.formats.as_ref() {
            Some(f) => f,
            None => return false,
        };
        self.need_swap = !fmts.want_big_endian;
        self.initialized = true;
        true
    }

    fn term(&mut self) {
        self.close();
    }

    fn open(&mut self, path: &Path) -> DecodeResult<()> {
        let data = std::fs::read(path).map_err(|e| DecodeError::IoError(e.to_string()))?;
        let name = path.to_string_lossy();
        self.open_from_bytes(&data, &name)
    }

    fn open_from_bytes(&mut self, _data: &[u8], _name: &str) -> DecodeResult<()> {
        todo!("P05: parser impl — AIFF header parsing and validation")
    }

    fn close(&mut self) {
        self.data = Vec::new();
        self.data_pos = 0;
        self.max_pcm = 0;
        self.cur_pcm = 0;
        self.prev_val = [0; MAX_CHANNELS];
        self.common = CommonChunk::default();
        self.comp_type = CompressionType::None;
        self.bits_per_sample = 0;
        self.block_align = 0;
        self.file_block = 0;
        self.frequency = 0;
        self.format = AudioFormat::Stereo16;
        self.length = 0.0;
    }

    fn decode(&mut self, _buf: &mut [u8]) -> DecodeResult<usize> {
        todo!("P08/P11: PCM and SDX2 decode impl")
    }

    fn seek(&mut self, _pcm_pos: u32) -> DecodeResult<u32> {
        todo!("P14: seek impl")
    }

    fn get_frame(&self) -> u32 {
        0
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
    fn test_new_decoder() {
        let dec = AiffDecoder::new();
        assert_eq!(dec.frequency, 0);
        assert_eq!(dec.format, AudioFormat::Stereo16);
        assert_eq!(dec.length, 0.0);
        assert!(!dec.need_swap);
        assert_eq!(dec.last_error, 0);
        assert!(dec.formats.is_none());
        assert!(!dec.initialized);
        assert_eq!(dec.comp_type, CompressionType::None);
        assert_eq!(dec.prev_val, [0; MAX_CHANNELS]);
    }

    #[test]
    fn test_name() {
        let dec = AiffDecoder::new();
        assert_eq!(dec.name(), "AIFF");
    }

    #[test]
    fn test_get_error_clears() {
        let mut dec = AiffDecoder::new();
        dec.last_error = -2;
        assert_eq!(dec.get_error(), -2);
        assert_eq!(dec.get_error(), 0);
    }

    #[test]
    fn test_init_module_stores_formats() {
        let mut dec = AiffDecoder::new();
        let formats = DecoderFormats::default();
        assert!(dec.init_module(0, &formats));
        assert!(dec.formats.is_some());
    }

    #[test]
    fn test_term_module_clears_formats() {
        let mut dec = AiffDecoder::new();
        let formats = DecoderFormats::default();
        dec.init_module(0, &formats);
        dec.term_module();
        assert!(dec.formats.is_none());
    }

    #[test]
    fn test_init_sets_need_swap() {
        let mut dec = AiffDecoder::new();
        let formats = DecoderFormats::default(); // want_big_endian = false
        dec.init_module(0, &formats);
        assert!(dec.init());
        assert!(dec.need_swap); // !false = true
        assert!(dec.initialized);
    }

    #[test]
    fn test_init_without_formats_fails() {
        let mut dec = AiffDecoder::new();
        assert!(!dec.init());
        assert!(!dec.initialized);
    }

    #[test]
    fn test_close_resets_state() {
        let mut dec = AiffDecoder::new();
        dec.frequency = 44100;
        dec.length = 5.0;
        dec.data = vec![1, 2, 3];
        dec.data_pos = 10;
        dec.max_pcm = 100;
        dec.cur_pcm = 50;
        dec.prev_val = [1, 2, 3, 4];
        dec.comp_type = CompressionType::Sdx2;
        dec.close();
        assert_eq!(dec.frequency, 0);
        assert_eq!(dec.length, 0.0);
        assert!(dec.data.is_empty());
        assert_eq!(dec.data_pos, 0);
        assert_eq!(dec.max_pcm, 0);
        assert_eq!(dec.cur_pcm, 0);
        assert_eq!(dec.prev_val, [0; MAX_CHANNELS]);
        assert_eq!(dec.comp_type, CompressionType::None);
    }

    #[test]
    fn test_accessors() {
        let mut dec = AiffDecoder::new();
        dec.frequency = 22050;
        dec.format = AudioFormat::Mono8;
        dec.length = 3.5;
        dec.need_swap = true;

        assert_eq!(dec.frequency(), 22050);
        assert_eq!(dec.format(), AudioFormat::Mono8);
        assert_eq!(dec.length(), 3.5);
        assert!(dec.needs_swap());
        assert!(!dec.is_null());
        assert_eq!(dec.get_frame(), 0);
    }

    #[test]
    fn test_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AiffDecoder>();
    }
}
