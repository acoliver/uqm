//! Hyperspace displacement proof from the game's own logical position.
//!
//! `record_hyperspace_position` snapshots `GLOBAL_SIS(log_x/log_y)` and
//! `assert_hyperspace_displacement` requires the ship to have moved at least a
//! script-chosen distance since that snapshot. Those are the authoritative
//! coordinates that `hyper.c` updates when the flagship moves, not the HUD
//! text and not the solar-system ship stamp used for departure.
//!
//! The tracker is pure: the live mode and position are passed in, so the
//! decisions are testable without a linked game. A snapshot is consumed by the
//! assertion that uses it, so one leg of flight can never satisfy a later leg.

use crate::automation::mode::GameMode;

/// Logical hyperspace coordinates as stored in `SIS_STATE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HyperspacePosition {
    pub log_x: i32,
    pub log_y: i32,
}

/// Why a position record or displacement assertion did not hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlightPositionError {
    /// The game exposes no logical position to this build.
    PositionUnavailable,
    /// The game was not in the hyperspace starmap, or its mode was ambiguous.
    NotHyperspace { observed: String },
    /// The assertion ran with no snapshot from the same flight leg.
    NoBaseline,
    /// The restore assertion ran with no saved position to compare with.
    NoSavedPosition,
    /// The loaded position differs from the one recorded before the save.
    NotRestored {
        saved: HyperspacePosition,
        observed: HyperspacePosition,
    },
    /// The ship moved farther than the allowed distance.
    Moved {
        from: HyperspacePosition,
        to: HyperspacePosition,
        moved: u64,
        allowed: u64,
    },
    /// The ship moved less than the required distance.
    Insufficient {
        from: HyperspacePosition,
        to: HyperspacePosition,
        moved: u64,
        required: u64,
    },
}

impl FlightPositionError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self) -> String {
        match self {
            Self::PositionUnavailable => "hyperspace_position_unavailable".to_string(),
            Self::NotHyperspace { observed } => {
                format!("hyperspace_position_wrong_mode:observed={observed}")
            }
            Self::NoBaseline => "hyperspace_displacement_no_baseline".to_string(),
            Self::NoSavedPosition => "hyperspace_position_restore_no_saved_position".to_string(),
            Self::NotRestored { saved, observed } => format!(
                "hyperspace_position_restore_failed:saved={},{}:observed={},{}",
                saved.log_x, saved.log_y, observed.log_x, observed.log_y
            ),
            Self::Moved {
                from,
                to,
                moved,
                allowed,
            } => format!(
                "hyperspace_stationary_failed:from={},{}:to={},{}:moved={moved}:maximum={allowed}",
                from.log_x, from.log_y, to.log_x, to.log_y
            ),
            Self::Insufficient {
                from,
                to,
                moved,
                required,
            } => format!(
                "hyperspace_displacement_failed:from={},{}:to={},{}:moved={moved}:minimum={required}",
                from.log_x, from.log_y, to.log_x, to.log_y
            ),
        }
    }
}

/// A displacement that met its minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedDisplacement {
    pub from: HyperspacePosition,
    pub to: HyperspacePosition,
    pub moved: u64,
    pub required: u64,
}

impl VerifiedDisplacement {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "hyperspace_displacement_verified:from={},{}:to={},{}:moved={}:minimum={}",
            self.from.log_x,
            self.from.log_y,
            self.to.log_x,
            self.to.log_y,
            self.moved,
            self.required
        )
    }
}

/// A displacement that stayed within its maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedStationary {
    pub from: HyperspacePosition,
    pub to: HyperspacePosition,
    pub moved: u64,
    pub allowed: u64,
}

impl VerifiedStationary {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "hyperspace_stationary_verified:from={},{}:to={},{}:moved={}:maximum={}",
            self.from.log_x,
            self.from.log_y,
            self.to.log_x,
            self.to.log_y,
            self.moved,
            self.allowed
        )
    }
}

/// The semantic trace label that records a snapshot.
#[must_use]
pub fn recorded_trace_label(position: HyperspacePosition) -> String {
    format!(
        "hyperspace_position_recorded:x={}:y={}",
        position.log_x, position.log_y
    )
}

/// The semantic trace label that records the position kept for a later load.
#[must_use]
pub fn saved_trace_label(position: HyperspacePosition) -> String {
    format!(
        "hyperspace_position_saved:x={}:y={}",
        position.log_x, position.log_y
    )
}

