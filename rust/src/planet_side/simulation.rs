//! Pure per-tick lander movement, firing, and takeoff reducer.

use crate::battle::battle_types::{cosine, facing_to_angle, normalize_facing, sine};
use crate::battle::velocity::{world_to_velocity, VelocityDesc};

use super::hazards::SoundCue;
use super::model::{CrewCount, LanderUpgrades, SurfacePoint};

const TURN_WAIT: u8 = 2;
const FIRE_WAIT: u8 = 15;
const SHOT_LIFE: u8 = 12;
const LANDER_SPEED_DENOMINATOR: i32 = 10;
const NORMAL_SPEED: i32 = 2 * 8;
const IMPROVED_SPEED: i32 = 2 * 14;
const SHOT_SPEED: i32 = 2 * 3;

/// Input held for one canonical 35-Hz simulation tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameInput {
    pub turn_left: bool,
    pub turn_right: bool,
    pub thrust: bool,
    pub fire: bool,
    pub takeoff: bool,
    pub abort: bool,
}

/// Projectile created by the lander reducer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    pub position: SurfacePoint,
    pub facing: u8,
    pub velocity_x: i32,
    pub velocity_y: i32,
    pub life: u8,
}

/// Typed gameplay effects emitted in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationEffect {
    Play(SoundCue),
    SpawnShot(Shot),
}

/// Active lander state required by movement and weapon rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanderState {
    pub position: SurfacePoint,
    pub facing: u8,
    pub crew: CrewCount,
    pub velocity: VelocityDesc,
    pub turn_wait: u8,
    pub weapon_wait: u8,
    pub in_transit: bool,
    pub upgrades: LanderUpgrades,
}

impl LanderState {
    #[must_use]
    pub fn new(
        position: SurfacePoint,
        facing: u8,
        crew: CrewCount,
        upgrades: LanderUpgrades,
    ) -> Self {
        let facing = normalize_facing(u16::from(facing)) as u8;
        let mut state = Self {
            position,
            facing,
            crew,
            velocity: VelocityDesc::new(),
            turn_wait: 0,
            weapon_wait: 0,
            in_transit: false,
            upgrades,
        };
        state.update_velocity();
        state
    }

    fn update_velocity(&mut self) {
        let speed = if self.upgrades.improved_speed {
            IMPROVED_SPEED
        } else {
            NORMAL_SPEED
        };
        let angle = facing_to_angle(u16::from(self.facing));
        self.velocity.set_components(
            cosine(angle, world_to_velocity(speed)) / LANDER_SPEED_DENOMINATOR,
            sine(angle, world_to_velocity(speed)) / LANDER_SPEED_DENOMINATOR,
        );
    }
}

/// Result of reducing one frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickResult {
    Continue(Vec<SimulationEffect>),
    Takeoff,
    Aborted,
}

