//! Command-line option parsing for UQM — replaces C `parseOptions()`.
//!
//! Uses clap for parsing, then exposes the result to C via FFI globals.
//!
//! @plan Port parseOptions to Rust

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::OnceLock;

use clap::{Arg, ArgAction, Command};

// ===========================================================================
//  C constants (mirrored from C headers)
// ===========================================================================

// TFB_GFXFLAGS_SCALE_* (gfx_common.h)
const TFB_GFXFLAGS_SCALE_XBRZ3: c_int = 3;
const TFB_GFXFLAGS_SCALE_XBRZ4: c_int = 4;
const TFB_GFXFLAGS_SCALE_HQXX: c_int = 2;
// audio_DRIVER_* (sound.h)
const AUDIO_DRIVER_MIXSDL: c_int = 1;
const AUDIO_DRIVER_NOSOUND: c_int = 2;
// audio_QUALITY_* (sound.h)
const AUDIO_QUALITY_LOW: c_int = 0;
const AUDIO_QUALITY_MEDIUM: c_int = 1;
const AUDIO_QUALITY_HIGH: c_int = 2;
// TFB_SCALE_* (gfx_common.h)
const TFB_SCALE_TRILINEAR: c_int = 2;
const TFB_SCALE_STEP: c_int = 1;
// OPT_PC / OPT_3DO (options.h)
const OPT_PC: c_int = 2;
const OPT_3DO: c_int = 1;

pub const DEFAULT_RESOLUTION_WIDTH: c_int = 1280;
pub const DEFAULT_RESOLUTION_HEIGHT: c_int = 960;
// ===========================================================================
//  Parsed options — accessible from C via FFI
// ===========================================================================

/// Run mode mirroring C's `runMode_normal/usage/version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum RunMode {
    Normal = 0,
    Usage = 1,
    Version = 2,
}

/// The full set of parsed options, equivalent to C's `options_struct`.
#[derive(Debug, Clone)]
pub struct ParsedOptions {
    pub log_file: Option<String>,
    pub run_mode: RunMode,
    pub config_dir: Option<String>,
    pub content_dir: Option<String>,
    pub addon_dir: Option<String>,
    pub addons: Vec<String>,
    pub graphics_backend: Option<String>,

    // Bool options
    pub opengl: bool,
    pub fullscreen: bool,
    pub scanlines: bool,
    pub show_fps: bool,
    pub keep_aspect_ratio: bool,
    pub use_3do_music: bool,
    pub use_remix_music: bool,
    pub use_speech: bool,
    pub subtitles: bool,
    pub stereo_sfx: bool,
    pub safe_mode: bool,

    // Int options
    pub scaler: c_int,
    pub sound_driver: c_int,
    pub sound_quality: c_int,
    pub which_coarse_scan: c_int,
    pub which_menu: c_int,
    pub which_fonts: c_int,
    pub which_intro: c_int,
    pub which_shield: c_int,
    pub smooth_scroll: c_int,
    pub melee_scale: c_int,

    // Float options
    pub gamma: f32,
    pub music_volume_scale: f32,
    pub sfx_volume_scale: f32,
    pub speech_volume_scale: f32,

    // Resolution
    pub resolution_width: c_int,
    pub resolution_height: c_int,

    /// Player control templates from config file (None = use C defaults).
    pub player1_control: Option<c_int>,
    pub player2_control: Option<c_int>,

    /// Non-zero if parsing failed.
    pub parse_error: bool,

    /// Automation options (--automation-script, --automation-output).
    /// None when not supplied. When both are Some, automation is active.
    pub automation_script: Option<String>,
    pub automation_output: Option<String>,
    pub native_window_proof: Option<String>,
}

impl Default for ParsedOptions {
    fn default() -> Self {
        Self {
            log_file: None,
            run_mode: RunMode::Normal,
            config_dir: None,
            content_dir: None,
            addon_dir: None,
            addons: Vec::new(),
            graphics_backend: None,
            opengl: false,
            fullscreen: false,
            scanlines: false,
            show_fps: false,
            keep_aspect_ratio: true,
            use_3do_music: true,
            use_remix_music: true,
            use_speech: true,
            subtitles: true,
            stereo_sfx: true,
            safe_mode: false,
            scaler: TFB_GFXFLAGS_SCALE_XBRZ3,
            sound_driver: AUDIO_DRIVER_MIXSDL,
            sound_quality: AUDIO_QUALITY_HIGH,
            which_coarse_scan: OPT_3DO,
            which_menu: OPT_PC,
            which_fonts: OPT_PC,
            which_intro: OPT_3DO,
            which_shield: OPT_3DO,
            smooth_scroll: OPT_3DO,
            melee_scale: TFB_SCALE_TRILINEAR,
            gamma: 1.0,
            music_volume_scale: 1.0,
            sfx_volume_scale: 1.0,
            speech_volume_scale: 1.0,
            resolution_width: DEFAULT_RESOLUTION_WIDTH,
            resolution_height: DEFAULT_RESOLUTION_HEIGHT,
            player1_control: None,
            player2_control: None,
            parse_error: false,
            automation_script: None,
            automation_output: None,
            native_window_proof: None,
        }
    }
}

// ===========================================================================
//  Global storage — read by C bridge `rust_get_parsed_options_*`
// ===========================================================================

static PARSED: OnceLock<ParsedOptions> = OnceLock::new();

/// Return a reference to the globally-stored parsed options.
///
/// # Panics
/// Panics if [`parse_and_store`] has not been called yet.
pub fn parsed() -> &'static ParsedOptions {
    PARSED
        .get()
        .expect("parse_and_store() must be called first")
}

// ===========================================================================
//  Clap command definition
// ===========================================================================

