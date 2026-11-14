//! Devices menu, ported from `planets/devices.c`.

use super::device_effects::{effect_state, repair_ultron, ULTRON_COMPONENTS};
use crate::battle::c_bridge::SleepThread;
use crate::engine::{battle::Cx, observe::Io};
use crate::engine::{StoryKey, World};

use crate::mainloop::{
    input_abi::live::SetMenuSounds,
    restart_menu::c_extern::{uqm_sleep_thread_until as SleepThreadUntil, FadeScreen},
    types::activity_flags::{CHECK_LOAD, START_ENCOUNTER},
};

use crate::comm::{
    dispatch::ship,
    locdata::{CColor, CRect},
};
use crate::planets::generate::identities::{CHMMR_DEFINED, ILWRATH_DEFINED, SAMATRA_DEFINED};
use crate::starbase::{
    gfx::{Align, ContextId, FontId, Gfx, Point, Rect, Rgba, TextDraw},
    graphics::Graphics,
};
use crate::state::game_state_keys::{get_game_state as get, set_game_state as set};

fn publish(kernel: &crate::engine::Kernel, io: &mut Io<'_>) {
    io.observer
        .checkpoint(&crate::engine::observe::CheckpointView::Devices(
            &kernel.device_observation,
        ));
}

use crate::navigation::hyper_transition::save_sis_hyper_state;

use crate::engine::world::fleet::start_sphere_tracking;

use crate::navigation::{
    globals::{in_hqspace, in_hyper_space},
    groups::live::put_group_info,
};

use crate::engine::world::generation::Body;
use crate::planets::solar_system::SolarSystem;
use crate::planets::solarsys::lifecycle::save_solar_location;

use crate::ui::util::draw_starcon_box;

