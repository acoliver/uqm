#![cfg(not(test))]

//! Shipyard menu, crew transactions and hangar animation, ported from shipyard.c.

use crate::ui::{rgb15, settings::play_menu_sound};
mod catalog;
mod crew;
use super::shipyard_selection::{
    dms_flash_active_ship, dms_flash_escort_ship_crew_count, dms_flash_flag_ship_crew_count,
    dms_set_mode, ModifyMode, SOUND_ARROWS, SOUND_CANCEL, SOUND_SELECT,
};
use super::{shipyard_frame as frame, shipyard_hangar as hangar};
use catalog::{get_available_race_count, get_available_race_from_index};
use crew::{crew_transaction, purchased_crew};

use crate::engine::battle::{link_queue::LinkId, Cx};
use crate::engine::world::queues::{FleetInfo, Fragment};
use crate::engine::{input::InputSession, World};
use crate::navigation::abi::*;
use crate::starbase::cffi::{avail_race_queue, built_ship_queue};
use crate::ui::menu::draw_campaign_menu;
use crate::ui::sis::{
    clear_flash, clear_sis_rect, delta_sis_gauges, draw_campaign_message, draw_title,
    highlight_rect, set_menu_flash,
};

use std::{mem, ptr};

use crate::front_end::fmv::runtime::DoShipSpin;

use super::shipyard_layout::{
    DMS_GetEscortShipRect, DMS_MoveCursor, HANGAR_DY, HANGAR_SHIPS_ROW, HANGAR_X_COORDS, HANGAR_Y,
    SHIP_WIN_HEIGHT, SHIP_WIN_WIDTH, USE_3DO_HANGAR,
};
const HANGAR_ANIM_RATE: u32 = 15;
const SHIPYARD: u32 = 2;
const SHIPYARD_CREW: u8 = 0;
const SHIPYARD_SAVELOAD: u8 = 1;
const SHIPYARD_EXIT: u8 = 2;
const SHIP_WIN_FRAMES: i32 = ((SHIP_WIN_WIDTH >> 1) + 1) as i32;
const MODIFY_CREW_FLAG: SIZE = 1 << 8;
use crate::starbase::shipyard::SHIP_COST;

const BLACK: Color = rgb15(0, 0, 0);

static mut POWER_STAMP: STAMP = STAMP {
    origin: POINT { x: 0, y: 0 },
    frame: ptr::null_mut(),
};
static mut POWER_COLORMAP: COLORMAP = ptr::null_mut();
static mut POWER_NEXT_TIME: TimeCount = 0;

unsafe fn animate_power_lines(menu: Option<&MENU_STATE>) {
    let now = GetTimeCounter();
    if let Some(menu) = menu {
        POWER_STAMP.origin.x = 0;
        POWER_STAMP.origin.y = 0;
        POWER_STAMP.frame = SetAbsFrameIndex(menu.ModuleFrame, 24);
        POWER_COLORMAP = SetAbsStringTableIndex(menu.CurString, 0);
    }
    if now >= POWER_NEXT_TIME || menu.is_some() {
        POWER_NEXT_TIME = now.wrapping_add(ONE_SECOND / HANGAR_ANIM_RATE);
        SetColorMap(GetColorMapAddress(POWER_COLORMAP));
        DrawStamp(ptr::addr_of_mut!(POWER_STAMP));
        POWER_COLORMAP = SetRelStringTableIndex(POWER_COLORMAP, 1);
    }
}

