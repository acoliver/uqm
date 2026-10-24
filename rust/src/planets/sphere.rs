//! Planet sphere rotation, orbit zoom, and scan tinting from `pl_stuff.c`.

#[derive(Clone, Copy)]
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

fn zoom_scale(frame: i32, frame_count: i32) -> f64 {
    if frame < frame_count {
        1.134 - (-2.0 * f64::from(frame) / f64::from(frame_count)).exp()
    } else {
        1.0
    }
}

#[cfg(not(test))]
mod live {
    use super::{zoom_scale, Rotation};
    use crate::c_bindings::{global_state::DrawMode, solar_system};
    use crate::comm::locdata::{CColor, CRect};
    use crate::mainloop::restart_menu::c_extern::{
        uqm_sleep_thread_until as SleepThreadUntil, SetContext,
    };
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{Gfx, Point, Rect, Rgba, Stamp};
    use std::ffi::c_void;

    static mut ROTATION: Rotation = Rotation {
        frame: 0,
        direction: 0,
        point: 0,
        throb: false,
    };

    use crate::time::counter_abi::GetTimeCounter;
    extern "C" {
        static PlanetContext: *mut c_void;
        static OffScreenContext: *mut c_void;
        static optWhichShield: i32;
        static ScreenWidth: i32;
        static ScreenHeight: i32;
        fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
        fn GetFrameRect(frame: *mut c_void, rect: *mut c_void);
        fn CaptureDrawable(drawable: *mut c_void) -> *mut c_void;
        fn CreateDrawable(flags: u8, width: i16, height: i16, frames: u16) -> *mut c_void;
        fn ReleaseDrawable(frame: *mut c_void) -> *mut c_void;
        fn DestroyDrawable(drawable: *mut c_void) -> u32;
        fn RenderPlanetSphere(frame: *mut c_void, point: i32, throb: u32);
        fn SetShieldThrobEffect(work: *mut c_void, point: i32, frame: *mut c_void);

        fn RepairBackRect(rect: *mut CRect);
        fn SetContextFGFrame(frame: *mut c_void) -> *mut c_void;
        fn SetContextClipRect(rect: *mut CRect);
        fn SetContextDrawMode(mode: DrawMode) -> DrawMode;
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
        GetFrameRect(frame, (&mut r as *mut CRect).cast());
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

    #[no_mangle]
    unsafe extern "C" fn DrawDefaultPlanetSphere() {
        let old = SetContext(PlanetContext);
        DrawPlanetSphere((ScreenWidth - 78) / 2, (ScreenHeight - 13 - 75 - 5 - 9) / 2);
        SetContext(old);
    }

    #[no_mangle]
    unsafe extern "C" fn InitSphereRotation(direction: i32, shielded: u32) {
        let orbit = std::ptr::addr_of_mut!((*solar_system::current()).Orbit);
        let throb = shielded != 0 && optWhichShield == 1;
        ROTATION = Rotation {
            frame: 1,
            direction,
            point: 0,
            throb,
        };
        if throb {
            (*orbit).WorkFrame = (*orbit).ObjectFrame;
            let size = bounds((*orbit).ObjectFrame.cast());
            (*orbit).ObjectFrame =
                CaptureDrawable(CreateDrawable(2 | 8, size.width, size.height, 2)).cast();
        }
        PrepareNextRotationFrame();
    }

    #[no_mangle]
    unsafe extern "C" fn UninitSphereRotation() {
        let orbit = std::ptr::addr_of_mut!((*solar_system::current()).Orbit);
        if !(*orbit).WorkFrame.is_null() {
            DestroyDrawable(ReleaseDrawable((*orbit).ObjectFrame.cast()));
            (*orbit).ObjectFrame = (*orbit).WorkFrame;
            (*orbit).WorkFrame = std::ptr::null_mut();
        }
    }

    #[no_mangle]
    unsafe extern "C" fn PrepareNextRotationFrame() {
        let orbit = std::ptr::addr_of_mut!((*solar_system::current()).Orbit);
        let mut rotation = ROTATION;
        rotation.advance(ScreenWidth - 78);
        ROTATION = rotation;
        (*orbit).SphereFrame =
            SetAbsFrameIndex((*orbit).SphereFrame.cast(), rotation.frame as u16).cast();
        RenderPlanetSphere(
            (*orbit).SphereFrame.cast(),
            rotation.point,
            u32::from(rotation.throb),
        );
        if rotation.throb {
            (*orbit).ObjectFrame =
                SetAbsFrameIndex((*orbit).ObjectFrame.cast(), rotation.frame as u16).cast();
            SetShieldThrobEffect(
                (*orbit).WorkFrame.cast(),
                rotation.point,
                (*orbit).ObjectFrame.cast(),
            );
        }
    }

    #[no_mangle]
    unsafe extern "C" fn ZoomInPlanetSphere() {
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
            SetContext(PlanetContext);
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
            PrepareNextRotationFrame();
            SleepThreadUntil(next_time);
        }
    }

    #[no_mangle]
    unsafe extern "C" fn RotatePlanetSphere(keep_rate: u32) {
        static mut NEXT_TIME: u32 = 0;
        let now = GetTimeCounter();
        if keep_rate != 0 && now < NEXT_TIME {
            return;
        }
        NEXT_TIME = now.wrapping_add((840 * 12 / (ScreenWidth - 78)) as u32);
        DrawDefaultPlanetSphere();
        PrepareNextRotationFrame();
    }

    unsafe fn tint_frame(color: CColor) {
        let system = solar_system::current();
        let old_context = SetContext(OffScreenContext);
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
    unsafe extern "C" fn DrawPlanet(tint_y: i32, color: CColor) {
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
