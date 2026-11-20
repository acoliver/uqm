use crate::battle::status_native::draw_battle_crew_amount as draw_crew;
use crate::engine::battle::link_queue::PickShip;
use crate::engine::battle::record::SHIP_INFO;
use crate::engine::Kernel;
use crate::graphics::{
    geometry::Color, geometry::Extent, geometry::Point, geometry::Rect, geometry::TextAlign,
};

const STATUS_WIDTH: i16 = 64;
const SHIP_INFO_HEIGHT: i16 = 65;
const GAUGE_YOFFS: i16 = SHIP_INFO_HEIGHT - 10;
const STARCON_TEXT_HEIGHT: i16 = 7;
const CREW_XOFFS: i16 = 4;
const ENERGY_XOFFS: i16 = 52;
const STAT_WIDTH: i16 = 7;
const UNIT_WIDTH: i16 = 2;
const UNIT_HEIGHT: i16 = 1;
const MAX_CREW_SIZE: u16 = 42;
const CREW_IMMUNE: u16 = 1 << 6;
const PLAYER_CAPTAIN: u16 = 1 << 13;
const OPT_PC: i32 = 0x02;
const RPG_PLAYER_NUM: i16 = 0;

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

pub fn draw_crew_energy_labels(
    context: &crate::graphics::context::ContextLease,
    frames: &crate::graphics::drawable::DrawableLease,
    pc_menu: bool,
    y: i16,
    state: i16,
) {
    let y = y + GAUGE_YOFFS + STARCON_TEXT_HEIGHT;
    let index = if pc_menu { 4 } else { 0 };
    if state == 0 {
        context.draw_stamp(
            &frames.context_frame(index),
            Point {
                x: CREW_XOFFS + (STAT_WIDTH >> 1) + 6,
                y,
            },
        );
    }
    let origin = Point {
        x: ENERGY_XOFFS + (STAT_WIDTH >> 1) - 5,
        y,
    };
    let energy = frames.context_frame(index + 1);
    if state >= 0 {
        context.draw_stamp(&energy, origin);
    } else {
        context.draw_filled_stamp(&energy, origin, rgb15(0x1f, 0x1f, 0x0a));
    }
}

pub fn draw_ship_name_text(
    context: &crate::graphics::context::ContextLease,
    font: &crate::graphics::font::FontLease,
    text: &std::ffi::CStr,
    char_count: u16,
    y: i16,
) {
    context.with_text_state(|| {
        for (baseline_y, color) in [
            (STARCON_TEXT_HEIGHT + 3 + y, rgb15(0x10, 0x10, 0x10)),
            (STARCON_TEXT_HEIGHT + 2 + y, rgb15(0, 0, 0)),
        ] {
            context.draw_text_bounded(
                font,
                text,
                Point {
                    x: STATUS_WIDTH >> 1,
                    y: baseline_y,
                },
                TextAlign::Center,
                color,
                char_count,
            );
        }
    });
    context.set_foreground(rgb15(0, 0, 0));
}

pub fn clear_ship_status(context: &crate::graphics::context::ContextLease, y: i16) {
    context.fill_rectangle(
        rgb15(0x0a, 0x0a, 0x0a),
        Rect {
            corner: Point { x: 2, y: 3 + y },
            extent: Extent {
                width: STATUS_WIDTH - 4,
                height: SHIP_INFO_HEIGHT - 3,
            },
        },
    );
}

pub fn outline_ship_status(context: &crate::graphics::context::ContextLease, y: i16) {
    let mut r = Rect {
        corner: Point { x: 0, y: 1 + y },
        extent: Extent {
            width: STATUS_WIDTH,
            height: 1,
        },
    };
    let dark = rgb15(0x08, 0x08, 0x08);
    context.fill_rectangle(dark, r);
    r.corner.y += 1;
    r.extent.width -= 1;
    context.fill_rectangle(dark, r);
    r.extent.width = 1;
    r.extent.height = SHIP_INFO_HEIGHT - 2;
    context.fill_rectangle(dark, r);
    r.corner.x += 1;
    context.fill_rectangle(dark, r);
    let light = rgb15(0x10, 0x10, 0x10);
    r.corner.x = STATUS_WIDTH - 1;
    context.fill_rectangle(light, r);
    r.corner.x = STATUS_WIDTH - 2;
    r.corner.y += 1;
    r.extent.height -= 1;
    context.fill_rectangle(light, r);
    r.corner.x = 0;
    r.corner.y = y;
    r.extent.width = STATUS_WIDTH;
    r.extent.height = 1;
    context.fill_rectangle(rgb15(0, 0, 0), r);
}

