//! Pulsed MENU key snapshot used by the Starbase and Outfit screens.
//!
//! Each field mirrors one `PulsedInputState.menu[KEY_MENU_*]` entry that
//! `starbase.c`, `outfit.c` and `menu.c` read (`controls.h:47-55`). These are
//! menu keys only. Ship flight uses PLAYER input, a separate state that
//! these screens never read or write.

/// Edge-triggered menu keys for one input frame. `Default` is "nothing
/// pressed".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MenuPulses {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    /// `KEY_MENU_SPECIAL`.
    pub special: bool,
    pub cancel: bool,
    pub page_up: bool,
    pub page_down: bool,
}

impl MenuPulses {
    /// `Left || Up`, the "previous item" test in `DoStarBase` and
    /// `DoMenuChooser`.
    #[must_use]
    pub const fn previous(self) -> bool {
        self.left || self.up
    }

    /// `Right || Down`, the "next item" test in `DoStarBase` and
    /// `DoMenuChooser`.
    #[must_use]
    pub const fn next(self) -> bool {
        self.right || self.down
    }
}
