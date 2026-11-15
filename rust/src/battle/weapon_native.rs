//! Live laser/missile, collision, silhouette and homing behavior from weapon.c.

use crate::ships::native::abi::*;

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::collections::observation::image;
    use crate::engine::battle::{Cx, ElementId};

    pub fn modify_silhouette(
        cx: &mut Cx<'_>,
        e: ElementId,
        origin: POINT,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    ) {
        let mut object = INTERSECT_CONTROL {
            IntersectStamp: STAMP {
                origin,
                frame: frame.raw(),
            },
            EndPoint: POINT { x: 0, y: 0 },
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
            let mut ship_intersect = INTERSECT_CONTROL {
                IntersectStamp: STAMP {
                    origin: POINT { x: 0, y: 0 },
                    frame: std::ptr::null_mut(),
                },
                EndPoint: POINT { x: 0, y: 0 },
                last_time_val: 0,
            };
            image!(frame; ship_intersect.IntersectStamp.frame, silhouette.raw());
            if silhouette.is_absent() {
                return;
            }
            let rect = cx.frame_rect(silhouette.view());
            loop {
                object.IntersectStamp.origin.x = (i32::from(cx.random() as u16)
                    % (i32::from(rect.extent.width) - i32::from(object_rect.extent.width))
                    + ((i32::from(object_rect.extent.width) - i32::from(rect.extent.width)) >> 1))
                    as i16;
                object.IntersectStamp.origin.y = (i32::from(cx.random() as u16)
                    % (i32::from(rect.extent.height) - i32::from(object_rect.extent.height))
                    + ((i32::from(object_rect.extent.height) - i32::from(rect.extent.height)) >> 1))
                    as i16;
                object.EndPoint = object.IntersectStamp.origin;
                if cx.projected_intersect(&mut object, &mut ship_intersect, MAX_TIME_VALUE as u16)
                    != 0
                {
                    break;
                }
            }
            object.IntersectStamp.origin.x = object.IntersectStamp.origin.x.wrapping_add(64 >> 1);
            object.IntersectStamp.origin.y = object.IntersectStamp.origin.y.wrapping_add(31);
        }
        object.IntersectStamp.origin.y = object
            .IntersectStamp
            .origin
            .y
            .wrapping_add(cx.kernel.status_offsets[cx.element(e).player as usize]);
        if flags & MODIFY_SWAP as u8 != 0 {
            object_rect.corner.x = object_rect
                .corner
                .x
                .wrapping_add(object.IntersectStamp.origin.x);
            object_rect.corner.y = object_rect
                .corner
                .y
                .wrapping_add(object.IntersectStamp.origin.y);
            cx.init_ship_status(ship, Some(object_rect));
        } else {
            cx.draw_stamp(frame.view(), object.IntersectStamp.origin, true);
        }
    }
}

#[cfg(not(test))]
pub use engine::*;