struct Menu {
    icons: crate::graphics::context_abi::ContextFrame,
    context: crate::graphics::context_abi::ContextLease,
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

fn draw_device(g: &Graphics, id: u8, pos: usize, selected: bool) {
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

fn draw(menu: &Menu, g: &Graphics, old: Option<usize>, refresh: bool) {
    g.batch();
    g.set_context(ContextId::Status);
    if refresh {
        let rect = CRect {
            corner: crate::comm::locdata::CPoint { x: 2, y: 20 },
            width: 60,
            height: 109,
        };
        draw_starcon_box(
            &menu.context,
            crate::graphics::native_abi::RECT {
                corner: rect.corner,
                extent: crate::graphics::native_abi::EXTENT {
                    width: rect.width,
                    height: rect.height,
                },
            },
            1,
            color(16, 16, 16),
            color(8, 8, 8),
            Some(color(0, 0, 20)),
        );
        g.set_font(FontId::StarCon);
        g.set_fg_color(Rgba::rgb15(10, 31, 31));
        g.draw_text(&TextDraw {
            baseline: Point { x: 31, y: 27 },
            align: Align::Center,
            text: g.game_string(133),
        });
        for (pos, id) in menu.list.iter().skip(menu.top).take(5).enumerate() {
            menu.context.draw_stamp(
                &menu.icons.indexed_frame(77 + usize::from(*id)),
                crate::graphics::native_abi::POINT {
                    x: 4,
                    y: 34 + pos as i16 * 18,
                },
            );
            draw_device(g, *id, pos, false);
        }
    } else if let Some(old) = old.filter(|old| *old != menu.selected) {
        draw_device(g, menu.list[old], old - menu.top, false);
    }
    draw_device(g, menu.list[menu.selected], menu.selected - menu.top, true);
    g.unbatch();
}

unsafe fn reset_groups(world: &mut World) {
    world.encounter_group = 0;
    put_group_info(world, 0, 255);
    world.ip_groups().reinit();
    assert_eq!(world.npc_built_ships_ref().count(), 0);
}

fn orbit_is(solar: Option<&SolarSystem>, planet: u8) -> bool {
    solar.is_some_and(|solar| {
        solar.in_orbit() && solar.selected_body() == Some(Body::Planet(planet))
    })
}

unsafe fn use_caster(
    cx: &mut Cx<'_>,
    solar: Option<&SolarSystem>,
    next_activity: &mut u16,
) -> bool {
    if in_hqspace(cx.world) != 0 {
        if get(cx.world, StoryKey::ARILOU_SPACE_SIDE) <= 1 {
            set(cx.world, StoryKey::USED_BROADCASTER, 1);
            return true;
        }
        return false;
    }
    let Some(solar) = solar.filter(|_| cx.world.activity() & 0xff == 4) else {
        return false;
    };
    let star = solar.sys.star_index();
    if orbit_is(Some(solar), 1)
        && star == CHMMR_DEFINED
        && get(cx.world, StoryKey::CHMMR_UNLEASHED) == 0
    {
        *next_activity |= CHECK_LOAD;
        cx.world.set_activity(cx.world.activity() | START_ENCOUNTER);
        reset_groups(cx.world);
        set(cx.world, StoryKey::GLOBAL_FLAGS_AND_DATA, 1 << 7);
        save_solar_location(cx.world, solar);
        return true;
    }
    let home = star == ILWRATH_DEFINED;
    let mut found = home && start_sphere_tracking(cx.world, u16::from(ship::ILWRATH_SHIP)) != 0;
    if !found {
        let groups = cx.world.ip_groups_ref();
        found = groups
            .head()
            .is_some_and(|id| groups.get(id).race_id == ship::ILWRATH_SHIP);
    }
    if !found {
        return false;
    }
    *next_activity |= CHECK_LOAD;
    cx.world.set_activity(cx.world.activity() | START_ENCOUNTER);
    reset_groups(cx.world);
    set(
        cx.world,
        StoryKey::GLOBAL_FLAGS_AND_DATA,
        if home { 1 << 4 } else { 1 << 5 },
    );
    if solar.in_orbit() {
        save_solar_location(cx.world, solar);
    }
    true
}

/// Returns the original DeviceStatus (failure, success, success without sound).
unsafe fn invoke(
    cx: &mut Cx<'_>,
    solar: Option<&SolarSystem>,
    id: u8,
    next_activity: &mut u16,
) -> u8 {
    match id {
        4..=6 => repair_ultron(cx.world, ULTRON_COMPONENTS[usize::from(id - 4)]),
        3 => {
            let Some(solar) =
                solar.filter(|solar| cx.world.activity() & 0xff == 4 && solar.in_orbit())
            else {
                return 0;
            };
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 3);
            cx.with_legacy(|| SleepThreadUntil(FadeScreen(250, 840).wrapping_add(1680)));
            let star = solar.sys.star_index();
            if star != CHMMR_DEFINED || !orbit_is(Some(solar), 1) {
                cx.with_legacy(|| FadeScreen(253, 1680));
            } else {
                set(cx.world, StoryKey::CHMMR_EMERGING, 1);
                cx.world.set_activity(cx.world.activity() | START_ENCOUNTER);
                reset_groups(cx.world);
                crate::comm::dispatch::clone_npc_fragment(cx.world, ship::CHMMR_SHIP, 0);
            }
            2
        }
        2 => {
            set(cx.world, StoryKey::UTWIG_BOMB, 0);
            cx.world.set_activity(cx.world.activity() & !0x0200);
            cx.world.data.flagship.crew_enlisted = u16::MAX;
            1
        }
        1 => {
            *next_activity |= CHECK_LOAD;
            cx.world.set_activity(cx.world.activity() | START_ENCOUNTER);
            set(cx.world, StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
            if in_hqspace(cx.world) != 0 {
                if cx.world.encounters_ref().head().is_some() {
                    set(cx.world, StoryKey::SHIP_TO_COMPEL, 1);
                }
                cx.world.set_activity(cx.world.activity() & !0x0200);
                save_sis_hyper_state(cx);
            } else {
                cx.world.encounter_group = 0;
                if cx.world.ip_groups_ref().head().is_some() {
                    set(cx.world, StoryKey::SHIP_TO_COMPEL, 1);
                    reset_groups(cx.world);
                }
                let Some(solar) = solar else {
                    panic!("device effect requires a solar visit");
                };
                let star = solar.sys.star_index();
                if star == SAMATRA_DEFINED {
                    set(cx.world, StoryKey::READY_TO_CONFUSE_URQUAN, 1);
                }
                if solar.in_orbit() {
                    save_solar_location(cx.world, solar);
                }
            }
            1
        }
        12 | 13 => u8::from(use_caster(cx, solar, next_activity)),
        0 => {
            if in_hyper_space(cx.world) == 0 || cx.world.data.flagship.fuel_on_board < 1000 {
                return 0;
            }
            cx.world.data.flagship.fuel_on_board -= 1000;
            set(cx.world, StoryKey::PORTAL_COUNTER, 1);
            1
        }
        _ => 0,
    }
}

unsafe fn frame(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    input: &crate::engine::input::InputSession,
    solar: Option<&SolarSystem>,
    menu: &mut Menu,
    next_activity: &mut u16,
) -> i32 {
    let keys = input.pulsed().menu;
    if cx.world.activity() & 0x4000 != 0 || keys[10] != 0 {
        return 0;
    }
    if keys[9] != 0 {
        let id = menu.list[menu.selected];
        let before = effect_state(cx.world);
        let status = invoke(cx, solar, id, next_activity);
        if status == 0 {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
        } else if status == 1 {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 3);
        }
        if status != 0 {
            let after = effect_state(cx.world);
            let verified = super::device_effects::device_effect_holds(id, &before, &after);
            let state = before
                .iter()
                .map(|(key, value)| format!("before_{key}={value},after_{key}={}", after[key]))
                .collect::<Vec<_>>()
                .join(",");
            let obs = &mut cx.kernel.device_observation;
            let generation = obs.activation.as_ref().map_or(1, |(n, _, _, _)| n + 1);
            obs.activation = Some((generation, id, state, verified));
            publish(cx.kernel, io);
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
        draw(
            menu,
            &Graphics::new(cx.kernel, cx.screen_dimensions()),
            Some(old),
            refresh,
        );
        cx.kernel.device_observation.selected = next;
        publish(cx.kernel, io);
    }
    cx.with_legacy(|| SleepThread(840 / 30));
    1
}

/// # Safety
/// Requires the initialized UI and campaign on the owning game thread.
pub unsafe fn devices_menu(cx: &mut Cx<'_>, io: &mut Io<'_>, solar: Option<&SolarSystem>) -> i32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let Some(mut menu) = prepare_devices_menu(cx) else {
        return 0;
    };
    cx.with_legacy(|| SetMenuSounds(input, 0x000f | 0x0080 | 0x0100, 0x0010));
    publish(cx.kernel, io);
    cx.run_input(io, true, |cx, io| {
        let mut next_activity = 0;
        let result = frame(cx, io, input, solar, &mut menu, &mut next_activity) != 0;
        cx.world.next_activity |= next_activity;
        result
    });
    cx.with_legacy(|| SetMenuSounds(input, 0x000f, 0x0010));
    cx.kernel.device_observation.list = None;
    publish(cx.kernel, io);
    if cx.world.data.flagship.crew_enlisted == u16::MAX || cx.world.activity() & 0x4000 != 0 {
        return 0;
    }
    {
        let dimensions = cx.screen_dimensions();
        crate::ui::sis::clear_sis_rect(cx.kernel, cx.world, dimensions, 8)
    };
    i32::from(
        cx.world.story_state(StoryKey::PORTAL_COUNTER) == 0
            && cx.world.activity() & START_ENCOUNTER == 0
            && cx.world.data.flagship.crew_enlisted != u16::MAX,
    )
}

unsafe fn prepare_devices_menu(cx: &mut Cx<'_>) -> Option<Menu> {
    let list = super::device_effects::inventory(cx.world, 28);
    if list.is_empty() {
        return None;
    }
    let menu = Menu {
        icons: crate::graphics::context_abi::ContextFrame::retain(std::ptr::NonNull::new(
            cx.kernel.misc_data_frame,
        )),
        context: crate::graphics::context_abi::ContextLease::require(cx.kernel.status_context),
        list,
        selected: 0,
        top: 0,
    };
    cx.kernel.device_observation.list = Some(menu.list.clone());
    cx.kernel.device_observation.selected = 0;
    draw(
        &menu,
        &Graphics::new(cx.kernel, cx.screen_dimensions()),
        None,
        true,
    );
    Some(menu)
}
