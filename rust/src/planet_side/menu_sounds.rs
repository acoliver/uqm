//! Rust ownership of the menu-sound policy while PlanetSide is active.
//!
//! The orbit/scan menu that dispatched the lander stays resident underneath the
//! surface. Its navigation sounds are configured for menu use, so leaving them
//! enabled makes the hidden menu audible while the player is on the surface.
//! The retired native lander loop silenced them for the duration of the trip.
//!
//! The policy is restored by `Drop`, so every structured exit restores it
//! exactly once: normal return, lander destroyed, abort, adapter error and
//! frame budget exhaustion alike.

use std::marker::PhantomData;

/// No menu sound. Mirrors `MENU_SOUND_NONE` in `sc2/src/uqm/sounds.h`.
pub const MENU_SOUND_NONE: u16 = 0;

/// Read the caller's menu-sound policy.
///
/// # Safety
/// `GetMenuSounds`/`SetMenuSounds` read and write unsynchronised C statics, so
/// they may only be used from the thread that owns the game loop. That is
/// enforced by keeping [`MenuSoundSilence`] `!Send`.
#[cfg(feature = "linked_c_archive")]
mod policy {
    extern "C" {
        fn GetMenuSounds(sound_0: *mut u16, sound_1: *mut u16);
        fn SetMenuSounds(sound_0: u16, sound_1: u16);
    }

    pub(super) fn get() -> (u16, u16) {
        let mut sound_0 = 0u16;
        let mut sound_1 = 0u16;
        // SAFETY: plain out-parameters; caller is the game-loop thread because
        // the guard that reaches this is `!Send`.
        unsafe { GetMenuSounds(&mut sound_0, &mut sound_1) };
        (sound_0, sound_1)
    }

    pub(super) fn set(sound_0: u16, sound_1: u16) {
        // SAFETY: as above; writes two C statics owned by the game loop.
        unsafe { SetMenuSounds(sound_0, sound_1) };
    }
}

/// Unlinked builds keep the policy in thread-local state, which gives the tests
/// the same observable behaviour as the linked game without a C archive.
#[cfg(not(feature = "linked_c_archive"))]
mod policy {
    use std::cell::Cell;

    thread_local! {
        static CURRENT: Cell<(u16, u16)> = const { Cell::new((0, 0)) };
        static SETS: Cell<u32> = const { Cell::new(0) };
    }

    pub(super) fn get() -> (u16, u16) {
        CURRENT.with(Cell::get)
    }

    pub(super) fn set(sound_0: u16, sound_1: u16) {
        CURRENT.with(|current| current.set((sound_0, sound_1)));
        SETS.with(|sets| sets.set(sets.get() + 1));
    }

    #[cfg(test)]
    pub(super) fn reset(policy: (u16, u16)) {
        CURRENT.with(|current| current.set(policy));
        SETS.with(|sets| sets.set(0));
    }

    #[cfg(test)]
    pub(super) fn set_count() -> u32 {
        SETS.with(Cell::get)
    }
}

/// Silences menu sounds for as long as it is held.
///
/// Deliberately `!Send`: the underlying C statics are unsynchronised and belong
/// to the game-loop thread, so the guard must be dropped on the thread that
/// acquired it. Moving one across threads does not compile:
///
/// ```compile_fail
/// use uqm_rust::planet_side::menu_sounds::MenuSoundSilence;
/// fn needs_send<T: Send>(_value: T) {}
/// needs_send(MenuSoundSilence::acquire());
/// ```
pub struct MenuSoundSilence {
    restore: Option<(u16, u16)>,
    _not_send: PhantomData<*const ()>,
}

impl MenuSoundSilence {
    /// Silence menu navigation sounds, remembering the caller's policy.
    #[must_use]
    pub fn acquire() -> Self {
        let restore = policy::get();
        policy::set(MENU_SOUND_NONE, MENU_SOUND_NONE);
        Self {
            restore: Some(restore),
            _not_send: PhantomData,
        }
    }

    /// The policy that will be restored, for tests and diagnostics.
    #[must_use]
    pub fn restores(&self) -> Option<(u16, u16)> {
        self.restore
    }
}

impl Drop for MenuSoundSilence {
    fn drop(&mut self) {
        if let Some((sound_0, sound_1)) = self.restore.take() {
            policy::set(sound_0, sound_1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The thread-local policy records what was set; the linked build drives
    // unsynchronised C statics instead and cannot observe that.
    #[cfg(not(feature = "linked_c_archive"))]
    #[test]
    fn silencing_restores_the_callers_policy_exactly_once() {
        policy::reset((0x0F, 0x10));

        {
            let guard = MenuSoundSilence::acquire();
            assert_eq!(guard.restores(), Some((0x0F, 0x10)));
            assert_eq!(
                policy::get(),
                (MENU_SOUND_NONE, MENU_SOUND_NONE),
                "the resident menu must be silent during the trip"
            );
            assert_eq!(policy::set_count(), 1);
        }

        assert_eq!(
            policy::get(),
            (0x0F, 0x10),
            "the caller's policy must come back"
        );
        assert_eq!(
            policy::set_count(),
            2,
            "exactly one silence and one restore"
        );
    }

    // The thread-local policy records what was set; the linked build drives
    // unsynchronised C statics instead and cannot observe that.
    #[cfg(not(feature = "linked_c_archive"))]
    #[test]
    fn nested_trips_each_restore_their_own_caller_policy() {
        policy::reset((0x21, 0x22));
        {
            let _outer = MenuSoundSilence::acquire();
            {
                let _inner = MenuSoundSilence::acquire();
                assert_eq!(policy::get(), (MENU_SOUND_NONE, MENU_SOUND_NONE));
            }
            // The inner guard restores what it found, which was silence.
            assert_eq!(policy::get(), (MENU_SOUND_NONE, MENU_SOUND_NONE));
        }
        assert_eq!(policy::get(), (0x21, 0x22));
    }

    #[test]
    fn silence_uses_the_no_sound_flag() {
        assert_eq!(MENU_SOUND_NONE, 0, "must match MENU_SOUND_NONE in sounds.h");
    }
}
