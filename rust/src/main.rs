mod cli;
mod config;
mod logging;
mod memory;
mod c_bindings;

use cli::Cli;
use anyhow::Result;
use std::env;
use std::ffi::CString;

fn main() -> Result<i32> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging (early)
    unsafe {
        logging::log_init(15);
        log_info!("Phase 0 Rust launcher starting...");
    }

    // Load configuration file (defaults to user config directory)
    // For Phase 0, this returns default options
    let options = config::load_config(&cli.configdir)?;

    // Merge CLI options into config
    let mut options = cli.merge_into_options(options)?;

    // Handle special modes from the CLI
    if let Some(log_file) = &options.log_file {
        unsafe {
            log_info!("Redirecting logs to: {}", log_file);
        }
    }

    unsafe {
        // Initialize memory management
        if !memory::rust_mem_init() {
            log_error!("Failed to initialize memory management");
            return 1;
        }

        // Prepare arguments for C code
        let args: Vec<String> = env::args().collect();
        log_info!("Command line arguments: {:?}", args);

        // Convert arguments to C-compatible format
        let c_args: Vec<CString> = args
            .iter()
            .map(|s| CString::new(s.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to convert arguments: {}", e))?;

        let mut c_argv: Vec<*mut i8> = c_args
            .iter()
            .map(|cs| cs.as_ptr() as *mut i8)
            .collect();
        c_argv.push(std::ptr::null_mut()); // Null-terminate

        // Display configuration
        log_info!("Configuration:");
        if let Some(res) = &options.resolution {
            log_info!("  Resolution: {}x{}", res.width, res.height);
        }
        if let Some(fullscreen) = options.fullscreen {
            log_info!("  Fullscreen: {}", fullscreen);
        }
        if let Some(opengl) = options.opengl {
            log_info!("  OpenGL: {}", opengl);
        }
        if let Some(log_file) = &options.log_file {
            log_info!("  Log file: {}", log_file);
        }
        if let Some(content_dir) = &options.content_dir {
            log_info!("  Content dir: {}", content_dir);
        }
        if let Some(config_dir) = &options.config_dir {
            log_info!("  Config dir: {}", config_dir);
        }

        log_info!("Calling C entry point...");
        let exit_code = c_bindings::c_entry_point(args.len() as i32, c_argv.as_mut_ptr());

        // Cleanup
        if !memory::rust_mem_uninit() {
            log_warning!("Failed to deinitialize memory management");
        }

        log_info!("C entry point returned: {}", exit_code);
        Ok(exit_code)
    }
}
