//! The C entry points of the ship picker: `GetEncounterStarShip` and
//! `DrawArmadaPickShip`, with the drawing and input calls of `pickship.c` in
//! the order the C made them. The rules they apply are in the parent module.

use crate::comm::dispatch::production_captain_name;
use crate::engine::{
    battle::{
        link_queue::{LinkQueue, Starship},
        Cx, ShipId,
    },
    observe::Io,
    world::activity::ActivityControl,
    Kernel as KernelOwner, WorldData as WorldFields,
};

use std::ffi::{c_char, c_int};

use super::{
    crew_label, crew_strip, find_escort, highlight, icon_origin, moved, name_strip, slot, Area,
    Cursor, Direction, Escort, Slot, ICON_HEIGHT, ICON_WIDTH, NO_ID,
};
use crate::c_bindings::controller_input::{
    KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_RIGHT, KEY_MENU_SELECT, KEY_MENU_UP,
};

use crate::comm::{
    locdata::{CColor, CPoint, CRect},
    sis_graphics::CText,
};

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
const ALIGN_CENTER: c_int = 1;
/// `RUN_AWAY_FUEL_COST`.
const RUN_AWAY_FUEL_COST: u32 = 5 * FUEL_TANK_SCALE;

use crate::sound::heart_ffi::wait_for_sound_end as WaitForSoundEnd;

use crate::graphics::{
    context_abi::SetContextForeGroundColor,
    font_abi::{font_DrawText, SetContextFont},
    frame_abi::DrawFilledRectangle,
};

use crate::ui::sis::{capacity::GetCrewPodCapacity, SetFlashRect};

use crate::battle::encount_native::engine::fleet_is_infinite;

use crate::strings::string_table::{GetStringAddress, GetStringLength, SetAbsStringTableIndex};

use crate::engine::world::IN_HYPERSPACE;

