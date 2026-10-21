"""The Perfect Loop - renderer.

    python -m reelkit.sheet render.py test/sheet.png 0.5 5.5 10 15 20 26 30 41 50 58
    python -m reelkit.video render.py audio.wav perfect_loop.mp4

Per frame: dusk void -> cut-away edges -> ray-traced attic shell (plank floor with the sun pool
through the round window, the bobs' moving shadows, coloured bob light, lamp light + static
shadow masks, plaster/timber walls, sloped ceiling, sunset backdrop in the window) -> objects in
painter order (sunlit through the window + AABB occluders, rim-lit), the 15 bob slots (strings,
lantern, beam segment), clocks, cat -> locked depth of field -> sunbeam + motes -> glows -> bloom
-> grade -> vignette/scrim -> group tie-lines, tags, HUD (phase ring), captions."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, nrm, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade, sky_gradient
from reelkit.shadows import MaskGrid, directional_mask, point_mask, proj_dir, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, slide_offset, MINT, CREAM, AMBER, CORAL, SKYB, INK
from reelkit import font as RF
from reelkit.font import text_mask, stamp, rich_line, ADV
from reelkit.geometry import Obj, face, thin_quad
import timeline as T
import scene as S

# glyphs this video needs (added at runtime, reelkit itself is unchanged)
for ch, rows in {'→': ".....|..#..|...#.|#####|...#.|..#..|.....",
}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

# ---------------------------------------------------------------- lights + shadow masks
SUN = S.SUN
WARM = np.array([1.0, 0.63, 0.32])
BOUNCE = (np.array([0.05, 0.08, -0.05]), 1.5, np.array([0.22, 0.16, 0.11]))      # fake GI from the sunlit floor pool
FLOOR = MaskGrid((S.RX0, S.RX1, S.RZ0, S.RZ1, 0.01))
FINE = MaskGrid((-0.45, 0.75, -1.05, 0.8, 0.004))                                 # the bobs' moving shadows
STATIC_CAST = [o for o in S.OBJS if o.cast] + [S.BEAM_OBJ]
SUN_MASK = directional_mask(FLOOR, [Q for o in STATIC_CAST for Q, n in o.faces], SUN, blur=1.0)
_lamp_grp = []
for lp, R, k in S.LAMPS:
    near = min(S.OBJS, key=lambda o: np.linalg.norm(o.ctr - lp))
    _lamp_grp.append(near.grp)
LAMP_MASKS = [point_mask(FLOOR, [Q for o in STATIC_CAST if o.grp != _lamp_grp[i] and o.name != 'chimney' for Q, n in o.faces], lp, blur=2.5)
              for i, (lp, R, k) in enumerate(S.LAMPS)]
OCC = Occluders(S.OBJS + [S.BEAM_OBJ])
BOB_COL = np.array(S.RAINBOW, float)


def flicker(t, i):
    """lamp flame flicker; integer cycles per minute, so the loop is seamless"""
    w = 2 * math.pi / 60.0
    return 0.92 + 0.05 * math.sin(w * (97 + 13 * i) * t + i) * math.sin(w * (131 + 7 * i) * t) + 0.03 * math.sin(w * (233 + 11 * i) * t + 2 * i)


def make_lights(t, cam, fl):
    L = Lights(amb=(0.4, 0.41, 0.54), sky=(0.05, 0.05, 0.08))
    L.view = cam.view
    L.key('sun', SUN, S.SUN_RGB * 1.05)
    L.fill(nrm([0.4, 0.6, 0.9]), (0.07, 0.08, 0.14))
    ks = []
    for i, (lp, R, k) in enumerate(S.LAMPS):
        kk = k * flicker(t, i)
        ks.append(kk)
        L.point(lp, WARM * kk * 0.9, R)
    L.point(BOUNCE[0], BOUNCE[2], BOUNCE[1])
    for k in range(T.K):                                  # each lantern lights its neighbourhood a little
        pass
    return L, ks


# ---------------------------------------------------------------- camera: gentle 2.5D moves, few cuts
SUBJ = np.array([0.0, 0.32, -0.38])
HOOK = (0.0, SUBJ + [0, -0.02, 0], 1.9, 28, 11, 42, 440)
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)   target sits at screen y = sy
    HOOK,
    (3.4, SUBJ + [0, -0.02, 0.02], 1.78, 25, 12, 42, 440),
    (6.2, np.array([-0.1, 0.95, -0.6]), 4.4, 16, 20, 46, 330),        # establishing: the whole attic
    (7.9, np.array([-0.05, 0.75, -0.5]), 3.7, 26, 20, 46, 340),
    (9.8, SUBJ + [0, 0, -0.02], 2.85, 58, 15, 42, 385),               # side: longer = slower, length tags
    (12.2, SUBJ, 2.0, 48, 36, 42, 370),                              # rise over the snake
    (16.0, SUBJ, 1.95, 38, 44, 42, 365),
    (21.0, SUBJ, 1.9, -34, 38, 42, 370),                             # chaos: orbit to the cat's side
    (27.6, SUBJ, 1.95, -40, 28, 42, 385),
    (30.0, SUBJ + [0, 0, -0.04], 1.95, 0, 36, 42, 395),              # halftime: down the row, 2 teams
    (34.4, SUBJ + [0, 0, -0.04], 1.95, 5, 37, 42, 395),
    (37.4, SUBJ + [0, 0, 0.04], 2.2, 32, 42, 42, 300),                             # HUD up: subject higher
    (42.0, SUBJ + [0, 0, 0.07], 2.3, 42, 46, 42, 295),                              # high 3/4: count the groups
    (47.5, SUBJ + [0, 0, 0.07], 2.3, 50, 38, 42, 300),
    (52.0, SUBJ + [0, 0.05, 0], 2.4, 36, 28, 42, 300),               # try it: pull back
    (55.6, SUBJ + [0, -0.02, 0], 1.95, 29, 13, 42, 430),
    (60.0,) + HOOK[1:],
]


def camera(t):
    return interp_keys(KEYS, t)


# ---------------------------------------------------------------- procedural textures
def hash2(a, b):
    v = np.sin(a * 127.1 + b * 311.7) * 43758.5453
    return v - np.floor(v)


def floor_albedo(X, Z):
    """wide planks running toward the window, staggered joints, nail dots"""
    pw = 0.16
    col_i = np.floor(X / pw)
    off = hash2(col_i, 7.0) * 1.3
    row = np.floor((Z + off) / 1.1)
    h = hash2(col_i, row)
    base = np.array([146, 102, 74], float)[None] * (0.84 + 0.24 * h)[:, None]
    base *= (0.96 + 0.06 * np.sin(Z * 21 + col_i * 2.3 + np.sin(Z * 5) * 2))[:, None]
    u = (X / pw) % 1.0
    v = ((Z + off) / 1.1) % 1.0
    seam = (np.abs(u - 0.5) > 0.47) | (np.abs(v - 0.5) > 0.494)
    base[seam] *= 0.66
    nail = (np.abs(u - 0.5) > 0.36) & (np.abs(u - 0.5) < 0.42) & ((np.abs(v - 0.02) < 0.006) | (np.abs(v - 0.98) < 0.006))
    base[nail] *= 0.6
    return base


def back_albedo(X, Y):
    """warm plaster between dark timber framing (gable end)"""
    col = np.zeros((len(X), 3))
    n = hash2(np.floor(X * 9), np.floor(Y * 9))
    col[:] = np.array([196, 186, 172.0])[None] * (0.95 + 0.06 * n)[:, None]
    timber = np.array([90, 58, 42.0])
    post = (np.abs(X + 1.22) < 0.05) | (np.abs(X - 0.26) < 0.05) | (np.abs(X - 1.62) < 0.05)
    beam = (np.abs(Y - 2.32) < 0.055)
    sill = np.abs(Y - 0.1) < 0.1
    brace = (np.abs((Y - 0.18) - (X + 1.17) * 1.3) < 0.05) & (X > -1.17) & (X < -0.62) & (Y < 2.3)
    brace |= (np.abs((Y - 0.18) + (X - 0.21) * 1.3) < 0.05) & (X > -0.2) & (X < 0.21) & (Y > 2.0) & False
    brace |= (np.abs((Y - 2.27) + (X - 0.31) * 1.25) < 0.05) & (X > 0.31) & (X < 0.95) & (Y > 1.4)
    tm = post | beam | sill | brace
    grain = 0.9 + 0.12 * hash2(np.floor(X * 40), np.floor(Y * 6))
    col[tm] = timber[None] * grain[tm, None]
    top = Y > S.gable_top(X) - 0.06
    col[top] = timber * 0.9
    return col


def knee_albedo(Z, Y):
    """vertical boards on the knee wall"""
    b = np.floor(Z / 0.13)
    col = np.array([128, 96, 74.0])[None] * (0.85 + 0.2 * hash2(b, 3.0))[:, None]
    col[np.abs((Z / 0.13) % 1.0 - 0.5) > 0.46] *= 0.7
    col[Y < 0.08] = [92, 58, 38]
    return col


def ceil_albedo(Z, S_):
    """boards along the slope (rafters are real objects)"""
    b = np.floor(S_ / 0.18)
    col = np.array([116, 86, 68.0])[None] * (0.86 + 0.18 * hash2(b, 5.0))[:, None]
    col[np.abs((S_ / 0.18) % 1.0 - 0.5) > 0.47] *= 0.72
    return col


def backdrop(X, Y, t, sun_xy):
    """sunset over the rooftops, seen through the round window (positions relative to the sun)"""
    sx, sy = sun_xy
    dy = Y - sy
    k = np.clip((dy + 0.5) / 2.4, 0, 1)
    c0, c1, c2 = np.array([255, 178, 96.0]), np.array([238, 118, 128.0]), np.array([104, 78, 150.0])
    col = np.where((k < 0.4)[:, None], c0 * (1 - k[:, None] / 0.4) + c1 * (k[:, None] / 0.4), c1 * (1 - (k[:, None] - 0.4) / 0.6) + c2 * ((k[:, None] - 0.4) / 0.6))
    d = np.sqrt((X - sx) ** 2 + (Y - sy) ** 2)
    col += (np.clip(1 - d / 1.1, 0, 1) ** 2)[:, None] * np.array([60, 50, 20.0])[None]
    disc = d < 0.2
    col[disc] = [255, 238, 176]
    col[(d >= 0.2) & (d < 0.24)] = col[(d >= 0.2) & (d < 0.24)] * 0.5 + np.array([255, 214, 140.0]) * 0.5
    # cloud streaks lit from below
    for cy_, amp, sp, wd in ((0.55, 0.06, 0.05, 0.035), (0.95, 0.08, -0.03, 0.03), (1.35, 0.05, 0.04, 0.025)):
        cl = sy + cy_ + amp * np.sin((X - sx) * 2.1 + 2 * math.pi * t / 60.0)
        band = (np.abs(Y - cl) < wd * (1 + 0.6 * np.sin((X - sx) * 5.0 + cy_ * 9)))
        col[band] = col[band] * 0.45 + np.array([255, 170, 150.0]) * 0.55
    # rooftops, chimneys and a clock tower (dusk silhouettes, a few lit windows)
    rx = X - sx
    hroof = sy - 0.34 + 0.12 * np.sin(rx * 2.4 + 1.0) + 0.08 * (np.floor(rx * 1.7) % 2) + 0.05 * hash2(np.floor(rx * 3.1), 2.0)
    chim = (np.abs(((rx * 2.3) % 1.0) - 0.5) < 0.05) & (hash2(np.floor(rx * 2.3), 4.0) > 0.4)
    hroof = np.where(chim, hroof + 0.12, hroof)
    tw = np.abs(rx + 0.62) < 0.09
    hroof = np.where(tw, sy + 0.42, hroof)
    spire = np.abs(rx + 0.62) < 0.09 * np.clip((sy + 0.75 - Y) / 0.33, 0, 1)
    roof = (Y < hroof) | (spire & (Y < sy + 0.75))
    col[roof] = [96, 58, 92]
    face_ = tw & (np.sqrt((rx + 0.62) ** 2 + (Y - sy - 0.28) ** 2) < 0.055)
    col[face_] = [255, 214, 140]
    lit = roof & ~tw & (hash2(np.floor(rx * 14), np.floor(Y * 14)) > 0.93) & (Y < hroof - 0.06)
    col[lit] = [255, 200, 112]
    # three birds crossing (periodic: whole passes per minute)
    for i, (v, y0, ph) in enumerate(((3, 0.5, 0.0), (4, 0.75, 0.35), (2, 0.25, 0.7))):
        u = ((v * t / 60.0 + ph) % 1.0)
        bx = sx - 1.6 + 3.2 * u
        by = sy + y0 + 0.04 * math.sin(2 * math.pi * 9 * t / 60.0 + i)
        flap = 0.025 * math.sin(2 * math.pi * 60 * t / 60.0 * (1 + 0.3 * i) + i)
        ddx = np.abs(X - bx)
        bird = (ddx < 0.06) & (np.abs((Y - by) - ddx * 0.55 + flap * (ddx / 0.06)) < 0.011)
        col[bird] = [70, 40, 70]
    return col


# ---------------------------------------------------------------- the attic shell, per pixel
def shell_pass(fr, cam, t, L, ks, P_bob, fl):
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    INF = 1e9
    with np.errstate(divide='ignore', invalid='ignore'):
        tf = np.where(dw[..., 1] < -1e-6, -p[1] / dw[..., 1], INF)                    # floor y = 0
        Xf, Zf = p[0] + tf * dw[..., 0], p[2] + tf * dw[..., 2]
        okf = (tf > 0) & (Xf >= S.RX0) & (Xf <= S.RX1) & (Zf >= S.RZ0) & (Zf <= S.RZ1)
        tb = np.where(dw[..., 2] < -1e-6, (S.RZ0 - p[2]) / dw[..., 2], INF)           # back wall z = RZ0
        Xb, Yb = p[0] + tb * dw[..., 0], p[1] + tb * dw[..., 1]
        okb = (tb > 0) & (Xb >= S.RX0) & (Xb <= S.RX1) & (Yb >= 0) & (Yb <= S.gable_top(Xb))
        tk = np.where(dw[..., 0] < -1e-6, (S.RX0 - p[0]) / dw[..., 0], INF)           # knee wall x = RX0
        Yk, Zk = p[1] + tk * dw[..., 1], p[2] + tk * dw[..., 2]
        okk = (tk > 0) & (Zk >= S.RZ0) & (Zk <= S.RZ1) & (Yk >= 0) & (Yk <= S.KNEE)
        den = dw[..., 1] - dw[..., 0] * S.SLOPE                                        # sloped ceiling
        tc = (S.KNEE + (p[0] - S.RX0) * S.SLOPE - p[1]) / np.where(np.abs(den) < 1e-6, 1e-6, den)
        Xc, Yc, Zc = p[0] + tc * dw[..., 0], p[1] + tc * dw[..., 1], p[2] + tc * dw[..., 2]
        okc = (tc > 0) & (Xc >= S.RX0) & (Xc <= S.RIDGE_X) & (Zc >= S.RZ0) & (Zc <= S.RZ1)
    outside = (dw @ S.CEIL_N) > 0                                                       # roof seen from above
    winpx = okb & S.win_open(Xb, Yb)
    TT = [np.where(m, tt, INF) for m, tt in ((okf, tf), (okb, tb), (okk, tk), (okc, tc))]
    tmin = np.minimum(np.minimum(TT[0], TT[1]), np.minimum(TT[2], TT[3]))
    mf = okf & (TT[0] <= tmin)
    mb = okb & (TT[1] <= tmin) & ~mf
    mk = okk & (TT[2] <= tmin) & ~mf & ~mb
    mc = okc & (TT[3] <= tmin) & ~mf & ~mb & ~mk

    def lamps_at(P, nv, floor=False):
        I = np.zeros((len(P), 3))
        for i, (lp, R, k) in enumerate(S.LAMPS):
            v = lp[None] - P
            dist = np.linalg.norm(v, axis=1) + 1e-6
            lam = np.clip((v * nv[None]).sum(1) / dist, 0, None) * 0.8 + 0.2
            sh = (1 - FLOOR.sample(LAMP_MASKS[i], P[:, 0], P[:, 2]) * 0.8) if floor else 1.0
            I += (lam * sh * ks[i] / (1 + (dist / R) ** 2))[:, None] * WARM[None]
        bp, bR, bc = BOUNCE
        v = bp[None] - P
        dist = np.linalg.norm(v, axis=1) + 1e-6
        I += (np.clip((v * nv[None]).sum(1) / dist, 0, None) * 0.7 + 0.3)[:, None] / (1 + (dist / bR) ** 2)[:, None] * bc[None] * (0.0 if floor else 1.0)
        return I

    # ---- floor: sun pool through the window, static + moving bob shadows, lantern light
    ys, xs = np.nonzero(mf)
    if len(xs):
        X, Z = Xf[ys, xs], Zf[ys, xs]
        P = np.stack([X, np.zeros_like(X), Z], 1)
        alb = floor_albedo(X, Z)
        I = np.tile(L.amb + L.sky, (len(X), 1))
        sh = np.maximum(FLOOR.sample(SUN_MASK, X, Z), FINE.sample(fr.bob_mask, X, Z))
        slit = S.win_lit(P + [0, 0.002, 0]) * (1 - sh)
        I += (slit * SUN[1] * 1.35)[:, None] * S.SUN_RGB[None]
        I += lamps_at(P, np.array([0, 1.0, 0]), True)
        # coloured light from each lantern, strongest under it, a burst on every pluck
        near = (X > -0.5) & (X < 0.6) & (Z > -1.15) & (Z < 0.45)
        if near.any():
            Xn, Zn = X[near], Z[near]
            add = np.zeros((len(Xn), 3))
            for k in range(T.K):
                bx, by, bz = P_bob[k]
                d2 = (Xn - bx) ** 2 + by ** 2 + (Zn - bz) ** 2
                add += ((by / np.sqrt(d2)) * (0.16 + 0.55 * fl[k]) / (1 + d2 / 0.012))[:, None] * (BOB_COL[k] / 255.0)[None]
            I[near] += add
        I *= contact_ao(X, Z, S.FEET)[:, None]
        I *= (1 - 0.3 * np.exp(-(X - S.RX0) / 0.25) - 0.3 * np.exp(-(Z - S.RZ0) / 0.25))[:, None]
        col = alb * I
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tf[ys, xs]
    # ---- back wall (gable end) + window
    ys, xs = np.nonzero(mb & ~winpx)
    if len(xs):
        X, Y = Xb[ys, xs], Yb[ys, xs]
        P = np.stack([X, Y, np.full_like(X, S.RZ0)], 1)
        alb = back_albedo(X, Y)
        I = np.tile(L.amb * 0.92, (len(X), 1)) + lamps_at(P, np.array([0, 0, 1.0]))
        I += (np.clip(np.array([0, 0, 1.0]) @ L.fills[0][0], 0, None) * L.fills[0][1])[None]
        # warm glow around the window (light scattered by the sill and reveal)
        dwn = np.sqrt((X - S.WCX) ** 2 + (Y - S.WCY) ** 2)
        I += (np.clip(1 - (dwn - S.WR) / 0.5, 0, 1) ** 2 * 0.18)[:, None] * S.SUN_RGB[None]
        ao = (1 - 0.35 * np.exp(-Y / 0.2)) * (1 - 0.3 * np.exp(-(X - S.RX0) / 0.3)) * (1 - 0.25 * np.exp(-(S.gable_top(X) - Y) / 0.25))
        col = alb * I * ao[:, None]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tb[ys, xs]
    ys, xs = np.nonzero(mb & winpx)
    sun_scr = None
    if len(xs):
        Xw, Yw = Xb[ys, xs], Yb[ys, xs]
        d2 = dw[ys, xs]
        tr = (S.RZ0 - S.WT - p[2]) / d2[:, 2]
        reveal = ~S.win_open(p[0] + tr * d2[:, 0], p[1] + tr * d2[:, 1])
        mull = S.win_mullion(Xw, Yw)
        tbd = (S.BACKDROP_Z - p[2]) / d2[:, 2]
        Xbd, Ybd = p[0] + tbd * d2[:, 0], p[1] + tbd * d2[:, 1]
        # the sun is pinned low in the window for every camera (stage backdrop)
        spt = np.array([S.WCX + 0.11, S.WCY - 0.13, S.RZ0])
        dirs = spt - p
        s_ = (S.BACKDROP_Z - p[2]) / dirs[2]
        sun_xy = (p[0] + s_ * dirs[0], p[1] + s_ * dirs[1])
        col = backdrop(Xbd, Ybd, t, sun_xy)
        frame_c = np.array([92, 58, 40.0])
        Iw = (L.amb * 0.55)[None] + lamps_at(np.stack([Xw, Yw, np.full_like(Xw, S.RZ0)], 1), np.array([0, 0, 1.0])) * 0.8
        col[mull] = (frame_c[None] * Iw)[mull]
        rc = np.array([200, 176, 146.0])[None] * (Iw * 0.8 + np.array([0.3, 0.2, 0.12]))
        col[reveal & ~mull] = rc[reveal & ~mull]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = np.where(reveal | mull, tb[ys, xs], 5e5 + tbd)
        sun_scr = cam.wp(np.array([sun_xy[0], sun_xy[1], S.BACKDROP_Z]))
    # ---- knee wall
    ys, xs = np.nonzero(mk)
    if len(xs):
        Z, Y = Zk[ys, xs], Yk[ys, xs]
        P = np.stack([np.full_like(Z, S.RX0), Y, Z], 1)
        I = np.tile(L.amb * 0.9, (len(Z), 1)) + lamps_at(P, np.array([1.0, 0, 0]))
        ao = (1 - 0.3 * np.exp(-Y / 0.2)) * (1 - 0.3 * np.exp(-(Z - S.RZ0) / 0.3)) * (1 - 0.3 * np.exp(-(S.KNEE - Y) / 0.15))
        col = knee_albedo(Z, Y) * I * ao[:, None]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tk[ys, xs]
    # ---- sloped ceiling (inside) / roof (outside)
    ys, xs = np.nonzero(mc)
    if len(xs):
        X, Y, Z = Xc[ys, xs], Yc[ys, xs], Zc[ys, xs]
        P = np.stack([X, Y, Z], 1)
        sl = (X - S.RX0) * math.sqrt(1 + S.SLOPE ** 2)
        out_ = outside[ys, xs]
        I = np.tile(L.amb * 0.85, (len(X), 1)) + lamps_at(P, S.CEIL_N)
        ao = (1 - 0.3 * np.exp(-(Y - S.KNEE) / 0.25)) * (1 - 0.25 * np.exp(-(Z - S.RZ0) / 0.3))
        col = ceil_albedo(Z, sl) * I * ao[:, None]
        sh_ = np.array([84, 60, 72.0])[None] * (0.9 + 0.15 * hash2(np.floor(sl / 0.12), np.floor(Z / 0.2 + (np.floor(sl / 0.12) % 2) * 0.5)))[:, None]
        col[out_] = sh_[out_]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tc[ys, xs]
    fr.set_array(arr, depth)
    return sun_scr


def cut_edges(cam, fr):
    """the cut-away's sawn edges: floor slab, knee wall, gable top, roof"""
    slab = (78, 50, 36)
    edge = (176, 150, 120)
    G = S.gable_top
    polys = [
        ([[S.RX0 - S.WT, -S.SLAB, S.RZ1], [S.RX1, -S.SLAB, S.RZ1], [S.RX1, 0, S.RZ1], [S.RX0 - S.WT, 0, S.RZ1]], slab),
        ([[S.RX1, -S.SLAB, S.RZ1], [S.RX1, -S.SLAB, S.RZ0 - S.WT], [S.RX1, 0, S.RZ0 - S.WT], [S.RX1, 0, S.RZ1]], (58, 38, 28)),
        ([[S.RX1, 0, S.RZ0 - S.WT], [S.RX1, float(G(S.RX1)), S.RZ0 - S.WT], [S.RX1, float(G(S.RX1)), S.RZ0], [S.RX1, 0, S.RZ0]], (150, 128, 104)),
        ([[S.RX0 - S.WT, 0, S.RZ1], [S.RX0, 0, S.RZ1], [S.RX0, S.KNEE, S.RZ1], [S.RX0 - S.WT, S.KNEE, S.RZ1]], (150, 128, 104)),
    ]
    xs = np.linspace(S.RIDGE_X, S.RX1, 4)
    for a, b in zip(xs, xs[1:]):
        polys.append(([[a, float(G(a)), S.RZ0 - S.WT], [b, float(G(b)), S.RZ0 - S.WT], [b, float(G(b)), S.RZ0], [a, float(G(a)), S.RZ0]], edge))
    # roof slab cut edge at the front (thickness along the slope)
    n = -S.CEIL_N * 0.14
    a0, a1 = np.array([S.RX0, S.KNEE, S.RZ1]), np.array([S.RIDGE_X, S.RIDGE_H, S.RZ1])
    polys.append(([a0, a1, a1 + n, a0 + n], (120, 84, 62)))
    for Q, c in polys:
        fr.poly(cam, np.array(Q, float), c)


