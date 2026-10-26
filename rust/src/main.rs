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

    if parsed.parse_error {
        eprintln!("Invalid option. Run with -h to see the allowed arguments.");
        exit(1);
    }

    // Handle early-exit modes (version/usage) that bypass the full init
    match parsed.run_mode {
        options::RunMode::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            exit(0);
        }
        options::RunMode::Usage => {
            print_usage();
            exit(0);
        }
        options::RunMode::CheckContent => {
            match uqm_rust::mainloop::content::check(parsed.content_dir.as_deref()) {
                Ok(path) => println!("Content OK: {}", path.display()),
                Err(error) => {
                    eprintln!("Content check failed: {error:#}");
                    exit(1);
                }
            }
            exit(0);
        }
        options::RunMode::Normal => {}
    }

    uqm_rust::time::source::install(parsed.replay.is_some());
    logging::init(&parsed.log_file);

    // The frame loop runs on this thread; see threading::qos. Declared after
    // argument handling so --version, --help and --check-content stay inert.
    #[cfg(target_os = "macos")]
    uqm_rust::threading::qos::declare_game_thread_interactive();

    // Automation setup: validate before run_uqm (REQ-MODE-001)
    let mut auto_opts = uqm_rust::automation::AutomationOptions {
        script_path: parsed.automation_script.clone(),
        output_dir: parsed.automation_output.clone(),
    };
    if let Some(scenario) = &parsed.replay {
        if parsed.native_window_proof.is_some() {
            eprintln!("Replay cannot be combined with native-window proof.");
            exit(1);
        }
        let path = std::path::PathBuf::from(scenario);
        let path = if path.is_file() {
            path
        } else {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts")
                .join(format!("{scenario}.json"))
        };
        auto_opts.script_path = Some(path.to_string_lossy().into_owned());
        if auto_opts.output_dir.is_none() {
            let output = parsed.replay_summary.as_ref().map_or_else(
                || std::path::PathBuf::from("tmp/replay"),
                |path| std::path::PathBuf::from(path).with_extension("artifacts"),
            );
            auto_opts.output_dir = Some(output.to_string_lossy().into_owned());
        }
    }

    // Build capabilities resolved from build.vars at compile time
    let caps = uqm_rust::automation::BuildCapabilities::from_flags(&[
        ("RUST_OWNS_MAIN", true),
        ("USE_RUST_THREADS", true),
        ("USE_RUST_GFX", true),
        ("USE_RUST_COMM", true),
        ("USE_RUST_RESTART", true),
    ]);

    match uqm_rust::automation::setup_automation(&auto_opts, &caps) {
        Ok(Some(setup)) => {
            #[cfg(feature = "debug-process")]
            if let Some(config) = parsed.native_window_proof.as_deref() {
                if let Err(error) = uqm_rust::automation::activate_native_window_proof(
                    std::path::Path::new(config),
                    &setup.output_root,
                ) {
                    eprintln!("Native-window proof setup failed: {error}");
                    exit(1);
                }
            }
            // Active automation validated — initialize the coordinator.
            uqm_rust::automation::Coordinator::init(setup.script, setup.output_root);
        }
        Ok(None) => {
            if parsed.native_window_proof.is_some() {
                eprintln!("Native-window proof requires active automation.");
                exit(1);
            }
            // Inactive — proceed normally
        }
        Err(e) => {
            eprintln!("Automation setup failed: {e}");
            exit(1);
        }
    }

    // Run the full UQM lifecycle: init -> game loop -> teardown
    // SAFETY: the main thread keeps all argument CStrings and argv storage alive through shutdown.
    let exit_code = unsafe { init_sequence::run_uqm(args.len() as c_int, c_argv.as_mut_ptr()) };

    // Finalize automation after teardown and propagate its terminal/evidence status.
    let exit_code = match uqm_rust::automation::Coordinator::finalize(exit_code) {
        Ok(status) => status,
        Err(error) => {
            eprintln!("Automation finalization failed: {error}");
            1
        }
    };

    if let Some(path) = &parsed.replay_summary {
        if let Err(error) =
            uqm_rust::automation::Coordinator::write_replay_summary(std::path::Path::new(path))
        {
            eprintln!("Replay summary failed: {error}");
            exit(1);
        }
    }
    exit(exit_code);
}

fn print_usage() {
    println!("Options:");
    println!("  --version                 (print version and exit) ");
    println!("  --check-content           (verify content without config, saves or SDL)");
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
    println!("  --replay=SCENARIO         (deterministic scenario execution)");
    println!("  --replay-summary=FILE     (write replay counters and completion)");
    println!("  --automation-script=FILE   (enable runtime automation)");
    println!("  --automation-output=DIR    (automation output directory)");
    #[cfg(feature = "debug-process")]
    println!("  --native-window-proof=FILE (proof-only native-window configuration)");
    println!("The following can take '3do' or 'pc':");
    println!("  -i, --intro   : Intro/ending version (default 3do)");
    println!("  --cscan       : coarse-scan display (default 3do)");
    println!("  --menu        : menu type (default pc)");
    println!("  --font        : font types and colors (default pc)");
    println!("  --shield      : slave shield type (default 3do)");
    println!("  --scroll      : comm scroll (default 3do)");
}
