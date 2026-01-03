use crate::config::{parse_gamma, parse_resolution, parse_volume};
use crate::config::{ChoiceOption, MeleeScale, Options, Scaler, SoundDriver, SoundQuality};
use anyhow::{Context, Result};
use clap::Parser;

/// The Ur-Quan Masters - A modernized Rust implementation
#[derive(Parser, Debug)]
#[command(name = "uqm")]
#[command(version = "0.8.0")]
#[command(about = "The Ur-Quan Masters - space exploration strategy game", long_about = None)]
pub struct Cli {
    /// Screen resolution (e.g., 640x480)
    #[arg(short, long, value_name = "WIDTHxHEIGHT")]
    pub res: Option<String>,

    /// Enable fullscreen mode
    #[arg(short, long)]
    pub fullscreen: bool,

    /// Enable OpenGL rendering
    #[arg(short, long)]
    pub opengl: bool,

    /// Disable OpenGL rendering
    #[arg(long = "nogl")]
    pub nogl: bool,

    /// Enable aspect ratio preservation
    #[arg(short, long)]
    pub keepaspectratio: bool,

    /// Scaler mode (bilinear, biadapt, biadv, triscan, hq, none)
    #[arg(short, long, value_name = "MODE")]
    pub scale: Option<String>,

    /// Melee scaling mode (step/pc, smooth/3do, bilinear)
    #[arg(long = "meleezoom", value_name = "MODE")]
    pub meleezoom: Option<String>,

    /// Enable scanlines effect
    #[arg(short, long)]
    pub scanlines: bool,

    /// Show FPS counter
    #[arg(short, long)]
    pub fps: bool,

    /// Gamma correction value (default 1.0)
    #[arg(short, long, value_name = "CORRECTIONVALUE")]
    pub gamma: Option<String>,

    /// Configuration directory path
    #[arg(short, long, value_name = "CONFIGDIR")]
    pub configdir: Option<String>,

    /// Content directory path
    #[arg(short, long, value_name = "CONTENTDIR")]
    pub contentdir: Option<String>,

    /// Music volume (0-100)
    #[arg(long, value_name = "VOLUME")]
    pub musicvol: Option<String>,

    /// Sound effects volume (0-100)
    #[arg(long, value_name = "VOLUME")]
    pub sfxvol: Option<String>,

    /// Speech volume (0-100)
    #[arg(long, value_name = "VOLUME")]
    pub speechvol: Option<String>,

    /// Audio quality (high, medium, low)
    #[arg(short, long, value_name = "QUALITY")]
    pub audioquality: Option<String>,

    /// Disable subtitles
    #[arg(short, long)]
    pub nosubtitles: bool,

    /// Log file path
    #[arg(short, long, value_name = "FILE")]
    pub logfile: Option<String>,

    /// Intro/version (pc, 3do)
    #[arg(short, long, value_name = "CHOICE")]
    pub intro: Option<String>,

    /// Coarse scan display (pc, 3do)
    #[arg(long, value_name = "CHOICE")]
    pub cscan: Option<String>,

    /// Menu type (pc, 3do)
    #[arg(long, value_name = "CHOICE")]
    pub menu: Option<String>,

    /// Font style (pc, 3do)
    #[arg(long, value_name = "CHOICE")]
    pub font: Option<String>,

    /// Shield type (pc, 3do)
    #[arg(long, value_name = "CHOICE")]
    pub shield: Option<String>,

    /// Scroll behavior (pc, 3do)
    #[arg(long, value_name = "CHOICE")]
    pub scroll: Option<String>,

    /// Sound driver (openal, mixsdl, none)
    #[arg(long, value_name = "DRIVER")]
    pub sound: Option<String>,

    /// Enable stereo SFX
    #[arg(long)]
    pub stereosfx: bool,

    /// Add addon path (can be specified multiple times)
    #[arg(long, value_name = "ADDON")]
    pub addon: Vec<String>,

    /// Addon directory
    #[arg(long, value_name = "ADDONDIR")]
    pub addondir: Option<String>,

    /// CPU acceleration (mmx, sse, 3dnow, none, detect)
    #[arg(long, value_name = "ACCEL")]
    pub accel: Option<String>,

    /// Start in safe mode
    #[arg(long)]
    pub safe: bool,

    /// Select named rendering engine
    #[arg(long, value_name = "NAME")]
    pub renderer: Option<String>,

