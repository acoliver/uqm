//! Starbase and Outfit production logic (pure state and transaction models).
//!
//! This module ports the decision logic of the legacy C files
//! `sc2/src/uqm/starbase.c`, `outfit.c`, the `PM_FUEL` stepping in `menu.c`,
//! and the fuel capacity helpers in `sis.c`. It owns rules, not effects:
//! every function here is pure, takes plain data and returns typed results
//! or typed commands for a caller to carry out.
//!
//! # Runtime integration
//!
//! The game-facing C-ABI providers live in `crate::ui::{outfit,shipyard,starbase}`.
//! They preserve the engine menu callbacks, graphics and encounter lifecycle.
//! The SIS provider shares this module's tested fuel-capacity rule. The pure
//! screen models below remain separate from those runtime event loops; their
//! Save/Load and ShipSpin routes still return typed `Unported` errors rather
//! than driving the engine presentation and save interfaces.
//!
//! # Input
//!
//! Menus read pulsed MENU keys ([`MenuPulses`]). Ship flight reads PLAYER
//! input, which is a different state and is never touched here.
//!
//! # Layout
//!
//! - [`input`]: pulsed menu key snapshot.
//! - [`sis_modules`]: module types, slots, fuel tank capacity, fuel tank
//!   physical layout (`sis.c`).
//! - [`fuel`]: fuel purchase and drain transaction (`outfit.c`
//!   `ChangeFuelQuantity`, `sis.c` `DeltaSISGauges`).
//! - [`module_guard`]: module purchase and removal rules (`outfit.c`
//!   `DoInstallModule`).
//! - [`module_chooser`]: the module chooser's cursor and frame step
//!   (`outfit.c` `DoInstallModule`).
//! - `module_screen` (private): the chooser's drawing and SIS changes.
//! - [`starbase_menu`]: the four-state Starbase menu (`starbase.c`
//!   `DoStarBase`).
//! - [`visit`]: arrival time passage and post-visit cleanup (`starbase.c`
//!   `VisitStarBase`, `CleanupAfterStarBase`).
//! - [`outfit_menu`]: Outfit menu states (`outfit.c` `DoOutfit`).
//! - [`shipyard`]: Shipyard rules (`shipyard.c`); `shipyard_screen` (private)
//!   is the screen (`DoShipyard`, `DoModifyShips`).

#[cfg(not(test))]
pub mod cffi;
pub mod draw;
pub mod fuel;
pub mod gfx;
pub mod ids;
pub mod input;
pub mod module_chooser;
pub mod module_guard;
mod module_screen;
pub mod outfit_menu;
pub mod platform;
pub mod screen;
pub mod shipyard;
mod shipyard_screen;
pub mod sis_modules;
pub mod starbase_menu;
pub mod visit;

pub use fuel::{
    change_fuel_quantity, round_fuel_on_outfit_entry, FuelChange, FuelError, FuelKey, FuelRuState,
    GaugeDelta,
};
pub use input::MenuPulses;
pub use module_guard::{
    check_module_purchase, check_module_removal, module_price, ModuleBlock, ModuleCheck,
    ModuleCosts, ModuleCostsError, ShipLoad, SlotIndexOutOfRange,
};
pub use outfit_menu::{OutfitFrameInput, OutfitMenu, OutfitState, OutfitStep, SaveLoadReturn};
pub use platform::{Platform, StarbaseError, UnportedRoute};
pub use screen::{
    cleanup_after_visit, do_time_passage, finish_unallied_commander_talk, run_outfit, run_shipyard,
    run_starbase_menu, StatusModeGuard, UnalliedVisit,
};
pub use sis_modules::{
    fuel_bar_layout, FuelBar, FuelLayoutError, ModuleSlot, ModuleSlots, ModuleSlotsError,
    ModuleType,
};
pub use starbase_menu::{
    MenuCommand, StarbaseFrameInput, StarbaseMenu, StarbaseState, StarbaseStep,
};
pub use visit::{
    cleanup_after_starbase, entry_time_passage, CleanupOutcome, TimePassage, IN_STARBASE_MARK,
    LOST_DAYS,
};

#[cfg(test)]
mod module_route_tests;
#[cfg(test)]
mod new_game_reference;
#[cfg(test)]
mod shipyard_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
