//! Tests for colormap loading, palette transformations and screen fades,
//! checking the palettes that end up in the manager.

use super::*;

const SPAN: Duration = Duration::from_secs(10);

fn raw_palette(shade: impl Fn(usize) -> [u8; 3]) -> Vec<u8> {
    (0..NUMBER_OF_PLUTVALS).flat_map(shade).collect()
}

fn manager_with_black_map(index: i32) -> ColorMapManager {
    let mut mgr = ColorMapManager::new();
    mgr.set_colors(index, index, &raw_palette(|_| [0, 0, 0]))
        .unwrap();
    mgr
}

fn red_ramp() -> Vec<Color> {
    (0..NUMBER_OF_PLUTVALS)
        .map(|i| Color::new(i as u8, 0, 0))
        .collect()
}

fn palette_of(mgr: &ColorMapManager, index: i32) -> Vec<Color> {
    mgr.colormaps[index as usize].as_ref().unwrap().get_colors()
}

/// Moves the transformation's clock so that it started `elapsed` ago and
/// ends `remaining` from now.
fn set_xform_clock(mgr: &ColorMapManager, slot: usize, elapsed: Duration, remaining: Duration) {
    let now = Instant::now();
    let mut xforms = mgr.xforms.lock().unwrap();
    xforms[slot].start_time = now.checked_sub(elapsed).unwrap();
    xforms[slot].end_time = now + remaining;
    xforms[slot].duration_ms = (elapsed + remaining).as_millis() as u64;
}

fn expire_xform(mgr: &ColorMapManager, slot: usize) {
    let now = Instant::now();
    let mut xforms = mgr.xforms.lock().unwrap();
    xforms[slot].end_time = now.checked_sub(Duration::from_millis(1)).unwrap();
}

#[test]
fn set_colors_stores_each_rgb_triplet_as_an_opaque_color() {
    let mut mgr = manager_with_black_map(0);
    mgr.set_colors(
        4,
        4,
        &raw_palette(|i| [i as u8, (255 - i) as u8, (i * 3) as u8]),
    )
    .unwrap();
    let map = mgr.get_colormap(4).unwrap();
    assert_eq!(map.get_color(0), Color::new(0, 255, 0));
    assert_eq!(map.get_color(10), Color::new(10, 245, 30));
    assert_eq!(map.get_color(255), Color::new(255, 0, 253));
    assert!(map.get_colors().iter().all(|color| color.is_opaque()));
    assert_eq!(map.index(), 4);
    assert_eq!(mgr.map_count(), 5);
}

#[test]
fn set_colors_over_several_maps_assigns_consecutive_palettes() {
    let mut mgr = ColorMapManager::new();
    let mut raw = raw_palette(|_| [1, 1, 1]);
    raw.extend(raw_palette(|_| [2, 2, 2]));
    mgr.set_colors(7, 8, &raw).unwrap();
    assert_eq!(
        mgr.get_colormap(7).unwrap().get_color(100),
        Color::new(1, 1, 1)
    );
    assert_eq!(
        mgr.get_colormap(8).unwrap().get_color(100),
        Color::new(2, 2, 2)
    );
    assert!(mgr.get_colormap(9).is_none());
}

#[test]
fn set_colors_replaces_an_existing_palette() {
    let mut mgr = manager_with_black_map(2);
    assert_eq!(
        mgr.get_colormap(2).unwrap().get_color(0),
        Color::new(0, 0, 0)
    );
    mgr.set_colors(2, 2, &raw_palette(|_| [9, 8, 7])).unwrap();
    let map = mgr.get_colormap(2).unwrap();
    assert_eq!(map.get_color(0), Color::new(9, 8, 7));
    assert_eq!(map.get_color(255), Color::new(9, 8, 7));
}

#[test]
fn set_colors_rejects_ranges_that_do_not_fit() {
    let mut mgr = ColorMapManager::new();
    let one = raw_palette(|_| [0, 0, 0]);
    assert!(mgr.set_colors(-1, 0, &one).is_err());
    assert!(mgr.set_colors(0, MAX_COLORMAPS as i32, &one).is_err());
    assert_eq!(
        mgr.map_count(),
        0,
        "a refused load must not grow the map count"
    );
}

#[test]
fn set_colors_reports_the_expected_size_when_the_data_is_short() {
    let mut mgr = ColorMapManager::new();
    let error = mgr
        .set_colors(0, 1, &raw_palette(|_| [0, 0, 0]))
        .unwrap_err();
    assert!(error.contains("expected 1536"), "{error}");
    assert!(error.contains("got 768"), "{error}");
    let error = mgr.set_colors(3, 1, &[]).unwrap_err();
    assert!(error.contains("start 3 > end 1"), "{error}");
}

