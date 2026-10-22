//! The game's implementation of [`Gfx`] and [`Platform`].
//!
//! Drawing, resource, music, input and SIS-frame calls go to the existing
//! graphics, resource and input exports that the rest of the Rust game
//! already calls. The decisions (what a key press means, what a fuel
//! purchase costs, which bars to draw) are made by the Rust models in this
//! module's siblings, not here.

use std::ffi::{c_char, c_int, c_void, CStr, CString};

use super::fuel::GaugeDelta;
use super::gfx::{
    Align, ContextHandle, ContextId, FlashTarget, FontEffectHandle, FontHandle, FontId, FrameId,
    Gfx, Point, Rect, Rgba, Stamp, TextDraw,
};
use super::input::MenuPulses;
use super::platform::{
    AlliedState, ColorMapId, Escort, MenuSounds, MusicId, Platform, RaceTemplate, ResUnitsUpdate,
    SisView, StarbaseError, StateKey, StatusMessageMode, UiStyle,
};
use crate::c_bindings::controller_input::{
    PulsedInputState, KEY_MENU_CANCEL, KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_PAGE_DOWN,
    KEY_MENU_PAGE_UP, KEY_MENU_RIGHT, KEY_MENU_SELECT, KEY_MENU_SPECIAL, KEY_MENU_UP,
};
use crate::collections::queue::{FreeLink, InsertQueue, Queue, RemoveQueue};
use crate::comm::dispatch::{CFleetInfo, CShipFragment};
use crate::comm::locdata::{CColor, CGlobData, CPoint, CRect, CStamp};
use crate::comm::sis_graphics::CText;
use crate::mainloop::restart_menu::c_extern as base;
use crate::mainloop::types::ActivityValue;

/// `OPT_PC` (`options.h`).
const OPT_PC: c_int = 0x02;
/// `MENU_SOUND_FAILURE` (`sounds.h`).
const MENU_SOUND_FAILURE: c_int = 2;
/// `SFR_MENU_3DO` is `(RECT*)~0L`.
const SFR_MENU_3DO: usize = usize::MAX;
/// `SFR_MENU_ANY` is `(RECT*)~1L`.
const SFR_MENU_ANY: usize = usize::MAX - 1;
/// `UNDEFINED_DELTA` (`sis.h`).
const UNDEFINED_DELTA: i16 = 0x7FFF;

extern "C" {
    static mut ScreenWidth: c_int;
    static mut ScreenHeight: c_int;
    static mut ScreenContext: *mut c_void;
    static mut StatusContext: *mut c_void;
    static mut SpaceContext: *mut c_void;
    static mut StarConFont: *mut c_void;
    static mut TinyFont: *mut c_void;
    static mut MicroFont: *mut c_void;
    static mut FontGradFrame: *mut c_void;
    static mut PlayFrame: *mut c_void;
    static mut GameStrings: *mut c_void;
    static mut optWhichMenu: c_int;
    static mut optWhichFonts: c_int;
    static mut GlobData: CGlobData;

    fn SetContextForeGroundColor(color: CColor) -> CColor;
    fn SetContextFont(font: *mut c_void) -> *mut c_void;
    fn SetContextFontEffect(frame: *mut c_void) -> *mut c_void;
    fn GetContextFontLeading(leading: *mut i16);
    fn font_DrawText(text: *mut CText);
    fn DrawFilledRectangle(rect: *mut CRect);
    fn DrawRectangle(rect: *mut CRect);
    fn DrawPoint(point: *mut CPoint);
    fn DrawStamp(stamp: *mut CStamp);
    fn DrawFilledStamp(stamp: *mut CStamp);
    fn SetRelFrameIndex(frame: *mut c_void, offset: i16) -> *mut c_void;
    fn IncFrameIndex(frame: *mut c_void) -> *mut c_void;
    fn DecFrameIndex(frame: *mut c_void) -> *mut c_void;
    fn GetFrameCount(frame: *mut c_void) -> u16;
    fn GetFrameIndex(frame: *mut c_void) -> u16;
    fn SetFlashRect(rect: *const c_void);
    fn PreUpdateFlashRect();
    fn PostUpdateFlashRect();
    fn DeltaSISGauges(crew_delta: i16, fuel_delta: i16, resunit_delta: i32);
    fn DrawSISFrame();
    fn DrawSISMessage(message: *const c_char);
    fn DrawSISTitle(title: *const c_char);
    fn FlushInput();
    fn PlayMenuSound(effect: c_int);
    fn SetAbsStringTableIndex(table: *mut c_void, index: u16) -> *mut c_void;
    fn GetStringAddress(entry: *mut c_void) -> *const c_char;
    fn SetContextClipRect(rect: *mut CRect);
    fn AnyButtonPress(detect_special: u8) -> u8;
    fn LoadStringTableInstance(resource: *const c_char) -> *mut c_void;
    fn CaptureStringTable(table: *mut c_void) -> *mut c_void;
    fn ReleaseStringTable(entry: *mut c_void) -> *mut c_void;
    fn DestroyStringTable(table: *mut c_void) -> c_int;
    fn SetRelStringTableIndex(entry: *mut c_void, offset: i16) -> *mut c_void;
    fn GetColorMapAddress(map: *mut c_void) -> *mut c_void;
    fn SetColorMap(map: *mut c_void) -> c_int;
}

