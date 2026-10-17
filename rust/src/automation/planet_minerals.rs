//! Driving the planet-side lander to collect mineral deposits.
//!
//! `collect_planet_minerals` steers the lander with the same player-one
//! thrust and turn keys a person would hold, until the lander hold carries
//! the requested number of units of one element category. Everything it reads
//! is published by the planet-side session each frame (`planet_side::telemetry`);
//! it never writes game state, only input.
//!
//! The decisions are pure: the observation is passed in, so steering, the
//! completion test, and every failure are testable without a linked game.

use crate::automation::navigation::NavigationControl;
use crate::automation::script::CollectPlanetMineralsStep;
use crate::battle::battle_types::{angle_to_facing, arctan, shortest_path_delta};
use crate::planet_side::cargo::{ElementCategory, NUM_ELEMENT_CATEGORIES};
use crate::planet_side::model::SurfacePoint;
use crate::planet_side::telemetry::{self, MineralDeposit};
use crate::planet_side::world::WORLD_WIDTH;

/// `SessionPhase::Active`, the only phase in which the lander answers input.
const ACTIVE_PHASE: u32 = 4;
/// Phases that precede `Active`: the lander is still being lowered.
const LAST_ARRIVAL_PHASE: u32 = 3;

const FACINGS: i32 = 16;

/// One read of the planet-side session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetSnapshot {
    pub active: bool,
    pub generation: u64,
    pub phase: u32,
    pub position: SurfacePoint,
    pub facing: u8,
    pub units: [u16; NUM_ELEMENT_CATEGORIES],
    /// `None` until the surface world has published for this trip.
    pub deposits: Option<Vec<MineralDeposit>>,
}

impl PlanetSnapshot {
    /// The live session, read field by field on the game thread.
    #[must_use]
    pub fn live() -> Self {
        let observation = telemetry::observation();
        Self {
            active: observation.active,
            generation: observation.generation,
            phase: observation.phase,
            position: SurfacePoint {
                x: observation.current_x,
                y: observation.current_y,
            },
            facing: observation.facing,
            units: observation.mineral_units,
            deposits: telemetry::deposits(),
        }
    }
}

/// Why the collection could not be completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectError {
    /// No lander session is running, or the trip ended or changed.
    LeftPlanetSide { phase: u32 },
    /// The surface has no deposit of the requested category.
    NoDeposits,
    /// The tick budget ran out first.
    Timeout { units: u16 },
}

/// A collection that reached its minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectedMinerals {
    pub category: ElementCategory,
    pub units: u16,
    pub minimum: u16,
}

impl CollectedMinerals {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "planet_minerals_collected:category={}:units={}:minimum={}",
            self.category.name(),
            self.units,
            self.minimum
        )
    }
}

impl CollectError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self, step: &CollectPlanetMineralsStep) -> String {
        let reason = match self {
            Self::LeftPlanetSide { phase } => {
                format!("left_planet_side:phase={}", telemetry::phase_name(*phase))
            }
            Self::NoDeposits => "no_deposits".to_string(),
            Self::Timeout { units } => format!("timeout:units={units}"),
        };
        format!(
            "planet_minerals_collect_failed:category={}:minimum={}:reason={reason}",
            step.category.name(),
            step.minimum_units
        )
    }
}

/// What the lander should do after one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectProgress {
    /// Hold these controls.
    Steering(NavigationControl),
    /// The minimum was reached: release every control.
    Complete(CollectedMinerals),
}

fn wrapped_delta(from: SurfacePoint, to: (i32, i32)) -> (i32, i32) {
    (
        shortest_path_delta(from.x, to.0, WORLD_WIDTH),
        to.1 - from.y,
    )
}

fn wrapped_distance_squared(from: SurfacePoint, to: (i32, i32)) -> i64 {
    let (dx, dy) = wrapped_delta(from, to);
    i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
}

/// The nearest deposit of `category`, measuring across the wrapped seam.
#[must_use]
pub fn nearest_deposit(
    position: SurfacePoint,
    deposits: &[MineralDeposit],
    category: ElementCategory,
) -> Option<MineralDeposit> {
    deposits
        .iter()
        .filter(|deposit| usize::from(deposit.category) == category.index())
        .min_by_key(|deposit| wrapped_distance_squared(position, (deposit.x, deposit.y)))
        .copied()
}

