//! Font context, measurement and rendering ABI, ported from font.c (GPL-2.0-or-later).
use super::native_abi::*;
use std::ptr;

unsafe fn char_frame(font: *mut font_desc, ch: u32) -> *mut TFB_Char {
    let page_start = ch & CHARACTER_PAGE_MASK;
    let mut page = (*font).fontPages;
    while !page.is_null() && (*page).pageStart != page_start {
        page = (*page).next;
    }
    if page.is_null() {
        return ptr::null_mut();
    }
    let index = ch.wrapping_sub((*page).firstChar) as usize;
    if ch >= (*page).firstChar
        && index < (*page).numChars
        && !(*(*page).charDesc.add(index)).data.is_null()
    {
        (*page).charDesc.add(index)
    } else {
        ptr::null_mut()
    }
}

/// # Safety
/// Font must be null or live; caller must serialize context changes.
#[no_mangle]
pub unsafe extern "C" fn SetContextFont(font: *mut font_desc) -> *mut font_desc {
    let last = _CurFontPtr;
    _CurFontPtr = font;
    if _GraphicsStatusFlags & 4 != 0 {
        (*_pCurContext).Font = font;
        (*_pCurContext).BackingFlags |= FBK_DIRTY as u16;
    }
    last
}
extern "C" {
    fn _ReleaseFontData(handle: *mut std::ffi::c_void) -> BOOLEAN;
}
/// # Safety
/// Font must be null or an owned live font; consumed by this call.
#[no_mangle]
pub unsafe extern "C" fn DestroyFont(font: *mut font_desc) -> BOOLEAN {
    if font.is_null() {
        return 0;
    }
    if !_CurFontPtr.is_null() && _CurFontPtr == font {
        SetContextFont(ptr::null_mut());
    }
    _ReleaseFontData(font.cast())
}
/// # Safety
/// Text and its string must be live on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn font_DrawText(text: *mut TEXT) {
    let mut clip = std::mem::zeroed();
    let mut origin = std::mem::zeroed();
    FixContextFontEffect();
    if !super::frame_abi::graphics_active()
        || super::frame_abi::GetContextValidRect(ptr::null_mut(), &mut origin) == 0
    {
        return;
    }
    let mut text = *text;
    if TextRect(&mut text, &mut clip, ptr::null_mut()) == 0 {
        return;
    }
    _text_blt(&mut clip, &mut text, origin);
}
/// # Safety
/// Text and graphics context must be live on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn font_DrawTracedText(text: *mut TEXT, color: Color, trace: Color) {
    let old = SetContextForeGroundColor(trace);
    (*text).baseline.x = (*text).baseline.x.wrapping_sub(1);
    font_DrawText(text);
    (*text).baseline.x = (*text).baseline.x.wrapping_add(2);
    font_DrawText(text);
    (*text).baseline.x = (*text).baseline.x.wrapping_sub(1);
    (*text).baseline.y = (*text).baseline.y.wrapping_sub(1);
    font_DrawText(text);
    (*text).baseline.y = (*text).baseline.y.wrapping_add(2);
    font_DrawText(text);
    (*text).baseline.y = (*text).baseline.y.wrapping_sub(1);
    SetContextForeGroundColor(color);
    font_DrawText(text);
    SetContextForeGroundColor(old);
}
/// # Safety
/// Height must be writable; current font, if any, must be live.
#[no_mangle]
pub unsafe extern "C" fn GetContextFontLeading(height: *mut i16) -> BOOLEAN {
    *height = if _CurFontPtr.is_null() {
        0
    } else {
        (*_CurFontPtr).Leading as i16
    };
    u32::from(!_CurFontPtr.is_null())
}
/// # Safety
/// Width must be writable; current font, if any, must be live.
#[no_mangle]
pub unsafe extern "C" fn GetContextFontLeadingWidth(width: *mut i16) -> BOOLEAN {
    *width = if _CurFontPtr.is_null() {
        0
    } else {
        (*_CurFontPtr).LeadingWidth as i16
    };
    u32::from(!_CurFontPtr.is_null())
}

