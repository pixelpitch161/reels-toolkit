"""Locked-focus audit without rendering: for every frame, the circle of confusion of every LED-wall pixel and
of the wall props must be 0 (the sharp band covers them).

    python focus_check.py"""
import math
import os
import sys
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS
from reelkit.lens import coc_map
import render as R
import scene as S
import timeline as T


def wall_depths(cam):
    corners = [S.WC + [sx * S.RW, sy * S.RW, 0] for sx in (-1, 1) for sy in (-1, 1)]
    sc = np.array([cam.wp(c) for c in corners])
    x0, x1 = int(max(0, sc[:, 0].min() - 2)), int(min(W, sc[:, 0].max() + 3))
    y0, y1 = int(max(0, sc[:, 1].min() - 2)), int(min(H, sc[:, 1].max() + 3))
    ys, xs = np.mgrid[y0:y1, x0:x1]
    dw = cam.pixel_rays(xs.ravel().astype(float), ys.ravel().astype(float))
    t = (0.0 - cam.pos[2]) / dw[:, 2]
    P = cam.pos[None] + t[:, None] * dw
    u = (P[:, 0] - S.WC[0]) / S.RW
    v = (P[:, 1] - S.WC[1]) / S.RW
    return t[(u * u + v * v <= 1) & (t > 0)]


if __name__ == '__main__':
    worst, wf = 0.0, 0
    for f in range(T.NF):
        cam = R.camera(f / FPS)
        zf = cam.depth(S.WC)[0]
        sharp = R.focus_band(cam)
        z = np.concatenate([wall_depths(cam), cam.depth(R.SUBJ_PTS)])
        c = coc_map(z, zf, 5.4, sharp=sharp).max()
        if c > worst:
            worst, wf = c, f
    print(f'subject (LED wall + props) max CoC over all frames: {worst:.3f} px (frame {wf})')
