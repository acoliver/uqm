//! Battle sound queue, effect-channel reuse and positional-object tracking.

#[cfg(not(test))]
use crate::engine::battle::Cx;
use crate::engine::battle::ElementId;

use crate::engine::battle::kernel::Channels;
#[cfg(test)]
use crate::engine::battle::kernel::SoundQueue;

#[cfg(not(test))]
use crate::sound::heart_ffi::{
    get_positional_object, set_positional_object, update_sound_position,
};

#[cfg(not(test))]
pub(crate) mod engine {
    pub use super::*;
    use crate::ships::native::abi::*;

    /// # Safety
    /// Called while the game-thread battle context is published.
    pub(crate) fn position(
        cx: &mut Cx<'_>,
        object: Option<ElementId>,
    ) -> crate::sound::types::SoundPosition {
        let Some(id) = object.filter(|id| cx.elements().contains(*id)) else {
            return crate::sound::types::SoundPosition::non_positional();
        };
        let point = cx.sound_point(id);
        let (width, height) = cx.screen_dimensions();
        crate::sound::types::SoundPosition {
            positional: true,
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
                if pos.positional {
                    update_sound_position(i, pos);
                } else {
                    set_positional_object(i, 0);
                }
            }
        }
    }

    pub(crate) fn play_sound(
        world: &crate::engine::World,
        channels: &mut Channels,
        sound: &crate::sound::owned_bank::SoundClip,
        pos: crate::sound::types::SoundPosition,
        object: Option<ElementId>,
        priority: u8,
    ) {
        let channel = channels.select(sound.identity(), |c| {
            crate::sound::heart_ffi::channel_playing(c as u32 + MIN_FX_CHANNEL) != 0
        });
        world.play_sound_effect(
            sound,
            channel + MIN_FX_CHANNEL as usize,
            pos,
            encode_object(object),
            priority,
        );
    }

    pub(crate) fn flush_sounds(cx: &mut Cx<'_>) {
        let mut i = 0;
        while cx.kernel.sounds.count > 0 {
            let sound = cx.kernel.sounds.sounds[i].take();
            let object = cx.kernel.sounds.objects[i];
            let pos = position(cx, object);
            if let Some(sound) = sound {
                play_sound(
                    cx.world,
                    &mut cx.kernel.channels.borrow_mut(),
                    &sound,
                    pos,
                    object,
                    GAME_SOUND_PRIORITY as u8,
                );
            }
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
pub use engine::NON_POSITIONAL;

fn encode_object(object: Option<ElementId>) -> usize {
    object.map_or(0, ElementId::sound_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_bank() -> crate::sound::owned_bank::LoadedSoundBank {
        crate::sound::owned_bank::LoadedSoundBank::from_samples(
            (0..12)
                .map(|_| crate::sound::types::SoundSample::new())
                .collect(),
        )
    }

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
            assert_eq!(channels.select(i + 1, |_| true), i);
        }
        assert_eq!(
            channels.select(2, |_| panic!("reused channel must not query playback")),
            1
        );
        assert_eq!(channels.select(5, |_| true), 0);
        assert_eq!(channels.select(6, |i| i != 2), 2);
        assert_eq!(channels.lru, [3, 1, 0, 2]);
    }

    #[test]
    fn queue_preserves_order_caps_at_eight_and_silence_suppresses_sounds() {
        let mut queue = SoundQueue::new();
        let bank = fixture_bank();
        for i in 1..=10 {
            queue.push(Some(bank.clip(i - 1)), Some(ElementId::fixture(i as u8, 0)));
        }
        assert_eq!(queue.count, 8);
        assert_eq!(
            queue.sounds[7].as_ref().unwrap().identity(),
            bank.clip(7).identity()
        );
        queue.silence();
        queue.push(Some(bank.clip(11)), None);
        assert_eq!(queue.count, 8);
        assert!(queue.sounds.iter().all(Option::is_none));
        assert!(queue.objects.iter().all(Option::is_none));
    }
    #[test]
    fn queued_clip_retains_its_allocation_after_admission_owner_drops() {
        let bank = crate::sound::owned_bank::LoadedSoundBank::from_samples(vec![
            crate::sound::types::SoundSample::new(),
        ]);
        let clip = bank.clip(0);
        let identity = clip.identity();
        let mut queue = SoundQueue::new();
        queue.push(Some(clip), None);
        drop(bank);
        assert_eq!(queue.sounds[0].as_ref().unwrap().identity(), identity);
    }

    #[test]
    fn freeing_a_source_detaches_only_its_generation_without_removing_sounds() {
        let old = ElementId::fixture(2, 7);
        let replacement = ElementId::fixture(2, 8);
        let mut queue = SoundQueue::new();
        let bank = fixture_bank();
        queue.push(Some(bank.clip(0)), Some(old));
        queue.push(Some(bank.clip(1)), Some(replacement));
        queue.push(Some(bank.clip(2)), None);
        queue.remove_object(Some(old));
        assert_eq!(queue.count, 3);
        assert_eq!(
            queue.sounds[..3]
                .iter()
                .map(|s| s.as_ref().unwrap().identity())
                .collect::<Vec<_>>(),
            (0..3).map(|i| bank.clip(i).identity()).collect::<Vec<_>>()
        );
        assert_eq!(&queue.objects[..3], &[None, Some(replacement), None]);
    }
}
