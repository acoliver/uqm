#!/usr/bin/env python3
"""Compile only the unchanged original RenderTopography body for RAM parity.

The supplied source is retained original plangen.c. All graphics operations
are injected function pointers to the linked test's actual context renderer.
No graphics implementation or resource effect is supplied by this harness.
"""
import argparse
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("source", type=Path)
parser.add_argument("output", type=Path)
parser.add_argument("--evidence", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
args.output.parent.mkdir(parents=True, exist_ok=True)
args.evidence.mkdir(parents=True, exist_ok=True)
source = args.source.read_text()
start = source.index("static void\nRenderTopography (")
end = source.index("\nstatic inline void\nP3mult", start)
body = source[start:end]
header = '''#include "planets.h"
#define NUM_BATCH_POINTS 64
struct operations {
 FRAME (*frame)(FRAME);
 POINT (*origin)(POINT);
 BOOLEAN (*get_clip)(RECT *);
 BOOLEAN (*set_clip)(RECT *);
 BYTE *(*colors)(COLORMAP);
 void (*batch)(PRIMITIVE *, PRIM_LINKS, BATCH_FLAGS);
 const PlanetFrame *table;
 SOLARSYS_STATE *system;
};
static const struct operations *ops;
#define SetContextFGFrame ops->frame
#define SetContextOrigin ops->origin
#define GetContextClipRect ops->get_clip
#define SetContextClipRect ops->set_clip
#define GetColorMapAddress ops->colors
#define DrawBatch ops->batch
#define PlanData ops->table
#define pSolarSysState ops->system
'''
footer = '''
void original_render_topography(const struct operations *input,
 FRAME frame, SBYTE *depth, int w, int h) {
 ops = input;
 RenderTopography(frame, depth, w, h);
}
'''
unit = args.evidence / "original-render-topography.c.txt"
unit.write_text(header + body + footer)
command = ["clang", "-O2", "-std=gnu99", "-dynamiclib", "-x", "c",
           "-I" + str(root / "sc2/src/uqm/planets"),
           "-I" + str(root / "sc2/src"), "-I" + str(root / "sc2"),
           str(unit), "-o", str(args.output)]
print(" ".join(command), flush=True)
subprocess.run(command, check=True)
print(f"Compiled unchanged RenderTopography body ({len(body)} source bytes)")
