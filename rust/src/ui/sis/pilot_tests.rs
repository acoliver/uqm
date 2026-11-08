use super::*;
use crate::graphics::context_abi::ContextFrame;
use crate::{campaign_display_test::CampaignDisplay, engine::Game};

#[test]
#[serial_test::serial]
fn pilot_uses_the_retained_panel_and_preserves_phase_across_reset() {
    if !crate::native_test_process::own_graphics_process(concat!(
        module_path!(),
        "::pilot_uses_the_retained_panel_and_preserves_phase_across_reset"
    )) {
        return;
    }
    let mut game = Game::new();
    let display = CampaignDisplay::install(&mut game)
        .unwrap_or_else(|error| panic!("pilot display fixture: {error}"));
    game.world.data.game.autopilot.x = 0;
    game.world.data.game.autopilot.y = 0;
    game.world.data.flagship.fuel_on_board = 100;
    let context = ContextLease::require(game.kernel.off_screen_context);
    let frame = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen));
    let font = game.kernel.tiny_font_lease().unwrap();
    let measurement = font.measure(
        c"A",
        POINT { x: 85, y: 6 },
        TEXT_ALIGN_ALIGN_CENTER,
        u16::MAX,
    );
    let point = POINT {
        x: 8 + measurement.rect.corner.x,
        y: 2 + measurement.rect.corner.y,
    };
    let rgba = |color: Color| (color.r, color.g, color.b, color.a);
    let mut samples = [0, 2].into_iter();
    draw_autopilot_message(&game.kernel, &game.world, 320, || samples.next().unwrap());
    context.flush();
    assert_eq!(rgba(frame.pixel(point)), rgba(pilot_color(0)));
    game.kernel.pilot_flash.borrow_mut().reset();
    draw_autopilot_message(&game.kernel, &game.world, 320, || 64);
    context.flush();
    assert_eq!(rgba(frame.pixel(point)), rgba(pilot_color(0)));
    draw_autopilot_message(&game.kernel, &game.world, 320, || 65);
    context.flush();
    assert_eq!(rgba(frame.pixel(point)), rgba(pilot_color(1)));
    game.world.data.flagship.fuel_on_board = 0;
    game.world.data.game.autopilot.x = -1;
    game.world.data.game.autopilot.y = -1;
    draw_autopilot_message(&game.kernel, &game.world, 320, || 128);
    context.flush();
    assert_eq!(rgba(frame.pixel(point)), rgba(pilot_color(2)));
    game.world.data.flagship.fuel_on_board = 100;
    draw_autopilot_message(&game.kernel, &game.world, 320, || {
        panic!("clear samples clock")
    });
    context.flush();
    assert_eq!(rgba(frame.pixel(point)), rgba(rgb15(0, 0, 20)));
    draw_autopilot_message(&game.kernel, &game.world, 320, || {
        panic!("inactive samples clock")
    });
    drop(font);
    drop(frame);
    drop(context);
    display.finish(game);
}
