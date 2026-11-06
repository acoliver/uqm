//! Real virtual controls, restart dispatch, DoInput reset and save enumeration.
use uqm_rust::graphics::flash::FlashContext;
#[path = "support/battle_context.rs"]
mod battle_context;
use std::{ffi::CString, ptr};
use uqm_rust::{
    front_end::gameopt::{load_game_descriptions, picker_action, PickerAction},
    graphics::dcqueue_abi::{Init_DrawCommandQueue, Uninit_DrawCommandQueue},
    input::ffi::*,
    io::uio_bridge as uio,
    mainloop::{
        dir_prep::saveDir,
        input_abi::{DoInput, FlushInput, SetDefaultMenuRepeatDelay, UpdateInputState},
        restart_menu::{
            do_restart::{do_restart_frame, DoRestartState},
            restart_ops::CffiOps,
        },
    },
    navigation::abi::SUMMARY_DESC,
    savegame::{decode_save, SaveSlots},
};

const SELECT: usize = 9;
const CANCEL: usize = 10;
const RETURN: i32 = 13;
const ESCAPE: i32 = 27;

unsafe fn key_event(symbol: i32, down: bool, repeat: bool) {
    let mut event: sdl2_sys::SDL_Event = std::mem::zeroed();
    event.key.type_ = if down { 0x300 } else { 0x301 };
    event.key.keysym.sym = symbol;
    event.key.repeat = u8::from(repeat);
    rust_vcontrol_handle_event(ptr::from_ref(&event).cast());
}

struct PickerProbe {
    occupied: bool,
    actions: Vec<PickerAction>,
    initial_pulses: (i32, i32),
    repeat_pulses: (i32, i32),
}

unsafe fn picker_frame(
    cx: &mut uqm_rust::engine::battle::Cx<'_>,
    state: &mut PickerProbe,
    input: &uqm_rust::engine::input::InputSession,
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
            key_event(RETURN, true, true);
            key_event(ESCAPE, true, true);
            UpdateInputState(cx, input);
            state.repeat_pulses = (input.pulsed().menu[SELECT], input.pulsed().menu[CANCEL]);
            key_event(RETURN, false, false);
            key_event(ESCAPE, false, false);
            key_event(RETURN, true, false);
            1
        }
        PickerAction::EmptySlot => {
            key_event(RETURN, false, false);
            key_event(ESCAPE, true, false);
            1
        }
        _ => 0,
    }
}

unsafe fn open_from_restart_while_select_is_held(
    input: &uqm_rust::engine::input::InputSession,
    occupied: bool,
) {
    FlushInput(input);
    uqm_rust::engine::World::current().write(|world| world.set_activity(0));
    key_event(RETURN, true, false);
    uqm_rust::app::bridge::enter(|cx| UpdateInputState(cx, input));
    assert_ne!(input.pulsed().menu[SELECT], 0);
    let mut flash: FlashContext = std::mem::zeroed();
    let mut menu = DoRestartState {
        initialized: true,
        cur_state: 1,
        flash_context: ptr::addr_of_mut!(flash) as usize,
        inact_timeout: u32::MAX,
        ..Default::default()
    };
    // SAFETY: the serialized fixture publishes its unique game context for this call.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let mut io = uqm_rust::engine::observe::Io {
                observer: &mut observer,
            };
            assert!(!do_restart_frame(&CffiOps::new(cx, &mut io), &mut menu));
        });
    }
    assert_eq!(
        uqm_rust::engine::Kernel::current().read(|kernel| kernel.last_activity),
        0x1000
    );
    assert_eq!(
        uqm_rust::engine::World::current().read(|world| world.activity()),
        4
    );
    // A queued Cancel is cleared at the same ownership boundary as Select.
    key_event(ESCAPE, true, false);
    let mut picker = PickerProbe {
        occupied,
        actions: Vec::new(),
        initial_pulses: (-1, -1),
        repeat_pulses: (-1, -1),
    };
    // No key-up for the menu's Select has happened before picker entry.
    DoInput(
        1,
        &mut |cx: &mut uqm_rust::engine::battle::Cx<'_>,
              _: &mut uqm_rust::engine::observe::Io<'_>| {
            picker_frame(cx, &mut picker, input) != 0
        },
    );
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
    key_event(RETURN, false, false);
    key_event(ESCAPE, false, false);
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
        unsafe {
            uqm_rust::graphics::sdl::lifecycle_abi::GameActive = 1;
            Init_DrawCommandQueue();
            SetDefaultMenuRepeatDelay(input);
            assert_eq!(rust_v_control_init(), 0);
            assert_eq!(
                rust_vcontrol_add_key_binding(
                    RETURN,
                    ptr::addr_of_mut!((*input.immediate_ptr()).menu[SELECT])
                ),
                0
            );
            assert_eq!(
                rust_vcontrol_add_key_binding(
                    ESCAPE,
                    ptr::addr_of_mut!((*input.immediate_ptr()).menu[CANCEL])
                ),
                0
            );

            let mut summaries: [SUMMARY_DESC; 50] = std::mem::zeroed();
            saveDir = ptr::null_mut();
            assert!(!profile.path().join("save").exists());
            load_game_descriptions(summaries.as_mut_ptr());
            assert!(summaries.iter().all(|summary| summary.year_index == 0));
            open_from_restart_while_select_is_held(input, false);

            let saves = profile.path().join("save");
            std::fs::create_dir(&saves).unwrap();
            uio::uio_init();
            let repo = uio::uio_openRepository(0);
            let path = CString::new(saves.to_str().unwrap()).unwrap();
            assert!(!uio::uio_mountDir(
                repo,
                c"/".as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null(),
                path.as_ptr(),
                ptr::null_mut(),
                4,
                ptr::null_mut()
            )
            .is_null());
            let dir = uio::uio_openDir(repo, c"/".as_ptr(), 0);
            saveDir = dir.cast();
            load_game_descriptions(summaries.as_mut_ptr());
            assert!(summaries.iter().all(|summary| summary.year_index == 0));
            let slots = SaveSlots::new(saveDir.cast()).unwrap();
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
                uqm_rust::engine::World::current().write(|world| world.set_activity(4));
                load_game_descriptions(summaries.as_mut_ptr());
                assert_eq!(
                    uqm_rust::engine::World::current().read(|world| world.activity()),
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
            saveDir = ptr::null_mut();
            uio::uio_closeDir(dir);
            uio::uio_closeRepository(repo);
            uio::uio_unInit();
            rust_v_control_uninit();
            Uninit_DrawCommandQueue();
        }
    });
}