unsafe fn draw_race_strings(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MENU_STATE,
    mut race: BYTE,
) {
    let old = SetContext(cx.kernel.status_context);
    let mut r: RECT = mem::zeroed();
    GetContextClipRect(&mut r);
    let mut stamp = STAMP {
        origin: POINT {
            x: (4 + ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32) as i16 - r.corner.x,
            y: (SIS_ORG_Y as i32 + ScreenHeight - SAFE_Y as i32 * 2 - 13 - RADAR_HEIGHT as i32)
                as i16
                - r.corner.y,
        },
        frame: ptr::null_mut(),
    };
    r.corner.x = stamp.origin.x - 1;
    r.corner.y = stamp.origin.y - 11;
    r.extent.width = RADAR_WIDTH as i16 + 2;
    r.extent.height = 11;
    BatchGraphics();
    {
        let dimensions = cx.screen_dimensions();
        clear_sis_rect(cx.kernel, cx.world, dimensions, CLEAR_SIS_RADAR as BYTE)
    };
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x0a));
    DrawFilledRectangle(&mut r);
    r.corner = stamp.origin;
    r.extent.width = RADAR_WIDTH as i16;
    r.extent.height = RADAR_HEIGHT as i16;
    SetContextForeGroundColor(BLACK);
    DrawFilledRectangle(&mut r);
    if race != BYTE::MAX {
        let Some((index, ship)) = cx.with_legacy(|| get_available_race_from_index(race)) else {
            panic!("the selected hangar race must be allied");
        };
        race = index as BYTE;
        stamp.frame = SetAbsFrameIndex(menu.ModuleFrame, 3 + race as COUNT);
        DrawStamp(&mut stamp);
        stamp.frame = ship.melee_icon;
        let mut text = TEXT {
            baseline: POINT {
                x: stamp.origin.x + RADAR_WIDTH as i16 - 2,
                y: stamp.origin.y + RADAR_HEIGHT as i16 - 2,
            },
            align: TEXT_ALIGN_ALIGN_RIGHT,
            CharCount: COUNT::MAX,
            pStr: ptr::null(),
        };
        stamp.origin.x += (RADAR_WIDTH >> 1) as i16;
        stamp.origin.y += (RADAR_HEIGHT >> 1) as i16;
        DrawStamp(&mut stamp);
        let cost = format!("{}\0", SHIP_COST[race as usize]);
        text.pStr = cost.as_ptr().cast();
        SetContextFont(cx.kernel.tiny_font);
        SetContextForeGroundColor(rgb15(0, 0x1f, 0));
        font_DrawText(&mut text);
    }
    UnbatchGraphics();
    SetContext(old);
}

unsafe fn show_combat_ship(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    window: COUNT,
    yanked: Option<LinkId<Fragment>>,
) {
    let (mut windows, num_ships) = cx.with_legacy(|| {
        built_ship_queue(|queue| hangar::prepare_windows(queue, &*menu, window, yanked))
    });
    if num_ships != 0 {
        let mut all_finished = false;
        let r = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: SHIP_WIN_WIDTH,
                height: SHIP_WIN_HEIGHT,
            },
        };
        FlushInput(input);
        let mut time_in = GetTimeCounter();
        let mut j = 0;
        while j < SHIP_WIN_FRAMES && !all_finished {
            cx.with_legacy(|| SleepThreadUntil(time_in.wrapping_add(ONE_SECOND / 24)));
            time_in = GetTimeCounter();
            if AnyButtonPress(cx, input, 0) != 0 {
                if yanked.is_some() {
                    windows[0].left_door.origin.x = 0;
                    windows[0].right_door.origin.x = 0;
                }
                all_finished = true;
            }
            let old = SetContext(cx.kernel.space_context);
            let mut old_clip: RECT = mem::zeroed();
            GetContextClipRect(&mut old_clip);
            SetContextBackGroundColor(BLACK);
            BatchGraphics();
            for i in 0..num_ships {
                let info = &mut windows[i as usize];
                let mut clip = RECT {
                    corner: POINT {
                        x: SIS_ORG_X as i16 + info.finished.x,
                        y: SIS_ORG_Y as i16 + info.finished.y,
                    },
                    extent: EXTENT {
                        width: SHIP_WIN_WIDTH,
                        height: SHIP_WIN_HEIGHT,
                    },
                };
                SetContextClipRect(&mut clip);
                ClearDrawable();
                DrawStamp(&mut info.image);
                let Some(ship) = info.ship else {
                    panic!("a prepared hangar window must have a ship");
                };
                show_ship_crew(cx.world, ship, &r);
                if !all_finished || yanked.is_some() {
                    DrawStamp(&mut info.left_door);
                    DrawStamp(&mut info.right_door);
                    if yanked.is_some() {
                        info.left_door.origin.x += 1;
                        info.right_door.origin.x -= 1;
                    } else {
                        info.left_door.origin.x -= 1;
                        info.right_door.origin.x += 1;
                    }
                }
            }
            SetContextClipRect(&mut old_clip);
            if !USE_3DO_HANGAR {
                cx.with_legacy(|| animate_power_lines(None));
            }
            UnbatchGraphics();
            SetContext(old);
            j += 1;
        }
    }
}

