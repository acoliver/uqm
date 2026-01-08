//! Color map (cmap) system implementation.
//!
//! This module provides the color palette management system, including:
//! - ColorMapManager: Manages all color maps with indexing and pooling
//! - ColorMap: Individual color map with refcounting and versioning
//! - NativePalette: Low-level palette storage
//! - Fade transitions: Screen fading effects (to black, color, white)
//! - Colormap transformations: Smooth palette transitions

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const MAX_COLORMAPS: usize = 250;
pub const NUMBER_OF_PLUTVALS: usize = 256;
pub const PLUTVAL_BYTE_SIZE: usize = 3;
pub const FADE_NO_INTENSITY: i32 = 0;
pub const FADE_NORMAL_INTENSITY: i32 = 255;
pub const FADE_FULL_INTENSITY: i32 = 510;
pub const SPARE_COLORMAPS: usize = 20;
pub const MAX_XFORMS: usize = 16;
pub const XFORM_SCALE: i32 = 0x10000;
pub const PLUTVAL_RED: usize = 0;
pub const PLUTVAL_GREEN: usize = 1;
pub const PLUTVAL_BLUE: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn to_rgba(self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
    pub const fn is_opaque(self) -> bool {
        self.a == 255
    }
    pub const fn is_transparent(self) -> bool {
        self.a == 0
    }
}

#[derive(Debug, Clone)]
pub struct NativePalette {
    colors: Vec<Color>,
}

impl NativePalette {
    pub fn new() -> Self {
        Self {
            colors: vec![Color::new(0, 0, 0); NUMBER_OF_PLUTVALS],
        }
    }
    pub fn get(&self, index: usize) -> Color {
        self.colors[index]
    }
    pub fn set(&mut self, index: usize, color: Color) {
        self.colors[index] = color;
    }
    pub fn colors(&self) -> &[Color] {
        &self.colors
    }
    pub fn colors_mut(&mut self) -> &mut [Color] {
        &mut self.colors
    }
    pub fn copy_from(&mut self, other: &NativePalette) {
        self.colors.copy_from_slice(&other.colors);
    }
}

impl Default for NativePalette {
    fn default() -> Self {
        Self::new()
    }
}

pub type ColorMapRef = Arc<ColorMapInner>;

pub struct ColorMapInner {
    index: i16,
    version: Mutex<u32>,
    refcount: Mutex<u32>,
    palette: Mutex<NativePalette>,
}

