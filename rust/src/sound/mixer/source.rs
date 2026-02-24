// source.rs - Audio source management

//! Audio source management for the mixer.
//!
//! Sources represent playback instances that can have buffers queued to them.
//! They support playing, pausing, stopping, and gain control.

use crate::sound::mixer::buffer::mixer_get_buffer;
use crate::sound::mixer::types::*;
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio source for playback
#[derive(Debug)]
pub struct MixerSource {
    /// Magic number for validation (MIXS)
    pub magic: u32,
    /// Whether the source is locked
    pub locked: bool,
    /// Current playback state
    pub state: u32,
    /// Whether playback should loop
    pub looping: bool,
    /// Volume gain (adjusted by MIX_GAIN_ADJ)
    pub gain: f32,
    /// Number of queued buffers
    pub queued_count: u32,
    /// Number of processed buffers
    pub processed_count: u32,
    /// Index of first queued buffer
    pub first_queued: Option<usize>,
    /// Index of next buffer to play
    pub next_queued: Option<usize>,
    /// Index of previously played buffer
    pub prev_queued: Option<usize>,
    /// Index of last queued buffer
    pub last_queued: Option<usize>,
    /// Current position in buffer (bytes)
    pub pos: u32,
    /// Fractional part of position
    pub count: u32,
    /// Cached sample for mono->stereo duplication
    pub sample_cache: f32,
}

impl MixerSource {
    /// Create a new uninitialized source
    pub fn new() -> Self {
        MixerSource {
            magic: MIXER_SRC_MAGIC,
            locked: false,
            state: SourceState::Initial as u32,
            looping: false,
            gain: MIX_GAIN_ADJ,
            queued_count: 0,
            processed_count: 0,
            first_queued: None,
            next_queued: None,
            prev_queued: None,
            last_queued: None,
            pos: 0,
            count: 0,
            sample_cache: 0.0,
        }
    }

    /// Validate that this is a valid source
    pub fn is_valid(&self) -> bool {
        self.magic == MIXER_SRC_MAGIC
    }

    /// Check if source is in a valid state for operations
    pub fn check_state(&self) -> Result<(), MixerError> {
        if self.magic != MIXER_SRC_MAGIC {
            return Err(MixerError::InvalidName);
        }
        if self.locked {
            return Err(MixerError::InvalidOperation);
        }
        Ok(())
    }
}

impl Default for MixerSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Global source storage
static SOURCE_POOL: Mutex<Vec<Option<Arc<Mutex<MixerSource>>>>> = Mutex::new(Vec::new());

/// Generate new source objects
///
/// Returns handles to newly created sources.
pub fn mixer_gen_sources(n: u32) -> Result<Vec<usize>, MixerError> {
    if n == 0 {
        return Ok(Vec::new());
    }

    let mut pool = SOURCE_POOL.lock();
    let mut handles = Vec::with_capacity(n as usize);

    for _ in 0..n {
        let source = Arc::new(Mutex::new(MixerSource::new()));
        let index = pool.len();
        pool.push(Some(source));
        handles.push(index);
    }

    Ok(handles)
}

/// Delete source objects
///
/// Removes sources from the pool. All sources must be valid and stopped.
pub fn mixer_delete_sources(handles: &[usize]) -> Result<(), MixerError> {
    if handles.is_empty() {
        return Ok(());
    }

    let mut pool = SOURCE_POOL.lock();

    // First pass: validate all sources can be deleted
    for &handle in handles {
        if handle >= pool.len() {
            return Err(MixerError::InvalidName);
        }

        let source = pool.get(handle).and_then(|s| s.as_ref());
        match source {
            None => return Err(MixerError::InvalidName),
            Some(src) => {
                let src_guard = src.lock();
                if !src_guard.is_valid() {
                    return Err(MixerError::InvalidName);
                }
            }
        }
    }

    // Second pass: delete all sources
    for &handle in handles {
        pool[handle] = None;
    }

    Ok(())
}

/// Check if a handle is a valid source
pub fn mixer_is_source(handle: usize) -> bool {
    let pool = SOURCE_POOL.lock();

    if handle >= pool.len() {
        return false;
    }

    match &pool[handle] {
        None => false,
        Some(src) => src.lock().is_valid(),
    }
}

