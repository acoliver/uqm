//! Terrain generation, elevation conditioning and relative lighting from `planets/plangen.c`.

use crate::comm::locdata::{CPoint, CRect};
use crate::math::RandomContext;
use crate::planets::topography::delta_topography;
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

#[derive(Clone, Copy)]
struct CraterInk {
    rim: i16,
    floor: i16,
    clear: bool,
}

fn crater_row(
    depth: &mut [i8],
    row: usize,
    mut left: i32,
    mut right: i32,
    rim_pixels: u16,
    ink: CraterInk,
) {
    if ink.clear {
        depth[row + left as usize..=row + right as usize].fill(0);
    }
    if left == right {
        depth[row + left as usize] = depth[row + left as usize].wrapping_add(ink.rim as i8);
        return;
    }
    for _ in 0..rim_pixels {
        depth[row + left as usize] = depth[row + left as usize].wrapping_add(ink.rim as i8);
        if left != right {
            depth[row + right as usize] = depth[row + right as usize].wrapping_add(ink.rim as i8);
        }
        left += 1;
        right -= 1;
    }
    while left <= right {
        depth[row + left as usize] = depth[row + left as usize].wrapping_add(ink.floor as i8);
        if left != right {
            depth[row + right as usize] = depth[row + right as usize].wrapping_add(ink.floor as i8);
        }
        left += 1;
        right -= 1;
    }
}

/// Draw the original midpoint crater, with byte-wrapping rim/floor additions.
///
/// The full map must fit C's unsigned 16-bit row offsets. Extents are
/// nonnegative, and the inclusive ellipse (corner + twice each half-extent)
/// must fit the map. Even extents therefore touch one cell beyond a usual
/// half-open rectangle. Zero axes retain C's degenerate point/line behavior.
/// Invalid geometry panics before changing the map; there is no clipping.
pub fn make_crater(
    depth: &mut [i8],
    width: usize,
    rect: &CRect,
    rim_delta: i16,
    crater_delta: i16,
    set_depth: bool,
) {
    assert_eq!(depth.len(), width * MAP_HEIGHT);
    assert!(width > 0 && depth.len() <= u16::MAX as usize);
    assert!(rect.width >= 0 && rect.height >= 0 && rect.corner.x >= 0 && rect.corner.y >= 0);
    let a = i64::from(rect.width >> 1);
    let b = i64::from(rect.height >> 1);
    assert!(i64::from(rect.corner.x) + 2 * a < width as i64);
    assert!(i64::from(rect.corner.y) + 2 * b < MAP_HEIGHT as i64);
    let center_x = (a + i64::from(rect.corner.x)) as i32;
    let center_y = (b + i64::from(rect.corner.y)) as usize;
    let (a_squared, b_squared) = (a * a, b * b);
    let (mut x, mut y) = (0_i32, b as usize);
    let (mut dx, mut dy) = (0_i64, 2 * a_squared * b);
    let mut d = b_squared - (dy >> 1) + (a_squared >> 2);
    let mut rim_pixels = 1_u16;
    let ink = CraterInk {
        rim: rim_delta,
        floor: crater_delta,
        clear: set_depth,
    };
    while dx < dy {
        if d > 0 {
            for row in [center_y - y, center_y + y] {
                crater_row(
                    depth,
                    row * width,
                    center_x - x,
                    center_x + x,
                    rim_pixels,
                    ink,
                );
            }
            // C consumes the rim counter in its drawing loop, including
            // the single-point special case, before accumulating the next rim.
            rim_pixels = 0;
            y -= 1;
            dy -= 2 * a_squared;
            d -= dy;
        }
        rim_pixels += 1;
        x += 1;
        dx += 2 * b_squared;
        d += b_squared + dx;
    }
    d += ((((a_squared - b_squared) * 3) >> 1) - (dx + dy)) >> 1;
    while y > 0 {
        for row in [center_y - y, center_y + y] {
            crater_row(
                depth,
                row * width,
                center_x - x,
                center_x + x,
                rim_pixels,
                ink,
            );
        }
        if d < 0 {
            x += 1;
            dx += 2 * b_squared;
            d += dx;
        }
        rim_pixels = 1;
        y -= 1;
        dy -= 2 * a_squared;
        d += a_squared - dy;
    }
    crater_row(
        depth,
        center_y * width,
        center_x - x,
        center_x + x,
        rim_pixels,
        ink,
    );
}

// plangen.c's NUM_BAND_COLORS and MakeStorms' local MAX_STORMS.
const NUM_BAND_COLORS: i32 = 4;
const MAX_STORMS: usize = 8;

fn check_storm_map(depth: &[i8], width: usize) {
    assert_eq!(depth.len(), width * MAP_HEIGHT);
    // The largest random storm has height 2*(MAP_HEIGHT/4)-1 and
    // width 2*height-1. Every candidate must leave a nonzero x divisor.
    assert!(width > 2 * (2 * (MAP_HEIGHT >> 2) - 1) - 1);
    assert!(depth.len() <= u16::MAX as usize);
}

fn inset_storm(rect: &mut CRect) {
    rect.corner.x += 1;
    rect.corner.y += 1;
    rect.width -= 2;
    rect.height -= 2;
}

