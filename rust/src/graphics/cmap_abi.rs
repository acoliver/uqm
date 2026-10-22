//! Engine colormap ownership and tick-based transforms, ported from cmap.c.
//! GPL-2.0-or-later. ABI palettes remain snapshots while queued draws hold refs.
use super::cmap::{FADE_FULL_INTENSITY, FADE_NORMAL_INTENSITY, MAX_COLORMAPS, NUMBER_OF_PLUTVALS};
use super::sdl::palette::{
    AllocNativePalette, Color, FreeNativePalette, GetNativePaletteColor, NativePalette,
    SetNativePaletteColor,
};
use crate::memory::{rust_hfree, rust_hmalloc};
use crate::time::counter_abi::GetTimeCounter;
use parking_lot::Mutex;
use std::ptr;

#[repr(C)]
pub struct ColorMap {
    pub index: i32,
    pub version: i32,
    pub refcount: i32,
    pub next: *mut ColorMap,
    pub palette: *mut NativePalette,
}
struct Maps {
    maps: [*mut ColorMap; MAX_COLORMAPS],
    pool: *mut ColorMap,
    pool_count: usize,
}
// All access, including reference-count changes, is serialized by MAPS.
unsafe impl Send for Maps {}
static MAPS: Mutex<Maps> = Mutex::new(Maps {
    maps: [ptr::null_mut(); MAX_COLORMAPS],
    pool: ptr::null_mut(),
    pool_count: 0,
});
impl Maps {
    unsafe fn alloc(&mut self, from: *mut ColorMap, index: i32) -> *mut ColorMap {
        let map = if !self.pool.is_null() {
            let m = self.pool;
            self.pool = (*m).next;
            self.pool_count -= 1;
            m
        } else {
            let m: *mut ColorMap = rust_hmalloc(std::mem::size_of::<ColorMap>()).cast();
            (*m).palette = AllocNativePalette();
            m
        };
        (*map).next = ptr::null_mut();
        (*map).index = index;
        (*map).refcount = 1;
        (*map).version = if from.is_null() {
            1
        } else {
            (*from).version.wrapping_add(1)
        };
        map
    }
    unsafe fn release(&mut self, map: *mut ColorMap) {
        if map.is_null() {
            return;
        }
        if (*map).refcount <= 0 {
            log::warn!("BUG: release_colormap(): refcount not >0");
            return;
        }
        (*map).refcount -= 1;
        if (*map).refcount != 0 {
            return;
        }
        if self.pool_count < 20 {
            (*map).next = self.pool;
            self.pool = map;
            self.pool_count += 1;
        } else {
            delete_map(map);
        }
    }
    unsafe fn set(&mut self, map: *const u8) -> u32 {
        if map.is_null() {
            return 1;
        }
        let start = usize::from(*map);
        let end = usize::from(*map.add(1));
        if start > end || start >= MAX_COLORMAPS {
            log::warn!("SetColorMap(): invalid map range {start}..{end}");
            return 0;
        }
        let end = end.min(MAX_COLORMAPS - 1);
        let mut colors = map.add(2);
        for index in start..=end {
            let old = self.maps[index];
            let new = self.alloc(old, index as i32);
            for i in 0..NUMBER_OF_PLUTVALS {
                SetNativePaletteColor(
                    (*new).palette,
                    i as i32,
                    Color {
                        r: *colors,
                        g: *colors.add(1),
                        b: *colors.add(2),
                        a: 255,
                    },
                );
                colors = colors.add(3);
            }
            self.maps[index] = new;
            self.release(old);
        }
        1
    }
}
unsafe fn delete_map(map: *mut ColorMap) {
    FreeNativePalette((*map).palette);
    rust_hfree(map.cast());
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
static FADE: Mutex<Fade> = Mutex::new(Fade {
    amount: FADE_NORMAL_INTENSITY,
    delta: 0,
    start: 0,
    interval: 0,
});
#[derive(Clone, Copy)]
struct Xform {
    index: i32,
    map: *const u8,
    ticks: i16,
    start: u32,
    end: u32,
    old: [Color; NUMBER_OF_PLUTVALS],
}
impl Xform {
    const EMPTY: Self = Self {
        index: -1,
        map: ptr::null(),
        ticks: 0,
        start: 0,
        end: 0,
        old: [Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }; NUMBER_OF_PLUTVALS],
    };
}
struct Xforms {
    slots: [Xform; 16],
    highest: i32,
}
// Target table pointers stay live for the duration of each transform per engine contract.
unsafe impl Send for Xforms {}
static XFORMS: Mutex<Xforms> = Mutex::new(Xforms {
    slots: [Xform::EMPTY; 16],
    highest: -1,
});
impl Xforms {
    unsafe fn finish(&mut self, which: usize) {
        MAPS.lock().set(self.slots[which].map);
        self.slots[which].index = -1;
        if which as i32 == self.highest {
            let mut x = which as i32 - 1;
            while x >= 0 && self.slots[x as usize].index == -1 {
                x -= 1;
            }
            self.highest = x;
        }
    }
    unsafe fn step(&mut self, now: u32) -> u32 {
        let mut changed = 0;
        let mut x = 0;
        while x as i32 <= self.highest {
            let control = self.slots[x];
            let left = control.end.wrapping_sub(now) as i32;
            if control.index < 0 {
                x += 1;
                continue;
            }
            let mut maps = MAPS.lock();
            let cur = maps.maps[control.index as usize];
            if cur.is_null() {
                drop(maps);
                log::error!("BUG: XFormColorMap_step(): no current map");
                self.finish(x);
                x += 1;
                continue;
            }
            if left > 0 {
                let new = maps.alloc(cur, control.index);
                let fraction = (i32::from(control.ticks) - left) * 65536 / i32::from(control.ticks);
                let mut clr = control.map.add(2);
                for i in 0..NUMBER_OF_PLUTVALS {
                    let old = control.old[i];
                    SetNativePaletteColor(
                        (*new).palette,
                        i as i32,
                        Color {
                            r: blend(old.r, *clr, fraction),
                            g: blend(old.g, *clr.add(1), fraction),
                            b: blend(old.b, *clr.add(2), fraction),
                            a: 255,
                        },
                    );
                    clr = clr.add(3);
                }
                maps.maps[control.index as usize] = new;
                maps.release(cur);
            }
            drop(maps);
            if left <= 0 {
                self.finish(x);
            }
            changed = 1;
            x += 1;
        }
        changed
    }
}
fn blend(from: u8, to: u8, weight: i32) -> u8 {
    (i32::from(from) + (i32::from(to) - i32::from(from)) * weight / 65536) as u8
}

