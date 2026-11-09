//! UI drawing and nested input waits.
use crate::engine::{
    battle::Cx,
    input::{frames::KEY_PAUSE, InputSession},
    observe::Io,
};
use crate::graphics::{
    context_abi::{ContextFrame, ContextLease},
    drawable_abi::LoadedDrawable,
    native_abi::{Color, POINT, RECT},
};
use crate::sound::heart_ffi::{pause_track, playing_track, resume_track};
use uqm_engine::graphics::sdl::lifecycle_abi::{game_active, quit_posted};
use uqm_engine::time::counter_abi::{GetTimeCounter, ONE_SECOND};

pub(crate) fn box_rectangles(
    rect: RECT,
    mut border: i16,
    top: Color,
    bottom: Color,
    interior: Option<Color>,
    mut draw: impl FnMut(Color, RECT),
) {
    let mut r = rect;
    if border == 0 {
        border = 2;
    } else {
        r.extent.height = 1;
        draw(top, r);
        if border == 2 {
            r.corner.x += 1;
            r.corner.y += 1;
            r.extent.width -= 2;
            draw(top, r);
        }
        r = rect;
        r.extent.width = 1;
        draw(top, r);
        if border == 2 {
            r.corner.x += 1;
            r.corner.y += 1;
            r.extent.height -= 2;
            draw(top, r);
        }
        r.corner.x = rect.corner.x + rect.extent.width - 1;
        r.corner.y = rect.corner.y + 1;
        r.extent.height = rect.extent.height - 1;
        draw(bottom, r);
        if border == 2 {
            r.corner.x -= 1;
            r.corner.y += 1;
            r.extent.height -= 2;
            draw(bottom, r);
        }
        r.corner.x = rect.corner.x;
        r.extent.width = rect.extent.width;
        r.corner.y = rect.corner.y + rect.extent.height - 1;
        r.extent.height = 1;
        draw(bottom, r);
        if border == 2 {
            r.corner.x += 1;
            r.corner.y -= 1;
            r.extent.width -= 2;
            draw(bottom, r);
        }
    }
    if let Some(color) = interior {
        r.corner.x = rect.corner.x + border;
        r.corner.y = rect.corner.y + border;
        r.extent.width = rect.extent.width - (border << 1);
        r.extent.height = rect.extent.height - (border << 1);
        draw(color, r);
    }
}

pub struct SavedContext {
    context: ContextLease,
    drawable: LoadedDrawable,
    origin: POINT,
}

impl SavedContext {
    pub fn capture(context: &ContextLease, area: Option<RECT>) -> Self {
        Self {
            context: context.clone(),
            drawable: LoadedDrawable::capture(context, area)
                .unwrap_or_else(|| panic!("UI background capture failed")),
            origin: area.map_or(POINT { x: 0, y: 0 }, |area| area.corner),
        }
    }

    pub fn restore(&self) {
        self.context
            .draw_stamp(&self.drawable.lease().context_frame(0), self.origin);
    }
}

pub fn draw_starcon_box(
    context: &ContextLease,
    rect: RECT,
    border: i16,
    top: Color,
    bottom: Color,
    interior: Option<Color>,
) {
    let mut current = None;
    box_rectangles(rect, border, top, bottom, interior, |color, rect| {
        let rgba = (color.r, color.g, color.b, color.a);
        if current != Some(rgba) {
            context.set_foreground(color);
            current = Some(rgba);
        }
        context.fill_rectangle(color, rect);
    });
}

pub fn seed_random_numbers() -> u32 {
    let now = GetTimeCounter();
    crate::math::seed_random(now);
    now
}

pub fn pause_game(cx: &mut Cx<'_>, input: &InputSession) -> u32 {
    if cx.kernel.activity_frame.is_null()
        || cx.world.activity() & (0x4000 | 0x100) != 0
        || cx.kernel.last_activity & (0x1000 | 0x2000) != 0
    {
        return 0;
    }
    cx.world.set_activity(cx.world.activity() | 0x100);
    if playing_track() != 0 {
        pause_track();
    }
    let context = ContextLease::require(cx.kernel.screen_context);
    let activity = ContextFrame::retain(std::ptr::NonNull::new(cx.kernel.activity_frame));
    let runtime = cx.kernel.input_runtime();
    context.with_selected(|| {
        context.with_origin(POINT { x: 0, y: 0 }, || {
            context.with_unclipped(|| {
                let bounds = context.clip_rect();
                let mut rect = activity.rect();
                rect.corner.x = (bounds.extent.width - rect.extent.width) >> 1;
                rect.corner.y = (bounds.extent.height - rect.extent.height) >> 1;
                let saved = SavedContext::capture(&context, Some(rect));
                context.system_rect(Some(rect));
                context.draw_stamp(&activity, rect.corner);
                context.flush();
                for pressed in [true, false, true] {
                    while (input.immediate().menu[KEY_PAUSE as usize] != 0) == pressed
                        && input.paused()
                    {
                        uqm_engine::input_port::sdl_input::BeginInputFrame();
                        cx.with_legacy(uqm_engine::threading::yield_game_thread);
                    }
                }
                input.set_paused(false);
                saved.restore();
                context.system_rect(None);
            })
        })
    });
    let mut observer = runtime.observer();
    let mut io = Io {
        observer: &mut *observer,
    };
    wait_for_no_input(cx, &mut io, ONE_SECOND / 4, 1);
    if playing_track() != 0 {
        resume_track();
    }
    cx.with_legacy(uqm_engine::threading::yield_game_thread);
    cx.world.set_activity(cx.world.activity() & !0x100);
    1
}

