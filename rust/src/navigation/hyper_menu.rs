//! Hyperspace menu sequencing over the retained game and input owners.
use super::abi::{CHECK_ABORT, CHECK_LOAD, IN_BATTLE, KEY_MENU_SELECT, START_ENCOUNTER};
use crate::engine::{battle::Cx, observe::Io, StoryKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    Open,
    HideFlash,
    Selection(u8),
    ShowFlash,
    Close,
    ClearRadar,
    Wait,
    Restore,
    Batch,
}

fn frame(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    state: &mut u8,
    choose: &mut impl FnMut(&mut Cx<'_>, &mut u8) -> bool,
    action: &mut impl FnMut(&mut Cx<'_>, &mut Io<'_>, u8) -> bool,
    display: &mut impl FnMut(&mut Cx<'_>, &mut Io<'_>, Display),
) -> bool {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) as u16 != 0
        || cx.world.data.flagship.crew_enlisted == u16::MAX
    {
        return false;
    }
    if choose(cx, state) || !select {
        return true;
    }
    display(cx, io, Display::HideFlash);
    match *state {
        2 => {
            select = action(cx, io, *state);
            if cx.world.story_state(StoryKey::PORTAL_COUNTER) != 0
                || cx.world.activity() & START_ENCOUNTER as u16 != 0
            {
                return false;
            }
        }
        3 => {
            action(cx, io, *state);
        }
        4 => select = action(cx, io, *state),
        5 => {
            if !action(cx, io, *state) {
                return false;
            }
        }
        1 => {
            action(cx, io, *state);
            return false;
        }
        6 => return false,
        _ => {}
    }
    if cx.world.activity() & CHECK_ABORT as u16 == 0 {
        if select {
            if input.preferences().menu != 2 {
                *state = 6;
            }
            display(cx, io, Display::Selection(*state));
        }
        display(cx, io, Display::ShowFlash);
    }
    true
}

pub fn run(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    mut choose: impl FnMut(&mut Cx<'_>, &mut u8) -> bool,
    mut action: impl FnMut(&mut Cx<'_>, &mut Io<'_>, u8) -> bool,
    mut display: impl FnMut(&mut Cx<'_>, &mut Io<'_>, Display),
) {
    display(cx, io, Display::Open);
    let mut state = 1;
    cx.run_input(io, true, |cx, io| {
        frame(cx, io, &mut state, &mut choose, &mut action, &mut display)
    });
    display(cx, io, Display::Close);
    if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) as u16 == 0 {
        display(cx, io, Display::ClearRadar);
        display(cx, io, Display::Wait);
    }
    display(cx, io, Display::Restore);
    if cx.world.activity() & IN_BATTLE as u16 == 0 {
        super::hyper_transition::cleanup(cx);
    }
    display(cx, io, Display::Batch);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::input::frames::InputState;
    use crate::engine::{
        battle::{input::InputContext, Battle},
        input::{Frame, InputControls, InputRuntime, InputSession},
        observe::{CampaignView, Observer, Unobserved},
        Game,
    };
    use std::rc::Rc;

    struct Driver(InputSession);
    impl InputRuntime for Driver {
        fn input(&self) -> &InputSession {
            &self.0
        }
        fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
            Box::new(Unobserved)
        }
        fn update(&self, _: &mut Cx<'_>) {}
        fn sleep(&self, _: &mut Cx<'_>, _: u32) {
            panic!("unexpected menu sleep")
        }
        fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
            panic!("unexpected menu deadline sleep")
        }
        fn flush(&self) {
            panic!("unexpected menu flush")
        }
        fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
            panic!("unexpected menu exit confirmation")
        }
        fn battle_input(&self, _: &InputContext) -> u8 {
            0
        }
        fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
            assert!(reset);
            assert!(!frame.frame(cx, io));
        }
    }
    fn fixture(test: impl FnOnce(&mut Cx<'_>, &mut Io<'_>)) {
        let mut game = Game::new();
        game.world.story = Some(crate::engine::world::story::GameState::new());
        let controls = InputControls::default();
        let mut pressed = InputState::default();
        pressed.menu[KEY_MENU_SELECT as usize] = 1;
        controls.advance(pressed, pressed, 1);
        game.kernel
            .install_input_runtime(Rc::new(Driver(InputSession::new(
                controls,
                game.kernel.configuration.clone(),
            ))));
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
            panic!("menu RNG")
        });
        let mut observer = Unobserved;
        test(
            &mut cx,
            &mut Io {
                observer: &mut observer,
            },
        );
    }

    #[test]
    fn abort_load_and_dead_crew_exit_before_menu_selection() {
        fixture(|cx, io| {
            for (activity, crew) in [
                (CHECK_ABORT as u16, 0),
                (CHECK_LOAD as u16, 0),
                (0, u16::MAX),
            ] {
                cx.world.set_activity(activity);
                cx.world.data.flagship.crew_enlisted = crew;
                assert!(!frame(
                    cx,
                    io,
                    &mut 1,
                    &mut |_, _| panic!("unexpected choice"),
                    &mut |_, _, _| panic!("unexpected submenu"),
                    &mut |_, _, _| panic!("unexpected drawing")
                ));
            }
        });
    }

    #[test]
    fn menu_dispatch_preserves_selection_and_early_returns() {
        fixture(|cx, io| {
            for selection in 1..=6 {
                for menu in [0, 2] {
                    cx.kernel
                        .configuration
                        .update_preferences(|p| p.menu = menu);
                    let mut state = selection;
                    let mut calls = Vec::new();
                    let mut display = Vec::new();
                    let keep = frame(
                        cx,
                        io,
                        &mut state,
                        &mut |_, _| false,
                        &mut |_, _, choice| {
                            calls.push(choice);
                            true
                        },
                        &mut |_, _, event| display.push(event),
                    );
                    assert_eq!(keep, matches!(selection, 2..=5));
                    assert_eq!(
                        calls,
                        if selection == 6 {
                            vec![]
                        } else {
                            vec![selection]
                        }
                    );
                    assert_eq!(display[0], Display::HideFlash);
                    if keep {
                        assert_eq!(state, if menu == 2 { selection } else { 6 });
                        assert_eq!(
                            display,
                            [
                                Display::HideFlash,
                                Display::Selection(state),
                                Display::ShowFlash
                            ]
                        );
                    } else {
                        assert_eq!(display, [Display::HideFlash]);
                    }
                }
            }
            for portal in [false, true] {
                cx.world.set_activity(0);
                let mut events = Vec::new();
                assert!(!frame(
                    cx,
                    io,
                    &mut 2,
                    &mut |_, _| false,
                    &mut |cx, _, _| {
                        if portal {
                            cx.world.set_story_state(StoryKey::PORTAL_COUNTER, 1);
                        } else {
                            cx.world.set_activity(START_ENCOUNTER as u16);
                        }
                        true
                    },
                    &mut |_, _, event| events.push(event)
                ));
                assert_eq!(events, [Display::HideFlash]);
            }
        });
    }

    #[test]
    fn exit_restores_the_display_after_wait_and_before_rebatching() {
        fixture(|cx, io| {
            let mut events = Vec::new();
            run(
                cx,
                io,
                |_, state| {
                    *state = 6;
                    false
                },
                |_, _, _| panic!("exit opened a submenu"),
                |_, _, event| events.push(event),
            );
            assert_eq!(
                events,
                [
                    Display::Open,
                    Display::HideFlash,
                    Display::Close,
                    Display::ClearRadar,
                    Display::Wait,
                    Display::Restore,
                    Display::Batch
                ]
            );
        });
    }
}
