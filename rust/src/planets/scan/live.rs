use super::{landing_fuel, move_cursor};
use crate::battle::element::{Element, ElementFlags};
use crate::c_bindings::{controller_input::*, global_state::MENU_STATE, live_queues, solar_system};
use crate::collections::queue::{CountLinks, Queue, ReinitQueue};
use crate::comm::dispatch::{clone_ship_fragment, CShipFragment};
use crate::comm::locdata::{CColor, CPoint, CRect};
use crate::graphics::drawable_abi::SetContextFGFrame;
use crate::mainloop::restart_menu::c_extern::{
    self as base, uqm_sleep_thread_until as SleepThreadUntil, SetContext,
};
use crate::mainloop::types::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
use crate::planet_side::ffi::{
    uqm_rust_planet_side, PlanetSideReply, PlanetSideRequest, PlanetSideRunContext,
    PLANET_SIDE_ABI_VERSION, STATUS_OK,
};
use crate::starbase::cffi::CffiPlatform;
use crate::starbase::gfx::{FlashTarget, Gfx, Point, Rgba, Stamp};
use std::ffi::{c_void, CString};

mod display;

static mut SCAN_CONTEXT: *mut crate::graphics::native_abi::context_desc = std::ptr::null_mut();
static mut PLANET_LOCATION: CPoint = CPoint { x: 0, y: 0 };
static mut CURSOR_RECT: CRect = CRect {
    corner: CPoint { x: 0, y: 0 },
    width: 0,
    height: 0,
};
static mut ERASE_FRAME: *mut c_void = std::ptr::null_mut();

use crate::time::counter_abi::GetTimeCounter;

use crate::graphics::drawable_abi::GetFrameRect;

use crate::graphics::drawable_abi::GetFrameHot;

use crate::graphics::drawable_abi::SetFrameHot;

use crate::graphics::context_abi::CopyContextRect;

use crate::graphics::gfx_common_abi::DrawFromExtraScreen;

use crate::planets::solarsys::drawing::DrawStarBackGround;

use crate::planets::sphere::live::DrawDefaultPlanetSphere;

use crate::planets::sphere::live::DrawPlanet;

use crate::planets::orbit::live::DrawScannedObjects;

use crate::planets::orbit::live::RedrawPlanetOrbitDisplay;

use crate::ui::sis::DeltaSISGauges;

use crate::ui::sis::DrawStatusMessage;

use crate::battle::sounds::engine::PlayMenuSound;

use crate::mainloop::input_abi::live::SetInputCallback;

use crate::navigation::groups::live::PutGroupInfo;

use crate::planets::solarsys::lifecycle::SaveSolarSysLocation;

use crate::planet_side::init_lander::InitLander;

use crate::ui::util::live::WaitForAnyButtonUntil;

use crate::mainloop::input_abi::live::FlushInput;

use crate::ui::menu::DoMenuChooser;

use crate::ui::menu::DrawMenuStateStrings;

#[cfg(feature = "debug-process")]
use crate::graphics::context_abi::CreateContextAux;

#[cfg(not(feature = "debug-process"))]
use crate::graphics::context_abi::CreateContextAux;

use crate::graphics::context_abi::DestroyContext;

use crate::graphics::font_abi::SetContextFont;

extern "C" {
    static mut GlobData: crate::ships::native::abi::GLOBDATA;
    static mut NextActivity: u16;
    static mut SpaceContext: *mut crate::graphics::native_abi::context_desc;
    static mut PlanetContext: *mut crate::graphics::native_abi::context_desc;
    static mut Screen: *mut crate::graphics::native_abi::frame_desc;
    static mut MiscDataFrame: *mut crate::graphics::native_abi::frame_desc;
    static mut SpaceJunkFrame: *mut crate::graphics::native_abi::frame_desc;
    static mut optWhichMenu: i32;
    static mut optWhichCoarseScan: i32;
    static mut MicroFont: *mut crate::graphics::native_abi::font_desc;
    static mut TinyFont: *mut crate::graphics::native_abi::font_desc;
    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
}

