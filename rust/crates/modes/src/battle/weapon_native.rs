//! Battle-mode silhouette changes for the supplied status display.

use crate::engine::battle::flags::MAX_TIME_VALUE;
use crate::graphics::geometry::Point;
use crate::graphics::intersection::Stamp;
const MODIFY_IMAGE: u32 = 1;
const MODIFY_SWAP: u32 = 2;

pub(crate) mod engine {
    use super::*;
    use crate::collections::observation::image;
    use crate::engine::battle::{Cx, ElementId};

    pub fn modify_silhouette(
        cx: &mut Cx<'_>,
        e: ElementId,
        origin: Point,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    ) {
        let mut object = crate::graphics::intersection::IntersectControl {
            intersect_stamp: Stamp {
                origin,
                frame: frame.raw(),
            },
            end_point: Point { x: 0, y: 0 },
            last_time_val: 0,
        };
        let mut object_rect = cx.frame_rect(frame.view());
        let ship = cx.owner(e);
        if flags & MODIFY_IMAGE as u8 != 0 {
            let silhouette = cx.absolute_frame(
                crate::engine::battle::views::FrameSelection::retain(cx.desc(ship).ship_info.icons)
                    .view(),
                1,
            );
            let mut ship_intersect = crate::graphics::intersection::IntersectControl {
                intersect_stamp: Stamp {
                    origin: Point { x: 0, y: 0 },
                    frame: crate::graphics::drawable::FrameId::NULL,
                },
                end_point: Point { x: 0, y: 0 },
                last_time_val: 0,
            };
            image!(frame; ship_intersect.intersect_stamp.frame, silhouette.raw());
            if silhouette.is_absent() {
                return;
            }
            let rect = cx.frame_rect(silhouette.view());
            loop {
                object.intersect_stamp.origin.x = (i32::from(cx.random() as u16)
                    % (i32::from(rect.extent.width) - i32::from(object_rect.extent.width))
                    + ((i32::from(object_rect.extent.width) - i32::from(rect.extent.width)) >> 1))
                    as i16;
                object.intersect_stamp.origin.y = (i32::from(cx.random() as u16)
                    % (i32::from(rect.extent.height) - i32::from(object_rect.extent.height))
                    + ((i32::from(object_rect.extent.height) - i32::from(rect.extent.height)) >> 1))
                    as i16;
                object.end_point = object.intersect_stamp.origin;
                if cx.projected_intersect(&mut object, &mut ship_intersect, MAX_TIME_VALUE as u16)
                    != 0
                {
                    break;
                }
            }
            object.intersect_stamp.origin.x = object.intersect_stamp.origin.x.wrapping_add(64 >> 1);
            object.intersect_stamp.origin.y = object.intersect_stamp.origin.y.wrapping_add(31);
        }
        object.intersect_stamp.origin.y = object
            .intersect_stamp
            .origin
            .y
            .wrapping_add(cx.kernel.status_offsets[cx.element(e).player as usize]);
        if flags & MODIFY_SWAP as u8 != 0 {
            object_rect.corner.x = object_rect
                .corner
                .x
                .wrapping_add(object.intersect_stamp.origin.x);
            object_rect.corner.y = object_rect
                .corner
                .y
                .wrapping_add(object.intersect_stamp.origin.y);
            cx.init_ship_status(ship, Some(object_rect));
        } else {
            cx.draw_stamp(frame.view(), object.intersect_stamp.origin, true);
        }
    }
}

pub use engine::*;
