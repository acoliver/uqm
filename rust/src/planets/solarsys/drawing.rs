use super::*;
use crate::planets::orbit_tables::NUMBER_OF_PLANET_TYPES;

unsafe extern "C" {
    static mut SpaceContext: *mut c_void;
    static mut OffScreenContext: *mut c_void;
    static CurStarDescPtr: *const c::STAR_DESC;
    fn SetContext(context: *mut c_void) -> *mut c_void;
    fn CaptureDrawable(drawable: *mut c_void) -> *mut c_void;
    fn CreateDrawable(flags: u8, width: i16, height: i16, frames: u16) -> *mut c_void;
    fn ClearDrawable();
    fn DrawOval(rect: *mut c::RECT, thickness: u8);
    fn GetRandomSeedForStar(star: *const c::STAR_DESC) -> c::DWORD;
    fn DrawSISTitle(name: *const i8);
    fn DrawHyperCoords(point: c::POINT);
    fn GetColorMapAddress(map: *mut c_void) -> *mut c_void;
    fn SetColorMap(map: *mut c_void) -> i32;
    fn SetTransitionSource(rect: *mut c_void);
    fn RedrawQueue(force: c::BOOLEAN);
    fn ScreenTransition(kind: i32, rect: *const c_void);
}
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

const MIN_ZOOM_RADIUS: i32 = (128 << 6) >> 3;
const MAX_ZOOM_RADIUS: i32 = 128 << 6;
const WORLD_TYPE_SPECIAL: u8 = 0x80;
const HIERARCHY_STARBASE: u8 = 0xff;
const SA_MATRA: u8 = 0xfe;
const PLANETS_SIZE_MASK: u32 = 0x03;
const FACING_SHIFT: u32 = 4;

#[inline]
pub(super) unsafe fn system() -> *mut c::SOLARSYS_STATE {
    crate::c_bindings::solar_system::current()
}

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
    planet: *mut c::PLANET_DESC,
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