unsafe fn dms_spin_ship(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MENU_STATE,
    ship: Option<LinkId<Fragment>>,
) -> bool {
    if menu.CurState >> 4 != 0 {
        return false;
    }
    if ship.is_none() == (menu.delta_item & MODIFY_CREW_FLAG == 0) {
        return false;
    }
    let spin_ship = if let Some(id) = ship {
        let race = cx.with_legacy(|| built_ship_queue(|queue| queue.get(id).race_id));
        cx.with_legacy(|| {
            avail_race_queue(|queue| queue.at(usize::from(race)).map(|id| *queue.get(id)))
        })
    } else {
        cx.with_legacy(|| get_available_race_from_index(menu.delta_item as BYTE))
            .map(|(_, fleet)| fleet)
    };
    let Some(spin_ship) = spin_ship else {
        return false;
    };
    clear_flash(cx.kernel);
    use crate::ships::master_native::find;
    let species = spin_ship.species_id;
    let (context, index) = (
        cx.kernel.screen_context,
        find(&cx.kernel.master_queue, species),
    );
    let old = SetContext(context);
    let mut old_clip: RECT = mem::zeroed();
    GetContextClipRect(&mut old_clip);
    if index >= 0 && index < (MeleeShip_MELEE_ZOQFOTPIK + 1) as i32 {
        cx.with_legacy(|| DoShipSpin(index as COUNT, menu.hMusic));
    }
    SetContextClipRect(&mut old_clip);
    SetContext(old);
    true
}

unsafe fn dms_modify_crew(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    handle: Option<LinkId<Fragment>>,
    dy: SBYTE,
) {
    let delta = match handle {
        None => {
            let delta = if dy < 0 {
                dms_hire_flag_ship_crew(cx, input)
            } else {
                dms_dismiss_flag_ship_crew(cx, input)
            };
            if delta != 0 {
                dms_flash_flag_ship_crew_count(cx);
            }
            delta
        }
        Some(ship) => {
            let delta = if dy < 0 {
                dms_hire_escort_ship_crew(cx, ship)
            } else {
                dms_dismiss_escort_ship_crew(cx, ship)
            };
            if delta != 0 {
                let slot = cx.world.built_ships().get(ship).index;
                dms_flash_escort_ship_crew_count(cx, slot);
            }
            delta
        }
    };
    if delta == 0 {
        play_menu_sound(
            cx.kernel,
            cx.world,
            MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16,
        );
    }
    if handle.is_some() {
        menu.delta_item &= MODIFY_CREW_FLAG;
    }
    crew_transaction(cx, delta, |cx| {
        draw_campaign_menu(cx, PM_CREW as BYTE, SHIPYARD_CREW as i16);
    });
}

unsafe fn dms_try_add_escort_ship(cx: &mut Cx<'_>, input: &InputSession, menu: &mut MENU_STATE) {
    let (index, added) = {
        let Some((index, template)) =
            cx.with_legacy(|| get_available_race_from_index(menu.delta_item as BYTE))
        else {
            panic!("the purchased hangar race must be allied");
        };
        let added = cx.world.data.flagship.res_units >= SHIP_COST[index as usize] as DWORD
            && cx.with_legacy(|| {
                built_ship_queue(|queue| {
                    queue
                        .clone_fragment(
                            &template,
                            index as u8,
                            1,
                            crate::comm::dispatch::production_captain_name,
                        )
                        .is_some()
                })
            });
        (index, added)
    };
    if added {
        show_combat_ship(cx, input, menu, menu.CurState as COUNT, None);
        {
            draw_campaign_menu(cx, PM_CREW as BYTE, SHIPYARD_CREW as i16);
            delta_sis_gauges(
                cx.kernel,
                cx.world,
                UNDEFINED_DELTA as SIZE,
                UNDEFINED_DELTA as SIZE,
                -(SHIP_COST[index as usize] as i32),
            );
            dms_set_mode(cx, input, menu.CurState, ModifyMode::EditCrew);
        };
    } else {
        play_menu_sound(
            cx.kernel,
            cx.world,
            MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16,
        );
    }
}

