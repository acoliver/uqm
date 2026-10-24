//! UTF-8 incremental star search and text-entry callbacks from pstarmap.c.

use super::*;
use crate::memory::{rust_hfree, rust_hmalloc};

use crate::strings::unicode::get_char_from_string as getCharFromString;
use crate::strings::unicode::skip_utf8_chars as skipUTF8Chars;
use crate::strings::unicode::uni_char_to_upper as UniChar_toUpper;
use crate::strings::unicode::utf8_string_compare as utf8StringCompare;
use crate::strings::unicode::utf8_string_count as utf8StringCount;
use crate::strings::unicode::utf8_string_count_n as utf8StringCountN;
extern "C" {
    pub(super) fn strcmp(a: *const c_char, b: *const c_char) -> i32;
    pub(super) fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn qsort(
        base: *mut c_void,
        count: usize,
        size: usize,
        compare: unsafe extern "C" fn(*const c_void, *const c_void) -> i32,
    );

    fn DoTextEntry(entry: *mut TextEntryState) -> u32;
}

// controls.h is not part of the global-state bindgen input. This is the whole
// textentry_state, including the private fields DoTextEntry initializes.
#[repr(C)]
struct TextEntryState {
    InputFunc: Option<unsafe extern "C" fn(*mut TextEntryState) -> u32>,
    Initialized: u32,
    NextTime: u32,
    Success: u32,
    CacheStr: *mut c_char,
    JoyAlphaString: STRING,
    JoystickMode: u32,
    UpperRegister: u32,
    JoyAlpha: *mut c_void,
    JoyAlphaLength: i32,
    JoyUpper: *mut c_void,
    JoyLower: *mut c_void,
    JoyRegLength: i32,
    InsPt: *mut c_char,
    BaseStr: *mut c_char,
    CursorPos: i32,
    MaxSize: i32,
    ChangeCallback: Option<unsafe extern "C" fn(*mut TextEntryState) -> u32>,
    FrameCallback: Option<unsafe extern "C" fn(*mut TextEntryState) -> u32>,
    CbParam: *mut c_void,
}

#[repr(C)]
struct StarSearchState {
    pMS: *mut MENU_STATE,
    Text: [c_char; 256],
    LastText: [c_char; 256],
    LastChangeTime: u32,
    FirstIndex: i32,
    CurIndex: i32,
    LastIndex: i32,
    SingleClust: bool,
    SingleMatch: bool,
    Buffer: [c_char; 256],
    Prefix: *const c_char,
    Cluster: *const c_char,
    PrefixLen: i32,
    ClusterLen: i32,
    ClusterPos: i32,
    SortedStars: [i32; NUM_SOLAR_SYSTEMS],
}

unsafe fn compare_star_names(
    a: CStarDesc,
    b: CStarDesc,
    cluster: impl Fn(u8) -> *const c_char,
) -> i32 {
    if a.postfix != b.postfix {
        return utf8StringCompare((cluster(a.postfix)).cast(), (cluster(b.postfix)).cast());
    }
    if a.prefix < b.prefix {
        -1
    } else if a.prefix > b.prefix {
        1
    } else {
        0
    }
}
unsafe extern "C" fn comp_star_name(a: *const c_void, b: *const c_void) -> i32 {
    compare_star_names(
        *star_array.add(*a.cast::<i32>() as usize),
        *star_array.add(*b.cast::<i32>() as usize),
        |postfix| game_string(u16::from(postfix)),
    )
}
unsafe fn sort_stars_on_name(state: *mut StarSearchState) {
    for i in 0..NUM_SOLAR_SYSTEMS {
        (*state).SortedStars[i] = i as i32;
    }
    qsort(
        ptr::addr_of_mut!((*state).SortedStars).cast(),
        NUM_SOLAR_SYSTEMS,
        std::mem::size_of::<i32>(),
        comp_star_name,
    );
}

