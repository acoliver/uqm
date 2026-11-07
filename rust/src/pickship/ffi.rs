//! The C entry points of the ship picker: `GetEncounterStarShip` and
//! `DrawArmadaPickShip`, with the drawing and input calls of `pickship.c` in
//! the order the C made them. The rules they apply are in the parent module.

use crate::battle::context_access::links;
use crate::comm::dispatch::production_captain_name;
use crate::engine::{
    battle::{Cx, ShipId},
    observe::Io,
    world::activity::ActivityControl,
    Kernel as KernelOwner,
};

use std::ffi::c_int;

use super::{
    crew_label, crew_strip, find_escort, highlight, icon_origin, moved, name_strip, slot, Area,
    Cursor, Direction, Escort, Slot, ICON_HEIGHT, ICON_WIDTH, NO_ID,
};
use crate::c_bindings::controller_input::{
    KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_RIGHT, KEY_MENU_SELECT, KEY_MENU_UP,
};

use crate::comm::locdata::{CColor, CPoint, CRect};

use crate::mainloop::{restart_menu::c_extern as base, types::activity_flags::CHECK_ABORT};

use crate::starbase::{gfx::Rgba, platform::ONE_SECOND, sis_modules::FUEL_TANK_SCALE};

/// `RPG_PLAYER_NUM`.
const RPG_PLAYER_NUM: usize = 0;
/// `LOBYTE (CurrentActivity)` of the melee game.
const SUPER_MELEE: u16 = 0;
const MENU_SOUND_MOVE: u16 = 0;
const MENU_SOUND_SUCCESS: u16 = 1;
const MENU_SOUND_NONE: u16 = 0;
/// `MENU_SOUND_UP | DOWN | LEFT | RIGHT`.
const MENU_SOUND_ARROWS: u16 = 0x0F;
const MENU_SOUND_SELECT: u16 = 1 << 4;
/// `ALIGN_CENTER`.
const ALIGN_CENTER: u32 = 1;
/// `RUN_AWAY_FUEL_COST`.
const RUN_AWAY_FUEL_COST: u32 = 5 * FUEL_TANK_SCALE;

use crate::sound::heart_ffi::wait_for_sound_end as WaitForSoundEnd;

use crate::battle::encount_native::engine::fleet_is_infinite;

use crate::engine::world::IN_HYPERSPACE;

