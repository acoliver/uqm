//! The Shipyard screen as input-loop frame functions: `DoShipyard` and
//! `DoModifyShips` (`shipyard.c`).
//!
//! [`ShipyardScreen::frame`] is one call of the C `InputFunc`. The rules live
//! in [`super::shipyard`]; this module carries them out through
//! [`Platform`] and [`Gfx`] in the order the C code does. The art, colour map
//! and music the screen loads are owned by a guard, so every exit releases
//! them.
//!
//! Not ported: the ship spin presentation that the SPECIAL key starts
//! (`DoShipSpin` plays a slide-show video through `ShowPresentation`), and
//! `GameOptions` (Save/Load). Both end the screen with a typed
//! [`StarbaseError::Unported`]. The mineral and fuel fill indicators of the
//! blueprint (`DrawBluePrint`) are not drawn.

use super::draw::{clear_sis_radar, draw_ship_piece, draw_shipyard_menu_strings, gauge_rect};
use super::fuel::GaugeDelta;
use super::gfx::{
    Align, ContextId, FlashTarget, FontId, FrameId, Point, Rect, Rgba, Stamp, TextDraw,
};
use super::ids::{
    RadarLayout, HANGAR_COLOR_TAB, RADAR_HEIGHT, RADAR_WIDTH, SHIPYARD_MUSIC, SHIPYARD_PMAP_ANIM,
    SISBLU_MASK_ANIM, SIS_ORG_X, SIS_ORG_Y, STARBASE_STRING_BASE,
};
use super::input::MenuPulses;
use super::platform::{
    AlliedState, ColorMapId, Escort, MenuSounds, MusicId, Platform, RaceTemplate, ResUnitsUpdate,
    SisView, StarbaseError, StateKey, UnportedRoute, ONE_SECOND,
};
use super::shipyard::{
    available_races, crew_dot, crew_transaction, dismiss_refund, escort_dismiss, escort_hire,
    flagship_hire, ship_cost, slot_corner, Cursor, EscortDismiss, HANGAR_SHIPS_ROW,
    SHIP_WIN_FRAMES, SHIP_WIN_HEIGHT, SHIP_WIN_WIDTH,
};
use super::sis_modules::EMPTY_MODULE_SLOT;
use crate::mainloop::types::activity_flags;

/// `HANGAR_ANIM_RATE`: power line frames per second.
const HANGAR_ANIM_RATE: u32 = 15;
/// `SHOFIXTI_SHIP`: its `avail_race_q` position.
const SHOFIXTI_SHIP: usize = 5;
/// `SHIPYARD_CREW`, `SHIPYARD_SAVELOAD` and `SHIPYARD_EXIT`.
const MENU_CREW: u8 = 0;
const MENU_SAVELOAD: u8 = 1;
const MENU_EXIT: u8 = 2;

/// The art, colour map and music the Shipyard holds while it shows.
struct ShipyardAssets<'a, P: Platform + ?Sized> {
    platform: &'a P,
    module_frame: Option<FrameId>,
    color_map: Option<ColorMapId>,
    music: Option<MusicId>,
}

impl<P: Platform + ?Sized> Drop for ShipyardAssets<'_, P> {
    fn drop(&mut self) {
        if let Some(frame) = self.module_frame.take() {
            self.platform.destroy_drawable(frame);
        }
        if let Some(map) = self.color_map.take() {
            self.platform.destroy_color_map(map);
        }
        self.platform.stop_music();
        if let Some(music) = self.music.take() {
            self.platform.destroy_music(music);
        }
    }
}

/// The static state of `animatePowerLines`.
struct PowerLines {
    frame: FrameId,
    color_map: ColorMapId,
    next_time: u32,
}

/// `DMS_Mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditMode {
    Navigate,
    AddEscort,
    EditCrew,
    Exit,
}

/// `DoModifyShips` state: `CurState`, and `delta_item` split into its
/// `MODIFY_CREW_FLAG` and the highlighted ship.
#[derive(Debug, Clone, Copy)]
struct Hangar {
    cursor: Cursor,
    editing: bool,
    pick: u8,
}

enum Mode {
    Menu,
    Hangar(Hangar),
}

/// One ship window of `ShowCombatShip`.
struct ShipWindow {
    escort: Escort,
    left_door: i16,
    right_door: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoorMotion {
    Open,
    Close,
}

/// `DoShipyard`.
pub struct ShipyardScreen<'a, P: Platform + ?Sized> {
    platform: &'a P,
    assets: Option<ShipyardAssets<'a, P>>,
    power_lines: Option<PowerLines>,
    menu_state: u8,
    mode: Mode,
    /// `SetInputCallback (on_input_frame)` is set.
    input_callback: bool,
}

