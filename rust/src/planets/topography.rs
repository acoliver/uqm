//! Fault-line terrain generation, ported from `planets/gentopo.c`.
//!
//! `delta_topography` raises and lowers a wrapped band of the topography map
//! along pairs of random Bresenham lines. It draws from the shared system
//! generator stream in the same order and count as C, so the planets the
//! player sees (and everything drawn after them from that stream) stay stable.

use crate::math::RandomContext;

/// One of the two edges of a fault band (`LineDDA0` / `LineDDA1`).
struct LineDda {
    x_top: i32,
    x_incr: i32,
    delta_x: i32,
    error_term: i32,
}

impl LineDda {
    fn new(x_top: i32, x_bot: i32, delta_y: i32) -> Self {
        let delta_x = (x_bot - x_top) << 1;
        let (x_incr, delta_x) = if delta_x >= 0 {
            (1, delta_x)
        } else {
            (-1, -delta_x)
        };
        let error_term = if delta_x > delta_y {
            -(delta_x >> 1)
        } else {
            -(delta_y >> 1)
        };
        Self {
            x_top,
            x_incr,
            delta_x,
            error_term,
        }
    }

    /// Advance one map row. Returns how far the line moved horizontally.
    fn step(&mut self, delta_y: i32) -> i32 {
        let mut moved = 0;
        if delta_y >= self.delta_x {
            self.error_term += self.delta_x;
            if self.error_term >= 0 {
                moved = self.x_incr;
                self.error_term -= delta_y;
            }
        } else {
            loop {
                moved += self.x_incr;
                self.error_term += delta_y;
                if self.error_term >= 0 {
                    break;
                }
            }
            self.error_term -= self.delta_x;
        }
        self.x_top += moved;
        moved
    }
}

/// Add `delta` to `count` consecutive cells from `*cursor`, skipping cells the
/// addition would push out of the signed-byte range, as C does.
fn adjust_run(depth: &mut [i8], cursor: &mut isize, count: u16, delta: i32) {
    for _ in 0..count {
        let index = usize::try_from(*cursor).expect("topography cursor left the map");
        let cell = &mut depth[index];
        if let Ok(adjusted) = i8::try_from(i32::from(*cell) + delta) {
            *cell = adjusted;
        }
        *cursor += 1;
    }
}

/// `DeltaTopography (num_iterations, DepthArray, pRect, depth_delta)`, where
/// `pRect` contributes only its extent (`width` x `height`).
pub fn delta_topography(
    rng: &mut RandomContext,
    num_iterations: u16,
    depth: &mut [i8],
    width: i16,
    height: i16,
    mut depth_delta: i16,
) {
    let width = i32::from(width);
    let height = i32::from(height);
    let delta_y = (height - 1) << 1;
    let row = width as isize;

    let mut iterations_left = num_iterations;
    loop {
        if rng.next_random() & 1 == 0 {
            depth_delta = depth_delta.wrapping_neg();
        }

        let rand_val = rng.next_random();
        let w1 = rand_val as u16;
        let w2 = (rand_val >> 16) as u16;

        let bot0 = i32::from((w1 >> 8) as u8) % width;
        let mut line0 = LineDda::new(i32::from(w1 as u8) % width, bot0, delta_y);
        let top1 = (i32::from(w2 as u8) % (width - 1)) + line0.x_top + 1;
        let bot1 = (i32::from((w2 >> 8) as u8) % (width - 1)) + bot0 + 1;
        let mut line1 = LineDda::new(top1, bot1, delta_y);

        let mut cursor = line0.x_top as isize;
        let mut rows_left = height as u16;
        loop {
            let delta = i32::from(depth_delta);

            // The row splits into a span raised by `delta` and one lowered by
            // it; both lengths are fixed before either is drawn.
            let first = (line1.x_top - line0.x_top) as u16;
            let spans = [(first, 1), ((width as u16).wrapping_sub(first), -1)];
            for (mut span, sign) in spans {
                let run = if line0.x_top + i32::from(span) > width {
                    (width - line0.x_top) as u16
                } else {
                    line0.x_top += i32::from(span);
                    span
                };
                span = span.wrapping_sub(run);
                adjust_run(depth, &mut cursor, run, sign * delta);
                if span == 0 {
                    if line0.x_top == width {
                        line0.x_top = 0;
                        cursor -= row;
                    }
                } else {
                    line0.x_top = i32::from(span);
                    cursor -= row;
                    adjust_run(depth, &mut cursor, span, sign * delta);
                }
            }

            cursor += row;
            cursor += line0.step(delta_y) as isize;
            line1.step(delta_y);

            rows_left = rows_left.wrapping_sub(1);
            if rows_left == 0 {
                break;
            }
        }

        iterations_left = iterations_left.wrapping_sub(1);
        if iterations_left == 0 {
            break;
        }
    }
}
