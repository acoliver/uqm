//! Fighting a full-game battle with the same keys a person would press.
//!
//! `fight_battle` steers the flagship (side 0, the human player's) toward the
//! opposing ship and fires, until the opposing side has lost a ship and the
//! player's side has not. It reads the running battle through `BattleSnapshot`
//! and never writes game state, only the player-one keys.
//!
//! The decisions are pure: the snapshot is passed in, so aiming, firing, the
//! standoff distance, the outcome and every failure are testable without a
//! linked game.

use crate::automation::script::FightBattleStep;
use crate::battle::battle_types::{angle_to_facing, arctan, shortest_path_delta};

const FACINGS: i32 = 16;
/// `NUM_SIDES`: side 0 is the human player's, side 1 the opponent's.
const SIDES: usize = 2;
const PLAYER_SIDE: usize = 0;
const ENEMY_SIDE: usize = 1;

/// The flagship's ion bolt flies `BLASTER_SPEED` (24 display units, 96 world
/// units) for `BLASTER_LIFE` (12) frames. Firing stops short of that.
const FIRE_RANGE: i64 = 1000;
/// The Avenger's hellfire flies 25 world units for 8 frames from a 29 unit
/// offset (`ilwrath.c`), about 230 world units. Inside this the flagship backs
/// away instead of coasting.
const KEEP_OFF_RANGE: i64 = 400;
/// Farther than this the flagship closes the distance.
const APPROACH_RANGE: i64 = 800;

/// `LOG_SPACE_WIDTH` and `LOG_SPACE_HEIGHT` for a screen size: the battle
/// arena is `DISPLAY_TO_WORLD (SPACE) << MAX_REDUCTION` world units across,
/// where the space is the screen less the 64 pixel status panel (units.h).
#[must_use]
pub const fn arena_size(screen_width: i32, screen_height: i32) -> (i32, i32) {
    const STATUS_WIDTH: i32 = 64;
    ((screen_width - STATUS_WIDTH) << 5, screen_height << 5)
}

/// One ship in the battle, as `ELEMENT` and `STARSHIP` publish it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShipState {
    pub x: i32,
    pub y: i32,
    pub facing: u8,
    pub crew: u16,
}

/// One read of the running battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleSnapshot {
    pub in_battle: bool,
    /// `battle_counter`: ships left on each side.
    pub counters: [u8; SIDES],
    pub player: Option<ShipState>,
    pub enemy: Option<ShipState>,
    /// The wrapped arena in world units, width then height.
    pub arena: (i32, i32),
}

impl BattleSnapshot {
    /// The snapshot of no battle at all.
    #[must_use]
    pub const fn none(arena: (i32, i32)) -> Self {
        Self {
            in_battle: false,
            counters: [0; SIDES],
            player: None,
            enemy: None,
            arena,
        }
    }
}

/// The keys to hold for one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FightControl {
    pub thrust: bool,
    pub left: bool,
    pub right: bool,
    pub weapon: bool,
}

fn wrapped_delta(from: &ShipState, to: &ShipState, arena: (i32, i32)) -> (i32, i32) {
    (
        shortest_path_delta(from.x, to.x, arena.0),
        shortest_path_delta(from.y, to.y, arena.1),
    )
}

/// The facing that points along `(dx, dy)`; `None` for no displacement.
#[must_use]
pub fn bearing_facing(dx: i32, dy: i32) -> Option<u8> {
    if dx == 0 && dy == 0 {
        return None;
    }
    let facing = i32::from(angle_to_facing(arctan(dx, dy))) % FACINGS;
    u8::try_from(facing).ok()
}

