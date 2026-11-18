//! Shared deadline semantics for observational battle waits.
use super::*;

pub(super) fn reduce(
    action: &Action,
    state: &SchedulerState,
    sv: u64,
) -> Option<SchedulerTransition> {
    let max_ticks = match action {
        Action::WaitForBattleFrames(wait) => wait.max_ticks,
        Action::WaitForBattleShipLoss(wait) => wait.max_ticks,
        Action::WaitForPkunkRebirth(wait) => wait.max_ticks,
        _ => return None,
    };
    let mut new_state = SchedulerState {
        state_version: sv,
        ..*state
    };
    match state.phase {
        ActionPhase::WaitingForInput => {
            new_state.phase = ActionPhase::WaitingCondition {
                remaining: max_ticks.saturating_sub(1),
            }
        }
        ActionPhase::WaitingCondition { remaining: 0 } => {
            new_state.terminal = Some(TerminalOutcome::SemanticMismatch)
        }
        ActionPhase::WaitingCondition { remaining } => {
            new_state.phase = ActionPhase::WaitingCondition {
                remaining: remaining.saturating_sub(1),
            }
        }
        _ => return None,
    }
    Some(SchedulerTransition {
        new_state,
        effects: EffectPlan::none(),
    })
}
