use super::*;
use uqm_engine::graphics::geometry::Point;

struct Body {
    state_flags: u16,
    velocity: VelocityDesc,
    current: Point,
    next: Point,
}

impl Body {
    fn motion(&mut self) -> Motion<'_> {
        Motion {
            flags: &mut self.state_flags,
            velocity: &mut self.velocity,
        }
    }
}

fn body() -> Body {
    Body {
        state_flags: 0,
        velocity: VelocityDesc::new(),
        current: Point { x: 0, y: 0 },
        next: Point { x: 0, y: 0 },
    }
}

fn sample(random: &mut impl FnMut() -> u32, case: usize) -> (Body, u8, (u8, u8)) {
    let mut e = body();
    e.current = Point {
        x: random() as i16,
        y: random() as i16,
    };
    e.next = if case.is_multiple_of(3) {
        e.current
    } else {
        Point {
            x: random() as i16,
            y: random() as i16,
        }
    };
    let mass = (random() % 255 + 1) as u8;
    e.state_flags = if random() & 1 != 0 {
        PLAYER_SHIP as u16
    } else {
        0
    };
    if case.is_multiple_of(2) {
        e.state_flags |= DEFY_PHYSICS as u16;
    }
    e.velocity
        .set_components(i32::from(random() as i16), i32::from(random() as i16));
    let waits = ((random() % 5) as u8, (random() % 7) as u8);
    (e, mass, waits)
}

#[test]
fn collision_vectors_match_original_fixed_width_response() {
    // Recorded from the original pointer-based collide_native, before replacing it.
    let expected = [
        (-1098458400, [55, 47], [(0, 0), (0, 0)]),
        (1088987900, [18, 50], [(-21739, 368), (-14441, 16394)]),
        (57070104, [73, 39], [(-29783, 24820), (15424, -16005)]),
        (4553580, [25, 57], [(-14125, 14358), (7289, 27577)]),
        (4357008, [61, 29], [(22357, -26710), (5480, 16072)]),
        (-76273560, [56, 24], [(18977, -8948), (-5094, -32355)]),
        (-496959360, [67, 80], [(0, 0), (0, 0)]),
        (192720528, [5, 37], [(-11010, 27811), (-27129, 23482)]),
    ];
    let mut seed = 0x12345678u32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    for (case, (expected_scalar, expected_angles, velocities)) in expected.into_iter().enumerate() {
        let (mut a, ma, wa) = sample(&mut random, case);
        let (mut b, mb, wb) = sample(&mut random, case);
        let locations = [(a.current, a.next), (b.current, b.next)];
        let (scalar, angles) = impulse(&mut a.motion(), &mut b.motion(), locations, [ma, mb]);
        assert_eq!(scalar, expected_scalar, "case {case}");
        assert_eq!(angles, expected_angles, "case {case}");
        for (i, (e, mass, mut waits)) in
            [(&mut a, ma, wa), (&mut b, mb, wb)].into_iter().enumerate()
        {
            if movable(mass) {
                respond(
                    &mut e.motion(),
                    mass,
                    i16::from(ma) + i16::from(mb),
                    angles[i],
                    scalar,
                    &mut waits,
                );
            }
            let mut expected_velocity = VelocityDesc::new();
            expected_velocity.set_components(velocities[i].0, velocities[i].1);
            assert_eq!(e.velocity, expected_velocity, "case {case}, body {i}");
            if case.is_multiple_of(3) {
                assert_ne!(e.state_flags & (DEFY_PHYSICS | COLLISION) as u16, 0);
            }
        }
    }
}

#[test]
fn response_narrows_getter_and_sum_before_speed_floor() {
    let mut e = body();
    e.velocity.vector.width = 1024;
    respond(&mut e.motion(), 1, 2, QUADRANT as i16, 0, &mut (0, 0));
    assert_eq!(e.velocity.get_current_components(), (127, 0));
    e.velocity.set_components(32760, 0);
    respond(&mut e.motion(), 1, 2, QUADRANT as i16, 40, &mut (0, 0));
    assert_eq!(e.velocity.get_current_components(), (-32756, 0));
}

#[test]
fn gravity_mass_boundary_matches_original_plus_one_comparison() {
    let limit = (MAX_SHIP_MASS * 10) as u8;
    assert!(movable(limit - 1));
    assert!(!movable(limit));
    assert!(!movable(u8::MAX));
}

#[test]
fn repeated_stationary_collision_rotates_impact_and_zeros_velocity() {
    let mut a = body();
    let mut b = body();
    a.velocity.set_components(160, -80);
    b.velocity.set_components(-96, 64);
    let travel = [a.velocity.travel_angle, b.velocity.travel_angle];
    a.state_flags = DEFY_PHYSICS as u16;
    b.state_flags = DEFY_PHYSICS as u16;
    let locations = [(a.current, a.next), (b.current, b.next)];
    let (_, angles) = impulse(&mut a.motion(), &mut b.motion(), locations, [1, 1]);
    assert_eq!(angles, travel.map(|v| (v + (HALF_CIRCLE - OCTANT)) as i16));
    assert_eq!(a.velocity, VelocityDesc::new());
    assert_eq!(b.velocity, VelocityDesc::new());
    assert_eq!(a.state_flags, (DEFY_PHYSICS | COLLISION) as u16);
    assert_eq!(b.state_flags, (DEFY_PHYSICS | COLLISION) as u16);
}

#[test]
fn collision_waits_add_without_clamping_and_skip_defying_or_non_ship_bodies() {
    for (flags, initial, expected) in [
        (PLAYER_SHIP, (0, 0), (1, 3)),
        (PLAYER_SHIP, (2, 1), (2, 4)),
        (PLAYER_SHIP, (1, 3), (1, 3)),
        (PLAYER_SHIP | DEFY_PHYSICS, (0, 0), (0, 0)),
        (0, (0, 0), (0, 0)),
    ] {
        let mut e = body();
        e.state_flags = flags as u16;
        let mut waits = initial;
        respond(&mut e.motion(), 1, 2, QUADRANT as i16, 0, &mut waits);
        assert_eq!(waits, expected);
    }
}
