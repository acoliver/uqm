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
    prev_val: [i32; MAX_CHANNELS],
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

// --- Private helper functions ---
// @plan PLAN-20260225-AIFF-DECODER.P05
// @requirement REQ-FP-1..15, REQ-SV-1..13, REQ-CH-1..7

fn read_be_u16(cursor: &mut Cursor<&[u8]>) -> DecodeResult<u16> {
    let mut buf = [0u8; 2];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| DecodeError::InvalidData("read u16".into()))?;
    Ok(u16::from_be_bytes(buf))
}

fn read_be_u32(cursor: &mut Cursor<&[u8]>) -> DecodeResult<u32> {
    let mut buf = [0u8; 4];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| DecodeError::InvalidData("read u32".into()))?;
    Ok(u32::from_be_bytes(buf))
}

fn read_be_i16(cursor: &mut Cursor<&[u8]>) -> DecodeResult<i16> {
    let mut buf = [0u8; 2];
    cursor
        .read_exact(&mut buf)
        .map_err(|_| DecodeError::InvalidData("read i16".into()))?;
    Ok(i16::from_be_bytes(buf))
}

/// Parse IEEE 754 80-bit extended precision float to i32.
///
/// Layout: sign (1 bit) | biased exponent (15 bits) | significand (64 bits, explicit integer bit)
/// For normalized numbers: value = (-1)^sign × significand × 2^(biased_exp − 16383 − 63)
fn read_be_f80(cursor: &mut Cursor<&[u8]>) -> DecodeResult<i32> {
    let se = read_be_u16(cursor)?;
    let sig_hi = read_be_u32(cursor)?;
    let sig_lo = read_be_u32(cursor)?;

    let sign = (se >> 15) & 1;
    let biased_exp = se & 0x7FFF;
    let significand: u64 = ((sig_hi as u64) << 32) | (sig_lo as u64);

    // Class 1: Zero
    if biased_exp == 0 && significand == 0 {
        return Ok(0);
    }

    // Class 2: Denormalized — near-zero, will be caught by rate validation
    if biased_exp == 0 {
        return Ok(0);
    }

    // Class 3: Infinity / NaN
    if biased_exp == 0x7FFF {
        return Err(DecodeError::InvalidData(
            "invalid sample rate: infinity or NaN in f80".into(),
        ));
    }

    // Class 4: Normal
    let shift: i32 = (biased_exp as i32) - 16383 - 63;

    let abs_val: u64 = if shift >= 0 {
        let shifted = significand.checked_shl(shift as u32).unwrap_or(u64::MAX);
        if shifted > 0x7FFF_FFFF {
            0x7FFF_FFFF // clamp to i32::MAX
        } else {
            shifted
        }
    } else {
        let right_shift = (-shift) as u32;
        if right_shift >= 64 {
            0
        } else {
            significand >> right_shift
        }
    };

    let mut result = abs_val as i32;
    if sign == 1 {
        result = -result;
    }
    Ok(result)
}

fn read_chunk_header(cursor: &mut Cursor<&[u8]>) -> DecodeResult<ChunkHeader> {
    let id = read_be_u32(cursor)?;
    let size = read_be_u32(cursor)?;
    Ok(ChunkHeader { id, size })
}

impl AiffDecoder {
    fn read_common_chunk(
        &mut self,
        cursor: &mut Cursor<&[u8]>,
        chunk_size: u32,
    ) -> DecodeResult<CommonChunk> {
        if chunk_size < AIFF_COMM_SIZE as u32 {
            self.last_error = -2;
            return Err(DecodeError::InvalidData("COMM chunk too small".into()));
        }
        let start_pos = cursor.position();
        let mut common = CommonChunk::default();
        common.channels = read_be_u16(cursor)?;
        common.sample_frames = read_be_u32(cursor)?;
        common.sample_size = read_be_u16(cursor)?;
        common.sample_rate = read_be_f80(cursor)?;
        if chunk_size >= AIFF_EXT_COMM_SIZE as u32 {
            common.ext_type_id = read_be_u32(cursor)?;
        }
        let consumed = cursor.position() - start_pos;
        let remaining = chunk_size as u64 - consumed;
        if remaining > 0 {
            cursor
                .seek(SeekFrom::Current(remaining as i64))
                .map_err(|_| DecodeError::InvalidData("seek past COMM remainder".into()))?;
        }
        Ok(common)
    }

