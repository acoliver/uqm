//! Retained save-picker drawing over prepared summary values.
use super::{layout::BLACK_COLOR, picker::PickerState, preview};
use crate::starbase::ids::{
    NAVIGATION_STRING_BASE, PLANET_NUMBER_BASE, SAFE_X, SAFE_Y, SAVEGAME_STRING_BASE, SIS_ORG_X,
    SIS_ORG_Y, STARBASE_STRING_BASE, STATUS_WIDTH,
};
use crate::ui::{
    rgb15,
    sis::{clear_sis_rect, draw_status_message},
};
use std::ffi::{CStr, CString};
use uqm_engine::graphics::{
    context::ContextLease, drawable::ContextFrame, font::FontLease, geometry::Color,
    geometry::Extent, geometry::Point, geometry::Rect, geometry::TextAlign,
};
use uqm_engine::{save::summary_record::SummaryDesc, Kernel, World};

const SUMMARY_X_OFFS: i16 = 14;
const SUMMARY_SIDE_OFFS: i16 = 7;

pub(super) struct PickerDisplay {
    pub context: ContextLease,
    summary_frame: ContextFrame,
    misc: ContextFrame,
    tiny: FontLease,
    star: FontLease,
}

impl PickerDisplay {
    pub fn new(kernel: &Kernel) -> Self {
        Self {
            context: ContextLease::require(kernel.space_context),
            summary_frame: ContextFrame::retain(kernel.play_frame).wrapped_frame(39),
            misc: ContextFrame::retain(kernel.misc_data_frame),
            tiny: kernel
                .tiny_font_lease()
                .unwrap_or_else(|| panic!("picker tiny font absent")),
            star: FontLease::retain(
                kernel
                    .star_con_font
                    .unwrap_or_else(|| panic!("picker star font absent")),
            ),
        }
    }

    pub fn header(&self, saving: bool) {
        let count = self.summary_frame.frame_count();
        self.context.draw_stamp(
            &self
                .summary_frame
                .wrapped_frame(count.wrapping_sub(if saving { 3 } else { 2 })),
            Point {
                x: SUMMARY_X_OFFS + 1,
                y: 0,
            },
        );
    }

    pub fn redraw(
        &self,
        kernel: &Kernel,
        world: &mut World,
        dimensions: (i32, i32),
        random: fn() -> u32,
        state: &PickerState,
    ) {
        self.context.with_batch(|| {
            self.context.draw_stamp(
                &self
                    .summary_frame
                    .wrapped_frame(self.summary_frame.frame_count().wrapping_sub(1)),
                Point { x: 0, y: 0 },
            );
            self.summary(
                kernel,
                world,
                dimensions,
                random,
                &state.summary[usize::from(state.slot)],
            );
            self.selection(kernel, state);
        });
    }

    fn cargo(&self, sis: &uqm_engine::world::SisState) {
        const COLORS: [Color; 9] = [
            rgb15(2, 14, 19),
            rgb15(25, 0, 0),
            rgb15(16, 16, 16),
            rgb15(3, 5, 30),
            rgb15(0, 24, 0),
            rgb15(27, 27, 0),
            rgb15(30, 13, 0),
            rgb15(20, 0, 20),
            rgb15(15, 0, 25),
        ];
        self.context.select();
        self.context.with_batch(|| {
            self.context.set_font(&self.star);
            let mut origin = Point {
                x: 7 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS + 3,
                y: 17,
            };
            let mut baseline = Point {
                x: 33 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS + 3,
                y: 20,
            };

            for (i, &color) in COLORS[..8].iter().enumerate() {
                if i == 4 {
                    origin.x += 36;
                    origin.y = 17;
                    baseline.x += 36;
                    baseline.y = 20;
                }
                self.context
                    .draw_stamp(&self.misc.wrapped_frame(11 + i as u16 * 5), origin);
                origin.y += 12;
                self.context.draw_text(
                    &self.star,
                    &text(sis.element_amounts[i].to_string(), 39),
                    baseline,
                    TextAlign::Right,
                    color,
                );
                baseline.y += 12;
            }
            origin = Point {
                x: 24 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS,
                y: 68,
            };
            self.context
                .draw_stamp(&self.misc.wrapped_frame(68), origin);
            self.context.draw_text(
                &self.star,
                &text(sis.total_bio_mass.to_string(), 39),
                Point {
                    x: 50 + SUMMARY_X_OFFS,
                    y: 71,
                },
                TextAlign::Right,
                COLORS[8],
            );
        });
    }

