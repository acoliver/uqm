//! The input driver supplied by the application and owned by the kernel.

use super::{battle::input::InputContext, battle::Cx, observe::Io};
use std::rc::Rc;

pub use crate::input_port::*;
pub use crate::input_port::{controls::InputControls, session::InputSession};

pub use crate::input_port::session::InputFrameHook;

pub trait Frame {
    fn admit(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        view: super::observe::InputView,
    ) -> super::observe::Injected {
        admit_observer(cx, io, view)
    }

    fn frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool;
}

impl<F: FnMut(&mut Cx<'_>, &mut Io<'_>) -> bool> Frame for F {
    fn frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool {
        self(cx, io)
    }
}

pub fn admit_observer(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    view: super::observe::InputView,
) -> super::observe::Injected {
    let campaign = super::observe::CampaignView::for_input(view, || {
        super::observe::CampaignView::capture(cx.world, cx.kernel)
    });
    let admitted = cx.with_legacy(|| io.observer.input_admitted(&view, &campaign));
    if matches!(view, super::observe::InputView::AfterUpdate(_)) {
        super::observe::battle::observe_tick(cx, io, super::observe::battle::BattlePhase::Input);
    }
    admitted
}

pub trait InputRuntime {
    fn input(&self) -> &InputSession;
    fn observer(&self) -> Box<dyn crate::observe::Observer<Option<crate::observe::CampaignView>>>;
    fn yield_poll(&self, cx: &mut Cx<'_>);
    fn sleep(&self, cx: &mut Cx<'_>, ticks: u32);
    fn sleep_until(&self, cx: &mut Cx<'_>, deadline: u32);
    fn flush(&self);
    fn confirm_exit(&self, cx: &mut Cx<'_>) -> bool;
    fn update(&self, cx: &mut Cx<'_>);
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame);
    fn battle_input(&self, input: &InputContext) -> u8;
}

impl super::Kernel {
    pub fn install_input_runtime(&mut self, runtime: Rc<dyn InputRuntime>) {
        assert!(
            self.input_runtime.is_none(),
            "input runtime already installed"
        );
        self.input_runtime = Some(runtime);
    }

    pub fn input_controls(&self) -> &InputControls {
        self.input_runtime
            .as_ref()
            .unwrap_or_else(|| panic!("input runtime is not installed"))
            .input()
            .controls()
    }

    pub fn input_runtime(&self) -> Rc<dyn InputRuntime> {
        self.input_runtime
            .clone()
            .unwrap_or_else(|| panic!("input runtime is not installed"))
    }
}

impl Cx<'_> {
    pub fn human_input(&self, player: u16) -> u8 {
        self.kernel
            .input_runtime()
            .battle_input(&InputContext::new(player, 1))
    }

    /// Suspend this context for a synchronous provider that still uses the A4 slot.
    pub fn with_legacy<R>(&mut self, call: impl FnOnce() -> R) -> R {
        // SAFETY: the unique context is inaccessible to the caller until publication returns.
        unsafe { crate::handoff::publish(self, call) }
    }

    pub fn nested_input<R>(
        &mut self,
        io: &mut Io<'_>,
        sample: impl FnOnce(&mut Self) -> R,
    ) -> Option<R> {
        use super::observe::{CampaignView, CheckpointView, InputView};
        let campaign = CampaignView::for_input(InputView::Nested, || {
            CampaignView::capture(self.world, self.kernel)
        });
        let admitted =
            self.with_legacy(|| io.observer.input_admitted(&InputView::Nested, &campaign));
        if admitted.stop {
            return None;
        }
        let result = sample(self);
        let activity = self.world.activity();
        self.with_legacy(|| {
            io.observer
                .checkpoint(&CheckpointView::InputSampled(activity))
        });
        super::observe::battle::observe_tick(self, io, super::observe::battle::BattlePhase::Input);
        Some(result)
    }

    pub fn run_input(
        &mut self,
        io: &mut Io<'_>,
        reset: bool,
        mut frame: impl FnMut(&mut Cx<'_>, &mut Io<'_>) -> bool,
    ) {
        self.run_input_frame(io, reset, &mut frame);
    }
    pub fn run_input_frame(&mut self, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        self.kernel.input_runtime().run(self, io, reset, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{battle::Battle, observe::Unobserved, Game};
    use std::cell::RefCell;

    struct Driver(Rc<RefCell<Vec<bool>>>, InputSession);

    impl InputRuntime for Driver {
        fn input(&self) -> &InputSession {
            &self.1
        }

        fn observer(
            &self,
        ) -> Box<dyn crate::observe::Observer<Option<crate::observe::CampaignView>>> {
            Box::new(Unobserved)
        }

        fn yield_poll(&self, _: &mut Cx<'_>) {
            self.0.borrow_mut().push(true);
        }
        fn sleep(&self, _: &mut Cx<'_>, ticks: u32) {
            assert_eq!(ticks, 28);
            self.0.borrow_mut().push(false);
        }
        fn sleep_until(&self, _: &mut Cx<'_>, deadline: u32) {
            assert_eq!(deadline, 56);
            self.0.borrow_mut().push(true);
        }
        fn flush(&self) {
            self.0.borrow_mut().push(false);
        }
        fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
            self.0.borrow_mut().push(true);
            false
        }
        fn update(&self, _cx: &mut Cx<'_>) {}

        fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
            self.0.borrow_mut().push(reset);
            while frame.frame(cx, io) {}
        }

        fn battle_input(&self, input: &InputContext) -> u8 {
            input.player() as u8 + 7
        }
    }

    #[test]
    fn kernel_driver_receives_the_owner_reset_and_safe_frame_callback() {
        let resets = Rc::new(RefCell::new(Vec::new()));
        let mut game = Game::new();
        let configuration = game.kernel.configuration.clone();
        game.kernel.install_input_runtime(Rc::new(Driver(
            resets.clone(),
            InputSession::new(InputControls::default(), configuration),
        )));
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        cx.run_input(&mut io, true, |cx, _| {
            cx.world.set_activity(cx.world.activity() + 1);
            cx.world.activity() < 3
        });
        assert_eq!(cx.world.activity(), 3);
        assert_eq!(cx.human_input(1), 8);
        assert_eq!(*resets.borrow(), [true]);
        assert_eq!(
            cx.kernel
                .input_runtime()
                .battle_input(&InputContext::new(1, 1)),
            8
        );
    }

    #[test]
    fn retained_runtime_supplies_modal_and_pacing_operations() {
        let driver = Driver(Rc::default(), InputSession::default());
        let mut game = Game::new();
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        driver.sleep(&mut cx, 28);
        driver.sleep_until(&mut cx, 56);
        driver.flush();
        assert!(!driver.confirm_exit(&mut cx));
        driver.yield_poll(&mut cx);
        assert_eq!(*driver.0.borrow(), [false, true, false, true, true]);
    }

    #[test]
    fn retained_driver_keeps_its_runtime_after_kernel_teardown() {
        let mut game = Game::new();
        game.kernel
            .install_input_runtime(Rc::new(Driver(Rc::default(), InputSession::default())));
        let driver = game.kernel.input_runtime();
        drop(game);
        assert_eq!(driver.battle_input(&InputContext::new(1, 1)), 8);
    }

    #[test]
    #[serial_test::serial]
    fn explicit_context_handoff_publishes_and_restores_the_same_owner() {
        let mut game = Game::new();
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        cx.world.set_activity(7);
        cx.with_legacy(|| {
            crate::World::current().write(|world| world.set_activity(11));
        });
        assert_eq!(cx.world.activity(), 11);
    }

    #[test]
    #[should_panic(expected = "input runtime is not installed")]
    fn missing_driver_is_an_initialization_error() {
        Game::new().kernel.input_runtime();
    }

    #[test]
    #[should_panic(expected = "input runtime already installed")]
    fn installing_a_second_driver_is_an_initialization_error() {
        let mut game = Game::new();
        let driver = Rc::new(Driver(Rc::default(), InputSession::default()));
        game.kernel.install_input_runtime(driver.clone());
        game.kernel.install_input_runtime(driver);
    }

    struct NestedObserver {
        events: Rc<RefCell<Vec<&'static str>>>,
        stop: bool,
    }

    impl crate::observe::Observer<Option<crate::observe::CampaignView>> for NestedObserver {
        fn input_admitted(
            &mut self,
            view: &crate::observe::InputView,
            world: &Option<crate::observe::CampaignView>,
        ) -> crate::observe::Injected {
            assert_eq!(*view, crate::observe::InputView::Nested);
            let Some(world) = world else {
                panic!("nested input must supply its owner view");
            };
            assert_eq!(world.activity, 3);
            self.events.borrow_mut().push("admitted");
            crate::observe::Injected {
                stop: self.stop,
                seed: None,
                planet_side: None,
                comm_response: None,
            }
        }

        fn checkpoint(&mut self, view: &crate::observe::CheckpointView<'_>) {
            assert_eq!(*view, crate::observe::CheckpointView::InputSampled(0x4004));
            self.events.borrow_mut().push("sampled");
        }

        fn presented(&mut self, _: &mut crate::observe::Presented) {
            self.events.borrow_mut().push("presented");
        }
    }

    #[test]
    #[serial_test::serial]
    fn nested_input_records_the_owner_activity_after_one_sample() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = NestedObserver {
            events: events.clone(),
            stop: false,
        };
        let mut io = Io {
            observer: &mut observer,
        };
        let mut game = Game::new();
        game.world.set_activity(3);
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        assert_eq!(
            cx.nested_input(&mut io, |cx| {
                events.borrow_mut().push("sample");
                cx.world.set_activity(0x4004);
                7
            }),
            Some(7)
        );
        io.observer.presented(&mut crate::observe::Presented {
            frame: None,
            stop: false,
        });
        assert_eq!(
            *events.borrow(),
            ["admitted", "sample", "sampled", "presented"]
        );
        assert_eq!(cx.world.activity(), 0x4004);
    }

    #[test]
    #[serial_test::serial]
    fn stopped_nested_input_neither_samples_nor_publishes_a_checkpoint() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = NestedObserver {
            events: events.clone(),
            stop: true,
        };
        let mut io = Io {
            observer: &mut observer,
        };
        let mut game = Game::new();
        game.world.set_activity(3);
        let mut battle = Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        assert!(cx
            .nested_input(&mut io, |_| panic!("stopped input sampled"))
            .is_none());
        assert_eq!(*events.borrow(), ["admitted"]);
        assert_eq!(cx.world.activity(), 3);
    }
}
