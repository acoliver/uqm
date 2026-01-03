fn main() {
    // Build the minimal C wrapper for Phase 0
    cc::Build::new()
        .include("../sc2/src")
        .warnings(true)
        .file("../sc2/src/mem_wrapper.c")
        .cpp(false)
        .compile("uqm_core");


    println!("cargo:rerun-if-changed=../sc2/src/mem_wrapper.c");

    // Create minimal manual FFI bindings for Phase 0
    // In later phases, we'll use bindgen to auto-generate these
}