/// `&GLOBAL (built_ship_q)`.
fn built_ship_queue() -> *mut Queue {
    // SAFETY: only the address of the game thread's `GlobData` is taken.
    unsafe { std::ptr::addr_of_mut!(GlobData.game_state.built_ship_q) as *mut Queue }
}

/// `&GLOBAL (avail_race_q)`.
fn avail_race_queue() -> *mut Queue {
    // SAFETY: only the address of the game thread's `GlobData` is taken.
    unsafe { std::ptr::addr_of_mut!(GlobData.game_state.avail_race_q) as *mut Queue }
}

/// The links of a queue of `T` in queue order.
///
/// # Safety
/// `queue` is one of the game's queues, whose links are all `T`.
unsafe fn queue_links(queue: *mut Queue) -> Vec<*mut c_void> {
    let mut links = Vec::new();
    // SAFETY: the caller guarantees the queue is well formed.
    let mut link = unsafe { (*queue).head };
    while !link.is_null() {
        links.push(link);
        // SAFETY: every link starts with the `pred` and `succ` pointers.
        link = unsafe { (*link.cast::<CShipFragment>()).succ };
    }
    links
}

/// The escort link in a hangar slot.
fn escort_link(slot: u8) -> Result<*mut c_void, StarbaseError> {
    // SAFETY: `built_ship_q` holds `SHIP_FRAGMENT` links.
    unsafe { queue_links(built_ship_queue()) }
        .into_iter()
        // SAFETY: as above.
        .find(|link| unsafe { &*link.cast::<CShipFragment>() }.index == slot)
        .ok_or(StarbaseError::EscortSlot(slot))
}

