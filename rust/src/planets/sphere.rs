//! Planet sphere rotation, orbit zoom, and scan tinting from `pl_stuff.c`.

pub(crate) fn shield_level(offset: i32, width: i32) -> i32 {
    let cycle = (width << 8) / 12;
    let half = cycle >> 1;
    100 + (((offset << 8) % cycle) - half).abs() * 156 / half
}

fn map_light(value: u8, diffuse: u32, variance: i32) -> u8 {
    (((diffuse.wrapping_mul(u32::from(value)) >> 16) as i32) + ((variance * i32::from(value)) >> 7))
        .clamp(0, 255) as u8
}

#[derive(Clone, Copy, Default)]
struct Rotation {
    frame: i32,
    direction: i32,
    point: i32,
    throb: bool,
}

impl Rotation {
    fn advance(&mut self, width: i32) {
        self.frame ^= 1;
        self.point += self.direction;
        if self.point < 0 {
            self.point = width - 1;
        } else if self.point >= width {
            self.point = 0;
        }
    }
}

pub struct SphereState {
    rotation: Rotation,
    pub(crate) light: Box<crate::planets::render::sphere_geometry::SphereMask>,
    pub(crate) mapping: Box<crate::planets::render::sphere_geometry::SphereTiltMap>,
    pub(crate) width: usize,
    pub(crate) shielded: bool,
    pub(crate) elevation: Vec<i8>,
    pub(crate) colors: Vec<crate::engine::graphics::native_abi::Color>,
    pub(crate) scratch: Vec<crate::engine::graphics::native_abi::Color>,
}

impl Default for SphereState {
    fn default() -> Self {
        use crate::planets::render::sphere_geometry::{SphereMapPoint, DIAMETER};
        Self {
            rotation: Rotation::default(),
            light: Box::new([[0; DIAMETER]; DIAMETER]),
            mapping: Box::new([[SphereMapPoint::default(); DIAMETER]; DIAMETER]),
            width: 0,
            shielded: false,
            elevation: Vec::new(),
            colors: Vec::new(),
            scratch: Vec::new(),
        }
    }
}

impl SphereState {
    fn reset_rotation(&mut self, direction: i32, pulsing: bool) {
        self.rotation = Rotation {
            frame: 1,
            direction,
            point: 0,
            throb: self.shielded && pulsing,
        };
    }

    pub(crate) fn render_pixels(&mut self, offset: i32, throb: bool) {
        use crate::engine::graphics::{cmap::Color, native_abi::Color as NativeColor};
        use crate::planets::render::sphere_geometry as sphere;
        let width = self.width;
        let pixels = &self.colors[offset as usize..];
        let elev = &self.elevation;
        let level = shield_level(offset, width as i32);
        let shielded = self.shielded;
        for y in 0..75 {
            for x in 0..75 {
                let diffuse = self.light[y][x];
                let target_pixel = &mut self.scratch[y * 75 + x];
                if diffuse == 0 {
                    *target_pixel = NativeColor {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    };
                    continue;
                }
                let point = self.mapping[y][x];
                let sample = |i: usize| {
                    pixels[point.points[i].y as usize * (width + 75) + point.points[i].x as usize]
                };
                let elevation = || {
                    i32::from(
                        elev[point.points[0].y as usize * width
                            + (offset as usize + point.points[0].x as usize) % width],
                    )
                };
                let (mut color, variance) = if point.weights[0] == 0 {
                    (sample(0), elevation())
                } else {
                    let colors = std::array::from_fn(|i| {
                        let p = sample(i);
                        Color {
                            r: p.r,
                            g: p.g,
                            b: p.b,
                            a: p.a,
                        }
                    });
                    let color = NativeColor {
                        r: sphere::get_avg_channel(colors, point.weights, 0),
                        g: sphere::get_avg_channel(colors, point.weights, 1),
                        b: sphere::get_avg_channel(colors, point.weights, 2),
                        a: 255,
                    };
                    // C samples p[0] for every weight, including signed-to-unsigned wrap.
                    let sum = point.weights.iter().fold(0_i32, |sum, &weight| {
                        sum.wrapping_add(elevation().wrapping_mul(weight as i32))
                    });
                    (color, sum >> 16)
                };
                if shielded {
                    color.g = (color.g >> 1) + (color.g >> 2);
                    color.b = (color.b >> 1) + (color.b >> 2);
                }
                color.r = map_light(color.r, diffuse, variance);
                color.g = map_light(color.g, diffuse, variance);
                color.b = map_light(color.b, diffuse, variance);
                if shielded {
                    let mut red = i32::from(map_light(100, diffuse, 0)) + 120;
                    if throb {
                        red = red * level / 256;
                    }
                    color.r = (red + i32::from(color.r)).min(255) as u8;
                }
                color.a = 255;
                *target_pixel = color;
            }
        }
    }

