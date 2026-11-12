//! Campaign-lifetime interplanetary art, palettes and music.

use crate::{
    graphics::drawable_abi::LoadedDrawable,
    sound::{music::MusicRequest, resource_types::LoadedMusic},
    strings::sresins::load_binary_entries,
};
use std::ffi::CStr;

#[derive(Default)]
pub struct SolarPresentation {
    pub assets: Option<SolarResources>,
    pub frame_deadline: u32,
}

pub struct SolarResources {
    // Field order preserves the legacy teardown order.
    pub flagship: LoadedDrawable,
    pub sun: LoadedDrawable,
    pub sun_palettes: Vec<Vec<u8>>,
    pub orbital_palettes: Vec<Vec<u8>>,
    pub orbital: LoadedDrawable,
    pub junk: LoadedDrawable,
    pub music: Option<LoadedMusic>,
}

impl SolarResources {
    pub fn load() -> Result<Self, &'static str> {
        Self::load_with(LoadedDrawable::load, load_binary_entries, LoadedMusic::load)
    }

    pub fn retain(slot: &mut Option<Self>) -> &Self {
        Self::retain_with(slot, Self::load)
    }

    fn retain_with(
        slot: &mut Option<Self>,
        load: impl FnOnce() -> Result<Self, &'static str>,
    ) -> &Self {
        slot.get_or_insert_with(|| {
            load().unwrap_or_else(|key| panic!("missing interplanetary resource {key}"))
        })
    }

    fn load_with(
        mut graphic: impl FnMut(&CStr) -> Option<LoadedDrawable>,
        mut palettes: impl FnMut(&CStr) -> Option<Vec<Vec<u8>>>,
        mut music: impl FnMut(&CStr) -> Option<LoadedMusic>,
    ) -> Result<Self, &'static str> {
        let junk = graphic(c"graphics.orbitbackground").ok_or("graphics.orbitbackground")?;
        let flagship = graphic(c"graphics.flagship").ok_or("graphics.flagship")?;
        let orbital_palettes = palettes(c"colortable.orbplan").ok_or("colortable.orbplan")?;
        let orbital = graphic(c"graphics.planets").ok_or("graphics.planets")?;
        let sun_palettes = palettes(c"colortable.truespace").ok_or("colortable.truespace")?;
        let sun = graphic(c"graphics.truespacesun").ok_or("graphics.truespacesun")?;
        let music = music(c"music.space");
        Ok(Self {
            flagship,
            sun,
            sun_palettes,
            orbital_palettes,
            orbital,
            junk,
            music,
        })
    }

    pub fn music_request(&self) -> MusicRequest {
        self.music.as_ref().map_or(MusicRequest::Absent, |music| {
            MusicRequest::Track(music.playback())
        })
    }

    pub fn set_sun_palette(&self, index: usize) {
        install_palette(&self.sun_palettes[index % self.sun_palettes.len()]);
    }

    pub fn set_orbital_palette(&self, index: usize) {
        install_palette(&self.orbital_palettes[index % self.orbital_palettes.len()]);
    }
}

fn install_palette(bytes: &[u8]) {
    let [start, end, ..] = *bytes else {
        panic!("interplanetary palette has no range")
    };
    assert!(start <= end, "interplanetary palette range is reversed");
    let colors =
        (usize::from(end) - usize::from(start) + 1) * crate::graphics::cmap::NUMBER_OF_PLUTVALS * 3;
    assert!(
        bytes.len() >= colors + 2,
        "interplanetary palette is truncated"
    );
    // SAFETY: this owned slice includes all RGB triplets described by its range.
    unsafe { crate::graphics::cmap_abi::SetColorMap(bytes.as_ptr()) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn missing_resources_fail_at_their_original_load_position() {
        assert!(matches!(
            SolarResources::load(),
            Err("graphics.orbitbackground")
        ));
        for failed in 0..6 {
            let calls = std::cell::Cell::new(0);
            let next = || {
                let current = calls.get();
                calls.set(current + 1);
                current != failed
            };
            let result = SolarResources::load_with(
                |_| next().then(|| LoadedDrawable::create(2, 0, 0, 1)).flatten(),
                |_| next().then(|| vec![vec![0, 0]]),
                |_| panic!("music must not load after an asset failure"),
            );
            assert!(result.is_err());
            assert_eq!(calls.get(), failed + 1);
        }
    }

    #[test]
    #[serial_test::serial]
    fn retained_cache_keeps_real_drawables_and_owned_palette_bytes() {
        let resources = SolarResources::load_with(
            |_| LoadedDrawable::create(2, 0, 0, 2),
            |_| Some(vec![vec![0, 0]]),
            |_| None,
        )
        .unwrap_or_else(|key| panic!("fixture resource {key}"));
        assert!(resources.flagship.lease().frame(0).is_some());
        assert!(resources.sun.lease().frame(1).is_some());
        assert!(resources.sun.lease().frame(2).is_none());
        assert_eq!(resources.orbital_palettes, [vec![0, 0]]);
        assert!(matches!(resources.music_request(), MusicRequest::Absent));
    }

    #[test]
    fn palette_rejects_short_ranges_and_truncated_colors() {
        for bytes in [vec![], vec![0], vec![2, 1], vec![0, 0, 1, 2]] {
            assert!(std::panic::catch_unwind(|| install_palette(&bytes)).is_err());
        }
    }

    #[test]
    #[serial_test::serial]
    fn sequential_visits_reuse_the_retained_drawables_and_music_selection() {
        let loads = std::cell::Cell::new(0);
        let mut slot = None;
        let mut frame = None;
        for _ in 0..2 {
            let resources = SolarResources::retain_with(&mut slot, || {
                loads.set(loads.get() + 1);
                SolarResources::load_with(
                    |_| LoadedDrawable::create(2, 0, 0, 2),
                    |_| Some(vec![vec![0, 0]]),
                    |_| None,
                )
            });
            let current = resources.flagship.lease().frame(0);
            assert!(current.is_some());
            if let Some(previous) = frame {
                assert_eq!(current, previous);
            }
            frame = Some(current);
            assert!(matches!(resources.music_request(), MusicRequest::Absent));
        }
        assert_eq!(loads.get(), 1);
        assert!(slot.is_some());
    }
}
