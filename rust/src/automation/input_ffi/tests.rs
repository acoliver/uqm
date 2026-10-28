use super::*;

#[test]
fn setter_valid_index_returns_zero() {
    assert_eq!(rust_automation_set_immediate_menu_key(6, 1), 0);
}

#[test]
fn setter_invalid_index_returns_negative() {
    assert_eq!(rust_automation_set_immediate_menu_key(24, 1), -1);
    assert_eq!(rust_automation_set_immediate_menu_key(-1, 1), -1);
}

#[test]
fn setter_clear_returns_zero() {
    assert_eq!(rust_automation_set_immediate_menu_key(5, 0), 0);
}

#[test]
fn getter_invalid_index_returns_negative() {
    assert_eq!(rust_automation_get_current_menu_key(24), -1);
    assert_eq!(rust_automation_get_current_menu_key(-1), -1);
    assert_eq!(rust_automation_get_pulsed_menu_key(24), -1);
    assert_eq!(rust_automation_get_pulsed_menu_key(-1), -1);
}

#[test]
fn getter_valid_index_returns_zero() {
    assert_eq!(rust_automation_get_current_menu_key(6), 0);
    assert_eq!(rust_automation_get_pulsed_menu_key(6), 0);
}

#[test]
fn service_inactive_returns_zero() {
    // Without init_automation_runtime, this should return 0 (inactive).
    assert_eq!(rust_automation_service_do_input(), 0);
}

#[test]
fn observation_inactive_returns_zero() {
    assert_eq!(rust_automation_after_input_update(), 0);
}

/// Ground truth captured by compiling the real C macros from `units.h`
/// and `planets.h` against each screen size. The interplanetary view
/// centre is not a constant, so navigation must not assume one.
#[test]
fn view_geometry_matches_the_c_macros_at_every_supported_resolution() {
    let low = ViewGeometry::from_screen(320, 240);
    assert_eq!(low.half_width, 121);
    assert_eq!(low.half_height, 113);
    assert_eq!(low.display_to_loc, 56);

    let high = ViewGeometry::from_screen(640, 480);
    assert_eq!(high.half_width, 281);
    assert_eq!(high.half_height, 233);
    assert_eq!(high.display_to_loc, 136);
}

/// Earth's moon 0 (the Hierarchy Starbase) sits at display (86, 113) in
/// the 320x240 inner view. That y-coordinate is exactly the view centre,
/// which the previously hardcoded 91 would have mismatched by 22 pixels.
#[test]
fn view_geometry_centre_matches_the_observed_inner_system_layout() {
    let geometry = ViewGeometry::from_screen(320, 240);
    assert_eq!(geometry.half_height, 113);
    assert_eq!(geometry.half_width - 35, 86);
}

#[test]
fn planet_target_inside_its_own_inner_system_is_the_view_centre() {
    let geometry = ViewGeometry::from_screen(320, 240);
    // Mars's stale outer-view origin must not be steered at once the ship
    // is inside Mars's inner system, where the planet is at the centre.
    assert_eq!(
        planet_display_target(true, (200, 40), 56, &geometry),
        (121, 113)
    );
}

#[test]
fn planet_target_in_the_outer_system_is_the_rounded_outer_origin() {
    let geometry = ViewGeometry::from_screen(320, 240);
    // (200 - 121) * 112 / 56 = 158; 121 + 158 * 56 / 112 = 200.
    assert_eq!(
        planet_display_target(false, (200, 40), 112, &geometry),
        (200, 40)
    );
    assert_eq!(
        planet_display_target(false, (200, 40), 0, &geometry),
        (0, 0)
    );
}
