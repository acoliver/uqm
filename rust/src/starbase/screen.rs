//! The Starbase menu and Outfit screens as input-loop frame functions.
//!
//! [`StarbaseScreen`] is `DoStarBase` and [`OutfitScreen`] is `DoOutfit`
//! (`starbase.c`, `outfit.c`). Each `frame` call is one call of the C
//! `InputFunc`: it asks the pure menu model what the keys mean
//! ([`StarbaseMenu::step`], [`OutfitMenu::step`]) and carries out the result
//! through [`Platform`] and [`Gfx`].
//!
//! Resources a screen loads are owned by guard values, so every exit
//! releases them: a normal exit, an abort or load request, and an error.
//!
//! Routes the Rust game cannot serve (Shipyard, Save/Load) end the screen with
//! [`StarbaseError::Unported`].

use super::draw::{
    display_landers, draw_base_state_strings, draw_flagship_name, draw_flagship_stats,
    draw_outfit_menu_strings, draw_ship_piece, flash_fuel_gauge, redistribute_fuel,
    redistribute_fuel_from_sis, rotate_starbase, StarbaseAnimation,
};
use super::fuel::{change_fuel_quantity, round_fuel_on_outfit_entry, FuelChange, FuelRuState};
use super::gfx::{ContextHandle, ContextId, FlashTarget, FrameId, Gfx, Point, Rgba, Stamp};
use super::ids::{
    EMPTY_SLOT, MODULES_PMAP_ANIM, OUTFIT_MUSIC, OUTFIT_PMAP_ANIM, SISMODS_MASK_PMAP_ANIM,
    STARBASE_ANIM, STARBASE_MUSIC, STARBASE_STRING_BASE,
};
use super::module_chooser::ModuleChooser;
use super::module_screen::{chooser_frame, ChooserArt, ChooserOutcome};
use super::outfit_menu::{OutfitFrameInput, OutfitMenu, OutfitState, OutfitStep};
use super::platform::{
    MenuSounds, MusicId, Platform, StarbaseError, StateKey, StatusMessageMode, UiStyle,
    UnportedRoute, ONE_SECOND,
};
use super::sis_modules::{fuel_bar_layout, ModuleSlots};
use super::starbase_menu::{MenuCommand, StarbaseFrameInput, StarbaseMenu, StarbaseStep};
use super::visit::{cleanup_after_starbase, CleanupOutcome, TimePassage, IN_STARBASE_MARK};
use crate::mainloop::types::activity_flags;

/// Restores the previous drawing context when dropped (`OldContext =
/// SetContext (...)` ... `SetContext (OldContext)`).
struct ContextGuard<'a, P: Platform + ?Sized> {
    platform: &'a P,
    previous: ContextHandle,
}

impl<'a, P: Platform + ?Sized> ContextGuard<'a, P> {
    fn enter(platform: &'a P, context: ContextId) -> Self {
        let previous = platform.set_context(context);
        Self { platform, previous }
    }
}

impl<P: Platform + ?Sized> Drop for ContextGuard<'_, P> {
    fn drop(&mut self) {
        self.platform.restore_context(self.previous);
    }
}

/// Restores the status message mode when dropped.
pub struct StatusModeGuard<'a, P: Platform + ?Sized> {
    platform: &'a P,
    previous: StatusMessageMode,
}

impl<'a, P: Platform + ?Sized> StatusModeGuard<'a, P> {
    /// `prevMsgMode = SetStatusMessageMode (mode)`.
    pub fn enter(platform: &'a P, mode: StatusMessageMode) -> Self {
        let previous = platform.set_status_message_mode(mode);
        Self { platform, previous }
    }
}

impl<P: Platform + ?Sized> Drop for StatusModeGuard<'_, P> {
    fn drop(&mut self) {
        self.platform.set_status_message_mode(self.previous);
    }
}

/// Marks the Starbase menu as the active input loop for automation, and
/// clears the mark on every exit.
struct DockedGuard<'a, P: Platform + ?Sized> {
    platform: &'a P,
}

impl<'a, P: Platform + ?Sized> DockedGuard<'a, P> {
    fn enter(platform: &'a P) -> Self {
        platform.observe_starbase_menu(true);
        Self { platform }
    }
}

impl<P: Platform + ?Sized> Drop for DockedGuard<'_, P> {
    fn drop(&mut self) {
        self.platform.observe_starbase_menu(false);
    }
}

