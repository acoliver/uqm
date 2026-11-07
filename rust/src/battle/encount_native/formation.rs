//! Authored encounter icon rings in display coordinates.
use super::ENCOUNTER_DISPLAY_POINTS;
use crate::graphics::native_abi::{EXTENT, POINT};
use crate::math::{arctan, cosine, sine, square_root};

pub(super) fn encounter_icon(
    icon: crate::engine::battle::views::FrameRef<'_>,
) -> Option<crate::graphics::context_abi::ContextFrame> {
    // The noncombat Ur-Quan drone deliberately has no encounter icon.
    let frame = std::ptr::NonNull::new(icon.raw())?;
    Some(crate::graphics::context_abi::ContextFrame::retain(Some(frame)).indexed_frame(0))
}

pub(super) fn encounter_icon_origin(index: usize, bounds: EXTENT, dimensions: (i32, i32)) -> POINT {
    let (mut x, mut y) = ENCOUNTER_DISPLAY_POINTS[index % ENCOUNTER_DISPLAY_POINTS.len()];
    if index >= ENCOUNTER_DISPLAY_POINTS.len() {
        let radius = (square_root((i64::from(x).pow(2) + i64::from(y).pow(2)) as u32) as usize
            + (index / ENCOUNTER_DISPLAY_POINTS.len()) * 18) as u16;
        let angle = arctan(x, y);
        x = cosine(i32::from(angle), i32::from(radius));
        y = sine(i32::from(angle), i32::from(radius));
    }
    POINT {
        x: (i32::from(x) + (dimensions.0 >> 1) - (i32::from(bounds.width) >> 1)) as i16,
        y: (i32::from(y) + (dimensions.1 >> 1) - (i32::from(bounds.height) >> 1)) as i16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn noncombat_drone_has_no_encounter_icon() {
        let icon = crate::engine::battle::frames::FrameSelection::retain(std::ptr::null_mut());
        assert!(encounter_icon(icon.view()).is_none());
    }

    #[test]
    fn formation_centers_icons_without_changing_the_authored_first_ring() {
        let bounds = EXTENT {
            width: 8,
            height: 12,
        };
        let first = encounter_icon_origin(0, bounds, (242, 227));
        assert_eq!((first.x, first.y), (127, 158));
        let last = encounter_icon_origin(9, bounds, (242, 227));
        assert_eq!((last.x, last.y), (73, 80));
        let outer = encounter_icon_origin(10, bounds, (242, 227));
        assert!(outer.y > first.y);
        assert!(outer.x >= first.x);
    }
}
