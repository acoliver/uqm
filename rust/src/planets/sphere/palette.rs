//! Owned surface palette bytes, independent of solar records and native resource handles.

use crate::planets::render::topography::{TopographyError, TopographyPalette};

pub struct SurfacePalette {
    translation: [u8; 256],
    colors: Vec<u8>,
    planet_type: u8,
    base_elevation: i16,
}

impl SurfacePalette {
    pub fn from_bytes(
        index: u8,
        translation: &[u8],
        colors: Vec<u8>,
    ) -> Result<Self, TopographyError> {
        let planet = crate::planets::tables::PLANET_ARRAY
            .get(usize::from(index))
            .ok_or(TopographyError::PlanetIndex)?;
        let bytes = translation
            .get(6..262)
            .ok_or(TopographyError::TranslationLength)?;
        let mut translation = [0; 256];
        translation.copy_from_slice(bytes);
        Ok(Self {
            translation,
            colors,
            planet_type: planet.type_,
            base_elevation: planet.base_elevation,
        })
    }

    pub fn view(&self) -> TopographyPalette<'_> {
        TopographyPalette {
            translation: &self.translation,
            colors: &self.colors,
            planet_type: self.planet_type,
            base_elevation: self.base_elevation,
        }
    }
}

pub fn temperature_index(temperature: i16) -> u16 {
    if temperature > 100 {
        2
    } else if temperature > -40 {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planets::render::topography::TopographyError;

    #[test]
    fn palette_owns_translation_and_color_bytes_for_both_render_passes() {
        let mut translation = vec![99; 6];
        translation.extend(0..=255);
        let colors = vec![0, 0, 61, 62, 63];
        let palette = SurfacePalette::from_bytes(0, &translation, colors.clone()).unwrap();
        translation.fill(17);
        let view = palette.view();
        assert_eq!(*view.translation, std::array::from_fn(|index| index as u8));
        assert_eq!(view.colors, colors);
        assert_eq!(
            view.planet_type,
            crate::planets::tables::PLANET_ARRAY[0].type_
        );
        assert_eq!(
            view.base_elevation,
            crate::planets::tables::PLANET_ARRAY[0].base_elevation
        );
    }

    #[test]
    fn palette_validates_planet_then_translation_and_ignores_trailing_bytes() {
        assert!(matches!(
            SurfacePalette::from_bytes(127, &[], vec![]),
            Err(TopographyError::PlanetIndex)
        ));
        assert!(matches!(
            SurfacePalette::from_bytes(0, &[0; 261], vec![]),
            Err(TopographyError::TranslationLength)
        ));
        assert!(SurfacePalette::from_bytes(0, &[0; 263], vec![]).is_ok());
    }

    #[test]
    fn temperature_variants_keep_original_strict_boundaries() {
        for (temperature, expected) in [(-100, 0), (-40, 0), (-39, 1), (100, 1), (101, 2)] {
            assert_eq!(temperature_index(temperature), expected);
        }
    }
}
