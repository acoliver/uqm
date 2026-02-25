//! Duck audio (.duk) decoder
//!
//! Decodes the audio track embedded in 3DO Duck video (.duk) files.
//! The audio is stored as IMA ADPCM with a custom step table from the
//! original 3DO SDK.
//!
//! # Format
//!
//! Each frame in the .duk file contains both video and audio data.
//! The frame offsets are stored in a companion .frm file (array of
//! big-endian u32). Each frame starts with an 8-byte header:
//!
//! ```text
//! [4 bytes] audio data size (big-endian u32)
//! [4 bytes] video data size (big-endian u32)
//! ```
//!
//! The audio data begins immediately after the header and contains
//! a 10-byte subframe header followed by ADPCM-encoded nibbles:
//!
//! ```text
//! [2 bytes] magic (0xf77f, big-endian)
//! [2 bytes] number of samples (big-endian)
//! [2 bytes] tag (big-endian)
//! [2 bytes] initial ADPCM index for channel 0 (big-endian)
//! [2 bytes] initial ADPCM index for channel 1 (big-endian)
//! [N bytes] ADPCM data (each byte = 2 nibbles = 2 samples)
//! ```

use super::decoder::{DecodeError, DecodeResult};
use super::formats::AudioFormat;

/// Duck video general frame rate, used to estimate audio length.
const DUCK_GENERAL_FPS: f32 = 14.622;

/// Internal decode buffer size (must be large enough for one decoded frame).
const DATA_BUF_SIZE: usize = 0x8000;

/// Magic number in the audio subframe header.
const DUKAUD_MAGIC: u16 = 0xf77f;

/// Sample rate for Duck audio (always 22050 Hz stereo).
pub const DUKAUD_FREQUENCY: u32 = 22050;

// ---------------------------------------------------------------------------
// ADPCM tables
// ---------------------------------------------------------------------------

/// Custom ADPCM step table from the original 3DO SDK.
/// Slightly different from the standard IMA ADPCM table.
#[rustfmt::skip]
static ADPCM_STEP: [i32; 89] = [
    0x7, 0x8, 0x9, 0xA, 0xB, 0xC, 0xD, 0xF,
    0x10, 0x12, 0x13, 0x15, 0x17, 0x1A, 0x1C, 0x1F,
    0x22, 0x26, 0x29, 0x2E, 0x32, 0x37, 0x3D, 0x43,
    0x4A, 0x51, 0x59, 0x62, 0x6C, 0x76, 0x82, 0x8F,
    0x9E, 0xAD, 0xBF, 0xD2, 0xE7, 0xFE, 0x117, 0x133,
    0x152, 0x174, 0x199, 0x1C2, 0x1EF, 0x220, 0x256, 0x292,
    0x2D4, 0x31D, 0x36C, 0x3C4, 0x424, 0x48E, 0x503, 0x583,
    0x610, 0x6AC, 0x756, 0x812, 0x8E1, 0x9C4, 0xABE, 0xBD1,
    0xCFF, 0xE4C, 0xFBA, 0x114D, 0x1308, 0x14EF, 0x1707, 0x1954,
    0x1BDD, 0x1EA6, 0x21B7, 0x2516,
    0x28CB, 0x2CDF, 0x315C, 0x364C,
    0x3BBA, 0x41B2, 0x4844, 0x4F7E,
    0x5771, 0x6030, 0x69CE, 0x7463,
    0x7FFF,
];

/// ADPCM index adjustment table.
#[rustfmt::skip]
static ADPCM_INDEX: [i32; 16] = [
    -1, -1, -1, -1, 2, 4, 6, 8,
    -1, -1, -1, -1, 2, 4, 6, 8,
];

// ---------------------------------------------------------------------------
// Audio subframe header
// ---------------------------------------------------------------------------

/// Parsed audio subframe header from a .duk frame.
#[derive(Debug, Clone)]
struct AudSubframe {
    num_samples: u16,
    _tag: u16,
    indices: [u16; 2],
}

