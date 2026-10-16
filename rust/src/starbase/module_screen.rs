//! The module chooser screen: the effects of `DoInstallModule`
//! (`outfit.c:171-535`).
//!
//! [`ModuleChooser`] decides what a frame means. This module carries the
//! decision out through [`Platform`] and [`Gfx`] in the order the C code
//! does: the slot is written first, then RU change and the ship status is
//! redrawn, then the ship piece, the fuel bars and the flagship statistics.
//! The module chooser owns no resources. The art it draws from belongs to the
//! Outfit screen and is released with it.

use super::draw::{
    clear_sis_radar, display_landers, draw_flagship_stats, draw_module_strings,
    draw_outfit_menu_strings, draw_ship_piece, redistribute_fuel_from_sis, ModuleOnShow,
};
use super::fuel::FuelError;
use super::gfx::{ContextId, FlashTarget, FrameId, Rgba};
use super::module_chooser::{
    slot_flash_rect, Change, ChooserInput, ChooserStep, Cursor, ModuleChooser, Placement,
    Transaction,
};
use super::module_guard::{module_price, ModuleCosts};
use super::outfit_menu::OutfitState;
use super::platform::{
    MenuSounds, Platform, ResUnitsUpdate, SisView, StarbaseError, StateKey, UiStyle,
};
use super::sis_modules::ModuleType;

/// `BUILD_COLOR (MAKE_RGB15 (0x0A, 0x0A, 0x0A), 0x08)`, the status backdrop
/// that `ClearSISRect` draws with.
const STATUS_BACKDROP: Rgba = Rgba::rgb15(0x0A, 0x0A, 0x0A);

/// The art the chooser draws from. Both frames belong to the Outfit screen.
#[derive(Debug, Clone, Copy)]
pub(super) struct ChooserArt {
    /// `MENU_STATE.CurFrame`: the module menu pictures.
    pub menu_frame: FrameId,
    /// `MENU_STATE.ModuleFrame`: the ship pieces.
    pub module_frame: FrameId,
}

/// What the Outfit screen does after a chooser frame.
#[derive(Debug, Clone, Copy)]
pub(super) enum ChooserOutcome {
    /// The chooser keeps the input loop.
    Stay(ModuleChooser),
    /// Control is back with the Outfit menu (`pMS->InputFunc = DoOutfit`).
    BackToOutfit,
}

/// One `DoInstallModule` call.
pub(super) fn chooser_frame<P: Platform + ?Sized>(
    platform: &P,
    art: ChooserArt,
    chooser: ModuleChooser,
) -> Result<ChooserOutcome, StarbaseError> {
    let ship = platform.sis()?;
    let input = ChooserInput {
        pulses: platform.pulses(),
        activity: platform.current_activity(),
        ship: &ship,
        chmmr_bomb_state: platform.game_state(StateKey::ChmmrBombState),
    };
    let (chooser, step) = chooser.step(&input)?;
    // `CHECK_ABORT` is tested before `SetMenuSounds`.
    if step != ChooserStep::Abort {
        platform.set_menu_sounds(MenuSounds::ARROWS_SELECT);
    }
    let costs = ModuleCosts::from(ship.module_cost);
    let on_show = |module: Option<ModuleType>| {
        module.map(|module| ModuleOnShow {
            module,
            price: module_price(&costs, module),
        })
    };
    let strip = Strip { platform, art };

    match step {
        ChooserStep::Idle => {}
        ChooserStep::Abort => return Ok(ChooserOutcome::BackToOutfit),
        ChooserStep::Open { shown } => {
            platform.set_context(ContextId::Space);
            clear_sis_radar(platform, platform.menu_style(), ship.fuel_cost);
            platform.set_flash(FlashTarget::Off);
            strip.init_flash(&chooser, on_show(shown));
        }
        ChooserStep::Refused(_) => platform.play_failure_sound(),
        ChooserStep::Offer { piece } => {
            platform.set_context(ContextId::Space);
            platform.set_flash(FlashTarget::Off);
            strip.init_flash(&chooser, on_show(Some(piece)));
        }
        ChooserStep::ShowOffer { piece } => {
            platform.pre_update_flash();
            draw_module_strings(platform, art.menu_frame, on_show(Some(piece)));
            platform.post_update_flash();
        }
        ChooserStep::BackToSlots => {
            platform.set_context(ContextId::Space);
            platform.set_flash(FlashTarget::Off);
            strip.init_flash(&chooser, None);
        }
        ChooserStep::Move { shown } => {
            platform.set_context(ContextId::Space);
            strip.init_flash(&chooser, on_show(shown));
        }
        ChooserStep::Apply(transaction) => {
            platform.set_context(ContextId::Space);
            platform.set_flash(FlashTarget::Off);
            apply_transaction(platform, art, &ship, &transaction)?;
            strip.init_flash(&chooser, on_show(transaction.shown()));
        }
        ChooserStep::Leave => {
            platform.set_context(ContextId::Space);
            platform.set_flash(FlashTarget::Off);
            platform.set_context(ContextId::Status);
            draw_outfit_menu_strings(
                platform,
                platform.menu_style(),
                OutfitState::Modules as u8,
                ship.fuel_cost,
            );
            platform.set_flash(FlashTarget::Menu3do);
            clear_sis_display(platform);
            return Ok(ChooserOutcome::BackToOutfit);
        }
    }
    Ok(ChooserOutcome::Stay(chooser))
}