# ---------------------------------------------------------------- object shading
def draw_faces(fr, cam, L, faces, color, grp, emit=0.0, tone=None, double=False, sunvis=True):
    fc = [(cam.depth(Q.mean(0))[0], Q, n, j) for j, (Q, n) in enumerate(faces) if double or np.dot(n, cam.pos - Q.mean(0)) > 0]
    if not fc:
        return
    fc.sort(key=lambda a: -a[0])
    C = np.array([q.mean(0) for _, q, _, _ in fc])
    N = np.array([n for _, _, n, _ in fc])
    if double:
        flip = (N * (cam.pos[None] - C)).sum(1) < 0
        N = np.where(flip[:, None], -N, N)
    Pv = C + N * 0.01
    kv = (S.win_lit(Pv) * OCC.vis(Pv, grp, d=SUN))[:, None] if sunvis else np.zeros((len(C), 1))
    base = np.array([np.asarray(color, float) * (1 if tone is None else tone[j]) for *_, j in fc])
    cols = shade(L, N, C, base, kvis=kv, emit=emit)
    for (z, Q, n, j), c in zip(fc, cols):
        fr.poly(cam, Q, c)


def draw_obj(fr, cam, L, o, emit=0.0):
    draw_faces(fr, cam, L, o.faces, o.color, o.grp, emit=emit, tone=o.tone, double=o.double)