fn color(color: Rgba) -> CColor {
    CColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn rectangle(area: Area) -> crate::graphics::native_abi::RECT {
    use crate::graphics::native_abi::{EXTENT, POINT, RECT};
    RECT {
        corner: POINT {
            x: area.x,
            y: area.y,
        },
        extent: EXTENT {
            width: area.width,
            height: area.height,
        },
    }
}

/// The picker state retained by its synchronous input frame.
struct PickerState {
    initialized: bool,
    activity: ActivityControl,
    cursor: Cursor,
    /// `MENU_STATE.CurFrame`: the chosen `HSTARSHIP`, null when the cell is
    /// empty.
    chosen: Option<ShipId>,
    frame: (i16, i16),
    /// `MENU_STATE.flash_rect0`, which `SetFlashRect` keeps pointing at.
    context: crate::graphics::context_abi::ContextLease,
    screen: crate::graphics::context_abi::ContextLease,
    font: crate::graphics::native_font::FontLease,
}

/// Every `STARSHIP` of the player's queue before the flagship, which is the
/// tail.
fn escort_links(kernel: &KernelOwner) -> Vec<(ShipId, Escort)> {
    let tail = links::last_ship(kernel, RPG_PLAYER_NUM);
    links::all_links(kernel)
        .into_iter()
        .filter(|id| Some(*id) != tail)
        .map(|id| {
            let ship = links::ship_value(kernel, id);
            (
                id,
                Escort {
                    index: ship.index,
                    alive: ship.species != NO_ID,
                },
            )
        })
        .collect()
}

fn pulsed_directions(input: &crate::engine::input::InputSession) -> (Direction, bool) {
    // SAFETY: the input state is updated by `DoInput` on this thread.
    let menu = input.pulsed().menu;
    let pressed = |key: u32| menu[key as usize] != 0;
    (
        Direction {
            up: pressed(KEY_MENU_UP),
            down: pressed(KEY_MENU_DOWN),
            left: pressed(KEY_MENU_LEFT),
            right: pressed(KEY_MENU_RIGHT),
        },
        pressed(KEY_MENU_SELECT),
    )
}

fn change_selection(cx: &mut Cx<'_>, state: &mut PickerState) {
    let kernel = &*cx.kernel;
    crate::automation::ui_observation::observe_ship_picker_cursor(state.cursor);
    let area = highlight(state.frame, state.cursor);
    state.chosen = match slot(state.cursor) {
        Slot::Flagship => links::last_ship(kernel, RPG_PLAYER_NUM),
        Slot::Escort(ship_index) => {
            let links = escort_links(kernel);
            let escorts: Vec<Escort> = links.iter().map(|(_, escort)| *escort).collect();
            find_escort(&escorts, ship_index).map(|at| links[at].0)
        }
    };
    let context = &state.context;
    let name = name_strip(state.frame);
    context.fill_rectangle(color(Rgba::BLACK), rectangle(name));
    let (crew, max_crew) = if let Some(chosen) = state.chosen {
        context.set_font(&state.font);
        let baseline = crate::graphics::native_abi::POINT {
            x: name.x + (name.width >> 1),
            y: name.y + name.height - 1,
        };
        let ship = links::ship_value(kernel, chosen);
        let (crew, capacity) = if ship.captain == 0 {
            // Preserve the legacy empty flagship captain strip without its dangling pointer.
            context.draw_text_bounded(
                &state.font,
                c"",
                baseline,
                ALIGN_CENTER,
                color(Rgba::rgb15(20, 10, 0)),
                0,
            );
            (
                cx.world.data.flagship.crew_enlisted,
                cx.world
                    .data
                    .flagship
                    .module_slots
                    .iter()
                    .map(|&m| crate::ui::sis_capacity_math::module_crew_capacity(m))
                    .sum(),
            )
        } else {
            links::with_ship_resources(kernel, chosen, |resources| {
                let (text, count) = resources.label(u16::from(ship.captain));
                context.draw_text_bounded(
                    &state.font,
                    text,
                    baseline,
                    ALIGN_CENTER,
                    color(Rgba::rgb15(20, 10, 0)),
                    count,
                );
            });
            (ship.crew, ship.max_crew)
        };
        context.set_foreground(color(Rgba::BLACK));
        (crew, capacity)
    } else {
        (0, 0)
    };
    let crew_area = crew_strip(name);
    context.fill_rectangle(color(Rgba::BLACK), rectangle(crew_area));
    if let Some(label) = crew_label(crew, max_crew) {
        let text = std::ffi::CString::new(label).expect("a crew count has no NUL byte");
        context.draw_text(
            &state.font,
            &text,
            crate::graphics::native_abi::POINT {
                x: crew_area.x + (crew_area.width >> 1),
                y: name.y + name.height - 1,
            },
            ALIGN_CENTER,
            color(Rgba::rgb15(0, 20, 0)),
        );
    }
    let clip = context.clip_rect();
    let highlight = &kernel.sis_highlight;
    crate::ui::sis::set_flash_rect(highlight, &state.screen, clip, None);
    crate::ui::sis::set_flash_rect(highlight, &state.screen, clip, Some(rectangle(area)));
}

/// `DoPickBattleShip`: one input frame of the picker.
fn do_pick_battle_ship(
    cx: &mut Cx<'_>,
    state: &mut PickerState,
    input: &crate::engine::input::InputSession,
) -> c_int {
    if base_activity_aborted(&state.activity) {
        state.chosen = None;
        return 0;
    }

    // SAFETY: plain call.
    base::SetMenuSounds(input, MENU_SOUND_NONE, MENU_SOUND_NONE);

    if !state.initialized {
        state.initialized = true;
        // The flagship selection below is made in this same call, so a Select
        // injected after this observation confirms a real choice.
        crate::automation::ui_observation::observe_ship_picker(true);
        // SAFETY: `state` is the running picker.
        change_selection(cx, state);
    } else {
        let (keys, select) = pulsed_directions(input);
        if select {
            if !state.chosen.is_none() {
                crate::ui::settings::play_menu_sound(cx.kernel, cx.world, MENU_SOUND_SUCCESS);
                return 0;
            }
        } else if let Some(cursor) = moved(state.cursor, keys) {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, MENU_SOUND_MOVE);
            state.cursor = cursor;
            // SAFETY: `state` is the running picker.
            change_selection(cx, state);
        }
    }

    // SAFETY: plain call.
    cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
    1
}

fn base_activity_aborted(activity: &ActivityControl) -> bool {
    activity.activity() & CHECK_ABORT != 0
}

