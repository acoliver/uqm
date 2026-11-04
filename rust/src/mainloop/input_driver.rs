//! The application implementation of the kernel's typed input port.

use super::live::*;
use std::ptr;

unsafe fn run_loop(reset: bool, step: &mut InputLoop<'_, '_, '_, '_>) {
    // SAFETY: the native driver owns game-thread input for this synchronous loop.
    unsafe {
        if reset {
            step.legacy(|| FlushInput());
        }
        step.legacy(|| crate::mainloop::input_frame::rust_begin_input_loop());
        loop {
            step.legacy(|| {
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
            step.legacy(|| UpdateInputState());
            if step
                .input(crate::engine::observe::InputView::AfterUpdate)
                .stop
            {
                break;
            }
            let flags = menu_sound_flags(&ptr::addr_of!(PulsedInputState).read());
            let mut sound = step.menu_sounds();
            step.legacy(|| {
                if !sound.is_null() && flags & (SOUND_0 | SOUND_1) != 0 {
                    if flags & SOUND_1 != 0 {
                        sound =
                            crate::strings::string_table::SetAbsStringTableIndex(sound.cast(), 1)
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
                if let Some(callback) = INPUT_CALLBACK {
                    callback();
                }
            });
            let keep = step.frame();
            if !step.complete(keep) {
                break;
            }
        }
        if reset {
            step.legacy(|| FlushInput());
        }
    }
}

struct InputLoop<'a, 'b, 'c, 'd> {
    cx: &'a mut crate::engine::battle::Cx<'b>,
    io: &'a mut crate::engine::observe::Io<'c>,
    frame: &'a mut crate::engine::input::Frame<'d>,
}

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

    fn menu_sounds(&mut self) -> crate::graphics::native_abi::STRING {
        self.cx.kernel.menu_sounds
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

pub struct NativeInputRuntime {
    pub observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
}

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

    fn run(
        &self,
        cx: &mut crate::engine::battle::Cx<'_>,
        io: &mut crate::engine::observe::Io<'_>,
        reset: bool,
        frame: &mut crate::engine::input::Frame<'_>,
    ) {
        // SAFETY: the application runs this driver on the game thread with initialized input.
        unsafe { run_loop(reset, &mut InputLoop { cx, io, frame }) }
    }

    fn battle_input(&self, input: &crate::engine::battle::input::InputContext) -> u8 {
        // SAFETY: InputContext validates its player when installed by the game thread.
        unsafe { CurrentInputToBattleInput(input.player()) }
    }
}
