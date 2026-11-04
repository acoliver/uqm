//! Connect legacy mode boundaries to the application-owned observation port.

use crate::engine::observe::{CheckpointView, Injected, InputView, Io, Presented};

pub fn input(view: InputView) -> Injected {
    with_io(|io| io.observer.input_admitted(&view))
}

pub fn checkpoint(view: CheckpointView<'_>) {
    match view {
        CheckpointView::BattleReset => crate::collections::observation::reset(),
        CheckpointView::BattleAdvanced => crate::collections::observation::advance(),
        _ => {}
    }
    with_io(|io| io.observer.checkpoint(&view));
}

pub fn presented(presented: &mut Presented) {
    with_io(|io| io.observer.presented(presented));
}

pub fn nested_input<R>(sample: impl FnOnce() -> R) -> Option<R> {
    with_io(|io| io.nested_input(sample))
}

pub fn seed(domain: u32, fallback: u32) -> u32 {
    input(InputView::Seed { domain, fallback })
        .seed
        .unwrap_or(fallback)
}

fn with_io<R>(call: impl FnOnce(&mut Io<'_>) -> R) -> R {
    if let Some(observer) = super::bridge::observer() {
        let mut capture = || crate::app::bridge::campaign_view();
        let mut observer = crate::automation::observer::Handle(observer, &mut capture);
        call(&mut Io {
            observer: &mut observer,
        })
    } else {
        call(&mut Io {
            observer: &mut crate::engine::observe::Unobserved,
        })
    }
}

pub(crate) fn game_initialized(observer: &crate::automation::AutomationObserver) {
    observer.activate_save_capture();
    observer.activate_start_scene(|plan| {
        // SAFETY: initialization runs on the publishing game thread. Only copied
        // entry data leaves this transaction, before legacy scene operations resume.
        let screen_context = unsafe {
            crate::app::bridge::enter(|cx| {
                for &(key, value) in plan.game_state {
                    cx.world.set_story_state(key, value);
                }
                cx.kernel_mut().screen_context
            })
        };
        #[cfg(not(test))]
        crate::automation::scenario::enter_scene(
            plan,
            &mut crate::automation::scenario::LinkedSceneEntry { screen_context },
        );
        #[cfg(test)]
        let _ = screen_context;
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn initialization_without_a_scene_never_enters_the_owner_slot() {
        let source = br#"{"version":1,"name":"no-scene","steps":[{"action":"finish"}],"budgets":{"max_input_ticks":10,"max_presentations":10,"max_wallclock_seconds":60}}"#;
        let script = crate::automation::validate_script(
            crate::automation::parse_script(source, "test.json").unwrap(),
            "test.json",
        )
        .unwrap();
        let output = tempfile::tempdir().unwrap();
        let observer = crate::automation::AutomationObserver::new(script, output.path().into());
        let before = observer.runtime.commit.next_sequence();
        super::game_initialized(&observer);
        assert_eq!(observer.runtime.commit.next_sequence(), before);
    }
}