/// Place up to eight non-overlapping nested storms using the system stream.
///
/// This uses C's reverse insertion order and tests against the already inset
/// rectangles, not their initial bounds. Rejected candidates consume both
/// draws. The map must admit every random candidate (`width >= 70` for the
/// current 75-row map) and fit unsigned 16-bit row offsets. Invalid dimensions
/// or counts panic before map or RNG mutation. Rejection has no retry cap.
pub fn make_storms(rng: &mut RandomContext, depth: &mut [i8], width: usize, count: u16) {
    check_storm_map(depth, width);
    assert!(usize::from(count) <= MAX_STORMS);
    let mut storms = [CRect::default(); MAX_STORMS];
    for i in (0..usize::from(count)).rev() {
        let loword = loop {
            let random = rng.next_random();
            let hiword = (random >> 16) as u16;
            let byte = i32::from(hiword as u8);
            let height = match (hiword >> 8) & 31 {
                0 => byte % (MAP_HEIGHT as i32 >> 2) + (MAP_HEIGHT as i32 >> 2),
                1..=4 => byte % (MAP_HEIGHT as i32 >> 3) + (MAP_HEIGHT as i32 >> 3),
                _ => byte % (MAP_HEIGHT as i32 >> 4) + 4,
            };
            let height = if height <= 4 { height + 4 } else { height };
            let loword = rng.next_random() as u16;
            let w = height + i32::from(loword as u8) % height;
            let rect = CRect {
                corner: CPoint {
                    x: (i32::from(loword >> 8) % (width as i32 - w)) as i16,
                    y: (i32::from(loword as u8) % (MAP_HEIGHT as i32 - height)) as i16,
                },
                width: w as i16,
                height: height as i16,
            };
            let intersects = storms[i + 1..usize::from(count)].iter().any(|other| {
                let x = other.corner.x - rect.corner.x;
                let y = other.corner.y - rect.corner.y;
                x + other.width + 4 > 0
                    && y + other.height + 4 > 0
                    && x < rect.width + 4
                    && y < rect.height + 4
            });
            storms[i] = rect;
            if !intersects {
                break loword;
            }
        };
        let rect = &mut storms[i];
        make_crater(depth, width, rect, 6, 6, false);
        inset_storm(rect);
        let mut delta = ((loword >> 8) & ((3 << RANGE_SHIFT) + 20)) as i16;
        make_crater(depth, width, rect, delta, delta, true);
        inset_storm(rect);
        for _ in 0..2 {
            delta += 2;
            if rect.width > 2 && rect.height > 2 {
                make_crater(depth, width, rect, delta, delta, true);
                inset_storm(rect);
            }
        }
        delta += 4;
        make_crater(depth, width, rect, delta, delta, true);
    }
}

/// Add gas bands, 50 faults at each internal boundary, storms and dithering.
///
/// The extent is the complete map, as in GeneratePlanetSurface's call. Band
/// count must be nonzero and leave at least three rows per band; otherwise C
/// divides by zero or addresses outside the map. Dimensions use make_storms'
/// contract. All preconditions are checked before RNG or map mutation.
/// Signed band steps are multiplied rather than left-shifting negative values.
pub fn make_gas_giant(
    rng: &mut RandomContext,
    depth: &mut [i8],
    width: usize,
    num_bands: u16,
    depth_delta: i16,
) {
    check_storm_map(depth, width);
    assert!(num_bands > 0 && usize::from(num_bands) <= MAP_HEIGHT / 3);
    let band_height = MAP_HEIGHT as i32 / i32::from(num_bands);
    let band_bump = MAP_HEIGHT as i32 % i32::from(num_bands);
    let mut error = i32::from(num_bands >> 1);
    let mut delta = ((rng.next_random() as i32 & (NUM_BAND_COLORS - 1)) << RANGE_SHIFT)
        + (1 << (RANGE_SHIFT - 1));
    let (mut next_y, mut last_y) = (0_i32, 0_usize);
    for i in (1..=num_bands).rev() {
        let random = rng.next_random();
        let loword = random as u16;
        let hiword = (random >> 16) as u16;
        next_y += band_height;
        error -= band_bump;
        if error < 0 {
            next_y += 1;
            error += i32::from(num_bands);
        }
        let cur_y = if i == 1 {
            MAP_HEIGHT
        } else {
            let y = next_y + ((band_height - 2) >> 1)
                - (i32::from(hiword as u8) % (band_height - 2) + 1);
            let start = (y as usize - 2) * width;
            delta_topography(
                rng,
                50,
                &mut depth[start..start + 5 * width],
                width as i16,
                5,
                depth_delta,
            );
            y as usize
        };
        for cell in &mut depth[last_y * width..cur_y * width] {
            *cell = cell.wrapping_add(delta as i8);
        }
        last_y = cur_y;
        delta = (delta + ((i32::from(loword as u8 & 1) * 2 - 1) * (1 << RANGE_SHIFT)))
            & ((1 << RANGE_SHIFT) * NUM_BAND_COLORS - 1);
    }
    let storms = 5 + (rng.next_random() & 3) as u16;
    make_storms(rng, depth, width, storms);
    dither_map(rng, depth, width);
}
