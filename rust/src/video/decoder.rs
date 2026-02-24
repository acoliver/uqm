//! DukVid video decoder implementation
//!
//! This module implements the DukVid (.duk) video format decoder, ported
//! from the original C implementation in `dukvid.c`.
//!
//! # Usage
//!
//! ```ignore
//! use std::path::Path;
//! use uqm_rust::video::decoder::DukVideoDecoder;
//!
//! let decoder = DukVideoDecoder::open(Path::new("content/addons"), "intro")?;
//! let frame = decoder.decode_frame(0)?;
//! ```
//!
//! # File Format
//!
//! DukVid videos consist of four files:
//! - `.duk` - Video stream data (compressed frame data)
//! - `.frm` - Frame offset table (array of u32 big-endian)
//! - `.hdr` - Video header (version, dimensions, luma/chroma prototypes)
//! - `.tbl` - Vector table for decoding (256 * 16 bytes)

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use super::{
    DukVideoDeltas, DukVideoHeader, PixelFormat, VideoError, VideoFrame, DUCK_END_OF_SEQUENCE,
    DUCK_FPS, DUCK_MAX_FRAME_SIZE, NUM_VECTORS, NUM_VEC_ITEMS,
};

// ============================================================================
// DukVideoDecoder
// ============================================================================

/// DukVid video decoder
///
/// Decodes DukVid (.duk) format videos used in UQM for intro and cutscenes.
/// The decoder loads all necessary files on open and decodes frames on demand.
///
/// # Memory
///
/// The decoder loads the entire .duk file into memory for random access to frames.
/// For a typical UQM video, this is a few megabytes.
///
/// # Performance
///
/// The decoder reuses internal buffers to avoid per-frame allocations:
/// - `decode_buffer` - Packed pixel pairs from decompression
/// - `output_buffer` - RGBA output pixels for rendering
///
/// # Thread Safety
///
/// The decoder is not thread-safe. Use external synchronization if accessing
/// from multiple threads, or create separate decoder instances.
#[derive(Clone)]
pub struct DukVideoDecoder {
    /// Video header with dimensions and prototypes
    header: DukVideoHeader,
    /// Pre-computed delta tables for decoding
    deltas: DukVideoDeltas,
    /// Frame offset table (byte offsets into .duk data)
    frame_offsets: Vec<u32>,
    /// Current frame index (0-based)
    current_frame: u32,
    /// Decode buffer for packed pixel pairs (width * height/2)
    decode_buffer: Vec<u32>,
    /// Reusable output buffer for rendered frames (width * height)
    output_buffer: Vec<u32>,
    /// Loaded .duk file data
    duk_data: Vec<u8>,
    /// Pixel format for output conversion
    pixel_format: PixelFormat,
}

impl DukVideoDecoder {
    /// Opens a DukVid video from the specified base path and basename
    ///
    /// This loads all four component files (.duk, .frm, .hdr, .tbl) and
    /// initializes the decoder for frame decoding.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Directory containing the video files
    /// * `basename` - Base filename without extension (e.g., "intro")
    ///
    /// # Returns
    ///
    /// * `Ok(DukVideoDecoder)` - Decoder ready for frame decoding
    /// * `Err(VideoError)` - If any file is missing or corrupted
    ///
    /// # Example
    ///
    /// ```ignore
    /// let decoder = DukVideoDecoder::open(Path::new("content"), "intro")?;
    /// ```
    pub fn open(base_path: &Path, basename: &str) -> Result<Self, VideoError> {
        // Read frame offsets from .frm file
        let frm_path = base_path.join(format!("{}.frm", basename));
        let frame_offsets = Self::read_frame_offsets(&frm_path)?;

        // Read header from .hdr file
        let hdr_path = base_path.join(format!("{}.hdr", basename));
        let header = Self::read_header(&hdr_path)?;

        // Read vector table from .tbl file
        let tbl_path = base_path.join(format!("{}.tbl", basename));
        let vectors = Self::read_vectors(&tbl_path)?;

        // Compute deltas from header prototypes and vectors
        let luma_protos: [i32; 8] = header.lumas.map(|x| x as i32);
        let chroma_protos: [i32; 8] = header.chromas.map(|x| x as i32);
        let deltas = DukVideoDeltas::from_vectors(&luma_protos, &chroma_protos, &vectors)?;

        // Read .duk stream data
        let duk_path = base_path.join(format!("{}.duk", basename));
        let duk_data = Self::read_file(&duk_path)?;

        // Allocate decode buffer (width * height/2 packed pixel pairs)
        let decode_size = (header.width() * header.height() / 2) as usize;
        let decode_buffer = vec![0u32; decode_size];

        // Allocate output buffer (width * height pixels)
        let output_size = (header.width() * header.height()) as usize;
        let output_buffer = vec![0u32; output_size];

        Ok(Self {
            header,
            deltas,
            frame_offsets,
            current_frame: 0,
            decode_buffer,
            output_buffer,
            duk_data,
            pixel_format: PixelFormat::default(),
        })
    }

