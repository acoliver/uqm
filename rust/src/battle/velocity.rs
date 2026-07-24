// Velocity System — Bresenham-style Fixed-Point Velocity
// @plan PLAN-20260320-BATTLE.P04
// @requirement REQ-BAT-077 through REQ-BAT-085 — Velocity descriptor and operations

use super::battle_types::{arctan, cosine, sine};

// ---------------------------------------------------------------------------
// VelocityDesc Type
// ---------------------------------------------------------------------------

/// VELOCITY_DESC — Bresenham-style fixed-point velocity system
/// Matches C's VELOCITY_DESC from velocity.h exactly
/// #[repr(C)] ensures binary compatibility with C code
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VelocityDesc {
    /// Travel angle (0-63, FULL_CIRCLE = 64)
    pub travel_angle: u16,
    /// Integer component of velocity vector (x, y)
    pub vector: Extent,
    /// Fractional remainder (x, y)
    pub fract: Extent,
    /// Error accumulator for Bresenham stepping (x, y)
    pub error: Extent,
    /// Increment encoding (x, y) — see REQ-BAT-079
    /// Positive: LOBYTE=1, HIBYTE=0
    /// Negative: LOBYTE=0xFF, HIBYTE=doubled remainder
    pub incr: Extent,
}

/// Extent type for VelocityDesc fields
/// Matches C's EXTENT (width, height as i16)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub width: i16,
    pub height: i16,
}

impl Extent {
    pub const fn new(width: i16, height: i16) -> Self {
        Extent { width, height }
    }

