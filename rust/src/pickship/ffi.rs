//! The C entry points of the ship picker: `GetEncounterStarShip` and
//! `DrawArmadaPickShip`, with the drawing and input calls of `pickship.c` in
//! the order the C made them. The rules they apply are in the parent module.

use std::ffi::{c_char, c_int, c_void};

use super::{
    crew_label, crew_strip, find_escort, highlight, icon_origin, moved, name_strip, slot, Area,
    Cursor, Direction, Escort, Slot, ICON_HEIGHT, ICON_WIDTH, NO_ID,
};
use crate::c_bindings::controller_input::{
    PulsedInputState, KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_RIGHT, KEY_MENU_SELECT, KEY_MENU_UP,
};
use crate::c_bindings::global_state::STARSHIP;
use crate::c_bindings::live_queues;
use crate::comm::dispatch::production_captain_name;
use crate::comm::locdata::{CColor, CGlobData, CPoint, CRect, CStamp};
use crate::comm::sis_graphics::CText;
use crate::mainloop::restart_menu::c_extern as base;
use crate::mainloop::types::activity_flags::CHECK_ABORT;
use crate::starbase::gfx::Rgba;
use crate::starbase::platform::ONE_SECOND;
use crate::starbase::sis_modules::FUEL_TANK_SCALE;

/// `RPG_PLAYER_NUM`.
const RPG_PLAYER_NUM: usize = 0;
/// `LOBYTE (CurrentActivity)` of the melee game.
const SUPER_MELEE: u16 = 0;
const MENU_SOUND_MOVE: c_int = 0;
const MENU_SOUND_SUCCESS: c_int = 1;
const MENU_SOUND_NONE: u16 = 0;
/// `MENU_SOUND_UP | DOWN | LEFT | RIGHT`.
const MENU_SOUND_ARROWS: u16 = 0x0F;
const MENU_SOUND_SELECT: u16 = 1 << 4;
/// `ALIGN_CENTER`.
const ALIGN_CENTER: c_int = 1;
/// `SC2_PICK_PMAP_ANIM`.
const PICK_GRAPHIC: &[u8] = b"graphics.pickship\0";
/// `RUN_AWAY_FUEL_COST`.
const RUN_AWAY_FUEL_COST: u32 = 5 * FUEL_TANK_SCALE;

extern "C" {
    static mut SpaceContext: *mut c_void;
    static mut StatusFrame: *mut c_void;
    static mut StarConFont: *mut c_void;
    static mut TinyFont: *mut c_void;
    static mut GlobData: CGlobData;

    fn SetContextForeGroundColor(color: CColor) -> CColor;
    fn SetContextFont(font: *mut c_void) -> *mut c_void;
    fn font_DrawText(text: *mut CText);
    fn DrawFilledRectangle(rect: *mut CRect);
    fn DrawStamp(stamp: *mut CStamp);
    fn DrawFilledStamp(stamp: *mut CStamp);
    fn SetFrameHot(frame: *mut c_void, hot_spot: CPoint) -> CPoint;
    fn IncFrameIndex(frame: *mut c_void) -> *mut c_void;
    fn DecFrameIndex(frame: *mut c_void) -> *mut c_void;
    fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
    fn SetFlashRect(rect: *const c_void);
    fn PlayMenuSound(effect: c_int);
    fn SetAbsStringTableIndex(table: *mut c_void, index: u16) -> *mut c_void;
    fn GetStringAddress(entry: *mut c_void) -> *const c_char;
    fn GetStringLength(entry: *mut c_void) -> u16;
    fn WaitForSoundEnd(channel: u16);
    fn GetCrewPodCapacity() -> u16;
    fn FleetIsInfinite(player: u16) -> c_int;
    fn GetNextMeleeStarShip(player: u16, result: *mut *mut c_void) -> c_int;
    fn inHQSpace() -> c_int;
}

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
    unsafe { DrawFilledRectangle(&mut rect) };
}

fn draw_text(x: i16, y: i16, text_ptr: *const c_char, char_count: u16) {
    let mut text = CText::default();
    text.baseline = CPoint { x, y };
    text.p_str = text_ptr;
    text.align = ALIGN_CENTER;
    text.char_count = char_count;
    // SAFETY: `text` is valid for the call and `p_str` outlives it.
    unsafe { font_DrawText(&mut text) };
}

