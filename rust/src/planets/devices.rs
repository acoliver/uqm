//! Devices menu, ported from `planets/devices.c`.

use crate::battle::c_bridge::SleepThread;
use crate::mainloop::restart_menu::c_extern::uqm_sleep_thread_until as SleepThreadUntil;
use crate::mainloop::restart_menu::c_extern::{DoInput, FadeScreen};
use std::ffi::c_void;
use std::sync::Mutex;

use crate::automation::story_actions::DeviceObservation;
use crate::c_bindings::global_state::{CHMMR_DEFINED, ILWRATH_DEFINED, SAMATRA_DEFINED};
use crate::collections::queue::{CountLinks, Queue, ReinitQueue};
use crate::comm::dispatch::{clone_ship_fragment, ship};
use crate::comm::locdata::{CColor, CGlobData, CRect};
use crate::mainloop::types::activity_flags::{CHECK_LOAD, START_ENCOUNTER};
use crate::savegame::runtime::star::CStarDesc;
use crate::starbase::cffi::CffiPlatform;
use crate::starbase::gfx::{Align, ContextId, FontId, Gfx, Point, Rect, Rgba, Stamp, TextDraw};
use crate::state::game_state_keys::{get_game_state as get, set_game_state as set};

static OBSERVATION: Mutex<DeviceObservation> = Mutex::new(DeviceObservation {
    in_orbit: false,
    pc_menu: false,
    list: None,
    selected: 0,
    activation: None,
});

pub fn observation() -> DeviceObservation {
    let mut observed = OBSERVATION.lock().unwrap().clone();
    observed.in_orbit = unsafe { playerInPlanetOrbit() };
    observed.pc_menu = unsafe { crate::navigation::abi::optWhichMenu == 2 };
    observed
}

#[cfg(not(test))]
use crate::navigation::hyper_transition::SaveSisHyperState;
#[cfg(test)]
extern "C" {
    fn SaveSisHyperState();
}
#[allow(non_snake_case)]
extern "C" {
    static mut GlobData: CGlobData;
    static mut NextActivity: u16;
    static CurStarDescPtr: *const CStarDesc;
    static pSolarSysState: *mut crate::c_bindings::global_state::SOLARSYS_STATE;
    static MiscDataFrame: *mut c_void;

    fn SetMenuSounds(move_sound: u16, select_sound: u16);
    fn PlayMenuSound(sound: i32);

    fn ClearSISRect(flags: u16);
    fn DrawStarConBox(
        rect: *mut CRect,
        border: i16,
        medium: CColor,
        dark: CColor,
        fill: i32,
        background: CColor,
    );
    fn playerInSolarSystem() -> bool;
    fn playerInPlanetOrbit() -> bool;
    fn matchWorld(
        system: *const crate::c_bindings::global_state::SOLARSYS_STATE,
        world: *const crate::c_bindings::global_state::PLANET_DESC,
        planet: u8,
        moon: u8,
    ) -> bool;
    fn StartSphereTracking(race: u16) -> u16;
    fn PutGroupInfo(offset: u32, group: u8) -> u32;
    fn SaveSolarSysLocation();

    fn inHQSpace() -> i32;
    fn inHyperSpace() -> i32;
}

#[repr(C)]
struct Menu {
    input: unsafe extern "C" fn(*mut Menu) -> i32,
    list: Vec<u8>,
    selected: usize,
    top: usize,
}

