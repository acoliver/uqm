#!/usr/bin/env python3
"""Record the pre-port C scaler output. Usage: script ORIGINAL_CHECKOUT OUTPUT_DIR.

The input checkout must contain the seven original SDL scaler C sources.
Compilation and temporary files remain under OUTPUT_DIR. No C is linked into
Rust tests or the game by this fixture generator.
"""
import ctypes
import pathlib
import re
import struct
import subprocess
import sys
import tempfile

source = pathlib.Path(sys.argv[1]) / "sc2/src/libs/graphics/sdl"
out = pathlib.Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
names = ["nearest2x", "bilinear2x", "triscan2x", "biadv2x", "2xscalers", "hq2x", "scalers"]
functions = ["Nearest", "BilinearFilter", "TriScanFilter", "BiAdaptAdvFilter", "BiAdaptFilter", "HqFilter"]


def clean(text):
    return re.sub(r'^\s*#\s*include[^\n]*', '', text, flags=re.M)


preamble = '''#include <SDL.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
typedef int16_t sint16;
typedef int32_t sint32;
typedef void (*TFB_ScaleFunc)(SDL_Surface*,SDL_Surface*,SDL_Rect*);
typedef enum {PLATFORM_NULL, PLATFORM_C, PLATFORM_MMX, PLATFORM_SSE,
PLATFORM_3DNOW, PLATFORM_ALTIVEC} PLATFORM_TYPE;
#define USE_RUST_GFX 1
'''
for i, name in enumerate(["BILINEAR", "BIADAPT", "BIADAPTADV", "TRISCAN", "HQXX"]):
    preamble += f'#define TFB_GFXFLAGS_SCALE_{name} (1 << {i + 3})\n'
with tempfile.TemporaryDirectory(dir=out) as work:
    work = pathlib.Path(work)
    header = preamble + clean((source / 'scaleint.h').read_text()) + clean((source / '2xscalers.h').read_text())
    (work / 'support.h').write_text(header)
    files = []
    for name in names:
        path = work / (name + '.c')
        path.write_text('#include "support.h"\n' + clean((source / (name + '.c')).read_text()))
        files.append(str(path))
    hq = (source / 'hq2x.c').read_text()
    switch = hq[hq.index('switch (pattern)'):hq.rindex('\n\t\t}\n\t}')]
    defines = '\n'.join(line for line in hq.splitlines() if line.startswith('#define PIXEL'))
    edges = [('4', '2'), ('2', '6'), ('8', '4'), ('6', '8')]
    for bit, (a, b) in enumerate(edges):
        switch = re.sub(r'HQXX_DIFFYUV\s*\(yuv\[' + a + r'\], yuv\[' + b + r'\]\)', f'(edges & {1 << bit})', switch)
    assert 'HQXX_DIFFYUV' not in switch
    wrapper = '#include "support.h"\n' + defines + '''
void rules(int pattern, int edges, Uint32 *dst_p) {
    const int dlen = 2;
    Uint32 pix[10];
    for (int i=0;i<10;i++) pix[i] = 0x10203040u * i + 0x07050301u;
''' + switch + '\n}\n'
    wrapper += 'static TFB_ScaleFunc funcs[] = {' + ','.join('Scale_' + f for f in functions) + '};\n'
    wrapper += '''void scale(int kind, int w, int h, int sp, int dp,
            int format, Uint32 *src, Uint32 *dst, int *rect) {
    int shifts[4][3] = {{24,16,8},{16,8,0},{8,16,24},{0,8,16}};
    SDL_Surface *s = SDL_CreateRGBSurfaceFrom(src,w,h,32,sp*4,
        255u<<shifts[format][0],255u<<shifts[format][1],255u<<shifts[format][2],0);
    SDL_Surface *d = SDL_CreateRGBSurfaceFrom(dst,w*2,h*2,32,dp*4,
        255u<<shifts[format][0],255u<<shifts[format][1],255u<<shifts[format][2],0);
    SDL_Rect r = {rect[0],rect[1],rect[2],rect[3]};
    funcs[kind](s,d,&r);
    rect[0]=r.x; rect[1]=r.y; rect[2]=r.w; rect[3]=r.h;
    SDL_FreeSurface(s); SDL_FreeSurface(d);
}
'''
    (work / 'wrapper.c').write_text(wrapper)
    flags = subprocess.check_output(['pkg-config', '--cflags', '--libs', 'sdl2'], text=True).split()
    subprocess.run(['cc', '-shared', '-fPIC', '-O2', *files, str(work / 'wrapper.c'), *flags, '-o', str(work / 'oracle.dylib')], check=True)
    lib = ctypes.CDLL(str(work / 'oracle.dylib'))
    lib.Scale_Init()
    data = bytearray(b"HQ2X\0\0\0\0")
    for pattern in range(256):
        for edges in range(16):
            pixels = (ctypes.c_uint32 * 4)()
            lib.rules(pattern, edges, pixels)
            data += struct.pack('<4I', *pixels)
    (out / 'hq2x-rules.bin').write_bytes(data)
    data = bytearray()
    seed = 0x18A4E217
    for kind in range(6):
        for w, h in [(1, 1), (2, 3), (5, 5), (7, 6)]:
            for fmt in range(4):
                for variant in range(4):
                    sp, dp = w + 2, w * 2 + 3
                    src = []
                    for i in range(sp * h):
                        seed = (seed * 1664525 + 1013904223) & 0xffffffff
                        src.append(seed if variant == 0 else [0, 0xffffffff, 0x173583ff, 0x284694ff][(seed >> 28) % (2 if variant == 1 else 4)])
                    rect = [0, 0, w, h] if variant < 2 else [w // 2, h // 2, 1, 1]
                    before = rect[:]
                    r = (ctypes.c_int * 4)(*rect)
                    s = (ctypes.c_uint32 * len(src))(*src)
                    d = (ctypes.c_uint32 * (dp * h * 2))(*([0xdeadbeef] * (dp * h * 2)))
                    lib.scale(kind, w, h, sp, dp, fmt, s, d, r)
                    values = [kind,w,h,sp,dp,fmt,*before,*r,*src,*d]
                    data += struct.pack('<' + 'I' * len(values), *values)
    (out / 'software-scalers.bin').write_bytes(data)
