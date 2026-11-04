//! Response mutations and their ordered observer notifications.

use super::{CheckpointView, COMM_STATE};
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
    if ok {
        crate::app::observe::checkpoint(CheckpointView::CommunicationResponses(response_count));
    }
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
    crate::app::observe::checkpoint(CheckpointView::CommunicationResponses(0));
    crate::app::observe::checkpoint(CheckpointView::ResponsesReady(false));
}
