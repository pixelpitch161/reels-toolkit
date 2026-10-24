"""Focus audit: the ring, its tiles, the balls (anywhere inside the ring) and the projector lens have CoC = 0 on every
frame (no rendering needed: uses the same camera and sharp band as render.py).

    python focus_check.py"""
import math
import numpy as np
import render as RD
import timeline as T
import scene as S

worst = 0.0
for f in range(T.NF):
    t = f / T.FPS
    cam = RD.camera(t)
    R = T.ring_vis(t)
    zf = cam.depth(S.RC)[0]
    sharp = RD.focus_band(cam, R)
    ang = np.linspace(0, 2 * math.pi, 36, endpoint=False)
    pts = [S.RC[None] + np.stack([np.cos(ang) * r, np.sin(ang) * r, np.full_like(ang, z)], 1)
           for r in (max(R, 0.0) * S.RW + S.TILE_TH + 0.05, max(R, 0.0) * S.RW * 0.5) for z in (-S.TILE_DZ, S.TILE_DZ)]
    for q in range(4):
        if not T.popped(q, t):
            pts.append(RD.ball_world(q, t)[None] + np.array([[0, 0, S.BALL_R], [0, 0, -S.BALL_R]]))
    P = np.concatenate(pts + [S.PROJ_LENS[None]])
    z = cam.depth(P)
    coc = 3.8 * np.maximum(0.0, np.abs(1 - zf / z) - sharp)
    worst = max(worst, float(coc.max()))
print(f'subject CoC: worst {worst:.3f} px over {T.NF} frames ->', 'OK' if worst == 0 else 'BLUR')
