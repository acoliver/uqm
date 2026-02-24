// resample.rs - Audio resampling algorithms

//! Audio resampling algorithms for the mixer.
//!
//! Provides different quality levels of resampling:
//! - None: No resampling (same frequency)
//! - Nearest: Nearest neighbor resampling
//! - Linear: Linear interpolation (medium quality)
//! - Cubic: Cubic interpolation (high quality)

use crate::sound::mixer::buffer::MixerBuffer;
use crate::sound::mixer::source::MixerSource;
use crate::sound::mixer::types::*;

/// Resample without conversion (same frequency)
///
/// This is used when the source and mixer have the same frequency.
pub fn resample_none(
    _source: &MixerSource,
    buffer: &MixerBuffer,
    pos: u32,
    chansize: u32,
    _left: bool,
) -> (f32, u32) {
    let data = buffer.data.as_ref().unwrap();
    let d0 = &data[pos as usize..];

    let sample = if chansize == 2 {
        let s = i16::from_le_bytes([d0[0], d0[1]]);
        s as f32
    } else {
        let s = d0[0] as i8;
        s as f32
    };

    (sample, pos + chansize)
}

/// Resample using nearest neighbor
///
/// Simplest resampling method - just picks the closest sample.
pub fn resample_nearest(
    _source: &MixerSource,
    buffer: &MixerBuffer,
    pos: u32,
    chansize: u32,
    _left: bool,
) -> (f32, u32) {
    let data = buffer.data.as_ref().unwrap();
    let d0 = &data[pos as usize..];

    let sample = if chansize == 2 {
        let s = i16::from_le_bytes([d0[0], d0[1]]);
        s as f32
    } else {
        let s = d0[0] as i8;
        s as f32
    };

    // Move forward by the resampling step
    let new_pos = pos + buffer.high;

    (sample, new_pos)
}

/// Resample using linear interpolation
///
/// Better quality than nearest neighbor. Interpolates between adjacent samples.
pub fn resample_linear(
    source: &MixerSource,
    buffer: &MixerBuffer,
    pos: u32,
    chansize: u32,
    _left: bool,
) -> (f32, u32) {
    let data = buffer.data.as_ref().unwrap();

    // Get current sample
    let d0 = &data[pos as usize..];
    let s0 = if chansize == 2 {
        i16::from_le_bytes([d0[0], d0[1]]) as f32
    } else {
        d0[0] as i8 as f32
    };

    // Get next sample for interpolation
    let next_pos = pos + chansize;
    let s1 = if next_pos + chansize <= buffer.size {
        let d1 = &data[next_pos as usize..];
        if chansize == 2 {
            i16::from_le_bytes([d1[0], d1[1]]) as f32
        } else {
            d1[0] as i8 as f32
        }
    } else {
        s0 // At end, use current sample
    };

    // Interpolate
    let t = source.count as f32 / 65536.0;
    let sample = s0 + t * (s1 - s0);

    // Move forward by the resampling step
    let new_pos = pos + buffer.high;

    (sample, new_pos)
}

/// Resample using cubic interpolation
///
/// Highest quality method. Uses a cubic spline to interpolate between samples.
pub fn resample_cubic(
    source: &MixerSource,
    buffer: &MixerBuffer,
    pos: u32,
    chansize: u32,
    _left: bool,
) -> (f32, u32) {
    let data = buffer.data.as_ref().unwrap();

    // Get four samples for cubic interpolation
    // s0 = previous, s1 = current, s2 = next, s3 = next+1
    let s0 = if pos >= chansize {
        let d0 = &data[(pos - chansize) as usize..];
        if chansize == 2 {
            i16::from_le_bytes([d0[0], d0[1]]) as f32
        } else {
            d0[0] as i8 as f32
        }
    } else {
        // At beginning, duplicate current sample
        let d0 = &data[pos as usize..];
        if chansize == 2 {
            i16::from_le_bytes([d0[0], d0[1]]) as f32
        } else {
            d0[0] as i8 as f32
        }
    };

    let s1 = {
        let d1 = &data[pos as usize..];
        if chansize == 2 {
            i16::from_le_bytes([d1[0], d1[1]]) as f32
        } else {
            d1[0] as i8 as f32
        }
    };

    let s2 = if pos + chansize <= buffer.size - chansize {
        let d2 = &data[(pos + chansize) as usize..];
        if chansize == 2 {
            i16::from_le_bytes([d2[0], d2[1]]) as f32
        } else {
            d2[0] as i8 as f32
        }
    } else {
        s1
    };

    let s3 = if pos + 2 * chansize <= buffer.size - chansize {
        let d3 = &data[(pos + 2 * chansize) as usize..];
        if chansize == 2 {
            i16::from_le_bytes([d3[0], d3[1]]) as f32
        } else {
            d3[0] as i8 as f32
        }
    } else {
        s2
    };

    // Cubic interpolation
    let t = source.count as f32 / 65536.0;
    let t2 = t * t;

    let a = (3.0 * (s1 - s2) - s0 + s3) * 0.5;
    let b = 2.0 * s2 + s0 - ((5.0 * s1 + s3) * 0.5);
    let c = (s2 - s0) * 0.5;

    let sample = a * t2 * t + b * t2 + c * t + s1;

    // Move forward by the resampling step
    let new_pos = pos + buffer.high;

    (sample, new_pos)
}

