//! A recording [`Platform`] for the Starbase and Outfit tests.
//!
//! It keeps one ordered list of everything the screens asked for, applies
//! the SIS changes the way the game does, and feeds scripted menu keys to
//! the input loop.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

use super::fuel::{FuelRuState, GaugeDelta};
use super::gfx::{
    ContextHandle, ContextId, FlashTarget, FontEffectHandle, FontHandle, FontId, FrameId, Gfx,
    Point, Rect, Rgba, Stamp, TextDraw,
};
use super::input::MenuPulses;
use super::new_game_reference::NewGameFlagship;
use super::platform::{
    MenuSounds, MusicId, Platform, ResUnitsUpdate, SisView, StarbaseError, StateKey,
    StatusMessageMode, UiStyle,
};
use super::sis_modules::{ModuleSlots, EMPTY_MODULE_SLOT};
use crate::mainloop::types::ActivityValue;

/// Everything a screen asked the platform to do, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    SetContext(ContextId),
    RestoreContext,
    FillRect(Rect),
    DrawRect(Rect),
    DrawPoint(Point),
    DrawStamp(Stamp),
    DrawFilledStamp(Stamp),
    Text(TextDraw),
    SetFg(Rgba),
    SetBg(Rgba),
    SetFont(FontId),
    RestoreFont,
    SetFontEffect(Option<u16>),
    RestoreFontEffect,
    Batch,
    Unbatch,
    ClearDrawable,
    TransitionSourceNone,
    ScreenTransition(i32),
    Flash(FlashTarget),
    PreUpdateFlash,
    PostUpdateFlash,
    SetCurrentActivity(ActivityValue),
    SetLastActivity(ActivityValue),
    SetNextActivity(ActivityValue),
    SetState(StateKey, u32),
    SetMessageMode(StatusMessageMode),
    SetFuelOnBoard(u32),
    GaugeDelta(GaugeDelta),
    SetDriveSlot(usize, u8),
    SetJetSlot(usize, u8),
    SetModuleSlot(usize, u8),
    SetNumLanders(u8),
    ShipStatus(ResUnitsUpdate),
    SisFrame,
    SisMessage(String),
    SisTitle(String),
    MenuSounds(MenuSounds),
    FailureSound,
    FlushInput,
    LoadGraphic(String, FrameId),
    DestroyDrawable(FrameId),
    LoadMusic(String, MusicId),
    PlayMusic(MusicId),
    StopMusic,
    DestroyMusic(MusicId),
    Sleep(u32),
    SleepUntil(u32),
    Fade(u32),
    AdvanceDays(u32),
    Conversation,
    Docked(bool),
}

/// Frames the mock hands out are `1000 * set + index`.
const FRAME_SET: usize = 1000;
const FRAMES_PER_SET: u16 = 64;
const PLAY_FRAME: FrameId = 9000;
const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 240;
/// Most frames a scripted input loop may run before the test fails.
const LOOP_FRAME_LIMIT: usize = 5000;

pub struct RecordingPlatform {
    pub events: RefCell<Vec<Event>>,
    pub sis: RefCell<SisView>,
    pub activity: Cell<ActivityValue>,
    pub last_activity: Cell<ActivityValue>,
    pub states: RefCell<HashMap<StateKey, u32>>,
    pub script: RefCell<VecDeque<MenuPulses>>,
    pub menu_style: Cell<UiStyle>,
    pub font_style: Cell<UiStyle>,
    /// Make the gauge update apply a different fuel than requested.
    pub corrupt_gauge: Cell<bool>,
    /// Make the ship status update apply one RU more than requested.
    pub corrupt_res_units: Cell<bool>,
    /// A resource name that fails to load.
    pub failing_resource: RefCell<Option<String>>,
    /// Abort the game after this many pulses were consumed in total.
    pub abort_after_pulses: Cell<Option<usize>>,
    consumed: Cell<usize>,
    current_pulses: Cell<MenuPulses>,
    clock: Cell<u32>,
    next_frame_set: Cell<usize>,
    next_music: Cell<MusicId>,
    message_mode: Cell<StatusMessageMode>,
}

pub fn default_sis() -> SisView {
    let mut module_slots = [EMPTY_MODULE_SLOT; 16];
    module_slots[0] = 5;
    SisView {
        res_units: 2500,
        fuel_on_board: 1000,
        crew_enlisted: 12,
        total_element_mass: 0,
        module_slots,
        drive_slots: [1, 1, 1, 1, 1, 1, 20, 20, 20, 20, 20],
        jet_slots: [2, 2, 2, 2, 21, 21, 21, 21],
        num_landers: 3,
        ship_name: "Vindicator".to_owned(),
        fuel_cost: 20,
        module_cost: [0; 20],
    }
}

