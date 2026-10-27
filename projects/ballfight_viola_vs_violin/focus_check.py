"""Focus audit for #42: the circle of confusion on the subject (the canvas corners, both balls, the bow tip, the darts,
the whole notes - VIOLA JOKE's falls from 4.6 up: checked from the moment it is within 1.5 of its rolling height, as #36
checked the hovering bell once it stomps - and the shards) must be 0 px on every frame (focus locked on the ring).

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
        if k in R.BLADE:
            tip = st['p'] + (R.R + st['blen']) * np.array([math.cos(st['th']), math.sin(st['th'])])
            pts.append(S.to_world(tip[0], tip[1], R.R))
            names.append('bow tip')
    if T.T_FIGHT <= t < R.T_HS:
        for row in R.RUN['K'][R.sim_i(t)]:
            if not np.isnan(row[0]):
                pts.append(S.to_world(row[0], row[1], 0.45))
                names.append('dart')
    for nid, c, u, phi, r, lift, sc, glow, owner, kind in R.notes_at(t):
        if lift > 1.5:
            continue
        for Q, n, kd in R.ST.note_quads(cam, c, r, phi, lift, sc)[::6]:
            pts.append(Q.mean(0))
            names.append('whole note')
    for kind, p, rot, size, curl, a in R.parts_at(t):
        pts.append(S.to_world(p[0], p[2], p[1]))
        names.append('shard')
    c = coc_map(cam.depth(np.array(pts)), zf, 26.0, 0.17)
    i = int(np.argmax(c))
    if c[i] > worst[0]:
        worst = (float(c[i]), t, names[i])
print(f'max subject CoC over {T.NF} frames: {worst[0]:.3f} px' + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else ''))
raise SystemExit(0 if worst[0] == 0 else 1)
