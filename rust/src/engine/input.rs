//! The input driver supplied by the application and owned by the kernel.

use super::{battle::input::InputContext, battle::Cx, observe::Io};
use std::rc::Rc;

pub type Frame<'a> = dyn FnMut(&mut Cx<'_>, &mut Io<'_>) -> bool + 'a;

pub trait InputRuntime {
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut Frame<'_>);
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

    pub fn input_runtime(&self) -> Rc<dyn InputRuntime> {
        self.input_runtime
            .clone()
            .unwrap_or_else(|| panic!("input runtime is not installed"))
    }
}

impl Cx<'_> {
    pub fn run_input(
        &mut self,
        io: &mut Io<'_>,
        reset: bool,
        mut frame: impl FnMut(&mut Cx<'_>, &mut Io<'_>) -> bool,
    ) {
        self.kernel.input_runtime().run(self, io, reset, &mut frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{battle::Battle, observe::Unobserved, Game};
    use std::cell::RefCell;

    struct Driver(Rc<RefCell<Vec<bool>>>);

    impl InputRuntime for Driver {
        fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut Frame<'_>) {
            self.0.borrow_mut().push(reset);
            while frame(cx, io) {}
        }

        fn battle_input(&self, input: &InputContext) -> u8 {
            input.player() as u8 + 7
        }
    }

    #[test]
    fn kernel_driver_receives_the_owner_reset_and_safe_frame_callback() {
        let resets = Rc::new(RefCell::new(Vec::new()));
        let mut game = Game::new();
        game.kernel
            .install_input_runtime(Rc::new(Driver(resets.clone())));
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
        assert_eq!(*resets.borrow(), [true]);
        assert_eq!(
            cx.kernel
                .input_runtime()
                .battle_input(&InputContext::new(1, 1)),
            8
        );
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
        let driver = Rc::new(Driver(Rc::default()));
        game.kernel.install_input_runtime(driver.clone());
        game.kernel.install_input_runtime(driver);
    }
}
