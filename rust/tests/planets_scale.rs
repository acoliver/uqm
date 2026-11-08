//! Full-buffer fixtures from the original TopoScale4x, including its persistent RNG.
use sha2::{Digest, Sha256};
use uqm_rust::planets::render::topo_scale::TopoScaler;

#[test]
fn original_c_four_consecutive_maps() {
    let kernel = uqm_rust::engine::Kernel::new();
    let mut scaler = TopoScaler::with_seed(kernel.planet_animation.topography_seed());
    let hashes = [
        "c0e0ab5bdf8d76a0ef75a289518aef715df00006e7f8c61c0f33198a1cbecdba",
        "484446a237fe38b1dbc2975c4d4b567d9bb758270d5b535d665891a7ef7f9e3d",
        "d1ba8972288c39f77bf35b50c60dd6a8b820f2bed00932bc35f94f474166f9f2",
        "5961185d1ff5d51ae9d43d0ac2b990fa86328b7b8d961e275a8b9016bae8ff2d",
    ];
    for (pattern, expected) in hashes.iter().enumerate() {
        let source: Vec<i8> = (0..242 * 75)
            .map(|i| match pattern {
                0 => 0,
                1 => -128,
                2 => 127,
                _ => (((i * 73 + (i / 242) * 151) % 256) - 128) as i8,
            })
            .collect();
        let output = scaler.scale(&source, 242, 75, pattern as i32 * 50, pattern as i32 * 3);
        let bytes: Vec<u8> = output.into_iter().map(|v| v as u8).collect();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            *expected,
            "pattern {pattern}"
        );
    }
}

#[test]
fn resumed_scaler_keeps_rng_sequence_in_one_kernel_only() {
    let first = uqm_rust::engine::Kernel::new();
    let second = uqm_rust::engine::Kernel::new();
    let initial_seed = first.planet_animation.topography_seed();
    let mut continuous = TopoScaler::with_seed(initial_seed);
    for source in [[0, 20, -20, 127], [-128, 1, 2, 3], [9, -4, 5, 6]] {
        let expected = continuous.scale(&source, 2, 2, 100, 6);
        let mut visit = TopoScaler::with_seed(first.planet_animation.topography_seed());
        assert_eq!(visit.scale(&source, 2, 2, 100, 6), expected);
        first.planet_animation.set_topography_seed(visit.seed());
        assert_eq!(first.planet_animation.topography_seed(), continuous.seed());
        assert_eq!(second.planet_animation.topography_seed(), initial_seed);
    }
}
