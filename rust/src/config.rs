use anyhow::{Context, Result};

/// Application options that can be set via CLI or config file
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Options {
    // Commandline-only options
    pub log_file: Option<String>,
    pub config_dir: Option<String>,
    pub content_dir: Option<String>,
    pub addon_dir: Option<String>,
    pub addons: Vec<String>,
    pub graphics_backend: Option<String>,

    // Commandline and user config options
    pub opengl: Option<bool>,
    pub resolution: Option<Resolution>,
    pub fullscreen: Option<bool>,
    pub scanlines: Option<bool>,
    pub scaler: Option<Scaler>,
    pub show_fps: Option<bool>,
    pub keep_aspect_ratio: Option<bool>,
    pub gamma: Option<f32>,
    pub sound_driver: Option<SoundDriver>,
    pub sound_quality: Option<SoundQuality>,
    pub use_3do_music: Option<bool>,
    pub use_remix_music: Option<bool>,
    pub use_speech: Option<bool>,
    pub which_coarse_scan: Option<ChoiceOption>,
    pub which_menu: Option<ChoiceOption>,
    pub which_fonts: Option<ChoiceOption>,
    pub which_intro: Option<ChoiceOption>,
    pub which_shield: Option<ChoiceOption>,
    pub smooth_scroll: Option<ChoiceOption>,
    pub melee_scale: Option<MeleeScale>,
    pub subtitles: Option<bool>,
    pub stereo_sfx: Option<bool>,
    pub music_volume: Option<f32>,
    pub sfx_volume: Option<f32>,
    pub speech_volume: Option<f32>,
    pub safe_mode: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaler {
    Bilinear,
    Biadapt,
    Biadv,
    Triscan,
    Hq,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundDriver {
    OpenAl,
    MixSdl,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundQuality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceOption {
    Pc,
    ThreeDo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeleeScale {
    Smooth,
    Step,
    Bilinear,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            log_file: None,
            config_dir: None,
            content_dir: None,
            addon_dir: None,
            addons: Vec::new(),
            graphics_backend: None,
            opengl: None,
            resolution: Some(Resolution {
                width: 640,
                height: 480,
            }),
            fullscreen: None,
            scanlines: None,
            scaler: None,
            show_fps: None,
            keep_aspect_ratio: None,
            gamma: None,
            sound_driver: None,
            sound_quality: None,
            use_3do_music: None,
            use_remix_music: None,
            use_speech: None,
            which_coarse_scan: None,
            which_menu: None,
            which_fonts: None,
            which_intro: None,
            which_shield: None,
            smooth_scroll: None,
            melee_scale: None,
            subtitles: None,
            stereo_sfx: None,
            music_volume: None,
            sfx_volume: None,
            speech_volume: None,
            safe_mode: None,
        }
    }
}

/// Load configuration from uqm.cfg file
/// For Phase 0, this is a stub that will be expanded in later phases
pub fn load_config(_config_dir: &Option<String>) -> Result<Options> {
    // For Phase 0, we'll return default options
    // In later phases, this will read from uqm.cfg
    Ok(Options::default())
}

/// Parse a resolution string in the format "WIDTHxHEIGHT"
pub fn parse_resolution(s: &str) -> Result<Resolution> {
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() != 2 {
        anyhow::bail!("Resolution must be in WIDTHxHEIGHT format");
    }

    let width: u32 = parts[0].parse().context("Invalid width value")?;
    let height: u32 = parts[1].parse().context("Invalid height value")?;

    if width == 0 || height == 0 {
        anyhow::bail!("Resolution values must be positive");
    }

    Ok(Resolution { width, height })
}

/// Parse a volume value (0-100) to a float (0.0-1.0)
pub fn parse_volume(vol: i32) -> f32 {
    if vol < 0 {
        return 0.0;
    }
    if vol > 100 {
        return 1.0;
    }
    vol as f32 / 100.0
}

/// Parse a gamma correction value
pub fn parse_gamma(s: &str) -> Result<f32> {
    let gamma: f32 = s.parse().context("Invalid gamma value")?;

    const GAMMA_SCALE: f32 = 1000.0;
    const MIN_GAMMA: f32 = 0.03 * GAMMA_SCALE / GAMMA_SCALE;
    const MAX_GAMMA: f32 = 9.9 * GAMMA_SCALE / GAMMA_SCALE;

    if gamma < MIN_GAMMA || gamma > MAX_GAMMA {
        anyhow::bail!("Gamma correction value out of range (0.03 to 9.9)");
    }

    Ok(gamma)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_resolution_valid() {
        let res = parse_resolution("640x480").unwrap();
        assert_eq!(res.width, 640);
        assert_eq!(res.height, 480);
    }

    #[test]
    fn test_parse_resolution_invalid_format() {
        assert!(parse_resolution("640-480").is_err());
        assert!(parse_resolution("640x480x120").is_err());
    }

    #[test]
    fn test_parse_resolution_invalid_values() {
        assert!(parse_resolution("0x480").is_err());
        assert!(parse_resolution("640x0").is_err());
        assert!(parse_resolution("abcxdef").is_err());
    }

    #[test]
    fn test_parse_volume() {
        assert_eq!(parse_volume(0), 0.0);
        assert_eq!(parse_volume(50), 0.5);
        assert_eq!(parse_volume(100), 1.0);
        assert_eq!(parse_volume(-10), 0.0);
        assert_eq!(parse_volume(150), 1.0);
    }

    #[test]
    fn test_parse_gamma() {
        assert_eq!(parse_gamma("1.0").unwrap(), 1.0);
        assert!(parse_gamma("0.02").is_err()); // Too low
        assert!(parse_gamma("10.0").is_err()); // Too high
        assert!(parse_gamma("abc").is_err()); // Invalid
    }

    #[test]
    fn test_options_default() {
        let opts = Options::default();
        assert_eq!(
            opts.resolution,
            Some(Resolution {
                width: 640,
                height: 480
            })
        );
        assert!(opts.opengl.is_none());
        assert!(opts.addons.is_empty());
    }
}