def hand(fr, cam, c, z, ang, length, w, col, back=0.15):
    """clock hand on a dial facing +z (ang = 0 at 12 o'clock, clockwise)"""
    u = np.array([math.sin(ang), math.cos(ang), 0.0])
    v = np.array([math.cos(ang), -math.sin(ang), 0.0])
    c = np.array([c[0], c[1], z])
    Q = np.array([c - u * length * back - v * w, c + u * length - v * w * 0.4, c + u * length + v * w * 0.4, c - u * length * back + v * w])
    fr.poly(cam, Q, col)


def draw_clock_hands(fr, cam, t, c, r, z, minute_ang):
    sec = 2 * math.pi * t / 60.0                       # one sweep per minute: loop-safe
    for k in range(12):
        a = 2 * math.pi * k / 12
        p0 = np.array([c[0] + math.sin(a) * r * 0.86, c[1] + math.cos(a) * r * 0.86, z])
        p1 = np.array([c[0] + math.sin(a) * r * 0.74, c[1] + math.cos(a) * r * 0.74, z])
        fr.line(cam, [p0, p1], (60, 44, 36), 1)
    hand(fr, cam, c, z, minute_ang, r * 0.5, r * 0.06, (40, 30, 28))
    hand(fr, cam, c, z + 0.001, minute_ang * 12 % (2 * math.pi), r * 0.72, r * 0.045, (40, 30, 28))
    hand(fr, cam, c, z + 0.002, sec, r * 0.8, r * 0.02, (196, 52, 40), back=0.25)