    pub fn elevation(&self) -> &[i8] {
        &self.elevation
    }

    pub fn colors(&self) -> &[crate::engine::graphics::native_abi::Color] {
        &self.colors
    }

    pub(crate) fn allocate_pixels(&mut self, width: usize, shielded: bool) {
        let transparent = crate::engine::graphics::native_abi::Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        self.width = width;
        self.shielded = shielded;
        self.elevation = vec![0; width * 75];
        self.colors = vec![transparent; (width + 75) * 75];
        self.scratch = vec![transparent; 89 * 89];
    }

    pub fn generate(&mut self, location: crate::engine::graphics::drawable::Point, tilt: i32) {
        use crate::planets::render::sphere_geometry;
        sphere_geometry::generate_sphere_mask(location, &mut self.light);
        sphere_geometry::create_sphere_tilt_map(tilt, &mut self.mapping);
    }
}

fn zoom_scale(frame: i32, frame_count: i32) -> f64 {
    if frame < frame_count {
        1.134 - (-2.0 * f64::from(frame) / f64::from(frame_count)).exp()
    } else {
        1.0
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::{zoom_scale, SphereState};
    use crate::c_bindings::{global_state::DrawMode, solar_system};
    use crate::comm::locdata::{CColor, CRect};
    use crate::engine::Kernel as KernelOwner;
    use crate::graphics::context_abi::SetContextClipRect;
    use crate::graphics::drawable_abi::SetContextFGFrame;
    use crate::mainloop::restart_menu::c_extern::{
        uqm_sleep_thread_until as SleepThreadUntil, SetContext,
    };
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{Gfx, Point, Rect, Rgba, Stamp};
    use std::{cell::RefCell, ffi::c_void};

    use crate::engine::graphics::context_abi::ContextLease;
    use crate::engine::time::counter_abi::GetTimeCounter;

    use crate::graphics::pixmap_abi::SetAbsFrameIndex;

    use crate::graphics::drawable_abi::GetFrameRect;

    use crate::graphics::pixmap_abi::CaptureDrawable;

    use crate::graphics::drawable_abi::CreateDrawable;

    use crate::graphics::pixmap_abi::ReleaseDrawable;

    use crate::graphics::drawable_abi::DestroyDrawable;

    use crate::planets::plangen::live::render_sphere;

    use crate::planets::plangen::live::set_shield_throb_effect;

    use crate::planets::scan::live::RepairBackRect;

    use crate::graphics::context_abi::SetContextDrawMode;

    extern "C" {

        static mut ScreenWidth: i32;
        static mut ScreenHeight: i32;
    }
    use crate::graphics::gfx_common_abi::{SetGraphicScale, SetGraphicScaleMode};

    fn stamp(frame: *mut c_void, x: i16, y: i16) {
        CffiPlatform.draw_stamp(Stamp {
            origin: Point { x, y },
            frame: frame as usize,
        });
    }

    unsafe fn bounds(frame: *mut c_void) -> CRect {
        let mut r = CRect::default();
        GetFrameRect((frame).cast(), (&mut r as *mut CRect).cast());
        r
    }

    #[no_mangle]
    unsafe extern "C" fn DrawPlanetSphere(x: i32, y: i32) {
        let orbit = std::ptr::addr_of!((*solar_system::current()).Orbit);
        CffiPlatform.batch();
        stamp((*orbit).SphereFrame.cast(), x as i16, y as i16);
        if !(*orbit).ObjectFrame.is_null() {
            stamp((*orbit).ObjectFrame.cast(), x as i16, y as i16);
        }
        CffiPlatform.unbatch();
    }

    pub unsafe fn draw_default_planet_sphere(planet: &ContextLease) {
        planet.with_selected(|| {
            DrawPlanetSphere((ScreenWidth - 78) / 2, (ScreenHeight - 13 - 75 - 5 - 9) / 2);
        });
    }

    pub unsafe fn init_sphere_rotation(state: &mut SphereState, direction: i32, pulsing: bool) {
        let orbit = std::ptr::addr_of_mut!((*solar_system::current()).Orbit);
        state.reset_rotation(direction, pulsing);
        if state.rotation.throb {
            (*orbit).WorkFrame = (*orbit).ObjectFrame;
            let size = bounds((*orbit).ObjectFrame.cast());
            (*orbit).ObjectFrame =
                CaptureDrawable(CreateDrawable(2 | 8, size.width, size.height, 2)).cast();
        }
        prepare_next_rotation_frame(state);
    }

    #[no_mangle]
    pub unsafe extern "C" fn UninitSphereRotation() {
        let orbit = std::ptr::addr_of_mut!((*solar_system::current()).Orbit);
        if !(*orbit).WorkFrame.is_null() {
            DestroyDrawable(ReleaseDrawable((*orbit).ObjectFrame.cast()));
            (*orbit).ObjectFrame = (*orbit).WorkFrame;
            (*orbit).WorkFrame = std::ptr::null_mut();
        }
    }

    unsafe fn prepare_next_rotation_frame(state: &mut SphereState) {
        let orbit = std::ptr::addr_of_mut!((*solar_system::current()).Orbit);
        let mut rotation = state.rotation;
        rotation.advance(ScreenWidth - 78);
        state.rotation = rotation;
        (*orbit).SphereFrame =
            SetAbsFrameIndex((*orbit).SphereFrame.cast(), rotation.frame as u16).cast();
        render_sphere(
            state,
            (*orbit).SphereFrame.cast(),
            rotation.point,
            rotation.throb,
        );
        if rotation.throb {
            (*orbit).ObjectFrame =
                SetAbsFrameIndex((*orbit).ObjectFrame.cast(), rotation.frame as u16).cast();
            set_shield_throb_effect(
                state,
                (*orbit).WorkFrame.cast(),
                rotation.point,
                (*orbit).ObjectFrame.cast(),
            );
        }
    }

    pub unsafe fn zoom_in_planet_sphere(planet: &ContextLease, state: &RefCell<SphereState>) {
        let orbit = std::ptr::addr_of!((*solar_system::current()).Orbit);
        let width = ScreenWidth - 78;
        let scan_height = ScreenHeight - 13 - 75 - 5;
        let frame_count = (840 * 6 / 5) / (840 / 24);
        let corner = crate::math::random() as i32;
        let dx = 1 - (corner & 1) * 2;
        let dy = 1 - (corner & 2);
        let frame_rect = bounds(if !(*orbit).ObjectFrame.is_null() {
            (*orbit).ObjectFrame.cast()
        } else {
            (*orbit).SphereFrame.cast()
        });
        let mut repair = frame_rect;
        for i in 0..=frame_count {
            let next_time = GetTimeCounter().wrapping_add(840 / 24);
            let scale = zoom_scale(i, frame_count);
            let x = width / 2
                + (f64::from(dx) * (1.0 - scale) * f64::from(width * 6 / 10) + 0.5) as i32;
            let y = (scan_height - 9) / 2
                + (f64::from(dy) * (1.0 - scale) * f64::from(scan_height * 6 / 10) + 0.5) as i32;
            planet.select();
            CffiPlatform.batch();
            if i > 0 {
                RepairBackRect(&mut repair);
            }
            let old_mode = SetGraphicScaleMode(1);
            let old_scale = SetGraphicScale((256.0 * scale + 0.5) as i32);
            DrawPlanetSphere(x, y);
            SetGraphicScale(old_scale);
            SetGraphicScaleMode(old_mode);
            CffiPlatform.unbatch();
            repair.corner.x = x as i16 + frame_rect.corner.x;
            repair.corner.y = y as i16 + frame_rect.corner.y;
            prepare_next_rotation_frame(&mut state.borrow_mut());
            SleepThreadUntil(next_time);
        }
    }

    pub unsafe fn rotate_planet_sphere(
        planet: &ContextLease,
        state: &RefCell<SphereState>,
        animation: &crate::engine::kernel::planet_animation::PlanetAnimation,
        keep_rate: bool,
    ) {
        if !animation.rotation_due(GetTimeCounter(), keep_rate, ScreenWidth - 78) {
            return;
        }
        draw_default_planet_sphere(planet);
        prepare_next_rotation_frame(&mut state.borrow_mut());
    }

    unsafe fn tint_frame(color: CColor) {
        let system = solar_system::current();
        let old_context = SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
        SetContextFGFrame((*system).Orbit.TintFrame.cast());
        SetContextClipRect(std::ptr::null_mut());
        let rect = CffiPlatform.context_clip_rect();
        stamp((*system).TopoFrame.cast(), 0, 0);
        let old_mode = SetContextDrawMode(DrawMode {
            kind: 1,
            factor: 255 / 2,
        });
        CffiPlatform.set_fg_color(Rgba {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        });
        CffiPlatform.fill_rect(rect);
        SetContextDrawMode(old_mode);
        SetContext(old_context);
    }

    fn same_rgb(a: CColor, b: CColor) -> bool {
        a.r == b.r && a.g == b.g && a.b == b.b && a.a == b.a
    }

    #[no_mangle]
    pub unsafe extern "C" fn DrawPlanet(tint_y: i32, color: CColor) {
        let system = solar_system::current();
        let orbit = std::ptr::addr_of_mut!((*system).Orbit);
        CffiPlatform.batch();
        if same_rgb(
            color,
            CColor {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        ) {
            stamp((*system).TopoFrame.cast(), 0, 0);
        } else {
            let frame = (*orbit).TintFrame;
            let height = i32::from(bounds(frame.cast()).height);
            let cached = (*orbit).TintColor;
            if !same_rgb(
                color,
                CColor {
                    r: cached.r,
                    g: cached.g,
                    b: cached.b,
                    a: cached.a,
                },
            ) {
                tint_frame(color);
                (*orbit).TintColor = crate::c_bindings::global_state::Color {
                    r: color.r,
                    g: color.g,
                    b: color.b,
                    a: color.a,
                };
            }
            if tint_y < height - 1 {
                stamp((*system).TopoFrame.cast(), 0, 0);
            }
            if tint_y >= 0 {
                let old = CffiPlatform.context_clip_rect();
                CffiPlatform.set_context_clip_rect(Rect {
                    height: (tint_y + 1) as i16,
                    ..old
                });
                stamp(frame.cast(), 0, 0);
                CffiPlatform.set_context_clip_rect(old);
            }
        }
        CffiPlatform.unbatch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shield_cycle_and_light_boundaries() {
        assert_eq!(shield_level(0, 242), 256);
        assert_eq!(shield_level(10, 242), 101);
        assert_eq!(shield_level(20, 242), 253);
        assert_eq!(shield_level(121, 242), 255);
        assert_eq!(shield_level(241, 242), 241);
        assert_eq!(map_light(255, 65536, 0), 255);
        assert_eq!(map_light(255, 65536, -128), 0);
        assert_eq!(map_light(100, 0, 64), 50);
    }
    #[test]
    fn rasterizes_owned_pixels_with_elevation_interpolation_and_shield_throb() {
        use crate::engine::graphics::{drawable::Point, native_abi::Color};
        let mut state = SphereState::default();
        state.allocate_pixels(242, false);
        let points = [
            Point::new(3, 4),
            Point::new(4, 4),
            Point::new(3, 5),
            Point::new(4, 5),
        ];
        state.light[37][37] = 65_536;
        state.mapping[37][37].points = points;
        let source = Color {
            r: 100,
            g: 80,
            b: 60,
            a: 17,
        };
        state.colors[4 * 317 + 3] = source;
        state.scratch[0] = source;
        state.scratch[75 * 75] = source;
        let channels = |p: Color| (p.r, p.g, p.b, p.a);
        let center = 37 * 75 + 37;
        state.render_pixels(0, false);
        assert_eq!(channels(state.scratch[center]), (100, 80, 60, 255));
        assert_eq!(channels(state.scratch[0]), (0, 0, 0, 0));
        assert_eq!(channels(state.scratch[75 * 75]), channels(source));
        for (i, point) in points.iter().enumerate() {
            let step = i as u8 * 40;
            state.colors[point.y as usize * 317 + point.x as usize] = Color {
                r: 100 + step,
                g: 80 + step,
                b: 60 + step,
                a: 17,
            };
            state.elevation[point.y as usize * 242 + point.x as usize] =
                if i == 0 { -32 } else { 64 };
        }
        state.mapping[37][37].weights = [16_384; 4];
        state.render_pixels(0, false);
        assert_eq!(channels(state.scratch[center]), (120, 105, 90, 255));
        state.elevation.fill(0);
        state.mapping[37][37].weights = [0; 4];
        state.shielded = true;
        state.render_pixels(0, false);
        assert_eq!(channels(state.scratch[center]), (255, 60, 45, 255));
        state.colors[4 * 317 + 3 + 10] = source;
        state.render_pixels(10, true);
        assert_eq!(channels(state.scratch[center]), (186, 60, 45, 255));
    }

    #[test]
    fn pixel_buffers_are_sized_cleared_and_owned_by_each_orbit() {
        let mut first = SphereState::default();
        let mut second = SphereState::default();
        first.allocate_pixels(242, false);
        second.allocate_pixels(242, true);
        assert_eq!(first.elevation, vec![0; 242 * 75]);
        assert_eq!(first.colors.len(), (242 + 75) * 75);
        assert_eq!(first.scratch.len(), 89 * 89);
        assert!(first.colors.iter().chain(&first.scratch).all(|p| p.a == 0));
        first.elevation[0] = 30;
        first.colors[0].r = 40;
        first.scratch[0].g = 50;
        assert_eq!(second.elevation[0], 0);
        assert_eq!(second.colors[0].r, 0);
        assert_eq!(second.scratch[0].g, 0);
        assert!(!first.shielded);
        assert!(second.shielded);
    }

    #[test]
    fn separate_orbits_keep_their_lookup_maps_and_rotation_state() {
        let mut first = SphereState::default();
        let mut second = SphereState::default();
        first.generate(crate::engine::graphics::drawable::Point::new(100, 0), 0);
        let first_light = first.light.clone();
        let first_mapping = first.mapping.clone();
        second.generate(crate::engine::graphics::drawable::Point::new(0, 100), 23);
        assert_eq!(first.light, first_light);
        assert_eq!(first.mapping, first_mapping);
        assert_ne!(first.light, second.light);
        assert_ne!(first.mapping, second.mapping);
        first.shielded = true;
        first.reset_rotation(1, true);
        assert_eq!((first.rotation.frame, first.rotation.point), (1, 0));
        assert!(first.rotation.throb);
        first.rotation.advance(242);
        assert_eq!((first.rotation.frame, first.rotation.point), (0, 1));
        assert_eq!((second.rotation.frame, second.rotation.point), (0, 0));
        assert!(!second.rotation.throb);
    }

    #[test]
    fn rotation_alternates_buffers_and_wraps_in_both_directions() {
        let mut rotation = Rotation {
            frame: 1,
            direction: 1,
            point: 0,
            throb: false,
        };
        rotation.advance(242);
        assert_eq!((rotation.frame, rotation.point), (0, 1));
        for _ in 0..241 {
            rotation.advance(242);
        }
        assert_eq!(rotation.point, 0);
        rotation.direction = -1;
        rotation.advance(242);
        assert_eq!(rotation.point, 241);
        assert!(!rotation.throb);
    }
    #[test]
    fn zoom_uses_exponential_curve_and_exact_final_scale() {
        let count = (840 * 6 / 5) / (840 / 24);
        assert_eq!(count, 28);
        assert!((zoom_scale(0, count) - 0.134).abs() < 1e-12);
        assert!((zoom_scale(14, count) - (1.134 - (-1.0_f64).exp())).abs() < 1e-12);
        assert_eq!(zoom_scale(count, count), 1.0);
    }
}
