//! Drawing procedures of the Starbase and Outfit screens.
//!
//! Rust bodies of `DrawBaseStateStrings`, `rotateStarbase`, `DrawShipPiece`
//! (`starbase.c`), `RedistributeFuel`, `DisplayLanders` (`outfit.c`),
//! `DrawMenuStateStrings` for the Outfit menu (`menu.c`), `GetGaugeRect`,
//! `DrawFlagshipName` and `DrawFlagshipStats` (`sis.c`). Each one issues the
//! same drawing calls, in the same order, as the C function, through
//! [`Gfx`].

use super::gfx::{
    Align, ContextId, FlashTarget, FontId, FrameId, Gfx, Point, Rect, Rgba, Stamp, TextDraw,
};
use super::ids::{
    RadarLayout, DRIVE_SIDE, DRIVE_TOP, EMPTY_SLOT, FLAGSHIP_STRING_BASE, FUSION_THRUSTER,
    JET_SIDE, JET_TOP, LANDER_WIDTH, LANDER_X, LANDER_Y, MAINMENU_STRING_BASE, MAX_LANDERS,
    MODULE_SIDE, MODULE_TOP, NAMING_STRING_BASE, NUM_MODULES, NUM_MODULE_SLOTS, PM_EXIT_OUTFIT,
    PM_FUEL, RADAR_HEIGHT, RADAR_WIDTH, SAFE_X, SAFE_Y, SHIP_NAME_HEIGHT, SHIP_PIECE_OFFSET,
    STARBASE_STRING_BASE, TURNING_JETS,
};
use super::platform::{SisView, StarbaseError, UiStyle, ONE_SECOND};
use super::sis_modules::{
    fuel_bar_layout, FuelBar, ModuleSlots, ModuleType, FUEL_TANK_CAPACITY, FUEL_TANK_SCALE,
    HEFUEL_TANK_CAPACITY,
};
use super::starbase_menu::StarbaseState;

const HIGHLIGHT_YELLOW: Rgba = Rgba::rgb15(0x1F, 0x1F, 0x0A);
const MENU_BACKDROP: Rgba = Rgba::rgb15(0x0A, 0x0A, 0x0A);
const THREEDO_MENU_TEXT: Rgba = Rgba::rgb15(0x00, 0x1F, 0x00);
const THREEDO_FLAGSHIP_NAME_TEXT: Rgba = Rgba::rgb15(0x14, 0x0A, 0x00);
const EMPTY_FUEL_BAR: Rgba = Rgba::rgb15(0x0B, 0x00, 0x00);
const PCMENU_BACKGROUND: Rgba = Rgba::rgb15(0x00, 0x00, 0x15);
const PCMENU_TOP_LEFT_BORDER: Rgba = Rgba::rgb15(0x0F, 0x0F, 0x0F);
const PCMENU_BOTTOM_RIGHT_BORDER: Rgba = Rgba::rgb15(0x06, 0x06, 0x06);
const PCMENU_TEXT: Rgba = Rgba::rgb15(0x00, 0x15, 0x15);
const PCMENU_SELECTION_TEXT: Rgba = Rgba::rgb15(0x0A, 0x1F, 0x1F);
const PCMENU_SELECTION_BACKGROUND: Rgba = Rgba::rgb15(0x0A, 0x0A, 0x1F);

/// `BATTLE_FRAME_RATE` (`battle.h`): `ONE_SECOND / 24`.
const BATTLE_FRAME_RATE: u32 = ONE_SECOND / 24;

const fn text_at(x: i32, y: i32, align: Align, text: String) -> TextDraw {
    TextDraw {
        baseline: Point {
            x: x as i16,
            y: y as i16,
        },
        align,
        text,
    }
}