fn to_c_color(color: Rgba) -> CColor {
    CColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn from_c_color(color: CColor) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

fn to_c_point(point: Point) -> CPoint {
    CPoint {
        x: point.x,
        y: point.y,
    }
}

fn to_c_rect(rect: Rect) -> CRect {
    CRect {
        corner: to_c_point(rect.corner),
        width: rect.width,
        height: rect.height,
    }
}

fn to_c_stamp(stamp: Stamp) -> CStamp {
    CStamp {
        origin: to_c_point(stamp.origin),
        frame: stamp.frame as *mut c_void,
    }
}

fn c_string(text: &str) -> CString {
    CString::new(text).expect("game text has no interior NUL byte")
}

/// The implementation backed by the running game.
pub struct CffiPlatform;

impl CffiPlatform {
    fn context_ptr(ctx: ContextId) -> *mut c_void {
        // SAFETY: the context globals are set once at startup, before any
        // screen runs, and only read afterwards.
        unsafe {
            match ctx {
                ContextId::Screen => std::ptr::addr_of!(ScreenContext).read(),
                ContextId::Status => std::ptr::addr_of!(StatusContext).read(),
                ContextId::Space => std::ptr::addr_of!(SpaceContext).read(),
            }
        }
    }

    fn font_ptr(font: FontId) -> *mut c_void {
        // SAFETY: the font globals are set once at startup.
        unsafe {
            match font {
                FontId::StarCon => std::ptr::addr_of!(StarConFont).read(),
                FontId::Tiny => std::ptr::addr_of!(TinyFont).read(),
                FontId::Micro => std::ptr::addr_of!(MicroFont).read(),
            }
        }
    }

    fn load_resource(
        resource: &str,
        load: unsafe extern "C" fn(*const c_char) -> *mut c_void,
    ) -> Result<*mut c_void, StarbaseError> {
        let name = c_string(resource);
        // SAFETY: `name` is a valid NUL-terminated string for the call.
        let loaded = unsafe { load(name.as_ptr()) };
        if loaded.is_null() {
            return Err(StarbaseError::ResourceLoad(resource.to_owned()));
        }
        Ok(loaded)
    }
}

impl Gfx for CffiPlatform {
    fn screen_size(&self) -> (i32, i32) {
        // SAFETY: set at startup, read-only afterwards.
        unsafe {
            (
                std::ptr::addr_of!(ScreenWidth).read(),
                std::ptr::addr_of!(ScreenHeight).read(),
            )
        }
    }

    fn set_context(&self, ctx: ContextId) -> ContextHandle {
        // SAFETY: the pointer is one of the game's context objects.
        ContextHandle(unsafe { base::SetContext(Self::context_ptr(ctx)) } as usize)
    }

    fn restore_context(&self, previous: ContextHandle) {
        // SAFETY: `previous` was returned by `SetContext`.
        unsafe { base::SetContext(previous.0 as *mut c_void) };
    }

    fn context_clip_rect(&self) -> Rect {
        use crate::c_bindings::global_state as c;

        let mut rect = c::RECT {
            corner: c::POINT { x: 0, y: 0 },
            extent: c::EXTENT {
                width: 0,
                height: 0,
            },
        };
        // SAFETY: `rect` is a valid out-parameter.
        unsafe { c::GetContextClipRect(&mut rect) };
        Rect {
            corner: Point {
                x: rect.corner.x,
                y: rect.corner.y,
            },
            width: rect.extent.width,
            height: rect.extent.height,
        }
    }

    fn set_fg_color(&self, color: Rgba) -> Rgba {
        // SAFETY: plain value call into the graphics context.
        from_c_color(unsafe { SetContextForeGroundColor(to_c_color(color)) })
    }

    fn set_bg_color(&self, color: Rgba) -> Rgba {
        let color = base::Color {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        };
        // SAFETY: plain value call into the graphics context.
        let old = unsafe { base::SetContextBackGroundColor(color) };
        Rgba {
            r: old.r,
            g: old.g,
            b: old.b,
            a: old.a,
        }
    }

    fn set_font(&self, font: FontId) -> FontHandle {
        // SAFETY: the pointer is one of the game's font objects.
        FontHandle(unsafe { SetContextFont(Self::font_ptr(font)) } as usize)
    }

    fn restore_font(&self, previous: FontHandle) {
        // SAFETY: `previous` was returned by `SetContextFont`.
        unsafe { SetContextFont(previous.0 as *mut c_void) };
    }

    fn set_font_effect(&self, grad_index: Option<u16>) -> FontEffectHandle {
        // SAFETY: `FontGradFrame` is set at startup; the effect frame is
        // owned by the game.
        let effect = match grad_index {
            Some(index) => unsafe {
                base::SetAbsFrameIndex(std::ptr::addr_of!(FontGradFrame).read(), index)
            },
            None => std::ptr::null_mut(),
        };
        // SAFETY: plain call into the graphics context.
        FontEffectHandle(unsafe { SetContextFontEffect(effect) } as usize)
    }

    fn restore_font_effect(&self, previous: FontEffectHandle) {
        // SAFETY: `previous` was returned by `SetContextFontEffect`.
        unsafe { SetContextFontEffect(previous.0 as *mut c_void) };
    }

    fn font_leading(&self) -> i16 {
        let mut leading = 0;
        // SAFETY: `leading` is a valid out-parameter.
        unsafe { GetContextFontLeading(&mut leading) };
        leading
    }

    fn set_context_clip_rect(&self, rect: Rect) {
        let mut rect = to_c_rect(rect);
        // SAFETY: `rect` is valid for the call and copied by it.
        unsafe { SetContextClipRect(&mut rect) };
    }

    fn fill_rect(&self, rect: Rect) {
        let mut rect = to_c_rect(rect);
        // SAFETY: `rect` is valid for the call.
        unsafe { DrawFilledRectangle(&mut rect) };
    }

    fn draw_rect(&self, rect: Rect) {
        let mut rect = to_c_rect(rect);
        // SAFETY: `rect` is valid for the call.
        unsafe { DrawRectangle(&mut rect) };
    }

    fn draw_point(&self, point: Point) {
        let mut point = to_c_point(point);
        // SAFETY: `point` is valid for the call.
        unsafe { DrawPoint(&mut point) };
    }

    fn draw_stamp(&self, stamp: Stamp) {
        let mut stamp = to_c_stamp(stamp);
        // SAFETY: `stamp` is valid for the call.
        unsafe { DrawStamp(&mut stamp) };
    }

    fn draw_filled_stamp(&self, stamp: Stamp) {
        let mut stamp = to_c_stamp(stamp);
        // SAFETY: `stamp` is valid for the call.
        unsafe { DrawFilledStamp(&mut stamp) };
    }

    fn draw_text(&self, text: &TextDraw) {
        let string = c_string(&text.text);
        let align = match text.align {
            Align::Left => 0,
            Align::Center => 1,
            Align::Right => 2,
        };
        let mut c_text = CText::default();
        c_text.baseline = to_c_point(text.baseline);
        c_text.p_str = string.as_ptr();
        c_text.align = align;
        c_text.char_count = u16::MAX;
        // SAFETY: `c_text` and `string` are valid for the call.
        unsafe { font_DrawText(&mut c_text) };
    }

    fn abs_frame(&self, frame: FrameId, index: u16) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { base::SetAbsFrameIndex(frame as *mut c_void, index) as FrameId }
    }

    fn rel_frame(&self, frame: FrameId, offset: i16) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { SetRelFrameIndex(frame as *mut c_void, offset) as FrameId }
    }

    fn inc_frame(&self, frame: FrameId) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { IncFrameIndex(frame as *mut c_void) as FrameId }
    }

    fn dec_frame(&self, frame: FrameId) -> FrameId {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { DecFrameIndex(frame as *mut c_void) as FrameId }
    }

    fn frame_count(&self, frame: FrameId) -> u16 {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { GetFrameCount(frame as *mut c_void) }
    }

    fn frame_index(&self, frame: FrameId) -> u16 {
        // SAFETY: `frame` is a frame handle owned by the caller.
        unsafe { GetFrameIndex(frame as *mut c_void) }
    }

    fn play_frame(&self) -> FrameId {
        // SAFETY: `PlayFrame` is set at startup.
        unsafe { std::ptr::addr_of!(PlayFrame).read() as FrameId }
    }

    fn batch(&self) {
        // SAFETY: balanced with `unbatch` by the callers.
        unsafe { base::BatchGraphics() };
    }

    fn unbatch(&self) {
        // SAFETY: balanced with `batch` by the callers.
        unsafe { base::UnbatchGraphics() };
    }

    fn clear_drawable(&self) {
        // SAFETY: plain call into the current context.
        unsafe { base::ClearDrawable() };
    }

    fn set_transition_source_none(&self) {
        // SAFETY: a null source selects the whole screen.
        unsafe { base::SetTransitionSource(std::ptr::null_mut()) };
    }

    fn screen_transition(&self, kind: i32) {
        // SAFETY: a null rectangle selects the whole screen.
        unsafe { base::ScreenTransition(kind, std::ptr::null_mut()) };
    }

    fn set_flash(&self, target: FlashTarget) {
        match target {
            FlashTarget::Off => {
                // SAFETY: NULL stops the flash.
                unsafe { SetFlashRect(std::ptr::null()) };
            }
            FlashTarget::Menu3do => {
                // SAFETY: `SFR_MENU_3DO` is the C sentinel pointer `~0`.
                unsafe { SetFlashRect(SFR_MENU_3DO as *const c_void) };
            }
            FlashTarget::MenuAny => {
                // SAFETY: `SFR_MENU_ANY` is the C sentinel pointer `~1`.
                unsafe { SetFlashRect(SFR_MENU_ANY as *const c_void) };
            }
            FlashTarget::Rect(rect) => {
                let rect = to_c_rect(rect);
                // SAFETY: the rectangle is copied by the call.
                unsafe { SetFlashRect(std::ptr::from_ref(&rect).cast()) };
            }
        }
    }

    fn pre_update_flash(&self) {
        // SAFETY: balanced with `post_update_flash` by the callers.
        unsafe { PreUpdateFlashRect() };
    }

    fn post_update_flash(&self) {
        // SAFETY: balanced with `pre_update_flash` by the callers.
        unsafe { PostUpdateFlashRect() };
    }

    fn game_string(&self, index: i32) -> String {
        // SAFETY: `GameStrings` is loaded before any screen runs; the
        // returned string lives as long as the table.
        unsafe {
            let index = u16::try_from(index).expect("game string indexes fit a COUNT");
            let entry = SetAbsStringTableIndex(std::ptr::addr_of!(GameStrings).read(), index);
            let text = GetStringAddress(entry);
            if text.is_null() {
                return String::new();
            }
            CStr::from_ptr(text).to_string_lossy().into_owned()
        }
    }
}

