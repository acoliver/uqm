// @plan PLAN-20260225-AUDIO-HEART.P15
// @requirement REQ-FILEINST-LOAD-01..07
#![allow(dead_code, unused_imports, unused_variables)]

//! File instance loading — resource loading with concurrent-load guards.
//!
//! Wraps `music::get_music_data` and `sfx::get_sound_bank_data` with
//! a `FileLoadGuard` RAII type that ensures `cur_resfile_name` is cleared
//! on all exit paths (success, error, panic).

use parking_lot::Mutex;

use super::music;
use super::sfx;
use super::types::*;

// =============================================================================
// State
// =============================================================================

/// File loading state — tracks whether a resource is currently being loaded.
struct FileInstState {
    /// Name of the resource file currently being loaded (None if idle).
    cur_resfile_name: Option<String>,
}

impl FileInstState {
    fn new() -> Self {
        FileInstState {
            cur_resfile_name: None,
        }
    }
}

static FILE_STATE: std::sync::LazyLock<Mutex<FileInstState>> =
    std::sync::LazyLock::new(|| Mutex::new(FileInstState::new()));

// =============================================================================
// RAII Guard
// =============================================================================

/// RAII guard that clears `cur_resfile_name` when dropped.
struct FileLoadGuard;

impl Drop for FileLoadGuard {
    fn drop(&mut self) {
        let mut state = FILE_STATE.lock();
        state.cur_resfile_name = None;
    }
}

fn acquire_load_guard(filename: &str) -> AudioResult<FileLoadGuard> {
    let mut state = FILE_STATE.lock();
    if state.cur_resfile_name.is_some() {
        return Err(AudioError::AlreadyInitialized);
    }
    state.cur_resfile_name = Some(filename.to_string());
    Ok(FileLoadGuard)
}

// =============================================================================
// Public API
// =============================================================================

/// Load a sound bank from a resource file.
pub fn load_sound_file(filename: &str) -> AudioResult<SoundBank> {
    todo!("P17: load_sound_file")
}

/// Load music from a resource file.
pub fn load_music_file(filename: &str) -> AudioResult<MusicRef> {
    todo!("P17: load_music_file")
}

/// Destroy/release a sound bank.
pub fn destroy_sound(bank: SoundBank) -> AudioResult<()> {
    todo!("P17: destroy_sound")
}

/// Destroy/release a music reference.
pub fn destroy_music(music_ref: MusicRef) -> AudioResult<()> {
    todo!("P17: destroy_music")
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_inst_state_new() {
        let state = FileInstState::new();
        assert!(state.cur_resfile_name.is_none());
    }

    /// Serialization lock for tests that touch FILE_STATE.
    static TEST_LOCK: std::sync::LazyLock<std::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| std::sync::Mutex::new(()));

    #[test]
    fn test_acquire_load_guard() {
        let _serial = TEST_LOCK.lock().unwrap();
        // Ensure clean state
        FILE_STATE.lock().cur_resfile_name = None;

        let guard = acquire_load_guard("test.wav");
        assert!(guard.is_ok());
        {
            let state = FILE_STATE.lock();
            assert_eq!(state.cur_resfile_name.as_deref(), Some("test.wav"));
        }
        drop(guard);
        {
            let state = FILE_STATE.lock();
            assert!(state.cur_resfile_name.is_none());
        }
    }

    #[test]
    fn test_concurrent_load_rejected() {
        let _serial = TEST_LOCK.lock().unwrap();
        FILE_STATE.lock().cur_resfile_name = None;

        let _guard = acquire_load_guard("first.wav").unwrap();
        let result = acquire_load_guard("second.wav");
        assert!(result.is_err());
    }
}
