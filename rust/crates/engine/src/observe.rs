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

/// Setup operations lent synchronously by the admitted frame, not stored by observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerSetup {
    pub ready: bool,
    pub disconnected: bool,
}

pub trait InputSetup<W = ()> {
    fn configure_fleet(&mut self, ships: [[u8; 7]; 2], cyborg: bool) -> Result<(), String>;
    fn connect_peer(
        &mut self,
        remote_player: u16,
        server: bool,
        port: u16,
        observer: &mut dyn Observer<W>,
    ) -> Result<Option<u16>, String>;
    fn peer_setup(&self, remote_player: u16) -> Option<PeerSetup>;
    fn poll_peer(&mut self, observer: &mut dyn Observer<W>);
    /// Run an unconverted observation effect in the frame's existing publication scope.
    fn with_legacy(&mut self, operation: &mut dyn FnMut());
}

pub trait Observer<W = ()> {
    fn input_admitted(&mut self, view: &InputView, world: &W) -> Injected;
    fn input_admitted_with_setup(
        &mut self,
        view: &InputView,
        world: &W,
        _setup: &mut dyn InputSetup<W>,
    ) -> Injected {
        self.input_admitted(view, world)
    }
    fn checkpoint(&mut self, view: &CheckpointView<'_>);
    fn presented(&mut self, presented: &mut Presented);
}

pub struct ObservationIo<'a, W = ()> {
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
        let io = ObservationIo {
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

    #[test]
    fn setup_admission_lends_the_invoking_observer_synchronously() {
        struct Observed {
            nested: usize,
        }
        impl Observer for Observed {
            fn input_admitted(&mut self, _: &InputView, _: &()) -> Injected {
                self.nested += 1;
                Injected::default()
            }
            fn input_admitted_with_setup(
                &mut self,
                _: &InputView,
                _: &(),
                setup: &mut dyn InputSetup,
            ) -> Injected {
                setup.configure_fleet([[3; 7]; 2], true).unwrap();
                setup.connect_peer(1, true, 2020, self).unwrap();
                Injected {
                    stop: false,
                    seed: Some(self.nested as u32),
                }
            }
            fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
            fn presented(&mut self, _: &mut Presented) {}
        }
        struct Setup<'a> {
            effects: &'a mut Vec<&'static str>,
        }
        impl InputSetup for Setup<'_> {
            fn configure_fleet(&mut self, ships: [[u8; 7]; 2], cyborg: bool) -> Result<(), String> {
                assert_eq!(ships, [[3; 7]; 2]);
                assert!(cyborg);
                self.effects.push("configure");
                Ok(())
            }
            fn connect_peer(
                &mut self,
                player: u16,
                server: bool,
                port: u16,
                observer: &mut dyn Observer,
            ) -> Result<Option<u16>, String> {
                assert_eq!((player, server, port), (1, true, 2020));
                self.effects.push("connect");
                let io = Io { observer };
                assert_eq!(
                    io.observer.input_admitted(&InputView::Nested, &()),
                    Injected::default()
                );
                self.effects.push("connected");
                Ok(Some(port))
            }
            fn peer_setup(&self, _: u16) -> Option<PeerSetup> {
                None
            }
            fn poll_peer(&mut self, _: &mut dyn Observer) {
                panic!("unexpected peer poll")
            }
            fn with_legacy(&mut self, operation: &mut dyn FnMut()) {
                operation()
            }
        }
        let mut observer = Observed { nested: 0 };
        let mut effects = Vec::new();
        let result = observer.input_admitted_with_setup(
            &InputView::BeforeUpdate,
            &(),
            &mut Setup {
                effects: &mut effects,
            },
        );
        assert_eq!(
            result,
            Injected {
                stop: false,
                seed: Some(1)
            }
        );
        assert_eq!(observer.nested, 1);
        assert_eq!(effects, ["configure", "connect", "connected"]);
    }
}
