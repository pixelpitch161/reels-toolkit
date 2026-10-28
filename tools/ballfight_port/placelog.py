"""log every Mix.place on chosen buses: python placelog.py ep|kit <folder-or-example> <out.txt> bus[,bus]"""
import os, sys, hashlib, importlib.util
import numpy as np
mode, name, out, buses = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4].split(',')
from reelkit import audio as A
LOG = []
_place = A.Mix.place
def place(self, bus, x, t, gain=1.0, pan=0.0):
    if bus in buses:
        x_ = np.asarray(x)
        LOG.append((bus, round(float(t), 9), repr(gain), repr(pan), len(x_), hashlib.md5(np.ascontiguousarray(x_).tobytes()).hexdigest()[:10]))
    return _place(self, bus, x, t, gain, pan)
A.Mix.place = place
if mode == 'ep':
    folder = os.path.abspath(name)
    sys.path.insert(0, folder); os.chdir(folder)
    A.write = lambda p, x: None
    A.section_levels = lambda p, spans: []
    spec = importlib.util.spec_from_file_location('audio_ep', os.path.join(folder, 'audio.py'))
    AU = importlib.util.module_from_spec(spec); spec.loader.exec_module(AU)
    AU.main()
else:
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'projects'))
    from ballfight.kit import episode
    episode.load(name)
    from ballfight.kit import music as M
    M.score()
with open(out, 'w') as f:
    for r in LOG:
        f.write(' '.join(map(str, r)) + '\n')
print(len(LOG), 'placements')