fn color(rgb: Rgba) -> CColor {
    CColor {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
        a: rgb.a,
    }
}
fn black() -> CColor {
    color(Rgba::BLACK)
}
unsafe fn width() -> i16 {
    (ScreenWidth - 78) as i16
}
fn draw(frame: *mut c_void, point: CPoint, filled: bool) {
    let stamp = Stamp {
        origin: Point {
            x: point.x,
            y: point.y,
        },
        frame: frame as usize,
    };
    if filled {
        CffiPlatform.draw_filled_stamp(stamp);
    } else {
        CffiPlatform.draw_stamp(stamp);
    }
}
unsafe fn bounds(frame: *mut c_void) -> CRect {
    let mut rect = CRect::default();
    GetFrameRect((frame).cast(), (&mut rect as *mut CRect).cast());
    rect
}
unsafe fn cursor_frame() -> *mut c_void {
    (base::SetAbsFrameIndex((MiscDataFrame).cast(), 105)).cast()
}

#[no_mangle]
pub unsafe extern "C" fn RepairBackRect(rect: *mut CRect) {
    let old = CffiPlatform.context_clip_rect();
    let mut new = *rect;
    new.corner.x += old.corner.x;
    new.corner.y += old.corner.y;
    new.height += new.corner.y & 1;
    new.corner.y &= !1;
    DrawFromExtraScreen(std::ptr::from_mut(&mut new).cast());
}

unsafe fn erase_coarse() {
    SetContext(PlanetContext);
    CffiPlatform.batch();
    DrawStarBackGround();
    DrawDefaultPlanetSphere();
    CffiPlatform.unbatch();
}

unsafe fn save_location_image() {
    let hot = GetFrameHot((cursor_frame()).cast());
    base::DestroyDrawable(base::ReleaseDrawable((ERASE_FRAME).cast()));
    let mut rect = CURSOR_RECT;
    rect.corner.x -= hot.x;
    rect.corner.y -= hot.y;
    ERASE_FRAME =
        (base::CaptureDrawable(CopyContextRect(std::ptr::from_mut(&mut rect).cast()))).cast();
    SetFrameHot((ERASE_FRAME).cast(), hot);
}
unsafe fn restore_location_image() {
    draw(ERASE_FRAME, CURSOR_RECT.corner, false);
}
unsafe fn draw_cursor(filled: bool) {
    draw(cursor_frame(), CURSOR_RECT.corner, filled);
}
unsafe fn set_cursor_location(point: CPoint) {
    CURSOR_RECT.corner = CPoint {
        x: point.x >> 2,
        y: point.y >> 2,
    };
}
unsafe fn set_location(point: CPoint, restore_old: bool) {
    PLANET_LOCATION = point;
    SetContext(SCAN_CONTEXT);
    if restore_old {
        restore_location_image();
    }
    set_cursor_location(point);
    save_location_image();
}

unsafe fn flash_location() {
    static mut SHADE: u8 = 0;
    static mut DELTA: i32 = -2;
    static mut PREVIOUS: CPoint = CPoint { x: 0, y: 0 };
    static mut NEXT_TIME: u32 = 0;
    let now = GetTimeCounter();
    let changed = PREVIOUS.x != CURSOR_RECT.corner.x || PREVIOUS.y != CURSOR_RECT.corner.y;
    if !changed && now < NEXT_TIME {
        return;
    }
    if changed {
        SHADE = 0;
        DELTA = -2;
        PREVIOUS = CURSOR_RECT.corner;
        NEXT_TIME = now.wrapping_add(840 / 16);
    } else {
        if SHADE == 0 || SHADE == 0x1a {
            DELTA = -DELTA;
        }
        SHADE = (i32::from(SHADE) + DELTA) as u8;
        if now.wrapping_sub(NEXT_TIME) > 840 / 16 {
            NEXT_TIME = now.wrapping_add(840 / 16);
        } else {
            NEXT_TIME = NEXT_TIME.wrapping_add(840 / 16);
        }
    }
    SetContext(SCAN_CONTEXT);
    CffiPlatform.set_fg_color(Rgba::rgb15(SHADE, SHADE, SHADE));
    draw_cursor(true);
}

