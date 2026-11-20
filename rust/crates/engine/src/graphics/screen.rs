//! Screen copies and the cross-fade transition, ported from gfx_common.c
//! (GPL-2.0-or-later).
use super::dcq::Screen;
use super::geometry::Rect;
use super::state::{self, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::time::counter_abi::get_time_counter;

/// C: `DrawFromExtraScreen`.
pub fn draw_from_extra_screen(rect: Option<Rect>) {
    state::with(|g| g.enqueue_copy(rect, Screen::Extra, Screen::Main));
}

/// C: `LoadIntoExtraScreen`.
pub fn load_into_extra_screen(rect: Option<Rect>) {
    state::with(|g| g.enqueue_copy(rect, Screen::Main, Screen::Extra));
}

/// Grow `rect` by `expansion` on every side, within the screen.
fn expand_rect(rect: &mut Rect, expansion: i32, width: i32, height: i32) {
    if i32::from(rect.corner.x) - expansion >= 0 {
        rect.extent.width = (i32::from(rect.extent.width) + expansion) as i16;
        rect.corner.x = (i32::from(rect.corner.x) - expansion) as i16;
    } else {
        rect.extent.width = rect.extent.width.wrapping_add(rect.corner.x);
        rect.corner.x = 0;
    }
    if i32::from(rect.corner.y) - expansion >= 0 {
        rect.extent.height = (i32::from(rect.extent.height) + expansion) as i16;
        rect.corner.y = (i32::from(rect.corner.y) - expansion) as i16;
    } else {
        rect.extent.height = rect.extent.height.wrapping_add(rect.corner.y);
        rect.corner.y = 0;
    }
    rect.extent.width =
        if i32::from(rect.corner.x) + i32::from(rect.extent.width) + expansion <= width {
            (i32::from(rect.extent.width) + expansion) as i16
        } else {
            (width - i32::from(rect.corner.x)) as i16
        };
    rect.extent.height =
        if i32::from(rect.corner.y) + i32::from(rect.extent.height) + expansion <= height {
            (i32::from(rect.extent.height) + expansion) as i16
        } else {
            (height - i32::from(rect.corner.y)) as i16
        };
}

/// C: `SetTransitionSource`: copy the screen (or an area grown by two
/// pixels) to the transition screen.
pub fn set_transition_source(rect: Option<Rect>) {
    let rect = rect.map(|mut rect| {
        expand_rect(&mut rect, 2, SCREEN_WIDTH, SCREEN_HEIGHT);
        rect
    });
    state::with(|g| g.enqueue_copy(rect, Screen::Main, Screen::Transition));
}

/// C: `ScreenTransition`: cross-fade from the transition screen over half a
/// second, presenting as it goes.
pub fn screen_transition(rect: Option<Rect>) {
    const DURATION: u32 = 840 * 31 / 60;
    state::with(|g| {
        g.transition_clip =
            rect.unwrap_or(Rect::new(0, 0, SCREEN_WIDTH as i16, SCREEN_HEIGHT as i16));
        g.transition_amount = 0;
    });
    state::wait_for_signal();
    let start = get_time_counter();
    while state::with(|g| g.transition_amount) < 255 {
        // SAFETY: transitions run on the game thread with graphics and input up.
        crate::threading::pacing::linked::sleep_thread(840 / 100);
        let delta = get_time_counter().wrapping_sub(start);
        let amount = ((delta.wrapping_mul(255) / DURATION) as i32).min(255);
        state::with(|g| g.transition_amount = amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_sources_grow_by_two_pixels_inside_the_screen() {
        let mut rect = Rect::new(1, 10, 20, 30);
        expand_rect(&mut rect, 2, 320, 240);
        assert_eq!(rect, Rect::new(0, 8, 23, 34));
        let mut rect = Rect::new(300, 220, 20, 20);
        expand_rect(&mut rect, 2, 320, 240);
        assert_eq!(rect, Rect::new(298, 218, 22, 22));
    }
}
