//! Real virtual controls, restart dispatch, input reset and save enumeration.
#![forbid(unsafe_code)]
#[path = "support/battle_context.rs"]
mod battle_context;
use uqm_engine::input_port::frames::Target;
use uqm_engine::save::summary_record::SummaryDesc as SUMMARY_DESC;
use uqm_modes::front_end::gameopt::{picker_action, PickerAction};
use uqm_rust::{
    mainloop::{
        input_abi::{flush_input, update_input_state},
        restart_menu::{
            do_restart::{do_restart_frame, DoRestartState},
            restart_ops::{RestartHost, RestartMenu, RestartMenuOps},
        },
    },
    savegame::live::load::load_game_descriptions,
    savegame::{decode_save, SaveSlots},
};

struct NoLifecycle;
impl RestartHost for NoLifecycle {
    fn setup(
        &mut self,
        _: &mut uqm_engine::battle::Cx<'_>,
        _: &mut uqm_engine::observe::Io<'_>,
        _: uqm_engine::graphics::context::ContextLease,
        _: uqm_engine::graphics::font::FontLease,
    ) {
        panic!("unexpected setup");
    }
    fn melee(&mut self, _: &mut uqm_engine::battle::Cx<'_>, _: &mut uqm_engine::observe::Io<'_>) {
        panic!("unexpected melee");
    }
    fn credits(
        &mut self,
        _: &mut uqm_engine::battle::Cx<'_>,
        _: &mut uqm_engine::observe::Io<'_>,
        _: bool,
    ) {
        panic!("unexpected credits");
    }
    fn free_game_data(&mut self, _: &mut uqm_engine::battle::Cx<'_>) {
        panic!("unexpected cleanup");
    }
}

const SELECT: usize = 9;
const CANCEL: usize = 10;
const RETURN: i32 = 13;
const ESCAPE: i32 = 27;

fn key_event(input: &uqm_engine::input::InputSession, symbol: i32, down: bool, repeat: bool) {
    input.vcontrol(|vc| match (down, repeat) {
        (true, false) => vc.handle_key_down(symbol),
        (true, true) => {}
        (false, _) => vc.handle_key_up(symbol),
    });
}

struct PickerProbe {
    occupied: bool,
    actions: Vec<PickerAction>,
    initial_pulses: (i32, i32),
    repeat_pulses: (i32, i32),
}

fn picker_frame(
    cx: &mut uqm_engine::battle::Cx<'_>,
    state: &mut PickerProbe,
    input: &uqm_engine::input::InputSession,
) -> u32 {
    let select = input.pulsed().menu[SELECT];
    let cancel = input.pulsed().menu[CANCEL];
    if state.actions.is_empty() {
        state.initial_pulses = (select, cancel);
    }
    let action = picker_action(false, cancel != 0, select != 0, false, state.occupied);
    state.actions.push(action);
    match action {
        PickerAction::Continue if state.actions.len() == 1 => {
            key_event(input, RETURN, true, true);
            key_event(input, ESCAPE, true, true);
            update_input_state(cx, input);
            state.repeat_pulses = (input.pulsed().menu[SELECT], input.pulsed().menu[CANCEL]);
            key_event(input, RETURN, false, false);
            key_event(input, ESCAPE, false, false);
            key_event(input, RETURN, true, false);
            1
        }
        PickerAction::EmptySlot => {
            key_event(input, RETURN, false, false);
            key_event(input, ESCAPE, true, false);
            1
        }
        _ => 0,
    }
}

fn open_from_restart_while_select_is_held(input: &uqm_engine::input::InputSession, occupied: bool) {
    flush_input(input);
    crate::battle_context::with(|cx| cx.world.set_activity(0));
    key_event(input, RETURN, true, false);
    crate::battle_context::with(|cx| update_input_state(cx, input));
    assert_ne!(input.pulsed().menu[SELECT], 0);
    let context = uqm_engine::graphics::context::LoadedContext::create(c"load-picker-restart");
    let mut owner = RestartMenu::default();
    let mut host = NoLifecycle;
    let mut menu = DoRestartState {
        initialized: true,
        cur_state: 1,
        inact_timeout: u32::MAX,
        ..Default::default()
    };
    crate::battle_context::with(|cx| {
        let previous = cx.kernel.screen_context;
        cx.kernel.screen_context = Some(context.context());
        let mut observer = uqm_engine::observe::Unobserved;
        let mut io = uqm_engine::observe::Io {
            observer: &mut observer,
        };
        {
            let mut ops = owner.operations(cx, &mut io, &mut host);
            ops.set_screen_context();
            ops.create_flash_overlay();
            assert!(!do_restart_frame(&mut ops, &mut menu));
        }
        cx.kernel.screen_context = previous;
    });
    assert_eq!(
        crate::battle_context::with(|cx| cx.kernel.last_activity),
        0x1000
    );
    assert_eq!(crate::battle_context::with(|cx| cx.world.activity()), 4);
    // A queued Cancel is cleared at the same ownership boundary as Select.
    key_event(input, ESCAPE, true, false);
    let mut picker = PickerProbe {
        occupied,
        actions: Vec::new(),
        initial_pulses: (-1, -1),
        repeat_pulses: (-1, -1),
    };
    // No key-up for the menu's Select has happened before picker entry.
    crate::battle_context::with(|cx| {
        let mut observer = uqm_engine::observe::Unobserved;
        let mut io = uqm_engine::observe::Io {
            observer: &mut observer,
        };
        cx.run_input(&mut io, true, |cx, _| {
            picker_frame(cx, &mut picker, input) != 0
        });
    });
    assert_eq!(picker.initial_pulses, (0, 0));
    assert_eq!(picker.repeat_pulses, (0, 0));
    let expected = if occupied {
        vec![PickerAction::Continue, PickerAction::Select]
    } else {
        vec![
            PickerAction::Continue,
            PickerAction::EmptySlot,
            PickerAction::Cancel,
        ]
    };
    assert_eq!(picker.actions, expected);
    key_event(input, RETURN, false, false);
    key_event(input, ESCAPE, false, false);
}

