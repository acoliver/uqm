//! Frame indexing ABI, ported from pixmap.c (GPL-2.0-or-later).
use super::native_abi::{drawable_desc, frame_desc};
use std::ptr::null_mut;

/// # Safety
/// Frame pointers must be null or live frames belonging to a drawable.
#[no_mangle]
pub unsafe extern "C" fn GetFrameParentDrawable(frame: *mut frame_desc) -> *mut drawable_desc {
    if frame.is_null() {
        null_mut()
    } else {
        (*frame).parent
    }
}
/// # Safety
/// Drawable must be null or live and contain at least one frame.
#[no_mangle]
pub unsafe extern "C" fn CaptureDrawable(drawable: *mut drawable_desc) -> *mut frame_desc {
    if drawable.is_null() {
        null_mut()
    } else {
        (*drawable).Frame
    }
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn ReleaseDrawable(frame: *mut frame_desc) -> *mut drawable_desc {
    GetFrameParentDrawable(frame)
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn GetFrameCount(frame: *mut frame_desc) -> u16 {
    if frame.is_null() {
        0
    } else {
        (*(*frame).parent).MaxIndex.wrapping_add(1)
    }
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn GetFrameIndex(frame: *mut std::ffi::c_void) -> u16 {
    // The automation ABI accepts an opaque frame handle; interpretation stays here.
    let frame = frame.cast::<frame_desc>();
    if frame.is_null() {
        0
    } else {
        (*frame).Index
    }
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn SetAbsFrameIndex(frame: *mut frame_desc, index: u16) -> *mut frame_desc {
    if frame.is_null() {
        return frame;
    }
    let d = &*(*frame).parent;
    d.Frame.add(index as usize % (usize::from(d.MaxIndex) + 1))
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn SetRelFrameIndex(
    frame: *mut frame_desc,
    mut offset: i16,
) -> *mut frame_desc {
    if frame.is_null() {
        return frame;
    }
    let d = &*(*frame).parent;
    let count = d.MaxIndex.wrapping_add(1);
    if offset < 0 {
        loop {
            offset = (i32::from(offset) + i32::from(count)) as i16;
            if offset >= 0 {
                break;
            }
        }
    }
    let index = (i32::from((*frame).Index as i16) + i32::from(offset)) % i32::from(count);
    d.Frame.offset(index as isize)
}
/// # Safety
/// Both frames must be null or live drawable frames.
#[no_mangle]
pub unsafe extern "C" fn SetEquFrameIndex(
    dst: *mut frame_desc,
    src: *mut frame_desc,
) -> *mut frame_desc {
    if dst.is_null() || src.is_null() {
        null_mut()
    } else {
        SetAbsFrameIndex(dst, GetFrameIndex(src.cast()))
    }
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn IncFrameIndex(frame: *mut frame_desc) -> *mut frame_desc {
    if frame.is_null() {
        return frame;
    }
    let d = &*(*frame).parent;
    if (*frame).Index < d.MaxIndex {
        frame.add(1)
    } else {
        d.Frame
    }
}
/// # Safety
/// Frame must be null or a live drawable frame.
#[no_mangle]
pub unsafe extern "C" fn DecFrameIndex(frame: *mut frame_desc) -> *mut frame_desc {
    if frame.is_null() {
        return frame;
    }
    if (*frame).Index > 0 {
        frame.sub(1)
    } else {
        let d = &*(*frame).parent;
        d.Frame.add(d.MaxIndex as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indexing_wraps_in_both_directions() {
        unsafe {
            let mut frames: [frame_desc; 3] = std::mem::zeroed();
            let mut d = drawable_desc {
                Flags: 0,
                MaxIndex: 2,
                Frame: frames.as_mut_ptr(),
            };
            for (i, f) in frames.iter_mut().enumerate() {
                f.Index = i as u16;
                f.parent = &mut d;
            }
            let first = frames.as_mut_ptr();
            assert_eq!(CaptureDrawable(&mut d), first);
            assert_eq!(GetFrameCount(first), 3);
            assert_eq!(SetAbsFrameIndex(first, 7), first.add(1));
            assert_eq!(SetRelFrameIndex(first, -7), first.add(2));
            assert_eq!(IncFrameIndex(first.add(2)), first);
            assert_eq!(DecFrameIndex(first), first.add(2));
            assert_eq!(ReleaseDrawable(first), &mut d as *mut _);
            assert!(SetEquFrameIndex(first, null_mut()).is_null());
            assert_eq!(GetFrameCount(null_mut()), 0);
        }
    }
}