fn color(color: Rgba) -> CColor {
    CColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn set_color(rgba: Rgba) {
    // SAFETY: plain value call on the current context.
    unsafe { SetContextForeGroundColor(color(rgba)) };
}

fn fill(area: Area) {
    let mut rect = CRect {
        corner: CPoint {
            x: area.x,
            y: area.y,
        },
        width: area.width,
        height: area.height,
    };
    // SAFETY: `rect` is valid for the call.
    unsafe { DrawFilledRectangle(std::ptr::from_mut(&mut rect).cast()) };
}

fn draw_text(x: i16, y: i16, text_ptr: *const c_char, char_count: u16) {
    let mut text = CText::default();
    text.baseline = CPoint { x, y };
    text.p_str = text_ptr;
    text.align = ALIGN_CENTER;
    text.char_count = char_count;
    // SAFETY: `text` is valid for the call and `p_str` outlives it.
    unsafe { font_DrawText(std::ptr::from_mut(&mut text).cast()) };
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
    flash: CRect,
}

#[cfg(not(test))]
pub(crate) fn live_escort_indices() -> Vec<u16> {
    escort_links()
        .into_iter()
        .filter_map(|(_, escort)| escort.alive.then_some(escort.index))
        .collect()
}

/// Every `STARSHIP` of the player's queue before the flagship, which is the
/// tail.
fn escort_links() -> Vec<(ShipId, Escort)> {
    let tail = LinkQueue::<Starship>::last_ship(RPG_PLAYER_NUM);
    LinkQueue::<Starship>::all_links()
        .into_iter()
        .filter(|id| Some(*id) != tail)
        .map(|id| {
            let ship = LinkQueue::<Starship>::ship_value(id);
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

/// `ChangeSelection:` of `DoPickBattleShip`: move the highlight to the cursor
/// cell, redraw the captain's name and crew, and re-aim the flashing rect.
///
/// # Safety
/// `state` is the picker's state, which `DoInput` is running.
unsafe fn change_selection(state: &mut PickerState) {
    crate::automation::ui_observation::observe_ship_picker_cursor(state.cursor);
    let area = highlight(state.frame, state.cursor);
    state.flash = CRect {
        corner: CPoint {
            x: area.x,
            y: area.y,
        },
        width: area.width,
        height: area.height,
    };

    state.chosen = match slot(state.cursor) {
        Slot::Flagship => LinkQueue::<Starship>::last_ship(RPG_PLAYER_NUM),
        Slot::Escort(ship_index) => {
            let links = escort_links();
            let escorts: Vec<Escort> = links.iter().map(|(_, escort)| *escort).collect();
            find_escort(&escorts, ship_index).map(|at| links[at].0)
        }
    };

    set_color(Rgba::BLACK);
    let name = name_strip(state.frame);
    fill(name);

    let (crew, max_crew) = if let Some(chosen) = state.chosen {
        // SAFETY: font globals are set at startup.
        unsafe { SetContextFont((KernelOwner::current().read(|kernel| kernel.tiny_font)).cast()) };
        let baseline_x = name.x + (name.width >> 1);
        let baseline_y = name.y + (name.height - 1);
        // SAFETY: `chosen` is a link of `race_q`.
        let ship = LinkQueue::<Starship>::ship_value(chosen);
        let (text, length, crew, max_crew) = if ship.captain == 0 {
            // The legacy picker left the flagship captain strip empty: its name
            // pointer escaped a temporary SIS copy. Preserve that presentation
            // without keeping the dangling pointer; crew still comes from World.
            // SAFETY: the published lifecycle owns the flagship on this thread.
            let crew = WorldFields::current().read(|world| world.flagship.crew_enlisted);
            // SAFETY: plain call.
            let capacity = GetCrewPodCapacity();
            (c"".as_ptr(), 0, crew, capacity)
        } else {
            // SAFETY: `race_strings` is the ship's name table.
            let entry = unsafe {
                SetAbsStringTableIndex(
                    LinkQueue::with_ship_resources(chosen, |r| r.strings()).cast(),
                    u16::from(ship.captain),
                )
            };
            // SAFETY: `entry` came from `SetAbsStringTableIndex`.
            let (text, length) = unsafe { (GetStringAddress(entry), GetStringLength(entry)) };
            (text.cast_const(), length, ship.crew, ship.max_crew)
        };
        set_color(Rgba::rgb15(0x14, 0x0A, 0x00));
        draw_text(baseline_x, baseline_y, text, length);
        set_color(Rgba::BLACK);
        (crew, max_crew)
    } else {
        (0, 0)
    };
    set_color(Rgba::BLACK);
    let crew_area = crew_strip(name);
    fill(crew_area);

    if let Some(label) = crew_label(crew, max_crew) {
        set_color(Rgba::rgb15(0x00, 0x14, 0x00));
        let text = std::ffi::CString::new(label).expect("a crew count has no NUL byte");
        draw_text(
            crew_area.x + (crew_area.width >> 1),
            name.y + (name.height - 1),
            text.as_ptr(),
            u16::MAX,
        );
    }

    // SAFETY: `state.flash` outlives the flash: the picker clears it after
    // `DoInput` returns.
    unsafe {
        SetFlashRect(std::ptr::null());
        SetFlashRect(std::ptr::addr_of!(state.flash).cast());
    }
}

/// `DoPickBattleShip`: one input frame of the picker.
unsafe fn do_pick_battle_ship(
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
        cx.with_legacy(|| unsafe { change_selection(state) });
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
            cx.with_legacy(|| unsafe { change_selection(state) });
        }
    }

    // SAFETY: plain call.
    cx.with_legacy(|| unsafe { crate::battle::c_bridge::SleepThread(ONE_SECOND / 30) });
    1
}

fn base_activity_aborted(activity: &ActivityControl) -> bool {
    activity.activity() & CHECK_ABORT != 0
}

/// `GetArmadaStarShip`: let the player pick one of the fleet's ships.
///
/// # Safety
/// Game thread, inside a full-game battle.
unsafe fn get_armada_star_ship(
    input: &crate::engine::input::InputSession,
    activity: ActivityControl,
) -> Option<ShipId> {
    // SAFETY: `battle_counter` has a slot for each side.
    if unsafe { crate::app::bridge::battle(|b| b.counters[1]) } == 0 {
        // No opponents left.
        return None;
    }

    // SAFETY: the context global is set at startup.
    let old_context =
        unsafe { base::SetContext(KernelOwner::current().read(|kernel| kernel.space_context)) };
    let mut pick = CRect::default();
    // SAFETY: `pick` is valid for the call.
    unsafe { draw_armada_pick_ship(0, &mut pick) };

    let mut state = PickerState {
        initialized: false,
        activity,
        cursor: Cursor::START,
        chosen: None,
        frame: (pick.corner.x, pick.corner.y),
        flash: CRect::default(),
    };
    // SAFETY: the synchronous frame retains its private picker state and input resources.
    unsafe {
        base::SetMenuSounds(input, MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        base::DoInput(0, &mut |cx: &mut Cx<'_>, _: &mut Io<'_>| {
            do_pick_battle_ship(cx, &mut state, input) != 0
        });
    }
    crate::automation::ui_observation::observe_ship_picker(false);
    // SAFETY: plain call.
    unsafe { SetFlashRect(std::ptr::null()) };

    let chosen = state.chosen;
    if chosen.is_some() {
        if chosen == LinkQueue::<Starship>::last_ship(RPG_PLAYER_NUM) {
            // Player chose SIS. There will be no more choices.
            // SAFETY: `battle_counter` has a slot for each side.
            unsafe { crate::app::bridge::battle(|b| b.counters[RPG_PLAYER_NUM] = 1) };
        }
        // SAFETY: plain call.
        WaitForSoundEnd(0);
    }

    // SAFETY: restores the context saved above.
    unsafe { base::SetContext(old_context) };
    chosen
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
            return cx.with_legacy(|| unsafe { get_armada_star_ship(input, activity) });
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
#[export_name = "DrawArmadaPickShip"]
pub unsafe extern "C" fn draw_armada_pick_ship(draw_salvage_frame: c_int, pick_rect: *mut CRect) {
    use crate::graphics::{
        context_abi::ContextLease, drawable_abi::LoadedDrawable, native_font::FontLease,
    };
    use std::{ffi::CStr, ptr::NonNull};
    let (context, font, status_frame) = KernelOwner::current().read(|kernel| {
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
    });
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
        let ship_name = WorldFields::current().read(|world| world.flagship.ship_name);
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
        for id in LinkQueue::<Starship>::all_links() {
            draw_escort_icon(&context, &status_frame, id, pick);
        }
        result
    });
    // SAFETY: the legacy caller supplies its output; resources stay retained through the batch.
    unsafe {
        *pick_rect = CRect {
            corner: CPoint {
                x: result.corner.x,
                y: result.corner.y,
            },
            width: result.extent.width,
            height: result.extent.height,
        };
    }
}

fn draw_escort_icon(
    context: &crate::graphics::context_abi::ContextLease,
    status: &crate::graphics::context_abi::ContextFrame,
    id: ShipId,
    pick: crate::graphics::native_abi::RECT,
) {
    use crate::graphics::{
        context_abi::ContextFrame,
        native_abi::{EXTENT, POINT, RECT},
    };
    let ship = LinkQueue::<Starship>::ship_value(id);
    if ship.captain == 0 {
        return;
    }
    let (x, y) = icon_origin((pick.corner.x, pick.corner.y), ship.index);
    let icon = LinkQueue::with_ship_resources(id, |resources| {
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
