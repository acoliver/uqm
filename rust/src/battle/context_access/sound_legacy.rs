#[cfg(not(test))]
use crate::battle::sounds::engine;
#[cfg(not(test))]
use crate::engine::battle::{record::ELEMENT, Cx, ElementId};
#[cfg(not(test))]
use crate::ships::native::abi::{SoundPosition, SOUND};

#[cfg(not(test))]
unsafe fn with_object<R>(
    object: *mut ELEMENT,
    call: impl FnOnce(&mut Cx<'_>, Option<ElementId>) -> R,
) -> R {
    // SAFETY: caller holds the published game-thread context and a live optional element.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let id = cx.elements().id(object.cast());
            call(cx, id)
        })
    }
}

/// # Safety
/// Sound and a non-null object remain live on the owning game thread.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn PlaySound(
    sound: SOUND,
    position: SoundPosition,
    object: *mut ELEMENT,
    priority: u8,
) {
    // SAFETY: legacy storage is decoded before entering the id-based sound queue.
    unsafe {
        with_object(object, |cx, id| {
            engine::play_sound(cx, sound, position, id, priority)
        });
    }
}
/// # Safety
/// Sound and a non-null object remain live on the owning game thread.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn ProcessSound(sound: SOUND, object: *mut ELEMENT) {
    // SAFETY: legacy storage is decoded before entering the id-based sound queue.
    unsafe {
        with_object(object, |cx, id| cx.b.kernel_sounds().push(sound, id));
    }
}
/// # Safety
/// A non-null object is live and owns its display primitive on the game thread.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn CalcSoundPosition(object: *mut ELEMENT) -> SoundPosition {
    // SAFETY: no raw record borrow survives the projection.
    unsafe { with_object(object, engine::position) }
}
/// # Safety
/// Called on the game thread before the element's storage is released.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn RemoveSoundsForObject(object: *mut ELEMENT) {
    // SAFETY: the source is decoded before removal and remains live through it.
    unsafe {
        with_object(object, |_, id| engine::remove_sounds(id));
    }
}

/// # Safety
/// Called on the owning game thread with a published battle context.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn UpdateSoundPositions() {
    // SAFETY: the legacy entry lends the context to the native sound pass.
    unsafe {
        with_object(std::ptr::null_mut(), |cx, _| engine::update_positions(cx));
    }
}
/// # Safety
/// Called on the owning game thread with a published battle context.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn FlushSounds() {
    // SAFETY: the legacy entry lends the context to the native sound pass.
    unsafe {
        with_object(std::ptr::null_mut(), |cx, _| engine::flush_sounds(cx));
    }
}
