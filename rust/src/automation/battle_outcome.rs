//! Super Melee ship losses from the game's own per-side fleet counters.
//!
//! `battle_counter[side]` is the number of ships each side still has. The
//! game decrements it when a ship dies (`tactrans.c`) and Super Melee never
//! raises it again, so a drop since the first sample of a battle is a ship
//! the battle really destroyed. `wait_for_battle_ship_loss` waits for that
//! drop and records both fleets in the trace.
//!
//! The decision is pure: the live mode, frame counter and fleet sizes are
//! passed in, so every boundary is testable without a linked game.

use crate::automation::mode::GameMode;

/// Ships left on each side, `battle_counter[0]` and `battle_counter[1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleFleets {
    pub player_one: u8,
    pub player_two: u8,
}

impl BattleFleets {
    fn lost_since(self, baseline: Self) -> u32 {
        u32::from(baseline.player_one.saturating_sub(self.player_one))
            + u32::from(baseline.player_two.saturating_sub(self.player_two))
    }
}

/// Why a ship-loss observation could not be made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleLossError {
    /// The game exposes no fleet counters to this build.
    Unavailable,
    /// The first sample of the wait was not taken inside a battle.
    NotInBattle { observed: String },
}

impl BattleLossError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self) -> String {
        match self {
            Self::Unavailable => "battle_fleets_unavailable".to_string(),
            Self::NotInBattle { observed } => {
                format!("battle_ship_loss_wrong_mode:observed={observed}")
            }
        }
    }
}

/// A battle that has destroyed at least the requested number of ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedShipLoss {
    pub baseline: BattleFleets,
    pub now: BattleFleets,
    pub lost: u32,
    pub minimum: u32,
}

impl VerifiedShipLoss {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "battle_ship_loss_reached:lost={}:minimum={}:start={},{}:left={},{}",
            self.lost,
            self.minimum,
            self.baseline.player_one,
            self.baseline.player_two,
            self.now.player_one,
            self.now.player_two
        )
    }
}

/// Holds the fleet sizes seen at the start of the battle being watched.
#[derive(Debug, Default)]
pub struct BattleLossTracker {
    /// The first sample of the current battle and the frame count it was taken at.
    baseline: Option<(BattleFleets, u64)>,
}

impl BattleLossTracker {
    /// Sample the fleets once. `Ok(None)` means the battle has not yet
    /// destroyed `minimum` ships.
    ///
    /// The first sample must be taken inside a battle. A frame counter lower
    /// than the one the baseline was taken at means the battle restarted, so
    /// the baseline is retaken and a previous battle's losses never count.
    ///
    /// # Errors
    ///
    /// Returns an error when the game exposes no fleet counters or the first
    /// sample is not inside a battle.
    pub fn observe(
        &mut self,
        mode: Result<GameMode, String>,
        frame: u64,
        fleets: Option<BattleFleets>,
        minimum: u32,
    ) -> Result<Option<VerifiedShipLoss>, BattleLossError> {
        let now = fleets.ok_or(BattleLossError::Unavailable)?;
        let restarted = self
            .baseline
            .is_some_and(|(_, baseline_frame)| frame < baseline_frame);
        if self.baseline.is_none() || restarted {
            match mode {
                Ok(GameMode::Battle) => self.baseline = Some((now, frame)),
                Ok(other) => {
                    return Err(BattleLossError::NotInBattle {
                        observed: other.name().to_string(),
                    })
                }
                Err(ambiguous) => {
                    return Err(BattleLossError::NotInBattle {
                        observed: ambiguous,
                    })
                }
            }
        }
        let Some((baseline, _)) = self.baseline else {
            return Ok(None);
        };
        let lost = now.lost_since(baseline);
        Ok((lost >= minimum).then_some(VerifiedShipLoss {
            baseline,
            now,
            lost,
            minimum,
        }))
    }
}