#[test]
fn get_colormap_outside_the_table_is_none() {
    let mgr = manager_with_black_map(0);
    assert!(mgr.get_colormap(-1).is_none());
    assert!(mgr.get_colormap(MAX_COLORMAPS as i32).is_none());
}

#[test]
fn get_and_return_colormap_track_references() {
    let mgr = manager_with_black_map(1);
    let held = mgr.get_colormap(1).unwrap();
    assert_eq!(held.refcount(), 2);
    mgr.return_colormap(&held);
    assert_eq!(held.refcount(), 1);
    assert_eq!(mgr.spare_head.lock().unwrap().len(), 0);
}

#[test]
fn returning_the_last_reference_parks_a_spare_that_a_later_load_reuses() {
    let mut mgr = manager_with_black_map(1);
    let map = mgr.get_colormap(1).unwrap();
    mgr.return_colormap(&map);
    mgr.return_colormap(&map);
    assert_eq!(map.refcount(), 0);
    assert_eq!(mgr.spare_head.lock().unwrap().len(), 1);

    mgr.set_colors(6, 6, &raw_palette(|_| [5, 5, 5])).unwrap();
    assert_eq!(mgr.spare_head.lock().unwrap().len(), 0);
    let reused = mgr.get_colormap(6).unwrap();
    assert_eq!(reused.index(), 6, "a recycled map must take the new index");
    assert_eq!(reused.get_color(0), Color::new(5, 5, 5));
}

#[test]
#[should_panic(expected = "refcount == 0")]
fn releasing_a_map_with_no_references_is_a_bug() {
    let mgr = manager_with_black_map(1);
    let map = mgr.get_colormap(1).unwrap();
    mgr.return_colormap(&map);
    mgr.return_colormap(&map);
    mgr.return_colormap(&map);
}

#[test]
fn spare_pool_is_capped() {
    let mgr = ColorMapManager::new();
    for index in 0..(SPARE_COLORMAPS as i16 + 5) {
        let map: ColorMapRef = Arc::new(ColorMapInner::new(index));
        mgr.return_colormap(&map);
    }
    assert_eq!(mgr.spare_head.lock().unwrap().len(), SPARE_COLORMAPS);
}

#[test]
fn uninit_drops_maps_spares_and_pending_transformations() {
    let mut mgr = manager_with_black_map(3);
    mgr.transform_colormap(3, &red_ramp(), 1000).unwrap();
    mgr.uninit();
    assert!(mgr.get_colormap(3).is_none());
    assert_eq!(mgr.map_count(), 0);
    assert!(!mgr.xforms.lock().unwrap().iter().any(|x| x.in_use()));
    assert!(!mgr.step_transformations());
}

#[test]
fn transformation_snapshots_the_starting_palette_and_deadline() {
    let mgr = manager_with_black_map(0);
    let before = Instant::now();
    let deadline = mgr.transform_colormap(0, &red_ramp(), 500).unwrap();
    assert!(deadline >= before + Duration::from_millis(500));
    let xforms = mgr.xforms.lock().unwrap();
    assert_eq!(xforms[0].cmap_index, 0);
    assert_eq!(xforms[0].old_colors[200], Color::new(0, 0, 0));
    assert_eq!(xforms[0].target_colors[200], Color::new(200, 0, 0));
    assert_eq!(*mgr.highest_xform.lock().unwrap(), 1);
}

#[test]
fn a_second_transformation_of_the_same_map_reuses_its_slot() {
    let mgr = manager_with_black_map(0);
    mgr.transform_colormap(0, &red_ramp(), 500).unwrap();
    mgr.transform_colormap(0, &red_ramp(), 900).unwrap();
    let xforms = mgr.xforms.lock().unwrap();
    assert_eq!(xforms.iter().filter(|x| x.in_use()).count(), 1);
    assert_eq!(xforms[0].duration_ms, 900);
}

#[test]
fn transformations_run_out_of_slots_after_the_sixteenth_map() {
    let mut mgr = ColorMapManager::new();
    let mut raw = Vec::new();
    for _ in 0..=MAX_XFORMS {
        raw.extend(raw_palette(|_| [0, 0, 0]));
    }
    mgr.set_colors(0, MAX_XFORMS as i32, &raw).unwrap();
    for index in 0..MAX_XFORMS as i32 {
        mgr.transform_colormap(index, &red_ramp(), 1000).unwrap();
    }
    assert_eq!(*mgr.highest_xform.lock().unwrap(), MAX_XFORMS);
    let error = mgr
        .transform_colormap(MAX_XFORMS as i32, &red_ramp(), 1000)
        .unwrap_err();
    assert!(error.contains("no available xform slots"), "{error}");
}

