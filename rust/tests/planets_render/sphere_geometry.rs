//! Called only from planets_parity's existing serialized global-state owner.
use uqm_rust::graphics::{cmap::Color, drawable::Point};
use uqm_rust::math::{self, RandomContext};
use uqm_rust::planets::render::sphere_geometry as geometry;
#[path = "sphere_cases.rs"]
mod cases;

struct SavedState {
    pointer: *mut RandomContext,
    system_seed: u32,
    global_seed: u32,
    screen: i32,
}
impl SavedState {
    unsafe fn snapshot() -> Self {
        let pointer = super::SysGenRNG;
        assert!(!pointer.is_null());
        let global_seed = math::seed_random(1);
        math::seed_random(global_seed);
        Self {
            pointer,
            system_seed: (*pointer).seed(),
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
        let pointer = super::SysGenRNG;
        let screen = super::ScreenWidth;
        assert_eq!(pointer, self.pointer, "SysGenRNG pointer changed");
        assert_eq!((*self.pointer).seed(), self.system_seed);
        assert_eq!(screen, self.screen);
        let current_seed = math::seed_random(self.global_seed);
        assert_eq!(current_seed, self.global_seed, "TFB global seed changed");
        assert_eq!(self.draws(), expected, "random stream changed");
    }
}
impl Drop for SavedState {
    fn drop(&mut self) {
        unsafe {
            super::SysGenRNG = self.pointer;
            (*self.pointer).reseed(self.system_seed);
            super::ScreenWidth = self.screen;
        }
        math::seed_random(self.global_seed);
    }
}

pub fn original_c_and_rust_match_fixture_without_rng_effects() {
    let saved = unsafe { SavedState::snapshot() };
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
