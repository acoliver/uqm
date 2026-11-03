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
    if input(InputView::Nested).stop {
        return None;
    }
    let result = sample();
    checkpoint(CheckpointView::InputSampled);
    Some(result)
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
