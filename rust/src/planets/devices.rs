//! Devices menu, ported from `planets/devices.c`.

use super::device_effects::{repair_ultron, ULTRON_COMPONENTS};
use crate::battle::c_bridge::SleepThread;
use crate::engine::{battle::Cx, observe::Io};
use crate::engine::{
    Kernel as KernelOwner, StoryKey, World as WorldOwner, WorldData as WorldFields,
};

use crate::mainloop::{
    input_abi::live::SetMenuSounds,
    restart_menu::c_extern::{uqm_sleep_thread_until as SleepThreadUntil, FadeScreen},
    types::activity_flags::{CHECK_LOAD, START_ENCOUNTER},
};

use std::sync::Mutex;

use crate::c_bindings::global_state::{CHMMR_DEFINED, ILWRATH_DEFINED, SAMATRA_DEFINED};
use crate::comm::{
    dispatch::ship,
    locdata::{CColor, CRect},
};
use crate::engine::observe::DeviceObservation;

use crate::starbase::{
    cffi::CffiPlatform,
    gfx::{Align, ContextId, FontId, Gfx, Point, Rect, Rgba, Stamp, TextDraw},
};

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
use crate::navigation::hyper_transition::save_sis_hyper_state;
#[cfg(test)]
extern "C" {
    fn save_sis_hyper_state();
}

use crate::battle::sounds::engine::PlayMenuSound;
use crate::engine::world::fleet::start_sphere_tracking;

use crate::navigation::{
    globals::{in_hqspace, in_hyper_space},
    groups::live::put_group_info,
};

use crate::planets::solarsys::{
    lifecycle::SaveSolarSysLocation, matchWorld, playerInPlanetOrbit, playerInSolarSystem,
};

use crate::ui::{sis::ClearSISRect, util::live::DrawStarConBox};

#[allow(non_snake_case)]
extern "C" {

    static mut NextActivity: u16;

}

struct Menu {
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
            std::ptr::from_mut(&mut rect).cast(),
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
                frame: g.abs_frame(
                    KernelOwner::current().read(|owner| owner.misc_data_frame) as usize,
                    77 + u16::from(*id),
                ),
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
    crate::engine::World::current().write(|world| {
        put_group_info(world, 0, 255);
        world.ip_groups().reinit();
        assert_eq!(world.npc_built_ships_ref().count(), 0);
    });
}

unsafe fn orbit_is(planet: u8) -> bool {
    let system = crate::planets::solarsys::lifecycle::P_SOLAR_SYS_STATE;
    playerInPlanetOrbit() && matchWorld(system, (*system).pOrbitalDesc, planet, 255)
}

unsafe fn use_caster() -> bool {
    if WorldOwner::current().read(in_hqspace) != 0 {
        if WorldOwner::current().read(|world| get(world, StoryKey::ARILOU_SPACE_SIDE)) <= 1 {
            WorldOwner::current().write(|world| set(world, StoryKey::USED_BROADCASTER, 1));
            return true;
        }
        return false;
    }
    if (**WorldFields::current()).game.current_activity & 0xff != 4 || !playerInSolarSystem() {
        return false;
    }
    if orbit_is(1)
        && u32::from(
            (*(**WorldOwner::current()).star_pointer(crate::planets::starmap::current_catalog()))
                .index,
        ) == CHMMR_DEFINED
        && WorldOwner::current().read(|world| get(world, StoryKey::CHMMR_UNLEASHED)) == 0
    {
        NextActivity |= CHECK_LOAD;
        (**WorldFields::current()).game.current_activity |= START_ENCOUNTER;
        reset_groups();
        WorldOwner::current().write(|world| set(world, StoryKey::GLOBAL_FLAGS_AND_DATA, 1 << 7));
        SaveSolarSysLocation();
        return true;
    }
    let home = u32::from(
        (*(**WorldOwner::current()).star_pointer(crate::planets::starmap::current_catalog())).index,
    ) == ILWRATH_DEFINED;
    let mut found = home
        && WorldOwner::current()
            .write(|world| start_sphere_tracking(world, u16::from(ship::ILWRATH_SHIP)))
            != 0;
    if !found {
        found = WorldOwner::current().read(|world| {
            let groups = world.ip_groups_ref();
            groups
                .head()
                .is_some_and(|id| groups.get(id).race_id == ship::ILWRATH_SHIP)
        });
    }
    if !found {
        return false;
    }
    NextActivity |= CHECK_LOAD;
    (**WorldFields::current()).game.current_activity |= START_ENCOUNTER;
    reset_groups();
    WorldOwner::current().write(|world| {
        set(
            world,
            StoryKey::GLOBAL_FLAGS_AND_DATA,
            if home { 1 << 4 } else { 1 << 5 },
        )
    });
    if playerInPlanetOrbit() {
        SaveSolarSysLocation();
    }
    true
}