fn build_command() -> Command {
    let command = Command::new("uqm")
        .about("The Ur-Quan Masters")
        .disable_help_flag(true)
        .disable_version_flag(true)
        // Short options that mirror C's optString
        .arg(
            Arg::new("res")
                .short('r')
                .long("res")
                .value_name("WIDTHxHEIGHT")
                .help("Resolution (default 1280x960)"),
        )
        .arg(
            Arg::new("fullscreen")
                .short('f')
                .long("fullscreen")
                .action(ArgAction::SetTrue)
                .help("Fullscreen mode"),
        )
        .arg(
            Arg::new("windowed")
                .short('w')
                .long("windowed")
                .action(ArgAction::SetTrue)
                .help("Windowed mode (default)"),
        )
        .arg(
            Arg::new("opengl")
                .short('o')
                .long("opengl")
                .action(ArgAction::SetTrue)
                .help("Use OpenGL renderer"),
        )
        .arg(
            Arg::new("nogl")
                .short('x')
                .long("nogl")
                .action(ArgAction::SetTrue)
                .help("Use pure SDL2 renderer (default)"),
        )
        .arg(
            Arg::new("scale")
                .short('c')
                .long("scale")
                .value_name("MODE")
                .help("Scaler: hq, xbrz3, xbrz4, or none (default xbrz3)"),
        )
        .arg(
            Arg::new("meleezoom")
                .short('b')
                .long("meleezoom")
                .value_name("MODE")
                .help("Melee zoom: step (pc) or smooth (3do)"),
        )
        .arg(
            Arg::new("scanlines")
                .short('s')
                .long("scanlines")
                .action(ArgAction::SetTrue)
                .help("Enable scanlines"),
        )
        .arg(
            Arg::new("fps")
                .short('p')
                .long("fps")
                .action(ArgAction::SetTrue)
                .help("Show FPS counter"),
        )
        .arg(
            Arg::new("configdir")
                .short('C')
                .long("configdir")
                .value_name("DIR"),
        )
        .arg(
            Arg::new("contentdir")
                .short('n')
                .long("contentdir")
                .value_name("DIR"),
        )
        .arg(
            Arg::new("musicvol")
                .short('M')
                .long("musicvol")
                .value_name("0-100"),
        )
        .arg(
            Arg::new("sfxvol")
                .short('S')
                .long("sfxvol")
                .value_name("0-100"),
        )
        .arg(
            Arg::new("speechvol")
                .short('T')
                .long("speechvol")
                .value_name("0-100"),
        )
        .arg(
            Arg::new("audioquality")
                .short('q')
                .long("audioquality")
                .value_name("QUALITY")
                .help("high, medium, or low"),
        )
        .arg(
            Arg::new("nosubtitles")
                .short('u')
                .long("nosubtitles")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("gamma")
                .short('g')
                .long("gamma")
                .value_name("VALUE"),
        )
        .arg(
            Arg::new("logfile")
                .short('l')
                .long("logfile")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("intro")
                .short('i')
                .long("intro")
                .value_name("VERSION")
                .help("3do or pc"),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("keepaspectratio")
                .short('k')
                .long("keepaspectratio")
                .action(ArgAction::SetTrue)
                .help("Keep aspect ratio (default on)"),
        )
        // Long-only options (no short equivalent in C)
        .arg(
            Arg::new("cscan")
                .long("cscan")
                .value_name("VERSION")
                .help("3do or pc"),
        )
        .arg(
            Arg::new("menu")
                .long("menu")
                .value_name("TYPE")
                .help("3do or pc"),
        )
        .arg(
            Arg::new("font")
                .long("font")
                .value_name("TYPE")
                .help("3do or pc"),
        )
        .arg(
            Arg::new("shield")
                .long("shield")
                .value_name("TYPE")
                .help("3do or pc"),
        )
        .arg(
            Arg::new("scroll")
                .long("scroll")
                .value_name("TYPE")
                .help("3do or pc"),
        )
        .arg(
            Arg::new("sound")
                .long("sound")
                .value_name("DRIVER")
                .help("openal, mixsdl, or none"),
        )
        .arg(
            Arg::new("stereosfx")
                .long("stereosfx")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("addon")
                .long("addon")
                .value_name("ADDON")
                .action(ArgAction::Append),
        )
        .arg(Arg::new("addondir").long("addondir").value_name("DIR"))
        .arg(
            Arg::new("safe")
                .long("safe")
                .action(ArgAction::SetTrue)
                .help("Start in safe mode"),
        )
        .arg(Arg::new("renderer").long("renderer").value_name("NAME"))
        // Automation options
        .arg(
            Arg::new("automation-script")
                .long("automation-script")
                .value_name("FILE")
                .help("Path to automation script JSON"),
        )
        .arg(
            Arg::new("automation-output")
                .long("automation-output")
                .value_name("DIR")
                .help("Path to automation output directory"),
        );
    #[cfg(feature = "debug-process")]
    {
        command.arg(
            Arg::new("native-window-proof")
                .long("native-window-proof")
                .value_name("FILE")
                .help("Path to proof-only native-window configuration"),
        )
    }
    #[cfg(not(feature = "debug-process"))]
    command
}

// ===========================================================================
//  Parsing logic
// ===========================================================================

/// Parse command-line arguments and store the result globally.
///
/// Returns the parsed [`ParsedOptions`]. Also stores them for C FFI access.
///
/// On macOS, filters out the `-psn_XXXX` argument that Finder injects.
/// If no content directory is specified, searches common locations.
///
/// Priority: CLI args > config file (`uqm.cfg`) > compiled defaults.
pub fn parse_and_store(args: &[String]) -> ParsedOptions {
    // Start with config file values (lowest explicit priority)
    let mut opts = load_config_defaults();

    // Override with CLI args (highest priority)
    parse_args_into(args, &mut opts);

    if opts.content_dir.is_none() {
        opts.content_dir = discover_content_dir();
    }
    let _ = PARSED.set(opts.clone());
    opts
}

/// Load config file values from `{config_dir}/uqm.cfg`.
///
/// The config file format is `key = TYPE:value` where TYPE is
/// BOOLEAN, INT32, or STRING. These values provide defaults that
/// CLI args can override.
fn load_config_defaults() -> ParsedOptions {
    let mut opts = ParsedOptions::default();

    // Determine config dir (same logic as prepareConfigDir)
    let config_dir = std::env::var("UQM_CONFIG_DIR")
        .or_else(|_| std::env::var("HOME").map(|h| format!("{}/.uqm", h.trim_end_matches('/'))))
        .unwrap_or_else(|_| "~/.uqm".to_string());
    let cfg_path = format!("{}/uqm.cfg", config_dir.trim_end_matches('/'));

    let content = match std::fs::read_to_string(&cfg_path) {
        Ok(c) => c,
        Err(_) => return opts, // no config file is fine
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            apply_config_value(&mut opts, key.trim(), val.trim());
        }
    }
    tracing::debug!("Loaded config from {}", cfg_path);
    opts
}

/// Apply a single config file key=value pair to ParsedOptions.
fn apply_config_value(opts: &mut ParsedOptions, key: &str, raw_val: &str) {
    // raw_val is "TYPE:value" e.g. "BOOLEAN:true", "INT32:960", "STRING:mixsdl"
    let Some((vtype, vstr)) = raw_val.split_once(':') else {
        return;
    };

    match vtype {
        "BOOLEAN" => apply_boolean_config(opts, key, vstr),
        "INT32" => apply_int32_config(opts, key, vstr),
        "STRING" => apply_string_config(opts, key, vstr),
        _ => {}
    }
}

fn apply_boolean_config(opts: &mut ParsedOptions, key: &str, vstr: &str) {
    let enabled = vstr == "true";
    match key {
        // alwaysgl only applies when true
        "alwaysgl" if enabled => opts.opengl = true,
        "usegl" => opts.opengl = enabled,
        "fullscreen" => opts.fullscreen = enabled,
        "scanlines" => opts.scanlines = enabled,
        "showfps" => opts.show_fps = enabled,
        "keepaspectratio" => opts.keep_aspect_ratio = enabled,
        "subtitles" => opts.subtitles = enabled,
        "textmenu" => opts.which_menu = if enabled { OPT_PC } else { OPT_3DO },
        "textgradients" => opts.which_fonts = if enabled { OPT_PC } else { OPT_3DO },
        "iconicscan" => opts.which_coarse_scan = if enabled { OPT_3DO } else { OPT_PC },
        "smoothscroll" => opts.smooth_scroll = if enabled { OPT_3DO } else { OPT_PC },
        "pulseshield" => opts.which_shield = if enabled { OPT_3DO } else { OPT_PC },
        "3domovies" => opts.which_intro = if enabled { OPT_3DO } else { OPT_PC },
        "3domusic" => opts.use_3do_music = enabled,
        "remixmusic" => opts.use_remix_music = enabled,
        "speech" => opts.use_speech = enabled,
        "smoothmelee" => {
            opts.melee_scale = if enabled {
                TFB_SCALE_TRILINEAR
            } else {
                TFB_SCALE_STEP
            }
        }
        "positionalsfx" => opts.stereo_sfx = enabled,
        _ => {} // unknown key, skip
    }
}

