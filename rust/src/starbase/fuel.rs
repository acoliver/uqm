//! Fuel purchase and drain transaction.
//!
//! Ports `ChangeFuelQuantity` (`outfit.c:543-597`) and the fuel and RU parts
//! of `DeltaSISGauges` (`sis.c:1027-1112`) as pure arithmetic. The caller
//! reads the SIS state, asks [`change_fuel_quantity`] what the key press
//! does, and applies the returned [`GaugeDelta`] through its own gauge
//! update.
//!
//! # Differences from the C arithmetic
//!
//! The C code mixes `DWORD`, `int` and `SIZE` (16-bit). `maxAfford =
//! ResUnits / FuelCost` is cast to `int` and then multiplied by 100, which
//! overflows for very large RU balances. Every clamp here is computed in
//! `i64`, which gives the value the C code means for all inputs where C is
//! defined, and the same result for the huge balances where it is not. The
//! chosen increment is always one of +/-100 or +/-1000 or a clamp of those,
//! so it fits `i16` and never equals `UNDEFINED_DELTA` (0x7FFF).
//!
//! Inputs the C code would mishandle are explicit errors: `FuelCost == 0`
//! divides by zero in C, and fuel above capacity makes C's unsigned
//! `capacity - fuel` wrap.

use super::sis_modules::{FUEL_TANK_SCALE, FUEL_VOLUME_PER_ROW};
use crate::starbase::input::MenuPulses;

/// A fuel key press while the DOFUEL state is active (`outfit.c:545-553`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelKey {
    /// `KEY_MENU_UP`: +1 unit (+100 raw).
    UnitUp,
    /// `KEY_MENU_DOWN`: -1 unit (-100 raw).
    UnitDown,
    /// `KEY_MENU_PAGE_UP`: +1 bar (+1000 raw).
    BarUp,
    /// `KEY_MENU_PAGE_DOWN`: -1 bar (-1000 raw).
    BarDown,
}

impl FuelKey {
    /// The key `ChangeFuelQuantity` acts on. When several are pulsed in one
    /// frame the C `if/else if` order decides: up, down, page up, page down.
    #[must_use]
    pub const fn from_pulses(pulses: MenuPulses) -> Option<Self> {
        if pulses.up {
            Some(FuelKey::UnitUp)
        } else if pulses.down {
            Some(FuelKey::UnitDown)
        } else if pulses.page_up {
            Some(FuelKey::BarUp)
        } else if pulses.page_down {
            Some(FuelKey::BarDown)
        } else {
            None
        }
    }

    const fn requested_increment(self) -> i64 {
        match self {
            FuelKey::UnitUp => FUEL_TANK_SCALE as i64,
            FuelKey::UnitDown => -(FUEL_TANK_SCALE as i64),
            FuelKey::BarUp => FUEL_VOLUME_PER_ROW as i64,
            FuelKey::BarDown => -(FUEL_VOLUME_PER_ROW as i64),
        }
    }
}

/// The two SIS values a fuel transaction reads and changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuelRuState {
    /// `SIS_STATE.FuelOnBoard`, raw fuel points (100 per displayed unit).
    pub fuel_on_board: u32,
    /// `SIS_STATE.ResUnits`.
    pub res_units: u32,
}

/// The `(fuel_delta, resunit_delta)` pair passed to
/// `DeltaSISGauges (0, fuel, resunit)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GaugeDelta {
    /// Raw fuel points, C `SIZE`.
    pub fuel: i16,
    /// Resource units, C `int`. Negative pays, positive refunds.
    pub res_units: i32,
}

/// What a fuel key press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuelChange {
    /// No room, not enough RU, or no fuel to drain. The C code plays
    /// `MENU_SOUND_FAILURE` and changes nothing (`outfit.c:573-577`).
    Rejected,
    /// Apply this delta, then redistribute the fuel bars.
    Apply(GaugeDelta),
}

/// Input the fuel arithmetic cannot give a meaningful answer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FuelError {
    #[error("fuel cost per unit is zero")]
    ZeroFuelCost,
    #[error("fuel on board ({fuel}) exceeds tank capacity ({capacity})")]
    FuelExceedsCapacity { fuel: u32, capacity: u32 },
    #[error("resource units overflow after refund")]
    ResUnitsOverflow,
    #[error("fuel overflow while rounding")]
    FuelOverflow,
}

