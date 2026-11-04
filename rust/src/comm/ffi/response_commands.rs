//! Response mutations and their ordered observer notifications.

use super::COMM_STATE;
use std::ffi::{c_int, c_uint, CStr};

/// Add a response option, rejecting absent or non-UTF-8 resource text.
/// `func` receives `response_ref` when the response is selected.
pub fn do_response_phrase(
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

    let ok = COMM_STATE
        .write()
        .add_response(response_ref, text_str, func);
    let response_count = COMM_STATE.read().responses().count();
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
pub fn rust_clear_responses() {
    COMM_STATE.write().clear_responses();
}

pub(crate) fn observe_responses(io: &mut crate::engine::observe::Io<'_>) {
    let count = COMM_STATE.read().responses().count();
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::CommunicationResponses(count));
}

pub(super) fn deliver_selected(
    cx: &mut crate::engine::battle::Cx<'_>,
    io: &mut crate::engine::observe::Io<'_>,
) {
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::CommunicationResponses(0));
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::ResponsesReady(
            false,
        ));
    let result = {
        let mut state = COMM_STATE.write();
        cx.with_legacy(|| crate::comm::talk_segue::select_response(&mut state))
    };
    if let Some((func, reference)) = result {
        eprintln!("[DBG] rust_DoCommunication: calling response callback ref={reference}");
        cx.with_legacy(|| func(reference));
        observe_responses(io);
        eprintln!(
            "[DBG] rust_DoCommunication: callback returned, talking_finished={}",
            COMM_STATE.read().is_talking_finished()
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
            assert!(
                COMM_STATE.try_write().is_some(),
                "observer must run outside the state lock"
            );
            if let CheckpointView::CommunicationResponses(count) = view {
                self.0.push(*count);
            }
        }
        fn presented(&mut self, _: &mut Presented) {}
    }

    #[test]
    #[serial_test::serial]
    fn selected_callback_releases_state_and_publishes_its_result_to_the_observer() {
        fn callback(reference: u32) {
            assert_eq!(reference, 17);
            let mut state = COMM_STATE
                .try_write()
                .expect("response callback must run outside the state lock");
            assert_eq!(state.responses().count(), 0);
            state.add_response(18, "Next response", None);
        }
        {
            let mut state = COMM_STATE.write();
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
            &mut cx,
            &mut Io {
                observer: &mut counts,
            },
        );
        assert_eq!(counts.0, [0, 1]);
        COMM_STATE.write().uninit();
    }
}
