use super::*;
use crate::engine::battle::Cx;
use crate::engine::kernel::solar_resources::SolarResources;
use crate::engine::world::planet::PlanetDesc;
use crate::engine::{Kernel, World};
use crate::graphics::context_abi::{ContextFrame, ContextLease, SetContext};
use crate::graphics::drawable_abi::LoadedDrawable;
use crate::graphics::frame_abi::ClearDrawable;
use crate::graphics::gfx_common_abi::{
    BatchGraphics, ScreenTransition, SetTransitionSource, UnbatchGraphics,
};
use crate::planets::generate::orbit_tables::NUMBER_OF_PLANET_TYPES;
use crate::planets::render::c_api::NativeOvalGfx;
use crate::planets::solar_coords::{star_seed, SolarCoordinates};
use crate::planets::solar_system::{SolarSystem, SystemLocation};

const MIN_ZOOM_RADIUS: i32 = (128 << 6) >> 3;
const MAX_ZOOM_RADIUS: i32 = 128 << 6;
const WORLD_TYPE_SPECIAL: u8 = 0x80;
const HIERARCHY_STARBASE: u8 = 0xff;
const SA_MATRA: u8 = 0xfe;
const PLANETS_SIZE_MASK: u32 = 0x03;
const FACING_SHIFT: u32 = 4;

pub(super) fn orbit_rect(
    coordinates: SolarCoordinates,
    dx: i32,
    dy: i32,
    radius: i32,
    xn: i32,
    yn: i32,
    den: i32,
) -> c::RECT {
    let (width, height) = coordinates.dimensions();
    let orbit_width = (i64::from(radius) * i64::from(xn << 1) / i64::from(den)) as i16;
    c::RECT {
        corner: c::POINT {
            x: (i64::from(width >> 1) - i64::from(dx) * i64::from(xn) / i64::from(den)) as i16,
            y: (i64::from(height >> 1) - i64::from(dy) * i64::from(yn) / i64::from(den)) as i16,
        },
        extent: c::EXTENT {
            width: orbit_width,
            height: orbit_width >> 1,
        },
    }
}

fn planet_orbit_rect(
    coordinates: SolarCoordinates,
    planet: &PlanetDesc,
    xn: i32,
    yn: i32,
    den: i32,
) -> c::RECT {
    let mut dx = i32::from(planet.radius);
    let mut dy = dx;
    if xn > coordinates.display_factor() {
        dx = (dx + i32::from(planet.location.x)) as i16 as i32;
        dy = ((dy + i32::from(planet.location.y)) << 1) as i16 as i32;
    }
    orbit_rect(coordinates, dx, dy, i32::from(planet.radius), xn, yn, den)
}

fn validate_orbit(
    coordinates: SolarCoordinates,
    assets: &SolarResources,
    planet: &mut PlanetDesc,
    moon: bool,
    origin: c::POINT,
    scale: (i32, i32, i32),
) {
    let (xn, yn, den) = scale;
    if xn <= coordinates.display_factor() {
        let mut rect = planet_orbit_rect(coordinates, planet, xn, yn, den);
        rect.corner.x += rect.extent.width >> 1;
        rect.corner.y += rect.extent.height >> 1;
        rect.corner.x = (i64::from(rect.corner.x)
            + i64::from(planet.location.x) * i64::from(xn) / i64::from(den))
            as _;
        rect.corner.y = (i64::from(rect.corner.y)
            + i64::from(planet.location.y) * i64::from(xn / 2) / i64::from(den))
            as _;
        planet.image.origin = rect.corner;
    }
    let index = planet.data_index & !WORLD_TYPE_SPECIAL;
    if usize::from(index) < NUMBER_OF_PLANET_TYPES {
        let typ = crate::planets::tables::PLANET_ARRAY.0[usize::from(index)].type_;
        let mut size = u32::from(typ & PLANETS_SIZE_MASK as u8);
        if xn > coordinates.display_factor() {
            size += 3;
        } else if moon {
            size += 2;
        } else if den <= (MAX_ZOOM_RADIUS >> 2) {
            size += 1;
            if den == MIN_ZOOM_RADIUS {
                size += 1;
            }
        }
        let angle = crate::math::arctan(origin.x, origin.y);
        planet.image.frame = resources::frame(
            &assets.orbital,
            ((size << FACING_SHIFT) + u32::from(((angle + 2) >> 2) & 15)) as u16,
        )
        .cast();
    } else if planet.data_index == HIERARCHY_STARBASE {
        planet.image.frame = resources::frame(&assets.junk, 16).cast();
    } else if planet.data_index == SA_MATRA {
        planet.image.frame = resources::frame(&assets.junk, 19).cast();
    }
}