    /// Opens a DukVid video from pre-loaded data buffers
    ///
    /// This allows opening videos when the data has already been loaded,
    /// such as through the UIO virtual filesystem.
    ///
    /// # Arguments
    ///
    /// * `hdr_data` - Contents of the .hdr file
    /// * `tbl_data` - Contents of the .tbl file  
    /// * `frm_data` - Contents of the .frm file
    /// * `duk_data` - Contents of the .duk file
    ///
    /// # Returns
    ///
    /// * `Ok(DukVideoDecoder)` - Decoder ready for frame decoding
    /// * `Err(VideoError)` - If any data is invalid
    pub fn open_from_data(
        hdr_data: &[u8],
        tbl_data: &[u8],
        frm_data: &[u8],
        duk_data: &[u8],
    ) -> Result<Self, VideoError> {
        // Parse header
        let header = DukVideoHeader::from_bytes(hdr_data)?;

        // Parse frame offsets
        if frm_data.len() % 4 != 0 {
            return Err(VideoError::BadFile(format!(
                "Frame file size {} not multiple of 4",
                frm_data.len()
            )));
        }
        let count = frm_data.len() / 4;
        let mut frame_offsets = Vec::with_capacity(count);
        for i in 0..count {
            let offset = i * 4;
            let value = u32::from_be_bytes([
                frm_data[offset],
                frm_data[offset + 1],
                frm_data[offset + 2],
                frm_data[offset + 3],
            ]);
            frame_offsets.push(value);
        }

        // Validate vector table size
        let expected = NUM_VECTORS * NUM_VEC_ITEMS;
        if tbl_data.len() < expected {
            return Err(VideoError::BadFile(format!(
                "Vector table too short: {} bytes, expected {}",
                tbl_data.len(),
                expected
            )));
        }

        // Compute deltas from header prototypes and vectors
        let luma_protos: [i32; 8] = header.lumas.map(|x| x as i32);
        let chroma_protos: [i32; 8] = header.chromas.map(|x| x as i32);
        let deltas = DukVideoDeltas::from_vectors(&luma_protos, &chroma_protos, tbl_data)?;

        // Allocate decode buffer (width * height/2 packed pixel pairs)
        let decode_size = (header.width() * header.height() / 2) as usize;
        let decode_buffer = vec![0u32; decode_size];

        // Allocate output buffer (width * height pixels)
        let output_size = (header.width() * header.height()) as usize;
        let output_buffer = vec![0u32; output_size];

        Ok(Self {
            header,
            deltas,
            frame_offsets,
            current_frame: 0,
            decode_buffer,
            output_buffer,
            duk_data: duk_data.to_vec(),
            pixel_format: PixelFormat::default(),
        })
    }