/// What pressing `key` does with the given ship state.
///
/// `tank_capacity` is `GetFuelTankCapacity ()` and `fuel_cost` is
/// `GLOBAL (FuelCost)`, RU per displayed fuel unit (20 in a normal game).
pub fn change_fuel_quantity(
    state: FuelRuState,
    tank_capacity: u32,
    fuel_cost: u8,
    key: FuelKey,
) -> Result<FuelChange, FuelError> {
    if fuel_cost == 0 {
        return Err(FuelError::ZeroFuelCost);
    }
    if state.fuel_on_board > tank_capacity {
        return Err(FuelError::FuelExceedsCapacity {
            fuel: state.fuel_on_board,
            capacity: tank_capacity,
        });
    }

    let scale = i64::from(FUEL_TANK_SCALE);
    let max_fit = i64::from(tank_capacity - state.fuel_on_board);
    let max_afford_units = i64::from(state.res_units / u32::from(fuel_cost));
    let min_fit = -i64::from(state.fuel_on_board);

    let mut incr = key.requested_increment();
    incr = incr.min(max_fit);
    incr = incr.min(max_afford_units * scale);
    incr = incr.max(min_fit);

    if incr == 0 {
        return Ok(FuelChange::Rejected);
    }

    // C integer division truncates toward zero, so a drain of less than one
    // whole unit refunds nothing.
    let cost = (incr / scale) * i64::from(fuel_cost);
    Ok(FuelChange::Apply(GaugeDelta {
        fuel: i16::try_from(incr).expect("increment is bounded by one bar"),
        res_units: i32::try_from(-cost).expect("cost is bounded by one bar of RU"),
    }))
}

impl FuelRuState {
    /// The fuel and RU effect of `DeltaSISGauges (0, delta.fuel,
    /// delta.res_units)`: fuel floors at zero and caps at `tank_capacity`
    /// (`sis.c:1042-1054`), RU floors at zero (`sis.c:1088-1092`).
    pub fn apply(self, delta: GaugeDelta, tank_capacity: u32) -> Result<Self, FuelError> {
        let fuel_change = i64::from(delta.fuel);
        let fuel = (i64::from(self.fuel_on_board) + fuel_change).clamp(0, i64::from(tank_capacity));

        let res_units = if delta.res_units < 0 {
            self.res_units
                .saturating_sub(delta.res_units.unsigned_abs())
        } else {
            self.res_units
                .checked_add(delta.res_units.unsigned_abs())
                .ok_or(FuelError::ResUnitsOverflow)?
        };

        Ok(Self {
            // The clamp above bounds the value by a u32 capacity.
            fuel_on_board: u32::try_from(fuel).expect("clamped to a u32 capacity"),
            res_units,
        })
    }
}

