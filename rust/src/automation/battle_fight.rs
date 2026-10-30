//! Fighting a full-game battle with the same keys a person would press.
//!
//! `fight_battle` steers the flagship (side 0, the human player's) toward the
//! opposing ship and fires until the battle has ended with the opposing side
//! one ship down and the flagship still alive. It reads the running battle
//! through `BattleSnapshot` and never writes game state, only the player-one
//! keys.
//!
//! The decisions are pure: the snapshot is passed in, so aiming, firing, the
//! outcome and every failure are testable without a linked game.

use crate::automation::script::FightBattleStep;
use crate::battle::battle_types::{
    angle_to_facing, arctan, cosine, facing_to_angle, shortest_path_delta, sine,
};

const FACINGS: i32 = 16;
/// `NUM_SIDES`: side 0 is the human player's, side 1 the opponent's.
const SIDES: usize = 2;
const PLAYER_SIDE: usize = 0;
const ENEMY_SIDE: usize = 1;

/// The flagship's ion bolt flies `BLASTER_SPEED` (24 display units, 96 world
/// units) for `BLASTER_LIFE` (12) frames: 1152 world units.
const BOLT_RANGE: i64 = 1152;
/// Hull radius of the Avenger in world units: about 16 space pixels from its
/// large avenger frame at 4 world units a pixel. The frame sizes are in the
/// content package, which the Rust side does not read, so this is a
/// conservative estimate and not a measured extent.
const ENEMY_RADIUS: i64 = 64;
/// Radius of the ion bolt in world units: about 4 space pixels at 4 world
/// units a pixel (same caveat as `ENEMY_RADIUS`).
const BOLT_RADIUS: i64 = 16;
/// A bolt whose line passes this close to the enemy's centre can touch it.
const HIT_RADIUS: i64 = ENEMY_RADIUS + BOLT_RADIUS;
/// One facing off is still a hit on a ship-sized target at any bolt range
/// the fight can fire at: a missed bolt costs one energy, a withheld one
/// costs the fight.
const NEAR_FACING_ERROR: i32 = 1;
/// Two facings off only counts as aimed when the enemy is this close.
const POINT_BLANK: i64 = 250;
/// The Avenger covers 25 world units a frame, so while the flagship needs
/// 18 frames per facing it closes the bolt range in about 46 frames. Only an
/// enemy farther than twice the bolt range is approached.
const APPROACH_RANGE: i64 = 2300;
/// One `battle_fight_state` label per this many ticks.
const TRACE_INTERVAL: u64 = 30;
/// Unit vectors from `sine`/`cosine` are scaled by this (`1 << 14`).
const UNIT: i32 = 1 << 14;

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

impl FightControl {
    /// The held keys as a trace word: `hold`, or `turn_left`, `turn_right`,
    /// `thrust` and `fire` joined by `+`.
    #[must_use]
    pub fn action_label(&self) -> String {
        let held: Vec<&str> = [
            (self.left, "turn_left"),
            (self.right, "turn_right"),
            (self.thrust, "thrust"),
            (self.weapon, "fire"),
        ]
        .into_iter()
        .filter_map(|(on, word)| on.then_some(word))
        .collect();
        if held.is_empty() {
            "hold".to_owned()
        } else {
            held.join("+")
        }
    }
}

/// What the flagship sees of the enemy and the keys it holds because of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Engagement {
    /// World units to the enemy along the shorter way round the arena.
    pub distance: u32,
    /// Facings the bearing is clockwise of the flagship's heading, 0..16.
    pub facing_error: u8,
    pub control: FightControl,
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

