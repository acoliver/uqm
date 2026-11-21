//! Flashing screen regions, ported from flash.c (GPL-2.0-or-later).
//!
//! A flash fades a screen rectangle between its original pixels and a
//! brightened (highlight) or overlaid (overlay) version, drawing cached
//! intermediate frames through its context.
use super::canvas::raster::{DRAW_ADDITIVE, DRAW_REPLACE};
use super::context::{ContextId, ContextLease};
use super::drawable::{ContextFrame, FrameId, LoadedDrawable, WANT_PIXMAP};
use super::flash_math;
use super::geometry::{Color, DrawMode, Point, Rect};
use super::state::{self, Graphics};
use crate::callback::AlarmId;
use crate::time::counter_abi::{get_time_counter, ONE_SECOND};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

const DEFAULT_CACHE_SIZE: usize = 9;
const DRAW_FACTOR_1: i32 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    FadeIn = 0,
    On = 1,
    FadeOut = 2,
    Off = 3,
}

impl Phase {
    fn next(self) -> Self {
        match self {
            Self::FadeIn => Self::On,
            Self::On => Self::FadeOut,
            Self::FadeOut => Self::Off,
            Self::Off => Self::FadeIn,
        }
    }
}

enum Kind {
    Highlight,
    Overlay(Option<ContextFrame>),
}

/// C: `FlashContext`.
struct Flash {
    context: ContextLease,
    rect: Rect,
    original: Option<LoadedDrawable>,
    kind: Kind,
    start_numer: i32,
    end_numer: i32,
    denom: i32,
    fade_in_time: u32,
    on_time: u32,
    fade_out_time: u32,
    off_time: u32,
    frame_time: u32,
    phase: Phase,
    last_state_time: u32,
    last_frame_time: u32,
    started: bool,
    paused: bool,
    cache: Vec<Option<LoadedDrawable>>,
    last_frame_index: Option<usize>,
}

impl Graphics {
    /// The context flashes draw their frames with (C: `workGfxContext`).
    fn flash_work_context(&mut self) -> ContextId {
        if let Some(context) = self.flash_work {
            return context;
        }
        let context = self.create_context(c"Flash.workGfxContext");
        self.flash_work = Some(context);
        context
    }
}

fn select(context: Option<ContextId>) -> Option<ContextId> {
    state::with(|g| g.set_context(context))
}

impl Flash {
    fn new(context: ContextLease, kind: Kind) -> Self {
        state::with(|g| g.flash_work_context());
        Self {
            context,
            rect: Rect::default(),
            original: None,
            kind,
            start_numer: 0,
            end_numer: 1,
            denom: 1,
            fade_in_time: 0,
            on_time: ONE_SECOND / 8,
            fade_out_time: 0,
            off_time: ONE_SECOND / 8,
            frame_time: 0,
            phase: Phase::Off,
            last_state_time: 0,
            last_frame_time: 0,
            started: false,
            paused: false,
            cache: Vec::new(),
            last_frame_index: None,
        }
    }

    fn original_frame(&self) -> FrameId {
        self.original
            .as_ref()
            .map_or(FrameId::NULL, LoadedDrawable::first_frame)
    }

    fn clear_cache(&mut self) {
        for frame in &mut self.cache {
            *frame = None;
        }
    }

    fn grab_original(&mut self) {
        self.original = None;
        let old = select(Some(self.context.context()));
        let rect = self.rect;
        self.original = state::with(|g| g.copy_context_rect(Some(rect))).map(LoadedDrawable::adopt);
        select(old);
        state::wait_for_signal();
    }

    fn draw_frame(&self, frame: FrameId) {
        let old = select(Some(self.context.context()));
        let corner = self.rect.corner;
        state::with(|g| g.draw_stamp(frame, corner));
        select(old);
    }

