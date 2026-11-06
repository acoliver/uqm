//! The application implementation of the kernel's typed input port.

#[cfg(not(test))]
use super::live::*;
#[cfg(not(test))]
use std::ptr;

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
        step.legacy(|| unsafe { UpdateInputState(controls) });
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
}

#[cfg(not(test))]
impl InputLoop<'_, '_, '_, '_> {
    fn input(
        &mut self,
        view: crate::engine::observe::InputView,
    ) -> crate::engine::observe::Injected {
        self.frame.admit(self.cx, self.io, view)
    }

    fn play_menu_sound(&mut self) {
        let mut sound = self.cx.kernel.menu_sounds;
        let flags = menu_sound_flags(&self.input.pulsed());
        let (first, second) = self.input.menu_sounds();
        // SAFETY: the game thread retains initialized sound resources.
        self.legacy(|| unsafe {
            if !sound.is_null() && flags & (first | second) != 0 {
                if flags & second != 0 {
                    sound = crate::strings::string_table::SetAbsStringTableIndex(sound.cast(), 1)
                        .cast();
                }
                crate::ui::settings::PlaySoundEffect(
                    sound.cast(),
                    0,
                    crate::navigation::abi::SoundPosition {
                        positional: 0,
                        x: 0,
                        y: 0,
                    },
                    ptr::null_mut(),
                    0,
                );
            }
        });
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

    fn update(&self) {
        // SAFETY: the owner publishes the initialized game-thread input scope.
        unsafe { UpdateInputState(&self.input) };
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
            },
        );
    }

    fn battle_input(&self, input: &crate::engine::battle::input::InputContext) -> u8 {
        // SAFETY: InputContext validates its player when installed by the game thread.
        CurrentInputToBattleInput(&self.input, input.player())
    }
}