unsafe fn dms_add_escort_ship(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    special: bool,
    select: bool,
    cancel: bool,
    movement: (SBYTE, SBYTE),
) {
    let highlight = cx.kernel.sis_highlight.clone();
    let (dx, dy) = movement;
    assert!(menu.delta_item & MODIFY_CREW_FLAG != 0);
    if special {
        {
            let ship = cx.with_legacy(|| {
                built_ship_queue(|queue| {
                    queue.find(|ship| COUNT::from(ship.index) == menu.delta_item as COUNT)
                })
            });
            if dms_spin_ship(cx, menu, ship) {
                dms_set_mode(cx, input, menu.CurState, ModifyMode::AddEscort);
            }
        };
        return;
    }
    if cancel {
        {
            menu.delta_item &= !MODIFY_CREW_FLAG;
            clear_flash(cx.kernel);
            draw_campaign_menu(cx, PM_CREW as BYTE, SHIPYARD_CREW as i16);
            dms_set_mode(cx, input, menu.CurState, ModifyMode::Navigate);
        };
    } else if select {
        dms_try_add_escort_ship(cx, input, menu);
    } else if dx != 0 || dy != 0 {
        {
            let available = cx.with_legacy(get_available_race_count);
            let mut current = menu.delta_item as BYTE;
            if dx < 0 || dy < 0 {
                let old = current;
                current = current.wrapping_sub(1);
                if old == 0 {
                    current = available.wrapping_sub(1) as BYTE;
                }
            } else if dx > 0 || dy > 0 {
                current = current.wrapping_add(1);
                if current as COUNT == available {
                    current = 0;
                }
            }
            if current != menu.delta_item as BYTE {
                highlight.pre_update_flash_rect();
                draw_race_strings(cx, menu, current);
                highlight.post_update_flash_rect();
                menu.delta_item = current as SIZE | MODIFY_CREW_FLAG;
            }
        };
    }
}

unsafe fn dms_scrap_escort_ship(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    handle: LinkId<Fragment>,
) {
    clear_flash(cx.kernel);
    show_combat_ship(cx, input, menu, menu.CurState as COUNT, Some(handle));
    {
        cx.with_legacy(|| {
            built_ship_queue(|queue| {
                queue.remove(handle);
                queue.free(handle);
            })
        });
        delta_sis_gauges(
            cx.kernel,
            cx.world,
            UNDEFINED_DELTA as SIZE,
            UNDEFINED_DELTA as SIZE,
            UNDEFINED_DELTA as i32,
        );
        SetContext(cx.kernel.space_context);
        dms_set_mode(cx, input, menu.CurState, ModifyMode::Navigate);
    };
}

unsafe fn dms_edit_crew_mode(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    ship: Option<LinkId<Fragment>>,
    select: bool,
    cancel: bool,
    dy: SBYTE,
) {
    if select || cancel {
        if let Some(ship) = ship {
            let crew = cx.with_legacy(|| built_ship_queue(|queue| queue.get(ship).crew_level));
            if crew == 0 {
                dms_scrap_escort_ship(cx, input, menu, ship);
            }
        }
        menu.delta_item &= !MODIFY_CREW_FLAG;
        dms_set_mode(cx, input, menu.CurState, ModifyMode::Navigate);
    } else if dy != 0 {
        dms_modify_crew(cx, input, menu, ship, dy);
    }
}

