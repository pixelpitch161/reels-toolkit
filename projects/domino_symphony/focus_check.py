"""Locked-focus audit over all 1800 frames without rendering: the circle of confusion of every subject point
(the camera's target, every actively falling domino near it, the holdout during 31.5-38.5 s, the paw and
the cat's face during the tap) must be 0 px on every frame.

    python focus_check.py"""
import os
import sys
import numpy as np
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
import render as R
import timeline as T
import cat as CAT

worst = np.zeros(T.NF)
for f in range(T.NF):
    t = f / T.FPS
    tau = T.scene_time(t)
    cam = R.camera(t)
    zf = max(cam.depth(R.subject(t, tau))[0], 0.05)
    need = R.focus_need(t, tau, cam, zf)
    K = 0.012 * cam.f * min(1.2, 4.5 / zf)
    sharp = max(need, min(0.3, 1.2 / zf))
    act = R.DOM & (R.B.t0 <= tau) & (tau < R.B.t0 + R.B.tc + 0.25)
    pts = np.stack([R.B.cx[act], R.B.y0[act] + 0.02, R.B.cz[act]], 1) if act.any() else np.zeros((0, 3))
    tgt = R.subject(t, tau)
    if len(pts):
        pts = pts[np.linalg.norm(pts - tgt, axis=1) < 0.6]
    extra = [tgt]
    if 31.5 < tau < 38.5:
        extra += [R.HOLD_P + [0, 0.05, 0], R.HOLD_P + [0, -0.05, 0]]
    if tau < 1.0:
        extra += [CAT.paw_target(tau), CAT.faces(tau)[2] + [0, 0.17, -0.05]]
    pts = np.concatenate([pts, np.array(extra)]) if len(pts) else np.array(extra)
    z = cam.depth(pts)
    z = z[z > 0.05]
    coc = K * np.maximum(0.0, np.abs(1 - zf / z) - sharp)
    worst[f] = coc.max() if len(coc) else 0.0
print(f'frames with subject blur > 0: {int((worst > 1e-6).sum())} of {T.NF}   max CoC {worst.max():.4f} px')