/// `DrawBaseStateStrings`. `old` of `None` is the C `(STARBASE_STATE)~0`
/// of the first draw: every entry but the highlighted one is drawn, and the
/// loop leaves the old state at `DEPART_BASE`, which is then drawn too.
pub fn draw_base_state_strings<G: Gfx + ?Sized>(
    g: &G,
    old: Option<StarbaseState>,
    new: StarbaseState,
) {
    g.set_context(ContextId::Screen);
    g.set_font(FontId::StarCon);
    g.set_fg_color(Rgba::BLACK);

    let x = 73 - 4 + i32::from(SAFE_X);
    let row_y = |state: StarbaseState| {
        106 + 28 + (i32::from(SAFE_Y) + 4) + (23 - 4) * i32::from(state as u8)
    };
    let string =
        |state: StarbaseState| g.game_string(STARBASE_STRING_BASE + 1 + i32::from(state as u8));

    let old = match old {
        Some(state) => state,
        None => {
            for state in [
                StarbaseState::TalkCommander,
                StarbaseState::OutfitStarship,
                StarbaseState::Shipyard,
            ] {
                if state != new {
                    g.draw_text(&text_at(x, row_y(state), Align::Center, string(state)));
                }
            }
            StarbaseState::DepartBase
        }
    };

    g.draw_text(&text_at(x, row_y(old), Align::Center, string(old)));
    g.set_fg_color(HIGHLIGHT_YELLOW);
    g.draw_text(&text_at(x, row_y(new), Align::Center, string(new)));
}

/// The animation state `rotateStarbase` keeps in function statics and in
/// `MENU_STATE.flash_frame0`/`flash_rect0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StarbaseAnimation {
    next_time: u32,
    frame: FrameId,
    origin: Point,
}

/// `rotateStarbase`. `setup_frame` is the `AniFrame` argument of the first
/// call.
pub fn rotate_starbase<G: Gfx + ?Sized>(
    g: &G,
    animation: &mut StarbaseAnimation,
    setup_frame: Option<FrameId>,
    now: u32,
) {
    if let Some(frame) = setup_frame {
        animation.frame = frame;
        animation.origin = Point {
            x: SAFE_X,
            y: SAFE_Y + 4,
        };
    }

    if now >= animation.next_time || setup_frame.is_some() {
        animation.next_time = now + ONE_SECOND / 20;

        g.draw_stamp(Stamp {
            origin: animation.origin,
            frame: animation.frame,
        });

        let mut next = g.inc_frame(animation.frame);
        if g.frame_index(next) == 0 {
            next = g.inc_frame(next);
        }
        animation.frame = next;
    }
}

fn piece_slot(slots: &[u8], index: usize) -> u16 {
    u16::from(slots[index])
}

fn fill(g: &(impl Gfx + ?Sized), corner: Point, width: i32, height: i32) {
    g.fill_rect(Rect {
        corner,
        width: width as i16,
        height: height as i16,
    });
}

fn shifted(point: Point, dx: i32, dy: i32) -> Point {
    Point {
        x: (i32::from(point.x) + dx) as i16,
        y: (i32::from(point.y) + dy) as i16,
    }
}

