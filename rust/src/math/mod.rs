//! Math library — replaces C `libs/math/` (random.c, random2.c, sqrt.c).
//!
//! Park-Miller minimal standard RNG (CACM 1988) + integer square root.
//! All functions are exported as `#[no_mangle]` with exact C signatures
//! so the C `.o` files can be removed from the build.

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
// C FFI exports
// ---------------------------------------------------------------------------

/// C: `DWORD TFB_Random(void)`
#[no_mangle]
pub extern "C" fn TFB_Random() -> u32 {
    let old = SEED.load(std::sync::atomic::Ordering::Relaxed);
    let new = park_miller(old);
    SEED.store(new, std::sync::atomic::Ordering::Relaxed);
    new
}

/// C: `DWORD TFB_SeedRandom(DWORD new_seed)`
#[no_mangle]
pub extern "C" fn TFB_SeedRandom(new_seed: u32) -> u32 {
    let mut s = new_seed;
    if s == 0 {
        s = 1;
    } else if s > M {
        s -= M;
    }
    SEED.swap(s, std::sync::atomic::Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// RandomContext — heap-allocated independent RNG streams
// ---------------------------------------------------------------------------

/// C: `struct RandomContext { DWORD seed; }`
#[repr(C)]
pub struct RandomContext {
    seed: u32,
}

extern "C" {
    fn rust_hmalloc(size: usize) -> *mut c_void;
    fn rust_hfree(ptr: *mut c_void);
}

/// C: `RandomContext *RandomContext_New(void)`
#[no_mangle]
pub extern "C" fn RandomContext_New() -> *mut RandomContext {
    unsafe {
        let ptr = rust_hmalloc(std::mem::size_of::<RandomContext>()) as *mut RandomContext;
        if !ptr.is_null() {
            (*ptr).seed = 12345;
        }
        ptr
    }
}

/// C: `void RandomContext_Delete(RandomContext *context)`
#[no_mangle]
pub extern "C" fn RandomContext_Delete(context: *mut RandomContext) {
    if !context.is_null() {
        unsafe { rust_hfree(context as *mut c_void) };
    }
}

/// C: `RandomContext *RandomContext_Copy(const RandomContext *source)`
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn RandomContext_Copy(source: *const RandomContext) -> *mut RandomContext {
    unsafe {
        let ptr = rust_hmalloc(std::mem::size_of::<RandomContext>()) as *mut RandomContext;
        if !ptr.is_null() {
            (*ptr).seed = (*source).seed;
        }
        ptr
    }
}

/// C: `DWORD RandomContext_Random(RandomContext *context)`
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn RandomContext_Random(context: *mut RandomContext) -> u32 {
    unsafe {
        (*context).seed = park_miller((*context).seed);
        (*context).seed
    }
}

/// C: `DWORD RandomContext_SeedRandom(RandomContext *context, DWORD new_seed)`
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn RandomContext_SeedRandom(context: *mut RandomContext, new_seed: u32) -> u32 {
    unsafe {
        let mut s = new_seed;
        if s == 0 {
            s = 1;
        } else if s > M {
            s -= M;
        }
        let old = (*context).seed;
        (*context).seed = s;
        old
    }
}

/// C: `DWORD RandomContext_GetSeed(RandomContext *context)`
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn RandomContext_GetSeed(context: *mut RandomContext) -> u32 {
    unsafe { (*context).seed }
}

// ---------------------------------------------------------------------------
// Integer square root — bit-manipulation algorithm from C sqrt.c
// ---------------------------------------------------------------------------

/// C: `COUNT square_root(DWORD value)` — returns u16
#[no_mangle]
pub extern "C" fn square_root(value: u32) -> u16 {
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
// Sine table + ARCTAN — replaces C trans.c
// ---------------------------------------------------------------------------

/// C: `SIZE sinetab[]` — 64-entry sine lookup table.
/// Each entry is sin(angle) * 16384 (fixed-point, SIN_SHIFT=14).
/// UQM angle system: 0=North(-Y), 16=East(+X), 32=South(+Y), 48=West(-X).
/// Exported as `#[no_mangle]` so C macros `SINVAL`, `COSVAL`, `SINE`, `COSINE`
/// in units.h can index into it directly.
#[no_mangle]
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

/// C: `COUNT ARCTAN(SIZE delta_x, SIZE delta_y)`
///
/// Integer arctangent returning angle in UQM's 64-position circle.
/// Exact port of C trans.c algorithm.
#[no_mangle]
pub extern "C" fn ARCTAN(delta_x: i16, delta_y: i16) -> u16 {
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

    #[test]
    fn test_tfb_seed_random_returns_old() {
        let old = TFB_SeedRandom(42);
        let now = SEED.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(now, 42);
        // Restore
        TFB_SeedRandom(old);
    }

    #[test]
    fn test_tfb_seed_random_coerces_zero() {
        let old = TFB_SeedRandom(0);
        let now = SEED.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(now, 1);
        TFB_SeedRandom(old);
    }

    #[test]
    fn test_tfb_seed_random_coerces_overflow() {
        let old = TFB_SeedRandom(M + 100);
        let now = SEED.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(now, 100);
        TFB_SeedRandom(old);
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
