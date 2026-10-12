//! Flagship fuel and resource-unit precondition from the game's own state.
//!
//! `assert_ship_resources` compares `GLOBAL_SIS(FuelOnBoard)` and
//! `GLOBAL_SIS(ResUnits)` with the values a script expects. Those are the
//! fields `DeltaSISGauges` changes, so a Commander reward or an Outfit fuel
//! purchase is only visible here after the game itself applied it. The
//! assertion never writes either field and has no way to grant resources.
//!
//! The decision is pure: the live mode and values are passed in, so every
//! boundary is testable without a linked game. A mismatch is an explicit
//! error that names both the observed and the expected values.

use crate::automation::mode::GameMode;
use crate::automation::script::ShipResourcesAssertion;

/// The two values a fuel purchase or reward changes, as stored in `SIS_STATE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipResources {
    /// `SIS_STATE.FuelOnBoard`, raw fuel points (100 per displayed unit).
    pub fuel_on_board: u32,
    /// `SIS_STATE.ResUnits`.
    pub res_units: u32,
}

/// Why a resource assertion did not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShipResourcesError {
    /// The game exposes no flagship state to this build.
    Unavailable,
    /// No game is running, or the mode was ambiguous.
    NotInGame { observed: String },
    /// A live value differs from the expected one.
    Mismatch {
        observed: ShipResources,
        expected: ShipResourcesAssertion,
    },
}

/// A resource state that met every expectation of the assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedShipResources {
    pub observed: ShipResources,
    pub expected: ShipResourcesAssertion,
}

fn expectation(value: Option<u32>) -> String {
    value.map_or_else(|| "any".to_string(), |value| value.to_string())
}

fn label_fields(observed: ShipResources, expected: ShipResourcesAssertion) -> String {
    format!(
        "fuel={}:ru={}:expected_fuel={}:expected_ru={}",
        observed.fuel_on_board,
        observed.res_units,
        expectation(expected.fuel_on_board),
        expectation(expected.res_units),
    )
}

impl ShipResourcesError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self) -> String {
        match self {
            Self::Unavailable => "ship_resources_unavailable".to_string(),
            Self::NotInGame { observed } => {
                format!("ship_resources_wrong_mode:observed={observed}")
            }
            Self::Mismatch { observed, expected } => {
                format!(
                    "ship_resources_failed:{}",
                    label_fields(*observed, *expected)
                )
            }
        }
    }
}

impl VerifiedShipResources {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "ship_resources_verified:{}",
            label_fields(self.observed, self.expected)
        )
    }
}

/// Whether `label` is the verified record of exactly this assertion.
///
/// The label must carry the assertion's own expectations and observed values
/// that meet them, so a proof of one precondition never satisfies another.
#[must_use]
pub fn verified_label_satisfies(label: &str, expected: ShipResourcesAssertion) -> bool {
    let mut fields = label.split(':');
    if fields.next() != Some("ship_resources_verified") {
        return false;
    }
    let mut next = |key: &str| fields.next().and_then(|field| field.strip_prefix(key));
    let (Some(fuel), Some(ru), Some(expected_fuel), Some(expected_ru)) = (
        next("fuel="),
        next("ru="),
        next("expected_fuel="),
        next("expected_ru="),
    ) else {
        return false;
    };
    if fields.next().is_some() {
        return false;
    }
    let meets = |observed: &str, label_expected: &str, wanted: Option<u32>| match wanted {
        Some(wanted) => label_expected == wanted.to_string() && observed == wanted.to_string(),
        None => label_expected == "any" && observed.parse::<u32>().is_ok(),
    };
    meets(fuel, expected_fuel, expected.fuel_on_board) && meets(ru, expected_ru, expected.res_units)
}

