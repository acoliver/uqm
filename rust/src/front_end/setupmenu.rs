//! Owned setup session and persistent options, ported from uqm/setupmenu.c.
use crate::engine::{battle::Cx, input::InputSession, observe::Io};
use crate::ui::getchar::{edit_text, TextEntryCallbacks, TextEntryView};
use std::ffi::{CStr, CString};
use uqm_engine::{
    configuration::runtime::RuntimeSettings,
    input_port::{sdl_input, templates},
    resource::settings::Settings,
    time::counter_abi::GetTimeCounter,
};

mod drawing;
mod gamma;
mod options;
mod strings;
mod widgets;
pub use drawing::SetupResources;
use drawing::View;
pub use gamma::GammaRange;
pub use options::{get_global_options, set_global_options, GlobalOptions};
#[cfg(test)]
use strings::split_lines;
use widgets::{Action, Event, MenuState};

const FRAME: u32 = 840 / 20;

pub struct SetupConfiguration {
    pub input: InputSession,
    pub runtime: RuntimeSettings,
    pub settings: Settings,
    pub names: [CString; 6],
}

#[derive(Clone, Default)]
struct NameEntry {
    text: CString,
    cursor: usize,
    editing: bool,
    block: bool,
}

struct SetupSession {
    configuration: SetupConfiguration,
    resources: SetupResources,
    menu: MenuState,
    gamma: GammaRange,
    names: [CString; 6],
    controls: [[CString; 2]; 7],
    entry: NameEntry,
    next_time: u32,
    initialized: bool,
    transition: bool,
    done: bool,
}

impl SetupSession {
    fn new(configuration: SetupConfiguration, resources: SetupResources) -> Self {
        let mut gamma = GammaRange {
            min: 0.4,
            max: 2.5,
            min_x: 0.0,
            max_x: 0.0,
        };
        let names = configuration.names.clone();
        let options = get_global_options(
            &configuration.input,
            configuration.runtime.display,
            configuration.runtime.audio,
            configuration.settings.boolean("config.alwaysgl"),
            &mut gamma,
        );
        let mut counts =
            std::array::from_fn(|index| resources.strings.choices[index].options.len());
        counts[0] = if uqm_engine::graphics::sdl::platform_abi::TFB_SupportsHardwareScaling() != 0 {
            6
        } else {
            2
        };
        counts[0] += usize::from(options.res == 6);
        let mut menu = MenuState::new(counts);
        menu.choices = options.choices();
        menu.sliders = options.sliders();
        Self {
            configuration,
            resources,
            menu,
            gamma,
            names,
            controls: std::array::from_fn(|_| std::array::from_fn(|_| CString::default())),
            entry: NameEntry::default(),
            next_time: 0,
            initialized: false,
            transition: true,
            done: false,
        }
    }

