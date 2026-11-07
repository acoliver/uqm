//! Hyperspace movement and encounter spawning.

#[cfg(not(test))]
use super::abi::*;
#[cfg(not(test))]
#[cfg(not(test))]
use crate::engine::world::FuelDistance;
#[cfg(not(test))]
use crate::engine::StoryKey;
#[cfg(not(test))]
use crate::engine::{World as WorldOwner, WorldData as WorldFields};

fn move_axis(position: &mut i32, delta: i16, maximum: i32) -> i16 {
    *position -= delta as i32;
    if *position < 0 {
        let clipped = delta.wrapping_add(*position as i16);
        *position = 0;
        clipped
    } else if *position > maximum {
        let clipped = delta.wrapping_add((*position - maximum) as i16);
        *position = maximum;
        clipped
    } else {
        delta
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::*;

    use crate::navigation::get;
    use std::ptr;

    const PERCENT: [u16; 24] = [
        20, 0, 0, 20, 40, 0, 20, 40, 60, 40, 40, 60, 0, 30, 60, 40, 2, 30, 70, 0, 0, 70, 60, 0,
    ];
    pub(crate) unsafe fn width() -> i32 {
        (ScreenWidth - 64) << 5
    }
    pub(crate) unsafe fn height() -> i32 {
        ScreenHeight << 5
    }
    pub(crate) unsafe fn logx(value: i16) -> i32 {
        crate::navigation::hyper_units::log_x(value, ScreenWidth - 64)
    }
    pub(crate) unsafe fn logy(value: i16) -> i32 {
        crate::navigation::hyper_units::log_y(value, ScreenHeight)
    }
    pub(crate) unsafe fn universe_x(value: i32) -> i16 {
        crate::navigation::hyper_units::universe_x(value, ScreenWidth - 64)
    }
    pub(crate) unsafe fn universe_y(value: i32) -> i16 {
        crate::navigation::hyper_units::universe_y(value, ScreenHeight)
    }
    pub(crate) fn sine(angle: u16, magnitude: i32) -> i16 {
        ((crate::math::sinetab[(angle & 63) as usize] as i64 * magnitude as i64) >> SIN_SHIFT)
            as i16
    }
    pub(crate) fn cosine(angle: u16, magnitude: i32) -> i16 {
        sine(angle.wrapping_add(16), magnitude)
    }
    /// # Safety
    /// Called on the game thread when entering hyperspace.
    pub unsafe extern "C" fn rust_reset_hyper_distance() {
        (**WorldOwner::current()).hyper_distance = FuelDistance {
            dx: 0,
            dy: 0,
            extra: 0,
            ticks: 1,
        };
    }

    pub fn move_sis(cx: &mut crate::engine::battle::Cx<'_>, dx: &mut i16, dy: &mut i16) {
        cx.with_legacy(|| {
            // SAFETY: the explicit owner lends the unchanged movement and display operations.
            unsafe {
                let world = WorldFields::current();
                let mut x = move_axis(
                    &mut *ptr::addr_of_mut!((**world).flagship.log_x),
                    *dx,
                    maximum_x(),
                );
                let mut y = move_axis(
                    &mut *ptr::addr_of_mut!((**world).flagship.log_y),
                    *dy,
                    maximum_y(),
                );
                drop(world);
                if x != *dx || y != *dy {
                    *dx = x;
                    *dy = y;
                    crate::app::bridge::enter(|cx| {
                        let mut handle = cx.elements().tail();
                        while let Some(element) = handle {
                            if cx.element(element).flags & PLAYER_SHIP as u16 == 0 {
                                handle = cx.elements().pred(element);
                            } else {
                                cx.element_mut(element).next.location = POINT {
                                    x: ((width() >> 1) - x as i32) as i16,
                                    y: ((height() >> 1) - y as i32) as i16,
                                };
                                break;
                            }
                        }
                    });
                }
                if WorldFields::current().read(|world| world.flagship.fuel_on_board) != 0
                    && WorldOwner::current().read(|world| get(world, StoryKey::ARILOU_SPACE_SIDE))
                        <= 1
                {
                    x = x.wrapping_abs();
                    y = y.wrapping_abs();
                    WorldOwner::current().write(|world| {
                        world.hyper_distance.dx = world.hyper_distance.dx.wrapping_add(x as u16);
                        world.hyper_distance.dy = world.hyper_distance.dy.wrapping_add(y as u16);
                    });
                    let adjusted_x = universe_x(
                        16 * WorldOwner::current().read(|owner| owner.hyper_distance.dx) as i32,
                    ) as u32;
                    let adjusted_y = (9999
                        - universe_y(
                            16 * WorldOwner::current().read(|owner| owner.hyper_distance.dy) as i32,
                        ) as i32) as u32;
                    let distance = square_root(
                        adjusted_x
                            .wrapping_mul(adjusted_x)
                            .wrapping_add(adjusted_y.wrapping_mul(adjusted_y)),
                    )
                    .wrapping_add(WorldOwner::current().read(|owner| owner.hyper_distance.extra));
                    let mut ticks = distance >> 4;
                    if ticks > WorldOwner::current().read(|owner| owner.hyper_distance.ticks) as u16
                    {
                        DeltaSISGauges(
                            0,
                            WorldOwner::current().read(|owner| owner.hyper_distance.ticks) as i16
                                - ticks as i16,
                            0,
                        );
                        if ticks > 255 {
                            (**WorldOwner::current()).hyper_distance.dx = 0;
                            (**WorldOwner::current()).hyper_distance.dy = 0;
                            (**WorldOwner::current()).hyper_distance.extra = distance & 15;
                            ticks = 0;
                        }
                        (**WorldOwner::current()).hyper_distance.ticks = ticks as u8;
                    }
                }
            }
        });
    }
    unsafe fn maximum_x() -> i32 {
        logx(10000) - (logx(1) >> 1) - 1
    }
    unsafe fn maximum_y() -> i32 {
        logy(-1) - (logy(9998) >> 1) - 1
    }

    /// # Safety
    /// Game-thread access to initialized world fleet and encounter queues.
    pub unsafe fn check_hyperspace_encounter(world: &mut crate::engine::World) {
        let universe = POINT {
            x: universe_x(world.data.flagship.log_x),
            y: universe_y(world.data.flagship.log_y),
        };
        let mut handle = world.avail_races_ref().head();
        let mut race = 0usize;
        while let Some(id) = handle {
            if world.activity() & IN_BATTLE as u16 == 0 {
                break;
            }
            let fleets = world.avail_races_ref();
            let fleet = *fleets.get(id);
            handle = fleets.succ(id);
            let mut radius = fleet.actual_strength;
            if radius != 0 {
                let mut flags = 0;
                let mut percent = PERCENT[race];
                if radius != u16::MAX {
                    radius = ((radius as u32 * SPHERE_RADIUS_INCREMENT) >> 1) as u16;
                } else {
                    radius = 20000;
                    if race == SLYLANDRO_SHIP as usize {
                        flags = ONE_SHOT_ENCOUNTER as u8;
                        percent = if (world.story_state(StoryKey::STARBASE_AVAILABLE) as u8) == 0 {
                            100
                        } else {
                            percent
                                * (world.story_state(StoryKey::SLYLANDRO_MULTIPLIER) as u8) as u16
                        };
                    } else if race == MELNORME_SHIP as usize
                        && (world.data.flagship.fuel_on_board == 0
                            || (world.story_state(StoryKey::USED_BROADCASTER) as u8) != 0)
                        && (world.story_state(StoryKey::MELNORME_ANGER) as u8) < 3
                    {
                        percent = if (world.story_state(StoryKey::USED_BROADCASTER) as u8) == 0 {
                            30
                        } else {
                            100
                        };
                        flags = ONE_SHOT_ENCOUNTER as u8;
                    }
                    if world
                        .encounters_ref()
                        .find(|encounter| usize::from(encounter.race_id) == race)
                        .is_some()
                    {
                        percent = 0;
                    }
                    if percent == 100 && race == MELNORME_SHIP as usize {
                        world.set_story_state(StoryKey::BROADCASTER_RESPONSE, 1);
                    }
                }
                let dx = universe.x.wrapping_sub(fleet.loc.x).wrapping_abs() as u16;
                let dy = universe.y.wrapping_sub(fleet.loc.y).wrapping_abs() as u16;
                if dx < radius
                    && dy < radius
                    && dx as u32 * dx as u32 + dy as u32 * (dy as u32)
                        < radius as u32 * radius as u32
                    && (TFB_Random() as u16 % 100) < percent
                {
                    let mut encounters = world.encounters();
                    if let Some(id) = encounters.alloc() {
                        let current = encounters.get_mut(id);
                        current.origin = fleet.loc;
                        current.radius = radius;
                        current.flags = flags;
                        current.race_id = race as u8;
                        encounters.put(id);
                    }
                }
            }
            race += 1;
        }
        world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    }
}

pub fn move_sis(
    cx: &mut crate::engine::battle::Cx<'_>,
    dx: &mut i16,
    dy: &mut i16,
    geometry: super::hyper_units::Geometry,
    mut debit: impl FnMut(&mut crate::engine::battle::Cx<'_>, i16),
) {
    let maximum_x = geometry.log_x(10000) - (geometry.log_x(1) >> 1) - 1;
    let maximum_y = geometry.log_y(-1) - (geometry.log_y(9998) >> 1) - 1;
    let mut x = move_axis(&mut cx.world.data.flagship.log_x, *dx, maximum_x);
    let mut y = move_axis(&mut cx.world.data.flagship.log_y, *dy, maximum_y);
    if x != *dx || y != *dy {
        *dx = x;
        *dy = y;
        let mut handle = cx.elements().tail();
        while let Some(element) = handle {
            if cx.element(element).flags & PLAYER_SHIP as u16 == 0 {
                handle = cx.elements().pred(element);
            } else {
                cx.element_mut(element).next.location = POINT {
                    x: ((geometry.width() >> 1) - i32::from(x)) as i16,
                    y: ((geometry.height() >> 1) - i32::from(y)) as i16,
                };
                break;
            }
        }
    }
    if cx.world.data.flagship.fuel_on_board != 0
        && cx.world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1
    {
        x = x.wrapping_abs();
        y = y.wrapping_abs();
        cx.world.hyper_distance.dx = cx.world.hyper_distance.dx.wrapping_add(x as u16);
        cx.world.hyper_distance.dy = cx.world.hyper_distance.dy.wrapping_add(y as u16);
        let adjusted_x = geometry.universe_x(16 * i32::from(cx.world.hyper_distance.dx)) as u32;
        let adjusted_y = (9999
            - i32::from(geometry.universe_y(16 * i32::from(cx.world.hyper_distance.dy))))
            as u32;
        let distance = square_root(
            adjusted_x
                .wrapping_mul(adjusted_x)
                .wrapping_add(adjusted_y.wrapping_mul(adjusted_y)),
        )
        .wrapping_add(cx.world.hyper_distance.extra);
        let mut ticks = distance >> 4;
        if ticks > u16::from(cx.world.hyper_distance.ticks) {
            debit(cx, i16::from(cx.world.hyper_distance.ticks) - ticks as i16);
            if ticks > 255 {
                cx.world.hyper_distance.dx = 0;
                cx.world.hyper_distance.dy = 0;
                cx.world.hyper_distance.extra = distance & 15;
                ticks = 0;
            }
            cx.world.hyper_distance.ticks = ticks as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_clipping_returns_the_actual_displacement() {
        let mut position = 2;
        assert_eq!(move_axis(&mut position, 5, 100), 2);
        assert_eq!(position, 0);
        position = 98;
        assert_eq!(move_axis(&mut position, -5, 100), -2);
        assert_eq!(position, 100);
        assert_eq!(move_axis(&mut position, 7, 100), 7);
        assert_eq!(position, 93);
    }

    fn movement_game(fuel: u32, side: u32) -> crate::engine::Game {
        let mut game = crate::engine::Game::new();
        game.world.story = Some(crate::engine::world::story::GameState::new());
        game.world
            .set_story_state(StoryKey::ARILOU_SPACE_SIDE, side);
        game.world.data.flagship.fuel_on_board = fuel;
        game.world.data.flagship.log_x = 100000;
        game.world.data.flagship.log_y = 100000;
        game.world.hyper_distance.ticks = 1;
        game
    }

    fn move_retained(
        game: &mut crate::engine::Game,
        mut dx: i16,
        mut dy: i16,
    ) -> Vec<(i16, u16, u16, u16, u8)> {
        let mut battle = crate::engine::battle::Battle::new();
        let mut cx =
            crate::engine::battle::Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
                panic!("movement RNG")
            });
        let mut debit = Vec::new();
        move_sis(
            &mut cx,
            &mut dx,
            &mut dy,
            super::super::hyper_units::Geometry::new((320, 240)),
            |cx, amount| {
                let distance = &cx.world.hyper_distance;
                debit.push((
                    amount,
                    distance.dx,
                    distance.dy,
                    distance.extra,
                    distance.ticks,
                ));
                cx.world.data.flagship.fuel_on_board =
                    (cx.world.data.flagship.fuel_on_board as i32 + i32::from(amount)) as u32;
            },
        );
        debit
    }

    #[test]
    fn retained_movement_keeps_initial_tick_and_skips_fuel_outside_hyperspace() {
        let mut game = movement_game(1000, 0);
        assert!(move_retained(&mut game, 1, -1).is_empty());
        assert_eq!(
            (
                game.world.hyper_distance.dx,
                game.world.hyper_distance.dy,
                game.world.hyper_distance.ticks
            ),
            (1, 1, 1)
        );
        for (fuel, side) in [(0, 0), (1000, 2), (1000, 3)] {
            let mut game = movement_game(fuel, side);
            assert!(move_retained(&mut game, 2000, -1000).is_empty());
            assert_eq!(
                (
                    game.world.data.flagship.log_x,
                    game.world.data.flagship.log_y
                ),
                (98000, 101000)
            );
            assert_eq!(
                (
                    game.world.hyper_distance.dx,
                    game.world.hyper_distance.dy,
                    game.world.hyper_distance.ticks
                ),
                (0, 0, 1)
            );
        }
    }

    #[test]
    fn fuel_debit_precedes_tick_commit_and_residual_rollover() {
        let mut game = movement_game(1000, 0);
        assert_eq!(
            move_retained(&mut game, 2000, 0),
            vec![(-124, 2000, 0, 0, 1)]
        );
        assert_eq!(
            (
                game.world.hyper_distance.ticks,
                game.world.data.flagship.fuel_on_board
            ),
            (125, 876)
        );
        let mut game = movement_game(1000, 0);
        assert_eq!(
            move_retained(&mut game, 5001, 0),
            vec![(-312, 5001, 0, 0, 1)]
        );
        assert_eq!(
            (
                game.world.hyper_distance.dx,
                game.world.hyper_distance.dy,
                game.world.hyper_distance.extra,
                game.world.hyper_distance.ticks
            ),
            (0, 0, 1, 0)
        );
        assert_eq!(move_retained(&mut game, 20, 0), vec![(-1, 20, 0, 1, 0)]);
    }

    #[test]
    fn clipped_movement_repositions_the_last_player_ship_with_actual_deltas() {
        let mut game = movement_game(0, 0);
        game.world.data.flagship.log_x = 2;
        game.world.data.flagship.log_y = 3;
        game.kernel.reset_element_capacity(2);
        let mut battle = crate::engine::battle::Battle::new();
        let mut cx =
            crate::engine::battle::Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
                panic!("movement RNG")
            });
        let ship = cx.alloc_element().unwrap();
        cx.element_mut(ship).flags = PLAYER_SHIP as u16;
        cx.put(ship);
        let other = cx.alloc_element().unwrap();
        cx.put(other);
        let (mut dx, mut dy) = (5, 6);
        move_sis(
            &mut cx,
            &mut dx,
            &mut dy,
            super::super::hyper_units::Geometry::new((320, 240)),
            |_, _| panic!("zero-fuel debit"),
        );
        assert_eq!((dx, dy), (2, 3));
        assert_eq!(
            (cx.world.data.flagship.log_x, cx.world.data.flagship.log_y),
            (0, 0)
        );
        assert_eq!(
            (
                cx.element(ship).next.location.x,
                cx.element(ship).next.location.y
            ),
            (4094, 3837)
        );
        assert_eq!(
            (
                cx.element(other).next.location.x,
                cx.element(other).next.location.y
            ),
            (0, 0)
        );
    }
}