pub struct StatusTitle<'a> {
    pub name: &'a std::ffi::CStr,
    pub name_count: u16,
    pub caption: Option<(&'a std::ffi::CStr, u16)>,
}

pub struct StatusGraphics {
    pub context: crate::graphics::context::ContextLease,
    pub frames: crate::graphics::drawable::DrawableLease,
    pub font: crate::graphics::font::FontLease,
    pub tiny_font: crate::graphics::font::FontLease,
    pub crew_font: Option<crate::graphics::font::FontLease>,
    pub pc_menu: bool,
}

impl StatusGraphics {
    pub fn new(kernel: &Kernel, crew_font: Option<crate::graphics::font::FontLease>) -> Self {
        use crate::graphics::{context::ContextLease, font::FontLease};

        Self {
            context: ContextLease::retain(
                kernel
                    .status_context
                    .unwrap_or_else(|| panic!("status context not admitted")),
            ),
            frames: crate::graphics::drawable::retain_frames(&[kernel.status_frame])[0].clone(),
            font: FontLease::retain(
                kernel
                    .star_con_font
                    .unwrap_or_else(|| panic!("status font not admitted")),
            ),
            tiny_font: FontLease::retain(
                kernel
                    .tiny_font
                    .unwrap_or_else(|| panic!("status tiny font not admitted")),
            ),
            crew_font,
            pc_menu: kernel.configuration.preferences().menu == OPT_PC,
        }
    }

    pub fn draw(
        &self,
        ship: &mut SHIP_INFO,
        portrait: &crate::graphics::drawable::ContextFrame,
        y: i16,
        clip: Option<Rect>,
        title: Option<StatusTitle<'_>>,
    ) {
        let context = &self.context;
        let draw = || {
            context.draw_batch(|| {
                outline_ship_status(context, y);
                clear_ship_status(context, y);
                context.draw_stamp(
                    portrait,
                    Point {
                        x: STATUS_WIDTH >> 1,
                        y: 31 + y,
                    },
                );
                draw_gauge_frames(context, ship.max_crew, ship.max_energy, y);
                if let Some(title) = title {
                    draw_crew_energy_labels(context, &self.frames, self.pc_menu, y, 0);
                    draw_ship_name_text(context, &self.font, title.name, title.name_count, y);
                    if let Some((caption, characters)) = title.caption {
                        context.with_text_state(|| {
                            context.draw_text_bounded(
                                &self.tiny_font,
                                caption,
                                Point {
                                    x: STATUS_WIDTH >> 1,
                                    y: y + GAUGE_YOFFS + 3,
                                },
                                TextAlign::Center,
                                rgb15(0, 0, 0),
                                characters,
                            )
                        });
                        context.set_foreground(rgb15(0, 0, 0));
                    }
                }
                let changes = (ship.crew_level as i16, ship.energy_level as i16);
                ship.crew_level = 0;
                ship.energy_level = 0;
                update_statistics(
                    Some(context),
                    true,
                    self.pc_menu,
                    self.crew_font.as_ref(),
                    ship,
                    y,
                    changes,
                );
            })
        };
        context.with_selected(|| {
            if let Some(clip) = clip {
                let mut bounds = context.clip_rect();
                bounds.corner.x += clip.corner.x;
                bounds.corner.y += clip.corner.y & !1;
                bounds.extent = clip.extent;
                bounds.extent.height += clip.corner.y & 1;
                context.with_clip(bounds, || {
                    context.with_origin(
                        Point {
                            x: -clip.corner.x,
                            y: -(clip.corner.y & !1),
                        },
                        draw,
                    )
                });
            } else {
                draw();
            }
        });
    }
}

pub fn draw_battle_status(
    cx: &mut crate::engine::battle::Cx<'_>,
    id: crate::engine::battle::ShipId,
    clip: Option<Rect>,
) {
    if cx.kernel.status_context.is_none() {
        return;
    }
    let graphics = StatusGraphics::new(cx.kernel, cx.kernel.battle_font());
    let selected = cx.pick_ship(id);
    let y = cx.kernel.status_offsets[selected.player as usize];
    let descriptor = cx.desc(id);
    let mut info = descriptor.ship_info;
    let portrait = crate::graphics::drawable::ContextFrame::retain(info.icons).next_frame();
    let title = battle_title(descriptor, selected, &cx.world.data.flagship.ship_name);
    graphics.draw(&mut info, &portrait, y, clip, title);
    let mut stats = cx.race_stats_mut(id);
    stats.ship_info.crew_level = info.crew_level;
    stats.ship_info.energy_level = info.energy_level;
}

