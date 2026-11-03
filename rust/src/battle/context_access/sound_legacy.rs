#[cfg(not(test))]
use crate::battle::sounds::engine;
#[cfg(not(test))]
use crate::engine::battle::ElementId;
#[cfg(not(test))]
use crate::ships::native::abi::{SoundPosition, SOUND};

#[cfg(not(test))]
/// # Safety
/// The game thread retains the initialized battle context and all resources used by this call.
pub unsafe fn play_sound(
    sound: SOUND,
    position: SoundPosition,
    source: Option<ElementId>,
    priority: u8,
) {
    // SAFETY: the published game thread retains the checked optional source and loaded sound.
    unsafe {
        crate::app::bridge::enter(|cx| {
            engine::play_sound_in_context(cx, sound, position, source, priority)
        });
    }
}
