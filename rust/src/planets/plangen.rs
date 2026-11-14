//! Live surface generation and rotating sphere pixels from plangen.c.

pub(crate) mod live {
    use crate::comm::locdata::{CPoint, CRect};
    use crate::graphics::context_abi::{ContextFrame, LoadedContext};
    use crate::graphics::drawable::Point;
    use crate::graphics::native_abi as c;
    use crate::planets::render::{
        c_api::render_native_topography, topo_gen, topo_scale::TopoScaler,
    };
    use crate::planets::sphere::shield_level;

    use crate::planets::sphere::live::init_sphere_rotation;
    use crate::planets::sphere::SphereState;

    fn init_orbit(state: &mut SphereState, width: usize, shielded: bool) {
        state.images = Some(crate::planets::sphere::images::OrbitImages::new(width));
        state.allocate_pixels(width, shielded);
    }

    fn create_shield(
        state: &mut SphereState,
    ) -> crate::engine::graphics::drawable_abi::LoadedDrawable {
        let image = crate::planets::sphere::images::image(2 | 8, 89, 89, 1);
        let target = image.lease().context_frame(0);
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
        target.write_colors(pixels, 89, 89);
        target.replace_hotspot(c::POINT { x: 45, y: 45 });
        image
    }

    pub fn set_shield_throb_effect(
        state: &mut SphereState,
        shield: &ContextFrame,
        offset: i32,
        throb: &ContextFrame,
    ) {
        let level = shield_level(offset, state.width as i32);
        let (width, height) = (
            i32::from(shield.rect().extent.width),
            i32::from(shield.rect().extent.height),
        );
        shield.read_colors(&mut state.scratch, width, height);
        for pixel in &mut state.scratch[..(width * height) as usize] {
            if pixel.a == 255 {
                pixel.r = (i32::from(pixel.r) * level / 256) as u8;
                pixel.g = (i32::from(pixel.g) * level / 256) as u8;
                pixel.b = (i32::from(pixel.b) * level / 256) as u8;
            } else if pixel.a > 0 {
                pixel.a = (i32::from(pixel.a) * level / 256) as u8;
            }
        }
        throb.write_colors(&state.scratch, width, height);
        let rect = shield.rect();
        throb.replace_hotspot(c::POINT {
            x: -rect.corner.x,
            y: -rect.corner.y,
        });
    }

    pub fn render_sphere(state: &mut SphereState, target: &ContextFrame, offset: i32, throb: bool) {
        state.render_pixels(offset, throb);
        target.write_colors(&state.scratch, 75, 75);
        target.replace_hotspot(c::POINT { x: 38, y: 38 });
    }

    fn load_surface_palette(
        index: u8,
        temperature: i16,
    ) -> crate::planets::sphere::palette::SurfacePalette {
        use crate::planets::sphere::palette::{temperature_index, SurfacePalette};
        use crate::strings::sresins::load_binary_entry;
        let data = &crate::planets::tables::PLANET_ARRAY[usize::from(index)];
        let variant = temperature_index(temperature);
        let colors = load_binary_entry(data.cmap_instance, variant)
            .unwrap_or_else(|| panic!("planet color map is missing"));
        let translation = load_binary_entry(data.xlat_tab_instance, variant)
            .unwrap_or_else(|| panic!("planet translation table is missing"));
        SurfacePalette::from_bytes(index, &translation, colors)
            .unwrap_or_else(|error| panic!("planet palette: {error:?}"))
    }