impl RecordingPlatform {
    pub fn new() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            sis: RefCell::new(default_sis()),
            activity: Cell::new(ActivityValue::new(0)),
            last_activity: Cell::new(ActivityValue::new(0)),
            states: RefCell::new(HashMap::new()),
            script: RefCell::new(VecDeque::new()),
            menu_style: Cell::new(UiStyle::ThreeDo),
            font_style: Cell::new(UiStyle::ThreeDo),
            corrupt_gauge: Cell::new(false),
            corrupt_res_units: Cell::new(false),
            failing_resource: RefCell::new(None),
            abort_after_pulses: Cell::new(None),
            consumed: Cell::new(0),
            current_pulses: Cell::new(MenuPulses::default()),
            clock: Cell::new(10_000),
            next_frame_set: Cell::new(1),
            next_music: Cell::new(1),
            message_mode: Cell::new(StatusMessageMode::Date),
        }
    }

    /// A platform whose ship is the one a new game starts with, read from
    /// the C initializer, with `res_units` to spend.
    pub fn with_new_game_flagship(res_units: u32) -> Self {
        let platform = Self::new();
        let mut flagship = NewGameFlagship::from_c_source();
        flagship.res_units = res_units;
        *platform.sis.borrow_mut() = flagship.sis_view();
        platform
    }

    pub fn push_pulses(&self, pulses: impl IntoIterator<Item = MenuPulses>) {
        self.script.borrow_mut().extend(pulses);
    }

    pub fn set_pulses(&self, pulses: MenuPulses) {
        self.current_pulses.set(pulses);
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.borrow().clone()
    }

    pub fn state(&self, key: StateKey) -> u32 {
        self.states.borrow().get(&key).copied().unwrap_or(0)
    }

    fn record(&self, event: Event) {
        self.events.borrow_mut().push(event);
    }

    /// Count of loaded graphics and music that were never destroyed.
    pub fn leaked_resources(&self) -> (Vec<FrameId>, Vec<MusicId>) {
        let events = self.events.borrow();
        let mut frames = Vec::new();
        let mut music = Vec::new();
        for event in events.iter() {
            match event {
                Event::LoadGraphic(_, frame) => frames.push(*frame),
                Event::DestroyDrawable(frame) => frames.retain(|f| f != frame),
                Event::LoadMusic(_, id) => music.push(*id),
                Event::DestroyMusic(id) => music.retain(|m| m != id),
                _ => {}
            }
        }
        (frames, music)
    }
}

pub fn press(set: impl FnOnce(&mut MenuPulses)) -> MenuPulses {
    let mut pulses = MenuPulses::default();
    set(&mut pulses);
    pulses
}

pub fn idle() -> MenuPulses {
    MenuPulses::default()
}

pub fn select() -> MenuPulses {
    press(|p| p.select = true)
}

pub fn right() -> MenuPulses {
    press(|p| p.right = true)
}

pub fn left() -> MenuPulses {
    press(|p| p.left = true)
}

pub fn up() -> MenuPulses {
    press(|p| p.up = true)
}

pub fn down() -> MenuPulses {
    press(|p| p.down = true)
}

pub fn cancel() -> MenuPulses {
    press(|p| p.cancel = true)
}

impl Gfx for RecordingPlatform {
    fn screen_size(&self) -> (i32, i32) {
        (SCREEN_WIDTH, SCREEN_HEIGHT)
    }

    fn set_context(&self, ctx: ContextId) -> ContextHandle {
        self.record(Event::SetContext(ctx));
        ContextHandle(7)
    }

    fn restore_context(&self, _previous: ContextHandle) {
        self.record(Event::RestoreContext);
    }

    fn context_clip_rect(&self) -> Rect {
        Rect {
            corner: Point { x: 250, y: 0 },
            width: 70,
            height: 240,
        }
    }

    fn set_fg_color(&self, color: Rgba) -> Rgba {
        self.record(Event::SetFg(color));
        Rgba::rgb15(1, 2, 3)
    }

    fn set_bg_color(&self, color: Rgba) -> Rgba {
        self.record(Event::SetBg(color));
        Rgba::rgb15(9, 8, 7)
    }

    fn set_font(&self, font: FontId) -> FontHandle {
        self.record(Event::SetFont(font));
        FontHandle(3)
    }

    fn restore_font(&self, _previous: FontHandle) {
        self.record(Event::RestoreFont);
    }

    fn set_font_effect(&self, grad_index: Option<u16>) -> FontEffectHandle {
        self.record(Event::SetFontEffect(grad_index));
        FontEffectHandle(5)
    }

    fn restore_font_effect(&self, _previous: FontEffectHandle) {
        self.record(Event::RestoreFontEffect);
    }

    fn font_leading(&self) -> i16 {
        9
    }

    fn fill_rect(&self, rect: Rect) {
        self.record(Event::FillRect(rect));
    }