#[test]
fn first_load_entry_flushes_held_keys_and_enumeration_cannot_exit_the_picker() {
    battle_context::run(|input| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tmp");
        std::fs::create_dir_all(&root).unwrap();
        let profile = tempfile::tempdir_in(root).unwrap();
        {
            uqm_engine::graphics::state::init_headless(Default::default());
            input.set_default_menu_repeat(uqm_engine::time::counter_abi::get_time_counter());
            uqm_engine::input_port::sdl_input::attach_controls(input.controls());
            input.vcontrol(|vc| {
                vc.init();
                assert!(vc.add_key_binding(RETURN, Target::Menu(SELECT)));
                assert!(vc.add_key_binding(ESCAPE, Target::Menu(CANCEL)));
            });

            let mut summaries = [SUMMARY_DESC::default(); 50];
            assert!(!profile.path().join("save").exists());
            load_game_descriptions(&mut summaries);
            assert!(summaries.iter().all(|summary| summary.year_index == 0));
            open_from_restart_while_select_is_held(input, false);

            let saves = profile.path().join("save");
            std::fs::create_dir(&saves).unwrap();
            let vfs = uqm_engine::io::Vfs::new();
            vfs.mount_dir("/", &saves, uqm_engine::io::Placement::Top, false);
            let files = vfs.open_dir(std::path::Path::new("/"));
            vfs.set_save_dir(files.clone());
            let previous = uqm_engine::io::install(vfs);
            load_game_descriptions(&mut summaries);
            assert!(summaries.iter().all(|summary| summary.year_index == 0));
            let slots = SaveSlots::new(&files);
            let save = decode_save(include_bytes!("fixtures/savegame/legacy-save.00")).unwrap();
            for slot in 0..50 {
                if slot % 3 == 1 {
                    std::fs::write(saves.join(format!("uqmsave.{slot:02}")), b"corrupt save")
                        .unwrap();
                } else if slot % 3 == 0 {
                    slots.write(slot, &save).unwrap();
                }
            }
            // The old header has alignment words after SIS and at its end.
            // A valid legacy summary must win over an unreadable current slot.
            let modern = uqm_rust::savegame::encode_save(&save).unwrap();
            let body = &modern[12..172];
            let mut legacy = [0xa5; 164];
            legacy[..122].copy_from_slice(&body[..122]);
            legacy[124..162].copy_from_slice(&body[122..160]);
            std::fs::write(saves.join("starcon2.49"), legacy).unwrap();
            std::fs::write(saves.join("starcon2.47"), b"bad").unwrap();
            for _ in 0..10 {
                crate::battle_context::with(|cx| cx.world.set_activity(4));
                load_game_descriptions(&mut summaries);
                assert_eq!(
                    crate::battle_context::with(|cx| cx.world.activity()),
                    4,
                    "enumeration must not set abort/load flags"
                );
                for (slot, summary) in summaries.iter().enumerate() {
                    assert_eq!(
                        summary.year_index != 0,
                        slot % 3 == 0 || slot == 49,
                        "slot {slot}"
                    );
                }
                open_from_restart_while_select_is_held(input, true);
                open_from_restart_while_select_is_held(input, false);
            }
            assert_eq!(
                picker_action(true, true, true, false, true),
                PickerAction::Abort
            );
            assert_eq!(
                picker_action(false, true, true, false, true),
                PickerAction::Cancel
            );
            assert_eq!(
                picker_action(false, false, true, true, false),
                PickerAction::Select
            );
            match previous {
                Some(previous) => {
                    uqm_engine::io::install(previous);
                }
                None => {
                    uqm_engine::io::uninstall();
                }
            }
            uqm_engine::input_port::sdl_input::uninit_input();
            uqm_engine::graphics::state::uninit_headless();
        }
    });
}
