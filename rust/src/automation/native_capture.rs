//! Shared native OS-image normalization and linked-playable pixel predicates.

use super::native_window::NativeWindowBounds;

pub(crate) fn normalize_native_capture(
    bytes: &[u8],
    os_bounds: NativeWindowBounds,
    client_bounds: NativeWindowBounds,
    byte_budget: u64,
) -> Result<Vec<u8>, String> {
    use image::GenericImageView as _;

    if !os_bounds.contains(client_bounds) {
        return Err("native capture bounds do not contain the client bounds".to_string());
    }
    let image = super::native_window::decode_png_bounded(bytes, byte_budget)
        .map_err(|error| format!("decode native screenshot: {error}"))?;
    let (width, height) = image.dimensions();
    if os_bounds.width == 0
        || os_bounds.height == 0
        || width % os_bounds.width != 0
        || height % os_bounds.height != 0
    {
        return Err("native screenshot dimensions do not match the observed OS bounds".to_string());
    }
    let scale = width / os_bounds.width;
    if scale == 0 || height / os_bounds.height != scale {
        return Err("native screenshot pixel density is inconsistent".to_string());
    }
    let relative_x = u32::try_from(i64::from(client_bounds.x) - i64::from(os_bounds.x))
        .map_err(|_| "native screenshot client x offset is negative".to_string())?;
    let relative_y = u32::try_from(i64::from(client_bounds.y) - i64::from(os_bounds.y))
        .map_err(|_| "native screenshot client y offset is negative".to_string())?;
    let crop_x = relative_x
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop x overflowed".to_string())?;
    let crop_y = relative_y
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop y overflowed".to_string())?;
    let crop_width = client_bounds
        .width
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop width overflowed".to_string())?;
    let crop_height = client_bounds
        .height
        .checked_mul(scale)
        .ok_or_else(|| "native screenshot crop height overflowed".to_string())?;
    if crop_x
        .checked_add(crop_width)
        .is_none_or(|right| right > width)
        || crop_y
            .checked_add(crop_height)
            .is_none_or(|bottom| bottom > height)
    {
        return Err("native screenshot crop exceeds captured pixels".to_string());
    }
    let cropped = image.crop_imm(crop_x, crop_y, crop_width, crop_height);
    let cropped = convert_capture_to_srgb(bytes, cropped, byte_budget)?;
    let normalized = if scale == 1 {
        cropped
    } else {
        cropped.resize_exact(
            client_bounds.width,
            client_bounds.height,
            image::imageops::FilterType::Lanczos3,
        )
    };
    let mut encoded = std::io::Cursor::new(Vec::new());
    normalized
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|error| format!("encode normalized native screenshot: {error}"))?;
    let encoded = encoded.into_inner();
    if encoded.is_empty() || encoded.len() as u64 > byte_budget {
        return Err("normalized native screenshot exceeds its byte budget".to_string());
    }
    Ok(encoded)
}

fn convert_capture_to_srgb(
    bytes: &[u8],
    image: image::DynamicImage,
    byte_budget: u64,
) -> Result<image::DynamicImage, String> {
    use image::ImageDecoder as _;
    use moxcms::{ColorProfile, Layout, TransformOptions};

    let mut limits = image::Limits::default();
    limits.max_alloc = Some(byte_budget);
    let mut decoder =
        image::codecs::png::PngDecoder::with_limits(std::io::Cursor::new(bytes), limits)
            .map_err(|error| format!("inspect native screenshot profile: {error}"))?;
    let Some(profile) = decoder
        .icc_profile()
        .map_err(|error| format!("read native screenshot profile: {error}"))?
    else {
        return Ok(image);
    };
    // macOS screenshots carry the display profile; renderer readback is sRGB.
    // Convert before resampling and stripping metadata from the client crop.
    let source = ColorProfile::new_from_slice(&profile)
        .map_err(|error| format!("parse native screenshot profile: {error}"))?;
    let transform = source
        .create_transform_8bit(
            Layout::Rgba,
            &ColorProfile::new_srgb(),
            Layout::Rgba,
            TransformOptions::default(),
        )
        .map_err(|error| format!("create native screenshot color transform: {error}"))?;
    let source = image.to_rgba8();
    let mut destination = image::RgbaImage::new(source.width(), source.height());
    transform
        .transform(source.as_raw(), destination.as_mut())
        .map_err(|error| format!("convert native screenshot to sRGB: {error}"))?;
    Ok(image::DynamicImage::ImageRgba8(destination))
}

pub(crate) fn validate_playable_screenshot_difference(
    stable: &[u8],
    playable: &[u8],
) -> Result<(), String> {
    let stable = image::load_from_memory(stable)
        .map_err(|error| format!("decode stable native screenshot: {error}"))?
        .to_rgba8();
    let playable = image::load_from_memory(playable)
        .map_err(|error| format!("decode playable native screenshot: {error}"))?
        .to_rgba8();
    if stable.dimensions() != playable.dimensions() {
        return Err("stable and playable native screenshots have different dimensions".to_string());
    }
    let pixel_count = u64::from(stable.width()) * u64::from(stable.height());
    if pixel_count == 0 {
        return Err("native screenshots contain no pixels".to_string());
    }
    let materially_changed = stable
        .pixels()
        .zip(playable.pixels())
        .filter(|(before, after)| {
            before.0[..3]
                .iter()
                .zip(&after.0[..3])
                .any(|(left, right)| left.abs_diff(*right) >= 24)
        })
        .count() as u64;
    let minimum_changed = pixel_count.div_ceil(100);
    if materially_changed < minimum_changed {
        return Err(format!(
            "playable screenshot differs materially at only {materially_changed}/{pixel_count} pixels; expected at least {minimum_changed}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder as _;

    fn tagged_png(profile: Vec<u8>) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = image::codecs::png::PngEncoder::new(&mut bytes);
        encoder.set_icc_profile(profile).unwrap();
        encoder
            .write_image(
                &[97, 211, 62, 255].repeat(16),
                4,
                4,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn display_p3_capture_is_converted_to_renderer_srgb_before_resampling() {
        let bytes =
            tagged_png(include_bytes!("../../tests/fixtures/native/display-p3.icc").to_vec());
        let bounds = NativeWindowBounds {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };
        let normalized = normalize_native_capture(&bytes, bounds, bounds, 65536).unwrap();
        let pixels = image::load_from_memory(&normalized).unwrap().to_rgba8();
        assert_eq!(pixels.dimensions(), (2, 2));
        // The pmid-59 OS green (97,211,62) represents renderer sRGB (0,214,0).
        for pixel in pixels.pixels() {
            for (actual, expected) in pixel.0.into_iter().zip([0u8, 214, 0, 255]) {
                assert!(actual.abs_diff(expected) <= 2, "{pixel:?}");
            }
        }
    }

    #[test]
    fn malformed_embedded_profile_is_rejected() {
        let bytes = tagged_png(b"not an ICC profile".to_vec());
        let bounds = NativeWindowBounds {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        assert!(normalize_native_capture(&bytes, bounds, bounds, 65536).is_err());
    }
}
