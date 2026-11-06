//! Ordered observation boundaries shared by modes and their host.

use crate::graphics::presented_frame::PresentedFrame;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputView {
    BeforeUpdate,
    Nested,
    AfterUpdate(u16),
    Active,
    MainMenuReady,
    MenuTransition { from: u8, to: u8 },
    QuitSelected,
    ReplacesIntroduction,
    Seed { domain: u32, fallback: u32 },
    PendingSpeech,
    PendingResponse { count: usize },
    ClosingSpeech,
    CommunicationCompleted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointView<'a> {
    Simulation(&'a str),
    BattleReset,
    BattleAdvanced,
    InputSampled(u16),
    FlagshipDestroyed,
    ShipEscaped,
    EncounterDispatch(u32),
    DialogueDispatch(u32),
    EncounterMenu(bool),
    CommunicationResponses(usize),
    ResponsesReady(bool),
    CommunicationStarted,
    CommunicationEnded,
    CommunicationReplay(bool),
    GameOptions(bool),
    PlanetMenu(usize),
    StarbaseMenu(bool),
    Won,
    PresentationStarted,
    PresentationCancelled,
    PresentationCompleted,
    CreditsStarted,
    CreditsReady,
    CreditsCompleted,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Injected {
    pub stop: bool,
    pub seed: Option<u32>,
}

pub struct Presented {
    pub frame: Option<PresentedFrame>,
    pub stop: bool,
}

pub trait Observer<W = ()> {
    fn input_admitted(&mut self, view: &InputView, world: &W) -> Injected;
    fn checkpoint(&mut self, view: &CheckpointView<'_>);
    fn presented(&mut self, presented: &mut Presented);
}

pub struct Io<'a, W = ()> {
    pub observer: &'a mut dyn Observer<W>,
}

#[derive(Default)]
pub struct Unobserved;

impl<W> Observer<W> for Unobserved {
    fn input_admitted(&mut self, _: &InputView, _: &W) -> Injected {
        Injected::default()
    }

    fn checkpoint(&mut self, _: &CheckpointView<'_>) {}

    fn presented(&mut self, _: &mut Presented) {}
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DeviceObservation {
    pub in_orbit: bool,
    pub pc_menu: bool,
    pub list: Option<Vec<u8>>,
    pub selected: usize,
    /// Successful invocation published by the device owner, with actual state effects.
    pub activation: Option<(u64, u8, String, bool)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unobserved_boundaries_do_not_inject_or_stop() {
        let mut observer = Unobserved;
        let io = Io {
            observer: &mut observer,
        };
        assert_eq!(
            io.observer.input_admitted(&InputView::BeforeUpdate, &()),
            Injected::default()
        );
        io.observer.checkpoint(&CheckpointView::BattleAdvanced);
        let mut presented = Presented {
            frame: None,
            stop: false,
        };
        io.observer.presented(&mut presented);
        assert!(!presented.stop);
    }
}