def draw_pendulum(fr, cam, L, pivot, length, ang, r_bob, col=(232, 180, 80)):
    tip = pivot + np.array([math.sin(ang) * length, -math.cos(ang) * length, 0])
    fr.line(cam, [pivot, tip], (150, 120, 70), 1)
    s = cam.wp(tip)
    zc = cam.depth(tip)[0]
    if zc > 0.1:
        rr = r_bob * cam.f / zc
        fr.disc(s, rr, col, n=10, z=zc)
        fr.disc((s[0] - rr * 0.3, s[1] - rr * 0.3), rr * 0.35, (255, 236, 170), n=6, z=zc)


# ---------------------------------------------------------------- the 15 lantern slots
def bob_light(k, P, fl):
    return BOB_COL[k]


def draw_slot(fr, cam, L, k, P, th, fl, glows):
    """strings, lantern, then the beam segment above (cameras look down on the rig)"""
    for a, b in S.string_pts(k, P, th):
        fr.line(cam, [a, b], (236, 226, 204), 1)
    glass, metal = S.bob_faces(k, P, th)
    draw_faces(fr, cam, L, metal, S.BRASS, S.RIG_G + 1000 + k)
    em = 0.85 + 0.9 * fl
    col = BOB_COL[k] * 0.62 + 18
    draw_faces(fr, cam, L, glass, col, S.RIG_G + 1000 + k, emit=em)
    # a hot core so each lantern reads as a light
    zc = cam.depth(P)[0]
    if zc > 0.1:
        s = cam.wp(P)
        rr = S.BOB_R * 0.45 * cam.f / zc
        core = np.minimum(255, BOB_COL[k] * 0.5 + 150 + 60 * fl)
        fr.disc(s, rr, core, n=8, z=zc)
        glows.append((s[0], s[1], (0.05 + 0.03 * fl) * cam.f / zc, tuple(BOB_COL[k]), 0.2 + 0.55 * fl))
    draw_faces(fr, cam, L, S.BEAM_SEG[k], (122, 80, 52), S.RIG_G)


