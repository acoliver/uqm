//! Ordered observation boundaries shared by modes and their host.

pub mod planet_side;

use crate::battle::Cx;
use crate::graphics::presented_frame::PresentedFrame;

pub mod starmap;
pub use starmap::StarMapView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputView {
    BeforeUpdate,
    Nested,
    AfterUpdate(u16),
    Active,
    PlanetSideBegin,
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
    PlanetSide(&'a planet_side::PlanetSideSnapshot),
    PlanetSideReport(bool),
    BattleReset,
    BattleAdvanced,
    InputSampled(u16),
    FlagshipDestroyed,
    ShipEscaped,
    EncounterDispatch(u32),
    DialogueDispatch(u32),
    EncounterMenu(bool),
    CommunicationResponses {
        count: usize,
        choices: &'a ResponseChoices,
    },
    ResponsesReady(bool),
    CommunicationStarted,
    CommunicationEnded,
    CommunicationReplay(bool),
    GameOptions(bool),
    PlanetMenu(usize),
    StarbaseMenu(bool),
    ShipPicker(bool),
    ShipPickerCursor {
        x: i16,
        y: i16,
    },
    StarMap(Option<StarMapView>),
    Devices(&'a DeviceObservation),
    Solar(Option<&'a crate::world::generation::SolarSnapshot>),
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
    pub planet_side: Option<planet_side::TripAdmission>,
    pub comm_response: Option<usize>,
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

pub trait InputSetup {
    fn configure_fleet(&mut self, ships: [[u8; 7]; 2], cyborg: bool) -> Result<(), String>;
    fn connect_peer(
        &mut self,
        remote_player: u16,
        server: bool,
        port: u16,
        observer: &mut dyn Observer,
    ) -> Result<Option<u16>, String>;
    fn peer_setup(&self, remote_player: u16) -> Option<PeerSetup>;
    fn poll_peer(&mut self, observer: &mut dyn Observer);
    /// Lend the admitted frame's context to a synchronous observation effect.
    fn with_context(&mut self, operation: &mut dyn FnMut(&mut Cx<'_>));
}

/// Observation boundaries. Admission and simulation checkpoints receive the
/// supplied context; observers copy what they record and retain no borrow.
pub trait Observer {
    fn input_admitted(&mut self, view: &InputView, cx: &mut Cx<'_>) -> Injected;
    fn input_admitted_with_setup(
        &mut self,
        view: &InputView,
        setup: &mut dyn InputSetup,
    ) -> Injected {
        let mut injected = Injected::default();
        setup.with_context(&mut |cx| injected = self.input_admitted(view, cx));
        injected
    }
    fn battle_diagnostics(&self) -> bool {
        false
    }
    fn battle_tick(&mut self, _tick: &crate::observe::battle::BattleTick) {}
    fn replacement_gate(&mut self, _gate: &crate::observe::battle::ReplacementGate) {}
    /// A completed simulation step whose state the observer may record.
    fn simulation(&mut self, _cx: &Cx<'_>, _boundary: &str) {}
    fn checkpoint(&mut self, view: &CheckpointView<'_>);
    fn presented(&mut self, presented: &mut Presented);
}

pub struct ObservationIo<'a> {
    pub observer: &'a mut dyn Observer,
}

#[derive(Default)]
pub struct Unobserved;

impl Observer for Unobserved {
    fn input_admitted(&mut self, _: &InputView, _: &mut Cx<'_>) -> Injected {
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

    fn with_context<R>(body: impl FnOnce(&mut Cx<'_>) -> R) -> R {
        let mut game = crate::Game::new();
        let mut battle = crate::battle::Battle::new();
        body(&mut Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        ))
    }

    #[test]
    fn unobserved_boundaries_do_not_inject_or_stop() {
        let mut observer = Unobserved;
        let io = ObservationIo {
            observer: &mut observer,
        };
        with_context(|cx| {
            assert_eq!(
                io.observer.input_admitted(&InputView::BeforeUpdate, cx),
                Injected::default()
            );
            io.observer.simulation(cx, "battle");
        });
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
            fn input_admitted(&mut self, _: &InputView, _: &mut Cx<'_>) -> Injected {
                self.nested += 1;
                Injected::default()
            }
            fn input_admitted_with_setup(
                &mut self,
                _: &InputView,
                setup: &mut dyn InputSetup,
            ) -> Injected {
                setup.configure_fleet([[3; 7]; 2], true).unwrap();
                setup.connect_peer(1, true, 2020, self).unwrap();
                Injected {
                    stop: false,
                    seed: Some(self.nested as u32),
                    planet_side: None,
                    comm_response: None,
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
                let io = ObservationIo { observer };
                with_context(|cx| {
                    assert_eq!(
                        io.observer.input_admitted(&InputView::Nested, cx),
                        Injected::default()
                    )
                });
                self.effects.push("connected");
                Ok(Some(port))
            }
            fn peer_setup(&self, _: u16) -> Option<PeerSetup> {
                None
            }
            fn poll_peer(&mut self, _: &mut dyn Observer) {
                panic!("unexpected peer poll")
            }
            fn with_context(&mut self, operation: &mut dyn FnMut(&mut Cx<'_>)) {
                with_context(|cx| operation(cx))
            }
        }
        let mut observer = Observed { nested: 0 };
        let mut effects = Vec::new();
        let result = observer.input_admitted_with_setup(
            &InputView::BeforeUpdate,
            &mut Setup {
                effects: &mut effects,
            },
        );
        assert_eq!(
            result,
            Injected {
                stop: false,
                seed: Some(1),
                planet_side: None,
                comm_response: None,
            }
        );
        assert_eq!(observer.nested, 1);
        assert_eq!(effects, ["configure", "connect", "connected"]);
    }
}

/// Callable phrase names copied at a conversation's observation boundary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResponseChoices(pub Vec<Option<String>>);

impl ResponseChoices {
    pub fn index(&self, phrase: &str) -> Result<usize, String> {
        let mut matches = self
            .0
            .iter()
            .enumerate()
            .filter_map(|(i, name)| (name.as_deref() == Some(phrase)).then_some(i));
        match (matches.next(), matches.next()) {
            (Some(i), None) => Ok(i),
            (None, _) => Err(format!("response_not_offered:phrase={phrase}")),
            _ => Err(format!("response_ambiguous:phrase={phrase}")),
        }
    }
}
