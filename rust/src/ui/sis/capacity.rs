//! Flagship capacities and physical crew, mineral and fuel positions.
use crate::starbase::sis_modules as equipment;

use crate::engine::world::SisState;
use crate::graphics::{
    context_abi::ContextLease,
    native_abi::{Color, EXTENT, POINT, RECT},
};
use crate::ui::rgb15;
use equipment::{fuel_bar_at, ModuleType};

const CREW_PER_ROW: i32 = 5;
const SBAY_MASS_PER_ROW: i32 = 50;
const SHIP_PIECE_OFFSET: i32 = 12;

fn module_fuel_capacity(module: u8) -> u32 {
    ModuleType::from_u8(module).map_or(0, ModuleType::fuel_capacity)
}

fn crew_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 10] = [
        (10, 30, 9),
        (0, 30, 0),
        (0, 27, 0),
        (0, 24, 0),
        (0, 21, 0),
        (0, 18, 0),
        (0, 16, 0),
        (0, 13, 0),
        (0, 10, 0),
        (0, 7, 0),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}
fn storage_color(i: usize) -> Color {
    const V: [u8; 10] = [31, 28, 24, 21, 18, 16, 13, 10, 8, 5];
    rgb15(V[i], V[i], V[i])
}
fn fuel_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 11] = [
        (15, 0, 0),
        (19, 0, 0),
        (23, 0, 0),
        (27, 0, 0),
        (31, 3, 0),
        (31, 7, 0),
        (31, 10, 0),
        (31, 14, 0),
        (31, 17, 0),
        (31, 21, 0),
        (31, 24, 0),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}

pub(crate) struct CapacityCell {
    pub point: POINT,
    pub foreground: Color,
}

pub(crate) fn crew_capacity(state: &SisState) -> u16 {
    state
        .module_slots
        .iter()
        .map(|&module| crate::ui::sis_capacity_math::module_crew_capacity(module))
        .sum()
}

pub(crate) fn storage_capacity(state: &SisState) -> u16 {
    state
        .module_slots
        .iter()
        .map(|&module| crate::ui::sis_capacity_math::module_storage_capacity(module))
        .sum()
}

pub(crate) fn fuel_capacity(state: &SisState) -> u32 {
    equipment::FUEL_RESERVE
        + state
            .module_slots
            .iter()
            .map(|&module| module_fuel_capacity(module))
            .sum::<u32>()
}

pub(crate) fn crew_cell(state: &SisState, pc_fonts: bool) -> Option<CapacityCell> {
    let (slot, seat) =
        crate::ui::sis_capacity_math::crew_pod_for(&state.module_slots, state.crew_enlisted)?;
    let row = i32::from(seat) / CREW_PER_ROW;
    let col = i32::from(seat) % CREW_PER_ROW;
    Some(CapacityCell {
        point: POINT {
            x: (27 + i32::from(slot) * SHIP_PIECE_OFFSET - col * 2) as i16,
            y: (34 - row * 2) as i16,
        },
        foreground: if pc_fonts {
            crew_color(row as usize)
        } else {
            rgb15(5, 0x10, 5)
        },
    })
}

pub(crate) fn storage_cell(state: &SisState) -> Option<CapacityCell> {
    let (slot, cell) = crate::ui::sis_capacity_math::storage_cell_for(
        &state.module_slots,
        state.total_element_mass,
    )?;
    let row = i32::from(cell) / SBAY_MASS_PER_ROW;
    Some(CapacityCell {
        point: POINT {
            x: (19 + i32::from(slot) * SHIP_PIECE_OFFSET) as i16,
            y: (34 - if row == 0 { 0 } else { row - 1 } * 2) as i16,
        },
        foreground: if row == 0 {
            rgb15(0, 0, 0)
        } else {
            storage_color(row as usize - 1)
        },
    })
}

pub(crate) fn fuel_cell(state: &SisState) -> Option<(CapacityCell, Color)> {
    let capacities = state.module_slots.map(module_fuel_capacity);
    let bar = fuel_bar_at(&capacities, state.fuel_on_board, state.fuel_on_board)?;
    Some((
        CapacityCell {
            point: POINT {
                x: bar.x as i16,
                y: bar.y as i16,
            },
            foreground: fuel_color(bar.row as usize),
        },
        fuel_color(bar.row as usize + 1),
    ))
}

fn draw_fuel_bar(context: &ContextLease, state: &SisState, filled: bool) {
    let (cell, background) =
        fuel_cell(state).unwrap_or_else(|| panic!("fuel bar below flagship capacity"));
    context.set_foreground(cell.foreground);
    context.set_background(background);
    let mut rect = RECT {
        corner: cell.point,
        extent: EXTENT {
            width: 3,
            height: 1,
        },
    };
    let color = if filled {
        context.draw_point(cell.foreground, rect.corner);
        rect.corner.x += rect.extent.width + 1;
        context.draw_point(cell.foreground, rect.corner);
        rect.corner.x -= rect.extent.width;
        context.set_background(rgb15(0, 0, 0));
        background
    } else {
        rect.extent.width = 5;
        rgb15(0x0b, 0, 0)
    };
    context.fill_rectangle(color, rect);
}