impl ColorMapInner {
    pub fn new(index: i16) -> Self {
        Self {
            index,
            version: Mutex::new(0),
            refcount: Mutex::new(1),
            palette: Mutex::new(NativePalette::new()),
        }
    }
    pub fn index(&self) -> i16 {
        self.index
    }
    pub fn version(&self) -> u32 {
        *self.version.lock().unwrap()
    }
    fn increment_version(&self) {
        let mut version = self.version.lock().unwrap();
        *version = version.wrapping_add(1);
    }
    fn replace_palette(&self, colors: &[Color]) {
        let mut palette = self.palette.lock().unwrap();
        palette.colors_mut().copy_from_slice(colors);
        self.increment_version();
    }
    pub fn refcount(&self) -> u32 {
        *self.refcount.lock().unwrap()
    }
    fn add_ref(&self) {
        *self.refcount.lock().unwrap() += 1;
    }
    fn release(&self) -> bool {
        let mut count = self.refcount.lock().unwrap();
        if *count == 0 {
            panic!("release() called on ColorMap with refcount == 0");
        }
        *count -= 1;
        *count == 0
    }
    pub fn get_color(&self, index: usize) -> Color {
        self.palette.lock().unwrap().get(index)
    }
    pub fn get_colors(&self) -> Vec<Color> {
        self.palette.lock().unwrap().colors().to_vec()
    }
    pub fn set_color(&self, index: usize, color: Color) {
        self.palette.lock().unwrap().set(index, color);
        self.increment_version();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FadeType {
    FadeToBlack,
    FadeToColor,
    FadeToWhite,
}

#[derive(Debug)]
struct FadeState {
    target: i32,
    current: i32,
    delta: i32,
    interval_ms: u64,
    start_time: Instant,
    active: bool,
}

impl FadeState {
    fn start(&mut self, fade_type: FadeType, duration_ms: u64) {
        self.target = match fade_type {
            FadeType::FadeToBlack => FADE_NO_INTENSITY,
            FadeType::FadeToColor => FADE_NORMAL_INTENSITY,
            FadeType::FadeToWhite => FADE_FULL_INTENSITY,
        };
        if duration_ms == 0 {
            self.current = self.target;
            self.active = false;
        } else {
            self.delta = self.target - self.current;
            self.interval_ms = duration_ms;
            self.start_time = Instant::now();
            self.active = true;
        }
    }
    fn finish(&mut self) {
        if self.active {
            self.current += self.delta;
            self.active = false;
        }
    }
    pub fn current_amount(&self) -> i32 {
        if !self.active {
            return self.current;
        }
        let elapsed = self.start_time.elapsed().as_millis() as u64;
        if elapsed >= self.interval_ms {
            return self.target;
        }
        let progress = elapsed as i64 * XFORM_SCALE as i64 / self.interval_ms as i64;
        let change = self.delta as i64 * progress / XFORM_SCALE as i64;
        (self.current as i64 + change) as i32
    }
}

#[derive(Debug, Clone)]
struct XformControl {
    cmap_index: i32,
    target_colors: Vec<Color>,
    old_colors: Vec<Color>,
    duration_ms: u64,
    start_time: Instant,
    end_time: Instant,
}

impl XformControl {
    fn in_use(&self) -> bool {
        self.cmap_index >= 0
    }
    #[allow(dead_code)]
    fn is_complete(&self) -> bool {
        Instant::now() >= self.end_time
    }
}

pub struct ColorMapManager {
    colormaps: Vec<Option<ColorMapRef>>,
    spare_head: Mutex<Vec<ColorMapRef>>,
    map_count: Mutex<usize>,
    fade_state: Mutex<FadeState>,
    xforms: Mutex<Vec<XformControl>>,
    highest_xform: Mutex<usize>,
}

impl ColorMapManager {
    pub fn new() -> Self {
        let mut xforms = Vec::with_capacity(MAX_XFORMS);
        xforms.resize_with(MAX_XFORMS, XformControl::default);
        Self {
            colormaps: vec![None; MAX_COLORMAPS],
            spare_head: Mutex::new(Vec::with_capacity(SPARE_COLORMAPS)),
            map_count: Mutex::new(0),
            fade_state: Mutex::new(FadeState::default()),
            xforms: Mutex::new(xforms),
            highest_xform: Mutex::new(0),
        }
    }
    pub fn init(&mut self) {}
    pub fn uninit(&mut self) {
        for cmap in self.colormaps.iter_mut() {
            *cmap = None;
        }
        self.spare_head.lock().unwrap().clear();
        *self.map_count.lock().unwrap() = 0;
        for xform in self.xforms.lock().unwrap().iter_mut() {
            xform.cmap_index = -1;
        }
        *self.highest_xform.lock().unwrap() = 0;
    }
    pub fn get_colormap(&self, index: i32) -> Option<ColorMapRef> {
        if index < 0 || index >= MAX_COLORMAPS as i32 {
            return None;
        }
        let cmap = self.colormaps[index as usize].as_ref()?;
        cmap.add_ref();
        Some(cmap.clone())
    }
    pub fn return_colormap(&self, cmap: &ColorMapRef) {
        if cmap.release() {
            let mut pool = self.spare_head.lock().unwrap();
            if pool.len() < SPARE_COLORMAPS {
                pool.push(Arc::new(ColorMapInner::new(cmap.index())));
            }
        }
    }
    pub fn set_colors(&mut self, index: i32, end_index: i32, colors: &[u8]) -> Result<(), String> {
        if index > end_index {
            return Err(format!("start {} > end {}", index, end_index));
        }
        if index < 0 || index >= MAX_COLORMAPS as i32 || end_index >= MAX_COLORMAPS as i32 {
            return Err("index out of range".to_string());
        }
        let num_maps = (end_index - index + 1) as usize;
        let expected_size = num_maps * NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE;
        if colors.len() != expected_size {
            return Err(format!(
                "size mismatch: expected {}, got {}",
                expected_size,
                colors.len()
            ));
        }
        {
            let mut count = self.map_count.lock().unwrap();
            *count = (*count).max(end_index as usize + 1);
        }
        for i in 0..=end_index - index {
            let map_idx = (index + i) as usize;
            let base_offset = i as usize * NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE;
            let new_cmap = self.allocate_colormap(map_idx as i16);
            let mut palette = vec![Color::new(0, 0, 0); NUMBER_OF_PLUTVALS];
            for (j, color) in palette.iter_mut().enumerate().take(NUMBER_OF_PLUTVALS) {
                let offset = base_offset + j * PLUTVAL_BYTE_SIZE;
                if offset + PLUTVAL_BYTE_SIZE <= colors.len() {
                    *color = Color::new(
                        colors[offset + PLUTVAL_RED],
                        colors[offset + PLUTVAL_GREEN],
                        colors[offset + PLUTVAL_BLUE],
                    );
                }
            }
            match Arc::try_unwrap(new_cmap) {
                Ok(mut cmap) => {
                    cmap.index = map_idx as i16;
                    cmap.replace_palette(&palette);
                    self.colormaps[map_idx] = Some(Arc::new(cmap));
                }
                Err(arc) => {
                    arc.replace_palette(&palette);
                    self.colormaps[map_idx] = Some(arc);
                }
            }
        }
        Ok(())
    }
    fn allocate_colormap(&self, index: i16) -> ColorMapRef {
        let mut pool = self.spare_head.lock().unwrap();
        if let Some(cmap) = pool.pop() {
            let cmap = Arc::try_unwrap(cmap).unwrap_or_else(|_| ColorMapInner::new(index));
            Arc::new(cmap)
        } else {
            Arc::new(ColorMapInner::new(index))
        }
    }
    pub fn get_colormap_colors(cmap: &ColorMapRef) -> Vec<Color> {
        cmap.get_colors()
    }

    pub fn get_fade_amount(&self) -> i32 {
        self.fade_state.lock().unwrap().current_amount()
    }

    pub fn fade_screen(&self, fade_type: FadeType, duration_ms: u64) -> Instant {
        let mut fade = self.fade_state.lock().unwrap();
        fade.finish();
        fade.start(fade_type, duration_ms);
        if duration_ms > 0 {
            fade.start_time + Duration::from_millis(duration_ms)
        } else {
            Instant::now()
        }
    }
    pub fn finish_fade(&self) {
        self.fade_state.lock().unwrap().finish();
    }
    pub fn transform_colormap(
        &self,
        index: i32,
        target_colors: &[Color],
        duration_ms: u64,
    ) -> Result<Instant, String> {
        if index < 0 || index >= MAX_COLORMAPS as i32 {
            return Err("index out of range".to_string());
        }
        if target_colors.len() != NUMBER_OF_PLUTVALS {
            return Err("target_colors wrong length".to_string());
        }
        let cmap = self.colormaps[index as usize]
            .as_ref()
            .ok_or("colormap not found")?;
        let old_colors = cmap.get_colors();
        let now = Instant::now();
        let mut xforms = self.xforms.lock().unwrap();
        let mut slot = None;
        for i in 0..MAX_XFORMS {
            if !xforms[i].in_use() || xforms[i].cmap_index == index {
                slot = Some(i);
                break;
            }
        }
        let slot = slot.ok_or("no available xform slots")?;
        xforms[slot] = XformControl {
            cmap_index: index,
            target_colors: target_colors.to_vec(),
            old_colors,
            duration_ms,

            start_time: now,
            end_time: now + Duration::from_millis(duration_ms),
        };
        let mut highest = self.highest_xform.lock().unwrap();
        *highest = (*highest).max(slot + 1);
        Ok(now + Duration::from_millis(duration_ms))
    }
    pub fn step_transformations(&mut self) -> bool {
        self.fade_state.lock().unwrap().current_amount();
        let mut has_active = false;
        let highest = *self.highest_xform.lock().unwrap();
        if highest == 0 {
            return false;
        }
        let now = Instant::now();
        let mut pending_applies = Vec::new();
        let mut pending_finishes = Vec::new();
        {
            let mut xforms = self.xforms.lock().unwrap();
            for (idx, xform) in xforms.iter_mut().take(highest).enumerate() {
                if !xform.in_use() {
                    continue;
                }
                if now >= xform.end_time {
                    pending_finishes.push(idx);
                    xform.cmap_index = -1;
                    continue;
                }
                pending_applies.push(idx);
                has_active = true;
            }
        }
        for idx in pending_applies {
            let xform = self.xforms.lock().unwrap()[idx].clone();
            self.apply_blended_colors(&xform, now);
        }
        for idx in pending_finishes {
            let xform = self.xforms.lock().unwrap()[idx].clone();
            self.apply_target_colors(&xform);
        }
        if !has_active {
            let mut highest_ref = self.highest_xform.lock().unwrap();
            let mut new_highest = 0;
            let xforms = self.xforms.lock().unwrap();
            for (idx, xform) in xforms.iter().enumerate() {
                if xform.in_use() {
                    new_highest = idx + 1;
                }
            }
            *highest_ref = new_highest;
        }
        has_active
    }
    pub fn finish_transformations(&self) {
        let mut pending = Vec::new();
        {
            let mut xforms = self.xforms.lock().unwrap();
            for (idx, xform) in xforms.iter_mut().enumerate() {
                if xform.in_use() {
                    pending.push(idx);
                    xform.cmap_index = -1;
                }
            }
        }
        for idx in pending {
            let xform = self.xforms.lock().unwrap()[idx].clone();
            self.apply_target_colors(&xform);
        }
        *self.highest_xform.lock().unwrap() = 0;
    }
    pub fn flush_color_xforms(&self) {
        self.finish_fade();
        self.finish_transformations();
    }

    fn apply_blended_colors(&self, xform: &XformControl, now: Instant) {
        let cmap = match self.colormaps.get(xform.cmap_index as usize) {
            Some(Some(cmap)) => cmap,
            _ => return,
        };
        let elapsed = now.saturating_duration_since(xform.start_time);
        let duration = Duration::from_millis(xform.duration_ms).max(Duration::from_millis(1));
        let progress =
            elapsed.as_millis() as i64 * XFORM_SCALE as i64 / duration.as_millis() as i64;
        let progress = progress.min(XFORM_SCALE as i64);
        let mut palette = cmap.palette.lock().unwrap();
        for (idx, (old, target)) in xform
            .old_colors
            .iter()
            .zip(xform.target_colors.iter())
            .enumerate()
        {
            let blend = |from: u8, to: u8| {
                let delta = to as i64 - from as i64;
                let change = delta * progress / XFORM_SCALE as i64;
                (from as i64 + change) as u8
            };
            palette.set(
                idx,
                Color::with_alpha(
                    blend(old.r, target.r),
                    blend(old.g, target.g),
                    blend(old.b, target.b),
                    blend(old.a, target.a),
                ),
            );
        }
        cmap.increment_version();
    }

    fn apply_target_colors(&self, xform: &XformControl) {
        let cmap = match self.colormaps.get(xform.cmap_index as usize) {
            Some(Some(cmap)) => cmap,
            _ => return,
        };
        let mut palette = cmap.palette.lock().unwrap();
        for (idx, color) in xform.target_colors.iter().enumerate() {
            palette.set(idx, *color);
        }
        cmap.increment_version();
    }

    pub fn map_count(&self) -> usize {
        *self.map_count.lock().unwrap()
    }
}

impl Default for ColorMapManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for FadeState {
    fn default() -> Self {
        Self {
            target: FADE_NORMAL_INTENSITY,
            current: FADE_NORMAL_INTENSITY,
            delta: 0,
            interval_ms: 0,
            start_time: Instant::now(),
            active: false,
        }
    }
}

impl Default for XformControl {
    fn default() -> Self {
        Self {
            cmap_index: -1,
            target_colors: Vec::new(),
            old_colors: Vec::new(),
            duration_ms: 0,
            start_time: Instant::now(),
            end_time: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_palette() -> Vec<Color> {
        (0..NUMBER_OF_PLUTVALS)
            .map(|i| Color::new((i * 255 / NUMBER_OF_PLUTVALS) as u8, 0, 0))
            .collect()
    }

    #[test]
    fn test_color_new() {
        let c = Color::new(255, 128, 64);
        assert_eq!((c.r, c.g, c.b, c.a), (255, 128, 64, 255));
    }

    #[test]
    fn test_color_with_alpha() {
        let c = Color::with_alpha(255, 128, 64, 128);
        assert_eq!(c.a, 128);
    }

    #[test]
    fn test_color_is_opaque() {
        assert!(Color::new(255, 0, 0).is_opaque());
        assert!(!Color::with_alpha(255, 0, 0, 128).is_opaque());
    }

    #[test]
    fn test_color_is_transparent() {
        assert!(Color::with_alpha(255, 0, 0, 0).is_transparent());
        assert!(!Color::new(255, 0, 0).is_transparent());
    }

    #[test]
    fn test_native_palette_new() {
        let p = NativePalette::new();
        assert_eq!(p.colors.len(), NUMBER_OF_PLUTVALS);
    }

    #[test]
    fn test_native_palette_set_get() {
        let mut p = NativePalette::new();
        p.set(10, Color::new(255, 128, 64));
        assert_eq!(p.get(10), Color::new(255, 128, 64));
    }

    #[test]
    fn test_native_palette_copy_from() {
        let mut p1 = NativePalette::new();
        let mut p2 = NativePalette::new();
        p1.set(5, Color::new(255, 0, 0));
        p2.copy_from(&p1);
        assert_eq!(p2.get(5), Color::new(255, 0, 0));
    }

    #[test]
    fn test_colormap_manager_new() {
        let mgr = ColorMapManager::new();
        assert_eq!(mgr.map_count(), 0);
        assert_eq!(mgr.get_fade_amount(), FADE_NORMAL_INTENSITY);
    }

    #[test]
    fn test_colormap_manager_init_uninit() {
        let mut mgr = ColorMapManager::new();
        mgr.init();
        mgr.uninit();
    }

    #[test]
    fn test_colormap_manager_get_none() {
        let mgr = ColorMapManager::new();
        assert!(mgr.get_colormap(0).is_none());
    }

    #[test]
    fn test_colormap_manager_set_colors_basic() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        assert!(mgr.set_colors(0, 0, &colors).is_ok());
        assert_eq!(mgr.map_count(), 1);
    }

    #[test]
    fn test_colormap_manager_set_multiple() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; 3 * NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        assert!(mgr.set_colors(0, 2, &colors).is_ok());
        assert_eq!(mgr.map_count(), 3);
    }

    #[test]
    fn test_colormap_manager_set_invalid_range() {
        let mut mgr = ColorMapManager::new();
        assert!(mgr.set_colors(5, 2, &[]).is_err());
    }

    #[test]
    fn test_colormap_manager_set_out_of_bounds() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        assert!(mgr
            .set_colors(MAX_COLORMAPS as i32, MAX_COLORMAPS as i32, &colors)
            .is_err());
    }

    #[test]
    fn test_colormap_manager_set_wrong_size() {
        let mut mgr = ColorMapManager::new();
        assert!(mgr.set_colors(0, 0, &vec![0u8; 100]).is_err());
    }

    #[test]
    fn test_colormap_refcount() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        mgr.set_colors(0, 0, &colors).unwrap();
        let cmap1 = mgr.get_colormap(0).unwrap();
        let cmap2 = mgr.get_colormap(0).unwrap();
        assert_eq!(Arc::strong_count(&cmap1), 3);
    }