unsafe fn draw_orbit(
    coordinates: SolarCoordinates,
    context: &ContextLease,
    planet: &PlanetDesc,
    xn: i32,
    yn: i32,
    den: i32,
) {
    let rect = planet_orbit_rect(coordinates, planet, xn, yn, den);
    context.set_foreground(planet.temp_color);
    let (width, height) = coordinates.dimensions();
    let mut renderer = NativeOvalGfx::new(context, width + 78, height + 13);
    let rectangle = crate::comm::locdata::CRect {
        corner: crate::comm::locdata::CPoint {
            x: rect.corner.x,
            y: rect.corner.y,
        },
        width: rect.extent.width,
        height: rect.extent.height,
    };
    crate::planets::render::oval::draw_oval(&rectangle, 1, &mut renderer);
}

pub(super) fn find_radius(
    solar: &mut SolarSystem,
    mut from_radius: i32,
    ship_loc: c::POINT,
) -> i32 {
    let coordinates = solar.coordinates();
    loop {
        from_radius >>= 1;
        let next = if from_radius > MIN_ZOOM_RADIUS {
            from_radius >> 1
        } else {
            0
        };
        let rect = orbit_rect(
            coordinates,
            next,
            next,
            next,
            coordinates.display_factor(),
            coordinates.display_factor() >> 2,
            from_radius,
        );
        solar.scale_rect = rect;
        let display = coordinates.to_display(ship_loc, from_radius as _);
        if !point_within_rect(rect, display) {
            break;
        }
    }
    from_radius
}

pub(super) fn validate_orbits(assets: &SolarResources, solar: &mut SolarSystem) {
    let coordinates = solar.coordinates();
    let radius = i32::from(solar.sys.sun.radius);
    for planet in &mut solar.sys.planets[..usize::from(solar.sys.sun.num_planets)] {
        validate_orbit(
            coordinates,
            assets,
            planet,
            false,
            planet.location,
            (
                coordinates.display_factor(),
                coordinates.display_factor() / 4,
                radius,
            ),
        );
    }
}

pub(super) unsafe fn draw_inner_system(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
    validate_inner_orbits(resources::retained(cx.kernel), solar);
    let SystemLocation::Inner(index) = solar.location() else {
        panic!("drawing inner system outside it");
    };
    draw_system(
        cx.world,
        cx.kernel,
        solar,
        solar.sys.planets[usize::from(index)].radius,
    );
    let name = std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
        .unwrap_or_else(|error| panic!("invalid inner-system name: {error}"));
    crate::ui::sis::draw_title(cx.kernel, cx.screen_dimensions().0, name);
}

pub(super) unsafe fn draw_outer_system(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
    validate_orbits(resources::retained(cx.kernel), solar);
    draw_system(cx.world, cx.kernel, solar, solar.sys.sun.radius);
    let point = cx.world.catalog()[solar.star().0].star_pt;
    crate::ui::sis::draw_hyper_coords(cx.kernel, cx.screen_dimensions().0, point);
}

pub(super) unsafe fn draw_system_transition(cx: &mut Cx<'_>, solar: &mut SolarSystem, inner: bool) {
    cx.with_legacy(|| {
        SetTransitionSource(ptr::null_mut());
        BatchGraphics();
    });
    if inner {
        draw_inner_system(cx, solar);
    } else {
        draw_outer_system(cx, solar);
    }
    solar.refresh_frame(cx);
    crate::battle::process_native::engine::redraw_queue(cx, false);
    cx.with_legacy(|| {
        ScreenTransition(3, ptr::null_mut());
        UnbatchGraphics();
    });
}