/// The picker frame's corner and what `DoInput` hands back to the callback.
/// `input_func` first, as `DoInput` requires; the rest is private to the
/// picker.
#[repr(C)]
struct PickerState {
    input_func: unsafe extern "C" fn(*mut PickerState) -> c_int,
    initialized: bool,
    cursor: Cursor,
    /// `MENU_STATE.CurFrame`: the chosen `HSTARSHIP`, null when the cell is
    /// empty.
    chosen: *mut c_void,
    frame: (i16, i16),
    /// `MENU_STATE.flash_rect0`, which `SetFlashRect` keeps pointing at.
    flash: CRect,
}

fn tail_of(queue: *const crate::comm::locdata::CQueue) -> *mut c_void {
    // SAFETY: `queue` is `race_q[RPG_PLAYER_NUM]`, which C keeps well formed.
    unsafe { (*queue).tail }
}

fn player_queue() -> *const crate::comm::locdata::CQueue {
    // SAFETY: game thread only; the queue is read while C is not changing it.
    unsafe { live_queues::player_ships(RPG_PLAYER_NUM) }
}

/// Every `STARSHIP` of the player's queue before the flagship, which is the
/// tail.
fn escort_links() -> Vec<(*mut c_void, Escort)> {
    let queue = player_queue();
    let tail = tail_of(queue);
    // SAFETY: as in `tail_of`.
    let mut link = unsafe { (*queue).head };
    let mut found = Vec::new();
    while !link.is_null() && link != tail {
        // SAFETY: every link of `race_q` is a `STARSHIP`.
        let ship = unsafe { &*link.cast::<STARSHIP>() };
        found.push((
            link,
            Escort {
                index: ship.index,
                alive: ship.SpeciesID != NO_ID,
            },
        ));
        link = ship.succ;
    }
    found
}