unsafe fn split_star_name(state: *mut StarSearchState) {
    let mut buf = ptr::addr_of_mut!((*state).Buffer).cast::<c_char>();
    (*state).Prefix = ptr::null();
    (*state).PrefixLen = 0;
    (*state).Cluster = ptr::null();
    (*state).ClusterLen = 0;
    (*state).ClusterPos = 0;
    let mut next = buf.cast::<u8>().cast_const();
    while *next != 0 && getCharFromString(&mut next) == u32::from(b' ') {
        buf = next.cast_mut().cast();
    }
    if *buf == 0 {
        return;
    }
    (*state).Prefix = buf;
    buf = next.cast_mut().cast();
    while *next != 0 && getCharFromString(&mut next) != u32::from(b' ') {
        buf = next.cast_mut().cast();
    }
    let mut sep = ptr::null_mut();
    if *buf != 0 {
        sep = buf;
        buf = next.cast_mut().cast();
        while *next != 0 && getCharFromString(&mut next) == u32::from(b' ') {
            buf = next.cast_mut().cast();
        }
    }
    if *buf == 0 {
        (*state).Cluster = (*state).Prefix;
        (*state).ClusterLen = utf8StringCount(((*state).Cluster).cast()) as i32;
        (*state).ClusterPos = utf8StringCountN(
            ptr::addr_of!((*state).Buffer).cast(),
            ((*state).Cluster).cast(),
        ) as i32;
        (*state).Prefix = ptr::null();
        return;
    }
    (*state).Cluster = buf;
    (*state).ClusterLen = utf8StringCount(((*state).Cluster).cast()) as i32;
    (*state).ClusterPos = utf8StringCountN(
        ptr::addr_of!((*state).Buffer).cast(),
        ((*state).Cluster).cast(),
    ) as i32;
    *sep = 0;
    (*state).PrefixLen = utf8StringCount(((*state).Prefix).cast()) as i32;
}

fn skip_star_cluster(stars: &[CStarDesc], sorted: &[i32], mut i: usize) -> usize {
    let postfix = stars[sorted[i] as usize].postfix;
    i += 1;
    while i < sorted.len() && stars[sorted[i] as usize].postfix == postfix {
        i += 1;
    }
    i
}
unsafe fn matches_chars(source: *const c_char, destination: *const c_char, count: i32) -> bool {
    let mut source = source.cast::<u8>();
    let mut destination = destination.cast::<u8>();
    for _ in 0..count {
        let sc = getCharFromString(&mut source);
        let dc = getCharFromString(&mut destination);
        if UniChar_toUpper(sc) != UniChar_toUpper(dc) {
            return false;
        }
    }
    true
}

unsafe fn find_next_index(
    state: *const StarSearchState,
    from: i32,
    within_cluster: bool,
    stars: &[CStarDesc],
    cluster_name: impl Fn(u8) -> *const c_char,
    full_name: impl Fn(*const CStarDesc, *mut c_char),
) -> i32 {
    if (*state).Cluster.is_null() {
        return -1;
    }
    let sorted = &(&(*state).SortedStars)[..stars.len()];
    let mut i = from as usize;
    while i < stars.len() {
        let star = &stars[sorted[i] as usize];
        let name = cluster_name(star.postfix);
        if (*state).ClusterLen > utf8StringCount((name).cast()) as i32
            || !matches_chars((*state).Cluster, name, (*state).ClusterLen)
        {
            i = skip_star_cluster(stars, sorted, i);
            continue;
        }
        if !(*state).Prefix.is_null() && star.prefix == 0 {
            i += 1;
            continue;
        }
        if within_cluster {
            break;
        }
        if (*state).Prefix.is_null() {
            if i == 0 || star.postfix != stars[sorted[i - 1] as usize].postfix {
                break;
            }
            i = skip_star_cluster(stars, sorted, i);
            continue;
        }
        let mut name = [0 as c_char; 256];
        full_name(star, name.as_mut_ptr());
        if (*state).PrefixLen <= utf8StringCount((name.as_ptr()).cast()) as i32
            && matches_chars((*state).Prefix, name.as_ptr(), (*state).PrefixLen)
        {
            break;
        }
        i += 1;
    }
    if i < stars.len() {
        i as i32
    } else {
        -1
    }
}
unsafe fn find_next_star_index(state: *const StarSearchState, from: i32, within: bool) -> i32 {
    find_next_index(
        state,
        from,
        within,
        std::slice::from_raw_parts(star_array, NUM_SOLAR_SYSTEMS),
        |postfix| game_string(u16::from(postfix)),
        |star, name| GetClusterName(star, name),
    )
}