impl<'a, P: Platform + ?Sized> ShipyardScreen<'a, P> {
    #[must_use]
    pub fn new(platform: &'a P) -> Self {
        Self {
            platform,
            assets: None,
            power_lines: None,
            menu_state: MENU_CREW,
            mode: Mode::Menu,
            input_callback: false,
        }
    }

    /// One `DoShipyard` or `DoModifyShips` call. `Ok(true)` keeps the loop
    /// running.
    pub fn frame(&mut self) -> Result<bool, StarbaseError> {
        let p = self.platform;
        if self.input_callback {
            let previous = p.set_context(ContextId::Space);
            self.animate_power_lines(false)?;
            p.restore_context(previous);
        }
        if p.current_activity().0 & activity_flags::CHECK_ABORT != 0 {
            return Ok(match self.mode {
                // `pMS->InputFunc = DoShipyard`; the next frame exits.
                Mode::Hangar(_) => {
                    self.mode = Mode::Menu;
                    true
                }
                Mode::Menu => {
                    self.leave();
                    false
                }
            });
        }
        match self.mode {
            Mode::Menu => self.menu_frame(),
            Mode::Hangar(hangar) => self.hangar_frame(hangar),
        }
    }

    fn leave(&mut self) {
        self.input_callback = false;
        self.power_lines = None;
        self.assets = None;
    }

    fn module_frame(&self) -> FrameId {
        self.assets
            .as_ref()
            .and_then(|assets| assets.module_frame)
            .expect("the Shipyard art is loaded")
    }

    // --- DoShipyard ---------------------------------------------------

    fn menu_frame(&mut self) -> Result<bool, StarbaseError> {
        let p = self.platform;
        let pulses = p.pulses();
        p.set_menu_sounds(MenuSounds::ARROWS_SELECT);
        if self.assets.is_none() {
            self.initialize()?;
        } else if pulses.cancel || (pulses.select && self.menu_state == MENU_EXIT) {
            self.leave();
            return Ok(false);
        } else if pulses.select {
            if self.menu_state == MENU_SAVELOAD {
                return Err(StarbaseError::Unported(UnportedRoute::SaveLoad));
            }
            self.open_hangar()?;
        } else {
            self.choose_menu_entry(pulses);
        }
        Ok(true)
    }

    /// `DoMenuChooser (pMS, PM_CREW)`.
    fn choose_menu_entry(&mut self, pulses: MenuPulses) {
        let p = self.platform;
        let new_state = if pulses.previous() {
            (self.menu_state + MENU_EXIT) % (MENU_EXIT + 1)
        } else if pulses.next() {
            (self.menu_state + 1) % (MENU_EXIT + 1)
        } else {
            return;
        };
        self.draw_menu_strings(new_state);
        self.menu_state = new_state;
        p.sleep_thread(ONE_SECOND / 20);
    }

    /// `DrawMenuStateStrings (PM_CREW, state)`.
    fn draw_menu_strings(&self, state: u8) {
        let p = self.platform;
        draw_shipyard_menu_strings(p, p.menu_style(), state, p.crew_cost());
    }

    fn initialize(&mut self) -> Result<(), StarbaseError> {
        let p = self.platform;
        let mut assets = ShipyardAssets {
            platform: p,
            module_frame: None,
            color_map: None,
            music: None,
        };
        let module_frame = p.load_graphic(SHIPYARD_PMAP_ANIM)?;
        assets.module_frame = Some(module_frame);
        assets.color_map = Some(p.load_color_map(HANGAR_COLOR_TAB)?);
        let music = p.load_music(SHIPYARD_MUSIC)?;
        assets.music = Some(music);
        let color_map = assets.color_map.expect("the colour map was just loaded");

        p.set_transition_source_none();
        p.batch();
        p.draw_sis_frame();
        p.draw_sis_message(&p.game_string(STARBASE_STRING_BASE + 3));
        p.draw_sis_title(&p.game_string(STARBASE_STRING_BASE));
        p.set_context(ContextId::Space);
        self.draw_blueprint()?;

        self.menu_state = MENU_CREW;
        self.draw_menu_strings(MENU_CREW);

        p.set_context(ContextId::Space);
        // The PC ship dock overwrites the border: widen the clip by a pixel.
        let clip = p.context_clip_rect();
        p.set_context_clip_rect(Rect {
            corner: Point {
                x: clip.corner.x - 1,
                y: clip.corner.y,
            },
            width: clip.width + 2,
            height: clip.height + 1,
        });
        p.draw_stamp(Stamp {
            origin: Point { x: 0, y: 0 },
            frame: p.abs_frame(module_frame, 0),
        });
        p.set_context_clip_rect(clip);
        self.power_lines = Some(PowerLines {
            frame: p.abs_frame(module_frame, 24),
            color_map: p.abs_color_map(color_map, 0),
            next_time: 0,
        });
        self.assets = Some(assets);
        self.animate_power_lines(true)?;
        p.set_font(FontId::Tiny);

        p.screen_transition(3);
        p.unbatch();
        p.play_music(music);

        let escorts = p.escorts()?;
        self.show_combat_ships(escorts, DoorMotion::Open)?;
        self.input_callback = true;
        p.set_flash(FlashTarget::Menu3do);
        Ok(())
    }

