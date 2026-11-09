use super::*;
use crate::engine::world::planet::PlanetDesc;
use crate::engine::{Kernel as KernelOwner, World as WorldOwner};
use crate::planets::generate::orbit_tables::NUMBER_OF_PLANET_TYPES;
use crate::planets::solar_system::{SolarSystem, SystemLocation};

use crate::graphics::context_abi::SetContext;

use crate::graphics::pixmap_abi::CaptureDrawable;

use crate::graphics::drawable_abi::CreateDrawable;

use crate::graphics::frame_abi::ClearDrawable;

use crate::planets::render::c_api::NativeOvalGfx;

use crate::planets::solarsys::GetRandomSeedForStar;

use crate::graphics::cmap_abi::GetColorMapAddress;

use crate::graphics::cmap_abi::SetColorMap;

use crate::graphics::gfx_common_abi::SetTransitionSource;

use crate::battle::process_native::engine::redraw_queue;
use crate::engine::battle::Cx;

use crate::graphics::gfx_common_abi::ScreenTransition;

use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

const MIN_ZOOM_RADIUS: i32 = (128 << 6) >> 3;
const MAX_ZOOM_RADIUS: i32 = 128 << 6;
const WORLD_TYPE_SPECIAL: u8 = 0x80;
const HIERARCHY_STARBASE: u8 = 0xff;
const SA_MATRA: u8 = 0xfe;
const PLANETS_SIZE_MASK: u32 = 0x03;
const FACING_SHIFT: u32 = 4;

#[inline]
pub(super) unsafe fn orbit_rect(
    rect: *mut c::RECT,
    dx: i32,
    dy: i32,
    radius: i32,
    xn: i32,
    yn: i32,
    den: i32,
) {
    let (width, height) = sis_dimensions();
    (*rect).corner.x =
        (i64::from(width >> 1) + (-i64::from(dx) * i64::from(xn) / i64::from(den))) as _;
    (*rect).corner.y =
        (i64::from(height >> 1) + (-i64::from(dy) * i64::from(yn) / i64::from(den))) as _;
    let orbit_width = i64::from(radius) * i64::from(xn << 1) / i64::from(den);
    (*rect).extent.width = orbit_width as _;
    (*rect).extent.height = ((*rect).extent.width >> 1) as _;
}

pub(super) unsafe fn planet_orbit_rect(
    rect: *mut c::RECT,
    planet: *mut PlanetDesc,
    xn: i32,
    yn: i32,
    den: i32,
) {
    let mut dx = i32::from((*planet).radius);
    let mut dy = dx;
    if xn > display_factor() {
        dx = (dx + i32::from((*planet).location.x)) as i16 as i32;
        dy = ((dy + i32::from((*planet).location.y)) << 1) as i16 as i32;
    }
    orbit_rect(rect, dx, dy, i32::from((*planet).radius), xn, yn, den);
}

