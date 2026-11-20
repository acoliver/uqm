//! The typed driver exercises the same nested native input and completion loop.
#[path = "support/battle_context.rs"]
mod battle_context;

use std::{cell::RefCell, rc::Rc};
use uqm_rust::engine::observe::{
    CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented,
};

struct Recording {
    events: Rc<RefCell<Vec<&'static str>>>,
    activities: Rc<RefCell<Vec<u16>>>,
    stop: bool,
}

impl Observer for Recording {
    fn input_admitted(
        &mut self,
        view: &InputView,
        cx: &mut uqm_rust::engine::battle::Cx<'_>,
    ) -> Injected {
        let campaign = CampaignView::for_input(*view, || CampaignView::capture(cx));
        match view {
            InputView::BeforeUpdate => {
                assert!(
                    campaign.is_some(),
                    "admission receives an owned campaign view"
                );
                self.events.borrow_mut().push("before");
            }
            InputView::AfterUpdate(activity) => {
                self.activities.borrow_mut().push(*activity);
                assert!(
                    campaign.is_none(),
                    "post-update does not capture the campaign"
                );
                self.events.borrow_mut().push("after");
            }
            InputView::Active => {
                assert!(campaign.is_none());
                self.events.borrow_mut().push("hook");
            }
            _ => panic!("unexpected input boundary {view:?}"),
        }
        Injected {
            stop: self.stop,
            seed: None,
            planet_side: None,
            comm_response: None,
        }
    }

    fn simulation(&mut self, _: &uqm_rust::engine::battle::Cx<'_>, boundary: &str) {
        assert_eq!(boundary, "input-loop");
        self.events.borrow_mut().push("complete");
    }

    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        assert_eq!(*view, CheckpointView::InputSampled(HOOK_SAMPLE));
        self.events.borrow_mut().push("hook");
    }

    fn presented(&mut self, _: &mut Presented) {
        panic!("this test does not install a window or presentation hook");
    }
}

fn record_hook(
    emit: &mut dyn FnMut(CheckpointView<'_>),
    input: &uqm_rust::engine::input::InputControls,
) {
    assert_eq!(input.menu_sounds(), (32, 64));
    emit(CheckpointView::InputSampled(HOOK_SAMPLE));
}

/// The sample the installed input hook reports, distinct from any test activity.
const HOOK_SAMPLE: u16 = 0x7e57;

#[test]
fn nested_callbacks_complete_before_the_outer_frame_and_stopped_admission_does_not_update() {
    battle_context::run(|input| {
        uqm_rust::graphics::state::init_headless(Default::default());
        input.set_default_menu_repeat(uqm_rust::engine::time::counter_abi::GetTimeCounter());
        input.set_menu_sounds(32, 64);
        uqm_rust::mainloop::input_abi::SetInputCallback(
            input,
            Some(uqm_rust::engine::input::InputFrameHook::new(record_hook)),
        );
        let events = Rc::new(RefCell::new(Vec::new()));
        let activities = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording {
            events: events.clone(),
            activities: activities.clone(),
            stop: false,
        };
        let mut io = Io {
            observer: &mut observer,
        };
        crate::battle_context::with(|cx| {
            cx.world.set_activity(0);
            cx.run_input(&mut io, true, |cx, io| {
                events.borrow_mut().push("outer");
                cx.world.set_activity(1);
                cx.run_input(io, false, |cx, _| {
                    assert_eq!(cx.world.activity(), 1);
                    cx.world.set_activity(2);
                    events.borrow_mut().push("nested");
                    false
                });
                assert_eq!(cx.world.activity(), 2);
                false
            });
        });
        assert_eq!(
            *events.borrow(),
            [
                "before", "after", "hook", "outer", "before", "after", "hook", "nested",
                "complete", "complete"
            ]
        );
        assert_eq!(*activities.borrow(), [0, 1]);
        observer.stop = true;
        events.borrow_mut().clear();
        let mut io = Io {
            observer: &mut observer,
        };
        crate::battle_context::with(|cx| {
            cx.run_input(&mut io, false, |_, _| {
                panic!("stopped admission invoked a frame")
            })
        });
        assert_eq!(*events.borrow(), ["before"]);
        uqm_rust::mainloop::input_abi::SetInputCallback(input, None);
        uqm_rust::graphics::state::uninit_headless();
    });
}

struct AdmittedFrame {
    events: Rc<RefCell<Vec<&'static str>>>,
    admissions: usize,
}

impl uqm_rust::engine::input::Frame for AdmittedFrame {
    fn admit(
        &mut self,
        cx: &mut uqm_rust::engine::battle::Cx<'_>,
        io: &mut Io<'_>,
        view: InputView,
    ) -> Injected {
        self.events.borrow_mut().push(match view {
            InputView::BeforeUpdate => "mode-before",
            InputView::AfterUpdate(_) => "mode-after",
            _ => panic!("unexpected mode admission {view:?}"),
        });
        self.admissions += 1;
        uqm_rust::engine::input::admit_observer(cx, io, view)
    }

    fn frame(&mut self, _: &mut uqm_rust::engine::battle::Cx<'_>, _: &mut Io<'_>) -> bool {
        assert_eq!(self.admissions, 2);
        self.events.borrow_mut().push("mode-frame");
        false
    }
}

#[test]
fn mode_owned_frame_admission_keeps_the_native_observer_order() {
    battle_context::run(|input| {
        uqm_rust::graphics::state::init_headless(Default::default());
        input.set_default_menu_repeat(uqm_rust::engine::time::counter_abi::GetTimeCounter());
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording {
            events: events.clone(),
            activities: Default::default(),
            stop: false,
        };
        let mut frame = AdmittedFrame {
            events: events.clone(),
            admissions: 0,
        };
        crate::battle_context::with(|cx| {
            cx.run_input_frame(
                &mut Io {
                    observer: &mut observer,
                },
                false,
                &mut frame,
            );
        });
        assert_eq!(
            *events.borrow(),
            [
                "mode-before",
                "before",
                "mode-after",
                "after",
                "mode-frame",
                "complete"
            ]
        );
        uqm_rust::graphics::state::uninit_headless();
    });
}