    /// Decodes a specific frame and returns it as a VideoFrame
    ///
    /// # Arguments
    ///
    /// * `frame` - Frame index (0-based)
    ///
    /// # Returns
    ///
    /// * `Ok(VideoFrame)` - Decoded frame with RGBA pixel data
    /// * `Err(VideoError::Eof)` - If frame is out of range
    /// * `Err(VideoError::BadFile)` - If frame data is corrupted
    ///
    /// # Example
    ///
    /// ```ignore
    /// let frame = decoder.decode_frame(0)?;
    /// assert_eq!(frame.width, decoder.width());
    /// ```
    pub fn decode_frame(&mut self, frame: u32) -> Result<VideoFrame, VideoError> {
        if frame >= self.frame_count() {
            return Err(VideoError::Eof);
        }

        self.current_frame = frame;

        // Get frame offset from table
        let offset = self.frame_offsets[frame as usize] as usize;

        // Read frame header (vofs: u32, vsize: u32)
        if offset + 8 > self.duk_data.len() {
            return Err(VideoError::BadFile("Frame header truncated".into()));
        }

        let vofs = u32::from_be_bytes([
            self.duk_data[offset],
            self.duk_data[offset + 1],
            self.duk_data[offset + 2],
            self.duk_data[offset + 3],
        ]) as usize;

        let vsize = u32::from_be_bytes([
            self.duk_data[offset + 4],
            self.duk_data[offset + 5],
            self.duk_data[offset + 6],
            self.duk_data[offset + 7],
        ]) as usize;

        if vsize > DUCK_MAX_FRAME_SIZE {
            return Err(VideoError::OutOfBuffer);
        }

        // Frame data starts at offset + 8 + vofs
        let data_start = offset + 8 + vofs;
        let data_end = data_start + vsize;

        if data_end > self.duk_data.len() {
            return Err(VideoError::BadFile("Frame data truncated".into()));
        }

        // Use slice directly - no copy needed
        let frame_size = data_end - data_start;

        // Check version byte at offset 0 to determine decoder
        if frame_size < 2 {
            return Err(VideoError::BadFile("Frame data too short".into()));
        }

        let ver = u16::from_be_bytes([self.duk_data[data_start], self.duk_data[data_start + 1]]);

        // Decode into internal buffer (frame data starts at offset 0x10)
        if frame_size < 0x10 {
            return Err(VideoError::BadFile("Frame data missing payload".into()));
        }

        // Payload starts at data_start + 0x10
        let payload_start = data_start + 0x10;
        let payload_end = data_end;

        if ver == 0x0300 {
            self.decode_frame_v3_range(payload_start, payload_end)?;
        } else {
            self.decode_frame_v2_range(payload_start, payload_end)?;
        }

        // Convert decode buffer to output frame (reuses output_buffer)
        self.render_frame(frame)
    }

    /// Returns the total number of frames in the video
    #[inline]
    pub fn frame_count(&self) -> u32 {
        self.frame_offsets.len() as u32
    }

    /// Returns the video width in pixels
    #[inline]
    pub fn width(&self) -> u32 {
        self.header.width()
    }

    /// Returns the video height in pixels
    #[inline]
    pub fn height(&self) -> u32 {
        self.header.height()
    }

    /// Returns the frame rate in frames per second
    #[inline]
    pub fn fps(&self) -> f32 {
        DUCK_FPS
    }

    /// Returns the total video duration in seconds
    #[inline]
    pub fn duration(&self) -> f32 {
        self.frame_count() as f32 / DUCK_FPS
    }

    /// Returns the current frame index
    #[inline]
    pub fn current_frame(&self) -> u32 {
        self.current_frame
    }

    /// Returns the header information
    pub fn header(&self) -> &DukVideoHeader {
        &self.header
    }

    /// Sets the pixel format for output conversion
    pub fn set_pixel_format(&mut self, format: PixelFormat) {
        self.pixel_format = format;
    }

    /// Returns the current pixel format
    pub fn pixel_format(&self) -> &PixelFormat {
        &self.pixel_format
    }

    // ========================================================================
    // Private methods - File loading
    // ========================================================================

    /// Reads a file entirely into a byte vector
    fn read_file(path: &Path) -> Result<Vec<u8>, VideoError> {
        let mut file = File::open(path).map_err(|e| {
            VideoError::IoError(format!("Failed to open {}: {}", path.display(), e))
        })?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| {
            VideoError::IoError(format!("Failed to read {}: {}", path.display(), e))
        })?;

