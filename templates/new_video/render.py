"""Template renderer showing the full reelkit pipeline in ~150 lines:
sky -> far layer -> per-pixel lit floor with real shadows -> objects (shadowed, rim-lit)
-> locked depth of field -> glows -> god rays -> bloom -> grade -> vignette -> captions/HUD.

    python -m reelkit.sheet render.py test/sheet.png 1 6 12 30 45 58
    python -m reelkit.video render.py audio.wav out.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))   # repo root on the path
from reelkit.config import W, H, FPS, BAYER_FULL, SAFE, ss, ease, dirv, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade, fog, sky_gradient
from reelkit.shadows import MaskGrid, directional_mask, point_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, label, MINT, CREAM
import timeline as T
import scene as S

MOON = dirv(-12, 14)                    # visible in the wide shots, low enough for long shadows
FLOOR = MaskGrid((-6.0, 6.0, -6.0, 6.0, 0.025))
CAST_FACES = [Q for o in S.OBJS if o.cast for Q, n in o.faces]
OCC = Occluders(S.OBJS)
MOON_MASK = directional_mask(FLOOR, CAST_FACES, MOON, blur=2.0)                     # static light: cache once
LAMP_MASK = point_mask(FLOOR, [Q for o in S.OBJS if o.cast and o.name != 'post' for Q, n in o.faces], S.LANTERN + [0, 0.2, 0])

KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)  -- gentle 2.5D moves, few cuts
    (0.0, S.SUBJECT, 4.0, 0, 70, 40, 320),
    (4.0, S.SUBJECT, 4.0, 10, 60, 40, 320),
    (7.0, S.SUBJECT + [0, 0.8, -1.0], 11.0, 0, 8, 50, 330),
    (12.0, S.SUBJECT, 5.0, -20, 28, 44, 330),
    (35.0, S.SUBJECT, 5.0, 25, 30, 44, 330),
    (52.0, S.SUBJECT, 4.2, 0, 60, 40, 320),
    (60.0, S.SUBJECT, 4.0, 0, 70, 40, 320),
]


def lights(t, cam):
    L = Lights(amb=(0.16, 0.19, 0.36), sky=(0.05, 0.06, 0.1))
    L.view = cam.view
    L.key('moon', MOON, (0.55, 0.65, 1.0))
    L.fill(dirv(120, 35), (0.12, 0.14, 0.24))
    flick = 0.9 + 0.1 * math.sin(t * 13) * math.sin(t * 7.7)
    lamp_k = (0.85 + 0.5 * T.beat_env(t)) * flick
    L.point(S.LANTERN + [0, 0.2, 0], np.array([1.1, 0.68, 0.32]) * lamp_k, 1.6)
    return L, lamp_k


def render(f):
    t = f / FPS
    cam = interp_keys(KEYS, t)
    L, lamp_k = lights(t, cam)
    fr = Frame(sky_gradient(cam, (10, 12, 48), (60, 110, 190), glow_dir=MOON, glow_col=(150, 170, 230), glow_k=0.4))
    ms = cam.screen_of_dir(MOON)
    if ms is not None:
        fr.disc(ms, 0.03 * cam.f, (236, 232, 210))
        fr.glow(ms[0], ms[1], 0.1 * cam.f, (150, 170, 230), 0.25)
    # far layer
    for Q, n in S.FAR:
        c = fog(shade(L, n[None], Q.mean(0)[None], (60, 70, 120)), 60.0, (60, 110, 190), 10, 80, 0.6)[0]
        fr.poly(cam, Q, c, depth=False)
    # floor: albedo pass -> per-pixel moonlight + lantern light with real shadows + contact AO
    alb = Image.new('RGB', (W, H)); msk = Image.new('L', (W, H))
    ad, md = ImageDraw.Draw(alb), ImageDraw.Draw(msk)
    for Q, col in [(np.array([[S.FLOOR_R * math.cos(a), 0, S.FLOOR_R * math.sin(a)] for a in np.linspace(0, 6.283, 33)[:-1]]), (90, 78, 70))] + S.TILES:
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] > cam.near).all():
            pts = [tuple(p) for p in cam.proj(Pc)]
            ad.polygon(pts, fill=tuple(int(c) for c in col)); md.polygon(pts, fill=255)
    arr, depth = fr.array(), fr.depth()
    ys, xs = np.nonzero(np.asarray(msk))
    if len(xs):
        X, Z, d = cam.ground_hit(xs.astype(float), ys.astype(float))
        I = np.tile(L.amb + L.sky, (len(X), 1))
        I += ((1 - FLOOR.sample(MOON_MASK, X, Z)) * 0.6)[:, None] * L.keys[0][2][None]
        lp = S.LANTERN + [0, 0.2, 0]
        dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
        I += (lp[1] / dist * (1 - FLOOR.sample(LAMP_MASK, X, Z)) * 1.2 * lamp_k / (1 + (dist / 1.5) ** 2))[:, None] * np.array([1.0, 0.62, 0.3])
        I *= contact_ao(X, Z, S.FEET)[:, None]
        col = np.asarray(alb)[ys, xs] * I
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = d
    fr.set_array(arr, depth)
    # objects: painter order, shadowed by other objects (AABB rays), rim-lit by the moon
    for _, o in painter([(np.linalg.norm(o.ctr - cam.pos), o) for o in S.OBJS]):
        fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(o.faces) if np.dot(n, cam.pos - Q.mean(0)) > 0]
        if not fc:
            continue
        fc.sort(key=lambda a: -a[0])
        C = np.array([q.mean(0) for _, q, _, _ in fc]); N = np.array([n for _, _, n, _ in fc])
        kv = OCC.vis(C + N * 0.02, o.grp, d=MOON)[:, None]
        cols = shade(L, N, C, np.array([o.color * o.tone[j] for *_, j in fc]), kvis=kv, emit=0.8 if o.name == 'lamp' else 0.0)
        for (z, Q, n, j), c in zip(fc, cols):
            fr.poly(cam, Q, c)
        if o.name == 'lamp':
            s = cam.wp(S.LANTERN + [0, 0.2, 0])
            fr.glow(s[0], s[1], 0.5 * cam.f / max(cam.depth(S.LANTERN)[0], 0.5), (255, 170, 80), 0.45 * lamp_k)
    # lens (focus locked on the subject), then glows, shafts, bloom, grade
    img, depth = fr.array(), fr.depth()
    zf = cam.depth(S.SUBJECT)[0]
    img = dof(img, coc_map(depth, zf, 0.011 * cam.f * min(1.2, 4.5 / zf), sharp=min(0.3, 1.2 / zf)))
    img = apply_glows(img, fr.glows)
    if ms is not None:
        img = god_rays(img, depth, ms, (150, 175, 235), 0.35)
    img = grade(bloom(np.clip(img, 0, 255)))
    img = np.clip(img * vignette() * top_scrim(), 0, 255).astype(np.uint8)
    # UI on top (never blurred). Keep panels and labels inside SAFE (reelkit.config): the bottom 35 % of the
    # frame and the right-hand strip are covered by app overlays on TikTok / Reels / Shorts.
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    x0, _, x1, y1 = SAFE
    if t >= T.T_SONG:
        panel(d, x0, y1 - 48, x1 - 4, y1 - 4)
    out = np.asarray(pil).copy()
    if t >= T.T_SONG:
        label(out, f"{T.BPM} BPM", x0 + 12, y1 - 37, MINT, 3)
    draw_captions(out, t, T.CAPTIONS)
    return out
