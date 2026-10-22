#!/usr/bin/env python3
"""Build a read-only test oracle from byte-unchanged plangen.c sphere bodies."""
import argparse
import hashlib
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--evidence", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
source_path = root / "sc2/src/uqm/planets/plangen.c"
source = source_path.read_text()
start = source.index("static inline void\nP3mult (")
end = source.index("\n//CreateShieldMask", start)
bodies = source[start:end]
assert hashlib.sha256(bodies.encode()).hexdigest() == "df7feecb5c700ca27b71a9ed9ab69cf08e6af89c62b338359eb1f74adf8e3f39"
macros = source[source.index("#define NUM_BATCH_POINTS 64"):source.index("DWORD light_diff")]
map_type = source[source.index("typedef struct \n{"):source.index("MAP3D_POINT map_rotate")]
p3_start = source.index("typedef struct\n{\n\tdouble x, y, z;")
p3_type = source[p3_start:source.index("static void\nRenderTopography", p3_start)]
header = r'''#include <stdint.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <limits.h>
#include <math.h>
#include "planets.h"
'''
# Use the header's POINT/COORD/DWORD/Color and the source's actual record types.
# Only the storage declarations change; the extracted function bodies do not.
guards = r'''
_Static_assert(sizeof(COORD) == 2 && (COORD)-1 < 0, "signed COORD");
_Static_assert(sizeof(POINT) == 4 && offsetof(POINT, x) == 0 && offsetof(POINT, y) == 2, "POINT");
_Static_assert(sizeof(DWORD) == 4 && (DWORD)-1 > 0, "DWORD");
_Static_assert(sizeof(Color) == 4 && offsetof(Color, r) == 0 && offsetof(Color, g) == 1 && offsetof(Color, b) == 2 && offsetof(Color, a) == 3, "Color");
_Static_assert(sizeof(MAP3D_POINT) == 32 && offsetof(MAP3D_POINT, p) == 0 && offsetof(MAP3D_POINT, m) == 16, "MAP3D_POINT");
_Static_assert(sizeof(POINT3) == 24 && offsetof(POINT3, x) == 0 && offsetof(POINT3, y) == 8 && offsetof(POINT3, z) == 16, "POINT3");
_Static_assert(sizeof(double) == 8 && sizeof(int) == 4 && SAFE_Y == 0 && MAP_HEIGHT == 75 && SPHERE_SPAN_X == 75 && DIAMETER == 75, "sphere dimensions");
struct MaskStorage { uint32_t before[16]; DWORD value[DIAMETER][DIAMETER]; uint32_t after[16]; };
struct TiltStorage { uint32_t before[16]; MAP3D_POINT value[DIAMETER][DIAMETER]; uint32_t after[16]; };
struct AaStorage { uint32_t before[16]; MAP3D_POINT value; uint32_t after[16]; };
_Static_assert(offsetof(struct MaskStorage, value) == 64 && offsetof(struct MaskStorage, after) == 64 + 75*75*4, "mask guards adjacent");
_Static_assert(offsetof(struct TiltStorage, value) == 64 && offsetof(struct TiltStorage, after) == 64 + 75*75*32, "tilt guards adjacent");
_Static_assert(offsetof(struct AaStorage, value) == 64 && offsetof(struct AaStorage, after) == 96, "AA guards adjacent");
static struct MaskStorage guarded_mask;
static struct TiltStorage guarded_tilt;
#define light_diff guarded_mask.value
#define map_rotate guarded_tilt.value
'''
wrapper = r'''
#define CAPACITY (4*1024*1024)
struct Capture { uint32_t before[16]; uint8_t value[CAPACITY]; uint32_t after[16]; };
_Static_assert(offsetof(struct Capture, value) == 64 && offsetof(struct Capture, after) == 64 + CAPACITY, "capture guards adjacent");
static struct Capture capture;
static uint8_t guard_output[65536];
static size_t used, guard_used;
static void init_guards(uint32_t *before, uint32_t *after) {
    for (unsigned i=0; i<16; ++i) before[i] = after[i] = 0x51a7cafeu;
}
static void put(uint64_t value, unsigned bytes) {
    assert(used + bytes <= CAPACITY);
    for (unsigned i=0; i<bytes; ++i) capture.value[used++] = (uint8_t)(value >> (8*i));
}
static void put_double(double value) {
    uint64_t bits; memcpy(&bits, &value, 8); put(bits, 8);
}
static void put_p3(POINT3 value) { put_double(value.x); put_double(value.y); put_double(value.z); }
static void put_point(MAP3D_POINT *value) {
    for (unsigned i=0; i<4; ++i) { put((uint16_t)value->p[i].x,2); put((uint16_t)value->p[i].y,2); }
    for (unsigned i=0; i<4; ++i) put(value->m[i],4);
}
static void record_guards(const uint32_t *before, const uint32_t *after) {
    assert(guard_used + 128 <= sizeof(guard_output));
    for (unsigned side=0; side<2; ++side) for (unsigned i=0; i<16; ++i) {
        uint32_t value = (side ? after : before)[i];
        assert(value == 0x51a7cafeu);
        for (unsigned j=0; j<4; ++j) guard_output[guard_used++] = (uint8_t)(value >> (8*j));
    }
}
struct Reader {const uint8_t *bytes; size_t length, offset;};
static uint64_t get(struct Reader *r, unsigned n) {
    assert(r->offset + n <= r->length);
    uint64_t value=0;
    for(unsigned i=0; i<n; ++i) value |= (uint64_t)r->bytes[r->offset++] << (8*i);
    return value;
}
static double get_double(struct Reader *r) {uint64_t bits=get(r,8); double value; memcpy(&value,&bits,8); assert(isfinite(value)); return value;}
static int16_t get_i16(struct Reader *r) {uint16_t bits=(uint16_t)get(r,2); int16_t value; memcpy(&value,&bits,2); return value;}
static int32_t get_i32(struct Reader *r) {uint32_t bits=(uint32_t)get(r,4); int32_t value; memcpy(&value,&bits,4); return value;}
static POINT3 get_p3(struct Reader *r) {POINT3 value; value.x=get_double(r); value.y=get_double(r); value.z=get_double(r); return value;}
size_t original_sphere_geometry(uint8_t *out, size_t capacity, const uint8_t *input, size_t input_length) {
    used = guard_used = 0;
    init_guards(capture.before,capture.after);
    struct Reader r={input,input_length,0};
    while(r.offset < r.length) {
        switch(get(&r,1)) {
        case 1: {
            unsigned alias=(unsigned)get(&r,1); assert(alias<=2);
            POINT3 a=get_p3(&r), b=get_p3(&r), result, first, second;
            const POINT3 saved_a=a, saved_b=b;
            double k=get_double(&r);
            assert(P3dot(&a,&a)>0 && isfinite(P3dot(&a,&a)));
            first=a; second=b;
            if(alias) {P3mult(&first,&first,k); result=first;} else P3mult(&result,&first,k);
            put_p3(result); assert(memcmp(&second,&b,sizeof(b))==0);
            if(!alias) assert(memcmp(&first,&a,sizeof(a))==0);
            first=a; second=b;
            if(alias==1) {P3sub(&first,&first,&second); result=first;}
            else if(alias==2) {P3sub(&second,&first,&second); result=second;}
            else P3sub(&result,&first,&second);
            put_p3(result);
            if(alias!=1) assert(memcmp(&first,&a,sizeof(a))==0);
            if(alias!=2) assert(memcmp(&second,&b,sizeof(b))==0);
            put_double(P3dot(&a,&b));
            first=a;
            if(alias) {P3norm(&first,&first); result=first;} else P3norm(&result,&first);
            put_p3(result);
            if(!alias) assert(memcmp(&first,&a,sizeof(a))==0);
            assert(memcmp(&a,&saved_a,sizeof(a))==0);
            assert(memcmp(&b,&saved_b,sizeof(b))==0);
            break;
        }
        case 2: {
            POINT loc; loc.x=get_i16(&r); loc.y=get_i16(&r);
            int64_t squared=(int64_t)loc.x*loc.x+(int64_t)loc.y*loc.y;
            assert(squared>0 && squared<=INT_MAX);
            init_guards(guarded_mask.before,guarded_mask.after);
            memset(light_diff,0xa5,sizeof(light_diff));
            GenerateSphereMask(loc);
            for(unsigned y=0;y<DIAMETER;++y) for(unsigned x=0;x<DIAMETER;++x) put(light_diff[y][x],4);
            record_guards(guarded_mask.before,guarded_mask.after);
            break;
        }
        case 3: {
            int angle=get_i32(&r);
            init_guards(guarded_tilt.before,guarded_tilt.after);
            memset(map_rotate,0xa5,sizeof(map_rotate));
            CreateSphereTiltMap(angle);
            for(unsigned y=0;y<DIAMETER;++y) for(unsigned x=0;x<DIAMETER;++x) put_point(&map_rotate[y][x]);
            record_guards(guarded_tilt.before,guarded_tilt.after);
            break;
        }
        case 4: {
            double x=get_double(&r), y=get_double(&r);
            struct AaStorage point;
            init_guards(point.before,point.after); memset(&point.value,0x6d,sizeof(point.value));
            create_aa_points(&point.value,x,y); put_point(&point.value);
            record_guards(point.before,point.after);
            break;
        }
        case 5: {
            Color colors[4]={{255,1,2,3},{17,230,31,4},{91,44,178,5},{0,255,128,6}};
            DWORD weights[4]; for(unsigned i=0;i<4;++i) weights[i]=(DWORD)get(&r,4);
            Color before_colors[4]; DWORD before_weights[4];
            memcpy(before_colors,colors,sizeof(colors)); memcpy(before_weights,weights,sizeof(weights));
            for(int channel=-1;channel<=3;++channel) {
                for(unsigned i=0;i<4;++i) put(get_color_channel(colors[i],channel),1);
                put(get_avg_channel(colors,weights,channel),1);
            }
            assert(memcmp(before_colors,colors,sizeof(colors))==0);
            assert(memcmp(before_weights,weights,sizeof(weights))==0);
            break;
        }
        default: abort();
        }
    }
    for(size_t i=0;i<guard_used;++i) put(guard_output[i],1);
    record_guards(capture.before,capture.after);
    for(size_t i=guard_used-128;i<guard_used;++i) put(guard_output[i],1);
    for(size_t i=0;i<input_length;++i) put(input[i],1);
    assert(used<=capacity);
    memcpy(out,capture.value,used);
    return used;
}
'''
args.evidence.mkdir(parents=True, exist_ok=True)
args.output.parent.mkdir(parents=True, exist_ok=True)
harness = args.evidence / "sphere-geometry-original-c.c"
harness.write_text(header + macros + map_type + p3_type + guards + bodies + wrapper)
command = ["clang", "-O2", "-std=gnu99", "-dynamiclib", "-x", "c",
           "-I" + str(root / "sc2/src/uqm/planets"),
           "-I" + str(root / "sc2/src"), "-I" + str(root / "sc2"),
           str(harness), "-o", str(args.output)]
environment = os.environ.copy()
for variable in ("SDKROOT", "MACOSX_DEPLOYMENT_TARGET"):
    environment.pop(variable, None)
assert environment["CARGO_TARGET_DIR"] == "/Volumes/XS1000/acoliver/projects/uqm/tmp/build/current"
assert environment["CARGO_BUILD_JOBS"] == "4"
assert environment["TMPDIR"] == "/Volumes/XS1000/acoliver/projects/uqm/tmp/sg"
print("original plangen.c sha256:", hashlib.sha256(source_path.read_bytes()).hexdigest())
print("extracted bodies sha256:", hashlib.sha256(bodies.encode()).hexdigest())
print("compile:", " ".join(command), flush=True)
subprocess.run(command, check=True, env=environment)
print("compiled original bodies bytes:", len(bodies.encode()))
