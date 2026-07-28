//! Production surface drawing adapter over low-level graphics services.

use std::collections::HashMap;
use std::ffi::c_void;

use super::assembly::SharedSurface;
use super::entities::SurfaceEntityId;
#[cfg(feature = "linked_c_archive")]
use super::resources::LanderGraphic;
use super::resources::PlanetSideAssetAccess;
use super::runtime::{AdapterError, PlanetSideGraphics, RenderSnapshot};
use super::selection::{LANDING_CURSOR_STEP, MAP_WIDTH};

const SURFACE_WIDTH: i32 = 242;
const SURFACE_HEIGHT: i32 = 162;
const LANDER_SCREEN_X: i32 = SURFACE_WIDTH / 2;
const LANDER_SCREEN_Y: i32 = SURFACE_HEIGHT / 2;

/// Drawable frame assigned to a Rust surface entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceFrame {
    pub base: *mut c_void,
    pub index: u16,
}

/// Frame assignments remain outside deterministic entity state.
#[derive(Default)]
pub struct SurfaceFrameRegistry {
    frames: HashMap<SurfaceEntityId, SurfaceFrame>,
}

impl SurfaceFrameRegistry {
    pub fn insert(&mut self, entity: SurfaceEntityId, frame: SurfaceFrame) {
        self.frames.insert(entity, frame);
    }

    pub fn remove(&mut self, entity: SurfaceEntityId) {
        self.frames.remove(&entity);
    }

    #[must_use]
    pub fn get(&self, entity: SurfaceEntityId) -> Option<SurfaceFrame> {
        self.frames.get(&entity).copied()
    }
}

/// Concrete graphics adapter. It renders the Rust world in insertion order and
/// draws the lander at the center of the PlanetContext viewport.
pub struct CffiPlanetSideGraphics<'a, A: PlanetSideAssetAccess> {
    pub surface: SharedSurface,
    pub assets: &'a A,
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct CPoint {
    x: i16,
    y: i16,
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct CStamp {
    origin: CPoint,
    frame: *mut c_void,
}
#[cfg(feature = "linked_c_archive")]
extern "C" {
    static mut PlanetContext: *mut c_void;
    #[link_name = "pSolarSysState"]
    static mut P_SOLAR_SYSTEM_STATE: *mut u8;
    fn SetContext(context: *mut c_void) -> *mut c_void;
    fn BatchGraphics();
    fn UnbatchGraphics();
    fn ClearDrawable();
    fn DrawStamp(stamp: *mut CStamp);
    fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
    fn TFB_FlushGraphicsEx(skip_swap: i32);
    fn TFB_SwapBuffers(force_full_redraw: i32);
}

impl<A: PlanetSideAssetAccess> PlanetSideGraphics for CffiPlanetSideGraphics<'_, A> {
    fn render(&mut self, snapshot: &RenderSnapshot) -> Result<(), AdapterError> {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            if PlanetContext.is_null() {
                return Err(AdapterError::new("planet_context_not_initialized"));
            }
            let old_context = SetContext(PlanetContext);
            BatchGraphics();

            let topo_frame = surface_topography_frame()?;
            draw_surface_tiles(topo_frame, snapshot.lander.position);

            {
                let surface = self.surface.borrow();
                for (id, entity) in surface.world.iter() {
                    let Some(frame) = surface.frames.get(id) else {
                        continue;
                    };
                    let Some((x, y)) =
                        project_surface_point(entity.position, snapshot.lander.position)
                    else {
                        continue;
                    };
                    let mut stamp = CStamp {
                        origin: CPoint { x, y },
                        frame: SetAbsFrameIndex(frame.base, frame.index),
                    };
                    DrawStamp(&mut stamp);
                }
            }

            let lander_base = self.assets.graphic(LanderGraphic::Lander);
            if lander_base.is_null() {
                UnbatchGraphics();
                SetContext(old_context);
                return Err(AdapterError::new("lander_frame_not_loaded"));
            }
            let mut lander = CStamp {
                origin: CPoint {
                    x: LANDER_SCREEN_X as i16,
                    y: LANDER_SCREEN_Y as i16,
                },
                frame: SetAbsFrameIndex(lander_base, u16::from(snapshot.lander.facing)),
            };
            DrawStamp(&mut lander);
            UnbatchGraphics();
            SetContext(old_context);
            TFB_FlushGraphicsEx(1);
            TFB_SwapBuffers(1);
        }
        #[cfg(not(feature = "linked_c_archive"))]
        let _ = snapshot;
        Ok(())
    }
}