fn apply_int32_config(opts: &mut ParsedOptions, key: &str, vstr: &str) {
    let Ok(value) = vstr.parse::<i32>() else {
        return;
    };
    match key {
        "reswidth" => opts.resolution_width = value,
        "resheight" => opts.resolution_height = value,
        "gamma" => opts.gamma = value as f32 / 1000.0,
        "musicvol" => opts.music_volume_scale = value as f32 / 100.0,
        "sfxvol" => opts.sfx_volume_scale = value as f32 / 100.0,
        "speechvol" => opts.speech_volume_scale = value as f32 / 100.0,
        "player1control" => opts.player1_control = Some(value),
        "player2control" => opts.player2_control = Some(value),
        _ => {} // unknown key, skip
    }
}

fn apply_string_config(opts: &mut ParsedOptions, key: &str, vstr: &str) {
    match key {
        "scaler" => {
            opts.scaler = match vstr {
                "hq" => TFB_GFXFLAGS_SCALE_HQXX,
                "xbrz3" => TFB_GFXFLAGS_SCALE_XBRZ3,
                "xbrz4" => TFB_GFXFLAGS_SCALE_XBRZ4,
                "none" => 0,
                _ => opts.scaler,
            };
        }
        "audiodriver" => {
            opts.sound_driver = match vstr {
                "mixsdl" => AUDIO_DRIVER_MIXSDL,
                "none" => AUDIO_DRIVER_NOSOUND,
                _ => opts.sound_driver,
            };
        }
        "audioquality" => {
            opts.sound_quality = match vstr {
                "low" => AUDIO_QUALITY_LOW,
                "medium" => AUDIO_QUALITY_MEDIUM,
                "high" => AUDIO_QUALITY_HIGH,
                _ => opts.sound_quality,
            };
        }
        _ => {} // unknown key, skip
    }
}

fn parse_args_into(args: &[String], opts: &mut ParsedOptions) {
    // macOS: Finder injects -psn_XXXXXX on double-click launch.
    if args.len() >= 2 && args[1].starts_with("-psn_") {
        return;
    }

    let cmd = build_command();
    let matches = match cmd.try_get_matches_from(args) {
        Ok(m) => m,
        Err(_) => {
            opts.parse_error = true;
            return;
        }
    };

    if matches.get_flag("help") {
        opts.run_mode = RunMode::Usage;
        return;
    }
    if matches.get_flag("version") {
        opts.run_mode = RunMode::Version;
        return;
    }

    apply_display_arguments(&matches, opts);
    apply_path_arguments(&matches, opts);
    if !apply_enum_arguments(&matches, opts) {
        return;
    }
    apply_choice_arguments(&matches, opts);
    apply_scalar_arguments(&matches, opts);
}

fn apply_display_arguments(matches: &clap::ArgMatches, opts: &mut ParsedOptions) {
    // Resolution: WIDTHxHEIGHT
    if let Some(res) = matches.get_one::<String>("res") {
        if let Some((w, h)) = parse_resolution(res) {
            opts.resolution_width = w;
            opts.resolution_height = h;
        }
    }

    // Bool flags
    if matches.get_flag("fullscreen") {
        opts.fullscreen = true;
    }
    if matches.get_flag("windowed") {
        opts.fullscreen = false;
    }
    if matches.get_flag("opengl") {
        opts.opengl = true;
    }
    if matches.get_flag("nogl") {
        opts.opengl = false;
    }
    if matches.get_flag("scanlines") {
        opts.scanlines = true;
    }
    if matches.get_flag("fps") {
        opts.show_fps = true;
    }
    if matches.get_flag("keepaspectratio") {
        opts.keep_aspect_ratio = true;
    }
    if matches.get_flag("nosubtitles") {
        opts.subtitles = false;
    }
    if matches.get_flag("stereosfx") {
        opts.stereo_sfx = true;
    }
    if matches.get_flag("safe") {
        opts.safe_mode = true;
    }
}

fn apply_path_arguments(matches: &clap::ArgMatches, opts: &mut ParsedOptions) {
    // String options
    if let Some(v) = matches.get_one::<String>("configdir") {
        opts.config_dir = Some(v.clone());
    }
    if let Some(v) = matches.get_one::<String>("contentdir") {
        opts.content_dir = Some(v.clone());
    }
    if let Some(v) = matches.get_one::<String>("addondir") {
        opts.addon_dir = Some(v.clone());
    }
    if let Some(v) = matches.get_one::<String>("logfile") {
        opts.log_file = Some(v.clone());
    }
    if let Some(v) = matches.get_one::<String>("renderer") {
        opts.graphics_backend = Some(v.clone());
    }

    // Automation options
    if let Some(v) = matches.get_one::<String>("automation-script") {
        opts.automation_script = Some(v.clone());
    }
    if let Some(v) = matches.get_one::<String>("automation-output") {
        opts.automation_output = Some(v.clone());
    }
    #[cfg(feature = "debug-process")]
    if let Some(v) = matches.get_one::<String>("native-window-proof") {
        opts.native_window_proof = Some(v.clone());
    }

    // Addons (can be specified multiple times)
    if let Some(addons) = matches.get_many::<String>("addon") {
        opts.addons = addons.cloned().collect();
    }
}

/// Apply enumerated selections. Returns false when a value is unrecognized,
/// which stops argument parsing exactly as the combined parser did.
fn apply_enum_arguments(matches: &clap::ArgMatches, opts: &mut ParsedOptions) -> bool {
    // List/enum options
    if let Some(v) = matches.get_one::<String>("scale") {
        opts.scaler = match v.as_str() {
            "xbrz3" => TFB_GFXFLAGS_SCALE_XBRZ3,
            "xbrz4" => TFB_GFXFLAGS_SCALE_XBRZ4,
            "hq" => TFB_GFXFLAGS_SCALE_HQXX,
            "none" | "no" => 0,
            _ => {
                opts.parse_error = true;
                return false;
            }
        };
    }
    if let Some(v) = matches.get_one::<String>("meleezoom") {
        opts.melee_scale = match v.as_str() {
            "smooth" | "3do" => TFB_SCALE_TRILINEAR,
            "step" | "pc" => TFB_SCALE_STEP,
            "bilinear" => 0,
            _ => {
                opts.parse_error = true;
                return false;
            }
        };
    }
    if let Some(v) = matches.get_one::<String>("audioquality") {
        opts.sound_quality = match v.as_str() {
            "low" => AUDIO_QUALITY_LOW,
            "medium" => AUDIO_QUALITY_MEDIUM,
            "high" => AUDIO_QUALITY_HIGH,
            _ => {
                opts.parse_error = true;
                return false;
            }
        };
    }
    if let Some(v) = matches.get_one::<String>("sound") {
        opts.sound_driver = match v.as_str() {
            "mixsdl" => AUDIO_DRIVER_MIXSDL,
            "none" | "nosound" => AUDIO_DRIVER_NOSOUND,
            _ => {
                opts.parse_error = true;
                return false;
            }
        };
    }
    true
}