pub(super) unsafe fn validate_orbit(planet: *mut c::PLANET_DESC, xn: i32, yn: i32, den: i32) {
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
        let typ = crate::planets::ffi::plan_data()[usize::from(index)].Type;
        let mut size = u32::from(typ & PLANETS_SIZE_MASK as u8);
        if xn > display_factor() {
            size += 3;
        } else if (*planet).pPrevDesc != (*system()).SunDesc.as_mut_ptr() {
            size += 2;
        } else if den <= (MAX_ZOOM_RADIUS >> 2) {
            size += 1;
            if den == MIN_ZOOM_RADIUS {
                size += 1;
            }
        }
        let origin = if (*planet).pPrevDesc == (*system()).SunDesc.as_mut_ptr() {
            (*planet).location
        } else {
            (*(*planet).pPrevDesc).location
        };
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

pub(super) unsafe fn draw_orbit(planet: *mut c::PLANET_DESC, xn: i32, yn: i32, den: i32) {
    let mut rect: c::RECT = std::mem::zeroed();
    planet_orbit_rect(&mut rect, planet, xn, yn, den);
    set_foreground((*planet).temp_color);
    DrawOval(&mut rect, 1);
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

pub(super) unsafe fn validate_orbits() {
    let s = system();
    let count = (*s).SunDesc[0].NumPlanets;
    for i in 0..usize::from(count) {
        validate_orbit(
            (*s).PlanetDesc.as_mut_ptr().add(i),
            display_factor(),
            display_factor() / 4,
            i32::from((*s).SunDesc[0].radius),
        );
    }
}

pub(super) unsafe fn draw_inner_system() {
    let s = system();
    let planet = (*s).pOrbitalDesc;
    validate_inner_orbits();
    draw_system((*planet).radius, true);
    extern "C" {
        static mut GlobData: crate::comm::locdata::CGlobData;
    }
    DrawSISTitle(
        std::ptr::addr_of_mut!((*std::ptr::addr_of_mut!(GlobData)).sis_state.planet_name).cast(),
    );
}

pub(super) unsafe fn draw_outer_system() {
    validate_orbits();
    let s = system();
    draw_system((*s).SunDesc[0].radius, false);
    DrawHyperCoords((*CurStarDescPtr).star_pt);
}

pub(super) unsafe fn draw_system_transition(inner: bool) {
    SetTransitionSource(std::ptr::null_mut());
    BatchGraphics();
    if inner {
        draw_inner_system();
    } else {
        draw_outer_system();
    }
    RedrawQueue(0);
    ScreenTransition(3, std::ptr::null_mut());
    UnbatchGraphics();
}

pub(super) unsafe fn draw_system(radius: c::SIZE, inner: bool) {
    let s = system();
    if SOLAR_SYS_FRAME.is_null() {
        let mut clip: c::RECT = std::mem::zeroed();
        c::GetContextClipRect(&mut clip);
        SOLAR_SYS_FRAME =
            CaptureDrawable(CreateDrawable(2, clip.extent.width, clip.extent.height, 1)).cast();
    }
    let old = SetContext(OffScreenContext);
    set_frame(SOLAR_SYS_FRAME);
    set_clip(std::ptr::null_mut());
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: STARS_FRAME,
    };
    draw_stamp(&mut stamp);
    let base = (*s).pBaseDesc;
    if inner {
        draw_orbit(
            (*s).pOrbitalDesc,
            display_factor() * 4,
            display_factor(),
            i32::from(radius),
        );
    }
    for i in 0..usize::from((*(*base).pPrevDesc).NumPlanets) {
        let current = base.add(i);
        if inner {
            draw_orbit(current, 2, 1, 2);
        } else {
            draw_orbit(
                current,
                display_factor(),
                display_factor() / 4,
                i32::from(radius),
            );
        }
    }
    if inner {
        draw_inner_planets((*s).pOrbitalDesc);
    } else {
        calc_sun_size((*s).SunDesc.as_mut_ptr(), radius);
        let mut index = (*s).FirstPlanetIndex;
        loop {
            let current = (*s)
                .PlanetDesc
                .as_mut_ptr()
                .offset(i32::from(index) as isize);
            if current == (*s).SunDesc.as_mut_ptr() {
                let star_color = ((*CurStarDescPtr).Type & 0xe0) >> 5;
                SetColorMap(GetColorMapAddress(set_color_map_index(
                    SUN_CMAP,
                    u32::from(star_color),
                )));
            } else {
                set_planet_color_map(current);
            }
            draw_stamp(&mut (*current).image);
            if index == (*s).LastPlanetIndex {
                break;
            }
            index = (*current).NextIndex as i16;
        }
    }
    SetContext(old);
    stamp.frame = SOLAR_SYS_FRAME;
    draw_stamp(&mut stamp);
}

pub(super) unsafe fn create_star_background() -> *mut c::FRAME_DESC {
    let mut clip: c::RECT = std::mem::zeroed();
    let old = SetContext(SpaceContext);
    c::GetContextClipRect(&mut clip);
    let frame = CaptureDrawable(CreateDrawable(2, clip.extent.width, clip.extent.height, 1))
        .cast::<c::FRAME_DESC>();
    SetContext(OffScreenContext);
    set_frame(frame);
    set_clip(std::ptr::null_mut());
    set_background(c::Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    });
    ClearDrawable();
    (*SysGenRNG).reseed(GetRandomSeedForStar(CurStarDescPtr));
    let (width, height) = sis_dimensions();
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: SpaceJunkFrame,
    };
    for pieces in [5, 30] {
        for _ in 0..8 {
            for _ in 0..pieces {
                let random = (*SysGenRNG).next_random();
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

pub(super) unsafe fn sort_planet_positions() {
    let s = system();
    let count = usize::from((*s).SunDesc[0].NumPlanets);
    let mut order = [0_i16; 17];
    for (i, slot) in order.iter_mut().enumerate().take(count + 1) {
        *slot = i as i16 - 1;
    }
    for i in 0..=count {
        for j in (i + 1..=count).rev() {
            let a = order[i];
            let b = order[j];
            if (*(*s).PlanetDesc.as_mut_ptr().offset(isize::from(a)))
                .image
                .origin
                .y
                > (*(*s).PlanetDesc.as_mut_ptr().offset(isize::from(b)))
                    .image
                    .origin
                    .y
            {
                order.swap(i, j);
            }
        }
    }
    (*s).FirstPlanetIndex = order[0] as _;
    (*s).LastPlanetIndex = order[count] as _;
    for i in 0..=count {
        let next = if i < count { order[i + 1] } else { 0 };
        (*(*s).PlanetDesc.as_mut_ptr().offset(isize::from(order[i]))).NextIndex = next as _;
    }
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

unsafe fn set_planet_color_map(planet: *mut c::PLANET_DESC) {
    let index = usize::from((*planet).data_index & !WORLD_TYPE_SPECIAL);
    assert!(index < NUMBER_OF_PLANET_TYPES);
    let color = crate::planets::ffi::plan_data()[index].Type >> 4;
    SetColorMap(GetColorMapAddress(set_color_map_index(
        ORBITAL_CMAP,
        u32::from(color),
    )));
}

unsafe fn calc_sun_size(sun: *mut c::PLANET_DESC, radius: c::SIZE) {
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

unsafe fn draw_inner_planets(planet: *mut c::PLANET_DESC) {
    set_planet_color_map(planet);
    let (width, height) = sis_dimensions();
    let mut stamp = c::STAMP {
        origin: c::POINT {
            x: (width >> 1) as i16,
            y: (height >> 1) as i16,
        },
        frame: (*planet).image.frame,
    };
    let index = usize::from((*planet).data_index & !WORLD_TYPE_SPECIAL);
    if index < NUMBER_OF_PLANET_TYPES && (*planet).data_index & WORLD_TYPE_SPECIAL != 0 {
        stamp.frame = set_frame_index(SpaceJunkFrame, 17);
    }
    draw_stamp(&mut stamp);
    let system = system();
    for i in 0..usize::from((*planet).NumPlanets) {
        let moon = (*system).MoonDesc.as_mut_ptr().add(i);
        if (*moon).data_index & WORLD_TYPE_SPECIAL == 0 {
            set_planet_color_map(moon);
        }
        draw_stamp(&mut (*moon).image);
    }
}

#[no_mangle]
unsafe extern "C" fn DrawStarBackGround() {
    let mut stamp = c::STAMP {
        origin: c::POINT { x: 0, y: 0 },
        frame: STARS_FRAME,
    };
    draw_stamp(&mut stamp);
}

pub(super) unsafe fn validate_inner_orbits() {
    assert!(playerInInnerSystem());
    let s = system();
    let planet = (*s).pOrbitalDesc;
    validate_orbit(
        planet,
        display_factor() * 4,
        display_factor(),
        i32::from((*planet).radius),
    );
    for i in 0..usize::from((*planet).NumPlanets) {
        validate_orbit((*s).MoonDesc.as_mut_ptr().add(i), 2, 1, 2);
    }
}

mod native {
    use crate::comm::locdata::{CColor, CStamp};
    pub(super) use crate::graphics::context_abi::SetContextClipRect;
    pub(super) use crate::graphics::drawable_abi::SetContextFGFrame;
    extern "C" {
        pub(super) fn SetContextBackGroundColor(color: CColor) -> CColor;
        pub(super) fn SetContextForeGroundColor(color: CColor) -> CColor;
        pub(super) fn DrawStamp(stamp: *mut CStamp);
    }
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
unsafe fn set_foreground(color: c::Color) {
    native::SetContextForeGroundColor(crate::comm::locdata::CColor {
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