    /// Enable windowed mode
    #[arg(short, long)]
    pub windowed: bool,
}

impl Cli {
    /// Merge CLI arguments into the options struct
    pub fn merge_into_options(&self, mut opts: Options) -> Result<Options> {
        // Override with command line arguments
        if let Some(ref res) = self.res {
            opts.resolution = Some(parse_resolution(res).context("Invalid resolution format")?);
        }

        if self.fullscreen {
            opts.fullscreen = Some(true);
        }
        if self.windowed {
            opts.fullscreen = Some(false);
        }
        if self.opengl {
            opts.opengl = Some(true);
        }
        if self.nogl {
            opts.opengl = Some(false);
        }
        if self.keepaspectratio {
            opts.keep_aspect_ratio = Some(true);
        }

        if let Some(ref scale) = self.scale {
            opts.scaler = Some(Self::parse_scale(scale)?);
        }

        if let Some(ref zoom) = self.meleezoom {
            opts.melee_scale = Some(Self::parse_melee_zoom(zoom)?);
        }

        if self.scanlines {
            opts.scanlines = Some(true);
        }

        if self.fps {
            opts.show_fps = Some(true);
        }

        if let Some(ref gamma) = self.gamma {
            opts.gamma = Some(parse_gamma(gamma)?);
        }

        if let Some(ref config_dir) = self.configdir {
            opts.config_dir = Some(config_dir.clone());
        }

        if let Some(ref content_dir) = self.contentdir {
            opts.content_dir = Some(content_dir.clone());
        }

        if let Some(ref vol) = self.musicvol {
            let int_vol: i32 = vol.parse().context("Invalid music volume")?;
            opts.music_volume = Some(parse_volume(int_vol));
        }

        if let Some(ref vol) = self.sfxvol {
            let int_vol: i32 = vol.parse().context("Invalid SFX volume")?;
            opts.sfx_volume = Some(parse_volume(int_vol));
        }

        if let Some(ref vol) = self.speechvol {
            let int_vol: i32 = vol.parse().context("Invalid speech volume")?;
            opts.speech_volume = Some(parse_volume(int_vol));
        }

        if let Some(ref quality) = self.audioquality {
            opts.sound_quality = Some(Self::parse_audio_quality(quality)?);
        }

        if self.nosubtitles {
            opts.subtitles = Some(false);
        }

        if let Some(ref log_file) = self.logfile {
            opts.log_file = Some(log_file.clone());
        }

        if let Some(ref intro) = self.intro {
            opts.which_intro = Some(Self::parse_choice(intro)?);
        }

        if let Some(ref cscan) = self.cscan {
            opts.which_coarse_scan = Some(Self::parse_choice(cscan)?);
        }

        if let Some(ref menu) = self.menu {
            opts.which_menu = Some(Self::parse_choice(menu)?);
        }

        if let Some(ref font) = self.font {
            opts.which_fonts = Some(Self::parse_choice(font)?);
        }

        if let Some(ref shield) = self.shield {
            opts.which_shield = Some(Self::parse_choice(shield)?);
        }

        if let Some(ref scroll) = self.scroll {
            opts.smooth_scroll = Some(Self::parse_choice(scroll)?);
        }

        if let Some(ref sound) = self.sound {
            opts.sound_driver = Some(Self::parse_sound_driver(sound)?);
        }

        if self.stereosfx {
            opts.stereo_sfx = Some(true);
        }

        if !self.addon.is_empty() {
            opts.addons = self.addon.clone();
        }

        if let Some(ref addon_dir) = self.addondir {
            opts.addon_dir = Some(addon_dir.clone());
        }

        if self.safe {
            opts.safe_mode = Some(true);
        }

        if let Some(ref renderer) = self.renderer {
            opts.graphics_backend = Some(renderer.clone());
        }

        Ok(opts)
    }

    fn parse_scale(s: &str) -> Result<Scaler> {
        match s.to_lowercase().as_str() {
            "bilinear" => Ok(Scaler::Bilinear),
            "biadapt" => Ok(Scaler::Biadapt),
            "biadv" => Ok(Scaler::Biadv),
            "triscan" => Ok(Scaler::Triscan),
            "hq" => Ok(Scaler::Hq),
            "none" | "no" => Ok(Scaler::None),
            _ => anyhow::bail!("Invalid scaler mode: {}. Valid options: bilinear, biadapt, biadv, triscan, hq, none", s),
        }
    }

