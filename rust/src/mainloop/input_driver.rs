//! The application implementation of the kernel's typed input port.

use super::live::*;

fn run_loop(reset: bool, step: &mut InputLoop<'_, '_, '_, '_>) {
    let controls = step.input;
    if reset {
        flush_input(controls);
    }
    crate::mainloop::input_frame::rust_begin_input_loop();
    loop {
        uqm_engine::host::process_async();
        uqm_engine::graphics::events::process_events();
        process_utility_keys(controls);
        super::yield_input_loop(uqm_engine::threading::yield_task);
        if step
            .input(uqm_engine::observe::InputView::BeforeUpdate)
            .stop
        {
            break;
        }
        update_input_state(step.cx, controls);
        let activity = step.cx.world.activity();
        if step
            .input(uqm_engine::observe::InputView::AfterUpdate(activity))
            .stop
        {
            break;
        }
        step.play_menu_sound();
        step.input_hook();
        let keep = step.frame();
        if !step.complete(keep) {
            break;
        }
    }
    if reset {
        flush_input(controls);
    }
}

/// The abort and fullscreen utility keys, applied after immediate input is sampled.
fn process_utility_keys(input: &uqm_engine::input::InputSession) {
    use uqm_engine::input::frames::{KEY_ABORT, KEY_FULLSCREEN};
    let state = input.immediate();
    if state.menu[KEY_ABORT as usize] != 0 {
        uqm_engine::log_abi::log_show_box(false, false);
        std::process::exit(0);
    }
    if state.menu[KEY_FULLSCREEN as usize] != 0 {
        flush_input(input);
        uqm_engine::graphics::present::toggle_fullscreen();
    }
}

struct InputLoop<'a, 'b, 'c, 'd> {
    cx: &'a mut uqm_engine::battle::Cx<'b>,
    io: &'a mut uqm_engine::observe::Io<'c>,
    frame: &'a mut (dyn uqm_engine::input::Frame + 'd),
    input: &'a uqm_engine::input::InputSession,
    automation: Option<&'a crate::automation::AutomationObserver>,
}

impl InputLoop<'_, '_, '_, '_> {
    fn input(&mut self, view: uqm_engine::observe::InputView) -> uqm_engine::observe::Injected {
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
                uqm_modes::ui::settings::play_loaded_sound(
                    self.cx.world,
                    &bank.clip(index),
                    0,
                    uqm_engine::sound::types::SoundPosition::non_positional(),
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
            callback.call(&mut |view| io.observer.checkpoint(&view), controls);
        }
    }

    fn frame(&mut self) -> bool {
        self.frame.frame(self.cx, self.io)
    }

    fn complete(&mut self, keep: bool) -> bool {
        crate::mainloop::input_frame::rust_complete_input_frame(self.cx, i32::from(keep), self.io)
            != 0
    }
}

pub struct NativeInputRuntime {
    observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
    input: uqm_engine::input::InputSession,
}

impl NativeInputRuntime {
    pub fn new(
        observer: Option<std::sync::Arc<crate::automation::AutomationObserver>>,
        controls: uqm_engine::input::InputControls,
        configuration: uqm_engine::configuration::Configuration,
    ) -> Self {
        Self {
            observer,
            input: uqm_engine::input::InputSession::new(controls, configuration),
        }
    }
}

impl uqm_engine::input::InputRuntime for NativeInputRuntime {
    fn input(&self) -> &uqm_engine::input::InputSession {
        &self.input
    }

    fn observer(&self) -> Box<dyn uqm_engine::observe::Observer> {
        match &self.observer {
            Some(observer) => Box::new(crate::automation::observer::Handle(observer.clone())),
            None => Box::new(uqm_engine::observe::Unobserved),
        }
    }

    fn yield_poll(&self, _cx: &mut uqm_engine::battle::Cx<'_>) {
        uqm_engine::host::process_async();
        uqm_engine::threading::yield_task();
    }

    fn sleep(&self, _cx: &mut uqm_engine::battle::Cx<'_>, ticks: u32) {
        uqm_engine::threading::pacing::linked::sleep_thread(ticks);
    }

    fn sleep_until(&self, _cx: &mut uqm_engine::battle::Cx<'_>, deadline: u32) {
        uqm_engine::threading::pacing::linked::sleep_thread_until(deadline);
    }

    fn flush(&self) {
        flush_input(&self.input);
    }

    fn confirm_exit(&self, cx: &mut uqm_engine::battle::Cx<'_>) -> bool {
        confirm_exit(cx, &self.input) != 0
    }

    fn update(&self, cx: &mut uqm_engine::battle::Cx<'_>) {
        update_input_state(cx, &self.input);
    }

    fn run(
        &self,
        cx: &mut uqm_engine::battle::Cx<'_>,
        io: &mut uqm_engine::observe::Io<'_>,
        reset: bool,
        frame: &mut dyn uqm_engine::input::Frame,
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

    fn battle_input(&self, input: &uqm_engine::battle::input::InputContext) -> u8 {
        self.input.current_battle_input(input.player())
    }
}
