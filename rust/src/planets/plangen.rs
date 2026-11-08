//! Live surface generation and rotating sphere pixels from plangen.c.

#[cfg(not(test))]
pub(crate) mod live {
    use crate::c_bindings::{global_state as c, solar_system};
    use crate::comm::locdata::{CPoint, CRect};
    use crate::graphics::context_abi::SetContextClipRect;
    use crate::graphics::drawable::Point;
    use crate::graphics::drawable_abi::SetContextFGFrame;
    use crate::planets::render::{
        c_api::render_system_topography, topo_gen, topo_scale::TopoScaler,
    };
    use crate::planets::sphere::shield_level;

    use std::ptr::{addr_of_mut, null_mut};

    use crate::planets::sphere::{images::required_frame, SphereState};

    #[cfg(feature = "debug-process")]
    use crate::graphics::context_abi::CreateContextAux;

    #[cfg(not(feature = "debug-process"))]
    use crate::graphics::context_abi::CreateContextAux;

    use crate::graphics::context_abi::SetContext;

    use crate::graphics::context_abi::DestroyContext;

    use crate::graphics::pixmap_abi::GetFrameCount;

    use crate::graphics::drawable_abi::ReadFramePixelIndexes;

    use crate::graphics::drawable_abi::ReadFramePixelColors;

    use crate::graphics::drawable_abi::WriteFramePixelColors;

    use crate::graphics::drawable_abi::SetFrameHot;

    use crate::strings::string_table::CaptureStringTable;

    use crate::strings::sresins::LoadStringTableInstance;

    use crate::strings::string_table::SetAbsStringTableIndex;

    use crate::strings::string_table::GetStringAddress;

    use crate::planets::sphere::live::init_sphere_rotation;

    use crate::graphics::context_abi::SetContextDrawMode;

    use crate::graphics::context_abi::SetContextForeGroundColor;

    use crate::graphics::frame_abi::DrawFilledRectangle;

    extern "C" {
        static mut ScreenWidth: i32;
    }

    unsafe fn init_orbit(state: &mut SphereState, width: usize, shielded: bool) {
        state.images = Some(crate::planets::sphere::images::OrbitImages::new(width));
        state.allocate_pixels(width, shielded);
    }

    unsafe fn create_shield(
        state: &mut SphereState,
    ) -> crate::engine::graphics::drawable_abi::LoadedDrawable {
        let image = crate::planets::sphere::images::image(2 | 8, 89, 89, 1);
        let target = required_frame(&image.lease(), 0);
        let pixels = &mut state.scratch;
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
        image
    }

    pub unsafe fn set_shield_throb_effect(
        state: &mut SphereState,
        shield: *mut c::FRAME_DESC,
        offset: i32,
        throb: *mut c::FRAME_DESC,
    ) {
        let level = shield_level(offset, state.width as i32);
        let (width, height) = (
            i32::from((*shield).Bounds.width),
            i32::from((*shield).Bounds.height),
        );
        ReadFramePixelColors(shield, state.scratch.as_mut_ptr(), width, height);
        for pixel in &mut state.scratch[..(width * height) as usize] {
            if pixel.a == 255 {
                pixel.r = (i32::from(pixel.r) * level / 256) as u8;
                pixel.g = (i32::from(pixel.g) * level / 256) as u8;
                pixel.b = (i32::from(pixel.b) * level / 256) as u8;
            } else if pixel.a > 0 {
                pixel.a = (i32::from(pixel.a) * level / 256) as u8;
            }
        }
        WriteFramePixelColors(throb, state.scratch.as_ptr(), width, height);
        SetFrameHot(
            throb.cast(),
            CPoint {
                x: (*shield).HotSpot.x,
                y: (*shield).HotSpot.y,
            },
        );
    }

    pub unsafe fn render_sphere(
        state: &mut SphereState,
        target: *mut c::FRAME_DESC,
        offset: i32,
        throb: bool,
    ) {
        state.render_pixels(offset, throb);
        WriteFramePixelColors(target, state.scratch.as_ptr(), 75, 75);
        SetFrameHot(target.cast(), CPoint { x: 38, y: 38 });
    }

    unsafe fn load_surface_palette(
        system: &mut crate::ships::native::abi::SOLARSYS_STATE,
        data: &crate::engine::world::planet::PlanetFrame,
        temperature: i16,
    ) {
        system.OrbitalCMap = CaptureStringTable(LoadStringTableInstance(data.cmap_instance)).cast();
        system.XlatRef = CaptureStringTable(LoadStringTableInstance(data.xlat_tab_instance)).cast();
        let index = if temperature > 100 {
            2
        } else if temperature > -40 {
            1
        } else {
            0
        };
        if index != 0 {
            system.OrbitalCMap = SetAbsStringTableIndex(system.OrbitalCMap.cast(), index).cast();
            system.XlatRef = SetAbsStringTableIndex(system.XlatRef.cast(), index).cast();
        }
        system.XlatPtr = GetStringAddress(system.XlatRef.cast()).cast();
    }

