"""Focus audit for #41: the circle of confusion on the subject (the canvas corners, both balls, the lance tip, the cobras'
heads and baskets, PEDAL TONE's giant bell and the shards) must be 0 px on every frame (focus locked on the ring).

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
            tip = st['p'] + (R.R + st.get('ext', st['blen'])) * np.array([math.cos(st['th']), math.sin(st['th'])])
            pts.append(S.to_world(tip[0], tip[1], R.R))
            names.append('lance tip')
    if T.T_FIGHT <= t < R.T_HS:
        for row in R.RUN['K'][R.sim_i(t)]:
            if not np.isnan(row[0]):
                pts.append(S.to_world(row[0], row[1], 0.45))
                names.append('key')
    if T.T_FIGHT <= t < R.T_HS:                            # PEDAL TONE's bell once it stomps onto the canvas (while it
        for owner, cx, cz, t0, t_slam, rad in R.RUN['SLAMS'][R.sim_i(t)]:     # hovers high above it is scenery, as in #34)
            h = R.ST.SB.bell_height(t, t0, t_slam)
            if owner in R.BLADE and h is not None and h <= 1.0:
                pts.append(S.to_world(cx, cz, h))
                names.append('pedal bell')
    fs = [R.fighter(k, t) for k in (0, 1)]
    for home, bk, cp, t_drop, S_, aim, bid in R.cobras_at(t, fs):      # baskets + the cobras (a raised hood up to ~1.3 high,
        pts.append(S.to_world(home[0], home[1], 0.0))                   # a strike's head out to its reach)
        pts.append(S.to_world(cp[0], cp[1], 1.3))
        if S_ is not None:
            pts.append(S.to_world(cp[0] + S_[4] * math.cos(S_[3]), cp[1] + S_[4] * math.sin(S_[3]), R.R))
        names += ['basket', 'cobra'] + (['strike'] if S_ is not None else [])
    for kind, p, rot, size, curl, a in R.parts_at(t):
        pts.append(S.to_world(p[0], p[2], p[1]))
        names.append('shard')
    c = coc_map(cam.depth(np.array(pts)), zf, 26.0, 0.17)
    i = int(np.argmax(c))
    if c[i] > worst[0]:
        worst = (float(c[i]), t, names[i])
print(f'max subject CoC over {T.NF} frames: {worst[0]:.3f} px' + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else ''))
raise SystemExit(0 if worst[0] == 0 else 1)