    fn summary(
        &self,
        kernel: &Kernel,
        world: &mut World,
        dimensions: (i32, i32),
        random: fn() -> u32,
        summary: &SummaryDesc,
    ) {
        self.context.with_batch(|| {
            if summary.year_index == 0 {
                self.context.draw_stamp(
                    &self
                        .summary_frame
                        .wrapped_frame(self.summary_frame.frame_count().wrapping_sub(4)),
                    Point { x: 0, y: 0 },
                );
                return;
            }
            let status = ContextLease::require(kernel.status_context);
            status.with_selected(|| {
                let rect = Rect {
                    corner: Point {
                        x: (i32::from(SIS_ORG_X)
                            + ((sis_width(dimensions) - STATUS_WIDTH) >> 1)
                            + i32::from(SAFE_X)
                            - 16
                            + i32::from(SUMMARY_X_OFFS)) as i16,
                        y: SIS_ORG_Y as i16,
                    },
                    extent: Extent {
                        width: STATUS_WIDTH as i16,
                        height: (dimensions.1 - i32::from(SAFE_Y) * 2) as i16,
                    },
                };
                preview::with_preview(world, summary, random, |world| {
                    status.with_clip(rect, || {
                        let date = summary_date(kernel, summary, 255);
                        clear_sis_rect(kernel, world, dimensions, 8);
                        draw_status_message(kernel, world, Some(&date));
                        world.built_ships().reinit();
                    });
                    self.summary_details(kernel, world, dimensions, summary);
                });
            });
        });
    }

    fn summary_details(
        &self,
        kernel: &Kernel,
        world: &World,
        dimensions: (i32, i32),
        summary: &SummaryDesc,
    ) {
        self.context.select();
        self.devices(summary);
        self.context.set_font(&self.star);
        let x = 173 + SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS;
        if summary.flags & 128 != 0 {
            self.context.draw_stamp(
                &self.summary_frame,
                Point {
                    x: SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS + 6,
                    y: 0,
                },
            );
            self.context.draw_stamp(
                &self.summary_frame.next_frame(),
                Point {
                    x: SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS,
                    y: 0,
                },
            );
        } else {
            self.cargo(&summary.sis);
            let radar = ContextLease::require(kernel.radar_context);
            radar.with_selected(|| {
                let rect = Rect {
                    corner: Point {
                        x: SIS_ORG_X + 10 + SUMMARY_X_OFFS - SUMMARY_SIDE_OFFS,
                        y: SIS_ORG_Y as i16 + 84,
                    },
                    extent: radar.clip_rect().extent,
                };
                radar.with_clip(rect, || preview::lander(kernel, summary));
            });
            self.context.select();
            self.context.draw_text(
                &self.star,
                &text(summary.sis.res_units.to_string(), 255),
                Point { x, y: 102 },
                TextAlign::Center,
                rgb15(16, 0, 16),
            );
        }
        self.context.draw_text(
            &self.star,
            &text(
                (u32::from(summary.mcredit_lo) | u32::from(summary.mcredit_hi) << 8).to_string(),
                255,
            ),
            Point { x, y: 126 },
            TextAlign::Center,
            rgb15(16, 0, 16),
        );
        let (location, title) = location_labels(kernel, world, dimensions, summary);
        self.context.set_font(&self.tiny);
        self.context.draw_text(
            &self.tiny,
            &location,
            Point { x: 6, y: 145 },
            TextAlign::Left,
            rgb15(27, 0, 27),
        );
        self.context.draw_text(
            &self.tiny,
            &title,
            Point {
                x: (sis_width(dimensions) - 57 - 4 + (55 >> 1)) as i16,
                y: 145,
            },
            TextAlign::Center,
            rgb15(27, 0, 27),
        );
    }

    fn devices(&self, summary: &SummaryDesc) {
        for i in 0..4 {
            for j in 0..4 {
                let index = i * 4 + j;
                if index < usize::from(summary.num_devices) {
                    self.context.draw_stamp(
                        &self
                            .misc
                            .wrapped_frame(77 + u16::from(summary.device_list[index])),
                        Point {
                            x: 140 + SUMMARY_X_OFFS + SUMMARY_SIDE_OFFS + j as i16 * 18,
                            y: 13 + i as i16 * 18,
                        },
                    );
                }
            }
        }
    }

