//! Sticky observations written by victory, presentation and credits owners.

use parking_lot::Mutex;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ending {
    pub won: bool,
    pub presentation_started: bool,
    pub presentation_completed: bool,
    pub credits_started: bool,
    pub credits_completed: bool,
    pub credits_controls_ready: bool,
    presentation_cancelled: bool,
}

impl Ending {
    fn start_presentation(&mut self) {
        self.presentation_started |= self.won;
        self.presentation_cancelled = false;
    }

    fn complete_presentation(&mut self) {
        self.presentation_completed |= self.presentation_started && !self.presentation_cancelled;
    }

    fn start_credits(&mut self) {
        self.credits_started |= self.presentation_completed;
    }

    fn complete_credits(&mut self) {
        self.credits_completed |= self.credits_started;
    }
}

static ENDING: Mutex<Ending> = Mutex::new(Ending {
    won: false,
    presentation_started: false,
    presentation_completed: false,
    credits_started: false,
    credits_completed: false,
    credits_controls_ready: false,
    presentation_cancelled: false,
});

pub fn snapshot() -> Ending {
    *ENDING.lock()
}

pub(crate) fn won() {
    ENDING.lock().won = true;
}

pub(crate) fn presentation_started() {
    ENDING.lock().start_presentation();
}

pub(crate) fn presentation_cancelled() {
    let mut ending = ENDING.lock();
    if ending.presentation_started && !ending.presentation_completed {
        ending.presentation_cancelled = true;
    }
}

pub(crate) fn presentation_completed() {
    ENDING.lock().complete_presentation();
}

pub(crate) fn credits_started() {
    ENDING.lock().start_credits();
}

pub(crate) fn credits_controls_ready() {
    let mut ending = ENDING.lock();
    ending.credits_controls_ready |= ending.credits_started;
}

pub(crate) fn credits_completed() {
    ENDING.lock().complete_credits();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_callbacks_retain_terminal_observations() {
        won();
        presentation_started();
        presentation_completed();
        credits_started();
        credits_controls_ready();
        assert!(snapshot().credits_controls_ready);
        credits_completed();
        presentation_cancelled();
        assert!(snapshot().credits_completed);
    }

    #[test]
    fn title_credits_and_unrelated_presentations_cannot_prove_an_ending() {
        let mut ending = Ending::default();
        ending.start_presentation();
        ending.complete_presentation();
        ending.start_credits();
        ending.complete_credits();
        assert_eq!(ending, Ending::default());
    }

    #[test]
    fn won_alone_and_cancelled_presentation_do_not_prove_completion() {
        let mut ending = Ending {
            won: true,
            ..Ending::default()
        };
        ending.start_presentation();
        ending.start_credits();
        assert!(ending.won && ending.presentation_started);
        assert!(!ending.presentation_completed && !ending.credits_started);
        // Cancelling a nested CALL must also prevent the outer script from
        // claiming completion when it resumes after that child returns.
        ending.presentation_cancelled = true;
        ending.complete_presentation();
        assert!(!ending.presentation_completed);
        ending.start_presentation();
        ending.complete_presentation();
        ending.start_credits();
        ending.complete_credits();
        assert!(
            ending.presentation_completed && ending.credits_started && ending.credits_completed
        );
        ending.start_presentation();
        assert!(
            ending.credits_completed,
            "observations survive later UI activity"
        );
    }
}