        Ok(data)
    }

    /// Reads frame offsets from .frm file
    fn read_frame_offsets(path: &Path) -> Result<Vec<u32>, VideoError> {
        let data = Self::read_file(path)?;

        if data.len() % 4 != 0 {
            return Err(VideoError::BadFile(format!(
                "Frame file size {} not multiple of 4",
                data.len()
            )));
        }

        let count = data.len() / 4;
        let mut offsets = Vec::with_capacity(count);

        for i in 0..count {
            let offset = i * 4;
            let value = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offsets.push(value);
        }

        Ok(offsets)
    }

    /// Reads header from .hdr file
    fn read_header(path: &Path) -> Result<DukVideoHeader, VideoError> {
        let data = Self::read_file(path)?;
        DukVideoHeader::from_bytes(&data)
    }

    /// Reads vector table from .tbl file
    fn read_vectors(path: &Path) -> Result<Vec<u8>, VideoError> {
        let data = Self::read_file(path)?;

        let expected = NUM_VECTORS * NUM_VEC_ITEMS;
        if data.len() < expected {
            return Err(VideoError::BadFile(format!(
                "Vector table too short: {} bytes, expected {}",
                data.len(),
                expected
            )));
        }

        Ok(data)
    }

    /// Converts decode buffer (packed pairs) to output VideoFrame
    ///
    /// Reuses internal output_buffer to avoid per-frame allocation.
    fn render_frame(&mut self, frame_idx: u32) -> Result<VideoFrame, VideoError> {
        let w = self.width() as usize;
        let h = self.height() as usize;
        let timestamp = frame_idx as f32 / DUCK_FPS;

        // Decode buffer contains packed pixel pairs (2 rows per entry)
        // Each entry holds: upper_pixel (bits 31-16) and lower_pixel (bits 15-0)
        let half_h = h / 2;

        for y in 0..half_h {
            for x in 0..w {
                let pair = self.decode_buffer[y * w + x];

                // Upper 16 bits -> even row pixel
                let upper = (pair >> 16) as u16;
                // Lower 16 bits -> odd row pixel
                let lower = (pair & 0xFFFF) as u16;

                self.output_buffer[y * 2 * w + x] = self.pixel_format.convert_from_duk(upper);
                self.output_buffer[(y * 2 + 1) * w + x] = self.pixel_format.convert_from_duk(lower);
            }
        }

        // Clone the output buffer for VideoFrame (caller owns the frame)
        VideoFrame::from_data(w as u32, h as u32, self.output_buffer.clone(), timestamp)
    }

    /// Decodes a frame using V2 algorithm with direct duk_data access
    ///
    /// V2 decoder processes 4x2 pixel blocks with running accumulators
    /// for luma and chroma, using XOR corrections on alternate pixels.
    fn decode_frame_v2_range(
        &mut self,
        payload_start: usize,
        payload_end: usize,
    ) -> Result<(), VideoError> {
        if payload_start >= payload_end {
            return Err(VideoError::BadFile("Empty frame data".into()));
        }

        let wb = self.header.block_dimensions.0 as usize;
        let hb = self.header.block_dimensions.1 as usize;
        let w = wb * 4;

        let mut src_idx = payload_start;
        let mut ivec = self.duk_data[src_idx] as usize;
        src_idx += 1;
        let mut iseq = 0;

        for y in 0..hb {
            let d_p0_base = y * w * 2;
            let d_p1_base = d_p0_base + w;

            let mut accum0: i32 = 0;
            let mut accum1: i32 = 0;
            let mut corr0: i32 = 0;
            let mut corr1: i32 = 0;

            for x in 0..wb {
                // Get previous row pixels or zero
                let mut pix: [i32; 4] = if y == 0 {
                    [0, 0, 0, 0]
                } else {
                    let prev_base = (y - 1) * w * 2 + x * 4;
                    [
                        self.decode_buffer[prev_base] as i32,
                        self.decode_buffer[prev_base + 1] as i32,
                        self.decode_buffer[prev_base + 2] as i32,
                        self.decode_buffer[prev_base + 3] as i32,
                    ]
                };

                // Start with chroma delta (matches C: iSeq++ then iSeq++)
                let delta = self.deltas.chromas[ivec][iseq];
                iseq += 1;
                // skip corrector (matches C: iSeq++;)
                iseq += 1;

                accum0 = accum0.wrapping_add(delta >> 1);

                if (delta & DUCK_END_OF_SEQUENCE) != 0 {
                    if src_idx < payload_end {
                        ivec = self.duk_data[src_idx] as usize;
                        src_idx += 1;
                    }
                    iseq = 0;
                }

                // Line 0 (4 pixels)
                for i in 0..4 {
                    // Bounds check for iseq
                    if iseq + 1 >= NUM_VEC_ITEMS {
                        break;
                    }
                    let delta = self.deltas.lumas[ivec][iseq];
                    iseq += 1;
                    let corr = self.deltas.lumas[ivec][iseq];
                    iseq += 1;

                    accum0 = accum0.wrapping_add(delta >> 1);
                    corr0 ^= corr;
                    pix[i] = pix[i].wrapping_add(accum0);
                    pix[i] ^= corr0;

                    if (delta & DUCK_END_OF_SEQUENCE) != 0 {
                        if src_idx < payload_end {
                            ivec = self.duk_data[src_idx] as usize;
                            src_idx += 1;
                        }
                        iseq = 0;
                    }

                    self.decode_buffer[d_p0_base + x * 4 + i] = pix[i] as u32;
                }

                // Line 1 (4 pixels)
                for i in 0..4 {
                    // Bounds check for iseq
                    if iseq + 1 >= NUM_VEC_ITEMS {
                        break;
                    }
                    let delta = self.deltas.lumas[ivec][iseq];
                    iseq += 1;
                    let corr = self.deltas.lumas[ivec][iseq];
                    iseq += 1;

                    accum1 = accum1.wrapping_add(delta >> 1);
                    corr1 ^= corr;
                    pix[i] = pix[i].wrapping_add(accum1);
                    pix[i] ^= corr1;

                    if (delta & DUCK_END_OF_SEQUENCE) != 0 {
                        if src_idx < payload_end {
                            ivec = self.duk_data[src_idx] as usize;
                            src_idx += 1;
                        }
                        iseq = 0;
                    }

                    self.decode_buffer[d_p1_base + x * 4 + i] = pix[i] as u32;
                }
            }
        }

        Ok(())
    }

    /// Decodes a frame using V3 algorithm with direct duk_data access
    ///
    /// V3 decoder uses simpler single-line processing without XOR corrections.
    fn decode_frame_v3_range(
        &mut self,
        payload_start: usize,
        payload_end: usize,
    ) -> Result<(), VideoError> {
        if payload_start >= payload_end {
            return Err(VideoError::BadFile("Empty frame data".into()));
        }

        let wb = self.header.block_dimensions.0 as usize;
        let hb = self.header.block_dimensions.1 as usize * 2; // V3 doubles hb
        let w = wb * 4;

        let mut src_idx = payload_start;
        let mut ivec = self.duk_data[src_idx] as usize;
        src_idx += 1;
        let mut iseq = 0;

        for y in 0..hb {
            let d_p_base = y * w;
            let mut accum: i32 = 0;

            for x in 0..wb {
                // Bounds check for iseq (chroma)
                if iseq + 1 >= NUM_VEC_ITEMS {
                    // Reset when running out of delta items
                    iseq = 0;
                }

                // Start with chroma delta (C: iseq += 2 total)
                let delta = self.deltas.chromas[ivec][iseq];
                iseq += 2; // Skip corrector

                accum = accum.wrapping_add(delta >> 1);

                if (delta & DUCK_END_OF_SEQUENCE) != 0 {
                    if src_idx < payload_end {
                        ivec = self.duk_data[src_idx] as usize;
                        src_idx += 1;
                    }
                    iseq = 0;
                }

                // 4 pixels per block
                for i in 0..4 {
                    let pix = if y == 0 {
                        0i32
                    } else {
                        self.decode_buffer[d_p_base - w + x * 4 + i] as i32
                    };

                    // Bounds check for iseq
                    if iseq + 1 >= NUM_VEC_ITEMS {
                        break;
                    }

                    // Get next luma delta
                    let delta = self.deltas.lumas[ivec][iseq];
                    iseq += 2; // Skip corrector

                    accum = accum.wrapping_add(delta >> 1);
                    let result = pix.wrapping_add(accum);

                    if (delta & DUCK_END_OF_SEQUENCE) != 0 {
                        if src_idx < payload_end {
                            ivec = self.duk_data[src_idx] as usize;
                            src_idx += 1;
                        }
                        iseq = 0;
                    }

                    self.decode_buffer[d_p_base + x * 4 + i] = result as u32;
                }
            }
        }

        Ok(())
    }
}