    /// `animatePowerLines`: `start` is the call with a `MENU_STATE`.
    fn animate_power_lines(&mut self, start: bool) -> Result<(), StarbaseError> {
        let p = self.platform;
        let now = p.now();
        let lines = self
            .power_lines
            .as_mut()
            .expect("the power lines are set up");
        if now >= lines.next_time || start {
            lines.next_time = now + ONE_SECOND / HANGAR_ANIM_RATE;
            p.apply_color_map(lines.color_map);
            p.draw_stamp(Stamp {
                origin: Point { x: 0, y: 0 },
                frame: lines.frame,
            });
            lines.color_map = p.rel_color_map(lines.color_map, 1);
        }
        Ok(())
    }

    /// `DrawBluePrint`, without the mineral and fuel fill indicators.
    fn draw_blueprint(&self) -> Result<(), StarbaseError> {
        let p = self.platform;
        let sis = p.sis()?;
        let fonts = p.font_style();
        // Every crew member needs a seat before anything is drawn.
        let mut dots = Vec::with_capacity(usize::from(sis.crew_enlisted));
        for crew in 0..sis.crew_enlisted {
            dots.push(crew_dot(&sis.module_slots, crew, fonts).ok_or(
                StarbaseError::CrewExceedsPods {
                    crew: sis.crew_enlisted,
                },
            )?);
        }

        let frame = p.load_graphic(SISBLU_MASK_ANIM)?;
        p.set_fg_color(Rgba::rgb15(0x00, 0x00, 0x16));
        p.draw_filled_stamp(Stamp {
            origin: Point { x: 0, y: 0 },
            frame: p.dec_frame(frame),
        });
        for (slot, piece) in sis.drive_slots.iter().enumerate() {
            draw_ship_piece(p, &sis, frame, u16::from(*piece), slot as u16, true);
        }
        for (slot, piece) in sis.jet_slots.iter().enumerate() {
            draw_ship_piece(p, &sis, frame, u16::from(*piece), slot as u16, true);
        }
        let crew_pod = super::sis_modules::ModuleType::CrewPod as u8;
        for (slot, piece) in sis.module_slots.iter().enumerate() {
            if *piece != crew_pod {
                draw_ship_piece(p, &sis, frame, u16::from(*piece), slot as u16, true);
            }
        }
        p.set_fg_color(Rgba::rgb15(0x0A, 0x0A, 0x1F));
        for (slot, piece) in sis.module_slots.iter().enumerate() {
            if *piece == crew_pod {
                draw_ship_piece(p, &sis, frame, u16::from(*piece), slot as u16, true);
            }
        }
        for (point, color) in dots {
            p.set_fg_color(color);
            p.draw_point(point);
        }
        p.destroy_drawable(frame);
        Ok(())
    }

    // --- ShowCombatShip -------------------------------------------------