fn get_armada_star_ship(
    cx: &mut Cx<'_>,
    input: &crate::engine::input::InputSession,
    activity: ActivityControl,
) -> Option<ShipId> {
    use crate::graphics::context_abi::ContextLease;
    use std::ptr::NonNull;
    if cx.b.counters[1] == 0 {
        return None;
    }
    let context = ContextLease::retain(
        NonNull::new(cx.kernel.space_context).unwrap_or_else(|| panic!("picker context")),
    );
    context.with_selected(|| {
        let mut pick = CRect::default();
        draw_armada_pick_ship_in(cx.kernel, cx.world, 0, &mut pick);
        let mut state = PickerState {
            initialized: false,
            activity,
            cursor: Cursor::START,
            chosen: None,
            frame: (pick.corner.x, pick.corner.y),
            context: context.clone(),
            screen: ContextLease::retain(
                NonNull::new(cx.kernel.screen_context).unwrap_or_else(|| panic!("picker screen")),
            ),
            font: cx
                .kernel
                .tiny_font_lease()
                .unwrap_or_else(|| panic!("picker font")),
        };
        base::SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        let runtime = cx.kernel.input_runtime();
        let mut observer = runtime.observer();
        let mut io = Io {
            observer: &mut *observer,
        };
        cx.run_input(&mut io, false, |cx, _| {
            do_pick_battle_ship(cx, &mut state, input) != 0
        });
        crate::automation::ui_observation::observe_ship_picker(false);
        crate::ui::sis::set_flash_rect(
            &cx.kernel.sis_highlight,
            &state.screen,
            context.clip_rect(),
            None,
        );
        if let Some(chosen) = state.chosen {
            if Some(chosen) == links::last_ship(cx.kernel, RPG_PLAYER_NUM) {
                cx.b.counters[RPG_PLAYER_NUM] = 1;
            }
            WaitForSoundEnd(0);
        }
        state.chosen
    })
}

pub use crate::battle::context_access::reader_legacy::{
    get_encounter_star_ship, get_encounter_star_ship_with_picker,
};

