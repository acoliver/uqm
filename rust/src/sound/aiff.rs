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

// @plan PLAN-20260225-AIFF-DECODER.P04
// @requirement REQ-FP-1..15, REQ-SV-1..6, REQ-CH-1..4, REQ-EH-1..4
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // --- P03 stub tests (kept from Phase 03) ---

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

    // --- P04 TDD tests: test helper and parser tests ---

    /// Build a synthetic AIFF file for testing.
    ///
    /// Returns a byte vector containing a valid (or intentionally invalid) AIFF file.
    struct AiffBuilder {
        form_type: u32, // AIFF or AIFC
        channels: u16,
        sample_frames: u32,
        sample_size: u16,
        sample_rate_bytes: [u8; 10],       // raw IEEE 754 80-bit
        ext_type_id: Option<u32>,          // AIFC compression type
        ssnd_data: Option<Vec<u8>>,        // None = no SSND chunk
        extra_chunks: Vec<(u32, Vec<u8>)>, // additional chunks before SSND
    }

    impl AiffBuilder {
        fn new() -> Self {
            Self {
                form_type: FORM_TYPE_AIFF,
                channels: 1,
                sample_frames: 100,
                sample_size: 16,
                // 44100 Hz: biased_exp=0x400E, significand=0xAC44_0000_0000_0000
                sample_rate_bytes: [0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                ext_type_id: None,
                ssnd_data: Some(vec![0u8; 200]), // 100 frames * 1 channel * 2 bytes
                extra_chunks: Vec::new(),
            }
        }

        fn form_type(mut self, ft: u32) -> Self {
            self.form_type = ft;
            self
        }

        fn channels(mut self, ch: u16) -> Self {
            self.channels = ch;
            self
        }

        fn sample_frames(mut self, sf: u32) -> Self {
            self.sample_frames = sf;
            self
        }

        fn sample_size(mut self, ss: u16) -> Self {
            self.sample_size = ss;
            self
        }

        fn sample_rate_bytes(mut self, sr: [u8; 10]) -> Self {
            self.sample_rate_bytes = sr;
            self
        }

        fn ext_type_id(mut self, et: u32) -> Self {
            self.ext_type_id = Some(et);
            self
        }

        fn ssnd_data(mut self, data: Vec<u8>) -> Self {
            self.ssnd_data = Some(data);
            self
        }

        fn no_ssnd(mut self) -> Self {
            self.ssnd_data = None;
            self
        }

        fn extra_chunk(mut self, id: u32, data: Vec<u8>) -> Self {
            self.extra_chunks.push((id, data));
            self
        }

        fn build(self) -> Vec<u8> {
            let mut chunks = Vec::new();

            // COMM chunk
            let comm_data_size = if self.ext_type_id.is_some() {
                22u32
            } else {
                18u32
            };
            chunks.extend_from_slice(&COMMON_ID.to_be_bytes());
            chunks.extend_from_slice(&comm_data_size.to_be_bytes());
            chunks.extend_from_slice(&self.channels.to_be_bytes());
            chunks.extend_from_slice(&self.sample_frames.to_be_bytes());
            chunks.extend_from_slice(&self.sample_size.to_be_bytes());
            chunks.extend_from_slice(&self.sample_rate_bytes);
            if let Some(et) = self.ext_type_id {
                chunks.extend_from_slice(&et.to_be_bytes());
            }
            // Pad COMM to even boundary
            if (comm_data_size % 2) != 0 {
                chunks.push(0);
            }

            // Extra chunks
            for (id, data) in &self.extra_chunks {
                chunks.extend_from_slice(&id.to_be_bytes());
                chunks.extend_from_slice(&(data.len() as u32).to_be_bytes());
                chunks.extend_from_slice(data);
                if (data.len() % 2) != 0 {
                    chunks.push(0); // pad to even
                }
            }

            // SSND chunk
            if let Some(ref audio_data) = self.ssnd_data {
                let ssnd_size = 8 + audio_data.len() as u32; // offset(4) + block_size(4) + data
                chunks.extend_from_slice(&SOUND_DATA_ID.to_be_bytes());
                chunks.extend_from_slice(&ssnd_size.to_be_bytes());
                chunks.extend_from_slice(&0u32.to_be_bytes()); // offset
                chunks.extend_from_slice(&0u32.to_be_bytes()); // block_size
                chunks.extend_from_slice(audio_data);
                if (audio_data.len() % 2) != 0 {
                    chunks.push(0);
                }
            }

            // FORM header
            let form_size = 4 + chunks.len() as u32; // form_type + chunks
            let mut file = Vec::new();
            file.extend_from_slice(&FORM_ID.to_be_bytes());
            file.extend_from_slice(&form_size.to_be_bytes());
            file.extend_from_slice(&self.form_type.to_be_bytes());
            file.extend_from_slice(&chunks);
            file
        }
    }

    /// Create a ready-to-use decoder with formats initialized
    fn make_decoder() -> AiffDecoder {
        let mut dec = AiffDecoder::new();
        let formats = DecoderFormats::default();
        dec.init_module(0, &formats);
        dec.init();
        dec
    }

    // --- Test 1: Valid mono 16-bit PCM AIFF ---
    #[test]
    fn test_parse_valid_aiff_mono16() {
        let data = AiffBuilder::new()
            .channels(1)
            .sample_size(16)
            .sample_frames(100)
            .ssnd_data(vec![0u8; 200]) // 100 * 1 * 2
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "test_mono16.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.frequency(), 44100);
        assert_eq!(dec.format(), AudioFormat::Mono16);
    }

    // --- Test 2: Valid stereo 16-bit PCM AIFF ---
    #[test]
    fn test_parse_valid_aiff_stereo16() {
        let data = AiffBuilder::new()
            .channels(2)
            .sample_size(16)
            .sample_frames(100)
            .ssnd_data(vec![0u8; 400]) // 100 * 2 * 2
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "test_stereo16.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.frequency(), 44100);
        assert_eq!(dec.format(), AudioFormat::Stereo16);
    }

    // --- Test 3: Valid mono 8-bit PCM AIFF ---
    #[test]
    fn test_parse_valid_aiff_mono8() {
        let data = AiffBuilder::new()
            .channels(1)
            .sample_size(8)
            .sample_frames(100)
            .ssnd_data(vec![0u8; 100]) // 100 * 1 * 1
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "test_mono8.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.frequency(), 44100);
        assert_eq!(dec.format(), AudioFormat::Mono8);
    }

    // --- Test 4: Valid stereo 8-bit PCM AIFF ---
    #[test]
    fn test_parse_valid_aiff_stereo8() {
        let data = AiffBuilder::new()
            .channels(2)
            .sample_size(8)
            .sample_frames(100)
            .ssnd_data(vec![0u8; 200]) // 100 * 2 * 1
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "test_stereo8.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.frequency(), 44100);
        assert_eq!(dec.format(), AudioFormat::Stereo8);
    }

    // --- Test 5: Valid AIFC with SDX2 compression ---
    #[test]
    fn test_parse_valid_aifc_sdx2() {
        let data = AiffBuilder::new()
            .form_type(FORM_TYPE_AIFC)
            .channels(2)
            .sample_size(16)
            .sample_frames(100)
            .ext_type_id(SDX2_COMPRESSION)
            .ssnd_data(vec![0u8; 200]) // SDX2: 100 * 2 * 1 compressed byte each
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "test_sdx2.aifc");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.comp_type, CompressionType::Sdx2);
    }

    // --- Test 6: Non-FORM header ---
    #[test]
    fn test_reject_non_form_header() {
        let mut data = AiffBuilder::new().build();
        // Corrupt the FORM ID
        data[0] = b'X';
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "bad_header.aiff");
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // --- Test 7: Wrong form type ---
    #[test]
    fn test_reject_non_aiff_form_type() {
        let data = AiffBuilder::new()
            .form_type(0x57415645) // "WAVE" instead of "AIFF"
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "wave_file.wav");
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // --- Test 8: COMM chunk too small ---
    #[test]
    fn test_reject_small_comm_chunk() {
        // Build a file manually with a tiny COMM chunk
        let mut file = Vec::new();
        file.extend_from_slice(&FORM_ID.to_be_bytes());
        file.extend_from_slice(&20u32.to_be_bytes()); // form size
        file.extend_from_slice(&FORM_TYPE_AIFF.to_be_bytes());
        file.extend_from_slice(&COMMON_ID.to_be_bytes());
        file.extend_from_slice(&4u32.to_be_bytes()); // COMM size = 4 (too small, need 18)
        file.extend_from_slice(&[0u8; 4]); // 4 bytes of data
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&file, "tiny_comm.aiff");
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
        assert_eq!(dec.last_error, -2);
    }

    // --- Test 9: Zero sample frames ---
    #[test]
    fn test_reject_zero_sample_frames() {
        let data = AiffBuilder::new().sample_frames(0).build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "zero_frames.aiff");
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // --- Test 10: No SSND chunk ---
    #[test]
    fn test_reject_no_ssnd_chunk() {
        let data = AiffBuilder::new().no_ssnd().build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "no_ssnd.aiff");
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // --- Test 11: Unsupported bits per sample ---
    #[test]
    fn test_reject_unsupported_bits_per_sample() {
        let data = AiffBuilder::new()
            .sample_size(24) // >16
            .ssnd_data(vec![0u8; 300])
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "24bit.aiff");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 12: Unsupported channel count ---
    #[test]
    fn test_reject_unsupported_channels() {
        let data = AiffBuilder::new()
            .channels(3) // not 1 or 2
            .ssnd_data(vec![0u8; 600])
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "3ch.aiff");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 13: Sample rate too low ---
    #[test]
    fn test_reject_sample_rate_too_low() {
        // 200 Hz: biased_exp = 7 + 16383 = 0x3FF0, significand = 200 << 56 = 0xC800_0000_0000_0000
        let data = AiffBuilder::new()
            .sample_rate_bytes([0x40, 0x06, 0xC8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "low_rate.aiff");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 14: Sample rate too high ---
    #[test]
    fn test_reject_sample_rate_too_high() {
        // 200000 Hz: biased_exp = 17 + 16383 = 0x4011, significand = 200000 << 46
        // 200000 = 0x30D40, log2 ≈ 17.6, floor = 17
        // significand = 200000 << (63 - 17) = 200000 << 46
        let data = AiffBuilder::new()
            .sample_rate_bytes([0x40, 0x10, 0xC3, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "high_rate.aiff");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 15: AIFF with ext_type_id != 0 ---
    #[test]
    fn test_reject_aiff_with_extension() {
        // Plain AIFF (not AIFC) should not have a non-zero compression type
        let data = AiffBuilder::new()
            .form_type(FORM_TYPE_AIFF)
            .ext_type_id(SDX2_COMPRESSION) // unexpected for plain AIFF
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "aiff_with_ext.aiff");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 16: AIFC with unknown compression ---
    #[test]
    fn test_reject_aifc_unknown_compression() {
        let data = AiffBuilder::new()
            .form_type(FORM_TYPE_AIFC)
            .ext_type_id(0x554C4157) // "ULAW" — not supported
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "ulaw.aifc");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 17: f80 known sample rates ---
    #[test]
    fn test_f80_known_rates() {
        let cases: &[([u8; 10], i32)] = &[
            (
                [0x40, 0x0D, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                22050,
            ),
            (
                [0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                44100,
            ),
            (
                [0x40, 0x0E, 0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                48000,
            ),
            (
                [0x40, 0x0B, 0xFA, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                8000,
            ),
            (
                [0x40, 0x0C, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                11025,
            ),
            (
                [0x40, 0x0F, 0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                96000,
            ),
        ];
        for (bytes, expected) in cases {
            let mut cursor = Cursor::new(bytes.as_slice());
            let result = read_be_f80(&mut cursor);
            assert_eq!(
                result,
                Ok(*expected),
                "f80 for rate {} failed: got {:?}",
                expected,
                result
            );
        }
    }

    // --- Test 18: f80 zero ---
    #[test]
    fn test_f80_zero() {
        let bytes = [0x00u8; 10];
        let mut cursor = Cursor::new(bytes.as_slice());
        assert_eq!(read_be_f80(&mut cursor), Ok(0));
    }

    // --- Test 19: f80 denormalized returns zero ---
    #[test]
    fn test_f80_denormalized_returns_zero() {
        let bytes: [u8; 10] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00];
        let mut cursor = Cursor::new(bytes.as_slice());
        assert_eq!(read_be_f80(&mut cursor), Ok(0));
    }

    // --- Test 20: f80 infinity returns error ---
    #[test]
    fn test_f80_infinity_returns_error() {
        let bytes: [u8; 10] = [0x7F, 0xFF, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut cursor = Cursor::new(bytes.as_slice());
        let result = read_be_f80(&mut cursor);
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // --- Test 21: f80 NaN returns error ---
    #[test]
    fn test_f80_nan_returns_error() {
        let bytes: [u8; 10] = [0x7F, 0xFF, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut cursor = Cursor::new(bytes.as_slice());
        let result = read_be_f80(&mut cursor);
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }

    // --- Test 22: f80 negative rate ---
    #[test]
    fn test_f80_negative_rate() {
        // -44100: sign bit set
        let bytes: [u8; 10] = [0xC0, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut cursor = Cursor::new(bytes.as_slice());
        assert_eq!(read_be_f80(&mut cursor), Ok(-44100));
    }

    // --- Test 23: Chunk alignment padding ---
    #[test]
    fn test_chunk_alignment_padding() {
        // Add an extra chunk with odd size — parser should skip pad byte
        let data = AiffBuilder::new()
            .extra_chunk(0x4D41524B, vec![0u8; 5]) // "MARK" chunk, 5 bytes (odd)
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "padded.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    }

    // --- Test 24: Unknown chunk skipped ---
    #[test]
    fn test_unknown_chunk_skipped() {
        let data = AiffBuilder::new()
            .extra_chunk(0x58595A00, vec![0u8; 20]) // unknown "XYZ\0" chunk
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "unknown_chunk.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
    }

    // --- Test 25: Duplicate COMM chunk ---
    #[test]
    fn test_duplicate_comm_chunk() {
        // Build a file with two COMM chunks manually
        let mut chunks = Vec::new();

        // First COMM (channels=1, sample_frames=50, sample_size=8, rate=22050)
        chunks.extend_from_slice(&COMMON_ID.to_be_bytes());
        chunks.extend_from_slice(&18u32.to_be_bytes());
        chunks.extend_from_slice(&1u16.to_be_bytes());
        chunks.extend_from_slice(&50u32.to_be_bytes());
        chunks.extend_from_slice(&8u16.to_be_bytes());
        chunks.extend_from_slice(&[0x40, 0x0D, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // Second COMM (channels=2, sample_frames=100, sample_size=16, rate=44100)
        chunks.extend_from_slice(&COMMON_ID.to_be_bytes());
        chunks.extend_from_slice(&18u32.to_be_bytes());
        chunks.extend_from_slice(&2u16.to_be_bytes());
        chunks.extend_from_slice(&100u32.to_be_bytes());
        chunks.extend_from_slice(&16u16.to_be_bytes());
        chunks.extend_from_slice(&[0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // SSND chunk
        let audio = vec![0u8; 400]; // 100 * 2 * 2
        let ssnd_size = 8 + audio.len() as u32;
        chunks.extend_from_slice(&SOUND_DATA_ID.to_be_bytes());
        chunks.extend_from_slice(&ssnd_size.to_be_bytes());
        chunks.extend_from_slice(&0u32.to_be_bytes());
        chunks.extend_from_slice(&0u32.to_be_bytes());
        chunks.extend_from_slice(&audio);

        let form_size = 4 + chunks.len() as u32;
        let mut file = Vec::new();
        file.extend_from_slice(&FORM_ID.to_be_bytes());
        file.extend_from_slice(&form_size.to_be_bytes());
        file.extend_from_slice(&FORM_TYPE_AIFF.to_be_bytes());
        file.extend_from_slice(&chunks);

        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&file, "dup_comm.aiff");
        // Second COMM should take effect
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.frequency(), 44100);
        assert_eq!(dec.format(), AudioFormat::Stereo16);
    }

    // --- Test 26: Metadata set correctly after open ---
    #[test]
    fn test_open_sets_metadata() {
        // 22050 Hz, mono, 16-bit, 441 frames → length = 441 / 22050 ≈ 0.02 seconds
        let data = AiffBuilder::new()
            .channels(1)
            .sample_size(16)
            .sample_frames(441)
            .sample_rate_bytes([0x40, 0x0D, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
            .ssnd_data(vec![0u8; 882]) // 441 * 1 * 2
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "metadata.aiff");
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);
        assert_eq!(dec.frequency(), 22050);
        assert_eq!(dec.format(), AudioFormat::Mono16);
        assert_eq!(dec.max_pcm, 441);
        // length = sample_frames / frequency
        let expected_len = 441.0_f32 / 22050.0;
        assert!((dec.length() - expected_len).abs() < 0.001);
    }

    // --- Test 27: SDX2 requires 16-bit ---
    #[test]
    fn test_sdx2_requires_16bit() {
        let data = AiffBuilder::new()
            .form_type(FORM_TYPE_AIFC)
            .ext_type_id(SDX2_COMPRESSION)
            .sample_size(8) // invalid for SDX2
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "sdx2_8bit.aifc");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 28: SDX2 channel limit ---
    #[test]
    fn test_sdx2_channel_limit() {
        let data = AiffBuilder::new()
            .form_type(FORM_TYPE_AIFC)
            .ext_type_id(SDX2_COMPRESSION)
            .channels(5) // > MAX_CHANNELS
            .sample_size(16)
            .ssnd_data(vec![0u8; 500])
            .build();
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&data, "sdx2_5ch.aifc");
        assert!(matches!(result, Err(DecodeError::UnsupportedFormat(_))));
    }

    // --- Test 29: Chunk size exceeds remaining ---
    #[test]
    fn test_chunk_size_exceeds_remaining() {
        // Manually build a file where a chunk claims to be bigger than what remains
        let mut file = Vec::new();
        file.extend_from_slice(&FORM_ID.to_be_bytes());
        file.extend_from_slice(&30u32.to_be_bytes()); // FORM size = 30 (small)
        file.extend_from_slice(&FORM_TYPE_AIFF.to_be_bytes());
        // Chunk with size 9999
        file.extend_from_slice(&COMMON_ID.to_be_bytes());
        file.extend_from_slice(&9999u32.to_be_bytes());
        file.extend_from_slice(&[0u8; 18]); // only 18 bytes available
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&file, "oversized_chunk.aiff");
        assert!(result.is_err());
    }

    // --- Test 30: Truncated file mid-COMM ---
    #[test]
    fn test_truncated_file_mid_comm_chunk() {
        let mut file = Vec::new();
        file.extend_from_slice(&FORM_ID.to_be_bytes());
        file.extend_from_slice(&100u32.to_be_bytes());
        file.extend_from_slice(&FORM_TYPE_AIFF.to_be_bytes());
        file.extend_from_slice(&COMMON_ID.to_be_bytes());
        file.extend_from_slice(&18u32.to_be_bytes()); // claims 18 bytes
        file.extend_from_slice(&[0u8; 10]); // but only 10 bytes present
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&file, "truncated.aiff");
        assert!(result.is_err());
    }

    // --- Test 31: File exceeds 64MB size limit ---
    #[test]
    fn test_file_exceeds_size_limit() {
        let big = vec![0u8; MAX_FILE_SIZE + 1];
        let mut dec = make_decoder();
        let result = dec.open_from_bytes(&big, "huge.aiff");
        assert!(matches!(result, Err(DecodeError::InvalidData(_))));
    }
}