    /// `ShowCrew`-less part of `ShowCombatShip`: slide the doors of the ship
    /// windows open (a ship shown) or closed (a ship sold).
    fn show_combat_ships(
        &mut self,
        escorts: Vec<Escort>,
        motion: DoorMotion,
    ) -> Result<(), StarbaseError> {
        if escorts.is_empty() {
            return Ok(());
        }
        let p = self.platform;
        let module_frame = self.module_frame();
        let left_frame = p.inc_frame(module_frame);
        let right_frame = p.inc_frame(left_frame);
        let reach = match motion {
            DoorMotion::Open => 1,
            DoorMotion::Close => SHIP_WIN_WIDTH >> 1,
        };
        let mut windows: Vec<ShipWindow> = escorts
            .into_iter()
            .map(|escort| ShipWindow {
                escort,
                left_door: -reach,
                right_door: reach,
            })
            .collect();

        let mut all_doors_finished = false;
        p.flush_input();
        let mut time_in = p.now();
        for _ in 0..SHIP_WIN_FRAMES {
            if all_doors_finished {
                break;
            }
            p.sleep_thread_until(time_in + ONE_SECOND / 24);
            time_in = p.now();
            if p.any_button_press() {
                if motion == DoorMotion::Close {
                    windows[0].left_door = 0;
                    windows[0].right_door = 0;
                }
                all_doors_finished = true;
            }

            let previous = p.set_context(ContextId::Space);
            let old_clip = p.context_clip_rect();
            p.set_bg_color(Rgba::BLACK);
            p.batch();
            for window in &mut windows {
                let corner = slot_corner(window.escort.slot);
                p.set_context_clip_rect(Rect {
                    corner: Point {
                        x: SIS_ORG_X + corner.x,
                        y: SIS_ORG_Y as i16 + corner.y,
                    },
                    width: SHIP_WIN_WIDTH,
                    height: SHIP_WIN_HEIGHT,
                });
                p.clear_drawable();
                p.draw_stamp(Stamp {
                    origin: Point {
                        x: (SHIP_WIN_WIDTH >> 1) + 1,
                        y: SHIP_WIN_WIDTH >> 1,
                    },
                    frame: window.escort.melee_icon,
                });
                self.show_ship_crew(
                    &window.escort,
                    Rect {
                        corner: Point { x: 0, y: 0 },
                        width: SHIP_WIN_WIDTH,
                        height: SHIP_WIN_HEIGHT,
                    },
                )?;
                if !all_doors_finished || motion == DoorMotion::Close {
                    p.draw_stamp(Stamp {
                        origin: Point {
                            x: window.left_door,
                            y: 0,
                        },
                        frame: left_frame,
                    });
                    p.draw_stamp(Stamp {
                        origin: Point {
                            x: window.right_door,
                            y: 0,
                        },
                        frame: right_frame,
                    });
                    let step = if motion == DoorMotion::Close { 1 } else { -1 };
                    window.left_door += step;
                    window.right_door -= step;
                }
            }
            p.set_context_clip_rect(old_clip);
            self.animate_power_lines(false)?;
            p.unbatch();
            p.restore_context(previous);
        }
        Ok(())
    }

    /// `ShowShipCrew`: the crew count on top of the ship image.
    fn show_ship_crew(&self, escort: &Escort, rect: Rect) -> Result<(), StarbaseError> {
        let p = self.platform;
        let max_crew = self.template(escort.race_id)?.crew_level;
        let text = if escort.crew_level >= max_crew {
            escort.crew_level.to_string()
        } else if escort.crew_level == 0 {
            "SCRAP".to_owned()
        } else {
            format!("{}/{}", escort.crew_level, max_crew)
        };
        let baseline = Point {
            x: rect.corner.x + (rect.width >> 1),
            y: rect.corner.y + rect.height - 1,
        };
        if rect.corner.y != 0 {
            p.set_fg_color(Rgba::BLACK);
            p.fill_rect(Rect {
                corner: Point {
                    x: rect.corner.x,
                    y: baseline.y - 6,
                },
                width: SHIP_WIN_WIDTH,
                height: 6,
            });
        }
        p.set_fg_color(if escort.crew_level != 0 {
            Rgba::rgb15(0x00, 0x14, 0x00)
        } else {
            Rgba::rgb15(0x12, 0x00, 0x00)
        });
        p.draw_text(&TextDraw {
            baseline,
            align: Align::Center,
            text,
        });
        Ok(())
    }

    fn template(&self, race_id: u8) -> Result<RaceTemplate, StarbaseError> {
        self.platform
            .race_templates()?
            .get(usize::from(race_id))
            .copied()
            .ok_or(StarbaseError::UnknownRace(race_id))
    }

    // --- DoModifyShips ---------------------------------------------------

    /// `DoShipyard` selecting Crew: `DoModifyShips` sets itself up.
    fn open_hangar(&mut self) -> Result<(), StarbaseError> {
        let p = self.platform;
        let hangar = Hangar {
            cursor: Cursor::START,
            editing: false,
            pick: 0,
        };
        p.set_context(ContextId::Space);
        self.mode = Mode::Hangar(hangar);
        self.set_edit_mode(&hangar, EditMode::Navigate);
        p.sleep_thread(ONE_SECOND / 30);
        Ok(())
    }

