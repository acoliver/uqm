//! The caller's menu-sound policy is restored when a surface trip returns.

use crate::engine::input::InputControls;

pub const MENU_SOUND_NONE: u16 = 0;

/// The restoration runs on the provider's owner thread.
///
/// ```compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<uqm_rust::planet_side::menu_sounds::MenuSoundSilence<'static>>();
/// ```
pub struct MenuSoundSilence<'a> {
    input: &'a InputControls,
    restore: (u16, u16),
    owner_thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl<'a> MenuSoundSilence<'a> {
    #[must_use]
    pub fn acquire(input: &'a InputControls) -> Self {
        let restore = input.menu_sounds();
        input.set_menu_sounds(MENU_SOUND_NONE, MENU_SOUND_NONE);
        Self {
            input,
            restore,
            owner_thread: std::marker::PhantomData,
        }
    }

    #[must_use]
    pub fn restores(&self) -> Option<(u16, u16)> {
        Some(self.restore)
    }
}

impl Drop for MenuSoundSilence<'_> {
    fn drop(&mut self) {
        self.input.set_menu_sounds(self.restore.0, self.restore.1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silencing_restores_the_callers_policy() {
        let input = InputControls::default();
        input.set_menu_sounds(0x0f, 0x10);
        {
            let guard = MenuSoundSilence::acquire(&input);
            assert_eq!(guard.restores(), Some((0x0f, 0x10)));
            assert_eq!(input.menu_sounds(), (MENU_SOUND_NONE, MENU_SOUND_NONE));
        }
        assert_eq!(input.menu_sounds(), (0x0f, 0x10));
    }

    #[test]
    fn nested_trips_each_restore_their_own_caller_policy() {
        let input = InputControls::default();
        input.set_menu_sounds(0x21, 0x22);
        {
            let _outer = MenuSoundSilence::acquire(&input);
            {
                let _inner = MenuSoundSilence::acquire(&input);
                assert_eq!(input.menu_sounds(), (MENU_SOUND_NONE, MENU_SOUND_NONE));
            }
            assert_eq!(input.menu_sounds(), (MENU_SOUND_NONE, MENU_SOUND_NONE));
        }
        assert_eq!(input.menu_sounds(), (0x21, 0x22));
    }

    #[test]
    fn unwinding_restores_the_saved_policy() {
        let input = InputControls::default();
        input.set_menu_sounds(0x51, 0x52);
        let result = std::panic::catch_unwind(|| {
            let _guard = MenuSoundSilence::acquire(&input);
            assert_eq!(input.menu_sounds(), (MENU_SOUND_NONE, MENU_SOUND_NONE));
            panic!("surface trip failed");
        });
        assert!(result.is_err());
        assert_eq!(input.menu_sounds(), (0x51, 0x52));
    }

    #[test]
    fn silence_does_not_change_another_runtime() {
        let first = InputControls::default();
        let second = InputControls::default();
        first.set_menu_sounds(1, 2);
        second.set_menu_sounds(3, 4);
        let _guard = MenuSoundSilence::acquire(&first);
        assert_eq!(second.menu_sounds(), (3, 4));
        assert_eq!(MENU_SOUND_NONE, 0);
    }
}
