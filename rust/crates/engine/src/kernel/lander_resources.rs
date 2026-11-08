//! Campaign-owned lander graphics, sound bank and orbital themes.

use crate::{
    graphics::{context_abi::ContextFrame, drawable_abi::LoadedDrawable},
    sound::{owned_bank::LoadedSoundBank, resource_types::LoadedMusic, types::MusicRef},
};
use std::ffi::CStr;

const GRAPHICS: [&CStr; 8] = [
    c"graphics.lander",
    c"graphics.quake",
    c"graphics.lightning",
    c"graphics.lavaspot",
    c"graphics.landershield",
    c"graphics.landerlaunch",
    c"graphics.landerreturn",
    c"graphics.orbview",
];
const THEMES: [&CStr; 5] = [
    c"music.orbit1",
    c"music.orbit2",
    c"music.orbit3",
    c"music.orbit4",
    c"music.orbit5",
];

pub struct LanderResources {
    themes: [Option<LoadedMusic>; 5],
    graphics: Vec<LoadedDrawable>,
    sounds: Option<LoadedSoundBank>,
}

impl LanderResources {
    pub fn load() -> Result<Self, &'static str> {
        let mut owner = Self {
            themes: THEMES.map(LoadedMusic::load),
            graphics: Vec::with_capacity(8),
            sounds: None,
        };
        for key in GRAPHICS {
            let Some(graphic) = LoadedDrawable::load(key) else {
                while owner.graphics.pop().is_some() {}
                return Err(key.to_str().unwrap_or_else(|_| unreachable!()));
            };
            owner.graphics.push(graphic);
        }
        owner.sounds = LoadedSoundBank::load(c"sounds.lander");
        if owner.sounds.is_none() {
            while owner.graphics.pop().is_some() {}
            return Err("sounds.lander");
        }
        Ok(owner)
    }

    pub fn graphic(&self, index: usize) -> ContextFrame {
        self.graphics[index].lease().context_frame(0)
    }

    pub fn sounds(&self) -> LoadedSoundBank {
        self.sounds
            .as_ref()
            .unwrap_or_else(|| panic!("loaded lander has no sound bank"))
            .clone()
    }

    pub fn music(&self, planet_type: u8) -> Option<MusicRef> {
        self.themes[usize::from(planet_type) % THEMES.len()]
            .as_ref()
            .map(LoadedMusic::playback)
    }
}

impl Drop for LanderResources {
    fn drop(&mut self) {
        for theme in &mut self.themes {
            drop(theme.take());
        }
        self.graphics.clear();
        self.sounds = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn missing_graphics_fail_without_publishing_partial_assets() {
        assert!(matches!(LanderResources::load(), Err("graphics.lander")));
    }
}