/// Choose the keys that aim the player's ship at the enemy and fire.
///
/// Facings grow clockwise and a right turn adds one. The ship turns toward the
/// enemy along the shorter way round, fires once within one facing of the
/// bearing and within bolt range, closes in when far away, and thrusts only
/// while facing away from an enemy inside hellfire range.
#[must_use]
pub fn fight(player: &ShipState, enemy: &ShipState, arena: (i32, i32)) -> FightControl {
    let (dx, dy) = wrapped_delta(player, enemy, arena);
    let Some(bearing) = bearing_facing(dx, dy) else {
        return FightControl {
            weapon: true,
            ..FightControl::default()
        };
    };
    let distance_squared = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
    let error = (i32::from(bearing) - i32::from(player.facing)).rem_euclid(FACINGS);
    let (right, left) = match error {
        0 => (false, false),
        1..=8 => (true, false),
        _ => (false, true),
    };
    let toward = error <= 2 || error >= FACINGS - 2;
    let away = (5..=11).contains(&error);
    let thrust = if distance_squared > APPROACH_RANGE * APPROACH_RANGE {
        toward
    } else if distance_squared < KEEP_OFF_RANGE * KEEP_OFF_RANGE {
        away
    } else {
        false
    };
    let aligned = error <= 1 || error == FACINGS - 1;
    FightControl {
        thrust,
        left,
        right,
        weapon: aligned && distance_squared <= FIRE_RANGE * FIRE_RANGE,
    }
}

/// Why the fight could not be won.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightError {
    /// No battle is running, or it ended with neither side losing a ship.
    NotInBattle,
    /// The player's side lost a ship.
    PlayerShipLost { player_left: u8, enemy_left: u8 },
    /// The tick budget ran out first.
    Timeout,
}

impl FightError {
    /// The semantic trace label that records this failure.
    #[must_use]
    pub fn trace_label(&self) -> String {
        let reason = match self {
            Self::NotInBattle => "not_in_battle",
            Self::PlayerShipLost { .. } => "player_ship_lost",
            Self::Timeout => "timeout",
        };
        format!("battle_fight_failed:reason={reason}")
    }
}

/// A battle in which the enemy side lost a ship and the player's did not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleWon {
    pub player_left: u8,
    pub enemy_left: u8,
    pub ticks: u64,
}

impl BattleWon {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "battle_won:player_left={}:enemy_left={}:ticks={}",
            self.player_left, self.enemy_left, self.ticks
        )
    }
}

/// What the player's ship should do after one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightProgress {
    /// Hold these keys.
    Fighting(FightControl),
    /// The enemy side lost a ship: release every key.
    Won(BattleWon),
}

#[derive(Debug, Clone, Copy)]
struct FightStart {
    ticks: u64,
    counters: [u8; SIDES],
}

/// Counts the ticks of the fight in progress and the ships each side began with.
#[derive(Debug, Default)]
pub struct BattleFightTracker {
    started: Option<FightStart>,
}

impl BattleFightTracker {
    /// Forget the fight in progress, so the next observation starts one.
    pub fn finish(&mut self) {
        self.started = None;
    }

    /// Observe the battle once. A win or a failure ends the fight.
    ///
    /// # Errors
    ///
    /// Returns an error when there is no battle, the player's side lost a ship,
    /// or `max_ticks` observations passed first.
    pub fn observe(
        &mut self,
        snapshot: &BattleSnapshot,
        step: &FightBattleStep,
    ) -> Result<FightProgress, FightError> {
        let result = self.decide(snapshot, step);
        if !matches!(result, Ok(FightProgress::Fighting(_))) {
            self.started = None;
        }
        result
    }

    fn decide(
        &mut self,
        snapshot: &BattleSnapshot,
        step: &FightBattleStep,
    ) -> Result<FightProgress, FightError> {
        if self.started.is_none() && !snapshot.in_battle {
            return Err(FightError::NotInBattle);
        }
        let start = self.started.get_or_insert(FightStart {
            ticks: 0,
            counters: snapshot.counters,
        });
        start.ticks += 1;
        let (ticks, began) = (start.ticks, start.counters);

        let [player_left, enemy_left] = snapshot.counters;
        if player_left < began[PLAYER_SIDE] {
            return Err(FightError::PlayerShipLost {
                player_left,
                enemy_left,
            });
        }
        if enemy_left < began[ENEMY_SIDE] {
            return Ok(FightProgress::Won(BattleWon {
                player_left,
                enemy_left,
                ticks,
            }));
        }
        if !snapshot.in_battle {
            return Err(FightError::NotInBattle);
        }
        if ticks >= step.max_ticks {
            return Err(FightError::Timeout);
        }
        let control = match (&snapshot.player, &snapshot.enemy) {
            (Some(player), Some(enemy)) => fight(player, enemy, snapshot.arena),
            _ => FightControl::default(),
        };
        Ok(FightProgress::Fighting(control))
    }
}