def bob_shadow_mask(P, th):
    polys = []
    for k in range(T.K):
        glass, metal = S.bob_faces(k, P[k], th[k])
        for Q, n in glass + metal:
            polys.append(proj_dir(Q, SUN))
        for a, b in S.string_pts(k, P[k], th[k]):
            q = np.array([a + [0.0012, 0, 0], b + [0.0012, 0, 0], b - [0.0012, 0, 0], a - [0.0012, 0, 0]])
            polys.append(proj_dir(q, SUN))
    return FINE.raster(polys, blur=0.7)


# ---------------------------------------------------------------- sunbeam (volumetric, quarter resolution) + motes
QH, QW = H // 4, W // 4
_QY, _QX = np.mgrid[0:QH, 0:QW].astype(np.float32)


def sunbeam(cam, depth):
    dirs = cam.pixel_rays(_QX * 4 + 2, _QY * 4 + 2)
    dq = np.minimum(depth[2::4, 2::4], 12.0)
    n = 16
    acc = np.zeros((QH, QW), np.float32)
    jit = BAYER_FULL[2::4, 2::4] - 0.5
    for i in range(n):
        s = (i + 0.5 + jit) / n * dq
        P = cam.pos[None, None] + dirs * s[..., None]
        inside = (P[..., 0] > S.RX0) & (P[..., 0] < S.RX1) & (P[..., 2] > S.RZ0) & (P[..., 2] < S.RZ1) & (P[..., 1] > 0) & (P[..., 1] < 3.4)
        lit = S.win_lit(P.reshape(-1, 3)).reshape(QH, QW) * inside
        acc += lit * dq / n
    beam = 1 - np.exp(-acc * 0.5)
    im = Image.fromarray((beam * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.2))
    return np.asarray(im.resize((W, H), Image.BILINEAR), np.float32) / 255.0