/// `DrawShipPiece`.
pub fn draw_ship_piece<G: Gfx + ?Sized>(
    g: &G,
    sis: &SisView,
    module_frame: FrameId,
    which_piece: u16,
    which_slot: u16,
    blueprint: bool,
) {
    let offset = i32::from(SHIP_PIECE_OFFSET);
    let slot = usize::from(which_slot);
    let last_slot = NUM_MODULE_SLOTS - 1;
    let mut piece = which_piece;
    let mut repair: i32 = 0;

    let (side_origin, top_origin) = if piece == FUSION_THRUSTER || piece == EMPTY_SLOT {
        (DRIVE_SIDE, DRIVE_TOP)
    } else if piece == TURNING_JETS || piece == EMPTY_SLOT + 1 {
        (JET_SIDE, JET_TOP)
    } else {
        if piece < EMPTY_SLOT + 2 {
            repair = 1;
            if piece < EMPTY_SLOT
                && (slot == 0 || piece_slot(&sis.module_slots, slot - 1) < EMPTY_SLOT)
            {
                repair += 1;
            }
        } else if !blueprint {
            if slot == 0 || slot >= NUM_MODULE_SLOTS - 3 {
                piece += 1;
            }
            if slot < last_slot && piece_slot(&sis.module_slots, slot + 1) < EMPTY_SLOT {
                repair = -1;
                if piece == EMPTY_SLOT + 3 || slot + 1 == NUM_MODULE_SLOTS - 3 {
                    repair -= 1;
                }
            }
        }
        (MODULE_SIDE, MODULE_TOP)
    };

    let slot_shift = i32::from(which_slot) * offset;
    let side_origin = shifted(side_origin, slot_shift, 0);
    let mut side_frame: FrameId = 0;
    let mut old_color: Option<Rgba> = None;

    // Frames per module: NUM_MODULES - 1 + 6 is the offset between a top view
    // and its side view.
    let side_view_offset = (i32::from(NUM_MODULES) - 1) + 6;

    if repair < 0 {
        let index =
            ((i32::from(NUM_MODULES) - 1) + (6 - 2)) + (i32::from(NUM_MODULES) + 6) - (repair + 1);
        side_frame = g.abs_frame(module_frame, index as u16);
        g.draw_stamp(Stamp {
            origin: side_origin,
            frame: side_frame,
        });
    } else if repair != 0 {
        old_color = Some(g.set_fg_color(Rgba::BLACK));
        fill(g, side_origin, offset, 1);
        fill(g, shifted(side_origin, 0, 23 - 1), offset, 1);

        if repair == 2 {
            fill(g, side_origin, 1, 8);
            fill(g, shifted(side_origin, 0, 15), 1, 8);
        }
        if slot < last_slot {
            fill(g, shifted(side_origin, offset, 0), 1, 8);
            fill(g, shifted(side_origin, offset, 15), 1, 8);
        }
    }

    if blueprint {
        if repair != 0 {
            if let Some(color) = old_color {
                g.set_fg_color(color);
            }
        }
        side_frame = g.abs_frame(module_frame, piece - 1);
        g.draw_filled_stamp(Stamp {
            origin: side_origin,
            frame: side_frame,
        });
        return;
    }

    let top_origin = shifted(top_origin, slot_shift, 0);
    if repair < 0 {
        let top_frame = g.rel_frame(side_frame, -(side_view_offset as i16));
        g.draw_stamp(Stamp {
            origin: top_origin,
            frame: top_frame,
        });
    } else if repair != 0 {
        fill(g, top_origin, offset, 1);
        fill(g, shifted(top_origin, 0, 32 - 1), offset, 1);

        if repair == 2 {
            fill(g, top_origin, 1, 12);
            fill(g, shifted(top_origin, 0, 20), 1, 12);
        }
        repair = i32::from(slot < last_slot);
        if repair != 0 {
            fill(g, shifted(top_origin, offset, 0), 1, 12);
            fill(g, shifted(top_origin, offset, 20), 1, 12);
        }
    }

    let top_frame = g.abs_frame(module_frame, piece);
    g.draw_stamp(Stamp {
        origin: top_origin,
        frame: top_frame,
    });
    let side_frame = g.rel_frame(top_frame, side_view_offset as i16);
    g.draw_stamp(Stamp {
        origin: side_origin,
        frame: side_frame,
    });

    if which_slot == 1 && piece == EMPTY_SLOT + 2 {
        let frame = g.abs_frame(module_frame, NUM_MODULES + 5);
        g.draw_stamp(Stamp {
            origin: shifted(top_origin, -offset, 0),
            frame,
        });
        let frame = g.rel_frame(frame, side_view_offset as i16);
        g.draw_stamp(Stamp {
            origin: shifted(side_origin, -offset, 0),
            frame,
        });
    }

    if repair != 0 {
        let next_slot = slot + 1;
        let mut next_piece = piece_slot(&sis.module_slots, next_slot);
        if next_piece == EMPTY_SLOT + 2 && next_slot >= NUM_MODULE_SLOTS - 3 {
            next_piece += 1;
        }
        let top_frame = g.abs_frame(module_frame, next_piece);
        g.draw_stamp(Stamp {
            origin: shifted(top_origin, offset, 0),
            frame: top_frame,
        });
        let side_frame = g.rel_frame(top_frame, side_view_offset as i16);
        g.draw_stamp(Stamp {
            origin: shifted(side_origin, offset, 0),
            frame: side_frame,
        });
    }
}

/// `RedistributeFuel` (`outfit.c:100-135`): draw the fuel bars of
/// `layout`, which [`super::sis_modules::fuel_bar_layout`] computed from the
/// slots and the fuel on board.
pub fn redistribute_fuel<G: Gfx + ?Sized>(g: &G, layout: &[FuelBar]) {
    let previous = g.set_context(ContextId::Space);
    g.batch();
    for bar in layout {
        let mut rect = Rect {
            corner: Point {
                x: bar.x as i16,
                y: bar.y as i16,
            },
            width: 0,
            height: 1,
        };
        if bar.filled {
            rect.width = 3;
            g.draw_point(rect.corner);
            rect.corner.x += rect.width + 1;
            g.draw_point(rect.corner);
            rect.corner.x -= rect.width;
            // SetContextForeGroundColor (SetContextBackGroundColor (BLACK)):
            // the old background becomes the foreground.
            let background = g.set_bg_color(Rgba::BLACK);
            g.set_fg_color(background);
        } else {
            rect.width = 5;
            g.set_fg_color(EMPTY_FUEL_BAR);
        }
        g.fill_rect(rect);
    }
    g.unbatch();
    g.restore_context(previous);
}