#[no_mangle]
pub unsafe extern "C" fn RedrawSurfaceScan(location: *const CPoint) {
    let old = SetContext(SCAN_CONTEXT);
    CffiPlatform.batch();
    DrawPlanet(0, black());
    DrawScannedObjects(1);
    if !location.is_null() {
        set_location(*location, false);
        draw_cursor(false);
    }
    CffiPlatform.unbatch();
    SetContext(old);
}

unsafe fn fuel() -> u16 {
    landing_fuel((*solar_system::current()).SysInfo.PlanetInfo.SurfaceGravity)
}

unsafe fn spawn_fwiffo() {
    crate::planets::c_api::set_encounter_group(0);
    PutGroupInfo(0, u8::MAX);
    ReinitQueue(std::ptr::addr_of_mut!(GlobData.Game_state.ip_group_q).cast::<Queue>());
    let queue = std::ptr::addr_of_mut!(GlobData.Game_state.npc_built_ship_q).cast::<Queue>();
    assert_eq!(CountLinks(queue), 0);
    let ship = clone_ship_fragment(crate::comm::dispatch::ship::SPATHI_SHIP, queue, 1)
        .cast::<CShipFragment>();
    if !ship.is_null() {
        (*ship).captains_name_index = 5 + 16;
    }
}

unsafe fn dispatch_lander() -> bool {
    let landing_fuel = fuel() as i16;
    erase_coarse();
    let old_callback = SetInputCallback(None);
    DeltaSISGauges(0, -landing_fuel, 0);
    SetContext(SCAN_CONTEXT);
    draw_cursor(false);
    let system = solar_system::current();
    let mut context = PlanetSideRunContext {
        solar_system: system.cast(),
        world: (*system).pOrbitalDesc.cast(),
        misc_data_frame: MiscDataFrame.cast(),
        energy_frame: (*system).PlanetSideFrame[1].cast(),
        life_frames: std::array::from_fn(|i| (*system).PlanetSideFrame[i + 3].cast()),
        landing_x: i32::from(PLANET_LOCATION.x),
        landing_y: i32::from(PLANET_LOCATION.y),
        facing: crate::math::random() as u8,
        retrieval_masks: (*system).SysInfo.PlanetInfo.ScanRetrieveMask,
        tick_period: 840 / 35,
        frame_budget: u32::MAX,
        tectonics_rating: (*system).SysInfo.PlanetInfo.Tectonics as u8,
        weather_rating: (*system).SysInfo.PlanetInfo.Weather as u8,
        temperature: i32::from((*system).SysInfo.PlanetInfo.SurfaceTemperature),
        ..PlanetSideRunContext::default()
    };
    let request = PlanetSideRequest {
        abi_version: PLANET_SIDE_ABI_VERSION,
        operation: 3,
        context: (&mut context as *mut PlanetSideRunContext).cast(),
        ..PlanetSideRequest::default()
    };
    let mut reply = PlanetSideReply::default();
    if uqm_rust_planet_side(&request, &mut reply) != STATUS_OK {
        crate::logging::log_add(
            crate::logging::LogLevel::Fatal,
            &format!("Rust PlanetSide failed with status {}", reply.status),
        );
        GlobData.Game_state.CurrentActivity |= CHECK_ABORT;
    } else {
        (*system).SysInfo.PlanetInfo.ScanRetrieveMask = context.retrieval_masks;
        if reply.detail == 4 {
            crate::logging::log_add(
                crate::logging::LogLevel::Error,
                &format!(
                    "Rust PlanetSide adapter failed (operation code {}); returning to orbit",
                    reply.value0 as u64
                ),
            );
        } else if reply.detail == 5 {
            crate::logging::log_add(
                crate::logging::LogLevel::Error,
                "Rust PlanetSide frame budget exhausted; returning to orbit",
            );
        }
    }
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT != 0 {
        return false;
    }
    if crate::state::game_state_keys::get_game_state("FOUND_PLUTO_SPATHI") == 1 {
        spawn_fwiffo();
        NextActivity |= CHECK_LOAD;
        GlobData.Game_state.CurrentActivity |= START_ENCOUNTER;
        SaveSolarSysLocation();
        return false;
    }
    RedrawPlanetOrbitDisplay();
    display::coarse();
    SetInputCallback(old_callback);
    true
}