pub(super) unsafe fn validate_orbit(
    planet: *mut PlanetDesc,
    moon: bool,
    origin: c::POINT,
    xn: i32,
    yn: i32,
    den: i32,
) {
    if xn <= display_factor() {
        let mut rect: c::RECT = std::mem::zeroed();
        planet_orbit_rect(&mut rect, planet, xn, yn, den);
        rect.corner.x += rect.extent.width >> 1;
        rect.corner.y += rect.extent.height >> 1;
        rect.corner.x = (i64::from(rect.corner.x)
            + i64::from((*planet).location.x) * i64::from(xn) / i64::from(den))
            as _;
        rect.corner.y = (i64::from(rect.corner.y)
            + i64::from((*planet).location.y) * i64::from(xn / 2) / i64::from(den))
            as _;
        (*planet).image.origin = rect.corner;
    }
    let index = (*planet).data_index & !WORLD_TYPE_SPECIAL;
    if usize::from(index) < NUMBER_OF_PLANET_TYPES {
        let typ = crate::planets::tables::PLANET_ARRAY.0[usize::from(index)].type_;
        let mut size = u32::from(typ & PLANETS_SIZE_MASK as u8);
        if xn > display_factor() {
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
        (*planet).image.frame = set_frame_index(
            ORBITAL_FRAME,
            (size << FACING_SHIFT) + u32::from(((angle + 2) >> 2) & 15),
        );
    } else if (*planet).data_index == HIERARCHY_STARBASE {
        (*planet).image.frame = set_frame_index(SpaceJunkFrame, 16);
    } else if (*planet).data_index == SA_MATRA {
        (*planet).image.frame = set_frame_index(SpaceJunkFrame, 19);
    }
}

pub(super) unsafe fn draw_orbit(
    context: &crate::graphics::context_abi::ContextLease,
    planet: *mut PlanetDesc,
    xn: i32,
    yn: i32,
    den: i32,
) {
    let mut rect: c::RECT = std::mem::zeroed();
    planet_orbit_rect(&mut rect, planet, xn, yn, den);
    context.set_foreground((*planet).temp_color);
    let mut renderer = NativeOvalGfx::new(context, c::ScreenWidth, c::ScreenHeight);
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

pub(super) unsafe fn find_radius(mut from_radius: i32, ship_loc: c::POINT) -> i32 {
    loop {
        from_radius >>= 1;
        let next = if from_radius > MIN_ZOOM_RADIUS {
            from_radius >> 1
        } else {
            0
        };
        let mut r: c::RECT = std::mem::zeroed();
        orbit_rect(
            &mut r,
            next,
            next,
            next,
            display_factor(),
            display_factor() >> 2,
            from_radius,
        );
        SCALE_RECT = r;
        let display = locationToDisplay(ship_loc, from_radius as _);
        if !point_within_rect(r, display) {
            break;
        }
    }
    from_radius
}

pub(super) unsafe fn validate_orbits(solar: &mut SolarSystem) {
    let radius = i32::from(solar.sys.sun.radius);
    for planet in &mut solar.sys.planets[..usize::from(solar.sys.sun.num_planets)] {
        validate_orbit(
            planet,
            false,
            planet.location,
            display_factor(),
            display_factor() / 4,
            radius,
        );
    }
}

pub(super) unsafe fn draw_inner_system(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
    let SystemLocation::Inner(planet) = solar.location() else {
        panic!("drawing inner system outside it");
    };
    cx.with_legacy(|| {
        validate_inner_orbits(solar);
        draw_system(solar, solar.sys.planets[usize::from(planet)].radius);
    });
    let name = std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
        .unwrap_or_else(|error| panic!("invalid inner-system name: {error}"));
    crate::ui::sis::draw_title(cx.kernel, cx.screen_dimensions().0, name);
}

pub(super) unsafe fn draw_outer_system(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
    cx.with_legacy(|| {
        validate_orbits(solar);
        draw_system(solar, solar.sys.sun.radius);
    });
    let point = cx.world.catalog()[solar.star().0].star_pt;
    crate::ui::sis::draw_hyper_coords(cx.kernel, cx.screen_dimensions().0, point);
}

pub(super) unsafe fn draw_system_transition(cx: &mut Cx<'_>, solar: &mut SolarSystem, inner: bool) {
    cx.with_legacy(|| {
        SetTransitionSource(std::ptr::null_mut());
        BatchGraphics();
    });
    if inner {
        draw_inner_system(cx, solar);
    } else {
        draw_outer_system(cx, solar);
    }
    solar.refresh_frame(cx);
    redraw_queue(cx, false);
    cx.with_legacy(|| {
        ScreenTransition(3, std::ptr::null_mut());
        UnbatchGraphics();
    });
}

pub(super) unsafe fn draw_system(solar: &mut SolarSystem, radius: c::SIZE) {
    if SOLAR_SYS_FRAME.is_null() {
        let mut clip: c::RECT = std::mem::zeroed();
        c::GetContextClipRect(&mut clip);
        SOLAR_SYS_FRAME =
            CaptureDrawable(CreateDrawable(2, clip.extent.width, clip.extent.height, 1)).cast();
    }
    let offscreen = KernelOwner::current().read(|owner| owner.off_screen_context);
    let context = crate::graphics::context_abi::ContextLease::require(offscreen);
    let old = SetContext(offscreen);
    set_frame(SOLAR_SYS_FRAME);
    set_clip(std::ptr::null_mut());
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: STARS_FRAME,
    };
    draw_stamp(&mut stamp);
    let inner_planet = match solar.location() {
        SystemLocation::Outer => None,
        SystemLocation::Inner(planet) => Some(planet),
    };
    if let Some(index) = inner_planet {
        let planet = &mut solar.sys.planets[usize::from(index)];
        draw_orbit(
            &context,
            planet,
            display_factor() * 4,
            display_factor(),
            i32::from(radius),
        );
        for moon in &mut solar.sys.moons[..usize::from(planet.num_planets)] {
            draw_orbit(&context, moon, 2, 1, 2);
        }
        draw_inner_planets(solar, index);
    } else {
        for planet in &mut solar.sys.planets[..usize::from(solar.sys.sun.num_planets)] {
            draw_orbit(
                &context,
                planet,
                display_factor(),
                display_factor() / 4,
                i32::from(radius),
            );
        }
        calc_sun_size(&mut solar.sys.sun, radius);
        for index in solar.draw_order[..=usize::from(solar.sys.sun.num_planets)]
            .iter()
            .copied()
        {
            let current = if let Some(index) = index {
                let planet = &mut solar.sys.planets[usize::from(index)];
                set_planet_color_map(planet);
                planet
            } else {
                let star_color =
                    ((*(**WorldOwner::current()).star_pointer()).star_type & 0xe0) >> 5;
                SetColorMap(GetColorMapAddress(
                    set_color_map_index(SUN_CMAP, u32::from(star_color)).cast(),
                ));
                &mut solar.sys.sun
            };
            draw_stamp(&mut current.image);
        }
    }
    SetContext(old);
    stamp.frame = SOLAR_SYS_FRAME;
    draw_stamp(&mut stamp);
}

pub(super) unsafe fn create_star_background(solar: &mut SolarSystem) -> *mut c::FRAME_DESC {
    let mut clip: c::RECT = std::mem::zeroed();
    let old = SetContext(KernelOwner::current().read(|owner| owner.space_context));
    c::GetContextClipRect(&mut clip);
    let frame = CaptureDrawable(CreateDrawable(2, clip.extent.width, clip.extent.height, 1))
        .cast::<c::FRAME_DESC>();
    SetContext(KernelOwner::current().read(|owner| owner.off_screen_context));
    set_frame(frame);
    set_clip(std::ptr::null_mut());
    set_background(c::Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });
    ClearDrawable();
    solar.sys.rng.reseed(GetRandomSeedForStar(
        (**WorldOwner::current()).star_pointer().cast(),
    ));
    let (width, height) = sis_dimensions();
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: SpaceJunkFrame,
    };
    for pieces in [5, 30] {
        for _ in 0..8 {
            for _ in 0..pieces {
                let random = solar.sys.rng.next_random();
                stamp.origin.x = (random as u16 % width as u16) as _;
                stamp.origin.y = ((random >> 16) as u16 % height as u16) as _;
                draw_stamp(&mut stamp);
            }
            stamp.frame = inc_frame(stamp.frame);
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

unsafe fn set_color_map_index(map: *mut c_void, index: u32) -> *mut c_void {
    crate::strings::string_table::SetAbsStringTableIndex(map.cast(), index as u16).cast()
}

unsafe fn set_planet_color_map(planet: *mut PlanetDesc) {
    let index = usize::from((*planet).data_index & !WORLD_TYPE_SPECIAL);
    assert!(index < NUMBER_OF_PLANET_TYPES);
    let color = crate::planets::tables::PLANET_ARRAY.0[index].type_ >> 4;
    SetColorMap(GetColorMapAddress(
        (set_color_map_index(ORBITAL_CMAP, u32::from(color))).cast(),
    ));
}

unsafe fn calc_sun_size(sun: *mut PlanetDesc, radius: c::SIZE) {
    let mut index = 0;
    if i32::from(radius) <= MAX_ZOOM_RADIUS >> 1 {
        index += 1;
        if i32::from(radius) <= MAX_ZOOM_RADIUS >> 2 {
            index += 1;
        }
    }
    let (width, height) = sis_dimensions();
    (*sun).image.origin = c::POINT {
        x: (width >> 1) as i16,
        y: (height >> 1) as i16,
    };
    (*sun).image.frame =
        crate::graphics::pixmap_abi::SetRelFrameIndex(SUN_FRAME.cast(), index).cast();
}

unsafe fn draw_inner_planets(solar: &mut SolarSystem, index: u8) {
    let planet = &mut solar.sys.planets[usize::from(index)];
    set_planet_color_map(planet);
    let (width, height) = sis_dimensions();
    let mut stamp = c::STAMP {
        origin: c::POINT {
            x: (width >> 1) as i16,
            y: (height >> 1) as i16,
        },
        frame: planet.image.frame,
    };
    let index = usize::from(planet.data_index & !WORLD_TYPE_SPECIAL);
    if index < NUMBER_OF_PLANET_TYPES && planet.data_index & WORLD_TYPE_SPECIAL != 0 {
        stamp.frame = set_frame_index(SpaceJunkFrame, 17);
    }
    draw_stamp(&mut stamp);
    for moon in &mut solar.sys.moons[..usize::from(planet.num_planets)] {
        if moon.data_index & WORLD_TYPE_SPECIAL == 0 {
            set_planet_color_map(moon);
        }
        draw_stamp(&mut moon.image);
    }
}

#[no_mangle]
pub unsafe extern "C" fn DrawStarBackGround() {
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: STARS_FRAME,
    };
    draw_stamp(&mut stamp);
}

pub(super) unsafe fn validate_inner_orbits(solar: &mut SolarSystem) {
    let SystemLocation::Inner(index) = solar.location() else {
        panic!("validating inner system outside it");
    };
    let planet = &mut solar.sys.planets[usize::from(index)];
    let origin = planet.location;
    validate_orbit(
        planet,
        false,
        origin,
        display_factor() * 4,
        display_factor(),
        i32::from(planet.radius),
    );
    for moon in &mut solar.sys.moons[..usize::from(planet.num_planets)] {
        validate_orbit(moon, true, origin, 2, 1, 2);
    }
}

mod native {

    pub(super) use crate::graphics::context_abi::SetContextClipRect;
    pub(super) use crate::graphics::drawable_abi::SetContextFGFrame;

    pub(super) use crate::graphics::context_abi::SetContextBackGroundColor;

    pub(super) use crate::graphics::frame_abi::DrawStamp;
}
unsafe fn set_frame(frame: *mut c::FRAME_DESC) -> *mut c::FRAME_DESC {
    native::SetContextFGFrame(frame.cast()).cast()
}
unsafe fn set_clip(rect: *mut c::RECT) {
    native::SetContextClipRect(rect.cast());
}
unsafe fn set_background(color: c::Color) {
    native::SetContextBackGroundColor(crate::comm::locdata::CColor {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    });
}
unsafe fn draw_stamp(stamp: *mut c::STAMP) {
    native::DrawStamp(stamp.cast());
}
unsafe fn inc_frame(frame: *mut c::FRAME_DESC) -> *mut c::FRAME_DESC {
    crate::graphics::pixmap_abi::IncFrameIndex(frame.cast()).cast()
}
unsafe fn set_frame_index(frame: *mut c::FRAME_DESC, index: u32) -> *mut c::FRAME_DESC {
    crate::graphics::pixmap_abi::SetAbsFrameIndex(frame.cast(), index as u16).cast()
}
