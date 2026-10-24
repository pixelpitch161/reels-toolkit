"""Focus audit without rendering: the box (every corner it can reach), the ring, the hoppers and the pile have
circle of confusion 0 on every frame (the DOF focus is locked on the box centre)."""
import math
import numpy as np
from reelkit.config import FPS
from reelkit.lens import coc_map
import render as R
import timeline as T
import scene as S

worst = 0.0
pts = np.concatenate([R._SUBJ, np.array([S.BC + [x, y, S.ZF] for x in (-S.HALF, S.HALF) for y in (-S.HALF, S.HALF)])])
for f in range(T.NF):
    cam = R.camera(f / FPS)
    zf = cam.depth(S.BC)[0]
    c = coc_map(cam.depth(pts), zf, 7.0, sharp=R.focus_band(cam))
    worst = max(worst, float(c.max()))
print(f'subject CoC over {T.NF} frames: max {worst:.3f} px', 'OK' if worst == 0 else 'BLUR')
