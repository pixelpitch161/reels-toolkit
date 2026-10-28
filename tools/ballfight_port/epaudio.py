"""a delivered episode's own audio: python ../tools/ballfight_port/epaudio.py ballfight_<a>_vs_<b> <out.wav> [buses.npz]:
runs its audio.main() with audio.wav redirected to out.wav (the folder is not changed) and its master wrapped to capture
the pre-master buses"""
import os, sys, importlib.util
import numpy as np
folder = os.path.abspath(sys.argv[1])
sys.path.insert(0, folder)
os.chdir(folder)
from reelkit import audio as A
out, bus_out = sys.argv[2], (sys.argv[3] if len(sys.argv) > 3 else None)
_w = A.write
def write(path, x):
    if os.path.basename(path) == 'audio.wav':
        _w(out, x)
A.write = write
_sl = A.section_levels
A.section_levels = lambda path, spans: _sl(out if os.path.basename(path) == 'audio.wav' else path, spans)
spec = importlib.util.spec_from_file_location('audio_ep', os.path.join(folder, 'audio.py'))
AU = importlib.util.module_from_spec(spec)
spec.loader.exec_module(AU)
_m = AU.master
def master(mix, *a, **k):
    if bus_out:
        np.savez(bus_out, **{kk: v for kk, v in mix.buses.items()})
    return _m(mix, *a, **k)
AU.master = master
AU.main()
print('wrote', out)
