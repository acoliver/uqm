//! Live surface generation and rotating sphere pixels from plangen.c.

fn shield_level(offset: i32, width: i32) -> i32 {
    let cycle = (width << 8) / 12;
    let half = cycle >> 1;
    100 + (((offset << 8) % cycle) - half).abs() * 156 / half
}

fn map_light(value: u8, diffuse: u32, variance: i32) -> u8 {
    (((diffuse.wrapping_mul(u32::from(value)) >> 16) as i32) + ((variance * i32::from(value)) >> 7))
        .clamp(0, 255) as u8
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
mod live {
    use super::{map_light, shield_level};
    use crate::c_bindings::{global_state as c, solar_system};
    use crate::comm::locdata::{CPoint, CRect};
    use crate::graphics::{cmap::Color, drawable::Point};
    use crate::memory::{rust_hcalloc, rust_hmalloc};
    use crate::planets::render::{
        c_api::render_system_topography, sphere_geometry as sphere, topo_gen,
        topo_scale::TopoScaler,
    };
    use std::ffi::{c_char, c_void};
    use std::ptr::{addr_of_mut, null_mut};
    use std::sync::Mutex;

    static SCALER: Mutex<Option<TopoScaler>> = Mutex::new(None);
    static mut LIGHT_DIFF: sphere::SphereMask = [[0; sphere::DIAMETER]; sphere::DIAMETER];
    static mut MAP_ROTATE: sphere::SphereTiltMap = [[sphere::SphereMapPoint {
        points: [Point { x: 0, y: 0 }; 4],
        weights: [0; 4],
    }; sphere::DIAMETER]; sphere::DIAMETER];

    extern "C" {
        static ScreenWidth: i32;
        #[cfg(feature = "debug-process")]
        fn CreateContextAux(name: *const c_char) -> *mut c_void;
        #[cfg(not(feature = "debug-process"))]
        fn CreateContextAux() -> *mut c_void;
        fn SetContext(context: *mut c_void) -> *mut c_void;
        fn DestroyContext(context: *mut c_void) -> i32;
        fn CaptureDrawable(drawable: *mut c_void) -> *mut c_void;
        fn CreateDrawable(flags: u8, width: i16, height: i16, frames: u16) -> *mut c_void;
        fn ReleaseDrawable(frame: *mut c_void) -> *mut c_void;
        fn DestroyDrawable(drawable: *mut c_void) -> u32;
        fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
        fn GetFrameCount(frame: *mut c_void) -> u16;
        fn RescaleFrame(frame: *mut c_void, width: i32, height: i32) -> *mut c_void;
        fn ReadFramePixelIndexes(
            frame: *mut c_void,
            pixels: *mut u8,
            width: i32,
            height: i32,
        ) -> u32;
        fn ReadFramePixelColors(
            frame: *mut c::FRAME_DESC,
            pixels: *mut c::Color,
            width: i32,
            height: i32,
        ) -> u32;
        fn WriteFramePixelColors(
            frame: *mut c::FRAME_DESC,
            pixels: *const c::Color,
            width: i32,
            height: i32,
        ) -> u32;
        fn SetFrameHot(frame: *mut c_void, hot: CPoint) -> CPoint;
        fn CaptureStringTable(table: *mut c_void) -> *mut c_void;
        fn LoadStringTableInstance(resource: *const c_char) -> *mut c_void;
        fn SetAbsStringTableIndex(table: *mut c_void, index: u16) -> *mut c_void;
        fn GetStringAddress(table: *mut c_void) -> *const c_char;
        fn InitSphereRotation(direction: i32, shielded: u32);
        fn SetContextFGFrame(frame: *mut c_void) -> *mut c_void;
        fn SetContextClipRect(rect: *mut CRect);
        fn SetContextDrawMode(mode: c::DrawMode) -> c::DrawMode;
        fn SetContextForeGroundColor(color: c::Color) -> c::Color;
        fn DrawFilledRectangle(rect: *mut CRect);
    }

    unsafe fn frame(flags: u8, width: usize, height: usize, count: u16) -> *mut c::FRAME_DESC {
        CaptureDrawable(CreateDrawable(flags, width as i16, height as i16, count)).cast()
    }

    unsafe fn init_orbit(width: usize) {
        let orbit = addr_of_mut!((*solar_system::current()).Orbit);
        (*orbit).SphereFrame = frame(2 | 8, 75, 75, 2);
        (*orbit).TintFrame = frame(2, width, 75, 1);
        (*orbit).ObjectFrame = null_mut();
        (*orbit).WorkFrame = null_mut();
        (*orbit).lpTopoData = rust_hcalloc(width * 75).cast();
        (*orbit).TopoZoomFrame = frame(2, width * 4, 75 * 4, 1);
        (*orbit).TopoColors =
            rust_hmalloc(std::mem::size_of::<c::Color>() * 75 * (width + 75)).cast();
        (*orbit).ScratchArray = rust_hmalloc(std::mem::size_of::<c::Color>() * 89 * 89).cast();
    }

    unsafe fn create_shield() -> *mut c::FRAME_DESC {
        let orbit = addr_of_mut!((*solar_system::current()).Orbit);
        let target = frame(2 | 8, 89, 89, 1);
        let pixels = std::slice::from_raw_parts_mut((*orbit).ScratchArray, 89 * 89);
        for y in -44_i32..=44 {
            for x in -44_i32..=44 {
                let rad2 = x * x + y * y;
                let pixel = &mut pixels[((y + 44) * 89 + x + 44) as usize];
                *pixel = c::Color {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                };
                if rad2 >= 45 * 45 || rad2 <= 37 * 37 {
                    continue;
                }
                let radius = f64::from(rad2).sqrt();
                let (red, alpha) = if radius <= 37.8 {
                    (255, (220.0 * (radius - 37.0)) as i32)
                } else {
                    (220 - (165.0 * (radius - 37.0) / 7.0) as i32, 255)
                };
                *pixel = c::Color {
                    r: red.max(0) as u8,
                    g: 0,
                    b: 0,
                    a: alpha as u8,
                };
            }
        }
        WriteFramePixelColors(target, pixels.as_ptr(), 89, 89);
        SetFrameHot(target.cast(), CPoint { x: 45, y: 45 });
        target
    }

    #[no_mangle]
    unsafe extern "C" fn SetShieldThrobEffect(
        shield: *mut c::FRAME_DESC,
        offset: i32,
        throb: *mut c::FRAME_DESC,
    ) {
        let orbit = addr_of_mut!((*solar_system::current()).Orbit);
        let level = shield_level(offset, ScreenWidth - 78);
        let (width, height) = (
            i32::from((*shield).Bounds.width),
            i32::from((*shield).Bounds.height),
        );
        ReadFramePixelColors(shield, (*orbit).ScratchArray, width, height);
        for pixel in
            std::slice::from_raw_parts_mut((*orbit).ScratchArray, (width * height) as usize)
        {
            if pixel.a == 255 {
                pixel.r = (i32::from(pixel.r) * level / 256) as u8;
                pixel.g = (i32::from(pixel.g) * level / 256) as u8;
                pixel.b = (i32::from(pixel.b) * level / 256) as u8;
            } else if pixel.a > 0 {
                pixel.a = (i32::from(pixel.a) * level / 256) as u8;
            }
        }
        WriteFramePixelColors(throb, (*orbit).ScratchArray, width, height);
        SetFrameHot(
            throb.cast(),
            CPoint {
                x: (*shield).HotSpot.x,
                y: (*shield).HotSpot.y,
            },
        );
    }

    #[no_mangle]
    unsafe extern "C" fn RenderPlanetSphere(target: *mut c::FRAME_DESC, offset: i32, throb: u32) {
        let system = solar_system::current();
        let orbit = addr_of_mut!((*system).Orbit);
        let width = (ScreenWidth - 78) as usize;
        let pixels = (*orbit).TopoColors.add(offset as usize);
        let elev = (*orbit).lpTopoData;
        let level = shield_level(offset, width as i32);
        let shielded = (*(*system).pOrbitalDesc).data_index & 128 != 0;
        for y in 0..75 {
            for x in 0..75 {
                let diffuse = (*addr_of_mut!(LIGHT_DIFF))[y][x];
                let target_pixel = (*orbit).ScratchArray.add(y * 75 + x);
                if diffuse == 0 {
                    target_pixel.write(c::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    continue;
                }
                let point = (*addr_of_mut!(MAP_ROTATE))[y][x];
                let sample = |i: usize| {
                    *pixels
                        .add(point.points[i].y as usize * (width + 75) + point.points[i].x as usize)
                };
                let elevation = || {
                    i32::from(*elev.add(
                        point.points[0].y as usize * width
                            + (offset as usize + point.points[0].x as usize) % width,
                    ))
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
                    let color = c::Color {
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
                    if throb != 0 {
                        red = red * level / 256;
                    }
                    color.r = (red + i32::from(color.r)).min(255) as u8;
                }
                color.a = 255;
                target_pixel.write(color);
            }
        }
        WriteFramePixelColors(target, (*orbit).ScratchArray, 75, 75);
        SetFrameHot(target.cast(), CPoint { x: 38, y: 38 });
    }

    #[no_mangle]
    unsafe extern "C" fn GeneratePlanetSurface(
        planet: *mut c::PLANET_DESC,
        mut definition: *mut c::FRAME_DESC,
    ) {
        let system = solar_system::current();
        let width = (ScreenWidth - 78) as usize;
        let shielded = (*planet).data_index & 128 != 0;
        crate::planets::ffi::sys_gen_rng().reseed((*planet).rand_seed);
        #[cfg(feature = "debug-process")]
        let context = CreateContextAux(c"Plangen.TopoContext".as_ptr());
        #[cfg(not(feature = "debug-process"))]
        let context = CreateContextAux();
        let previous_context = SetContext(context);
        init_orbit(width);
        let orbit = addr_of_mut!((*system).Orbit);
        let info = addr_of_mut!((*system).SysInfo.PlanetInfo);
        let data = crate::planets::tables::PLANET_ARRAY.0[usize::from((*planet).data_index & !128)];
        let algo = data.Type & 12;
        if !definition.is_null() {
            definition = SetAbsFrameIndex(definition.cast(), 0).cast();
            let resized = i32::from((*definition).Bounds.width) != width as i32
                || (*definition).Bounds.height != 75;
            (*system).TopoFrame = if resized {
                CaptureDrawable(RescaleFrame(definition.cast(), width as i32, 75)).cast()
            } else {
                definition
            };
            if GetFrameCount(definition.cast()) > 1 {
                let mut elevation = SetAbsFrameIndex(definition.cast(), 1).cast::<c::FRAME_DESC>();
                if (*elevation).Bounds.width != width as i16 || (*elevation).Bounds.height != 75 {
                    elevation =
                        CaptureDrawable(RescaleFrame(elevation.cast(), width as i32, 75)).cast();
                }
                ReadFramePixelIndexes(
                    elevation.cast(),
                    (*orbit).lpTopoData.cast(),
                    width as i32,
                    75,
                );
                for value in std::slice::from_raw_parts_mut((*orbit).lpTopoData, width * 75) {
                    *value = (*value as u8).wrapping_sub(128) as i8;
                }
            } else {
                std::slice::from_raw_parts_mut((*orbit).lpTopoData, width * 75).fill(0);
            }
            if resized {
                DestroyDrawable(ReleaseDrawable(definition.cast()));
            }
        } else {
            let depth = std::slice::from_raw_parts_mut((*orbit).lpTopoData, width * 75);
            let rng = crate::planets::ffi::sys_gen_rng();
            match algo {
                8 => topo_gen::make_gas_giant(rng, depth, width, data.num_faults, data.fault_depth),
                0 | 4 => {
                    if data.num_faults != 0 {
                        crate::planets::topography::delta_topography(
                            rng,
                            data.num_faults,
                            depth,
                            width as i16,
                            75,
                            data.fault_depth,
                        );
                    }
                    for _ in 0..data.num_blemishes {
                        let random = rng.next_random() as u16;
                        let low = i16::from(random as u8);
                        let size = match (random >> 8) & 31 {
                            0 => low % 18 + 18,
                            1..=4 => low % 9 + 9,
                            _ => low % 4 + 4,
                        };
                        let random = rng.next_random() as u16;
                        let rect = CRect {
                            corner: CPoint {
                                x: ((random >> 8) % (width as u16 - size as u16)) as i16,
                                y: (random as u8 as u16 % (75 - size as u16)) as i16,
                            },
                            width: size,
                            height: size,
                        };
                        topo_gen::make_crater(
                            depth,
                            width,
                            &rect,
                            data.fault_depth << 2,
                            -(data.fault_depth << 2),
                            false,
                        );
                    }
                    if algo == 4 {
                        topo_gen::dither_map(rng, depth, width);
                    }
                    topo_gen::validate_map(depth, width);
                }
                _ => {}
            }
            (*system).TopoFrame = frame(2, width, 75, 1);
            (*system).OrbitalCMap =
                CaptureStringTable(LoadStringTableInstance(data.CMapInstance)).cast();
            (*system).XlatRef =
                CaptureStringTable(LoadStringTableInstance(data.XlatTabInstance)).cast();
            let index = if (*info).SurfaceTemperature > 100 {
                2
            } else if (*info).SurfaceTemperature > -40 {
                1
            } else {
                0
            };
            if index != 0 {
                (*system).OrbitalCMap =
                    SetAbsStringTableIndex((*system).OrbitalCMap.cast(), index).cast();
                (*system).XlatRef = SetAbsStringTableIndex((*system).XlatRef.cast(), index).cast();
            }
            (*system).XlatPtr = GetStringAddress((*system).XlatRef.cast()).cast_mut().cast();
            render_system_topography(system, (*system).TopoFrame, depth, width as i32, 75)
                .expect("render planet topography");
        }
        if !shielded && (*info).AtmoDensity != 65535 {
            let depth = std::slice::from_raw_parts((*orbit).lpTopoData, width * 75);
            let scaled = SCALER
                .lock()
                .expect("topography scaler")
                .get_or_insert_with(TopoScaler::default)
                .scale(
                    depth,
                    width,
                    75,
                    i32::from(data.num_faults),
                    i32::from(data.fault_depth) * if algo == 4 { 2 } else { 1 },
                );
            render_system_topography(
                system,
                (*orbit).TopoZoomFrame,
                &scaled,
                (width * 4) as i32,
                300,
            )
            .expect("render scaled planet topography");
        }
        ReadFramePixelColors(
            (*system).TopoFrame,
            (*orbit).TopoColors,
            (width + 75) as i32,
            75,
        );
        for y in 0..75 {
            std::ptr::copy_nonoverlapping(
                (*orbit).TopoColors.add(y * (width + 75)),
                (*orbit).TopoColors.add(y * (width + 75) + width),
                75,
            );
        }
        let depth = std::slice::from_raw_parts_mut((*orbit).lpTopoData, width * 75);
        if algo != 8 {
            topo_gen::generate_light_map(depth, width);
        } else {
            depth.fill(0);
        }
        let desc = (*system).pOrbitalDesc;
        let loc = if (*desc).pPrevDesc == addr_of_mut!((*system).SunDesc[0]) {
            (*desc).location
        } else {
            (*(*desc).pPrevDesc).location
        };
        sphere::generate_sphere_mask(Point::new(loc.x, loc.y), &mut *addr_of_mut!(LIGHT_DIFF));
        sphere::create_sphere_tilt_map(
            i32::from((*info).AxialTilt),
            &mut *addr_of_mut!(MAP_ROTATE),
        );
        if shielded {
            (*orbit).ObjectFrame = create_shield();
        }
        InitSphereRotation(
            1 - 2 * (i32::from((*info).AxialTilt) & 1),
            u32::from(shielded),
        );
        if shielded {
            let old = SetContextFGFrame((*system).TopoFrame.cast());
            SetContextClipRect(null_mut());
            let mut rect: c::RECT = std::mem::zeroed();
            c::GetContextClipRect(&mut rect);
            let old_mode = SetContextDrawMode(c::DrawMode {
                kind: 1,
                factor: 255,
            });
            SetContextForeGroundColor(c::Color {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            });
            DrawFilledRectangle((&mut rect as *mut c::RECT).cast());
            SetContextDrawMode(old_mode);
            SetContextFGFrame(old);
        }
        SetContext(previous_context);
        DestroyContext(context);
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
}
