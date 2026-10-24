"""Focus audit without rendering: every point of the board (rim, floor corners, wall tops, the centre) has circle of
confusion 0 on every frame (the DOF focus is locked on the board centre)."""
import numpy as np
from reelkit.config import FPS
from reelkit.lens import coc_map
import render as R
import timeline as T
import scene as S

a = S.HALF + S.RIM_W
local = np.array([[x, y, z] for x in np.linspace(-a, a, 5) for z in np.linspace(-a, a, 5) for y in (-S.SLAB, 0.0, S.RIM_H)])
worst = 0.0
for f in range(T.NF):
    t = f / FPS
    cam = R.camera(t)
    Rm = R.tilt_matrix(t)
    zf, band = R.focus(cam, Rm)
    pts = R.to_world(local, Rm)
    c = coc_map(cam.depth(pts), zf, 0.012 * cam.f * min(1.2, 60.0 / zf), sharp=band)
    worst = max(worst, float(c.max()))
print(f'board CoC over {T.NF} frames: max {worst:.3f} px', 'OK' if worst == 0 else 'BLUR')