/// Whether a bolt fired along `facing` now can hit an enemy at `(dx, dy)`.
///
/// It can when the facing line passes within `HIT_RADIUS` of the enemy's
/// centre before the bolt burns out, or when the facing is within one of the
/// bearing and the enemy is inside bolt range, or within two and at point
/// blank. `error` is the bearing's clockwise offset from the facing, 0..16.
fn bolt_can_hit(facing: u8, error: i32, dx: i32, dy: i32) -> bool {
    let angle = facing_to_angle(u16::from(facing));
    let (fx, fy) = (i64::from(cosine(angle, UNIT)), i64::from(sine(angle, UNIT)));
    let (dx, dy) = (i64::from(dx), i64::from(dy));
    let ahead = dx * fx + dy * fy;
    let miss = (dx * fy - dy * fx).abs();
    let unit = i64::from(UNIT);
    let on_the_line = ahead > 0 && ahead <= BOLT_RANGE * unit && miss <= HIT_RADIUS * unit;
    let distance = (dx * dx + dy * dy).isqrt();
    let off = error.min(FACINGS - error);
    let near_facing = off <= NEAR_FACING_ERROR && distance <= BOLT_RANGE;
    let point_blank = off == NEAR_FACING_ERROR + 1 && distance < POINT_BLANK;
    on_the_line || near_facing || point_blank
}

/// Choose the keys that face the enemy and fire.
///
/// The flagship cannot kite the Avenger (thrust 10 against 25, one facing per
/// 18 frames against one per 3), so it stands its ground and trades blows at
/// range. It turns toward the bearing along the shorter way round and fires
/// whenever the bolt can plausibly hit (`bolt_can_hit`), turning and firing
/// together. It never thrusts inside twice the bolt range: speed it gains
/// cannot be shed and spoils the aim.
#[must_use]
pub fn engage(player: &ShipState, enemy: &ShipState, arena: (i32, i32)) -> Engagement {
    let (dx, dy) = wrapped_delta(player, enemy, arena);
    let squared = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
    let distance = u32::try_from(squared.unsigned_abs().isqrt()).unwrap_or(u32::MAX);
    let Some(bearing) = bearing_facing(dx, dy) else {
        return Engagement {
            distance,
            facing_error: 0,
            control: FightControl {
                weapon: true,
                ..FightControl::default()
            },
        };
    };
    let error = (i32::from(bearing) - i32::from(player.facing)).rem_euclid(FACINGS);
    let weapon = bolt_can_hit(player.facing, error, dx, dy);
    let (right, left) = match error {
        1..=8 => (true, false),
        9..=15 => (false, true),
        _ => (false, false),
    };
    let roughly_aimed = error <= 2 || error >= FACINGS - 2;
    Engagement {
        distance,
        facing_error: u8::try_from(error).unwrap_or(0),
        control: FightControl {
            thrust: roughly_aimed && squared > APPROACH_RANGE * APPROACH_RANGE,
            left,
            right,
            weapon,
        },
    }
}

/// Why the fight could not be won.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightError {
    /// No battle is running, or it ended with neither side losing a ship.
    NotInBattle,
    /// The player's flagship died.
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

/// A battle that ended with the enemy side a ship down and the flagship alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleWon {
    pub player_left: u8,
    pub enemy_left: u8,
    pub ticks: u64,
    /// The flagship's crew at the last tick of the battle.
    pub player_crew: u16,
}

impl BattleWon {
    /// The semantic trace label that records this proof.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "battle_won:player_left={}:enemy_left={}:ticks={}:player_crew={}",
            self.player_left, self.enemy_left, self.ticks, self.player_crew
        )
    }
}

/// A periodic reading of the fight, for tuning the strategy from real runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FightReport {
    pub tick: u64,
    pub engagement: Engagement,
    pub player_crew: u16,
    pub enemy_crew: u16,
}

impl FightReport {
    /// The trace label that records this reading.
    #[must_use]
    pub fn trace_label(&self) -> String {
        format!(
            "battle_fight_state:tick={}:distance={}:facing_error={}:player_crew={}:enemy_crew={}:action={}",
            self.tick,
            self.engagement.distance,
            self.engagement.facing_error,
            self.player_crew,
            self.enemy_crew,
            self.engagement.control.action_label()
        )
    }
}

/// What the player's ship should do after one observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightProgress {
    /// Hold these keys; `report` is set on the ticks that are traced.
    Fighting {
        control: FightControl,
        report: Option<FightReport>,
    },
    /// The battle ended with the flagship alive: release every key.
    Won(BattleWon),
}

