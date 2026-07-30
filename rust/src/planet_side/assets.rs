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
}

impl CffiPlanetSideAudio {
    #[must_use]
    pub const fn new(sounds: LanderSoundTable) -> Self {
        Self { sounds }
    }
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
#[derive(Clone, Copy)]
struct SoundPosition {
    positional: i32,
    x: i32,
    y: i32,
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn SetAbsStringTableIndex(sound: *mut c_void, index: i32) -> *mut c_void;
    fn NotPositional() -> SoundPosition;
    fn PlaySound(
        sound: *mut c_void,
        position: SoundPosition,
        positional_object: *mut c_void,
        priority: u8,
    );
}

impl PlanetSideAudio for CffiPlanetSideAudio {
    fn play(&mut self, cue: SoundCue) -> Result<(), AdapterError> {
        if !self.sounds.is_loaded() {
            return Err(AdapterError::new("lander_sound_table_not_loaded"));
        }

        #[cfg(feature = "linked_c_archive")]
        unsafe {
            let sound = SetAbsStringTableIndex(self.sounds.0, i32::from(sound_index(cue)));
            let priority = if matches!(cue, SoundCue::Returns | SoundCue::Destroyed) {
                3
            } else {
                2
            };
            PlaySound(sound, NotPositional(), std::ptr::null_mut(), priority);
        }
        #[cfg(not(feature = "linked_c_archive"))]
        let _ = cue;
        super::telemetry::sound(cue);
        Ok(())
    }
}

#[cfg(any(feature = "linked_c_archive", test))]
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
        let mut audio = CffiPlanetSideAudio::new(LanderSoundTable::from_raw(std::ptr::null_mut()));
        assert_eq!(
            audio.play(SoundCue::Pickup),
            Err(AdapterError::new("lander_sound_table_not_loaded"))
        );
    }

    #[test]
    fn loaded_non_linked_adapter_accepts_all_cues() {
        let raw = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let mut audio = CffiPlanetSideAudio::new(LanderSoundTable::from_raw(raw));
        assert_eq!(audio.play(SoundCue::Destroyed), Ok(()));
    }
}