struct PickState {
    success: bool,
}

unsafe extern "C" fn pick_input(menu: *mut MENU_STATE) -> u32 {
    let state = &mut *((*menu).privData.cast::<PickState>());
    let time_in = GetTimeCounter();
    let pulse = std::ptr::addr_of!(PulsedInputState).read();
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT != 0
        || pulse.menu[KEY_MENU_CANCEL as usize] != 0
    {
        state.success = false;
        return 0;
    } else if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
        state.success = true;
        return 0;
    }
    let current = std::ptr::addr_of!(CurrentInputState).read();
    let mut dx = 0;
    let mut dy = 0;
    if current.menu[KEY_MENU_LEFT as usize] != 0 {
        dx = -1;
    }
    if current.menu[KEY_MENU_RIGHT as usize] != 0 {
        dx = 1;
    }
    if current.menu[KEY_MENU_UP as usize] != 0 {
        dy = -1;
    }
    if current.menu[KEY_MENU_DOWN as usize] != 0 {
        dy = 1;
    }
    CffiPlatform.batch();
    let point = PLANET_LOCATION;
    let next = move_cursor((point.x, point.y), dx, dy, width());
    if next != (point.x, point.y) {
        set_location(
            CPoint {
                x: next.0,
                y: next.1,
            },
            true,
        );
    }
    flash_location();
    CffiPlatform.unbatch();
    SleepThreadUntil(time_in.wrapping_add(840 / 40));
    1
}

unsafe fn pick_side() -> bool {
    let mut menu: MENU_STATE = std::mem::zeroed();
    let mut state = PickState { success: false };
    menu.privData = (&mut state as *mut PickState).cast();
    let fuel = fuel();

    use crate::ui::sis::ClearSISRect;
    ClearSISRect(1 << 2);
    SetContext(SCAN_CONTEXT);
    CffiPlatform.batch();
    DrawPlanet(0, black());
    DrawScannedObjects(0);
    CffiPlatform.unbatch();
    let message = CString::new(format!(
        "{}{:.1}",
        CffiPlatform.game_string(435 + 5),
        f32::from(fuel) / 100.0
    ))
    .unwrap();
    DrawStatusMessage(message.as_ptr());
    set_cursor_location(PLANET_LOCATION);
    save_location_image();
    InitLander(0);
    base::SetMenuSounds(0, 16);
    crate::automation::ui_observation::rust_automation_observe_planet_menu(4);
    menu.InputFunc = Some(pick_input);
    base::DoInput((&mut menu as *mut MENU_STATE).cast(), 1);
    crate::automation::ui_observation::rust_automation_observe_planet_menu(3);
    DrawStatusMessage(std::ptr::null());
    let mut result = true;
    if state.success {
        result = dispatch_lander();
    } else {
        restore_location_image();
    }
    base::SetMenuSounds(15, 16);
    result
}

