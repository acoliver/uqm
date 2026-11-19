//! Strongly typed values shared by planet-side gameplay reducers.

/// A location on the wrapping planet surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SurfacePoint {
    pub x: i32,
    pub y: i32,
}

/// Lander crew count.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CrewCount(u8);

impl CrewCount {
    #[must_use]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    pub(crate) fn lose_one(&mut self) -> bool {
        self.lose(1) != 0
    }

    /// Remove up to `count` crew, returning the number actually removed.
    pub(crate) fn lose(&mut self, count: u8) -> u8 {
        let removed = self.0.min(count);
        self.0 -= removed;
        removed
    }
}

/// Bit set describing installed lander shields.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShieldSet(u8);

impl ShieldSet {
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits)
    }

    #[must_use]
    pub const fn contains(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

/// Installed lander upgrades that alter deterministic gameplay rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LanderUpgrades {
    pub improved_speed: bool,
    pub improved_cargo: bool,
    pub improved_shot: bool,
    pub shields: ShieldSet,
}
