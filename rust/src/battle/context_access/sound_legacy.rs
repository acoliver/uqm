#[cfg(not(test))]
use crate::battle::sounds::engine;
#[cfg(not(test))]
use crate::engine::battle::{Cx, ElementId};
#[cfg(not(test))]
use crate::ships::native::abi::{SoundPosition, SOUND};

#[cfg(not(test))]
pub unsafe fn play_sound(
    sound: SOUND,
    position: SoundPosition,
    source: Option<ElementId>,
    priority: u8,
) {
    // SAFETY: the published game thread retains the checked optional source and loaded sound.
    unsafe {
        crate::app::bridge::enter(|cx| engine::play_sound(cx, sound, position, source, priority));
    }
}

/// # Safety
/// Called on the owning game thread with a published battle context.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn UpdateSoundPositions() {
    // SAFETY: the legacy entry lends the context to the native sound pass.
    unsafe {
        crate::app::bridge::enter(|cx| engine::update_positions(cx));
    }
}
/// # Safety
/// Called on the owning game thread with a published battle context.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn FlushSounds() {
    // SAFETY: the legacy entry lends the context to the native sound pass.
    unsafe {
        crate::app::bridge::enter(|cx| engine::flush_sounds(cx));
    }
}
