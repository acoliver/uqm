//! Setup choices and their conversion to supplied runtime configuration.
use super::GammaRange;
use crate::engine::input::InputSession;
use uqm_engine::{
    configuration::runtime::{AudioSettings, DisplaySettings},
    resource::settings::Settings,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlobalOptions {
    pub scaler: u32,
    pub res: u32,
    pub driver: u32,
    pub adriver: u32,
    pub aquality: u32,
    pub fullscreen: u32,
    pub subtitles: u32,
    pub scanlines: u32,
    pub fps: u32,
    pub stereo: u32,
    pub music3do: u32,
    pub musicremix: u32,
    pub speech: u32,
    pub keepaspect: u32,
    pub menu: u32,
    pub text: u32,
    pub cscan: u32,
    pub scroll: u32,
    pub intro: u32,
    pub meleezoom: u32,
    pub shield: u32,
    pub player1: u32,
    pub player2: u32,
    pub speechvol: i32,
    pub musicvol: i32,
    pub sfxvol: i32,
    pub gamma: i32,
}

impl GlobalOptions {
    pub(super) fn choices(&self) -> [usize; 24] {
        [
            self.res,
            self.driver,
            self.scaler,
            self.scanlines,
            self.menu,
            self.text,
            self.cscan,
            self.scroll,
            self.subtitles,
            self.music3do,
            self.fullscreen,
            self.intro,
            self.fps,
            self.meleezoom,
            self.stereo,
            self.adriver,
            self.aquality,
            self.shield,
            self.player1,
            self.player2,
            0,
            self.musicremix,
            self.speech,
            self.keepaspect,
        ]
        .map(|value| value as usize)
    }

    pub(super) fn from_widgets(c: &[usize; 24], s: [i32; 4]) -> Self {
        Self {
            res: c[0] as u32,
            driver: c[1] as u32,
            scaler: c[2] as u32,
            scanlines: c[3] as u32,
            menu: c[4] as u32,
            text: c[5] as u32,
            cscan: c[6] as u32,
            scroll: c[7] as u32,
            subtitles: c[8] as u32,
            music3do: c[9] as u32,
            fullscreen: c[10] as u32,
            intro: c[11] as u32,
            fps: c[12] as u32,
            meleezoom: c[13] as u32,
            stereo: c[14] as u32,
            adriver: c[15] as u32,
            aquality: c[16] as u32,
            shield: c[17] as u32,
            player1: c[18] as u32,
            player2: c[19] as u32,
            musicremix: c[21] as u32,
            speech: c[22] as u32,
            keepaspect: c[23] as u32,
            musicvol: s[0],
            sfxvol: s[1],
            speechvol: s[2],
            gamma: s[3],
        }
    }

    pub(super) fn sliders(&self) -> [i32; 4] {
        [self.musicvol, self.sfxvol, self.speechvol, self.gamma]
    }
}

pub fn get_global_options(
    input: &InputSession,
    display: DisplaySettings,
    audio: AudioSettings,
    always_gl: Option<bool>,
    range: &mut GammaRange,
) -> GlobalOptions {
    let p = input.preferences();
    let mut opts = GlobalOptions {
        scaler: if display.flags & 256 != 0 {
            2
        } else if display.flags & 512 != 0 {
            3
        } else if display.flags & 128 != 0 {
            1
        } else {
            2
        },
        fullscreen: u32::from(display.flags & 1 != 0),
        scanlines: u32::from(display.flags & 4 != 0),
        fps: u32::from(display.flags & 2 != 0),
        subtitles: u32::from(p.subtitles != 0),
        menu: u32::from(p.menu == 1),
        text: u32::from(p.fonts == 1),
        cscan: u32::from(p.coarse_scan == 1),
        scroll: u32::from(p.smooth_scroll == 1),
        intro: u32::from(p.intro == 1),
        shield: u32::from(p.shield == 1),
        meleezoom: u32::from(p.melee_scale != 0),
        stereo: u32::from(p.stereo_sfx != 0),
        music3do: u32::from(p.music_3do != 0),
        musicremix: u32::from(p.music_remix != 0),
        speech: u32::from(p.speech != 0),
        keepaspect: u32::from(p.keep_aspect_ratio != 0),
        adriver: match audio.driver {
            2 => 2,
            0 => 1,
            _ => 0,
        },
        aquality: if audio.flags & 1 != 0 {
            2
        } else if audio.flags & 4 != 0 {
            0
        } else {
            1
        },
        player1: input.player_controls()[0],
        player2: input.player_controls()[1],
        musicvol: rounded_volume(audio.music),
        sfxvol: rounded_volume(audio.sfx),
        speechvol: rounded_volume(audio.speech),
        ..GlobalOptions::default()
    };
    (opts.res, opts.driver) = resolution_choice(display);
    if let Some(always_gl) = always_gl {
        opts.driver = u32::from(always_gl);
    }
    let upper = p.gamma < range.max;
    if p.gamma <= range.min {
        range.min = p.gamma - 0.03;
    } else if p.gamma >= range.max {
        range.max = p.gamma + 0.3;
    }
    range.update_bounds(upper);
    opts.gamma = range.gamma_to_slider(p.gamma);
    opts
}

fn rounded_volume(volume: f32) -> i32 {
    ((volume * 100.0) as i32 + 2) / 5 * 5
}

fn resolution_choice(display: DisplaySettings) -> (u32, u32) {
    match display.width {
        320 | 640 => {
            let (height, res) = if display.width == 320 {
                (240, 0)
            } else {
                (480, 1)
            };
            if display.driver == 1 {
                (res, 0)
            } else if display.height != height {
                (6, 0)
            } else {
                (res, 1)
            }
        }
        800 if display.height == 600 => (2, 0),
        1024 if display.height == 768 => (3, 0),
        1280 if display.height == 960 => (4, 0),
        960 if display.height == 720 => (5, 0),
        _ => (6, 0),
    }
}

pub(super) fn prepare_display(
    opts: &GlobalOptions,
    mut display: DisplaySettings,
    settings: &Settings,
) -> DisplaySettings {
    let resolutions = [
        (320, 240),
        (640, 480),
        (800, 600),
        (1024, 768),
        (1280, 960),
        (960, 720),
    ];
    if let Some(&(width, height)) = resolutions.get(opts.res as usize) {
        display.width = width;
        display.height = height;
        display.driver = if opts.res < 2 { 1 } else { 0 };
    }
    display.flags &= !1016;
    let (scale_flag, scaler) = match opts.scaler {
        1 => (128, "hq"),
        2 => (256, "xbrz3"),
        3 => (512, "xbrz4"),
        _ => (0, "no"),
    };
    display.flags |= scale_flag;
    for (value, flag) in [(opts.scanlines, 4), (opts.fullscreen, 1)] {
        if value != 0 {
            display.flags |= flag;
        } else {
            display.flags &= !flag;
        }
    }
    settings.put_integer("config.reswidth", display.width);
    settings.put_integer("config.resheight", display.height);
    settings.put_boolean("config.alwaysgl", opts.driver == 1);
    settings.put_boolean("config.usegl", display.driver == 0);
    settings.put_string("config.scaler", scaler);
    settings.put_boolean("config.scanlines", opts.scanlines != 0);
    settings.put_boolean("config.fullscreen", opts.fullscreen != 0);
    display
}

pub fn set_global_options(
    input: &InputSession,
    opts: &GlobalOptions,
    range: &GammaRange,
    settings: &Settings,
) {
    apply_preferences(input, opts, range);
    persist_preferences(input, opts, settings);
}

fn console_type(value: u32) -> i32 {
    if value == 1 {
        1
    } else {
        2
    }
}

fn apply_preferences(input: &InputSession, o: &GlobalOptions, range: &GammaRange) {
    input.configuration.update_preferences(|p| {
        p.gamma = range.slider_to_gamma(o.gamma);
        p.subtitles = u32::from(o.subtitles == 1);
        p.menu = console_type(o.menu);
        p.fonts = console_type(o.text);
        p.coarse_scan = console_type(o.cscan);
        p.smooth_scroll = console_type(o.scroll);
        p.shield = console_type(o.shield);
        p.intro = console_type(o.intro);
        p.melee_scale = if o.meleezoom == 1 { 3 } else { 0 };
        p.music_3do = u32::from(o.music3do == 1);
        p.music_remix = u32::from(o.musicremix == 1);
        p.speech = u32::from(o.speech == 1);
        p.stereo_sfx = u32::from(o.stereo == 1);
        p.keep_aspect_ratio = u32::from(o.keepaspect == 1);
    });
    input.set_player_control(0, o.player1);
    input.set_player_control(1, o.player2);
}

fn persist_preferences(input: &InputSession, o: &GlobalOptions, settings: &Settings) {
    for (key, value) in [
        ("config.subtitles", o.subtitles == 1),
        ("config.textmenu", o.menu == 0),
        ("config.textgradients", o.text == 0),
        ("config.iconicscan", o.cscan == 1),
        ("config.smoothscroll", o.scroll == 1),
        ("config.3domusic", o.music3do == 1),
        ("config.remixmusic", o.musicremix == 1),
        ("config.speech", o.speech == 1),
        ("config.3domovies", o.intro == 1),
        ("config.showfps", o.fps == 1),
        ("config.smoothmelee", o.meleezoom == 1),
        ("config.positionalsfx", o.stereo == 1),
        ("config.pulseshield", o.shield == 1),
        ("config.keepaspectratio", o.keepaspect == 1),
    ] {
        settings.put_boolean(key, value);
    }
    settings.put_integer(
        "config.gamma",
        ((input.preferences().gamma * 1000.0) as f64 + 0.5) as i32,
    );
    settings.put_integer("config.player1control", o.player1 as i32);
    settings.put_integer("config.player2control", o.player2 as i32);
    if let Some(name) = ["none", "mixsdl", "openal"].get(o.adriver as usize) {
        settings.put_string("config.audiodriver", name);
    }
    if let Some(name) = ["low", "medium", "high"].get(o.aquality as usize) {
        settings.put_string("config.audioquality", name);
    }
    for (key, value) in [
        ("config.musicvol", o.musicvol),
        ("config.sfxvol", o.sfxvol),
        ("config.speechvol", o.speechvol),
    ] {
        settings.put_integer(key, value);
    }
}