/// Returns the original DeviceStatus (failure, success, success without sound).
unsafe fn invoke(id: u8) -> u8 {
    match id {
        4..=6 => WorldOwner::current()
            .write(|world| repair_ultron(world, ULTRON_COMPONENTS[usize::from(id - 4)])),
        3 => {
            if (**WorldFields::current()).game.current_activity & 0xff != 4
                || !playerInPlanetOrbit()
            {
                return 0;
            }
            PlayMenuSound(3);
            SleepThreadUntil(FadeScreen(250, 840).wrapping_add(1680));
            let catalog = crate::planets::starmap::current_catalog();
            let star = (**WorldOwner::current()).star_pointer(catalog);
            if u32::from((*star).index) != CHMMR_DEFINED || !orbit_is(1) {
                FadeScreen(253, 1680);
            } else {
                WorldOwner::current().write(|world| set(world, StoryKey::CHMMR_EMERGING, 1));
                (**WorldFields::current()).game.current_activity |= START_ENCOUNTER;
                reset_groups();
                WorldOwner::current().write(|world| {
                    crate::comm::dispatch::clone_npc_fragment(world, ship::CHMMR_SHIP, 0);
                });
            }
            2
        }
        2 => {
            WorldOwner::current().write(|world| set(world, StoryKey::UTWIG_BOMB, 0));
            (**WorldFields::current()).game.current_activity &= !0x0200;
            (**WorldFields::current()).flagship.crew_enlisted = u16::MAX;
            1
        }
        1 => {
            NextActivity |= CHECK_LOAD;
            (**WorldFields::current()).game.current_activity |= START_ENCOUNTER;
            WorldOwner::current().write(|world| set(world, StoryKey::GLOBAL_FLAGS_AND_DATA, 0));
            if WorldOwner::current().read(in_hqspace) != 0 {
                if WorldOwner::current().read(|world| world.encounters_ref().head().is_some()) {
                    WorldOwner::current().write(|world| set(world, StoryKey::SHIP_TO_COMPEL, 1));
                }
                (**WorldFields::current()).game.current_activity &= !0x0200;
                #[cfg(not(test))]
                crate::app::bridge::enter(save_sis_hyper_state);
                #[cfg(test)]
                save_sis_hyper_state();
            } else {
                crate::c_bindings::global_state::EncounterGroup = 0;
                if WorldOwner::current().read(|world| world.ip_groups_ref().head().is_some()) {
                    WorldOwner::current().write(|world| set(world, StoryKey::SHIP_TO_COMPEL, 1));
                    reset_groups();
                }
                let catalog = crate::planets::starmap::current_catalog();
                let star = (**WorldOwner::current()).star_pointer(catalog);
                if u32::from((*star).index) == SAMATRA_DEFINED {
                    WorldOwner::current()
                        .write(|world| set(world, StoryKey::READY_TO_CONFUSE_URQUAN, 1));
                }
                if playerInPlanetOrbit() {
                    SaveSolarSysLocation();
                }
            }
            1
        }
        12 | 13 => u8::from(use_caster()),
        0 => {
            if WorldOwner::current().read(in_hyper_space) == 0
                || (**WorldFields::current()).flagship.fuel_on_board < 1000
            {
                return 0;
            }
            (**WorldFields::current()).flagship.fuel_on_board -= 1000;
            WorldOwner::current().write(|world| set(world, StoryKey::PORTAL_COUNTER, 1));
            1
        }
        _ => 0,
    }
}

fn effect_state() -> std::collections::BTreeMap<String, u32> {
    let mut state: std::collections::BTreeMap<_, _> = super::device_effects::DEVICE_STORY_KEYS
        .iter()
        .copied()
        .map(|key| {
            (
                key.to_owned(),
                StoryKey::from_name(key)
                    .map_or(0, |key| WorldOwner::current().read(|world| get(world, key))),
            )
        })
        .collect();
    {
        state.insert(
            "activity".into(),
            u32::from(WorldFields::current().read(|owner| owner.game.current_activity)),
        );
        state.insert(
            "fuel".into(),
            WorldFields::current().read(|owner| owner.flagship.fuel_on_board),
        );
    }
    state
}

unsafe fn frame(menu: &mut Menu) -> i32 {
    let keys = std::ptr::addr_of!(crate::c_bindings::controller_input::PulsedInputState)
        .read()
        .menu;
    if WorldFields::current().read(|owner| owner.game.current_activity) & 0x4000 != 0
        || keys[10] != 0
    {
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
            let verified = super::device_effects::device_effect_holds(id, &before, &after);
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
    let state = WorldOwner::current()
        .read(crate::state::ffi::snapshot_game_state)
        .expect("initialized device inventory");
    let state = crate::state::game_state::GameState::from_bytes(&state);
    let list = crate::savegame::runtime::summary::inventory_devices(&state, usize::from(size));
    std::ptr::copy_nonoverlapping(list.as_ptr(), map, list.len());
    list.len() as i16
}

/// # Safety
/// Requires the initialized UI and campaign on the owning game thread.
pub unsafe fn devices_menu(cx: &mut Cx<'_>, io: &mut Io<'_>) -> i32 {
    let Some(mut menu) = cx.with_legacy(|| prepare_devices_menu()) else {
        return 0;
    };
    cx.with_legacy(|| SetMenuSounds(0x000f | 0x0080 | 0x0100, 0x0010));
    cx.run_input(io, true, |cx, _| cx.with_legacy(|| frame(&mut menu) != 0));
    cx.with_legacy(|| SetMenuSounds(0x000f, 0x0010));
    OBSERVATION.lock().unwrap().list = None;
    if cx.world.data.flagship.crew_enlisted == u16::MAX || cx.world.activity() & 0x4000 != 0 {
        return 0;
    }
    cx.with_legacy(|| ClearSISRect(8));
    i32::from(
        cx.world.story_state(StoryKey::PORTAL_COUNTER) == 0
            && cx.world.activity() & START_ENCOUNTER == 0
            && cx.world.data.flagship.crew_enlisted != u16::MAX,
    )
}

unsafe fn prepare_devices_menu() -> Option<Menu> {
    let mut list = [0; 28];
    let count = InventoryDevices(list.as_mut_ptr(), 28) as usize;
    if count == 0 {
        return None;
    }
    let menu = Menu {
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
    Some(menu)
}
