"""Focus audit: the stage (rim, boards, every token) must have CoC 0 on every frame (focus locked on the subject).

    python focus_check.py          # prints the worst subject CoC over all frames (must be 0)"""
import math
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.lens import coc_map
import render as R
import timeline as T
import scene as S

worst = 0.0
for f in range(T.NF):
    t = f / T.FPS
    cam = R.camera(t)
    zf = cam.depth(S.SUBJECT)[0]
    pts = np.concatenate([R._SUBJ, np.c_[T.POS[f, :, 0], np.full(T.NPC, 0.15), T.POS[f, :, 1]]])
    z = cam.depth(pts)
    c = coc_map(z, zf, 5.5, sharp=R.focus_band(cam))
    worst = max(worst, float(c.max()))
print('worst subject CoC over all frames:', worst)
