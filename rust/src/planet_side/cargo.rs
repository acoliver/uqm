//! Deterministic mineral and biological cargo accounting.

pub const NUM_ELEMENT_CATEGORIES: usize = 8;
const MAX_SCROUNGED: u16 = 50;

/// Outcome of attempting to collect a surface node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoPickup {
    Full,
    Collected { amount: u16, complete: bool },
}

/// Mineral cargo accumulated during one lander trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MineralCargo {
    level: u16,
    capacity: u16,
    categories: [u16; NUM_ELEMENT_CATEGORIES],
}

impl MineralCargo {
    #[must_use]
    pub fn new(storage_capacity: u16, current_ship_mass: u16, improved: bool) -> Self {
        let lander_limit = if improved {
            MAX_SCROUNGED * 2
        } else {
            MAX_SCROUNGED
        };
        Self {
            level: 0,
            capacity: storage_capacity
                .saturating_sub(current_ship_mass)
                .min(lander_limit),
            categories: [0; NUM_ELEMENT_CATEGORIES],
        }
    }

    #[must_use]
    pub const fn level(&self) -> u16 {
        self.level
    }

    #[must_use]
    pub const fn capacity(&self) -> u16 {
        self.capacity
    }

    #[must_use]
    pub const fn categories(&self) -> &[u16; NUM_ELEMENT_CATEGORIES] {
        &self.categories
    }

    pub fn collect(&mut self, category: usize, available: u16) -> CargoPickup {
        if self.level >= self.capacity || category >= NUM_ELEMENT_CATEGORIES {
            return CargoPickup::Full;
        }
        let amount = available.min(self.capacity - self.level);
        self.level += amount;
        self.categories[category] += amount;
        CargoPickup::Collected {
            amount,
            complete: amount == available,
        }
    }
}

/// Biological cargo accumulated during one lander trip.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BioCargo {
    level: u16,
}

impl BioCargo {
    #[must_use]
    pub const fn level(self) -> u16 {
        self.level
    }

    pub fn collect(&mut self, available: u16) -> CargoPickup {
        if self.level >= MAX_SCROUNGED {
            return CargoPickup::Full;
        }
        let amount = available.min(MAX_SCROUNGED - self.level);
        self.level += amount;
        CargoPickup::Collected {
            amount,
            complete: amount == available,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_and_improved_landers_use_distinct_internal_limits() {
        assert_eq!(MineralCargo::new(200, 0, false).capacity(), 50);
        assert_eq!(MineralCargo::new(200, 0, true).capacity(), 100);
    }

    #[test]
    fn ship_storage_free_space_caps_lander_capacity() {
        assert_eq!(MineralCargo::new(100, 75, true).capacity(), 25);
        assert_eq!(MineralCargo::new(50, 70, false).capacity(), 0);
    }

    #[test]
    fn mineral_collection_tracks_partial_pickup_and_category() {
        let mut cargo = MineralCargo::new(10, 0, false);
        assert_eq!(
            cargo.collect(3, 7),
            CargoPickup::Collected {
                amount: 7,
                complete: true
            }
        );
        assert_eq!(
            cargo.collect(3, 8),
            CargoPickup::Collected {
                amount: 3,
                complete: false
            }
        );
        assert_eq!(cargo.level(), 10);
        assert_eq!(cargo.categories()[3], 10);
        assert_eq!(cargo.collect(3, 1), CargoPickup::Full);
    }

    #[test]
    fn biological_collection_is_capped_at_fifty() {
        let mut cargo = BioCargo::default();
        assert_eq!(
            cargo.collect(48),
            CargoPickup::Collected {
                amount: 48,
                complete: true
            }
        );
        assert_eq!(
            cargo.collect(5),
            CargoPickup::Collected {
                amount: 2,
                complete: false
            }
        );
        assert_eq!(cargo.level(), 50);
        assert_eq!(cargo.collect(1), CargoPickup::Full);
    }
}