#[cfg(feature = "linked_c_archive")]
unsafe fn surface_topography_frame() -> Result<*mut c_void, AdapterError> {
    if P_SOLAR_SYSTEM_STATE.is_null() {
        return Err(AdapterError::new("solar_system_not_initialized"));
    }
    let frame = std::ptr::read_unaligned(P_SOLAR_SYSTEM_STATE.add(1224).cast::<*mut c_void>());
    if frame.is_null() {
        Err(AdapterError::new("surface_topography_not_loaded"))
    } else {
        Ok(frame)
    }
}

#[cfg(feature = "linked_c_archive")]
unsafe fn draw_surface_tiles(frame: *mut c_void, camera: super::model::SurfacePoint) {
    ClearDrawable();
    let mut stamp = CStamp {
        origin: CPoint {
            x: (-camera.x + SURFACE_WIDTH / 2) as i16,
            y: (-camera.y + SURFACE_HEIGHT / 2) as i16,
        },
        frame,
    };
    DrawStamp(&mut stamp);
    stamp.origin.x = stamp
        .origin
        .x
        .wrapping_add((MAP_WIDTH * LANDING_CURSOR_STEP) as i16);
    DrawStamp(&mut stamp);
    stamp.origin.x = stamp
        .origin
        .x
        .wrapping_sub((MAP_WIDTH * LANDING_CURSOR_STEP * 2) as i16);
    DrawStamp(&mut stamp);
}

/// Convert fixed-point world coordinates to a viewport coordinate, choosing
/// the shortest horizontal route across the wrapping surface.
fn project_surface_point(
    point: super::model::SurfacePoint,
    camera: super::model::SurfacePoint,
) -> Option<(i16, i16)> {
    let world_width = MAP_WIDTH * LANDING_CURSOR_STEP;
    let mut dx = (point.x - camera.x).rem_euclid(world_width);
    if dx > world_width / 2 {
        dx -= world_width;
    }
    let screen_x = LANDER_SCREEN_X + dx;
    let screen_y = LANDER_SCREEN_Y + point.y - camera.y;
    if !(i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&screen_x)
        || !(i32::from(i16::MIN)..=i32::from(i16::MAX)).contains(&screen_y)
    {
        return None;
    }
    Some((screen_x as i16, screen_y as i16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::model::SurfacePoint;

    #[test]
    fn camera_position_projects_to_lander_center() {
        let point = SurfacePoint { x: 400, y: 200 };
        assert_eq!(
            project_surface_point(point, point),
            Some((LANDER_SCREEN_X as i16, LANDER_SCREEN_Y as i16))
        );
    }

    #[test]
    fn fixed_point_offsets_remain_in_surface_render_units() {
        assert_eq!(
            project_surface_point(
                SurfacePoint { x: 408, y: 196 },
                SurfacePoint { x: 400, y: 200 }
            ),
            Some(((LANDER_SCREEN_X + 8) as i16, (LANDER_SCREEN_Y - 4) as i16))
        );
    }

    #[test]
    fn horizontal_projection_uses_wrapped_shortest_distance() {
        let world_width = MAP_WIDTH * LANDING_CURSOR_STEP;
        assert_eq!(
            project_surface_point(
                SurfacePoint {
                    x: world_width - 4,
                    y: 0
                },
                SurfacePoint { x: 0, y: 0 }
            ),
            Some(((LANDER_SCREEN_X - 4) as i16, LANDER_SCREEN_Y as i16))
        );
    }
}