fn color(r: u8, g: u8, b: u8) -> CColor {
    let c = Rgba::rgb15(r, g, b);
    CColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn draw_device(id: u8, pos: usize, selected: bool) {
    let g = CffiPlatform;
    let y = 33 + pos as i16 * 18 + 2;
    g.set_fg_color(if selected {
        Rgba::rgb15(10, 10, 31)
    } else {
        Rgba::rgb15(0, 0, 20)
    });
    g.fill_rect(Rect {
        corner: Point { x: 20, y },
        width: 41,
        height: 14,
    });
    g.set_font(FontId::Tiny);
    g.set_fg_color(if selected {
        Rgba::rgb15(10, 31, 31)
    } else {
        Rgba::rgb15(0, 20, 20)
    });
    let text = g.game_string(134 + i32::from(id));
    let (first, second) = text.split_once(' ').unwrap_or((&text, ""));
    for (line, offset) in [(first, 0), (second, 7)] {
        g.draw_text(&TextDraw {
            baseline: Point {
                x: 40,
                y: y + 6 + offset,
            },
            align: Align::Center,
            text: line.to_owned(),
        });
    }
}

unsafe fn draw(menu: &Menu, old: Option<usize>, refresh: bool) {
    let g = CffiPlatform;
    g.batch();
    g.set_context(ContextId::Status);
    if refresh {
        let mut rect = CRect {
            corner: crate::comm::locdata::CPoint { x: 2, y: 20 },
            width: 60,
            height: 109,
        };
        DrawStarConBox(
            &mut rect,
            1,
            color(16, 16, 16),
            color(8, 8, 8),
            1,
            color(0, 0, 20),
        );
        g.set_font(FontId::StarCon);
        g.set_fg_color(Rgba::rgb15(10, 31, 31));
        g.draw_text(&TextDraw {
            baseline: Point { x: 31, y: 27 },
            align: Align::Center,
            text: g.game_string(133),
        });
        for (pos, id) in menu.list.iter().skip(menu.top).take(5).enumerate() {
            g.draw_stamp(Stamp {
                origin: Point {
                    x: 4,
                    y: 34 + pos as i16 * 18,
                },
                frame: g.abs_frame(MiscDataFrame as usize, 77 + u16::from(*id)),
            });
            draw_device(*id, pos, false);
        }
    } else if let Some(old) = old.filter(|old| *old != menu.selected) {
        draw_device(menu.list[old], old - menu.top, false);
    }
    draw_device(menu.list[menu.selected], menu.selected - menu.top, true);
    g.unbatch();
}

unsafe fn reset_groups() {
    crate::c_bindings::global_state::EncounterGroup = 0;
    PutGroupInfo(0, 255);
    ReinitQueue(std::ptr::addr_of_mut!(GlobData.game_state.ip_group_q).cast::<Queue>());
    assert_eq!(
        CountLinks(std::ptr::addr_of_mut!(GlobData.game_state.npc_built_ship_q).cast::<Queue>()),
        0
    );
}

unsafe fn orbit_is(planet: u8) -> bool {
    playerInPlanetOrbit() && matchWorld(pSolarSysState, (*pSolarSysState).pOrbitalDesc, planet, 255)
}

unsafe fn use_caster() -> bool {
    if inHQSpace() != 0 {
        if get("ARILOU_SPACE_SIDE") <= 1 {
            set("USED_BROADCASTER", 1);
            return true;
        }
        return false;
    }
    if GlobData.game_state.current_activity & 0xff != 4 || !playerInSolarSystem() {
        return false;
    }
    if orbit_is(1)
        && u32::from((*CurStarDescPtr).index) == CHMMR_DEFINED
        && get("CHMMR_UNLEASHED") == 0
    {
        NextActivity |= CHECK_LOAD;
        GlobData.game_state.current_activity |= START_ENCOUNTER;
        reset_groups();
        set("GLOBAL_FLAGS_AND_DATA", 1 << 7);
        SaveSolarSysLocation();
        return true;
    }
    let home = u32::from((*CurStarDescPtr).index) == ILWRATH_DEFINED;
    let mut found = home && StartSphereTracking(u16::from(ship::ILWRATH_SHIP)) != 0;
    if !found {
        let queue = std::ptr::addr_of!(GlobData.game_state.ip_group_q);
        let group = (*queue)
            .head
            .cast::<crate::c_bindings::global_state::IP_GROUP>();
        if !group.is_null() {
            found = (*group).race_id == ship::ILWRATH_SHIP;
        }
    }
    if !found {
        return false;
    }
    NextActivity |= CHECK_LOAD;
    GlobData.game_state.current_activity |= START_ENCOUNTER;
    reset_groups();
    set("GLOBAL_FLAGS_AND_DATA", if home { 1 << 4 } else { 1 << 5 });
    if playerInPlanetOrbit() {
        SaveSolarSysLocation();
    }
    true
}

/// Returns the original DeviceStatus (failure, success, success without sound).
unsafe fn invoke(id: u8) -> u8 {
    match id {
        4..=6 => {
            let condition = get("ULTRON_CONDITION");
            if condition == 0 {
                return 0;
            }
            set("ULTRON_CONDITION", condition + 1);
            set(
                [
                    "ROSY_SPHERE_ON_SHIP",
                    "AQUA_HELIX_ON_SHIP",
                    "CLEAR_SPINDLE_ON_SHIP",
                ][usize::from(id - 4)],
                0,
            );
            set("DISCUSSED_ULTRON", 0);
            set("SUPOX_ULTRON_HELP", 0);
            1
        }
        3 => {
            if GlobData.game_state.current_activity & 0xff != 4 || !playerInPlanetOrbit() {
                return 0;
            }
            PlayMenuSound(3);
            SleepThreadUntil(FadeScreen(250, 840).wrapping_add(1680));
            if u32::from((*CurStarDescPtr).index) != CHMMR_DEFINED || !orbit_is(1) {
                FadeScreen(253, 1680);
            } else {
                set("CHMMR_EMERGING", 1);
                GlobData.game_state.current_activity |= START_ENCOUNTER;
                reset_groups();
                clone_ship_fragment(
                    ship::CHMMR_SHIP,
                    std::ptr::addr_of_mut!(GlobData.game_state.npc_built_ship_q).cast::<Queue>(),
                    0,
                );
            }
            2
        }
        2 => {
            set("UTWIG_BOMB", 0);
            GlobData.game_state.current_activity &= !0x0200;
            GlobData.sis_state.crew_enlisted = u16::MAX;
            1
        }
        1 => {
            NextActivity |= CHECK_LOAD;
            GlobData.game_state.current_activity |= START_ENCOUNTER;
            set("GLOBAL_FLAGS_AND_DATA", 0);
            if inHQSpace() != 0 {
                if !GlobData.game_state.encounter_q.head.is_null() {
                    set("SHIP_TO_COMPEL", 1);
                }
                GlobData.game_state.current_activity &= !0x0200;
                SaveSisHyperState();
            } else {
                crate::c_bindings::global_state::EncounterGroup = 0;
                if !GlobData.game_state.ip_group_q.head.is_null() {
                    set("SHIP_TO_COMPEL", 1);
                    reset_groups();
                }
                if u32::from((*CurStarDescPtr).index) == SAMATRA_DEFINED {
                    set("READY_TO_CONFUSE_URQUAN", 1);
                }
                if playerInPlanetOrbit() {
                    SaveSolarSysLocation();
                }
            }
            1
        }
        12 | 13 => u8::from(use_caster()),
        0 => {
            if inHyperSpace() == 0 || GlobData.sis_state.fuel_on_board < 1000 {
                return 0;
            }
            GlobData.sis_state.fuel_on_board -= 1000;
            set("PORTAL_COUNTER", 1);
            1
        }
        _ => 0,
    }
}

fn effect_state() -> std::collections::BTreeMap<String, u32> {
    let mut state: std::collections::BTreeMap<_, _> =
        crate::automation::story_actions::DEVICE_STORY_KEYS
            .iter()
            .copied()
            .map(|key| (key.to_owned(), get(key)))
            .collect();
    unsafe {
        state.insert(
            "activity".into(),
            u32::from(GlobData.game_state.current_activity),
        );
        state.insert("fuel".into(), GlobData.sis_state.fuel_on_board);
    }
    state
}

unsafe extern "C" fn frame(menu: *mut Menu) -> i32 {
    let menu = &mut *menu;
    let keys = std::ptr::addr_of!(crate::c_bindings::controller_input::PulsedInputState)
        .read()
        .menu;
    if GlobData.game_state.current_activity & 0x4000 != 0 || keys[10] != 0 {
        return 0;
    }
    if keys[9] != 0 {
        let id = menu.list[menu.selected];
        let before = effect_state();
        let status = invoke(id);
        if status == 0 {
            PlayMenuSound(2);
        } else if status == 1 {
            PlayMenuSound(3);
        }
        if status != 0 {
            let after = effect_state();
            let verified =
                crate::automation::story_actions::device_effect_holds(id, &before, &after);
            let state = before
                .iter()
                .map(|(key, value)| format!("before_{key}={value},after_{key}={}", after[key]))
                .collect::<Vec<_>>()
                .join(",");
            let mut obs = OBSERVATION.lock().unwrap();
            let generation = obs.activation.as_ref().map_or(1, |(n, _, _, _)| n + 1);
            obs.activation = Some((generation, id, state, verified));
        }
        return i32::from(status == 0);
    }
    let delta = if keys[5] != 0 || keys[7] != 0 {
        -1
    } else if keys[6] != 0 || keys[8] != 0 {
        1
    } else if keys[13] != 0 {
        5
    } else if keys[12] != 0 {
        -5
    } else {
        0
    };
    let next = (menu.selected as i32 + delta).clamp(0, menu.list.len() as i32 - 1) as usize;
    if next != menu.selected {
        let old = menu.selected;
        menu.selected = next;
        let top = if next < menu.top || next >= menu.top + 5 {
            next - next % 5
        } else {
            menu.top
        };
        let refresh = top != menu.top;
        menu.top = top;
        draw(menu, Some(old), refresh);
        OBSERVATION.lock().unwrap().selected = next;
    }
    SleepThread(840 / 30);
    1
}

/// # Safety
/// `map` must have room for `size` device IDs. Called on the game thread.
#[no_mangle]
pub unsafe extern "C" fn InventoryDevices(map: *mut u8, size: u16) -> i16 {
    let state = crate::state::ffi::snapshot_game_state().expect("initialized device inventory");
    let state = crate::state::game_state::GameState::from_bytes(&state);
    let list = crate::savegame::runtime::summary::inventory_devices(&state, usize::from(size));
    std::ptr::copy_nonoverlapping(list.as_ptr(), map, list.len());
    list.len() as i16
}

/// # Safety
/// Requires the initialized UI and campaign on the owning game thread.
#[no_mangle]
pub unsafe extern "C" fn DevicesMenu() -> i32 {
    let mut list = [0; 28];
    let count = InventoryDevices(list.as_mut_ptr(), 28) as usize;
    if count == 0 {
        return 0;
    }
    let mut menu = Menu {
        input: frame,
        list: list[..count].to_vec(),
        selected: 0,
        top: 0,
    };
    {
        let mut obs = OBSERVATION.lock().unwrap();
        obs.list = Some(menu.list.clone());
        obs.selected = 0;
    }
    draw(&menu, None, true);
    SetMenuSounds(0x000f | 0x0080 | 0x0100, 0x0010);
    DoInput(std::ptr::addr_of_mut!(menu).cast(), 1);
    SetMenuSounds(0x000f, 0x0010);
    OBSERVATION.lock().unwrap().list = None;
    if GlobData.sis_state.crew_enlisted != u16::MAX
        && GlobData.game_state.current_activity & 0x4000 == 0
    {
        ClearSISRect(8);
        if get("PORTAL_COUNTER") == 0
            && GlobData.game_state.current_activity & START_ENCOUNTER == 0
            && GlobData.sis_state.crew_enlisted != u16::MAX
        {
            return 1;
        }
    }
    0
}
