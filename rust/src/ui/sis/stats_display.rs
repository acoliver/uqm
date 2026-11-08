//! Equipment statistics drawn through retained campaign resources.
use super::*;
use crate::graphics::{context_abi::ContextFrame, native_font::FontLease};
use std::{ffi::CString, ptr::NonNull};

fn weapon_name(kernel: &Kernel, module: u8) -> CString {
    let offset = match u32::from(module) {
        GUN_WEAPON => 8,
        BLASTER_WEAPON => 9,
        CANNON_WEAPON => 10,
        BOMB_MODULE_0..=BOMB_MODULE_5 => 11,
        _ => 12,
    };
    let mut bytes = kernel
        .game_text((FLAGSHIP_STRING_BASE + offset) as usize)
        .to_bytes()
        .to_vec();
    bytes.truncate(7);
    bytes.resize(7, b' ');
    CString::new(bytes).unwrap_or_else(|error| panic!("invalid equipment label: {error}"))
}

fn numbers(stats: &crate::ui::flagship_stats::Stats) -> [CString; 4] {
    let energy = ((100 * ONE_SECOND * u32::from(stats.energy_regen)
        / ((1 + u32::from(stats.energy_wait)) * (ONE_SECOND / 24)))
        + 5)
        / 10;
    [
        format!("{:>4}", i32::from(stats.thrust) * 4),
        format!("{:>4}", 1 + 17 - i32::from(stats.turn_wait)),
        format!("{:>2}.{}", energy / 10, energy % 10),
        format!("{:>4}", stats.fuel / FUEL_TANK_SCALE),
    ]
    .map(|value| {
        CString::new(value).unwrap_or_else(|error| panic!("invalid equipment statistic: {error}"))
    })
}

fn column(
    context: &ContextLease,
    font: &FontLease,
    position: (i16, i16, i16),
    align: TEXT_ALIGN,
    labels: [&CStr; 4],
) {
    let (x, top, leading) = position;
    for (row, text) in labels.into_iter().enumerate() {
        context.draw_text(
            font,
            text,
            POINT {
                x,
                y: top + leading * row as i16,
            },
            align,
            rgb15(0, 0, 0),
        );
    }
}

pub fn draw_flagship_stats(kernel: &Kernel, world: &World, dimensions: (i32, i32)) {
    let state = &world.data.flagship;
    let stats = crate::ui::flagship_stats::calculate(state);
    let context = ContextLease::require(kernel.space_context);
    let font = kernel
        .battle_font()
        .unwrap_or_else(|| panic!("equipment display requires its font"));
    let gradient = ContextFrame::retain(NonNull::new(kernel.font_grad_frame));
    let leading = font.leading();
    let rect = RECT {
        corner: POINT {
            x: 0,
            y: (dimensions.1 - 13 - 4 * i32::from(leading)) as i16,
        },
        extent: EXTENT {
            width: (dimensions.0 - 78) as i16,
            height: 4 * leading,
        },
    };
    let top = rect.corner.y + leading;
    let leading = leading - 3;
    let label_x = rect.extent.width / 6;
    let weapons = [15, 14, 13, 0].map(|slot| weapon_name(kernel, state.module_slots[slot]));
    let values = numbers(&stats);
    context.with_text_state(|| {
        context.set_font(&font);
        context.with_font_effect(None, || {
            context.fill_rectangle(rgb15(0, 0, 0), rect);
            context.with_font_effect(Some(&gradient.indexed_frame(4)), || {
                column(
                    &context,
                    &font,
                    (label_x, top, leading),
                    TEXT_ALIGN_ALIGN_RIGHT,
                    std::array::from_fn(|row| {
                        kernel.game_text(FLAGSHIP_STRING_BASE as usize + row)
                    }),
                );
                column(
                    &context,
                    &font,
                    (label_x + 5, top, leading),
                    TEXT_ALIGN_ALIGN_LEFT,
                    weapons.each_ref().map(|text| text.as_c_str()),
                );
            });
            context.with_font_effect(Some(&gradient.indexed_frame(5)), || {
                column(
                    &context,
                    &font,
                    (rect.extent.width - 25, top, leading),
                    TEXT_ALIGN_ALIGN_RIGHT,
                    std::array::from_fn(|row| {
                        kernel.game_text(FLAGSHIP_STRING_BASE as usize + row + 4)
                    }),
                );
                column(
                    &context,
                    &font,
                    (rect.extent.width - 2, top, leading),
                    TEXT_ALIGN_ALIGN_RIGHT,
                    values.each_ref().map(|text| text.as_c_str()),
                );
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_text_preserves_legacy_padding_and_tenths() {
        let state = crate::engine::Game::new().world.data.flagship;
        let stats = crate::ui::flagship_stats::calculate(&state);
        assert_eq!(
            numbers(&stats).map(|text| text.to_bytes().to_vec()),
            [
                b"  40".to_vec(),
                b"   1".to_vec(),
                b" 2.2".to_vec(),
                b"  10".to_vec()
            ]
        );
        let mut kernel = Kernel::new();
        let mut strings = vec![b"\0".to_vec(); FLAGSHIP_STRING_BASE as usize + 13];
        for (offset, label) in [
            (8, b"GUN\0".as_slice()),
            (9, b"BLASTER!\0".as_slice()),
            (10, b"CANNON\0".as_slice()),
            (11, b"BOMB\0".as_slice()),
            (12, b"\0".as_slice()),
        ] {
            strings[FLAGSHIP_STRING_BASE as usize + offset] = label.to_vec();
        }
        kernel.install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(
            strings,
        ));
        for (module, label) in [
            (GUN_WEAPON, c"GUN    "),
            (BLASTER_WEAPON, c"BLASTER"),
            (CANNON_WEAPON, c"CANNON "),
            (BOMB_MODULE_0, c"BOMB   "),
            (BOMB_MODULE_5, c"BOMB   "),
            (0, c"       "),
        ] {
            assert_eq!(weapon_name(&kernel, module as u8), label.to_owned());
        }
    }
}