unsafe fn dms_navigate_ship_slots(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    special: bool,
    select: bool,
    cancel: bool,
    direction: (SBYTE, SBYTE),
) {
    let (dx, dy) = direction;
    let ship = cx.with_legacy(|| {
        built_ship_queue(|queue| {
            queue.find(|ship| COUNT::from(ship.index) == menu.CurState as COUNT)
        })
    });
    if dx != 0 || dy != 0 {
        let new_state = DMS_MoveCursor(menu.CurState, dx, dy);
        if new_state != menu.CurState {
            menu.CurState = new_state;
            dms_flash_active_ship(cx, menu.CurState);
        }
    }
    if special {
        if dms_spin_ship(cx, menu, ship) {
            dms_set_mode(cx, input, menu.CurState, ModifyMode::Navigate);
        }
    } else if select {
        if ship.is_none() && menu.CurState >> 4 == 0 {
            menu.delta_item = MODIFY_CREW_FLAG;
            draw_race_strings(cx, menu, 0);
            dms_set_mode(cx, input, menu.CurState, ModifyMode::AddEscort);
        } else {
            menu.delta_item |= MODIFY_CREW_FLAG;
            dms_set_mode(cx, input, menu.CurState, ModifyMode::EditCrew);
        }
    } else if cancel {
        menu.InputFunc = Some(do_shipyard);
        menu.CurState = SHIPYARD_CREW;
        draw_campaign_menu(cx, PM_CREW as BYTE, menu.CurState as i16);
        dms_set_mode(cx, input, menu.CurState, ModifyMode::Exit);
    }
}

unsafe fn do_modify_ships(cx: &mut Cx<'_>, menu: &mut MENU_STATE, input: &InputSession) -> BOOLEAN {
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        menu.InputFunc = Some(do_shipyard);
        return 1;
    }
    if menu.Initialized == 0 {
        {
            menu.InputFunc = Some(do_modify_ships);
            menu.Initialized = 1;
            menu.CurState = 0xf0;
            menu.delta_item = 0;
            SetContext(cx.kernel.space_context);
            dms_set_mode(cx, input, menu.CurState, ModifyMode::Navigate);
        };
    } else {
        let special = input.pulsed().menu[KEY_MENU_SPECIAL as usize] != 0;
        let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
        let cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
        let mut dx = 0;
        let mut dy = 0;
        if input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0 {
            dx = 1;
        }
        if input.pulsed().menu[KEY_MENU_LEFT as usize] != 0 {
            dx = -1;
        }
        if input.pulsed().menu[KEY_MENU_UP as usize] != 0 {
            dy = -1;
        }
        if input.pulsed().menu[KEY_MENU_DOWN as usize] != 0 {
            dy = 1;
        }
        if menu.delta_item & MODIFY_CREW_FLAG == 0 {
            dms_navigate_ship_slots(cx, input, menu, special, select, cancel, (dx, dy));
        } else {
            let ship = cx.with_legacy(|| {
                built_ship_queue(|queue| {
                    queue.find(|ship| COUNT::from(ship.index) == menu.CurState as COUNT)
                })
            });
            if ship.is_none() && menu.CurState >> 4 == 0 {
                dms_add_escort_ship(cx, input, menu, special, select, cancel, (dx, dy));
            } else {
                dms_edit_crew_mode(cx, input, menu, ship, select, cancel, dy);
            }
        }
    }
    cx.with_legacy(|| SleepThread(ONE_SECOND / 30));
    1
}

unsafe fn draw_blue_print(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    flagship: &crate::engine::world::SisState,
    context: &crate::graphics::context_abi::ContextLease,
) {
    let frame = CaptureDrawable(LoadGraphicInstance(c"graphics.blueprints".as_ptr()).cast());
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: DecFrameIndex(frame),
    };
    SetContextForeGroundColor(rgb15(0, 0, 0x16));
    DrawFilledStamp(&mut stamp);
    for slot in 0..NUM_DRIVE_SLOTS {
        let piece = flagship.drive_slots[slot as usize];
        cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
    }
    for slot in 0..NUM_JET_SLOTS {
        let piece = flagship.jet_slots[slot as usize];
        cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
    }
    for slot in 0..NUM_MODULE_SLOTS {
        let piece = flagship.module_slots[slot as usize];
        if !(menu.CurState as u32 == SHIPYARD && piece as u32 == CREW_POD) {
            cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
        }
    }
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x1f));
    for slot in 0..NUM_MODULE_SLOTS {
        let piece = flagship.module_slots[slot as usize];
        if menu.CurState as u32 == SHIPYARD && piece as u32 == CREW_POD {
            cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
        }
    }
    super::sis::capacity::draw_blueprint_cells(
        context,
        flagship,
        input.preferences().fonts == OPT_PC as i32,
    );
    DestroyDrawable(ReleaseDrawable(frame));
}