#[cfg(all(feature = "linked_c_archive", not(test)))]
impl BattleSnapshot {
    /// The running battle, read on the game thread.
    ///
    /// # Panics
    ///
    /// Panics when `disp_q` is not a valid element pool while a battle runs:
    /// the game's own queue is broken and no fight can be judged from it.
    #[must_use]
    pub fn live() -> Self {
        use crate::c_bindings::global_state::{ELEMENT, PLAYER_SHIP, STARSHIP};
        use crate::c_bindings::live_queues;
        use crate::mainloop::types::activity_flags::IN_BATTLE;
        use crate::savegame::runtime::queues::{walk, QueueKind};

        let (width, height) = crate::automation::input_ffi::screen_size();
        let arena = arena_size(width, height);
        let activity = crate::mainloop::ffi::get_current_activity().0;
        if activity & IN_BATTLE == 0 {
            return Self::none(arena);
        }
        // SAFETY: the production game invokes the automation hooks
        // synchronously on the game thread, so neither the battle counters nor
        // the display queue change during this read.
        unsafe {
            let display_list = &*live_queues::display_list();
            let elements = walk::<ELEMENT>(display_list, QueueKind::DisplayList)
                .expect("disp_q is an element pool while a battle runs");
            let (mut player, mut enemy) = (None, None);
            for link in elements {
                let element = &*link;
                if u32::from(element.state_flags) & PLAYER_SHIP == 0 || element.pParent.is_null() {
                    continue;
                }
                let owner = &*element.pParent.cast::<STARSHIP>();
                let ship = ShipState {
                    x: i32::from(element.current.location.x),
                    y: i32::from(element.current.location.y),
                    facing: u8::try_from(owner.ShipFacing % 16).expect("a facing below 16"),
                    crew: element.__bindgen_anon_2.crew_level,
                };
                match element.playerNr {
                    0 => player = Some(ship),
                    1 => enemy = Some(ship),
                    _ => {}
                }
            }
            Self {
                in_battle: true,
                counters: live_queues::battle_counters(),
                player,
                enemy,
                arena,
            }
        }
    }
}