    #[test]
    fn test_fade_state_start_immediate() {
        let mut fade = FadeState::default();
        fade.start(FadeType::FadeToBlack, 0);
        assert!(!fade.active);
        assert_eq!(fade.current, FADE_NO_INTENSITY);
    }

    #[test]
    fn test_fade_state_start_timed() {
        let mut fade = FadeState::default();
        fade.start(FadeType::FadeToWhite, 100);
        assert!(fade.active);
        assert_eq!(fade.target, FADE_FULL_INTENSITY);
    }

    #[test]
    fn test_fade_state_finish() {
        let mut fade = FadeState::default();
        fade.start(FadeType::FadeToBlack, 100);
        fade.finish();
        assert!(!fade.active);
    }

    #[test]
    fn test_fade_screen_immediate() {
        let mgr = ColorMapManager::new();
        mgr.fade_screen(FadeType::FadeToBlack, 0);
        assert_eq!(mgr.get_fade_amount(), FADE_NO_INTENSITY);
    }

    #[test]
    fn test_fade_screen_timed() {
        let mgr = ColorMapManager::new();
        mgr.fade_screen(FadeType::FadeToWhite, 100);
        assert!(mgr.fade_state.lock().unwrap().active);
    }

    #[test]
    fn test_finish_fade() {
        let mgr = ColorMapManager::new();
        mgr.fade_screen(FadeType::FadeToBlack, 1000);
        mgr.finish_fade();
        assert!(!mgr.fade_state.lock().unwrap().active);
    }

