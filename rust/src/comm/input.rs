//! Conversation-owned selection is applied at the admitted input boundary.

use super::Comm;
use crate::engine::{
    battle::Cx,
    input::{self, Frame},
    observe::{Injected, InputView, Io},
};

struct ConversationFrame<'a, F> {
    comm: &'a mut Comm,
    action: F,
}

impl<F: FnMut(&mut Comm, &mut Cx<'_>, &mut Io<'_>) -> bool> Frame for ConversationFrame<'_, F> {
    fn admit(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, view: InputView) -> Injected {
        let admitted = input::admit_observer(cx, io, view);
        self.comm.apply_input(admitted);
        admitted
    }

    fn frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool {
        (self.action)(self.comm, cx, io)
    }
}

impl Comm {
    pub(crate) fn admit_observer(
        &mut self,
        io: &mut Io<'_>,
        view: &InputView,
        campaign: &Option<crate::engine::observe::CampaignView>,
    ) -> Injected {
        let admitted = io.observer.input_admitted(view, campaign);
        self.apply_input(admitted);
        admitted
    }

    pub(crate) fn apply_input(&mut self, admitted: Injected) {
        if let Some(index) = admitted.comm_response {
            // The offered list is bounded by MAX_RESPONSES, so this also proves i32 fits.
            assert!(index < self.responses().count());
            assert!(self.responses_mut().select(index as i32));
        }
    }

    pub(crate) fn run_input(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        reset: bool,
        action: impl FnMut(&mut Comm, &mut Cx<'_>, &mut Io<'_>) -> bool,
    ) {
        cx.run_input_frame(io, reset, &mut ConversationFrame { comm: self, action });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::observe::{CampaignView, CheckpointView, Observer, Presented};

    struct Select {
        calls: usize,
    }

    impl Observer<Option<CampaignView>> for Select {
        fn input_admitted(&mut self, _: &InputView, _: &Option<CampaignView>) -> Injected {
            self.calls += 1;
            Injected {
                comm_response: Some(1),
                ..Injected::default()
            }
        }
        fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
        fn presented(&mut self, _: &mut Presented) {}
    }

    #[test]
    #[serial_test::serial]
    fn direct_and_frame_admission_select_only_the_supplied_conversation() {
        let mut first = Comm::new();
        let mut second = Comm::new();
        for comm in [&mut first, &mut second] {
            comm.add_response(17, "First", None);
            comm.add_response(23, "Second", None);
        }
        let mut observer = Select { calls: 0 };
        let mut io = Io {
            observer: &mut observer,
        };
        first.admit_observer(&mut io, &InputView::PendingResponse { count: 2 }, &None);
        assert_eq!(first.selected_response(), 1);
        assert_eq!(second.selected_response(), 0);
        first.responses_mut().select(0);
        let mut frame = ConversationFrame {
            comm: &mut first,
            action: |comm: &mut Comm, _: &mut Cx<'_>, _: &mut Io<'_>| {
                assert_eq!(comm.selected_response(), 1);
                false
            },
        };
        crate::comm::with_test_context(|cx| {
            frame.admit(cx, &mut io, InputView::BeforeUpdate);
            assert!(!frame.frame(cx, &mut io));
        });
        assert_eq!(second.selected_response(), 0);
        assert_eq!(observer.calls, 2);
    }

    #[test]
    #[serial_test::serial]
    fn installed_driver_applies_selection_before_the_session_frame() {
        let mut comm = Comm::new();
        comm.add_response(17, "First", None);
        comm.add_response(23, "Second", None);
        let mut observer = Select { calls: 0 };
        let mut io = Io {
            observer: &mut observer,
        };
        crate::comm::with_test_context(|cx| {
            fixture::install(cx);
            comm.run_input(cx, &mut io, true, |session, cx, _| {
                assert_eq!(
                    session
                        .responses()
                        .get(session.selected_response() as usize)
                        .unwrap()
                        .response_ref,
                    23
                );
                cx.world.set_activity(17);
                false
            });
            assert_eq!(cx.world.activity(), 17);
        });
        assert_eq!(comm.selected_response(), 1);
        assert_eq!(observer.calls, 1);
    }
}

#[cfg(test)]
pub(crate) mod fixture {
    use crate::engine::{
        battle::{input::InputContext, Cx},
        input::{Frame, InputRuntime, InputSession},
        observe::{CampaignView, InputView, Io, Observer, Unobserved},
    };

    pub struct Driver(pub InputSession);
    impl InputRuntime for Driver {
        fn input(&self) -> &InputSession {
            &self.0
        }
        fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
            Box::new(Unobserved)
        }
        fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
            assert!(reset);
            loop {
                if frame.admit(cx, io, InputView::BeforeUpdate).stop || !frame.frame(cx, io) {
                    break;
                }
            }
        }
        fn yield_poll(&self, _: &mut Cx<'_>) {
            panic!("unexpected yield");
        }
        fn sleep(&self, _: &mut Cx<'_>, _: u32) {
            panic!("unexpected sleep");
        }
        fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
            panic!("unexpected sleep-until");
        }
        fn flush(&self) {
            panic!("unexpected flush");
        }
        fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
            panic!("unexpected exit confirmation");
        }
        fn update(&self, _: &mut Cx<'_>) {
            panic!("unexpected update");
        }
        fn battle_input(&self, _: &InputContext) -> u8 {
            panic!("unexpected battle input");
        }
    }

    pub fn install(cx: &mut Cx<'_>) {
        let input = InputSession::new(Default::default(), cx.kernel.configuration.clone());
        cx.kernel
            .install_input_runtime(std::rc::Rc::new(Driver(input)));
    }
}