/// Get a sample from a buffer in internal format
///
/// Handles both 8-bit and 16-bit formats.
pub fn get_sample_int(data: &[u8], pos: usize, chansize: u32) -> f32 {
    if chansize == 2 {
        let s = i16::from_le_bytes([data[pos], data[pos + 1]]);
        s as f32
    } else {
        data[pos] as i8 as f32
    }
}

/// Put a sample to a buffer in internal format
///
/// Handles both 8-bit and 16-bit formats.
pub fn put_sample_int(data: &mut [u8], pos: usize, chansize: u32, sample: i32) {
    if chansize == 2 {
        let s = sample as i16;
        data[pos] = (s & 0xFF) as u8;
        data[pos + 1] = ((s >> 8) & 0xFF) as u8;
    } else {
        data[pos] = (sample as i8) as u8;
    }
}

/// Get a sample from external buffer (unsigned 8-bit)
pub fn get_sample_ext(data: &[u8], pos: usize, bpc: u32) -> i32 {
    if bpc == 2 {
        i16::from_le_bytes([data[pos], data[pos + 1]]) as i32
    } else {
        (data[pos] as i32) - 128
    }
}

/// Put a sample to external buffer (unsigned 8-bit)
pub fn put_sample_ext(data: &mut [u8], pos: usize, bpc: u32, sample: i32) {
    if bpc == 2 {
        let s = sample as i16;
        data[pos] = (s & 0xFF) as u8;
        data[pos + 1] = ((s >> 8) & 0xFF) as u8;
    } else {
        data[pos] = (sample ^ 0x80) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_none_mono8() {
        let source = MixerSource::new();
        let mut buffer = MixerBuffer::new();

        // Create test data: silent samples
        buffer.data = Some(vec![128u8 ^ 0x80; 10]); // Converted to signed
        buffer.size = 10;
        buffer.high = 1;
        buffer.low = 0;

        let (sample, new_pos) = resample_none(&source, &buffer, 0, 1, true);
        assert_eq!(sample, 0.0);
        assert_eq!(new_pos, 1);
    }

    #[test]
    fn test_resample_none_mono16() {
        let source = MixerSource::new();
        let mut buffer = MixerBuffer::new();

        // Create test data: value 100
        let data: [u8; 2] = [100, 0]; // i16::to_le(100)
        buffer.data = Some(data.to_vec());
        buffer.size = 2;
        buffer.high = 2;
        buffer.low = 0;

        let (sample, new_pos) = resample_none(&source, &buffer, 0, 2, true);
        assert_eq!(sample, 100.0);
        assert_eq!(new_pos, 2);
    }

    #[test]
    fn test_resample_nearest() {
        let source = MixerSource::new();
        let mut buffer = MixerBuffer::new();

        buffer.data = Some(vec![10, 20, 30, 40, 50]);
        buffer.size = 5;
        buffer.high = 2; // Skip one sample
        buffer.low = 0;

        let (sample, new_pos) = resample_nearest(&source, &buffer, 0, 1, true);
        assert_eq!(sample, 10.0);
        assert_eq!(new_pos, 2);
    }

    #[test]
    fn test_resample_linear() {
        let mut source = MixerSource::new();
        source.count = 32768; // t = 0.5

        let mut buffer = MixerBuffer::new();
        buffer.data = Some(vec![0, 10, 20, 30, 40]);
        buffer.size = 5;
        buffer.high = 1;
        buffer.low = 0;

        let (sample, new_pos) = resample_linear(&source, &buffer, 0, 1, true);
        // Linear interpolation: s0 + t * (s1 - s0) = 0 + 0.5 * 10 = 5.0
        assert!((sample - 5.0).abs() < 0.01);
        assert_eq!(new_pos, 1);
    }

    #[test]
    fn test_resample_cubic() {
        let mut source = MixerSource::new();
        source.count = 16384; // t = 0.25

        let mut buffer = MixerBuffer::new();
        buffer.data = Some(vec![0, 10, 20, 30, 40, 50]);
        buffer.size = 6;
        buffer.high = 1;
        buffer.low = 0;

        let (sample, _new_pos) = resample_cubic(&source, &buffer, 1, 1, true);
        // Cubic interpolation should give a value between s1 (10) and s2 (20)
        assert!(sample >= 10.0 && sample <= 20.0);
    }

    #[test]
    fn test_get_sample_int_mono8() {
        let data = vec![10i8 as u8, 20, 30];
        let sample = get_sample_int(&data, 0, 1);
        assert_eq!(sample, 10.0);

        let sample = get_sample_int(&data, 1, 1);
        assert_eq!(sample, 20.0);
    }

    #[test]
    fn test_get_sample_int_mono16() {
        let data: Vec<u8> = vec![100u8, 0, 200, 0]; // i16::to_le(100) and i16::to_le(200)
        let sample = get_sample_int(&data, 0, 2);
        assert_eq!(sample, 100.0);

        let sample = get_sample_int(&data, 2, 2);
        assert_eq!(sample, 200.0);
    }

    #[test]
    fn test_put_sample_int_mono8() {
        let mut data = vec![0u8; 3];
        put_sample_int(&mut data, 0, 1, 10);
        put_sample_int(&mut data, 1, 1, -20);
        put_sample_int(&mut data, 2, 1, 30);

        assert_eq!(data[0] as i8, 10);
        assert_eq!(data[1] as i8, -20);
        assert_eq!(data[2] as i8, 30);
    }

    #[test]
    fn test_put_sample_int_mono16() {
        let mut data = vec![0u8; 6];
        put_sample_int(&mut data, 0, 2, 100);
        put_sample_int(&mut data, 2, 2, -200);
        put_sample_int(&mut data, 4, 2, 300);

        assert_eq!(i16::from_le_bytes([data[0], data[1]]), 100);
        assert_eq!(i16::from_le_bytes([data[2], data[3]]), -200);
        assert_eq!(i16::from_le_bytes([data[4], data[5]]), 300);
    }

    #[test]
    fn test_get_sample_ext_mono8() {
        let data = vec![128u8, 138, 118]; // 0, 10, -10 in unsigned
        let sample = get_sample_ext(&data, 0, 1);
        assert_eq!(sample, 0);

        let sample = get_sample_ext(&data, 1, 1);
        assert_eq!(sample, 10);

        let sample = get_sample_ext(&data, 2, 1);
        assert_eq!(sample, -10);
    }

    #[test]
    fn test_put_sample_ext_mono8() {
        let mut data = vec![0u8; 3];
        put_sample_ext(&mut data, 0, 1, 0);
        put_sample_ext(&mut data, 1, 1, 10);
        put_sample_ext(&mut data, 2, 1, -10);

        assert_eq!(data[0], 128);
        assert_eq!(data[1], 138);
        assert_eq!(data[2], 118);
    }

    #[test]
    fn test_resample_at_end() {
        let source = MixerSource::new();
        let mut buffer = MixerBuffer::new();

        buffer.data = Some(vec![10, 20, 30]);
        buffer.size = 3;
        buffer.high = 1;
        buffer.low = 0;

        // At end of buffer, should handle gracefully
        let (sample, _new_pos) = resample_none(&source, &buffer, 2, 1, true);
        assert_eq!(sample, 30.0);
    }
}
