//! A verified turn of the hyperspace flagship.
//!
//! `turn_flagship` holds a turn key until the live `STARSHIP.ShipFacing` has
//! changed by exactly the requested number of the sixteen facings. The ship
//! turns one facing per `turn_wait` expiry (`ship.c`), and how many input
//! ticks that takes is not a constant a script can guess: a 60 tick hold
//! turned the flagship five facings, not four. So the script states the
//! outcome, and this tracker only reads the facing that the game reports.
//!
//! The tracker is pure: the live mode and facing are passed in, so each
//! decision is testable without a linked game. It never writes game state.

use crate::automation::mode::GameMode;
use crate::automation::script::TurnDirection;

/// `NORMALIZE_FACING` wraps at the sixteen facings of a ship (`units.h`).
pub const FACINGS: u8 = 16;

/// Why a turn could not be verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlagshipTurnError {
    /// The game exposes no flagship facing to this build, or no flagship is
    /// in flight.
    FacingUnavailable,
    /// The game was not in the hyperspace starmap, or its mode was ambiguous.
    NotHyperspace { observed: String },
    /// The facing is not one of the sixteen.
    InvalidFacing { facing: u8 },
    /// The flagship passed the requested number of facings.
    Overshot {
        direction: TurnDirection,
        wanted: u8,
        turned: u8,
    },
}

impl FlagshipTurnError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self) -> String {
        match self {
            Self::FacingUnavailable => "flagship_facing_unavailable".to_string(),
            Self::NotHyperspace { observed } => {
                format!("flagship_turn_wrong_mode:observed={observed}")
            }
            Self::InvalidFacing { facing } => {
                format!("flagship_turn_invalid_facing:facing={facing}")
            }
            Self::Overshot {
                direction,
                wanted,
                turned,
            } => format!(
                "flagship_turn_overshot:direction={}:facings={wanted}:turned={turned}",
                direction.name()
            ),
        }
    }
}

/// A turn that reached its requested number of facings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedTurn {
    pub direction: TurnDirection,
    pub facings: u8,
    pub from: u8,
    pub to: u8,
}

impl VerifiedTurn {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "flagship_turn_verified:direction={}:facings={}:from={}:to={}",
            self.direction.name(),
            self.facings,
            self.from,
            self.to
        )
    }
}

/// What the flagship should do after one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnProgress {
    /// Keep the turn key down.
    Turning { turned: u8 },
    /// The requested facings were reached: release the key.
    Complete(VerifiedTurn),
}

/// Holds the facing at which the current turn began.
#[derive(Debug, Default)]
pub struct FlagshipTurnTracker {
    start: Option<u8>,
}

impl FlagshipTurnTracker {
    /// Forget the turn in progress, so the next observation starts a new one.
    pub fn finish(&mut self) {
        self.start = None;
    }

    /// Observe the flagship once. The first observation of a turn fixes where
    /// it started. A failure ends the turn.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is not in hyperspace, reports no valid
    /// facing, or the flagship turned past `facings`.
    pub fn observe(
        &mut self,
        mode: Result<GameMode, String>,
        facing: Option<u8>,
        direction: TurnDirection,
        facings: u8,
    ) -> Result<TurnProgress, FlagshipTurnError> {
        let result = self.decide(mode, facing, direction, facings);
        if !matches!(result, Ok(TurnProgress::Turning { .. })) {
            self.start = None;
        }
        result
    }

    fn decide(
        &mut self,
        mode: Result<GameMode, String>,
        facing: Option<u8>,
        direction: TurnDirection,
        facings: u8,
    ) -> Result<TurnProgress, FlagshipTurnError> {
        match mode {
            Ok(GameMode::Starmap) => {}
            Ok(other) => {
                return Err(FlagshipTurnError::NotHyperspace {
                    observed: format!("{other:?}"),
                })
            }
            Err(observed) => return Err(FlagshipTurnError::NotHyperspace { observed }),
        }
        let now = facing.ok_or(FlagshipTurnError::FacingUnavailable)?;
        if now >= FACINGS {
            return Err(FlagshipTurnError::InvalidFacing { facing: now });
        }
        let from = *self.start.get_or_insert(now);
        let turned = match direction {
            TurnDirection::Right => (now + FACINGS - from) % FACINGS,
            TurnDirection::Left => (from + FACINGS - now) % FACINGS,
        };
        match turned.cmp(&facings) {
            std::cmp::Ordering::Less => Ok(TurnProgress::Turning { turned }),
            std::cmp::Ordering::Equal => Ok(TurnProgress::Complete(VerifiedTurn {
                direction,
                facings,
                from,
                to: now,
            })),
            std::cmp::Ordering::Greater => Err(FlagshipTurnError::Overshot {
                direction,
                wanted: facings,
                turned,
            }),
        }
    }
}