    fn load_definition(
        state: &mut SphereState,
        definition: crate::engine::graphics::drawable_abi::LoadedDrawable,
        width: usize,
    ) {
        let frame = definition.lease().context_frame(0);
        let resized = i32::from(frame.rect().extent.width) != width as i32
            || frame.rect().extent.height != 75;
        let topography = resized.then(|| {
            definition
                .rescaled(0, width as i32, 75)
                .unwrap_or_else(|| panic!("rescaling planet definition"))
        });
        if frame.frame_count() > 1 {
            let elevation = definition.lease().context_frame(1);
            let scaled = if elevation.rect().extent.width != width as i16
                || elevation.rect().extent.height != 75
            {
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
                .map_or(elevation, |image| image.lease().context_frame(0));
            let mut indexes = vec![0; state.elevation.len()];
            elevation.read_indexes(&mut indexes, width as i32, 75);
            for (value, index) in state.elevation.iter_mut().zip(indexes) {
                *value = index.wrapping_sub(128) as i8;
            }
        } else {
            state.elevation.fill(0);
        }
        state.images_mut().topography = Some(topography.unwrap_or(definition));
    }

    fn generate_elevation(
        rng: &mut crate::math::RandomContext,
        depth: &mut [i8],
        width: usize,
        data: &crate::engine::world::planet::PlanetFrame,
    ) {
        let algo = data.type_ & 12;
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
    }

    pub struct SurfaceInputs<'a> {
        pub rng: &'a mut crate::math::RandomContext,
        pub planet: &'a crate::engine::world::planet::PlanetDesc,
        pub info: &'a crate::engine::world::planet::PlanetInfo,
        pub location: Point,
        pub width: usize,
    }

    pub fn generate_planet_surface(
        state: &mut SphereState,
        animation: &crate::engine::kernel::planet_animation::PlanetAnimation,
        pulsing_shield: bool,
        source: SurfaceInputs<'_>,
        definition: Option<crate::engine::graphics::drawable_abi::LoadedDrawable>,
    ) {
        let SurfaceInputs {
            rng,
            planet,
            info,
            location,
            width,
        } = source;
        let shielded = planet.data_index & 128 != 0;
        rng.reseed(planet.rand_seed);
        let context = LoadedContext::create(c"Plangen.TopoContext");
        let retained_context = context.lease();
        retained_context.with_selected(|| {
            init_orbit(state, width, shielded);
            let data = crate::planets::tables::PLANET_ARRAY[usize::from(planet.data_index & !128)];
            let algo = data.type_ & 12;
            let palette = if let Some(definition) = definition {
                load_definition(state, definition, width);
                None
            } else {
                let depth = &mut state.elevation;
                generate_elevation(rng, depth, width, &data);
                let topography = crate::planets::sphere::images::image(2, width, 75, 1);
                let palette = load_surface_palette(planet.data_index & !128, info.temperature);
                render_native_topography(
                    &retained_context,
                    topography.lease().context_frame(0),
                    depth,
                    width as i32,
                    75,
                    Some(palette.view()),
                )
                .expect("render planet topography");
                state.images_mut().topography = Some(topography);
                Some(palette)
            };
            if !shielded && info.atmo_density != 65535 {
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
                render_native_topography(
                    &retained_context,
                    state.zoomed_topography().context_frame(0),
                    &scaled,
                    (width * 4) as i32,
                    300,
                    palette.as_ref().map(|palette| palette.view()),
                )
                .expect("render scaled planet topography");
            }
            state.topography().context_frame(0).read_colors(
                &mut state.colors,
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
            state.generate(location, i32::from(info.axial_tilt));
            if shielded {
                let shield = create_shield(state);
                state.images_mut().shield = Some(shield);
            }
            let direction = 1 - 2 * (i32::from(info.axial_tilt) & 1);
            init_sphere_rotation(state, direction, pulsing_shield);
            if shielded {
                overlay_shield(&retained_context, state);
            }
        });
    }

    fn overlay_shield(context: &crate::graphics::context_abi::ContextLease, state: &SphereState) {
        let old = context.target();
        context.set_frame(state.topography().context_frame(0));
        context.clear_clip();
        let rect = context.clip_rect();
        context.with_draw_mode(
            c::DrawMode {
                kind: 1,
                factor: 255,
            },
            || {
                context.fill_rectangle(
                    c::Color {
                        r: 255,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                    rect,
                );
            },
        );
        context.set_frame(old);
    }
}