    fn read_sound_data_header(
        &mut self,
        cursor: &mut Cursor<&[u8]>,
    ) -> DecodeResult<SoundDataHeader> {
        let offset = read_be_u32(cursor)?;
        let block_size = read_be_u32(cursor)?;
        Ok(SoundDataHeader { offset, block_size })
    }

    // @plan PLAN-20260225-AIFF-DECODER.P08
    // @requirement REQ-DP-1..6
    fn decode_pcm(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        // REQ-DP-6: EOF check
        if self.cur_pcm >= self.max_pcm {
            return Err(DecodeError::EndOfFile);
        }

        // REQ-DP-1: frame count
        let block_align = self.block_align as usize;
        if block_align == 0 {
            return Ok(0);
        }
        let dec_pcm = std::cmp::min(
            buf.len() / block_align,
            (self.max_pcm - self.cur_pcm) as usize,
        );
        if dec_pcm == 0 {
            return Ok(0);
        }

        let file_block = self.file_block as usize;
        let read_bytes = dec_pcm * file_block;
        let write_bytes = dec_pcm * block_align;

        // REQ-DP-2: copy raw big-endian PCM data (no inline byte swap)
        buf[..write_bytes].copy_from_slice(&self.data[self.data_pos..self.data_pos + read_bytes]);

        // REQ-DP-5: 8-bit signed→unsigned conversion
        if self.bits_per_sample == 8 {
            for byte in buf[..write_bytes].iter_mut() {
                *byte = byte.wrapping_add(128);
            }
        }

        // REQ-DP-3: position update
        self.cur_pcm += dec_pcm as u32;
        self.data_pos += read_bytes;

        // REQ-DP-4: return bytes written
        Ok(write_bytes)
    }

    // @plan PLAN-20260225-AIFF-DECODER.P11
    // @requirement REQ-DS-1..8
    fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        // REQ-DS-8: EOF check
        if self.cur_pcm >= self.max_pcm {
            return Err(DecodeError::EndOfFile);
        }

        // REQ-DS-1: frame count
        let block_align = self.block_align as usize;
        if block_align == 0 {
            return Ok(0);
        }
        let dec_pcm = std::cmp::min(
            buf.len() / block_align,
            (self.max_pcm - self.cur_pcm) as usize,
        );
        if dec_pcm == 0 {
            return Ok(0);
        }

        let channels = self.common.channels as usize;
        let file_block = self.file_block as usize;
        let compressed_bytes = dec_pcm * file_block;

        // REQ-DS-2: read compressed data
        let compressed = &self.data[self.data_pos..self.data_pos + compressed_bytes];

        let mut out_pos = 0;

        // REQ-DS-4, REQ-DS-5: SDX2 decode loop
        for frame_idx in 0..dec_pcm {
            for ch in 0..channels {
                let byte_idx = frame_idx * channels + ch;
                let sample_byte = compressed[byte_idx] as i8;
                let sample = sample_byte as i32;
                let abs_val = sample.abs();
                let mut v = (sample * abs_val) << 1;

                // Delta mode: odd sample bytes add to predictor
                if (sample_byte as u8) & 1 != 0 {
                    v += self.prev_val[ch];
                }

                // Saturate to i16 range
                v = v.clamp(-32768, 32767);
                self.prev_val[ch] = v;

                // Write i16 to output
                let sample_i16 = v as i16;
                let bytes = if self.need_swap {
                    sample_i16.swap_bytes().to_ne_bytes()
                } else {
                    sample_i16.to_ne_bytes()
                };
                buf[out_pos] = bytes[0];
                buf[out_pos + 1] = bytes[1];
                out_pos += 2;
            }
        }