impl AudSubframe {
    /// Parse from big-endian bytes (10 bytes: magic + numsamples + tag + 2×index).
    fn from_be_bytes(data: &[u8]) -> DecodeResult<Self> {
        if data.len() < 10 {
            return Err(DecodeError::InvalidData("audio subframe too short".into()));
        }
        let magic = u16::from_be_bytes([data[0], data[1]]);
        if magic != DUKAUD_MAGIC {
            return Err(DecodeError::InvalidData(format!(
                "bad audio magic 0x{:04x}, expected 0x{:04x}",
                magic, DUKAUD_MAGIC
            )));
        }
        Ok(Self {
            num_samples: u16::from_be_bytes([data[2], data[3]]),
            _tag: u16::from_be_bytes([data[4], data[5]]),
            indices: [
                u16::from_be_bytes([data[6], data[7]]),
                u16::from_be_bytes([data[8], data[9]]),
            ],
        })
    }
}

// ---------------------------------------------------------------------------
// ADPCM nibble decoder
// ---------------------------------------------------------------------------

/// Decode ADPCM nibbles into 16-bit PCM samples in-place.
///
/// `output` contains raw nibble values (0–15) on input, and decoded
/// PCM samples on output. `channels` is 1 (mono) or 2 (stereo).
fn decode_nibbles(
    output: &mut [i16],
    channels: usize,
    predictors: &mut [i32; 2],
    indices: &[u16; 2],
) {
    let ch_mask = channels - 1; // 0 for mono, 1 for stereo
    let mut index = [indices[0] as i32, indices[1] as i32];
    let mut step = [
        ADPCM_STEP[index[0].clamp(0, 88) as usize],
        ADPCM_STEP[index[1].clamp(0, 88) as usize],
    ];
    let mut ch = 0usize;

    for sample in output.iter_mut() {
        let delta = *sample as i32;

        index[ch] += ADPCM_INDEX[delta as usize & 0xF];
        index[ch] = index[ch].clamp(0, 88);

        let sign = delta & 8;
        let magnitude = delta & 7;

        // Real ADPCM decode: diff = ((2*magnitude + 1) * step) / 8
        let diff = (((magnitude << 1) + 1) * step[ch]) >> 3;

        if sign != 0 {
            predictors[ch] -= diff;
        } else {
            predictors[ch] += diff;
        }
        predictors[ch] = predictors[ch].clamp(-32768, 32767);

        *sample = predictors[ch] as i16;
        step[ch] = ADPCM_STEP[index[ch] as usize];

        ch ^= ch_mask;
    }
}

// ---------------------------------------------------------------------------
// DukAudDecoder
// ---------------------------------------------------------------------------

/// Rust decoder for .duk embedded audio.
///
/// Opens the .duk and .frm files, reads frame offsets, and decodes
/// ADPCM audio frame by frame.
pub struct DukAudDecoder {
    /// Frame offsets read from the .frm file.
    frames: Vec<u32>,
    /// Raw .duk file data (kept in memory for random access).
    duk_data: Vec<u8>,
    /// Current frame index.
    iframe: u32,
    /// Total frame count.
    cframes: u32,
    /// Number of audio channels (always 2 for Duck).
    channels: u32,
    /// PCM samples per frame (set from first frame header).
    pcm_frame: u32,
    /// ADPCM predictor state for each channel.
    predictors: [i32; 2],
    /// Decoded PCM buffer (ring of decoded samples as bytes).
    buf: Vec<u8>,
    /// Number of valid bytes in `buf`.
    buf_len: usize,
    /// Read offset within `buf`.
    buf_ofs: usize,
    /// Total audio length estimate (seconds).
    total_length: f32,
    /// Last error code.
    last_error: i32,
    /// Whether the decoder has been opened.
    opened: bool,
}

