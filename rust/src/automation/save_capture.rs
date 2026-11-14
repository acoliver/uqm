//! V3b capture requests. The coordinator owns their one-shot lifecycle.

use std::path::PathBuf;

#[derive(Debug)]
pub(super) struct Request {
    pub source: PathBuf,
    pub output: PathBuf,
    pub loaded_activity: Option<u16>,
    pub load_admitted: bool,
    pub loaded_tick: u32,
    pub captured: bool,
    pub pending_admission: Option<u64>,
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
            load_admitted: false,
            loaded_tick: 0,
            captured: false,
            pending_admission: None,
        })
    }

    pub fn admit_load(&mut self) -> bool {
        if self.loaded_activity.is_some() || self.load_admitted {
            return false;
        }
        self.load_admitted = true;
        true
    }

    pub fn needs_capture(&self, button_wait: bool) -> bool {
        self.loaded_activity.is_some()
            && !self.captured
            && self.pending_admission.is_none()
            && !button_wait
    }

    pub fn admits_capture(&self, activity: u16, button_wait: bool) -> bool {
        self.needs_capture(button_wait) && activity & 0x1000 == 0
    }

    pub fn reserve_capture(&mut self, activity: u16, button_wait: bool, input: u64) -> bool {
        if !self.admits_capture(activity, button_wait) {
            return false;
        }
        self.pending_admission = Some(input);
        true
    }
}

#[cfg(not(test))]
impl Request {
    /// The caller is the post-game-initialization hook on the game thread.
    pub fn load(&mut self, cx: &mut crate::engine::battle::Cx<'_>) -> anyhow::Result<()> {
        anyhow::ensure!(
            std::mem::take(&mut self.load_admitted),
            "save load has no admission"
        );
        anyhow::ensure!(self.loaded_activity.is_none(), "save capture loaded twice");
        anyhow::ensure!(
            self.source.is_file(),
            "missing save {}",
            self.source.display()
        );
        let loaded = crate::savegame::live::load::load_game(
            0,
            crate::savegame::live::load::LoadRequest::Restore(cx),
        );
        anyhow::ensure!(loaded != 0, "LoadGame failed for {}", self.source.display());
        let activity = cx.world.next_activity;
        self.loaded_activity = Some(activity);
        self.loaded_tick = crate::time::source::now_ms();
        // Preserve NextActivity for the production main-loop load resolution.
        cx.world.set_activity(0x1000);
        cx.kernel.last_activity = 0x1000;
        Ok(())
    }

    /// Capture at the first non-button mode admission, before consuming input.
    pub fn capture(&mut self, cx: &mut crate::engine::battle::Cx<'_>) -> anyhow::Result<()> {
        let input_admission = self
            .pending_admission
            .take()
            .ok_or_else(|| anyhow::anyhow!("save capture has no input admission"))?;
        let activity = cx.world.activity();
        anyhow::ensure!(
            self.admits_capture(activity, false),
            "invalid save capture boundary"
        );
        let tick = crate::time::source::now_ms();
        // SAFETY: the main loop has resolved CHECK_LOAD to NextActivity and
        // initialized the mode; its first admitted input has not been consumed.
        let save = unsafe {
            crate::savegame::live::save::capture(cx, cx.solar_frame, b"runtime-capture")?
        };
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
    fn load_admission_waits_for_the_initialized_owner_and_does_not_admit_capture() {
        let mut request = Request {
            source: PathBuf::from("source"),
            output: PathBuf::from("output"),
            loaded_activity: None,
            load_admitted: false,
            loaded_tick: 0,
            captured: false,
            pending_admission: None,
        };
        assert!(request.admit_load());
        assert!(!request.admit_load());
        assert_eq!(request.loaded_activity, None);
        assert_eq!(request.loaded_tick, 0);
        assert!(!request.needs_capture(false));
        request.loaded_activity = Some(0x0804);
        assert!(!request.admit_load());
    }

    #[test]
    fn capture_reserves_the_first_loaded_non_button_admission_until_the_owner_returns() {
        let mut request = Request {
            source: PathBuf::from("source"),
            output: PathBuf::from("output"),
            loaded_activity: Some(0x0804),
            load_admitted: false,
            loaded_tick: 17,
            captured: false,
            pending_admission: None,
        };
        assert!(!request.reserve_capture(0x1000, false, 1));
        assert!(!request.reserve_capture(0x0804, true, 2));
        assert!(request.reserve_capture(0x0804, false, 3));
        assert_eq!(request.pending_admission, Some(3));
        assert!(!request.captured);
        assert_eq!(request.loaded_tick, 17);
        assert!(!request.reserve_capture(0x0804, false, 4));
        assert_eq!(request.pending_admission, Some(3));
        assert!(!request.admits_capture(0x0804, false));
    }

    #[test]
    fn only_loaded_non_button_admission_captures_once() {
        let mut request = Request {
            source: PathBuf::from("source"),
            output: PathBuf::from("output"),
            loaded_activity: None,
            load_admitted: false,
            loaded_tick: 0,
            captured: false,
            pending_admission: None,
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