/// The semantic trace label that proves a loaded game is where it was saved.
///
/// Both coordinate pairs are spelled out so a reader of the trace alone can see
/// that they are equal.
#[must_use]
pub fn restored_trace_label(saved: HyperspacePosition, observed: HyperspacePosition) -> String {
    format!(
        "hyperspace_position_restored:x={}:y={}:saved_x={}:saved_y={}",
        observed.log_x, observed.log_y, saved.log_x, saved.log_y
    )
}

/// Holds the snapshots between a record step and the assertion that uses it.
///
/// The displacement baseline and the position kept for a load are separate so
/// a verified thrust leg can happen between the save and the load without
/// consuming the position the load must restore.
#[derive(Debug, Default)]
pub struct FlightPositionTracker {
    baseline: Option<HyperspacePosition>,
    saved: Option<HyperspacePosition>,
}

impl FlightPositionTracker {
    /// Snapshot the ship. A failed record leaves no snapshot behind.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is not in hyperspace or exposes no
    /// position.
    pub fn record(
        &mut self,
        mode: Result<GameMode, String>,
        position: Option<HyperspacePosition>,
    ) -> Result<HyperspacePosition, FlightPositionError> {
        self.baseline = None;
        let position = require_hyperspace(mode, position)?;
        self.baseline = Some(position);
        Ok(position)
    }

    /// Keep the ship's position for a later `assert_restored`. A failed record
    /// leaves nothing behind.
    ///
    /// # Errors
    ///
    /// Returns an error when the game is not in hyperspace or exposes no
    /// position.
    pub fn record_saved(
        &mut self,
        mode: Result<GameMode, String>,
        position: Option<HyperspacePosition>,
    ) -> Result<HyperspacePosition, FlightPositionError> {
        self.saved = None;
        let position = require_hyperspace(mode, position)?;
        self.saved = Some(position);
        Ok(position)
    }

    /// Require the ship to be exactly where `record_saved` kept it, consuming
    /// the kept position whether or not the requirement holds.
    ///
    /// # Errors
    ///
    /// Returns an error when nothing was kept, the game is not in hyperspace,
    /// or the coordinates differ on either axis.
    pub fn assert_restored(
        &mut self,
        mode: Result<GameMode, String>,
        position: Option<HyperspacePosition>,
    ) -> Result<(HyperspacePosition, HyperspacePosition), FlightPositionError> {
        let saved = self
            .saved
            .take()
            .ok_or(FlightPositionError::NoSavedPosition)?;
        let observed = require_hyperspace(mode, position)?;
        if observed != saved {
            return Err(FlightPositionError::NotRestored { saved, observed });
        }
        Ok((saved, observed))
    }

    /// Require at least `minimum` logical units of movement since the snapshot,
    /// consuming the snapshot whether or not the requirement holds.
    ///
    /// # Errors
    ///
    /// Returns an error when there is no snapshot, the game is not in
    /// hyperspace, or the ship moved less than `minimum`.
    pub fn assert_displacement(
        &mut self,
        mode: Result<GameMode, String>,
        position: Option<HyperspacePosition>,
        minimum: u32,
    ) -> Result<VerifiedDisplacement, FlightPositionError> {
        let from = self
            .baseline
            .take()
            .ok_or(FlightPositionError::NoBaseline)?;
        let to = require_hyperspace(mode, position)?;
        let required = u64::from(minimum);
        let moved = distance(from, to);
        if moved < required {
            return Err(FlightPositionError::Insufficient {
                from,
                to,
                moved,
                required,
            });
        }
        Ok(VerifiedDisplacement {
            from,
            to,
            moved,
            required,
        })
    }

    /// Require at most `maximum` logical units of movement since the snapshot,
    /// consuming the snapshot whether or not the requirement holds.
    ///
    /// # Errors
    ///
    /// Returns an error when there is no snapshot, the game is not in
    /// hyperspace, or the ship moved more than `maximum`.
    pub fn assert_stationary(
        &mut self,
        mode: Result<GameMode, String>,
        position: Option<HyperspacePosition>,
        maximum: u32,
    ) -> Result<VerifiedStationary, FlightPositionError> {
        let from = self
            .baseline
            .take()
            .ok_or(FlightPositionError::NoBaseline)?;
        let to = require_hyperspace(mode, position)?;
        let allowed = u64::from(maximum);
        let moved = distance(from, to);
        if moved > allowed {
            return Err(FlightPositionError::Moved {
                from,
                to,
                moved,
                allowed,
            });
        }
        Ok(VerifiedStationary {
            from,
            to,
            moved,
            allowed,
        })
    }
}