fn state_key_name(key: StateKey) -> &'static str {
    match key {
        StateKey::StarbaseVisited => "STARBASE_VISITED",
        StateKey::StarbaseAvailable => "STARBASE_AVAILABLE",
        StateKey::ProbeIlwrathEncounter => "PROBE_ILWRATH_ENCOUNTER",
        StateKey::GlobalFlagsAndData => "GLOBAL_FLAGS_AND_DATA",
        StateKey::MoonbaseOnShip => "MOONBASE_ON_SHIP",
        StateKey::ChmmrBombState => "CHMMR_BOMB_STATE",
        StateKey::LanderShields => "LANDER_SHIELDS",
        StateKey::CrewPurchased0 => "CREW_PURCHASED0",
        StateKey::CrewPurchased1 => "CREW_PURCHASED1",
    }
}

fn message_mode_to_c(mode: StatusMessageMode) -> c_int {
    match mode {
        StatusMessageMode::Undefined => 0,
        StatusMessageMode::Date => 1,
        StatusMessageMode::ResUnits => 2,
        StatusMessageMode::Credits => 3,
    }
}

fn message_mode_from_c(mode: c_int) -> StatusMessageMode {
    match mode {
        1 => StatusMessageMode::Date,
        2 => StatusMessageMode::ResUnits,
        3 => StatusMessageMode::Credits,
        _ => StatusMessageMode::Undefined,
    }
}