/// Select from the caller's live fleet, lending the same owner to the blocking picker.
pub(crate) fn encounter_ship(
    cx: &mut Cx<'_>,
    input: &crate::engine::input::InputSession,
    last: Option<ShipId>,
    which_player: u16,
    activity: ActivityControl,
    pick: impl FnOnce(&mut Cx<'_>, u16) -> Option<ShipId>,
) -> Option<ShipId> {
    let activity_word = activity.activity();
    let side = usize::from(which_player);
    if activity_word as u8 == IN_HYPERSPACE as u8 {
        assert_eq!(side, RPG_PLAYER_NUM);
        let queue = cx.kernel.ships(side);
        return queue.head().map(|id| queue.ship_id(id));
    }
    if activity_word & 0xFF == SUPER_MELEE {
        if cx.b.counters.contains(&0) {
            return None;
        }
        return pick(cx, which_player);
    }
    if side == RPG_PLAYER_NUM {
        let ships_left = cx.b.counters[side];
        if last.is_none() && ships_left == 1 {
            let queue = cx.kernel.ships(side);
            return queue.tail().map(|id| queue.ship_id(id));
        }
        if ships_left != 0 {
            // SAFETY: the campaign renderer remains on its existing bounded provider path.
            return get_armada_star_ship(cx, input, activity);
        }
        if let Some(last) = last {
            let crew = cx.pick_ship(last).crew;
            let sis = &mut cx.world.data.flagship;
            if crew == 0 {
                sis.crew_enlisted = u16::MAX;
            } else if sis.fuel_on_board > RUN_AWAY_FUEL_COST {
                sis.fuel_on_board -= RUN_AWAY_FUEL_COST;
            } else {
                sis.fuel_on_board = 0;
            }
        }
        return None;
    }
    if fleet_is_infinite(cx.world, which_player) != 0 {
        if let Some(last) = last {
            let captain = cx.with_legacy(production_captain_name);
            let mut queue = cx.kernel.ships(side);
            queue.edit(queue.link(last), |value| {
                value.crew = value.max_crew;
                value.player = which_player as i16;
                value.captain = captain;
            });
        }
        cx.b.counters[side] += 1;
        let queue = cx.kernel.ships(side);
        return queue.head().map(|id| queue.ship_id(id));
    }
    let queue = cx.kernel.ships(side);
    last.map_or_else(|| queue.head(), |last| queue.succ(queue.link(last)))
        .map(|id| queue.ship_id(id))
}

/// `DrawArmadaPickShip`: draw the picker frame, the SIS name and the escorts'
/// icons; `pick_rect` receives the frame (and, with the salvage frame, the
/// rectangle of that).
///
/// # Safety
/// `pick_rect` is valid for writing; game thread only.
pub(crate) fn draw_armada_pick_ship_in(
    kernel: &KernelOwner,
    world: &crate::engine::World,
    draw_salvage_frame: c_int,
    pick_rect: &mut CRect,
) {
    use crate::graphics::{
        context_abi::ContextLease, drawable_abi::LoadedDrawable, native_font::FontLease,
    };
    use std::{ffi::CStr, ptr::NonNull};
    let (context, font, status_frame) = {
        (
            ContextLease::retain(
                NonNull::new(kernel.space_context.cast())
                    .unwrap_or_else(|| panic!("missing picker context")),
            ),
            FontLease::retain(
                NonNull::new(kernel.star_con_font.cast())
                    .unwrap_or_else(|| panic!("missing picker font")),
            ),
            crate::graphics::drawable_abi::retain_frame_allocations(&[kernel.status_frame.cast()])
                [0]
            .context_frame(3),
        )
    };
    let owner = LoadedDrawable::load(c"graphics.pickship")
        .unwrap_or_else(|| panic!("missing picker graphic"));
    let frames = owner.lease();
    let frame = frames.context_frame(0);
    frame.replace_hotspot(crate::graphics::native_abi::POINT { x: 0, y: 0 });
    let mut pick = frame.rect();
    let result = context.draw_batch(|| {
        use crate::graphics::native_abi::POINT;
        let clip = context.clip_rect();
        pick.corner.x = (clip.extent.width >> 1) - (pick.extent.width >> 1);
        pick.corner.y = (clip.extent.height >> 1) - (pick.extent.height >> 1);
        let result = if draw_salvage_frame == 0 {
            pick
        } else {
            let salvage_frame = frames.context_frame(1);
            salvage_frame.replace_hotspot(POINT { x: 0, y: 0 });
            let mut salvage = salvage_frame.rect();
            let origin = POINT {
                x: (clip.extent.width >> 1) - (salvage.extent.width >> 1),
                y: pick.corner.y - (salvage.extent.height >> 1),
            };
            context.draw_stamp(&salvage_frame, origin);
            pick.corner.y = origin.y + salvage.extent.height;
            salvage.corner = POINT {
                x: pick.corner.x,
                y: origin.y,
            };
            salvage
        };
        context.draw_stamp(&frame, pick.corner);
        let ship_name = world.data.flagship.ship_name;
        let ship_name = CStr::from_bytes_until_nul(&ship_name)
            .unwrap_or_else(|error| panic!("unterminated flagship name: {error}"));
        context.draw_text(
            &font,
            ship_name,
            POINT {
                x: pick.corner.x + (pick.extent.width >> 1),
                y: pick.corner.y + pick.extent.height - 5,
            },
            1,
            color(Rgba::rgb15(0x12, 0x12, 0x12)),
        );
        for id in links::all_links(kernel) {
            draw_escort_icon(kernel, &context, &status_frame, id, pick);
        }
        result
    });
    *pick_rect = CRect {
        corner: CPoint {
            x: result.corner.x,
            y: result.corner.y,
        },
        width: result.extent.width,
        height: result.extent.height,
    };
}

fn draw_escort_icon(
    kernel: &KernelOwner,
    context: &crate::graphics::context_abi::ContextLease,
    status: &crate::graphics::context_abi::ContextFrame,
    id: ShipId,
    pick: crate::graphics::native_abi::RECT,
) {
    use crate::graphics::{
        context_abi::ContextFrame,
        native_abi::{EXTENT, POINT, RECT},
    };
    let ship = links::ship_value(kernel, id);
    if ship.captain == 0 {
        return;
    }
    let (x, y) = icon_origin((pick.corner.x, pick.corner.y), ship.index);
    let icon = links::with_ship_resources(kernel, id, |resources| {
        ContextFrame::retain(std::ptr::NonNull::new(resources.icon().raw().cast()))
    });
    context.fill_rectangle(
        color(Rgba::BLACK),
        RECT {
            corner: POINT { x, y },
            extent: EXTENT {
                width: ICON_WIDTH,
                height: ICON_HEIGHT,
            },
        },
    );
    if ship.species != NO_ID || ship.crew == 0 {
        context.draw_stamp(&icon, POINT { x, y });
        if ship.species == NO_ID {
            context.draw_stamp(status, POINT { x: x - 1, y });
        }
    } else {
        context.draw_filled_stamp(&icon, POINT { x, y }, color(Rgba::rgb15(0x00, 0x00, 0x14)));
    }
}