unsafe fn exit_shipyard(input: &InputSession, menu: &mut MENU_STATE) -> BOOLEAN {
    SetInputCallback(input, None);
    DestroyDrawable(ReleaseDrawable(menu.ModuleFrame));
    menu.ModuleFrame = ptr::null_mut();
    DestroyStringTable(ReleaseStringTable(menu.CurString));
    menu.CurString = ptr::null_mut();
    0
}

/// Processes one shipyard input frame, including entry and exit resource handling.
///
/// # Safety
/// `menu` must point to the live `MENU_STATE` owned by the game-thread input loop.
/// Global game state, queues, graphics, resource and input systems must be initialized;
/// callbacks and this function must run exclusively on that same thread.
pub unsafe fn do_shipyard(cx: &mut Cx<'_>, menu: &mut MENU_STATE, input: &InputSession) -> BOOLEAN {
    let size = cx.screen_dimensions();
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        return cx.with_legacy(|| exit_shipyard(input, menu));
    }
    let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
    let cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
    SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
    if menu.Initialized == 0 {
        let flagship = cx.world.data.flagship;
        let context = crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context);
        {
            menu.InputFunc = Some(do_shipyard);
            menu.ModuleFrame =
                CaptureDrawable(LoadGraphicInstance(c"graphics.shipyard".as_ptr()).cast());
            menu.CurString =
                CaptureStringTable(LoadStringTableInstance(c"colortable.hangar".as_ptr()));
            menu.hMusic = (LoadMusicInstance(c"music.shipyard".as_ptr())).cast();
            SetTransitionSource(ptr::null());
            BatchGraphics();
            crate::ui::border::draw_sis_frame(cx.kernel, cx.world, size);
            let message = cx.kernel.game_text((STARBASE_STRING_BASE + 3) as usize);
            let title = cx.kernel.game_text(STARBASE_STRING_BASE as usize);
            draw_campaign_message(cx.kernel, cx.world, size.0, Some(message));
            draw_title(cx.kernel, size.0, title);
            SetContext(cx.kernel.space_context);
            draw_blue_print(cx, input, menu, &flagship, &context);
            menu.CurState = SHIPYARD_CREW;
            draw_campaign_menu(cx, PM_CREW as BYTE, menu.CurState as i16);
            let mut stamp = STAMP {
                origin: POINT { x: 0, y: 0 },
                frame: SetAbsFrameIndex(menu.ModuleFrame, 0),
            };
            if USE_3DO_HANGAR {
                DrawStamp(&mut stamp);
            } else {
                let mut old_r: RECT = mem::zeroed();
                GetContextClipRect(&mut old_r);
                let mut r = old_r;
                r.corner.x -= 1;
                r.extent.width += 2;
                r.extent.height += 1;
                SetContextClipRect(&mut r);
                DrawStamp(&mut stamp);
                SetContextClipRect(&mut old_r);
                animate_power_lines(Some(&*menu));
            }
            SetContextFont(cx.kernel.tiny_font);
            ScreenTransition(3, ptr::null());
            UnbatchGraphics();
            crate::ui::settings::play_music_for_world(cx.world, menu.hMusic, 1, 1);
        };
        show_combat_ship(cx, input, menu, COUNT::MAX, None);
        frame::install(input, context, || {
            // SAFETY: the shipyard retains its published menu until this hook is removed.
            unsafe { animate_power_lines(None) };
        });
        set_menu_flash(cx.kernel, size, false);
        menu.Initialized = 1;
    } else if cancel || (select && menu.CurState == SHIPYARD_EXIT) {
        return cx.with_legacy(|| exit_shipyard(input, menu));
    } else if select && menu.CurState != SHIPYARD_SAVELOAD {
        menu.Initialized = 0;
        do_modify_ships(cx, menu, input);
    } else if select {
        let mut io = crate::engine::observe::Io {
            observer: &mut *cx.kernel.input_runtime().observer(),
        };
        if crate::front_end::gameopt::game_options(cx, &mut io) == 0 {
            return cx.with_legacy(|| exit_shipyard(input, menu));
        }
        draw_campaign_menu(cx, PM_CREW as BYTE, menu.CurState as i16);
        set_menu_flash(cx.kernel, size, false);
    } else {
        crate::ui::menu::choose_menu_state(cx, input, &mut menu.CurState, PM_CREW as BYTE);
    }
    1
}