    fn hangar_frame(&mut self, mut hangar: Hangar) -> Result<bool, StarbaseError> {
        let p = self.platform;
        let pulses = p.pulses();
        let select = pulses.select;
        let cancel = pulses.cancel;
        let mut dx = 0i8;
        let mut dy = 0i8;
        if pulses.right {
            dx = 1;
        }
        if pulses.left {
            dx = -1;
        }
        if pulses.up {
            dy = -1;
        }
        if pulses.down {
            dy = 1;
        }

        if !hangar.editing {
            self.navigate(&mut hangar, pulses.special, select, cancel, dx, dy)?;
        } else {
            let escort = self.escort_at(&hangar.cursor)?;
            if escort.is_none() && !hangar.cursor.is_flagship() {
                self.add_escort_frame(&mut hangar, pulses.special, select, cancel, dx, dy)?;
            } else {
                self.edit_crew_frame(&mut hangar, escort, select, cancel, dy)?;
            }
        }
        // `DMS_NavigateShipSlots` may have handed control back to the menu.
        if let Mode::Hangar(_) = self.mode {
            self.mode = Mode::Hangar(hangar);
        }
        p.sleep_thread(ONE_SECOND / 30);
        Ok(true)
    }

    /// `GetEscortByStarShipIndex (CurState)`: the escort in the cursor's
    /// slot; the flagship has none.
    fn escort_at(&self, cursor: &Cursor) -> Result<Option<Escort>, StarbaseError> {
        if cursor.is_flagship() {
            return Ok(None);
        }
        Ok(self
            .platform
            .escorts()?
            .into_iter()
            .find(|escort| escort.slot == cursor.slot()))
    }

    /// `DMS_NavigateShipSlots`.
    fn navigate(
        &mut self,
        hangar: &mut Hangar,
        special: bool,
        select: bool,
        cancel: bool,
        dx: i8,
        dy: i8,
    ) -> Result<(), StarbaseError> {
        let p = self.platform;
        let escort = self.escort_at(&hangar.cursor)?;
        if dx != 0 || dy != 0 {
            let moved = hangar.cursor.moved(dx, dy);
            if moved != hangar.cursor {
                hangar.cursor = moved;
                self.flash_active_ship(hangar);
            }
        }

        if special {
            // `DMS_SpinShip` spins an escort under the cursor.
            if escort.is_some() && !hangar.cursor.is_flagship() {
                return Err(StarbaseError::Unported(UnportedRoute::ShipSpin));
            }
        } else if select {
            if escort.is_none() && !hangar.cursor.is_flagship() {
                if available_races(&p.race_templates()?).is_empty() {
                    return Err(StarbaseError::NoShipsAvailable);
                }
                hangar.editing = true;
                hangar.pick = 0;
                self.draw_race_strings(Some(0))?;
                self.set_edit_mode(hangar, EditMode::AddEscort);
            } else {
                hangar.editing = true;
                self.set_edit_mode(hangar, EditMode::EditCrew);
            }
        } else if cancel {
            self.mode = Mode::Menu;
            self.menu_state = MENU_CREW;
            self.draw_menu_strings(MENU_CREW);
            self.set_edit_mode(hangar, EditMode::Exit);
        }
        Ok(())
    }

    /// `DMS_AddEscortShip`.
    fn add_escort_frame(
        &mut self,
        hangar: &mut Hangar,
        special: bool,
        select: bool,
        cancel: bool,
        dx: i8,
        dy: i8,
    ) -> Result<(), StarbaseError> {
        let p = self.platform;
        if special {
            return Err(StarbaseError::Unported(UnportedRoute::ShipSpin));
        }
        if cancel {
            hangar.editing = false;
            p.set_flash(FlashTarget::Off);
            self.draw_menu_strings(MENU_CREW);
            self.set_edit_mode(hangar, EditMode::Navigate);
        } else if select {
            self.try_add_escort(hangar)?;
        } else if dx != 0 || dy != 0 {
            let count = available_races(&p.race_templates()?).len() as u8;
            let mut pick = hangar.pick;
            if dx < 0 || dy < 0 {
                pick = if pick == 0 { count - 1 } else { pick - 1 };
            } else if pick + 1 == count {
                pick = 0;
            } else {
                pick += 1;
            }
            if pick != hangar.pick {
                p.pre_update_flash();
                self.draw_race_strings(Some(pick))?;
                p.post_update_flash();
                hangar.pick = pick;
            }
        }
        Ok(())
    }

