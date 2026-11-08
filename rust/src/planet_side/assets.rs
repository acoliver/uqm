//! Typed PlanetSide asset ownership and lander sound indexing.

use std::ffi::c_void;

use super::hazards::SoundCue;
use super::runtime::{AdapterError, PlanetSideAudio};

/// Sound availability metadata from a resource port, or a retained bank.
///
/// Production audio requires the retained bank admitted by the asset owner.
/// Raw port metadata is never dereferenced for playback.
#[derive(Debug, Clone)]
pub struct LanderSoundTable {
    raw: *mut c_void,
    owner: Option<crate::sound::owned_bank::LoadedSoundBank>,
}

impl LanderSoundTable {
    #[must_use]
    pub const fn from_raw(raw: *mut c_void) -> Self {
        Self { raw, owner: None }
    }

    #[must_use]
    pub const fn is_loaded(&self) -> bool {
        !self.raw.is_null()
    }
    pub(crate) fn from_bank(bank: crate::sound::owned_bank::LoadedSoundBank) -> Self {
        Self {
            raw: bank.as_raw().cast(),
            owner: Some(bank),
        }
    }

    fn clip(&self, index: u16) -> crate::sound::owned_bank::SoundClip {
        self.owner
            .as_ref()
            .unwrap_or_else(|| panic!("lander audio requires its retained sound bank"))
            .clip(index)
    }
}

/// Concrete audio adapter for an already loaded lander sound table.
pub struct CffiPlanetSideAudio {
    sounds: LanderSoundTable,
    telemetry: std::rc::Rc<std::cell::RefCell<super::telemetry::TripTelemetry>>,
    channels: std::rc::Rc<std::cell::RefCell<crate::engine::battle::kernel::Channels>>,
}

impl CffiPlanetSideAudio {
    #[must_use]
    pub(crate) const fn new(
        sounds: LanderSoundTable,
        channels: std::rc::Rc<std::cell::RefCell<crate::engine::battle::kernel::Channels>>,
        telemetry: std::rc::Rc<std::cell::RefCell<super::telemetry::TripTelemetry>>,
    ) -> Self {
        Self {
            sounds,
            channels,
            telemetry,
        }
    }
}

impl PlanetSideAudio for CffiPlanetSideAudio {
    fn play(&mut self, world: &crate::engine::World, cue: SoundCue) -> Result<(), AdapterError> {
        if !self.sounds.is_loaded() {
            return Err(AdapterError::new("lander_sound_table_not_loaded"));
        }

        let sound = self.sounds.clip(sound_index(cue));
        #[cfg(not(test))]
        {
            let priority = if matches!(cue, SoundCue::Returns | SoundCue::Destroyed) {
                3
            } else {
                2
            };
            crate::battle::sounds::engine::play_sound(
                world,
                &mut self.channels.borrow_mut(),
                &sound,
                crate::sound::types::SoundPosition::non_positional(),
                None,
                priority,
            );
        }
        #[cfg(test)]
        let _ = (world, sound);
        self.telemetry.borrow_mut().sound(cue);
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
            super::super::telemetry::tests::trip(),
        );
        assert!(std::rc::Rc::ptr_eq(&audio.channels, &channels));
        drop(game);
        assert_eq!(audio.channels.borrow_mut().select(1, |_| false), 0);
    }

    #[test]
    fn lander_audio_retains_its_bank_and_accepts_an_unpublished_world() {
        let bank = crate::sound::owned_bank::LoadedSoundBank::from_samples(
            (0..13)
                .map(|_| crate::sound::types::SoundSample::new())
                .collect(),
        );
        let identity = bank.clip(8).identity();
        let sounds = LanderSoundTable::from_bank(bank.clone());
        let channels = std::rc::Rc::new(std::cell::RefCell::new(
            crate::engine::battle::kernel::Channels::new(),
        ));
        let mut audio =
            CffiPlanetSideAudio::new(sounds, channels, super::super::telemetry::tests::trip());
        drop(bank);
        assert_eq!(audio.sounds.clip(8).identity(), identity);
        audio
            .play(&crate::engine::World::new(), SoundCue::Pickup)
            .unwrap();
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
            super::super::telemetry::tests::trip(),
        );
        assert_eq!(
            audio.play(&crate::engine::World::new(), SoundCue::Pickup),
            Err(AdapterError::new("lander_sound_table_not_loaded"))
        );
    }

    #[test]
    fn loaded_non_linked_adapter_accepts_all_cues() {
        let mut audio = CffiPlanetSideAudio::new(
            LanderSoundTable::from_bank(crate::sound::owned_bank::LoadedSoundBank::from_samples(
                vec![crate::sound::types::SoundSample::new()],
            )),
            crate::engine::Game::new().kernel.channels,
            super::super::telemetry::tests::trip(),
        );
        assert_eq!(
            audio.play(&crate::engine::World::new(), SoundCue::Destroyed),
            Ok(())
        );
    }
}
