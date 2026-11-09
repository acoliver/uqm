//! Campaign crew-purchase accounting.

use crate::engine::input::InputSession;
use crate::engine::{
    battle::{link_queue::LinkId, Cx},
    world::queues::Fragment,
    StoryKey, World,
};
use crate::graphics::{menu as draw, native_abi::*};
use crate::starbase::shipyard::SHIP_COST;
use crate::ui::{
    rgb15,
    settings::play_menu_sound,
    shipyard_layout::{DMS_GetEscortShipRect, SHIP_WIN_WIDTH},
    sis::delta_sis_gauges,
};

const BLACK: Color = rgb15(0, 0, 0);

pub(super) fn purchased_crew(world: &World) -> SIZE {
    u16::from_le_bytes([
        world.story_state(StoryKey::CREW_PURCHASED0) as BYTE,
        world.story_state(StoryKey::CREW_PURCHASED1) as BYTE,
    ]) as SIZE
}

pub(super) fn crew_transaction(cx: &mut Cx<'_>, delta: SIZE, redraw: impl FnOnce(&mut Cx<'_>)) {
    if delta == 0 {
        return;
    }
    let mut bought = purchased_crew(cx.world).wrapping_add(delta);
    if bought < 0 {
        bought = if delta < 0 { 0 } else { 0x7fff };
    } else if delta > 0 {
        if bought as i32 >= crate::starbase::shipyard::CREW_EXPENSE_THRESHOLD as i32
            && (bought as i32 - delta as i32)
                < crate::starbase::shipyard::CREW_EXPENSE_THRESHOLD as i32
        {
            cx.world.data.game.crew_cost += 2;
            redraw(cx);
        }
    } else if (bought as i32) < crate::starbase::shipyard::CREW_EXPENSE_THRESHOLD as i32
        && bought as i32 - delta as i32 >= crate::starbase::shipyard::CREW_EXPENSE_THRESHOLD as i32
    {
        cx.world.data.game.crew_cost -= 2;
        redraw(cx);
    }
    if crate::engine::world::fleet::check_alliance(
        cx.world,
        crate::comm::dispatch::ship::SHOFIXTI_SHIP as COUNT,
    ) != (crate::starbase::platform::AlliedState::GoodGuy as u16) as COUNT
    {
        cx.world
            .set_story_state(StoryKey::CREW_PURCHASED0, bought as BYTE as u32);
        cx.world
            .set_story_state(StoryKey::CREW_PURCHASED1, ((bought as u16) >> 8) as u32);
    }
}

pub(super) fn label(world: &mut World, id: LinkId<Fragment>) -> (bool, String) {
    let ship = *world.built_ships().get(id);
    let max_crew = {
        let races = world.avail_races();
        let Some(template) = races.at(usize::from(ship.race_id)) else {
            panic!("a hangar ship must have a fleet template");
        };
        races.get(template).crew_level
    };
    let buffer = if ship.crew_level >= max_crew {
        format!("{}\0", ship.crew_level)
    } else if ship.crew_level == 0 {
        String::from("SCRAP\0")
    } else {
        format!("{}/{}\0", ship.crew_level, max_crew)
    };
    (ship.crew_level != 0, buffer)
}

pub(super) fn show_ship_crew(world: &mut World, id: LinkId<Fragment>, rect: &RECT) {
    let (has_crew, buffer) = label(world, id);
    let mut r = *rect;
    let baseline = POINT {
        x: r.corner.x + (r.extent.width >> 1),
        y: r.corner.y + r.extent.height - 1,
    };
    if r.corner.y != 0 {
        r.corner.y = baseline.y - 6;
        r.extent.width = SHIP_WIN_WIDTH;
        r.extent.height = 6;
        draw::foreground(BLACK);
        draw::fill(r);
    }
    draw::foreground(if has_crew {
        rgb15(0, 0x14, 0)
    } else {
        rgb15(0x12, 0, 0)
    });
    super::session::text(&buffer, baseline, TEXT_ALIGN_ALIGN_CENTER);
}

