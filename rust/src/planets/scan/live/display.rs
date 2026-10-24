use super::super::{planet_mass, scan_value};
use super::*;
use crate::comm::sis_graphics::CText;
use crate::starbase::gfx::{Align, TextDraw};
use std::ffi::CStr;

use crate::planets::solarsys::lifecycle::GetNamedPlanetaryBody;

use crate::graphics::font_abi::TextRect;

pub(super) fn text(x: i16, y: i16, align: Align, text: String) {
    CffiPlatform.draw_text(&TextDraw {
        baseline: Point { x, y },
        align,
        text,
    });
}

unsafe fn title() -> String {
    let named = GetNamedPlanetaryBody();
    if !named.is_null() {
        return CStr::from_ptr(named).to_string_lossy().into_owned();
    }
    let world_type = (*(*solar_system::current()).pOrbitalDesc).data_index & !0x80;
    if world_type >= 50 {
        CffiPlatform.game_string(305 + 4 + 51)
    } else {
        format!(
            "{} {}",
            CffiPlatform.game_string(305 + 4 + i32::from(world_type)),
            CffiPlatform.game_string(305 + 4 + 50)
        )
    }
}

unsafe fn pc_value(x: i16, y: i16, label: i32, value: String) {
    let g = CffiPlatform;
    let title = g.game_string(486 + label);
    g.set_fg_color(Rgba::rgb15(0, 0, 21));
    text(x, y, Align::Left, title.clone());
    let string = CString::new(title).unwrap();
    let mut ctext = CText::default();
    ctext.baseline = CPoint { x, y };
    ctext.p_str = string.as_ptr();
    ctext.char_count = u16::MAX;
    let mut rect = CRect::default();
    TextRect(
        std::ptr::from_mut(&mut ctext).cast(),
        std::ptr::from_mut(&mut rect).cast(),
        std::ptr::null_mut(),
    );
    g.set_fg_color(Rgba::rgb15(15, 0, 25));
    text(x + rect.width, y, Align::Left, value);
}

pub(super) unsafe fn coarse() {
    if optWhichCoarseScan == 2 {
        pc();
    } else {
        three_do();
    }
}

unsafe fn pc() {
    let info = (*solar_system::current()).SysInfo.PlanetInfo;
    let g = CffiPlatform;
    SetContext(PlanetContext);
    g.set_fg_color(Rgba::rgb15(0, 0, 21));
    SetContextFont((MicroFont).cast());
    text(width() >> 1, 13, Align::Center, title());
    SetContextFont((TinyFont).cast());
    let orbit = (i64::from(info.PlanetToSunDist) * 100 + 256) / 512;
    pc_value(5, 40, 0, scan_value(orbit, &g.game_string(487)));
    let atmosphere = if info.AtmoDensity == u16::MAX {
        g.game_string(489)
    } else if info.AtmoDensity == 0 {
        g.game_string(490)
    } else {
        scan_value(
            (i64::from(info.AtmoDensity) * 100 + 25) / 50,
            &g.game_string(491),
        )
    };
    pc_value(5, 54, 2, atmosphere);
    pc_value(5, 68, 6, format!("{}° c", info.SurfaceTemperature));
    pc_value(
        5,
        82,
        7,
        if info.AtmoDensity == 0 {
            g.game_string(494)
        } else {
            format!("{} {}", g.game_string(495), info.Weather + 1)
        },
    );
    pc_value(
        5,
        96,
        10,
        if (*info.PlanDataPtr).Type & 3 == 2 {
            g.game_string(494)
        } else {
            format!("{} {}", g.game_string(495), info.Tectonics + 1)
        },
    );
    let right = width() - 75;
    pc_value(
        right,
        40,
        11,
        scan_value(
            i64::from(planet_mass(info.PlanetRadius, info.PlanetDensity)),
            &g.game_string(498),
        ),
    );
    pc_value(
        right,
        54,
        13,
        scan_value(i64::from(info.PlanetRadius), &g.game_string(498)),
    );
    pc_value(
        right,
        68,
        14,
        scan_value(i64::from(info.SurfaceGravity.max(1)), &g.game_string(501)),
    );
    pc_value(
        right,
        82,
        16,
        scan_value(
            i64::from(info.RotationPeriod) * 10 / 24,
            &g.game_string(503),
        ),
    );
    pc_value(
        right,
        96,
        18,
        format!("{}°", i32::from(info.AxialTilt).abs()),
    );
}

unsafe fn three_do() {
    let info = (*solar_system::current()).SysInfo.PlanetInfo;
    let g = CffiPlatform;
    SetContext(PlanetContext);
    g.set_fg_color(Rgba::rgb15(15, 0, 25));
    SetContextFont((MicroFont).cast());
    text(width() >> 1, 13, Align::Center, title());
    draw(
        (base::SetAbsFrameIndex((SpaceJunkFrame).cast(), 20)).cast(),
        CPoint { x: 16, y: 0 },
        false,
    );
    let orbit = (i64::from(info.PlanetToSunDist) * 100 + 256) / 512;
    text(43, 25, Align::Left, scan_value(orbit, "♁"));
    text(
        43,
        44,
        Align::Left,
        if info.AtmoDensity == u16::MAX {
            "∞".into()
        } else {
            scan_value((i64::from(info.AtmoDensity) * 100 + 25) / 50, "♁")
        },
    );
    text(43, 63, Align::Left, format!("{}°", info.SurfaceTemperature));
    text(
        43,
        82,
        Align::Left,
        format!(
            "<{}>",
            if info.AtmoDensity == 0 {
                0
            } else {
                info.Weather + 1
            }
        ),
    );
    text(
        43,
        101,
        Align::Left,
        format!(
            "<{}>",
            if (*info.PlanDataPtr).Type & 3 == 2 {
                0
            } else {
                info.Tectonics + 1
            }
        ),
    );
    let right = width() - 43;
    text(
        right,
        25,
        Align::Right,
        scan_value(
            i64::from(planet_mass(info.PlanetRadius, info.PlanetDensity)),
            "♁",
        ),
    );
    text(
        right,
        44,
        Align::Right,
        scan_value(i64::from(info.PlanetRadius), "♁"),
    );
    text(
        right,
        63,
        Align::Right,
        scan_value(i64::from(info.SurfaceGravity.max(1)), "♁"),
    );
    text(
        right,
        82,
        Align::Right,
        format!("{}°", i32::from(info.AxialTilt).abs()),
    );
    text(
        right,
        101,
        Align::Right,
        scan_value(i64::from(info.RotationPeriod) * 10 / 24, "♁"),
    );
}