/// Get a reference to a source by handle
pub fn mixer_get_source(handle: usize) -> Option<Arc<Mutex<MixerSource>>> {
    let pool = SOURCE_POOL.lock();

    if handle >= pool.len() {
        return None;
    }

    pool[handle].as_ref().map(|s| Arc::clone(s))
}

/// Get all sources with their handles (for mixing)
pub fn get_all_sources() -> Vec<(usize, Arc<Mutex<MixerSource>>)> {
    let pool = SOURCE_POOL.lock();
    let mut result = Vec::new();

    for (i, slot) in pool.iter().enumerate() {
        if let Some(source) = slot {
            result.push((i, Arc::clone(source)));
        }
    }

    result
}

/// Set an integer property on a source
pub fn mixer_source_i(handle: usize, prop: SourceProp, value: i32) -> Result<(), MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    src.check_state()?;

    match prop {
        SourceProp::Looping => {
            src.looping = value != 0;
            Ok(())
        }
        SourceProp::SourceState => {
            if value == SourceState::Initial as i32 {
                // Rewind to initial state
                src.state = SourceState::Initial as u32;
                src.pos = 0;
                src.count = 0;
                src.processed_count = 0;
                src.next_queued = src.first_queued;
                src.prev_queued = None;
                Ok(())
            } else {
                // Other state changes should use specific methods
                Err(MixerError::InvalidEnum)
            }
        }
        _ => Err(MixerError::InvalidEnum),
    }
}

/// Set a float property on a source
pub fn mixer_source_f(handle: usize, prop: SourceProp, value: f32) -> Result<(), MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    src.check_state()?;

    match prop {
        SourceProp::Gain => {
            src.gain = value * MIX_GAIN_ADJ;
            Ok(())
        }
        _ => Err(MixerError::InvalidEnum),
    }
}

/// Get an integer property from a source
pub fn mixer_get_source_i(handle: usize, prop: SourceProp) -> Result<i32, MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let src = source.lock();

    src.check_state()?;

    match prop {
        SourceProp::Looping => Ok(src.looping as i32),
        SourceProp::SourceState => Ok(src.state as i32),
        SourceProp::BuffersQueued => Ok(src.queued_count as i32),
        SourceProp::BuffersProcessed => Ok(src.processed_count as i32),
        _ => Err(MixerError::InvalidEnum),
    }
}

/// Get a float property from a source
pub fn mixer_get_source_f(handle: usize, prop: SourceProp) -> Result<f32, MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let src = source.lock();

    src.check_state()?;

    match prop {
        SourceProp::Gain => Ok(src.gain / MIX_GAIN_ADJ),
        _ => Err(MixerError::InvalidEnum),
    }
}

/// Start playback on a source
pub fn mixer_source_play(handle: usize) -> Result<(), MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    src.check_state()?;

    // Activate source if not already playing
    if src.state < (SourceState::Playing as u32) {
        if src.first_queued.is_some() && src.next_queued.is_none() {
            // Rewind if at end
            src.pos = 0;
            src.count = 0;
            src.processed_count = 0;
            src.next_queued = src.first_queued;
            src.prev_queued = None;
        }
    }

    src.state = SourceState::Playing as u32;

    Ok(())
}

/// Pause playback on a source
pub fn mixer_source_pause(handle: usize) -> Result<(), MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    src.check_state()?;

    src.state = SourceState::Paused as u32;
    Ok(())
}

/// Stop playback on a source
pub fn mixer_source_stop(handle: usize) -> Result<(), MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    src.check_state()?;

    // Unqueue all buffers
    src.first_queued = None;
    src.next_queued = None;
    src.prev_queued = None;
    src.last_queued = None;
    src.queued_count = 0;
    src.processed_count = 0;
    src.pos = 0;
    src.count = 0;

    src.state = SourceState::Stopped as u32;
    Ok(())
}

/// Rewind a source to the beginning
pub fn mixer_source_rewind(handle: usize) -> Result<(), MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    src.check_state()?;

    src.pos = 0;
    src.count = 0;
    src.processed_count = 0;
    src.next_queued = src.first_queued;
    src.prev_queued = None;
    src.state = SourceState::Initial as u32;
    Ok(())
}

