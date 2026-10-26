//! Animation and bitmap-font loaders, ported from gfxload.c (GPL-2.0-or-later).
use super::native_abi::*;
use crate::io::uio_bridge as uio;
use crate::memory::{rust_hcalloc, rust_hfree, rust_hmalloc};
use crate::strings::string_table as strings;

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;

use crate::graphics::tfb_draw_abi::TFB_DrawImage_New;

extern "C" {
    static mut repository: *mut uio::uio_Repository;
}
#[derive(Clone, Copy)]
struct AniData {
    transparent: i32,
    colormap: i32,
    hotspot_x: i32,
    hotspot_y: i32,
}

fn path_parts(path: &[u8]) -> (&[u8], &[u8], &[u8]) {
    let pos = path
        .iter()
        .rposition(|&b| b == b'/')
        .or_else(|| path.iter().rposition(|&b| b == b'\\'));
    if let Some(i) = pos {
        (&path[..i], &path[i + 1..], &path[..i + 1])
    } else {
        (b".", path, b"")
    }
}
unsafe fn process_image(frame: *mut frame_desc, canvas: *mut c_void, ani: AniData, index: usize) {
    (*frame).Type = ROM_DRAWABLE as u16;
    (*frame).Index = index as u16;
    if TFB_DrawCanvas_IsPaletted(canvas) != 0 {
        if ani.transparent >= 0 {
            TFB_DrawCanvas_SetTransparentIndex(canvas, ani.transparent, 0);
        }
    } else if ani.transparent == 0 {
        TFB_DrawCanvas_SetTransparentColor(
            canvas,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            0,
        );
    }
    if ani.transparent == -1 {
        TFB_DrawCanvas_SetTransparentIndex(canvas, -1, 0);
    }
    (*frame).image = TFB_DrawImage_New(canvas).cast();
    let image = &mut *(*frame).image;
    image.colormap_index = ani.colormap;
    (*frame).HotSpot = HOT_SPOT {
        x: ani.hotspot_x as i16,
        y: ani.hotspot_y as i16,
    };
    (*frame).Bounds = image.extent;
}
unsafe fn process_font_char(ch: *mut TFB_Char, canvas: *mut c_void) {
    TFB_DrawCanvas_GetExtent(canvas, &mut (*ch).extent);
    let pitch = (*ch).extent.width as usize;
    let data = rust_hmalloc(pitch * (*ch).extent.height as usize).cast();
    TFB_DrawCanvas_GetFontCharData(canvas, data, pitch as u32);
    (*ch).data = data;
    (*ch).pitch = pitch as u32;
    (*ch).disp = EXTENT {
        width: (*ch).extent.width.wrapping_add(1),
        height: (*ch).extent.height.wrapping_add(1),
    };
    (*ch).HotSpot = HOT_SPOT {
        x: 0,
        y: tuned_baseline((*ch).extent.height),
    };
}
fn tuned_baseline(height: i16) -> i16 {
    let tune = if height == 8 {
        -1
    } else if height == 9 {
        -2
    } else if height > 9 {
        -3
    } else {
        0
    };
    height.wrapping_add(tune)
}
unsafe fn parse_ani(line: *const c_char) -> Option<(CString, AniData)> {
    let mut filename = [0 as c_char; 1024];
    let mut ani = AniData {
        transparent: 0,
        colormap: 0,
        hotspot_x: 0,
        hotspot_y: 0,
    };
    let count = libc::sscanf(
        line,
        c"%1023s %d %d %d %d".as_ptr(),
        filename.as_mut_ptr(),
        &mut ani.transparent,
        &mut ani.colormap,
        &mut ani.hotspot_x,
        &mut ani.hotspot_y,
    );
    if count != 5 {
        return None;
    }
    Some((CStr::from_ptr(filename.as_ptr()).to_owned(), ani))
}
/// # Safety
/// Stream and resource path must remain live for this serialized resource load.
pub unsafe extern "C" fn get_cel_data(fp: *mut c_void, length: u32) -> *mut c_void {
    let fp = fp.cast::<uio::uio_Stream>();
    let mut position = uio::uio_ftell(fp);
    let path = if _cur_resfile_name.is_null() {
        &b""[..]
    } else {
        CStr::from_ptr(_cur_resfile_name).to_bytes()
    };
    let (parent, name, prefix) = path_parts(path);
    let mut header = [0u8; 4];
    uio::uio_fread(header.as_mut_ptr().cast(), 4, 1, fp);
    let mut mount = ptr::null_mut();
    let (dir, file, prefix) = if !path.is_empty() && u32::from_le_bytes(header) == 0x04034b50 {
        let parent = CString::new(parent).unwrap();
        let name = CString::new(name).unwrap();
        let dir = uio::uio_openDir(repository, parent.as_ptr(), 0);
        let mut auto_mount = [ptr::null_mut()];
        mount = uio::uio_mountDir(
            repository,
            parent.as_ptr(),
            2,
            dir,
            name.as_ptr(),
            c"/".as_ptr(),
            auto_mount.as_mut_ptr(),
            2 | 4,
            ptr::null_mut(),
        );
        let file = uio::uio_fopen(dir, name.as_ptr(), c"r".as_ptr());
        position = 0;
        (dir, file, &b""[..])
    } else {
        (contentDir.cast(), fp, prefix)
    };
    let mut line = [0 as c_char; 1024];
    uio::uio_fseek(file, position, libc::SEEK_SET);
    let mut total = 0;
    while !uio::uio_fgets(line.as_mut_ptr(), line.len() as i32, file).is_null() {
        total += 1;
    }
    let mut images = Vec::with_capacity(total);
    uio::uio_fseek(file, position, libc::SEEK_SET);
    while images.len() < total
        && !uio::uio_fgets(line.as_mut_ptr(), line.len() as i32, file).is_null()
    {
        let Some((name, ani)) = parse_ani(line.as_ptr()) else {
            log::warn!("get_cel_data: invalid animation entry");
            break;
        };
        let filename = CString::new([prefix, name.as_bytes()].concat()).unwrap();
        let canvas = TFB_DrawCanvas_LoadFromFile(dir.cast(), filename.as_ptr());
        if canvas.is_null() {
            log::warn!("get_cel_data: Unable to load image!");
        } else {
            images.push((canvas, ani));
        }
        if (uio::uio_ftell(file) as i32).wrapping_sub(position as i32) >= length as i32 {
            break;
        }
    }
    let mut drawable = ptr::null_mut();
    if !images.is_empty() {
        drawable = AllocDrawable(images.len() as u16);
        if !drawable.is_null() {
            (*drawable).Flags = 2;
            (*drawable).MaxIndex = (images.len() - 1) as u16;
            for (i, &(canvas, ani)) in images.iter().enumerate().rev() {
                process_image((*drawable).Frame.add(i), canvas, ani, i);
            }
        } else {
            for (canvas, _) in images {
                TFB_DrawCanvas_Delete(canvas);
            }
        }
    }
    if drawable.is_null() {
        log::warn!("Couldn't get cel data");
    }
    if !mount.is_null() {
        uio::uio_fclose(file);
        uio::uio_closeDir(dir);
        uio::uio_unmountDir(mount);
    }
    drawable.cast()
}
/// # Safety
/// Handle must be null or an owned drawable, consumed here; queued image deletion remains deferred.
pub unsafe extern "C" fn release_cel_data(handle: *mut c_void) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let drawable = handle.cast::<drawable_desc>();
    let count = usize::from((*drawable).MaxIndex) + 1;
    let frames = (*drawable).Frame;
    rust_hfree(handle);
    if !frames.is_null() {
        for i in 0..count {
            let frame = frames.add(i);
            let img = (*frame).image;
            if !img.is_null() {
                (*frame).image = ptr::null_mut();
                TFB_DrawScreen_DeleteImage(img);
            }
        }
        rust_hfree(frames.cast());
    }
    1
}
#[repr(C)]
struct BuildChar {
    canvas: *mut c_void,
    index: u32,
}
unsafe extern "C" fn compare_char(a: *const c_void, b: *const c_void) -> i32 {
    (*a.cast::<BuildChar>()).index as i32 - (*b.cast::<BuildChar>()).index as i32
}
/// # Safety
/// Resource path and directory/stream sentinel must be valid for a serialized font load.
pub unsafe extern "C" fn get_font_data(fp: *mut c_void, _length: u32) -> *mut c_void {
    let fp = fp.cast::<uio_Stream>();
    if _cur_resfile_name.is_null() {
        return ptr::null_mut();
    }
    let path = CStr::from_ptr(_cur_resfile_name).to_bytes();
    let mut mount = ptr::null_mut();
    if fp as usize != usize::MAX {
        let (parent, name, _) = path_parts(path);
        let parent = CString::new(parent).unwrap();
        let name = CString::new(name).unwrap();
        let dir = uio::uio_openDir(repository, parent.as_ptr(), 0);
        let mut auto_mount = [ptr::null_mut()];
        mount = uio::uio_mountDir(
            repository,
            _cur_resfile_name,
            2,
            dir,
            name.as_ptr(),
            c"/".as_ptr(),
            auto_mount.as_mut_ptr(),
            2 | 4,
            ptr::null_mut(),
        );
        uio::uio_closeDir(dir);
    }
    let table = crate::resource::direct::LoadDirEntryTable(
        contentDir.cast(),
        _cur_resfile_name,
        c".".as_ptr(),
        3,
    );
    let entries = strings::CaptureStringTable(table);
    if entries.is_null() {
        if !mount.is_null() {
            uio::uio_unmountDir(mount);
        }
        return ptr::null_mut();
    }
    let count = strings::GetStringTableCount(entries);
    let dir = uio::uio_openDirRelative(contentDir.cast(), _cur_resfile_name, 0);
    if dir.is_null() {
        strings::DestroyStringTable(strings::ReleaseStringTable(entries));
        if !mount.is_null() {
            uio::uio_unmountDir(mount);
        }
        return ptr::null_mut();
    }
    let mut chars = Vec::with_capacity(count as usize);
    for i in 0..count {
        let name = strings::GetStringAddress(strings::SetAbsStringTableIndex(entries, i));
        let mut index = 0u32;
        if libc::sscanf(name, c"%x.".as_ptr(), &mut index) != 1 || index > 65535 {
            continue;
        }
        let canvas = TFB_DrawCanvas_LoadFromFile(dir.cast(), name);
        if canvas.is_null() {
            continue;
        }
        let mut size = std::mem::zeroed();
        TFB_DrawCanvas_GetExtent(canvas, &mut size);
        if size.width == 0 || size.height == 0 {
            TFB_DrawCanvas_Delete(canvas);
            continue;
        }
        chars.push(BuildChar { canvas, index });
    }
    uio::uio_closeDir(dir);
    strings::DestroyStringTable(strings::ReleaseStringTable(entries));
    if !mount.is_null() {
        uio::uio_unmountDir(mount);
    }
    libc::qsort(
        chars.as_mut_ptr().cast(),
        chars.len(),
        std::mem::size_of::<BuildChar>(),
        Some(compare_char),
    );
    let font: *mut font_desc = rust_hcalloc(std::mem::size_of::<font_desc>()).cast();
    let mut start = 0;
    let mut page_end = ptr::addr_of_mut!((*font).fontPages);
    while start < chars.len() {
        let page_start = chars[start].index & CHARACTER_PAGE_MASK;
        let mut end = start;
        while end < chars.len() && chars[end].index & CHARACTER_PAGE_MASK == page_start {
            end += 1;
        }
        let count = (chars[end - 1].index + 1 - chars[start].index) as usize;
        let page: *mut FontPage = rust_hmalloc(std::mem::size_of::<FontPage>()).cast();
        (*page).charDesc = rust_hcalloc(count * std::mem::size_of::<TFB_Char>()).cast();
        (*page).pageStart = page_start;
        (*page).firstChar = chars[start].index;
        (*page).numChars = count;
        *page_end = page;
        page_end = ptr::addr_of_mut!((*page).next);
        for bcd in &chars[start..end] {
            let ch = (*page)
                .charDesc
                .add((bcd.index - (*page).firstChar) as usize);
            if !(*ch).data.is_null() {
                TFB_DrawCanvas_Delete(bcd.canvas);
                continue;
            }
            process_font_char(ch, bcd.canvas);
            TFB_DrawCanvas_Delete(bcd.canvas);
            if (*ch).disp.height as i32 > i32::from((*font).Leading) {
                (*font).Leading = (*ch).disp.height as u16;
            }
            if (*ch).disp.width as i32 > i32::from((*font).LeadingWidth) {
                (*font).LeadingWidth = (*ch).disp.width as u16;
            }
        }
        start = end;
    }
    *page_end = ptr::null_mut();
    (*font).Leading = (*font).Leading.wrapping_add(1);
    font.cast()
}
/// # Safety
/// Handle must be null or an owned font allocation, consumed here.
pub unsafe extern "C" fn release_font_data(handle: *mut c_void) -> i32 {
    if handle.is_null() {
        return 0;
    }
    let font = handle.cast::<font_desc>();
    let mut page = (*font).fontPages;
    while !page.is_null() {
        for i in 0..(*page).numChars {
            let ch = (*page).charDesc.add(i);
            if !(*ch).data.is_null() {
                TFB_DrawScreen_DeleteData((*ch).data.cast());
            }
        }
        let next = (*page).next;
        rust_hfree((*page).charDesc.cast());
        rust_hfree(page.cast());
        page = next;
    }
    rust_hfree(handle);
    1
}
#[cfg(test)]
mod tests {
    #[test]
    fn resource_callbacks_use_the_registered_signatures() {
        use crate::resource::ffi_types::{ResourceFreeFun, ResourceLoadFileFun};
        let _: [ResourceLoadFileFun; 2] = [super::get_cel_data, super::get_font_data];
        let release: [ResourceFreeFun; 2] = [super::release_cel_data, super::release_font_data];
        for callback in release {
            assert_eq!(unsafe { callback(std::ptr::null_mut()) }, 0);
        }
    }

    use super::*;
    #[test]
    fn font_baseline_tuning_preserves_legacy_spacing() {
        assert_eq!(
            (
                tuned_baseline(7),
                tuned_baseline(8),
                tuned_baseline(9),
                tuned_baseline(10),
                tuned_baseline(15)
            ),
            (7, 7, 7, 7, 12)
        );
        assert_eq!(
            path_parts(b"font/char.png"),
            (&b"font"[..], &b"char.png"[..], &b"font/"[..])
        );
        unsafe {
            let (name, ani) = parse_ani(c"planet.png -2 17 -5 11".as_ptr()).unwrap();
            assert_eq!(name.as_bytes(), b"planet.png");
            assert_eq!(
                (ani.transparent, ani.colormap, ani.hotspot_x, ani.hotspot_y),
                (-2, 17, -5, 11)
            );
        }
    }
}