/// `RedistributeFuel` for the fuel on board and the module slots of `sis`.
pub fn redistribute_fuel_from_sis<G: Gfx + ?Sized>(
    g: &G,
    sis: &SisView,
) -> Result<(), StarbaseError> {
    let slots = ModuleSlots::from_raw(&sis.module_slots)?;
    let layout = fuel_bar_layout(&slots, sis.fuel_on_board)?;
    redistribute_fuel(g, &layout);
    Ok(())
}

/// `DisplayLanders` (`outfit.c:143-169`).
pub fn display_landers<G: Gfx + ?Sized>(
    g: &G,
    sis: &SisView,
    chmmr_bomb_state: u32,
    module_frame: FrameId,
) {
    if chmmr_bomb_state == 3 {
        g.draw_stamp(Stamp {
            origin: Point { x: 0, y: 0 },
            frame: g.dec_frame(module_frame),
        });
        return;
    }

    let mut origin = Point {
        x: LANDER_X,
        y: LANDER_Y,
    };
    let landers = sis.num_landers;
    for _ in 0..landers {
        g.draw_stamp(Stamp {
            origin,
            frame: module_frame,
        });
        origin.x += LANDER_WIDTH;
    }

    g.set_fg_color(Rgba::BLACK);
    for _ in landers..MAX_LANDERS {
        g.draw_filled_stamp(Stamp {
            origin,
            frame: module_frame,
        });
        origin.x += LANDER_WIDTH;
    }
}

/// `DrawPCMenuFrame` (`menu.c:44-66`).
fn draw_pc_menu_frame<G: Gfx + ?Sized>(g: &G, rect: &mut Rect) {
    g.set_fg_color(PCMENU_TOP_LEFT_BORDER);
    g.draw_rect(*rect);

    rect.corner.x += 1;
    rect.corner.y += 1;
    rect.height -= 1;
    rect.width -= 1;
    g.set_fg_color(PCMENU_BOTTOM_RIGHT_BORDER);
    g.draw_rect(*rect);

    rect.height -= 1;
    rect.width -= 1;
    g.set_fg_color(PCMENU_BACKGROUND);
    g.fill_rect(*rect);
}

/// `DrawPCMenu` (`menu.c:73-129`) for the Outfit entries.
fn draw_pc_outfit_menu<G: Gfx + ?Sized>(g: &G, new_state: u8, fuel_cost: u8, rect: &mut Rect) {
    const PC_MENU_HEIGHT: i32 = 8;
    let begin = PM_FUEL;
    let end = PM_EXIT_OUTFIT;
    let selected = begin + new_state;
    let items = i32::from(1 + end - begin);

    rect.corner.x -= 1;
    rect.width += 1;
    g.fill_rect(*rect);
    if items * PC_MENU_HEIGHT <= i32::from(rect.height) {
        rect.corner.y += ((i32::from(rect.height) - items * PC_MENU_HEIGHT) / 2) as i16;
    }
    rect.height = (items * PC_MENU_HEIGHT + 4) as i16;
    draw_pc_menu_frame(g, rect);

    let previous_font = g.set_font(FontId::StarCon);
    let mut baseline = Point {
        x: rect.corner.x + 2,
        y: rect.corner.y + PC_MENU_HEIGHT as i16 - 1,
    };
    rect.corner.x += 1;
    rect.width -= 2;
    for index in begin..=end {
        let label = if index == PM_FUEL {
            format!(
                "{}({})",
                g.game_string(MAINMENU_STRING_BASE + i32::from(PM_FUEL)),
                fuel_cost
            )
        } else {
            g.game_string(MAINMENU_STRING_BASE + i32::from(index))
        };
        let text = TextDraw {
            baseline,
            align: Align::Left,
            text: label,
        };
        if index == selected {
            g.set_fg_color(PCMENU_SELECTION_BACKGROUND);
            rect.corner.y = baseline.y - PC_MENU_HEIGHT as i16 + 2;
            rect.height = PC_MENU_HEIGHT as i16 - 1;
            g.fill_rect(*rect);
            g.set_fg_color(PCMENU_SELECTION_TEXT);
        } else {
            g.set_fg_color(PCMENU_TEXT);
        }
        g.draw_text(&text);
        baseline.y += PC_MENU_HEIGHT as i16;
    }
    g.restore_font(previous_font);
}