#[cfg(not(all(feature = "linked_c_archive", not(test))))]
impl BattleSnapshot {
    /// Without the linked game there is no battle to read.
    #[must_use]
    pub fn live() -> Self {
        let (width, height) = crate::automation::input_ffi::screen_size();
        Self::none(arena_size(width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARENA: (i32, i32) = (8192, 7680);

    fn step(max_ticks: u64) -> FightBattleStep {
        FightBattleStep { max_ticks }
    }

    fn ship(x: i32, y: i32, facing: u8) -> ShipState {
        ShipState {
            x,
            y,
            facing,
            crew: 10,
        }
    }

    // Offsets whose bearing is exactly facing `n`, 600 world units away.
    const OFFSETS: [(i32, i32); 16] = [
        (0, -600),
        (228, -552),
        (426, -426),
        (552, -228),
        (600, 0),
        (552, 228),
        (426, 426),
        (228, 552),
        (0, 600),
        (-228, 552),
        (-426, 426),
        (-552, 228),
        (-600, 0),
        (-552, -228),
        (-426, -426),
        (-228, -552),
    ];

    fn snapshot(
        counters: [u8; 2],
        player: Option<ShipState>,
        enemy: Option<ShipState>,
    ) -> BattleSnapshot {
        BattleSnapshot {
            in_battle: true,
            counters,
            player,
            enemy,
            arena: ARENA,
        }
    }

    fn duel(player: ShipState, enemy: ShipState) -> BattleSnapshot {
        snapshot([1, 1], Some(player), Some(enemy))
    }

    #[test]
    fn the_arena_is_the_screen_less_the_status_panel_in_world_units() {
        assert_eq!(arena_size(320, 240), (8192, 7680));
        assert_eq!(arena_size(640, 480), (18432, 15360));
    }

    #[test]
    fn the_bearing_names_each_of_the_sixteen_facings() {
        for (facing, (dx, dy)) in OFFSETS.into_iter().enumerate() {
            assert_eq!(
                bearing_facing(dx, dy),
                Some(u8::try_from(facing).unwrap()),
                "offset ({dx}, {dy})"
            );
        }
        assert_eq!(bearing_facing(0, 0), None);
    }

    #[test]
    fn a_ship_aimed_at_a_mid_range_enemy_fires_without_turning_or_thrusting() {
        for (facing, (dx, dy)) in OFFSETS.into_iter().enumerate() {
            let player = ship(4000, 4000, u8::try_from(facing).unwrap());
            let enemy = ship(4000 + dx, 4000 + dy, 0);
            assert_eq!(
                fight(&player, &enemy, ARENA),
                FightControl {
                    weapon: true,
                    ..FightControl::default()
                },
                "facing {facing}"
            );
        }
    }

    #[test]
    fn the_ship_turns_the_shorter_way_round_for_every_facing_pair() {
        for facing in 0..16u8 {
            for (bearing, (dx, dy)) in OFFSETS.into_iter().enumerate() {
                let player = ship(4000, 4000, facing);
                let enemy = ship(4000 + dx, 4000 + dy, 0);
                let control = fight(&player, &enemy, ARENA);
                let error = (bearing as i32 - i32::from(facing)).rem_euclid(16);
                assert_eq!(
                    control.right,
                    (1..=8).contains(&error),
                    "{facing}->{bearing}"
                );
                assert_eq!(control.left, error > 8, "{facing}->{bearing}");
                assert_eq!(
                    control.weapon,
                    error <= 1 || error == 15,
                    "{facing}->{bearing}"
                );
            }
        }
    }

    #[test]
    fn the_bearing_crosses_the_arena_seam_by_the_shorter_way() {
        // Enemy 292 units west of the player across the x seam.
        let player = ship(100, 4000, 12);
        let enemy = ship(ARENA.0 - 192, 4000, 0);
        let control = fight(&player, &enemy, ARENA);
        assert!(control.weapon);
        assert!(!control.left && !control.right);
        // The same on the y seam, enemy to the north across it.
        let player = ship(4000, 50, 0);
        let enemy = ship(4000, ARENA.1 - 150, 0);
        assert!(fight(&player, &enemy, ARENA).weapon);
    }

    #[test]
    fn a_far_enemy_is_approached_only_when_roughly_aimed_at_it() {
        let enemy = ship(4000 + 2000, 4000, 0);
        let aimed = fight(&ship(4000, 4000, 4), &enemy, ARENA);
        assert!(aimed.thrust);
        assert!(!aimed.weapon, "outside bolt range");
        let off = fight(&ship(4000, 4000, 8), &enemy, ARENA);
        assert!(!off.thrust);
    }

    #[test]
    fn an_enemy_inside_hellfire_range_is_backed_away_from_only_when_facing_away() {
        let enemy = ship(4000 + 200, 4000, 0);
        let facing_it = fight(&ship(4000, 4000, 4), &enemy, ARENA);
        assert!(!facing_it.thrust);
        assert!(facing_it.weapon);
        let facing_away = fight(&ship(4000, 4000, 12), &enemy, ARENA);
        assert!(facing_away.thrust);
        assert!(!facing_away.weapon);
    }

    #[test]
    fn on_top_of_the_enemy_the_ship_only_fires() {
        let control = fight(&ship(4000, 4000, 3), &ship(4000, 4000, 0), ARENA);
        assert_eq!(
            control,
            FightControl {
                weapon: true,
                ..FightControl::default()
            }
        );
    }

    #[test]
    fn a_missing_ship_holds_every_key_until_both_exist() {
        let mut tracker = BattleFightTracker::default();
        let waiting = snapshot([1, 1], None, None);
        assert_eq!(
            tracker.observe(&waiting, &step(10)),
            Ok(FightProgress::Fighting(FightControl::default()))
        );
        let enemy_only = snapshot([1, 1], None, Some(ship(1, 1, 0)));
        assert_eq!(
            tracker.observe(&enemy_only, &step(10)),
            Ok(FightProgress::Fighting(FightControl::default()))
        );
    }

    #[test]
    fn the_enemy_side_losing_its_ship_wins_the_fight() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        assert!(matches!(
            tracker.observe(&live, &step(10)),
            Ok(FightProgress::Fighting(_))
        ));
        let won = BattleSnapshot {
            counters: [1, 0],
            ..live
        };
        let progress = tracker.observe(&won, &step(10)).unwrap();
        let FightProgress::Won(proof) = progress else {
            panic!("expected a win, got {progress:?}");
        };
        assert_eq!(
            proof.trace_label(),
            "battle_won:player_left=1:enemy_left=0:ticks=2"
        );
    }

    #[test]
    fn a_win_is_still_a_win_once_the_battle_flag_has_cleared() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let ended = BattleSnapshot {
            in_battle: false,
            counters: [1, 0],
            ..live
        };
        assert!(matches!(
            tracker.observe(&ended, &step(10)),
            Ok(FightProgress::Won(_))
        ));
    }

    #[test]
    fn losing_the_players_ship_fails_even_when_the_enemy_fell_too() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let both = BattleSnapshot {
            counters: [0, 0],
            ..live
        };
        let error = tracker.observe(&both, &step(10)).unwrap_err();
        assert_eq!(
            error,
            FightError::PlayerShipLost {
                player_left: 0,
                enemy_left: 0
            }
        );
        assert_eq!(
            error.trace_label(),
            "battle_fight_failed:reason=player_ship_lost"
        );
    }

