//! V3b capture requests. The coordinator owns their one-shot lifecycle.

use std::path::PathBuf;

#[derive(Debug)]
pub(super) struct Request {
    pub source: PathBuf,
    pub output: PathBuf,
    pub loaded_activity: Option<u16>,
    pub loaded_tick: u32,
    pub captured: bool,
}

impl Request {
    pub fn from_environment() -> Option<Self> {
        let source = std::env::var_os("UQM_SAVE_CAPTURE_SOURCE")?;
        let output = std::env::var_os("UQM_SAVE_CAPTURE_OUT")
            .unwrap_or_else(|| panic!("UQM_SAVE_CAPTURE_SOURCE requires UQM_SAVE_CAPTURE_OUT"));
        assert!(
            crate::time::source::is_replay(),
            "save capture requires --replay"
        );
        let source = PathBuf::from(source);
        let output = PathBuf::from(output);
        assert!(source.is_absolute(), "save capture source must be absolute");
        assert!(output.is_absolute(), "save capture output must be absolute");
        Some(Self {
            source,
            output,
            loaded_activity: None,
            loaded_tick: 0,
            captured: false,
        })
    }

    pub fn admits_capture(&self, activity: u16, button_wait: bool) -> bool {
        self.loaded_activity.is_some() && !self.captured && !button_wait && activity & 0x1000 == 0
    }
}

#[cfg(not(test))]
impl Request {
    /// The caller is the post-game-initialization hook on the game thread.
    pub fn load(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(self.loaded_activity.is_none(), "save capture loaded twice");
        anyhow::ensure!(
            self.source.is_file(),
            "missing save {}",
            self.source.display()
        );
        // SAFETY: InitGameStructures and InitGameClock completed; the runner staged
        // the source in slot zero of this subprocess's isolated save directory.
        let loaded = unsafe { crate::savegame::live::load::LoadGame(0, std::ptr::null_mut()) };
        anyhow::ensure!(loaded != 0, "LoadGame failed for {}", self.source.display());
        let activity = crate::mainloop::ffi::get_next_activity().0;
        self.loaded_activity = Some(activity);
        self.loaded_tick = crate::time::source::now_ms();
        // Preserve NextActivity for the production main-loop load resolution.
        let load = crate::mainloop::types::ActivityValue(0x1000);
        crate::mainloop::ffi::set_current_activity(load);
        crate::mainloop::ffi::set_last_activity(load);
        Ok(())
    }

    /// Capture at the first non-button mode admission, before consuming input.
    pub fn capture(&mut self, input_admission: u64) -> anyhow::Result<()> {
        let activity = crate::mainloop::ffi::get_current_activity().0;
        anyhow::ensure!(
            self.admits_capture(activity, false),
            "invalid save capture boundary"
        );
        let tick = crate::time::source::now_ms();
        // SAFETY: the main loop has resolved CHECK_LOAD to NextActivity and
        // initialized the mode; its first admitted input has not been consumed.
        let save = unsafe { crate::savegame::live::save::capture(b"runtime-capture")? };
        anyhow::ensure!(
            tick == crate::time::source::now_ms(),
            "capture advanced replay clock"
        );
        let bytes = crate::savegame::encode_save(&save)?;
        std::fs::write(&self.output, bytes)?;
        let metadata = serde_json::json!({
            "schema": "uqm-save-runtime-capture-v1",
            "source_activity": self.loaded_activity,
            "captured_activity": save.global.current_activity,
            "loaded_tick": self.loaded_tick,
            "capture_tick": tick,
            "input_admission": true,
            "admitted_input_count": input_admission,
            "clock_unchanged_during_capture": true,
        });
        let sidecar = PathBuf::from(format!("{}.json", self.output.display()));
        std::fs::write(sidecar, format!("{metadata}\n"))?;
        self.captured = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loaded_non_button_admission_captures_once() {
        let mut request = Request {
            source: PathBuf::from("source"),
            output: PathBuf::from("output"),
            loaded_activity: None,
            loaded_tick: 0,
            captured: false,
        };
        assert!(
            !request.admits_capture(4, false),
            "main menu cannot capture"
        );
        request.loaded_activity = Some(0x0804);
        assert!(
            !request.admits_capture(0x1000, false),
            "load picker cannot capture"
        );
        assert!(
            !request.admits_capture(0x0804, true),
            "initialization waits cannot capture"
        );
        assert!(request.admits_capture(0x0804, false));
        assert_eq!(request.source, PathBuf::from("source"));
        assert_eq!(request.output, PathBuf::from("output"));
        assert_eq!(
            request.loaded_tick, 0,
            "boundary admission must not mutate capture timing"
        );
        request.captured = true;
        assert!(!request.admits_capture(0x0804, false));
    }
}