#[no_mangle]
pub extern "C" fn InitColorMaps() {
    let mut xforms = XFORMS.lock();
    xforms.highest = -1;
    for slot in &mut xforms.slots {
        slot.index = -1;
    }
}
/// # Safety
/// No queued command may retain a colormap when the subsystem is shut down.
#[no_mangle]
pub unsafe extern "C" fn UninitColorMaps() {
    let mut maps = MAPS.lock();
    for i in 0..MAX_COLORMAPS {
        let map = maps.maps[i];
        if !map.is_null() {
            maps.release(map);
            maps.maps[i] = ptr::null_mut();
        }
    }
    while !maps.pool.is_null() {
        let next = (*maps.pool).next;
        delete_map(maps.pool);
        maps.pool = next;
        maps.pool_count -= 1;
    }
}
/// # Safety
/// Map must be null or a live reference acquired from TFB_GetColorMap.
#[no_mangle]
pub unsafe extern "C" fn TFB_ReturnColorMap(map: *mut ColorMap) {
    MAPS.lock().release(map);
}
/// # Safety
/// Index must identify a map previously installed with SetColorMap.
#[no_mangle]
pub unsafe extern "C" fn TFB_GetColorMap(index: i32) -> *mut ColorMap {
    let maps = MAPS.lock();
    let map = maps.maps[index as usize];
    assert!(!map.is_null(), "BUG: get_colormap(): map not present");
    (*map).refcount += 1;
    map
}
/// # Safety
/// Non-null map must be live and colors must hold 256 Color records.
#[no_mangle]
pub unsafe extern "C" fn GetColorMapColors(colors: *mut Color, map: *mut ColorMap) {
    if map.is_null() {
        return;
    }
    for i in 0..NUMBER_OF_PLUTVALS {
        *colors.add(i) = GetNativePaletteColor((*map).palette, i as i32);
    }
}
/// # Safety
/// Non-null table must contain start/end bytes followed by all RGB entries.
#[no_mangle]
pub unsafe extern "C" fn SetColorMap(map: *const u8) -> u32 {
    MAPS.lock().set(map)
}
#[no_mangle]
pub extern "C" fn GetFadeAmount() -> i32 {
    FADE.lock().amount_at(GetTimeCounter())
}
extern "C" {
    static QuitPosted: i32;
}
fn fade_target(kind: u32) -> Option<i32> {
    match kind {
        252 | 254 => Some(0),
        253 | 255 => Some(FADE_NORMAL_INTENSITY),
        250 | 251 => Some(FADE_FULL_INTENSITY),
        _ => None,
    }
}
/// # Safety
/// The engine owns QuitPosted; calls must obey engine lifecycle ordering.
#[no_mangle]
pub unsafe extern "C" fn FadeScreen(kind: u32, mut interval: i16) -> u32 {
    let Some(target) = fade_target(kind) else {
        return GetTimeCounter();
    };
    if QuitPosted != 0 {
        interval = 0;
    }
    let mut fade = FADE.lock();
    fade.finish();
    if interval <= 0 {
        fade.amount = target;
        fade.interval = 0;
        GetTimeCounter()
    } else {
        fade.interval = i32::from(interval);
        fade.delta = target - fade.amount;
        fade.start = GetTimeCounter();
        fade.start.wrapping_add(interval as u32).wrapping_add(1)
    }
}
/// # Safety
/// Tables associated with active transforms must remain live.
#[no_mangle]
pub unsafe extern "C" fn XFormColorMap_step() -> u32 {
    let now = GetTimeCounter();
    XFORMS.lock().step(now)
}
/// # Safety
/// Table must stay live until the transform is completed or flushed.
#[no_mangle]
pub unsafe extern "C" fn XFormColorMap(map: *const u8, mut interval: i16) -> u32 {
    if map.is_null() {
        return 0;
    }
    if QuitPosted != 0 {
        interval = 0;
    }
    let now = GetTimeCounter();
    let index = i32::from(*map);
    let mut xforms = XFORMS.lock();
    let mut x = 0;
    let mut available = None;
    while x as i32 <= xforms.highest && index != xforms.slots[x].index {
        if available.is_none() && xforms.slots[x].index == -1 {
            available = Some(x);
        }
        x += 1;
    }
    if x < 16 && index == xforms.slots[x].index {
        xforms.finish(x);
    } else if let Some(first) = available {
        x = first;
    } else if x >= 16 {
        x = xforms.highest as usize;
        xforms.finish(x);
    }
    xforms.highest = xforms.highest.max(x as i32);
    let maps = MAPS.lock();
    let current = maps.maps[index as usize];
    if current.is_null() {
        log::warn!("BUG: XFormPLUT(): no current map");
        return 0;
    }
    let control = &mut xforms.slots[x];
    GetColorMapColors(control.old.as_mut_ptr(), current);
    drop(maps);
    control.index = index;
    control.map = map;
    control.ticks = interval.max(0);
    control.start = now;
    control.end = now.wrapping_add(control.ticks as u32);
    control.end
}
/// # Safety
/// Tables associated with active transforms must remain live.
#[no_mangle]
pub unsafe extern "C" fn FlushColorXForms() {
    FADE.lock().finish();
    let mut xforms = XFORMS.lock();
    let mut i = 0;
    while i as i32 <= xforms.highest {
        if xforms.slots[i].index >= 0 {
            xforms.finish(i);
        }
        i += 1;
    }
    xforms.highest = -1;
}
/// # Safety
/// Colormap must be a valid string-table entry handle.
#[no_mangle]
pub unsafe extern "C" fn GetColorMapAddress(
    colormap: *mut crate::strings::string_table::StringTableEntry,
) -> *mut u8 {
    crate::strings::string_table::GetStringAddress(colormap).cast()
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn map_replacement_preserves_held_snapshot_and_increments_version() {
        unsafe {
            let mut maps = Maps {
                maps: [ptr::null_mut(); MAX_COLORMAPS],
                pool: ptr::null_mut(),
                pool_count: 0,
            };
            let mut table = vec![17, 17];
            table.extend(std::iter::repeat_n(90, 768));
            assert_eq!(maps.set(table.as_ptr()), 1);
            let old = maps.maps[17];
            (*old).refcount += 1;
            table[2] = 12;
            assert_eq!(maps.set(table.as_ptr()), 1);
            let new = maps.maps[17];
            assert_eq!((*new).version, 2);
            assert_eq!((*old).refcount, 1);
            assert_eq!(GetNativePaletteColor((*old).palette, 0).r, 90);
            assert_eq!(GetNativePaletteColor((*new).palette, 0).r, 12);
            maps.release(old);
            maps.release(new);
            while !maps.pool.is_null() {
                let map = maps.pool;
                maps.pool = (*map).next;
                delete_map(map);
            }
        }
    }
}