unsafe fn show_ship_crew(world: &mut World, id: LinkId<Fragment>, rect: &RECT) {
    let (has_crew, buffer) = crew::label(world, id);
    let mut r = *rect;
    let mut text = TEXT {
        baseline: POINT {
            x: r.corner.x + (r.extent.width >> 1),
            y: r.corner.y + r.extent.height - 1,
        },
        align: TEXT_ALIGN_ALIGN_CENTER,
        pStr: buffer.as_ptr().cast(),
        CharCount: COUNT::MAX,
    };
    if r.corner.y != 0 {
        r.corner.y = text.baseline.y - 6;
        r.extent.width = SHIP_WIN_WIDTH;
        r.extent.height = 6;
        SetContextForeGroundColor(BLACK);
        DrawFilledRectangle(&mut r);
    }
    SetContextForeGroundColor(if has_crew {
        rgb15(0, 0x14, 0)
    } else {
        rgb15(0x12, 0, 0)
    });
    font_DrawText(&mut text);
}

unsafe fn dms_hire_escort_ship_crew(cx: &mut Cx<'_>, id: LinkId<Fragment>) -> SIZE {
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

unsafe fn dms_dismiss_escort_ship_crew(cx: &mut Cx<'_>, id: LinkId<Fragment>) -> SIZE {
    let ship = *cx.world.built_ships().get(id);
    let mut delta = 0;
    if ship.crew_level > 0 {
        let cost = if ship.crew_level > 1 {
            let bought = purchased_crew(cx.world);
            cx.world.data.game.crew_cost as i32
                - if bought == CREW_EXPENSE_THRESHOLD as SIZE {
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
            MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16,
        );
    }
    cx.kernel.sis_highlight.pre_update_flash_rect();
    let slot = cx.world.built_ships().get(id).index;
    let r = DMS_GetEscortShipRect(slot);
    show_ship_crew(cx.world, id, &r);
    cx.kernel.sis_highlight.post_update_flash_rect();
    delta
}

unsafe fn dms_hire_flag_ship_crew(cx: &mut Cx<'_>, input: &InputSession) -> SIZE {
    let Some(cell) = super::sis::capacity::crew_cell(
        &cx.world.data.flagship,
        input.preferences().fonts == OPT_PC as i32,
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

unsafe fn dms_dismiss_flag_ship_crew(cx: &mut Cx<'_>, input: &InputSession) -> SIZE {
    let highlight = cx.kernel.sis_highlight.clone();
    if cx.world.data.flagship.crew_enlisted == 0 {
        return 0;
    }
    let bought = purchased_crew(cx.world);
    cx.with_legacy(|| highlight.pre_update_flash_rect());
    let cost = cx.world.data.game.crew_cost as i32
        - if bought == CREW_EXPENSE_THRESHOLD as SIZE {
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
        input.preferences().fonts == OPT_PC as i32,
    )
    .unwrap_or_else(|| panic!("dismissed flagship crew exceeds capacity"));
    let context = crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context);
    context.set_foreground(cell.foreground);
    context.draw_point(BLACK, cell.point);
    -1
}
