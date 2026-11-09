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

impl Observer<Option<CampaignView>> for Recording {
    fn input_admitted(&mut self, view: &InputView, campaign: &Option<CampaignView>) -> Injected {
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

    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        let event = match view {
            CheckpointView::Simulation("input-loop") => "complete",
            CheckpointView::Simulation("input-hook") => "hook",
            _ => panic!("unexpected checkpoint {view:?}"),
        };
        self.events.borrow_mut().push(event);
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
    emit(CheckpointView::Simulation("input-hook"));
}

#[test]
fn nested_callbacks_complete_before_the_outer_frame_and_stopped_admission_does_not_update() {
    // SAFETY: the serialized fixture owns input and the draw queue on the game thread.
    battle_context::run(|input| unsafe {
        use uqm_rust::graphics::{dcqueue_abi, sdl::lifecycle_abi};
        lifecycle_abi::GameActive = 1;
        dcqueue_abi::Init_DrawCommandQueue();
        uqm_rust::mainloop::input_abi::SetDefaultMenuRepeatDelay(input);
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
        uqm_rust::app::bridge::enter(|cx| {
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
        uqm_rust::app::bridge::enter(|cx| {
            cx.run_input(&mut io, false, |_, _| {
                panic!("stopped admission invoked a frame")
            })
        });
        assert_eq!(*events.borrow(), ["before"]);
        uqm_rust::mainloop::input_abi::SetInputCallback(input, None);
        dcqueue_abi::Uninit_DrawCommandQueue();
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
    // SAFETY: this serialized fixture owns input and initializes its draw queue before dispatch.
    battle_context::run(|input| unsafe {
        use uqm_rust::graphics::{dcqueue_abi, sdl::lifecycle_abi};
        lifecycle_abi::GameActive = 1;
        dcqueue_abi::Init_DrawCommandQueue();
        uqm_rust::mainloop::input_abi::SetDefaultMenuRepeatDelay(input);
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
        uqm_rust::app::bridge::enter(|cx| {
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
        dcqueue_abi::Uninit_DrawCommandQueue();
    });
}
