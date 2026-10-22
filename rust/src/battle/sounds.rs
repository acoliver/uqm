//! Battle sound queue, effect-channel reuse and positional-object tracking.

use crate::ships::native::abi::{ELEMENT, SOUND};
use std::ptr;

struct SoundQueue {
    count: usize,
    sounds: [SOUND; 8],
    objects: [*mut ELEMENT; 8],
}

impl SoundQueue {
    const fn new() -> Self {
        Self {
            count: 0,
            sounds: [ptr::null_mut(); 8],
            objects: [ptr::null_mut(); 8],
        }
    }

    fn push(&mut self, sound: SOUND, object: *mut ELEMENT) {
        if sound as usize == usize::MAX {
            self.sounds.fill(ptr::null_mut());
            self.objects.fill(ptr::null_mut());
            self.count = 8;
        } else if self.count < 8 {
            self.sounds[self.count] = sound;
            self.objects[self.count] = object;
            self.count += 1;
        }
    }
}

struct Channels {
    lru: [usize; 4],
    sounds: [SOUND; 4],
}

impl Channels {
    const fn new() -> Self {
        Self {
            lru: [0, 1, 2, 3],
            sounds: [ptr::null_mut(); 4],
        }
    }

    fn select(&mut self, sound: SOUND, mut playing: impl FnMut(usize) -> bool) -> usize {
        let channel = self
            .sounds
            .iter()
            .position(|s| *s == sound)
            .or_else(|| (0..4).find(|&i| !playing(i)))
            .unwrap_or(self.lru[0]);
        self.sounds[channel] = sound;
        if let Some(i) = self.lru[..3].iter().position(|&c| c == channel) {
            self.lru.copy_within(i + 1..4, i);
        }
        self.lru[3] = channel;
        channel
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
#[allow(non_snake_case)]
mod engine {
    use super::*;
    use crate::ships::native::abi::*;

    #[no_mangle]
    pub static mut MenuSounds: SOUND = ptr::null_mut();
    #[no_mangle]
    pub static mut GameSounds: SOUND = ptr::null_mut();
    static mut QUEUE: SoundQueue = SoundQueue::new();
    static mut CHANNELS: Channels = Channels::new();

    /// # Safety
    /// Called on the game thread with a live sound and optional live element.
    #[no_mangle]
    pub unsafe extern "C" fn PlaySound(
        sound: SOUND,
        pos: SoundPosition,
        object: *mut ELEMENT,
        priority: u8,
    ) {
        if sound.is_null() {
            return;
        }
        let channels = &raw mut CHANNELS;
        let channel = (*channels).select(sound, |c| {
            ChannelPlaying((c + MIN_FX_CHANNEL as usize) as u16) != 0
        });
        PlaySoundEffect(
            sound,
            (channel + MIN_FX_CHANNEL as usize) as u16,
            pos,
            object.cast(),
            priority,
        );
    }

    /// # Safety
    /// Menu sound resources must be loaded on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn PlayMenuSound(sound: u32) {
        PlaySoundEffect(
            SetAbsStringTableIndex(MenuSounds, sound as u16),
            0,
            NotPositional(),
            ptr::null_mut(),
            GAME_SOUND_PRIORITY as u8,
        );
    }

    /// # Safety
    /// Called on the game thread; object remains live until removed or flushed.
    #[no_mangle]
    pub unsafe extern "C" fn ProcessSound(sound: SOUND, object: *mut ELEMENT) {
        let queue = &raw mut QUEUE;
        (*queue).push(sound, object);
    }

    /// # Safety
    /// A non-null object must have a live DisplayArray primitive.
    #[no_mangle]
    pub unsafe extern "C" fn CalcSoundPosition(object: *mut ELEMENT) -> SoundPosition {
        if object.is_null() {
            return SoundPosition {
                positional: 0,
                x: 0,
                y: 0,
            };
        }
        let prim = &*(&raw const DisplayArray)
            .cast::<PRIMITIVE>()
            .add((*object).PrimIndex as usize);
        let point = if prim.Type == gfx_object_LINE_PRIM as u8 {
            prim.Object.Line.first
        } else {
            prim.Object.Point
        };
        SoundPosition {
            positional: 1,
            x: i32::from(point.x) - ((ScreenWidth - 64) >> 1),
            y: i32::from(point.y) - (ScreenHeight >> 1),
        }
    }

    /// # Safety
    /// Must run on the game thread like the positional variant.
    #[no_mangle]
    pub unsafe extern "C" fn NotPositional() -> SoundPosition {
        CalcSoundPosition(ptr::null_mut())
    }

    /// # Safety
    /// Audio positional objects must still be live game elements.
    #[no_mangle]
    pub unsafe extern "C" fn UpdateSoundPositions() {
        for i in FIRST_SFX_CHANNEL..=LAST_SFX_CHANNEL {
            if ChannelPlaying(i as u16) == 0 {
                continue;
            }
            let object = GetPositionalObject(i as u16).cast::<ELEMENT>();
            if !object.is_null() {
                let pos = CalcSoundPosition(object);
                if pos.positional != 0 {
                    UpdateSoundPosition(i as u16, pos);
                }
            }
        }
    }

    /// # Safety
    /// Called on the game thread while queued elements and sounds remain live.
    #[no_mangle]
    pub unsafe extern "C" fn FlushSounds() {
        let mut i = 0;
        while QUEUE.count > 0 {
            let sound = QUEUE.sounds[i];
            let object = QUEUE.objects[i];
            PlaySound(
                sound,
                CalcSoundPosition(object),
                object,
                GAME_SOUND_PRIORITY as u8,
            );
            i += 1;
            QUEUE.count -= 1;
        }
    }

    /// # Safety
    /// Called on the game thread before releasing an element's storage.
    #[no_mangle]
    pub unsafe extern "C" fn RemoveSoundsForObject(object: *mut ELEMENT) {
        for i in FIRST_SFX_CHANNEL..=LAST_SFX_CHANNEL {
            if GetPositionalObject(i as u16) == object.cast() {
                SetPositionalObject(i as u16, ptr::null_mut());
            }
        }
        let queue_ptr = &raw mut QUEUE;
        let queue = &mut *queue_ptr;
        for entry in &mut queue.objects[..queue.count] {
            if *entry == object {
                *entry = ptr::null_mut();
            }
        }
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use engine::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_reuse_precedes_idle_search_and_lru_eviction() {
        let mut channels = Channels::new();
        for i in 0..4 {
            assert_eq!(channels.select((i + 1) as SOUND, |_| true), i);
        }
        assert_eq!(
            channels.select(2 as SOUND, |_| panic!(
                "reused channel must not query playback"
            )),
            1
        );
        assert_eq!(channels.select(5 as SOUND, |_| true), 0);
        assert_eq!(channels.select(6 as SOUND, |i| i != 2), 2);
        assert_eq!(channels.lru, [3, 1, 0, 2]);
    }

    #[test]
    fn queue_preserves_order_caps_at_eight_and_sentinel_suppresses_sounds() {
        let mut queue = SoundQueue::new();
        for i in 1..=10 {
            queue.push(i as SOUND, i as *mut ELEMENT);
        }
        assert_eq!(queue.count, 8);
        assert_eq!(queue.sounds[7] as usize, 8);
        queue.push(usize::MAX as SOUND, ptr::null_mut());
        queue.push(12 as SOUND, ptr::null_mut());
        assert_eq!(queue.count, 8);
        assert!(queue.sounds.iter().all(|p| p.is_null()));
        assert!(queue.objects.iter().all(|p| p.is_null()));
    }
}