        // REQ-DS-3: position update
        self.cur_pcm += dec_pcm as u32;
        self.data_pos += compressed_bytes;

        // REQ-DS-6: return bytes written
        let write_bytes = dec_pcm * block_align;
        Ok(write_bytes)
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

    fn open_from_bytes(&mut self, data: &[u8], _name: &str) -> DecodeResult<()> {
        // Reset state (REQ-LF-7)
        self.close();
        self.common = CommonChunk::default();

        // Minimum header size
        if data.len() < 12 {
            self.last_error = -2;
            return Err(DecodeError::InvalidData(
                "file too small for AIFF header".into(),
            ));
        }

        // Memory guard (64MB)
        if data.len() > MAX_FILE_SIZE {
            self.last_error = -2;
            return Err(DecodeError::InvalidData(
                "AIFF file exceeds 64MB safety limit".into(),
            ));
        }

        let mut cursor = Cursor::new(data);

        // Parse FORM header (REQ-FP-1)
        let chunk_id = read_be_u32(&mut cursor)?;
        let chunk_size = read_be_u32(&mut cursor)?;
        let form_type = read_be_u32(&mut cursor)?;

        // Validate FORM (REQ-FP-2)
        if chunk_id != FORM_ID {
            self.last_error = -2;
            self.close();
            return Err(DecodeError::InvalidData("not a FORM file".into()));
        }

        // Validate form type (REQ-FP-3)
        let is_aifc = match form_type {
            FORM_TYPE_AIFF => false,
            FORM_TYPE_AIFC => true,
            _ => {
                self.last_error = -2;
                self.close();
                return Err(DecodeError::InvalidData("unsupported form type".into()));
            }
        };

        // Chunk iteration (REQ-FP-4, REQ-FP-6)
        let mut remaining = chunk_size as i64 - 4;
        let mut data_ofs: u64 = 0;
        let mut ssnd_found = false;

        while remaining > 0 {
            let chunk_hdr = match read_chunk_header(&mut cursor) {
                Ok(h) => h,
                Err(_) => break, // ran out of data in chunk headers
            };
            let mut consume = 8 + chunk_hdr.size as i64;
            if chunk_hdr.size & 1 != 0 {
                consume += 1; // alignment padding (REQ-FP-5)
            }

            // Overflow guard
            if consume > remaining + 8 {
                self.last_error = -2;
                self.close();
                return Err(DecodeError::InvalidData(
                    "chunk size exceeds remaining file data".into(),
                ));
            }

            match chunk_hdr.id {
                COMMON_ID => {
                    self.common = self.read_common_chunk(&mut cursor, chunk_hdr.size)?;
                }
                SOUND_DATA_ID => {
                    let ssnd_hdr = self.read_sound_data_header(&mut cursor)?;
                    data_ofs = cursor.position() + ssnd_hdr.offset as u64;
                    ssnd_found = true;
                    let skip = chunk_hdr.size.saturating_sub(AIFF_SSND_SIZE as u32);
                    if skip > 0 {
                        cursor
                            .seek(SeekFrom::Current(skip as i64))
                            .map_err(|_| DecodeError::InvalidData("seek past SSND data".into()))?;
                    }
                }
                _ => {
                    // Skip unknown chunks (REQ-FP-7)
                    if chunk_hdr.size > 0 {
                        cursor
                            .seek(SeekFrom::Current(chunk_hdr.size as i64))
                            .map_err(|_| {
                                DecodeError::InvalidData("seek past unknown chunk".into())
                            })?;
                    }
                }
            }

            // Alignment padding (REQ-FP-5)
            if chunk_hdr.size & 1 != 0 {
                let _ = cursor.seek(SeekFrom::Current(1));
            }

            remaining -= consume;
        }

        // Validation phase

        // REQ-SV-5: sample frames > 0
        if self.common.sample_frames == 0 {
            self.last_error = -2;
            self.close();
            return Err(DecodeError::InvalidData("no sound data".into()));
        }

        // REQ-SV-1: round bits to byte boundary
        self.bits_per_sample = (self.common.sample_size + 7) & !7;

        // REQ-SV-2: bits per sample range
        if self.bits_per_sample == 0 || self.bits_per_sample > 16 {
            self.last_error = -2;
            self.close();
            return Err(DecodeError::UnsupportedFormat(
                "bits_per_sample must be 1-16".into(),
            ));
        }

        // REQ-SV-3: channel count
        if self.common.channels != 1 && self.common.channels != 2 {
            self.last_error = -2;
            self.close();
            return Err(DecodeError::UnsupportedFormat(
                "only mono and stereo supported".into(),
            ));
        }

        // REQ-SV-4: sample rate range
        if self.common.sample_rate < MIN_SAMPLE_RATE as i32
            || self.common.sample_rate > MAX_SAMPLE_RATE as i32
        {
            self.last_error = -2;
            self.close();
            return Err(DecodeError::UnsupportedFormat("sample_rate".into()));
        }

        // REQ-SV-6: SSND required
        if !ssnd_found {
            self.last_error = -2;
            self.close();
            return Err(DecodeError::InvalidData("no SSND chunk".into()));
        }

        // Compression handling (REQ-CH-1 through REQ-CH-4)
        if !is_aifc {
            if self.common.ext_type_id != 0 {
                self.close();
                return Err(DecodeError::UnsupportedFormat("AIFF with extension".into()));
            }
            self.comp_type = CompressionType::None;
        } else {
            if self.common.ext_type_id == SDX2_COMPRESSION {
                self.comp_type = CompressionType::Sdx2;
            } else {
                self.close();
                return Err(DecodeError::UnsupportedFormat(
                    "unknown AIFC compression".into(),
                ));
            }
        }

        // SDX2-specific validation (REQ-CH-5, REQ-CH-6)
        if self.comp_type == CompressionType::Sdx2 {
            if self.bits_per_sample != 16 {
                self.close();
                return Err(DecodeError::UnsupportedFormat(
                    "SDX2 requires 16-bit".into(),
                ));
            }
            if self.common.channels as usize > MAX_CHANNELS {
                self.close();
                return Err(DecodeError::UnsupportedFormat(
                    "SDX2 too many channels".into(),
                ));
            }
        }

        // Block sizes (REQ-SV-7..9)
        self.block_align = (self.bits_per_sample / 8) * self.common.channels;
        if self.comp_type == CompressionType::None {
            self.file_block = self.block_align;
        } else {
            self.file_block = self.block_align / 2; // 2:1 SDX2 compression
        }

        // Extract audio data (REQ-SV-10)
        let data_start = data_ofs as usize;
        let data_size = self.common.sample_frames as usize * self.file_block as usize;
        if data_start + data_size > data.len() {
            self.close();
            return Err(DecodeError::InvalidData(
                "audio data extends past file".into(),
            ));
        }
        self.data = data[data_start..data_start + data_size].to_vec();

        // Set metadata (REQ-SV-11..13)
        self.format = match (self.common.channels, self.bits_per_sample) {
            (1, 8) => AudioFormat::Mono8,
            (2, 8) => AudioFormat::Stereo8,
            (1, 16) => AudioFormat::Mono16,
            _ => AudioFormat::Stereo16,
        };
        self.frequency = self.common.sample_rate as u32;
        self.max_pcm = self.common.sample_frames;
        self.cur_pcm = 0;
        self.data_pos = 0;
        self.length = self.max_pcm as f32 / self.frequency as f32;
        self.last_error = 0;

        // Set need_swap (REQ-LF-5, REQ-CH-7)
        if let Some(ref fmts) = self.formats {
            if self.comp_type == CompressionType::Sdx2 {
                self.need_swap = fmts.big_endian != fmts.want_big_endian;
            } else {
                self.need_swap = !fmts.want_big_endian;
            }
        }

        // Predictor initialization (REQ-DS-7)
        self.prev_val = [0; MAX_CHANNELS];

        Ok(())
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

    fn decode(&mut self, buf: &mut [u8]) -> DecodeResult<usize> {
        match self.comp_type {
            CompressionType::None => self.decode_pcm(buf),
            CompressionType::Sdx2 => self.decode_sdx2(buf),
        }
    }

    // @plan PLAN-20260225-AIFF-DECODER.P14
    // @requirement REQ-SK-1..4
    fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        // REQ-SK-1: clamp to max
        let pcm_pos = pcm_pos.min(self.max_pcm);

        // REQ-SK-2: update position
        self.cur_pcm = pcm_pos;
        self.data_pos = pcm_pos as usize * self.file_block as usize;

        // REQ-SK-3: reset predictor
        self.prev_val = [0i32; MAX_CHANNELS];

        // REQ-SK-4
        Ok(pcm_pos)
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

    // --- P07/P08 PCM decode tests ---

    /// Helper: create a decoder opened with known PCM data
    fn make_pcm_decoder(channels: u16, sample_size: u16, audio_data: Vec<u8>) -> AiffDecoder {
        let sample_frames =
            audio_data.len() as u32 / (((sample_size + 7) & !7) as u32 / 8 * channels as u32);
        let data = AiffBuilder::new()
            .channels(channels)
            .sample_size(sample_size)
            .sample_frames(sample_frames)
            .ssnd_data(audio_data)
            .build();
        let mut dec = make_decoder();
        dec.open_from_bytes(&data, "test.aiff").unwrap();
        dec
    }

    #[test]
    fn test_decode_pcm_mono16() {
        let audio = vec![0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04]; // 4 frames
        let mut dec = make_pcm_decoder(1, 16, audio.clone());
        let mut buf = vec![0u8; 8];
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(&buf[..8], &audio[..]);
    }

    #[test]
    fn test_decode_pcm_stereo16() {
        // L=0x0001 R=0x0002, L=0x0003 R=0x0004
        let audio = vec![0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04];
        let mut dec = make_pcm_decoder(2, 16, audio.clone());
        let mut buf = vec![0u8; 8];
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(&buf[..8], &audio[..]);
    }

    #[test]
    fn test_decode_pcm_mono8_signed_to_unsigned() {
        // Signed: -128(0x80), -1(0xFF), 0(0x00), 127(0x7F)
        let audio = vec![0x80, 0xFF, 0x00, 0x7F];
        let mut dec = make_pcm_decoder(1, 8, audio);
        let mut buf = vec![0u8; 4];
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 4);
        // After wrapping_add(128): 0, 127, 128, 255
        assert_eq!(buf, vec![0x00, 0x7F, 0x80, 0xFF]);
    }