unsafe fn draw_matched_star_name(entry: *mut TextEntryState) {
    let state = (*entry).CbParam.cast::<StarSearchState>();
    let mut buf = [0 as c_char; 256];
    let mut cur_pos = -1i16;
    let ex_pos;
    let star = star_array.add((*state).SortedStars[(*state).CurIndex as usize] as usize);
    let flags;
    if (*state).SingleClust || (*state).SingleMatch {
        GetClusterName(star, buf.as_mut_ptr());
        ex_pos = -1;
        flags = 1;
    } else {
        strcpy(buf.as_mut_ptr(), ptr::addr_of!((*state).Text).cast());
        ex_pos = ((*state).ClusterPos + (*state).ClusterLen) as i16;
        let pstr = skipUTF8Chars((buf.as_ptr()).cast(), (*state).ClusterPos as usize);
        strcpy((pstr).cast(), game_string(u16::from((*star).postfix)));
        cur_pos = (*entry).CursorPos as i16;
        flags = 2 | if (*entry).JoystickMode != 0 { 4 } else { 0 };
    }
    DrawSISMessageEx(buf.as_ptr(), cur_pos, ex_pos, flags);
    DrawHyperCoords(cursorLoc);
}

unsafe fn match_next_with(
    state: *mut StarSearchState,
    reset: bool,
    find: impl Fn(*const StarSearchState, i32, bool) -> i32,
) {
    if reset {
        (*state).FirstIndex = -1;
    }
    if (*state).FirstIndex < 0 {
        (*state).CurIndex = -1;
        (*state).LastIndex = -1;
        (*state).SingleClust = false;
        (*state).SingleMatch = false;
        strcpy(
            ptr::addr_of_mut!((*state).Buffer).cast(),
            ptr::addr_of!((*state).Text).cast(),
        );
        split_star_name(state);
    }
    (*state).CurIndex = find(state, (*state).CurIndex + 1, (*state).SingleClust);
    if (*state).FirstIndex < 0 {
        (*state).FirstIndex = (*state).CurIndex;
    }
    if (*state).CurIndex >= 0 {
        (*state).LastIndex = (*state).CurIndex;
    } else {
        (*state).CurIndex = (*state).FirstIndex;
        if (*state).FirstIndex == (*state).LastIndex && (*state).FirstIndex != -1 {
            if (*state).Prefix.is_null() {
                (*state).SingleClust = true;
            } else {
                (*state).SingleMatch = true;
            }
        }
    }
}
unsafe fn match_next_star(state: *mut StarSearchState, reset: bool) {
    match_next_with(state, reset, |s, from, within| {
        find_next_star_index(s, from, within)
    });
}

unsafe extern "C" fn on_star_name_change(entry: *mut TextEntryState) -> u32 {
    let state = (*entry).CbParam.cast::<StarSearchState>();
    if strcmp(
        ptr::addr_of!((*state).Text).cast(),
        ptr::addr_of!((*state).LastText).cast(),
    ) != 0
    {
        (*state).LastChangeTime = GetTimeCounter();
        strcpy(
            ptr::addr_of_mut!((*state).LastText).cast(),
            ptr::addr_of!((*state).Text).cast(),
        );
        match_next_star(state, true);
    }
    if (*state).CurIndex < 0 {
        let flags = if (*state).Text[0] == 0 { 1 } else { 2 }
            | if (*entry).JoystickMode != 0 { 4 } else { 0 };
        DrawSISMessageEx(
            ptr::addr_of!((*state).Text).cast(),
            (*entry).CursorPos as i16,
            -1,
            flags,
        )
    } else {
        let star = star_array.add((*state).SortedStars[(*state).CurIndex as usize] as usize);
        UpdateCursorLocation(0, 0, ptr::addr_of!((*star).star_pt).cast());
        draw_matched_star_name(entry);
        UpdateFuelRequirement();
        1
    }
}
unsafe extern "C" fn on_star_name_frame(entry: *mut TextEntryState) -> u32 {
    let state = (*entry).CbParam.cast::<StarSearchState>();
    if ptr::addr_of!(crate::c_bindings::controller_input::PulsedInputState)
        .read()
        .menu[22]
        != 0
    {
        match_next_star(state, false);
        if (*state).CurIndex < 0 {
            if ptr::addr_of!(crate::c_bindings::controller_input::PulsedInputState)
                .read()
                .menu[22]
                != 0
            {
                PlayMenuSound(2);
            }
            return 1;
        }
        let star = star_array.add((*state).SortedStars[(*state).CurIndex as usize] as usize);
        UpdateCursorLocation(0, 0, ptr::addr_of!((*star).star_pt).cast());
        draw_matched_star_name(entry);
        UpdateFuelRequirement();
    }
    flashCurrentLocation(ptr::null());
    SleepThread(ONE_SECOND / 30);
    1
}

