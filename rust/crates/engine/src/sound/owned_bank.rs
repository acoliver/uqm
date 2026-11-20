//! Sound entries retaining the detached bank that owns their decoded samples.

use super::{
    sfx,
    types::{SoundPosition, SoundSample},
};
use crate::strings::owned_table::EntryRef;
use std::{ffi::CStr, rc::Rc};

#[derive(Debug, Clone)]
pub struct LoadedSoundBank(Rc<Bank>);

pub(super) struct Bank {
    samples: Box<[SoundSample]>,
}

impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bank")
            .field("samples", &self.samples.len())
            .finish()
    }
}

impl Drop for Bank {
    fn drop(&mut self) {
        for sample in &mut self.samples {
            let _ = super::stream::destroy_sound_sample(sample);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SoundClip {
    bank: LoadedSoundBank,
    index: usize,
}

impl LoadedSoundBank {
    pub fn from_samples(samples: Vec<SoundSample>) -> Self {
        assert!(
            !samples.is_empty() && samples.len() <= i32::MAX as usize,
            "sound bank requires a bounded nonempty sample list"
        );
        Self(Rc::new(Bank {
            samples: samples.into_boxed_slice(),
        }))
    }

    pub fn load(resource: &CStr) -> Option<Self> {
        let path = crate::resource::path_of(resource, "SNDRES")?;
        super::heart_ffi::load_sound_bank(path.to_str().ok()?)
    }

    pub fn len(&self) -> usize {
        self.0.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.samples.is_empty()
    }

    /// This bank with sound `index` current, as game records name it.
    pub fn entry_ref(&self, index: u16) -> EntryRef {
        EntryRef::new(Rc::as_ptr(&self.0) as usize, index)
    }

    pub fn clip(&self, index: u16) -> SoundClip {
        SoundClip {
            bank: self.clone(),
            index: usize::from(index) % self.0.samples.len(),
        }
    }
}

impl SoundClip {
    /// The address of this clip's sample: equal for clips of one sample.
    pub fn identity(&self) -> usize {
        std::ptr::from_ref(&self.bank.0.samples[self.index]) as usize
    }

    pub fn play(&self, channel: usize, position: SoundPosition, object: usize, priority: u8) {
        let sample = &self.bank.0.samples[self.index];
        let _ = sfx::play_sample(channel, sample, position, object, i32::from(priority));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_retain_their_bank_and_keep_wrapping_entry_selection() {
        let bank = LoadedSoundBank::from_samples(vec![SoundSample::new()]);
        let weak = Rc::downgrade(&bank.0);
        let clip = bank.clip(0);
        assert_eq!(clip.identity(), bank.clip(u16::MAX).identity());
        let identity = clip.identity();
        drop(bank);
        assert!(weak.upgrade().is_some());
        assert_eq!(clip.identity(), identity);
        drop(clip);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn entry_refs_name_the_bank_and_the_current_sound() {
        let bank = LoadedSoundBank::from_samples(vec![SoundSample::new(), SoundSample::new()]);
        let other = LoadedSoundBank::from_samples(vec![SoundSample::new()]);
        assert_eq!(bank.len(), 2);
        assert_eq!(bank.entry_ref(1), bank.entry_ref(0).with_index(1));
        assert_ne!(bank.entry_ref(0).table(), other.entry_ref(0).table());
        assert_ne!(bank.clip(0).identity(), bank.clip(1).identity());
    }
}