    fn draw_rect(&self, rect: Rect) {
        self.record(Event::DrawRect(rect));
    }

    fn draw_point(&self, point: Point) {
        self.record(Event::DrawPoint(point));
    }

    fn draw_stamp(&self, stamp: Stamp) {
        self.record(Event::DrawStamp(stamp));
    }

    fn draw_filled_stamp(&self, stamp: Stamp) {
        self.record(Event::DrawFilledStamp(stamp));
    }

    fn draw_text(&self, text: &TextDraw) {
        self.record(Event::Text(text.clone()));
    }

    fn abs_frame(&self, frame: FrameId, index: u16) -> FrameId {
        frame / FRAME_SET * FRAME_SET + usize::from(index)
    }

    fn rel_frame(&self, frame: FrameId, offset: i16) -> FrameId {
        (frame as i64 + i64::from(offset)) as FrameId
    }

    fn inc_frame(&self, frame: FrameId) -> FrameId {
        let set = frame / FRAME_SET * FRAME_SET;
        let next = (frame - set + 1) % usize::from(FRAMES_PER_SET);
        set + next
    }

    fn dec_frame(&self, frame: FrameId) -> FrameId {
        frame - 1
    }

    fn frame_count(&self, _frame: FrameId) -> u16 {
        FRAMES_PER_SET
    }

    fn frame_index(&self, frame: FrameId) -> u16 {
        (frame % FRAME_SET) as u16
    }

    fn play_frame(&self) -> FrameId {
        PLAY_FRAME
    }

    fn batch(&self) {
        self.record(Event::Batch);
    }

    fn unbatch(&self) {
        self.record(Event::Unbatch);
    }

    fn clear_drawable(&self) {
        self.record(Event::ClearDrawable);
    }

    fn set_transition_source_none(&self) {
        self.record(Event::TransitionSourceNone);
    }

    fn screen_transition(&self, kind: i32) {
        self.record(Event::ScreenTransition(kind));
    }

    fn set_flash(&self, target: FlashTarget) {
        self.record(Event::Flash(target));
    }

    fn pre_update_flash(&self) {
        self.record(Event::PreUpdateFlash);
    }

    fn post_update_flash(&self) {
        self.record(Event::PostUpdateFlash);
    }

    fn game_string(&self, index: i32) -> String {
        format!("S{index}")
    }
}

impl Platform for RecordingPlatform {
    fn current_activity(&self) -> ActivityValue {
        self.activity.get()
    }

    fn set_current_activity(&self, value: ActivityValue) {
        self.activity.set(value);
        self.record(Event::SetCurrentActivity(value));
    }

    fn last_activity(&self) -> ActivityValue {
        self.last_activity.get()
    }

    fn set_last_activity(&self, value: ActivityValue) {
        self.last_activity.set(value);
        self.record(Event::SetLastActivity(value));
    }

    fn set_next_activity(&self, value: ActivityValue) {
        self.record(Event::SetNextActivity(value));
    }

    fn game_state(&self, key: StateKey) -> u32 {
        self.state(key)
    }

    fn set_game_state(&self, key: StateKey, value: u32) {
        self.states.borrow_mut().insert(key, value);
        self.record(Event::SetState(key, value));
    }

    fn set_status_message_mode(&self, mode: StatusMessageMode) -> StatusMessageMode {
        self.record(Event::SetMessageMode(mode));
        self.message_mode.replace(mode)
    }

    fn sis(&self) -> Result<SisView, StarbaseError> {
        Ok(self.sis.borrow().clone())
    }

    fn set_fuel_on_board(&self, fuel: u32) {
        self.sis.borrow_mut().fuel_on_board = fuel;
        self.record(Event::SetFuelOnBoard(fuel));
    }

    fn apply_gauge_delta(&self, delta: GaugeDelta) {
        self.record(Event::GaugeDelta(delta));
        let mut sis = self.sis.borrow_mut();
        let capacity = ModuleSlots::from_raw(&sis.module_slots)
            .expect("mock slots are valid")
            .fuel_tank_capacity();
        let applied = FuelRuState {
            fuel_on_board: sis.fuel_on_board,
            res_units: sis.res_units,
        }
        .apply(delta, capacity)
        .expect("mock gauge delta applies");
        sis.fuel_on_board = applied.fuel_on_board;
        sis.res_units = applied.res_units;
        if self.corrupt_gauge.get() {
            sis.fuel_on_board += 100;
        }
    }

    fn set_drive_slot(&self, index: usize, piece: u8) {
        self.sis.borrow_mut().drive_slots[index] = piece;
        self.record(Event::SetDriveSlot(index, piece));
    }

    fn set_jet_slot(&self, index: usize, piece: u8) {
        self.sis.borrow_mut().jet_slots[index] = piece;
        self.record(Event::SetJetSlot(index, piece));
    }