/// `DoInput` state: the first field is the `InputFunc`.
#[repr(C)]
struct LoopState {
    input_func: unsafe extern "C" fn(*mut LoopState) -> c_int,
    frame: *mut c_void,
}

unsafe extern "C" fn loop_frame(state: *mut LoopState) -> c_int {
    // SAFETY: `state` is the `LoopState` that `run_input_loop` passed to
    // `DoInput`, and `frame` points at its live closure reference.
    let frame = unsafe { &mut *(*state).frame.cast::<&mut dyn FnMut() -> bool>() };
    c_int::from(frame())
}

impl Platform for CffiPlatform {
    fn current_activity(&self) -> ActivityValue {
        crate::mainloop::ffi::get_current_activity()
    }

    fn set_current_activity(&self, value: ActivityValue) {
        crate::mainloop::ffi::set_current_activity(value);
    }

    fn last_activity(&self) -> ActivityValue {
        crate::mainloop::ffi::get_last_activity()
    }

    fn set_last_activity(&self, value: ActivityValue) {
        crate::mainloop::ffi::set_last_activity(value);
    }

    fn set_next_activity(&self, value: ActivityValue) {
        crate::mainloop::ffi::set_next_activity(value);
    }

    fn game_state(&self, key: StateKey) -> u32 {
        crate::state::game_state_keys::get_game_state(state_key_name(key))
    }

    fn set_game_state(&self, key: StateKey, value: u32) {
        crate::state::game_state_keys::set_game_state(state_key_name(key), value);
    }

    fn set_status_message_mode(&self, mode: StatusMessageMode) -> StatusMessageMode {
        // SAFETY: plain value call.
        let previous =
            unsafe { crate::mainloop::c_extern::SetStatusMessageMode(message_mode_to_c(mode)) };
        message_mode_from_c(previous)
    }

    fn sis(&self) -> Result<SisView, StarbaseError> {
        // SAFETY: the game thread owns `GlobData`; no other thread writes it
        // while a screen runs.
        let glob = unsafe { &*std::ptr::addr_of!(GlobData) };
        let name_bytes = glob
            .sis_state
            .ship_name
            .split(|byte| *byte == 0)
            .next()
            .unwrap_or_default();
        let ship_name = std::str::from_utf8(name_bytes)
            .map_err(|_| StarbaseError::ShipName)?
            .to_owned();
        let mut module_cost = [0u8; 20];
        module_cost.copy_from_slice(&glob.game_state.module_cost);
        Ok(SisView {
            res_units: glob.sis_state.res_units,
            fuel_on_board: glob.sis_state.fuel_on_board,
            crew_enlisted: glob.sis_state.crew_enlisted,
            total_element_mass: glob.sis_state.total_element_mass,
            module_slots: glob.sis_state.module_slots,
            drive_slots: glob.sis_state.drive_slots,
            jet_slots: glob.sis_state.jet_slots,
            num_landers: glob.sis_state.num_landers,
            ship_name,
            fuel_cost: glob.game_state.fuel_cost,
            module_cost,
        })
    }

