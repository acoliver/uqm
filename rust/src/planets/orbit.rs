//! Orbit display, planet resource ownership, and menu from `planets.c`.

fn submenu_selection(current: u8, selected: bool, pc_menu: bool) -> u8 {
    if selected && !pc_menu {
        6
    } else {
        current
    }
}

#[cfg(not(test))]
mod live {
    use super::submenu_selection;
    use crate::battle::element::{Element, ElementFlags};
    use crate::c_bindings::{
        global_state::{MENU_STATE, PLANET_INFO},
        live_queues, solar_system,
    };
    use crate::comm::locdata::{CGlobData, CRect};
    use crate::graphics::drawable_abi::SetContextFGFrame;
    use crate::mainloop::restart_menu::c_extern::{self as base, SetContext};
    use crate::mainloop::types::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
    use crate::memory::rust_hfree;
    use crate::planet_side::{orbit_music, orbit_scan};
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{ContextId, FlashTarget, Gfx, Point, Rect, Rgba, Stamp};
    use std::ffi::{c_char, c_void};

    #[no_mangle]
    static mut PlanetContext: *mut c_void = std::ptr::null_mut();

    use crate::ui::border::DrawSISFrame;
    use crate::ui::sis::RepairSISBorder;
    extern "C" {
        static GlobData: CGlobData;
        static LastActivity: u16;
        static SpaceContext: *mut c_void;
        static Screen: *mut c_void;
        static optWhichMenu: i32;
        #[cfg(feature = "debug-process")]
        fn CreateContextAux(name: *const c_char) -> *mut c_void;
        #[cfg(not(feature = "debug-process"))]
        fn CreateContextAux() -> *mut c_void;
        fn DestroyContext(context: *mut c_void) -> i32;
        fn GetScanContext(owner: *mut i32) -> *mut c_void;
        fn DestroyScanContext();
        fn DrawStarBackGround();

        fn DrawSISMessage(message: *const c_char);
        fn DrawSISTitle(title: *const c_char);
        fn LoadIntoExtraScreen(rect: *mut CRect);
        fn DrawDefaultPlanetSphere();
        fn DrawPlanet(y: i32, color: crate::comm::locdata::CColor);
        fn ZoomInPlanetSphere();
        fn UninitSphereRotation();
        fn RotatePlanetSphere(keep_rate: u32);
        fn GeneratePlanetSurface(
            world: *mut crate::c_bindings::global_state::PLANET_DESC,
            frame: *mut crate::c_bindings::global_state::FRAME_DESC,
        );
        fn PLRPlaying(music: *mut c_void) -> i32;
        fn ReleaseStringTable(table: *mut c_void) -> *mut c_void;
        fn DestroyStringTable(table: *mut c_void) -> i32;
        fn DestroyFont(font: *mut c_void) -> i32;
        fn ScanSystem();
        fn DevicesMenu() -> i32;
        fn CargoMenu();
        fn RosterMenu() -> i32;
        fn GameOptions() -> u32;
        fn StarMap() -> i32;

        fn DoMenuChooser(menu: *mut MENU_STATE, first: u8) -> u32;
        fn DrawMenuStateStrings(first: u8, state: i16);
        fn SetInputCallback(
            callback: Option<unsafe extern "C" fn()>,
        ) -> Option<unsafe extern "C" fn()>;
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
        PlanetContext = new_context(c"PlanetContext".as_ptr());
        SetContext(PlanetContext);
        SetContextFGFrame(Screen.cast());
        r.height -= 75 + 5;
        CffiPlatform.set_context_clip_rect(r);
        SetContext(old);
    }

    #[no_mangle]
    unsafe extern "C" fn DrawScannedObjects(reversed: u32) {
        let queue = live_queues::display_list();
        let mut handle = if reversed != 0 {
            (*queue).tail
        } else {
            (*queue).head
        } as usize;
        while handle != 0 {
            let element = handle as *const Element;
            handle = if reversed != 0 {
                (*element).pred as usize
            } else {
                (*element).succ as usize
            };
            if (*element).state_flags.contains(ElementFlags::APPEARING) {
                CffiPlatform.draw_stamp(Stamp {
                    origin: Point {
                        x: (*element).current.location.x,
                        y: (*element).current.location.y,
                    },
                    frame: (*element).next.frame as usize,
                });
            }
        }
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
            DrawSISTitle(std::ptr::addr_of!(GlobData.sis_state.planet_name).cast());
            DrawStarBackGround();
            DrawPlanetSurfaceBorder();
        }
        if mode == WAIT {
            SetContext(GetScanContext(std::ptr::null_mut()));
            let frame =
                base::CaptureDrawable(base::LoadGraphicInstance(c"graphics.orbitenter".as_ptr()));
            g.draw_stamp(Stamp {
                origin: Point { x: 0, y: 0 },
                frame: frame as usize,
            });
            base::DestroyDrawable(base::ReleaseDrawable(frame));
        } else if mode == FULL {
            DrawDefaultPlanetSphere();
        }
        if mode != WAIT {
            SetContext(GetScanContext(std::ptr::null_mut()));
            DrawPlanet(0, black());
        }
        if mode != UPDATE {
            base::ScreenTransition(3, (&mut r as *mut CRect).cast());
        }
        g.unbatch();
        LoadIntoExtraScreen(&mut r);
    }

    #[no_mangle]
    unsafe extern "C" fn RedrawPlanetOrbitDisplay() {
        draw_display(FULL);
    }

    #[no_mangle]
    unsafe extern "C" fn LoadPlanet(surface: *mut crate::c_bindings::global_state::FRAME_DESC) {
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
            base::PlayMusic(orbit_music::LANDER_MUSIC, 1, 1);
        }
        if wait {
            ZoomInPlanetSphere();
            draw_display(UPDATE);
        } else {
            draw_display(FULL);
        }
    }

    #[no_mangle]
    unsafe extern "C" fn FreePlanet() {
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
    unsafe extern "C" fn LoadStdLanderFont(info: *mut PLANET_INFO) {
        (*info).LanderFont = base::LoadGraphicInstance(c"font.lander".as_ptr()).cast();
        (*info).LanderFontEff = base::CaptureDrawable(base::LoadGraphicInstance(
            c"graphics.landerfonteffect".as_ptr(),
        ))
        .cast();
    }

    #[no_mangle]
    unsafe extern "C" fn FreeLanderFont(info: *mut PLANET_INFO) {
        DestroyFont((*info).LanderFont.cast());
        (*info).LanderFont = std::ptr::null_mut();
        base::DestroyDrawable(base::ReleaseDrawable((*info).LanderFontEff.cast()));
        (*info).LanderFontEff = std::ptr::null_mut();
    }

    unsafe extern "C" fn do_orbit(menu: *mut MENU_STATE) -> u32 {
        use crate::c_bindings::controller_input::{PulsedInputState, KEY_MENU_SELECT};
        let mut selected = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
        if GlobData.game_state.current_activity & (CHECK_ABORT | CHECK_LOAD) != 0
            || GlobData.sis_state.crew_enlisted == u16::MAX
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
                if GlobData.game_state.current_activity & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            2 => {
                selected = DevicesMenu() != 0;
                if GlobData.game_state.current_activity & START_ENCOUNTER != 0 {
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
                if GlobData.game_state.current_activity & CHECK_ABORT != 0 {
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
        if GlobData.game_state.current_activity & CHECK_ABORT == 0 {
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
    unsafe extern "C" fn PlanetOrbitMenu() {
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
            CreateContextAux(name)
        }
        #[cfg(not(feature = "debug-process"))]
        {
            let _ = name;
            CreateContextAux()
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