/// The module strip and the flash rectangle, which `InitFlash` draws.
struct Strip<'a, P: Platform + ?Sized> {
    platform: &'a P,
    art: ChooserArt,
}

impl<P: Platform + ?Sized> Strip<'_, P> {
    /// `DrawModuleStrings`, then the flash: the module menu while offering,
    /// the slot's rectangle while choosing a slot.
    fn init_flash(&self, chooser: &ModuleChooser, shown: Option<ModuleOnShow>) {
        let p = self.platform;
        draw_module_strings(p, self.art.menu_frame, shown);
        let target = match chooser.cursor() {
            Cursor::Offer(_) => FlashTarget::MenuAny,
            Cursor::Slots(kind) => FlashTarget::Rect(slot_flash_rect(kind, chooser.item())),
        };
        p.set_flash(target);
    }
}

/// Write the slot, pay or refund, and redraw (`outfit.c:310-356`).
fn apply_transaction<P: Platform + ?Sized>(
    platform: &P,
    art: ChooserArt,
    before: &SisView,
    transaction: &Transaction,
) -> Result<(), StarbaseError> {
    let new_piece = transaction.new_piece();
    match transaction.placement {
        Placement::Drive(item) => platform.set_drive_slot(usize::from(item), new_piece),
        Placement::Jet(item) => platform.set_jet_slot(usize::from(item), new_piece),
        Placement::Module(item) => platform.set_module_slot(usize::from(item), new_piece),
        Placement::Landers { count_after } => platform.set_num_landers(count_after),
    }

    let expected = match transaction.change {
        Change::Install { price, .. } => before
            .res_units
            .checked_sub(price)
            .expect("the purchase check guarantees the balance covers the price"),
        Change::Remove { refund, .. } => before
            .res_units
            .checked_add(refund)
            .ok_or(FuelError::ResUnitsOverflow)?,
    };
    platform.redraw_ship_status(ResUnitsUpdate::Change(transaction.res_units_delta()));
    let after = platform.sis()?;
    if after.res_units != expected {
        return Err(StarbaseError::ResUnitsMismatch {
            expected,
            actual: after.res_units,
        });
    }

    match transaction.placement {
        Placement::Landers { .. } => display_landers(
            platform,
            &after,
            platform.game_state(StateKey::ChmmrBombState),
            art.module_frame,
        ),
        Placement::Drive(item) | Placement::Jet(item) | Placement::Module(item) => {
            draw_ship_piece(
                platform,
                &after,
                art.module_frame,
                u16::from(new_piece),
                u16::from(item),
                false,
            );
            // Only module slots change the fuel tanks.
            if new_piece > ModuleType::TurningJets as u8
                && transaction.old_piece() > ModuleType::TurningJets as u8
            {
                redistribute_fuel_from_sis(platform, &after)?;
            }
            if platform.font_style() == UiStyle::Pc {
                draw_flagship_stats(platform, &after);
            }
        }
    }
    Ok(())
}

/// `ClearSISRect (DRAW_SIS_DISPLAY)` (`sis.c:96-99`): redraw the ship status.
fn clear_sis_display<P: Platform + ?Sized>(platform: &P) {
    let previous = platform.set_context(ContextId::Status);
    let old_color = platform.set_fg_color(STATUS_BACKDROP);
    platform.batch();
    platform.redraw_ship_status(ResUnitsUpdate::Redraw);
    platform.unbatch();
    platform.set_fg_color(old_color);
    platform.restore_context(previous);
}