/// `DrawMenuStateStrings (PM_FUEL, new_state)` (`menu.c:469-598`). The
/// alternate-menu conversion of the PC hierarchy does not apply to the
/// Outfit entries.
pub fn draw_outfit_menu_strings<G: Gfx + ?Sized>(
    g: &G,
    style: UiStyle,
    new_state: u8,
    fuel_cost: u8,
) {
    let last_entry = PM_EXIT_OUTFIT - PM_FUEL;
    let frame = (new_state <= last_entry)
        .then(|| g.abs_frame(g.play_frame(), u16::from(PM_FUEL + new_state)));

    g.pre_update_flash();
    let previous = g.set_context(ContextId::Status);
    let clip = g.context_clip_rect();
    let (screen_width, screen_height) = g.screen_size();
    let radar = RadarLayout::for_screen(screen_width, screen_height);
    let origin = Point {
        x: (radar.x - i32::from(clip.corner.x)) as i16,
        y: (radar.y - i32::from(clip.corner.y)) as i16,
    };
    let mut rect = Rect {
        corner: Point {
            x: origin.x - 1,
            y: origin.y - 11,
        },
        width: (RADAR_WIDTH + 2) as i16,
        height: 0,
    };
    g.batch();
    g.set_fg_color(MENU_BACKDROP);

    let mut stamp_frame = frame;
    if frame.is_some() && style == UiStyle::Pc {
        rect.height = (RADAR_HEIGHT + 11) as i16;
        draw_pc_outfit_menu(g, new_state, fuel_cost, &mut rect);
        stamp_frame = None;
    } else {
        if style == UiStyle::Pc {
            rect.corner.x -= 1;
            rect.width += 1;
            rect.height = (RADAR_HEIGHT + 11) as i16;
        } else {
            rect.height = 11;
        }
        g.fill_rect(rect);
    }

    if let Some(frame) = stamp_frame {
        g.draw_stamp(Stamp { origin, frame });
        if new_state == 0 {
            g.set_font(FontId::Tiny);
            g.set_fg_color(THREEDO_MENU_TEXT);
            g.draw_text(&text_at(
                i32::from(origin.x) + RADAR_WIDTH - 2,
                i32::from(origin.y) + RADAR_HEIGHT - 2,
                Align::Right,
                fuel_cost.to_string(),
            ));
        }
    }
    g.unbatch();
    g.restore_context(previous);
    g.post_update_flash();
}

/// What `DrawModuleStrings` shows for a module: its picture and price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleOnShow {
    pub module: ModuleType,
    /// `ModuleCost[module] * MODULE_COST_SCALE`.
    pub price: u32,
}

/// `DrawModuleStrings` (`outfit.c:49-98`): the picture of the module and its
/// price in the radar area, or a blank radar area for `None` (an
/// `EMPTY_SLOT` byte). `menu_frame` is the module menu art
/// (`MENU_STATE.CurFrame`).
pub fn draw_module_strings<G: Gfx + ?Sized>(
    g: &G,
    menu_frame: FrameId,
    shown: Option<ModuleOnShow>,
) {
    let previous = g.set_context(ContextId::Status);
    let clip = g.context_clip_rect();
    let (screen_width, screen_height) = g.screen_size();
    let radar = RadarLayout::for_screen(screen_width, screen_height);
    let origin = Point {
        x: (radar.x - i32::from(clip.corner.x)) as i16,
        y: (radar.y - i32::from(clip.corner.y)) as i16,
    };
    g.batch();
    g.set_fg_color(MENU_BACKDROP);
    g.fill_rect(Rect {
        corner: Point {
            x: origin.x - 1,
            y: origin.y - 11,
        },
        width: (RADAR_WIDTH + 2) as i16,
        height: 11,
    });
    match shown {
        None => {
            g.set_fg_color(Rgba::BLACK);
            g.fill_rect(Rect {
                corner: origin,
                width: RADAR_WIDTH as i16,
                height: RADAR_HEIGHT as i16,
            });
        }
        Some(view) => {
            g.draw_stamp(Stamp {
                origin,
                frame: g.abs_frame(menu_frame, u16::from(view.module as u8)),
            });
            g.set_font(FontId::Tiny);
            g.set_fg_color(THREEDO_MENU_TEXT);
            g.draw_text(&text_at(
                i32::from(origin.x) + RADAR_WIDTH - 2,
                i32::from(origin.y) + RADAR_HEIGHT - 2,
                Align::Right,
                view.price.to_string(),
            ));
        }
    }
    g.unbatch();
    g.restore_context(previous);
}