/// The animation frames and music the Starbase menu holds while it shows.
struct StarbaseAssets<'a, P: Platform + ?Sized> {
    platform: &'a P,
    frame: Option<FrameId>,
    music: Option<MusicId>,
}

impl<P: Platform + ?Sized> Drop for StarbaseAssets<'_, P> {
    fn drop(&mut self) {
        if let Some(frame) = self.frame.take() {
            self.platform.destroy_drawable(frame);
        }
        self.platform.stop_music();
        if let Some(music) = self.music.take() {
            self.platform.destroy_music(music);
        }
    }
}

/// `DoStarBase`.
pub struct StarbaseScreen<'a, P: Platform + ?Sized> {
    platform: &'a P,
    menu: StarbaseMenu,
    assets: Option<StarbaseAssets<'a, P>>,
    animation: StarbaseAnimation,
}

impl<'a, P: Platform + ?Sized> StarbaseScreen<'a, P> {
    #[must_use]
    pub fn new(platform: &'a P) -> Self {
        Self {
            platform,
            menu: StarbaseMenu::new(),
            assets: None,
            animation: StarbaseAnimation::default(),
        }
    }

    /// One `DoStarBase` call. `Ok(true)` keeps the loop running.
    pub fn frame(&mut self) -> Result<bool, StarbaseError> {
        let p = self.platform;
        let activity = p.current_activity();
        let leaving = activity.0 & (activity_flags::CHECK_ABORT | activity_flags::CHECK_LOAD) != 0;
        if !leaving {
            p.set_menu_sounds(MenuSounds::ARROWS_SELECT);
        }

        let input = StarbaseFrameInput {
            pulses: p.pulses(),
            activity,
            last_activity: p.last_activity(),
        };
        let (menu, step) = self.menu.step(input);
        self.menu = menu;

        match step {
            StarbaseStep::Initialize { last_activity } => {
                self.initialize(last_activity)?;
                Ok(true)
            }
            StarbaseStep::Idle => {
                self.animate_frame(|_| {});
                Ok(true)
            }
            StarbaseStep::Highlight { from, to } => {
                self.animate_frame(|screen| {
                    draw_base_state_strings(screen.platform, Some(from), to);
                });
                Ok(true)
            }
            StarbaseStep::Run(command) => {
                self.assets = None;
                self.run_command(command)?;
                Ok(true)
            }
            StarbaseStep::Depart {
                clear_starbase_visited,
            } => {
                self.assets = None;
                if clear_starbase_visited {
                    p.set_game_state(StateKey::StarbaseVisited, 0);
                }
                Ok(false)
            }
        }
    }

    fn initialize(
        &mut self,
        last_activity: crate::mainloop::types::ActivityValue,
    ) -> Result<(), StarbaseError> {
        let p = self.platform;
        p.set_last_activity(last_activity);
        p.set_flash(FlashTarget::Off);

        let mut assets = StarbaseAssets {
            platform: p,
            frame: None,
            music: None,
        };
        let frame = p.load_graphic(STARBASE_ANIM)?;
        assets.frame = Some(frame);
        let music = p.load_music(STARBASE_MUSIC)?;
        assets.music = Some(music);

        p.set_context(ContextId::Screen);
        p.set_transition_source_none();
        p.batch();
        p.set_bg_color(Rgba::BLACK);
        p.clear_drawable();
        rotate_starbase(p, &mut self.animation, Some(frame), p.now());
        draw_base_state_strings(p, None, self.menu.state());
        p.screen_transition(3);
        p.play_music(music);
        p.unbatch();
        self.assets = Some(assets);
        Ok(())
    }

    /// The common tail of an idle or highlight frame (`starbase.c:405-413`).
    fn animate_frame(&mut self, draw: impl FnOnce(&Self)) {
        let p = self.platform;
        p.batch();
        p.set_context(ContextId::Screen);
        draw(self);
        rotate_starbase(p, &mut self.animation, None, p.now());
        p.unbatch();
        p.sleep_thread(ONE_SECOND / 30);
    }

    fn run_command(&self, command: MenuCommand) -> Result<(), StarbaseError> {
        let p = self.platform;
        match command {
            MenuCommand::Talk => {
                p.flush_input();
                // The conversation is its own mode while it runs.
                p.observe_starbase_menu(false);
                p.run_commander_conversation();
                p.observe_starbase_menu(true);
                // The conversation clears the flag; the Starbase needs it.
                p.set_game_state(StateKey::GlobalFlagsAndData, u32::from(IN_STARBASE_MARK));
                Ok(())
            }
            MenuCommand::Outfit => {
                p.set_menu_sounds(MenuSounds::ARROWS_SELECT);
                run_outfit(p)
            }
            MenuCommand::Shipyard => Err(StarbaseError::Unported(UnportedRoute::Shipyard)),
        }
    }
}