    /// C: `Flash_makeFrame`: draw the blend `n/d` into `dest`.
    fn make_frame(&self, dest: FrameId, n: i32, d: i32) {
        assert!(n >= 0 && n <= d);
        let (bn, bd) =
            flash_math::blend_fraction(n, d, self.start_numer, self.end_numer, self.denom);
        let original = self.original_frame();
        let overlay = match &self.kind {
            Kind::Highlight => None,
            Kind::Overlay(frame) => Some(frame.as_ref().map_or(FrameId::NULL, ContextFrame::frame)),
        };
        state::with(|g| {
            let work = g.flash_work_context();
            let old = g.set_context(Some(work));
            g.set_context_fg_frame(dest);
            let origin = Point::default();
            let additive = DrawMode::new(DRAW_ADDITIVE, (DRAW_FACTOR_1 * bn / bd) as i16);
            match overlay {
                None => {
                    g.set_background(Color::rgba(0, 0, 0, 0));
                    g.clear_drawable();
                    g.set_draw_mode(additive);
                    g.draw_stamp(original, origin);
                }
                Some(overlay) => {
                    g.set_draw_mode(DrawMode::new(DRAW_REPLACE, 0));
                    g.draw_stamp(original, origin);
                    g.set_draw_mode(additive);
                    let hot = overlay.hot_spot_in(g);
                    let previous = std::mem::replace(&mut g.frame_mut(dest).hot_spot, hot);
                    g.draw_stamp(overlay, origin);
                    g.frame_mut(dest).hot_spot = previous;
                }
            }
            g.set_context(old);
        });
    }

    fn prepare_cache(&mut self, i: usize) {
        if self.cache[i].is_some() {
            return;
        }
        let drawable = LoadedDrawable::create(
            WANT_PIXMAP,
            self.rect.extent.width,
            self.rect.extent.height,
            1,
        )
        .unwrap_or_else(|| unreachable!("pixmap creation does not fail"));
        self.make_frame(
            drawable.first_frame(),
            i as i32,
            self.cache.len() as i32 - 1,
        );
        self.cache[i] = Some(drawable);
    }

    fn draw_current(&mut self) {
        let (n, d) = match self.phase {
            Phase::Off => (0, 1),
            Phase::On => (1, 1),
            phase => {
                let now = get_time_counter();
                let d = if phase == Phase::FadeIn {
                    self.fade_in_time as i32
                } else {
                    self.fade_out_time as i32
                };
                let mut n = (now.wrapping_sub(self.last_state_time) as i32).min(d);
                if phase == Phase::FadeOut {
                    n = self.fade_out_time as i32 - n;
                }
                (n, d)
            }
        };
        let i = flash_math::cache_position(self.cache.len() as u16, n, d) as usize;
        self.prepare_cache(i);
        if self.last_frame_index != Some(i) {
            let frame = self.cache[i]
                .as_ref()
                .map_or(FrameId::NULL, LoadedDrawable::first_frame);
            self.draw_frame(frame);
            self.last_frame_index = Some(i);
        }
    }

    fn duration(&self, phase: Phase) -> u32 {
        match phase {
            Phase::FadeIn => self.fade_in_time,
            Phase::On => self.on_time,
            Phase::FadeOut => self.fade_out_time,
            Phase::Off => self.off_time,
        }
    }

    fn fix_state(&mut self) {
        while self.duration(self.phase) == 0 {
            self.phase = self.phase.next();
        }
    }

    fn next_state(&mut self) {
        self.phase = self.phase.next();
        self.fix_state();
    }

    fn set_state(&mut self, phase: Phase, spent: u32) {
        let now = get_time_counter();
        self.phase = phase;
        self.fix_state();
        self.last_state_time = now.wrapping_sub(spent);
        self.last_frame_time = now;
        if self.started {
            self.draw_current();
        }
    }

    fn start(&mut self) {
        if self.started {
            log::warn!("Flash_start() called on already started FlashContext.");
            return;
        }
        self.cache = (0..DEFAULT_CACHE_SIZE).map(|_| None).collect();
        self.started = true;
        self.paused = false;
        self.grab_original();
        self.last_frame_index = Some(0);
        self.fix_state();
        self.draw_current();
    }

    fn process(&mut self) {
        if !self.started || self.paused {
            return;
        }
        let now = get_time_counter();
        let duration = self.duration(self.phase);
        match self.phase {
            Phase::FadeIn | Phase::FadeOut => {
                if now >= self.last_state_time.wrapping_add(duration) {
                    self.next_state();
                    self.last_state_time = now;
                }
                self.last_frame_time = now;
            }
            Phase::On | Phase::Off => {
                if now < self.last_state_time.wrapping_add(duration) {
                    return;
                }
                self.next_state();
                self.last_state_time = now;
            }
        }
        self.draw_current();
    }

