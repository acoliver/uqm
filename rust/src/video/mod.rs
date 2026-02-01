//! Video subsystem for UQM
//!
//! This module provides types and utilities for video decoding,
//! specifically for the DukVid (.duk) format used in UQM.
//!
//! # DukVid Format
//!
//! The DukVid format consists of multiple files:
//! - `.duk` - Video stream data
//! - `.frm` - Frame offset table (array of u32 big-endian)
//! - `.hdr` - Video header with dimensions and lookup tables
//! - `.tbl` - Vector table for decoding
//!
//! # Example
//!
//! ```
//! use uqm_rust::video::{DukVideoHeader, DUCK_FPS};
//!
//! // Calculate video duration
//! fn video_duration(frame_count: u32) -> f32 {
//!     frame_count as f32 / DUCK_FPS
//! }
//! ```

pub mod decoder;
pub mod ffi;
pub mod player;
pub mod scaler;

use std::fmt;
use std::io;

// ============================================================================
// Constants
// ============================================================================

/// Default frame rate for DukVid videos (in frames per second)
pub const DUCK_FPS: f32 = 14.622;

/// Maximum size of a single frame in bytes (32KB)
pub const DUCK_MAX_FRAME_SIZE: usize = 0x8000;

/// Number of items per vector in the decoding table
pub const NUM_VEC_ITEMS: usize = 16;

/// Number of vectors in the decoding table
pub const NUM_VECTORS: usize = 256;

/// End of sequence marker in delta values
pub const DUCK_END_OF_SEQUENCE: i32 = 1;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during video operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VideoError {
    /// Invalid or corrupted file format
    BadFile(String),
    /// End of file reached unexpectedly
    Eof,
    /// Buffer too small for frame data
    OutOfBuffer,
    /// Video decoder not initialized
    NotInitialized,
    /// I/O error during file operations
    IoError(String),
    /// Invalid argument provided
    BadArg(String),
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VideoError::BadFile(msg) => write!(f, "Bad file: {}", msg),
            VideoError::Eof => write!(f, "End of file"),
            VideoError::OutOfBuffer => write!(f, "Buffer overflow"),
            VideoError::NotInitialized => write!(f, "Video not initialized"),
            VideoError::IoError(msg) => write!(f, "I/O error: {}", msg),
            VideoError::BadArg(msg) => write!(f, "Bad argument: {}", msg),
        }
    }
}

impl std::error::Error for VideoError {}

impl From<io::Error> for VideoError {
    fn from(err: io::Error) -> Self {
        VideoError::IoError(err.to_string())
    }
}

// ============================================================================
// Pixel Format
// ============================================================================

/// Describes the pixel format for video rendering
///
/// This structure defines how pixel color components are packed into bytes,
/// including bit shifts and precision losses for each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelFormat {
    /// Number of bytes per pixel (1, 2, 3, or 4)
    pub bytes_per_pixel: u8,
    /// Number of bits to shift red component left
    pub r_shift: u8,
    /// Number of bits to shift green component left
    pub g_shift: u8,
    /// Number of bits to shift blue component left
    pub b_shift: u8,
    /// Number of bits to shift alpha component left
    pub a_shift: u8,
    /// Number of bits lost from red component (8 - red_bits)
    pub r_loss: u8,
    /// Number of bits lost from green component (8 - green_bits)
    pub g_loss: u8,
    /// Number of bits lost from blue component (8 - blue_bits)
    pub b_loss: u8,
    /// Number of bits lost from alpha component (8 - alpha_bits)
    pub a_loss: u8,
}

impl Default for PixelFormat {
    /// Creates a default 32-bit RGBA pixel format
    fn default() -> Self {
        // Standard 32-bit RGBA format (8 bits per channel, no loss)
        Self {
            bytes_per_pixel: 4,
            r_shift: 0,
            g_shift: 8,
            b_shift: 16,
            a_shift: 24,
            r_loss: 0,
            g_loss: 0,
            b_loss: 0,
            a_loss: 0,
        }
    }
}

impl PixelFormat {
    /// Creates a new pixel format with the specified parameters
    pub fn new(
        bytes_per_pixel: u8,
        r_shift: u8,
        g_shift: u8,
        b_shift: u8,
        a_shift: u8,
        r_loss: u8,
        g_loss: u8,
        b_loss: u8,
        a_loss: u8,
    ) -> Self {
        Self {
            bytes_per_pixel,
            r_shift,
            g_shift,
            b_shift,
            a_shift,
            r_loss,
            g_loss,
            b_loss,
            a_loss,
        }
    }

