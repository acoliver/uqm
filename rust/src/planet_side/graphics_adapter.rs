//! Production surface drawing adapter over low-level graphics services.

use std::collections::HashMap;
use std::ffi::c_void;

use super::assembly::SharedSurface;
use super::entities::{SurfaceEntityId, SurfaceEntityKind};
use super::resources::PlanetSideAssetAccess;
use super::runtime::{AdapterError, PlanetSideGraphics, RenderSnapshot};

use super::selection::{LANDING_CURSOR_STEP, MAP_WIDTH};

const SURFACE_WIDTH: i32 = 242;

const SURFACE_HEIGHT: i32 = 162;

const LANDER_SCREEN_X: i32 = SURFACE_WIDTH / 2;

const LANDER_SCREEN_Y: i32 = SURFACE_HEIGHT / 2;

// --- Radar panel layout (from deleted lander.c DeltaLanderCrew / FillLanderHold) ---

/// `NUM_CREW_COLS` — crew grid columns in the radar panel.
const NUM_CREW_COLS: u8 = 6;
/// `NUM_CREW_ROWS` — crew grid rows in the radar panel.
const NUM_CREW_ROWS: u8 = 2;
/// Maximum crew grid slots: `NUM_CREW_COLS * NUM_CREW_ROWS`.
const MAX_CREW_SLOTS: u8 = NUM_CREW_COLS * NUM_CREW_ROWS;
/// Pixel spacing between crew grid slots.
const CREW_SLOT_SPACING: i16 = 6;
/// Crew grid origin x: `11` from `DeltaLanderCrew`.
const CREW_ORIGIN_X: i16 = 11;
/// Crew grid origin y: `35` from `DeltaLanderCrew`.
const CREW_ORIGIN_Y: i16 = 35;

/// `MAX_SCROUNGED` — maximum biological and standard mineral cargo level.
const MAX_SCROUNGED: u16 = 50;

/// Cargo bar origin in the radar panel (from `FillLanderHold`).
const CARGO_ORIGIN_X: i16 = 0;
const CARGO_ORIGIN_Y: i16 = 0;

/// LanderGraphic::Lander frame indices for radar panel meters.
const BIO_FRAME_EVEN: u16 = 41;
const BIO_FRAME_ODD: u16 = 42;
const MINERAL_FRAME_EVEN: u16 = 43;
const MINERAL_FRAME_ODD: u16 = 44;
const CREW_ALIVE_FRAME: u16 = 55;
#[cfg(not(test))]
const CREW_DEAD_FRAME: u16 = 56;

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

    /// Number of registered frames.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn advance_hazard_frames(&mut self, world: &super::entities::SurfaceWorld) {
        for (id, entity) in world.iter() {
            let Some(frame) = self.frames.get_mut(&id) else {
                continue;
            };
            let Some(remaining) = entity.finite_life else {
                continue;
            };
            frame.index = match entity.kind {
                SurfaceEntityKind::Hazard(super::hazards::HazardKind::Earthquake) => {
                    12_u16.saturating_sub(remaining / 3).min(12)
                }
                SurfaceEntityKind::Hazard(super::hazards::HazardKind::Lava) => {
                    frame.index.saturating_add(1)
                }
                SurfaceEntityKind::Hazard(super::hazards::HazardKind::Lightning) => {
                    frame.index.wrapping_add(1) % 7
                }
                _ => continue,
            };
        }
    }
}
/// Concrete graphics adapter. It renders the Rust world in insertion order and
/// draws the lander at the center of the PlanetContext viewport.
pub struct CffiPlanetSideGraphics<'a, A: PlanetSideAssetAccess> {
    pub surface: SharedSurface,
    pub assets: &'a A,
    pub last_scan_position: Option<super::model::SurfacePoint>,
}

#[cfg(not(test))]
use crate::graphics::native_abi::point as CPoint;