impl fmt::Debug for DukVideoDecoder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DukVideoDecoder")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("frame_count", &self.frame_count())
            .field("fps", &self.fps())
            .field("duration", &format!("{:.2}s", self.duration()))
            .field("current_frame", &self.current_frame)
            .field("duk_size", &self.duk_data.len())
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Creates a minimal valid video file set for testing
    fn create_test_video(dir: &Path, basename: &str, frames: u32, wb: u16, hb: u16) {
        // Create .frm file (frame offsets)
        let mut frm_data = Vec::new();
        for i in 0..frames {
            frm_data.extend_from_slice(&(i * 0x100).to_be_bytes());
        }
        std::fs::write(dir.join(format!("{}.frm", basename)), &frm_data).unwrap();

        // Create .hdr file
        let mut hdr_data = vec![0u8; 48];
        hdr_data[0..4].copy_from_slice(&3u32.to_be_bytes()); // version
        hdr_data[4..8].copy_from_slice(&0u32.to_be_bytes()); // scrn_x_ofs
        hdr_data[8..12].copy_from_slice(&0u32.to_be_bytes()); // scrn_y_ofs
        hdr_data[12..14].copy_from_slice(&wb.to_be_bytes()); // width in blocks
        hdr_data[14..16].copy_from_slice(&hb.to_be_bytes()); // height in blocks
                                                             // lumas and chromas default to 0
        std::fs::write(dir.join(format!("{}.hdr", basename)), &hdr_data).unwrap();

        // Create .tbl file (256 * 16 bytes)
        let tbl_data = vec![0u8; NUM_VECTORS * NUM_VEC_ITEMS];
        std::fs::write(dir.join(format!("{}.tbl", basename)), &tbl_data).unwrap();

        // Create .duk file with frame data
        let mut duk_data = Vec::new();
        for i in 0..frames {
            let frame_offset = i as usize * 0x100;
            // Pad to frame offset
            while duk_data.len() < frame_offset {
                duk_data.push(0);
            }
            // Frame header: vofs=0, vsize=0x20
            duk_data.extend_from_slice(&0u32.to_be_bytes()); // vofs
            duk_data.extend_from_slice(&0x20u32.to_be_bytes()); // vsize
                                                                // Frame data (version + padding + payload)
            duk_data.extend_from_slice(&0x0300u16.to_be_bytes()); // V3 version
            for _ in 0..0x1E {
                duk_data.push(0);
            }
        }
        std::fs::write(dir.join(format!("{}.duk", basename)), &duk_data).unwrap();
    }

    #[test]
    fn test_decoder_open_missing_files() {
        let temp_dir = TempDir::new().unwrap();

        // Missing all files
        let result = DukVideoDecoder::open(temp_dir.path(), "missing");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VideoError::IoError(_)));

        // Create only .frm file
        std::fs::write(temp_dir.path().join("partial.frm"), &[0u8; 4]).unwrap();
        let result = DukVideoDecoder::open(temp_dir.path(), "partial");
        assert!(result.is_err());
    }

    #[test]
    fn test_decoder_frame_count() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 10, 4, 4);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        assert_eq!(decoder.frame_count(), 10);
    }

    #[test]
    fn test_decoder_dimensions() {
        let temp_dir = TempDir::new().unwrap();
        // 10 blocks x 8 blocks = 40x32 pixels
        create_test_video(temp_dir.path(), "test", 5, 10, 8);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        assert_eq!(decoder.width(), 40);
        assert_eq!(decoder.height(), 32);
    }

    #[test]
    fn test_decoder_fps() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 5, 4, 4);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        assert!((decoder.fps() - DUCK_FPS).abs() < 0.001);
    }

    #[test]
    fn test_decoder_duration() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 100, 4, 4);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        let expected = 100.0 / DUCK_FPS;
        assert!((decoder.duration() - expected).abs() < 0.01);
    }

    #[test]
    fn test_decode_buffer_init() {
        let temp_dir = TempDir::new().unwrap();
        // 8x6 blocks = 32x24 pixels = 32*12 decode buffer entries
        create_test_video(temp_dir.path(), "test", 5, 8, 6);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        // decode_buffer size = width * height / 2
        assert_eq!(decoder.decode_buffer.len(), 32 * 24 / 2);
    }

    #[test]
    fn test_frame_seek_bounds() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 10, 4, 4);

        let mut decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();

        // Valid frame
        assert!(decoder.decode_frame(0).is_ok());
        assert!(decoder.decode_frame(9).is_ok());

        // Out of bounds
        assert!(matches!(decoder.decode_frame(10), Err(VideoError::Eof)));
        assert!(matches!(decoder.decode_frame(100), Err(VideoError::Eof)));
    }

    #[test]
    fn test_decoder_debug() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 10, 8, 6);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        let debug_str = format!("{:?}", decoder);

        assert!(debug_str.contains("DukVideoDecoder"));
        assert!(debug_str.contains("width"));
        assert!(debug_str.contains("height"));
        assert!(debug_str.contains("frame_count"));
    }

    #[test]
    fn test_frame_offsets_parsing() {
        let temp_dir = TempDir::new().unwrap();

        // Create .frm file with specific offsets
        let offsets: Vec<u32> = vec![0, 0x100, 0x200, 0x300];
        let mut frm_data = Vec::new();
        for offset in &offsets {
            frm_data.extend_from_slice(&offset.to_be_bytes());
        }
        std::fs::write(temp_dir.path().join("test.frm"), &frm_data).unwrap();

        let result = DukVideoDecoder::read_frame_offsets(&temp_dir.path().join("test.frm"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), offsets);
    }

    #[test]
    fn test_invalid_frm_size() {
        let temp_dir = TempDir::new().unwrap();

        // Create .frm file with invalid size (not multiple of 4)
        std::fs::write(temp_dir.path().join("bad.frm"), &[0u8, 1, 2]).unwrap();

        let result = DukVideoDecoder::read_frame_offsets(&temp_dir.path().join("bad.frm"));
        assert!(matches!(result, Err(VideoError::BadFile(_))));
    }

    #[test]
    fn test_decoder_pixel_format() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 5, 4, 4);

        let mut decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();

        // Default format
        assert_eq!(decoder.pixel_format().bytes_per_pixel, 4);

        // Change format
        decoder.set_pixel_format(PixelFormat::rgb565());
        assert_eq!(decoder.pixel_format().bytes_per_pixel, 2);
    }

    #[test]
    fn test_decoder_current_frame() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 10, 4, 4);

        let mut decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        assert_eq!(decoder.current_frame(), 0);

        decoder.decode_frame(5).ok();
        assert_eq!(decoder.current_frame(), 5);
    }

    #[test]
    fn test_decoder_header_access() {
        let temp_dir = TempDir::new().unwrap();
        create_test_video(temp_dir.path(), "test", 5, 10, 8);

        let decoder = DukVideoDecoder::open(temp_dir.path(), "test").unwrap();
        let header = decoder.header();

        assert_eq!(header.block_dimensions, (10, 8));
        assert_eq!(header.width(), 40);
        assert_eq!(header.height(), 32);
    }
}