unsafe fn draw_system(world: &World, kernel: &Kernel, solar: &mut SolarSystem, radius: c::SIZE) {
    let assets = resources::retained(kernel);
    let coordinates = solar.coordinates();
    let image = solar.system_image.get_or_insert_with(|| {
        let mut clip = c::RECT::default();
        c::GetContextClipRect(&mut clip);
        LoadedDrawable::create(2, clip.extent.width, clip.extent.height, 1)
            .unwrap_or_else(|| panic!("solar-system drawable allocation failed"))
    });
    let image = resources::frame(image, 0).cast();
    let context = ContextLease::require(kernel.off_screen_context);
    let old = SetContext(kernel.off_screen_context);
    set_frame(image);
    set_clip(ptr::null_mut());
    let background = solar
        .stars
        .as_ref()
        .unwrap_or_else(|| panic!("star background is not initialized"));
    draw_star_background(&context, &background.lease().context_frame(0));
    if let SystemLocation::Inner(index) = solar.location() {
        let planet = &mut solar.sys.planets[usize::from(index)];
        draw_orbit(
            coordinates,
            &context,
            planet,
            coordinates.display_factor() * 4,
            coordinates.display_factor(),
            i32::from(radius),
        );
        for moon in &solar.sys.moons[..usize::from(planet.num_planets)] {
            draw_orbit(coordinates, &context, moon, 2, 1, 2);
        }
        draw_inner_planets(assets, solar, index);
    } else {
        for planet in &solar.sys.planets[..usize::from(solar.sys.sun.num_planets)] {
            draw_orbit(
                coordinates,
                &context,
                planet,
                coordinates.display_factor(),
                coordinates.display_factor() / 4,
                i32::from(radius),
            );
        }
        calc_sun_size(coordinates, assets, &mut solar.sys.sun, radius);
        for index in solar.draw_order[..=usize::from(solar.sys.sun.num_planets)]
            .iter()
            .copied()
        {
            let current = if let Some(index) = index {
                let planet = &solar.sys.planets[usize::from(index)];
                set_planet_color_map(assets, planet);
                planet
            } else {
                assets.set_sun_palette(usize::from(
                    (world.catalog()[solar.star().0].star_type & 0xe0) >> 5,
                ));
                &solar.sys.sun
            };
            draw_stamp(&current.image);
        }
    }
    SetContext(old);
    draw_stamp(&c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: image,
    });
}

pub(super) unsafe fn create_star_background(
    world: &World,
    kernel: &Kernel,
    solar: &mut SolarSystem,
) -> LoadedDrawable {
    let coordinates = solar.coordinates();
    let assets = resources::retained(kernel);
    let old = SetContext(kernel.space_context);
    let mut clip = c::RECT::default();
    c::GetContextClipRect(&mut clip);
    let frame = LoadedDrawable::create(2, clip.extent.width, clip.extent.height, 1)
        .unwrap_or_else(|| panic!("star-background drawable allocation failed"));
    SetContext(kernel.off_screen_context);
    set_frame(resources::frame(&frame, 0).cast());
    set_clip(ptr::null_mut());
    set_background(c::Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });
    ClearDrawable();
    solar
        .sys
        .rng
        .reseed(star_seed(world.catalog()[solar.star().0].star_pt));
    let (width, height) = coordinates.dimensions();
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: resources::frame(&assets.junk, 0).cast(),
    };
    for pieces in [5, 30] {
        for _ in 0..8 {
            for _ in 0..pieces {
                let random = solar.sys.rng.next_random();
                stamp.origin.x = (random as u16 % width as u16) as _;
                stamp.origin.y = ((random >> 16) as u16 % height as u16) as _;
                draw_stamp(&stamp);
            }
            stamp.frame = crate::graphics::pixmap_abi::IncFrameIndex(stamp.frame.cast()).cast();
        }
    }
    SetContext(old);
    frame
}

pub(super) fn sort_planet_positions(solar: &mut SolarSystem) {
    let count = usize::from(solar.sys.sun.num_planets);
    let mut order = [None; 17];
    for (index, slot) in order[1..=count].iter_mut().enumerate() {
        *slot = Some(index as u8);
    }
    let y = |index: Option<u8>| {
        index.map_or(solar.sys.sun.image.origin.y, |index| {
            solar.sys.planets[usize::from(index)].image.origin.y
        })
    };
    for i in 0..=count {
        for j in (i + 1..=count).rev() {
            if y(order[i]) > y(order[j]) {
                order.swap(i, j);
            }
        }
    }
    solar.draw_order = order;
}

