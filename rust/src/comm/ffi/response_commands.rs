//! Response mutations and their ordered observer notifications.

use crate::comm::Comm;
use std::ffi::{c_int, c_uint, CStr};

/// Add a response option, rejecting absent or non-UTF-8 resource text.
/// `func` receives `response_ref` when the response is selected.
pub fn do_response_phrase(
    comm: &mut Comm,
    response_ref: c_uint,
    text: Option<&CStr>,
    func: Option<crate::comm::response::ResponseFunc>,
) -> c_int {
    eprintln!(
        "[DBG] rust_DoResponsePhrase: ref={} text_null={} func={}",
        response_ref,
        text.is_none(),
        func.is_some()
    );
    let Some(text) = text else {
        return 0;
    };
    let Ok(text_str) = text.to_str() else {
        return 0;
    };

    let ok = comm.add_response(response_ref, text_str, func);
    let response_count = comm.responses().count();
    eprintln!(
        "[DBG] rust_DoResponsePhrase: added={}, count={}",
        ok, response_count
    );
    if ok {
        1
    } else {
        0
    }
}

/// Clear all responses
pub fn rust_clear_responses(comm: &mut Comm) {
    comm.clear_responses();
}

pub(crate) fn observe_responses(comm: &mut Comm, io: &mut crate::engine::observe::Io<'_>) {
    let choices = crate::comm::ffi::response_choices(comm);
    io.observer.checkpoint(
        &crate::engine::observe::CheckpointView::CommunicationResponses {
            count: comm.responses().count(),
            choices: &choices,
        },
    );
}

pub(super) fn deliver_selected(
    comm: &mut Comm,
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    io.observer.checkpoint(
        &crate::engine::observe::CheckpointView::CommunicationResponses {
            count: 0,
            choices: &Default::default(),
        },
    );
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::ResponsesReady(
            false,
        ));
    let result = {
        let state = &mut *comm;
        let activity = cx.world.activity();
        cx.with_legacy(|| crate::comm::talk_segue::select_response(activity, state))
    };
    if let Some((func, reference)) = result {
        eprintln!("[DBG] rust_DoCommunication: calling response callback ref={reference}");
        func(comm, cx, reference);
        observe_responses(comm, io);
        eprintln!(
            "[DBG] rust_DoCommunication: callback returned, talking_finished={}",
            comm.is_talking_finished()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{
        battle::{Battle, Cx},
        observe::{CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented},
        Game,
    };

    struct Counts(Vec<usize>);

    impl Observer<Option<CampaignView>> for Counts {
        fn input_admitted(&mut self, _: &InputView, _: &Option<CampaignView>) -> Injected {
            Injected::default()
        }
        fn checkpoint(&mut self, view: &CheckpointView<'_>) {
            if let CheckpointView::CommunicationResponses { count, .. } = view {
                self.0.push(*count);
            }
        }
        fn presented(&mut self, _: &mut Presented) {}
    }

    #[test]
    #[serial_test::serial]
    fn selected_callback_releases_state_and_publishes_its_result_to_the_observer() {
        let mut owner = Comm::new();
        let comm = &mut owner;
        fn callback(comm: &mut Comm, _cx: &mut Cx<'_>, reference: u32) {
            assert_eq!(reference, 17);
            let state = comm;
            assert_eq!(state.responses().count(), 0);
            state.add_response(18, "Next response", None);
        }
        {
            let state = &mut *comm;
            state.uninit();
            state.init().unwrap();
            state.add_response(17, "Select", Some(callback));
            state.responses_mut().start_display();
            state.responses_mut().select(0);
        }
        let mut game = Game::new();
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let mut counts = Counts(Vec::new());
        deliver_selected(
            comm,
            &mut cx,
            &mut Io {
                observer: &mut counts,
            },
        );
        assert_eq!(counts.0, [0, 1]);
        comm.uninit();
    }
}
