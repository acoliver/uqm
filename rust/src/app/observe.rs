//! Connect legacy mode boundaries to the application-owned observation port.

use crate::engine::observe::{CheckpointView, Injected, InputView, Io};

pub fn input(view: InputView) -> Injected {
    let campaign =
        crate::engine::observe::CampaignView::for_input(view, super::bridge::campaign_view);
    with_io(|io| io.observer.input_admitted(&view, &campaign))
}

pub fn checkpoint(view: CheckpointView<'_>) {
    match view {
        CheckpointView::BattleReset => crate::collections::observation::reset(),
        CheckpointView::BattleAdvanced => crate::collections::observation::advance(),
        _ => {}
    }
    with_io(|io| io.observer.checkpoint(&view));
}

pub fn nested_input<R>(sample: impl FnOnce() -> R) -> Option<R> {
    let campaign = Some(super::bridge::campaign_view());
    with_io(|io| io.nested_input(&campaign, sample))
}

pub fn seed(domain: u32, fallback: u32) -> u32 {
    input(InputView::Seed { domain, fallback })
        .seed
        .unwrap_or(fallback)
}

fn with_io<R>(call: impl FnOnce(&mut Io<'_>) -> R) -> R {
    if let Some(observer) = super::bridge::observer() {
        let mut observer = crate::automation::observer::Handle(observer);
        call(&mut Io {
            observer: &mut observer,
        })
    } else {
        call(&mut Io {
            observer: &mut crate::engine::observe::Unobserved,
        })
    }
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
        assert!(observer.prepare_start_scene().is_none());
        assert_eq!(observer.runtime.commit.next_sequence(), before);
    }
}