/// `DoOutfit` rounds the fuel to the nearest whole unit when the screen
/// opens (`outfit.c:702-706`): `(fuel + 50) / 100 * 100`. Half a unit rounds
/// up. The result of any fuel at or below a tank capacity stays at or below
/// it because capacities are whole multiples of 100.
pub fn round_fuel_on_outfit_entry(fuel_on_board: u32) -> Result<u32, FuelError> {
    let half = FUEL_TANK_SCALE >> 1;
    let rounded = fuel_on_board
        .checked_add(half)
        .ok_or(FuelError::FuelOverflow)?;
    Ok(rounded / FUEL_TANK_SCALE * FUEL_TANK_SCALE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COST: u8 = 20;

    fn state(fuel_on_board: u32, res_units: u32) -> FuelRuState {
        FuelRuState {
            fuel_on_board,
            res_units,
        }
    }

    fn press(s: FuelRuState, capacity: u32, key: FuelKey) -> (FuelRuState, Option<GaugeDelta>) {
        match change_fuel_quantity(s, capacity, COST, key).unwrap() {
            FuelChange::Rejected => (s, None),
            FuelChange::Apply(delta) => (s.apply(delta, capacity).unwrap(), Some(delta)),
        }
    }

    #[test]
    fn fifty_unit_purchases_fill_a_6000_tank_for_1000_ru() {
        // outfit.c:560-583 with FuelCost 20, 1000 raw fuel, 2500 RU, 6000 cap.
        let mut s = state(1000, 2500);
        for tap in 1..=50 {
            let (next, delta) = press(s, 6000, FuelKey::UnitUp);
            assert_eq!(
                delta,
                Some(GaugeDelta {
                    fuel: 100,
                    res_units: -20
                }),
                "tap {tap}"
            );
            s = next;
        }
        assert_eq!(s, state(6000, 1500));
    }

    #[test]
    fn the_51st_purchase_is_rejected_without_change() {
        let mut s = state(1000, 2500);
        for _ in 0..50 {
            s = press(s, 6000, FuelKey::UnitUp).0;
        }
        assert_eq!(
            change_fuel_quantity(s, 6000, COST, FuelKey::UnitUp),
            Ok(FuelChange::Rejected)
        );
        assert_eq!(
            change_fuel_quantity(s, 6000, COST, FuelKey::BarUp),
            Ok(FuelChange::Rejected)
        );
    }

    #[test]
    fn bar_purchase_moves_a_thousand_raw_for_200_ru() {
        let (s, delta) = press(state(1000, 2500), 6000, FuelKey::BarUp);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: 1000,
                res_units: -200
            })
        );
        assert_eq!(s, state(2000, 2300));
    }

    #[test]
    fn purchase_clamps_to_exact_remaining_capacity() {
        let (s, delta) = press(state(5500, 2500), 6000, FuelKey::BarUp);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: 500,
                res_units: -100
            })
        );
        assert_eq!(s, state(6000, 2400));
    }

    #[test]
    fn purchase_clamps_to_what_the_player_can_afford() {
        // 1500 RU at 20 RU per unit buys 75 units at most.
        let capacity = 161_000;
        let mut s = state(0, 1500);
        for _ in 0..7 {
            s = press(s, capacity, FuelKey::BarUp).0;
        }
        assert_eq!(s, state(7000, 100));
        let (s, delta) = press(s, capacity, FuelKey::BarUp);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: 500,
                res_units: -100
            })
        );
        assert_eq!(s, state(7500, 0));
    }

    #[test]
    fn purchase_with_less_than_one_unit_of_ru_is_rejected() {
        assert_eq!(
            change_fuel_quantity(state(1000, 19), 6000, COST, FuelKey::UnitUp),
            Ok(FuelChange::Rejected)
        );
        assert_eq!(
            change_fuel_quantity(state(1000, 0), 6000, COST, FuelKey::BarUp),
            Ok(FuelChange::Rejected)
        );
    }

    #[test]
    fn empty_tank_cannot_be_drained_and_full_tank_cannot_be_filled() {
        assert_eq!(
            change_fuel_quantity(state(0, 500), 6000, COST, FuelKey::UnitDown),
            Ok(FuelChange::Rejected)
        );
        assert_eq!(
            change_fuel_quantity(state(0, 500), 6000, COST, FuelKey::BarDown),
            Ok(FuelChange::Rejected)
        );
        assert_eq!(
            change_fuel_quantity(state(6000, 500), 6000, COST, FuelKey::BarUp),
            Ok(FuelChange::Rejected)
        );
    }

    #[test]
    fn drain_refunds_ru_with_a_positive_delta() {
        let (s, delta) = press(state(3000, 100), 6000, FuelKey::UnitDown);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: -100,
                res_units: 20
            })
        );
        assert_eq!(s, state(2900, 120));
        let (s, delta) = press(s, 6000, FuelKey::BarDown);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: -1000,
                res_units: 200
            })
        );
        assert_eq!(s, state(1900, 320));
    }

    #[test]
    fn drain_floors_at_the_fuel_on_board() {
        let (s, delta) = press(state(300, 0), 6000, FuelKey::BarDown);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: -300,
                res_units: 60
            })
        );
        assert_eq!(s, state(0, 60));
    }

    #[test]
    fn draining_a_partial_unit_refunds_nothing() {
        // -50 / 100 truncates to 0 in C, so the refund is 0 and the fuel
        // still goes.
        let (s, delta) = press(state(50, 10), 6000, FuelKey::UnitDown);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: -50,
                res_units: 0
            })
        );
        assert_eq!(s, state(0, 10));
    }

    #[test]
    fn purchase_that_ends_on_a_partial_unit_charges_whole_units_only() {
        // Capacity not on a unit boundary: 50 raw fits, charged 0 RU.
        let (s, delta) = press(state(5950, 100), 6000, FuelKey::UnitUp);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: 50,
                res_units: 0
            })
        );
        assert_eq!(s, state(6000, 100));
    }

    #[test]
    fn huge_ru_balance_does_not_truncate() {
        // C casts ResUnits / FuelCost to int and multiplies by 100, which
        // overflows here. The exact result is the plain one-unit purchase.
        let (s, delta) = press_with_cost(state(0, u32::MAX), 6000, 1, FuelKey::UnitUp);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: 100,
                res_units: -1
            })
        );
        assert_eq!(s, state(100, u32::MAX - 1));
    }

    fn press_with_cost(
        s: FuelRuState,
        capacity: u32,
        cost: u8,
        key: FuelKey,
    ) -> (FuelRuState, Option<GaugeDelta>) {
        match change_fuel_quantity(s, capacity, cost, key).unwrap() {
            FuelChange::Rejected => (s, None),
            FuelChange::Apply(delta) => (s.apply(delta, capacity).unwrap(), Some(delta)),
        }
    }

    #[test]
    fn largest_tank_largest_cost_stays_in_range() {
        // 161000 raw capacity, cost 255: one bar costs 2550 RU, fits i32.
        let (_, delta) = press_with_cost(state(0, u32::MAX), 161_000, 255, FuelKey::BarUp);
        assert_eq!(
            delta,
            Some(GaugeDelta {
                fuel: 1000,
                res_units: -2550
            })
        );
    }

    #[test]
    fn zero_fuel_cost_is_an_error_not_a_division_by_zero() {
        assert_eq!(
            change_fuel_quantity(state(0, 100), 6000, 0, FuelKey::UnitUp),
            Err(FuelError::ZeroFuelCost)
        );
    }

    #[test]
    fn fuel_above_capacity_is_an_error() {
        assert_eq!(
            change_fuel_quantity(state(6100, 100), 6000, COST, FuelKey::UnitDown),
            Err(FuelError::FuelExceedsCapacity {
                fuel: 6100,
                capacity: 6000
            })
        );
    }

    #[test]
    fn refund_overflow_is_an_error() {
        let delta = GaugeDelta {
            fuel: -100,
            res_units: 20,
        };
        assert_eq!(
            state(1000, u32::MAX - 5).apply(delta, 6000),
            Err(FuelError::ResUnitsOverflow)
        );
    }

    #[test]
    fn gauge_apply_floors_ru_and_fuel_at_zero_and_caps_fuel() {
        let pay = GaugeDelta {
            fuel: -500,
            res_units: -100,
        };
        assert_eq!(state(300, 40).apply(pay, 6000), Ok(state(0, 0)));
        let fill = GaugeDelta {
            fuel: 1000,
            res_units: 0,
        };
        assert_eq!(state(5500, 40).apply(fill, 6000), Ok(state(6000, 40)));
    }

    #[test]
    fn key_priority_follows_the_c_else_if_chain() {
        let all = MenuPulses {
            up: true,
            down: true,
            page_up: true,
            page_down: true,
            ..MenuPulses::default()
        };
        assert_eq!(FuelKey::from_pulses(all), Some(FuelKey::UnitUp));
        let no_up = MenuPulses { up: false, ..all };
        assert_eq!(FuelKey::from_pulses(no_up), Some(FuelKey::UnitDown));
        let page = MenuPulses {
            page_up: true,
            page_down: true,
            ..MenuPulses::default()
        };
        assert_eq!(FuelKey::from_pulses(page), Some(FuelKey::BarUp));
        let page_down = MenuPulses {
            page_down: true,
            ..MenuPulses::default()
        };
        assert_eq!(FuelKey::from_pulses(page_down), Some(FuelKey::BarDown));
        assert_eq!(FuelKey::from_pulses(MenuPulses::default()), None);
        let other = MenuPulses {
            left: true,
            right: true,
            select: true,
            cancel: true,
            ..MenuPulses::default()
        };
        assert_eq!(FuelKey::from_pulses(other), None);
    }

    #[test]
    fn entry_rounding_is_nearest_unit_with_half_up() {
        // outfit.c:702-706. This is nearest rounding, not the floor that a
        // `DeltaSISGauges (0, -(FuelOnBoard % FUEL_TANK_SCALE), 0)` variant
        // would give.
        assert_eq!(round_fuel_on_outfit_entry(0), Ok(0));
        assert_eq!(round_fuel_on_outfit_entry(49), Ok(0));
        assert_eq!(round_fuel_on_outfit_entry(50), Ok(100));
        assert_eq!(round_fuel_on_outfit_entry(149), Ok(100));
        assert_eq!(round_fuel_on_outfit_entry(150), Ok(200));
        assert_eq!(round_fuel_on_outfit_entry(1000), Ok(1000));
        assert_eq!(round_fuel_on_outfit_entry(5999), Ok(6000));
    }

    #[test]
    fn entry_rounding_never_leaves_the_tank() {
        for fuel in 0..=6000 {
            let rounded = round_fuel_on_outfit_entry(fuel).unwrap();
            assert!(rounded <= 6000, "{fuel} -> {rounded}");
            assert_eq!(rounded % 100, 0);
            assert!(rounded.abs_diff(fuel) <= 50);
        }
    }

    #[test]
    fn entry_rounding_reports_overflow() {
        assert_eq!(
            round_fuel_on_outfit_entry(u32::MAX),
            Err(FuelError::FuelOverflow)
        );
    }
}
