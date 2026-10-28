// Copyright (C) 2003 Maxim Stepin; portions (C) 2005 Alex Volkov.
// GPL-2.0-or-later. Mechanical port of all 256 hq2x switch patterns.
use super::{blend_211, blend_233, blend_31, blend_521, blend_611, blend_e11};
pub(super) fn rules(pattern: u8, edges: [bool; 4], pix: &[u32; 10]) -> [u32; 4] {
    let mut out = [0u32; 4];
    match pattern {
        0 | 1 | 4 | 32 | 128 | 5 | 132 | 160 | 33 | 129 | 36 | 133 | 164 | 161 | 37 | 165 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        2 | 34 | 130 | 162 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        16 | 17 | 48 | 49 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        64 | 65 | 68 | 69 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        8 | 12 | 136 | 140 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        3 | 35 | 131 | 163 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        6 | 38 | 134 | 166 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        20 | 21 | 52 | 53 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        144 | 145 | 176 | 177 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        192 | 193 | 196 | 197 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        96 | 97 | 100 | 101 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        40 | 44 | 168 | 172 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        9 | 13 | 137 | 141 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        18 | 50 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        80 | 81 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        72 | 76 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        10 | 138 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        66 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        24 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        7 | 39 | 135 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        148 | 149 | 180 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        224 | 228 | 225 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        41 | 169 | 45 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        22 | 54 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        208 | 209 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        104 | 108 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        11 | 139 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        19 | 51 => {
            if edges[1] {
                out[0] = blend_31(pix[5], pix[4]);
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[0] = blend_521(pix[5], pix[2], pix[4]);
                out[1] = blend_233(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        146 | 178 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
                out[3] = blend_31(pix[5], pix[8]);
            } else {
                out[1] = blend_233(pix[5], pix[2], pix[6]);
                out[3] = blend_521(pix[5], pix[6], pix[8]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
        }
        84 | 85 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            if edges[3] {
                out[1] = blend_31(pix[5], pix[2]);
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[1] = blend_521(pix[5], pix[6], pix[2]);
                out[3] = blend_233(pix[5], pix[6], pix[8]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
        }
        112 | 113 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[3] {
                out[2] = blend_31(pix[5], pix[4]);
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[2] = blend_521(pix[5], pix[8], pix[4]);
                out[3] = blend_233(pix[5], pix[6], pix[8]);
            }
        }
        200 | 204 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
                out[3] = blend_31(pix[5], pix[6]);
            } else {
                out[2] = blend_233(pix[5], pix[8], pix[4]);
                out[3] = blend_521(pix[5], pix[8], pix[6]);
            }
        }
        73 | 77 => {
            if edges[2] {
                out[0] = blend_31(pix[5], pix[2]);
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[0] = blend_521(pix[5], pix[4], pix[2]);
                out[2] = blend_233(pix[5], pix[8], pix[4]);
            }
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        42 | 170 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
                out[2] = blend_31(pix[5], pix[8]);
            } else {
                out[0] = blend_233(pix[5], pix[4], pix[2]);
                out[2] = blend_521(pix[5], pix[4], pix[8]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        14 | 142 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
                out[1] = blend_31(pix[5], pix[6]);
            } else {
                out[0] = blend_233(pix[5], pix[4], pix[2]);
                out[1] = blend_521(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        67 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        70 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        28 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        152 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        194 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        98 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        56 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        25 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        26 | 31 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        82 | 214 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        88 | 248 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        74 | 107 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        27 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[3]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        86 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_31(pix[5], pix[9]);
        }
        216 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_31(pix[5], pix[7]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        106 => {
            out[0] = blend_31(pix[5], pix[1]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        30 => {
            out[0] = blend_31(pix[5], pix[1]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        210 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_31(pix[5], pix[3]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        120 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[9]);
        }
        75 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_31(pix[5], pix[7]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        29 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        198 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        184 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        99 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        57 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        71 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        156 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        226 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        60 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        195 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        102 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        153 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        58 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        83 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        92 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        202 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        78 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        154 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        114 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        89 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        90 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        55 | 23 => {
            if edges[1] {
                out[0] = blend_31(pix[5], pix[4]);
                out[1] = pix[5];
            } else {
                out[0] = blend_521(pix[5], pix[2], pix[4]);
                out[1] = blend_233(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        182 | 150 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = pix[5];
                out[3] = blend_31(pix[5], pix[8]);
            } else {
                out[1] = blend_233(pix[5], pix[2], pix[6]);
                out[3] = blend_521(pix[5], pix[6], pix[8]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
        }
        213 | 212 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            if edges[3] {
                out[1] = blend_31(pix[5], pix[2]);
                out[3] = pix[5];
            } else {
                out[1] = blend_521(pix[5], pix[6], pix[2]);
                out[3] = blend_233(pix[5], pix[6], pix[8]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
        }
        241 | 240 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[3] {
                out[2] = blend_31(pix[5], pix[4]);
                out[3] = pix[5];
            } else {
                out[2] = blend_521(pix[5], pix[8], pix[4]);
                out[3] = blend_233(pix[5], pix[6], pix[8]);
            }
        }
        236 | 232 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            if edges[2] {
                out[2] = pix[5];
                out[3] = blend_31(pix[5], pix[6]);
            } else {
                out[2] = blend_233(pix[5], pix[8], pix[4]);
                out[3] = blend_521(pix[5], pix[8], pix[6]);
            }
        }
        109 | 105 => {
            if edges[2] {
                out[0] = blend_31(pix[5], pix[2]);
                out[2] = pix[5];
            } else {
                out[0] = blend_521(pix[5], pix[4], pix[2]);
                out[2] = blend_233(pix[5], pix[8], pix[4]);
            }
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        171 | 43 => {
            if edges[0] {
                out[0] = pix[5];
                out[2] = blend_31(pix[5], pix[8]);
            } else {
                out[0] = blend_233(pix[5], pix[4], pix[2]);
                out[2] = blend_521(pix[5], pix[4], pix[8]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        143 | 15 => {
            if edges[0] {
                out[0] = pix[5];
                out[1] = blend_31(pix[5], pix[6]);
            } else {
                out[0] = blend_233(pix[5], pix[4], pix[2]);
                out[1] = blend_521(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        124 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[9]);
        }
        203 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_31(pix[5], pix[7]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        62 => {
            out[0] = blend_31(pix[5], pix[1]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        211 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[3]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        118 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[9]);
        }
        217 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_31(pix[5], pix[7]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        110 => {
            out[0] = blend_31(pix[5], pix[1]);
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        155 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[3]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        188 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        185 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        61 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        157 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        103 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        227 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        230 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        199 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        220 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        158 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        234 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        242 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        59 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        121 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        87 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        79 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        122 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        94 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        218 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        91 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        229 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        167 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        173 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        181 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        186 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        115 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        93 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        206 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        205 | 201 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            if edges[2] {
                out[2] = blend_31(pix[5], pix[7]);
            } else {
                out[2] = blend_611(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        174 | 46 => {
            if edges[0] {
                out[0] = blend_31(pix[5], pix[1]);
            } else {
                out[0] = blend_611(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        179 | 147 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = blend_31(pix[5], pix[3]);
            } else {
                out[1] = blend_611(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        117 | 116 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = blend_31(pix[5], pix[9]);
            } else {
                out[3] = blend_611(pix[5], pix[6], pix[8]);
            }
        }
        189 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        231 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        126 => {
            out[0] = blend_31(pix[5], pix[1]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[9]);
        }
        219 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[3]);
            out[2] = blend_31(pix[5], pix[7]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        125 => {
            if edges[2] {
                out[0] = blend_31(pix[5], pix[2]);
                out[2] = pix[5];
            } else {
                out[0] = blend_521(pix[5], pix[4], pix[2]);
                out[2] = blend_233(pix[5], pix[8], pix[4]);
            }
            out[1] = blend_31(pix[5], pix[2]);
            out[3] = blend_31(pix[5], pix[9]);
        }
        221 => {
            out[0] = blend_31(pix[5], pix[2]);
            if edges[3] {
                out[1] = blend_31(pix[5], pix[2]);
                out[3] = pix[5];
            } else {
                out[1] = blend_521(pix[5], pix[6], pix[2]);
                out[3] = blend_233(pix[5], pix[6], pix[8]);
            }
            out[2] = blend_31(pix[5], pix[7]);
        }
        207 => {
            if edges[0] {
                out[0] = pix[5];
                out[1] = blend_31(pix[5], pix[6]);
            } else {
                out[0] = blend_233(pix[5], pix[4], pix[2]);
                out[1] = blend_521(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[7]);
            out[3] = blend_31(pix[5], pix[6]);
        }
        238 => {
            out[0] = blend_31(pix[5], pix[1]);
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = pix[5];
                out[3] = blend_31(pix[5], pix[6]);
            } else {
                out[2] = blend_233(pix[5], pix[8], pix[4]);
                out[3] = blend_521(pix[5], pix[8], pix[6]);
            }
        }
        190 => {
            out[0] = blend_31(pix[5], pix[1]);
            if edges[1] {
                out[1] = pix[5];
                out[3] = blend_31(pix[5], pix[8]);
            } else {
                out[1] = blend_233(pix[5], pix[2], pix[6]);
                out[3] = blend_521(pix[5], pix[6], pix[8]);
            }
            out[2] = blend_31(pix[5], pix[8]);
        }
        187 => {
            if edges[0] {
                out[0] = pix[5];
                out[2] = blend_31(pix[5], pix[8]);
            } else {
                out[0] = blend_233(pix[5], pix[4], pix[2]);
                out[2] = blend_521(pix[5], pix[4], pix[8]);
            }
            out[1] = blend_31(pix[5], pix[3]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        243 => {
            out[0] = blend_31(pix[5], pix[4]);
            out[1] = blend_31(pix[5], pix[3]);
            if edges[3] {
                out[2] = blend_31(pix[5], pix[4]);
                out[3] = pix[5];
            } else {
                out[2] = blend_521(pix[5], pix[8], pix[4]);
                out[3] = blend_233(pix[5], pix[6], pix[8]);
            }
        }
        119 => {
            if edges[1] {
                out[0] = blend_31(pix[5], pix[4]);
                out[1] = pix[5];
            } else {
                out[0] = blend_521(pix[5], pix[2], pix[4]);
                out[1] = blend_233(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            out[3] = blend_31(pix[5], pix[9]);
        }
        237 | 233 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[2], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        175 | 47 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[6], pix[8]);
        }
        183 | 151 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[8], pix[4]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        245 | 244 => {
            out[0] = blend_211(pix[5], pix[4], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
        250 => {
            out[0] = blend_31(pix[5], pix[1]);
            out[1] = blend_31(pix[5], pix[3]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        123 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[3]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[9]);
        }
        95 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[7]);
            out[3] = blend_31(pix[5], pix[9]);
        }
        222 => {
            out[0] = blend_31(pix[5], pix[1]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[7]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        252 => {
            out[0] = blend_211(pix[5], pix[1], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
        249 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_211(pix[5], pix[3], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        235 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_211(pix[5], pix[3], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        111 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_211(pix[5], pix[9], pix[6]);
        }
        63 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_211(pix[5], pix[9], pix[8]);
        }
        159 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        215 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_211(pix[5], pix[7], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        246 => {
            out[0] = blend_211(pix[5], pix[1], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
        254 => {
            out[0] = blend_31(pix[5], pix[1]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
        253 => {
            out[0] = blend_31(pix[5], pix[2]);
            out[1] = blend_31(pix[5], pix[2]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
        251 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[3]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        239 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            out[1] = blend_31(pix[5], pix[6]);
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[6]);
        }
        127 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_211(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_211(pix[5], pix[8], pix[4]);
            }
            out[3] = blend_31(pix[5], pix[9]);
        }
        191 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[8]);
            out[3] = blend_31(pix[5], pix[8]);
        }
        223 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_211(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[7]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_211(pix[5], pix[6], pix[8]);
            }
        }
        247 => {
            out[0] = blend_31(pix[5], pix[4]);
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            out[2] = blend_31(pix[5], pix[4]);
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
        255 => {
            if edges[0] {
                out[0] = pix[5];
            } else {
                out[0] = blend_e11(pix[5], pix[4], pix[2]);
            }
            if edges[1] {
                out[1] = pix[5];
            } else {
                out[1] = blend_e11(pix[5], pix[2], pix[6]);
            }
            if edges[2] {
                out[2] = pix[5];
            } else {
                out[2] = blend_e11(pix[5], pix[8], pix[4]);
            }
            if edges[3] {
                out[3] = pix[5];
            } else {
                out[3] = blend_e11(pix[5], pix[6], pix[8]);
            }
        }
    }
    out
}
