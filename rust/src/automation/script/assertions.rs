use super::*;

fn validate_collisions(
    assertion: &PlanetSideCollisionAssertion,
    index: usize,
    path: &str,
) -> Result<(), AutomationError> {
    if assertion.mineral_pickups == 0 && assertion.creature_hits == 0 && assertion.seam_hits == 0 {
        return Err(AutomationError::step(
            path,
            index,
            "assert_planet_side_collisions: at least one of mineral_pickups, creature_hits, seam_hits must be positive",
        ));
    }
    Ok(())
}

fn validate_minimum<T: Eq + From<u8>>(
    minimum: T,
    action: &str,
    index: usize,
    path: &str,
) -> Result<(), AutomationError> {
    if minimum == T::from(0) {
        return Err(AutomationError::step(
            path,
            index,
            format!("{action}: minimum must be greater than zero"),
        ));
    }
    Ok(())
}

/// Validate captures, assertions, and terminal steps.
pub(super) fn validate_assertion_step(
    step: &Action,
    index: usize,
    path: &str,
    requirements: &mut StepRequirements,
) -> Result<(), AutomationError> {
    match step {
        Action::ConfigureParityMelee(request) => request
            .validate()
            .map_err(|error| AutomationError::step(path, index, error))?,
        Action::RunParityCorpus(request) => request
            .validate()
            .map_err(|error| AutomationError::step(path, index, error))?,
        Action::Capture(capture) => validate_capture(capture, index, path)?,
        Action::AssertActivity(assertion) => validate_activity(assertion, index, path)?,
        Action::AssertCommunicationResponses(assertion) => {
            validate_minimum(
                assertion.minimum,
                "assert_communication_responses",
                index,
                path,
            )?;
        }
        Action::AssertBattleFrames(assertion) => {
            validate_minimum(assertion.minimum, "assert_battle_frames", index, path)?;
        }
        Action::AssertHyperspaceDisplacement(assertion) => {
            if assertion.minimum_logical_units == 0 {
                return Err(AutomationError::step(
                    path,
                    index,
                    "assert_hyperspace_displacement: minimum_logical_units must be greater than zero",
                ));
            }
        }
        Action::AssertShipElement(assertion) => {
            validate_minimum(assertion.minimum, "assert_ship_element", index, path)?;
        }
        Action::AssertPlanetSideCollisions(assertion) => {
            validate_collisions(assertion, index, path)?;
        }
        Action::AssertMainMenuTransition(transition) => {
            let from = parse_menu_item(&transition.from, index, path)?;
            let to = parse_menu_item(&transition.to, index, path)?;
            requirements
                .transitions
                .push(MainMenuTransition::new(from, to));
        }
        _ => return validate_live_state(step, index, path),
    }
    Ok(())
}

fn validate_live_state(step: &Action, index: usize, path: &str) -> Result<(), AutomationError> {
    match step {
        Action::AssertShipResources(assertion) => {
            if assertion.fuel_on_board.is_none() && assertion.res_units.is_none() {
                return Err(AutomationError::step(
                    path,
                    index,
                    "assert_ship_resources: state fuel_on_board, res_units or both",
                ));
            }
        }
        Action::AssertState(predicate) => {
            if let Err(reason) = predicate.validate() {
                return Err(AutomationError::step(
                    path,
                    index,
                    format!("assert_state: {reason}"),
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_capture(
    capture: &CaptureStep,
    index: usize,
    path: &str,
) -> Result<(), AutomationError> {
    if !is_valid_label(&capture.label) {
        return Err(AutomationError::step(
                    path,
                    index,
                    format!(
                        "capture label '{}' is invalid: must be nonempty and reject separators, '..', and control characters",
                        capture.label
                    ),
                ));
    }
    Ok(())
}
fn validate_activity(
    assertion: &ActivityAssertion,
    index: usize,
    path: &str,
) -> Result<(), AutomationError> {
    // REQ-SCRIPT-004: equals & !mask == 0
    if (assertion.equals & !assertion.mask) != 0 {
        return Err(AutomationError::step(
            path,
            index,
            format!(
                "assert_activity: equals (0x{:04X}) has bits outside mask (0x{:04X})",
                assertion.equals, assertion.mask
            ),
        ));
    }
    Ok(())
}
