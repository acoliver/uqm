//! Colour maps (palettes indexed by colour-map number), screen fades and
//! colour-map transforms, ported from cmap.c (GPL-2.0-or-later).
//!
//! A colour map is an immutable snapshot: replacing map `n` installs a new
//! snapshot with the next version, while queued draws keep the one they
//! took.
use super::geometry::Color;
use std::sync::Arc;

pub const MAX_COLORMAPS: usize = 250;
pub const NUMBER_OF_PLUTVALS: usize = 256;
pub const FADE_NO_INTENSITY: i32 = 0;
pub const FADE_NORMAL_INTENSITY: i32 = 255;
pub const FADE_FULL_INTENSITY: i32 = 510;

pub const FADE_ALL_TO_WHITE: u32 = 250;
pub const FADE_ALL_TO_BLACK: u32 = 252;
pub const FADE_ALL_TO_COLOR: u32 = 253;

const MAX_XFORMS: usize = 16;
const ENTRY_BYTES: usize = NUMBER_OF_PLUTVALS * 3;

/// One installed palette.
#[derive(Debug, PartialEq, Eq)]
pub struct ColorMap {
    pub index: i32,
    pub version: i32,
    pub palette: [Color; NUMBER_OF_PLUTVALS],
}

/// A rejected colour-map table.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ColorMapError {
    #[error("colour-map table of {0} bytes has no range header")]
    MissingHeader(usize),
    #[error("colour-map range {start}..={end} is reversed or beyond {MAX_COLORMAPS} maps")]
    InvalidRange { start: usize, end: usize },
    #[error("colour-map range {start}..={end} needs {needed} bytes, table has {actual}")]
    Truncated {
        start: usize,
        end: usize,
        needed: usize,
        actual: usize,
    },
}

/// A colour-map table: a two-byte inclusive range header followed by 256 RGB
/// triples for every map in the range, validated before any map is read.
#[derive(Debug, Clone, Copy)]
pub struct ColorMapTable<'a> {
    start: usize,
    end: usize,
    colors: &'a [u8],
}

impl<'a> ColorMapTable<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ColorMapError> {
        let [start, end, ..] = *bytes else {
            return Err(ColorMapError::MissingHeader(bytes.len()));
        };
        let (start, end) = (usize::from(start), usize::from(end));
        if start > end || end >= MAX_COLORMAPS {
            return Err(ColorMapError::InvalidRange { start, end });
        }
        let needed = 2 + (end - start + 1) * ENTRY_BYTES;
        if bytes.len() < needed {
            return Err(ColorMapError::Truncated {
                start,
                end,
                needed,
                actual: bytes.len(),
            });
        }
        Ok(Self {
            start,
            end,
            colors: &bytes[2..needed],
        })
    }

    pub fn start(&self) -> usize {
        self.start
    }

    fn entry(&self, index: usize) -> [Color; NUMBER_OF_PLUTVALS] {
        let entry = &self.colors[(index - self.start) * ENTRY_BYTES..][..ENTRY_BYTES];
        std::array::from_fn(|i| Color::rgb(entry[i * 3], entry[i * 3 + 1], entry[i * 3 + 2]))
    }

    fn bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.start as u8, self.end as u8];
        bytes.extend_from_slice(self.colors);
        bytes
    }
}

#[derive(Clone, Copy)]
struct Fade {
    amount: i32,
    delta: i32,
    start: u32,
    interval: i32,
}

impl Fade {
    fn finish(&mut self) {
        if self.interval != 0 {
            self.amount += self.delta;
            self.interval = 0;
        }
    }

    fn amount_at(&mut self, now: u32) -> i32 {
        if self.interval == 0 {
            return self.amount;
        }
        let elapsed = (now.wrapping_sub(self.start) as i32).min(self.interval);
        let amount = self.amount
            + (i64::from(self.delta) * i64::from(elapsed) / i64::from(self.interval)) as i32;
        if elapsed >= self.interval {
            self.amount = amount;
            self.interval = 0;
        }
        amount
    }
}

struct Xform {
    index: i32,
    table: Vec<u8>,
    ticks: i16,
    end: u32,
    old: [Color; NUMBER_OF_PLUTVALS],
}

impl Xform {
    const EMPTY: Self = Self {
        index: -1,
        table: Vec::new(),
        ticks: 0,
        end: 0,
        old: [Color::rgba(0, 0, 0, 0); NUMBER_OF_PLUTVALS],
    };
}

/// Installed colour maps, the screen fade and running transforms.
pub(crate) struct ColorMaps {
    maps: Vec<Option<Arc<ColorMap>>>,
    fade: Fade,
    xforms: [Xform; MAX_XFORMS],
    highest: i32,
}

fn blend(from: u8, to: u8, weight: i32) -> u8 {
    (i32::from(from) + (i32::from(to) - i32::from(from)) * weight / 65536) as u8
}