unsafe fn measure(
    font: *mut font_desc,
    text: *mut TEXT,
    rect: *mut RECT,
    mut delta: *mut u8,
) -> BOOLEAN {
    let mut deltas = [0u8; 100];
    if !font.is_null() && (*text).CharCount != 0 {
        let mut count = (*text).CharCount;
        if delta.is_null() {
            delta = deltas.as_mut_ptr();
            if count > MAX_DELTAS as u16 {
                count = MAX_DELTAS as u16;
                (*text).CharCount = count;
            }
        }
        let (mut top, mut bottom, mut width) = (0i16, 0i16, 0i16);
        let mut string = (*text).pStr.cast::<u8>();
        let mut next = 0;
        if count > 0 {
            next = getCharFromString(&mut string);
            if next == 0 {
                count = 0;
            }
        }
        while count > 0 {
            count -= 1;
            let last_width = width;
            let ch = next;
            if count > 0 {
                next = getCharFromString(&mut string);
                if next == 0 {
                    (*text).CharCount -= count;
                    count = 0;
                }
            }
            let frame = char_frame(font, ch);
            if !frame.is_null() && (*frame).disp.width != 0 {
                let y = (*frame).HotSpot.y.wrapping_neg();
                if y < top {
                    top = y;
                }
                let y = y.wrapping_add((*frame).disp.height);
                if y > bottom {
                    bottom = y;
                }
                width = width.wrapping_add((*frame).disp.width);
            }
            *delta = width.wrapping_sub(last_width) as u8;
            delta = delta.add(1);
        }
        if width > 0 {
            bottom = bottom.wrapping_sub(top);
            if bottom > 0 {
                if *delta.sub(1) > 0 {
                    *delta.sub(1) -= 1;
                    width -= 1;
                }
                (*rect).corner.x = if (*text).align == TEXT_ALIGN_ALIGN_LEFT {
                    0
                } else if (*text).align == TEXT_ALIGN_ALIGN_CENTER {
                    -(width >> 1)
                } else {
                    -width
                };
                (*rect).corner.y = top;
                (*rect).extent = EXTENT {
                    width,
                    height: bottom,
                };
                (*rect).corner.x = (*rect).corner.x.wrapping_add((*text).baseline.x);
                (*rect).corner.y = (*rect).corner.y.wrapping_add((*text).baseline.y);
                return 1;
            }
        }
    }
    (*rect).corner = (*text).baseline;
    (*rect).extent = EXTENT {
        width: 0,
        height: 0,
    };
    0
}
/// # Safety
/// Text/string and output must be live; non-null delta must hold CharCount bytes.
#[no_mangle]
pub unsafe extern "C" fn TextRect(text: *mut TEXT, rect: *mut RECT, delta: *mut u8) -> BOOLEAN {
    measure(_CurFontPtr, text, rect, delta)
}
/// # Safety
/// All records must be live; called on graphics thread with active context.
#[no_mangle]
pub unsafe extern "C" fn _text_blt(clip: *mut RECT, text: *mut TEXT, ctx_origin: POINT) {
    let font = _CurFontPtr;
    let mode = (*_pCurContext).Mode;
    if font.is_null() {
        return;
    }
    let backing = (*_pCurContext).FontBacking;
    if backing.is_null() {
        return;
    }
    let mut origin = POINT {
        x: (*clip).corner.x,
        y: (*text).baseline.y,
    };
    let mut count = (*text).CharCount;
    if count == 0 {
        return;
    }
    let mut string = (*text).pStr.cast::<u8>();
    let mut next = getCharFromString(&mut string);
    if next == 0 {
        count = 0;
    }
    while count > 0 {
        count -= 1;
        let ch = next;
        if count > 0 {
            next = getCharFromString(&mut string);
            if next == 0 {
                count = 0;
            }
        }
        let frame = char_frame(font, ch);
        if !frame.is_null() && (*frame).disp.width != 0 {
            let mut r = RECT {
                corner: POINT {
                    x: origin.x.wrapping_sub((*frame).HotSpot.x),
                    y: origin.y.wrapping_sub((*frame).HotSpot.y),
                },
                extent: (*frame).disp,
            };
            if BoxIntersect(ptr::addr_of_mut!(r), clip, ptr::addr_of_mut!(r)) != 0 {
                TFB_Prim_FontChar(origin, frame, backing, mode, ctx_origin);
            }
            origin.x = origin.x.wrapping_add((*frame).disp.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measurement_caps_internal_deltas_and_preserves_empty_text_contract() {
        unsafe {
            let mut alpha = 255;
            let mut glyph: TFB_Char = std::mem::zeroed();
            glyph.data = &mut alpha;
            glyph.disp = EXTENT {
                width: 4,
                height: 9,
            };
            glyph.HotSpot = HOT_SPOT { x: 0, y: 7 };
            let mut page = FontPage {
                next: ptr::null_mut(),
                pageStart: 0,
                firstChar: 65,
                numChars: 1,
                charDesc: &mut glyph,
            };
            let mut font = font_desc {
                Leading: 10,
                LeadingWidth: 4,
                fontPages: &mut page,
            };
            let text_bytes = std::ffi::CString::new("A".repeat(110)).unwrap();
            let mut text = TEXT {
                baseline: POINT { x: 20, y: 30 },
                pStr: text_bytes.as_ptr(),
                align: TEXT_ALIGN_ALIGN_RIGHT,
                CharCount: 110,
            };
            let mut rect = std::mem::zeroed();
            assert_eq!(measure(&mut font, &mut text, &mut rect, ptr::null_mut()), 1);
            assert_eq!(text.CharCount, 100);
            assert_eq!(
                (
                    rect.corner.x,
                    rect.corner.y,
                    rect.extent.width,
                    rect.extent.height
                ),
                (-379, 23, 399, 9)
            );
            text.pStr = c"".as_ptr();
            text.CharCount = 9;
            assert_eq!(measure(&mut font, &mut text, &mut rect, ptr::null_mut()), 0);
            assert_eq!(text.CharCount, 9);
            assert_eq!(
                (
                    rect.corner.x,
                    rect.corner.y,
                    rect.extent.width,
                    rect.extent.height
                ),
                (20, 30, 0, 0)
            );
            text.pStr = c"A".as_ptr();
            text.CharCount = 1;
            (*page.charDesc).data = ptr::null_mut();
            assert_eq!(measure(&mut font, &mut text, &mut rect, ptr::null_mut()), 0);
        }
    }
    #[test]
    fn sparse_page_measurement_updates_count_spacing_and_alignment() {
        unsafe {
            let mut data = 255;
            let mut chars: [TFB_Char; 2] = std::mem::zeroed();
            for c in &mut chars {
                c.data = &mut data;
                c.disp = EXTENT {
                    width: 4,
                    height: 9,
                };
                c.HotSpot = HOT_SPOT { x: 0, y: 7 };
            }
            let mut page = FontPage {
                next: ptr::null_mut(),
                pageStart: 0,
                firstChar: 65,
                numChars: 2,
                charDesc: chars.as_mut_ptr(),
            };
            let mut font = font_desc {
                Leading: 10,
                LeadingWidth: 4,
                fontPages: &mut page,
            };
            let mut text = TEXT {
                baseline: POINT { x: 20, y: 30 },
                pStr: c"AB?".as_ptr(),
                align: TEXT_ALIGN_ALIGN_CENTER,
                CharCount: 9,
            };
            let mut rect = std::mem::zeroed();
            let mut delta = [0; 9];
            assert_eq!(
                measure(&mut font, &mut text, &mut rect, delta.as_mut_ptr()),
                1
            );
            assert_eq!(text.CharCount, 3);
            assert_eq!(&delta[..3], &[4, 4, 0]);
            assert_eq!(
                (
                    rect.corner.x,
                    rect.corner.y,
                    rect.extent.width,
                    rect.extent.height
                ),
                (16, 23, 8, 9)
            );
            text.pStr = c"AB".as_ptr();
            text.CharCount = 2;
            assert_eq!(
                measure(&mut font, &mut text, &mut rect, delta.as_mut_ptr()),
                1
            );
            assert_eq!(&delta[..2], &[4, 3]);
            assert_eq!(rect.extent.width, 7);
            assert!(char_frame(&mut font, 2048).is_null());
        }
    }
}