    fn next_time(&self) -> u32 {
        if !self.started || self.paused {
            return u32::MAX;
        }
        match self.phase {
            Phase::FadeIn | Phase::FadeOut => self.last_frame_time.wrapping_add(self.frame_time),
            Phase::On => self.last_state_time.wrapping_add(self.on_time),
            Phase::Off => self.last_state_time.wrapping_add(self.off_time),
        }
    }

    /// Restore the original pixels and drop the cache before a change.
    ///
    /// The screen no longer shows any cached frame afterwards, so the next
    /// `draw_current` must draw even if the frame index is unchanged (#890;
    /// flash.c only reset the index in `Flash_setRect`).
    fn pre_update(&mut self) {
        if self.started {
            self.draw_frame(self.original_frame());
            self.clear_cache();
            self.last_frame_index = None;
        }
    }

    /// Recapture and redraw after a change.
    fn post_update(&mut self) {
        if self.started {
            self.grab_original();
            self.draw_current();
        }
    }

    fn set_merge_factors(&mut self, start: i32, end: i32, denom: i32) {
        self.pre_update();
        self.start_numer = start;
        self.end_numer = end;
        self.denom = denom;
        self.post_update();
    }

    fn set_rect(&mut self, rect: Rect) {
        assert!(matches!(self.kind, Kind::Highlight));
        self.pre_update();
        self.rect = rect;
        self.post_update();
    }

    fn set_overlay(&mut self, origin: Point, frame: ContextFrame) {
        assert!(matches!(self.kind, Kind::Overlay(_)));
        self.pre_update();
        let mut rect = frame.rect();
        rect.corner.x += origin.x;
        rect.corner.y += origin.y;
        self.rect = rect;
        self.kind = Kind::Overlay(Some(frame));
        self.post_update();
    }
}

impl FrameId {
    fn hot_spot_in(self, g: &Graphics) -> super::geometry::HotSpot {
        if self.is_null() {
            super::geometry::HotSpot::default()
        } else {
            g.frame(self).hot_spot
        }
    }
}

impl Drop for Flash {
    fn drop(&mut self) {
        if self.started {
            self.draw_frame(self.original_frame());
            self.clear_cache();
            self.original = None;
        }
    }
}

/// An exclusive flashing rectangle drawn through its context.
pub struct Highlight(Flash);

impl Highlight {
    pub(super) fn new(context: ContextLease, rect: Rect) -> Self {
        let mut flash = Flash::new(context, Kind::Highlight);
        flash.rect = rect;
        Self(flash)
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.0.set_rect(rect);
    }
    pub fn set_merge_factors(&mut self, start: i32, end: i32, denominator: i32) {
        assert_ne!(
            denominator, 0,
            "highlight merge denominator must be nonzero"
        );
        self.0.set_merge_factors(start, end, denominator);
    }
    pub fn set_frame_time(&mut self, time: u32) {
        self.0.frame_time = time;
    }
    pub fn set_speed(&mut self, fade_in: u32, on: u32, fade_out: u32, off: u32) {
        let flash = &mut self.0;
        (
            flash.fade_in_time,
            flash.on_time,
            flash.fade_out_time,
            flash.off_time,
        ) = (fade_in, on, fade_out, off);
    }
    pub fn start(&mut self) {
        self.0.start();
    }
    pub fn process(&mut self) {
        self.0.process();
    }
    pub fn pre_update(&mut self) {
        self.0.pre_update();
    }
    pub fn post_update(&mut self) {
        self.0.post_update();
    }

    /// Start the flash and keep it running from alarms until dropped.
    pub fn schedule(mut self) -> ScheduledHighlight {
        self.start();
        let id = SCHEDULED.with(|scheduled| {
            let mut scheduled = scheduled.borrow_mut();
            scheduled.next = scheduled
                .next
                .checked_add(1)
                .unwrap_or_else(|| panic!("highlight registration ID exhausted"));
            scheduled.next
        });
        let state = Rc::new(RefCell::new(ScheduledState {
            id,
            highlight: self,
            alarm: None,
            paused: false,
            update_depth: 0,
        }));
        SCHEDULED.with(|scheduled| {
            scheduled
                .borrow_mut()
                .live
                .insert(id, Rc::downgrade(&state));
        });
        state.borrow_mut().schedule();
        ScheduledHighlight(state)
    }
}

/// An exclusive overlay flash; it restores the screen before releasing its
/// frame.
pub struct Overlay(Flash);

