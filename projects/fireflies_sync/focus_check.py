"""Locked-focus audit over all 1800 frames (no rendering): the hero's whole body must have circle of
confusion 0 on every frame, from the wide shots down to the macro.

    python focus_check.py"""
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import render as R
import sim as M
import timeline as T
from reelkit.lens import coc_map

worst = []
for f in range(T.NF):
    t = f / T.FPS
    cam = R.camera(t)
    zf, sharp, K = R.focus(cam, t)
    h = M.hero(t)
    Rm = R.hero_frame(t)
    pts = [h + Rm[0] * a + Rm[1] * b + Rm[2] * c for a in (-0.0066, 0.0, 0.0105) for b in (-0.003, 0.0, 0.0025) for c in (-0.003, 0.0, 0.003)]
    z = cam.depth(np.array(pts))
    worst.append(float(coc_map(z, zf, K, sharp=sharp).max()))
worst = np.array(worst)
print(f'frames with hero blur > 0: {int((worst > 0).sum())} of {T.NF}   max CoC {worst.max():.3f} px')