/// The art and music the Outfit screen holds while it shows.
struct OutfitAssets<'a, P: Platform + ?Sized> {
    platform: &'a P,
    menu_frame: Option<FrameId>,
    module_frame: Option<FrameId>,
    music: Option<MusicId>,
}

impl<P: Platform + ?Sized> Drop for OutfitAssets<'_, P> {
    fn drop(&mut self) {
        if let Some(frame) = self.menu_frame.take() {
            self.platform.destroy_drawable(frame);
        }
        if let Some(frame) = self.module_frame.take() {
            self.platform.destroy_drawable(frame);
        }
        self.platform.stop_music();
        if let Some(music) = self.music.take() {
            self.platform.destroy_music(music);
        }
    }
}

impl<P: Platform + ?Sized> OutfitAssets<'_, P> {
    /// The art the module chooser draws from. The Outfit screen loads all of
    /// it before any frame can open the chooser.
    fn chooser_art(&self) -> ChooserArt {
        ChooserArt {
            menu_frame: self.menu_frame.expect("the Outfit art is loaded"),
            module_frame: self.module_frame.expect("the Outfit art is loaded"),
        }
    }
}

/// `DoOutfit`. While the module chooser (`DoInstallModule`) is open it holds
/// the input loop, as `MENU_STATE.InputFunc` does in C.
pub struct OutfitScreen<'a, P: Platform + ?Sized> {
    platform: &'a P,
    menu: OutfitMenu,
    assets: Option<OutfitAssets<'a, P>>,
    chooser: Option<ModuleChooser>,
}

impl<'a, P: Platform + ?Sized> OutfitScreen<'a, P> {
    #[must_use]
    pub fn new(platform: &'a P) -> Self {
        Self {
            platform,
            menu: OutfitMenu::new(),
            assets: None,
            chooser: None,
        }
    }