fn fade_target(kind: u32) -> Option<i32> {
    match kind {
        FADE_ALL_TO_BLACK | 254 => Some(FADE_NO_INTENSITY),
        FADE_ALL_TO_COLOR | 255 => Some(FADE_NORMAL_INTENSITY),
        FADE_ALL_TO_WHITE | 251 => Some(FADE_FULL_INTENSITY),
        _ => None,
    }
}

impl ColorMaps {
    pub const fn new() -> Self {
        Self {
            maps: Vec::new(),
            fade: Fade {
                amount: FADE_NORMAL_INTENSITY,
                delta: 0,
                start: 0,
                interval: 0,
            },
            xforms: [const { Xform::EMPTY }; MAX_XFORMS],
            highest: -1,
        }
    }

    /// C: `InitColorMaps`.
    pub fn init(&mut self) {
        self.highest = -1;
        for xform in &mut self.xforms {
            xform.index = -1;
        }
    }

    /// C: `UninitColorMaps`.
    pub fn uninit(&mut self) {
        self.maps.clear();
    }

    fn slot(&mut self, index: usize) -> &mut Option<Arc<ColorMap>> {
        if self.maps.len() < MAX_COLORMAPS {
            self.maps.resize(MAX_COLORMAPS, None);
        }
        &mut self.maps[index]
    }

    fn install(&mut self, index: usize, palette: [Color; NUMBER_OF_PLUTVALS]) {
        let slot = self.slot(index);
        let version = slot.as_ref().map_or(1, |old| old.version.wrapping_add(1));
        *slot = Some(Arc::new(ColorMap {
            index: index as i32,
            version,
            palette,
        }));
    }

    /// C: `SetColorMap`.
    pub fn set(&mut self, table: ColorMapTable<'_>) {
        for index in table.start..=table.end {
            self.install(index, table.entry(index));
        }
    }

    /// C: `TFB_GetColorMap`: the current snapshot of map `index`.
    pub fn get(&self, index: i32) -> Arc<ColorMap> {
        self.maps
            .get(index as usize)
            .and_then(Option::clone)
            .unwrap_or_else(|| panic!("BUG: get_colormap(): map {index} not present"))
    }

    /// C: `GetFadeAmount`.
    pub fn fade_amount(&mut self, now: u32) -> i32 {
        self.fade.amount_at(now)
    }

    /// C: `FadeScreen`; returns the tick the fade ends.
    pub fn fade_screen(
        &mut self,
        kind: u32,
        mut interval: i16,
        now: u32,
        quit_posted: bool,
    ) -> u32 {
        let Some(target) = fade_target(kind) else {
            return now;
        };
        if quit_posted {
            interval = 0;
        }
        self.fade.finish();
        if interval <= 0 {
            self.fade.amount = target;
            self.fade.interval = 0;
            now
        } else {
            self.fade.interval = i32::from(interval);
            self.fade.delta = target - self.fade.amount;
            self.fade.start = now;
            now.wrapping_add(interval as u32).wrapping_add(1)
        }
    }

    fn finish_xform(&mut self, which: usize) {
        let table = std::mem::take(&mut self.xforms[which].table);
        let table = ColorMapTable::parse(&table)
            .unwrap_or_else(|error| panic!("validated transform table: {error}"));
        self.set(table);
        self.xforms[which].index = -1;
        if which as i32 == self.highest {
            let mut x = which as i32 - 1;
            while x >= 0 && self.xforms[x as usize].index == -1 {
                x -= 1;
            }
            self.highest = x;
        }
    }

    /// C: `XFormColorMap_step`: advance every running transform.
    pub fn step(&mut self, now: u32) -> bool {
        let mut changed = false;
        let mut x = 0;
        while x as i32 <= self.highest {
            if self.xforms[x].index < 0 {
                x += 1;
                continue;
            }
            let index = self.xforms[x].index as usize;
            let left = self.xforms[x].end.wrapping_sub(now) as i32;
            if self.maps.get(index).and_then(Option::as_ref).is_none() {
                log::error!("BUG: XFormColorMap_step(): no current map");
                self.finish_xform(x);
                x += 1;
                continue;
            }
            if left > 0 {
                let control = &self.xforms[x];
                let ticks = i32::from(control.ticks);
                let fraction = (ticks - left) * 65536 / ticks;
                let target = &control.table[2..];
                let palette = std::array::from_fn(|i| {
                    let old = control.old[i];
                    Color::rgb(
                        blend(old.r, target[i * 3], fraction),
                        blend(old.g, target[i * 3 + 1], fraction),
                        blend(old.b, target[i * 3 + 2], fraction),
                    )
                });
                self.install(index, palette);
            } else {
                self.finish_xform(x);
            }
            changed = true;
            x += 1;
        }
        changed
    }