/// Queue buffers to a source
pub fn mixer_source_queue_buffers(
    handle: usize,
    buffer_handles: &[usize],
) -> Result<(), MixerError> {
    if buffer_handles.is_empty() {
        return Ok(());
    }

    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;

    // First pass: validate all buffers
    for &buf_handle in buffer_handles {
        let buffer = mixer_get_buffer(buf_handle).ok_or(MixerError::InvalidName)?;
        let buf = buffer.lock();

        if !buf.is_valid() {
            return Err(MixerError::InvalidName);
        }
        if buf.locked {
            return Err(MixerError::InvalidOperation);
        }
        if buf.state != (BufferState::Filled as u32) {
            return Err(MixerError::InvalidOperation);
        }
    }

    // Second pass: queue all buffers
    let mut src = source.lock();

    for &buf_handle in buffer_handles {
        let buffer = mixer_get_buffer(buf_handle).unwrap();
        let mut buf = buffer.lock();

        // Mark buffer as queued
        buf.state = BufferState::Queued as u32;

        // Add to source's queue
        if let Some(last) = src.last_queued {
            // Link previous last buffer to this one
            let last_buf = mixer_get_buffer(last).unwrap();
            let mut last_buf = last_buf.lock();
            last_buf.next = Some(buf_handle);
        }

        src.last_queued = Some(buf_handle);

        if src.first_queued.is_none() {
            src.first_queued = Some(buf_handle);
            src.next_queued = Some(buf_handle);
            src.prev_queued = None;
        }

        src.queued_count += 1;
    }

    Ok(())
}

