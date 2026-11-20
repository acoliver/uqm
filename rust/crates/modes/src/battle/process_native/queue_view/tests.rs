use super::*;
use crate::graphics::geometry::Point;

fn view(step: bool) -> ViewConfig {
    ViewConfig {
        limit: if step {
            MAX_VIS_REDUCTION as i16
        } else {
            MAX_ZOOM_OUT as i16
        },
        zoom: if step { 3 } else { 1280 },
        step,
        activity: IN_ENCOUNTER as u8,
        dimensions: (8192, 7680),
    }
}

#[test]
fn empty_queue_retains_valid_zoom_and_resets_out_of_range_zoom() {
    for (step, zoom, expected) in [
        (true, 3, 0),
        (true, 2, 2),
        (false, 1280, 256),
        (false, 512, 512),
    ] {
        let config = view(step);
        let mut bounds = CameraBounds::new(step, config.dimensions);
        assert_eq!(bounds.finish(config.limit, zoom, step), expected);
        assert_eq!(bounds.ships_alive, 0);
        assert_eq!(bounds.origin, Point { x: 4096, y: 3840 });
    }
}

#[test]
fn queue_camera_aligns_and_wraps_in_link_order() {
    let config = view(true);
    let mut bounds = CameraBounds::new(true, config.dimensions);
    bounds.include(Point { x: 8191, y: 7679 }, 0, 2, &config);
    assert_eq!(bounds.origin, Point { x: 8188, y: 7676 });
    bounds.include(Point { x: 7, y: 11 }, 1, 2, &config);
    assert_eq!(bounds.origin, Point { x: 8192, y: 7680 });
    assert_eq!(bounds.ships_alive, 2);
    assert_eq!(bounds.finish(config.limit, config.zoom, config.step), 0);
}

#[test]
fn camera_reads_configuration_supplied_after_each_hook() {
    let mut config = view(false);
    let mut bounds = CameraBounds::new(false, config.dimensions);
    bounds.include(Point { x: 0, y: 0 }, 0, 2, &config);
    config.activity = IN_LAST_BATTLE as u8;
    bounds.include(Point { x: 0, y: 0 }, 1, 2, &config);
    assert_eq!(bounds.finish(config.limit, config.zoom, config.step), 512);
}

#[test]
fn projected_line_wraps_origin_once_and_narrows_displacement_before_scaling() {
    let projection = Projection {
        dimensions: (8192, 7680),
        origin: Point { x: 0, y: 0 },
        reduction: 2,
        step: true,
    };
    let (next, first, second) =
        projection.line(Point { x: 8190, y: 7679 }, Point { x: 5, y: 2 }, (4, 4));
    assert_eq!(first, Point { x: 0, y: 0 });
    assert_eq!(next, Point { x: -8183, y: -7674 });
    assert_eq!(second, Point { x: -2046, y: -1919 });
    let (next, _, second) =
        projection.line(Point { x: 32760, y: 0 }, Point { x: -32756, y: 0 }, (0, 0));
    assert_eq!(next.x, 24588);
    assert_eq!(second.x, 6147);
}

#[test]
fn point_projection_keeps_continuous_negative_division_and_single_wrap() {
    let projection = Projection {
        dimensions: (100, 100),
        origin: Point { x: 1, y: 1 },
        reduction: 1024,
        step: false,
    };
    let next = projection.scroll(Point { x: 201, y: -201 }, (0, 0));
    assert_eq!(next, Point { x: 101, y: -101 });
    assert_eq!(projection.display(next), Point { x: 25, y: -25 });
    assert_eq!(
        projection.display(Point { x: 0, y: 0 }),
        Point { x: 0, y: 0 }
    );
}