    #[test]
    fn the_tick_budget_ends_a_fight_that_is_not_decided() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        assert!(tracker.observe(&live, &step(3)).is_ok());
        assert!(tracker.observe(&live, &step(3)).is_ok());
        let error = tracker.observe(&live, &step(3)).unwrap_err();
        assert_eq!(error, FightError::Timeout);
        assert_eq!(error.trace_label(), "battle_fight_failed:reason=timeout");
    }

    #[test]
    fn no_battle_fails_at_the_start_and_when_it_ends_undecided() {
        let mut tracker = BattleFightTracker::default();
        let none = BattleSnapshot::none(ARENA);
        let error = tracker.observe(&none, &step(10)).unwrap_err();
        assert_eq!(error, FightError::NotInBattle);
        assert_eq!(
            error.trace_label(),
            "battle_fight_failed:reason=not_in_battle"
        );

        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let ended = BattleSnapshot {
            in_battle: false,
            ..live
        };
        assert_eq!(
            tracker.observe(&ended, &step(10)),
            Err(FightError::NotInBattle)
        );
    }

    #[test]
    fn a_finished_fight_starts_the_next_one_afresh() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(2)).unwrap();
        assert_eq!(tracker.observe(&live, &step(2)), Err(FightError::Timeout));
        // A new fight records its own starting counters and tick count.
        let fresh = snapshot([2, 3], None, None);
        assert!(tracker.observe(&fresh, &step(2)).is_ok());
        let one_down = BattleSnapshot {
            counters: [2, 2],
            ..fresh
        };
        assert!(matches!(
            tracker.observe(&one_down, &step(5)),
            Ok(FightProgress::Won(BattleWon { ticks: 2, .. }))
        ));
    }
}
