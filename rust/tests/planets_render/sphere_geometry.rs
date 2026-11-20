//! Called only from planets_parity's existing serialized global-state owner.
use uqm_rust::graphics::{cmap::Color, drawable::Point};
use uqm_rust::math::{self, RandomContext};
use uqm_rust::planets::render::sphere_geometry as geometry;
#[path = "sphere_cases.rs"]
mod cases;
use std::ffi::{c_void, CString};

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

fn original_output() -> Option<Vec<u8>> {
    type Run = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
    let path = std::env::var_os("UQM_SPHERE_GEOMETRY_ORACLE")?;
    let path = CString::new(path.as_encoded_bytes())
        .unwrap_or_else(|error| panic!("oracle path has a NUL: {error}"));
    let input = cases::input();
    let input_before = input.clone();
    // Guard the caller's capture allocation as well as the C internal buffer.
    let mut output = vec![0xcd; cases::FIXTURE.len() + 128];
    // SAFETY: the oracle exports `original_sphere_geometry` with this signature;
    // `output` holds the fixture plus 64 guard bytes on each side and `input`
    // outlives the call.
    unsafe {
        let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
        assert!(
            !handle.is_null(),
            "load original C sphere oracle: {:?}",
            std::ffi::CStr::from_ptr(libc::dlerror())
        );
        let symbol = libc::dlsym(handle, c"original_sphere_geometry".as_ptr());
        assert!(!symbol.is_null());
        let run = std::mem::transmute::<*mut c_void, Run>(symbol);
        assert_eq!(
            run(
                output.as_mut_ptr().add(64),
                cases::FIXTURE.len(),
                input.as_ptr(),
                input.len()
            ),
            cases::FIXTURE.len(),
            "C result length"
        );
        assert_eq!(libc::dlclose(handle), 0);
    }
    assert_eq!(input, input_before, "C changed its inputs");
    assert_eq!(&output[..64], &[0xcd; 64]);
    assert_eq!(&output[output.len() - 64..], &[0xcd; 64]);
    Some(output[64..output.len() - 64].to_vec())
}

pub fn original_c_and_rust_match_fixture_without_rng_effects(rng: &RandomContext) {
    let saved = unsafe { SavedState::snapshot(rng) };
    assert_eq!(saved.screen, 320, "native runtime logical width");
    let expected = saved.draws();
    let original = original_output();
    let live_c = original.is_some();
    if let Some(output) = original {
        cases::check_fixture(&output);
    }
    unsafe { saved.check(expected) };
    cases::check_fixture(&cases::rust_output());
    unsafe { saved.check(expected) };
    cases::report_coverage(live_c);
}