fn pulsed_directions() -> (Direction, bool) {
    // SAFETY: the input state is updated by `DoInput` on this thread.
    let menu = unsafe { std::ptr::addr_of!(PulsedInputState).read().menu };
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
    let area = highlight(state.frame, state.cursor);
    state.flash = CRect {
        corner: CPoint {
            x: area.x,
            y: area.y,
        },
        width: area.width,
        height: area.height,
    };

    let queue = player_queue();
    state.chosen = match slot(state.cursor) {
        Slot::Flagship => tail_of(queue),
        Slot::Escort(ship_index) => {
            let links = escort_links();
            let escorts: Vec<Escort> = links.iter().map(|(_, escort)| *escort).collect();
            find_escort(&escorts, ship_index).map_or(std::ptr::null_mut(), |at| links[at].0)
        }
    };

    set_color(Rgba::BLACK);
    let name = name_strip(state.frame);
    fill(name);

    let (crew, max_crew) = if state.chosen.is_null() {
        (0, 0)
    } else {
        // SAFETY: font globals are set at startup.
        unsafe { SetContextFont(std::ptr::addr_of!(TinyFont).read()) };
        let baseline_x = name.x + (name.width >> 1);
        let baseline_y = name.y + (name.height - 1);
        // SAFETY: `chosen` is a link of `race_q`.
        let ship = unsafe { &*state.chosen.cast::<STARSHIP>() };
        let (text, length, crew, max_crew) = if ship.captains_name_index == 0 {
            // SAFETY: the game thread owns `GlobData`.
            let sis = unsafe { &*std::ptr::addr_of!(GlobData) }.sis_state;
            // SAFETY: plain call.
            let capacity = unsafe { GetCrewPodCapacity() };
            (
                sis.commander_name.as_ptr().cast::<c_char>(),
                u16::MAX,
                sis.crew_enlisted,
                capacity,
            )
        } else {
            // SAFETY: `race_strings` is the ship's name table.
            let entry = unsafe {
                SetAbsStringTableIndex(
                    ship.race_strings as usize as *mut c_void,
                    u16::from(ship.captains_name_index),
                )
            };
            // SAFETY: `entry` came from `SetAbsStringTableIndex`.
            let (text, length) = unsafe { (GetStringAddress(entry), GetStringLength(entry)) };
            (text, length, ship.crew_level, ship.max_crew)
        };
        set_color(Rgba::rgb15(0x14, 0x0A, 0x00));
        draw_text(baseline_x, baseline_y, text, length);
        set_color(Rgba::BLACK);
        (crew, max_crew)
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
unsafe extern "C" fn do_pick_battle_ship(state: *mut PickerState) -> c_int {
    // SAFETY: `DoInput` passes the state `get_armada_star_ship` owns.
    let state = unsafe { &mut *state };
    if base_activity_aborted() {
        state.chosen = std::ptr::null_mut();
        return 0;
    }

    // SAFETY: plain call.
    unsafe { base::SetMenuSounds(MENU_SOUND_NONE, MENU_SOUND_NONE) };

    if !state.initialized {
        state.initialized = true;
        // The flagship selection below is made in this same call, so a Select
        // injected after this observation confirms a real choice.
        crate::automation::ui_observation::observe_ship_picker(true);
        // SAFETY: `state` is the running picker.
        unsafe { change_selection(state) };
    } else {
        let (keys, select) = pulsed_directions();
        if select {
            if !state.chosen.is_null() {
                // SAFETY: plain call.
                unsafe { PlayMenuSound(MENU_SOUND_SUCCESS) };
                return 0;
            }
        } else if let Some(cursor) = moved(state.cursor, keys) {
            // SAFETY: plain call.
            unsafe { PlayMenuSound(MENU_SOUND_MOVE) };
            state.cursor = cursor;
            // SAFETY: `state` is the running picker.
            unsafe { change_selection(state) };
        }
    }

    // SAFETY: plain call.
    unsafe { crate::battle::c_bridge::SleepThread((ONE_SECOND / 30) as c_int) };
    1
}

fn base_activity_aborted() -> bool {
    crate::mainloop::ffi::get_current_activity().0 & CHECK_ABORT != 0
}

/// `GetArmadaStarShip`: let the player pick one of the fleet's ships.
///
/// # Safety
/// Game thread, inside a full-game battle.
unsafe fn get_armada_star_ship() -> *mut c_void {
    // SAFETY: `battle_counter` has a slot for each side.
    if unsafe { *live_queues::battle_counter_slot(1) } == 0 {
        // No opponents left.
        return std::ptr::null_mut();
    }

    // SAFETY: the context global is set at startup.
    let old_context = unsafe { base::SetContext(std::ptr::addr_of!(SpaceContext).read()) };
    let mut pick = CRect::default();
    // SAFETY: `pick` is valid for the call.
    unsafe { draw_armada_pick_ship(0, &mut pick) };

    let mut state = PickerState {
        input_func: do_pick_battle_ship,
        initialized: false,
        cursor: Cursor::START,
        chosen: std::ptr::null_mut(),
        frame: (pick.corner.x, pick.corner.y),
        flash: CRect::default(),
    };
    // SAFETY: `state` starts with the `InputFunc` pointer `DoInput` calls and
    // outlives the loop.
    unsafe {
        base::SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        base::DoInput(std::ptr::addr_of_mut!(state).cast(), 0);
    }
    crate::automation::ui_observation::observe_ship_picker(false);
    // SAFETY: plain call.
    unsafe { SetFlashRect(std::ptr::null()) };

    let chosen = state.chosen;
    if !chosen.is_null() {
        if chosen == tail_of(player_queue()) {
            // Player chose SIS. There will be no more choices.
            // SAFETY: `battle_counter` has a slot for each side.
            unsafe { *live_queues::battle_counter_slot(RPG_PLAYER_NUM) = 1 };
        }
        // SAFETY: plain call.
        unsafe { WaitForSoundEnd(0) };
    }

    // SAFETY: restores the context saved above.
    unsafe { base::SetContext(old_context) };
    chosen
}

/// `GetEncounterStarShip`: the next ship to use for `which_player`.
///
/// # Safety
/// `last` is null or the player's previous `STARSHIP`; game thread only.
#[export_name = "GetEncounterStarShip"]
pub unsafe extern "C" fn get_encounter_star_ship(
    last: *mut STARSHIP,
    which_player: u16,
) -> *mut c_void {
    let side = usize::from(which_player);
    // SAFETY: `battle_counter` and `race_q` have a slot for each side.
    let counter = unsafe { live_queues::battle_counter_slot(side) };
    // SAFETY: as above.
    let queue = unsafe { live_queues::player_ships(side) };

    // SAFETY: plain call.
    if unsafe { inHQSpace() } != 0 {
        assert_eq!(side, RPG_PLAYER_NUM);
        // SIS for the Hyperspace flight
        // SAFETY: `queue` is `race_q[side]`.
        return unsafe { (*queue).head };
    }
    if crate::mainloop::ffi::get_current_activity().0 & 0xFF == SUPER_MELEE {
        // Let the player chose their own ship. (May be a computer player).
        // SAFETY: `battle_counter` has a slot for each side.
        let (own, other) = unsafe {
            (
                *live_queues::battle_counter_slot(0),
                *live_queues::battle_counter_slot(1),
            )
        };
        if own == 0 || other == 0 {
            // One side is out of ships. Game over.
            return std::ptr::null_mut();
        }
        let mut ship = std::ptr::null_mut();
        // SAFETY: `ship` is a valid out-parameter.
        if unsafe { GetNextMeleeStarShip(which_player, &mut ship) } == 0 {
            return std::ptr::null_mut();
        }
        return ship;
    }

    // Full game.
    if side == RPG_PLAYER_NUM {
        // Human player in a full game.
        // SAFETY: `counter` points into `battle_counter`.
        let ships_left = unsafe { *counter };
        if last.is_null() && ships_left == 1 {
            // First time picking a ship and player has no escorts: SIS is the
            // last ship in the queue (though there is only one).
            return tail_of(queue);
        }
        if ships_left != 0 {
            // Player still has ships left
            // SAFETY: game thread inside a full-game battle.
            return unsafe { get_armada_star_ship() };
        }
        if !last.is_null() {
            // The last ship was the flagship.
            // SAFETY: `last` is a live `STARSHIP` and `GlobData` is owned by
            // the game thread.
            unsafe {
                let sis = &mut (*std::ptr::addr_of_mut!(GlobData)).sis_state;
                if (*last).crew_level == 0 {
                    // Died in the line of duty
                    sis.crew_enlisted = u16::MAX;
                } else if sis.fuel_on_board > RUN_AWAY_FUEL_COST {
                    // Player ran away
                    sis.fuel_on_board -= RUN_AWAY_FUEL_COST;
                } else {
                    sis.fuel_on_board = 0;
                }
            }
        }
        return std::ptr::null_mut();
    }

    // NPC player in a full game
    // SAFETY: plain call.
    if unsafe { FleetIsInfinite(which_player) } != 0 {
        if !last.is_null() {
            // The current STARSHIP is reused for the next one; update with new
            // info.
            // SAFETY: `last` is a live `STARSHIP`.
            unsafe {
                (*last).crew_level = (*last).max_crew;
                (*last).playerNr = which_player as _;
                (*last).captains_name_index = production_captain_name();
            }
        }
        // SAFETY: `counter` points into `battle_counter`.
        unsafe { *counter += 1 };
        // SAFETY: `queue` is `race_q[side]`.
        return unsafe { (*queue).head };
    }
    if !last.is_null() {
        // Get the next ship for the computer
        // SAFETY: `last` is a live `STARSHIP`.
        return unsafe { (*last).succ };
    }
    // Get the very first ship for the computer
    // SAFETY: `queue` is `race_q[side]`.
    unsafe { (*queue).head }
}

/// `DrawArmadaPickShip`: draw the picker frame, the SIS name and the escorts'
/// icons; `pick_rect` receives the frame (and, with the salvage frame, the
/// rectangle of that).
///
/// # Safety
/// `pick_rect` is valid for writing; game thread only.
#[export_name = "DrawArmadaPickShip"]
pub unsafe extern "C" fn draw_armada_pick_ship(draw_salvage_frame: c_int, pick_rect: *mut CRect) {
    // SAFETY: the context global is set at startup.
    let old_context = unsafe { base::SetContext(std::ptr::addr_of!(SpaceContext).read()) };
    // SAFETY: `PICK_GRAPHIC` is NUL terminated.
    let pick_frame =
        unsafe { base::CaptureDrawable(base::LoadGraphicInstance(PICK_GRAPHIC.as_ptr().cast())) };

    // SAFETY: balanced by `UnbatchGraphics` below.
    unsafe { base::BatchGraphics() };

    let mut stamp = CStamp {
        origin: CPoint::default(),
        frame: pick_frame,
    };
    let no_hot_spot = CPoint { x: 0, y: 0 };
    let mut pick = CRect::default();
    let mut clip = CRect::default();
    // SAFETY: the rectangles are valid for the calls and the frame is loaded.
    unsafe {
        SetFrameHot(stamp.frame, no_hot_spot);
        crate::battle::c_bridge::GetFrameRect(stamp.frame, std::ptr::addr_of_mut!(pick).cast());
        crate::c_bindings::global_state::GetContextClipRect(std::ptr::addr_of_mut!(clip).cast());
    }
    pick.corner.x = (clip.width >> 1) - (pick.width >> 1);
    pick.corner.y = (clip.height >> 1) - (pick.height >> 1);

    if draw_salvage_frame == 0 {
        // SAFETY: the caller's contract.
        unsafe { *pick_rect = pick };
    } else {
        stamp.origin.x = clip.width >> 1;
        let mut salvage = CRect::default();
        // SAFETY: the frames are loaded and `salvage` is valid.
        unsafe {
            stamp.frame = IncFrameIndex(stamp.frame);
            SetFrameHot(stamp.frame, no_hot_spot);
            crate::battle::c_bridge::GetFrameRect(
                stamp.frame,
                std::ptr::addr_of_mut!(salvage).cast(),
            );
        }
        stamp.origin.x -= salvage.width >> 1;
        stamp.origin.y = pick.corner.y - (salvage.height >> 1);
        // SAFETY: as above.
        unsafe {
            DrawStamp(&mut stamp);
            stamp.frame = DecFrameIndex(stamp.frame);
        }
        pick.corner.y = stamp.origin.y + salvage.height;

        salvage.corner.x = pick.corner.x;
        salvage.corner.y = stamp.origin.y;
        // SAFETY: the caller's contract.
        unsafe { *pick_rect = salvage };
    }
    stamp.origin = pick.corner;
    // SAFETY: the stamp's frame is loaded.
    unsafe { DrawStamp(&mut stamp) };

    // SAFETY: the game thread owns `GlobData`; the name is a C string.
    let ship_name = unsafe { (*std::ptr::addr_of!(GlobData)).sis_state.ship_name.as_ptr() };
    set_color(Rgba::rgb15(0x12, 0x12, 0x12));
    // SAFETY: font globals are set at startup.
    unsafe { SetContextFont(std::ptr::addr_of!(StarConFont).read()) };
    draw_text(
        pick.corner.x + (pick.width >> 1),
        pick.corner.y + pick.height - 5,
        ship_name.cast(),
        u16::MAX,
    );

    for (link, _) in all_links() {
        // SAFETY: every link of `race_q` is a `STARSHIP`.
        let ship = unsafe { &*link.cast::<STARSHIP>() };
        if ship.captains_name_index == 0 {
            // SIS, not an escort
            continue;
        }
        let (x, y) = icon_origin((pick.corner.x, pick.corner.y), ship.index);
        stamp.origin = CPoint { x, y };
        stamp.frame = ship.icons as usize as *mut c_void;
        set_color(Rgba::BLACK);
        fill(Area {
            x,
            y,
            width: ICON_WIDTH,
            height: ICON_HEIGHT,
        });
        let species = ship.SpeciesID;
        if species != NO_ID || ship.crew_level == 0 {
            // SAFETY: the icon frame belongs to the ship.
            unsafe { DrawStamp(&mut stamp) };
            if species == NO_ID {
                // Dead ship - mark with an X.
                stamp.origin.x -= 1;
                // SAFETY: `StatusFrame` is loaded at startup.
                unsafe {
                    stamp.frame = SetAbsFrameIndex(std::ptr::addr_of!(StatusFrame).read(), 3);
                    DrawStamp(&mut stamp);
                }
            }
        } else {
            // Ship ran away
            set_color(Rgba::rgb15(0x00, 0x00, 0x14));
            // SAFETY: the icon frame belongs to the ship.
            unsafe { DrawFilledStamp(&mut stamp) };
        }
    }

    // SAFETY: balances `BatchGraphics`; the frame is released once.
    unsafe {
        base::UnbatchGraphics();
        base::DestroyDrawable(base::ReleaseDrawable(pick_frame));
        base::SetContext(old_context);
    }
}

/// Every `STARSHIP` of the player's queue, flagship included.
fn all_links() -> Vec<(*mut c_void, ())> {
    let queue = player_queue();
    // SAFETY: as in `tail_of`.
    let mut link = unsafe { (*queue).head };
    let mut found = Vec::new();
    while !link.is_null() {
        found.push((link, ()));
        // SAFETY: every link of `race_q` is a `STARSHIP`.
        link = unsafe { (*link.cast::<STARSHIP>()).succ };
    }
    found
}
