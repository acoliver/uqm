//! The fourfold interpolator from plangen.c. Its private RNG persists between visits.

type FillLine = (usize, usize, usize, usize, i32, i32);

const LINES: &[&[FillLine]] = &[
    &[
        (0, 2, 2, 0, 1, -1),
        (0, 3, 3, 0, 1, -1),
        (0, 4, 4, 0, 1, -1),
        (1, 4, 4, 1, 1, -1),
        (2, 4, 4, 2, 1, -1),
    ],
    &[
        (0, 2, 2, 4, 1, 1),
        (0, 1, 3, 4, 1, 1),
        (0, 0, 4, 4, 1, 1),
        (1, 0, 4, 3, 1, 1),
        (2, 0, 4, 2, 1, 1),
    ],
    &[(0, 1, 4, 1, 1, 0), (0, 2, 4, 2, 1, 0), (0, 3, 4, 3, 1, 0)],
    &[(1, 0, 1, 4, 0, 1), (2, 0, 2, 4, 0, 1), (3, 0, 3, 4, 0, 1)],
];

pub struct TopoScaler {
    seed: u32,
}

impl Default for TopoScaler {
    fn default() -> Self {
        Self { seed: 0x12345678 }
    }
}

impl TopoScaler {
    fn random(&mut self) -> u32 {
        if self.seed == 0 {
            self.seed = 15807;
        }
        self.seed = (self.seed >> 4).wrapping_mul(227);
        self.seed
    }

    fn variance(&mut self, factor: i32) -> i32 {
        if factor == 0 {
            0
        } else {
            (self.random() % factor as u32).wrapping_sub((factor >> 1) as u32) as i32
        }
    }

    pub fn scale(
        &mut self,
        source: &[i8],
        width: usize,
        height: usize,
        faults: i32,
        fault_var: i32,
    ) -> Vec<i8> {
        assert_eq!(source.len(), width * height);
        let pitch = width * 4;
        let mut output = vec![0; width * height * 16];
        let mut previous = vec![0; pitch + 1];
        let allowed = (faults << 8) / 250;
        let minimum = fault_var << 8;
        let factor = |step: i32| ((step.abs() * allowed) >> 8) + minimum;
        previous[0] = i32::from(source[0]) << 8;
        for x in 0..width {
            let column = x * 4;
            previous[column + 4] = i32::from(source[(x + 1) % width]) << 8;
            let mut value = previous[column];
            let step = (previous[column + 4] - value) / 3;
            let variance = factor(step);
            value += step;
            for x2 in 1..4 {
                previous[column + x2] = value + self.variance(variance);
                value += step;
            }
        }
        let mut elev = [[0; 5]; 5];
        for y in 0..height {
            elev[4][0] = previous[0];
            elev[4][4] = if y < height - 1 {
                i32::from(source[(y + 1) * width]) << 8
            } else {
                elev[4][0]
            };
            let mut value = elev[4][0];
            let step = (elev[4][4] - value) / 3;
            let variance = factor(step);
            value += step;
            for cell in &mut elev[4][1..4] {
                *cell = value + self.variance(variance);
                value += step;
            }
            for x in 0..width {
                for x2 in 0..=4 {
                    elev[x2][0] = previous[x * 4 + x2];
                }
                let previous_column = elev[4];
                elev[0][1..=4].copy_from_slice(&previous_column[1..=4]);
                elev[4][4] = if y < height - 1 {
                    i32::from(source[(y + 1) * width + (x + 1) % width]) << 8
                } else {
                    elev[4][0]
                };
                let mut value = elev[0][4];
                let step = (elev[4][4] - value) / 3;
                let variance = factor(step);
                value += step;
                for column in elev.iter_mut().take(4).skip(1) {
                    column[4] = value + self.variance(variance);
                    value += step;
                }
                let mut value = elev[4][0];
                let step = (elev[4][4] - value) / 3;
                let variance = factor(step);
                value += step;
                for cell in &mut elev[4][1..4] {
                    *cell = value + self.variance(variance);
                    value += step;
                }
                for &(x0, y0, x1, y1, dx, dy) in LINES[(self.random() & 3) as usize] {
                    let value = elev[x0][y0];
                    let mut steps = x1 as i32 - x0 as i32;
                    if steps == 0 {
                        steps = y1 as i32 - y0 as i32;
                    }
                    let step = (elev[x1][y1] - value) / steps;
                    let variance = factor(step);
                    let (mut x2, mut y2, mut value) =
                        (x0 as i32 + dx, y0 as i32 + dy, value + step);
                    while x2 != x1 as i32 || y2 != y1 as i32 {
                        elev[x2 as usize][y2 as usize] = value + self.variance(variance);
                        x2 += dx;
                        y2 += dy;
                        value += step;
                    }
                }
                for y2 in 0..4 {
                    for x2 in 0..4 {
                        output[(y * 4 + y2) * pitch + x * 4 + x2] =
                            (elev[x2][y2] >> 8).clamp(-128, 127) as i8;
                    }
                }
                for x2 in 0..4 {
                    previous[x * 4 + x2] = elev[x2][4];
                }
            }
            previous[pitch] = elev[4][4];
        }
        output
    }
}