    #[test]
    fn test_transform_colormap() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        mgr.set_colors(0, 0, &colors).unwrap();
        let target = create_test_palette();
        assert!(mgr.transform_colormap(0, &target, 100).is_ok());
    }

    #[test]
    fn test_transform_colormap_invalid_index() {
        let mgr = ColorMapManager::new();
        let target = create_test_palette();
        assert!(mgr.transform_colormap(-1, &target, 100).is_err());
        assert!(mgr
            .transform_colormap(MAX_COLORMAPS as i32, &target, 100)
            .is_err());
    }

    #[test]
    fn test_transform_colormap_invalid_size() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        mgr.set_colors(0, 0, &colors).unwrap();
        assert!(mgr
            .transform_colormap(0, &vec![Color::new(0, 0, 0); 100], 100)
            .is_err());
    }

    #[test]
    fn test_transform_colormap_missing() {
        let mgr = ColorMapManager::new();
        let target = create_test_palette();
        assert!(mgr.transform_colormap(0, &target, 100).is_err());
    }

    #[test]
    fn test_transform_colormap_immediate() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        mgr.set_colors(0, 0, &colors).unwrap();
        let target = create_test_palette();
        assert!(mgr.transform_colormap(0, &target, 0).is_ok());
    }

    #[test]
    fn test_finish_transformations() {
        let mut mgr = ColorMapManager::new();
        let colors = vec![0u8; NUMBER_OF_PLUTVALS * PLUTVAL_BYTE_SIZE];
        mgr.set_colors(0, 0, &colors).unwrap();
        let target = create_test_palette();
        mgr.transform_colormap(0, &target, 1000).unwrap();
        mgr.finish_transformations();
        assert!(!mgr.xforms.lock().unwrap().iter().any(|x| x.in_use()));
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_COLORMAPS, 250);
        assert_eq!(NUMBER_OF_PLUTVALS, 256);
        assert_eq!(FADE_NO_INTENSITY, 0);
        assert_eq!(FADE_NORMAL_INTENSITY, 255);
        assert_eq!(FADE_FULL_INTENSITY, 510);
    }

    #[test]
    fn test_step_transformations_no_active() {
        let mut mgr = ColorMapManager::new();
        assert!(!mgr.step_transformations());
    }
}
