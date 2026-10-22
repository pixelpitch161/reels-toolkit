"""Locked-focus audit over all 1800 frames (no rendering): the subject (window + mullions, the cat
wherever it is, the sill, the lamp, the HUD-adjacent floor in front of the window where the flash
shadows land) must have circle of confusion 0 on every frame.

    python focus_check.py"""
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import render as R
import scene as S
import timeline as T
from reelkit.lens import coc_map

worst = []
for f in range(T.NF):
    t = f / T.FPS
    cam = R.camera(t)
    zf, sharp, K = R.focus(cam)
    cat = R.cat_state(t)[0]
    pts = np.array([[S.WX0, S.WY0, S.RZ0], [S.WX1, S.WY1, S.RZ0], [S.WX0, S.WY1, S.RZ0], [S.WX1, S.WY0, S.RZ0],
                    cat, cat + [0, 0.3, 0], S.CAT_SILL, S.LAMP_P, S.FIRE_P,
                    [S.WXC, 0, S.RZ0 + 0.3], [S.WXC, 0, S.RZ1 - 0.1]])
    z = cam.depth(pts)
    worst.append(float(coc_map(z, zf, K, sharp=sharp).max()))
worst = np.array(worst)
print(f'frames with subject blur > 0: {int((worst > 0).sum())} of {T.NF}   max CoC {worst.max():.3f} px')
