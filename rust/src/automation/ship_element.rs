//! The flagship's element cargo, read from the game's own state.
//!
//! `assert_ship_element` compares one entry of `GLOBAL_SIS (ElementAmounts)`
//! with a minimum. The lander adds its hold to those amounts only when it
//! returns to the ship, so a passing assertion proves minerals really arrived
//! aboard. The assertion only reads: nothing here grants an element.
//!
//! The decision is pure: the live mode and amounts are passed in, so every
//! boundary is testable without a linked game.

use crate::automation::mode::GameMode;
use crate::automation::script::ShipElementAssertion;
use crate::planet_side::cargo::NUM_ELEMENT_CATEGORIES;

/// `SIS_STATE.ElementAmounts`, indexed by element category.
pub type ElementAmounts = [u16; NUM_ELEMENT_CATEGORIES];

/// Why an element assertion did not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShipElementError {
    /// The game exposes no flagship state to this build.
    Unavailable,
    /// No game is running, or the mode was ambiguous.
    NotInGame { observed: String },
    /// The flagship holds fewer units than the assertion requires.
    Below {
        assertion: ShipElementAssertion,
        amount: u16,
    },
}

/// An element amount that met the assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedShipElement {
    pub assertion: ShipElementAssertion,
    pub amount: u16,
}

impl ShipElementError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self) -> String {
        match self {
            Self::Unavailable => "ship_element_unavailable".to_string(),
            Self::NotInGame { observed } => format!("ship_element_wrong_mode:observed={observed}"),
            Self::Below { assertion, amount } => format!(
                "ship_element_failed:category={}:amount={amount}:minimum={}",
                assertion.category.name(),
                assertion.minimum
            ),
        }
    }
}

impl VerifiedShipElement {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "ship_element_verified:category={}:amount={}:minimum={}",
            self.assertion.category.name(),
            self.amount,
            self.assertion.minimum
        )
    }
}

/// Require the live flagship to hold at least the asserted units.
///
/// # Errors
///
/// Returns an error when no game is running, the game exposes no flagship
/// state, or the flagship holds fewer units than the assertion requires.
pub fn assert_ship_element(
    mode: Result<GameMode, String>,
    amounts: Option<ElementAmounts>,
    assertion: ShipElementAssertion,
) -> Result<VerifiedShipElement, ShipElementError> {
    match mode {
        Ok(running @ (GameMode::MainMenu | GameMode::Quit)) => {
            return Err(ShipElementError::NotInGame {
                observed: running.name().to_string(),
            })
        }
        Ok(_) => {}
        Err(ambiguous) => {
            return Err(ShipElementError::NotInGame {
                observed: ambiguous,
            })
        }
    }
    let amount = amounts.ok_or(ShipElementError::Unavailable)?[assertion.category.index()];
    if amount >= assertion.minimum {
        Ok(VerifiedShipElement { assertion, amount })
    } else {
        Err(ShipElementError::Below { assertion, amount })
    }
}

/// The live flagship element amounts, when this build links the game.
#[must_use]
pub fn live_amounts() -> Option<ElementAmounts> {
    crate::automation::input_ffi::ship_element_amounts()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::cargo::ElementCategory;

    fn radioactive(minimum: u16) -> ShipElementAssertion {
        ShipElementAssertion {
            category: ElementCategory::Radioactive,
            minimum,
        }
    }

    fn holding(radioactive_units: u16) -> Option<ElementAmounts> {
        let mut amounts = [9; NUM_ELEMENT_CATEGORIES];
        amounts[ElementCategory::Radioactive.index()] = radioactive_units;
        Some(amounts)
    }

    #[test]
    fn enough_units_of_the_category_verify() {
        let proof =
            assert_ship_element(Ok(GameMode::Starbase), holding(2), radioactive(1)).unwrap();
        assert_eq!(
            proof.trace_label(),
            "ship_element_verified:category=radioactive:amount=2:minimum=1"
        );
    }

    #[test]
    fn only_the_named_category_counts() {
        let error =
            assert_ship_element(Ok(GameMode::Solar), holding(0), radioactive(1)).unwrap_err();
        assert_eq!(
            error.trace_label(),
            "ship_element_failed:category=radioactive:amount=0:minimum=1"
        );
    }

    #[test]
    fn the_minimum_is_inclusive() {
        assert!(assert_ship_element(Ok(GameMode::Solar), holding(5), radioactive(5)).is_ok());
        assert!(assert_ship_element(Ok(GameMode::Solar), holding(4), radioactive(5)).is_err());
    }

    #[test]
    fn a_missing_game_or_flagship_is_an_explicit_error() {
        assert_eq!(
            assert_ship_element(Ok(GameMode::MainMenu), holding(9), radioactive(1)),
            Err(ShipElementError::NotInGame {
                observed: "main_menu".to_string()
            })
        );
        assert_eq!(
            assert_ship_element(Err("ambiguous".to_string()), holding(9), radioactive(1))
                .unwrap_err()
                .trace_label(),
            "ship_element_wrong_mode:observed=ambiguous"
        );
        assert_eq!(
            assert_ship_element(Ok(GameMode::Solar), None, radioactive(1))
                .unwrap_err()
                .trace_label(),
            "ship_element_unavailable"
        );
    }
}
