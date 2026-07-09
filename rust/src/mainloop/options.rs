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
const OPT_PC: c_int = 0;
const OPT_3DO: c_int = 1;

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

    /// Non-zero if parsing failed.
    pub parse_error: bool,
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
            resolution_width: 1280,
            resolution_height: 960,
            parse_error: false,
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
    Command::new("uqm")
        .about("The Ur-Quan Masters")
        .disable_help_flag(true)
        .disable_version_flag(true)
        // Short options that mirror C's optString
        .arg(Arg::new("res")
            .short('r').long("res")
            .value_name("WIDTHxHEIGHT")
            .help("Resolution (default 1280x960)"))
        .arg(Arg::new("fullscreen")
            .short('f').long("fullscreen")
            .action(ArgAction::SetTrue)
            .help("Fullscreen mode"))
        .arg(Arg::new("windowed")
            .short('w').long("windowed")
            .action(ArgAction::SetTrue)
            .help("Windowed mode (default)"))
        .arg(Arg::new("opengl")
            .short('o').long("opengl")
            .action(ArgAction::SetTrue)
            .help("Use OpenGL renderer"))
        .arg(Arg::new("nogl")
            .short('x').long("nogl")
            .action(ArgAction::SetTrue)
            .help("Use pure SDL2 renderer (default)"))
        .arg(Arg::new("scale")
            .short('c').long("scale")
            .value_name("MODE")
            .help("Scaler: hq, xbrz3, xbrz4, or none (default xbrz3)"))
        .arg(Arg::new("meleezoom")
            .short('b').long("meleezoom")
            .value_name("MODE")
            .help("Melee zoom: step (pc) or smooth (3do)"))
        .arg(Arg::new("scanlines")
            .short('s').long("scanlines")
            .action(ArgAction::SetTrue)
            .help("Enable scanlines"))
        .arg(Arg::new("fps")
            .short('p').long("fps")
            .action(ArgAction::SetTrue)
            .help("Show FPS counter"))
        .arg(Arg::new("configdir")
            .short('C').long("configdir")
            .value_name("DIR"))
        .arg(Arg::new("contentdir")
            .short('n').long("contentdir")
            .value_name("DIR"))
        .arg(Arg::new("musicvol")
            .short('M').long("musicvol")
            .value_name("0-100"))
        .arg(Arg::new("sfxvol")
            .short('S').long("sfxvol")
            .value_name("0-100"))
        .arg(Arg::new("speechvol")
            .short('T').long("speechvol")
            .value_name("0-100"))
        .arg(Arg::new("audioquality")
            .short('q').long("audioquality")
            .value_name("QUALITY")
            .help("high, medium, or low"))
        .arg(Arg::new("nosubtitles")
            .short('u').long("nosubtitles")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("gamma")
            .short('g').long("gamma")
            .value_name("VALUE"))
        .arg(Arg::new("logfile")
            .short('l').long("logfile")
            .value_name("FILE"))
        .arg(Arg::new("intro")
            .short('i').long("intro")
            .value_name("VERSION")
            .help("3do or pc"))
        .arg(Arg::new("help")
            .short('h').long("help")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("version")
            .short('v').long("version")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("keepaspectratio")
            .short('k').long("keepaspectratio")
            .action(ArgAction::SetTrue)
            .help("Keep aspect ratio (default on)"))
        // Long-only options (no short equivalent in C)
        .arg(Arg::new("cscan")
            .long("cscan")
            .value_name("VERSION")
            .help("3do or pc"))
        .arg(Arg::new("menu")
            .long("menu")
            .value_name("TYPE")
            .help("3do or pc"))
        .arg(Arg::new("font")
            .long("font")
            .value_name("TYPE")
            .help("3do or pc"))
        .arg(Arg::new("shield")
            .long("shield")
            .value_name("TYPE")
            .help("3do or pc"))
        .arg(Arg::new("scroll")
            .long("scroll")
            .value_name("TYPE")
            .help("3do or pc"))
        .arg(Arg::new("sound")
            .long("sound")
            .value_name("DRIVER")
            .help("openal, mixsdl, or none"))
        .arg(Arg::new("stereosfx")
            .long("stereosfx")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("addon")
            .long("addon")
            .value_name("ADDON")
            .action(ArgAction::Append))
        .arg(Arg::new("addondir")
            .long("addondir")
            .value_name("DIR"))
        .arg(Arg::new("safe")
            .long("safe")
            .action(ArgAction::SetTrue)
            .help("Start in safe mode"))
        .arg(Arg::new("renderer")
            .long("renderer")
            .value_name("NAME"))
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
pub fn parse_and_store(args: &[String]) -> ParsedOptions {
    let mut opts = parse_args(args);
    if opts.content_dir.is_none() {
        opts.content_dir = discover_content_dir();
    }
    let _ = PARSED.set(opts.clone());
    opts
}