/// `ClearSISRect (CLEAR_SIS_RADAR)` (`sis.c:82-120`): the menu strip with no
/// menu entry drawn.
pub fn clear_sis_radar<G: Gfx + ?Sized>(g: &G, style: UiStyle, fuel_cost: u8) {
    let previous = g.set_context(ContextId::Status);
    let old_color = g.set_fg_color(MENU_BACKDROP);
    g.batch();
    // `DrawMenuStateStrings ((BYTE)~0, 1)`: no entry has that number, so no
    // menu art is chosen and only the backdrop is filled.
    draw_outfit_menu_strings(g, style, NO_MENU_ENTRY, fuel_cost);
    g.unbatch();
    g.set_fg_color(old_color);
    g.restore_context(previous);
}

/// A menu entry number past every Outfit entry.
const NO_MENU_ENTRY: u8 = u8::MAX;

/// `GetGaugeRect` (`sis.c:830-837`); the rectangle is relative to the
/// status context.
#[must_use]
pub const fn gauge_rect(is_crew_rect: bool) -> Rect {
    let width = 24;
    Rect {
        corner: Point {
            x: (super::ids::STATUS_WIDTH as i16 >> 1) - (width >> 1),
            y: if is_crew_rect { 117 } else { 38 },
        },
        width,
        height: 5,
    }
}

/// The flash target for the fuel gauge, drawn from the status context.
pub fn flash_fuel_gauge<G: Gfx + ?Sized>(g: &G) {
    let previous = g.set_context(ContextId::Status);
    g.set_flash(FlashTarget::Rect(gauge_rect(false)));
    g.restore_context(previous);
}

/// `DrawFlagshipName (FALSE)` (`sis.c:480-535`).
pub fn draw_flagship_name<G: Gfx + ?Sized>(g: &G, fonts: UiStyle, sis: &SisView) {
    let previous_context = g.set_context(ContextId::Space);
    let previous_font = g.set_font(FontId::Micro);
    let (screen_width, screen_height) = g.screen_size();
    let layout = RadarLayout::for_screen(screen_width, screen_height);

    let rect = Rect {
        corner: Point { x: 0, y: 1 },
        width: layout.sis_screen_width as i16,
        height: SHIP_NAME_HEIGHT,
    };
    let name = format!(
        "{} {}",
        g.game_string(NAMING_STRING_BASE + 1),
        sis.ship_name
    )
    .to_ascii_uppercase();

    let previous_effect = g.set_font_effect(None);
    let previous_color = g.set_fg_color(Rgba::BLACK);
    g.fill_rect(rect);

    let text = text_at(
        i32::from(rect.corner.x) + i32::from(rect.width >> 1),
        i32::from(rect.corner.y) + i32::from(SHIP_NAME_HEIGHT),
        Align::Center,
        name,
    );
    if fonts == UiStyle::Pc {
        g.set_font_effect(Some(3));
    } else {
        g.set_fg_color(THREEDO_FLAGSHIP_NAME_TEXT);
    }
    g.draw_text(&text);

    g.restore_font_effect(previous_effect);
    g.set_fg_color(previous_color);
    g.restore_font(previous_font);
    g.restore_context(previous_context);
}

const GUN_WEAPON: u8 = 9;
const BLASTER_WEAPON: u8 = 10;
const CANNON_WEAPON: u8 = 11;
const FIRST_BOMB_MODULE: u8 = 14;
const LAST_BOMB_MODULE: u8 = 19;
const FUEL_TANK: u8 = 5;
const HIGHEFF_FUELSYS: u8 = 6;
const DYNAMO_UNIT: u8 = 7;
const SHIVA_FURNACE: u8 = 8;

