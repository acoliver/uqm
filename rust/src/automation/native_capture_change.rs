//! Script-declared scene change checks on decoded OS client pixels.

/// PNG encoding, metadata and alpha alone cannot satisfy a visible scene change.
pub(super) fn validate_change(previous: &[u8], current: &[u8], limit: u64) -> Result<(), String> {
    let previous = super::native_window::decode_png_bounded(previous, limit)
        .map_err(|error| format!("decode previous checkpoint: {error}"))?
        .to_rgba8();
    let current = super::native_window::decode_png_bounded(current, limit)
        .map_err(|error| format!("decode current checkpoint: {error}"))?
        .to_rgba8();
    if previous.dimensions() != current.dimensions() {
        return Err("script capture dimensions changed".into());
    }
    if !previous
        .pixels()
        .zip(current.pixels())
        .any(|(left, right)| left.0[..3] != right.0[..3])
    {
        return Err("script expect_change has no changed OS client RGB pixel".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alternate_encoding_and_alpha_do_not_fake_a_visible_change() {
        let rgb = image::RgbImage::from_pixel(8, 8, image::Rgb([12, 34, 56]));
        let rgba = image::RgbaImage::from_pixel(8, 8, image::Rgba([12, 34, 56, 127]));
        let mut first = std::io::Cursor::new(Vec::new());
        let mut second = std::io::Cursor::new(Vec::new());
        rgb.write_to(&mut first, image::ImageFormat::Png).unwrap();
        rgba.write_to(&mut second, image::ImageFormat::Png).unwrap();
        assert_ne!(first.get_ref(), second.get_ref());
        assert!(validate_change(first.get_ref(), second.get_ref(), 65536).is_err());
        let mut changed = rgba;
        changed.put_pixel(1, 1, image::Rgba([13, 34, 56, 127]));
        let mut third = std::io::Cursor::new(Vec::new());
        changed
            .write_to(&mut third, image::ImageFormat::Png)
            .unwrap();
        assert!(validate_change(first.get_ref(), third.get_ref(), 65536).is_ok());
    }
}