pub(crate) fn draw_fuel_bars(context: &ContextLease, state: &SisState) {
    let mut display = *state;
    display.fuel_on_board = equipment::FUEL_RESERVE;
    let capacity = fuel_capacity(state);
    context.draw_batch(|| {
        while display.fuel_on_board < capacity {
            draw_fuel_bar(
                context,
                &display,
                display.fuel_on_board < state.fuel_on_board,
            );
            display.fuel_on_board += equipment::FUEL_VOLUME_PER_ROW;
        }
    });
}

pub(crate) fn draw_blueprint_cells(context: &ContextLease, state: &SisState, pc_fonts: bool) {
    let mut display = *state;
    context.with_selected(|| {
        for crew in 0..state.crew_enlisted {
            display.crew_enlisted = crew;
            let cell = crew_cell(&display, pc_fonts)
                .unwrap_or_else(|| panic!("blueprint crew exceeds flagship capacity"));
            context.draw_point(cell.foreground, cell.point);
        }
        display.total_element_mass = 0;
        let mut remaining = state.total_element_mass;
        while remaining != 0 {
            let mass = remaining.min(SBAY_MASS_PER_ROW as u16);
            display.total_element_mass += mass;
            let cell = storage_cell(&display)
                .unwrap_or_else(|| panic!("blueprint cargo exceeds flagship capacity"));
            context.fill_rectangle(
                cell.foreground,
                RECT {
                    corner: cell.point,
                    extent: EXTENT {
                        width: 9,
                        height: 1,
                    },
                },
            );
            remaining -= mass;
        }
        if state.fuel_on_board > equipment::FUEL_RESERVE {
            display.fuel_on_board = equipment::FUEL_RESERVE;
            let mut remaining = state.fuel_on_board - equipment::FUEL_RESERVE;
            while remaining != 0 {
                draw_fuel_bar(context, &display, true);
                let mass = remaining.min(equipment::FUEL_VOLUME_PER_ROW);
                display.fuel_on_board += mass;
                remaining -= mass;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flagship() -> SisState {
        let mut state = crate::engine::World::new().data.flagship;
        state.module_slots.fill(equipment::EMPTY_MODULE_SLOT);
        state
    }

    #[test]
    #[serial_test::serial]
    fn capacity_drawing_uses_retained_context_and_preserves_flagship_inventory() {
        if !crate::native_test_process::own_graphics_process(concat!(
            module_path!(),
            "::capacity_drawing_uses_retained_context_and_preserves_flagship_inventory"
        )) {
            return;
        }
        use crate::graphics::context_abi::{ContextFrame, ContextLease};
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game)
            .unwrap_or_else(|error| panic!("capacity fixture: {error}"));
        game.world.data.flagship = flagship();
        let state = &mut game.world.data.flagship;
        state.module_slots[11] = ModuleType::CrewPod as u8;
        state.module_slots[3] = ModuleType::StorageBay as u8;
        state.module_slots[5] = ModuleType::FuelTank as u8;
        state.crew_enlisted = 2;
        state.total_element_mass = 51;
        state.fuel_on_board = 1001;
        let before = *state;
        let context = ContextLease::require(game.kernel.status_context);
        let frame = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen));
        context.set_frame(frame.clone());
        context.set_background(rgb15(0, 0, 0));
        context.clear();
        draw_blueprint_cells(&context, state, true);
        context.flush();
        let extent = frame.rect().extent;
        for point in [
            POINT { x: -1, y: 0 },
            POINT {
                x: extent.width,
                y: 0,
            },
            POINT {
                x: 0,
                y: extent.height,
            },
        ] {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| frame.pixel(point)))
                    .is_err()
            );
        }
        let pixel = |x, y| {
            let color = frame.pixel(POINT { x, y });
            (color.r, color.g, color.b)
        };
        for (x, y, rgb) in [
            (159, 34, (82, 247, 74)),
            (157, 34, (82, 247, 74)),
            (155, 34, (0, 0, 0)),
            (55, 34, (255, 255, 255)),
            (63, 34, (255, 255, 255)),
            (81, 27, (123, 0, 0)),
            (85, 27, (123, 0, 0)),
            (82, 27, (156, 0, 0)),
        ] {
            assert_eq!(pixel(x, y), rgb, "pixel ({x},{y})");
        }
        context.clear();
        draw_fuel_bars(&context, state);
        context.flush();
        assert_eq!(pixel(81, 27), (123, 0, 0));
        assert_eq!(pixel(82, 27), (156, 0, 0));
        assert_eq!(pixel(81, 26), (90, 0, 0));
        assert_eq!(pixel(85, 23), (90, 0, 0));
        draw_blueprint_cells(&context, state, false);
        context.flush();
        assert_eq!(pixel(159, 34), (41, 132, 41));
        assert_eq!(state.module_slots, before.module_slots);
        assert_eq!(
            (
                state.crew_enlisted,
                state.total_element_mass,
                state.fuel_on_board
            ),
            (
                before.crew_enlisted,
                before.total_element_mass,
                before.fuel_on_board
            )
        );
        drop(frame);
        drop(context);
        display.finish(game);
    }

    #[test]
    fn queries_read_only_the_supplied_flagship() {
        let mut first = flagship();
        let mut second = flagship();
        first.module_slots[2] = ModuleType::CrewPod as u8;
        first.module_slots[8] = ModuleType::StorageBay as u8;
        first.module_slots[1] = ModuleType::FuelTank as u8;
        second.module_slots[15] = ModuleType::HighEffFuelSys as u8;
        assert_eq!(
            (
                crew_capacity(&first),
                storage_capacity(&first),
                fuel_capacity(&first)
            ),
            (50, 500, 6000)
        );
        assert_eq!(
            (
                crew_capacity(&second),
                storage_capacity(&second),
                fuel_capacity(&second)
            ),
            (0, 0, 11000)
        );
        assert_eq!(
            (
                crew_capacity(&first),
                storage_capacity(&first),
                fuel_capacity(&first)
            ),
            (50, 500, 6000)
        );
    }

    #[test]
    fn crew_positions_fill_from_rear_excluding_the_full_endpoint() {
        let mut state = flagship();
        state.module_slots[3] = ModuleType::CrewPod as u8;
        state.module_slots[11] = ModuleType::CrewPod as u8;
        for (crew, x, y) in [
            (0, 159, 34),
            (4, 151, 34),
            (5, 159, 32),
            (49, 151, 16),
            (50, 63, 34),
            (99, 55, 16),
        ] {
            state.crew_enlisted = crew;
            let cell = crew_cell(&state, true).unwrap();
            assert_eq!((cell.point.x, cell.point.y), (x, y));
        }
        state.crew_enlisted = 100;
        assert!(crew_cell(&state, true).is_none());
    }

    #[test]
    fn crew_palette_retains_pc_gradient_and_flat_console_color() {
        let mut state = flagship();
        state.module_slots[0] = ModuleType::CrewPod as u8;
        let rgb = |cell: CapacityCell| (cell.foreground.r, cell.foreground.g, cell.foreground.b);
        assert_eq!(rgb(crew_cell(&state, true).unwrap()), (82, 247, 74));
        assert_eq!(rgb(crew_cell(&state, false).unwrap()), (41, 132, 41));
        state.crew_enlisted = 49;
        assert_eq!(rgb(crew_cell(&state, true).unwrap()), (0, 57, 0));
    }

    #[test]
    fn storage_positions_include_full_endpoint_and_empty_zero_cell() {
        let mut state = flagship();
        let empty = storage_cell(&state).unwrap();
        assert_eq!(
            (empty.point.x, empty.point.y, empty.foreground.r),
            (199, 34, 0)
        );
        state.module_slots[3] = ModuleType::StorageBay as u8;
        state.module_slots[11] = ModuleType::StorageBay as u8;
        for (mass, x, y, shade) in [
            (1, 151, 34, 0),
            (50, 151, 34, 255),
            (500, 151, 16, 41),
            (501, 55, 34, 0),
            (1000, 55, 16, 41),
        ] {
            state.total_element_mass = mass;
            let cell = storage_cell(&state).unwrap();
            assert_eq!(
                (cell.point.x, cell.point.y, cell.foreground.r),
                (x, y, shade)
            );
        }
        state.total_element_mass = 1001;
        assert!(storage_cell(&state).is_none());
    }

    #[test]
    fn fuel_positions_keep_reserve_strict_endpoint_and_adjacent_colors() {
        let mut state = flagship();
        state.module_slots[11] = ModuleType::FuelTank as u8;
        state.module_slots[3] = ModuleType::HighEffFuelSys as u8;
        for (fuel, x, y) in [
            (1000, 153, 27),
            (5999, 153, 23),
            (6000, 57, 30),
            (15999, 57, 21),
        ] {
            state.fuel_on_board = fuel;
            let (cell, background) = fuel_cell(&state).unwrap();
            assert_eq!((cell.point.x, cell.point.y), (x, y));
            assert_ne!(
                (cell.foreground.r, cell.foreground.g, cell.foreground.b),
                (background.r, background.g, background.b)
            );
        }
        state.fuel_on_board = 1000;
        let (cell, background) = fuel_cell(&state).unwrap();
        assert_eq!((cell.foreground.r, background.r), (123, 156));
        for fuel in [0, 999, 16000, u32::MAX] {
            state.fuel_on_board = fuel;
            assert!(fuel_cell(&state).is_none());
        }
    }
}
