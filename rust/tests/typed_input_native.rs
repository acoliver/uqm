//! The typed driver exercises the same nested native input and completion loop.
#[path = "support/battle_context.rs"]
mod battle_context;

use std::{cell::RefCell, rc::Rc};
use uqm_rust::engine::observe::{
    CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented,
};

struct Recording {
    events: Rc<RefCell<Vec<&'static str>>>,
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
            InputView::AfterUpdate => {
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
        }
    }

    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        assert_eq!(*view, CheckpointView::Simulation("input-loop"));
        self.events.borrow_mut().push("complete");
    }

    fn presented(&mut self, _: &mut Presented) {
        panic!("this test does not install a window or presentation hook");
    }
}

fn record_hook(io: &mut Io<'_>) {
    io.observer.input_admitted(&InputView::Active, &None);
}

#[test]
fn nested_callbacks_complete_before_the_outer_frame_and_stopped_admission_does_not_update() {
    // SAFETY: the serialized fixture owns input and the draw queue on the game thread.
    battle_context::run(|| unsafe {
        use uqm_rust::graphics::{dcqueue_abi, sdl::lifecycle_abi};
        lifecycle_abi::GameActive = 1;
        dcqueue_abi::Init_DrawCommandQueue();
        uqm_rust::mainloop::input_abi::SetDefaultMenuRepeatDelay();
        uqm_rust::mainloop::input_abi::SetInputCallback(Some(record_hook));
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording {
            events: events.clone(),
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
        uqm_rust::mainloop::input_abi::SetInputCallback(None);
        dcqueue_abi::Uninit_DrawCommandQueue();
    });
}