fn apply_choice_arguments(matches: &clap::ArgMatches, opts: &mut ParsedOptions) {
    // 3do/pc choice options
    if let Some(v) = matches.get_one::<String>("intro") {
        opts.which_intro = parse_choice(v, opts);
    }
    if let Some(v) = matches.get_one::<String>("cscan") {
        opts.which_coarse_scan = parse_choice(v, opts);
    }
    if let Some(v) = matches.get_one::<String>("menu") {
        opts.which_menu = parse_choice(v, opts);
    }
    if let Some(v) = matches.get_one::<String>("font") {
        opts.which_fonts = parse_choice(v, opts);
    }
    if let Some(v) = matches.get_one::<String>("shield") {
        opts.which_shield = parse_choice(v, opts);
    }
    if let Some(v) = matches.get_one::<String>("scroll") {
        opts.smooth_scroll = parse_choice(v, opts);
    }
}

fn apply_scalar_arguments(matches: &clap::ArgMatches, opts: &mut ParsedOptions) {
    // Volume options (0-100 → 0.0-1.0 scale)
    if let Some(v) = matches.get_one::<String>("musicvol") {
        opts.music_volume_scale = parse_volume(v).unwrap_or_else(|| {
            opts.parse_error = true;
            1.0
        });
    }
    if let Some(v) = matches.get_one::<String>("sfxvol") {
        opts.sfx_volume_scale = parse_volume(v).unwrap_or_else(|| {
            opts.parse_error = true;
            1.0
        });
    }
    if let Some(v) = matches.get_one::<String>("speechvol") {
        opts.speech_volume_scale = parse_volume(v).unwrap_or_else(|| {
            opts.parse_error = true;
            1.0
        });
    }

    // Gamma
    if let Some(v) = matches.get_one::<String>("gamma") {
        opts.gamma = v.parse().unwrap_or_else(|_| {
            opts.parse_error = true;
            1.0
        });
    }
}

fn parse_resolution(s: &str) -> Option<(c_int, c_int)> {
    let (w, h) = s.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn parse_volume(s: &str) -> Option<f32> {
    let v: i32 = s.parse().ok()?;
    if !(0..=100).contains(&v) {
        return None;
    }
    Some(v as f32 / 100.0)
}

/// Parse a "3do"/"pc" choice value, setting parse_error on invalid input.
fn parse_choice(v: &str, opts: &mut ParsedOptions) -> c_int {
    match v {
        "3do" => OPT_3DO,
        "pc" => OPT_PC,
        _ => {
            opts.parse_error = true;
            OPT_PC
        }
    }
}

/// Search for the UQM content directory by looking for a `version` file
/// inside candidate directories. Tries:
/// 1. Current working directory
/// 2. `content` relative to cwd
/// 3. Walk up from the executable's location looking for `content/version`
/// 4. `../../content` (for running from target/debug or target/release)
fn discover_content_dir() -> Option<String> {
    let candidates = [
        // CWD
        ".".to_string(),
        "content".to_string(),
        "../../content".to_string(),
        "../../../content".to_string(),
    ];

    for c in &candidates {
        if std::path::Path::new(c).join("version").exists() {
            return Some(if c == "." {
                std::env::current_dir().ok()?.to_string_lossy().to_string()
            } else {
                std::fs::canonicalize(c)
                    .unwrap_or_else(|_| std::path::PathBuf::from(c))
                    .to_string_lossy()
                    .to_string()
            });
        }
    }

    // Walk up from the executable's directory
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent()?.to_path_buf();
        for _ in 0..6 {
            let content = dir.join("content");
            if content.join("version").exists() {
                return Some(content.to_string_lossy().to_string());
            }
            if !dir.pop() {
                break;
            }
        }
    }

    None
}

// ===========================================================================
//  FFI exports — called by C bridge to read parsed values
// ===========================================================================

/// C-callable accessor for the parsed options. Returns a semicolon-separated
/// string that the C bridge unpacks into the `options_struct`.
///
/// Format: `log_file\0run_mode\0config_dir\0content_dir\0...`
/// Each field is separated by NUL. Empty values are represented as empty strings.
#[no_mangle]
pub extern "C" fn rust_options_get_run_mode() -> c_int {
    parsed().run_mode as c_int
}

#[no_mangle]
pub extern "C" fn rust_options_parse_error() -> c_int {
    if parsed().parse_error {
        1
    } else {
        0
    }
}

// --- String getters (return owned C string, caller must free) ---

/// Returns a heap-allocated C string. Caller MUST call `rust_options_free_string`.
fn to_c_string(s: &Option<String>) -> *mut c_char {
    let val = s.as_deref().unwrap_or("");
    CString::new(val).unwrap_or_default().into_raw()
}

#[no_mangle]
pub extern "C" fn rust_options_get_log_file() -> *mut c_char {
    to_c_string(&parsed().log_file)
}

#[no_mangle]
pub extern "C" fn rust_options_get_config_dir() -> *mut c_char {
    to_c_string(&parsed().config_dir)
}

#[no_mangle]
pub extern "C" fn rust_options_get_content_dir() -> *mut c_char {
    to_c_string(&parsed().content_dir)
}

#[no_mangle]
pub extern "C" fn rust_options_get_addon_dir() -> *mut c_char {
    to_c_string(&parsed().addon_dir)
}

#[no_mangle]
pub extern "C" fn rust_options_get_graphics_backend() -> *mut c_char {
    to_c_string(&parsed().graphics_backend)
}

#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn rust_options_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

#[no_mangle]
pub extern "C" fn rust_options_get_addon_count() -> c_int {
    parsed().addons.len() as c_int
}

#[no_mangle]
pub extern "C" fn rust_options_get_addon(index: c_int) -> *mut c_char {
    let addons = &parsed().addons;
    if index < 0 || index as usize >= addons.len() {
        return std::ptr::null_mut();
    }
    CString::new(addons[index as usize].as_str())
        .unwrap_or_default()
        .into_raw()
}

// --- Bool getters ---

macro_rules! bool_getter {
    ($name:ident, $field:ident) => {
        #[no_mangle]
        pub extern "C" fn $name() -> c_int {
            if parsed().$field {
                1
            } else {
                0
            }
        }
    };
}

bool_getter!(rust_options_opengl, opengl);
bool_getter!(rust_options_fullscreen, fullscreen);
bool_getter!(rust_options_scanlines, scanlines);
bool_getter!(rust_options_show_fps, show_fps);
bool_getter!(rust_options_keep_aspect_ratio, keep_aspect_ratio);
bool_getter!(rust_options_use_3do_music, use_3do_music);
bool_getter!(rust_options_use_remix_music, use_remix_music);
bool_getter!(rust_options_use_speech, use_speech);
bool_getter!(rust_options_subtitles, subtitles);
bool_getter!(rust_options_stereo_sfx, stereo_sfx);
bool_getter!(rust_options_safe_mode, safe_mode);

// --- Int getters ---

