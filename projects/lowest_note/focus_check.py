"""Focus audit without rendering: circle of confusion on the subject for every frame.

Subject = the black hole (sphere + accretion disc) and the inner pressure-wave rings (r <= 1.2 on the gas),
using the same lens settings as render.py (zf = subject depth, K and sharp band from it).

    python focus_check.py"""
import math
import sys
import numpy as np

sys.argv = sys.argv[:1]
import render as R
import scene as S
import timeline as T

pts = [S.CORE + 0.2 * np.array([math.cos(a) * math.cos(b), math.sin(b), math.sin(a) * math.cos(b)])
       for a in np.linspace(0, 2 * math.pi, 12, endpoint=False) for b in (-1.2, -0.6, 0, 0.6, 1.2)]
pts += [S.CORE + np.array([0.8 * math.cos(a), 0.0, 0.8 * math.sin(a)]) for a in np.linspace(0, 2 * math.pi, 16, endpoint=False)]
pts += [np.array([r * math.cos(a), S.POOL_Y, r * math.sin(a)]) for r in (0.5, 0.85, 1.2) for a in np.linspace(0, 2 * math.pi, 24, endpoint=False)]
P = np.array(pts)

worst = (0.0, -1)
for f in range(T.NF):
    t = f / T.FPS
    cam = R.cam_at(t)
    zf = cam.depth(S.SUBJECT)[0]
    K = 0.022 * cam.f * min(1.2, 8.0 / zf)
    sharp = 3.2 / zf
    z = cam.depth(P)
    coc = np.clip(K * np.maximum(0.0, np.abs(1 - zf / np.maximum(z, 1e-3)) - sharp), 0, 10.5)
    if coc.max() > worst[0]:
        worst = (float(coc.max()), f)
print(f'subject CoC over {T.NF} frames: max {worst[0]:.3f} px (frame {worst[1]})')