fn parse_args(args: &[String]) -> ParsedOptions {
    let mut opts = ParsedOptions::default();

    // macOS: Finder injects -psn_XXXXXX on double-click launch.
    if args.len() >= 2 && args[1].starts_with("-psn_") {
        return opts;
    }

    let cmd = build_command();
    let matches = match cmd.try_get_matches_from(args) {
        Ok(m) => m,
        Err(_) => {
            opts.parse_error = true;
            return opts;
        }
    };

    if matches.get_flag("help") {
        opts.run_mode = RunMode::Usage;
        return opts;
    }
    if matches.get_flag("version") {
        opts.run_mode = RunMode::Version;
        return opts;
    }

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

    // Addons (can be specified multiple times)
    if let Some(addons) = matches.get_many::<String>("addon") {
        opts.addons = addons.cloned().collect();
    }

    // List/enum options
    if let Some(v) = matches.get_one::<String>("scale") {
        opts.scaler = match v.as_str() {
            "xbrz3" => TFB_GFXFLAGS_SCALE_XBRZ3,
            "xbrz4" => TFB_GFXFLAGS_SCALE_XBRZ4,
            "hq" => TFB_GFXFLAGS_SCALE_HQXX,
            "none" | "no" => 0,
            _ => {
                opts.parse_error = true;
                return opts;
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
                return opts;
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
                return opts;
            }
        };
    }
    if let Some(v) = matches.get_one::<String>("sound") {
        opts.sound_driver = match v.as_str() {
            "mixsdl" => AUDIO_DRIVER_MIXSDL,
            "none" | "nosound" => AUDIO_DRIVER_NOSOUND,
            _ => {
                opts.parse_error = true;
                return opts;
            }
        };
    }

    // 3do/pc choice options
    if let Some(v) = matches.get_one::<String>("intro") {
        opts.which_intro = parse_choice(v, &mut opts);
    }
    if let Some(v) = matches.get_one::<String>("cscan") {
        opts.which_coarse_scan = parse_choice(v, &mut opts);
    }
    if let Some(v) = matches.get_one::<String>("menu") {
        opts.which_menu = parse_choice(v, &mut opts);
    }
    if let Some(v) = matches.get_one::<String>("font") {
        opts.which_fonts = parse_choice(v, &mut opts);
    }
    if let Some(v) = matches.get_one::<String>("shield") {
        opts.which_shield = parse_choice(v, &mut opts);
    }
    if let Some(v) = matches.get_one::<String>("scroll") {
        opts.smooth_scroll = parse_choice(v, &mut opts);
    }

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

    opts
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
                std::env::current_dir()
                    .ok()?
                    .to_string_lossy()
                    .to_string()
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
    CString::new(val)
        .unwrap_or_default()
        .into_raw()
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
            if parsed().$field { 1 } else { 0 }
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
        parse_args(&owned)
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
        assert_eq!(opts.resolution_width, 1280);
        assert_eq!(opts.resolution_height, 960);
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
        let opts = parse_args(&[
            "uqm".to_string(),
            "-psn_0_123456".to_string(),
        ]);
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
            assert!(std::path::Path::new(path).join("version").exists(),
                "discovered content dir {path} should contain version file");
        }
    }
}
