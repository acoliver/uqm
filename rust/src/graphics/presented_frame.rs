use image::ImageEncoder;

/// Immutable RGBA readback of the exact renderer output immediately before
/// the backend presents it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentedFrame {
    pub count: u64,
    pub generation: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl PresentedFrame {
    /// Validate dimensions and packed RGBA byte length.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.count == 0 || self.width == 0 || self.height == 0 {
            return Err("presented frame metadata contains zero");
        }
        let expected = u64::from(self.width)
            .checked_mul(u64::from(self.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or("presented frame dimensions overflow")?;
        if self.rgba.len() != expected {
            return Err("presented frame byte length does not match dimensions");
        }
        Ok(())
    }

    /// Encode the exact immutable presented-frame bytes as RGBA PNG.
    pub fn encode_png(&self) -> Result<Vec<u8>, String> {
        self.validate().map_err(str::to_owned)?;
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(
                &self.rgba,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|error| format!("PNG encoding failed: {error}"))?;
        Ok(png)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> PresentedFrame {
        PresentedFrame {
            count: 1,
            generation: 9,
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 128],
        }
    }

    #[test]
    fn accepts_packed_rgba_and_preserves_alpha_in_png() {
        let frame = frame();
        assert_eq!(frame.validate(), Ok(()));
        let png = frame.encode_png().unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.as_raw(), &frame.rgba);
    }

    #[test]
    fn rejects_zero_metadata_and_wrong_byte_lengths() {
        for invalid in [
            PresentedFrame {
                count: 0,
                ..frame()
            },
            PresentedFrame {
                width: 0,
                ..frame()
            },
            PresentedFrame {
                height: 0,
                ..frame()
            },
            PresentedFrame {
                rgba: vec![0; 7],
                ..frame()
            },
        ] {
            assert!(invalid.validate().is_err());
            assert!(invalid.encode_png().is_err());
        }
    }

    #[test]
    fn rejects_dimensions_whose_byte_size_overflows() {
        let frame = PresentedFrame {
            width: u32::MAX,
            height: u32::MAX,
            ..frame()
        };
        assert_eq!(frame.validate(), Err("presented frame dimensions overflow"));
    }
}
