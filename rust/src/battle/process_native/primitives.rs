//! Display primitive allocation and ordered link insertion.
use super::*;

// The primitive pool stores only link words here. The other fields survive a
// free/reallocate cycle until the caller writes them, as in the C macros.
pub(crate) unsafe fn alloc_display_prim(array: *mut PRIMITIVE, free: *mut COUNT) -> COUNT {
    // SAFETY: the caller owns the initialized primitive pool and free-list head.
    unsafe {
        let index = *free;
        *free = ((*array.add(usize::from(index))).Links >> 16) as COUNT;
        index
    }
}

pub(crate) unsafe fn free_display_prim(array: *mut PRIMITIVE, free: *mut COUNT, index: COUNT) {
    // SAFETY: index belongs to the caller's primitive pool.
    unsafe {
        (*array.add(usize::from(index))).Links = make_links(END, *free);
        *free = index;
    }
}

pub(super) unsafe fn insert_prim_links(
    array: *mut PRIMITIVE,
    list: *mut PRIM_LINKS,
    index: COUNT,
    before: COUNT,
) {
    // SAFETY: the caller supplies live pool indices or the END sentinel.
    unsafe {
        let link;
        if before == END {
            link = (*list >> 16) as COUNT;
            *list = if link == END {
                make_links(index, index)
            } else {
                make_links(*list as COUNT, index)
            };
        } else {
            let pl = (*array.add(usize::from(before))).Links;
            if before != *list as COUNT {
                link = pl as COUNT;
            } else {
                link = END;
                *list = make_links(index, (*list >> 16) as COUNT);
            }
            (*array.add(usize::from(before))).Links = make_links(index, (pl >> 16) as COUNT);
        }
        if link != END {
            let primitive = array.add(usize::from(link));
            (*primitive).Links = make_links((*primitive).Links as COUNT, index);
        }
        (*array.add(usize::from(index))).Links = make_links(link, before);
    }
}
