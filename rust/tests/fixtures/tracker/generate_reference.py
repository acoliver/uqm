import ctypes as C
from pathlib import Path
import json, math, hashlib
ROOT = Path(__file__).resolve().parents[4]
OUT = ROOT / 'tmp/issue34-final-20261008/continuation-commfix/decoder'
OUT.mkdir(parents=True, exist_ok=True)
FIX = ROOT / 'rust/tests/fixtures/tracker'
FIX.mkdir(parents=True, exist_ok=True)
lib = C.CDLL('/opt/homebrew/lib/libopenmpt.dylib')
def bind(name,args,result):
    f=getattr(lib,'openmpt_module_'+name);f.argtypes=args;f.restype=result;return f
create=bind('create_from_memory2',[C.c_void_p,C.c_size_t]+[C.c_void_p]*7,C.c_void_p)
destroy=bind('destroy',[C.c_void_p],None)
duration=bind('get_duration_seconds',[C.c_void_p],C.c_double)
param=bind('set_render_param',[C.c_void_p,C.c_int,C.c_int],C.c_int)
ctl=bind('ctl_set_boolean',[C.c_void_p,C.c_char_p,C.c_int],C.c_int)
text=bind('ctl_set_text',[C.c_void_p,C.c_char_p,C.c_char_p],C.c_int)
read=bind('read_interleaved_float_stereo',[C.c_void_p,C.c_int,C.c_size_t,C.POINTER(C.c_float)],C.c_size_t)
def module(notes, sample_len=256, loop=True):
    d=bytearray(1084+1024+sample_len)
    d[42:44]=(sample_len//2).to_bytes(2,'big');d[45]=64
    if loop:d[48:50]=(sample_len//2).to_bytes(2,'big')
    d[950]=1;d[951]=127;d[1080:1084]=b'M.K.'
    for row,ch,samp,period,effect,arg in notes:
        o=1084+(row*4+ch)*4
        d[o:o+4]=bytes([(samp&0xf0)|(period>>8),period&255,(samp<<4&0xf0)|effect,arg])
    for i in range(sample_len):d[2108+i]=round(100*math.sin(2*math.pi*i/32))&255
    return bytes(d)
cases={
    'low-b':module([(0,0,1,907,0,0),(7,0,0,0,13,0)]),
    'short-periods':module([(0,0,1,53,15,1),(1,0,1,40,0,0),(2,0,1,36,0,0),(3,0,1,24,0,0),(4,0,1,12,0,0),(5,0,1,3,0,0),(6,0,0,0,13,0)]),
    'extended-slide':module([(0,0,1,1712,2,192),(1,0,0,0,1,192),(2,0,0,0,13,0)]),
    'off-grid':module([(0,0,1,381,0,0),(1,0,1,286,0,0),(2,0,1,1143,0,0),(3,0,1,321,0,0),(4,0,0,0,13,0)]),
    'pcm':module([(0,0,1,428,14,1),(1,0,0,0,0,0x37),(2,0,0,0,1,3),(3,0,0,0,2,4),(4,0,0,0,10,4),(5,0,0,0,12,32),(6,0,0,0,15,3),(7,0,0,0,13,0)]),
    'delay':module([(0,0,1,428,14,0xe2),(1,0,0,0,13,0)]),
    'loop':module([(0,0,1,428,14,0x60),(1,0,0,0,14,0x62),(2,0,0,0,13,0)]),
    'jump':module([(0,0,1,428,0,0),(1,0,0,0,11,0)]),
    'tempo':module([(0,0,1,428,15,150),(1,0,0,0,15,3),(2,0,0,0,13,0)]),
    'fractional-tempo':module([(0,0,1,428,15,137),(1,0,0,0,15,3),(2,0,0,0,13,0)]),
    'mix':module([(0,ch,1,428,14,1) for ch in range(4)]+[(1,0,0,0,13,0)]),
    'finite':module([(0,0,1,428,0,0),(1,0,0,0,13,0)],sample_len=256,loop=False),
}
empty_word = bytearray(module([(0,0,1,428,0,0),(1,0,3,428,0,0),(2,0,0,0,13,0)]))
empty_word[72:74] = (1).to_bytes(2,'big')
empty_word[102:104] = (128).to_bytes(2,'big')
empty_word[105] = 64
empty_word[108:110] = (128).to_bytes(2,'big')
empty_word.extend(bytes((-v)&255 for v in empty_word[2108:2364]))
empty_word.extend(b'\0\0')
cases['empty-word'] = bytes(empty_word)
results=[]
for name,data in cases.items():
    (FIX/(name+'.mod')).write_bytes(data)
    b=C.create_string_buffer(data);m=create(b,len(data),None,None,None,None,None,None,None)
    assert m
    for p,v in [(2,200),(3,1),(4,0)]:assert param(m,p,v)
    assert ctl(m,b'render.resampler.emulate_amiga',0)
    assert text(m,b'play.at_end',b'stop')
    n=44100 if name != 'finite' else 4096
    pcm=(C.c_float*(n*2))();got=read(m,44100,n,pcm)
    (FIX/(name+'.f32le')).write_bytes(bytes(pcm)[:got*8])
    total_frames = got
    tail = (C.c_float*88200)()
    while True:
        count = read(m,44100,44100,tail)
        total_frames += count
        if count == 0: break
    r=dict(name=name,duration=total_frames/44100,estimated_duration=duration(m),total_frames=total_frames,frames=got,peak=max(abs(x) for x in pcm),sha256=hashlib.sha256(data).hexdigest())
    results.append(r);destroy(m)
manifest=dict(library='/opt/homebrew/lib/libopenmpt.dylib',reference='libopenmpt nearest-neighbor, no ramping, Amiga emulator off, stereo separation 200%, gain default',cases=results)
(FIX/'reference.json').write_text(json.dumps(manifest,indent=2)+'\n')
(OUT/'reference-generation.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps(manifest,indent=2))