macro_rules! int_getter {
    ($name:ident, $field:ident) => {
        #[no_mangle]
        pub extern "C" fn $name() -> c_int {
            parsed().$field
        }
    };
}

int_getter!(rust_options_scaler, scaler);
int_getter!(rust_options_sound_driver, sound_driver);
int_getter!(rust_options_sound_quality, sound_quality);
int_getter!(rust_options_which_coarse_scan, which_coarse_scan);
int_getter!(rust_options_which_menu, which_menu);
int_getter!(rust_options_which_fonts, which_fonts);
int_getter!(rust_options_which_intro, which_intro);
int_getter!(rust_options_which_shield, which_shield);
int_getter!(rust_options_smooth_scroll, smooth_scroll);
int_getter!(rust_options_melee_scale, melee_scale);
int_getter!(rust_options_resolution_width, resolution_width);
int_getter!(rust_options_resolution_height, resolution_height);

/// Player control overrides from config file. Returns -1 if not set.
#[no_mangle]
pub extern "C" fn rust_options_player1_control() -> c_int {
    parsed().player1_control.unwrap_or(-1)
}

/// Player control overrides from config file. Returns -1 if not set.
#[no_mangle]
pub extern "C" fn rust_options_player2_control() -> c_int {
    parsed().player2_control.unwrap_or(-1)
}

// --- Float getters ---

macro_rules! float_getter {
    ($name:ident, $field:ident) => {
        #[no_mangle]
        pub extern "C" fn $name() -> f32 {
            parsed().$field
        }
    };
}

float_getter!(rust_options_gamma, gamma);
float_getter!(rust_options_music_volume_scale, music_volume_scale);
float_getter!(rust_options_sfx_volume_scale, sfx_volume_scale);
float_getter!(rust_options_speech_volume_scale, speech_volume_scale);