/// The live flagship's facing, or `None` when the game exposes none.
#[must_use]
pub fn live_facing() -> Option<u8> {
    crate::automation::input_ffi::hyperspace_facing()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hyperspace() -> Result<GameMode, String> {
        Ok(GameMode::Starmap)
    }

    /// Feed `facings` into a fresh tracker until it stops asking to turn.
    fn run(
        direction: TurnDirection,
        wanted: u8,
        facings: &[u8],
    ) -> Result<TurnProgress, FlagshipTurnError> {
        let mut tracker = FlagshipTurnTracker::default();
        let mut last = Ok(TurnProgress::Turning { turned: 0 });
        for &facing in facings {
            last = tracker.observe(hyperspace(), Some(facing), direction, wanted);
            if !matches!(last, Ok(TurnProgress::Turning { .. })) {
                break;
            }
        }
        last
    }

    #[test]
    fn a_right_turn_completes_at_exactly_the_requested_facings() {
        let progress = run(TurnDirection::Right, 4, &[2, 2, 3, 3, 4, 5, 6]).unwrap();
        assert_eq!(
            progress,
            TurnProgress::Complete(VerifiedTurn {
                direction: TurnDirection::Right,
                facings: 4,
                from: 2,
                to: 6
            })
        );
    }

    #[test]
    fn a_turn_still_short_of_its_facings_keeps_turning() {
        assert_eq!(
            run(TurnDirection::Right, 4, &[2, 3, 4, 5]),
            Ok(TurnProgress::Turning { turned: 3 })
        );
    }

    #[test]
    fn turns_wrap_around_the_sixteen_facings_in_both_directions() {
        let right = run(TurnDirection::Right, 4, &[14, 15, 0, 1, 2]).unwrap();
        assert!(matches!(right, TurnProgress::Complete(turn) if turn.to == 2 && turn.from == 14));
        let left = run(TurnDirection::Left, 4, &[1, 0, 15, 14, 13]).unwrap();
        assert!(matches!(left, TurnProgress::Complete(turn) if turn.to == 13 && turn.from == 1));
    }

    #[test]
    fn five_facings_where_four_were_requested_is_an_overshoot() {
        let error = run(TurnDirection::Right, 4, &[0, 5]).unwrap_err();
        assert_eq!(
            error,
            FlagshipTurnError::Overshot {
                direction: TurnDirection::Right,
                wanted: 4,
                turned: 5
            }
        );
        assert_eq!(
            error.trace_label(),
            "flagship_turn_overshot:direction=right:facings=4:turned=5"
        );
    }

    #[test]
    fn turning_the_other_way_is_an_overshoot_not_a_completion() {
        assert!(matches!(
            run(TurnDirection::Right, 4, &[5, 4]),
            Err(FlagshipTurnError::Overshot { turned: 15, .. })
        ));
    }

    #[test]
    fn a_finished_turn_starts_the_next_one_from_the_new_facing() {
        let mut tracker = FlagshipTurnTracker::default();
        for facing in [0, 1, 2, 3, 4] {
            let _ = tracker.observe(hyperspace(), Some(facing), TurnDirection::Right, 4);
        }
        let next = tracker
            .observe(hyperspace(), Some(4), TurnDirection::Right, 4)
            .unwrap();
        assert_eq!(next, TurnProgress::Turning { turned: 0 });
    }

    #[test]
    fn the_wrong_mode_and_a_missing_or_invalid_facing_are_rejected() {
        let mut tracker = FlagshipTurnTracker::default();
        assert!(matches!(
            tracker.observe(Ok(GameMode::MainMenu), Some(0), TurnDirection::Right, 4),
            Err(FlagshipTurnError::NotHyperspace { .. })
        ));
        assert!(matches!(
            tracker.observe(Err("ambiguous".into()), Some(0), TurnDirection::Right, 4),
            Err(FlagshipTurnError::NotHyperspace { .. })
        ));
        assert_eq!(
            tracker.observe(hyperspace(), None, TurnDirection::Right, 4),
            Err(FlagshipTurnError::FacingUnavailable)
        );
        assert_eq!(
            tracker.observe(hyperspace(), Some(16), TurnDirection::Right, 4),
            Err(FlagshipTurnError::InvalidFacing { facing: 16 })
        );
    }

    #[test]
    fn the_proof_label_is_stable() {
        let proof = VerifiedTurn {
            direction: TurnDirection::Left,
            facings: 4,
            from: 3,
            to: 15,
        };
        assert_eq!(
            proof.trace_label(),
            "flagship_turn_verified:direction=left:facings=4:from=3:to=15"
        );
    }
}
