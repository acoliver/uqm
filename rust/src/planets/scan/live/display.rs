use super::super::{planet_mass, scan_value};
use super::*;
use crate::comm::sis_graphics::CText;
use crate::graphics::{drawable_abi::DrawableLease, native_font::FontLease};
use crate::starbase::gfx::{Align, TextDraw};

use crate::planets::solarsys::lifecycle::named_body_index;

use crate::graphics::font_abi::TextRect;

pub(super) fn text(x: i16, y: i16, align: Align, text: String) {
    CffiPlatform.draw_text(&TextDraw {
        baseline: Point { x, y },
        align,
        text,
    });
}

unsafe fn title(solar: &SolarSystem) -> String {
    if let Some(index) = named_body_index(solar) {
        return CffiPlatform.game_string(i32::from(index));
    }
    let world_type = solar.sys.descriptor(solar.require_body()).data_index & !0x80;
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

pub(super) unsafe fn coarse(
    cx: &mut Cx<'_>,
    solar: &SolarSystem,
    planet: &ContextLease,
    style: i32,
) {
    let micro = FontLease::retain(
        std::ptr::NonNull::new(cx.kernel.micro_font)
            .unwrap_or_else(|| panic!("scan micro font is not loaded")),
    );
    if style == 2 {
        let tiny = FontLease::retain(
            std::ptr::NonNull::new(cx.kernel.tiny_font)
                .unwrap_or_else(|| panic!("scan tiny font is not loaded")),
        );
        cx.with_legacy(|| pc(solar, planet, &micro, &tiny));
    } else {
        let junk = crate::planets::solarsys::resources::retained(cx.kernel)
            .junk
            .lease();
        cx.with_legacy(|| three_do(solar, planet, &micro, &junk));
    }
}

unsafe fn pc(solar: &SolarSystem, planet: &ContextLease, micro: &FontLease, tiny: &FontLease) {
    let info = solar.sys.info.planet_info;
    let g = CffiPlatform;
    planet.select();
    g.set_fg_color(Rgba::rgb15(0, 0, 21));
    planet.set_font(micro);
    text(width() >> 1, 13, Align::Center, title(solar));
    planet.set_font(tiny);
    let orbit = (i64::from(info.planet_to_sun_dist) * 100 + 256) / 512;
    pc_value(5, 40, 0, scan_value(orbit, &g.game_string(487)));
    let atmosphere = if info.atmo_density == u16::MAX {
        g.game_string(489)
    } else if info.atmo_density == 0 {
        g.game_string(490)
    } else {
        scan_value(
            (i64::from(info.atmo_density) * 100 + 25) / 50,
            &g.game_string(491),
        )
    };
    pc_value(5, 54, 2, atmosphere);
    pc_value(5, 68, 6, format!("{}° c", info.temperature));
    pc_value(
        5,
        82,
        7,
        if info.atmo_density == 0 {
            g.game_string(494)
        } else {
            format!("{} {}", g.game_string(495), info.weather + 1)
        },
    );
    pc_value(
        5,
        96,
        10,
        if (*info.plan_data_ptr).type_ & 3 == 2 {
            g.game_string(494)
        } else {
            format!("{} {}", g.game_string(495), info.tectonics + 1)
        },
    );
    let right = width() - 75;
    pc_value(
        right,
        40,
        11,
        scan_value(
            i64::from(planet_mass(info.planet_radius, info.planet_density)),
            &g.game_string(498),
        ),
    );
    pc_value(
        right,
        54,
        13,
        scan_value(i64::from(info.planet_radius), &g.game_string(498)),
    );
    pc_value(
        right,
        68,
        14,
        scan_value(i64::from(info.gravity.max(1)), &g.game_string(501)),
    );
    pc_value(
        right,
        82,
        16,
        scan_value(
            i64::from(info.rotation_period) * 10 / 24,
            &g.game_string(503),
        ),
    );
    pc_value(
        right,
        96,
        18,
        format!("{}°", i32::from(info.axial_tilt).abs()),
    );
}

unsafe fn three_do(
    solar: &SolarSystem,
    planet: &ContextLease,
    micro: &FontLease,
    junk: &DrawableLease,
) {
    let info = solar.sys.info.planet_info;
    let g = CffiPlatform;
    planet.select();
    g.set_fg_color(Rgba::rgb15(15, 0, 25));
    planet.set_font(micro);
    text(width() >> 1, 13, Align::Center, title(solar));
    planet.draw_stamp(
        &junk.context_frame(0).wrapped_frame(20),
        crate::graphics::native_abi::POINT { x: 16, y: 0 },
    );
    let orbit = (i64::from(info.planet_to_sun_dist) * 100 + 256) / 512;
    text(43, 25, Align::Left, scan_value(orbit, "♁"));
    text(
        43,
        44,
        Align::Left,
        if info.atmo_density == u16::MAX {
            "∞".into()
        } else {
            scan_value((i64::from(info.atmo_density) * 100 + 25) / 50, "♁")
        },
    );
    text(43, 63, Align::Left, format!("{}°", info.temperature));
    text(
        43,
        82,
        Align::Left,
        format!(
            "<{}>",
            if info.atmo_density == 0 {
                0
            } else {
                info.weather + 1
            }
        ),
    );
    text(
        43,
        101,
        Align::Left,
        format!(
            "<{}>",
            if (*info.plan_data_ptr).type_ & 3 == 2 {
                0
            } else {
                info.tectonics + 1
            }
        ),
    );
    let right = width() - 43;
    text(
        right,
        25,
        Align::Right,
        scan_value(
            i64::from(planet_mass(info.planet_radius, info.planet_density)),
            "♁",
        ),
    );
    text(
        right,
        44,
        Align::Right,
        scan_value(i64::from(info.planet_radius), "♁"),
    );
    text(
        right,
        63,
        Align::Right,
        scan_value(i64::from(info.gravity.max(1)), "♁"),
    );
    text(
        right,
        82,
        Align::Right,
        format!("{}°", i32::from(info.axial_tilt).abs()),
    );
    text(
        right,
        101,
        Align::Right,
        scan_value(i64::from(info.rotation_period) * 10 / 24, "♁"),
    );
}