/// The facing that points from `position` at `target`, over the shortest
/// wrapped horizontal path. `None` when the lander is on the target.
#[must_use]
pub fn bearing_facing(position: SurfacePoint, target: (i32, i32)) -> Option<u8> {
    let (dx, dy) = wrapped_delta(position, target);
    if dx == 0 && dy == 0 {
        return None;
    }
    let facing = i32::from(angle_to_facing(arctan(dx, dy))) % FACINGS;
    u8::try_from(facing).ok()
}

/// Choose the controls that bring the lander toward `target`.
///
/// The lander turns one facing per `TURN_WAIT` and moves only while thrust is
/// held, so it turns in place until its heading is within one facing of the
/// bearing, then thrusts. One facing off it keeps turning while it thrusts so
/// the heading converges instead of drifting past the deposit.
#[must_use]
pub fn steer(position: SurfacePoint, facing: u8, target: (i32, i32)) -> NavigationControl {
    let Some(bearing) = bearing_facing(position, target) else {
        return NavigationControl::default();
    };
    // Facings grow clockwise, and a right turn adds one.
    let error = (i32::from(bearing) - i32::from(facing)).rem_euclid(FACINGS);
    let (turn_right, turn_left, distance) = match error {
        0 => (false, false, 0),
        1..=8 => (true, false, error),
        _ => (false, true, FACINGS - error),
    };
    NavigationControl {
        thrust: distance <= 1,
        right: turn_right,
        left: turn_left,
        ..NavigationControl::default()
    }
}

/// Counts the ticks of the collection in progress and binds it to one trip.
#[derive(Debug, Default)]
pub struct PlanetMineralsTracker {
    started: Option<(u64, u64)>,
}

impl PlanetMineralsTracker {
    /// Forget the collection in progress, so the next observation starts one.
    pub fn finish(&mut self) {
        self.started = None;
    }

    /// Observe the lander once. A completion or a failure ends the collection.
    ///
    /// # Errors
    ///
    /// Returns an error when the trip ended or changed, the surface has no
    /// deposit of the category, or `max_ticks` observations passed first.
    pub fn observe(
        &mut self,
        snapshot: &PlanetSnapshot,
        step: &CollectPlanetMineralsStep,
    ) -> Result<CollectProgress, CollectError> {
        let result = self.decide(snapshot, step);
        if !matches!(result, Ok(CollectProgress::Steering(_))) {
            self.started = None;
        }
        result
    }

