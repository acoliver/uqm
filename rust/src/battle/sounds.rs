//! Battle sound queue, effect-channel reuse and positional-object tracking.

#[cfg(not(test))]
use crate::engine::battle::Cx;
use crate::engine::battle::ElementId;
use crate::ships::native::abi::SOUND;
use std::ptr;

use crate::engine::battle::record::{Channels, SoundQueue};

impl SoundQueue {
    pub(crate) const fn new() -> Self {
        Self {
            count: 0,
            sounds: [ptr::null_mut(); 8],
            objects: [None; 8],
        }
    }

    pub(crate) fn push(&mut self, sound: SOUND, object: Option<ElementId>) {
        if sound as usize == usize::MAX {
            self.sounds.fill(ptr::null_mut());
            self.objects.fill(None);
            self.count = 8;
        } else if self.count < 8 {
            self.sounds[self.count] = sound;
            self.objects[self.count] = object;
            self.count += 1;
        }
    }
    pub(crate) fn remove_object(&mut self, object: Option<ElementId>) {
        for entry in &mut self.objects[..self.count] {
            if *entry == object {
                *entry = None;
            }
        }
    }
}
impl Channels {
    pub(crate) const fn new() -> Self {
        Self {
            lru: [0, 1, 2, 3],
            sounds: [ptr::null_mut(); 4],
        }
    }

    pub(crate) fn select(&mut self, sound: SOUND, mut playing: impl FnMut(usize) -> bool) -> usize {
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

#[cfg(not(test))]
use crate::sound::heart_ffi::{
    get_positional_object, set_positional_object, update_sound_position,
};

#[cfg(not(test))]
#[allow(non_snake_case)]
pub(crate) mod engine {
    pub use super::*;
    use crate::ships::native::abi::*;

    /// # Safety
    /// Called on the game thread with a live sound and optional live element.
    pub(crate) unsafe fn play_sound(
        cx: &mut Cx<'_>,
        sound: SOUND,
        pos: SoundPosition,
        object: Option<ElementId>,
        priority: u8,
    ) {
        if sound.is_null() {
            return;
        }
        let channels = cx.b.kernel_channels();
        let channel = channels.select(sound, |c| {
            ChannelPlaying((c + MIN_FX_CHANNEL as usize) as u32) != 0
        });
        PlaySoundEffect(
            sound,
            (channel + MIN_FX_CHANNEL as usize) as u16,
            pos,
            encode_object(object) as *mut std::ffi::c_void,
            priority,
        );
    }

    /// # Safety
    /// Menu sound resources must be loaded on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn PlayMenuSound(sound: u32) {
        PlaySoundEffect(
            SetAbsStringTableIndex(Kernel::current().menu_sounds, sound as u16),
            0,
            NotPositional(),
            ptr::null_mut(),
            GAME_SOUND_PRIORITY as u8,
        );
    }

    /// # Safety
    /// Called while the game-thread battle context is published.
    pub(crate) fn position(cx: &mut Cx<'_>, object: Option<ElementId>) -> SoundPosition {
        let Some(id) = object.filter(|id| cx.elements().contains(*id)) else {
            return SoundPosition {
                positional: 0,
                x: 0,
                y: 0,
            };
        };
        let point = cx.sound_point(id);
        let (width, height) = cx.screen_dimensions();
        SoundPosition {
            positional: 1,
            x: i32::from(point.x) - ((width - 64) >> 1),
            y: i32::from(point.y) - (height >> 1),
        }
    }

    /// # Safety
    /// Must run on the game thread like the positional variant.
    #[no_mangle]
    pub unsafe extern "C" fn NotPositional() -> SoundPosition {
        SoundPosition {
            positional: 0,
            x: 0,
            y: 0,
        }
    }

    /// # Safety
    /// Audio positional objects must still be live game elements.
    pub(crate) unsafe fn update_positions(cx: &mut Cx<'_>) {
        for i in FIRST_SFX_CHANNEL..=LAST_SFX_CHANNEL {
            if ChannelPlaying(i) == 0 {
                continue;
            }
            let object = decode_object(get_positional_object(i));
            if object.is_some() {
                let pos = position(cx, object);
                if pos.positional != 0 {
                    update_sound_position(
                        i,
                        crate::sound::types::SoundPosition {
                            positional: true,
                            x: pos.x,
                            y: pos.y,
                        },
                    );
                } else {
                    set_positional_object(i, 0);
                }
            }
        }
    }

    /// # Safety
    /// Called on the game thread while queued elements and sounds remain live.
    pub(crate) unsafe fn flush_sounds(cx: &mut Cx<'_>) {
        let mut i = 0;
        while Kernel::current().sounds.count > 0 {
            let sound = Kernel::current().sounds.sounds[i];
            let object = Kernel::current().sounds.objects[i];
            let pos = position(cx, object);
            play_sound(cx, sound, pos, object, GAME_SOUND_PRIORITY as u8);
            i += 1;
            Kernel::current().sounds.count -= 1;
        }
    }

    /// # Safety
    /// Called on the game thread before releasing an element's storage.
    pub(crate) unsafe fn remove_sounds(object: Option<ElementId>) {
        for i in FIRST_SFX_CHANNEL..=LAST_SFX_CHANNEL {
            if decode_object(get_positional_object(i)) == object {
                set_positional_object(i, 0);
            }
        }
        Kernel::current().sounds.remove_object(object);
    }
}

#[cfg(not(test))]
pub use crate::battle::context_access::sound_legacy::*;
#[cfg(not(test))]
pub use engine::{NotPositional, PlayMenuSound};

fn encode_object(object: Option<ElementId>) -> usize {
    object.map_or(0, |id| {
        (((u64::from(id.gen) << 8) | u64::from(id.slot)) + 1) as usize
    })
}
fn decode_object(token: usize) -> Option<ElementId> {
    token.checked_sub(1).map(|value| ElementId {
        slot: value as u8,
        gen: (value >> 8) as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_tokens_keep_generation_and_slot_without_pointer_identity() {
        assert_eq!(decode_object(encode_object(None)), None);
        for id in [
            ElementId { slot: 0, gen: 0 },
            ElementId {
                slot: 255,
                gen: u32::MAX,
            },
        ] {
            assert_eq!(decode_object(encode_object(Some(id))), Some(id));
            assert_ne!(encode_object(Some(id)), 0);
        }
    }

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
            queue.push(
                i as SOUND,
                Some(ElementId {
                    slot: i as u8,
                    gen: 0,
                }),
            );
        }
        assert_eq!(queue.count, 8);
        assert_eq!(queue.sounds[7] as usize, 8);
        queue.push(usize::MAX as SOUND, None);
        queue.push(12 as SOUND, None);
        assert_eq!(queue.count, 8);
        assert!(queue.sounds.iter().all(|p| p.is_null()));
        assert!(queue.objects.iter().all(Option::is_none));
    }
    #[test]
    fn freeing_a_source_detaches_only_its_generation_without_removing_sounds() {
        let old = ElementId { slot: 2, gen: 7 };
        let replacement = ElementId { slot: 2, gen: 8 };
        let mut queue = SoundQueue::new();
        queue.push(1 as SOUND, Some(old));
        queue.push(2 as SOUND, Some(replacement));
        queue.push(3 as SOUND, None);
        queue.remove_object(Some(old));
        assert_eq!(queue.count, 3);
        assert_eq!(queue.sounds.map(|s| s as usize), [1, 2, 3, 0, 0, 0, 0, 0]);
        assert_eq!(&queue.objects[..3], &[None, Some(replacement), None]);
    }
}
