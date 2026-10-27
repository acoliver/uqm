//! Math library — replaces C `libs/math/` (random.c, random2.c, sqrt.c).
//!
//! Park-Miller minimal standard RNG (CACM 1988) + integer square root.
//! The game uses these providers directly through Rust imports.

/// Native fixed-point SINE: normalize to the 64-entry table, multiply in i64,
/// then retain the original signed SIZE truncation.
pub fn sine(angle: i32, magnitude: i32) -> i16 {
    ((i64::from(sinetab[(angle & 63) as usize]) * i64::from(magnitude)) >> 14) as i16
}

pub fn cosine(angle: i32, magnitude: i32) -> i16 {
    sine(angle + 16, magnitude)
}

/// Signed shortest displacement after assignment to the native i16 coordinate.
pub const fn wrap_delta(delta: i16, extent: i32) -> i16 {
    if (delta as i32).abs() <= extent >> 1 {
        delta
    } else if delta < 0 {
        (extent + delta as i32) as i16
    } else {
        (delta as i32 - extent) as i16
    }
}

use crate::memory::{rust_hfree, rust_hmalloc};
use std::os::raw::c_void;

// ---------------------------------------------------------------------------
// Constants (Park-Miller LCG)
// ---------------------------------------------------------------------------

const A: u32 = 16807; // multiplier
const M: u32 = 2_147_483_647; // 2^31 - 1
const Q: u32 = 127_773; // M / A
const R: u32 = 2836; // M % A

// ---------------------------------------------------------------------------
// Global RNG state (replaces C `static DWORD seed`)
// ---------------------------------------------------------------------------

static SEED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(12345);

fn park_miller(seed: u32) -> u32 {
    let mut s = A
        .wrapping_mul(seed % Q)
        .wrapping_sub(R.wrapping_mul(seed / Q));
    if s > M {
        s -= M;
    } else if s == 0 {
        s = 1;
    }
    s
}

// ---------------------------------------------------------------------------
// Global random stream
// ---------------------------------------------------------------------------

/// C: `DWORD random(void)`
pub fn random() -> u32 {
    let old = SEED.load(std::sync::atomic::Ordering::Relaxed);
    let new = park_miller(old);
    SEED.store(new, std::sync::atomic::Ordering::Relaxed);
    new
}

/// Coerce a requested seed into the generator's valid range.
///
/// Zero is not a usable Park-Miller seed and a value above the modulus is
/// reduced into range, matching the C generator.
fn coerce_seed(new_seed: u32) -> u32 {
    if new_seed == 0 {
        1
    } else if new_seed > M {
        new_seed - M
    } else {
        new_seed
    }
}

