//! Game modes: battle, melee setup, navigation, solar system, orbit, planet
//! side, communication, starbase and the menus.
#![forbid(unsafe_code)]

pub mod battle;
pub mod comm;
pub mod front_end;
pub mod mainloop {
    pub mod restart_menu;
}
pub mod navigation;
pub mod pickship;
pub mod planet_side;
pub mod planets;
pub mod starbase;
pub mod supermelee;
pub mod ui;
