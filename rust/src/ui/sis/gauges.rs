//! Flagship gauges borrow campaign state and retained display resources.
use super::*;
use crate::engine::World;
use crate::graphics::{context_abi::ContextFrame, native_font::FontLease};
use std::{
    ffi::{CStr, CString},
    ptr::NonNull,
};

pub const UNDEFINED_DELTA: i16 = 0x7fff;

fn frame(raw: FRAME) -> ContextFrame {
    ContextFrame::retain(NonNull::new(raw))
}
fn font(kernel: &Kernel) -> FontLease {
    kernel
        .tiny_font_lease()
        .unwrap_or_else(|| panic!("SIS gauges require the tiny font"))
}
fn name(bytes: &[u8]) -> &CStr {
    CStr::from_bytes_until_nul(bytes)
        .unwrap_or_else(|error| panic!("unterminated flagship name: {error}"))
}
fn number(value: impl ToString) -> CString {
    CString::new(value.to_string()).unwrap_or_else(|error| panic!("invalid gauge label: {error}"))
}
pub fn gauge_rect(crew: bool) -> RECT {
    RECT {
        corner: POINT {
            x: 20,
            y: if crew { 117 } else { 38 },
        },
        extent: EXTENT {
            width: 24,
            height: 5,
        },
    }
}
fn update_crew(state: &mut SIS_STATE, delta: i16) {
    if delta == 0 || delta == UNDEFINED_DELTA {
        return;
    }
    let capacity = capacity::crew_capacity(state);
    state.crew_enlisted = if delta < 0 && state.crew_enlisted <= (-i32::from(delta)) as u16 {
        0
    } else {
        ((i32::from(state.crew_enlisted) + i32::from(delta)) as u16).min(capacity)
    };
}
fn update_fuel(state: &mut SIS_STATE, delta: i16) -> bool {
    if delta == 0 {
        return false;
    }
    let old = if delta == UNDEFINED_DELTA {
        u16::MAX
    } else {
        (state.fuel_on_board / FUEL_TANK_SCALE) as u16
    };
    if delta != UNDEFINED_DELTA {
        let capacity = capacity::fuel_capacity(state);
        state.fuel_on_board = if delta < 0 && state.fuel_on_board <= (-i32::from(delta)) as u32 {
            0
        } else {
            state
                .fuel_on_board
                .wrapping_add(i32::from(delta) as u32)
                .min(capacity)
        };
    }
    (state.fuel_on_board / FUEL_TANK_SCALE) as u16 != old
}
fn update_resources(state: &mut SIS_STATE, delta: i16, mode: StatMsgMode) {
    if delta == 0 || delta == UNDEFINED_DELTA {
        return;
    }
    state.res_units = if delta < 0 && state.res_units <= (-i32::from(delta)) as u32 {
        0
    } else {
        state.res_units.wrapping_add(i32::from(delta) as u32)
    };
    assert_eq!(mode, StatMsgMode_SMM_RES_UNITS);
}
fn draw_number(context: &ContextLease, font: &FontLease, crew: bool, value: u16) {
    let rect = gauge_rect(crew);
    context.fill_rectangle(rgb15(0, 0, 0), rect);
    context.draw_text(
        font,
        &number(value),
        POINT {
            x: 32,
            y: rect.corner.y + rect.extent.height,
        },
        TEXT_ALIGN_ALIGN_CENTER,
        if crew {
            rgb15(0, 14, 0)
        } else {
            rgb15(19, 0, 0)
        },
    );
}
pub fn draw_captains_name(kernel: &Kernel, world: &World) {
    let context = ContextLease::require(kernel.status_context);
    context.with_text_state(|| {
        context.fill_rectangle(
            rgb15(0, 0, 20),
            RECT {
                corner: POINT { x: 3, y: 10 },
                extent: EXTENT {
                    width: 58,
                    height: 7,
                },
            },
        );
        context.draw_text(
            &font(kernel),
            name(&world.data.flagship.commander_name),
            POINT { x: 31, y: 16 },
            TEXT_ALIGN_ALIGN_CENTER,
            rgb15(22, 11, 31),
        );
    });
}
pub fn draw_flagship_name(kernel: &Kernel, world: &World, width: i32, status: bool) {
    let context = ContextLease::require(if status {
        kernel.status_context
    } else {
        kernel.space_context
    });
    let font = if status {
        NonNull::new(kernel.star_con_font).map(FontLease::retain)
    } else {
        kernel.micro_font_lease()
    }
    .unwrap_or_else(|| panic!("flagship name requires its font"));
    let rect = RECT {
        corner: POINT {
            x: if status { 2 } else { 0 },
            y: if status { 20 } else { 1 },
        },
        extent: EXTENT {
            width: if status { 60 } else { (width - 78) as i16 },
            height: 7,
        },
    };
    let text = if status {
        name(&world.data.flagship.ship_name).to_owned()
    } else {
        let mut bytes = kernel
            .game_text((NAMING_STRING_BASE + 1) as usize)
            .to_bytes()
            .to_vec();
        bytes.push(b' ');
        bytes.extend_from_slice(name(&world.data.flagship.ship_name).to_bytes());
        bytes.truncate(249);
        bytes.make_ascii_uppercase();
        CString::new(bytes).unwrap_or_else(|error| panic!("invalid flagship title: {error}"))
    };
    let gradient = (kernel.configuration.preferences().fonts == OPT_PC)
        .then(|| frame(kernel.font_grad_frame).indexed_frame(if status { 0 } else { 3 }));
    context.with_text_state(|| {
        context.with_font_effect(None, || {
            context.fill_rectangle(rgb15(0, 0, 0), rect);
            context.with_font_effect(gradient.as_ref(), || {
                context.draw_text(
                    &font,
                    &text,
                    POINT {
                        x: rect.corner.x + (rect.extent.width >> 1),
                        y: rect.corner.y + 7 - i16::from(status),
                    },
                    TEXT_ALIGN_ALIGN_CENTER,
                    if gradient.is_some() {
                        rgb15(0, 0, 0)
                    } else {
                        rgb15(20, 10, 0)
                    },
                )
            });
        })
    });
}
pub fn draw_landers(kernel: &Kernel, world: &World) {
    let context = ContextLease::require(kernel.status_context);
    context.with_selected(|| {
        let lander = frame(kernel.flag_stat_frame).next_frame();
        let mut rect = lander.rect();
        let count = i32::from(world.data.flagship.num_landers);
        rect.corner.x = (32 - i32::from(rect.corner.x)) as i16;
        let mut origin = POINT {
            x: (i32::from(rect.corner.x)
                - ((i32::from(rect.extent.width) * count + 2 * (count - 1)) >> 1))
                as i16,
            y: 29,
        };
        let spacing = rect.extent.width + 2;
        rect.extent.width = (i32::from(rect.extent.width) * MAX_LANDERS as i32
            + 2 * (MAX_LANDERS as i32 - 1)
            + 2) as i16;
        rect.corner.x -= rect.extent.width >> 1;
        rect.corner.y += origin.y;
        context.fill_rectangle(rgb15(0, 0, 0), rect);
        for _ in 0..count {
            context.draw_stamp(&lander, origin);
            origin.x = origin.x.wrapping_add(spacing);
        }
    });
}
pub fn draw_storage_bays(kernel: &Kernel, world: &World, refresh: bool) {
    let context = ContextLease::require(kernel.status_context);
    context.with_selected(|| {
        let state = &world.data.flagship;
        let mut rect = RECT {
            corner: POINT { x: 0, y: 123 },
            extent: EXTENT {
                width: 2,
                height: 4,
            },
        };
        if refresh {
            rect.extent.width = (NUM_MODULE_SLOTS * 3) as i16;
            rect.corner.x = 32 - (rect.extent.width >> 1);
            context.fill_rectangle(rgb15(0, 0, 0), rect);
            rect.extent.width = 2;
        }
        let mut bays = state
            .module_slots
            .iter()
            .filter(|&&m| m == STORAGE_BAY as u8)
            .count() as u8;
        if bays == 0 {
            return;
        }
        rect.corner.x = (32 - ((i32::from(bays) * 3) >> 1)) as i16;
        let mut mass = state.total_element_mass;
        while mass >= STORAGE_BAY_CAPACITY as u16 {
            context.fill_rectangle(rgb15(10, 10, 10), rect);
            rect.corner.x += 3;
            bays = bays.wrapping_sub(1);
            mass -= STORAGE_BAY_CAPACITY as u16;
        }
        rect.extent.height = ((4 * i32::from(mass) + STORAGE_BAY_CAPACITY as i32 - 1)
            / STORAGE_BAY_CAPACITY as i32) as i16;
        if rect.extent.height != 0 {
            rect.corner.y += 4 - rect.extent.height;
            context.fill_rectangle(rgb15(10, 10, 10), rect);
            rect.extent.height = 4 - rect.extent.height;
            if rect.extent.height != 0 {
                rect.corner.y = 123;
                context.fill_rectangle(rgb15(6, 6, 6), rect);
            }
            rect.corner.x += 3;
            bays = bays.wrapping_sub(1);
        }
        rect.extent.height = 4;
        for _ in 0..bays {
            context.fill_rectangle(rgb15(6, 6, 6), rect);
            rect.corner.x += 3;
        }
    });
}
fn draw_slots(
    context: &ContextLease,
    frames: &ContextFrame,
    slots: &[u8],
    mut y: i16,
    mode: usize,
) {
    for &piece in slots {
        if piece < NUM_MODULES as u8 {
            let image = frames.indexed_frame(usize::from(piece) + 1 + mode);
            context.draw_stamp(&image, POINT { x: 1, y });
            if mode != 2 {
                context.draw_stamp(&image.next_frame(), POINT { x: 1, y });
            }
        }
        y = y.wrapping_sub(3);
    }
}
fn draw_pc_sis(kernel: &Kernel, context: &ContextLease) {
    let font = font(kernel);
    let gradients = frame(kernel.font_grad_frame);
    let mut rect = gauge_rect(false);
    let mut baseline = POINT {
        x: 32,
        y: rect.corner.y - 1,
    };
    rect.corner.y -= 6;
    rect.corner.x -= 1;
    rect.extent.width += 2;
    for (index, effect) in [(3, 1), (4, 2)] {
        context.fill_rectangle(rgb15(0, 0, 0), rect);
        context.with_font_effect(Some(&gradients.indexed_frame(effect)), || {
            context.draw_text(
                &font,
                kernel.game_text((STATUS_STRING_BASE + index) as usize),
                baseline,
                TEXT_ALIGN_ALIGN_CENTER,
                rgb15(0, 0, 0),
            )
        });
        rect.corner.y += 79;
        baseline.y += 79;
    }
    rect = RECT {
        corner: POINT { x: 3, y: 3 },
        extent: EXTENT {
            width: 58,
            height: 7,
        },
    };
    context.fill_rectangle(rgb15(0, 0, 20), rect);
    context.with_font_effect(None, || {
        context.draw_text(
            &font,
            kernel.game_text((STATUS_STRING_BASE + 5) as usize),
            POINT { x: 32, y: 9 },
            TEXT_ALIGN_ALIGN_CENTER,
            rgb15(2, 4, 30),
        )
    });
}
fn draw_full_panel(kernel: &Kernel, world: &World, context: &ContextLease) {
    let frames = frame(kernel.flag_stat_frame);
    context.draw_stamp(&frames, POINT { x: 0, y: 0 });
    if kernel.configuration.preferences().fonts == OPT_PC {
        draw_pc_sis(kernel, context);
    }
    let state = &world.data.flagship;
    draw_slots(context, &frames, &state.drive_slots, 0, 0);
    draw_slots(context, &frames, &state.jet_slots, 0, 1);
    draw_slots(context, &frames, &state.module_slots, 1, 2);
    let points = [
        (3, 62),
        (45, 62),
        (3, 78),
        (45, 78),
        (3, 46),
        (45, 46),
        (3, 94),
        (45, 94),
        (3, 30),
        (45, 30),
        (3, 110),
        (45, 110),
    ];
    let ships = world.built_ships_ref();
    for (id, (x, y)) in std::iter::successors(ships.head(), |&id| ships.succ(id)).zip(points) {
        let resources = world.fleet_resources(ships.get(id).species_id);
        context.draw_stamp(&frame(resources.icon().raw()), POINT { x, y });
    }
}
pub fn delta_sis_gauges(kernel: &Kernel, world: &mut World, crew: i16, fuel: i16, resources: i32) {
    if crew == 0 && fuel == 0 && resources == 0 {
        return;
    }
    let context = ContextLease::require(kernel.status_context);
    context.draw_batch(|| {
        if crew == UNDEFINED_DELTA {
            draw_full_panel(kernel, world, &context);
        }
        let font = font(kernel);
        context.set_font(&font);
        update_crew(&mut world.data.flagship, crew);
        if crew != 0 {
            draw_number(&context, &font, true, world.data.flagship.crew_enlisted);
        }
        if update_fuel(&mut world.data.flagship, fuel) {
            draw_number(
                &context,
                &font,
                false,
                (world.data.flagship.fuel_on_board / FUEL_TANK_SCALE) as u16,
            );
        }
        if crew == UNDEFINED_DELTA {
            draw_flagship_name(kernel, world, 0, true);
            draw_captains_name(kernel, world);
            draw_landers(kernel, world);
            draw_storage_bays(kernel, world, false);
        }
        let resources = resources as i16;
        if resources != 0 {
            update_resources(
                &mut world.data.flagship,
                resources,
                kernel.status_message_mode,
            );
            if resources == UNDEFINED_DELTA {
                context.fill_rectangle(rgb15(0, 8, 0), status::message_rect());
            }
            draw_status_message(kernel, world, None);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn owned_gauges_and_border_draw_without_publishing_campaign_state() {
        if !crate::native_test_process::own_graphics_process("ui::sis::gauges::tests::owned_gauges_and_border_draw_without_publishing_campaign_state") { return; }
        use crate::{
            campaign_display_test::CampaignDisplay,
            engine::Game,
            graphics::{context_abi::ContextFrame, drawable_abi::LoadedDrawable},
        };
        let mut game = Game::new();
        let display = CampaignDisplay::install(&mut game)
            .unwrap_or_else(|error| panic!("gauge fixture: {error}"));
        game.kernel
            .install_game_text(crate::strings::owned_table::LoadedTextTable::fixture(
                vec![b"A\0".to_vec(); (STATUS_STRING_BASE + 6) as usize],
            ));
        let font = font(&game.kernel);
        game.kernel.screen_context = game.kernel.status_context;
        // SAFETY: this serial fixture owns and releases its separate space context.
        let space = unsafe {
            #[cfg(feature = "debug-process")]
            {
                crate::graphics::context_abi::CreateContextAux(c"gauge-space".as_ptr())
            }
            #[cfg(not(feature = "debug-process"))]
            {
                crate::graphics::context_abi::CreateContextAux()
            }
        };
        game.kernel.space_context = space;
        let frames =
            LoadedDrawable::create(8, 64, 140, 100).unwrap_or_else(|| panic!("gauge frames"));
        game.kernel.flag_stat_frame = frames
            .lease()
            .frame(0)
            .unwrap_or_else(|| panic!("gauge frame"));
        game.kernel.font_grad_frame = game.kernel.flag_stat_frame;
        let context = ContextLease::require(game.kernel.status_context);
        context.set_frame(ContextFrame::retain(NonNull::new(game.kernel.screen)));
        game.world
            .data
            .flagship
            .module_slots
            .fill(NUM_MODULES as u8);
        game.world.data.flagship.drive_slots.fill(NUM_MODULES as u8);
        game.world.data.flagship.jet_slots.fill(NUM_MODULES as u8);
        game.world.data.flagship.module_slots[0] = CREW_POD as u8;
        game.world.data.flagship.module_slots[1] = STORAGE_BAY as u8;
        game.world.data.flagship.num_landers = 2;
        game.world.data.flagship.total_element_mass = 625;
        game.world.data.flagship.commander_name[..2].copy_from_slice(b"A\0");
        game.world.data.flagship.ship_name[..2].copy_from_slice(b"A\0");
        game.kernel
            .set_status_message_mode(StatMsgMode_SMM_RES_UNITS);
        delta_sis_gauges(&game.kernel, &mut game.world, 0, 0, 0);
        delta_sis_gauges(&game.kernel, &mut game.world, 5, 10, 20);
        assert_eq!(
            (
                game.world.data.flagship.crew_enlisted,
                game.world.data.flagship.res_units
            ),
            (5, 20)
        );
        for fonts in [OPT_PC, 1] {
            game.kernel
                .configuration
                .update_preferences(|preferences| preferences.fonts = fonts);
            delta_sis_gauges(
                &game.kernel,
                &mut game.world,
                UNDEFINED_DELTA,
                UNDEFINED_DELTA,
                i32::from(UNDEFINED_DELTA),
            );
            draw_flagship_name(&game.kernel, &game.world, 320, false);
        }
        draw_storage_bays(&game.kernel, &game.world, true);
        game.world.data.flagship.total_element_mass = 0;
        game.world
            .data
            .flagship
            .module_slots
            .fill(NUM_MODULES as u8);
        draw_storage_bays(&game.kernel, &game.world, true);
        crate::ui::border::draw_sis_frame(&game.kernel, &mut game.world, (320, 240));
        assert_eq!(
            (
                ContextLease::require(space).clip_rect().corner.x,
                ContextLease::require(space).clip_rect().extent.width
            ),
            (7, 242)
        );
        context.flush();
        game.kernel.flag_stat_frame = std::ptr::null_mut();
        game.kernel.font_grad_frame = std::ptr::null_mut();
        drop(context);
        drop(font);
        drop(frames);
        // SAFETY: queued draws are flushed; no borrowed space lease remains.
        unsafe { crate::graphics::context_abi::DestroyContext(space) };
        display.finish(game);
    }

    #[test]
    fn gauge_updates_preserve_saturation_wrapping_and_refresh_sentinels() {
        let mut state = crate::engine::World::new().data.flagship;
        state.crew_enlisted = 40;
        state.fuel_on_board = 105;
        state.res_units = 10;
        state.module_slots[0] = CREW_POD as u8;
        update_crew(&mut state, 20);
        assert_eq!(state.crew_enlisted, 50);
        update_crew(&mut state, -60);
        assert_eq!(state.crew_enlisted, 0);
        update_crew(&mut state, UNDEFINED_DELTA);
        assert_eq!(state.crew_enlisted, 0);
        assert!(!update_fuel(&mut state, -1));
        assert_eq!(state.fuel_on_board, 104);
        assert!(update_fuel(&mut state, -200));
        assert_eq!(state.fuel_on_board, 0);
        assert!(update_fuel(&mut state, UNDEFINED_DELTA));
        update_resources(&mut state, -11, StatMsgMode_SMM_RES_UNITS);
        assert_eq!(state.res_units, 0);
        state.res_units = u32::MAX;
        update_resources(&mut state, 2, StatMsgMode_SMM_RES_UNITS);
        assert_eq!(state.res_units, 1);
    }
}
