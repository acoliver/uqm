// Battle Types — Shared Foundation
// @plan PLAN-20260320-BATTLE.P03, P04
// @requirement Coordinate, angle, trigonometry, and velocity constants extracted from ships/runtime.rs

// ---------------------------------------------------------------------------
// Angle/Facing Constants
// ---------------------------------------------------------------------------

pub const FACING_SHIFT: u32 = 4;
pub const NUM_FACINGS: u16 = 1 << FACING_SHIFT; // 16
pub const CIRCLE_SHIFT: u32 = 6;
pub const FULL_CIRCLE: u16 = 1 << CIRCLE_SHIFT; // 64
pub const HALF_CIRCLE: u16 = FULL_CIRCLE >> 1; // 32
pub const QUADRANT: u16 = FULL_CIRCLE >> 2; // 16
pub const OCTANT: u16 = FULL_CIRCLE >> 3; // 8

// ---------------------------------------------------------------------------
// Coordinate Constants
// ---------------------------------------------------------------------------

pub const ONE_SHIFT: u32 = 2;
pub const SCALED_ONE: i32 = 1 << ONE_SHIFT; // 4
pub const MAX_REDUCTION: u32 = 3;
pub const MAX_VIS_REDUCTION: u32 = 2;

// Battle space dimensions (logical coordinates)
// LOG_SPACE_WIDTH = DISPLAY_TO_WORLD(SPACE_WIDTH) << MAX_REDUCTION
// Assuming SPACE_WIDTH = 256 (screen width - 64 status panel)
// LOG_SPACE_WIDTH = ((256 << 2) << 3) = 8192
pub const LOG_SPACE_WIDTH: i32 = 8192;
pub const LOG_SPACE_HEIGHT: i32 = 8192;

// Viewport dimensions (for transitions)
pub const TRANSITION_WIDTH: i32 = LOG_SPACE_WIDTH >> (MAX_REDUCTION - MAX_VIS_REDUCTION);
pub const TRANSITION_HEIGHT: i32 = LOG_SPACE_HEIGHT >> (MAX_REDUCTION - MAX_VIS_REDUCTION);

// Universe coordinates (0-9999 on each axis)
pub const MAX_X_UNIVERSE: u16 = 9999;
pub const MAX_Y_UNIVERSE: u16 = 9999;

// Status panel width
pub const STATUS_WIDTH: i32 = 64;

// Frame rate
pub const BATTLE_FRAME_RATE: u16 = 24;

// ---------------------------------------------------------------------------
// Element Constants (imported from element module)
// ---------------------------------------------------------------------------
// Note: These are defined in element.rs and re-exported through mod.rs

// ---------------------------------------------------------------------------
// Conversion Functions
// ---------------------------------------------------------------------------

pub const fn normalize_facing(f: u16) -> u16 {
    f & (NUM_FACINGS - 1)
}

pub const fn facing_to_angle(f: u16) -> u16 {
    f << (CIRCLE_SHIFT - FACING_SHIFT)
}

pub const fn angle_to_facing(a: u16) -> u16 {
    (a + (1 << (CIRCLE_SHIFT - FACING_SHIFT - 1))) >> (CIRCLE_SHIFT - FACING_SHIFT)
}

pub const fn normalize_angle(a: u16) -> u16 {
    a & (FULL_CIRCLE - 1)
}

pub const fn display_to_world(x: i32) -> i32 {
    x << ONE_SHIFT
}

// world_to_velocity and velocity_to_world are defined in velocity.rs
// gravity_mass is defined in element.rs
// These are re-exported through mod.rs

// ---------------------------------------------------------------------------
// Toroidal Wrapping Functions
// ---------------------------------------------------------------------------

/// Wraps an X coordinate to the toroidal space boundary.
/// The battle arena wraps at power-of-2 boundaries.
///
/// # Arguments
/// * `x` - The X coordinate in world units
/// * `log_width` - Log2 of the arena width (e.g., 10 for 1024)
///
/// # Returns
/// The wrapped X coordinate
pub fn wrap_x(x: i32, log_width: u32) -> i32 {
    let width = 1i32 << log_width;
    let mask = width - 1;
    x & mask
}

/// Wraps a Y coordinate to the toroidal space boundary.
/// The battle arena wraps at power-of-2 boundaries.
///
/// # Arguments
/// * `y` - The Y coordinate in world units
/// * `log_height` - Log2 of the arena height (e.g., 10 for 1024)
///
/// # Returns
/// The wrapped Y coordinate
pub fn wrap_y(y: i32, log_height: u32) -> i32 {
    let height = 1i32 << log_height;
    let mask = height - 1;
    y & mask
}

/// Computes the shortest path delta in a toroidal space.
///
/// If the absolute delta is greater than half the space dimension,
/// the shorter path wraps around the boundary.
///
/// # Arguments
/// * `from` - Starting coordinate
/// * `to` - Ending coordinate
/// * `size` - Total size of the dimension
///
/// # Returns
/// The shortest delta (may be negative)
pub fn shortest_path_delta(from: i32, to: i32, size: i32) -> i32 {
    let mut delta = to - from;
    let half_size = size / 2;

    if delta > half_size {
        delta -= size;
    } else if delta < -half_size {
        delta += size;
    }

    delta
}

