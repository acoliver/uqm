//! Header-derived ABI used while navigation callers are still C.

use std::fs;

pub fn generate() -> Result<(), String> {
    let headers = [
        "globdata.h",
        "coderes.h",
        "encount.h",
        "starmap.h",
        "master.h",
        "setup.h",
        "units.h",
        "hyper.h",
        "resinst.h",
        "nameref.h",
        "build.h",
        "state.h",
        "grpinfo.h",
        "grpintrn.h",
        "gamestr.h",
        "gameev.h",
        "gendef.h",
        "intel.h",
        "ipdisp.h",
        "collide.h",
        "init.h",
        "process.h",
        "sounds.h",
        "sis.h",
        "starbase.h",
        "menustat.h",
        "status.h",
        "ship.h",
        "controls.h",
        "gameopt.h",
        "shipcont.h",
        "util.h",
        "flash.h",
        "settings.h",
        "planets/solarsys.h",
    ];
    let mut builder = bindgen::Builder::default()
        .clang_arg("-DFLASH_INTERNAL")
        .clang_arg("-I../sc2")
        .clang_arg("-I../sc2/src")
        .allowlist_file(".*/sc2/src/.*")
        .generate_comments(false)
        .layout_tests(true)
        .formatter(crate::ABI_BINDING_FORMATTER);
    if std::env::var_os("CARGO_FEATURE_DEBUG_PROCESS").is_some() {
        builder = builder.clang_arg("-DDEBUG");
    }
    for header in headers {
        let path = format!("../sc2/src/uqm/{header}");
        println!("cargo:rerun-if-changed={path}");
        builder = builder.header(path);
    }
    for header in [
        "options.h",
        "port.h",
        "libs/memlib.h",
        "libs/log.h",
        "libs/graphics/widgets.h",
        "libs/resource/stringbank.h",
        "libs/mathlib.h",
        "libs/inplib.h",
        "libs/sound/trackplayer.h",
        "libs/threadlib.h",
        "libs/graphics/gfx_common.h",
    ] {
        let path = format!("../sc2/src/{header}");
        println!("cargo:rerun-if-changed={path}");
        builder = builder.header(path);
    }
    let bindings = builder
        .generate()
        .map_err(|error| format!("navigation ABI: {error}"))?;
    fs::write(
        crate::output_directory()?.join("navigation_abi.rs"),
        bindings.to_string(),
    )
    .map_err(|error| format!("write navigation ABI: {error}"))
}