    unsafe fn load_definition(
        state: &mut SphereState,
        definition: crate::engine::graphics::drawable_abi::LoadedDrawable,
        width: usize,
    ) {
        let frame = required_frame(&definition.lease(), 0);
        let resized =
            i32::from((*frame).Bounds.width) != width as i32 || (*frame).Bounds.height != 75;
        let topography = resized.then(|| {
            definition
                .rescaled(0, width as i32, 75)
                .unwrap_or_else(|| panic!("rescaling planet definition"))
        });
        if GetFrameCount(frame.cast()) > 1 {
            let elevation = required_frame(&definition.lease(), 1);
            let scaled =
                if (*elevation).Bounds.width != width as i16 || (*elevation).Bounds.height != 75 {
                    Some(
                        definition
                            .rescaled(1, width as i32, 75)
                            .unwrap_or_else(|| panic!("rescaling planet elevation")),
                    )
                } else {
                    None
                };
            let elevation = scaled
                .as_ref()
                .map_or(elevation, |image| required_frame(&image.lease(), 0));
            ReadFramePixelIndexes(
                elevation.cast(),
                state.elevation.as_mut_ptr().cast(),
                width as i32,
                75,
            );
            for value in &mut state.elevation {
                *value = (*value as u8).wrapping_sub(128) as i8;
            }
        } else {
            state.elevation.fill(0);
        }
        state.images_mut().topography = Some(topography.unwrap_or(definition));
    }

    /// # Safety
    /// Planet and definition must be live records from the current solar system;
    /// the graphics resources and generation context must be initialized on the game thread.
    pub unsafe fn generate_planet_surface(
        state: &mut SphereState,
        animation: &crate::engine::kernel::planet_animation::PlanetAnimation,
        pulsing_shield: bool,
        planet: *mut crate::engine::world::planet::PlanetDesc,
        definition: Option<crate::engine::graphics::drawable_abi::LoadedDrawable>,
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
        let retained_context = crate::graphics::context_abi::ContextLease::require(context);
        init_orbit(state, width, shielded);
        let info = addr_of_mut!((*system).SysInfo.planet_info);
        let data = crate::planets::tables::PLANET_ARRAY.0[usize::from((*planet).data_index & !128)];
        let algo = data.type_ & 12;
        if let Some(definition) = definition {
            load_definition(state, definition, width);
        } else {
            let depth = &mut state.elevation;
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
            let topography = crate::planets::sphere::images::image(2, width, 75, 1);
            let target = required_frame(&topography.lease(), 0);
            load_surface_palette(&mut *system, &data, (*info).temperature);
            render_system_topography(&retained_context, system, target, depth, width as i32, 75)
                .expect("render planet topography");
            state.images_mut().topography = Some(topography);
        }
        if !shielded && (*info).atmo_density != 65535 {
            let depth = &state.elevation;
            let mut scaler = TopoScaler::with_seed(animation.topography_seed());
            let scaled = scaler.scale(
                depth,
                width,
                75,
                i32::from(data.num_faults),
                i32::from(data.fault_depth) * if algo == 4 { 2 } else { 1 },
            );
            animation.set_topography_seed(scaler.seed());
            render_system_topography(
                &retained_context,
                system,
                required_frame(&state.zoomed_topography(), 0),
                &scaled,
                (width * 4) as i32,
                300,
            )
            .expect("render scaled planet topography");
        }
        ReadFramePixelColors(
            required_frame(&state.topography(), 0),
            state.colors.as_mut_ptr(),
            (width + 75) as i32,
            75,
        );
        for y in 0..75 {
            let row = y * (width + 75);
            state.colors.copy_within(row..row + 75, row + width);
        }
        let depth = &mut state.elevation;
        if algo != 8 {
            topo_gen::generate_light_map(depth, width);
        } else {
            depth.fill(0);
        }
        let desc = (*system).pOrbitalDesc;
        let loc = if (*desc).prev_desc == addr_of_mut!((*system).SunDesc[0]) {
            (*desc).location
        } else {
            (*(*desc).prev_desc).location
        };
        state.generate(Point::new(loc.x, loc.y), i32::from((*info).axial_tilt));
        if shielded {
            let shield = create_shield(state);
            state.images_mut().shield = Some(shield);
        }
        let direction = 1 - 2 * (i32::from((*info).axial_tilt) & 1);
        init_sphere_rotation(state, direction, pulsing_shield);
        if shielded {
            let old = SetContextFGFrame(required_frame(&state.topography(), 0).cast());
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