    /// C: `XFormColorMap`: start moving map `table.start()` towards the
    /// table over `interval` ticks; returns the end tick.
    pub fn xform(
        &mut self,
        table: ColorMapTable<'_>,
        mut interval: i16,
        now: u32,
        quit_posted: bool,
    ) -> u32 {
        if quit_posted {
            interval = 0;
        }
        let index = table.start as i32;
        let mut x = 0;
        let mut available = None;
        while x as i32 <= self.highest && index != self.xforms[x].index {
            if available.is_none() && self.xforms[x].index == -1 {
                available = Some(x);
            }
            x += 1;
        }
        if x < MAX_XFORMS && index == self.xforms[x].index {
            self.finish_xform(x);
        } else if let Some(first) = available {
            x = first;
        } else if x >= MAX_XFORMS {
            x = self.highest as usize;
            self.finish_xform(x);
        }
        self.highest = self.highest.max(x as i32);
        let Some(current) = self.maps.get(table.start).and_then(Option::as_ref) else {
            log::warn!("BUG: XFormPLUT(): no current map");
            return 0;
        };
        let old = current.palette;
        let control = &mut self.xforms[x];
        control.old = old;
        control.index = index;
        control.table = table.bytes();
        control.ticks = interval.max(0);
        control.end = now.wrapping_add(control.ticks as u32);
        control.end
    }

    /// C: `FlushColorXForms`.
    pub fn flush_xforms(&mut self) {
        self.fade.finish();
        let mut i = 0;
        while i as i32 <= self.highest {
            if self.xforms[i].index >= 0 {
                self.finish_xform(i);
            }
            i += 1;
        }
        self.highest = -1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(start: u8, end: u8, fill: &[u8]) -> Vec<u8> {
        let mut bytes = vec![start, end];
        for &value in fill {
            bytes.extend(std::iter::repeat_n(value, ENTRY_BYTES));
        }
        bytes
    }

    #[test]
    fn fade_signed_division_and_timer_wrap_match_c() {
        for kind in [250, 251] {
            assert_eq!(fade_target(kind), Some(510));
        }
        for kind in [252, 254] {
            assert_eq!(fade_target(kind), Some(0));
        }
        for kind in [253, 255] {
            assert_eq!(fade_target(kind), Some(255));
        }
        assert_eq!(fade_target(0), None);
        let mut fade = Fade {
            amount: 255,
            delta: -255,
            start: u32::MAX - 2,
            interval: 10,
        };
        assert_eq!(fade.amount_at(0), 179);
        assert_eq!(fade.amount_at(7), 0);
        assert_eq!(fade.interval, 0);
        assert_eq!(blend(255, 0, 32768), 128);
        assert_eq!(blend(0, 255, 32768), 127);
    }

    #[test]
    fn transform_interpolates_then_flush_installs_the_full_range() {
        let mut maps = ColorMaps::new();
        maps.init();
        maps.set(ColorMapTable::parse(&table(17, 18, &[0, 0])).unwrap());
        let target = table(17, 18, &[80, 160]);
        let end = maps.xform(ColorMapTable::parse(&target).unwrap(), 10, 100, false);
        assert_eq!(end, 110);
        drop(target);
        assert!(maps.step(105));
        assert_eq!(maps.get(17).palette[0].r, 40);
        maps.flush_xforms();
        assert_eq!(maps.get(17).palette[255].r, 80);
        assert_eq!(maps.get(18).palette[255].r, 160);
        assert_eq!(maps.highest, -1);
    }

    #[test]
    fn replacement_keeps_held_snapshots_and_increments_versions() {
        let mut maps = ColorMaps::new();
        let mut bytes = table(17, 17, &[90]);
        maps.set(ColorMapTable::parse(&bytes).unwrap());
        let held = maps.get(17);
        bytes[2] = 12;
        maps.set(ColorMapTable::parse(&bytes).unwrap());
        let current = maps.get(17);
        assert_eq!((held.version, current.version), (1, 2));
        assert_eq!((held.palette[0].r, current.palette[0].r), (90, 12));
    }

    /// #939: the header is validated against the table length before any map
    /// is read or installed.
    #[test]
    fn malformed_tables_are_rejected_before_any_map_changes() {
        assert_eq!(
            ColorMapTable::parse(&[]).unwrap_err(),
            ColorMapError::MissingHeader(0)
        );
        assert_eq!(
            ColorMapTable::parse(&[3]).unwrap_err(),
            ColorMapError::MissingHeader(1)
        );
        assert!(matches!(
            ColorMapTable::parse(&table(5, 4, &[0, 0])),
            Err(ColorMapError::InvalidRange { start: 5, end: 4 })
        ));
        assert!(matches!(
            ColorMapTable::parse(&table(249, 250, &[0, 0])),
            Err(ColorMapError::InvalidRange { .. })
        ));
        let mut short = table(0, 135, &[7; 136]);
        short.truncate(38_914);
        assert_eq!(
            ColorMapTable::parse(&short).unwrap_err(),
            ColorMapError::Truncated {
                start: 0,
                end: 135,
                needed: 104_450,
                actual: 38_914
            }
        );
        let full = table(0, 135, &[7; 136]);
        assert_eq!(full.len(), 104_450);
        let mut maps = ColorMaps::new();
        maps.set(ColorMapTable::parse(&full).unwrap());
        assert!((0..136).all(|i| maps
            .get(i)
            .palette
            .iter()
            .all(|c| *c == Color::rgb(7, 7, 7))));
    }
}