    /// `DMS_TryAddEscortShip`.
    fn try_add_escort(&mut self, hangar: &mut Hangar) -> Result<(), StarbaseError> {
        let p = self.platform;
        let races = available_races(&p.race_templates()?);
        let race = *races
            .get(usize::from(hangar.pick))
            .ok_or(StarbaseError::NoShipsAvailable)?;
        let cost = u32::from(ship_cost(race)?);
        let before = p.sis()?;
        let slot = hangar.cursor.slot();

        if before.res_units >= cost && p.add_escort(race, slot)? {
            let added = p
                .escorts()?
                .into_iter()
                .find(|escort| escort.slot == slot)
                .ok_or(StarbaseError::EscortSlot(slot))?;
            self.show_combat_ships(vec![added], DoorMotion::Open)?;
            self.draw_menu_strings(MENU_CREW);
            p.redraw_ship_status(ResUnitsUpdate::Change(-(cost as i32)));
            self.check_res_units(before.res_units.saturating_sub(cost))?;
            self.set_edit_mode(hangar, EditMode::EditCrew);
        } else {
            p.play_failure_sound();
        }
        Ok(())
    }

    /// `DMS_EditCrewMode`.
    fn edit_crew_frame(
        &mut self,
        hangar: &mut Hangar,
        escort: Option<Escort>,
        select: bool,
        cancel: bool,
        dy: i8,
    ) -> Result<(), StarbaseError> {
        if select || cancel {
            if let Some(escort) = escort {
                if escort.crew_level == 0 {
                    self.scrap_escort(hangar, escort)?;
                }
            }
            hangar.editing = false;
            self.set_edit_mode(hangar, EditMode::Navigate);
        } else if dy != 0 {
            self.modify_crew(hangar, escort, dy)?;
        }
        Ok(())
    }

    /// `DMS_ScrapEscortShip`.
    fn scrap_escort(&mut self, hangar: &Hangar, escort: Escort) -> Result<(), StarbaseError> {
        let p = self.platform;
        let slot = escort.slot;
        p.set_flash(FlashTarget::Off);
        self.show_combat_ships(vec![escort], DoorMotion::Close)?;
        p.remove_escort(slot)?;
        p.redraw_ship_status(ResUnitsUpdate::Redraw);
        p.set_context(ContextId::Space);
        self.set_edit_mode(hangar, EditMode::Navigate);
        Ok(())
    }

    /// `DMS_ModifyCrew`: `dy < 0` hires, `dy > 0` dismisses.
    fn modify_crew(
        &mut self,
        hangar: &mut Hangar,
        escort: Option<Escort>,
        dy: i8,
    ) -> Result<(), StarbaseError> {
        let p = self.platform;
        let crew_delta = match escort {
            None if dy < 0 => self.hire_flagship_crew()?,
            None => self.dismiss_flagship_crew()?,
            Some(escort) if dy < 0 => self.hire_escort_crew(escort)?,
            Some(escort) => self.dismiss_escort_crew(escort)?,
        };
        if crew_delta != 0 {
            match escort {
                None => self.flash_flagship_crew_count(),
                Some(escort) => self.flash_escort_crew_count(escort.slot),
            }
        } else {
            p.play_failure_sound();
        }
        if escort.is_some() {
            hangar.pick = 0;
        }
        self.crew_transaction(crew_delta)
    }

    fn crew_purchased(&self) -> i16 {
        let p = self.platform;
        let low = p.game_state(StateKey::CrewPurchased0) & 0xFF;
        let high = p.game_state(StateKey::CrewPurchased1) & 0xFF;
        (low | (high << 8)) as u16 as i16
    }

    /// `CrewTransaction`.
    fn crew_transaction(&self, crew_delta: i16) -> Result<(), StarbaseError> {
        if crew_delta == 0 {
            return Ok(());
        }
        let p = self.platform;
        let totals = crew_transaction(self.crew_purchased(), p.crew_cost(), crew_delta);
        if totals.cost_changed {
            p.set_crew_cost(totals.crew_cost);
            self.draw_menu_strings(MENU_CREW);
        }
        if self.template(SHOFIXTI_SHIP as u8)?.allied != AlliedState::GoodGuy {
            let bought = totals.purchased as u16;
            p.set_game_state(StateKey::CrewPurchased0, u32::from(bought & 0xFF));
            p.set_game_state(StateKey::CrewPurchased1, u32::from(bought >> 8));
        }
        Ok(())
    }

    fn check_res_units(&self, expected: u32) -> Result<(), StarbaseError> {
        let actual = self.platform.sis()?.res_units;
        if actual != expected {
            return Err(StarbaseError::ResUnitsMismatch { expected, actual });
        }
        Ok(())
    }

    fn check_crew_and_res_units(
        &self,
        before: &SisView,
        crew_delta: i16,
        expected_res_units: u32,
    ) -> Result<SisView, StarbaseError> {
        let after = self.platform.sis()?;
        let expected_crew = (i32::from(before.crew_enlisted) + i32::from(crew_delta)) as u16;
        if after.crew_enlisted != expected_crew || after.res_units != expected_res_units {
            return Err(StarbaseError::CrewMismatch {
                expected_crew,
                expected_res_units,
                actual_crew: after.crew_enlisted,
                actual_res_units: after.res_units,
            });
        }
        Ok(after)
    }