fn battle_title<'a>(
    descriptor: &'a crate::engine::battle::record::RaceDescriptor,
    ship: PickShip,
    flagship: &'a [u8],
) -> Option<StatusTitle<'a>> {
    if ship.captain != 0 {
        let (name, name_count) = descriptor.label(1);
        Some(StatusTitle {
            name,
            name_count,
            caption: Some(descriptor.label(u16::from(ship.captain))),
        })
    } else if ship.player == RPG_PLAYER_NUM {
        Some(StatusTitle {
            name: std::ffi::CStr::from_bytes_until_nul(flagship)
                .unwrap_or_else(|error| panic!("flagship name: {error}")),
            name_count: u16::MAX,
            caption: None,
        })
    } else {
        None
    }
}

pub fn update_statistics(
    context: Option<&crate::graphics::context::ContextLease>,
    numeric_crew: bool,
    pc_menu: bool,
    font: Option<&crate::graphics::font::FontLease>,
    ship: &mut SHIP_INFO,
    y_offs: i16,
    deltas: (i16, i16),
) {
    let mut draw = |color, rect| {
        if let Some(context) = context {
            match rect {
                Some(rect) => context.fill_rectangle(color, rect),
                None => context.set_foreground(color),
            }
        }
    };
    let (crew_delta, energy_delta) = deltas;
    if crew_delta != 0 {
        draw_crew_delta(&mut draw, ship, y_offs, crew_delta);
        if numeric_crew
            && crate::battle::status_native::has_crew_counter(ship.max_crew, ship.ship_flags)
        {
            if let Some(context) = context {
                draw_crew(
                    context,
                    font.unwrap_or_else(|| {
                        panic!("numeric crew display requires the loaded battle font")
                    }),
                    ship.crew_level,
                    y_offs,
                    pc_menu,
                );
            }
        }
    }
    if energy_delta != 0 {
        draw_energy_delta(&mut draw, ship, y_offs, energy_delta);
    }
}

fn draw_crew_delta(
    draw: &mut impl FnMut(Color, Option<Rect>),
    ship: &mut SHIP_INFO,
    y_offs: i16,
    crew_delta: i16,
) {
    let y = GAUGE_YOFFS + y_offs;
    let mut r = Rect {
        corner: Point { x: 0, y: 0 },
        extent: Extent {
            width: UNIT_WIDTH,
            height: UNIT_HEIGHT,
        },
    };
    let old_blocks = std::cmp::min(ship.crew_level, MAX_CREW_SIZE) as i16;
    let new_level = (ship.crew_level as i32 + crew_delta as i32) as u16;
    let new_blocks = std::cmp::min(new_level, MAX_CREW_SIZE) as i16;
    if crew_delta > 0 {
        r.corner.y = (y + 1) - (((old_blocks + 1) >> 1) * (UNIT_HEIGHT + 1));
        let color = if ship.ship_flags & CREW_IMMUNE != 0 {
            rgb15(0x0a, 0x0a, 0x0a)
        } else {
            rgb15(0, 0x14, 0)
        };
        draw(color, None);
        for i in old_blocks..new_blocks {
            r.corner.x = CREW_XOFFS + 1;
            if i & 1 == 0 {
                r.corner.x += UNIT_WIDTH + 1;
                r.corner.y -= UNIT_HEIGHT + 1;
            }
            draw(color, Some(r));
        }
    } else {
        let color = rgb15(0, 0, 0);
        draw(color, None);
        r.corner.y = (y + 1) - (((old_blocks + 2) >> 1) * (UNIT_HEIGHT + 1));
        let mut i = old_blocks;
        while i > new_blocks {
            r.corner.x = CREW_XOFFS + 1 + UNIT_WIDTH + 1;
            if i & 1 == 0 {
                r.corner.x -= UNIT_WIDTH + 1;
                r.corner.y += UNIT_HEIGHT + 1;
            }
            draw(color, Some(r));
            i -= 1;
        }
    }
    draw_crew_marker(draw, ship, new_level, y);
    ship.crew_level = new_level;
}