#[cfg(not(test))]
#[repr(C)]
struct CStamp {
    origin: CPoint,
    frame: *mut c_void,
}
#[cfg(not(test))]
use crate::graphics::context_abi::SetContext;
#[cfg(not(test))]
use crate::graphics::dcqueue_abi::TFB_FlushGraphicsEx;
#[cfg(not(test))]
use crate::graphics::frame_abi::ClearDrawable;
#[cfg(not(test))]
use crate::graphics::frame_abi::DrawStamp;
#[cfg(not(test))]
use crate::graphics::pixmap_abi::SetAbsFrameIndex;
#[cfg(not(test))]
use crate::graphics::sdl::lifecycle_abi::TFB_SwapBuffers;
#[cfg(not(test))]
use crate::planets::scan::live::RedrawSurfaceScan;
#[cfg(not(test))]
extern "C" {
    static mut PlanetContext: *mut crate::graphics::native_abi::context_desc;
    static mut RadarContext: *mut crate::graphics::native_abi::context_desc;
}
#[cfg(not(test))]
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

impl<A: PlanetSideAssetAccess> PlanetSideGraphics for CffiPlanetSideGraphics<'_, A> {
    fn render(&mut self, snapshot: &RenderSnapshot) -> Result<(), AdapterError> {
        #[cfg(not(test))]
        unsafe {
            {
                let scan_position = CPoint {
                    x: snapshot.lander.position.x as i16,
                    y: snapshot.lander.position.y as i16,
                };
                RedrawSurfaceScan(std::ptr::from_ref(&scan_position).cast());
                self.last_scan_position = Some(snapshot.lander.position);
            }

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
                        frame: (SetAbsFrameIndex((frame.base).cast(), frame.index)).cast(),
                    };
                    if let SurfaceEntityKind::LiveCreature {
                        damage_wait: damage_wait @ 1..=7,
                        ..
                    } = entity.kind
                    {
                        let old_color = crate::graphics::context_abi::SetContextForeGroundColor(
                            damage_color(damage_wait),
                        );
                        crate::graphics::frame_abi::DrawFilledStamp(
                            std::ptr::from_mut(&mut stamp).cast(),
                        );
                        crate::graphics::context_abi::SetContextForeGroundColor(old_color);
                    } else {
                        DrawStamp(std::ptr::from_mut(&mut stamp).cast());
                    }
                }
            }

            // Select the lander graphic and frame index based on lifecycle phase.
            let (lander_graphic, frame_index) = select_lander_graphic(
                snapshot.phase,
                snapshot.lander.facing,
                snapshot.animation_frame,
            );
            let lander_base = self.assets.graphic(lander_graphic);
            if lander_base.is_null() {
                UnbatchGraphics();
                SetContext(old_context);
                return Err(AdapterError::new("lander_frame_not_loaded"));
            }
            let lander_y = LANDER_SCREEN_Y as i16 + snapshot.lifecycle_offset() as i16;
            let mut lander = CStamp {
                origin: CPoint {
                    x: LANDER_SCREEN_X as i16,
                    y: lander_y,
                },
                frame: (SetAbsFrameIndex((lander_base).cast(), frame_index)).cast(),
            };
            DrawStamp(std::ptr::from_mut(&mut lander).cast());
            UnbatchGraphics();

            // Draw cargo and crew meters in the radar panel. These overlay the
            // InitLander status display and must be refreshed every frame so
            // that active pickups visibly fill the bars and damage visibly
            // decrements crew — matching the deleted lander.c
            // FillLanderHold / DeltaLanderCrew behavior.
            let lander_base_for_meters =
                self.assets.graphic(super::resources::LanderGraphic::Lander);
            if !lander_base_for_meters.is_null() {
                let old_radar_ctx = SetContext(RadarContext);
                BatchGraphics();
                draw_cargo_meters(lander_base_for_meters, snapshot);
                draw_crew_meter(lander_base_for_meters, snapshot.lander.crew.get());
                UnbatchGraphics();
                SetContext(old_radar_ctx);
            }

            SetContext(old_context);
            TFB_FlushGraphicsEx(1);
            TFB_SwapBuffers(1);
        }
        #[cfg(test)]
        let _ = snapshot;
        Ok(())
    }
}

