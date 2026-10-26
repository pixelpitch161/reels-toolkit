"""Focus audit for #37: the circle of confusion on the subject (the canvas corners, both balls, the zip lines' ends, the
A beam's far end, the reed's tip and the shards) must be 0 px on every frame (focus locked on the ring).

    python focus_check.py"""
import math
import numpy as np
import render as R
import timeline as T
import scene as S
from reelkit.lens import coc_map

worst = (0.0, None, None)
corners = [S.to_world(sx, sz, y) for sx in (-S.AP, S.AP) for sz in (-S.AP, S.AP) for y in (0.0, S.POST_H)]
for f in range(T.NF):
    t = f / T.FPS
    cam = R.camera(t)
    zf = cam.depth(S.SUBJECT)[0]
    pts, names = list(corners), ['ring'] * len(corners)
    for k in (0, 1):
        st = R.fighter(k, t)
        for dy in (0.0, 2 * R.R):
            pts.append(S.to_world(st['p'][0], st['p'][1], dy))
            names.append(R.NAMES[k])
        if k in R.TUNER:
            tip = st['p'] + (R.R + 0.34) * np.array([math.cos(st['th']), math.sin(st['th'])])
            pts.append(S.to_world(tip[0], tip[1], R.R))
            names.append('reed tip')
    for k, p0, p1, age, hot in R.live_zips(t):
        for p in (p0, p1):
            pts.append(S.to_world(p[0], p[1], R.R))
            names.append('zip')
    for k, o, a, ln, wd, kk, held in R.live_beams(t):
        d = np.array([math.cos(a), math.sin(a)])
        e = o + d * min(ln, R.ST.ring_exit(o, d))
        pts.append(S.to_world(e[0], e[1], R.R))
        names.append('beam end')
    for kind, p, rot, size, curl, a in R.parts_at(t):
        pts.append(S.to_world(p[0], p[2], p[1]))
        names.append('shard')
    c = coc_map(cam.depth(np.array(pts)), zf, 26.0, 0.17)
    i = int(np.argmax(c))
    if c[i] > worst[0]:
        worst = (float(c[i]), t, names[i])
print(f'max subject CoC over {T.NF} frames: {worst[0]:.3f} px' + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else ''))
raise SystemExit(0 if worst[0] == 0 else 1)