/// `describeWeapon` (`sis.c:702-720`).
fn describe_weapon<G: Gfx + ?Sized>(g: &G, module: u8) -> String {
    let offset = match module {
        GUN_WEAPON => 8,
        BLASTER_WEAPON => 9,
        CANNON_WEAPON => 10,
        FIRST_BOMB_MODULE..=LAST_BOMB_MODULE => 11,
        _ => 12,
    };
    g.game_string(FLAGSHIP_STRING_BASE + offset)
}

/// `DrawFlagshipStats` (`sis.c:541-692`).
pub fn draw_flagship_stats<G: Gfx + ?Sized>(g: &G, sis: &SisView) {
    const ENERGY_REGENERATION: u32 = 1;
    const ENERGY_WAIT: i32 = 10;
    const MAX_THRUST: u32 = 10;
    const TURN_WAIT: u32 = 17;

    let mut energy_regeneration = ENERGY_REGENERATION;
    let mut energy_wait = ENERGY_WAIT;
    let mut max_thrust = MAX_THRUST;
    let mut turn_wait = TURN_WAIT;
    let mut fuel = 10 * FUEL_TANK_SCALE;

    for module in sis.module_slots {
        match module {
            FUEL_TANK => fuel += FUEL_TANK_CAPACITY,
            HIGHEFF_FUELSYS => fuel += HEFUEL_TANK_CAPACITY,
            DYNAMO_UNIT => energy_wait = (energy_wait - 2).max(4),
            SHIVA_FURNACE => energy_regeneration += 1,
            _ => {}
        }
    }
    max_thrust += 2 * sis
        .drive_slots
        .iter()
        .filter(|slot| u16::from(**slot) == FUSION_THRUSTER)
        .count() as u32;
    turn_wait -= 2 * sis
        .jet_slots
        .iter()
        .filter(|slot| u16::from(**slot) == TURNING_JETS)
        .count() as u32;

    let previous_context = g.set_context(ContextId::Space);
    let previous_font = g.set_font(FontId::StarCon);
    let previous_effect = g.set_font_effect(None);
    let mut leading = i32::from(g.font_leading());
    let (screen_width, screen_height) = g.screen_size();
    let layout = RadarLayout::for_screen(screen_width, screen_height);

    let rect = Rect {
        corner: Point {
            x: 0,
            y: (layout.sis_screen_height - 4 * leading) as i16,
        },
        width: layout.sis_screen_width as i16,
        height: (4 * leading) as i16,
    };
    let previous_color = g.set_fg_color(Rgba::BLACK);
    g.fill_rect(rect);

    leading -= 3;
    let top = i32::from(rect.corner.y) + leading + 3;
    let labels_x = layout.sis_screen_width / 6;

    g.set_font_effect(Some(4));
    for (row, offset) in (0..4).zip(0..4) {
        g.draw_text(&text_at(
            labels_x,
            top + leading * row,
            Align::Right,
            g.game_string(FLAGSHIP_STRING_BASE + offset),
        ));
    }
    for (row, slot) in (0..).zip([15usize, 14, 13, 0]) {
        let weapon = describe_weapon(g, sis.module_slots[slot]);
        g.draw_text(&text_at(
            labels_x + 5,
            top + leading * row,
            Align::Left,
            format!("{weapon:<7.7}"),
        ));
    }

    g.set_font_effect(Some(5));
    let value_label_x = i32::from(rect.width) - 25;
    for (row, offset) in (0..4).zip(4..8) {
        g.draw_text(&text_at(
            value_label_x,
            top + leading * row,
            Align::Right,
            g.game_string(FLAGSHIP_STRING_BASE + offset),
        ));
    }

    let energy_per_10_sec = ((100 * ONE_SECOND * energy_regeneration)
        / ((1 + energy_wait as u32) * BATTLE_FRAME_RATE)
        + 5)
        / 10;
    let values = [
        format!("{:>4}", max_thrust * 4),
        format!("{:>4}", 1 + TURN_WAIT - turn_wait),
        format!("{:>2}.{}", energy_per_10_sec / 10, energy_per_10_sec % 10),
        format!("{:>4}", fuel / FUEL_TANK_SCALE),
    ];
    for (row, value) in (0..).zip(values) {
        g.draw_text(&text_at(
            i32::from(rect.width) - 2,
            top + leading * row,
            Align::Right,
            value,
        ));
    }

    g.restore_font_effect(previous_effect);
    g.set_fg_color(previous_color);
    g.restore_font(previous_font);
    g.restore_context(previous_context);
}
