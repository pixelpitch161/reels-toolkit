"""Focus audit for #31: the circle of confusion on the subject (the canvas corners, both balls, the record's rim, the
tonearm, the patch cables and the shards) must be 0 px on every frame (focus locked on the ring).

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
    tip, h, on = R.arm_state(t)                                    # the tonearm's headshell and the record's rim
    pts.append(S.to_world(tip[0], tip[1], h))
    names.append('tonearm')
    for a in np.linspace(0, 2 * math.pi, 9)[:-1]:
        pts.append(S.to_world(R.ST.REC_R * math.cos(a), R.ST.REC_R * math.sin(a), 0.0))
        names.append('record rim')
    if T.T_FIGHT <= t < R.T_HS and R.live_cables(t):               # a plugged cable's middle
        ps, pe = R.cable_ends(t)
        m = (ps + pe) / 2
        pts.append(S.to_world(m[0], m[1], 0.42))
        names.append('cable')
    for kind, p, rot, size, curl, a in R.parts_at(t):
        pts.append(S.to_world(p[0], p[2], p[1]))
        names.append('shard')
    c = coc_map(cam.depth(np.array(pts)), zf, 26.0, 0.17)
    i = int(np.argmax(c))
    if c[i] > worst[0]:
        worst = (float(c[i]), t, names[i])
print(f'max subject CoC over {T.NF} frames: {worst[0]:.3f} px' + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else ''))
raise SystemExit(0 if worst[0] == 0 else 1)