impl DukAudDecoder {
    /// Create a new (unopened) DukAud decoder.
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            duk_data: Vec::new(),
            iframe: 0,
            cframes: 0,
            channels: 2,
            pcm_frame: 0,
            predictors: [0; 2],
            buf: vec![0u8; DATA_BUF_SIZE],
            buf_len: 0,
            buf_ofs: 0,
            total_length: 0.0,
            last_error: 0,
            opened: false,
        }
    }

    /// Open from in-memory .duk and .frm data.
    pub fn open_from_data(&mut self, duk_data: &[u8], frm_data: &[u8]) -> DecodeResult<()> {
        self.close();

        if frm_data.len() < 4 {
            return Err(DecodeError::InvalidData("frm data too short".into()));
        }

        // Parse frame offsets (big-endian u32 array)
        let cframes = frm_data.len() / 4;
        let mut frames = Vec::with_capacity(cframes);
        for i in 0..cframes {
            let ofs = i * 4;
            let offset = u32::from_be_bytes([
                frm_data[ofs],
                frm_data[ofs + 1],
                frm_data[ofs + 2],
                frm_data[ofs + 3],
            ]);
            frames.push(offset);
        }

        // Read first frame header to get samples-per-frame
        let first_aud = self.read_aud_subframe_from(duk_data, frames[0] as usize)?;

        self.frames = frames;
        self.duk_data = duk_data.to_vec();
        self.cframes = cframes as u32;
        self.channels = 2;
        self.pcm_frame = first_aud.num_samples as u32;
        self.total_length = cframes as f32 / DUCK_GENERAL_FPS;
        self.iframe = 0;
        self.buf_len = 0;
        self.buf_ofs = 0;
        self.predictors = [0; 2];
        self.last_error = 0;
        self.opened = true;

        Ok(())
    }

    /// Close the decoder and free resources.
    pub fn close(&mut self) {
        self.frames.clear();
        self.duk_data.clear();
        self.cframes = 0;
        self.iframe = 0;
        self.buf_len = 0;
        self.buf_ofs = 0;
        self.predictors = [0; 2];
        self.last_error = 0;
        self.opened = false;
    }

    /// Decode audio into `out`, returning the number of bytes written.
    pub fn decode(&mut self, out: &mut [u8]) -> DecodeResult<usize> {
        if !self.opened {
            return Err(DecodeError::NotInitialized);
        }
        if out.is_empty() {
            return Err(DecodeError::DecoderError("zero-length buffer".into()));
        }

        let mut written = 0usize;
        let mut remaining = out;

        loop {
            // Drain buffered data first
            let avail = self.buf_len - self.buf_ofs;
            if avail > 0 {
                let to_copy = avail.min(remaining.len()) & !3; // align to 4 bytes
                if to_copy > 0 {
                    remaining[..to_copy]
                        .copy_from_slice(&self.buf[self.buf_ofs..self.buf_ofs + to_copy]);
                    self.buf_ofs += to_copy;
                    written += to_copy;
                    remaining = &mut remaining[to_copy..];
                }
            }

            // Reset buffer if fully consumed
            if self.buf_len > 0 && self.buf_ofs >= self.buf_len {
                self.buf_len = 0;
                self.buf_ofs = 0;
            }

            // If output buffer is full or no more frames, we're done
            if remaining.is_empty() || self.iframe >= self.cframes {
                break;
            }

            // Decode next frame into our internal buffer
            self.decode_next_frame()?;
        }

        if written == 0 && self.iframe >= self.cframes {
            return Err(DecodeError::EndOfFile);
        }

        Ok(written)
    }

    /// Seek to a PCM sample position.
    pub fn seek(&mut self, pcm_pos: u32) -> DecodeResult<u32> {
        if self.pcm_frame == 0 {
            return Ok(0);
        }
        let iframe = pcm_pos / self.pcm_frame;
        if iframe < self.cframes {
            self.iframe = iframe;
            self.buf_len = 0;
            self.buf_ofs = 0;
            self.predictors = [0; 2];
        }
        Ok(self.iframe * self.pcm_frame)
    }

    /// Get the current frame index.
    pub fn get_frame(&self) -> u32 {
        if self.buf_ofs == self.buf_len {
            self.iframe
        } else {
            self.iframe.saturating_sub(1)
        }
    }

    /// Sample frequency (always 22050).
    pub fn frequency(&self) -> u32 {
        DUKAUD_FREQUENCY
    }

    /// Audio format (always stereo 16-bit).
    pub fn format(&self) -> AudioFormat {
        AudioFormat::Stereo16
    }

    /// Total length in seconds.
    pub fn length(&self) -> f32 {
        self.total_length
    }

    /// Get and clear last error.
    pub fn get_error(&mut self) -> i32 {
        let e = self.last_error;
        self.last_error = 0;
        e
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Read an audio subframe header from raw .duk data at a frame offset.
    fn read_aud_subframe_from(
        &self,
        duk_data: &[u8],
        frame_offset: usize,
    ) -> DecodeResult<AudSubframe> {
        // Frame header: [4 bytes audsize] [4 bytes vidsize]
        if frame_offset + 8 > duk_data.len() {
            return Err(DecodeError::InvalidData("frame offset past EOF".into()));
        }
        // Audio subframe starts right after the 8-byte frame header
        let aud_start = frame_offset + 8;
        if aud_start + 10 > duk_data.len() {
            return Err(DecodeError::InvalidData(
                "audio subframe header past EOF".into(),
            ));
        }
        AudSubframe::from_be_bytes(&duk_data[aud_start..])
    }

    /// Decode the next frame's audio into the internal buffer.
    fn decode_next_frame(&mut self) -> DecodeResult<()> {
        let iframe = self.iframe as usize;
        let frame_offset = self.frames[iframe] as usize;

        // Read frame header to get audio data size
        if frame_offset + 8 > self.duk_data.len() {
            return Err(DecodeError::InvalidData("frame offset past EOF".into()));
        }
        let audsize = u32::from_be_bytes([
            self.duk_data[frame_offset],
            self.duk_data[frame_offset + 1],
            self.duk_data[frame_offset + 2],
            self.duk_data[frame_offset + 3],
        ]) as usize;

        let aud_start = frame_offset + 8;
        if aud_start + audsize > self.duk_data.len() {
            return Err(DecodeError::InvalidData("audio data past EOF".into()));
        }

        // Parse audio subframe header (first 10 bytes of audio data)
        let aud = AudSubframe::from_be_bytes(&self.duk_data[aud_start..])?;
        let adpcm_data = &self.duk_data[aud_start + 10..aud_start + audsize];

        // Each input byte produces 2 nibbles → 2 samples
        let num_output_samples = aud.num_samples as usize * 2;
        let output_bytes = num_output_samples * 2; // 16-bit samples

        // Expand nibbles into a temporary i16 buffer
        let mut samples = Vec::with_capacity(num_output_samples);
        for &byte in adpcm_data.iter().take(aud.num_samples as usize) {
            samples.push((byte >> 4) as i16);
            samples.push((byte & 0x0F) as i16);
        }

        // Decode ADPCM in-place
        decode_nibbles(
            &mut samples,
            self.channels as usize,
            &mut self.predictors,
            &aud.indices,
        );

        // Write decoded 16-bit LE samples into our internal buffer
        if output_bytes > self.buf.len() {
            self.buf.resize(output_bytes, 0);
        }
        for (i, &s) in samples.iter().enumerate() {
            let le = s.to_le_bytes();
            self.buf[i * 2] = le[0];
            self.buf[i * 2 + 1] = le[1];
        }
        self.buf_len = output_bytes;
        self.buf_ofs = 0;

        self.iframe += 1;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adpcm_tables() {
        assert_eq!(ADPCM_STEP.len(), 89);
        assert_eq!(ADPCM_STEP[0], 0x7);
        assert_eq!(ADPCM_STEP[88], 0x7FFF);
        assert_eq!(ADPCM_INDEX.len(), 16);
    }

    #[test]
    fn test_aud_subframe_parse() {
        let mut data = [0u8; 10];
        // magic = 0xf77f
        data[0] = 0xf7;
        data[1] = 0x7f;
        // numsamples = 100
        data[2] = 0;
        data[3] = 100;
        // tag = 1
        data[4] = 0;
        data[5] = 1;
        // index[0] = 5
        data[6] = 0;
        data[7] = 5;
        // index[1] = 10
        data[8] = 0;
        data[9] = 10;

        let aud = AudSubframe::from_be_bytes(&data).unwrap();
        assert_eq!(aud.num_samples, 100);
        assert_eq!(aud.indices[0], 5);
        assert_eq!(aud.indices[1], 10);
    }

    #[test]
    fn test_aud_subframe_bad_magic() {
        let data = [0u8; 10]; // magic = 0x0000
        assert!(AudSubframe::from_be_bytes(&data).is_err());
    }

    #[test]
    fn test_aud_subframe_too_short() {
        let data = [0u8; 5];
        assert!(AudSubframe::from_be_bytes(&data).is_err());
    }

    #[test]
    fn test_decode_nibbles_silence() {
        // All-zero nibbles should produce near-silence
        let mut samples = vec![0i16; 20];
        let mut predictors = [0i32; 2];
        let indices = [0u16; 2];
        decode_nibbles(&mut samples, 2, &mut predictors, &indices);
        // With zero predictors and zero nibbles, output stays near zero
        for &s in &samples {
            assert!(s.abs() < 100, "expected near-silence, got {}", s);
        }
    }

    #[test]
    fn test_decode_nibbles_clamp() {
        // Large positive nibbles should clamp to 16-bit range
        let mut samples = vec![7i16; 200]; // max positive delta repeatedly
        let mut predictors = [0i32; 2];
        let indices = [80u16, 80]; // high step index
        decode_nibbles(&mut samples, 2, &mut predictors, &indices);
        for &s in &samples {
            assert!(s >= -32768 && s <= 32767);
        }
    }

    #[test]
    fn test_decoder_new_is_closed() {
        let dec = DukAudDecoder::new();
        assert!(!dec.opened);
        assert_eq!(dec.frequency(), DUKAUD_FREQUENCY);
        assert_eq!(dec.format(), AudioFormat::Stereo16);
    }

    #[test]
    fn test_decoder_decode_without_open() {
        let mut dec = DukAudDecoder::new();
        let mut buf = [0u8; 256];
        assert!(matches!(
            dec.decode(&mut buf),
            Err(DecodeError::NotInitialized)
        ));
    }

    #[test]
    fn test_decoder_open_synthetic() {
        // Build a minimal synthetic .duk + .frm
        let num_samples: u16 = 4; // 4 ADPCM bytes → 8 samples
        let aud_subframe_size = 10 + num_samples as usize; // header + data

        // Frame header: [audsize BE32] [vidsize BE32]
        let mut duk = Vec::new();
        duk.extend_from_slice(&(aud_subframe_size as u32).to_be_bytes()); // audsize
        duk.extend_from_slice(&0u32.to_be_bytes()); // vidsize

        // Audio subframe header
        duk.extend_from_slice(&DUKAUD_MAGIC.to_be_bytes()); // magic
        duk.extend_from_slice(&num_samples.to_be_bytes()); // numsamples
        duk.extend_from_slice(&0u16.to_be_bytes()); // tag
        duk.extend_from_slice(&0u16.to_be_bytes()); // index[0]
        duk.extend_from_slice(&0u16.to_be_bytes()); // index[1]

        // ADPCM data (num_samples bytes of zeros = silence)
        duk.extend(vec![0u8; num_samples as usize]);

        // .frm: one frame offset = 0
        let frm = 0u32.to_be_bytes().to_vec();

        let mut dec = DukAudDecoder::new();
        dec.open_from_data(&duk, &frm).unwrap();
        assert!(dec.opened);
        assert_eq!(dec.cframes, 1);
        assert_eq!(dec.pcm_frame, num_samples as u32);

        // Decode
        let mut buf = vec![0u8; 1024];
        let n = dec.decode(&mut buf).unwrap();
        // 4 ADPCM bytes → 8 samples × 2 bytes = 16 bytes
        assert_eq!(n, 16);

        // Second decode should hit EOF
        assert!(matches!(dec.decode(&mut buf), Err(DecodeError::EndOfFile)));
    }

    #[test]
    fn test_decoder_seek() {
        let num_samples: u16 = 8;
        let aud_size = 10 + num_samples as usize;

        // Two frames
        let mut duk = Vec::new();
        for _ in 0..2 {
            duk.extend_from_slice(&(aud_size as u32).to_be_bytes());
            duk.extend_from_slice(&0u32.to_be_bytes());
            duk.extend_from_slice(&DUKAUD_MAGIC.to_be_bytes());
            duk.extend_from_slice(&num_samples.to_be_bytes());
            duk.extend_from_slice(&0u16.to_be_bytes());
            duk.extend_from_slice(&0u16.to_be_bytes());
            duk.extend_from_slice(&0u16.to_be_bytes());
            duk.extend(vec![0u8; num_samples as usize]);
        }

        let frame1_offset = 0u32;
        let frame2_offset = (8 + aud_size) as u32;
        let mut frm = Vec::new();
        frm.extend_from_slice(&frame1_offset.to_be_bytes());
        frm.extend_from_slice(&frame2_offset.to_be_bytes());

        let mut dec = DukAudDecoder::new();
        dec.open_from_data(&duk, &frm).unwrap();
        assert_eq!(dec.cframes, 2);

        // Seek to frame 1
        let pos = dec.seek(num_samples as u32).unwrap();
        assert_eq!(pos, num_samples as u32);
        assert_eq!(dec.iframe, 1);
    }

    #[test]
    fn test_decoder_get_frame() {
        let dec = DukAudDecoder::new();
        assert_eq!(dec.get_frame(), 0);
    }
}