fn wait_until(cx: &mut Cx<'_>, io: &mut Io<'_>, released: bool, timeout: u32, reset: u32) -> u32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    runtime.update(cx);
    let mut pressed = input.current().any_button_pressed();
    while pressed == released
        && (timeout == u32::MAX || GetTimeCounter() < timeout)
        && cx.world.activity() & 0x4000 == 0
        && !quit_posted()
    {
        let Some(sampled) = cx.nested_input(io, |cx| {
            runtime.sleep(cx, ONE_SECOND / 40);
            runtime.update(cx);
            input.current().any_button_pressed()
        }) else {
            break;
        };
        pressed = sampled;
    }
    if reset != 0 {
        runtime.flush();
    }
    u32::from(pressed != released)
}

pub fn wait_for_any_button_until(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    new_button: u32,
    timeout: u32,
    reset: u32,
) -> u32 {
    if new_button != 0 && wait_for_no_input_until(cx, io, timeout, 0) == 0 {
        return 0;
    }
    wait_until(cx, io, false, timeout, reset)
}

pub fn wait_for_any_button(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    new_button: u32,
    duration: u32,
    reset: u32,
) -> u32 {
    let timeout = if duration == u32::MAX {
        duration
    } else {
        duration.wrapping_add(GetTimeCounter())
    };
    wait_for_any_button_until(cx, io, new_button, timeout, reset)
}

pub fn wait_for_no_input_until(cx: &mut Cx<'_>, io: &mut Io<'_>, timeout: u32, reset: u32) -> u32 {
    wait_until(cx, io, true, timeout, reset)
}

pub fn wait_for_no_input(cx: &mut Cx<'_>, io: &mut Io<'_>, duration: u32, reset: u32) -> u32 {
    let timeout = if duration == u32::MAX {
        duration
    } else {
        duration.wrapping_add(GetTimeCounter())
    };
    wait_for_no_input_until(cx, io, timeout, reset)
}

pub fn sleep_game(cx: &mut Cx<'_>) {
    if quit_posted() {
        return;
    }
    log::debug!("Game is going to sleep");
    if playing_track() != 0 {
        pause_track();
    }
    crate::ui::settings::pause_music();
    let runtime = cx.kernel.input_runtime();
    while !game_active() && !quit_posted() {
        runtime.sleep(cx, ONE_SECOND / 2);
    }
    log::debug!("Game is waking up");
    let mut observer = runtime.observer();
    let mut io = Io {
        observer: &mut *observer,
    };
    wait_for_no_input(cx, &mut io, ONE_SECOND / 10, 1);
    crate::ui::settings::resume_music();
    if playing_track() != 0 {
        resume_track();
    }
    cx.with_legacy(uqm_engine::threading::yield_game_thread);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::native_abi::{EXTENT, POINT};

    fn rectangles(border: i16, fill: bool) -> Vec<(u8, i16, i16, i16, i16)> {
        let mut draws = Vec::new();
        let color = |r| Color {
            r,
            g: 0,
            b: 0,
            a: 255,
        };
        box_rectangles(
            RECT {
                corner: POINT { x: 10, y: 20 },
                extent: EXTENT {
                    width: 30,
                    height: 40,
                },
            },
            border,
            color(1),
            color(2),
            fill.then_some(color(3)),
            |c, r| draws.push((c.r, r.corner.x, r.corner.y, r.extent.width, r.extent.height)),
        );
        draws
    }

    #[test]
    fn double_border_preserves_corner_overlap_and_draw_order() {
        assert_eq!(
            rectangles(2, true),
            [
                (1, 10, 20, 30, 1),
                (1, 11, 21, 28, 1),
                (1, 10, 20, 1, 40),
                (1, 11, 21, 1, 38),
                (2, 39, 21, 1, 39),
                (2, 38, 22, 1, 37),
                (2, 10, 59, 30, 1),
                (2, 11, 58, 28, 1),
                (3, 12, 22, 26, 36),
            ]
        );
    }
    #[test]
    fn zero_border_only_fills_with_two_pixel_inset() {
        assert_eq!(rectangles(0, true), [(3, 12, 22, 26, 36)]);
        assert!(rectangles(0, false).is_empty());
    }
    #[test]
    fn single_border_uses_one_pixel_inset() {
        assert_eq!(
            rectangles(1, true),
            [
                (1, 10, 20, 30, 1),
                (1, 10, 20, 1, 40),
                (2, 39, 21, 1, 39),
                (2, 10, 59, 30, 1),
                (3, 11, 21, 28, 38)
            ]
        );
    }
}