    fn parse_melee_zoom(s: &str) -> Result<MeleeScale> {
        match s.to_lowercase().as_str() {
            "smooth" | "3do" => Ok(MeleeScale::Smooth),
            "step" | "pc" => Ok(MeleeScale::Step),
            "bilinear" => Ok(MeleeScale::Bilinear),
            _ => anyhow::bail!(
                "Invalid melee zoom mode: {}. Valid options: smooth, step, bilinear",
                s
            ),
        }
    }

    fn parse_audio_quality(s: &str) -> Result<SoundQuality> {
        match s.to_lowercase().as_str() {
            "low" => Ok(SoundQuality::Low),
            "medium" => Ok(SoundQuality::Medium),
            "high" => Ok(SoundQuality::High),
            _ => anyhow::bail!(
                "Invalid audio quality: {}. Valid options: low, medium, high",
                s
            ),
        }
    }

    fn parse_choice(s: &str) -> Result<ChoiceOption> {
        match s.to_lowercase().as_str() {
            "pc" => Ok(ChoiceOption::Pc),
            "3do" => Ok(ChoiceOption::ThreeDo),
            _ => anyhow::bail!("Invalid choice: {}. Valid options: pc, 3do", s),
        }
    }

    fn parse_sound_driver(s: &str) -> Result<SoundDriver> {
        match s.to_lowercase().as_str() {
            "openal" => Ok(SoundDriver::OpenAl),
            "mixsdl" => Ok(SoundDriver::MixSdl),
            "none" | "nosound" => Ok(SoundDriver::None),
            _ => anyhow::bail!(
                "Invalid sound driver: {}. Valid options: openal, mixsdl, none",
                s
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scaler() {
        assert_eq!(Cli::parse_scale("bilinear").unwrap(), Scaler::Bilinear);
        assert_eq!(Cli::parse_scale("NONE").unwrap(), Scaler::None);
        assert!(Cli::parse_scale("invalid").is_err());
    }

    #[test]
    fn test_parse_melee_zoom() {
        assert_eq!(Cli::parse_melee_zoom("smooth").unwrap(), MeleeScale::Smooth);
        assert_eq!(Cli::parse_melee_zoom("3do").unwrap(), MeleeScale::Smooth);
        assert_eq!(Cli::parse_melee_zoom("STEP").unwrap(), MeleeScale::Step);
        assert!(Cli::parse_melee_zoom("invalid").is_err());
    }

    #[test]
    fn test_parse_audio_quality() {
        assert_eq!(
            Cli::parse_audio_quality("high").unwrap(),
            SoundQuality::High
        );
        assert_eq!(Cli::parse_audio_quality("Low").unwrap(), SoundQuality::Low);
        assert!(Cli::parse_audio_quality("invalid").is_err());
    }

    #[test]
    fn test_parse_choice() {
        assert_eq!(Cli::parse_choice("pc").unwrap(), ChoiceOption::Pc);
        assert_eq!(Cli::parse_choice("3DO").unwrap(), ChoiceOption::ThreeDo);
        assert!(Cli::parse_choice("invalid").is_err());
    }

    #[test]
    fn test_parse_sound_driver() {
        assert_eq!(
            Cli::parse_sound_driver("openal").unwrap(),
            SoundDriver::OpenAl
        );
        assert_eq!(
            Cli::parse_sound_driver("MIXSDL").unwrap(),
            SoundDriver::MixSdl
        );
        assert_eq!(Cli::parse_sound_driver("none").unwrap(), SoundDriver::None);
        assert!(Cli::parse_sound_driver("invalid").is_err());
    }

    #[test]
    fn test_merge_basic_options() {
        let cli = Cli {
            res: Some("800x600".to_string()),
            fullscreen: true,
            ..Default::default()
        };

        let opts = cli.merge_into_options(Options::default()).unwrap();
        assert_eq!(
            opts.resolution,
            Some(Resolution {
                width: 800,
                height: 600
            })
        );
        assert_eq!(opts.fullscreen, Some(true));
    }

    #[test]
    fn test_invalid_resolution() {
        let cli = Cli {
            res: Some("invalid".to_string()),
            ..Default::default()
        };

        let result = cli.merge_into_options(Options::default());
        assert!(result.is_err());
    }
}