pub(super) fn dms_hire_escort_ship_crew(cx: &mut Cx<'_>, id: LinkId<Fragment>) -> SIZE {
    let res_units = cx.world.data.flagship.res_units;
    let crew_cost = cx.world.data.game.crew_cost;
    let ship = *cx.world.built_ships().get(id);
    let max_crew = {
        let races = cx.world.avail_races();
        let Some(template) = races.at(usize::from(ship.race_id)) else {
            panic!("a hangar ship must have a fleet template");
        };
        races.get(template).crew_level
    };
    if res_units < crew_cost as DWORD {
        return 0;
    }
    if ship.crew_level >= ship.max_crew || ship.crew_level >= max_crew {
        return 0;
    }
    let cost = if ship.crew_level > 0 {
        -(crew_cost as i32)
    } else {
        -(SHIP_COST[ship.race_id as usize] as i32)
    };
    delta_sis_gauges(cx.kernel, cx.world, 0, 0, cost);
    cx.world.built_ships().get_mut(id).crew_level += 1;
    cx.kernel.sis_highlight.pre_update_flash_rect();
    let slot = cx.world.built_ships().get(id).index;
    let r = DMS_GetEscortShipRect(slot);
    show_ship_crew(cx.world, id, &r);
    cx.kernel.sis_highlight.post_update_flash_rect();
    1
}

pub(super) fn dms_dismiss_escort_ship_crew(cx: &mut Cx<'_>, id: LinkId<Fragment>) -> SIZE {
    let ship = *cx.world.built_ships().get(id);
    let mut delta = 0;
    if ship.crew_level > 0 {
        let cost = if ship.crew_level > 1 {
            let bought = purchased_crew(cx.world);
            cx.world.data.game.crew_cost as i32
                - if bought == crate::starbase::shipyard::CREW_EXPENSE_THRESHOLD as SIZE {
                    2
                } else {
                    0
                }
        } else {
            SHIP_COST[ship.race_id as usize] as i32
        };
        delta_sis_gauges(cx.kernel, cx.world, 0, 0, cost);
        delta = -1;
        cx.world.built_ships().get_mut(id).crew_level -= 1;
    } else {
        play_menu_sound(
            cx.kernel,
            cx.world,
            crate::ui::settings::MENU_SOUND_FAILURE as u16,
        );
    }
    cx.kernel.sis_highlight.pre_update_flash_rect();
    let slot = cx.world.built_ships().get(id).index;
    let r = DMS_GetEscortShipRect(slot);
    show_ship_crew(cx.world, id, &r);
    cx.kernel.sis_highlight.post_update_flash_rect();
    delta
}

pub(super) fn dms_hire_flag_ship_crew(cx: &mut Cx<'_>, input: &InputSession) -> SIZE {
    let Some(cell) = super::sis::capacity::crew_cell(
        &cx.world.data.flagship,
        input.preferences().fonts == crate::ui::settings::OPT_PC as i32,
    ) else {
        return 0;
    };
    let context = crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context);
    if cx.world.data.flagship.res_units < cx.world.data.game.crew_cost as DWORD {
        context.set_foreground(cell.foreground);
        return 0;
    }
    context.draw_point(cell.foreground, cell.point);
    cx.kernel.sis_highlight.pre_update_flash_rect();
    let cost = -(cx.world.data.game.crew_cost as i32);
    delta_sis_gauges(cx.kernel, cx.world, 1, 0, cost);
    cx.kernel.sis_highlight.post_update_flash_rect();
    1
}

pub(super) fn dms_dismiss_flag_ship_crew(cx: &mut Cx<'_>, input: &InputSession) -> SIZE {
    let highlight = cx.kernel.sis_highlight.clone();
    if cx.world.data.flagship.crew_enlisted == 0 {
        return 0;
    }
    let bought = purchased_crew(cx.world);
    highlight.pre_update_flash_rect();
    let cost = cx.world.data.game.crew_cost as i32
        - if bought == crate::starbase::shipyard::CREW_EXPENSE_THRESHOLD as SIZE {
            2
        } else {
            0
        };
    {
        delta_sis_gauges(cx.kernel, cx.world, -1, 0, cost);
        highlight.post_update_flash_rect();
    };
    let cell = super::sis::capacity::crew_cell(
        &cx.world.data.flagship,
        input.preferences().fonts == crate::ui::settings::OPT_PC as i32,
    )
    .unwrap_or_else(|| panic!("dismissed flagship crew exceeds capacity"));
    let context = crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context);
    context.set_foreground(cell.foreground);
    context.draw_point(BLACK, cell.point);
    -1
}
