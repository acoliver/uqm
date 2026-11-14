use super::{
    picker::{PickerState, MAX_SAVED_GAMES},
    render::PickerDisplay,
};
use crate::engine::{save::summary_record::SummaryDesc, Game};
use crate::graphics::{
    context_abi::{ContextFrame, ContextLease},
    native_abi::{EXTENT, POINT, RECT},
};

#[test]
#[serial_test::serial]
fn prepared_picker_draws_empty_and_populated_pages_without_published_owners() {
    if !crate::native_test_process::own_graphics_process("front_end::gameopt::render_tests::prepared_picker_draws_empty_and_populated_pages_without_published_owners") { return; }
    use crate::graphics::drawable_abi::LoadedDrawable;
    let mut game = Game::new();
    let fixture = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    game.kernel.space_context = game.kernel.status_context;
    game.kernel.radar_context = game.kernel.status_context;
    game.kernel.star_con_font = game.kernel.tiny_font;
    let frames = LoadedDrawable::create(8, 1, 1, 100).unwrap();
    game.kernel.play_frame = frames.lease().frame(0).unwrap();
    game.kernel.misc_data_frame = game.kernel.play_frame;
    game.kernel.flag_stat_frame = game.kernel.play_frame;
    game.kernel.font_grad_frame = game.kernel.play_frame;
    game.kernel
        .install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(
            vec![b"A\0".to_vec(); 600],
        ));
    let target = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen));
    let context = ContextLease::require(game.kernel.space_context);
    context.set_frame(target.clone());
    context.set_clip(RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 320,
            height: 240,
        },
    });
    let display = PickerDisplay::new(&game.kernel);
    let mut state = PickerState::new([SummaryDesc::default(); MAX_SAVED_GAMES], false, 7);
    let no_rng = || panic!("empty preview must not consume RNG");
    display.redraw(&game.kernel, &mut game.world, (320, 240), no_rng, &state);
    display.header(false);
    display.header(true);
    context.flush();
    let selected = target.pixel(POINT { x: 8, y: 186 });
    assert_eq!((selected.r, selected.g, selected.b), (222, 0, 222));
    let unselected = target.pixel(POINT { x: 8, y: 160 });
    assert_eq!((unselected.r, unselected.g, unselected.b), (0, 0, 165));
    state.summary[7].year_index = 2155;
    state.summary[7].month_index = 1;
    state.summary[7].day_index = 7;
    state.summary[7].activity = 3;
    state.summary[7].sis.num_landers = 2;
    state.summary[7].sis.total_bio_mass = 12;
    state.summary[7].sis.element_amounts = [12; 8];
    state.summary[7].save_name[..2].copy_from_slice(b"A\0");
    game.world.data.flagship.num_landers = 9;
    game.world.data.flagship.total_bio_mass = 99;
    for flags in [0, 128] {
        state.summary[7].flags = flags;
        display.redraw(&game.kernel, &mut game.world, (320, 240), no_rng, &state);
        context.flush();
        assert_eq!(game.world.data.flagship.num_landers, 9);
        assert_eq!(game.world.data.flagship.total_bio_mass, 99);
        assert_eq!(context.clip_rect().extent.width, 320);
    }
    drop(display);
    drop(context);
    drop(target);
    drop(frames);
    fixture.finish(game);
}