    fn set_fuel_on_board(&self, fuel: u32) {
        // SAFETY: the game thread owns `GlobData`.
        unsafe { (*std::ptr::addr_of_mut!(GlobData)).sis_state.fuel_on_board = fuel };
    }

    fn apply_gauge_delta(&self, delta: GaugeDelta) {
        // SAFETY: plain value call; the status gauges redraw themselves.
        unsafe { DeltaSISGauges(0, delta.fuel, delta.res_units) };
    }

    fn apply_crew_gauge_delta(&self, crew: i16, res_units: i32) {
        // SAFETY: plain value call; the status gauges redraw themselves.
        unsafe { DeltaSISGauges(crew, 0, res_units) };
    }

    fn crew_cost(&self) -> u8 {
        // SAFETY: the game thread owns `GlobData`.
        unsafe { (*std::ptr::addr_of!(GlobData)).game_state.crew_cost }
    }

    fn set_crew_cost(&self, cost: u8) {
        // SAFETY: the game thread owns `GlobData`.
        unsafe { (*std::ptr::addr_of_mut!(GlobData)).game_state.crew_cost = cost };
    }

    fn race_templates(&self) -> Result<Vec<RaceTemplate>, StarbaseError> {
        // SAFETY: `avail_race_q` holds `FLEET_INFO` links.
        unsafe { queue_links(avail_race_queue()) }
            .into_iter()
            .map(|link| {
                // SAFETY: as above.
                let info = unsafe { &*link.cast::<CFleetInfo>() };
                let allied = match info.allied_state {
                    0 => AlliedState::DeadGuy,
                    1 => AlliedState::GoodGuy,
                    2 => AlliedState::BadGuy,
                    other => return Err(StarbaseError::UnknownAlliedState(other)),
                };
                Ok(RaceTemplate {
                    allied,
                    melee_icon: info.melee_icon as FrameId,
                    crew_level: info.crew_level,
                })
            })
            .collect()
    }

    fn escorts(&self) -> Result<Vec<Escort>, StarbaseError> {
        // SAFETY: `built_ship_q` holds `SHIP_FRAGMENT` links.
        Ok(unsafe { queue_links(built_ship_queue()) }
            .into_iter()
            .map(|link| {
                // SAFETY: as above.
                let fragment = unsafe { &*link.cast::<CShipFragment>() };
                Escort {
                    slot: fragment.index,
                    race_id: fragment.race_id,
                    crew_level: fragment.crew_level,
                    max_crew: fragment.max_crew,
                    melee_icon: fragment.melee_icon as FrameId,
                }
            })
            .collect())
    }

    fn add_escort(&self, race: u8, slot: u8) -> Result<bool, StarbaseError> {
        if escort_link(slot).is_ok() {
            return Err(StarbaseError::EscortQueue("the hangar slot is occupied"));
        }
        let queue = built_ship_queue();
        // SAFETY: `queue` is the game's `built_ship_q`; the clone is its tail.
        let link = unsafe { crate::comm::dispatch::clone_ship_fragment(race, queue, 1) };
        if link.is_null() {
            return Ok(false);
        }
        // `ShowCombatShip`: keep the queue ordered by slot.
        // SAFETY: the queue and its links are well formed.
        unsafe {
            RemoveQueue(queue, link);
            let mut after = (*queue).head;
            while !after.is_null() {
                let fragment = &*after.cast::<CShipFragment>();
                if fragment.index > slot {
                    break;
                }
                after = fragment.succ;
            }
            InsertQueue(queue, link, after);
            (*link.cast::<CShipFragment>()).index = slot;
        }
        Ok(true)
    }

    fn set_escort_crew(&self, slot: u8, crew_level: u16) -> Result<(), StarbaseError> {
        let link = escort_link(slot)?;
        // SAFETY: `link` is a `SHIP_FRAGMENT` of `built_ship_q`.
        unsafe { (*link.cast::<CShipFragment>()).crew_level = crew_level };
        Ok(())
    }

    fn remove_escort(&self, slot: u8) -> Result<(), StarbaseError> {
        let link = escort_link(slot)?;
        let queue = built_ship_queue();
        RemoveQueue(queue, link);
        FreeLink(queue, link);
        Ok(())
    }