    /// Creates a 16-bit RGB565 pixel format
    pub fn rgb565() -> Self {
        Self {
            bytes_per_pixel: 2,
            r_shift: 11,
            g_shift: 5,
            b_shift: 0,
            a_shift: 0,
            r_loss: 3, // 8 - 5 = 3
            g_loss: 2, // 8 - 6 = 2
            b_loss: 3, // 8 - 5 = 3
            a_loss: 8, // No alpha
        }
    }

    /// Creates a 15-bit RGB555 pixel format (as used in DukVid internal format)
    pub fn rgb555() -> Self {
        Self {
            bytes_per_pixel: 2,
            r_shift: 10,
            g_shift: 5,
            b_shift: 0,
            a_shift: 0,
            r_loss: 3, // 8 - 5 = 3
            g_loss: 3, // 8 - 5 = 3
            b_loss: 3, // 8 - 5 = 3
            a_loss: 8, // No alpha
        }
    }

    /// Converts an internal 15-bit DukVid pixel to this format
    ///
    /// DukVid uses internal 15-bit RGB555 format:
    /// - Red: bits 10-14
    /// - Green: bits 5-9
    /// - Blue: bits 0-4
    pub fn convert_from_duk(&self, pix: u16) -> u32 {
        // Extract RGB from 15-bit format
        let r = ((pix >> 7) & 0xf8) as u32;
        let g = ((pix >> 2) & 0xf8) as u32;
        let b = ((pix << 3) & 0xf8) as u32;
        
        // Full alpha (0xFF) for opaque pixels
        let a = 0xFF_u32;

        // Apply format shifts and losses
        ((r >> self.r_loss as u32) << self.r_shift as u32)
            | ((g >> self.g_loss as u32) << self.g_shift as u32)
            | ((b >> self.b_loss as u32) << self.b_shift as u32)
            | ((a >> self.a_loss as u32) << self.a_shift as u32)
    }
}

// ============================================================================
// DukVid Header
// ============================================================================

/// Header size in bytes for DukVid .hdr files
pub const DUK_HEADER_SIZE: usize = 44; // 4 + 4 + 4 + 2 + 2 + 16 + 16 = 44 bytes with padding considerations

/// Header information from a DukVid .hdr file
///
/// Contains video dimensions and lookup tables for delta decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DukVideoHeader {
    /// Format version number
    pub version: u32,
    /// Screen offset as (x, y) in pixels
    pub screen_offset: (u32, u32),
    /// Dimensions in 4x4 blocks as (width_blocks, height_blocks)
    pub block_dimensions: (u16, u16),
    /// Luminance delta prototypes (8 values)
    pub lumas: [i16; 8],
    /// Chrominance delta prototypes (8 values)
    pub chromas: [i16; 8],
}

impl DukVideoHeader {
    /// Returns the actual video width in pixels
    ///
    /// Width = block_width * 4
    pub fn width(&self) -> u32 {
        self.block_dimensions.0 as u32 * 4
    }

    /// Returns the actual video height in pixels
    ///
    /// Height = block_height * 4
    pub fn height(&self) -> u32 {
        self.block_dimensions.1 as u32 * 4
    }

    /// Parses a DukVideoHeader from raw bytes (big-endian format)
    ///
    /// # Arguments
    ///
    /// * `data` - Raw header bytes (at least 44 bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(DukVideoHeader)` - Parsed header
    /// * `Err(VideoError::BadFile)` - If data is too short
    pub fn from_bytes(data: &[u8]) -> Result<Self, VideoError> {
        // Minimum size: 4 + 4 + 4 + 2 + 2 + 16 + 16 = 48 bytes
        // But C struct may have different packing; let's check for at least 44
        if data.len() < 44 {
            return Err(VideoError::BadFile(format!(
                "Header too short: {} bytes, expected at least 44",
                data.len()
            )));
        }

        // Parse fields as big-endian
        let version = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let scrn_x_ofs = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let scrn_y_ofs = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let wb = u16::from_be_bytes([data[12], data[13]]);
        let hb = u16::from_be_bytes([data[14], data[15]]);

        // Parse lumas (8 x i16 = 16 bytes, starting at offset 16)
        let mut lumas = [0i16; 8];
        for i in 0..8 {
            let offset = 16 + i * 2;
            lumas[i] = i16::from_be_bytes([data[offset], data[offset + 1]]);
        }

        // Parse chromas (8 x i16 = 16 bytes, starting at offset 32)
        let mut chromas = [0i16; 8];
        for i in 0..8 {
            let offset = 32 + i * 2;
            chromas[i] = i16::from_be_bytes([data[offset], data[offset + 1]]);
        }

        Ok(Self {
            version,
            screen_offset: (scrn_x_ofs, scrn_y_ofs),
            block_dimensions: (wb, hb),
            lumas,
            chromas,
        })
    }
}

