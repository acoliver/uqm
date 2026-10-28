// Portions Copyright (C) 2003-2005 Alex Volkov. GPL-2.0-or-later.
// Mechanical port of the Advanced BiAdaptive integer kernel.
use super::{blend_11, blend_1111, cmp_yuv, intensity, rgb_delta, Image};
use sdl2_sys::SDL_PixelFormat;
const RESOLVE_COORD: [[i32; 2]; 8] = [
    [0, -1],
    [1, -1],
    [2, 0],
    [2, 1],
    [1, 2],
    [0, 2],
    [-1, 1],
    [-1, 0],
];
pub(super) fn advanced(image: &Image<'_>, fmt: &SDL_PixelFormat, x: i32, y: i32) -> [u32; 4] {
    let w = image.width;
    let h = image.height;
    let mut pixels = [[0u32; 4]; 4];
    let mut out = [0u32; 4];
    let mut dst_p = 0usize;
    let mut cmatch: i32;
    cmatch = 0;
    pixels[1][1] = image.pixel(x, y);
    out[dst_p] = pixels[1][1];
    if (y + 1) < h {
        pixels[1][2] = image.pixel(x, y + 1);
        if pixels[1][1] == pixels[1][2] {
            out[dst_p + 2] = pixels[1][1];
            cmatch |= 1;
        }
    } else {
        pixels[1][2] = pixels[1][1];
        out[dst_p + 2] = pixels[1][1];
        cmatch |= 1;
    }

    if (x + 1) < w {
        pixels[2][1] = image.pixel(x + 1, y);
        if pixels[1][1] == pixels[2][1] {
            out[dst_p + 1] = pixels[1][1];
            cmatch |= 2;
        }
    } else {
        pixels[2][1] = pixels[1][1];
        out[dst_p + 1] = pixels[1][1];
        cmatch |= 2;
    }

    if cmatch == 3 {
        if ((y + 1) >= h) || ((x + 1) >= w) {
            dst_p += 1;
            out[dst_p + 2] = pixels[1][1];
            return out;
        }

        pixels[2][2] = image.pixel(x + 1, y + 1);
        if pixels[1][1] == pixels[2][2] {
            dst_p += 1;
            out[dst_p + 2] = pixels[1][1];
            return out;
        }
    }

    if x > 0 {
        pixels[0][1] = image.pixel(x + -1, y);
    } else {
        pixels[0][1] = pixels[1][1];
    }

    if (x + 2) < w {
        pixels[3][1] = image.pixel(x + 2, y);
    } else {
        pixels[3][1] = pixels[2][1];
    }

    if (y + 1) < h {
        if x > 0 {
            pixels[0][2] = image.pixel(x + -1, y + 1);
        } else {
            pixels[0][2] = pixels[1][2];
        }

        if (x + 2) < w {
            pixels[2][2] = image.pixel(x + 1, y + 1);
            pixels[3][2] = image.pixel(x + 2, y + 1);
        } else {
            if (x + 1) < w {
                pixels[2][2] = image.pixel(x + 1, y + 1);
                pixels[3][2] = pixels[2][2];
            } else {
                pixels[2][2] = pixels[1][2];
                pixels[3][2] = pixels[1][2];
            }
        }
    } else {
        pixels[0][2] = pixels[0][1];
        pixels[2][2] = pixels[2][1];
        pixels[3][2] = pixels[3][1];
    }

    if (y + 2) < h {
        pixels[1][3] = image.pixel(x, y + 2);
        if x > 0 {
            pixels[0][3] = image.pixel(x + -1, y + 2);
        } else {
            pixels[0][3] = pixels[1][3];
        }

        if (x + 2) < w {
            pixels[2][3] = image.pixel(x + 1, y + 2);
            pixels[3][3] = image.pixel(x + 2, y + 2);
        } else {
            if (x + 1) < w {
                pixels[2][3] = image.pixel(x + 1, y + 2);
                pixels[3][3] = pixels[2][3];
            } else {
                pixels[2][3] = pixels[1][3];
                pixels[3][3] = pixels[1][3];
            }
        }
    } else {
        pixels[0][3] = pixels[0][2];
        pixels[1][3] = pixels[1][2];
        pixels[2][3] = pixels[2][2];
        pixels[3][3] = pixels[3][2];
    }

    if y > 0 {
        pixels[1][0] = image.pixel(x, y + -1);
        if x > 0 {
            pixels[0][0] = image.pixel(x + -1, y + -1);
        } else {
            pixels[0][0] = pixels[1][0];
        }

        if (x + 2) < w {
            pixels[2][0] = image.pixel(x + 1, y + -1);
            pixels[3][0] = image.pixel(x + 2, y + -1);
        } else {
            if (x + 1) < w {
                pixels[2][0] = image.pixel(x + 1, y + -1);
                pixels[3][0] = pixels[2][0];
            } else {
                pixels[2][0] = pixels[1][0];
                pixels[3][0] = pixels[1][0];
            }
        }
    } else {
        pixels[0][0] = pixels[0][1];
        pixels[1][0] = pixels[1][1];
        pixels[2][0] = pixels[2][1];
        pixels[3][0] = pixels[3][1];
    }

    if (cmatch & 1) == 0 {
        if cmp_yuv(fmt, pixels[1][1], pixels[1][2], 30) {
            out[dst_p + 2] = blend_11(pixels[1][1], pixels[1][2]);
            cmatch |= 1;
        } else {
            if ((((pixels[1][1] == pixels[0][1]) && (pixels[1][1] == pixels[2][2]))
                && (pixels[1][1] == pixels[3][2]))
                && ((((!(rgb_delta(fmt, pixels[1][1], pixels[0][0]) <= 8000)
                    && !(rgb_delta(fmt, pixels[1][1], pixels[1][0]) <= 8000))
                    && !(rgb_delta(fmt, pixels[1][1], pixels[2][1]) <= 8000))
                    && !(rgb_delta(fmt, pixels[1][1], pixels[3][1]) <= 8000))
                    || ((!(rgb_delta(fmt, pixels[1][1], pixels[0][2]) <= 8000)
                        && !(rgb_delta(fmt, pixels[1][1], pixels[2][3]) <= 8000))
                        && !(rgb_delta(fmt, pixels[1][1], pixels[3][3]) <= 8000))))
                || ((((pixels[1][1] == pixels[2][1]) && (pixels[1][1] == pixels[0][2]))
                    && (pixels[1][1] == pixels[3][0]))
                    && (((!(rgb_delta(fmt, pixels[1][1], pixels[0][1]) <= 8000)
                        && !(rgb_delta(fmt, pixels[1][1], pixels[1][0]) <= 8000))
                        && !(rgb_delta(fmt, pixels[1][1], pixels[2][0]) <= 8000))
                        || ((!(rgb_delta(fmt, pixels[1][1], pixels[0][3]) <= 8000)
                            && !(rgb_delta(fmt, pixels[1][1], pixels[2][2]) <= 8000))
                            && !(rgb_delta(fmt, pixels[1][1], pixels[3][1]) <= 8000))))
            {
                out[dst_p + 2] = pixels[1][1];
            } else {
                if ((((pixels[1][2] == pixels[0][1]) && (pixels[1][2] == pixels[2][2]))
                    && (pixels[1][2] == pixels[3][3]))
                    && (((!(rgb_delta(fmt, pixels[1][2], pixels[0][0]) <= 8000)
                        && !(rgb_delta(fmt, pixels[1][2], pixels[2][1]) <= 8000))
                        && !(rgb_delta(fmt, pixels[1][2], pixels[3][2]) <= 8000))
                        || ((!(rgb_delta(fmt, pixels[1][2], pixels[0][2]) <= 8000)
                            && !(rgb_delta(fmt, pixels[1][2], pixels[1][3]) <= 8000))
                            && !(rgb_delta(fmt, pixels[1][2], pixels[2][3]) <= 8000))))
                    || ((((pixels[1][2] == pixels[2][1]) && (pixels[1][2] == pixels[0][2]))
                        && (pixels[1][2] == pixels[3][1]))
                        && (((!(rgb_delta(fmt, pixels[1][2], pixels[0][1]) <= 8000)
                            && !(rgb_delta(fmt, pixels[1][2], pixels[2][0]) <= 8000))
                            && !(rgb_delta(fmt, pixels[1][2], pixels[3][0]) <= 8000))
                            || (((!(rgb_delta(fmt, pixels[1][2], pixels[0][3]) <= 8000)
                                && !(rgb_delta(fmt, pixels[1][2], pixels[1][3]) <= 8000))
                                && !(rgb_delta(fmt, pixels[1][2], pixels[2][2]) <= 8000))
                                && !(rgb_delta(fmt, pixels[1][2], pixels[3][2]) <= 8000))))
                {
                    out[dst_p + 2] = pixels[1][2];
                } else {
                    out[dst_p + 2] = blend_11(pixels[1][1], pixels[1][2]);
                }
            }
        }
    }

    dst_p += 1;
    if (cmatch & 2) == 0 {
        if cmp_yuv(fmt, pixels[1][1], pixels[2][1], 30) {
            out[dst_p] = blend_11(pixels[1][1], pixels[2][1]);
            cmatch |= 2;
        } else {
            if ((((pixels[1][1] == pixels[2][0]) && (pixels[1][1] == pixels[1][2]))
                && (pixels[1][1] == pixels[0][3]))
                && (((!(rgb_delta(fmt, pixels[1][1], pixels[1][0]) <= 8000)
                    && !(rgb_delta(fmt, pixels[1][1], pixels[0][1]) <= 8000))
                    && !(rgb_delta(fmt, pixels[1][1], pixels[0][2]) <= 8000))
                    || ((!(rgb_delta(fmt, pixels[1][1], pixels[3][0]) <= 8000)
                        && !(rgb_delta(fmt, pixels[1][1], pixels[2][2]) <= 8000))
                        && !(rgb_delta(fmt, pixels[1][1], pixels[1][3]) <= 8000))))
                || ((((pixels[1][1] == pixels[1][0]) && (pixels[1][1] == pixels[2][2]))
                    && (pixels[1][1] == pixels[2][3]))
                    && ((((!(rgb_delta(fmt, pixels[1][1], pixels[0][0]) <= 8000)
                        && !(rgb_delta(fmt, pixels[1][1], pixels[0][1]) <= 8000))
                        && !(rgb_delta(fmt, pixels[1][1], pixels[1][2]) <= 8000))
                        && !(rgb_delta(fmt, pixels[1][1], pixels[1][3]) <= 8000))
                        || ((!(rgb_delta(fmt, pixels[1][1], pixels[2][0]) <= 8000)
                            && !(rgb_delta(fmt, pixels[1][1], pixels[3][2]) <= 8000))
                            && !(rgb_delta(fmt, pixels[1][1], pixels[3][3]) <= 8000))))
            {
                out[dst_p] = pixels[1][1];
            } else {
                if ((((pixels[2][1] == pixels[2][0]) && (pixels[2][1] == pixels[1][2]))
                    && (pixels[2][1] == pixels[1][3]))
                    && (((!(rgb_delta(fmt, pixels[2][1], pixels[1][0]) <= 8000)
                        && !(rgb_delta(fmt, pixels[2][1], pixels[0][2]) <= 8000))
                        && !(rgb_delta(fmt, pixels[2][1], pixels[0][3]) <= 8000))
                        || (((!(rgb_delta(fmt, pixels[2][1], pixels[3][0]) <= 8000)
                            && !(rgb_delta(fmt, pixels[2][1], pixels[3][1]) <= 8000))
                            && !(rgb_delta(fmt, pixels[2][1], pixels[2][2]) <= 8000))
                            && !(rgb_delta(fmt, pixels[2][1], pixels[2][3]) <= 8000))))
                    || ((((pixels[2][1] == pixels[1][0]) && (pixels[2][1] == pixels[2][2]))
                        && (pixels[2][1] == pixels[3][3]))
                        && (((!(rgb_delta(fmt, pixels[2][1], pixels[0][0]) <= 8000)
                            && !(rgb_delta(fmt, pixels[2][1], pixels[1][2]) <= 8000))
                            && !(rgb_delta(fmt, pixels[2][1], pixels[2][3]) <= 8000))
                            || ((!(rgb_delta(fmt, pixels[2][1], pixels[2][0]) <= 8000)
                                && !(rgb_delta(fmt, pixels[2][1], pixels[3][1]) <= 8000))
                                && !(rgb_delta(fmt, pixels[2][1], pixels[3][2]) <= 8000))))
                {
                    out[dst_p] = pixels[2][1];
                } else {
                    out[dst_p] = blend_11(pixels[1][1], pixels[2][1]);
                }
            }
        }
    }

    if (pixels[1][1] == pixels[2][2]) && (pixels[2][1] == pixels[1][2]) {
        let mut cl: i32;
        let mut cr: i32;
        let mut clr: u32;
        cr = 2;
        cl = cr;
        for coord in RESOLVE_COORD {
            clr = pixels[(coord[0_usize] + 1) as usize][(coord[1_usize] + 1) as usize];
            if clr == pixels[1][1] || cmp_yuv(fmt, clr, pixels[1][1], 70) {
                cl += 1;
            } else {
                if clr == pixels[2][1] || cmp_yuv(fmt, clr, pixels[2][1], 70) {
                    cr += 1;
                }
            }
        }
        if cl > cr {
            clr = pixels[2][1];
        } else {
            if cr > cl {
                clr = pixels[1][1];
            } else {
                clr = blend_11(pixels[1][1], pixels[2][1]);
            }
        }

        out[dst_p + 2] = clr;
        return out;
    }

    if (cmatch == 3)
        || ((pixels[2][1] == pixels[1][2] || cmp_yuv(fmt, pixels[2][1], pixels[1][2], 30))
            && (pixels[2][1] == pixels[2][2] || cmp_yuv(fmt, pixels[2][1], pixels[2][2], 30)))
    {
        out[dst_p + 2] = blend_11(pixels[1][2], pixels[2][1]);
        return out;
    } else {
        if (cmatch != 0)
            && (pixels[1][1] == pixels[2][2] || cmp_yuv(fmt, pixels[1][1], pixels[2][2], 30))
        {
            out[dst_p + 2] = blend_11(pixels[1][1], pixels[2][2]);
            return out;
        }
    }

    if (pixels[1][1] == pixels[2][2] || cmp_yuv(fmt, pixels[1][1], pixels[2][2], 130))
        && (pixels[2][1] == pixels[1][2] || cmp_yuv(fmt, pixels[2][1], pixels[1][2], 130))
    {
        if intensity(fmt, pixels[1][1]) > intensity(fmt, pixels[2][1]) {
            out[dst_p + 2] = blend_11(pixels[1][1], pixels[2][2]);
        } else {
            out[dst_p + 2] = blend_11(pixels[2][1], pixels[1][2]);
        }
    } else {
        if pixels[1][1] == pixels[2][2] || cmp_yuv(fmt, pixels[1][1], pixels[2][2], 130) {
            out[dst_p + 2] = blend_11(pixels[1][1], pixels[2][2]);
        } else {
            if pixels[2][1] == pixels[1][2] || cmp_yuv(fmt, pixels[2][1], pixels[1][2], 130) {
                out[dst_p + 2] = blend_11(pixels[2][1], pixels[1][2]);
            } else {
                out[dst_p + 2] = blend_1111(pixels[1][1], pixels[1][2], pixels[2][1], pixels[2][2]);
            }
        }
    }

    out
}