def draw_motes(img, cam, depth, t):
    M = S.MOTES
    w = 2 * math.pi / 60.0
    P = M + np.stack([0.04 * np.sin(w * 3 * t + M[:, 1] * 5), 0.05 * np.sin(w * 2 * t + M[:, 0] * 3), 0.04 * np.cos(w * 4 * t + M[:, 2] * 4)], 1)
    lit = S.win_lit(P)
    zc = cam.depth(P)
    scr = cam.wp(P)
    tw = 0.6 + 0.4 * np.sin(w * 17 * t + M[:, 0] * 40)
    for z, l, (x, y), a in zip(zc, lit, scr, tw):
        if l <= 0 or z < 0.3:
            continue
        xi, yi = int(x), int(y)
        if 0 <= xi < W and 0 <= yi < H and depth[yi, xi] > z:
            img[yi, xi] = img[yi, xi] * (1 - 0.6 * a) + np.array([255, 236, 190.0]) * 0.6 * a


# ---------------------------------------------------------------- focus (locked on the rig)
def focus(cam, P_bob):
    zf = cam.depth(S.SUBJECT)[0]
    pts = np.concatenate([P_bob, [[S.RIG_X, S.TOP + 0.04, S.ZF], [S.RIG_X, S.TOP + 0.04, S.ZB], [S.RIG_X - 0.2, 0, S.ZF], [S.RIG_X + 0.2, 0, S.ZF],
                                  [S.RIG_X - 0.2, 0, S.ZB], [S.RIG_X + 0.2, 0, S.ZB]]])
    zr = cam.depth(pts)
    sharp = float(np.clip(np.abs(1 - zf / np.maximum(zr, 0.2)).max() + 0.03, 0.05, 0.9))
    K = 0.012 * cam.f * min(1.2, 2.0 / zf)
    return zf, sharp, K


def subject_blur(cam, P_bob, zf, sharp, K):
    """worst circle of confusion over every bob and the frame (must be 0)"""
    pts = np.concatenate([P_bob + d for d in ([0, 0, 0], [S.BOB_R, 0, 0], [-S.BOB_R, 0, 0], [0, S.BOB_R, 0], [0, -S.BOB_R, 0])])
    zr = cam.depth(pts)
    return float(coc_map(zr, zf, K, sharp).max())


# ---------------------------------------------------------------- overlays
def center_text(img, txt, x, y, col, scale=1, outline=(10, 20, 26)):
    m = text_mask(txt, scale)
    stamp(img, m, int(round(x - m.shape[1] / 2)), int(round(y)), col, outline=outline)


