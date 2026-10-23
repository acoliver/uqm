//! Small surfaces recorded from the original UQM C loaders and rotozoomer.
use super::{png2sdl, rotozoom};
use sdl2_sys::{
    SDL_CreateRGBSurface, SDL_FreeSurface, SDL_GetColorKey, SDL_LockSurface, SDL_RWFromConstMem,
    SDL_RWclose, SDL_Surface, SDL_UnlockSurface,
};
use serde::Deserialize;
use std::ptr;

#[derive(Debug, Deserialize, PartialEq)]
struct Surface {
    width: i32,
    height: i32,
    bpp: u8,
    masks: [u32; 4],
    key: Option<u32>,
    palette: Vec<[u8; 3]>,
    pixels: Vec<u8>,
}
#[derive(Deserialize)]
struct Fixture {
    name: String,
    png: Vec<u8>,
    operation: Vec<String>,
    expected: Surface,
}
unsafe fn describe(surface: *mut SDL_Surface) -> Surface {
    let f = &*(*surface).format;
    let mut key = 0;
    let key = if SDL_GetColorKey(surface, &mut key) == 0 {
        Some(key)
    } else {
        None
    };
    let mut palette = Vec::new();
    if !f.palette.is_null() {
        for i in 0..(*f.palette).ncolors.min(16) {
            let c = *(*f.palette).colors.add(i as usize);
            palette.push([c.r, c.g, c.b]);
        }
    }
    SDL_LockSurface(surface);
    let mut pixels = Vec::new();
    for y in 0..(*surface).h {
        pixels.extend_from_slice(std::slice::from_raw_parts(
            (*surface)
                .pixels
                .cast::<u8>()
                .add((y * (*surface).pitch) as usize),
            (*surface).w as usize * f.BytesPerPixel as usize,
        ));
    }
    SDL_UnlockSurface(surface);
    Surface {
        width: (*surface).w,
        height: (*surface).h,
        bpp: f.BitsPerPixel,
        masks: [f.Rmask, f.Gmask, f.Bmask, f.Amask],
        key,
        palette,
        pixels,
    }
}

#[test]
fn png_and_rotozoom_match_recorded_c_surfaces() {
    let fixtures: Vec<Fixture> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/graphics/sdl-surfaces.json"
    ))
    .unwrap();
    for case in fixtures {
        unsafe {
            let rw = SDL_RWFromConstMem(case.png.as_ptr().cast(), case.png.len() as i32);
            let src = png2sdl::TFB_png_to_sdl(rw);
            SDL_RWclose(rw);
            assert!(!src.is_null(), "{}", case.name);
            let dst = if case.operation.is_empty() {
                src
            } else {
                let a = case.operation[1].parse().unwrap();
                let b = case.operation[2].parse().unwrap();
                let smooth = case.operation[3].parse().unwrap();
                match case.operation[0].as_str() {
                    "zoom" => rotozoom::zoomSurface(src, a, b, smooth),
                    "rz" => rotozoom::rotozoomSurface(src, a, b, smooth),
                    "rot" => {
                        let (mut w, mut h) = (0, 0);
                        rotozoom::rotozoomSurfaceSize((*src).w, (*src).h, a, 1.0, &mut w, &mut h);
                        let f = &*(*src).format;
                        let dst = SDL_CreateRGBSurface(
                            0,
                            w,
                            h,
                            f.BitsPerPixel as i32,
                            f.Rmask,
                            f.Gmask,
                            f.Bmask,
                            f.Amask,
                        );
                        assert_eq!(
                            rotozoom::rotateSurface(src, dst, a, smooth),
                            0,
                            "{}",
                            case.name
                        );
                        dst
                    }
                    _ => ptr::null_mut(),
                }
            };
            assert!(!dst.is_null(), "{}", case.name);
            assert_eq!(describe(dst), case.expected, "{}", case.name);
            if dst != src {
                SDL_FreeSurface(dst);
            }
            SDL_FreeSurface(src);
        }
    }
}