unsafe fn scanned_stuff(y: u16, scan: u16) {
    let g = CffiPlatform;
    let old_color = g.set_fg_color(Rgba::BLACK);
    let queue = live_queues::display_list();
    let mut handle = (*queue).head as usize;
    while handle != 0 {
        let element = handle as *mut Element;
        handle = (*element).succ as usize;
        let mut dy = (i32::from(y) - i32::from((*element).current.location.y)) as i16;
        if (*element).life_span as u8 != scan as u8
            || dy < 0
            || crate::planet_side::orbit_scan::node_retrieved(
                &*element,
                &(*solar_system::current()).SysInfo.PlanetInfo,
            )
        {
            continue;
        }
        (*element).state_flags |= ElementFlags::APPEARING;
        let point = CPoint {
            x: (*element).current.location.x,
            y: (*element).current.location.y,
        };
        if dy >= 8 {
            draw((*element).next.frame, point, false);
        } else {
            let gradient = (255 - 255 * dy / 7) as u8;
            let mut c = Rgba::rgb15(31, 31, 31);
            match scan {
                0 => c.r = gradient,
                1 => {
                    c.g = gradient;
                    c.b = gradient;
                }
                2 => c.g = gradient,
                _ => unreachable!("scan type"),
            }
            g.set_fg_color(c);
            let size = g
                .frame_index((*element).next.frame as usize)
                .wrapping_sub(g.frame_index((*element).current.frame as usize));
            if dy as u16 > size {
                dy = size as i16;
            }
            let frame = g.rel_frame((*element).current.frame as usize, dy) as *mut c_void;
            draw(frame, point, true);
        }
    }
    g.set_fg_color(old_color);
}

unsafe fn scan_planet(scan_type: u16) {
    let (start, end) = if scan_type == 4 {
        (0, 2)
    } else {
        (scan_type, scan_type)
    };
    let text_colors = [
        Rgba::rgb15(19, 0, 0),
        Rgba::rgb15(12, 12, 12),
        Rgba::rgb15(0, 14, 0),
    ];
    let tint_colors = [
        Rgba::rgb15(31, 0, 0),
        Rgba::rgb15(31, 31, 31),
        Rgba::rgb15(0, 31, 0),
    ];
    let mut repair = CRect::default();
    for scan in start..=end {
        let baseline_y = ScreenHeight - 13 - 75 - 7;
        SetContext(PlanetContext);
        repair = CRect {
            corner: CPoint {
                x: 0,
                y: (baseline_y - 10) as i16,
            },
            width: width(),
            height: 11,
        };
        RepairBackRect(&mut repair);
        SetContextFont((MicroFont).cast());
        CffiPlatform.set_fg_color(text_colors[usize::from(scan)]);
        display::text(
            width() >> 1,
            baseline_y as i16,
            crate::starbase::gfx::Align::Center,
            CffiPlatform.game_string(305 + i32::from(scan)),
        );
        SetContext(SCAN_CONTEXT);
        CffiPlatform.batch();
        DrawPlanet(0, black());
        CffiPlatform.unbatch();
        let tint = color(tint_colors[usize::from(scan)]);
        let mut timeout = GetTimeCounter();
        let mut line = 0;
        while line < 84 {
            timeout = timeout.wrapping_add((840 * 7 / 4) / 84);
            if WaitForAnyButtonUntil(1, timeout, 0) != 0 {
                break;
            }
            CffiPlatform.batch();
            DrawPlanet(line, tint);
            scanned_stuff(line as u16, scan);
            CffiPlatform.unbatch();
            line += 1;
        }
        if line < 84 {
            CffiPlatform.batch();
            DrawPlanet(83, tint);
            scanned_stuff(83, scan);
            CffiPlatform.unbatch();
        }
    }
    SetContext(PlanetContext);
    RepairBackRect(&mut repair);
    SetContext(SCAN_CONTEXT);
    if scan_type == 4 {
        DrawPlanet(0, black());
        DrawScannedObjects(0);
    }
    FlushInput();
}

unsafe fn unavailable() -> bool {
    let system = solar_system::current();
    (*(*system).pOrbitalDesc).data_index & 0x80 != 0
        || (*system).SysInfo.PlanetInfo.AtmoDensity == u16::MAX
}