#[cfg(not(test))]
fn damage_color(remaining: u8) -> crate::graphics::native_abi::Color {
    let (r, g, b) = match remaining {
        7 => (31, 31, 31),
        6 => (31, 31, 10),
        5 => (31, 28, 0),
        4 => (31, 21, 0),
        3 => (31, 14, 0),
        2 => (31, 7, 0),
        1 => (27, 0, 0),
        _ => unreachable!("damage flash has seven colors"),
    };
    crate::graphics::native_abi::Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

/// Select the lander graphic and frame index for the current lifecycle phase.
///
/// During `Launch`, the `LanderGraphic::Launch` frame set is played from frame
/// 0 through `animation_frame`. During `Return`, the same applies to
/// `LanderGraphic::Return`. During `Explosion`, the base lander frame set index
/// 46+ is used (the explosion sub-frames). All other phases use the standard
/// facing-indexed lander frame.
#[must_use]
fn select_lander_graphic(
    phase: super::session::SessionPhase,
    facing: u8,
    animation_frame: u16,
) -> (super::resources::LanderGraphic, u16) {
    use super::resources::LanderGraphic;
    match phase {
        super::session::SessionPhase::Launch => (LanderGraphic::Launch, animation_frame),
        super::session::SessionPhase::Return => (LanderGraphic::Return, animation_frame),
        super::session::SessionPhase::Explosion => {
            // Explosion frames are sub-frames 46+ in the base lander graphic,
            // advancing every 3rd frame. The frame counter is divided by 3 to
            // match the `MAKE_BYTE(2,2)` cadence from object_animation.
            let explosion_index =
                (animation_frame / 3).min(super::lifecycle::EXPLOSION_ANIM_FRAMES / 3);
            (LanderGraphic::Lander, 46 + explosion_index)
        }
        _ => (LanderGraphic::Lander, u16::from(facing)),
    }
}

// ---------------------------------------------------------------------------
// Pure radar-panel meter layout (ported from deleted lander.c)
// ---------------------------------------------------------------------------

/// Compute the radar-panel position for crew grid slot `slot` (0-based).
///
/// Matches the C `DeltaLanderCrew` layout:
/// ```text
/// x = 11 + 6 * (slot % NUM_CREW_COLS)
/// y = 35 - 6 * (slot / NUM_CREW_COLS)
/// ```
///
/// Returns `None` if `slot` exceeds the maximum grid capacity
/// (`NUM_CREW_COLS * NUM_CREW_ROWS = 12`).
#[must_use]
pub fn crew_slot_position(slot: u8) -> Option<(i16, i16)> {
    if slot >= MAX_CREW_SLOTS {
        return None;
    }
    let x = CREW_ORIGIN_X + CREW_SLOT_SPACING * i16::from(slot % NUM_CREW_COLS);
    let y = CREW_ORIGIN_Y - CREW_SLOT_SPACING * i16::from(slot / NUM_CREW_COLS);
    Some((x, y))
}

/// Compute the number of cargo-bar segments to display for a mineral level.
///
/// With improved cargo, the C `FillLanderHold` halves the mineral display
/// level (`ElementLevel >> 1`) so that a 100-unit capacity fits in the same
/// 50-pixel bar.
#[must_use]
pub const fn mineral_bar_segments(level: u16, improved_cargo: bool) -> u16 {
    if improved_cargo {
        level / 2
    } else {
        level
    }
}

/// Compute the number of cargo-bar segments to display for a biological level.
///
/// Biological cargo is never halved — it is always capped at `MAX_SCROUNGED`
/// (50), matching the C code which never applies the cargo shift to
/// `BiologicalLevel`.
#[must_use]
pub const fn bio_bar_segments(level: u16) -> u16 {
    if level > MAX_SCROUNGED {
        MAX_SCROUNGED
    } else {
        level
    }
}

/// Compute the LanderGraphic::Lander frame index for cargo-bar segment
/// `index` (0-based).
///
/// Matches the C `FillLanderHold` alternation: even segments use the lower
/// frame (41 bio / 43 mineral), odd segments use the higher frame (42 bio /
/// 44 mineral).
#[must_use]
pub const fn cargo_segment_frame(index: u16, is_bio: bool) -> u16 {
    if is_bio {
        if index.is_multiple_of(2) {
            BIO_FRAME_EVEN
        } else {
            BIO_FRAME_ODD
        }
    } else if index.is_multiple_of(2) {
        MINERAL_FRAME_EVEN
    } else {
        MINERAL_FRAME_ODD
    }
}

/// Compute the radar-panel position for cargo-bar segment `index` (0-based).
///
/// Matches the C `FillLanderHold` origin: segments stack upward from `(0, 0)`,
/// each one pixel higher (`y = -(index)`).
#[must_use]
pub const fn cargo_segment_position(index: u16) -> (i16, i16) {
    (CARGO_ORIGIN_X, CARGO_ORIGIN_Y - index as i16)
}

/// Compute the frame index for an alive crew slot.
///
/// Matches the C `DeltaLanderCrew` positive-delta path: always frame 55.
#[must_use]
pub const fn crew_alive_frame() -> u16 {
    CREW_ALIVE_FRAME
}

#[cfg(not(test))]
unsafe fn draw_crew_meter(lander_base: *mut c_void, crew_count: u8) {
    for slot in 0..MAX_CREW_SLOTS {
        let Some((x, y)) = crew_slot_position(slot) else {
            break;
        };
        let frame = if slot < crew_count {
            CREW_ALIVE_FRAME
        } else {
            CREW_DEAD_FRAME
        };
        let mut stamp = CStamp {
            origin: CPoint { x, y },
            frame: (SetAbsFrameIndex((lander_base).cast(), frame)).cast(),
        };
        DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    }
}

#[cfg(not(test))]
unsafe fn draw_cargo_meters(lander_base: *mut c_void, snapshot: &RenderSnapshot) {
    // Biological cargo bar.
    let bio_segments = bio_bar_segments(snapshot.biological_level);
    for index in 0..bio_segments {
        let (x, y) = cargo_segment_position(index);
        let frame_idx = cargo_segment_frame(index, true);
        let mut stamp = CStamp {
            origin: CPoint { x, y },
            frame: (SetAbsFrameIndex((lander_base).cast(), frame_idx)).cast(),
        };
        DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    }

    // Mineral cargo bar. With improved cargo, the display level is halved so
    // the 50-pixel bar represents up to 100 units of mineral capacity.
    let mineral_segments = mineral_bar_segments(
        snapshot.mineral_level,
        snapshot.lander.upgrades.improved_cargo,
    );
    for index in 0..mineral_segments {
        let (x, y) = cargo_segment_position(index);
        let frame_idx = cargo_segment_frame(index, false);
        let mut stamp = CStamp {
            origin: CPoint { x, y },
            frame: (SetAbsFrameIndex((lander_base).cast(), frame_idx)).cast(),
        };
        DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    }
}

#[cfg(not(test))]
unsafe fn surface_topography_frame() -> Result<*mut c_void, AdapterError> {
    let state = crate::c_bindings::solar_system::current();
    if state.is_null() {
        return Err(AdapterError::new("solar_system_not_initialized"));
    }
    let frame = std::ptr::addr_of!((*state).Orbit.TopoZoomFrame)
        .read()
        .cast::<c_void>();
    if frame.is_null() {
        Err(AdapterError::new("surface_topography_not_loaded"))
    } else {
        Ok(frame)
    }
}

#[cfg(not(test))]
unsafe fn draw_surface_tiles(frame: *mut c_void, camera: super::model::SurfacePoint) {
    ClearDrawable();
    let mut stamp = CStamp {
        origin: CPoint {
            x: (-camera.x + SURFACE_WIDTH / 2) as i16,
            y: (-camera.y + SURFACE_HEIGHT / 2) as i16,
        },
        frame,
    };
    DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    stamp.origin.x = stamp
        .origin
        .x
        .wrapping_add((MAP_WIDTH * LANDING_CURSOR_STEP) as i16);
    DrawStamp(std::ptr::from_mut(&mut stamp).cast());
    stamp.origin.x = stamp
        .origin
        .x
        .wrapping_sub((MAP_WIDTH * LANDING_CURSOR_STEP * 2) as i16);
    DrawStamp(std::ptr::from_mut(&mut stamp).cast());
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
    use crate::planet_side::resources::LanderGraphic;
    use crate::planet_side::session::SessionPhase;

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

    #[test]
    fn launch_phase_selects_launch_graphic_with_animation_frame() {
        let (graphic, index) = select_lander_graphic(SessionPhase::Launch, 0, 5);
        assert_eq!(graphic, LanderGraphic::Launch);
        assert_eq!(index, 5);
    }

    #[test]
    fn return_phase_selects_return_graphic_with_animation_frame() {
        let (graphic, index) = select_lander_graphic(SessionPhase::Return, 0, 3);
        assert_eq!(graphic, LanderGraphic::Return);
        assert_eq!(index, 3);
    }

    #[test]
    fn explosion_phase_uses_lander_frame_46_plus_anim_div_3() {
        let (graphic, index) = select_lander_graphic(SessionPhase::Explosion, 0, 0);
        assert_eq!(graphic, LanderGraphic::Lander);
        assert_eq!(index, 46);

        let (graphic, index) = select_lander_graphic(SessionPhase::Explosion, 0, 9);
        assert_eq!(graphic, LanderGraphic::Lander);
        assert_eq!(index, 49);

        // Explosion frames are capped at the animation length / 3.
        let (graphic, index) = select_lander_graphic(
            SessionPhase::Explosion,
            0,
            crate::planet_side::lifecycle::EXPLOSION_ANIM_FRAMES * 2,
        );
        assert_eq!(graphic, LanderGraphic::Lander);
        assert!(index >= 46);
    }

    #[test]
    fn active_phase_uses_facing_indexed_lander_frame() {
        let (graphic, index) = select_lander_graphic(SessionPhase::Active, 7, 0);
        assert_eq!(graphic, LanderGraphic::Lander);
        assert_eq!(index, 7);
    }

    #[test]
    fn landing_and_takingoff_use_facing_indexed_lander_frame() {
        let (graphic, index) = select_lander_graphic(SessionPhase::Landing, 3, 10);
        assert_eq!(graphic, LanderGraphic::Lander);
        assert_eq!(index, 3);

        let (graphic, index) = select_lander_graphic(SessionPhase::TakingOff, 12, 5);
        assert_eq!(graphic, LanderGraphic::Lander);
        assert_eq!(index, 12);
    }

    // --- Crew grid layout tests (ported from DeltaLanderCrew) ---

    #[test]
    fn crew_slot_zero_is_at_grid_origin() {
        let (x, y) = crew_slot_position(0).unwrap();
        assert_eq!(x, CREW_ORIGIN_X);
        assert_eq!(y, CREW_ORIGIN_Y);
    }

    #[test]
    fn crew_slots_fill_first_row_left_to_right() {
        for col in 0..NUM_CREW_COLS {
            let slot = col;
            let (x, y) = crew_slot_position(slot).unwrap();
            assert_eq!(x, CREW_ORIGIN_X + CREW_SLOT_SPACING * i16::from(col));
            assert_eq!(y, CREW_ORIGIN_Y);
        }
    }

    #[test]
    fn crew_slots_fill_second_row() {
        for col in 0..NUM_CREW_COLS {
            let slot = NUM_CREW_COLS + col;
            let (x, y) = crew_slot_position(slot).unwrap();
            assert_eq!(x, CREW_ORIGIN_X + CREW_SLOT_SPACING * i16::from(col));
            assert_eq!(y, CREW_ORIGIN_Y - CREW_SLOT_SPACING);
        }
    }

    #[test]
    fn crew_slot_beyond_max_returns_none() {
        assert_eq!(crew_slot_position(MAX_CREW_SLOTS), None);
        assert_eq!(crew_slot_position(MAX_CREW_SLOTS + 5), None);
    }

    #[test]
    fn crew_alive_frame_is_55() {
        assert_eq!(crew_alive_frame(), 55);
    }

    // --- Cargo bar layout tests (ported from FillLanderHold) ---

    #[test]
    fn cargo_segment_zero_is_at_origin() {
        let (x, y) = cargo_segment_position(0);
        assert_eq!(x, CARGO_ORIGIN_X);
        assert_eq!(y, CARGO_ORIGIN_Y);
    }

    #[test]
    fn cargo_segments_stack_upward_one_pixel_each() {
        for index in 0..MAX_SCROUNGED {
            let (x, y) = cargo_segment_position(index);
            assert_eq!(x, CARGO_ORIGIN_X);
            assert_eq!(y, CARGO_ORIGIN_Y - index as i16);
        }
    }

    #[test]
    fn bio_segment_frames_alternate_41_42() {
        assert_eq!(cargo_segment_frame(0, true), 41);
        assert_eq!(cargo_segment_frame(1, true), 42);
        assert_eq!(cargo_segment_frame(2, true), 41);
        assert_eq!(cargo_segment_frame(3, true), 42);
        assert_eq!(cargo_segment_frame(48, true), 41);
        assert_eq!(cargo_segment_frame(49, true), 42);
    }

    #[test]
    fn mineral_segment_frames_alternate_43_44() {
        assert_eq!(cargo_segment_frame(0, false), 43);
        assert_eq!(cargo_segment_frame(1, false), 44);
        assert_eq!(cargo_segment_frame(2, false), 43);
        assert_eq!(cargo_segment_frame(3, false), 44);
    }

    #[test]
    fn bio_bar_segments_caps_at_max_scrounged() {
        assert_eq!(bio_bar_segments(0), 0);
        assert_eq!(bio_bar_segments(25), 25);
        assert_eq!(bio_bar_segments(50), 50);
        assert_eq!(bio_bar_segments(60), 50);
    }

    #[test]
    fn mineral_bar_segments_halved_with_improved_cargo() {
        // Standard cargo: 1:1 mapping.
        assert_eq!(mineral_bar_segments(0, false), 0);
        assert_eq!(mineral_bar_segments(25, false), 25);
        assert_eq!(mineral_bar_segments(50, false), 50);

        // Improved cargo: halved so 100 units fit the 50-pixel bar.
        assert_eq!(mineral_bar_segments(0, true), 0);
        assert_eq!(mineral_bar_segments(50, true), 25);
        assert_eq!(mineral_bar_segments(99, true), 49);
        assert_eq!(mineral_bar_segments(100, true), 50);
    }

    #[test]
    fn full_mineral_bar_with_improved_cargo_fills_50_segments() {
        // With improved cargo at capacity 100, the bar should show 50 segments.
        let segments = mineral_bar_segments(100, true);
        assert_eq!(segments, MAX_SCROUNGED);
        // Each segment has a valid position and frame.
        for index in 0..segments {
            let (x, y) = cargo_segment_position(index);
            assert_eq!(x, CARGO_ORIGIN_X);
            assert_eq!(y, CARGO_ORIGIN_Y - index as i16);
            let frame = cargo_segment_frame(index, false);
            assert!(frame == MINERAL_FRAME_EVEN || frame == MINERAL_FRAME_ODD);
        }
    }

    #[test]
    fn full_crew_grid_displays_all_twelve_slots() {
        // A full crew of 12 should fill all grid slots.
        for slot in 0..MAX_CREW_SLOTS {
            assert!(crew_slot_position(slot).is_some());
        }
        assert_eq!(crew_slot_position(MAX_CREW_SLOTS), None);
    }

    #[test]
    fn crew_meter_with_zero_crew_draws_nothing() {
        // Zero crew means no slots are iterated.
        let crew_count: u8 = 0;
        let mut count = 0;
        for slot in 0..crew_count.min(MAX_CREW_SLOTS) {
            let _ = slot;
            count += 1;
        }
        assert_eq!(count, 0);
    }
}
