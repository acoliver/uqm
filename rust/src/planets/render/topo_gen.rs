//! Elevation conditioning and relative lighting from `planets/plangen.c`.

use crate::math::RandomContext;
use crate::starbase::ids::SAFE_Y;

/// `MAP_HEIGHT` in `planets.h`. Width is the current SIS screen width.
pub const MAP_HEIGHT: usize = 75 - SAFE_Y as usize;
const RANGE_SHIFT: u32 = 6;
const DITHER_VARIANCE: u32 = 1 << (RANGE_SHIFT - 3);
const LMAP_AVG_BLOCK: usize = MAP_HEIGHT.div_ceil(5);
const LMAP_MAX_DIST: i32 = ((LMAP_AVG_BLOCK + 1) >> 1) as i32;
const LMAP_WEIGHT_THRES: i32 = LMAP_MAX_DIST * 2 / 3;
const LMAP_BLOCKS: usize = (2 * LMAP_MAX_DIST + 1) as usize;

/// Dither one complete map, consuming one system RNG value per four cells.
/// The original `COUNT` loop requires fewer than 65536 cells.
pub fn dither_map(rng: &mut RandomContext, depth: &mut [i8], width: usize) {
    assert_eq!(depth.len(), width * MAP_HEIGHT);
    assert!(depth.len() <= u16::MAX as usize);
    let mut random = 0;
    for (i, cell) in depth.iter_mut().enumerate() {
        if i & 3 == 0 {
            random = rng.next_random();
        } else {
            random >>= 8;
        }
        let delta = (DITHER_VARIANCE / 2) as i32 - (random & (DITHER_VARIANCE - 1)) as i32;
        *cell = (i32::from(*cell) + delta) as i8;
    }
}

/// Remove signed-byte elevation discontinuities, using the first row to
/// choose a starting side. Counts and remembered bytes wrap exactly as C's
/// `BYTE` counters do, including the tie selecting side one.
pub fn validate_map(depth: &mut [i8], width: usize) {
    assert_eq!(depth.len(), width * MAP_HEIGHT);
    assert!(width >= 2 && depth.len() <= u16::MAX as usize);
    let mut state = 0;
    let mut counts = [0_u8; 2];
    let mut first = [0_i8; 2];
    let mut last = depth[0];
    for &cell in &depth[1..width] {
        if counts[state] == 0 {
            first[state] = last;
        }
        counts[state] = counts[state].wrapping_add(1);
        if (i32::from(last) - i32::from(cell)).abs() > 128 {
            state ^= 1;
        }
        last = cell;
    }
    last = first[usize::from(counts[0] <= counts[1])];
    for cell in depth {
        if (i32::from(last) - i32::from(*cell)).abs() > 128 {
            *cell = last;
        }
        last = *cell;
    }
}

#[derive(Clone, Copy, Default)]
struct ElevationBlock {
    min: i32,
    max: i32,
    avg: i32,
}

fn weight(delta: i32) -> i32 {
    if delta >= LMAP_WEIGHT_THRES {
        255 * (LMAP_MAX_DIST - delta + 1) / (LMAP_MAX_DIST - LMAP_WEIGHT_THRES + 2)
    } else {
        255
    }
}

fn vertical_block(depth: &[i8], width: usize, x: i32, y: i32) -> ElevationBlock {
    let x = ((x + width as i32) % width as i32) as usize;
    let y0 = (y - LMAP_MAX_DIST).max(0);
    let y1 = (y + LMAP_MAX_DIST).min(MAP_HEIGHT as i32);
    let mut block = ElevationBlock {
        min: 127,
        max: -127,
        avg: 0,
    };
    let mut total_weight = 0;
    for row in y0..y1 {
        let value = i32::from(depth[row as usize * width + x]);
        let w = weight((row - y).abs());
        block.min = block.min.min(value);
        block.max = block.max.max(value);
        block.avg += value * w;
        total_weight += w;
    }
    // C divides twice, with truncation at each division.
    block.avg = (block.avg / total_weight) / (y1 - y0);
    block
}

/// Replace a complete elevation map with the original relative-light map.
/// Width must exceed the horizontal averaging radius. The transform is
/// deliberately in place: later blocks observe already transformed cells.
pub fn generate_light_map(depth: &mut [i8], width: usize) {
    assert_eq!(depth.len(), width * MAP_HEIGHT);
    assert!(width > LMAP_MAX_DIST as usize && width <= i16::MAX as usize);
    let min = i32::from(*depth.iter().min().expect("nonempty map"));
    let max = i32::from(*depth.iter().max().expect("nonempty map"));
    let med = (min + max) / 2;
    let spread = max - med;
    if spread == 0 {
        if max != 0 {
            depth.fill(0);
        }
        return;
    }
    let factor = if spread < 10 {
        30
    } else if spread < 30 {
        60
    } else {
        100
    };
    for cell in depth.iter_mut() {
        *cell = ((i32::from(*cell) - med) * factor / spread) as i8;
    }
    let mut blocks = [ElevationBlock::default(); LMAP_BLOCKS];
    for y in 0..MAP_HEIGHT as i32 {
        for i in -LMAP_MAX_DIST..LMAP_MAX_DIST {
            blocks[((i + LMAP_BLOCKS as i32) % LMAP_BLOCKS as i32) as usize] =
                vertical_block(depth, width, i, y);
        }
        for x in 0..width as i32 {
            blocks[((x + LMAP_MAX_DIST) % LMAP_BLOCKS as i32) as usize] =
                vertical_block(depth, width, x + LMAP_MAX_DIST, y);
            let mut min = 127;
            let mut max = -127;
            let mut average = 0;
            let mut total_weight = 0;
            for i in x - LMAP_MAX_DIST..=x + LMAP_MAX_DIST {
                let block = blocks[((i + LMAP_BLOCKS as i32) % LMAP_BLOCKS as i32) as usize];
                let w = weight((i - x).abs());
                min = min.min(block.min);
                max = max.max(block.max);
                average += block.avg * w;
                total_weight += w;
            }
            average /= total_weight;
            let med = ((min + max) / 2 + average) / 2;
            let cell = &mut depth[y as usize * width + x as usize];
            *cell = (i32::from(*cell) - med) as i8;
        }
    }
}
