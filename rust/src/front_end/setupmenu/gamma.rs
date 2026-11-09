//! Slider conversion and bounds for the setup gamma control.

use std::ffi::c_int;

const GAMMA_CURVE_B: f32 = 2.1972;

#[derive(Clone, Copy, Debug)]
pub struct GammaRange {
    pub min: f32,
    pub max: f32,
    pub(super) min_x: f32,
    pub(super) max_x: f32,
}
impl GammaRange {
    pub fn new(min: f32, max: f32) -> Self {
        let mut range = Self {
            min,
            max,
            min_x: 0.0,
            max_x: 0.0,
        };
        range.update_bounds(true);
        range
    }
    fn curve(x: f32) -> f32 {
        (((x - 1.0) * GAMMA_CURVE_B) as f64).exp() as f32
    }

    pub(super) fn step(
        &mut self,
        value: &mut i32,
        direction: i32,
        mut set: impl FnMut(f32) -> bool,
    ) {
        let previous = *value;
        *value += direction;
        let mut gamma = self.slider_to_gamma(*value);
        let accepted = set(gamma);
        if !accepted {
            *value = previous;
            gamma = self.slider_to_gamma(*value);
        }
        if gamma < self.min || (!accepted && direction < 0) {
            self.min = gamma;
            self.update_bounds(true);
            *value = 0;
        } else if gamma > self.max || (!accepted && direction > 0) {
            self.max = gamma;
            self.update_bounds(false);
            *value = 100;
        }
    }

    fn solve(y: f32) -> f32 {
        ((y as f64).ln() / GAMMA_CURVE_B as f64 + 1.0) as f32
    }
    pub fn gamma_to_slider(&self, gamma: f32) -> c_int {
        let x = Self::solve(gamma);
        let step = (self.max_x - self.min_x) / 100.0;
        (((x - self.min_x) / step) as f64 + 0.5) as c_int
    }
    pub fn slider_to_gamma(&self, value: c_int) -> f32 {
        let step = (self.max_x - self.min_x) / 100.0;
        let x = self.min_x + step * value as f32;
        let g = Self::curve(x);
        if ((g - 1.0) as f64).abs() < 0.001_f32 as f64 {
            1.0
        } else {
            g
        }
    }
    pub fn update_bounds(&mut self, use_upper: bool) {
        self.min_x = Self::solve(self.min);
        self.max_x = Self::solve(self.max);
        let slider = self.gamma_to_slider(1.0);
        let g = self.slider_to_gamma(slider);
        if g == 1.0 {
            return;
        }
        let x = Self::solve(g);
        if use_upper {
            let d = (x - 1.0) * 100.0 / slider as f32;
            self.max_x -= d;
            self.max = Self::curve(self.max_x);
        } else {
            let d = (x - 1.0) * 100.0 / (100 - slider) as f32;
            self.min_x -= d;
            self.min = Self::curve(self.min_x);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GammaRange;
    #[test]
    fn gamma_range_has_exact_unity_step() {
        for upper in [true, false] {
            let mut range = GammaRange::new(0.4, 2.5);
            range.update_bounds(upper);
            assert_eq!(range.slider_to_gamma(range.gamma_to_slider(1.0)), 1.0);
            assert!(range.slider_to_gamma(0) < 1.0);
            assert!(range.slider_to_gamma(100) > 1.0);
        }
    }
    #[test]
    fn gamma_slider_is_monotonic_and_round_trips() {
        let range = GammaRange::new(0.4, 2.5);
        for value in 0..=100 {
            assert_eq!(range.gamma_to_slider(range.slider_to_gamma(value)), value);
            if value > 0 {
                assert!(range.slider_to_gamma(value) > range.slider_to_gamma(value - 1));
            }
        }
    }
}