/// Require the live flagship to hold exactly the expected values.
///
/// # Errors
///
/// Returns an error when no game is running, the game exposes no flagship
/// state, or an expected value differs from the live one.
pub fn assert_ship_resources(
    mode: Result<GameMode, String>,
    resources: Option<ShipResources>,
    expected: ShipResourcesAssertion,
) -> Result<VerifiedShipResources, ShipResourcesError> {
    match mode {
        Ok(running @ (GameMode::MainMenu | GameMode::Quit)) => {
            return Err(ShipResourcesError::NotInGame {
                observed: running.name().to_string(),
            })
        }
        Ok(_) => {}
        Err(ambiguous) => {
            return Err(ShipResourcesError::NotInGame {
                observed: ambiguous,
            })
        }
    }
    let observed = resources.ok_or(ShipResourcesError::Unavailable)?;
    let fuel_holds = expected
        .fuel_on_board
        .is_none_or(|wanted| wanted == observed.fuel_on_board);
    let ru_holds = expected
        .res_units
        .is_none_or(|wanted| wanted == observed.res_units);
    if fuel_holds && ru_holds {
        Ok(VerifiedShipResources { observed, expected })
    } else {
        Err(ShipResourcesError::Mismatch { observed, expected })
    }
}

/// The live flagship resources, when this build links the game.
#[must_use]
pub fn live_resources() -> Option<ShipResources> {
    crate::automation::input_ffi::ship_resources().map(|(fuel_on_board, res_units)| ShipResources {
        fuel_on_board,
        res_units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resources(fuel_on_board: u32, res_units: u32) -> Option<ShipResources> {
        Some(ShipResources {
            fuel_on_board,
            res_units,
        })
    }

    fn expect(fuel: Option<u32>, ru: Option<u32>) -> ShipResourcesAssertion {
        ShipResourcesAssertion {
            fuel_on_board: fuel,
            res_units: ru,
        }
    }

    fn solar() -> Result<GameMode, String> {
        Ok(GameMode::Solar)
    }

    #[test]
    fn a_new_game_starts_with_one_thousand_raw_fuel_and_no_resource_units() {
        let proof = assert_ship_resources(solar(), resources(1000, 0), expect(Some(1000), Some(0)))
            .unwrap();
        assert_eq!(
            proof.trace_label(),
            "ship_resources_verified:fuel=1000:ru=0:expected_fuel=1000:expected_ru=0"
        );
    }

    #[test]
    fn the_reward_and_a_full_tank_are_distinct_exact_states() {
        assert!(assert_ship_resources(
            GameMode::Starbase.into_ok(),
            resources(1000, 2500),
            expect(Some(1000), Some(2500))
        )
        .is_ok());
        assert!(assert_ship_resources(
            GameMode::Starbase.into_ok(),
            resources(6000, 1500),
            expect(Some(6000), Some(1500))
        )
        .is_ok());
    }

    #[test]
    fn one_unit_off_on_either_side_of_every_boundary_is_rejected() {
        for (fuel, ru) in [
            (5999, 1500),
            (6001, 1500),
            (6000, 1499),
            (6000, 1501),
            (0, 0),
            (u32::MAX, u32::MAX),
        ] {
            let error =
                assert_ship_resources(solar(), resources(fuel, ru), expect(Some(6000), Some(1500)))
                    .unwrap_err();
            assert!(
                matches!(error, ShipResourcesError::Mismatch { .. }),
                "{fuel},{ru}: {error:?}"
            );
            assert_eq!(
                error.trace_label(),
                format!(
                    "ship_resources_failed:fuel={fuel}:ru={ru}:expected_fuel=6000:expected_ru=1500"
                )
            );
        }
    }

    #[test]
    fn an_unconstrained_field_accepts_any_value_and_a_constrained_one_does_not() {
        let fuel_only = expect(Some(6000), None);
        assert!(assert_ship_resources(solar(), resources(6000, 0), fuel_only).is_ok());
        assert!(assert_ship_resources(solar(), resources(6000, u32::MAX), fuel_only).is_ok());
        assert!(assert_ship_resources(solar(), resources(5000, 1500), fuel_only).is_err());
        let ru_only = expect(None, Some(2500));
        assert!(assert_ship_resources(solar(), resources(0, 2500), ru_only).is_ok());
        assert!(assert_ship_resources(solar(), resources(1000, 2499), ru_only).is_err());
        assert_eq!(
            assert_ship_resources(solar(), resources(7, 2500), ru_only)
                .unwrap()
                .trace_label(),
            "ship_resources_verified:fuel=7:ru=2500:expected_fuel=any:expected_ru=2500"
        );
    }

    #[test]
    fn no_game_state_is_an_explicit_failure_not_a_pass() {
        assert_eq!(
            assert_ship_resources(solar(), None, expect(Some(0), Some(0))),
            Err(ShipResourcesError::Unavailable)
        );
        assert_eq!(
            ShipResourcesError::Unavailable.trace_label(),
            "ship_resources_unavailable"
        );
    }

    #[test]
    fn the_main_menu_quit_and_an_ambiguous_mode_never_read_as_a_game() {
        for mode in [
            Ok(GameMode::MainMenu),
            Ok(GameMode::Quit),
            Err("activity 0x1234 names no mode".to_string()),
        ] {
            let error =
                assert_ship_resources(mode, resources(0, 0), expect(Some(0), Some(0))).unwrap_err();
            assert!(
                matches!(error, ShipResourcesError::NotInGame { .. }),
                "{error:?}"
            );
            assert!(error
                .trace_label()
                .starts_with("ship_resources_wrong_mode:"));
        }
        assert_eq!(
            ShipResourcesError::NotInGame {
                observed: "main_menu".to_string()
            }
            .trace_label(),
            "ship_resources_wrong_mode:observed=main_menu"
        );
    }

    #[test]
    fn every_in_game_mode_may_be_asserted() {
        for mode in GameMode::ALL {
            let result = assert_ship_resources(Ok(mode), resources(1, 2), expect(Some(1), Some(2)));
            let in_game = !matches!(mode, GameMode::MainMenu | GameMode::Quit);
            assert_eq!(result.is_ok(), in_game, "{}", mode.name());
        }
    }

    #[test]
    fn a_verified_label_satisfies_only_its_own_assertion() {
        let both = expect(Some(6000), Some(1500));
        let label = "ship_resources_verified:fuel=6000:ru=1500:expected_fuel=6000:expected_ru=1500";
        assert!(verified_label_satisfies(label, both));
        assert!(!verified_label_satisfies(label, expect(Some(6000), None)));
        assert!(!verified_label_satisfies(
            label,
            expect(Some(6000), Some(1000))
        ));
        assert!(!verified_label_satisfies(
            label,
            expect(Some(5000), Some(1500))
        ));
        // A label whose observed value differs from its own expectation is not a proof.
        assert!(!verified_label_satisfies(
            "ship_resources_verified:fuel=5999:ru=1500:expected_fuel=6000:expected_ru=1500",
            both
        ));
        assert!(!verified_label_satisfies(
            "ship_resources_failed:fuel=6000:ru=1500:expected_fuel=6000:expected_ru=1500",
            both
        ));
        assert!(!verified_label_satisfies(
            "ship_resources_verified:fuel=6000:ru=1500:expected_fuel=6000:expected_ru=1500:x=1",
            both
        ));
        assert!(!verified_label_satisfies(
            "ship_resources_verified:fuel=6000",
            both
        ));
        assert!(verified_label_satisfies(
            "ship_resources_verified:fuel=3:ru=2500:expected_fuel=any:expected_ru=2500",
            expect(None, Some(2500))
        ));
        assert!(!verified_label_satisfies(
            "ship_resources_verified:fuel=3:ru=2500:expected_fuel=any:expected_ru=2500",
            expect(Some(3), Some(2500))
        ));
    }

    trait IntoOk {
        fn into_ok(self) -> Result<GameMode, String>;
    }

    impl IntoOk for GameMode {
        fn into_ok(self) -> Result<GameMode, String> {
            Ok(self)
        }
    }
}