    fn set_drive_slot(&self, index: usize, piece: u8) {
        // SAFETY: the game thread owns `GlobData`; the caller bounds `index`
        // by `NUM_DRIVE_SLOTS`, and the array index checks it again.
        unsafe { (*std::ptr::addr_of_mut!(GlobData)).sis_state.drive_slots[index] = piece };
    }

    fn set_jet_slot(&self, index: usize, piece: u8) {
        // SAFETY: as for `set_drive_slot`.
        unsafe { (*std::ptr::addr_of_mut!(GlobData)).sis_state.jet_slots[index] = piece };
    }

    fn set_module_slot(&self, index: usize, piece: u8) {
        // SAFETY: as for `set_drive_slot`.
        unsafe { (*std::ptr::addr_of_mut!(GlobData)).sis_state.module_slots[index] = piece };
    }

    fn set_num_landers(&self, count: u8) {
        // SAFETY: the game thread owns `GlobData`.
        unsafe { (*std::ptr::addr_of_mut!(GlobData)).sis_state.num_landers = count };
    }

    fn redraw_ship_status(&self, res_units: ResUnitsUpdate) {
        let res_units = match res_units {
            ResUnitsUpdate::Redraw => i32::from(UNDEFINED_DELTA),
            ResUnitsUpdate::Change(delta) => delta,
        };
        // SAFETY: plain value call; the status panel redraws itself.
        unsafe { DeltaSISGauges(UNDEFINED_DELTA, UNDEFINED_DELTA, res_units) };
    }

    fn draw_sis_frame(&self) {
        // SAFETY: plain call.
        unsafe { DrawSISFrame() };
    }

    fn draw_sis_message(&self, text: &str) {
        let text = c_string(text);
        // SAFETY: `text` is valid for the call.
        unsafe { DrawSISMessage(text.as_ptr()) };
    }

    fn draw_sis_title(&self, text: &str) {
        let text = c_string(text);
        // SAFETY: `text` is valid for the call.
        unsafe { DrawSISTitle(text.as_ptr()) };
    }

    fn pulses(&self) -> MenuPulses {
        // SAFETY: the input state is updated by `DoInput` on this thread.
        let menu = unsafe { std::ptr::addr_of!(PulsedInputState).read().menu };
        let pressed = |key: u32| menu[key as usize] != 0;
        MenuPulses {
            up: pressed(KEY_MENU_UP),
            down: pressed(KEY_MENU_DOWN),
            left: pressed(KEY_MENU_LEFT),
            right: pressed(KEY_MENU_RIGHT),
            select: pressed(KEY_MENU_SELECT),
            special: pressed(KEY_MENU_SPECIAL),
            cancel: pressed(KEY_MENU_CANCEL),
            page_up: pressed(KEY_MENU_PAGE_UP),
            page_down: pressed(KEY_MENU_PAGE_DOWN),
        }
    }

    fn set_menu_sounds(&self, sounds: MenuSounds) {
        // SAFETY: plain value call.
        unsafe { base::SetMenuSounds(sounds.move_mask, sounds.select_mask) };
    }

    fn play_failure_sound(&self) {
        // SAFETY: plain value call.
        unsafe { PlayMenuSound(MENU_SOUND_FAILURE) };
    }

    fn flush_input(&self) {
        // SAFETY: plain call.
        unsafe { FlushInput() };
    }

    fn any_button_press(&self) -> bool {
        // SAFETY: plain call.
        unsafe { AnyButtonPress(0) != 0 }
    }

    fn menu_style(&self) -> UiStyle {
        // SAFETY: option set at startup.
        if unsafe { std::ptr::addr_of!(optWhichMenu).read() } == OPT_PC {
            UiStyle::Pc
        } else {
            UiStyle::ThreeDo
        }
    }

    fn font_style(&self) -> UiStyle {
        // SAFETY: option set at startup.
        if unsafe { std::ptr::addr_of!(optWhichFonts).read() } == OPT_PC {
            UiStyle::Pc
        } else {
            UiStyle::ThreeDo
        }
    }

    fn load_graphic(&self, resource: &str) -> Result<FrameId, StarbaseError> {
        let loaded = Self::load_resource(resource, base::LoadGraphicInstance)?;
        // SAFETY: `loaded` is the result of `LoadGraphicInstance`.
        let frame = unsafe { base::CaptureDrawable(loaded) };
        if frame.is_null() {
            return Err(StarbaseError::ResourceLoad(resource.to_owned()));
        }
        Ok(frame as FrameId)
    }