unsafe extern "C" fn scan_input(menu: *mut MENU_STATE) -> u32 {
    let pulse = std::ptr::addr_of!(PulsedInputState).read();
    let selected = pulse.menu[KEY_MENU_SELECT as usize] != 0;
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT != 0 {
        return 0;
    }
    if pulse.menu[KEY_MENU_CANCEL as usize] != 0 || (selected && (*menu).CurState == 3) {
        return 0;
    } else if selected {
        if (*menu).CurState == 5 {
            if unavailable() {
                PlayMenuSound(2);
                return 1;
            }
            let required = fuel();
            if GlobData.SIS_state.FuelOnBoard < u32::from(required)
                || GlobData.SIS_state.NumLanders == 0
                || GlobData.SIS_state.CrewEnlisted == 0
            {
                PlayMenuSound(2);
                return 1;
            }
            CffiPlatform.set_flash(FlashTarget::Off);
            if !pick_side() {
                return 0;
            }
            DrawMenuStateStrings(7, i16::from((*menu).CurState));
            CffiPlatform.set_flash(FlashTarget::Menu3do);
            return 1;
        }
        if (*(*solar_system::current()).pOrbitalDesc).data_index & 0x80 != 0 {
            PlayMenuSound(2);
            return 1;
        }
        scan_planet(u16::from((*menu).CurState));
        if (*menu).CurState == 4 {
            (*menu).CurState = 5;
            crate::automation::ui_observation::rust_automation_observe_planet_menu(3);
            DrawMenuStateStrings(7, i16::from((*menu).CurState));
        }
    } else if optWhichMenu == 2 || !unavailable() {
        DoMenuChooser(menu, 7);
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn GetScanContext(owner: *mut i32) -> *mut c_void {
    if !SCAN_CONTEXT.is_null() {
        if !owner.is_null() {
            *owner = 0;
        }
    } else {
        if !owner.is_null() {
            *owner = 1;
        }
        let old = SetContext(SpaceContext);
        let mut rect = CffiPlatform.context_clip_rect();
        let context = new_context(c"ScanContext".as_ptr());
        SetContext((context).cast());
        SetContextFGFrame(Screen.cast());
        rect.corner.x += rect.width - width();
        rect.corner.y += rect.height - 75;
        rect.width = width();
        rect.height = 75;
        CffiPlatform.set_context_clip_rect(rect);
        SetContext(old);
        SCAN_CONTEXT = (context).cast();
    }
    (SCAN_CONTEXT).cast()
}

#[no_mangle]
pub unsafe extern "C" fn DestroyScanContext() {
    if !SCAN_CONTEXT.is_null() {
        DestroyContext(SCAN_CONTEXT);
        SCAN_CONTEXT = std::ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern "C" fn ScanSystem() {
    let mut menu: MENU_STATE = std::mem::zeroed();
    GetScanContext(std::ptr::null_mut());
    if optWhichMenu == 1 && unavailable() {
        menu.CurState = 3;
    } else {
        menu.CurState = 4;
        PLANET_LOCATION = CPoint {
            x: (width() >> 1) << 2,
            y: (75 >> 1) << 2,
        };
        let cursor = bounds(cursor_frame());
        CURSOR_RECT.width = cursor.width;
        CURSOR_RECT.height = cursor.height;
        SetContext(SCAN_CONTEXT);
        DrawScannedObjects(0);
    }
    DrawMenuStateStrings(7, i16::from(menu.CurState));
    CffiPlatform.set_flash(FlashTarget::Menu3do);
    display::coarse();
    base::SetMenuSounds(15, 16);
    menu.InputFunc = Some(scan_input);
    crate::automation::ui_observation::rust_automation_observe_planet_menu(2);
    base::DoInput((&mut menu as *mut MENU_STATE).cast(), 0);
    crate::automation::ui_observation::rust_automation_observe_planet_menu(0);
    CffiPlatform.set_flash(FlashTarget::Off);
    CffiPlatform.batch();
    SetContext(SCAN_CONTEXT);
    DrawPlanet(0, black());
    erase_coarse();
    CffiPlatform.unbatch();
    base::DestroyDrawable(base::ReleaseDrawable((ERASE_FRAME).cast()));
    ERASE_FRAME = std::ptr::null_mut();
}

#[cfg(not(test))]
unsafe fn new_context(name: *const std::ffi::c_char) -> *mut std::ffi::c_void {
    #[cfg(feature = "debug-process")]
    {
        (CreateContextAux(name)).cast()
    }
    #[cfg(not(feature = "debug-process"))]
    {
        let _ = name;
        CreateContextAux().cast()
    }
}