// ============================================================================
// DukVid Deltas
// ============================================================================

/// Pre-computed delta values for DukVid decoding
///
/// These deltas are computed from the header's luma/chroma prototypes
/// and the vector table, used during frame decoding.
#[derive(Clone)]
pub struct DukVideoDeltas {
    /// Luminance deltas: [vector_index][item_index]
    pub lumas: [[i32; NUM_VEC_ITEMS]; NUM_VECTORS],
    /// Chrominance deltas: [vector_index][item_index]
    pub chromas: [[i32; NUM_VEC_ITEMS]; NUM_VECTORS],
}

impl Default for DukVideoDeltas {
    fn default() -> Self {
        Self::new()
    }
}

impl DukVideoDeltas {
    /// Creates a new zeroed delta table
    pub fn new() -> Self {
        Self {
            lumas: [[0; NUM_VEC_ITEMS]; NUM_VECTORS],
            chromas: [[0; NUM_VEC_ITEMS]; NUM_VECTORS],
        }
    }

    /// Creates deltas from luma/chroma prototypes and vector table
    ///
    /// # Arguments
    ///
    /// * `luma_protos` - 8 luminance prototype values from header
    /// * `chroma_protos` - 8 chrominance prototype values from header
    /// * `vectors` - Vector table (256 * 16 bytes from .tbl file)
    pub fn from_vectors(
        luma_protos: &[i32; 8],
        chroma_protos: &[i32; 8],
        vectors: &[u8],
    ) -> Result<Self, VideoError> {
        if vectors.len() < NUM_VECTORS * NUM_VEC_ITEMS {
            return Err(VideoError::BadFile(format!(
                "Vector table too short: {} bytes, expected {}",
                vectors.len(),
                NUM_VECTORS * NUM_VEC_ITEMS
            )));
        }

        let mut deltas = Self::new();

        for i in 0..NUM_VECTORS {
            let vector = &vectors[i * NUM_VEC_ITEMS..(i + 1) * NUM_VEC_ITEMS];
            Self::decode_vector(vector, luma_protos, false, &mut deltas.lumas[i]);
            Self::decode_vector(vector, chroma_protos, true, &mut deltas.chromas[i]);
        }

        Ok(deltas)
    }

    /// Decodes a single vector into delta/corrector pairs
    fn decode_vector(vec: &[u8], protos: &[i32; 8], is_chroma: bool, deltas: &mut [i32; 16]) {
        let citems = vec[0] as usize;

        let mut vec_idx = 1;
        let mut delta_idx = 0;

        let mut i = 0;
        while i < citems && delta_idx + 1 < 16 && vec_idx + 1 < vec.len() {
            let i1 = vec[vec_idx] as usize;
            let i2 = vec[vec_idx + 1] as usize;

            // Bounds check for prototype indices
            let i1 = i1.min(7);
            let i2 = i2.min(7);

            let mut d = Self::make_delta(protos, is_chroma, i1, i2);

            // Mark end of sequence
            if i == citems.saturating_sub(2) {
                d |= DUCK_END_OF_SEQUENCE;
            }

            deltas[delta_idx] = d;
            deltas[delta_idx + 1] = Self::make_corr(protos, is_chroma, i1, i2);

            vec_idx += 2;
            delta_idx += 2;
            i += 2;
        }
    }

    /// Creates a delta value from prototypes
    fn make_delta(protos: &[i32; 8], is_chroma: bool, i1: usize, i2: usize) -> i32 {
        if !is_chroma {
            // 0x421 is (r,g,b)=(1,1,1) in 15bit pixel coding
            let d1 = (protos[i1] >> 1) * 0x421;
            let d2 = (protos[i2] >> 1) * 0x421;
            ((d1 << 16) + d2) << 1
        } else {
            let d1 = (protos[i1] << 10) + protos[i2];
            ((d1 << 16) + d1) << 1
        }
    }

    /// Creates a corrector value from prototypes
    fn make_corr(protos: &[i32; 8], is_chroma: bool, i1: usize, i2: usize) -> i32 {
        if !is_chroma {
            let d1 = (protos[i1] & 1) << 15;
            let d2 = (protos[i2] & 1) << 15;
            (d1 << 16) + d2
        } else {
            ((i1 as i32) << 3) + i2 as i32
        }
    }
}

