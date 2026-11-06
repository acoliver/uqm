//! Typed PlanetSide asset ownership and lander sound indexing.

use std::ffi::c_void;

use super::hazards::SoundCue;
use super::runtime::{AdapterError, PlanetSideAudio};

/// Typed handle to the loaded lander sound table.
///
/// Ownership remains with the resource adapter that loaded the table. This
/// value is never exposed to the deterministic gameplay core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanderSoundTable(*mut c_void);

impl LanderSoundTable {
    #[must_use]
    pub const fn from_raw(raw: *mut c_void) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn is_loaded(self) -> bool {
        !self.0.is_null()
    }
}

/// Concrete audio adapter for an already loaded lander sound table.
pub struct CffiPlanetSideAudio {
    sounds: LanderSoundTable,
    channels: std::rc::Rc<std::cell::RefCell<crate::engine::battle::record::Channels>>,
}

impl CffiPlanetSideAudio {
    #[must_use]
    pub(crate) const fn new(
        sounds: LanderSoundTable,
        channels: std::rc::Rc<std::cell::RefCell<crate::engine::battle::record::Channels>>,
    ) -> Self {
        Self { sounds, channels }
    }
}

#[cfg(not(test))]
use crate::battle::sounds::engine::NON_POSITIONAL;
#[cfg(not(test))]
#[cfg(not(test))]
use crate::strings::string_table::SetAbsStringTableIndex;

impl PlanetSideAudio for CffiPlanetSideAudio {
    fn play(&mut self, cue: SoundCue) -> Result<(), AdapterError> {
        if !self.sounds.is_loaded() {
            return Err(AdapterError::new("lander_sound_table_not_loaded"));
        }

        #[cfg(not(test))]
        unsafe {
            let sound = SetAbsStringTableIndex((self.sounds.0).cast(), sound_index(cue));
            let priority = if matches!(cue, SoundCue::Returns | SoundCue::Destroyed) {
                3
            } else {
                2
            };
            crate::battle::sounds::engine::play_sound(
                &mut self.channels.borrow_mut(),
                sound,
                NON_POSITIONAL,
                None,
                priority,
            );
        }
        #[cfg(test)]
        let _ = cue;
        super::telemetry::sound(cue);
        Ok(())
    }
}

const fn sound_index(cue: SoundCue) -> u16 {
    match cue {
        SoundCue::BiologicalDisaster => 0,
        SoundCue::Earthquake => 1,
        SoundCue::Lightning => 2,
        SoundCue::Lava => 3,
        SoundCue::LanderInjured => 4,
        SoundCue::LanderShoots => 5,
        SoundCue::LanderHits => 6,
        SoundCue::LifeformCanned => 7,
        SoundCue::Pickup => 8,
        SoundCue::Full => 9,
        SoundCue::Departs => 10,
        SoundCue::Returns => 11,
        SoundCue::Destroyed => 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_retains_the_supplied_kernel_channel_state() {
        let game = crate::engine::Game::new();
        let channels = game.kernel.channels.clone();
        let audio = CffiPlanetSideAudio::new(
            LanderSoundTable::from_raw(std::ptr::null_mut()),
            channels.clone(),
        );
        assert!(std::rc::Rc::ptr_eq(&audio.channels, &channels));
        drop(game);
        assert_eq!(audio.channels.borrow_mut().select(1, |_| false), 0);
    }

    #[test]
    fn cue_indices_match_planets_h_order() {
        let cues = [
            SoundCue::BiologicalDisaster,
            SoundCue::Earthquake,
            SoundCue::Lightning,
            SoundCue::Lava,
            SoundCue::LanderInjured,
            SoundCue::LanderShoots,
            SoundCue::LanderHits,
            SoundCue::LifeformCanned,
            SoundCue::Pickup,
            SoundCue::Full,
            SoundCue::Departs,
            SoundCue::Returns,
            SoundCue::Destroyed,
        ];
        for (expected, cue) in cues.into_iter().enumerate() {
            assert_eq!(sound_index(cue), expected as u16);
        }
    }

    #[test]
    fn unloaded_sound_table_is_a_typed_error() {
        let mut audio = CffiPlanetSideAudio::new(
            LanderSoundTable::from_raw(std::ptr::null_mut()),
            crate::engine::Game::new().kernel.channels,
        );
        assert_eq!(
            audio.play(SoundCue::Pickup),
            Err(AdapterError::new("lander_sound_table_not_loaded"))
        );
    }

    #[test]
    fn loaded_non_linked_adapter_accepts_all_cues() {
        let raw = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let mut audio = CffiPlanetSideAudio::new(
            LanderSoundTable::from_raw(raw),
            crate::engine::Game::new().kernel.channels,
        );
        assert_eq!(audio.play(SoundCue::Destroyed), Ok(()));
    }
}
