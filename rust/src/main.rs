//! UQM (Ur-Quan Masters) -- Rust-owned main entry point.
//!
//! @plan PLAN-20260707-BINARY-INVERSION.P05
//!
//! When built with RUST_OWNS_MAIN, this binary is the process entry point.
//! It calls the C init sequence, runs the game loop on the main thread
//! (no separate game thread), and tears down subsystems on exit.

use std::ffi::CString;
use std::os::raw::c_int;
use std::process::exit;

use uqm_rust::mainloop::init_sequence;
use uqm_rust::mainloop::logging;
use uqm_rust::mainloop::options;

fn main() {
    // Collect args as C strings for the C init sequence
    let args: Vec<String> = std::env::args().collect();
    let c_args: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap_or_default())
        .collect();
    let mut c_argv: Vec<*mut std::os::raw::c_char> = c_args
        .iter()
        .map(|cs| cs.as_ptr() as *mut std::os::raw::c_char)
        .collect();
    c_argv.push(std::ptr::null_mut());

    // Parse command-line options in Rust (replaces C parseOptions)
    let parsed = options::parse_and_store(&args);

    // Initialize tracing-based logging early
    logging::init(&parsed.log_file);

    // Handle early-exit modes (version/usage) that bypass the full init
    match parsed.run_mode {
        options::RunMode::Version => {
            println!("0.8.0");
            exit(0);
        }
        options::RunMode::Usage => {
            print_usage();
            exit(0);
        }
        options::RunMode::Normal => {}
    }

    if parsed.parse_error {
        eprintln!("Invalid option. Run with -h to see the allowed arguments.");
        exit(1);
    }

    // Automation setup: validate before run_uqm (REQ-MODE-001)
    let auto_opts = uqm_rust::automation::AutomationOptions {
        script_path: parsed.automation_script.clone(),
        output_dir: parsed.automation_output.clone(),
    };

    // Build capabilities resolved from build.vars at compile time
    let caps = uqm_rust::automation::BuildCapabilities::from_flags(&[
        ("RUST_OWNS_MAIN", true),
        ("USE_RUST_THREADS", true),
        ("USE_RUST_GFX", true),
        ("USE_RUST_COMM", true),
        ("USE_RUST_RESTART", true),
    ]);

    match uqm_rust::automation::setup_automation(&auto_opts, &caps) {
        Ok(Some(_setup)) => {
            // Active automation validated — proceed to run_uqm
        }
        Ok(None) => {
            // Inactive — proceed normally
        }
        Err(e) => {
            eprintln!("Automation setup failed: {e}");
            exit(1);
        }
    }

    // Run the full UQM lifecycle: init -> game loop -> teardown
    let exit_code = init_sequence::run_uqm(args.len() as c_int, c_argv.as_mut_ptr());
    exit(exit_code);
}

fn print_usage() {
    println!("Options:");
    println!("  -r, --res=WIDTHxHEIGHT (default 1280x960)");
    println!("  -f, --fullscreen (default off)");
    println!("  -w, --windowed (default on)");
    println!("  -o, --opengl (default off)");
    println!("  -x, --nogl (default on)");
    println!("  -k, --keepaspectratio (default on)");
    println!("  -c, --scale=MODE (hq, xbrz3, xbrz4, or none) (default xbrz3)");
    println!("  -b, --meleezoom=MODE (step/pc or smooth/3do; default 3do)");
    println!("  -s, --scanlines (default off)");
    println!("  -p, --fps (default off)");
    println!("  -g, --gamma=CORRECTIONVALUE (default 1.0)");
    println!("  -C, --configdir=CONFIGDIR");
    println!("  -n, --contentdir=CONTENTDIR");
    println!("  -M, --musicvol=VOLUME (0-100, default 100)");
    println!("  -S, --sfxvol=VOLUME (0-100, default 100)");
    println!("  -T, --speechvol=VOLUME (0-100, default 100)");
    println!("  -q, --audioquality=QUALITY (high, medium or low, default medium)");
    println!("  -u, --nosubtitles");
    println!("  -l, --logfile=FILE (sends console output to logfile FILE)");
    println!("  --addon ADDON (may be specified multiple times)");
    println!("  --addondir=ADDONDIR");
    println!("  --renderer=name");
    println!("  --sound=DRIVER (openal, mixsdl, none; default mixsdl)");
    println!("  --stereosfx (positional sound effects)");
    println!("  --safe (start in safe mode)");
    println!("  --automation-script=FILE   (enable runtime automation)");
    println!("  --automation-output=DIR    (automation output directory)");
    println!("The following can take '3do' or 'pc':");
    println!("  -i, --intro   : Intro/ending version (default 3do)");
    println!("  --cscan       : coarse-scan display (default 3do)");
    println!("  --menu        : menu type (default pc)");
    println!("  --font        : font types and colors (default pc)");
    println!("  --shield      : slave shield type (default 3do)");
    println!("  --scroll      : comm scroll (default 3do)");
}