#[test]
fn stepping_halfway_through_a_transformation_blends_old_and_target_colors() {
    let mut mgr = manager_with_black_map(0);
    mgr.transform_colormap(0, &red_ramp(), 20_000).unwrap();
    set_xform_clock(&mgr, 0, SPAN, SPAN);
    let version = mgr.get_colormap(0).unwrap().version();

    assert!(
        mgr.step_transformations(),
        "a running transformation is active"
    );

    let blended = palette_of(&mgr, 0)[200];
    assert!(
        (99..=101).contains(&blended.r),
        "halfway between 0 and 200 is 100, got {}",
        blended.r
    );
    assert_eq!((blended.g, blended.b, blended.a), (0, 0, 255));
    assert!(mgr.get_colormap(0).unwrap().version() != version);
}

#[test]
fn stepping_blends_toward_darker_targets_too() {
    let mut mgr = ColorMapManager::new();
    mgr.set_colors(0, 0, &raw_palette(|_| [200, 200, 200]))
        .unwrap();
    let target = vec![Color::new(0, 0, 0); NUMBER_OF_PLUTVALS];
    mgr.transform_colormap(0, &target, 20_000).unwrap();
    set_xform_clock(&mgr, 0, SPAN, SPAN);
    assert!(mgr.step_transformations());
    let blended = palette_of(&mgr, 0)[0];
    assert!((99..=101).contains(&blended.r), "got {}", blended.r);
}

#[test]
fn stepping_after_the_deadline_installs_the_target_palette_and_goes_idle() {
    let mut mgr = manager_with_black_map(0);
    mgr.transform_colormap(0, &red_ramp(), 1000).unwrap();
    expire_xform(&mgr, 0);

    assert!(!mgr.step_transformations(), "nothing is left running");

    assert_eq!(palette_of(&mgr, 0), red_ramp());
    assert_eq!(*mgr.highest_xform.lock().unwrap(), 0);
    assert!(!mgr.xforms.lock().unwrap().iter().any(|x| x.in_use()));
}

#[test]
fn an_expired_transformation_does_not_stop_a_running_one() {
    let mut mgr = ColorMapManager::new();
    let mut raw = raw_palette(|_| [0, 0, 0]);
    raw.extend(raw_palette(|_| [0, 0, 0]));
    mgr.set_colors(0, 1, &raw).unwrap();
    mgr.transform_colormap(0, &red_ramp(), 1000).unwrap();
    mgr.transform_colormap(1, &red_ramp(), 20_000).unwrap();
    expire_xform(&mgr, 0);
    set_xform_clock(&mgr, 1, SPAN, SPAN);

    assert!(mgr.step_transformations());

    assert_eq!(
        palette_of(&mgr, 0),
        red_ramp(),
        "finished map has its target"
    );
    assert!(
        palette_of(&mgr, 1)[200].r > 0,
        "running map has started blending"
    );
    assert_eq!(*mgr.highest_xform.lock().unwrap(), 2);
}

#[test]
fn finishing_transformations_installs_every_target_palette() {
    let mut mgr = ColorMapManager::new();
    let mut raw = raw_palette(|_| [0, 0, 0]);
    raw.extend(raw_palette(|_| [0, 0, 0]));
    mgr.set_colors(0, 1, &raw).unwrap();
    let green: Vec<Color> = vec![Color::new(0, 77, 0); NUMBER_OF_PLUTVALS];
    mgr.transform_colormap(0, &red_ramp(), 60_000).unwrap();
    mgr.transform_colormap(1, &green, 60_000).unwrap();

    mgr.finish_transformations();

    assert_eq!(palette_of(&mgr, 0), red_ramp());
    assert_eq!(palette_of(&mgr, 1), green);
    assert_eq!(*mgr.highest_xform.lock().unwrap(), 0);
}

#[test]
fn flush_color_xforms_completes_fades_and_transformations() {
    let mut mgr = manager_with_black_map(0);
    mgr.transform_colormap(0, &red_ramp(), 60_000).unwrap();
    mgr.fade_screen(FadeType::FadeToBlack, 60_000);

    mgr.flush_color_xforms();

    assert_eq!(palette_of(&mgr, 0), red_ramp());
    assert_eq!(mgr.get_fade_amount(), FADE_NO_INTENSITY);
    assert!(!mgr.step_transformations());
}