impl ContextLease {
    pub fn overlay(&self) -> Overlay {
        Overlay(Flash::new(self.clone(), Kind::Overlay(None)))
    }
}

impl Overlay {
    pub fn set_frame(&mut self, origin: Point, frame: ContextFrame) {
        assert!(!frame.is_null(), "overlay requires a retained frame");
        self.0.set_overlay(origin, frame);
    }
    pub fn set_merge_factors(&mut self, start: i32, end: i32, denominator: i32) {
        assert_ne!(
            denominator, 0,
            "highlight merge denominator must be nonzero"
        );
        self.0.set_merge_factors(start, end, denominator);
    }
    pub fn set_speed(&mut self, fade_in: u32, on: u32, fade_out: u32, off: u32) {
        let flash = &mut self.0;
        (
            flash.fade_in_time,
            flash.on_time,
            flash.fade_out_time,
            flash.off_time,
        ) = (fade_in, on, fade_out, off);
    }
    pub fn set_frame_time(&mut self, time: u32) {
        self.0.frame_time = time;
    }
    pub fn fade_in(&mut self, spent: u32) {
        self.0.set_state(Phase::FadeIn, spent);
    }
    pub fn start(&mut self) {
        self.0.start();
    }
    pub fn process(&mut self) {
        self.0.process();
    }
    pub fn pause(&mut self) {
        self.0.paused = true;
    }
    pub fn resume(&mut self) {
        self.0.paused = false;
    }
}

#[derive(Default)]
struct Scheduled {
    next: u64,
    live: BTreeMap<u64, Weak<RefCell<ScheduledState>>>,
}

thread_local! {
    static SCHEDULED: RefCell<Scheduled> = RefCell::new(Scheduled::default());
}

struct ScheduledState {
    id: u64,
    highlight: Highlight,
    alarm: Option<AlarmId>,
    paused: bool,
    update_depth: u16,
}

impl ScheduledState {
    fn schedule(&mut self) {
        let next = self.highlight.0.next_time();
        let ms = (next / ONE_SECOND)
            .wrapping_mul(1000)
            .wrapping_add((next % ONE_SECOND) * 1000 / ONE_SECOND);
        self.alarm = Some(crate::host::dispatch().add_alarm_absolute_ms(ms, tick, self.id));
    }
}

impl Drop for ScheduledState {
    fn drop(&mut self) {
        cancel_alarm(self.alarm.take());
        SCHEDULED.with(|scheduled| scheduled.borrow_mut().live.remove(&self.id));
    }
}

fn cancel_alarm(alarm: Option<AlarmId>) {
    if let Some(alarm) = alarm {
        crate::host::dispatch().remove_alarm(alarm);
    }
}

/// An alarm for a scheduled highlight: process it and schedule the next.
fn tick(id: u64) {
    let state = SCHEDULED
        .with(|scheduled| scheduled.borrow().live.get(&id).and_then(Weak::upgrade))
        .unwrap_or_else(|| panic!("alarm fired for an unregistered highlight"));
    let mut state = state.borrow_mut();
    state.alarm = None;
    state.highlight.process();
    state.schedule();
}

/// A highlight kept running by alarms.
pub struct ScheduledHighlight(Rc<RefCell<ScheduledState>>);

impl ScheduledHighlight {
    pub fn set_rect(&mut self, rect: Rect) {
        self.0.borrow_mut().highlight.set_rect(rect);
    }

    pub fn pause(&mut self) {
        let mut state = self.0.borrow_mut();
        cancel_alarm(state.alarm.take());
        state.paused = true;
    }

    pub fn resume(&mut self) {
        let mut state = self.0.borrow_mut();
        if state.paused {
            state.schedule();
            state.paused = false;
        }
    }

    pub fn pre_update(&mut self) {
        let mut state = self.0.borrow_mut();
        if state.alarm.is_some() {
            state.update_depth = state.update_depth.wrapping_add(1);
            if state.update_depth <= 1 {
                state.highlight.pre_update();
            }
        }
    }

    pub fn post_update(&mut self) {
        let mut state = self.0.borrow_mut();
        if state.alarm.is_some() {
            state.update_depth = state.update_depth.wrapping_sub(1);
            if state.update_depth == 0 {
                state.highlight.post_update();
            }
        }
    }
}
