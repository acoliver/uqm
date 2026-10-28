// GPL-2.0-or-later. Mechanical port of the BiAdaptive integer kernel.
use super::{blend_11, blend_1111, Image};
pub(super) fn biadaptive(image: &Image<'_>, x: i32, y: i32) -> [u32; 4] {
    let w = image.width;
    let h = image.height;
    let mut out = [0u32; 4];
    let mut dst_p = 0usize;

    let pixval_bl;

    let pixval_tl = image.pixel(x, y);
    out[dst_p] = pixval_tl;
    if (y + 1) < h {
        pixval_bl = image.pixel(x, y + 1);
        if pixval_tl == pixval_bl {
            out[dst_p + 2] = pixval_tl;
        } else {
            out[dst_p + 2] = blend_11(pixval_tl, pixval_bl);
        }
    } else {
        out[dst_p + 2] = pixval_tl;
        pixval_bl = pixval_tl;
    }

    dst_p += 1;
    if (x + 1) >= w {
        out[dst_p] = pixval_tl;
        if pixval_tl == pixval_bl {
            out[dst_p + 2] = pixval_tl;
        } else {
            out[dst_p + 2] = blend_11(pixval_tl, pixval_bl);
        }

        return out;
    }

    let pixval_tr = image.pixel(x + 1, y);
    if pixval_tl == pixval_tr {
        out[dst_p] = pixval_tr;
    } else {
        out[dst_p] = blend_11(pixval_tl, pixval_tr);
    }

    if (y + 1) >= h {
        out[dst_p + 2] = pixval_tl;
        return out;
    }

    let pixval_br = image.pixel(x + 1, y + 1);
    if (pixval_tl == pixval_br) && (pixval_tr == pixval_bl) {
        let mut cl: i32;
        let mut cr: i32;
        let mut clr: u32;
        if pixval_tl == pixval_tr {
            out[dst_p + 2] = pixval_tl;
            return out;
        }

        cr = 1;
        cl = cr;
        if x > 0 {
            clr = image.pixel(x + -1, y);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }

            clr = image.pixel(x + -1, y + 1);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }
        }

        if y > 0 {
            clr = image.pixel(x, y + -1);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }

            clr = image.pixel(x + 1, y + -1);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }
        }

        if (x + 2) < w {
            clr = image.pixel(x + 2, y);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }

            clr = image.pixel(x + 2, y + 1);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }
        }

        if (y + 2) < h {
            clr = image.pixel(x, y + 2);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }

            clr = image.pixel(x + 1, y + 2);
            if clr == pixval_tl {
                cl += 1;
            } else {
                if clr == pixval_tr {
                    cr += 1;
                }
            }
        }

        if cl > cr {
            out[dst_p + 2] = pixval_tr;
        } else {
            if cr > cl {
                out[dst_p + 2] = pixval_tl;
            } else {
                out[dst_p + 2] = blend_11(pixval_tl, pixval_tr);
            }
        }
    } else {
        if pixval_tl == pixval_br {
            out[dst_p + 2] = pixval_tl;
        } else {
            if pixval_tr == pixval_bl {
                out[dst_p + 2] = pixval_tr;
            } else {
                out[dst_p + 2] = blend_1111(pixval_tl, pixval_bl, pixval_tr, pixval_br);
            }
        }
    }

    out
}