    /// One `DoOutfit` call. `Ok(true)` keeps the loop running.
    pub fn frame(&mut self) -> Result<bool, StarbaseError> {
        if let Some(chooser) = self.chooser.take() {
            return self.chooser_frame(chooser);
        }
        let p = self.platform;
        let input = OutfitFrameInput {
            pulses: p.pulses(),
            activity: p.current_activity(),
        };
        let (menu, step) = self.menu.step(input);
        self.menu = menu;

        match step {
            OutfitStep::Initialize => self.initialize()?,
            OutfitStep::Idle => {
                self.set_frame_sounds();
                self.pace_fuel_adjust();
            }
            OutfitStep::Highlight { to, .. } => {
                self.set_frame_sounds();
                let sis = p.sis()?;
                draw_outfit_menu_strings(p, p.menu_style(), to as u8, sis.fuel_cost);
                p.sleep_thread(ONE_SECOND / 20);
            }
            OutfitStep::EnterFuelAdjust => {
                flash_fuel_gauge_in_status(p);
            }
            OutfitStep::LeaveFuelAdjust => p.set_flash(FlashTarget::Menu3do),
            OutfitStep::AdjustFuel(key) => {
                self.set_frame_sounds();
                self.adjust_fuel(key)?;
                self.pace_fuel_adjust();
            }
            OutfitStep::EnterModules => {
                // `DoInstallModule` runs at once, in the frame that selected
                // Modules, to set itself up.
                let chooser = ModuleChooser::new(p.game_state(StateKey::ChmmrBombState));
                return self.chooser_frame(chooser);
            }
            OutfitStep::OpenSaveLoad => {
                return Err(StarbaseError::Unported(UnportedRoute::SaveLoad));
            }
            OutfitStep::Exit => {
                self.assets = None;
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// One `DoInstallModule` call.
    fn chooser_frame(&mut self, chooser: ModuleChooser) -> Result<bool, StarbaseError> {
        let art = self
            .assets
            .as_ref()
            .expect("the Outfit art is loaded")
            .chooser_art();
        match chooser_frame(self.platform, art, chooser)? {
            ChooserOutcome::Stay(chooser) => self.chooser = Some(chooser),
            ChooserOutcome::BackToOutfit => {}
        }
        Ok(true)
    }

    /// `SetMenuSounds` of the final branch of `DoOutfit`.
    fn set_frame_sounds(&self) {
        let sounds = if self.menu.state() == OutfitState::DoFuel {
            MenuSounds::FUEL_ADJUST
        } else {
            MenuSounds::ARROWS_SELECT
        };
        self.platform.set_menu_sounds(sounds);
    }

    /// `SleepThread (ONE_SECOND / 30)` after each fuel adjust frame.
    fn pace_fuel_adjust(&self) {
        if self.menu.state() == OutfitState::DoFuel {
            self.platform.sleep_thread(ONE_SECOND / 30);
        }
    }

    fn redistribute(&self, sis: &super::platform::SisView) -> Result<(), StarbaseError> {
        redistribute_fuel_from_sis(self.platform, sis)
    }

    fn initialize(&mut self) -> Result<(), StarbaseError> {
        let p = self.platform;
        let sis = p.sis()?;
        let slots = ModuleSlots::from_raw(&sis.module_slots)?;
        let rounded_fuel = round_fuel_on_outfit_entry(sis.fuel_on_board)?;
        let fonts = p.font_style();
        let chmmr_bomb_state = p.game_state(StateKey::ChmmrBombState);
        let lander_shields = p.game_state(StateKey::LanderShields);

        let mut assets = OutfitAssets {
            platform: p,
            menu_frame: None,
            module_frame: None,
            music: None,
        };
        assets.menu_frame = Some(p.load_graphic(MODULES_PMAP_ANIM)?);
        assets.music = Some(p.load_music(OUTFIT_MUSIC)?);
        let module_frame = p.load_graphic(SISMODS_MASK_PMAP_ANIM)?;
        assets.module_frame = Some(module_frame);
        let background = p.load_graphic(OUTFIT_PMAP_ANIM)?;
        let music = assets.music.expect("music was just loaded");

        p.set_transition_source_none();
        p.batch();
        p.draw_sis_frame();
        p.draw_sis_message(&p.game_string(STARBASE_STRING_BASE + 2));
        p.draw_sis_title(&p.game_string(STARBASE_STRING_BASE));

        p.set_context(ContextId::Space);
        p.draw_stamp(Stamp {
            origin: Point { x: 0, y: 0 },
            frame: background,
        });
        p.destroy_drawable(background);

        for (index, piece) in sis.drive_slots.iter().enumerate() {
            if u16::from(*piece) < EMPTY_SLOT {
                draw_ship_piece(
                    p,
                    &sis,
                    module_frame,
                    u16::from(*piece),
                    index as u16,
                    false,
                );
            }
        }
        for (index, piece) in sis.jet_slots.iter().enumerate() {
            if u16::from(*piece) < EMPTY_SLOT {
                draw_ship_piece(
                    p,
                    &sis,
                    module_frame,
                    u16::from(*piece),
                    index as u16,
                    false,
                );
            }
        }
        for (index, piece) in sis.module_slots.iter().enumerate() {
            if u16::from(*piece) < EMPTY_SLOT {
                draw_ship_piece(
                    p,
                    &sis,
                    module_frame,
                    u16::from(*piece),
                    index as u16,
                    false,
                );
            }
        }
        redistribute_fuel(p, &fuel_bar_layout(&slots, sis.fuel_on_board)?);
        display_landers(p, &sis, chmmr_bomb_state, module_frame);
        if chmmr_bomb_state < 3 {
            draw_lander_shields(p, module_frame, lander_shields);
        }

        draw_outfit_menu_strings(p, p.menu_style(), OutfitState::Fuel as u8, sis.fuel_cost);
        draw_flagship_name(p, fonts, &sis);
        if fonts == UiStyle::Pc {
            draw_flagship_stats(p, &sis);
        }

        p.screen_transition(3);
        p.play_music(music);
        p.unbatch();
        p.set_flash(FlashTarget::Menu3do);

        p.set_fuel_on_board(rounded_fuel);
        p.set_context(ContextId::Status);
        self.assets = Some(assets);
        Ok(())
    }

    /// `ChangeFuelQuantity` (`outfit.c:543-597`). The pure transaction model
    /// decides; the status gauges are updated through `DeltaSISGauges`, and
    /// the result is checked against the model's own prediction.
    fn adjust_fuel(&self, key: super::fuel::FuelKey) -> Result<(), StarbaseError> {
        let p = self.platform;
        let sis = p.sis()?;
        let capacity = ModuleSlots::from_raw(&sis.module_slots)?.fuel_tank_capacity();
        let before = FuelRuState {
            fuel_on_board: sis.fuel_on_board,
            res_units: sis.res_units,
        };

        match change_fuel_quantity(before, capacity, sis.fuel_cost, key)? {
            FuelChange::Rejected => p.play_failure_sound(),
            FuelChange::Apply(delta) => {
                let expected = before.apply(delta, capacity)?;
                p.pre_update_flash();
                p.apply_gauge_delta(delta);
                p.post_update_flash();

                let after = p.sis()?;
                if after.fuel_on_board != expected.fuel_on_board
                    || after.res_units != expected.res_units
                {
                    return Err(StarbaseError::GaugeMismatch {
                        expected_fuel: expected.fuel_on_board,
                        expected_res_units: expected.res_units,
                        actual_fuel: after.fuel_on_board,
                        actual_res_units: after.res_units,
                    });
                }
                self.redistribute(&after)?;
            }
        }
        flash_fuel_gauge(p);
        Ok(())
    }
}

/// `SetContext (StatusContext); GetGaugeRect (&r, FALSE); SetFlashRect (&r)`
/// when fuel adjust opens. The context stays the status context, as in C.
fn flash_fuel_gauge_in_status<G: Gfx + ?Sized>(g: &G) {
    g.set_context(ContextId::Status);
    g.set_flash(FlashTarget::Rect(super::draw::gauge_rect(false)));
}

/// The four lander shield stamps (`outfit.c:674-690`).
fn draw_lander_shields<G: Gfx + ?Sized>(g: &G, module_frame: FrameId, shield_flags: u32) {
    // EARTHQUAKE_DISASTER, BIOLOGICAL_DISASTER, LIGHTNING_DISASTER and
    // LAVASPOT_DISASTER (`planets.h`), in the order the art lists them.
    const SHIELD_BITS: [u32; 4] = [1, 0, 2, 3];
    let first = g.frame_count(module_frame) - 5;
    let mut frame = g.abs_frame(module_frame, first);
    for (position, bit) in SHIELD_BITS.into_iter().enumerate() {
        if position > 0 {
            frame = g.inc_frame(frame);
        }
        if shield_flags & (1 << bit) != 0 {
            g.draw_stamp(Stamp {
                origin: Point { x: 0, y: 0 },
                frame,
            });
        }
    }
}

/// Run the screen frames inside the platform's input loop and report the
/// first error.
fn run_loop(
    platform: &(impl Platform + ?Sized),
    mut frame: impl FnMut() -> Result<bool, StarbaseError>,
) -> Result<(), StarbaseError> {
    let mut failure = None;
    platform.run_input_loop(&mut || match frame() {
        Ok(keep_going) => keep_going,
        Err(error) => {
            failure = Some(error);
            false
        }
    });
    failure.map_or(Ok(()), Err)
}

/// Run the Outfit screen to its end (`DoInput (pMS, TRUE)` with `DoOutfit`).
pub fn run_outfit<P: Platform + ?Sized>(platform: &P) -> Result<(), StarbaseError> {
    let mut screen = OutfitScreen::new(platform);
    run_loop(platform, || screen.frame())
}

/// The menu loop of `VisitStarBase` (`starbase.c:505-511`), with the screen
/// context restored on every exit.
pub fn run_starbase_menu<P: Platform + ?Sized>(platform: &P) -> Result<(), StarbaseError> {
    let _docked = DockedGuard::enter(platform);
    let _context = ContextGuard::enter(platform, ContextId::Screen);
    let mut screen = StarbaseScreen::new(platform);
    run_loop(platform, || screen.frame())
}

/// `DoTimePassage` (`starbase.c:420-428`): fade to black, then lose the
/// days on the game clock.
pub fn do_time_passage<P: Platform + ?Sized>(platform: &P, passage: TimePassage) {
    let wake_time = platform.fade_to_black(passage.fade_seconds * ONE_SECOND);
    platform.sleep_thread_until(wake_time);
    platform.advance_clock_days(passage.days);
}

/// `CleanupAfterStarBase` (`starbase.c:517-529`).
pub fn cleanup_after_visit<P: Platform + ?Sized>(platform: &P) {
    if let CleanupOutcome::Apply {
        global_flags_and_data,
        current_activity,
        next_activity,
    } = cleanup_after_starbase(platform.current_activity())
    {
        platform.set_game_state(
            StateKey::GlobalFlagsAndData,
            u32::from(global_flags_and_data),
        );
        platform.set_current_activity(current_activity);
        platform.set_next_activity(next_activity);
    }
}