// ---------------------------------------------------------------------------
// Trigonometry Tables and Functions
// ---------------------------------------------------------------------------

// Quarter-circle sine table (16 entries for 0-90 degrees)
// Scaled by 16384 (1 << 14) to match C FLT_ADJUST
const SIN_SHIFT: u32 = 14;
/// Full 64-entry sine table from C trans.c (lines 23-89)
/// UQM angle system: 0=North(-Y), 16=East(+X), 32=South(+Y), 48=West(-X)
pub const SINE_TABLE: [i32; 64] = [
    -16384, // angle 0 = North (-Y) = -1.0
    -16305, -16069, -15679, -15137, -14449, -13623, -12665, -11585, // angle 8 = NE
    -10394, -9102, -7723, -6270, -4756, -3197, -1608, 0, // angle 16 = East (+X) = 0.0
    1608, 3197, 4756, 6270, 7723, 9102, 10394, 11585, // angle 24 = SE
    12665, 13623, 14449, 15137, 15679, 16069, 16305, 16384, // angle 32 = South (+Y) = +1.0
    16305, 16069, 15679, 15137, 14449, 13623, 12665, 11585, // angle 40 = SW
    10394, 9102, 7723, 6270, 4756, 3197, 1608, 0, // angle 48 = West (-X) = 0.0
    -1608, -3197, -4756, -6270, -7723, -9102, -10394, -11585, // angle 56 = NW
    -12665, -13623, -14449, -15137, -15679, -16069, -16305,
];

pub fn sine(angle: u16, magnitude: i32) -> i32 {
    let angle = normalize_angle(angle) as usize;
    let sin_val = SINE_TABLE[angle];
    ((sin_val as i64 * magnitude as i64) >> SIN_SHIFT) as i32
}

pub fn cosine(angle: u16, magnitude: i32) -> i32 {
    sine(angle + QUADRANT, magnitude)
}