    fn view(&self) -> View<'_> {
        View {
            menu: &self.menu,
            names: &self.names,
            controls: &self.controls,
            entry: &self.entry,
            gamma: self.gamma.slider_to_gamma(self.menu.sliders[3]),
        }
    }

    fn draw(&self) {
        self.resources.draw(&self.view());
    }

    fn populate_keys(&mut self) {
        let template = self.menu.choices[20];
        self.entry.text = self.names[template].clone();
        self.controls = std::array::from_fn(|control| {
            std::array::from_fn(|alternate| {
                let name =
                    sdl_input::control_name(template as i32, control as i32, alternate as i32);
                CString::new(name).unwrap_or_else(|_| panic!("binding description contains NUL"))
            })
        });
    }

    fn act(&mut self, action: Action, cx: &mut Cx<'_>, io: &mut Io<'_>) {
        match action {
            Action::None => (),
            Action::Finish => self.done = true,
            Action::Enter(menu, down) => {
                self.menu.enter(menu, down);
                self.transition = true;
            }
            Action::EditKeys => {
                self.menu.choices[20] = 0;
                self.populate_keys();
                self.menu.enter(7, true);
                self.transition = true;
            }
            Action::ChoiceChanged(20) => self.populate_keys(),
            Action::ChoiceChanged(_) => (),
            Action::GammaStep(direction) => {
                self.gamma
                    .step(&mut self.menu.sliders[3], direction, |gamma| {
                        uqm_engine::graphics::sdl::platform_abi::TFB_SetGamma(gamma)
                    })
            }
            Action::EditText => self.edit(cx, io),
            Action::Rebind(control) => {
                let runtime = cx.kernel.input_runtime();
                runtime.flush();
                self.resources.binding_prompt();
                sdl_input::RebindInputState(
                    self.menu.choices[20] as i32,
                    control as i32,
                    self.menu.binding_column as i32,
                );
                self.populate_keys();
                runtime.flush();
            }
            Action::Clear(control) => {
                sdl_input::RemoveInputState(
                    self.menu.choices[20] as i32,
                    control as i32,
                    self.menu.binding_column as i32,
                );
                self.populate_keys();
            }
        }
    }

    fn edit(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) {
        let saved = self.entry.clone();
        let mut buffer = [0u8; 50];
        let bytes = saved.text.as_bytes_with_nul();
        buffer[..bytes.len()].copy_from_slice(bytes);
        self.entry.cursor = saved.text.to_string_lossy().chars().count();
        self.entry.editing = true;
        self.entry.block = false;
        self.draw();
        let cursor = self.entry.cursor;
        let input = self.configuration.input.clone();
        input.set_menu_sounds(0, 1 << 4);
        let mut callbacks = SetupEditor {
            session: self,
            next_time: GetTimeCounter().wrapping_add(FRAME),
        };
        let accepted = edit_text(cx, io, &input, 49, &mut buffer, cursor, &mut callbacks);
        self.entry = saved;
        if accepted {
            self.entry.text = CStr::from_bytes_until_nul(&buffer)
                .unwrap_or_else(|_| panic!("edited template not terminated"))
                .to_owned();
            let template = self.menu.choices[20];
            self.names[template] = templates::rename_template(template, &self.entry.text);
        }
        input.set_menu_sounds(15, 1 << 4);
        self.draw();
    }

    fn finish(&self) {
        let input = &self.configuration.input;
        let settings = &self.configuration.settings;
        let options = GlobalOptions::from_widgets(&self.menu.choices, self.menu.sliders);
        let current = RuntimeSettings::capture(&self.resources.context);
        let display = options::prepare_display(&options, current.display, settings);
        if display != current.display {
            RuntimeSettings::apply(&self.resources.context, Some(display), None);
        }
        let gamma = self.gamma.slider_to_gamma(options.gamma);
        if input.preferences().gamma != 1.0 || gamma != 1.0 {
            if uqm_engine::graphics::sdl::platform_abi::TFB_SetGamma(gamma) {
                log::info!("Gamma correction set to {gamma:.4}.");
            } else {
                log::warn!("Unable to set gamma correction.");
            }
        }
        set_global_options(input, &options, &self.gamma, settings);
        RuntimeSettings::apply(
            &self.resources.context,
            None,
            Some([
                options.musicvol as f32 / 100.0,
                options.sfxvol as f32 / 100.0,
                options.speechvol as f32 / 100.0,
            ]),
        );
        for (index, name) in self.names.iter().enumerate() {
            settings.put_string(
                &format!("keys.{}.name", index + 1),
                name.to_str().unwrap_or_else(|_| panic!("template UTF-8")),
            );
        }
        for (file, prefix) in [(c"uqm.cfg", "config."), (c"flight.cfg", "keys.")] {
            if let Err(error) = settings.save(&self.configuration.runtime.directory, file, prefix) {
                log::warn!("Could not save {}: {error}", file.to_string_lossy());
            }
        }
    }
}

struct SetupEditor<'a> {
    session: &'a mut SetupSession,
    next_time: u32,
}
impl TextEntryCallbacks for SetupEditor<'_> {
    fn changed(
        &mut self,
        entry: TextEntryView<'_>,
        _cx: &mut Cx<'_>,
        _input: &InputSession,
    ) -> bool {
        self.session.entry.text = entry.text.to_owned();
        self.session.entry.cursor = entry.cursor;
        self.session.entry.block = entry.joystick;
        true
    }
    fn frame(
        &mut self,
        _entry: TextEntryView<'_>,
        cx: &mut Cx<'_>,
        _io: &mut Io<'_>,
        _input: &InputSession,
    ) -> bool {
        self.session.draw();
        cx.kernel.input_runtime().sleep_until(cx, self.next_time);
        self.next_time = GetTimeCounter().wrapping_add(FRAME);
        true
    }
}

fn do_setup_menu(state: &mut SetupSession, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool {
    state.configuration.input.set_paused(false);
    if !state.initialized {
        state
            .configuration
            .input
            .configure_repeat(70, 420, 70, false, GetTimeCounter());
        state.next_time = GetTimeCounter();
        state.initialized = true;
    }
    let context = &state.resources.context;
    if state.transition {
        context.transition_source(None);
    }
    context.draw_batch(|| {
        state.draw();
        if state.transition {
            context.transition(None);
        }
    });
    state.transition = false;
    if let Some(event) = Event::direction(&state.configuration.input) {
        let action = state.menu.event(event);
        state.act(action, cx, io);
    }
    for (key, event) in [(9, Event::Select), (10, Event::Cancel), (18, Event::Delete)] {
        if state.configuration.input.pulsed().menu[key] != 0 {
            let action = state.menu.event(event);
            state.act(action, cx, io);
        }
    }
    cx.kernel
        .input_runtime()
        .sleep_until(cx, state.next_time.wrapping_add(FRAME));
    state.next_time = GetTimeCounter();
    !state.done && cx.world.activity() & 0x4000 == 0
}

pub fn setup_menu(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    configuration: SetupConfiguration,
    resources: SetupResources,
) {
    configuration.input.set_menu_sounds(15, 1 << 4);
    let mut session = SetupSession::new(configuration, resources);
    cx.run_input(io, true, |cx, io| do_setup_menu(&mut session, cx, io));
    cx.world.set_activity(cx.world.activity() & !0x4000);
    session.finish();
}

pub mod runtime {
    pub use super::{get_global_options, set_global_options, setup_menu};
}

#[cfg(test)]
mod tests;