#[test]
fn a_transformation_of_a_map_that_was_unloaded_is_dropped_quietly() {
    let mut mgr = manager_with_black_map(0);
    mgr.transform_colormap(0, &red_ramp(), 60_000).unwrap();
    mgr.colormaps[0] = None;
    mgr.finish_transformations();
    assert!(mgr.get_colormap(0).is_none());
    assert!(!mgr.step_transformations());
}

#[test]
fn fade_to_each_target_reaches_its_intensity() {
    let mgr = ColorMapManager::new();
    mgr.fade_screen(FadeType::FadeToWhite, 0);
    assert_eq!(mgr.get_fade_amount(), FADE_FULL_INTENSITY);
    mgr.fade_screen(FadeType::FadeToBlack, 0);
    assert_eq!(mgr.get_fade_amount(), FADE_NO_INTENSITY);
    mgr.fade_screen(FadeType::FadeToColor, 0);
    assert_eq!(mgr.get_fade_amount(), FADE_NORMAL_INTENSITY);
}

#[test]
fn a_timed_fade_reports_its_deadline_and_settles_on_the_target() {
    let mgr = ColorMapManager::new();
    let before = Instant::now();
    let deadline = mgr.fade_screen(FadeType::FadeToBlack, 750);
    assert!(deadline >= before + Duration::from_millis(750));
    mgr.finish_fade();
    assert_eq!(mgr.get_fade_amount(), FADE_NO_INTENSITY);
    mgr.finish_fade();
    assert_eq!(
        mgr.get_fade_amount(),
        FADE_NO_INTENSITY,
        "finishing twice must not move the level again"
    );
}

#[test]
fn a_fade_in_progress_reports_an_intermediate_level() {
    let mgr = ColorMapManager::new();
    mgr.fade_screen(FadeType::FadeToBlack, 20_000);
    {
        let mut fade = mgr.fade_state.lock().unwrap();
        fade.start_time = Instant::now().checked_sub(SPAN).unwrap();
    }
    let level = mgr.get_fade_amount();
    assert!(
        (126..=128).contains(&level),
        "halfway from 255 to 0 is about 127, got {level}"
    );
}

#[test]
fn a_fade_past_its_interval_reports_the_target_before_it_is_finished() {
    let mgr = ColorMapManager::new();
    mgr.fade_screen(FadeType::FadeToWhite, 1000);
    {
        let mut fade = mgr.fade_state.lock().unwrap();
        fade.start_time = Instant::now().checked_sub(Duration::from_secs(5)).unwrap();
    }
    assert_eq!(mgr.get_fade_amount(), FADE_FULL_INTENSITY);
}

#[test]
fn starting_a_fade_finishes_the_previous_one_first() {
    let mgr = ColorMapManager::new();
    mgr.fade_screen(FadeType::FadeToBlack, 60_000);
    mgr.fade_screen(FadeType::FadeToWhite, 60_000);
    let fade = mgr.fade_state.lock().unwrap();
    assert_eq!(fade.current, FADE_NO_INTENSITY, "the black fade completed");
    assert_eq!(fade.delta, FADE_FULL_INTENSITY - FADE_NO_INTENSITY);
    assert!(fade.active);
}

#[test]
fn colors_expose_their_channels_and_alpha_state() {
    let color = Color::with_alpha(1, 2, 3, 0);
    assert_eq!(color.to_rgba(), (1, 2, 3, 0));
    assert!(color.is_transparent());
    assert!(!color.is_opaque());
}

#[test]
fn palette_color_slice_accessors_share_storage() {
    let mut palette = NativePalette::default();
    palette.colors_mut()[9] = Color::new(9, 9, 9);
    assert_eq!(palette.colors()[9], Color::new(9, 9, 9));
    assert_eq!(palette.get(9), Color::new(9, 9, 9));
}

#[test]
fn set_color_changes_one_entry_and_bumps_the_version() {
    let map = ColorMapInner::new(0);
    let version = map.version();
    map.set_color(5, Color::new(5, 6, 7));
    assert_eq!(map.get_color(5), Color::new(5, 6, 7));
    assert_eq!(map.get_color(6), Color::new(0, 0, 0));
    assert_eq!(map.version(), version + 1);
}

#[test]
fn colormap_colors_helper_returns_the_whole_palette() {
    let mgr = manager_with_black_map(0);
    let map = mgr.get_colormap(0).unwrap();
    assert_eq!(
        ColorMapManager::get_colormap_colors(&map).len(),
        NUMBER_OF_PLUTVALS
    );
}