def line_add(img, p0, p1, col, a, dash=(5, 3), width=2):
    """dashed screen line (group tie-lines), so it never reads as another string"""
    n = int(max(abs(p1[0] - p0[0]), abs(p1[1] - p0[1]))) + 1
    u = np.arange(n)
    on = (u % (dash[0] + dash[1])) < dash[0]
    xs = np.linspace(p0[0], p1[0], n)[on]
    ys = np.linspace(p0[1], p1[1], n)[on]
    vertical = abs(p1[1] - p0[1]) > abs(p1[0] - p0[0])
    for o in range(width):
        xi = (xs + (o if not vertical else 0) * 0 + (o if vertical else 0)).astype(int)
        yi = (ys + (o if not vertical else 0)).astype(int)
        ok = (xi >= 0) & (xi < W) & (yi >= 0) & (yi < H)
        img[yi[ok], xi[ok]] = img[yi[ok], xi[ok]] * (1 - a) + np.array(col, float) * a


def tie_lines(img, cam, P_bob, t):
    g, a = T.tie_alpha(t)
    if not g:
        return
    scr = cam.wp(P_bob)
    ph = (T.phase(t) / (2 * math.pi)) % 1.0
    cls = np.round(ph * g).astype(int) % g
    for c in range(g):
        ks = [k for k in range(T.K) if cls[k] == c]
        if len(ks) < 2:
            continue
        col = mixc(BOB_COL[ks[len(ks) // 2]], (255, 255, 255), 0.25)
        for k0, k1 in zip(ks, ks[1:]):
            line_add(img, scr[k0], scr[k1], col, 0.9 * a)


def tags(img, cam, P_bob, t):
    out = []
    a0, a1 = T.SWING_TAGS
    b0, b1 = T.LEN_TAGS
    for k, sw, cm in ((0, '51/MIN', '34 CM'), (T.K - 1, '65/MIN', '21 CM')):
        s = cam.wp(P_bob[k] + [0, -0.05, 0])
        if a0 <= t < a1:
            u = ss(a0, a0 + 0.25, t) * (1 - ss(a1 - 0.15, a1, t))
            if u > 0.05:
                out.append((sw, s[0], s[1] + 4 + 6 * (1 - u), AMBER, 2))
        if b0 <= t < b1:
            u = ss(b0, b0 + 0.25, t) * (1 - ss(b1 - 0.15, b1, t))
            if u > 0.05:
                out.append((cm, s[0], s[1] + 4 + 6 * (1 - u), SKYB, 2))
    # group count pop-ups in the first half (the HUD takes over at 35 s)
    gn = T.groups_now(t, 1.1)
    if gn and t < T.HUD_IN and gn[1] in (10.0, 12.0, 15.0):
        g, tm, c = gn
        u = ss(tm - 1.1, tm - 0.7, t) * (1 - ss(tm + 0.75, tm + 1.1, t))
        out.append((f'{g} GROUPS', W / 2, 470 + 8 * (1 - u), MINT, 3))
    for txt, x, y, col, sc in out:
        half = (len(txt) * ADV * sc) / 2
        center_text(img, txt, float(np.clip(x, half + 6, W - half - 6)), y, col, sc)


def count_caption(img, t):
    """45-52 s: '4... 5... 6...' revealed as each group pattern lands"""
    a, b, parts, end = T.COUNT_CAP
    if not (a <= t < end):
        return
    line = ''
    for tp, seg in parts:
        if t >= tp:
            line += seg
    line = line.rstrip()
    u = t - a
    slide = int(round(6 * (1 - ease(u / 0.25))))
    fade = 1 - ss(end - 0.12, end, t)
    rich_line(img, line, W // 2 - 0, 42 + slide, 3, CREAM, AMBER, SKYB, alpha=fade)


def draw_hud(out, t):
    """phase ring: every pendulum is a dot going round once per swing; clumps = groups"""
    if not (T.HUD_IN - 0.4 <= t < T.HUD_OUT + 0.4):
        return out
    off = slide_offset(t, T.HUD_IN, T.HUD_OUT, 180)
    y0 = 528 + off
    pil = Image.fromarray(out)
    dr = ImageDraw.Draw(pil)
    panel(dr, 14, y0, 346, y0 + 96)
    cx, cy, R = 66, y0 + 48, 34
    dr.ellipse([cx - R, cy - R, cx + R, cy + R], outline=(70, 84, 124))
    dr.ellipse([cx - 3, cy - R - 4, cx + 3, cy - R + 2], fill=(255, 236, 190))
    ph = T.phase(t)
    fl = T.flash(t)
    for k in range(T.K):
        a = ph[k]
        rr = R - 2 - (k % 3) * 0.0
        x, y = cx + rr * math.sin(a), cy - rr * math.cos(a)
        s = 3.2 + 1.6 * fl[k]
        c = tuple(int(v) for v in np.minimum(255, BOB_COL[k] * (0.85 + 0.3 * fl[k])))
        dr.ellipse([x - s, y - s, x + s, y + s], fill=c, outline=(12, 16, 30))
    out = np.asarray(pil).copy()
    gn = T.groups_now(t, 1.0)
    center_text(out, 'GROUPS', 236, y0 + 12, SKYB, 2)
    if gn:
        g, tm, c = gn
        center_text(out, str(g), 236, y0 + 34, AMBER, 4)
    else:
        center_text(out, '?', 236, y0 + 34, (150, 160, 190), 4)
    left = 60.0 - t
    center_text(out, f'LOOP IN {left:4.1f} S', 236, y0 + 74, MINT, 1)
    return out


# ---------------------------------------------------------------- render
def scene_state(t):
    cam = camera(t)
    P, th = S.bob_pos(t)
    return cam, P, th


def render(f):
    t = f / FPS
    cam, P_bob, th = scene_state(t)
    fl = T.flash(t)
    L, ks = make_lights(t, cam, fl)
    fr = Frame(sky_gradient(cam, (40, 30, 70), (128, 76, 104), step=7.0))
    fr.bob_mask = bob_shadow_mask(P_bob, th)
    cut_edges(cam, fr)
    sun_scr = shell_pass(fr, cam, t, L, ks, P_bob, fl)
    glows, labels = [], []
    items = [(np.linalg.norm(o.ctr - cam.pos), 'obj', o) for o in S.OBJS]
    for k in range(T.K):
        mid = np.array([S.RIG_X, 0.5 * (P_bob[k][1] + S.TOP), S.ZK[k]])
        items.append((np.linalg.norm(mid - cam.pos), 'slot', k))
    for i, (c, r, col, per) in enumerate(S.WALL_CLOCKS):
        items.append((np.linalg.norm(c - cam.pos) - 0.05, 'wclock', i))
    items.append((np.linalg.norm(S.GC_DIAL - cam.pos) - 0.05, 'gcdial', None))
    items.append((np.linalg.norm(S.GC_PIVOT + [0, -0.4, 0.03] - cam.pos) - 0.06, 'gcpend', None))
    items.append((np.linalg.norm(S.CAT_P + [0, 0.15, 0] - cam.pos), 'cat', None))
    items.append((np.linalg.norm(S.LAMP2 + [0, 0.3, 0] - cam.pos), 'chain', None))
    for _, kind, o in painter(items):
        if kind == 'obj':
            if o.name == 'chimney':
                draw_obj(fr, cam, L, o, emit=0.9)
            elif o.name == 'rafter':
                draw_faces(fr, cam, L, o.faces, o.color, o.grp, tone=o.tone, sunvis=False)
            else:
                draw_obj(fr, cam, L, o)
        elif kind == 'slot':
            draw_slot(fr, cam, L, o, P_bob[o], th[o], fl[o], glows)
        elif kind == 'wclock':
            c, r, col, per = S.WALL_CLOCKS[o]
            zd = (S.RZ0 + 0.086) if per else (c[2] + 0.008)
            draw_clock_hands(fr, cam, t, c, r * 0.84 if not per else r * 0.85, zd, [1.1, 4.2, 2.6][o])
            if per:
                pv = np.array([c[0], c[1] - r - 0.1, S.RZ0 + 0.058])
                draw_pendulum(fr, cam, L, pv, 0.17, 0.2 * math.sin(2 * math.pi * t / per), 0.018)
        elif kind == 'gcdial':
            draw_clock_hands(fr, cam, t, S.GC_DIAL, 0.13, S.GC_DIAL[2] + 0.002, 5.3)
        elif kind == 'gcpend':
            x0, x1, y0, y1, zg = S.GC_WIN
            fr.poly(cam, np.array([[x0, y0, zg], [x1, y0, zg], [x1, y1, zg], [x0, y1, zg]]), (44, 30, 30))
            draw_pendulum(fr, cam, L, S.GC_PIVOT + [0, -0.03, 0.03], 0.62, 0.13 * math.sin(2 * math.pi * t / 2.0), 0.045)
            fr.poly(cam, np.array([[x0, y1 - 0.02, zg + 0.031], [x1, y1 - 0.02, zg + 0.031], [x1, y1, zg + 0.031], [x0, y1, zg + 0.031]]), (104, 54, 36))
        elif kind == 'cat':
            faces, eyes, blink = S.cat_faces(t, P_bob[1][0] - S.RIG_X)
            for name, fcs in faces:
                draw_faces(fr, cam, L, fcs, (104, 102, 118) if name != 'tail' else (92, 90, 106), -11)
            for e in eyes:
                zc = cam.depth(e)[0]
                if zc > 0.1 and np.dot(cam.pos - e, nrm(e - (S.CAT_P + [0, 0.28, 0]))) > 0:
                    s = cam.wp(e)
                    if blink:
                        fr.draw.line([s[0] - 1, s[1], s[0] + 1, s[1]], fill=(30, 26, 30))
                    else:
                        fr.draw.rectangle([s[0] - 0.6, s[1] - 0.6, s[0] + 0.6, s[1] + 0.6], fill=(214, 236, 120))
        elif kind == 'chain':
            fr.line(cam, list(S.CHAIN), (70, 64, 58), 1)
    # lamp flames + glows
    for i, (lp, R, k) in enumerate(S.LAMPS):
        zc = cam.depth(lp)[0]
        if zc < 0.2:
            continue
        s = cam.wp(lp)
        f_ = flicker(t, i)
        r = 0.018 * cam.f / zc
        fr.draw.ellipse([s[0] - r * 0.5, s[1] - r * 1.4, s[0] + r * 0.5, s[1] + r * 0.4], fill=(255, 232, 160))
        glows.append((s[0], s[1] - r * 0.3, 0.2 * cam.f / zc * f_, (255, 150, 60), 0.5 * f_))
    if sun_scr is not None:
        glows.append((sun_scr[0], sun_scr[1], 0.6 * cam.f / 4.0, (255, 170, 90), 0.35))
    # lens: focus locked on the rig, the sharp band covers every bob and the frame
    img, depth = fr.array(), fr.depth()
    zf, sharp, K = focus(cam, P_bob)
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    beam = sunbeam(cam, depth)
    img = img + beam[..., None] * np.array([255, 214, 160.0])[None, None] * 0.13
    draw_motes(img, cam, depth, t)
    img = apply_glows(img, fr.glows + glows)
    img = grade(bloom(np.clip(img, 0, 255)))
    img = np.clip(img * vignette() * top_scrim(), 0, 255)
    tie_lines(img, cam, P_bob, t)
    out = np.clip(img, 0, 255).astype(np.uint8)
    tags(out, cam, P_bob, t)
    out = draw_hud(out, t)
    draw_captions(out, t, T.CAPTIONS)
    count_caption(out, t)
    if os.environ.get('COC_CHECK'):
        render.last_blur = subject_blur(cam, P_bob, zf, sharp, K)
    return out