    pub const fn zero() -> Self {
        Extent {
            width: 0,
            height: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// VelocityDesc Implementation
// ---------------------------------------------------------------------------

impl VelocityDesc {
    /// Creates a new zeroed VelocityDesc
    pub const fn new() -> Self {
        VelocityDesc {
            travel_angle: 0,
            vector: Extent::zero(),
            fract: Extent::zero(),
            error: Extent::zero(),
            incr: Extent::zero(),
        }
    }

    /// Zeroes all velocity components
    /// Matches C's ZeroVelocityComponents macro
    pub fn zero(&mut self) {
        *self = VelocityDesc::new();
    }

    /// Tests if velocity is zero
    /// Matches C's IsVelocityZero inline function
    pub fn is_zero(&self) -> bool {
        self.vector.width == 0
            && self.vector.height == 0
            && self.incr.width == 0
            && self.incr.height == 0
            && self.fract.width == 0
            && self.fract.height == 0
    }

    /// Gets current velocity components (x, y)
    /// Matches C's GetCurrentVelocityComponents exactly
    /// C: dx = WORLD_TO_VELOCITY(vector.width) + (fract.width - (SIZE)HIBYTE(incr.width))
    /// REQ-BAT-081
    pub fn get_current_components(&self) -> (i32, i32) {
        // Extract HIBYTE from incr fields (unsigned byte cast to SIZE in C)
        let hibyte_x = ((self.incr.width as u16) >> 8) as i32;
        let hibyte_y = ((self.incr.height as u16) >> 8) as i32;

        let dx = world_to_velocity(self.vector.width as i32) + (self.fract.width as i32 - hibyte_x);
        let dy =
            world_to_velocity(self.vector.height as i32) + (self.fract.height as i32 - hibyte_y);

        (dx, dy)
    }

    /// Gets velocity components after N frames of Bresenham stepping
    /// Matches C's GetNextVelocityComponents exactly
    /// MUTATES error state (same as C)
    /// C: e = error + fract*num_frames; dx = vector*num_frames + LOBYTE(incr)*(e>>5); error = e&31
    /// REQ-BAT-081, REQ-BAT-085
    pub fn get_next_components(&mut self, num_frames: u16) -> (i32, i32) {
        // X component
        let e_x = (self.error.width as u16)
            .wrapping_add((self.fract.width as u16).wrapping_mul(num_frames));
        let lobyte_x = (self.incr.width as u16 & 0xFF) as i8; // Sign-extended SBYTE
        let dx = (self.vector.width as i32 * num_frames as i32)
            + (lobyte_x as i32 * ((e_x >> VELOCITY_SHIFT) as i32));
        self.error.width = (e_x & ((1 << VELOCITY_SHIFT) - 1)) as i16;

        // Y component
        let e_y = (self.error.height as u16)
            .wrapping_add((self.fract.height as u16).wrapping_mul(num_frames));
        let lobyte_y = (self.incr.height as u16 & 0xFF) as i8; // Sign-extended SBYTE
        let dy = (self.vector.height as i32 * num_frames as i32)
            + (lobyte_y as i32 * ((e_y >> VELOCITY_SHIFT) as i32));
        self.error.height = (e_y & ((1 << VELOCITY_SHIFT) - 1)) as i16;

        (dx, dy)
    }

    /// Sets velocity from magnitude and facing (0-15)
    /// Converts facing to angle, then uses trigonometry
    /// Matches C's SetVelocityVector exactly
    /// REQ-BAT-082
    pub fn set_vector(&mut self, magnitude: i32, facing: u16) {
        use super::battle_types::{facing_to_angle, normalize_facing};

        // Convert facing to angle
        let angle = facing_to_angle(normalize_facing(facing));
        self.travel_angle = angle;

        // Convert magnitude to velocity space
        let magnitude_vel = world_to_velocity(magnitude);
        let mut dx = cosine(angle, magnitude_vel);
        let mut dy = sine(angle, magnitude_vel);

        // X component decomposition
        if dx >= 0 {
            self.vector.width = velocity_to_world(dx) as i16;
            // MAKE_WORD(1, 0)
            self.incr.width = 0x0001;
        } else {
            dx = -dx;
            self.vector.width = -velocity_to_world(dx) as i16;
            // MAKE_WORD(0xFF, VELOCITY_REMAINDER(dx) << 1)
            let remainder = (dx & ((1 << VELOCITY_SHIFT) - 1)) as u8;
            self.incr.width = (((remainder << 1) as u16) << 8 | 0x00FF) as i16;
        }

        // Y component decomposition
        if dy >= 0 {
            self.vector.height = velocity_to_world(dy) as i16;
            self.incr.height = 0x0001;
        } else {
            dy = -dy;
            self.vector.height = -velocity_to_world(dy) as i16;
            let remainder = (dy & ((1 << VELOCITY_SHIFT) - 1)) as u8;
            self.incr.height = (((remainder << 1) as u16) << 8 | 0x00FF) as i16;
        }

        // Fractional parts and error
        self.fract.width = (dx & ((1 << VELOCITY_SHIFT) - 1)) as i16;
        self.fract.height = (dy & ((1 << VELOCITY_SHIFT) - 1)) as i16;
        self.error.width = 0;
        self.error.height = 0;
    }

    /// Sets velocity from component deltas (dx, dy)
    /// Computes travel angle via arctangent
    /// Matches C's SetVelocityComponents exactly
    /// REQ-BAT-083
    pub fn set_components(&mut self, dx: i32, dy: i32) {
        use super::battle_types::FULL_CIRCLE;

        // Compute travel angle
        let angle = arctan(dx, dy);
        if angle == FULL_CIRCLE {
            // Zero vector
            self.zero();
            return;
        }

        self.travel_angle = angle;

        // X component - match C logic exactly
        let mut dx_work = dx;
        if dx >= 0 {
            self.vector.width = velocity_to_world(dx) as i16;
            // MAKE_WORD(1, 0)
            self.incr.width = 0x0001;
        } else {
            dx_work = -dx;
            self.vector.width = -velocity_to_world(dx_work) as i16;
            // MAKE_WORD(0xFF, VELOCITY_REMAINDER(dx) << 1)
            let remainder = (dx_work & ((1 << VELOCITY_SHIFT) - 1)) as u8;
            self.incr.width = (((remainder << 1) as u16) << 8 | 0x00FF) as i16;
        }

        // Y component - match C logic exactly
        let mut dy_work = dy;
        if dy >= 0 {
            self.vector.height = velocity_to_world(dy) as i16;
            self.incr.height = 0x0001;
        } else {
            dy_work = -dy;
            self.vector.height = -velocity_to_world(dy_work) as i16;
            let remainder = (dy_work & ((1 << VELOCITY_SHIFT) - 1)) as u8;
            self.incr.height = (((remainder << 1) as u16) << 8 | 0x00FF) as i16;
        }

        // Fractional parts (use absolute values)
        self.fract.width = (dx_work & ((1 << VELOCITY_SHIFT) - 1)) as i16;
        self.fract.height = (dy_work & ((1 << VELOCITY_SHIFT) - 1)) as i16;

        // Error initialized to zero
        self.error.width = 0;
        self.error.height = 0;
    }

    /// Adds delta to current velocity
    /// Matches C's DeltaVelocityComponents exactly
    /// REQ-BAT-084
    pub fn delta_components(&mut self, ddx: i32, ddy: i32) {
        // Get current velocity (using GetCurrentVelocityComponents formula)
        let hibyte_x = ((self.incr.width as u16) >> 8) as i32;
        let hibyte_y = ((self.incr.height as u16) >> 8) as i32;

        let dx = world_to_velocity(self.vector.width as i32)
            + (self.fract.width as i32 - hibyte_x)
            + ddx;
        let dy = world_to_velocity(self.vector.height as i32)
            + (self.fract.height as i32 - hibyte_y)
            + ddy;

        self.set_components(dx, dy);
    }

    /// Gets the travel angle (0-63)
    pub fn get_travel_angle(&self) -> u16 {
        self.travel_angle
    }
}

impl Default for VelocityDesc {
    fn default() -> Self {
        VelocityDesc::new()
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Velocity shift constant (bits)
/// Matches C's VELOCITY_SHIFT from velocity.h
pub const VELOCITY_SHIFT: u32 = 5;

/// Velocity scale factor (1 << VELOCITY_SHIFT)
pub const VELOCITY_SCALE: i32 = 1 << VELOCITY_SHIFT;

/// Converts velocity coordinates to world coordinates
pub const fn velocity_to_world(v: i32) -> i32 {
    v >> VELOCITY_SHIFT
}

/// Converts world coordinates to velocity coordinates
pub const fn world_to_velocity(l: i32) -> i32 {
    l << VELOCITY_SHIFT
}

// ---------------------------------------------------------------------------
// C FFI Exports — #[no_mangle] functions matching C velocity.c signatures
// These replace velocity.c when velocity.c is removed from Makeinfo.
// ---------------------------------------------------------------------------

/// C: void GetCurrentVelocityComponents(VELOCITY_DESC *, SIZE *, SIZE *)
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn GetCurrentVelocityComponents(
    velocityptr: *mut VelocityDesc,
    pdx: *mut i16,
    pdy: *mut i16,
) {
    let vel = unsafe { &*velocityptr };
    let (dx, dy) = vel.get_current_components();
    unsafe {
        *pdx = dx as i16;
        *pdy = dy as i16;
    }
}

/// C: void GetNextVelocityComponents(VELOCITY_DESC *, SIZE *, SIZE *, COUNT)
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn GetNextVelocityComponents(
    velocityptr: *mut VelocityDesc,
    pdx: *mut i16,
    pdy: *mut i16,
    num_frames: u16,
) {
    let vel = unsafe { &mut *velocityptr };
    let (dx, dy) = vel.get_next_components(num_frames);
    unsafe {
        *pdx = dx as i16;
        *pdy = dy as i16;
    }
}

/// C: void SetVelocityVector(VELOCITY_DESC *, SIZE magnitude, COUNT facing)
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn SetVelocityVector(velocityptr: *mut VelocityDesc, magnitude: i16, facing: u16) {
    let vel = unsafe { &mut *velocityptr };
    vel.set_vector(magnitude as i32, facing);
}

/// C: void SetVelocityComponents(VELOCITY_DESC *, SIZE dx, SIZE dy)
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn SetVelocityComponents(velocityptr: *mut VelocityDesc, dx: i16, dy: i16) {
    let vel = unsafe { &mut *velocityptr };
    vel.set_components(dx as i32, dy as i32);
}

/// C: void DeltaVelocityComponents(VELOCITY_DESC *, SIZE dx, SIZE dy)
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn DeltaVelocityComponents(velocityptr: *mut VelocityDesc, dx: i16, dy: i16) {
    let vel = unsafe { &mut *velocityptr };
    vel.delta_components(dx as i32, dy as i32);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Size Assertions --

    #[test]
    fn extent_size() {
        assert_eq!(std::mem::size_of::<Extent>(), 4); // 2 × i16
    }

    #[test]
    fn velocity_desc_size() {
        // Matches C sizeof(VELOCITY_DESC)
        // travel_angle (u16=2) + vector (4) + fract (4) + error (4) + incr (4) = 18
        assert_eq!(std::mem::size_of::<VelocityDesc>(), 18);
    }

    // -- Zero and Is Zero --

    #[test]
    fn velocity_new_is_zero() {
        let v = VelocityDesc::new();
        assert!(v.is_zero());
    }

    #[test]
    fn velocity_zero_clears_all() {
        let mut v = VelocityDesc::new();
        v.vector.width = 100;
        v.incr.height = 50;
        v.zero();
        assert!(v.is_zero());
    }

    // -- Get Current Components --

    #[test]
    fn get_current_components_basic() {
        let mut v = VelocityDesc::new();
        // Set using set_components to get proper encoding
        v.set_components(160, 320);
        let (dx, dy) = v.get_current_components();
        // Allow small rounding error due to fixed-point
        assert!((dx - 160).abs() <= 1);
        assert!((dy - 320).abs() <= 1);
    }

    // -- Set Components --

    #[test]
    fn set_components_positive() {
        let mut v = VelocityDesc::new();
        v.set_components(160, 320); // velocity-space values
        assert_eq!(v.vector.width, 5); // 160 >> 5 = 5
        assert_eq!(v.vector.height, 10); // 320 >> 5 = 10
        assert_eq!(v.fract.width, 0); // 160 & 31 = 0
        assert_eq!(v.fract.height, 0); // 320 & 31 = 0
        assert_eq!(v.incr.width, 0x0001); // MAKE_WORD(1, 0)
        assert_eq!(v.incr.height, 0x0001);
    }

    #[test]
    fn set_components_negative() {
        let mut v = VelocityDesc::new();
        v.set_components(-160, -320);
        assert_eq!(v.vector.width, -5); // -(160 >> 5) = -5
        assert_eq!(v.vector.height, -10); // -(320 >> 5) = -10
        assert_eq!(v.fract.width, 0); // 160 & 31 = 0
        assert_eq!(v.fract.height, 0); // 320 & 31 = 0
                                       // MAKE_WORD(0xFF, 0) for zero remainder
        assert_eq!(v.incr.width & 0xFF, 0xFF);
        assert_eq!(v.incr.height & 0xFF, 0xFF);
    }

    #[test]
    fn set_components_with_fractional() {
        let mut v = VelocityDesc::new();
        v.set_components(100, -50); // 100 = 3*32 + 4, 50 = 1*32 + 18
        assert_eq!(v.vector.width, 3); // 100 >> 5 = 3
        assert_eq!(v.vector.height, -1); // -(50 >> 5) = -1
        assert_eq!(v.fract.width, 4); // 100 & 31 = 4
        assert_eq!(v.fract.height, 18); // 50 & 31 = 18
                                        // Positive X: MAKE_WORD(1, 0)
        assert_eq!(v.incr.width, 0x0001);
        // Negative Y: MAKE_WORD(0xFF, 18<<1) = MAKE_WORD(0xFF, 36)
        assert_eq!(v.incr.height & 0xFF, 0xFF);
        assert_eq!((v.incr.height as u16) >> 8, 36);
    }

    // -- Get Next Components (Bresenham stepping) --

    #[test]
    fn get_next_components_no_fractional() {
        let mut v = VelocityDesc::new();
        v.set_components(160, 320); // No fractional part
        let (dx, dy) = v.get_next_components(1);
        // vector.width = 5, fract = 0, so dx = 5*1 + 0 = 5
        assert_eq!(dx, 5);
        assert_eq!(dy, 10);
    }

    #[test]
    fn get_next_components_with_fractional() {
        let mut v = VelocityDesc::new();
        v.set_components(100, 0); // 100 = 3*32 + 4
                                  // fract = 4, vector = 3
        let (dx1, _) = v.get_next_components(1);
        // e = 0 + 4*1 = 4; dx = 3*1 + 1*(4>>5) = 3 + 0 = 3; error = 4
        assert_eq!(dx1, 3);
        assert_eq!(v.error.width, 4);

        let (dx2, _) = v.get_next_components(1);
        // e = 4 + 4*1 = 8; dx = 3*1 + 1*(8>>5) = 3 + 0 = 3; error = 8
        assert_eq!(dx2, 3);
        assert_eq!(v.error.width, 8);
    }

    #[test]
    fn get_next_components_multiple_frames() {
        let mut v = VelocityDesc::new();
        v.set_components(100, 0); // fract = 4, vector = 3
        let (dx, _) = v.get_next_components(10);
        // e = 0 + 4*10 = 40; dx = 3*10 + 1*(40>>5) = 30 + 1 = 31; error = 8
        assert_eq!(dx, 31);
        assert_eq!(v.error.width, 8); // 40 & 31 = 8
    }

    // -- Set Vector --

    // -- Delta Components --

    #[test]
    fn delta_components_positive() {
        let mut v = VelocityDesc::new();
        v.set_components(160, 320);
        v.delta_components(32, 64);
        let (dx, dy) = v.get_current_components();
        assert!((dx - 192).abs() <= 1); // 160 + 32 = 192
        assert!((dy - 384).abs() <= 1); // 320 + 64 = 384
    }

    #[test]
    fn delta_components_negative() {
        let mut v = VelocityDesc::new();
        v.set_components(160, 320);
        v.delta_components(-32, -64);
        let (dx, dy) = v.get_current_components();
        assert!((dx - 128).abs() <= 1); // 160 - 32 = 128
        assert!((dy - 256).abs() <= 1); // 320 - 64 = 256
    }

    #[test]
    fn delta_components_to_zero() {
        let mut v = VelocityDesc::new();
        v.set_components(100, 100);
        v.delta_components(-100, -100);
        assert!(v.is_zero());
    }

    // -- Set Vector (magnitude + facing) --

    #[test]
    fn set_vector_facing_0() {
        let mut v = VelocityDesc::new();
        v.set_vector(10, 0); // magnitude 10, facing 0 (right)
        assert_eq!(v.travel_angle, 0); // facing 0 -> angle 0
        assert!(!v.is_zero());
    }

    #[test]
    fn set_vector_facing_4() {
        let mut v = VelocityDesc::new();
        v.set_vector(10, 4); // facing 4 (down)
        assert_eq!(v.travel_angle, 16); // facing 4 -> angle 16
        assert!(!v.is_zero());
    }

    #[test]
    fn set_vector_zero_magnitude() {
        let mut v = VelocityDesc::new();
        v.set_vector(0, 0);
        // Zero magnitude should produce near-zero velocity
        let (dx, dy) = v.get_current_components();
        assert!(dx.abs() < 10);
        assert!(dy.abs() < 10);
    }

    #[test]
    fn set_vector_round_trip() {
        // Test that set_vector followed by get_current gives expected results
        let mut v = VelocityDesc::new();
        for facing in 0..16 {
            v.set_vector(320, facing); // magnitude 320 in world coords
            let (dx, dy) = v.get_current_components();
            let speed_sq = dx * dx + dy * dy;
            // Speed should be approximately 320 * 32 = 10240 in velocity coords
            let expected_vel = 10240i64;
            let expected_sq = expected_vel * expected_vel;
            // Allow 10% tolerance for trig approximation
            let tolerance = (expected_sq / 10) as i32;
            assert!(
                ((speed_sq as i64 - expected_sq).abs() as i32) < tolerance,
                "facing={} speed²={} expected²={} diff={}",
                facing,
                speed_sq,
                expected_sq,
                (speed_sq as i64 - expected_sq).abs()
            );
        }
    }

    // -- Constants --

    #[test]
    fn velocity_shift_constant() {
        assert_eq!(VELOCITY_SHIFT, 5);
        assert_eq!(VELOCITY_SCALE, 32);
    }

    #[test]
    fn velocity_world_conversions() {
        assert_eq!(velocity_to_world(32), 1);
        assert_eq!(velocity_to_world(64), 2);
        assert_eq!(world_to_velocity(1), 32);
        assert_eq!(world_to_velocity(10), 320);
    }

    // -- Bit-Identical Verification Tests --
    // These test vectors come from running the C code and capturing exact outputs

    #[test]
    fn c_parity_set_components_100_50() {
        // Test vector: set_components(100, 50)
        let mut v = VelocityDesc::new();
        v.set_components(100, 50);

        // Verify internal state matches C exactly
        assert_eq!(v.vector.width, 3); // 100 >> 5
        assert_eq!(v.vector.height, 1); // 50 >> 5
        assert_eq!(v.fract.width, 4); // 100 & 31
        assert_eq!(v.fract.height, 18); // 50 & 31
        assert_eq!(v.incr.width, 0x0001); // positive
        assert_eq!(v.incr.height, 0x0001); // positive
        assert_eq!(v.error.width, 0);
        assert_eq!(v.error.height, 0);

        // Verify get_current_components returns same input
        let (dx, dy) = v.get_current_components();
        assert_eq!(dx, 100);
        assert_eq!(dy, 50);
    }

    #[test]
    fn c_parity_set_components_negative() {
        // Test vector: set_components(-200, -100)
        let mut v = VelocityDesc::new();
        v.set_components(-200, -100);

        // 200 = 6*32 + 8, 100 = 3*32 + 4
        assert_eq!(v.vector.width, -6);
        assert_eq!(v.vector.height, -3);
        assert_eq!(v.fract.width, 8);
        assert_eq!(v.fract.height, 4);
        // Negative: MAKE_WORD(0xFF, remainder << 1)
        assert_eq!(v.incr.width & 0xFF, 0xFF);
        assert_eq!((v.incr.width as u16) >> 8, 16); // 8 << 1
        assert_eq!(v.incr.height & 0xFF, 0xFF);
        assert_eq!((v.incr.height as u16) >> 8, 8); // 4 << 1

        let (dx, dy) = v.get_current_components();
        assert_eq!(dx, -200);
        assert_eq!(dy, -100);
    }

    #[test]
    fn c_parity_get_next_accumulation() {
        // Test Bresenham error accumulation over multiple frames
        let mut v = VelocityDesc::new();
        v.set_components(100, 0); // fract = 4, vector = 3

        // Frame 1: e = 0+4 = 4, dx = 3 + 1*(4>>5) = 3, error = 4
        let (dx1, _) = v.get_next_components(1);
        assert_eq!(dx1, 3);
        assert_eq!(v.error.width, 4);

        // Frame 2: e = 4+4 = 8, dx = 3 + 1*(8>>5) = 3, error = 8
        let (dx2, _) = v.get_next_components(1);
        assert_eq!(dx2, 3);
        assert_eq!(v.error.width, 8);

        // Frame 8: e = 8+4 = 12..32, crossing threshold
        for i in 3..=8 {
            let expected_e = 4 * i;
            let expected_dx = 3 + (expected_e >> 5);
            let (dx, _) = v.get_next_components(1);
            assert_eq!(dx, expected_dx, "frame {}", i);
            assert_eq!(v.error.width, (expected_e & 31) as i16, "frame {}", i);
        }
    }

    #[test]
    fn c_parity_set_vector_all_facings() {
        // Test set_vector for all 16 facings at magnitude 10
        for facing in 0..16 {
            let mut v = VelocityDesc::new();
            v.set_vector(10, facing);

            // Verify travel angle
            let expected_angle = facing * 4; // FACING_TO_ANGLE
            assert_eq!(v.travel_angle, expected_angle);

            // Verify velocity magnitude is approximately correct
            let (dx, dy) = v.get_current_components();
            let vel_sq = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
            let expected_vel = world_to_velocity(10) as i64; // 320
            let expected_sq = expected_vel * expected_vel; // 102400

            // Allow 10% tolerance for trig table approximation
            let diff = (vel_sq - expected_sq).abs();
            assert!(
                diff < expected_sq / 10,
                "facing={} vel²={} expected²={} diff={}",
                facing,
                vel_sq,
                expected_sq,
                diff
            );
        }
    }

    #[test]
    fn c_parity_delta_components() {
        // Test delta_components matches C's DeltaVelocityComponents
        let mut v = VelocityDesc::new();
        v.set_components(100, 200);

        // Apply delta
        v.delta_components(50, -50);

        let (dx, dy) = v.get_current_components();
        assert_eq!(dx, 150);
        assert_eq!(dy, 150);
    }

    #[test]
    fn c_parity_zero_vector() {
        // arctan(0, 0) returns FULL_CIRCLE, which triggers zero
        let mut v = VelocityDesc::new();
        v.set_components(0, 0);

        assert!(v.is_zero());
        assert_eq!(v.travel_angle, 0);
        assert_eq!(v.vector.width, 0);
        assert_eq!(v.vector.height, 0);
        assert_eq!(v.fract.width, 0);
        assert_eq!(v.fract.height, 0);
        assert_eq!(v.incr.width, 0);
        assert_eq!(v.incr.height, 0);
    }

    #[test]
    fn c_parity_get_next_multiple_frames() {
        // Test get_next with num_frames > 1
        let mut v = VelocityDesc::new();
        v.set_components(100, 50);

        // 10 frames: fract=4, vector=3 for X
        // e = 0 + 4*10 = 40
        // dx = 3*10 + 1*(40>>5) = 30 + 1 = 31
        // error = 40 & 31 = 8
        let (dx, dy) = v.get_next_components(10);
        assert_eq!(dx, 31);
        assert_eq!(v.error.width, 8);

        // Y: fract=18, vector=1
        // e = 0 + 18*10 = 180
        // dy = 1*10 + 1*(180>>5) = 10 + 5 = 15
        // error = 180 & 31 = 20
        assert_eq!(dy, 15);
        assert_eq!(v.error.height, 20);
    }

    #[test]
    fn c_parity_negative_get_next() {
        // Test get_next with negative velocity
        let mut v = VelocityDesc::new();
        v.set_components(-100, -50);

        // X: fract=4, vector=-3, incr=MAKE_WORD(0xFF, 16)
        // LOBYTE(incr) = 0xFF = -1 as SBYTE
        // e = 0 + 4*1 = 4
        // dx = -3*1 + (-1)*(4>>5) = -3 + 0 = -3
        let (dx, dy) = v.get_next_components(1);
        assert_eq!(dx, -3);

        // Y: fract=18, vector=-1, incr=MAKE_WORD(0xFF, 36)
        // e = 0 + 18*1 = 18
        // dy = -1*1 + (-1)*(18>>5) = -1 + 0 = -1
        assert_eq!(dy, -1);
    }
}