    fn decide(
        &mut self,
        snapshot: &PlanetSnapshot,
        step: &CollectPlanetMineralsStep,
    ) -> Result<CollectProgress, CollectError> {
        let left = || CollectError::LeftPlanetSide {
            phase: snapshot.phase,
        };
        if !snapshot.active {
            return Err(left());
        }
        let (generation, ticks) = self.started.get_or_insert((snapshot.generation, 0));
        if *generation != snapshot.generation {
            return Err(left());
        }
        *ticks += 1;
        let ticks = *ticks;

        let units = snapshot.units[step.category.index()];
        if units >= step.minimum_units {
            return Ok(CollectProgress::Complete(CollectedMinerals {
                category: step.category,
                units,
                minimum: step.minimum_units,
            }));
        }
        if snapshot.phase > ACTIVE_PHASE {
            return Err(left());
        }
        if ticks >= step.max_ticks {
            return Err(CollectError::Timeout { units });
        }
        let arriving = snapshot.phase <= LAST_ARRIVAL_PHASE;
        let Some(deposits) = snapshot.deposits.as_deref().filter(|_| !arriving) else {
            return Ok(CollectProgress::Steering(NavigationControl::default()));
        };
        let target = nearest_deposit(snapshot.position, deposits, step.category)
            .ok_or(CollectError::NoDeposits)?;
        Ok(CollectProgress::Steering(steer(
            snapshot.position,
            snapshot.facing,
            (target.x, target.y),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(minimum_units: u16, max_ticks: u64) -> CollectPlanetMineralsStep {
        CollectPlanetMineralsStep {
            category: ElementCategory::Radioactive,
            minimum_units,
            max_ticks,
        }
    }

    fn at(x: i32, y: i32) -> SurfacePoint {
        SurfacePoint { x, y }
    }

    fn deposit(x: i32, y: i32, category: ElementCategory) -> MineralDeposit {
        MineralDeposit {
            x,
            y,
            category: u8::try_from(category.index()).unwrap(),
        }
    }

    fn snapshot(deposits: Option<Vec<MineralDeposit>>) -> PlanetSnapshot {
        PlanetSnapshot {
            active: true,
            generation: 7,
            phase: ACTIVE_PHASE,
            position: at(100, 100),
            facing: 0,
            units: [0; NUM_ELEMENT_CATEGORIES],
            deposits,
        }
    }

    // Offsets from (100, 100) whose bearing is exactly facing `n` (a multiple
    // of 45 degrees is exact; the others use the 22.5 degree ratios).
    const OFFSETS: [(i32, i32); 16] = [
        (0, -100),
        (38, -92),
        (71, -71),
        (92, -38),
        (100, 0),
        (92, 38),
        (71, 71),
        (38, 92),
        (0, 100),
        (-38, 92),
        (-71, 71),
        (-92, 38),
        (-100, 0),
        (-92, -38),
        (-71, -71),
        (-38, -92),
    ];

    #[test]
    fn the_bearing_names_each_of_the_sixteen_facings() {
        for (facing, (dx, dy)) in OFFSETS.into_iter().enumerate() {
            assert_eq!(
                bearing_facing(at(100, 100), (100 + dx, 100 + dy)),
                Some(u8::try_from(facing).unwrap()),
                "offset ({dx}, {dy})"
            );
        }
        assert_eq!(bearing_facing(at(100, 100), (100, 100)), None);
    }

    #[test]
    fn a_lander_aligned_with_the_bearing_thrusts_without_turning() {
        for (facing, (dx, dy)) in OFFSETS.into_iter().enumerate() {
            let control = steer(
                at(100, 100),
                u8::try_from(facing).unwrap(),
                (100 + dx, 100 + dy),
            );
            assert!(
                control.thrust && !control.left && !control.right,
                "{facing}"
            );
        }
    }

    #[test]
    fn the_lander_turns_the_short_way_for_every_facing_pair() {
        for facing in 0..16u8 {
            for (bearing, (dx, dy)) in OFFSETS.into_iter().enumerate() {
                let control = steer(at(100, 100), facing, (100 + dx, 100 + dy));
                let error = (bearing as i32 - i32::from(facing)).rem_euclid(16);
                match error {
                    0 => assert!(!control.left && !control.right),
                    1..=8 => assert!(control.right && !control.left, "{facing}->{bearing}"),
                    _ => assert!(control.left && !control.right, "{facing}->{bearing}"),
                }
                assert_eq!(
                    control.thrust,
                    error <= 1 || error == 15,
                    "{facing}->{bearing}"
                );
            }
        }
    }

    #[test]
    fn a_deposit_across_the_seam_is_reached_by_wrapping() {
        // Lander near the left edge, deposit near the right edge: the short
        // way is west, not the whole width east.
        let position = at(10, 50);
        let target = (WORLD_WIDTH - 10, 50);
        assert_eq!(bearing_facing(position, target), Some(12));
        let control = steer(position, 12, target);
        assert!(control.thrust && !control.left && !control.right);
        // And mirrored: lander near the right edge heads east across the seam.
        assert_eq!(bearing_facing(at(WORLD_WIDTH - 10, 50), (10, 50)), Some(4));
    }

    #[test]
    fn the_nearest_deposit_of_the_category_wins_across_the_seam() {
        let deposits = [
            deposit(300, 50, ElementCategory::Radioactive),
            deposit(WORLD_WIDTH - 20, 50, ElementCategory::Radioactive),
            deposit(12, 50, ElementCategory::Common),
        ];
        assert_eq!(
            nearest_deposit(at(5, 50), &deposits, ElementCategory::Radioactive),
            Some(deposits[1])
        );
        assert_eq!(
            nearest_deposit(at(5, 50), &deposits, ElementCategory::Exotic),
            None
        );
    }

    #[test]
    fn a_surface_without_the_category_reports_failure() {
        let mut tracker = PlanetMineralsTracker::default();
        let other = snapshot(Some(vec![deposit(5, 5, ElementCategory::Common)]));
        assert_eq!(
            tracker.observe(&other, &step(1, 100)),
            Err(CollectError::NoDeposits)
        );
        let empty = snapshot(Some(Vec::new()));
        assert_eq!(
            tracker.observe(&empty, &step(1, 100)),
            Err(CollectError::NoDeposits)
        );
    }

    #[test]
    fn the_lander_waits_without_input_until_the_surface_is_published() {
        let mut tracker = PlanetMineralsTracker::default();
        let idle = Ok(CollectProgress::Steering(NavigationControl::default()));
        let mut arriving = snapshot(None);
        arriving.phase = 3;
        assert_eq!(tracker.observe(&arriving, &step(1, 100)), idle);
        arriving.deposits = Some(vec![deposit(200, 100, ElementCategory::Radioactive)]);
        assert_eq!(tracker.observe(&arriving, &step(1, 100)), idle);
        assert_eq!(tracker.observe(&snapshot(None), &step(1, 100)), idle);
    }

    #[test]
    fn the_lander_steers_toward_the_nearest_deposit_until_the_minimum() {
        let mut tracker = PlanetMineralsTracker::default();
        let mut live = snapshot(Some(vec![deposit(200, 100, ElementCategory::Radioactive)]));
        live.facing = 4;
        let Ok(CollectProgress::Steering(control)) = tracker.observe(&live, &step(2, 100)) else {
            panic!("expected steering");
        };
        assert!(control.thrust && !control.left && !control.right);

        live.units[ElementCategory::Radioactive.index()] = 1;
        assert!(matches!(
            tracker.observe(&live, &step(2, 100)),
            Ok(CollectProgress::Steering(_))
        ));
        live.units[ElementCategory::Radioactive.index()] = 3;
        let Ok(CollectProgress::Complete(done)) = tracker.observe(&live, &step(2, 100)) else {
            panic!("expected completion");
        };
        assert_eq!(
            done.trace_label(),
            "planet_minerals_collected:category=radioactive:units=3:minimum=2"
        );
    }

    #[test]
    fn units_of_another_category_do_not_complete_the_collection() {
        let mut tracker = PlanetMineralsTracker::default();
        let mut live = snapshot(Some(vec![deposit(200, 100, ElementCategory::Radioactive)]));
        live.units[ElementCategory::Common.index()] = 40;
        assert!(matches!(
            tracker.observe(&live, &step(1, 100)),
            Ok(CollectProgress::Steering(_))
        ));
    }

    #[test]
    fn the_collection_times_out_at_max_ticks() {
        let mut tracker = PlanetMineralsTracker::default();
        let live = snapshot(Some(vec![deposit(200, 100, ElementCategory::Radioactive)]));
        for _ in 0..2 {
            assert!(matches!(
                tracker.observe(&live, &step(1, 3)),
                Ok(CollectProgress::Steering(_))
            ));
        }
        let failure = tracker.observe(&live, &step(1, 3)).unwrap_err();
        assert_eq!(failure, CollectError::Timeout { units: 0 });
        assert_eq!(
            failure.trace_label(&step(1, 3)),
            "planet_minerals_collect_failed:category=radioactive:minimum=1:reason=timeout:units=0"
        );
    }

    #[test]
    fn a_completion_on_the_last_tick_is_not_a_timeout() {
        let mut tracker = PlanetMineralsTracker::default();
        let mut live = snapshot(Some(vec![deposit(200, 100, ElementCategory::Radioactive)]));
        assert!(tracker.observe(&live, &step(1, 2)).is_ok());
        live.units[ElementCategory::Radioactive.index()] = 1;
        assert!(matches!(
            tracker.observe(&live, &step(1, 2)),
            Ok(CollectProgress::Complete(_))
        ));
    }

    #[test]
    fn leaving_the_planet_side_or_changing_trips_fails() {
        let mut tracker = PlanetMineralsTracker::default();
        let mut live = snapshot(Some(vec![deposit(200, 100, ElementCategory::Radioactive)]));
        assert!(tracker.observe(&live, &step(1, 100)).is_ok());
        live.generation += 1;
        assert!(matches!(
            tracker.observe(&live, &step(1, 100)),
            Err(CollectError::LeftPlanetSide { .. })
        ));

        let mut gone = snapshot(None);
        gone.active = false;
        gone.phase = 8;
        let failure = tracker.observe(&gone, &step(1, 100)).unwrap_err();
        assert_eq!(
            failure.trace_label(&step(1, 100)),
            "planet_minerals_collect_failed:category=radioactive:minimum=1:reason=left_planet_side:phase=Complete"
        );

        let mut taking_off = snapshot(None);
        taking_off.phase = 5;
        assert!(matches!(
            tracker.observe(&taking_off, &step(1, 100)),
            Err(CollectError::LeftPlanetSide { .. })
        ));
    }

    #[test]
    fn a_finished_collection_starts_the_next_one_fresh() {
        let mut tracker = PlanetMineralsTracker::default();
        let live = snapshot(Some(vec![deposit(200, 100, ElementCategory::Radioactive)]));
        assert!(tracker.observe(&live, &step(1, 2)).is_ok());
        assert!(tracker.observe(&live, &step(1, 2)).is_err());
        assert!(tracker.observe(&live, &step(1, 2)).is_ok());
    }
}
