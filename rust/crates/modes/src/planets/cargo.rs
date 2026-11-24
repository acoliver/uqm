//! Cargo display and mineral-discard menu, ported from `planets/cargo.c`.

mod display;
pub use display::{draw_cargo, show_capacity};

fn next_element(current: u8, back: bool, forward: bool) -> u8 {
    if back {
        if current == 0 {
            7
        } else {
            current - 1
        }
    } else if forward {
        if current == 7 {
            0
        } else {
            current + 1
        }
    } else {
        current
    }
}

pub(crate) mod live {
    use super::next_element;
    use std::ffi::CString;
    use uqm_engine::input_port::frames::*;
    use uqm_engine::{battle::Cx, observe::Io};

    const CARGO_STRINGS: i32 = 162;

    fn description(cx: &mut Cx<'_>, element: u8) {
        let message = CString::new(
            cx.kernel
                .game_text((CARGO_STRINGS + 2 + i32::from(element)) as usize)
                .to_string_lossy()
                .as_ref(),
        )
        .unwrap_or_else(|error| panic!("invalid cargo description: {error}"));
        crate::ui::sis::draw_status_message(cx.kernel, cx.world, Some(&message));
    }

    struct Menu {
        current: u8,
    }

    fn discard(
        cx: &mut Cx<'_>,
        input: &uqm_engine::input::InputSession,
        menu: &mut Menu,
        display: &super::display::CargoDisplay,
    ) -> i32 {
        let pulse = input.pulsed();
        if cx.world.activity() & (1 << 14) != 0 || pulse.menu[KEY_MENU_CANCEL as usize] != 0 {
            return 0;
        }
        if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
            let amount = &mut cx.world.data.flagship.element_amounts[usize::from(menu.current)];
            if *amount != 0 {
                *amount -= 1;
                // SAFETY: the cargo menu retains the initialized campaign display.
                display.draw(cx.world, menu.current, menu.current);
                cx.world.data.flagship.total_element_mass -= 1;
                // SAFETY: the cargo menu retains the initialized capacity display.
                display.show_capacity(cx.world);
            } else {
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
            }
        } else {
            let next = next_element(
                menu.current,
                pulse.menu[KEY_MENU_UP as usize] != 0 || pulse.menu[KEY_MENU_LEFT as usize] != 0,
                pulse.menu[KEY_MENU_DOWN as usize] != 0 || pulse.menu[KEY_MENU_RIGHT as usize] != 0,
            );
            if next != menu.current {
                // SAFETY: the cargo menu retains its selected element and display resources.
                display.draw(cx.world, menu.current, next);
                description(cx, next);
                menu.current = next;
            }
        }
        cx.kernel.input_runtime().sleep(cx, 840 / 30);
        1
    }

    pub fn cargo_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let mut menu = Menu { current: 0 };
        let display = super::display::CargoDisplay::new(cx.kernel);
        display.draw(cx.world, u8::MAX, 0);
        description(cx, 0);
        input.controls().set_menu_sounds(15, 16);
        cx.run_input(io, true, |cx, _| {
            discard(cx, input, &mut menu, &display) != 0
        });
        let dimensions = cx.screen_dimensions();
        crate::ui::sis::clear_sis_rect(cx.kernel, cx.world, dimensions, 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use uqm_engine::graphics::{context::ContextLease, drawable::LoadedDrawable, font::LoadedFont};
    use uqm_engine::{
        battle::{input::InputContext, Battle, Cx},
        input::{frames::InputState, Frame, InputControls, InputRuntime, InputSession},
        observe::{Io, Observer, Unobserved},
        Game,
    };

    struct Fixture {
        game: Game,
        display: uqm_engine::test_support::campaign_display::CampaignDisplay,
        heading: LoadedFont,
        icons: LoadedDrawable,
        canvas: LoadedDrawable,
        context: ContextLease,
    }

    fn fixture() -> Fixture {
        let mut game = Game::new();
        let mut heading = None;
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install_with_resources(
                &mut game,
                |content| {
                    heading = LoadedFont::load(content, c"fixture.tiny");
                },
            )
            .unwrap();
        let heading = heading.unwrap();
        game.kernel.star_con_font = Some(heading.font());
        let icons = LoadedDrawable::create(2, 1, 1, 74).unwrap();
        game.kernel.misc_data_frame = icons.lease().frame(0).unwrap();
        let context = ContextLease::require(game.kernel.status_context);
        let canvas = LoadedDrawable::create(2, 64, 160, 1).unwrap();
        context.set_frame(canvas.lease().context_frame(0));
        Fixture {
            game,
            display,
            heading,
            icons,
            canvas,
            context,
        }
    }

    #[test]
    #[serial_test::serial]
    fn cargo_uses_supplied_campaign_and_retains_its_display_resources() {
        let Fixture {
            mut game,
            display,
            heading,
            icons,
            canvas,
            context,
        } = fixture();
        let cargo = super::display::CargoDisplay::new(&game.kernel);
        drop((icons, heading));
        let mut other = uqm_engine::Game::new();
        game.world.data.flagship.module_slots[0] =
            crate::starbase::sis_modules::ModuleType::StorageBay as u8;
        game.world.data.flagship.total_element_mass = 1;
        game.world.data.flagship.element_amounts[0] = 1;
        other.world.data.flagship.total_element_mass = 2;
        other.world.data.flagship.element_amounts[0] = 2;
        assert_eq!(super::display::remaining_capacity(&game.world), 499);
        assert_eq!(
            super::display::remaining_capacity(&other.world),
            u32::MAX - 1
        );
        let pixel = || {
            context.flush();
            let pixels = uqm_engine::test_support::native_probe::read_pixels(
                canvas.lease().frame(0).unwrap(),
                64,
                160,
            )
            .unwrap();
            let value = pixels[35 * 64 + 20];
            (value.r, value.g, value.b, value.a)
        };
        cargo.draw(&game.world, u8::MAX, 0);
        assert_eq!(pixel(), (82, 82, 255, 255));
        cargo.draw(&other.world, 0, 8);
        assert_eq!(pixel(), (0, 0, 165, 255));
        cargo.draw(&game.world, 8, u8::MAX);
        cargo.show_capacity(&other.world);
        context.flush();
        assert_eq!(game.world.data.flagship.element_amounts[0], 1);
        assert_eq!(other.world.data.flagship.element_amounts[0], 2);
        drop((cargo, context, canvas, other));
        game.kernel.star_con_font = None;
        game.kernel.misc_data_frame = uqm_engine::graphics::drawable::FrameId::NULL;
        display.finish(game);
    }

    struct Scripted {
        input: InputSession,
        script: Vec<Option<u32>>,
        sleeps: Cell<u32>,
    }

    impl InputRuntime for Scripted {
        fn input(&self) -> &InputSession {
            &self.input
        }
        fn observer(&self) -> Box<dyn Observer> {
            Box::new(Unobserved)
        }
        fn yield_poll(&self, _: &mut Cx<'_>) {
            panic!("unexpected yield")
        }
        fn sleep(&self, _: &mut Cx<'_>, ticks: u32) {
            assert_eq!(ticks, 840 / 30);
            self.sleeps.set(self.sleeps.get() + 1);
        }
        fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
            panic!("unexpected deadline")
        }
        fn flush(&self) {
            panic!("unexpected flush")
        }
        fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
            panic!("unexpected exit confirmation")
        }
        fn update(&self, _: &mut Cx<'_>) {
            panic!("unexpected update")
        }
        fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, _: bool, frame: &mut dyn Frame) {
            for (step, key) in self.script.iter().enumerate() {
                let mut state = InputState::default();
                if let Some(key) = key {
                    state.menu[*key as usize] = 1;
                }
                self.input.advance(state, state, (step as u32 + 1) * 1000);
                if !frame.frame(cx, io) {
                    assert_eq!(step, self.script.len() - 1, "menu ended early");
                    return;
                }
            }
            panic!("menu outlived its script");
        }
        fn battle_input(&self, _: &InputContext) -> u8 {
            panic!("unexpected battle input")
        }
    }

    #[test]
    #[serial_test::serial]
    fn cargo_menu_navigates_discards_and_leaves_on_cancel() {
        use uqm_engine::input_port::frames::*;
        let Fixture {
            mut game,
            display,
            heading,
            icons,
            canvas,
            context,
        } = fixture();
        let flagship = &mut game.world.data.flagship;
        for slots in [
            &mut flagship.drive_slots[..],
            &mut flagship.jet_slots[..],
            &mut flagship.module_slots[..],
        ] {
            slots.fill(u8::MAX);
        }
        let panel = LoadedDrawable::create(8, 64, 140, 100).unwrap();
        game.kernel.flag_stat_frame = panel.lease().frame(0).unwrap();
        game.world.data.flagship.total_element_mass = 3;
        game.world.data.flagship.element_amounts[0] = 2;
        game.world.data.flagship.element_amounts[1] = 1;
        // Each key is released before the next press so that repeats do not pulse.
        let script: Vec<Option<u32>> = [
            KEY_MENU_DOWN,
            KEY_MENU_SELECT,
            KEY_MENU_SELECT,
            KEY_MENU_UP,
            KEY_MENU_LEFT,
            KEY_MENU_RIGHT,
            KEY_MENU_SELECT,
            KEY_MENU_CANCEL,
        ]
        .into_iter()
        .flat_map(|key| [Some(key), None])
        .take(15)
        .collect();
        let steps = script.len() as u32;
        let runtime = Rc::new(Scripted {
            input: InputSession::new(InputControls::default(), game.kernel.configuration.clone()),
            script,
            sleeps: Cell::new(0),
        });
        game.kernel.install_input_runtime(runtime.clone());
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
        super::live::cargo_menu(&mut cx, &mut io);
        assert_eq!(runtime.sleeps.get(), steps - 1);
        assert_eq!(game.world.data.flagship.element_amounts[..2], [1, 0]);
        assert_eq!(game.world.data.flagship.total_element_mass, 1);
        drop((runtime, icons, heading, context, canvas));
        game.kernel.star_con_font = None;
        game.kernel.misc_data_frame = uqm_engine::graphics::drawable::FrameId::NULL;
        game.kernel.flag_stat_frame = uqm_engine::graphics::drawable::FrameId::NULL;
        drop(panel);
        display.finish(game);
    }

    #[test]
    fn mineral_navigation_wraps_and_back_takes_priority() {
        assert_eq!(next_element(0, true, false), 7);
        assert_eq!(next_element(7, false, true), 0);
        assert_eq!(next_element(3, true, true), 2);
        assert_eq!(next_element(3, false, false), 3);
        for i in 0..8 {
            assert!(next_element(i, false, true) < 8);
        }
    }
}