#[derive(Debug, Clone, Copy)]
struct FightStart {
    ticks: u64,
    counters: [u8; SIDES],
    /// Counters at the last tick the battle was running.
    last_counters: [u8; SIDES],
    /// The flagship's crew at the last tick it was seen with crew. A ship
    /// element starts with no crew until its first preprocess, so a zero
    /// reading before this is set is not a death.
    last_player_crew: u16,
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
    /// Returns an error when there is no battle, the flagship died, the battle
    /// ended with neither side a ship down, or `max_ticks` observations passed
    /// first.
    pub fn observe(
        &mut self,
        snapshot: &BattleSnapshot,
        step: &FightBattleStep,
    ) -> Result<FightProgress, FightError> {
        let result = self.decide(snapshot, step);
        if !matches!(result, Ok(FightProgress::Fighting { .. })) {
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
            last_counters: snapshot.counters,
            last_player_crew: 0,
        });
        start.ticks += 1;

        if !snapshot.in_battle {
            return Self::battle_ended(start);
        }
        let [player_left, enemy_left] = snapshot.counters;
        start.last_counters = snapshot.counters;
        if let Some(player) = &snapshot.player {
            if player.crew > 0 {
                start.last_player_crew = player.crew;
            } else if start.last_player_crew > 0 {
                return Err(FightError::PlayerShipLost {
                    player_left,
                    enemy_left,
                });
            }
        }
        if player_left < start.counters[PLAYER_SIDE] {
            return Err(FightError::PlayerShipLost {
                player_left,
                enemy_left,
            });
        }
        if start.ticks >= step.max_ticks {
            return Err(FightError::Timeout);
        }
        let enemy_down = enemy_left < start.counters[ENEMY_SIDE];
        let (control, engagement) = match (&snapshot.player, &snapshot.enemy) {
            // The enemy is dead: its flames are the only danger left and
            // nothing the flagship does outruns them, so it stops shooting
            // and waits for the battle to end.
            _ if enemy_down => (FightControl::default(), None),
            (Some(player), Some(enemy)) => {
                let engagement = engage(player, enemy, snapshot.arena);
                (
                    engagement.control,
                    Some((engagement, player.crew, enemy.crew)),
                )
            }
            _ => (FightControl::default(), None),
        };
        let report = engagement
            .filter(|_| start.ticks % TRACE_INTERVAL == 1)
            .map(|(engagement, player_crew, enemy_crew)| FightReport {
                tick: start.ticks,
                engagement,
                player_crew,
                enemy_crew,
            });
        Ok(FightProgress::Fighting { control, report })
    }

    /// Judge a battle whose `IN_BATTLE` flag has cleared from what the last
    /// running tick saw: the ship queue is gone by now.
    fn battle_ended(start: &FightStart) -> Result<FightProgress, FightError> {
        let [player_left, enemy_left] = start.last_counters;
        if start.last_player_crew == 0 {
            return Err(FightError::PlayerShipLost {
                player_left,
                enemy_left,
            });
        }
        if enemy_left >= start.counters[ENEMY_SIDE] {
            return Err(FightError::NotInBattle);
        }
        Ok(FightProgress::Won(BattleWon {
            player_left,
            enemy_left,
            ticks: start.ticks,
            player_crew: start.last_player_crew,
        }))
    }
}

#[cfg(not(test))]
impl BattleSnapshot {
    /// The running battle, read on the game thread.
    ///
    /// # Panics
    ///
    /// Panics when `disp_q` is not a valid element pool while a battle runs:
    /// the game's own queue is broken and no fight can be judged from it.
    #[must_use]
    pub fn live() -> Self {
        let (width, height) = crate::automation::input_ffi::screen_size();
        let arena = arena_size(width, height);
        let activity = crate::mainloop::ffi::get_current_activity().0;
        if activity & crate::mainloop::types::activity_flags::IN_BATTLE == 0 {
            return Self::none(arena);
        }
        // SAFETY: the automation snapshot is a synchronous game-thread operation.
        unsafe { crate::app::bridge::enter(|cx| Self::from_context(cx, arena)) }
    }

    fn from_context(cx: &crate::engine::battle::Cx<'_>, arena: (i32, i32)) -> Self {
        let ([player, enemy], counters) = super::parity_state::fighters(cx);
        Self {
            in_battle: true,
            counters,
            player,
            enemy,
            arena,
        }
    }
}