pub(super) fn point_within_rect(rect: c::RECT, point: c::POINT) -> bool {
    point.x >= rect.corner.x
        && point.y >= rect.corner.y
        && i32::from(point.x) < i32::from(rect.corner.x) + i32::from(rect.extent.width)
        && i32::from(point.y) < i32::from(rect.corner.y) + i32::from(rect.extent.height)
}

fn set_planet_color_map(assets: &SolarResources, planet: &PlanetDesc) {
    let index = usize::from(planet.data_index & !WORLD_TYPE_SPECIAL);
    assert!(index < NUMBER_OF_PLANET_TYPES);
    assets.set_orbital_palette(usize::from(
        crate::planets::tables::PLANET_ARRAY.0[index].type_ >> 4,
    ));
}

fn calc_sun_size(
    coordinates: SolarCoordinates,
    assets: &SolarResources,
    sun: &mut PlanetDesc,
    radius: c::SIZE,
) {
    let mut index = 0;
    if i32::from(radius) <= MAX_ZOOM_RADIUS >> 1 {
        index += 1;
        if i32::from(radius) <= MAX_ZOOM_RADIUS >> 2 {
            index += 1;
        }
    }
    let (width, height) = coordinates.dimensions();
    sun.image.origin = c::POINT {
        x: (width >> 1) as i16,
        y: (height >> 1) as i16,
    };
    sun.image.frame = resources::frame(&assets.sun, u16::from(sun.data_index) + index).cast();
}

unsafe fn draw_inner_planets(assets: &SolarResources, solar: &SolarSystem, index: u8) {
    let coordinates = solar.coordinates();
    let planet = &solar.sys.planets[usize::from(index)];
    set_planet_color_map(assets, planet);
    let (width, height) = coordinates.dimensions();
    let mut stamp = c::STAMP {
        origin: c::POINT {
            x: (width >> 1) as i16,
            y: (height >> 1) as i16,
        },
        frame: planet.image.frame,
    };
    let index = usize::from(planet.data_index & !WORLD_TYPE_SPECIAL);
    if index < NUMBER_OF_PLANET_TYPES && planet.data_index & WORLD_TYPE_SPECIAL != 0 {
        stamp.frame = resources::frame(&assets.junk, 17).cast();
    }
    draw_stamp(&stamp);
    for moon in &solar.sys.moons[..usize::from(planet.num_planets)] {
        if moon.data_index & WORLD_TYPE_SPECIAL == 0 {
            set_planet_color_map(assets, moon);
        }
        draw_stamp(&moon.image);
    }
}

pub(crate) fn draw_star_background(context: &ContextLease, background: &ContextFrame) {
    context.draw_stamp(
        background,
        crate::graphics::native_abi::POINT { x: 0, y: 0 },
    );
}

pub(super) fn validate_inner_orbits(assets: &SolarResources, solar: &mut SolarSystem) {
    let coordinates = solar.coordinates();
    let SystemLocation::Inner(index) = solar.location() else {
        panic!("validating inner system outside it");
    };
    let planet = &mut solar.sys.planets[usize::from(index)];
    let origin = planet.location;
    validate_orbit(
        coordinates,
        assets,
        planet,
        false,
        origin,
        (
            coordinates.display_factor() * 4,
            coordinates.display_factor(),
            i32::from(planet.radius),
        ),
    );
    for moon in &mut solar.sys.moons[..usize::from(planet.num_planets)] {
        validate_orbit(coordinates, assets, moon, true, origin, (2, 1, 2));
    }
}

unsafe fn set_frame(frame: *mut c::FRAME_DESC) {
    crate::graphics::drawable_abi::SetContextFGFrame(frame.cast());
}
unsafe fn set_clip(rect: *mut c::RECT) {
    crate::graphics::context_abi::SetContextClipRect(rect.cast());
}
unsafe fn set_background(color: c::Color) {
    crate::graphics::context_abi::SetContextBackGroundColor(crate::comm::locdata::CColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    });
}
unsafe fn draw_stamp(stamp: &c::STAMP) {
    let mut stamp = *stamp;
    crate::graphics::frame_abi::DrawStamp(std::ptr::from_mut(&mut stamp).cast());
}