pub fn arctan(dx: i32, dy: i32) -> u16 {
    // ARCTAN table from C trans.c (33 entries)
    const ATAN_TABLE: [u16; 33] = [
        0, 0, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7,
        8, 8, 8,
    ];

    if dx == 0 && dy == 0 {
        return FULL_CIRCLE; // Special case for zero vector
    }

    let v1 = dx.abs();
    let v2 = dy.abs();

    // Compute angle index using ATAN table
    // This matches the C implementation exactly
    let v1_result = if v1 > v2 {
        let ratio = (((v2 as u64) << (CIRCLE_SHIFT - 1)) + ((v1 >> 1) as u64)) / (v1 as u64);
        let index = ratio.min((ATAN_TABLE.len() - 1) as u64) as usize;
        QUADRANT - ATAN_TABLE[index]
    } else {
        let ratio = (((v1 as u64) << (CIRCLE_SHIFT - 1)) + ((v2 >> 1) as u64)) / (v2 as u64);
        let index = ratio.min((ATAN_TABLE.len() - 1) as u64) as usize;
        ATAN_TABLE[index]
    };

    // Adjust for quadrant based on sign of dx and dy
    let mut result = v1_result;
    if dx < 0 {
        result = FULL_CIRCLE.wrapping_sub(result) & 63;
    }
    if dy > 0 {
        result = HALF_CIRCLE.wrapping_sub(result) & 63;
    }

    normalize_angle(result)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Constants -----------------------------------------------------------

    #[test]
    fn constants_match_c() {
        assert_eq!(NUM_FACINGS, 16);
        assert_eq!(FULL_CIRCLE, 64);
        assert_eq!(HALF_CIRCLE, 32);
        assert_eq!(QUADRANT, 16);
        assert_eq!(OCTANT, 8);
        assert_eq!(ONE_SHIFT, 2);
    }

    // Element constants are tested in element.rs

    #[test]
    fn battle_space_constants() {
        assert_eq!(SCALED_ONE, 4);
        assert_eq!(MAX_REDUCTION, 3);
        assert_eq!(MAX_VIS_REDUCTION, 2);
        assert_eq!(LOG_SPACE_WIDTH, 8192);
        assert_eq!(LOG_SPACE_HEIGHT, 8192);
        assert_eq!(
            TRANSITION_WIDTH,
            LOG_SPACE_WIDTH >> (MAX_REDUCTION - MAX_VIS_REDUCTION)
        );
        assert_eq!(
            TRANSITION_HEIGHT,
            LOG_SPACE_HEIGHT >> (MAX_REDUCTION - MAX_VIS_REDUCTION)
        );
    }

    #[test]
    fn universe_constants() {
        assert_eq!(MAX_X_UNIVERSE, 9999);
        assert_eq!(MAX_Y_UNIVERSE, 9999);
    }

    #[test]
    fn display_constants() {
        assert_eq!(STATUS_WIDTH, 64);
        assert_eq!(BATTLE_FRAME_RATE, 24);
    }

    #[test]
    fn normalize_facing_wraps() {
        assert_eq!(normalize_facing(0), 0);
        assert_eq!(normalize_facing(15), 15);
        assert_eq!(normalize_facing(16), 0);
        assert_eq!(normalize_facing(17), 1);
    }

    #[test]
    fn facing_to_angle_conversion() {
        assert_eq!(facing_to_angle(0), 0);
        assert_eq!(facing_to_angle(1), 4);
        assert_eq!(facing_to_angle(4), 16); // QUADRANT
        assert_eq!(facing_to_angle(8), 32); // HALF_CIRCLE
        assert_eq!(facing_to_angle(15), 60);
    }

    #[test]
    fn angle_to_facing_conversion() {
        assert_eq!(angle_to_facing(0), 0);
        assert_eq!(angle_to_facing(4), 1);
        assert_eq!(angle_to_facing(16), 4);
        assert_eq!(angle_to_facing(32), 8);
    }

    // gravity_mass is tested in element.rs

    // -- Toroidal Wrapping ---------------------------------------------------

    #[test]
    fn wrap_x_at_boundary() {
        // For a 1024-width arena (log_width=10)
        assert_eq!(wrap_x(0, 10), 0);
        assert_eq!(wrap_x(512, 10), 512);
        assert_eq!(wrap_x(1024, 10), 0); // Wraps to 0
        assert_eq!(wrap_x(1025, 10), 1);
        assert_eq!(wrap_x(-1, 10), 1023); // Negative wraps around
    }

    #[test]
    fn wrap_y_at_boundary() {
        // For a 1024-height arena (log_height=10)
        assert_eq!(wrap_y(0, 10), 0);
        assert_eq!(wrap_y(512, 10), 512);
        assert_eq!(wrap_y(1024, 10), 0); // Wraps to 0
        assert_eq!(wrap_y(1025, 10), 1);
        assert_eq!(wrap_y(-1, 10), 1023); // Negative wraps around
    }

    #[test]
    fn shortest_path_delta_no_wrap() {
        // Distance less than half-size doesn't wrap
        assert_eq!(shortest_path_delta(100, 200, 1024), 100);
        assert_eq!(shortest_path_delta(200, 100, 1024), -100);
    }

    #[test]
    fn shortest_path_delta_with_wrap() {
        // Distance greater than half-size wraps around
        assert_eq!(shortest_path_delta(100, 900, 1024), -224); // Wraps backward
        assert_eq!(shortest_path_delta(900, 100, 1024), 224); // Wraps forward
    }

    #[test]
    fn shortest_path_delta_exact_half() {
        // Exactly half-size doesn't wrap
        assert_eq!(shortest_path_delta(0, 512, 1024), 512);
        assert_eq!(shortest_path_delta(512, 0, 1024), -512);
    }

    // -- Trigonometry --------------------------------------------------------

    #[test]
    fn sine_known_value() {
        // Test key angles with full 64-entry sine table
        // UQM coords: 0=North(-Y), 16=East(+X), 32=South(+Y), 48=West(-X)

        // angle 0 = North (-Y) = -90° math → sine = -1.0
        let result = sine(0, 100);
        assert_eq!(result, -100);

        // angle 16 = East (+X) = 0° math → sine = 0.0
        let result = sine(16, 100);
        assert_eq!(result, 0);

        // angle 32 = South (+Y) = +90° math → sine = +1.0
        let result = sine(32, 100);
        assert_eq!(result, 100);

        // angle 48 = West (-X) = 180° math → sine = 0.0
        let result = sine(48, 100);
        assert_eq!(result, 0);
    }

    #[test]
    fn cosine_is_sine_plus_quadrant() {
        for angle in 0..FULL_CIRCLE {
            let c = cosine(angle, 100);
            let s = sine(angle + QUADRANT, 100);
            assert_eq!(c, s, "cosine({}) != sine({} + QUADRANT)", angle, angle);
        }
    }

    #[test]
    fn sine_uqm_coordinate_system() {
        // UQM uses screen coordinates with 0=North(-Y), clockwise rotation
        // angle 0 = North (-Y) → sine = -1
        assert!(sine(0, 100) < -95);
        // angle 16 = East (+X) → sine = 0
        assert!(sine(16, 100).abs() < 5);
        // angle 32 = South (+Y) → sine = +1
        assert!(sine(32, 100) > 95);
        // angle 48 = West (-X) → sine = 0
        assert!(sine(48, 100).abs() < 5);
    }

    #[test]
    fn arctan_zero_vector() {
        assert_eq!(arctan(0, 0), FULL_CIRCLE);
    }

    #[test]
    fn arctan_cardinal_directions() {
        // Right (+X, 0) should be QUADRANT (16)
        let angle_right = arctan(100, 0);
        assert_eq!(angle_right, QUADRANT);

        // Down (0, +Y) should be HALF_CIRCLE (32)
        let angle_down = arctan(0, 100);
        assert_eq!(angle_down, HALF_CIRCLE);

        // Left (-X, 0) should be QUADRANT*3 (48)
        let angle_left = arctan(-100, 0);
        assert_eq!(angle_left, QUADRANT * 3);

        // Up (0, -Y) should be 0
        let angle_up = arctan(0, -100);
        assert_eq!(angle_up, 0);
    }
}