    fn destroy_drawable(&self, frame: FrameId) {
        // SAFETY: `frame` was returned by `load_graphic` and is released once.
        unsafe { base::DestroyDrawable(base::ReleaseDrawable(frame as *mut c_void)) };
    }

    fn load_color_map(&self, resource: &str) -> Result<ColorMapId, StarbaseError> {
        let loaded = Self::load_resource(resource, LoadStringTableInstance)?;
        // SAFETY: `loaded` is the result of `LoadStringTableInstance`.
        let map = unsafe { CaptureStringTable(loaded) };
        if map.is_null() {
            return Err(StarbaseError::ResourceLoad(resource.to_owned()));
        }
        Ok(map as ColorMapId)
    }

    fn destroy_color_map(&self, map: ColorMapId) {
        // SAFETY: `map` was returned by `load_color_map` and is released once.
        unsafe { DestroyStringTable(ReleaseStringTable(map as *mut c_void)) };
    }

    fn abs_color_map(&self, map: ColorMapId, index: u16) -> ColorMapId {
        // SAFETY: `map` is a colour map handle owned by the caller.
        unsafe { SetAbsStringTableIndex(map as *mut c_void, index) as ColorMapId }
    }

    fn rel_color_map(&self, map: ColorMapId, offset: i16) -> ColorMapId {
        // SAFETY: `map` is a colour map handle owned by the caller.
        unsafe { SetRelStringTableIndex(map as *mut c_void, offset) as ColorMapId }
    }

    fn apply_color_map(&self, map: ColorMapId) {
        // SAFETY: `map` is a colour map handle owned by the caller.
        unsafe { SetColorMap(GetColorMapAddress(map as *mut c_void)) };
    }

    fn load_music(&self, resource: &str) -> Result<MusicId, StarbaseError> {
        Ok(Self::load_resource(resource, base::LoadMusicInstance)? as MusicId)
    }

    fn play_music(&self, music: MusicId) {
        // SAFETY: `music` was returned by `load_music`; priority 1 as in C.
        unsafe { base::PlayMusic(music as *mut c_void, 1, 1) };
    }

    fn stop_music(&self) {
        // SAFETY: plain call.
        unsafe { base::StopMusic() };
    }

    fn destroy_music(&self, music: MusicId) {
        // SAFETY: `music` was returned by `load_music` and is destroyed once.
        unsafe { base::DestroyMusic(music as *mut c_void) };
    }

    fn now(&self) -> u32 {
        // SAFETY: plain call.
        unsafe { base::uqm_get_time_counter() }
    }

    fn sleep_thread(&self, ticks: u32) {
        // SAFETY: plain call; pumps and flushes the frame under RUST_OWNS_MAIN.
        unsafe { base::uqm_sleep_thread(ticks) };
    }

    fn sleep_thread_until(&self, wake_time: u32) {
        // SAFETY: plain call.
        unsafe { base::uqm_sleep_thread_until(wake_time) };
    }

    fn fade_to_black(&self, ticks: u32) -> u32 {
        let ticks = i16::try_from(ticks).expect("fade interval fits a SIZE");
        // SAFETY: plain call.
        unsafe { base::FadeScreen(base::FADE_ALL_TO_BLACK, ticks) }
    }

    fn advance_clock_days(&self, days: u32) {
        // SAFETY: plain call into the Rust game clock.
        unsafe {
            crate::time::clock_bridge::rust_clock_advance_days(
                c_int::try_from(days).expect("clock days fit a C int"),
            );
        }
    }

    fn observe_starbase_menu(&self, active: bool) {
        crate::automation::ui_observation::observe_starbase_menu(active);
    }

    fn run_commander_conversation(&self) {
        // SAFETY: the communication dispatch runs on the game thread.
        unsafe {
            crate::comm::dispatch::init_communication(crate::comm::dispatch::conv::COMMANDER);
        }
    }

    fn run_input_loop(&self, frame: &mut dyn FnMut() -> bool) {
        // `DoInput` returns before `frame_ref` goes out of scope.
        let mut frame_ref: &mut dyn FnMut() -> bool = frame;
        let mut state = LoopState {
            input_func: loop_frame,
            frame: std::ptr::addr_of_mut!(frame_ref).cast(),
        };
        // SAFETY: `state` starts with the `InputFunc` that `DoInput` calls.
        unsafe { base::DoInput(std::ptr::addr_of_mut!(state).cast(), 1) };
    }
}