/// The live fleet sizes, when this build links the game.
#[must_use]
pub fn live_fleets() -> Option<BattleFleets> {
    crate::automation::input_ffi::battle_fleets().map(|(player_one, player_two)| BattleFleets {
        player_one,
        player_two,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleets(player_one: u8, player_two: u8) -> Option<BattleFleets> {
        Some(BattleFleets {
            player_one,
            player_two,
        })
    }

    fn battle() -> Result<GameMode, String> {
        Ok(GameMode::Battle)
    }

    #[test]
    fn a_battle_with_every_ship_alive_has_lost_nothing() {
        let mut tracker = BattleLossTracker::default();
        assert_eq!(tracker.observe(battle(), 40, fleets(14, 14), 1), Ok(None));
        assert_eq!(tracker.observe(battle(), 900, fleets(14, 14), 1), Ok(None));
    }

    #[test]
    fn a_destroyed_ship_on_either_side_is_a_recorded_loss() {
        for (after, who) in [((13, 14), "13,14"), ((14, 13), "14,13")] {
            let mut tracker = BattleLossTracker::default();
            tracker.observe(battle(), 40, fleets(14, 14), 1).unwrap();
            let proof = tracker
                .observe(battle(), 700, fleets(after.0, after.1), 1)
                .unwrap()
                .unwrap();
            assert_eq!(proof.lost, 1);
            assert_eq!(
                proof.trace_label(),
                format!("battle_ship_loss_reached:lost=1:minimum=1:start=14,14:left={who}")
            );
        }
    }

    #[test]
    fn the_requested_number_of_ships_must_be_lost() {
        let mut tracker = BattleLossTracker::default();
        tracker.observe(battle(), 40, fleets(3, 3), 2).unwrap();
        assert_eq!(tracker.observe(battle(), 100, fleets(2, 3), 2), Ok(None));
        let proof = tracker
            .observe(battle(), 200, fleets(2, 2), 2)
            .unwrap()
            .unwrap();
        assert_eq!(proof.lost, 2);
    }

    #[test]
    fn a_battle_that_ended_after_the_baseline_still_reports_its_result() {
        let mut tracker = BattleLossTracker::default();
        tracker.observe(battle(), 40, fleets(1, 1), 1).unwrap();
        let proof = tracker
            .observe(Ok(GameMode::MainMenu), 900, fleets(1, 0), 1)
            .unwrap()
            .unwrap();
        assert_eq!(proof.now, fleets(1, 0).unwrap());
    }

    #[test]
    fn a_restarted_battle_does_not_inherit_the_last_battles_losses() {
        let mut tracker = BattleLossTracker::default();
        tracker.observe(battle(), 400, fleets(5, 5), 1).unwrap();
        tracker.observe(battle(), 600, fleets(4, 5), 1).unwrap();
        // The frame counter dropped, so this is a new battle with fresh fleets.
        assert_eq!(tracker.observe(battle(), 3, fleets(8, 8), 1), Ok(None));
        assert_eq!(tracker.observe(battle(), 50, fleets(8, 8), 1), Ok(None));
    }

    #[test]
    fn a_first_sample_outside_a_battle_is_rejected() {
        let mut tracker = BattleLossTracker::default();
        let error = tracker
            .observe(Ok(GameMode::MainMenu), 0, fleets(0, 0), 1)
            .unwrap_err();
        assert_eq!(
            error.trace_label(),
            "battle_ship_loss_wrong_mode:observed=main_menu"
        );
        assert!(matches!(
            tracker.observe(Err("ambiguous".to_string()), 0, fleets(0, 0), 1),
            Err(BattleLossError::NotInBattle { .. })
        ));
    }

    #[test]
    fn missing_counters_are_reported_not_guessed() {
        let mut tracker = BattleLossTracker::default();
        let error = tracker.observe(battle(), 10, None, 1).unwrap_err();
        assert_eq!(error, BattleLossError::Unavailable);
        assert_eq!(error.trace_label(), "battle_fleets_unavailable");
    }
}
