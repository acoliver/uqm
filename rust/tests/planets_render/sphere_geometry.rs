//! Called only from planets_parity's existing serialized global-state owner.
use uqm_rust::graphics::{cmap::Color, drawable::Point};
use uqm_rust::math::{self, RandomContext};
use uqm_rust::planets::render::sphere_geometry as geometry;
#[path = "sphere_cases.rs"]
mod cases;

struct SavedState<'a> {
    rng: &'a RandomContext,
    system_seed: u32,
    global_seed: u32,
    screen: i32,
}
impl<'a> SavedState<'a> {
    unsafe fn snapshot(rng: &'a RandomContext) -> Self {
        let global_seed = math::seed_random(1);
        math::seed_random(global_seed);
        Self {
            rng,
            system_seed: rng.seed(),
            global_seed,
            screen: super::ScreenWidth,
        }
    }
    fn draws(&self) -> [u32; 8] {
        let draws = std::array::from_fn(|_| math::random());
        math::seed_random(self.global_seed);
        draws
    }
    unsafe fn check(&self, expected: [u32; 8]) {
        let screen = super::ScreenWidth;
        assert_eq!(self.rng.seed(), self.system_seed);
        assert_eq!(screen, self.screen);
        let current_seed = math::seed_random(self.global_seed);
        assert_eq!(current_seed, self.global_seed, "TFB global seed changed");
        assert_eq!(self.draws(), expected, "random stream changed");
    }
}
impl Drop for SavedState<'_> {
    fn drop(&mut self) {
        unsafe {
            super::ScreenWidth = self.screen;
        }
        math::seed_random(self.global_seed);
    }
}

pub fn original_c_and_rust_match_fixture_without_rng_effects(rng: &RandomContext) {
    let saved = unsafe { SavedState::snapshot(rng) };
    assert_eq!(saved.screen, 320, "native runtime logical width");
    let expected = saved.draws();
    let original = cases::original_output();
    let live_c = original.is_some();
    if let Some(output) = original {
        cases::check_fixture(&output);
    }
    unsafe { saved.check(expected) };
    cases::check_fixture(&cases::rust_output());
    unsafe { saved.check(expected) };
    cases::report_coverage(live_c);
}