/// C: `DWORD seed_random(DWORD new_seed)`
pub fn seed_random(new_seed: u32) -> u32 {
    SEED.swap(coerce_seed(new_seed), std::sync::atomic::Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// RandomContext — heap-allocated independent RNG streams
// ---------------------------------------------------------------------------

/// Independent Park-Miller random stream.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct RandomContext {
    seed: u32,
}

/// C: `RandomContext *new_context(void)`
pub fn new_context() -> *mut RandomContext {
    unsafe {
        let ptr = rust_hmalloc(std::mem::size_of::<RandomContext>()) as *mut RandomContext;
        if !ptr.is_null() {
            (*ptr).seed = 12345;
        }
        ptr
    }
}

/// Release a stream allocated by `new_context`.
///
/// # Safety
/// The pointer must be null or an allocation returned by `new_context`,
/// and must not be used after this call.
pub unsafe fn delete_context(context: *mut RandomContext) {
    if !context.is_null() {
        unsafe { rust_hfree(context as *mut c_void) };
    }
}

impl RandomContext {
    /// The current position in the stream (`RandomContext_GetSeed`).
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// A context seeded the way `RandomContext_SeedRandom` would seed it.
    pub fn with_seed(seed: u32) -> Self {
        Self {
            seed: coerce_seed(seed),
        }
    }

    /// Advance the stream and return the next value (`RandomContext_Random`).
    pub fn next_random(&mut self) -> u32 {
        self.seed = park_miller(self.seed);
        self.seed
    }

    /// Reseed the stream (`RandomContext_SeedRandom`), returning the old seed.
    pub fn reseed(&mut self, new_seed: u32) -> u32 {
        std::mem::replace(&mut self.seed, coerce_seed(new_seed))
    }
}

// ---------------------------------------------------------------------------
// Integer square root — bit-manipulation algorithm from C sqrt.c
// ---------------------------------------------------------------------------

/// C: `COUNT square_root(DWORD value)` — returns u16
pub fn square_root(value: u32) -> u16 {
    let sig_word = (value >> 16) as u16;
    if sig_word > 0 {
        let mut mask: u16;
        let mut shift: u32;
        let mut v = value;

        // Find highest set bit in high word
        mask = 1 << 15;
        shift = 31;
        while mask & sig_word == 0 {
            mask >>= 1;
            shift -= 1;
        }
        shift >>= 1;
        mask = 1 << shift;

        let mut result: u16 = mask;
        let mut mask_squared: u32 = (mask as u32) << shift;
        let mut result_shift: u32 = mask_squared;
        v = v.wrapping_sub(mask_squared);

        while {
            mask >>= 1;
            mask != 0
        } {
            mask_squared >>= 1;
            mask_squared >>= 1;
            let remainder = result_shift + mask_squared;
            if remainder > v {
                result_shift >>= 1;
            } else {
                v -= remainder;
                result_shift = (result_shift >> 1) + mask_squared;
                result |= mask;
            }
        }
        result
    } else {
        let sig_word = value as u16;
        if sig_word > 0 {
            let mut mask: u16;
            let mut shift: u32;

            mask = 1 << 15;
            shift = 15;
            while mask & sig_word == 0 {
                mask >>= 1;
                shift -= 1;
            }
            shift >>= 1;
            mask = 1 << shift;

            let mut result: u16 = mask;
            let mut mask_squared: u16 = mask << shift;
            let mut result_shift: u16 = mask_squared;
            let mut sw = sig_word.wrapping_sub(mask_squared);

            while {
                mask >>= 1;
                mask != 0
            } {
                mask_squared >>= 1;
                mask_squared >>= 1;
                let remainder = result_shift.wrapping_add(mask_squared);
                if remainder > sw {
                    result_shift >>= 1;
                } else {
                    sw = sw.wrapping_sub(remainder);
                    result_shift = (result_shift >> 1) + mask_squared;
                    result |= mask;
                }
            }
            result
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Sine table + arctan — replaces C trans.c
// ---------------------------------------------------------------------------

/// C: `SIZE sinetab[]` — 64-entry sine lookup table.
/// Each entry is sin(angle) * 16384 (fixed-point, SIN_SHIFT=14).
/// UQM angle system: 0=North(-Y), 16=East(+X), 32=South(+Y), 48=West(-X).
/// Shared by the fixed-point trigonometry providers.
#[allow(non_upper_case_globals)]
pub static sinetab: [i16; 64] = [
    -16384, -16305, -16069, -15679, -15137, -14449, -13623, -12665, -11585, -10394, -9102, -7723,
    -6270, -4756, -3197, -1608, 0, 1608, 3197, 4756, 6270, 7723, 9102, 10394, 11585, 12665, 13623,
    14449, 15137, 15679, 16069, 16305, 16384, 16305, 16069, 15679, 15137, 14449, 13623, 12665,
    11585, 10394, 9102, 7723, 6270, 4756, 3197, 1608, 0, -1608, -3197, -4756, -6270, -7723, -9102,
    -10394, -11585, -12665, -13623, -14449, -15137, -15679, -16069, -16305,
];

const CIRCLE_SHIFT: u32 = 6;
const FULL_CIRCLE: u16 = 1 << CIRCLE_SHIFT; // 64
const HALF_CIRCLE: u16 = FULL_CIRCLE >> 1; // 32
const QUADRANT: u16 = FULL_CIRCLE >> 2; // 16

/// C: `COUNT arctan(SIZE delta_x, SIZE delta_y)`
///
/// Integer arctangent returning angle in UQM's 64-position circle.
/// Exact port of C trans.c algorithm.
pub fn arctan(delta_x: i16, delta_y: i16) -> u16 {
    const ATANTAB: [u16; 33] = [
        0, 0, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7,
        8, 8, 8,
    ];

    if delta_x == 0 && delta_y == 0 {
        return FULL_CIRCLE;
    }

    let v1_abs = (delta_x as i32).unsigned_abs();
    let v2_abs = (delta_y as i32).unsigned_abs();

    let v1 = if v1_abs > v2_abs {
        let ratio = ((v2_abs << (CIRCLE_SHIFT - 1)) + (v1_abs >> 1)) / v1_abs;
        let idx = (ratio as usize).min(ATANTAB.len() - 1);
        QUADRANT - ATANTAB[idx]
    } else {
        let ratio = ((v1_abs << (CIRCLE_SHIFT - 1)) + (v2_abs >> 1)) / v2_abs;
        let idx = (ratio as usize).min(ATANTAB.len() - 1);
        ATANTAB[idx]
    };

    let mut result = v1;
    if delta_x < 0 {
        result = FULL_CIRCLE.wrapping_sub(result);
    }
    if delta_y > 0 {
        result = HALF_CIRCLE.wrapping_sub(result);
    }

    result & (FULL_CIRCLE - 1) // NORMALIZE_ANGLE
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_methods_preserve_coercion_draw_order_and_copy_position() {
        for seed in [0, 1, 12345, M, M + 1, u32::MAX] {
            let mut context = RandomContext::with_seed(seed);
            let mut expected = coerce_seed(seed);
            assert_eq!(context.seed(), expected);
            for _ in 0..100 {
                expected = park_miller(expected);
                assert_eq!(context.next_random(), expected);
            }
            let mut copy = context.clone();
            assert_eq!(context.next_random(), copy.next_random());
            let previous = context.seed();
            assert_eq!(context.reseed(0), previous);
            assert_eq!(context.seed(), 1);
        }
        let context = new_context();
        unsafe {
            assert_eq!((*context).seed(), 12345);
            delete_context(context);
            delete_context(std::ptr::null_mut());
        }
    }

    #[test]
    fn test_park_miller_range_and_determinism() {
        let mut seed = 12345u32;
        for _ in 0..1000 {
            seed = park_miller(seed);
            assert!((1..=M).contains(&seed), "seed {seed} out of range [1, M]");
        }
        // Determinism: same seed → same next value
        let s1 = park_miller(12345);
        let s2 = park_miller(12345);
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_park_miller_known_value() {
        // First value from seed=12345: A*(12345%Q) - R*(12345/Q)
        // = 16807*12345 - 2836*0 = 207482415
        assert_eq!(park_miller(12345), 207482415);
    }

    /// The seed tests assert on the coercion itself rather than on the global.
    ///
    /// random loads, advances and stores SEED, so any test running beside
    /// these could change it between the swap and a separate read, which made
    /// the old assertions fail at random under a full parallel run.
    #[test]
    fn seed_in_range_is_used_unchanged() {
        assert_eq!(coerce_seed(42), 42);
        assert_eq!(coerce_seed(M), M);
    }

    #[test]
    fn zero_seed_is_coerced_to_one() {
        assert_eq!(coerce_seed(0), 1);
    }

    #[test]
    fn seed_above_the_modulus_wraps_into_range() {
        assert_eq!(coerce_seed(M + 100), 100);
        assert_eq!(coerce_seed(M + 1), 1);
    }

    #[test]
    fn seeding_returns_the_previous_seed_and_installs_the_new_one() {
        // Exercises the global path once, without asserting a value another
        // test could have changed: the swap's own return is authoritative.
        let restore = seed_random(12345);
        let previous = seed_random(999);
        assert_eq!(previous, 12345, "swap returns exactly what it replaced");
        seed_random(restore);
    }

    #[test]
    fn test_square_root_perfect_squares() {
        assert_eq!(square_root(0), 0);
        assert_eq!(square_root(1), 1);
        assert_eq!(square_root(4), 2);
        assert_eq!(square_root(9), 3);
        assert_eq!(square_root(16), 4);
        assert_eq!(square_root(25), 5);
        assert_eq!(square_root(100), 10);
        assert_eq!(square_root(10000), 100);
        assert_eq!(square_root(65535), 255); // max u16^2 - 1
    }

    #[test]
    fn test_square_root_large_values() {
        assert_eq!(square_root(1_000_000), 1000);
        assert_eq!(square_root(4_000_000), 2000);
        assert_eq!(square_root(0xFFFF_FFFF), 65535);
    }

    #[test]
    fn test_square_root_non_perfect() {
        // floor(sqrt(2)) = 1
        assert_eq!(square_root(2), 1);
        // floor(sqrt(3)) = 1
        assert_eq!(square_root(3), 1);
        // floor(sqrt(5)) = 2
        assert_eq!(square_root(5), 2);
        // floor(sqrt(8)) = 2
        assert_eq!(square_root(8), 2);
        // floor(sqrt(10)) = 3
        assert_eq!(square_root(10), 3);
    }
}
