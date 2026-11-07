//! The application implementation of the kernel's typed input port.

#[cfg(not(test))]
use super::live::*;

#[cfg(not(test))]
fn run_loop(reset: bool, step: &mut InputLoop<'_, '_, '_, '_>) {
    let controls = step.input;
    if reset {
        // SAFETY: the native driver owns game-thread input for this synchronous loop.
        step.legacy(|| unsafe { FlushInput(controls) });
    }
    step.legacy(crate::mainloop::input_frame::rust_begin_input_loop);
    loop {
        // SAFETY: event dispatch and task switching run on the initialized game thread.
        step.legacy(|| unsafe {
            crate::callback::ffi::Async_process();
            crate::graphics::sdl::lifecycle_abi::TFB_ProcessEvents();
            crate::mainloop::lifecycle_abi::process_utility_keys(controls);
            super::yield_input_loop(|| crate::threading::legacy_abi::TaskSwitch());
        });
        if step
            .input(crate::engine::observe::InputView::BeforeUpdate)
            .stop
        {
            break;
        }
        // SAFETY: the driver is the sole writer of this input frame.
        unsafe { UpdateInputState(step.cx, controls) };
        let activity = step.cx.world.activity();
        if step
            .input(crate::engine::observe::InputView::AfterUpdate(activity))
            .stop
        {
            break;
        }
        // SAFETY: the native loop owns initialized input and sound resources on this thread.
        step.play_menu_sound();
        step.input_hook();
        let keep = step.frame();
        if !step.complete(keep) {
            break;
        }
    }
    if reset {
        // SAFETY: the synchronous loop still owns initialized input on this thread.
        step.legacy(|| unsafe { FlushInput(controls) });
    }
}

#[cfg(not(test))]
struct InputLoop<'a, 'b, 'c, 'd> {
    cx: &'a mut crate::engine::battle::Cx<'b>,
    io: &'a mut crate::engine::observe::Io<'c>,
    frame: &'a mut (dyn crate::engine::input::Frame + 'd),
    input: &'a crate::engine::input::InputSession,
    automation: Option<&'a crate::automation::AutomationObserver>,
}

#[cfg(not(test))]
impl InputLoop<'_, '_, '_, '_> {
    fn input(
        &mut self,
        view: crate::engine::observe::InputView,
    ) -> crate::engine::observe::Injected {
        let injected = self.frame.admit(self.cx, self.io, view);
        if let Some(observer) = self.automation {
            observer.complete_save_capture(self.cx);
        }
        injected
    }

    fn play_menu_sound(&mut self) {
        let flags = menu_sound_flags(&self.input.pulsed());
        let (first, second) = self.input.menu_sounds();
        if flags & (first | second) != 0 {
            if let Some(bank) = self.cx.kernel.menu_sound_bank() {
                let index = u16::from(flags & second != 0);
                crate::ui::settings::play_loaded_sound(
                    self.cx.world,
                    &bank.clip(index),
                    0,
                    crate::sound::types::SoundPosition::non_positional(),
                    0,
                    0,
                );
            }
        }
    }

    fn input_hook(&mut self) {
        let callback = self.input.callback();
        if let Some(callback) = callback {
            let io = &mut self.io;
            let controls = self.input.controls();
            // SAFETY: the installed function remains valid until its owner removes it.
            self.cx.with_legacy(|| unsafe {
                callback(&mut |view| io.observer.checkpoint(&view), controls)
            });
        }
    }

    fn frame(&mut self) -> bool {
        self.frame.frame(self.cx, self.io)
    }

    fn legacy<R>(&mut self, call: impl FnOnce() -> R) -> R {
        self.cx.with_legacy(call)
    }

    fn complete(&mut self, keep: bool) -> bool {
        let io = &mut self.io;
        self.cx.with_legacy(|| {
            crate::mainloop::input_frame::rust_complete_input_frame(i32::from(keep), io) != 0
        })
    }
}

#[cfg(not(test))]
pub struct NativeInputRuntime {
    observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
    input: crate::engine::input::InputSession,
}

#[cfg(not(test))]
impl NativeInputRuntime {
    pub fn new(
        observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
        controls: crate::engine::input::InputControls,
        configuration: uqm_engine::configuration::Configuration,
    ) -> Self {
        Self {
            observer,
            input: crate::engine::input::InputSession::new(controls, configuration),
        }
    }
}

#[cfg(not(test))]
impl crate::engine::input::InputRuntime for NativeInputRuntime {
    fn input(&self) -> &crate::engine::input::InputSession {
        &self.input
    }

    fn observer(
        &self,
    ) -> Box<dyn crate::engine::observe::Observer<Option<crate::engine::observe::CampaignView>>>
    {
        match &self.observer {
            Some(observer) => Box::new(crate::automation::observer::Handle(observer.clone())),
            None => Box::new(crate::engine::observe::Unobserved),
        }
    }

    fn sleep(&self, cx: &mut crate::engine::battle::Cx<'_>, ticks: u32) {
        // SAFETY: the retained runtime supplies the initialized game-thread lifecycle.
        cx.with_legacy(|| unsafe { crate::threading::pacing::linked::sleep_thread(ticks) });
    }

    fn sleep_until(&self, cx: &mut crate::engine::battle::Cx<'_>, deadline: u32) {
        // SAFETY: pacing runs callbacks and presentation inside the caller's bounded handoff.
        cx.with_legacy(|| unsafe {
            crate::threading::pacing::linked::sleep_thread_until(deadline)
        });
    }

    fn flush(&self) {
        // SAFETY: this runtime retains the initialized input buffers.
        unsafe { FlushInput(&self.input) };
    }

    fn confirm_exit(&self, cx: &mut crate::engine::battle::Cx<'_>) -> bool {
        // SAFETY: the modal driver retains its input session and borrows the active owner.
        unsafe { ConfirmExit(cx, &self.input) != 0 }
    }

    fn update(&self, cx: &mut crate::engine::battle::Cx<'_>) {
        // SAFETY: the retained context supplies initialized game-thread input resources.
        unsafe { UpdateInputState(cx, &self.input) };
    }

    fn run(
        &self,
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        reset: bool,
        frame: &mut dyn crate::engine::input::Frame,
    ) {
        run_loop(
            reset,
            &mut InputLoop {
                cx,
                io,
                frame,
                input: &self.input,
                automation: self.observer.as_deref(),
            },
        );
    }

    fn battle_input(&self, input: &crate::engine::battle::input::InputContext) -> u8 {
        // SAFETY: InputContext validates its player when installed by the game thread.
        CurrentInputToBattleInput(&self.input, input.player())
    }
}