/// # Safety
/// Game-thread only; `menu` is the live map menu. DoTextEntry owns the nested
/// input loop and invokes the callbacks only until it returns.
#[no_mangle]
pub unsafe extern "C" fn DoStarSearch(menu: *mut MENU_STATE) -> u32 {
    let state = rust_hmalloc(std::mem::size_of::<StarSearchState>()).cast::<StarSearchState>();
    if state.is_null() {
        return 0;
    }
    // Initialize the cache as well as the C entry fields so Rust never reads
    // indeterminate integers before the first text-change callback.
    state.write(std::mem::zeroed());
    DrawSISMessageEx(c"".as_ptr(), 0, 0, 1);
    (*state).pMS = menu;
    (*state).LastChangeTime = 0;
    (*state).Text[0] = 0;
    (*state).LastText[0] = 0;
    (*state).FirstIndex = -1;
    sort_stars_on_name(state);
    let mut entry: TextEntryState = std::mem::zeroed();
    entry.Initialized = 0;
    entry.BaseStr = ptr::addr_of_mut!((*state).Text).cast();
    entry.MaxSize = 256;
    entry.CursorPos = 0;
    entry.CbParam = state.cast();
    entry.ChangeCallback = Some(on_star_name_change);
    entry.FrameCallback = Some(on_star_name_frame);
    SetMenuSounds(15, 16);
    SetDefaultMenuRepeatDelay();
    let success = DoTextEntry(&mut entry);
    DrawSISMessageEx(ptr::addr_of!((*state).Text).cast(), -1, -1, 2);
    rust_hfree(state.cast());
    success
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};

    fn state(text: &str) -> Box<StarSearchState> {
        let mut state: Box<StarSearchState> = Box::new(unsafe { std::mem::zeroed() });
        let text = CString::new(text).unwrap();
        unsafe {
            strcpy(state.Text.as_mut_ptr(), text.as_ptr());
            strcpy(state.Buffer.as_mut_ptr(), text.as_ptr());
        }
        for i in 0..NUM_SOLAR_SYSTEMS {
            state.SortedStars[i] = i as i32;
        }
        state.FirstIndex = -1;
        state
    }
    unsafe fn string(p: *const c_char) -> Option<String> {
        (!p.is_null()).then(|| CStr::from_ptr(p).to_str().unwrap().to_owned())
    }
    #[test]
    fn splits_names_with_utf8_character_positions_and_only_ascii_spaces() {
        for (input, prefix, cluster, position) in [
            ("", None, None, 0),
            ("   ", None, None, 0),
            ("  Cent", None, Some("Cent"), 2),
            ("Alpha   Centauri", Some("Alpha"), Some("Centauri"), 8),
            (" Éta  Céti", Some("Éta"), Some("Céti"), 6),
            ("Alpha ", None, Some("Alpha "), 0),
            ("Alpha Centauri A", Some("Alpha"), Some("Centauri A"), 6),
            ("\tSol", None, Some("\tSol"), 0),
        ] {
            let mut state = state(input);
            unsafe {
                split_star_name(&mut *state);
                assert_eq!(string(state.Prefix).as_deref(), prefix, "{input}");
                assert_eq!(string(state.Cluster).as_deref(), cluster, "{input}");
            }
            assert_eq!(state.ClusterPos, position, "{input}");
            assert_eq!(
                state.PrefixLen,
                prefix.map_or(0, |p| p.chars().count() as i32)
            );
            assert_eq!(
                state.ClusterLen,
                cluster.map_or(0, |p| p.chars().count() as i32)
            );
        }
    }
    fn star(prefix: u8, postfix: u8) -> CStarDesc {
        CStarDesc {
            star_pt: crate::comm::dispatch::CPoint { x: 0, y: 0 },
            star_type: 0,
            index: 0,
            prefix,
            postfix,
        }
    }
    struct Catalog {
        stars: Vec<CStarDesc>,
        clusters: Vec<CString>,
        full: Vec<CString>,
    }
    impl Catalog {
        fn new() -> Self {
            Self {
                stars: vec![star(1, 0), star(2, 0), star(1, 1), star(2, 1), star(0, 2)],
                clusters: ["Centauri", "Ceti", "Sol"]
                    .map(|s| CString::new(s).unwrap())
                    .into(),
                full: [
                    "Alpha Centauri",
                    "Beta Centauri",
                    "Alpha Ceti",
                    "Beta Ceti",
                    "Sol",
                ]
                .map(|s| CString::new(s).unwrap())
                .into(),
            }
        }
        unsafe fn find(&self, s: *const StarSearchState, from: i32, within: bool) -> i32 {
            find_next_index(
                s,
                from,
                within,
                &self.stars,
                |p| self.clusters[p as usize].as_ptr(),
                |star, dst| {
                    let i = star.offset_from(self.stars.as_ptr()) as usize;
                    strcpy(dst, self.full[i].as_ptr());
                },
            )
        }
    }
    #[test]
    fn matches_clusters_prefixes_singular_stars_and_case() {
        let catalog = Catalog::new();
        for (input, within, expected) in [
            ("ce", false, vec![0, 2]),
            ("ce", true, vec![0, 1, 2, 3]),
            ("b CE", false, vec![1, 3]),
            ("Alpha ce", false, vec![0, 2]),
            ("a Sol", false, vec![]),
            ("so", false, vec![4]),
            ("Centaurii", false, vec![]),
            ("", false, vec![]),
        ] {
            let mut s = state(input);
            unsafe {
                split_star_name(&mut *s);
            }
            let mut indices = Vec::new();
            let mut from = 0;
            loop {
                let found = unsafe { catalog.find(&*s, from, within) };
                if found < 0 {
                    break;
                }
                indices.push(found);
                from = found + 1;
            }
            assert_eq!(indices, expected, "{input}");
        }
    }
    #[test]
    fn wraps_and_expands_a_single_cluster_then_resets_cache_on_edits() {
        let catalog = Catalog::new();
        let mut s = state("Cent");
        unsafe {
            match_next_with(&mut *s, true, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, 0);
            assert!(!s.SingleClust);
            match_next_with(&mut *s, false, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, 0);
            assert!(s.SingleClust);
            match_next_with(&mut *s, false, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, 1);
            match_next_with(&mut *s, false, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, 0);
            strcpy(s.Text.as_mut_ptr(), c"Beta Ceti".as_ptr());
            match_next_with(&mut *s, true, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, 3);
            assert!(!s.SingleClust);
            match_next_with(&mut *s, false, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, 3);
            assert!(s.SingleMatch);
            strcpy(s.Text.as_mut_ptr(), c"absent".as_ptr());
            match_next_with(&mut *s, true, |s, f, w| catalog.find(s, f, w));
            assert_eq!(s.CurIndex, -1);
            assert_eq!(s.FirstIndex, -1);
            assert!(!s.SingleMatch);
        }
    }
    #[test]
    fn sorting_uses_localized_cluster_name_then_numeric_prefix() {
        let clusters = [c"Zeta", c"Alpha"];
        unsafe {
            assert!(
                compare_star_names(star(1, 0), star(1, 1), |p| clusters[p as usize].as_ptr()) > 0
            );
            assert!(
                compare_star_names(star(2, 0), star(1, 0), |p| clusters[p as usize].as_ptr()) > 0
            );
            assert_eq!(
                compare_star_names(star(1, 0), star(1, 0), |p| clusters[p as usize].as_ptr()),
                0
            );
        }
    }
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn text_entry_layout_matches_controls_header_on_64_bit_hosts() {
        assert_eq!(std::mem::size_of::<TextEntryState>(), 136);
        assert_eq!(std::mem::offset_of!(TextEntryState, BaseStr), 96);
        assert_eq!(std::mem::offset_of!(TextEntryState, CbParam), 128);
    }
}