#[cfg(test)]
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

    fn fight(player: &ShipState, enemy: &ShipState, arena: (i32, i32)) -> FightControl {
        engage(player, enemy, arena).control
    }

    fn fighting(control: FightControl) -> Result<FightProgress, FightError> {
        Ok(FightProgress::Fighting {
            control,
            report: None,
        })
    }

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
                assert!(!control.thrust, "{facing}->{bearing}");
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
    fn an_enemy_beyond_twice_the_bolt_range_is_approached_only_when_roughly_aimed() {
        let enemy = ship(4000 + 2600, 4000, 0);
        let aimed = fight(&ship(4000, 4000, 4), &enemy, ARENA);
        assert!(aimed.thrust);
        assert!(!aimed.weapon, "outside bolt range");
        let off = fight(&ship(4000, 4000, 8), &enemy, ARENA);
        assert!(!off.thrust);
    }

    #[test]
    fn an_enemy_at_mid_range_is_waited_for_not_approached() {
        // Between the bolt range and twice that, the aimed flagship holds
        // still: the Avenger closes by itself and every frame of its approach
        // is a free shot.
        let enemy = ship(4000 + 1500, 4000, 0);
        assert_eq!(
            fight(&ship(4000, 4000, 4), &enemy, ARENA),
            FightControl::default()
        );
        let turning = fight(&ship(4000, 4000, 2), &enemy, ARENA);
        assert!(turning.right && !turning.thrust && !turning.weapon);
    }

    #[test]
    fn an_enemy_approaching_head_on_is_shot_from_the_edge_of_bolt_range() {
        for gap in [1000, 600, 300, 120] {
            let enemy = ship(4000 + gap, 4000, 12);
            let control = fight(&ship(4000, 4000, 4), &enemy, ARENA);
            assert_eq!(
                control,
                FightControl {
                    weapon: true,
                    ..FightControl::default()
                },
                "gap {gap}"
            );
        }
        let out_of_reach = fight(&ship(4000, 4000, 4), &ship(4000 + 1200, 4000, 12), ARENA);
        assert!(!out_of_reach.weapon);
    }

    // Real-window telemetry of the flagship against the Avenger: enemy due
    // east (bearing facing 4), so a player facing of `4 - error` has that
    // facing error.
    fn east_duel(distance: i32, error: i32) -> FightControl {
        let facing = u8::try_from((4 - error).rem_euclid(16)).unwrap();
        fight(
            &ship(4000, 4000, facing),
            &ship(4000 + distance, 4000, 0),
            ARENA,
        )
    }

    #[test]
    fn telemetry_close_and_one_facing_off_fires_and_keeps_turning() {
        // tick 91: distance 147, facing error 15 and the flagship never fired.
        assert_eq!(
            east_duel(147, 15),
            FightControl {
                left: true,
                weapon: true,
                ..FightControl::default()
            }
        );
    }

    #[test]
    fn telemetry_far_off_facings_turn_without_firing() {
        // tick 31: distance 424, facing error 12.
        assert_eq!(
            east_duel(424, 12),
            FightControl {
                left: true,
                ..FightControl::default()
            }
        );
        // tick 1: distance 1067, facing error 9.
        assert_eq!(
            east_duel(1067, 9),
            FightControl {
                left: true,
                ..FightControl::default()
            }
        );
    }

    #[test]
    fn telemetry_aimed_at_300_fires_and_holds_still() {
        assert_eq!(
            east_duel(300, 0),
            FightControl {
                weapon: true,
                ..FightControl::default()
            }
        );
    }

    #[test]
    fn one_facing_off_fires_at_any_distance_inside_bolt_range() {
        for distance in [120, 600, 1000, 1152] {
            let control = east_duel(distance, 1);
            assert!(control.weapon && control.right, "clockwise {distance}");
            let control = east_duel(distance, 15);
            assert!(control.weapon && control.left, "anticlockwise {distance}");
        }
        assert!(!east_duel(1153, 1).weapon);
    }

    #[test]
    fn two_facings_off_fires_only_at_point_blank() {
        assert!(east_duel(249, 2).weapon);
        assert!(east_duel(249, 14).weapon);
        assert!(!east_duel(250, 2).weapon);
        assert!(!east_duel(600, 14).weapon);
    }

    #[test]
    fn an_enemy_off_the_bolt_line_by_more_than_both_radii_is_not_shot_at() {
        // Three facings off, 300 units away: the line passes about 277 units
        // from the Avenger, wider than the 80 unit hit radius.
        assert!(!east_duel(300, 3).weapon);
        assert!(!east_duel(600, 3).weapon);
    }

    #[test]
    fn a_bolt_line_through_the_hull_fires_even_when_the_facings_differ() {
        // Three facings off at 80 units passes about 74 units wide, inside
        // the 80 unit hit radius; at 100 units it passes 92 wide.
        assert!(east_duel(80, 3).weapon);
        assert!(!east_duel(100, 3).weapon);
    }

    #[test]
    fn a_close_enemy_behind_the_flagship_is_turned_to_without_thrust_or_fire() {
        // The Avenger sits 150 units astern. Thrusting away cannot outrun it
        // and the bolt only flies forward, so the flagship turns toward it.
        let enemy = ship(4000 - 150, 4000, 4);
        let control = fight(&ship(4000, 4000, 4), &enemy, ARENA);
        assert_eq!(
            control,
            FightControl {
                right: true,
                ..FightControl::default()
            }
        );
        let from_the_other_side = fight(&ship(4000, 4000, 11), &ship(4000 + 150, 4000, 4), ARENA);
        assert!(
            from_the_other_side.left && !from_the_other_side.thrust && !from_the_other_side.weapon
        );
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
    fn the_engagement_reports_distance_and_facing_error() {
        let engagement = engage(
            &ship(4000, 4000, 2),
            &ship(4000 + 300, 4000 + 400, 0),
            ARENA,
        );
        assert_eq!(engagement.distance, 500);
        // The bearing is facing 6 (143 degrees clockwise from north).
        assert_eq!(engagement.facing_error, 4);
    }

    #[test]
    fn actions_are_named_for_the_keys_held() {
        assert_eq!(FightControl::default().action_label(), "hold");
        assert_eq!(
            FightControl {
                left: true,
                weapon: true,
                ..FightControl::default()
            }
            .action_label(),
            "turn_left+fire"
        );
        assert_eq!(
            FightControl {
                right: true,
                thrust: true,
                ..FightControl::default()
            }
            .action_label(),
            "turn_right+thrust"
        );
    }

    #[test]
    fn a_missing_ship_holds_every_key_until_both_exist() {
        let mut tracker = BattleFightTracker::default();
        let waiting = snapshot([1, 1], None, None);
        assert_eq!(
            tracker.observe(&waiting, &step(10)),
            fighting(FightControl::default())
        );
        let enemy_only = snapshot([1, 1], None, Some(ship(1, 1, 0)));
        assert_eq!(
            tracker.observe(&enemy_only, &step(10)),
            fighting(FightControl::default())
        );
    }

    #[test]
    fn a_ship_with_no_crew_yet_is_not_a_dead_ship() {
        // A ship element starts with crew 0 until its first preprocess.
        let mut tracker = BattleFightTracker::default();
        let spawning = duel(
            ShipState {
                crew: 0,
                ..ship(4000, 4000, 4)
            },
            ship(4600, 4000, 0),
        );
        assert!(tracker.observe(&spawning, &step(10)).is_ok());
        let alive = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        assert!(tracker.observe(&alive, &step(10)).is_ok());
    }

    #[test]
    fn the_enemy_dying_is_not_a_win_until_the_battle_ends() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let enemy_down = BattleSnapshot {
            counters: [1, 0],
            ..live
        };
        // Standing down: no keys while its flames burn out.
        assert_eq!(
            tracker.observe(&enemy_down, &step(10)),
            fighting(FightControl::default())
        );
    }

    #[test]
    fn the_fight_is_won_when_the_battle_ends_with_the_flagship_alive() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let enemy_down = BattleSnapshot {
            counters: [1, 0],
            player: Some(ShipState {
                crew: 14,
                ..ship(4000, 4000, 4)
            }),
            ..live
        };
        tracker.observe(&enemy_down, &step(10)).unwrap();
        // The ship queue is gone once the battle has ended.
        let progress = tracker
            .observe(&BattleSnapshot::none(ARENA), &step(10))
            .unwrap();
        let FightProgress::Won(proof) = progress else {
            panic!("expected a win, got {progress:?}");
        };
        assert_eq!(
            proof.trace_label(),
            "battle_won:player_left=1:enemy_left=0:ticks=3:player_crew=14"
        );
    }

    #[test]
    fn the_flagship_dying_after_the_enemy_before_the_battle_ends_fails() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let enemy_down = BattleSnapshot {
            counters: [1, 0],
            ..live
        };
        tracker.observe(&enemy_down, &step(10)).unwrap();
        let both_down = BattleSnapshot {
            counters: [0, 0],
            ..live
        };
        let error = tracker.observe(&both_down, &step(10)).unwrap_err();
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
    fn the_flagship_losing_its_last_crew_fails_at_once() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(10)).unwrap();
        let dead = duel(
            ShipState {
                crew: 0,
                ..ship(4000, 4000, 4)
            },
            ship(4600, 4000, 0),
        );
        assert!(matches!(
            tracker.observe(&dead, &step(10)),
            Err(FightError::PlayerShipLost { .. })
        ));
    }

    #[test]
    fn a_battle_that_ended_after_the_flagship_died_is_not_a_win() {
        let mut tracker = BattleFightTracker::default();
        let spawning = duel(
            ShipState {
                crew: 0,
                ..ship(4000, 4000, 4)
            },
            ship(4600, 4000, 0),
        );
        tracker.observe(&spawning, &step(10)).unwrap();
        let enemy_down = BattleSnapshot {
            counters: [1, 0],
            ..spawning
        };
        tracker.observe(&enemy_down, &step(10)).unwrap();
        assert!(matches!(
            tracker.observe(&BattleSnapshot::none(ARENA), &step(10)),
            Err(FightError::PlayerShipLost { .. })
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
        assert!(matches!(
            tracker.observe(&both, &step(10)),
            Err(FightError::PlayerShipLost { .. })
        ));
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
    fn the_state_label_is_traced_on_the_first_tick_and_every_thirty_ticks() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4000 + 600, 4000, 0));
        let mut labels = Vec::new();
        for tick in 1..=95u64 {
            let Ok(FightProgress::Fighting { report, .. }) = tracker.observe(&live, &step(200))
            else {
                panic!("tick {tick} ended the fight");
            };
            if let Some(report) = report {
                labels.push(report.trace_label());
            }
        }
        assert_eq!(
            labels,
            [
                "battle_fight_state:tick=1:distance=600:facing_error=0:player_crew=10:enemy_crew=10:action=fire",
                "battle_fight_state:tick=31:distance=600:facing_error=0:player_crew=10:enemy_crew=10:action=fire",
                "battle_fight_state:tick=61:distance=600:facing_error=0:player_crew=10:enemy_crew=10:action=fire",
                "battle_fight_state:tick=91:distance=600:facing_error=0:player_crew=10:enemy_crew=10:action=fire",
            ]
        );
    }

    #[test]
    fn a_finished_fight_starts_the_next_one_afresh() {
        let mut tracker = BattleFightTracker::default();
        let live = duel(ship(4000, 4000, 4), ship(4600, 4000, 0));
        tracker.observe(&live, &step(2)).unwrap();
        assert_eq!(tracker.observe(&live, &step(2)), Err(FightError::Timeout));
        // A new fight records its own starting counters and tick count.
        let fresh = BattleSnapshot {
            counters: [2, 3],
            ..duel(ship(4000, 4000, 4), ship(4600, 4000, 0))
        };
        assert!(tracker.observe(&fresh, &step(5)).is_ok());
        let one_down = BattleSnapshot {
            counters: [2, 2],
            ..fresh
        };
        assert!(tracker.observe(&one_down, &step(5)).is_ok());
        assert!(matches!(
            tracker.observe(&BattleSnapshot::none(ARENA), &step(5)),
            Ok(FightProgress::Won(BattleWon {
                ticks: 3,
                player_left: 2,
                enemy_left: 2,
                ..
            }))
        ));
    }
}