    fn set_module_slot(&self, index: usize, piece: u8) {
        self.sis.borrow_mut().module_slots[index] = piece;
        self.record(Event::SetModuleSlot(index, piece));
    }

    fn set_num_landers(&self, count: u8) {
        self.sis.borrow_mut().num_landers = count;
        self.record(Event::SetNumLanders(count));
    }

    fn redraw_ship_status(&self, res_units: ResUnitsUpdate) {
        self.record(Event::ShipStatus(res_units));
        let mut sis = self.sis.borrow_mut();
        if let ResUnitsUpdate::Change(delta) = res_units {
            // `DeltaSISGauges_resunitDelta`: a payment floors at zero.
            sis.res_units = sis.res_units.saturating_add_signed(delta);
            if self.corrupt_res_units.get() {
                sis.res_units += 1;
            }
        }
    }

    fn draw_sis_frame(&self) {
        self.record(Event::SisFrame);
    }

    fn draw_sis_message(&self, text: &str) {
        self.record(Event::SisMessage(text.to_owned()));
    }

    fn draw_sis_title(&self, text: &str) {
        self.record(Event::SisTitle(text.to_owned()));
    }

    fn pulses(&self) -> MenuPulses {
        self.current_pulses.get()
    }

    fn set_menu_sounds(&self, sounds: MenuSounds) {
        self.record(Event::MenuSounds(sounds));
    }

    fn play_failure_sound(&self) {
        self.record(Event::FailureSound);
    }

    fn flush_input(&self) {
        self.record(Event::FlushInput);
    }

    fn menu_style(&self) -> UiStyle {
        self.menu_style.get()
    }

    fn font_style(&self) -> UiStyle {
        self.font_style.get()
    }

    fn load_graphic(&self, resource: &str) -> Result<FrameId, StarbaseError> {
        if self.failing_resource.borrow().as_deref() == Some(resource) {
            return Err(StarbaseError::ResourceLoad(resource.to_owned()));
        }
        let set = self.next_frame_set.get();
        self.next_frame_set.set(set + 1);
        let frame = set * FRAME_SET;
        self.record(Event::LoadGraphic(resource.to_owned(), frame));
        Ok(frame)
    }

    fn destroy_drawable(&self, frame: FrameId) {
        self.record(Event::DestroyDrawable(frame));
    }

    fn load_music(&self, resource: &str) -> Result<MusicId, StarbaseError> {
        if self.failing_resource.borrow().as_deref() == Some(resource) {
            return Err(StarbaseError::ResourceLoad(resource.to_owned()));
        }
        let id = self.next_music.get();
        self.next_music.set(id + 1);
        self.record(Event::LoadMusic(resource.to_owned(), id));
        Ok(id)
    }

    fn play_music(&self, music: MusicId) {
        self.record(Event::PlayMusic(music));
    }

    fn stop_music(&self) {
        self.record(Event::StopMusic);
    }

    fn destroy_music(&self, music: MusicId) {
        self.record(Event::DestroyMusic(music));
    }

    fn now(&self) -> u32 {
        self.clock.get()
    }

    fn sleep_thread(&self, ticks: u32) {
        self.clock.set(self.clock.get() + ticks);
        self.record(Event::Sleep(ticks));
    }

    fn sleep_thread_until(&self, wake_time: u32) {
        self.clock.set(self.clock.get().max(wake_time));
        self.record(Event::SleepUntil(wake_time));
    }

    fn fade_to_black(&self, ticks: u32) -> u32 {
        self.record(Event::Fade(ticks));
        self.clock.get() + ticks
    }

    fn advance_clock_days(&self, days: u32) {
        self.record(Event::AdvanceDays(days));
    }

    fn observe_starbase_menu(&self, active: bool) {
        self.record(Event::Docked(active));
    }

    fn run_commander_conversation(&self) {
        self.record(Event::Conversation);
        // The conversation clears the in-Starbase mark.
        self.states
            .borrow_mut()
            .insert(StateKey::GlobalFlagsAndData, 0);
    }

    fn run_input_loop(&self, frame: &mut dyn FnMut() -> bool) {
        for _ in 0..LOOP_FRAME_LIMIT {
            let pulses = self.script.borrow_mut().pop_front().unwrap_or_default();
            self.current_pulses.set(pulses);
            let consumed = self.consumed.get() + 1;
            self.consumed.set(consumed);
            if self.abort_after_pulses.get() == Some(consumed) {
                self.activity.set(ActivityValue::new(
                    crate::mainloop::types::activity_flags::CHECK_ABORT,
                ));
            }
            if !frame() {
                return;
            }
        }
        panic!("scripted input loop did not end within {LOOP_FRAME_LIMIT} frames");
    }
}
