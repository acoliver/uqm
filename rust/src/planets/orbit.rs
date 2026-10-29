//! Orbit display, planet resource ownership, and menu from `planets.c`.

fn submenu_selection(current: u8, selected: bool, pc_menu: bool) -> u8 {
    if selected && !pc_menu {
        6
    } else {
        current
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::submenu_selection;
    use crate::c_bindings::{
        global_state::{MENU_STATE, PLANET_INFO},
        solar_system,
    };
    use crate::comm::locdata::CRect;
    use crate::graphics::drawable_abi::SetContextFGFrame;
    use crate::mainloop::restart_menu::c_extern::{self as base, SetContext};
    use crate::mainloop::types::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
    use crate::memory::rust_hfree;
    use crate::planet_side::{orbit_music, orbit_scan};
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{ContextId, FlashTarget, Gfx, Point, Rect, Rgba, Stamp};
    use std::ffi::c_void;

    #[no_mangle]
    static mut PlanetContext: *mut crate::graphics::native_abi::context_desc = std::ptr::null_mut();

    use crate::ui::border::DrawSISFrame;
    use crate::ui::sis::RepairSISBorder;

    #[cfg(feature = "debug-process")]
    use crate::graphics::context_abi::CreateContextAux;

    #[cfg(not(feature = "debug-process"))]
    use crate::graphics::context_abi::CreateContextAux;

    use crate::graphics::context_abi::DestroyContext;

    use crate::planets::scan::live::GetScanContext;

    use crate::planets::scan::live::DestroyScanContext;

    use crate::planets::solarsys::drawing::DrawStarBackGround;

    use crate::ui::sis::DrawSISMessage;

    use crate::ui::sis::DrawSISTitle;

    use crate::graphics::gfx_common_abi::LoadIntoExtraScreen;

    use crate::planets::sphere::live::DrawDefaultPlanetSphere;

    use crate::planets::sphere::live::DrawPlanet;

    use crate::planets::sphere::live::ZoomInPlanetSphere;

    use crate::planets::sphere::live::UninitSphereRotation;

    use crate::planets::sphere::live::RotatePlanetSphere;

    use crate::planets::plangen::live::GeneratePlanetSurface;

    use crate::sound::heart_ffi::PLRPlaying;

    use crate::strings::string_table::ReleaseStringTable;

    use crate::strings::string_table::DestroyStringTable;

    use crate::graphics::font_abi::DestroyFont;

    use crate::planets::scan::live::ScanSystem;

    use crate::planets::devices::DevicesMenu;

    use crate::planets::cargo::live::CargoMenu;

    use crate::planets::roster::live::RosterMenu;

    use crate::front_end::gameopt::runtime::GameOptions;

    use crate::planets::pstarmap::StarMap;

    use crate::ui::menu::DoMenuChooser;

    use crate::ui::menu::DrawMenuStateStrings;

    use crate::mainloop::input_abi::live::SetInputCallback;

    extern "C" {
        static mut GlobData: crate::ships::native::abi::GLOBDATA;
        static mut LastActivity: u16;
        static mut SpaceContext: *mut crate::graphics::native_abi::context_desc;
        static mut Screen: *mut crate::graphics::native_abi::frame_desc;
        static mut optWhichMenu: i32;
    }

    fn black() -> crate::comm::locdata::CColor {
        crate::comm::locdata::CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    unsafe fn create_context() {
        assert!(PlanetContext.is_null());
        let old = SetContext(SpaceContext);
        let mut r = CffiPlatform.context_clip_rect();
        PlanetContext = (new_context(c"PlanetContext".as_ptr())).cast();
        SetContext(PlanetContext);
        SetContextFGFrame(Screen.cast());
        r.height -= 75 + 5;
        CffiPlatform.set_context_clip_rect(r);
        SetContext(old);
    }

    #[no_mangle]
    pub unsafe extern "C" fn DrawScannedObjects(reversed: u32) {
        crate::app::bridge::enter(|cx| {
            let mut handle = if reversed != 0 {
                cx.elements().tail()
            } else {
                cx.elements().head()
            };
            while let Some(element) = handle {
                handle = if reversed != 0 {
                    cx.elements().pred(element)
                } else {
                    cx.elements().succ(element)
                };
                let state = cx.element(element);
                if crate::planet_side::orbit_scan::node_visible(
                    state.flags,
                    state.life,
                    &(*solar_system::current()).SysInfo.PlanetInfo,
                ) {
                    CffiPlatform.draw_stamp(Stamp {
                        origin: Point {
                            x: state.current.location.x,
                            y: state.current.location.y,
                        },
                        frame: state.next.image.frame.raw() as usize,
                    });
                }
            }
        });
    }

    #[no_mangle]
    unsafe extern "C" fn DrawPlanetSurfaceBorder() {
        let g = CffiPlatform;
        let old = g.set_context(ContextId::Space);
        let old_clip = g.context_clip_rect();
        let mut clip = old_clip;
        clip.corner.x -= 1;
        clip.width += 2;
        clip.height += 1;
        g.set_context_clip_rect(clip);
        g.batch();
        g.set_fg_color(Rgba::rgb15(10, 10, 10));
        let mut r = Rect {
            corner: Point {
                x: 0,
                y: clip.height - 75 - 5,
            },
            width: clip.width,
            height: 3,
        };
        g.fill_rect(r);
        g.set_fg_color(Rgba::rgb15(8, 8, 8));
        r.width -= 1;
        r.height = 1;
        r.corner.x = 1;
        r.corner.y -= 1;
        g.fill_rect(r);
        r.width = 1;
        r.height = 77;
        r.corner.y += 4;
        r.corner.x = clip.width - 1;
        g.fill_rect(r);
        g.set_fg_color(Rgba::rgb15(16, 16, 16));
        r.corner.x -= g.screen_size().0 as i16 - 78 + 1;
        g.fill_rect(r);
        r.width = g.screen_size().0 as i16 - 78 + 2;
        r.height = 1;
        g.fill_rect(r);
        g.unbatch();
        g.set_context_clip_rect(old_clip);
        g.restore_context(old);
    }

    const FULL: u8 = 0;
    const WAIT: u8 = 1;
    const UPDATE: u8 = 2;

    unsafe fn draw_display(mode: u8) {
        let g = CffiPlatform;
        g.set_context(ContextId::Space);
        let clip = g.context_clip_rect();
        let mut r = CRect {
            corner: crate::comm::locdata::CPoint {
                x: clip.corner.x,
                y: clip.corner.y,
            },
            width: clip.width,
            height: clip.height,
        };
        g.batch();
        if mode != UPDATE {
            base::SetTransitionSource(std::ptr::null_mut());
            DrawSISFrame();
            DrawSISMessage(std::ptr::null());
            DrawSISTitle(std::ptr::addr_of!(GlobData.SIS_state.PlanetName).cast());
            DrawStarBackGround();
            DrawPlanetSurfaceBorder();
        }
        if mode == WAIT {
            SetContext((GetScanContext(std::ptr::null_mut())).cast());
            let frame = base::CaptureDrawable(
                (base::LoadGraphicInstance(c"graphics.orbitenter".as_ptr())).cast(),
            );
            g.draw_stamp(Stamp {
                origin: Point { x: 0, y: 0 },
                frame: frame as usize,
            });
            base::DestroyDrawable(base::ReleaseDrawable(frame));
        } else if mode == FULL {
            DrawDefaultPlanetSphere();
        }
        if mode != WAIT {
            SetContext((GetScanContext(std::ptr::null_mut())).cast());
            DrawPlanet(0, black());
        }
        if mode != UPDATE {
            base::ScreenTransition(3, (&mut r as *mut CRect).cast());
        }
        g.unbatch();
        LoadIntoExtraScreen(std::ptr::from_mut(&mut r).cast());
    }

    #[no_mangle]
    pub unsafe extern "C" fn RedrawPlanetOrbitDisplay() {
        draw_display(FULL);
    }

    #[no_mangle]
    pub unsafe extern "C" fn LoadPlanet(surface: *mut crate::c_bindings::global_state::FRAME_DESC) {
        #[cfg(feature = "debug-process")]
        if crate::front_end::uqmdebug::DISABLE_INTERACTIVITY != 0 {
            return;
        }
        let system = solar_system::current();
        let wait = LastActivity & CHECK_LOAD == 0;
        assert!((*system).InOrbit != 0 && (*system).TopoFrame.is_null());
        create_context();
        if wait {
            draw_display(WAIT);
        }
        base::StopMusic();
        let planet = (*system).pOrbitalDesc;
        GeneratePlanetSurface(planet, surface);
        orbit_music::set_planet_music((*planet).data_index & !0x80);
        orbit_scan::generate_planet_side();
        if PLRPlaying(usize::MAX as *mut c_void) == 0 {
            base::PlayMusic((orbit_music::LANDER_MUSIC).cast(), 1, 1);
        }
        if wait {
            ZoomInPlanetSphere();
            draw_display(UPDATE);
        } else {
            draw_display(FULL);
        }
    }

    #[no_mangle]
    /// # Safety
    /// Call on the game thread with a loaded solar system. No other code may retain
    /// references to the planet resources released by this call.
    pub unsafe extern "C" fn FreePlanet() {
        let system = solar_system::current();
        let orbit = std::ptr::addr_of_mut!((*system).Orbit);
        UninitSphereRotation();
        base::StopMusic();
        for i in 0..(*system).PlanetSideFrame.len() {
            base::DestroyDrawable(base::ReleaseDrawable((*system).PlanetSideFrame[i].cast()));
            (*system).PlanetSideFrame[i] = std::ptr::null_mut();
        }
        DestroyStringTable(ReleaseStringTable((*system).XlatRef.cast()));
        (*system).XlatRef = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*system).TopoFrame.cast()));
        (*system).TopoFrame = std::ptr::null_mut();
        DestroyStringTable(ReleaseStringTable((*system).OrbitalCMap.cast()));
        (*system).OrbitalCMap = std::ptr::null_mut();
        rust_hfree((*orbit).lpTopoData.cast());
        (*orbit).lpTopoData = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*orbit).TopoZoomFrame.cast()));
        (*orbit).TopoZoomFrame = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*orbit).SphereFrame.cast()));
        (*orbit).SphereFrame = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*orbit).TintFrame.cast()));
        (*orbit).TintFrame = std::ptr::null_mut();
        (*orbit).TintColor = crate::c_bindings::global_state::Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        base::DestroyDrawable(base::ReleaseDrawable((*orbit).ObjectFrame.cast()));
        (*orbit).ObjectFrame = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*orbit).WorkFrame.cast()));
        (*orbit).WorkFrame = std::ptr::null_mut();
        rust_hfree((*orbit).TopoColors.cast());
        (*orbit).TopoColors = std::ptr::null_mut();
        rust_hfree((*orbit).ScratchArray.cast());
        (*orbit).ScratchArray = std::ptr::null_mut();
        DestroyStringTable(ReleaseStringTable(
            (*system).SysInfo.PlanetInfo.DiscoveryString.cast(),
        ));
        (*system).SysInfo.PlanetInfo.DiscoveryString = std::ptr::null_mut();
        FreeLanderFont(std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo));
        SetContext(SpaceContext);
        if !PlanetContext.is_null() {
            DestroyContext(PlanetContext);
            PlanetContext = std::ptr::null_mut();
        }
        DestroyScanContext();
    }

    #[no_mangle]
    pub unsafe extern "C" fn LoadStdLanderFont(info: *mut PLANET_INFO) {
        (*info).LanderFont = base::LoadGraphicInstance(c"font.lander".as_ptr()).cast();
        (*info).LanderFontEff = base::CaptureDrawable(
            (base::LoadGraphicInstance(c"graphics.landerfonteffect".as_ptr())).cast(),
        )
        .cast();
    }

    #[no_mangle]
    pub unsafe extern "C" fn FreeLanderFont(info: *mut PLANET_INFO) {
        DestroyFont((*info).LanderFont.cast());
        (*info).LanderFont = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*info).LanderFontEff.cast()));
        (*info).LanderFontEff = std::ptr::null_mut();
    }

    unsafe extern "C" fn do_orbit(menu: *mut MENU_STATE) -> u32 {
        use crate::c_bindings::controller_input::{PulsedInputState, KEY_MENU_SELECT};
        let mut selected = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
        if GlobData.Game_state.CurrentActivity & (CHECK_ABORT | CHECK_LOAD) != 0
            || GlobData.SIS_state.CrewEnlisted == u16::MAX
        {
            return 0;
        }
        if DoMenuChooser(menu, 0) != 0 || !selected {
            return 1;
        }
        CffiPlatform.set_flash(FlashTarget::Off);
        match (*menu).CurState {
            0 => {
                ScanSystem();
                if GlobData.Game_state.CurrentActivity & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            2 => {
                selected = DevicesMenu() != 0;
                if GlobData.Game_state.CurrentActivity & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            3 => CargoMenu(),
            4 => selected = RosterMenu() != 0,
            5 => {
                if GameOptions() == 0 {
                    return 0;
                }
            }
            1 => {
                let old = SetInputCallback(None);
                RepairSISBorder();
                let autopilot = StarMap();
                if GlobData.Game_state.CurrentActivity & CHECK_ABORT != 0 {
                    return 0;
                }
                SetInputCallback(old);
                if autopilot == 0 {
                    draw_display(FULL);
                } else {
                    return 0;
                }
            }
            6 => return 0,
            _ => {}
        }
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT == 0 {
            if selected {
                (*menu).CurState = submenu_selection((*menu).CurState, selected, optWhichMenu == 2);
                DrawMenuStateStrings(0, i16::from((*menu).CurState));
            }
            CffiPlatform.set_flash(FlashTarget::Menu3do);
        }
        1
    }

    unsafe extern "C" fn input_frame() {
        RotatePlanetSphere(1);
    }

    #[no_mangle]
    pub unsafe extern "C" fn PlanetOrbitMenu() {
        let mut menu: MENU_STATE = std::mem::zeroed();
        DrawMenuStateStrings(0, 0);
        CffiPlatform.set_flash(FlashTarget::Menu3do);
        menu.CurState = 0;
        base::SetMenuSounds(15, 16);
        let old = SetInputCallback(Some(input_frame));
        crate::automation::ui_observation::rust_automation_observe_planet_menu(1);
        menu.InputFunc = Some(do_orbit);
        base::DoInput((&mut menu as *mut MENU_STATE).cast(), 1);
        crate::automation::ui_observation::rust_automation_observe_planet_menu(0);
        SetInputCallback(old);
        CffiPlatform.set_flash(FlashTarget::Off);
        DrawMenuStateStrings(1, -6);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn successful_3do_submenus_jump_to_navigation_but_pc_and_cancel_stay() {
        for current in 0..7 {
            assert_eq!(submenu_selection(current, true, false), 6);
            assert_eq!(submenu_selection(current, false, false), current);
            assert_eq!(submenu_selection(current, true, true), current);
        }
    }
}