// ===========================================================================
//  Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> ParsedOptions {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        // Bypass config file loading for unit tests
        let mut opts = ParsedOptions::default();
        parse_args_into(&owned, &mut opts);
        opts
    }

    #[test]
    fn test_defaults() {
        let opts = parse(&["uqm"]);
        assert_eq!(opts.run_mode, RunMode::Normal);
        assert!(!opts.opengl);
        assert!(!opts.fullscreen);
        assert!(opts.keep_aspect_ratio);
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_XBRZ3);
        assert_eq!(opts.sound_driver, AUDIO_DRIVER_MIXSDL);
        assert_eq!(DEFAULT_RESOLUTION_WIDTH, 1280);
        assert_eq!(DEFAULT_RESOLUTION_HEIGHT, 960);
        assert_eq!(opts.resolution_width, DEFAULT_RESOLUTION_WIDTH);
        assert_eq!(opts.resolution_height, DEFAULT_RESOLUTION_HEIGHT);
    }

    #[test]
    fn test_help() {
        let opts = parse(&["uqm", "-h"]);
        assert_eq!(opts.run_mode, RunMode::Usage);
    }

    #[test]
    fn test_version() {
        let opts = parse(&["uqm", "-v"]);
        assert_eq!(opts.run_mode, RunMode::Version);
    }

    #[test]
    fn test_resolution() {
        let opts = parse(&["uqm", "-r", "1920x1080"]);
        assert_eq!(opts.resolution_width, 1920);
        assert_eq!(opts.resolution_height, 1080);
    }

    #[test]
    fn test_resolution_long() {
        let opts = parse(&["uqm", "--res=640x480"]);
        assert_eq!(opts.resolution_width, 640);
        assert_eq!(opts.resolution_height, 480);
    }

    #[test]
    fn test_opengl_short() {
        let opts = parse(&["uqm", "-o"]);
        assert!(opts.opengl);
    }

    #[test]
    fn test_opengl_long() {
        let opts = parse(&["uqm", "--opengl"]);
        assert!(opts.opengl);
    }

    #[test]
    fn test_nogl() {
        let opts = parse(&["uqm", "-x"]);
        assert!(!opts.opengl);
    }

    #[test]
    fn test_fullscreen_windowed() {
        let opts = parse(&["uqm", "-f"]);
        assert!(opts.fullscreen);
        let opts = parse(&["uqm", "-w"]);
        assert!(!opts.fullscreen);
    }

    #[test]
    fn test_scale_xbrz4() {
        let opts = parse(&["uqm", "-c", "xbrz4"]);
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_XBRZ4);
    }

    #[test]
    fn test_scale_hq() {
        let opts = parse(&["uqm", "-c", "hq"]);
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_HQXX);
    }

    #[test]
    fn test_scale_none() {
        let opts = parse(&["uqm", "-c", "none"]);
        assert_eq!(opts.scaler, 0);
    }

    #[test]
    fn test_melee_zoom() {
        let opts = parse(&["uqm", "-b", "step"]);
        assert_eq!(opts.melee_scale, TFB_SCALE_STEP);
        let opts = parse(&["uqm", "-b", "smooth"]);
        assert_eq!(opts.melee_scale, TFB_SCALE_TRILINEAR);
    }

    #[test]
    fn test_volumes() {
        let opts = parse(&["uqm", "-M", "50", "-S", "75", "-T", "100"]);
        assert!((opts.music_volume_scale - 0.5).abs() < 0.001);
        assert!((opts.sfx_volume_scale - 0.75).abs() < 0.001);
        assert!((opts.speech_volume_scale - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_audio_quality() {
        let opts = parse(&["uqm", "-q", "low"]);
        assert_eq!(opts.sound_quality, AUDIO_QUALITY_LOW);
    }

    #[test]
    fn test_sound_driver() {
        let opts = parse(&["uqm", "--sound", "none"]);
        assert_eq!(opts.sound_driver, AUDIO_DRIVER_NOSOUND);
    }

    #[test]
    fn test_choice_3do_pc() {
        let opts = parse(&["uqm", "-i", "3do"]);
        assert_eq!(opts.which_intro, OPT_3DO);
        let opts = parse(&["uqm", "--menu", "pc"]);
        assert_eq!(opts.which_menu, OPT_PC);
    }

    #[test]
    fn test_content_dir() {
        let opts = parse(&["uqm", "-n", "/path/to/content"]);
        assert_eq!(opts.content_dir.as_deref(), Some("/path/to/content"));
    }

    #[test]
    fn test_multiple_addons() {
        let opts = parse(&["uqm", "--addon", "mod1", "--addon", "mod2"]);
        assert_eq!(opts.addons.len(), 2);
        assert_eq!(opts.addons[0], "mod1");
        assert_eq!(opts.addons[1], "mod2");
    }

    #[test]
    fn test_safe_mode() {
        let opts = parse(&["uqm", "--safe"]);
        assert!(opts.safe_mode);
    }

    #[test]
    fn test_gamma() {
        let opts = parse(&["uqm", "-g", "1.5"]);
        assert!((opts.gamma - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_logfile() {
        let opts = parse(&["uqm", "-l", "debug.log"]);
        assert_eq!(opts.log_file.as_deref(), Some("debug.log"));
    }

    #[test]
    fn test_macos_psn_filter() {
        let mut opts = ParsedOptions::default();
        parse_args_into(&["uqm".to_string(), "-psn_0_123456".to_string()], &mut opts);
        assert_eq!(opts.run_mode, RunMode::Normal);
        assert!(!opts.parse_error);
    }

    #[test]
    fn test_nosubtitles() {
        let opts = parse(&["uqm", "-u"]);
        assert!(!opts.subtitles);
    }

    #[test]
    fn test_stereosfx() {
        let opts = parse(&["uqm", "--stereosfx"]);
        assert!(opts.stereo_sfx);
    }

    #[test]
    fn test_invalid_scale() {
        let opts = parse(&["uqm", "-c", "bogus"]);
        assert!(opts.parse_error);
    }

    #[test]
    fn test_discover_content_from_cwd() {
        // When run from the project root, content/version should exist
        let discovered = discover_content_dir();
        // In CI/dev this should find it; just verify it doesn't panic
        if let Some(ref path) = discovered {
            assert!(
                std::path::Path::new(path).join("version").exists(),
                "discovered content dir {path} should contain version file"
            );
        }
    }

    #[test]
    fn test_config_file_parsing() {
        let mut opts = ParsedOptions::default();
        // Verify config key parsing matches the real uqm.cfg format
        apply_config_value(&mut opts, "reswidth", "INT32:1920");
        assert_eq!(opts.resolution_width, 1920);
        apply_config_value(&mut opts, "resheight", "INT32:1200");
        assert_eq!(opts.resolution_height, 1200);
        apply_config_value(&mut opts, "fullscreen", "BOOLEAN:true");
        assert!(opts.fullscreen);
        apply_config_value(&mut opts, "usegl", "BOOLEAN:true");
        assert!(opts.opengl);
        apply_config_value(&mut opts, "scaler", "STRING:xbrz4");
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_XBRZ4);
        apply_config_value(&mut opts, "gamma", "INT32:1500");
        assert!((opts.gamma - 1.5).abs() < 0.01);
        apply_config_value(&mut opts, "musicvol", "INT32:50");
        assert!((opts.music_volume_scale - 0.5).abs() < 0.01);
        apply_config_value(&mut opts, "textmenu", "BOOLEAN:false");
        assert_eq!(opts.which_menu, OPT_3DO);
        apply_config_value(&mut opts, "smoothmelee", "BOOLEAN:true");
        assert_eq!(opts.melee_scale, TFB_SCALE_TRILINEAR);
    }

    #[cfg(feature = "debug-process")]
    #[test]
    fn native_window_proof_option_is_parsed() {
        let opts = parse(&[
            "uqm",
            "--automation-script=/tmp/script.json",
            "--native-window-proof=/tmp/native-proof.json",
        ]);
        assert_eq!(opts.automation_script.as_deref(), Some("/tmp/script.json"));
        assert_eq!(
            opts.native_window_proof.as_deref(),
            Some("/tmp/native-proof.json")
        );
    }

    #[test]
    fn test_config_file_priority() {
        // Config values should NOT override CLI values when using parse_and_store.
        // Simulate: config says xbrz4, CLI says hq → hq wins.
        let mut opts = ParsedOptions::default();
        apply_config_value(&mut opts, "scaler", "STRING:xbrz4");
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_XBRZ4);
        // Now CLI override
        parse_args_into(
            &["uqm".to_string(), "-c".to_string(), "hq".to_string()],
            &mut opts,
        );
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_HQXX);
    }
}

#[cfg(test)]
mod config_file_tests {
    use super::*;
    use serial_test::serial;

    fn applied(lines: &[(&str, &str)]) -> ParsedOptions {
        let mut opts = ParsedOptions::default();
        for (key, raw) in lines {
            apply_config_value(&mut opts, key, raw);
        }
        opts
    }

    #[test]
    fn boolean_settings_switch_the_matching_option() {
        let opts = applied(&[
            ("usegl", "BOOLEAN:true"),
            ("fullscreen", "BOOLEAN:true"),
            ("scanlines", "BOOLEAN:true"),
            ("showfps", "BOOLEAN:true"),
            ("keepaspectratio", "BOOLEAN:false"),
            ("subtitles", "BOOLEAN:false"),
            ("3domusic", "BOOLEAN:false"),
            ("remixmusic", "BOOLEAN:false"),
            ("speech", "BOOLEAN:false"),
            ("positionalsfx", "BOOLEAN:false"),
        ]);

        assert!(opts.opengl && opts.fullscreen && opts.scanlines && opts.show_fps);
        assert!(!opts.keep_aspect_ratio && !opts.subtitles);
        assert!(!opts.use_3do_music && !opts.use_remix_music && !opts.use_speech);
        assert!(!opts.stereo_sfx);
    }

    #[test]
    fn the_menu_style_settings_choose_between_pc_and_3do() {
        let on = applied(&[
            ("textmenu", "BOOLEAN:true"),
            ("textgradients", "BOOLEAN:true"),
            ("iconicscan", "BOOLEAN:true"),
            ("smoothscroll", "BOOLEAN:true"),
            ("pulseshield", "BOOLEAN:true"),
            ("3domovies", "BOOLEAN:true"),
            ("smoothmelee", "BOOLEAN:true"),
        ]);
        assert_eq!((on.which_menu, on.which_fonts), (OPT_PC, OPT_PC));
        assert_eq!(on.which_coarse_scan, OPT_3DO);
        assert_eq!((on.smooth_scroll, on.which_shield), (OPT_3DO, OPT_3DO));
        assert_eq!(on.which_intro, OPT_3DO);
        assert_eq!(on.melee_scale, TFB_SCALE_TRILINEAR);

        let off = applied(&[
            ("textmenu", "BOOLEAN:false"),
            ("textgradients", "BOOLEAN:false"),
            ("iconicscan", "BOOLEAN:false"),
            ("smoothscroll", "BOOLEAN:false"),
            ("pulseshield", "BOOLEAN:false"),
            ("3domovies", "BOOLEAN:false"),
            ("smoothmelee", "BOOLEAN:false"),
        ]);
        assert_eq!((off.which_menu, off.which_fonts), (OPT_3DO, OPT_3DO));
        assert_eq!(off.which_coarse_scan, OPT_PC);
        assert_eq!((off.smooth_scroll, off.which_shield), (OPT_PC, OPT_PC));
        assert_eq!(off.which_intro, OPT_PC);
        assert_eq!(off.melee_scale, TFB_SCALE_STEP);
    }

    #[test]
    fn alwaysgl_can_only_turn_opengl_on() {
        let mut opts = ParsedOptions::default();
        apply_config_value(&mut opts, "alwaysgl", "BOOLEAN:false");
        assert!(!opts.opengl);
        apply_config_value(&mut opts, "alwaysgl", "BOOLEAN:true");
        assert!(opts.opengl);
        apply_config_value(&mut opts, "alwaysgl", "BOOLEAN:false");
        assert!(opts.opengl, "false never switches it back off");
        apply_config_value(&mut opts, "usegl", "BOOLEAN:false");
        assert!(!opts.opengl, "usegl is the setting that can");
    }

    #[test]
    fn integer_settings_are_stored_and_percentages_scaled() {
        let opts = applied(&[
            ("reswidth", "INT32:960"),
            ("resheight", "INT32:720"),
            ("gamma", "INT32:1250"),
            ("musicvol", "INT32:40"),
            ("sfxvol", "INT32:60"),
            ("speechvol", "INT32:80"),
            ("player1control", "INT32:3"),
            ("player2control", "INT32:5"),
        ]);

        assert_eq!((opts.resolution_width, opts.resolution_height), (960, 720));
        assert!((opts.gamma - 1.25).abs() < 1e-6);
        assert!((opts.music_volume_scale - 0.4).abs() < 1e-6);
        assert!((opts.sfx_volume_scale - 0.6).abs() < 1e-6);
        assert!((opts.speech_volume_scale - 0.8).abs() < 1e-6);
        assert_eq!(
            (opts.player1_control, opts.player2_control),
            (Some(3), Some(5))
        );
    }

    #[test]
    fn string_settings_choose_scaler_driver_and_quality() {
        let opts = applied(&[
            ("scaler", "STRING:xbrz4"),
            ("audiodriver", "STRING:none"),
            ("audioquality", "STRING:low"),
        ]);
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_XBRZ4);
        assert_eq!(opts.sound_driver, AUDIO_DRIVER_NOSOUND);
        assert_eq!(opts.sound_quality, AUDIO_QUALITY_LOW);

        for (value, expected) in [
            ("hq", TFB_GFXFLAGS_SCALE_HQXX),
            ("xbrz3", TFB_GFXFLAGS_SCALE_XBRZ3),
            ("none", 0),
        ] {
            let opts = applied(&[("scaler", &format!("STRING:{value}"))]);
            assert_eq!(opts.scaler, expected, "scaler {value}");
        }
        assert_eq!(
            applied(&[("audiodriver", "STRING:mixsdl")]).sound_driver,
            AUDIO_DRIVER_MIXSDL
        );
        assert_eq!(
            applied(&[("audioquality", "STRING:medium")]).sound_quality,
            AUDIO_QUALITY_MEDIUM
        );
        assert_eq!(
            applied(&[("audioquality", "STRING:high")]).sound_quality,
            AUDIO_QUALITY_HIGH
        );
    }

    #[test]
    fn unrecognised_string_values_keep_the_current_choice() {
        let opts = applied(&[
            ("scaler", "STRING:warp"),
            ("audiodriver", "STRING:openal"),
            ("audioquality", "STRING:ultra"),
        ]);
        let defaults = ParsedOptions::default();
        assert_eq!(opts.scaler, defaults.scaler);
        assert_eq!(opts.sound_driver, defaults.sound_driver);
        assert_eq!(opts.sound_quality, defaults.sound_quality);
    }

    #[test]
    fn malformed_or_unknown_entries_change_nothing() {
        let opts = applied(&[
            ("fullscreen", "true"),
            ("fullscreen", "FLOAT:true"),
            ("reswidth", "INT32:wide"),
            ("reswidth", "INT32:"),
            ("nosuchkey", "BOOLEAN:true"),
            ("nosuchkey", "INT32:7"),
            ("nosuchkey", "STRING:x"),
        ]);
        let defaults = ParsedOptions::default();
        assert_eq!(opts.fullscreen, defaults.fullscreen);
        assert_eq!(opts.resolution_width, defaults.resolution_width);
        assert_eq!(opts.player1_control, None);
    }

    struct ConfigDir {
        _dir: tempfile::TempDir,
        previous: Option<std::ffi::OsString>,
    }

    impl ConfigDir {
        fn with_file(contents: Option<&str>) -> Self {
            let dir = tempfile::TempDir::new().unwrap();
            if let Some(text) = contents {
                std::fs::write(dir.path().join("uqm.cfg"), text).unwrap();
            }
            let previous = std::env::var_os("UQM_CONFIG_DIR");
            std::env::set_var("UQM_CONFIG_DIR", dir.path());
            Self {
                _dir: dir,
                previous,
            }
        }
    }

    impl Drop for ConfigDir {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var("UQM_CONFIG_DIR", value),
                None => std::env::remove_var("UQM_CONFIG_DIR"),
            }
        }
    }

    #[test]
    #[serial]
    fn the_config_file_supplies_defaults_and_skips_comments_and_blanks() {
        let _config = ConfigDir::with_file(Some(
            "# written by the options menu\n\
             \n\
             fullscreen = BOOLEAN:true\n\
               reswidth = INT32:1024  \n\
             resheight=INT32:768\n\
             this line has no equals sign\n\
             scaler = STRING:hq\n",
        ));

        let opts = load_config_defaults();

        assert!(opts.fullscreen);
        assert_eq!((opts.resolution_width, opts.resolution_height), (1024, 768));
        assert_eq!(opts.scaler, TFB_GFXFLAGS_SCALE_HQXX);
        assert!(opts.subtitles, "unlisted settings keep their defaults");
    }

    #[test]
    #[serial]
    fn a_missing_config_file_leaves_the_defaults() {
        let _config = ConfigDir::with_file(None);

        let opts = load_config_defaults();

        assert!(!opts.fullscreen);
        assert_eq!(opts.resolution_width, DEFAULT_RESOLUTION_WIDTH);
        assert!(!opts.parse_error);
    }

    #[test]
    #[serial]
    fn command_line_arguments_override_the_config_file_and_reach_the_c_getters() {
        let _config = ConfigDir::with_file(Some(
            "fullscreen = BOOLEAN:true\n\
             reswidth = INT32:1024\n\
             resheight = INT32:768\n\
             player1control = INT32:4\n\
             gamma = INT32:1500\n",
        ));
        let args: Vec<String> = [
            "uqm",
            "--res=800x600",
            "--windowed",
            "--scanlines",
            "--fps",
            "--nosubtitles",
            "--safe",
            "--addon",
            "first",
            "--addon",
            "second",
            "--logfile",
            "run.log",
            "--configdir",
            "/cfg",
            "--addondir",
            "/addons",
            "--contentdir",
            "/content",
            "--renderer",
            "sdl2",
            "--scale=xbrz4",
            "--sound=nosound",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        let opts = parse_and_store(&args);

        assert!(!opts.fullscreen, "the command line beats the file");
        assert_eq!((opts.resolution_width, opts.resolution_height), (800, 600));
        assert_eq!(opts.player1_control, Some(4), "file-only settings survive");
        assert!((opts.gamma - 1.5).abs() < 1e-6);
        assert_eq!(rust_options_get_run_mode(), RunMode::Normal as c_int);
        assert_eq!(rust_options_parse_error(), 0);
        assert_eq!(rust_options_fullscreen(), 0);
        assert_eq!(rust_options_scanlines(), 1);
        assert_eq!(rust_options_show_fps(), 1);
        assert_eq!(rust_options_subtitles(), 0);
        assert_eq!(rust_options_safe_mode(), 1);
        assert_eq!(rust_options_opengl(), 0);
        assert_eq!(rust_options_keep_aspect_ratio(), 1);
        assert_eq!(rust_options_use_3do_music(), 1);
        assert_eq!(rust_options_use_remix_music(), 1);
        assert_eq!(rust_options_use_speech(), 1);
        assert_eq!(rust_options_stereo_sfx(), 1);
        assert_eq!(rust_options_resolution_width(), 800);
        assert_eq!(rust_options_resolution_height(), 600);
        assert_eq!(rust_options_scaler(), TFB_GFXFLAGS_SCALE_XBRZ4);
        assert_eq!(rust_options_sound_driver(), AUDIO_DRIVER_NOSOUND);
        assert_eq!(rust_options_sound_quality(), AUDIO_QUALITY_HIGH);
        assert_eq!(rust_options_which_coarse_scan(), OPT_3DO);
        assert_eq!(rust_options_which_menu(), OPT_PC);
        assert_eq!(rust_options_which_fonts(), OPT_PC);
        assert_eq!(rust_options_which_intro(), OPT_3DO);
        assert_eq!(rust_options_which_shield(), OPT_3DO);
        assert_eq!(rust_options_smooth_scroll(), OPT_3DO);
        assert_eq!(rust_options_melee_scale(), TFB_SCALE_TRILINEAR);
        assert_eq!(rust_options_player1_control(), 4);
        assert_eq!(rust_options_player2_control(), -1, "unset reads as -1");
        assert!((rust_options_gamma() - 1.5).abs() < 1e-6);
        assert_eq!(rust_options_music_volume_scale(), 1.0);
        assert_eq!(rust_options_sfx_volume_scale(), 1.0);
        assert_eq!(rust_options_speech_volume_scale(), 1.0);

        let text = |ptr: *mut c_char| {
            assert!(!ptr.is_null());
            let value = unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_str()
                .unwrap()
                .to_string();
            rust_options_free_string(ptr);
            value
        };
        assert_eq!(text(rust_options_get_log_file()), "run.log");
        assert_eq!(text(rust_options_get_config_dir()), "/cfg");
        assert_eq!(text(rust_options_get_addon_dir()), "/addons");
        assert_eq!(text(rust_options_get_content_dir()), "/content");
        assert_eq!(text(rust_options_get_graphics_backend()), "sdl2");

        assert_eq!(rust_options_get_addon_count(), 2);
        assert_eq!(text(rust_options_get_addon(0)), "first");
        assert_eq!(text(rust_options_get_addon(1)), "second");
        assert!(rust_options_get_addon(2).is_null());
        assert!(rust_options_get_addon(-1).is_null());
        rust_options_free_string(std::ptr::null_mut());
    }

    fn parse(args: &[&str]) -> ParsedOptions {
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut opts = ParsedOptions::default();
        parse_args_into(&owned, &mut opts);
        opts
    }

    #[test]
    fn unrecognised_enumerated_values_are_a_parse_error_that_stops_parsing() {
        for bad in [
            ["uqm", "--meleezoom", "huge"],
            ["uqm", "--audioquality", "ultra"],
            ["uqm", "--sound", "openal"],
            ["uqm", "--scale", "scale2x"],
        ] {
            let opts = parse(&[bad[0], bad[1], bad[2], "--musicvol", "10", "--intro", "pc"]);
            assert!(opts.parse_error, "{bad:?}");
            assert_eq!(
                opts.music_volume_scale, 1.0,
                "{bad:?}: later arguments are not applied"
            );
            assert_eq!(opts.which_intro, OPT_3DO, "{bad:?}");
        }
    }

    #[test]
    fn enumerated_aliases_map_to_their_values() {
        assert_eq!(parse(&["uqm", "--scale", "no"]).scaler, 0);
        assert_eq!(
            parse(&["uqm", "--meleezoom", "3do"]).melee_scale,
            TFB_SCALE_TRILINEAR
        );
        assert_eq!(
            parse(&["uqm", "--meleezoom", "pc"]).melee_scale,
            TFB_SCALE_STEP
        );
        assert_eq!(parse(&["uqm", "--meleezoom", "bilinear"]).melee_scale, 0);
        assert_eq!(
            parse(&["uqm", "--audioquality", "medium"]).sound_quality,
            AUDIO_QUALITY_MEDIUM
        );
        assert_eq!(
            parse(&["uqm", "--audioquality", "high"]).sound_quality,
            AUDIO_QUALITY_HIGH
        );
        assert_eq!(
            parse(&["uqm", "--sound", "mixsdl"]).sound_driver,
            AUDIO_DRIVER_MIXSDL
        );
        assert_eq!(
            parse(&["uqm", "--sound", "nosound"]).sound_driver,
            AUDIO_DRIVER_NOSOUND
        );
    }

    #[test]
    fn every_three_do_or_pc_choice_has_its_own_option() {
        let opts = parse(&[
            "uqm", "--intro", "pc", "--cscan", "pc", "--menu", "3do", "--font", "3do", "--shield",
            "pc", "--scroll", "pc",
        ]);

        assert_eq!(opts.which_intro, OPT_PC);
        assert_eq!(opts.which_coarse_scan, OPT_PC);
        assert_eq!(opts.which_menu, OPT_3DO);
        assert_eq!(opts.which_fonts, OPT_3DO);
        assert_eq!(opts.which_shield, OPT_PC);
        assert_eq!(opts.smooth_scroll, OPT_PC);
        assert!(!opts.parse_error);
    }

    #[test]
    fn a_bad_choice_is_flagged_and_falls_back_to_pc() {
        let opts = parse(&["uqm", "--menu", "amiga"]);

        assert!(opts.parse_error);
        assert_eq!(opts.which_menu, OPT_PC);
    }

    #[test]
    fn out_of_range_or_non_numeric_volumes_and_gamma_are_parse_errors() {
        for args in [
            ["uqm", "--musicvol", "101"],
            ["uqm", "--sfxvol", "-1"],
            ["uqm", "--speechvol", "loud"],
            ["uqm", "--gamma", "bright"],
        ] {
            let opts = parse(&args);
            assert!(opts.parse_error, "{args:?}");
        }
        let edge = parse(&["uqm", "--musicvol", "0", "--sfxvol", "100"]);
        assert_eq!(edge.music_volume_scale, 0.0);
        assert_eq!(edge.sfx_volume_scale, 1.0);
        assert!(!edge.parse_error);
    }

    #[test]
    fn malformed_resolutions_are_ignored() {
        for res in ["640", "axb", "640x", "x480", "640x480x2"] {
            let opts = parse(&["uqm", "--res", res]);
            assert_eq!(
                (opts.resolution_width, opts.resolution_height),
                (DEFAULT_RESOLUTION_WIDTH, DEFAULT_RESOLUTION_HEIGHT),
                "{res}"
            );
        }
    }

    #[test]
    fn unknown_flags_are_a_parse_error() {
        let opts = parse(&["uqm", "--warp-drive"]);

        assert!(opts.parse_error);
    }

    #[test]
    fn display_and_path_flags_are_applied() {
        let opts = parse(&[
            "uqm",
            "--keepaspectratio",
            "--stereosfx",
            "--automation-script",
            "run.json",
            "--automation-output",
            "out",
            "--configdir",
            "/c",
        ]);

        assert!(opts.keep_aspect_ratio && opts.stereo_sfx);
        assert_eq!(opts.automation_script.as_deref(), Some("run.json"));
        assert_eq!(opts.automation_output.as_deref(), Some("out"));
        assert_eq!(opts.config_dir.as_deref(), Some("/c"));
    }

    #[cfg(feature = "debug-process")]
    #[test]
    fn the_native_window_proof_path_is_kept() {
        let opts = parse(&["uqm", "--native-window-proof", "proof.json"]);

        assert_eq!(opts.native_window_proof.as_deref(), Some("proof.json"));
    }
}
