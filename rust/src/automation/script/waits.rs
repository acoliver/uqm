//! Bounded wait admission and callback accounting.

use super::*;

pub(super) fn validate_wait_step(
    step: &Action,
    index: usize,
    path: &str,
    requirements: &mut StepRequirements,
) -> Option<Result<(), AutomationError>> {
    if let Err(reason) = validate_constraints(step) {
        return Some(Err(AutomationError::step(path, index, reason)));
    }
    let (max_ticks, field) = match step {
        Action::WaitForDispatch(wait) => (wait.max_ticks, "wait_for_dispatch"),
        Action::WaitForState(wait) => (wait.max_ticks, "wait_for_state"),
        Action::WaitForBattleFrames(wait) => (wait.max_ticks, "wait_for_battle_frames"),
        Action::WaitForPkunkRebirth(wait) => (wait.max_ticks, "wait_for_pkunk_rebirth"),
        Action::WaitForBattleShipLoss(wait) => (wait.max_ticks, "wait_for_battle_ship_loss"),
        Action::WaitForCommunicationReplay(wait) => {
            (wait.max_ticks, "wait_for_communication_replay")
        }
        Action::WaitForPlanetSideStart(wait) => (wait.max_ticks, "wait_for_planet_side_start"),
        Action::WaitForPlanetSideEnd(wait) => (wait.max_ticks, "wait_for_planet_side_end"),
        Action::SelectPlanetMenu(select) => (select.max_ticks, "select_planet_menu"),
        Action::PickBattleShip(pick) => (pick.max_ticks, "pick_battle_ship"),
        Action::SelectResponse(select) => (select.max_ticks, "select_response"),
        Action::SelectCommunicationResponse(select) => {
            (select.max_ticks, "select_communication_response")
        }
        Action::WaitForCommunicationResponses(wait) => {
            (wait.max_ticks, "wait_for_communication_responses")
        }
        Action::WaitForCommunicationEnd(wait) => (wait.max_ticks, "wait_for_communication_end"),
        _ => return None,
    };
    if max_ticks == 0 {
        return Some(Err(AutomationError::step(
            path,
            index,
            format!("{field} max_ticks must be positive"),
        )));
    }
    Some(requirements.add_bounded_wait(max_ticks, index, path))
}

fn validate_constraints(step: &Action) -> Result<(), String> {
    match step {
        Action::WaitForState(wait) => wait
            .predicate
            .validate()
            .map_err(|reason| format!("wait_for_state: {reason}")),
        Action::WaitForBattleFrames(wait) => {
            positive(wait.minimum, "wait_for_battle_frames minimum")
        }
        Action::WaitForBattleShipLoss(wait) => positive(
            u64::from(wait.minimum_ships),
            "wait_for_battle_ship_loss minimum_ships",
        ),
        Action::WaitForCommunicationResponses(wait) => positive(
            wait.minimum as u64,
            "wait_for_communication_responses minimum",
        ),
        Action::WaitForCommunicationEnd(wait) => positive(
            wait.minimum_completions,
            "wait_for_communication_end minimum_completions",
        ),
        _ => validate_selection(step),
    }
}

fn positive(value: u64, field: &str) -> Result<(), String> {
    if value == 0 {
        Err(format!("{field} must be positive"))
    } else {
        Ok(())
    }
}

fn validate_selection(step: &Action) -> Result<(), String> {
    match step {
        Action::WaitForPkunkRebirth(wait) if wait.side > 1 => {
            Err("Pkunk side must be 0 or 1".into())
        }
        Action::SelectResponse(select)
            if select.phrase.is_empty()
                || !select
                    .phrase
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') =>
        {
            Err("select_response phrase must be a nonempty string-table name".into())
        }
        Action::SelectCommunicationResponse(select) if select.index >= 8 => {
            Err("select_communication_response index must be 0..=7".into())
        }
        _ => Ok(()),
    }
}