    fn selection(&self, kernel: &Kernel, state: &PickerState) {
        use super::picker::{MAX_SAVED_GAMES, SAVES_PER_PAGE};
        self.context.with_batch(|| {
            self.context.set_font(&self.tiny);
            self.context.fill_rectangle(
                BLACK_COLOR,
                Rect {
                    corner: Point { x: 1, y: 160 },
                    extent: Extent {
                        width: 240,
                        height: 65,
                    },
                },
            );
            let slot = u16::from(state.slot);
            let first = slot - slot % SAVES_PER_PAGE;
            for current in first..(first + SAVES_PER_PAGE).min(MAX_SAVED_GAMES as u16) {
                let summary = &state.summary[usize::from(current)];
                let color = if current == slot {
                    rgb15(27, 0, 27)
                } else {
                    rgb15(0, 0, 20)
                };
                let rect = Rect {
                    corner: Point {
                        x: 8,
                        y: (160 + (current - first) * 13) as i16,
                    },
                    extent: Extent {
                        width: if MAX_SAVED_GAMES > 99 { 20 } else { 15 },
                        height: 11,
                    },
                };
                self.context.draw_rectangle(color, rect);
                self.context.draw_text(
                    &self.tiny,
                    &text(format!("{current:02}"), 255),
                    Point {
                        x: rect.corner.x + 3,
                        y: rect.corner.y + 8,
                    },
                    TextAlign::Left,
                    color,
                );
                let rect = Rect {
                    corner: Point {
                        x: 30 + SAFE_X,
                        ..rect.corner
                    },
                    extent: Extent {
                        width: 204 - SAFE_X,
                        ..rect.extent
                    },
                };
                self.context.draw_rectangle(color, rect);
                let label = selection_label(kernel, summary);
                self.context.draw_text(
                    &self.tiny,
                    &label,
                    Point {
                        x: rect.corner.x + 3,
                        y: rect.corner.y + 8,
                    },
                    TextAlign::Left,
                    color,
                );
            }
        });
    }
}

fn sis_width(dimensions: (i32, i32)) -> i32 {
    dimensions.0 - STATUS_WIDTH - i32::from(SAFE_X) * 2 - 14
}

fn location_labels(
    kernel: &Kernel,
    world: &World,
    dimensions: (i32, i32),
    summary: &SummaryDesc,
) -> (CString, CString) {
    let x_factor = (((dimensions.0 - STATUS_WIDTH - i32::from(SAFE_X) * 2) << 5) >> 4) * 195;
    let y_factor = (((dimensions.1 - i32::from(SAFE_Y) * 2) << 5) >> 4) * 25;
    let mut point = Point {
        x: (summary
            .sis
            .log_x
            .wrapping_mul(6250)
            .wrapping_add(x_factor >> 1)
            / x_factor) as i16,
        y: (9999
            - summary
                .sis
                .log_y
                .wrapping_mul(625)
                .wrapping_add(y_factor >> 1)
                / y_factor) as i16,
    };
    let location = match summary.activity {
        1 | 4 | 7 | 8 => {
            match crate::planets::starmap::naming::name_near(kernel, world.catalog(), point) {
                Some((name, star)) => {
                    point = star;
                    text(name.to_bytes(), 255)
                }
                None => c"".to_owned(),
            }
        }
        3 => text(
            kernel.game_text(NAVIGATION_STRING_BASE as usize).to_bytes(),
            255,
        ),
        6 => text(
            kernel
                .game_text((NAVIGATION_STRING_BASE + 1) as usize)
                .to_bytes(),
            255,
        ),
        _ => c"".to_owned(),
    };
    let title = match summary.activity {
        8 => text(
            kernel.game_text(STARBASE_STRING_BASE as usize).to_bytes(),
            255,
        ),
        1 => text(
            kernel
                .game_text((PLANET_NUMBER_BASE + 32) as usize)
                .to_bytes(),
            255,
        ),
        7 => text(terminated(&summary.sis.planet_name).to_bytes(), 255),
        _ => text(
            format!(
                "{:03}.{:01} : {:03}.{:01}",
                (i32::from(point.x) / 10) as u32,
                (i32::from(point.x) % 10) as u32,
                (i32::from(point.y) / 10) as u32,
                (i32::from(point.y) % 10) as u32
            ),
            255,
        ),
    };
    (location, title)
}

fn selection_label(kernel: &Kernel, summary: &SummaryDesc) -> CString {
    if summary.year_index == 0 {
        return text(
            kernel
                .game_text((SAVEGAME_STRING_BASE + 3) as usize)
                .to_bytes(),
            255,
        );
    }
    let date = summary_date(kernel, summary, 79);
    let name = if summary.save_name[0] == 0 {
        kernel.game_text((SAVEGAME_STRING_BASE + 4) as usize)
    } else {
        terminated(&summary.save_name)
    };
    let mut bytes = date.into_bytes();
    bytes.extend_from_slice(b": ");
    bytes.extend_from_slice(name.to_bytes());
    text(bytes, 255)
}

pub(super) fn summary_date(kernel: &Kernel, summary: &SummaryDesc, max_bytes: usize) -> CString {
    text(
        crate::ui::sis::campaign_date(
            kernel,
            summary.month_index,
            summary.day_index,
            summary.year_index,
        )
        .into_bytes(),
        max_bytes,
    )
}

fn terminated(bytes: &[u8]) -> &CStr {
    CStr::from_bytes_until_nul(bytes)
        .unwrap_or_else(|error| panic!("unterminated save label: {error}"))
}

fn text(bytes: impl AsRef<[u8]>, limit: usize) -> CString {
    CString::new(&bytes.as_ref()[..bytes.as_ref().len().min(limit)])
        .unwrap_or_else(|error| panic!("invalid picker text: {error}"))
}