/// Unqueue processed buffers from a source
pub fn mixer_source_unqueue_buffers(handle: usize, n: u32) -> Result<Vec<usize>, MixerError> {
    let source = mixer_get_source(handle).ok_or(MixerError::InvalidName)?;
    let mut src = source.lock();

    if n > src.queued_count {
        return Err(MixerError::InvalidOperation);
    }

    let mut unqueued = Vec::with_capacity(n as usize);

    for _ in 0..n {
        if src.first_queued.is_none() {
            break;
        }

        let buf_handle = src.first_queued.unwrap();

        // Check if buffer is still playing
        let buffer = mixer_get_buffer(buf_handle).unwrap();
        let buf = buffer.lock();

        if buf.state == (BufferState::Playing as u32) {
            return Err(MixerError::InvalidOperation);
        }

        drop(buf); // Release lock before modifying

        // Remove buffer from queue
        if src.next_queued == Some(buf_handle) {
            src.next_queued = src
                .next_queued
                .and_then(|h| mixer_get_buffer(h).and_then(|b| b.lock().next));
        }
        if src.prev_queued == Some(buf_handle) {
            src.prev_queued = None;
        }
        if src.last_queued == Some(buf_handle) {
            src.last_queued = None;
        }

        src.first_queued = src
            .first_queued
            .and_then(|h| mixer_get_buffer(h).and_then(|b| b.lock().next));
        src.queued_count -= 1;

        // Mark buffer as filled
        let buffer = mixer_get_buffer(buf_handle).unwrap();
        let mut buf = buffer.lock();

        if buf.state == (BufferState::Processed as u32) {
            src.processed_count -= 1;
        }

        buf.state = BufferState::Filled as u32;
        buf.next = None;

        unqueued.push(buf_handle);
    }

    Ok(unqueued)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::mixer::buffer::{mixer_buffer_data, mixer_gen_buffers};

    #[test]
    fn test_gen_sources() {
        let handles = mixer_gen_sources(3).unwrap();
        assert_eq!(handles.len(), 3);
        assert!(mixer_is_source(handles[0]));
        assert!(mixer_is_source(handles[1]));
        assert!(mixer_is_source(handles[2]));
    }

    #[test]
    fn test_delete_sources() {
        let handles = mixer_gen_sources(2).unwrap();
        assert!(mixer_is_source(handles[0]));
        assert!(mixer_is_source(handles[1]));

        mixer_delete_sources(&handles).unwrap();
        assert!(!mixer_is_source(handles[0]));
        assert!(!mixer_is_source(handles[1]));
    }

    #[test]
    fn test_is_source() {
        let handles = mixer_gen_sources(1).unwrap();
        assert!(mixer_is_source(handles[0]));
        assert!(!mixer_is_source(999));
    }

    #[test]
    fn test_source_new() {
        let src = MixerSource::new();
        assert_eq!(src.magic, MIXER_SRC_MAGIC);
        assert_eq!(src.state, SourceState::Initial as u32);
        assert!(!src.looping);
        assert_eq!(src.gain, MIX_GAIN_ADJ);
        assert_eq!(src.queued_count, 0);
    }

    #[test]
    fn test_source_properties() {
        let handles = mixer_gen_sources(1).unwrap();

        // Test looping
        mixer_source_i(handles[0], SourceProp::Looping, 1).unwrap();
        assert_eq!(
            mixer_get_source_i(handles[0], SourceProp::Looping).unwrap(),
            1
        );

        mixer_source_i(handles[0], SourceProp::Looping, 0).unwrap();
        assert_eq!(
            mixer_get_source_i(handles[0], SourceProp::Looping).unwrap(),
            0
        );

        // Test gain
        mixer_source_f(handles[0], SourceProp::Gain, 0.5).unwrap();
        let gain = mixer_get_source_f(handles[0], SourceProp::Gain).unwrap();
        assert!((gain - 0.5).abs() < 0.001);

        // Test state
        let state = mixer_get_source_i(handles[0], SourceProp::SourceState).unwrap();
        assert_eq!(state, SourceState::Initial as i32);
    }

    #[test]
    fn test_source_play() {
        let handles = mixer_gen_sources(1).unwrap();
        mixer_source_play(handles[0]).unwrap();

        let state = mixer_get_source_i(handles[0], SourceProp::SourceState).unwrap();
        assert_eq!(state, SourceState::Playing as i32);
    }

    #[test]
    fn test_source_pause() {
        let handles = mixer_gen_sources(1).unwrap();
        mixer_source_play(handles[0]).unwrap();
        mixer_source_pause(handles[0]).unwrap();

        let state = mixer_get_source_i(handles[0], SourceProp::SourceState).unwrap();
        assert_eq!(state, SourceState::Paused as i32);
    }

    #[test]
    fn test_source_stop() {
        let handles = mixer_gen_sources(1).unwrap();
        mixer_source_play(handles[0]).unwrap();
        mixer_source_stop(handles[0]).unwrap();

        let state = mixer_get_source_i(handles[0], SourceProp::SourceState).unwrap();
        assert_eq!(state, SourceState::Stopped as i32);
    }

    #[test]
    fn test_source_rewind() {
        let handles = mixer_gen_sources(1).unwrap();
        mixer_source_play(handles[0]).unwrap();
        mixer_source_rewind(handles[0]).unwrap();

        let state = mixer_get_source_i(handles[0], SourceProp::SourceState).unwrap();
        assert_eq!(state, SourceState::Initial as i32);
    }

    #[test]
    fn test_source_queue_buffers() {
        let src_handles = mixer_gen_sources(1).unwrap();
        let buf_handles = mixer_gen_buffers(2).unwrap();
        let data = vec![0u8; 100];

        mixer_buffer_data(
            buf_handles[0],
            MixerFormat::Mono8 as u32,
            &data,
            44100,
            44100,
            MixerFormat::Mono8,
        )
        .unwrap();

        mixer_buffer_data(
            buf_handles[1],
            MixerFormat::Mono8 as u32,
            &data,
            44100,
            44100,
            MixerFormat::Mono8,
        )
        .unwrap();

        mixer_source_queue_buffers(src_handles[0], &buf_handles).unwrap();

        let queued = mixer_get_source_i(src_handles[0], SourceProp::BuffersQueued).unwrap();
        assert_eq!(queued, 2);
    }

    #[test]
    fn test_source_invalid_handle() {
        let result = mixer_source_play(999);
        assert_eq!(result, Err(MixerError::InvalidName));
    }

    #[test]
    fn test_source_invalid_property() {
        let handles = mixer_gen_sources(1).unwrap();
        let result = mixer_source_i(handles[0], SourceProp::Position, 0);
        assert_eq!(result, Err(MixerError::InvalidEnum));
    }

    #[test]
    fn test_delete_invalid_source() {
        let result = mixer_delete_sources(&[999]);
        assert_eq!(result, Err(MixerError::InvalidName));
    }
}