    /// `DMS_HireFlagShipCrew`.
    fn hire_flagship_crew(&self) -> Result<i16, StarbaseError> {
        let p = self.platform;
        let before = p.sis()?;
        let Some(cost) = flagship_hire(&before, p.crew_cost()) else {
            return Ok(0);
        };
        let (point, color) = crew_dot(&before.module_slots, before.crew_enlisted, p.font_style())
            .ok_or(StarbaseError::CrewExceedsPods {
            crew: before.crew_enlisted,
        })?;
        p.set_fg_color(color);
        p.draw_point(point);

        p.pre_update_flash();
        p.apply_crew_gauge_delta(1, -(cost as i32));
        p.post_update_flash();
        self.check_crew_and_res_units(&before, 1, before.res_units - cost)?;
        Ok(1)
    }

    /// `DMS_DismissFlagShipCrew`.
    fn dismiss_flagship_crew(&self) -> Result<i16, StarbaseError> {
        let p = self.platform;
        let before = p.sis()?;
        if before.crew_enlisted == 0 {
            return Ok(0);
        }
        let refund = dismiss_refund(p.crew_cost(), self.crew_purchased());
        p.pre_update_flash();
        p.apply_crew_gauge_delta(-1, refund as i32);
        p.post_update_flash();
        let after = self.check_crew_and_res_units(&before, -1, before.res_units + refund)?;

        let (point, _) = crew_dot(&after.module_slots, after.crew_enlisted, p.font_style()).ok_or(
            StarbaseError::CrewExceedsPods {
                crew: after.crew_enlisted,
            },
        )?;
        p.set_fg_color(Rgba::BLACK);
        p.draw_point(point);
        Ok(-1)
    }

    /// `DMS_HireEscortShipCrew`.
    fn hire_escort_crew(&self, escort: Escort) -> Result<i16, StarbaseError> {
        let p = self.platform;
        let before = p.sis()?;
        let max_crew = self.template(escort.race_id)?.crew_level;
        let Some(cost) = escort_hire(&escort, max_crew, before.res_units, p.crew_cost())? else {
            return Ok(0);
        };
        p.apply_gauge_delta(GaugeDelta {
            fuel: 0,
            res_units: -(cost as i32),
        });
        self.check_res_units(before.res_units.saturating_sub(cost))?;

        let hired = Escort {
            crew_level: escort.crew_level + 1,
            ..escort
        };
        p.set_escort_crew(hired.slot, hired.crew_level)?;
        p.pre_update_flash();
        self.show_ship_crew(&hired, escort_rect(hired.slot))?;
        p.post_update_flash();
        Ok(1)
    }

    /// `DMS_DismissEscortShipCrew`.
    fn dismiss_escort_crew(&self, escort: Escort) -> Result<i16, StarbaseError> {
        let p = self.platform;
        let before = p.sis()?;
        let mut shown = escort;
        let crew_delta = match escort_dismiss(&escort, p.crew_cost(), self.crew_purchased())? {
            EscortDismiss::NoCrew => {
                p.play_failure_sound();
                0
            }
            EscortDismiss::Crew { refund } | EscortDismiss::Scrap { refund } => {
                p.apply_gauge_delta(GaugeDelta {
                    fuel: 0,
                    res_units: refund as i32,
                });
                self.check_res_units(before.res_units + refund)?;
                shown.crew_level -= 1;
                p.set_escort_crew(shown.slot, shown.crew_level)?;
                -1
            }
        };
        p.pre_update_flash();
        self.show_ship_crew(&shown, escort_rect(shown.slot))?;
        p.post_update_flash();
        Ok(crew_delta)
    }

    // --- Race selection panel -------------------------------------------

