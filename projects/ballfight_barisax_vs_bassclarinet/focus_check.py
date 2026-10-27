"""Focus audit for #40: the circle of confusion on the subject (the canvas corners, both balls - the bari up on its cable
too, the bass clarinet sunk or not -, the endpin peg, CABLE SNAP's target and the shards) must be 0 px on every frame
(focus locked on the ring). The crane line high above is scenery, not the subject.

    python focus_check.py"""
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
        for dy in (0.0, 2 * R.R + st['h']):
            pts.append(S.to_world(st['p'][0], st['p'][1], dy))
            names.append(R.NAMES[k])
        if st['anc'] is not None:
            c = st['anc'][0]
            pts.append(S.to_world(c[0], c[1], 0.0))
            names.append('peg')
    for t0_, k_s, spot_, ts_, t1_, rad_ in R.SNAPS:
        if t0_ <= t < t1_:
            pts.append(S.to_world(spot_[0], spot_[1], 0.0))
            names.append('CABLE SNAP target')
    for kind, p, rot, size, curl, a in R.parts_at(t):
        pts.append(S.to_world(p[0], p[2], p[1]))
        names.append('shard')
    c = coc_map(cam.depth(np.array(pts)), zf, 26.0, 0.19)   # render.py's sharp band
    i = int(np.argmax(c))
    if c[i] > worst[0]:
        worst = (float(c[i]), t, names[i])
print(f'max subject CoC over {T.NF} frames: {worst[0]:.3f} px' + (f' (at {worst[1]:.2f} s, {worst[2]})' if worst[1] is not None else ''))
raise SystemExit(0 if worst[0] == 0 else 1)