    #[test]
    fn test_decode_pcm_stereo8_signed_to_unsigned() {
        let audio = vec![0x80, 0x7F, 0x00, 0xFF]; // 2 frames, 2 channels
        let mut dec = make_pcm_decoder(2, 8, audio);
        let mut buf = vec![0u8; 4];
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf, vec![0x00, 0xFF, 0x80, 0x7F]);
    }

    #[test]
    fn test_decode_pcm_partial_buffer() {
        let audio = vec![0u8; 200]; // 100 mono16 frames
        let mut dec = make_pcm_decoder(1, 16, audio);
        let mut buf = vec![0u8; 50]; // only room for 25 frames
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 50);
        assert_eq!(dec.cur_pcm, 25);
    }

    #[test]
    fn test_decode_pcm_multiple_calls() {
        let audio: Vec<u8> = (0..20).collect(); // 10 mono16 frames
        let mut dec = make_pcm_decoder(1, 16, audio.clone());
        let mut buf = vec![0u8; 10]; // 5 frames
        let n1 = dec.decode(&mut buf).unwrap();
        assert_eq!(n1, 10);
        assert_eq!(&buf[..10], &audio[..10]);
        assert_eq!(dec.cur_pcm, 5);
        let n2 = dec.decode(&mut buf).unwrap();
        assert_eq!(n2, 10);
        assert_eq!(&buf[..10], &audio[10..20]);
        assert_eq!(dec.cur_pcm, 10);
    }

    #[test]
    fn test_decode_pcm_eof() {
        let audio = vec![0u8; 4]; // 2 mono16 frames
        let mut dec = make_pcm_decoder(1, 16, audio);
        let mut buf = vec![0u8; 4];
        dec.decode(&mut buf).unwrap(); // consume all
        let result = dec.decode(&mut buf);
        assert!(matches!(result, Err(DecodeError::EndOfFile)));
    }

    #[test]
    fn test_decode_pcm_exact_fit() {
        let audio = vec![0u8; 6]; // 3 mono16 frames
        let mut dec = make_pcm_decoder(1, 16, audio);
        let mut buf = vec![0u8; 6]; // exact fit
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 6);
        assert_eq!(dec.cur_pcm, 3);
    }

    #[test]
    fn test_decode_pcm_returns_byte_count() {
        let audio = vec![0u8; 40]; // 10 stereo16 frames
        let mut dec = make_pcm_decoder(2, 16, audio);
        let mut buf = vec![0u8; 40];
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 40); // 10 frames * 4 bytes
    }

    #[test]
    fn test_decode_pcm_position_update() {
        let audio = vec![0u8; 20]; // 10 mono16 frames
        let mut dec = make_pcm_decoder(1, 16, audio);
        let mut buf = vec![0u8; 6]; // 3 frames
        dec.decode(&mut buf).unwrap();
        assert_eq!(dec.cur_pcm, 3);
        assert_eq!(dec.data_pos, 6);
    }

    #[test]
    fn test_decode_pcm_16bit_no_inline_swap() {
        let audio = vec![0x03, 0xE8]; // 1000 as big-endian i16
        let mut dec = make_pcm_decoder(1, 16, audio);
        dec.need_swap = true; // pretend mixer wants little-endian
        let mut buf = vec![0u8; 2];
        dec.decode(&mut buf).unwrap();
        // Raw bytes must be preserved — no inline swap
        assert_eq!(buf, vec![0x03, 0xE8]);
    }

    #[test]
    fn test_decode_pcm_16bit_raw_bytes_preserved() {
        let audio = vec![0x03, 0xE8];
        let mut dec = make_pcm_decoder(1, 16, audio);
        dec.need_swap = false;
        let mut buf = vec![0u8; 2];
        dec.decode(&mut buf).unwrap();
        assert_eq!(buf, vec![0x03, 0xE8]);
    }

    #[test]
    fn test_decode_pcm_16bit_stereo_raw_bytes() {
        let audio = vec![0x00, 0x01, 0x00, 0x02]; // L=1 R=2
        let mut dec = make_pcm_decoder(2, 16, audio);
        dec.need_swap = true;
        let mut buf = vec![0u8; 4];
        dec.decode(&mut buf).unwrap();
        assert_eq!(buf, vec![0x00, 0x01, 0x00, 0x02]);
    }

    #[test]
    fn test_decode_pcm_8bit_no_endian_swap() {
        let audio = vec![0x80, 0x00]; // 2 mono8 frames
        let mut dec = make_pcm_decoder(1, 8, audio);
        dec.need_swap = true; // should have no effect on 8-bit
        let mut buf = vec![0u8; 2];
        dec.decode(&mut buf).unwrap();
        // Only signed→unsigned: 0x80+128=0, 0x00+128=128
        assert_eq!(buf, vec![0x00, 0x80]);
    }

    #[test]
    fn test_decode_pcm_zero_length_buffer() {
        let audio = vec![0u8; 4];
        let mut dec = make_pcm_decoder(1, 16, audio);
        let mut buf = vec![];
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 0);
        assert_eq!(dec.cur_pcm, 0); // position unchanged
    }

    #[test]
    fn test_need_swap_set_correctly_for_16bit() {
        let data = AiffBuilder::new()
            .channels(1)
            .sample_size(16)
            .sample_frames(10)
            .ssnd_data(vec![0u8; 20])
            .build();
        let mut dec = make_decoder();
        dec.open_from_bytes(&data, "swap_test.aiff").unwrap();
        // Default DecoderFormats has want_big_endian = false
        // AIFF is big-endian → need_swap should be true
        assert!(dec.need_swap);
    }

    // --- P10/P11 SDX2 decode tests ---

    /// Helper: create a decoder opened with known SDX2 data
    fn make_sdx2_decoder(channels: u16, audio_data: Vec<u8>) -> AiffDecoder {
        // SDX2: file_block = block_align / 2 = (2 * channels) / 2 = channels
        let sample_frames = audio_data.len() as u32 / channels as u32;
        let data = AiffBuilder::new()
            .form_type(FORM_TYPE_AIFC)
            .channels(channels)
            .sample_size(16)
            .sample_frames(sample_frames)
            .ext_type_id(SDX2_COMPRESSION)
            .ssnd_data(audio_data)
            .build();
        let mut dec = make_decoder();
        dec.open_from_bytes(&data, "test.aifc").unwrap();
        dec
    }

    #[test]
    fn test_decode_sdx2_zero_input() {
        // All zero bytes → all zero samples
        let mut dec = make_sdx2_decoder(1, vec![0u8; 10]);
        let mut buf = vec![0u8; 20]; // 10 frames * 2 bytes
        let n = dec.decode(&mut buf).unwrap();
        assert_eq!(n, 20);
        // v = (0 * 0) << 1 = 0, even byte so no delta → all zeros
        for chunk in buf[..20].chunks_exact(2) {
            let sample = i16::from_ne_bytes([chunk[0], chunk[1]]);
            assert_eq!(sample, 0);
        }
    }

    #[test]
    fn test_decode_sdx2_positive_sample() {
        // Sample byte = 10 → v = (10 * 10) << 1 = 200
        // Even byte (10 & 1 == 0) → no delta
        let mut dec = make_sdx2_decoder(1, vec![10]);
        let mut buf = vec![0u8; 2];
        dec.decode(&mut buf).unwrap();
        let sample = i16::from_ne_bytes([buf[0], buf[1]]);
        assert_eq!(sample, 200);
    }

    #[test]
    fn test_decode_sdx2_negative_sample() {
        // Sample byte = -10 (0xF6) → v = (-10 * 10) << 1 = -200
        // Even byte (0xF6 & 1 == 0) → no delta
        let mut dec = make_sdx2_decoder(1, vec![0xF6]);
        let mut buf = vec![0u8; 2];
        dec.decode(&mut buf).unwrap();
        let sample = i16::from_ne_bytes([buf[0], buf[1]]);
        assert_eq!(sample, -200);
    }

    #[test]
    fn test_decode_sdx2_delta_mode() {
        // Frame 0: byte=10 (even) → v = 200, prev=200
        // Frame 1: byte=11 (odd)  → v = (11 * 11) << 1 = 242, + prev(200) = 442
        let mut dec = make_sdx2_decoder(1, vec![10, 11]);
        let mut buf = vec![0u8; 4];
        dec.decode(&mut buf).unwrap();
        let s0 = i16::from_ne_bytes([buf[0], buf[1]]);
        let s1 = i16::from_ne_bytes([buf[2], buf[3]]);
        assert_eq!(s0, 200);
        assert_eq!(s1, 442);
    }

    #[test]
    fn test_decode_sdx2_saturation() {
        // byte=127 → v = (127 * 127) << 1 = 32258
        // Then byte=127 (odd) → v = 32258 + 32258 = 64516 → clamped to 32767
        let mut dec = make_sdx2_decoder(1, vec![127, 127]);
        let mut buf = vec![0u8; 4];
        dec.decode(&mut buf).unwrap();
        let s0 = i16::from_ne_bytes([buf[0], buf[1]]);
        let s1 = i16::from_ne_bytes([buf[2], buf[3]]);
        assert_eq!(s0, 32258);
        assert_eq!(s1, 32767); // saturated
    }

    #[test]
    fn test_decode_sdx2_stereo() {
        // 2 channels: [L0=10, R0=20, L1=11, R1=21]
        let mut dec = make_sdx2_decoder(2, vec![10, 20, 11, 21]);
        let mut buf = vec![0u8; 8]; // 2 frames * 2 ch * 2 bytes
        dec.decode(&mut buf).unwrap();
        let l0 = i16::from_ne_bytes([buf[0], buf[1]]);
        let r0 = i16::from_ne_bytes([buf[2], buf[3]]);
        let l1 = i16::from_ne_bytes([buf[4], buf[5]]);
        let r1 = i16::from_ne_bytes([buf[6], buf[7]]);
        // L0: (10*10)<<1 = 200 (even, no delta)
        assert_eq!(l0, 200);
        // R0: (20*20)<<1 = 800 (even, no delta)
        assert_eq!(r0, 800);
        // L1: (11*11)<<1 = 242, odd → +200 = 442
        assert_eq!(l1, 442);
        // R1: (21*21)<<1 = 882, odd → +800 = 1682
        assert_eq!(r1, 1682);
    }

    #[test]
    fn test_decode_sdx2_eof() {
        let mut dec = make_sdx2_decoder(1, vec![10]);
        let mut buf = vec![0u8; 2];
        dec.decode(&mut buf).unwrap(); // consume the one frame
        let result = dec.decode(&mut buf);
        assert!(matches!(result, Err(DecodeError::EndOfFile)));
    }

    #[test]
    fn test_decode_sdx2_position_update() {
        let mut dec = make_sdx2_decoder(1, vec![0u8; 10]);
        let mut buf = vec![0u8; 6]; // 3 frames
        dec.decode(&mut buf).unwrap();
        assert_eq!(dec.cur_pcm, 3);
        assert_eq!(dec.data_pos, 3); // SDX2 file_block = 1 byte per frame (mono)
    }

    // --- P13/P14 Seek tests ---

    #[test]
    fn test_seek_to_beginning() {
        let mut dec = make_pcm_decoder(1, 16, vec![0u8; 20]);
        let mut buf = vec![0u8; 10];
        dec.decode(&mut buf).unwrap(); // advance to frame 5
        let pos = dec.seek(0).unwrap();
        assert_eq!(pos, 0);
        assert_eq!(dec.cur_pcm, 0);
        assert_eq!(dec.data_pos, 0);
    }

    #[test]
    fn test_seek_to_middle() {
        let mut dec = make_pcm_decoder(1, 16, vec![0u8; 20]);
        let pos = dec.seek(5).unwrap();
        assert_eq!(pos, 5);
        assert_eq!(dec.cur_pcm, 5);
        assert_eq!(dec.data_pos, 10); // 5 frames * 2 bytes
    }

    #[test]
    fn test_seek_past_end_clamps() {
        let mut dec = make_pcm_decoder(1, 16, vec![0u8; 20]); // 10 frames
        let pos = dec.seek(999).unwrap();
        assert_eq!(pos, 10); // clamped to max_pcm
        assert_eq!(dec.cur_pcm, 10);
    }

    #[test]
    fn test_seek_resets_predictor() {
        let mut dec = make_sdx2_decoder(1, vec![10, 11]); // predictor gets set
        let mut buf = vec![0u8; 4];
        dec.decode(&mut buf).unwrap();
        assert_ne!(dec.prev_val[0], 0); // predictor should be non-zero
        dec.seek(0).unwrap();
        assert_eq!(dec.prev_val, [0; MAX_CHANNELS]); // reset
    }

    #[test]
    fn test_seek_then_decode() {
        let audio: Vec<u8> = (0..20).collect(); // 10 mono16 frames
        let mut dec = make_pcm_decoder(1, 16, audio.clone());
        dec.seek(5).unwrap();
        let mut buf = vec![0u8; 4]; // 2 frames
        dec.decode(&mut buf).unwrap();
        // Should read frames 5-6 → bytes 10..14
        assert_eq!(&buf[..4], &audio[10..14]);
    }
}