fn require_hyperspace(
    mode: Result<GameMode, String>,
    position: Option<HyperspacePosition>,
) -> Result<HyperspacePosition, FlightPositionError> {
    match mode {
        Ok(GameMode::Starmap) => {}
        Ok(other) => {
            return Err(FlightPositionError::NotHyperspace {
                observed: other.name().to_string(),
            })
        }
        Err(ambiguous) => {
            return Err(FlightPositionError::NotHyperspace {
                observed: ambiguous,
            })
        }
    }
    position.ok_or(FlightPositionError::PositionUnavailable)
}

/// Euclidean distance in logical units, rounded down.
fn distance(from: HyperspacePosition, to: HyperspacePosition) -> u64 {
    let dx = (i128::from(to.log_x) - i128::from(from.log_x)).unsigned_abs();
    let dy = (i128::from(to.log_y) - i128::from(from.log_y)).unsigned_abs();
    let squared = dx * dx + dy * dy;
    let root = squared.isqrt();
    // The largest possible distance is about 2^32.5, so this never saturates.
    u64::try_from(root).unwrap_or(u64::MAX)
}

/// The live hyperspace position, when this build links the game.
#[must_use]
pub fn live_position() -> Option<HyperspacePosition> {
    crate::automation::input_ffi::hyperspace_position()
        .map(|(log_x, log_y)| HyperspacePosition { log_x, log_y })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(log_x: i32, log_y: i32) -> Option<HyperspacePosition> {
        Some(HyperspacePosition { log_x, log_y })
    }

    fn hyperspace() -> Result<GameMode, String> {
        Ok(GameMode::Starmap)
    }

    #[test]
    fn a_stationary_ship_is_rejected() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(1752, 1450)).unwrap();
        let error = tracker
            .assert_displacement(hyperspace(), at(1752, 1450), 16)
            .unwrap_err();
        assert_eq!(
            error,
            FlightPositionError::Insufficient {
                from: HyperspacePosition {
                    log_x: 1752,
                    log_y: 1450
                },
                to: HyperspacePosition {
                    log_x: 1752,
                    log_y: 1450
                },
                moved: 0,
                required: 16,
            }
        );
        assert!(error
            .trace_label()
            .starts_with("hyperspace_displacement_failed:"));
    }

    #[test]
    fn a_ship_that_keeps_moving_is_not_stationary() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(1752, 1450)).unwrap();
        let error = tracker
            .assert_stationary(hyperspace(), at(1752, 1462), 0)
            .unwrap_err();
        assert_eq!(
            error.trace_label(),
            "hyperspace_stationary_failed:from=1752,1450:to=1752,1462:moved=12:maximum=0"
        );
    }

    #[test]
    fn a_settled_ship_is_stationary_and_the_baseline_is_consumed() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(1752, 1450)).unwrap();
        let proof = tracker
            .assert_stationary(hyperspace(), at(1752, 1450), 0)
            .unwrap();
        assert_eq!(
            proof.trace_label(),
            "hyperspace_stationary_verified:from=1752,1450:to=1752,1450:moved=0:maximum=0"
        );
        assert_eq!(
            tracker
                .assert_stationary(hyperspace(), at(1752, 1450), 0)
                .unwrap_err(),
            FlightPositionError::NoBaseline
        );
    }

    #[test]
    fn stationary_needs_hyperspace() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(1, 1)).unwrap();
        let error = tracker
            .assert_stationary(Ok(GameMode::Solar), at(1, 1), 0)
            .unwrap_err();
        assert!(matches!(error, FlightPositionError::NotHyperspace { .. }));
    }

    #[test]
    fn real_movement_is_accepted_on_either_axis_and_direction() {
        for end in [
            (1800, 1450),
            (1752, 1500),
            (1700, 1450),
            (1752, 1400),
            (1790, 1480),
        ] {
            let mut tracker = FlightPositionTracker::default();
            tracker.record(hyperspace(), at(1752, 1450)).unwrap();
            let proof = tracker
                .assert_displacement(hyperspace(), at(end.0, end.1), 16)
                .unwrap();
            assert!(proof.moved >= 16, "{end:?} moved {}", proof.moved);
        }
    }

    #[test]
    fn movement_below_the_minimum_is_rejected_and_the_boundary_is_inclusive() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        assert!(matches!(
            tracker.assert_displacement(hyperspace(), at(15, 0), 16),
            Err(FlightPositionError::Insufficient { moved: 15, .. })
        ));
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        let proof = tracker
            .assert_displacement(hyperspace(), at(0, 16), 16)
            .unwrap();
        assert_eq!(proof.moved, 16);
    }

    #[test]
    fn distance_is_euclidean_and_overflow_safe() {
        let origin = HyperspacePosition { log_x: 0, log_y: 0 };
        let three_four = HyperspacePosition { log_x: 3, log_y: 4 };
        assert_eq!(distance(origin, three_four), 5);
        let low = HyperspacePosition {
            log_x: i32::MIN,
            log_y: i32::MIN,
        };
        let high = HyperspacePosition {
            log_x: i32::MAX,
            log_y: i32::MAX,
        };
        assert!(distance(low, high) > u64::from(u32::MAX));
        assert_eq!(distance(origin, origin), 0);
    }

    #[test]
    fn an_assertion_without_a_snapshot_is_an_error() {
        let mut tracker = FlightPositionTracker::default();
        assert_eq!(
            tracker.assert_displacement(hyperspace(), at(500, 500), 1),
            Err(FlightPositionError::NoBaseline)
        );
    }

    #[test]
    fn a_snapshot_proves_only_one_leg() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        tracker
            .assert_displacement(hyperspace(), at(100, 0), 16)
            .unwrap();
        assert_eq!(
            tracker.assert_displacement(hyperspace(), at(900, 0), 16),
            Err(FlightPositionError::NoBaseline)
        );
    }

    #[test]
    fn a_failed_assertion_also_consumes_the_snapshot() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        assert!(tracker
            .assert_displacement(hyperspace(), at(0, 0), 16)
            .is_err());
        assert_eq!(
            tracker.assert_displacement(hyperspace(), at(900, 0), 16),
            Err(FlightPositionError::NoBaseline)
        );
    }

    #[test]
    fn recording_again_replaces_the_snapshot() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        tracker.record(hyperspace(), at(1000, 0)).unwrap();
        assert!(tracker
            .assert_displacement(hyperspace(), at(1000, 0), 16)
            .is_err());
    }

    #[test]
    fn leaving_hyperspace_is_a_mode_error_not_movement() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        let error = tracker
            .assert_displacement(Ok(GameMode::Communication), at(5000, 5000), 16)
            .unwrap_err();
        assert_eq!(
            error,
            FlightPositionError::NotHyperspace {
                observed: "communication".to_string()
            }
        );
        assert_eq!(
            tracker.assert_displacement(hyperspace(), at(5000, 5000), 16),
            Err(FlightPositionError::NoBaseline)
        );
    }

    #[test]
    fn recording_outside_hyperspace_fails_and_clears_any_old_snapshot() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        assert!(matches!(
            tracker.record(Ok(GameMode::Solar), at(0, 0)),
            Err(FlightPositionError::NotHyperspace { .. })
        ));
        assert_eq!(
            tracker.assert_displacement(hyperspace(), at(900, 0), 16),
            Err(FlightPositionError::NoBaseline)
        );
    }

    #[test]
    fn an_ambiguous_mode_is_never_treated_as_hyperspace() {
        let mut tracker = FlightPositionTracker::default();
        let ambiguous = Err("ambiguous mode for activity 0x0007".to_string());
        assert!(matches!(
            tracker.record(ambiguous, at(0, 0)),
            Err(FlightPositionError::NotHyperspace { .. })
        ));
    }

    #[test]
    fn a_build_without_the_game_position_cannot_record_or_assert() {
        let mut tracker = FlightPositionTracker::default();
        assert_eq!(
            tracker.record(hyperspace(), None),
            Err(FlightPositionError::PositionUnavailable)
        );
        let mut tracker = FlightPositionTracker::default();
        tracker.record(hyperspace(), at(0, 0)).unwrap();
        assert_eq!(
            tracker.assert_displacement(hyperspace(), None, 1),
            Err(FlightPositionError::PositionUnavailable)
        );
    }

    #[test]
    fn a_new_tracker_for_a_new_run_has_no_snapshot() {
        let mut first = FlightPositionTracker::default();
        first.record(hyperspace(), at(0, 0)).unwrap();
        let mut second = FlightPositionTracker::default();
        assert_eq!(
            second.assert_displacement(hyperspace(), at(900, 0), 16),
            Err(FlightPositionError::NoBaseline)
        );
    }

    #[test]
    fn trace_labels_are_stable() {
        assert_eq!(
            recorded_trace_label(HyperspacePosition {
                log_x: 7,
                log_y: -3
            }),
            "hyperspace_position_recorded:x=7:y=-3"
        );
        let proof = VerifiedDisplacement {
            from: HyperspacePosition { log_x: 0, log_y: 0 },
            to: HyperspacePosition { log_x: 3, log_y: 4 },
            moved: 5,
            required: 1,
        };
        assert_eq!(
            proof.trace_label(),
            "hyperspace_displacement_verified:from=0,0:to=3,4:moved=5:minimum=1"
        );
    }

    #[test]
    fn a_load_that_restores_the_saved_position_is_accepted() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record_saved(hyperspace(), at(1752, 1450)).unwrap();
        tracker.record(hyperspace(), at(1752, 1450)).unwrap();
        tracker
            .assert_displacement(hyperspace(), at(1752, 1520), 16)
            .unwrap();
        let (saved, observed) = tracker
            .assert_restored(hyperspace(), at(1752, 1450))
            .unwrap();
        assert_eq!(saved, observed);
        assert_eq!(
            restored_trace_label(saved, observed),
            "hyperspace_position_restored:x=1752:y=1450:saved_x=1752:saved_y=1450"
        );
    }

    #[test]
    fn a_load_that_never_happened_leaves_the_moved_position_and_is_rejected() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record_saved(hyperspace(), at(1752, 1450)).unwrap();
        let error = tracker
            .assert_restored(hyperspace(), at(1752, 1520))
            .unwrap_err();
        assert_eq!(
            error.trace_label(),
            "hyperspace_position_restore_failed:saved=1752,1450:observed=1752,1520"
        );
    }

    #[test]
    fn one_unit_of_difference_on_either_axis_is_not_restored() {
        for observed in [(1753, 1450), (1752, 1451), (1751, 1450), (1752, 1449)] {
            let mut tracker = FlightPositionTracker::default();
            tracker.record_saved(hyperspace(), at(1752, 1450)).unwrap();
            assert!(matches!(
                tracker.assert_restored(hyperspace(), at(observed.0, observed.1)),
                Err(FlightPositionError::NotRestored { .. })
            ));
        }
    }

    #[test]
    fn restore_without_a_kept_position_or_outside_hyperspace_is_rejected() {
        let mut tracker = FlightPositionTracker::default();
        assert_eq!(
            tracker.assert_restored(hyperspace(), at(1, 1)).unwrap_err(),
            FlightPositionError::NoSavedPosition
        );
        tracker.record_saved(hyperspace(), at(1, 1)).unwrap();
        assert!(matches!(
            tracker.assert_restored(Ok(GameMode::Solar), at(1, 1)),
            Err(FlightPositionError::NotHyperspace { .. })
        ));
        // The kept position is consumed by the failed attempt.
        assert_eq!(
            tracker.assert_restored(hyperspace(), at(1, 1)).unwrap_err(),
            FlightPositionError::NoSavedPosition
        );
    }

    #[test]
    fn displacement_does_not_consume_the_position_kept_for_a_load() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record_saved(hyperspace(), at(10, 10)).unwrap();
        tracker.record(hyperspace(), at(10, 10)).unwrap();
        tracker
            .assert_displacement(hyperspace(), at(10, 90), 16)
            .unwrap();
        assert!(tracker.assert_restored(hyperspace(), at(10, 10)).is_ok());
    }

    #[test]
    fn a_failed_save_record_keeps_nothing() {
        let mut tracker = FlightPositionTracker::default();
        tracker.record_saved(hyperspace(), at(5, 5)).unwrap();
        assert!(tracker.record_saved(Ok(GameMode::Solar), at(5, 5)).is_err());
        assert_eq!(
            tracker.assert_restored(hyperspace(), at(5, 5)).unwrap_err(),
            FlightPositionError::NoSavedPosition
        );
    }
}
