"""Focus audit for #26: the circle of confusion on the subject (the ring, every ball still in play, the champion,
and the balls falling out / resting in the pit) must be 0 px on every frame (focus locked on the ring).

    python focus_check.py"""
import math
import numpy as np
import render as R
import timeline as T
from reelkit.lens import coc_map

worst = (0.0, None, None)
for f in range(T.NF):
    t = f / T.FPS
    cam = R.interp_keys(R.KEYS, t)
    zf = cam.depth(R.S.SUBJECT)[0]
    K = R.DOF_K * cam.f * min(1.2, 4.5 / zf)
    pts = [R.S.RING_C + [r * math.cos(a), r * math.sin(a), z] for a in np.linspace(0, 2 * math.pi, 24)
           for r in (R.R_IN, R.R_OUT) for z in (-R.ZD, R.ZD)]
    names = ['ring'] * len(pts)
    for b in range(16):
        p, st, xy = R.ball_world(b, t)
        for d in ([0, 0, R.T.R_BALL], [0, 0, -R.T.R_BALL]):
            pts.append(p + d)
            names.append(R.NAMES[b])
    d = cam.depth(np.array(pts))
    c = coc_map(d, zf, K, R.DOF_SHARP)
    i = int(np.argmax(c))
    if c[i] > worst[0]:
        worst = (float(c[i]), t, names[i])
print(f'max subject CoC over {T.NF} frames: {worst[0]:.3f} px' + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else ''))
raise SystemExit(0 if worst[0] == 0 else 1)
