//! Integer coordinate transforms used by interplanetary flight.
use crate::c_bindings::global_state::POINT;

pub(crate) fn to_display(point: POINT, radius: i16, width: i32, height: i32) -> POINT {
    let scale = ((width >> 1) - 8) >> 1;
    POINT {
        x: (i64::from(width >> 1) + i64::from(point.x) * i64::from(scale) / i64::from(radius))
            as i16,
        y: (i64::from(height >> 1) + i64::from(point.y) * i64::from(scale) / i64::from(radius))
            as i16,
    }
}

pub(crate) fn to_location(point: POINT, radius: i16, width: i32, height: i32) -> POINT {
    let scale = ((width >> 1) - 8) >> 1;
    POINT {
        x: ((i64::from(point.x) - i64::from(width >> 1)) * i64::from(radius) / i64::from(scale))
            as i16,
        y: ((i64::from(point.y) - i64::from(height >> 1)) * i64::from(radius) / i64::from(scale))
            as i16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_and_cardinals_keep_the_original_truncation() {
        let center = to_display(POINT { x: 0, y: 0 }, 8192, 242, 187);
        assert_eq!((center.x, center.y), (121, 93));
        let negative = to_display(POINT { x: -8192, y: -1 }, 8192, 242, 187);
        assert_eq!((negative.x, negative.y), (65, 93));
        let corner = to_location(POINT { x: 0, y: 0 }, 8192, 242, 187);
        assert_eq!((corner.x, corner.y), (-17700, -13604));
    }

    #[test]
    fn exact_scale_steps_round_trip_at_every_zoom() {
        for (width, height) in [(242, 187), (562, 347), (882, 587)] {
            let scale = ((width >> 1) - 8) >> 1;
            for radius in [1024, 2048, 4096, 8192] {
                for x in -3..=3 {
                    for y in -3..=3 {
                        let point = POINT {
                            x: (x * radius) as i16,
                            y: (y * radius) as i16,
                        };
                        let display = to_display(point, radius as i16, width, height);
                        assert_eq!(
                            (i32::from(display.x), i32::from(display.y)),
                            ((width >> 1) + x * scale, (height >> 1) + y * scale)
                        );
                        let restored = to_location(display, radius as i16, width, height);
                        assert_eq!((restored.x, restored.y), (point.x, point.y));
                    }
                }
            }
        }
    }
}
