//! Sticky observations written by victory, presentation and credits owners.

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
    pub(crate) fn observe_checkpoint(&mut self, view: crate::engine::observe::CheckpointView<'_>) {
        use crate::engine::observe::CheckpointView;
        match view {
            CheckpointView::Won => self.won(),
            CheckpointView::PresentationStarted => self.presentation_started(),
            CheckpointView::PresentationCancelled => self.presentation_cancelled(),
            CheckpointView::PresentationCompleted => self.presentation_completed(),
            CheckpointView::CreditsStarted => self.credits_started(),
            CheckpointView::CreditsReady => self.credits_controls_ready(),
            CheckpointView::CreditsCompleted => self.credits_completed(),
            _ => unreachable!("non-ending checkpoint passed to ending observation"),
        }
    }

    pub(crate) fn won(&mut self) {
        self.won = true;
    }

    pub(crate) fn presentation_started(&mut self) {
        self.presentation_started |= self.won;
        self.presentation_cancelled = false;
    }

    pub(crate) fn presentation_cancelled(&mut self) {
        if self.presentation_started && !self.presentation_completed {
            self.presentation_cancelled = true;
        }
    }

    pub(crate) fn presentation_completed(&mut self) {
        self.presentation_completed |= self.presentation_started && !self.presentation_cancelled;
    }

    pub(crate) fn credits_started(&mut self) {
        self.credits_started |= self.presentation_completed;
    }

    pub(crate) fn credits_controls_ready(&mut self) {
        self.credits_controls_ready |= self.credits_started;
    }

    pub(crate) fn credits_completed(&mut self) {
        self.credits_completed |= self.credits_started;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_callbacks_retain_terminal_observations() {
        let mut ending = Ending::default();
        ending.won();
        ending.presentation_started();
        ending.presentation_completed();
        ending.credits_started();
        ending.credits_controls_ready();
        assert!(ending.credits_controls_ready);
        ending.credits_completed();
        ending.presentation_cancelled();
        assert!(ending.credits_completed);
    }

    #[test]
    fn title_credits_and_unrelated_presentations_cannot_prove_an_ending() {
        let mut ending = Ending::default();
        ending.presentation_started();
        ending.presentation_completed();
        ending.credits_started();
        ending.credits_completed();
        assert_eq!(ending, Ending::default());
    }

    #[test]
    fn won_alone_and_cancelled_presentation_do_not_prove_completion() {
        let mut ending = Ending {
            won: true,
            ..Ending::default()
        };
        ending.presentation_started();
        ending.credits_started();
        assert!(ending.won && ending.presentation_started);
        assert!(!ending.presentation_completed && !ending.credits_started);
        // Cancelling a nested CALL must also prevent the outer script from
        // claiming completion when it resumes after that child returns.
        ending.presentation_cancelled = true;
        ending.presentation_completed();
        assert!(!ending.presentation_completed);
        ending.presentation_started();
        ending.presentation_completed();
        ending.credits_started();
        ending.credits_completed();
        assert!(
            ending.presentation_completed && ending.credits_started && ending.credits_completed
        );
        ending.presentation_started();
        assert!(
            ending.credits_completed,
            "observations survive later UI activity"
        );
    }
}