fn draw_energy_delta(
    draw: &mut impl FnMut(Color, Option<Rect>),
    ship: &mut SHIP_INFO,
    y_offs: i16,
    mut energy_delta: i16,
) {
    let y = GAUGE_YOFFS + y_offs;
    let mut r = Rect {
        corner: Point { x: 0, y: 0 },
        extent: Extent {
            width: UNIT_WIDTH,
            height: UNIT_HEIGHT,
        },
    };
    if energy_delta > 0 {
        let color = rgb15(0x14, 0, 0);
        draw(color, None);
        r.corner.y = (y + 1) - ((ship.energy_level as i16 + 1) >> 1) * (UNIT_HEIGHT + 1);
        while energy_delta > 0 {
            r.corner.x = ENERGY_XOFFS + 1;
            if ship.energy_level & 1 == 0 {
                r.corner.x += UNIT_WIDTH + 1;
                r.corner.y -= UNIT_HEIGHT + 1;
            }
            draw(color, Some(r));
            ship.energy_level += 1;
            energy_delta -= 1;
        }
    } else {
        let color = rgb15(0, 0, 0);
        draw(color, None);
        r.corner.y = (y + 1) - ((ship.energy_level as i16 + 2) >> 1) * (UNIT_HEIGHT + 1);
        while energy_delta < 0 {
            r.corner.x = ENERGY_XOFFS + 1 + UNIT_WIDTH + 1;
            if ship.energy_level & 1 == 0 {
                r.corner.x -= UNIT_WIDTH + 1;
                r.corner.y += UNIT_HEIGHT + 1;
            }
            draw(color, Some(r));
            ship.energy_level -= 1;
            energy_delta += 1;
        }
    }
}

fn draw_crew_marker(
    draw: &mut impl FnMut(Color, Option<Rect>),
    ship: &SHIP_INFO,
    new_level: u16,
    y: i16,
) {
    if ship.ship_flags & PLAYER_CAPTAIN != 0
        && ((ship.crew_level > MAX_CREW_SIZE) != (new_level > MAX_CREW_SIZE)
            || ship.crew_level == 0)
        && new_level != 0
    {
        let color = if new_level > MAX_CREW_SIZE {
            rgb15(0, 0x14, 0)
        } else {
            rgb15(0x0a, 0x0a, 0x1f)
        };
        let rect = Rect {
            corner: Point {
                x: CREW_XOFFS + 1 + UNIT_WIDTH + 1,
                y: y - UNIT_HEIGHT,
            },
            extent: Extent {
                width: UNIT_WIDTH,
                height: UNIT_HEIGHT,
            },
        };
        draw(color, Some(rect));
    }
}

pub fn draw_gauge_frames(
    context: &crate::graphics::context::ContextLease,
    max_crew: u16,
    max_energy: u8,
    y: i16,
) {
    let crew = (((std::cmp::min(max_crew, MAX_CREW_SIZE) + 1) & !1) + 1) as i16;
    let energy = ((((max_energy + 1) >> 1) << 1) + 1) as i16;
    let bottom = GAUGE_YOFFS + y;
    let draw = |color, x, y, width, height| {
        context.fill_rectangle(
            color,
            Rect {
                corner: Point { x, y },
                extent: Extent { width, height },
            },
        );
    };
    let dark = rgb15(0x08, 0x08, 0x08);
    draw(dark, CREW_XOFFS - 1, bottom + 1, STAT_WIDTH + 2, 1);
    draw(dark, ENERGY_XOFFS - 1, bottom + 1, STAT_WIDTH + 2, 1);
    draw(
        dark,
        ENERGY_XOFFS + STAT_WIDTH,
        bottom + 1 - energy,
        1,
        energy,
    );
    draw(dark, CREW_XOFFS + STAT_WIDTH, bottom + 1 - crew, 1, crew);
    let light = rgb15(0x10, 0x10, 0x10);
    draw(light, CREW_XOFFS - 1, bottom - crew, STAT_WIDTH + 2, 1);
    draw(light, ENERGY_XOFFS - 1, bottom - energy, STAT_WIDTH + 2, 1);
    draw(light, ENERGY_XOFFS - 1, bottom - energy, 1, energy + 1);
    draw(light, CREW_XOFFS - 1, bottom - crew, 1, crew + 1);
    let black = rgb15(0, 0, 0);
    draw(black, CREW_XOFFS, bottom + 1 - crew, STAT_WIDTH, crew);
    draw(black, ENERGY_XOFFS, bottom + 1 - energy, STAT_WIDTH, energy);
}