/// Reduce one canonical lander gameplay tick.
pub fn tick(state: &mut LanderState, input: FrameInput) -> TickResult {
    if input.abort {
        return TickResult::Aborted;
    }
    if state.crew.get() > 0 && (input.takeoff || state.in_transit) {
        return TickResult::Takeoff;
    }

    let mut effects = Vec::new();
    if state.crew.get() == 0 {
        return TickResult::Continue(effects);
    }

    if state.turn_wait > 0 {
        state.turn_wait -= 1;
    } else if input.turn_left || input.turn_right {
        if input.turn_left {
            state.facing = normalize_facing(u16::from(state.facing).wrapping_sub(1)) as u8;
        } else {
            state.facing = normalize_facing(u16::from(state.facing).wrapping_add(1)) as u8;
        }
        state.update_velocity();
        state.turn_wait = TURN_WAIT;
    }

    let inherited_velocity = if input.thrust {
        state.velocity.get_current_components()
    } else {
        (0, 0)
    };
    if input.thrust {
        let (dx, dy) = state.velocity.get_next_components(1);
        state.position.x += dx;
        state.position.y += dy;
    }

    if state.weapon_wait > 0 {
        state.weapon_wait -= 1;
    } else if input.fire {
        let angle = facing_to_angle(u16::from(state.facing));
        effects.push(SimulationEffect::SpawnShot(Shot {
            position: state.position,
            facing: state.facing,
            velocity_x: cosine(angle, world_to_velocity(SHOT_SPEED)) + inherited_velocity.0,
            velocity_y: sine(angle, world_to_velocity(SHOT_SPEED)) + inherited_velocity.1,
            life: SHOT_LIFE,
        }));
        effects.push(SimulationEffect::Play(SoundCue::LanderShoots));
        state.weapon_wait = if state.upgrades.improved_shot {
            FIRE_WAIT >> 1
        } else {
            FIRE_WAIT
        };
    }

    TickResult::Continue(effects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(upgrades: LanderUpgrades) -> LanderState {
        LanderState::new(
            SurfacePoint { x: 100, y: 100 },
            0,
            CrewCount::new(12),
            upgrades,
        )
    }

    #[test]
    fn turn_uses_two_frame_cooldown() {
        let mut lander = state(LanderUpgrades::default());
        let left = FrameInput {
            turn_left: true,
            ..FrameInput::default()
        };
        tick(&mut lander, left);
        assert_eq!(lander.facing, 15);
        tick(&mut lander, left);
        tick(&mut lander, left);
        assert_eq!(lander.facing, 15);
        tick(&mut lander, left);
        assert_eq!(lander.facing, 14);
    }

    #[test]
    fn no_thrust_means_no_displacement() {
        let mut lander = state(LanderUpgrades::default());
        let start = lander.position;
        tick(&mut lander, FrameInput::default());
        assert_eq!(lander.position, start);
    }

    #[test]
    fn improved_speed_moves_farther() {
        let mut normal = state(LanderUpgrades::default());
        let mut improved = state(LanderUpgrades {
            improved_speed: true,
            ..LanderUpgrades::default()
        });
        let input = FrameInput {
            thrust: true,
            ..FrameInput::default()
        };
        tick(&mut normal, input);
        tick(&mut improved, input);
        assert!(
            (improved.position.y - 100).abs() > (normal.position.y - 100).abs(),
            "improved lander should travel farther per tick"
        );
    }

    #[test]
    fn firing_spawns_twelve_tick_shot_and_sound() {
        let mut lander = state(LanderUpgrades::default());
        let result = tick(
            &mut lander,
            FrameInput {
                fire: true,
                ..FrameInput::default()
            },
        );
        let TickResult::Continue(effects) = result else {
            panic!("expected active session");
        };
        assert!(matches!(
            effects.as_slice(),
            [
                SimulationEffect::SpawnShot(Shot { life: 12, .. }),
                SimulationEffect::Play(SoundCue::LanderShoots)
            ]
        ));
        assert_eq!(lander.weapon_wait, 15);
    }

    #[test]
    fn thrust_velocity_is_inherited_by_shot() {
        let mut stationary_fire = state(LanderUpgrades::default());
        let mut thrust_fire = stationary_fire.clone();
        let TickResult::Continue(stationary_effects) = tick(
            &mut stationary_fire,
            FrameInput {
                fire: true,
                ..FrameInput::default()
            },
        ) else {
            unreachable!()
        };
        let TickResult::Continue(thrust_effects) = tick(
            &mut thrust_fire,
            FrameInput {
                thrust: true,
                fire: true,
                ..FrameInput::default()
            },
        ) else {
            unreachable!()
        };
        let SimulationEffect::SpawnShot(stationary) = stationary_effects[0] else {
            unreachable!()
        };
        let SimulationEffect::SpawnShot(thrusting) = thrust_effects[0] else {
            unreachable!()
        };
        assert_ne!(thrusting.velocity_y, stationary.velocity_y);
    }

    #[test]
    fn improved_shot_uses_seven_tick_cooldown() {
        let mut lander = state(LanderUpgrades {
            improved_shot: true,
            ..LanderUpgrades::default()
        });
        tick(
            &mut lander,
            FrameInput {
                fire: true,
                ..FrameInput::default()
            },
        );
        assert_eq!(lander.weapon_wait, 7);
    }

    #[test]
    fn takeoff_requires_surviving_crew_but_abort_does_not() {
        let mut alive = state(LanderUpgrades::default());
        assert_eq!(
            tick(
                &mut alive,
                FrameInput {
                    takeoff: true,
                    ..FrameInput::default()
                }
            ),
            TickResult::Takeoff
        );

        let mut dead = state(LanderUpgrades::default());
        dead.crew = CrewCount::new(0);
        assert_eq!(
            tick(
                &mut dead,
                FrameInput {
                    takeoff: true,
                    ..FrameInput::default()
                }
            ),
            TickResult::Continue(Vec::new())
        );
        assert_eq!(
            tick(
                &mut dead,
                FrameInput {
                    abort: true,
                    ..FrameInput::default()
                }
            ),
            TickResult::Aborted
        );
    }
}