    /// `DrawRaceStrings`: the ship picture, name and price in the radar
    /// area, or an empty panel for `None`.
    fn draw_race_strings(&self, pick: Option<u8>) -> Result<(), StarbaseError> {
        let p = self.platform;
        let sis = p.sis()?;
        let previous = p.set_context(ContextId::Status);
        let clip = p.context_clip_rect();
        let (screen_width, screen_height) = p.screen_size();
        let radar = RadarLayout::for_screen(screen_width, screen_height);
        let origin = Point {
            x: (radar.x - i32::from(clip.corner.x)) as i16,
            y: (radar.y - i32::from(clip.corner.y)) as i16,
        };
        p.batch();
        clear_sis_radar(p, p.menu_style(), sis.fuel_cost);
        p.set_fg_color(Rgba::rgb15(0x0A, 0x0A, 0x0A));
        p.fill_rect(Rect {
            corner: Point {
                x: origin.x - 1,
                y: origin.y - 11,
            },
            width: (RADAR_WIDTH + 2) as i16,
            height: 11,
        });
        p.set_fg_color(Rgba::BLACK);
        p.fill_rect(Rect {
            corner: origin,
            width: RADAR_WIDTH as i16,
            height: RADAR_HEIGHT as i16,
        });
        if let Some(pick) = pick {
            let templates = p.race_templates()?;
            let races = available_races(&templates);
            let race = *races
                .get(usize::from(pick))
                .ok_or(StarbaseError::NoShipsAvailable)?;
            // The ship name, above the ship image.
            p.draw_stamp(Stamp {
                origin,
                frame: p.abs_frame(self.module_frame(), 3 + u16::from(race)),
            });
            let baseline = Point {
                x: origin.x + RADAR_WIDTH as i16 - 2,
                y: origin.y + RADAR_HEIGHT as i16 - 2,
            };
            p.draw_stamp(Stamp {
                origin: Point {
                    x: origin.x + (RADAR_WIDTH >> 1) as i16,
                    y: origin.y + (RADAR_HEIGHT >> 1) as i16,
                },
                frame: templates[usize::from(race)].melee_icon,
            });
            p.set_font(FontId::Tiny);
            p.set_fg_color(Rgba::rgb15(0x00, 0x1F, 0x00));
            p.draw_text(&TextDraw {
                baseline,
                align: Align::Right,
                text: ship_cost(race)?.to_string(),
            });
        }
        p.unbatch();
        p.restore_context(previous);
        Ok(())
    }

    // --- Flash rectangles and modes -----------------------------------

    /// `DMS_SetMode`.
    fn set_edit_mode(&self, hangar: &Hangar, mode: EditMode) {
        let p = self.platform;
        match mode {
            EditMode::Navigate => {
                p.set_menu_sounds(MenuSounds::ARROWS_SELECT);
                self.flash_active_ship(hangar);
            }
            EditMode::AddEscort => {
                p.set_menu_sounds(MenuSounds::ARROWS_SELECT);
                p.set_flash(FlashTarget::MenuAny);
            }
            EditMode::EditCrew => {
                p.set_menu_sounds(MenuSounds::EDIT_CREW);
                if hangar.cursor.is_flagship() {
                    self.flash_flagship_crew_count();
                } else {
                    self.flash_escort_crew_count(hangar.cursor.slot());
                }
            }
            EditMode::Exit => {
                p.set_menu_sounds(MenuSounds::ARROWS_SELECT);
                p.set_flash(FlashTarget::Menu3do);
            }
        }
    }

    /// `DMS_FlashActiveShip`.
    fn flash_active_ship(&self, hangar: &Hangar) {
        let p = self.platform;
        if hangar.cursor.is_flagship() {
            let (screen_width, screen_height) = p.screen_size();
            let radar = RadarLayout::for_screen(screen_width, screen_height);
            p.set_flash(FlashTarget::Rect(Rect {
                corner: Point { x: 0, y: 0 },
                width: radar.sis_screen_width as i16,
                height: 61,
            }));
        } else {
            p.set_flash(FlashTarget::Rect(escort_rect(hangar.cursor.slot())));
        }
    }

    /// `DMS_FlashFlagShipCrewCount`.
    fn flash_flagship_crew_count(&self) {
        let p = self.platform;
        p.set_context(ContextId::Status);
        p.set_flash(FlashTarget::Rect(gauge_rect(true)));
        p.set_context(ContextId::Space);
    }

    /// `DMS_FlashEscortShipCrewCount`.
    fn flash_escort_crew_count(&self, slot: u8) {
        let p = self.platform;
        let corner = slot_corner(slot);
        p.set_context(ContextId::Space);
        p.set_flash(FlashTarget::Rect(Rect {
            corner: Point {
                x: corner.x,
                y: corner.y + (SHIP_WIN_HEIGHT - 6),
            },
            width: SHIP_WIN_WIDTH,
            height: 5,
        }));
    }
}

/// `DMS_GetEscortShipRect`.
fn escort_rect(slot: u8) -> Rect {
    Rect {
        corner: slot_corner(slot),
        width: SHIP_WIN_WIDTH,
        height: SHIP_WIN_HEIGHT,
    }
}

const _: () = assert!(HANGAR_SHIPS_ROW == 6 && EMPTY_MODULE_SLOT == 22);
