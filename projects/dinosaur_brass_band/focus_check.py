"""check every frame: worst blur on visible dinosaur faces must be 0, and nobody pops at walk-off end"""
import os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import numpy as np
import render as R
import timeline as T
worst = []
for f in range(T.NF):
    cam, dfaces = R.scene_state(f / T.FPS)
    zf, sharp, K = R.focus(cam, dfaces)
    worst.append(R.subject_blur(cam, dfaces, zf, sharp, K))
worst = np.array(worst)
print('frames with subject blur > 0:', int((worst > 0).sum()), ' max', worst.max().round(3))
# is an exiting animal still on screen at the moment it is removed?
for name, (t0, t1) in T.EXIT.items():
    k = [v['name'] for v in T.VOICES].index(name)
    cam = R.camera(t1 - 0.01)
    M, pos = R.pose(k, t1 - 0.01)
    V = np.concatenate([R.D.xf(m, p.V) for p, m in R.DINOS[k].parts(M)])
    s = cam.wp(V); dz = cam.depth(V)
    on = (dz > 0.1) & (s[:, 0] > 0) & (s[:, 0] < R.W) & (s[:, 1] > 0) & (s[:, 1] < R.H)
    print(f'{name}: on screen at removal: {bool(on.any())}')
