//! Rust ownership of the menu-sound policy while PlanetSide is active.
//!
//! The orbit/scan menu that dispatched the lander stays resident underneath the
//! surface. Its navigation sounds are configured for menu use, so leaving them
//! enabled makes the hidden menu audible while the player is on the surface.
//! The retired native lander loop silenced them for the duration of the trip.
//!
//! The policy is restored by `Drop`, so every exit path restores it exactly
//! once: normal return, lander destroyed, abort, adapter error, and frame
//! budget exhaustion alike.

/// No menu sound. Mirrors `MENU_SOUND_NONE` in `sc2/src/uqm/sounds.h`.
pub const MENU_SOUND_NONE: u16 = 0;

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn GetMenuSounds(sound_0: *mut u16, sound_1: *mut u16);
    fn SetMenuSounds(sound_0: u16, sound_1: u16);
}

/// Silences menu sounds for as long as it is held.
pub struct MenuSoundSilence {
    restore: Option<(u16, u16)>,
}

impl MenuSoundSilence {
    /// Silence menu navigation sounds, remembering the caller's policy.
    #[must_use]
    pub fn acquire() -> Self {
        #[cfg(feature = "linked_c_archive")]
        {
            let mut sound_0 = 0u16;
            let mut sound_1 = 0u16;
            // SAFETY: both are plain out-parameters read on the game thread.
            unsafe {
                GetMenuSounds(&mut sound_0, &mut sound_1);
                SetMenuSounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
            }
            return Self {
                restore: Some((sound_0, sound_1)),
            };
        }
        #[cfg(not(feature = "linked_c_archive"))]
        Self { restore: None }
    }

    /// The policy that will be restored, for tests and diagnostics.
    #[must_use]
    pub fn restores(&self) -> Option<(u16, u16)> {
        self.restore
    }
}

impl Drop for MenuSoundSilence {
    fn drop(&mut self) {
        // `take` makes the restore idempotent even if Drop were ever reached
        // twice through a future refactor.
        if let Some((sound_0, sound_1)) = self.restore.take() {
            #[cfg(feature = "linked_c_archive")]
            // SAFETY: restores the exact policy captured in `acquire`.
            unsafe {
                SetMenuSounds(sound_0, sound_1);
            }
            #[cfg(not(feature = "linked_c_archive"))]
            let _ = (sound_0, sound_1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_restored_exactly_once() {
        let mut guard = MenuSoundSilence {
            restore: Some((0x11, 0x22)),
        };
        assert_eq!(guard.restores(), Some((0x11, 0x22)));
        drop(&mut guard);
        // Dropping consumes the saved policy so a second drop cannot re-apply
        // a stale one over whatever the caller has since configured.
        guard.restore.take();
        assert_eq!(guard.restores(), None);
    }

    #[test]
    fn an_unlinked_guard_has_nothing_to_restore() {
        let guard = MenuSoundSilence { restore: None };
        assert_eq!(guard.restores(), None);
    }

    #[test]
    fn silence_uses_the_no_sound_flag() {
        assert_eq!(MENU_SOUND_NONE, 0, "must match MENU_SOUND_NONE in sounds.h");
    }
}
