//! The application implementation of the kernel's typed input port.

#[cfg(not(test))]
use super::live::*;
#[cfg(not(test))]
use std::ptr;

#[cfg(not(test))]
fn run_loop(reset: bool, step: &mut InputLoop<'_, '_, '_, '_>) {
    if reset {
        // SAFETY: the native driver owns game-thread input for this synchronous loop.
        step.legacy(|| unsafe { FlushInput() });
    }
    step.legacy(crate::mainloop::input_frame::rust_begin_input_loop);
    loop {
        // SAFETY: event dispatch and task switching run on the initialized game thread.
        step.legacy(|| unsafe {
            crate::callback::ffi::Async_process();
            crate::graphics::sdl::lifecycle_abi::TFB_ProcessEvents();
            crate::mainloop::lifecycle_abi::process_utility_keys();
            super::yield_input_loop(|| crate::threading::legacy_abi::TaskSwitch());
        });
        if step
            .input(crate::engine::observe::InputView::BeforeUpdate)
            .stop
        {
            break;
        }
        // SAFETY: the driver is the sole writer of this input frame.
        step.legacy(|| unsafe { UpdateInputState() });
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
        step.legacy(|| unsafe { FlushInput() });
    }
}

#[cfg(not(test))]
struct InputLoop<'a, 'b, 'c, 'd> {
    cx: &'a mut crate::engine::battle::Cx<'b>,
    io: &'a mut crate::engine::observe::Io<'c>,
    frame: &'a mut crate::engine::input::Frame<'d>,
}

#[cfg(not(test))]
impl InputLoop<'_, '_, '_, '_> {
    fn input(
        &mut self,
        view: crate::engine::observe::InputView,
    ) -> crate::engine::observe::Injected {
        let campaign = crate::engine::observe::CampaignView::for_input(view, || {
            crate::engine::observe::CampaignView::capture(self.cx.world)
        });
        self.cx
            .with_legacy(|| self.io.observer.input_admitted(&view, &campaign))
    }

    fn play_menu_sound(&mut self) {
        let mut sound = self.cx.kernel.menu_sounds;
        // SAFETY: the game thread owns the input snapshot and initialized sound resources.
        self.legacy(|| unsafe {
            let flags = menu_sound_flags(&ptr::addr_of!(PulsedInputState).read());
            if !sound.is_null() && flags & (SOUND_0 | SOUND_1) != 0 {
                if flags & SOUND_1 != 0 {
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
        // SAFETY: installation and invocation are serialized on the game thread.
        let callback = unsafe { INPUT_CALLBACK };
        if let Some(callback) = callback {
            let io = &mut self.io;
            // SAFETY: the installed function remains valid until its owner removes it.
            self.cx.with_legacy(|| unsafe { callback(io) });
        }
    }

    fn frame(&mut self) -> bool {
        (self.frame)(self.cx, self.io)
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
    pub observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
}

#[cfg(not(test))]
impl crate::engine::input::InputRuntime for NativeInputRuntime {
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
        unsafe { UpdateInputState() };
    }

    fn run(
        &self,
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        reset: bool,
        frame: &mut crate::engine::input::Frame<'_>,
    ) {
        run_loop(reset, &mut InputLoop { cx, io, frame });
    }

    fn battle_input(&self, input: &crate::engine::battle::input::InputContext) -> u8 {
        // SAFETY: InputContext validates its player when installed by the game thread.
        unsafe { CurrentInputToBattleInput(input.player()) }
    }
}