impl fmt::Debug for DukVideoDeltas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DukVideoDeltas")
            .field("lumas", &format!("[{}x{} array]", NUM_VECTORS, NUM_VEC_ITEMS))
            .field("chromas", &format!("[{}x{} array]", NUM_VECTORS, NUM_VEC_ITEMS))
            .finish()
    }
}

// ============================================================================
// Video Frame
// ============================================================================

/// A single decoded video frame
#[derive(Debug, Clone)]
pub struct VideoFrame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Decoded pixel data (RGBA format, row-major order)
    pub data: Vec<u32>,
    /// Frame timestamp in seconds
    pub timestamp: f32,
}

impl VideoFrame {
    /// Creates a new video frame with the given dimensions
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `timestamp` - Frame timestamp in seconds
    pub fn new(width: u32, height: u32, timestamp: f32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            data: vec![0; pixel_count],
            timestamp,
        }
    }

    /// Creates a video frame from existing pixel data
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `data` - Pixel data (must have width * height elements)
    /// * `timestamp` - Frame timestamp in seconds
    ///
    /// # Returns
    ///
    /// * `Ok(VideoFrame)` - Frame created successfully
    /// * `Err(VideoError::BadArg)` - If data size doesn't match dimensions
    pub fn from_data(
        width: u32,
        height: u32,
        data: Vec<u32>,
        timestamp: f32,
    ) -> Result<Self, VideoError> {
        let expected = (width * height) as usize;
        if data.len() != expected {
            return Err(VideoError::BadArg(format!(
                "Data size {} doesn't match dimensions {}x{} = {}",
                data.len(),
                width,
                height,
                expected
            )));
        }
        Ok(Self {
            width,
            height,
            data,
            timestamp,
        })
    }

    /// Returns the pixel count
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Gets a pixel at the specified coordinates
    ///
    /// Returns `None` if coordinates are out of bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<u32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = (y * self.width + x) as usize;
        self.data.get(idx).copied()
    }

    /// Sets a pixel at the specified coordinates
    ///
    /// Returns `false` if coordinates are out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, value: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = (y * self.width + x) as usize;
        if let Some(pixel) = self.data.get_mut(idx) {
            *pixel = value;
            true
        } else {
            false
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_error_display() {
        assert_eq!(
            VideoError::BadFile("test.duk".into()).to_string(),
            "Bad file: test.duk"
        );
        assert_eq!(VideoError::Eof.to_string(), "End of file");
        assert_eq!(VideoError::OutOfBuffer.to_string(), "Buffer overflow");
        assert_eq!(
            VideoError::NotInitialized.to_string(),
            "Video not initialized"
        );
        assert_eq!(
            VideoError::IoError("read failed".into()).to_string(),
            "I/O error: read failed"
        );
        assert_eq!(
            VideoError::BadArg("invalid size".into()).to_string(),
            "Bad argument: invalid size"
        );
    }

    #[test]
    fn test_pixel_format_default() {
        let fmt = PixelFormat::default();
        assert_eq!(fmt.bytes_per_pixel, 4);
        assert_eq!(fmt.r_shift, 0);
        assert_eq!(fmt.g_shift, 8);
        assert_eq!(fmt.b_shift, 16);
        assert_eq!(fmt.a_shift, 24);
        assert_eq!(fmt.r_loss, 0);
        assert_eq!(fmt.g_loss, 0);
        assert_eq!(fmt.b_loss, 0);
        assert_eq!(fmt.a_loss, 0);
    }

    #[test]
    fn test_pixel_format_rgb565() {
        let fmt = PixelFormat::rgb565();
        assert_eq!(fmt.bytes_per_pixel, 2);
        assert_eq!(fmt.r_shift, 11);
        assert_eq!(fmt.g_shift, 5);
        assert_eq!(fmt.b_shift, 0);
        assert_eq!(fmt.r_loss, 3);
        assert_eq!(fmt.g_loss, 2);
        assert_eq!(fmt.b_loss, 3);
    }

    #[test]
    fn test_duk_header_from_bytes() {
        // Create a minimal header (48 bytes to be safe)
        let mut data = vec![0u8; 48];

        // Version = 3 (big-endian)
        data[0..4].copy_from_slice(&3u32.to_be_bytes());
        // Screen X offset = 10
        data[4..8].copy_from_slice(&10u32.to_be_bytes());
        // Screen Y offset = 20
        data[8..12].copy_from_slice(&20u32.to_be_bytes());
        // Width in blocks = 40
        data[12..14].copy_from_slice(&40u16.to_be_bytes());
        // Height in blocks = 30
        data[14..16].copy_from_slice(&30u16.to_be_bytes());
        // Lumas[0] = 5
        data[16..18].copy_from_slice(&5i16.to_be_bytes());
        // Chromas[0] = -3
        data[32..34].copy_from_slice(&(-3i16).to_be_bytes());

        let header = DukVideoHeader::from_bytes(&data).unwrap();

        assert_eq!(header.version, 3);
        assert_eq!(header.screen_offset, (10, 20));
        assert_eq!(header.block_dimensions, (40, 30));
        assert_eq!(header.lumas[0], 5);
        assert_eq!(header.chromas[0], -3);
    }

    #[test]
    fn test_duk_header_from_bytes_too_short() {
        let data = vec![0u8; 10]; // Too short
        let result = DukVideoHeader::from_bytes(&data);
        assert!(matches!(result, Err(VideoError::BadFile(_))));
    }

    #[test]
    fn test_duk_header_dimensions() {
        let header = DukVideoHeader {
            version: 2,
            screen_offset: (0, 0),
            block_dimensions: (40, 30), // 40x30 blocks
            lumas: [0; 8],
            chromas: [0; 8],
        };

        // Width = 40 * 4 = 160 pixels
        assert_eq!(header.width(), 160);
        // Height = 30 * 4 = 120 pixels
        assert_eq!(header.height(), 120);
    }

    #[test]
    fn test_video_frame_new() {
        let frame = VideoFrame::new(320, 240, 1.5);

        assert_eq!(frame.width, 320);
        assert_eq!(frame.height, 240);
        assert_eq!(frame.timestamp, 1.5);
        assert_eq!(frame.data.len(), 320 * 240);
        assert_eq!(frame.pixel_count(), 76800);
    }

    #[test]
    fn test_video_frame_from_data() {
        let data = vec![0xFFu32; 100];
        let frame = VideoFrame::from_data(10, 10, data, 0.0).unwrap();
        assert_eq!(frame.width, 10);
        assert_eq!(frame.height, 10);
        assert_eq!(frame.data[0], 0xFF);
    }

    #[test]
    fn test_video_frame_from_data_wrong_size() {
        let data = vec![0u32; 50]; // Wrong size for 10x10
        let result = VideoFrame::from_data(10, 10, data, 0.0);
        assert!(matches!(result, Err(VideoError::BadArg(_))));
    }

    #[test]
    fn test_video_frame_pixel_access() {
        let mut frame = VideoFrame::new(10, 10, 0.0);

        // Set pixel
        assert!(frame.set_pixel(5, 5, 0xABCDEF));
        assert_eq!(frame.get_pixel(5, 5), Some(0xABCDEF));

        // Out of bounds
        assert_eq!(frame.get_pixel(10, 10), None);
        assert!(!frame.set_pixel(10, 10, 0));
    }

    #[test]
    fn test_constants() {
        assert!((DUCK_FPS - 14.622).abs() < 0.001);
        assert_eq!(DUCK_MAX_FRAME_SIZE, 0x8000);
        assert_eq!(NUM_VEC_ITEMS, 16);
        assert_eq!(NUM_VECTORS, 256);
        assert_eq!(DUCK_END_OF_SEQUENCE, 1);
    }

    #[test]
    fn test_duk_video_deltas_new() {
        let deltas = DukVideoDeltas::new();
        assert_eq!(deltas.lumas.len(), NUM_VECTORS);
        assert_eq!(deltas.chromas.len(), NUM_VECTORS);
        assert_eq!(deltas.lumas[0].len(), NUM_VEC_ITEMS);
        assert_eq!(deltas.chromas[0].len(), NUM_VEC_ITEMS);
        // Should be zeroed
        assert_eq!(deltas.lumas[0][0], 0);
    }

    #[test]
    fn test_duk_video_deltas_debug() {
        let deltas = DukVideoDeltas::new();
        let debug_str = format!("{:?}", deltas);
        assert!(debug_str.contains("DukVideoDeltas"));
        assert!(debug_str.contains("256x16"));
    }

    #[test]
    fn test_pixel_format_convert_from_duk() {
        let fmt = PixelFormat::default();
        // Test with a known 15-bit value
        // 15-bit format: XRRR RRGG GGGB BBBB
        // White (all 1s in 5-bit channels): 0x7FFF
        let white_15bit: u16 = 0x7FFF;
        let converted = fmt.convert_from_duk(white_15bit);
        // Should produce high values in R, G, B
        assert!(converted > 0);
    }

    #[test]
    fn test_video_error_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let video_err: VideoError = io_err.into();
        assert!(matches!(video_err, VideoError::IoError(_)));
    }
}
