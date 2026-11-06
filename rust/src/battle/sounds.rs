//! Battle sound queue, effect-channel reuse and positional-object tracking.

#[cfg(not(test))]
use crate::engine::battle::Cx;
use crate::engine::battle::ElementId;
#[cfg(not(test))]
use crate::engine::Kernel as KernelOwner;
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
pub(crate) mod engine {
    pub use super::*;
    use crate::ships::native::abi::*;

    /// # Safety
    /// Called inside a native handoff with a live sound and retained channel state.
    pub(crate) unsafe fn play_sound(
        channels: &mut Channels,
        sound: SOUND,
        pos: SoundPosition,
        object: Option<ElementId>,
        priority: u8,
    ) {
        if sound.is_null() {
            return;
        }
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
        if let Some(bank) = KernelOwner::current().read(|owner| owner.menu_sound_bank()) {
            PlaySoundEffect(
                SetAbsStringTableIndex(bank.as_raw().cast(), sound as u16),
                0,
                NON_POSITIONAL,
                ptr::null_mut(),
                GAME_SOUND_PRIORITY as u8,
            );
        }
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

    pub const NON_POSITIONAL: SoundPosition = SoundPosition {
        positional: 0,
        x: 0,
        y: 0,
    };

    pub(crate) fn update_positions(cx: &mut Cx<'_>) {
        for i in FIRST_SFX_CHANNEL..=LAST_SFX_CHANNEL {
            if crate::sound::heart_ffi::channel_playing(i) == 0 {
                continue;
            }
            if let Some(object) = cx.elements().sound_source(get_positional_object(i)) {
                let pos = position(cx, Some(object));
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
        while cx.kernel.sounds.count > 0 {
            let sound = cx.kernel.sounds.sounds[i];
            let object = cx.kernel.sounds.objects[i];
            let pos = position(cx, object);
            let channels = cx.kernel.channels.clone();
            cx.with_legacy(|| {
                play_sound(
                    &mut channels.borrow_mut(),
                    sound,
                    pos,
                    object,
                    GAME_SOUND_PRIORITY as u8,
                )
            });
            i += 1;
            cx.kernel.sounds.count -= 1;
        }
    }

    pub(crate) fn remove_sounds(kernel: &mut crate::engine::Kernel, object: Option<ElementId>) {
        for i in FIRST_SFX_CHANNEL..=LAST_SFX_CHANNEL {
            if kernel.element_sound_source(get_positional_object(i)) == object {
                set_positional_object(i, 0);
            }
        }
        kernel.sounds.remove_object(object);
    }
}

#[cfg(not(test))]
pub use engine::{PlayMenuSound, NON_POSITIONAL};

fn encode_object(object: Option<ElementId>) -> usize {
    object.map_or(0, ElementId::sound_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_tokens_keep_generation_and_owner_without_pointer_identity() {
        let mut elements = crate::engine::battle::ElementTable::with_capacity(1);
        assert_eq!(elements.sound_source(encode_object(None)), None);
        let old = elements.alloc().unwrap();
        let token = encode_object(Some(old));
        assert_ne!(token, 0);
        assert_eq!(elements.sound_source(token), Some(old));
        elements.free(old);
        let replacement = elements.alloc().unwrap();
        assert_eq!(elements.sound_source(token), None);
        assert_ne!(encode_object(Some(replacement)), token);
        let mut other = crate::engine::battle::ElementTable::with_capacity(1);
        other.alloc().unwrap();
        assert_eq!(other.sound_source(token), None);
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
            queue.push(i as SOUND, Some(ElementId::fixture(i as u8, 0)));
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
        let old = ElementId::fixture(2, 7);
        let replacement = ElementId::fixture(2, 8);
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
