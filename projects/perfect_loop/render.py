"""The Perfect Loop v2 (100 pendulums) - renderer.

    python -m reelkit.sheet render.py test/sheet.png 0.3 2 5 6.8 10 12 19 22 30 37 46 50.5 52 55
    python -m reelkit.video render.py audio.wav perfect_loop_v2.mp4

Per frame: dusk void -> cut-away edges -> ray-traced gallery shell (plank floor with four sun pools
through the round windows in the left wall, the bobs' moving shadows, coloured bob light, lamp light
+ static shadow masks, timber-framed walls, sloped ceiling, sunset backdrop in the windows) -> objects
in painter order (sunlit through the windows + AABB occluders, rim-lit), the 100 bob slots (string,
lantern, beam segment), clocks, cat -> locked depth of field -> sunbeams + motes -> light trails ->
glows -> bloom -> grade -> vignette/scrim -> group tie-lines, tags, HUD (phase ring + group counter),
captions."""
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
import timeline as T
import scene as S

# glyphs this video needs (added at runtime, reelkit itself is unchanged)
for ch, rows in {'→': ".....|..#..|...#.|#####|...#.|..#..|.....",
}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

# ---------------------------------------------------------------- lights + shadow masks
SUN = S.SUN
WARM = np.array([1.0, 0.63, 0.32])
BOUNCE = (np.array([0.6, 0.08, -0.4]), 1.8, np.array([0.2, 0.15, 0.1]))        # fake GI from the sunlit floor pools
FLOOR = MaskGrid((S.RX0, S.RX1, S.RZ0, S.RZ1, 0.01))
FINE = MaskGrid((-0.5, 1.2, -2.08, 1.2, 0.004))                                 # the bobs' moving shadows
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


def make_lights(t, cam):
    L = Lights(amb=(0.42, 0.42, 0.55), sky=(0.05, 0.05, 0.08))
    L.view = cam.view
    L.key('sun', SUN, S.SUN_RGB * 1.05)
    L.fill(nrm([0.6, 0.6, 0.6]), (0.07, 0.08, 0.14))
    ks = []
    for i, (lp, R, k) in enumerate(S.LAMPS):
        kk = k * flicker(t, i)
        ks.append(kk)
        L.point(lp, WARM * kk * 0.9, R)
    L.point(BOUNCE[0], BOUNCE[2], BOUNCE[1])
    return L, ks


# ---------------------------------------------------------------- camera: gentle 2.5D moves, few cuts
RX, ZM = S.RIG_X, S.Z_MID
HOOK = (0.0, np.array([RX, 0.58, -0.2]), 1.7, 14, 27, 44, 400)
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)   target sits at screen y = sy
    HOOK,
    (2.2, np.array([RX, 0.56, -0.35]), 1.95, 10, 31, 44, 395),
    (4.6, np.array([RX, 0.5, ZM + 0.2]), 3.3, 22, 30, 44, 380),           # both ends: 34 CM / 4 CM
    (6.6, np.array([-0.45, 1.0, -0.7]), 5.2, 46, 25, 50, 330),            # establishing: windows, beams, the gallery
    (8.0, np.array([RX, 0.6, 0.3]), 1.5, 36, 22, 44, 390),                # glide along the row, front ...
    (11.3, np.array([RX, 0.5, -1.4]), 1.5, 42, 22, 44, 395),              # ... to the long red end
    (13.2, np.array([RX, 0.5, ZM + 0.05]), 3.05, 8, 40, 42, 355),          # rise: the iconic row-up-the-frame view
    (16.4, np.array([RX, 0.5, ZM + 0.05]), 2.95, 6, 42, 42, 355),
    (18.6, np.array([RX, 0.56, -0.3]), 1.65, 13, 17, 44, 400),            # chaos: low and close, down the row
    (20.6, np.array([RX, 0.55, -0.45]), 1.85, 7, 22, 44, 395),
    (21.7, np.array([RX, 0.5, ZM + 0.05]), 2.7, 2, 32, 42, 380),          # halftime: end-on, 2 teams
    (26.4, np.array([RX, 0.5, ZM + 0.05]), 2.6, 4, 34, 42, 380),
    (29.0, np.array([RX, 0.5, ZM + 0.1]), 3.7, 7, 40, 42, 330),         # HUD up: subject higher
    (36.0, np.array([RX, 0.5, ZM + 0.1]), 3.6, 9, 39, 42, 330),         # count the groups
    (42.0, np.array([RX, 0.5, ZM + 0.1]), 3.6, 7, 41, 42, 326),
    (47.5, np.array([RX, 0.5, ZM + 0.12]), 3.55, 6, 43, 42, 324),          # the snake unwinds ...
    (51.6, np.array([RX, 0.5, ZM + 0.12]), 3.5, 5, 44, 42, 324),
    (52.8, np.array([RX, 0.52, ZM + 0.12]), 3.45, 7, 42, 42, 324),          # ... into 1 line
    (55.0, np.array([RX, 0.55, ZM + 0.15]), 3.5, 20, 30, 44, 345),         # try it: pull back
    (57.6, np.array([RX, 0.58, -0.1]), 1.8, 15, 27, 44, 398),
    (60.0,) + HOOK[1:],
]


PUNCHES = [(T.T_LINE, 0.075, 0.32), (22.0, 0.03, 0.25)]       # (time, zoom, decay): a lens punch on the drop


def camera(t):
    cam = interp_keys(KEYS, t)
    for tp_, amt, dec in PUNCHES:
        if tp_ <= t < tp_ + 6 * dec:
            cam.f *= 1 + amt * math.exp(-(t - tp_) / dec)
    return cam


# ---------------------------------------------------------------- procedural textures
def hash2(a, b):
    v = np.sin(a * 127.1 + b * 311.7) * 43758.5453
    return v - np.floor(v)


def floor_albedo(X, Z):
    """wide planks running along the gallery, staggered joints, nail dots"""
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


TIMBER = np.array([90, 58, 42.0])


def back_albedo(X, Y):
    """warm plaster between dark timber framing (gable end)"""
    col = np.zeros((len(X), 3))
    n = hash2(np.floor(X * 9), np.floor(Y * 9))
    col[:] = np.array([196, 186, 172.0])[None] * (0.95 + 0.06 * n)[:, None]
    post = (np.abs(X + 1.2) < 0.05) | (np.abs(X - 0.75) < 0.05) | (np.abs(X - 1.5) < 0.04)
    beam = (np.abs(Y - S.WALL_H) < 0.055)
    sill = np.abs(Y - 0.1) < 0.1
    brace = (np.abs((Y - 0.2) - (X + 1.15) * 1.25) < 0.05) & (X > -1.15) & (X < -0.6) & (Y < S.WALL_H)
    brace |= (np.abs((Y - 2.25) + (X - 0.8) * 1.25) < 0.05) & (X > 0.8) & (X < 1.45) & (Y > 1.4)
    king = (np.abs(X - S.RIDGE_X) < 0.05) & (Y > S.WALL_H)
    tm = post | beam | sill | brace | king
    grain = 0.9 + 0.12 * hash2(np.floor(X * 40), np.floor(Y * 6))
    col[tm] = TIMBER[None] * grain[tm, None]
    top = Y > S.gable_top(X) - 0.06
    col[top] = TIMBER * 0.9
    return col


def left_albedo(Z, Y):
    """the window wall: plaster, timber posts between the windows, sill and wall plate"""
    col = np.zeros((len(Z), 3))
    n = hash2(np.floor(Z * 9), np.floor(Y * 9))
    col[:] = np.array([194, 182, 166.0])[None] * (0.95 + 0.06 * n)[:, None]
    mids = list(0.5 * (S.WZ[1:] + S.WZ[:-1])) + [S.RZ0 + 0.06, S.RZ1 - 0.08]
    post = np.zeros(len(Z), bool)
    for m in mids:
        post |= np.abs(Z - m) < 0.045
    plate = np.abs(Y - (S.WALL_H - 0.06)) < 0.06
    sill = Y < 0.16
    rail = (np.abs(Y - 0.62) < 0.035)
    tm = post | plate | sill | rail
    grain = 0.9 + 0.12 * hash2(np.floor(Z * 40), np.floor(Y * 6))
    col[tm] = TIMBER[None] * grain[tm, None]
    return col


def ceil_albedo(Z, S_):
    """boards along the slope (rafters are real objects)"""
    b = np.floor(S_ / 0.18)
    col = np.array([116, 86, 68.0])[None] * (0.86 + 0.18 * hash2(b, 5.0))[:, None]
    col[np.abs((S_ / 0.18) % 1.0 - 0.5) > 0.47] *= 0.72
    return col


def backdrop(X, Y, t, sun_xy):
    """sunset over the rooftops, seen through the round windows (positions relative to the sun)"""
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
    for cy_, amp, sp, wd in ((0.55, 0.06, 0.05, 0.035), (0.95, 0.08, -0.03, 0.03), (1.35, 0.05, 0.04, 0.025)):
        cl = sy + cy_ + amp * np.sin((X - sx) * 2.1 + 2 * math.pi * t / 60.0)
        band = (np.abs(Y - cl) < wd * (1 + 0.6 * np.sin((X - sx) * 5.0 + cy_ * 9)))
        col[band] = col[band] * 0.45 + np.array([255, 170, 150.0]) * 0.55
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
    for i, (v, y0, ph) in enumerate(((3, 0.5, 0.0), (4, 0.75, 0.35), (2, 0.25, 0.7))):     # birds: whole passes per minute
        u = ((v * t / 60.0 + ph) % 1.0)
        bx = sx - 2.4 + 4.8 * u
        by = sy + y0 + 0.04 * math.sin(2 * math.pi * 9 * t / 60.0 + i)
        flap = 0.025 * math.sin(2 * math.pi * 60 * t / 60.0 * (1 + 0.3 * i) + i)
        ddx = np.abs(X - bx)
        bird = (ddx < 0.06) & (np.abs((Y - by) - ddx * 0.55 + flap * (ddx / 0.06)) < 0.011)
        col[bird] = [70, 40, 70]
    return col


# ---------------------------------------------------------------- coloured light from the 100 lanterns (coarse map)
LM_EXT = (S.RIG_X - 0.42, S.RIG_X + 0.42, S.ZB - 0.25, S.ZF + 0.25, 0.02)
_LX = np.arange(LM_EXT[0], LM_EXT[1], LM_EXT[4])
_LZ = np.arange(LM_EXT[2], LM_EXT[3], LM_EXT[4])
_LGZ, _LGX = np.meshgrid(_LZ, _LX, indexing='ij')


def bob_lightmap(P_bob, fl):
    """(nz, nx, 3) coloured light on the floor under the row; a burst on every pluck"""
    d2 = (_LGX[..., None] - P_bob[None, None, :, 0]) ** 2 + P_bob[None, None, :, 1] ** 2 + (_LGZ[..., None] - P_bob[None, None, :, 2]) ** 2
    w = (P_bob[None, None, :, 1] / np.sqrt(d2)) * (0.05 + 0.2 * fl[None, None]) / (1 + d2 / 0.02)
    return w @ (BOB_COL / 255.0)


def sample_lightmap(lm, X, Z):
    x0, x1, z0, z1, res = LM_EXT
    fx = (X - x0) / res
    fz = (Z - z0) / res
    ok = (fx >= 0) & (fx < lm.shape[1] - 1) & (fz >= 0) & (fz < lm.shape[0] - 1)
    out = np.zeros((len(X), 3))
    if ok.any():
        ix, iz = np.floor(fx[ok]).astype(int), np.floor(fz[ok]).astype(int)
        ux, uz = (fx[ok] - ix)[:, None], (fz[ok] - iz)[:, None]
        out[ok] = (lm[iz, ix] * (1 - ux) * (1 - uz) + lm[iz, ix + 1] * ux * (1 - uz) + lm[iz + 1, ix] * (1 - ux) * uz + lm[iz + 1, ix + 1] * ux * uz)
    return out


# ---------------------------------------------------------------- the gallery shell, per pixel
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
        tl = np.where(dw[..., 0] < -1e-6, (S.RX0 - p[0]) / dw[..., 0], INF)           # window wall x = RX0
        Yl, Zl = p[1] + tl * dw[..., 1], p[2] + tl * dw[..., 2]
        okl = (tl > 0) & (Zl >= S.RZ0) & (Zl <= S.RZ1) & (Yl >= 0) & (Yl <= S.WALL_H)
        den = dw[..., 1] - dw[..., 0] * S.SLOPE                                        # sloped ceiling
        tc = (S.WALL_H + (p[0] - S.RX0) * S.SLOPE - p[1]) / np.where(np.abs(den) < 1e-6, 1e-6, den)
        Xc, Yc, Zc = p[0] + tc * dw[..., 0], p[1] + tc * dw[..., 1], p[2] + tc * dw[..., 2]
        okc = (tc > 0) & (Xc >= S.RX0) & (Xc <= S.RIDGE_X) & (Zc >= S.RZ0) & (Zc <= S.RZ1)
    outside = (dw @ S.CEIL_N) > 0                                                       # roof seen from above
    TT = [np.where(m, tt, INF) for m, tt in ((okf, tf), (okb, tb), (okl, tl), (okc, tc))]
    tmin = np.minimum(np.minimum(TT[0], TT[1]), np.minimum(TT[2], TT[3]))
    mf = okf & (TT[0] <= tmin)
    mb = okb & (TT[1] <= tmin) & ~mf
    ml = okl & (TT[2] <= tmin) & ~mf & ~mb
    mc = okc & (TT[3] <= tmin) & ~mf & ~mb & ~ml
    winpx = ml & S.win_open(Zl, Yl)

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

    def window_glow(Z, Y, k=0.2):
        d = np.full(len(Z), 9.0)
        for wz in S.WZ:
            d = np.minimum(d, np.sqrt((Z - wz) ** 2 + (Y - S.WY) ** 2) - S.WR)
        return (np.clip(1 - d / 0.45, 0, 1) ** 2 * k)[:, None] * S.SUN_RGB[None]

    # ---- floor: four sun pools through the windows, static + moving bob shadows, lantern light
    ys, xs = np.nonzero(mf)
    if len(xs):
        X, Z = Xf[ys, xs], Zf[ys, xs]
        P = np.stack([X, np.zeros_like(X), Z], 1)
        alb = floor_albedo(X, Z)
        I = np.tile(L.amb + L.sky, (len(X), 1))
        sh = np.maximum(FLOOR.sample(SUN_MASK, X, Z), FINE.sample(fr.bob_mask, X, Z))
        slit = S.win_lit(P + [0, 0.002, 0]) * (1 - sh)
        I += (slit * SUN[1] * 1.45)[:, None] * S.SUN_RGB[None]
        I += lamps_at(P, np.array([0, 1.0, 0]), True)
        I += sample_lightmap(fr.bob_lm, X, Z)
        I *= contact_ao(X, Z, S.FEET)[:, None]
        I *= (1 - 0.3 * np.exp(-(X - S.RX0) / 0.25) - 0.3 * np.exp(-(Z - S.RZ0) / 0.25))[:, None]
        col = alb * I
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tf[ys, xs]
    # ---- back wall (gable end)
    ys, xs = np.nonzero(mb)
    if len(xs):
        X, Y = Xb[ys, xs], Yb[ys, xs]
        P = np.stack([X, Y, np.full_like(X, S.RZ0)], 1)
        I = np.tile(L.amb * 0.92, (len(X), 1)) + lamps_at(P, np.array([0, 0, 1.0]))
        I += (np.clip(np.array([0, 0, 1.0]) @ L.fills[0][0], 0, None) * L.fills[0][1])[None]
        ao = (1 - 0.35 * np.exp(-Y / 0.2)) * (1 - 0.3 * np.exp(-(X - S.RX0) / 0.3)) * (1 - 0.25 * np.exp(-(S.gable_top(X) - Y) / 0.25))
        col = back_albedo(X, Y) * I * ao[:, None]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tb[ys, xs]
    # ---- window wall (backlit: ambient, lamps and the glow scattered around each window)
    ys, xs = np.nonzero(ml & ~winpx)
    if len(xs):
        Z, Y = Zl[ys, xs], Yl[ys, xs]
        P = np.stack([np.full_like(Z, S.RX0), Y, Z], 1)
        I = np.tile(L.amb * 0.9, (len(Z), 1)) + lamps_at(P, np.array([1.0, 0, 0])) + window_glow(Z, Y)
        I += (np.clip(np.array([1.0, 0, 0]) @ L.fills[0][0], 0, None) * L.fills[0][1])[None]
        ao = (1 - 0.3 * np.exp(-Y / 0.2)) * (1 - 0.3 * np.exp(-(Z - S.RZ0) / 0.3)) * (1 - 0.25 * np.exp(-(S.WALL_H - Y) / 0.2))
        col = left_albedo(Z, Y) * I * ao[:, None]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tl[ys, xs]
    ys, xs = np.nonzero(winpx)
    sun_scr = None
    spt = np.array([S.RX0, S.WY - 0.13, S.WZ[S.WSUN] - 0.1])          # the sun is pinned low in one window
    if len(xs):
        Zw, Yw = Zl[ys, xs], Yl[ys, xs]
        d2 = dw[ys, xs]
        tr = (S.RX0 - S.WT - p[0]) / d2[:, 0]
        reveal = ~S.win_open(p[2] + tr * d2[:, 2], p[1] + tr * d2[:, 1])
        mull = S.win_mullion(Zw, Yw)
        tbd = (S.BACKDROP_X - p[0]) / d2[:, 0]
        Zbd, Ybd = p[2] + tbd * d2[:, 2], p[1] + tbd * d2[:, 1]
        dirs = spt - p
        s_ = (S.BACKDROP_X - p[0]) / dirs[0]
        sun_zy = (p[2] + s_ * dirs[2], p[1] + s_ * dirs[1])
        col = backdrop(-Zbd, Ybd, t, (-sun_zy[0], sun_zy[1]))              # screen-right = -z when looking at the wall
        frame_c = np.array([92, 58, 40.0])
        Iw = (L.amb * 0.55)[None] + lamps_at(np.stack([np.full_like(Zw, S.RX0), Yw, Zw], 1), np.array([1.0, 0, 0])) * 0.8
        col[mull] = (frame_c[None] * Iw)[mull]
        rc = np.array([200, 176, 146.0])[None] * (Iw * 0.8 + np.array([0.34, 0.22, 0.13]))
        col[reveal & ~mull] = rc[reveal & ~mull]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = np.where(reveal | mull, tl[ys, xs], 5e5 + tbd)
        sun_scr = cam.wp(np.array([S.BACKDROP_X, sun_zy[1], sun_zy[0]]))
    # ---- sloped ceiling (inside) / roof (outside)
    ys, xs = np.nonzero(mc)
    if len(xs):
        X, Y, Z = Xc[ys, xs], Yc[ys, xs], Zc[ys, xs]
        P = np.stack([X, Y, Z], 1)
        sl = (X - S.RX0) * math.sqrt(1 + S.SLOPE ** 2)
        out_ = outside[ys, xs]
        I = np.tile(L.amb * 0.85, (len(X), 1)) + lamps_at(P, S.CEIL_N)
        ao = (1 - 0.3 * np.exp(-(Y - S.WALL_H) / 0.25)) * (1 - 0.25 * np.exp(-(Z - S.RZ0) / 0.3))
        col = ceil_albedo(Z, sl) * I * ao[:, None]
        sh_ = np.array([84, 60, 72.0])[None] * (0.9 + 0.15 * hash2(np.floor(sl / 0.12), np.floor(Z / 0.2 + (np.floor(sl / 0.12) % 2) * 0.5)))[:, None]
        col[out_] = sh_[out_]
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tc[ys, xs]
    fr.set_array(arr, depth)
    return sun_scr


def cut_edges(cam, fr):
    """the cut-away's sawn edges: floor slab, window wall, gable top, roof"""
    slab = (78, 50, 36)
    edge = (176, 150, 120)
    G = S.gable_top
    X0, X1, Z0, Z1, WT = S.RX0, S.RX1, S.RZ0, S.RZ1, S.WT
    polys = [
        ([[X0 - WT, -S.SLAB, Z1], [X1, -S.SLAB, Z1], [X1, 0, Z1], [X0 - WT, 0, Z1]], slab),
        ([[X1, -S.SLAB, Z1], [X1, -S.SLAB, Z0 - WT], [X1, 0, Z0 - WT], [X1, 0, Z1]], (58, 38, 28)),
        ([[X1, 0, Z0 - WT], [X1, float(G(X1)), Z0 - WT], [X1, float(G(X1)), Z0], [X1, 0, Z0]], (150, 128, 104)),
        ([[X0 - WT, 0, Z1], [X0, 0, Z1], [X0, S.WALL_H, Z1], [X0 - WT, S.WALL_H, Z1]], (150, 128, 104)),
    ]
    xs = np.linspace(S.RIDGE_X, X1, 4)
    for a, b in zip(xs, xs[1:]):
        polys.append(([[a, float(G(a)), Z0 - WT], [b, float(G(b)), Z0 - WT], [b, float(G(b)), Z0], [a, float(G(a)), Z0]], edge))
    n = -S.CEIL_N * 0.14
    a0, a1 = np.array([X0, S.WALL_H, Z1]), np.array([S.RIDGE_X, S.RIDGE_H, Z1])
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


# ---------------------------------------------------------------- the 100 lantern slots
STRING_COL = (176, 164, 146)


def draw_slot(fr, cam, L, k, P, th, fl, glows):
    """string, lantern, then the beam segment above (cameras look down on the rig)"""
    a, b = S.string_pts(k, P, th)[0]
    a = np.array([S.RIG_X, S.TOP, S.ZK[k]])
    fr.line(cam, [a, b], STRING_COL, 1)
    glass, metal = S.bob_faces(k, P, th)
    draw_faces(fr, cam, L, metal, S.BRASS, S.RIG_G + 1000 + k)
    col = BOB_COL[k] * 0.55 + 8
    draw_faces(fr, cam, L, glass, col, S.RIG_G + 1000 + k, emit=0.55 + 0.7 * fl)
    zc = cam.depth(P)[0]
    if zc > 0.1:                                       # a saturated core so each lantern reads as its colour
        s = cam.wp(P)
        rr = max(0.6, S.BOB_R * 0.48 * cam.f / zc)
        core = np.minimum(255, BOB_COL[k] * (0.95 + 0.15 * fl) + 25 * fl)
        fr.disc(s, rr, core, n=6, z=zc)
        if fl > 0.08:                                  # the glow swells only on a pluck
            glows.append((s[0], s[1], max(1.5, (0.016 + 0.02 * fl) * cam.f / zc), tuple(BOB_COL[k] * 0.9), 0.32 * fl))
    draw_faces(fr, cam, L, S.BEAM_SEG[k], (122, 80, 52), S.RIG_G)


def bob_shadow_mask(P, th):
    polys = []
    for k in range(T.K):
        glass, metal = S.bob_faces(k, P[k], th[k])
        for Q, n in glass[::2] + metal[:6]:
            polys.append(proj_dir(Q, SUN))
        a = np.array([S.RIG_X, S.TOP, S.ZK[k]])
        b = S.string_pts(k, P[k], th[k])[0][1]
        q = np.array([a + [0, 0, 0.0011], b + [0, 0, 0.0011], b - [0, 0, 0.0011], a - [0, 0, 0.0011]])
        polys.append(proj_dir(q, SUN))
    return FINE.raster(polys, blur=0.6)


# ---------------------------------------------------------------- sunbeams (volumetric, quarter resolution) + motes
QH, QW = H // 4, W // 4
_QY, _QX = np.mgrid[0:QH, 0:QW].astype(np.float32)


def sunbeam(cam, depth):
    dirs = cam.pixel_rays(_QX * 4 + 2, _QY * 4 + 2)
    dq = np.minimum(depth[2::4, 2::4], 14.0)
    n = 18
    acc = np.zeros((QH, QW), np.float32)
    jit = BAYER_FULL[2::4, 2::4] - 0.5
    for i in range(n):
        s = (i + 0.5 + jit) / n * dq
        P = cam.pos[None, None] + dirs * s[..., None]
        inside = (P[..., 0] > S.RX0) & (P[..., 0] < S.RX1) & (P[..., 2] > S.RZ0) & (P[..., 2] < S.RZ1) & (P[..., 1] > 0) & (P[..., 1] < S.WALL_H + 0.4)
        lit = S.win_lit(P.reshape(-1, 3)).reshape(QH, QW) * inside
        acc += lit * dq / n
    beam = 1 - np.exp(-acc * 0.55)
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


# ---------------------------------------------------------------- light trails: each lantern smears a faint ribbon behind it
TRAIL_N, TRAIL_DT = 6, 0.02


def draw_trails(img, cam, depth, t):
    pts, cols, alph = [], [], []
    prev = None
    for j in range(TRAIL_N + 1):
        P, _ = S.bob_pos(t - j * TRAIL_DT)
        if prev is not None:
            a = 0.5 * (1 - (j - 1) / TRAIL_N) ** 1.3
            for u in np.linspace(0, 1, 5, endpoint=False):
                pts.append(prev + (P - prev) * u)
                cols.append(BOB_COL)
                alph.append(np.full(T.K, a * (1 - u * 0.2)))
        prev = P
    Pw = np.concatenate(pts)
    C = np.concatenate(cols)
    A = np.concatenate(alph)
    z = cam.depth(Pw)
    s = cam.wp(Pw)
    xi, yi = np.round(s[:, 0]).astype(int), np.round(s[:, 1]).astype(int)
    ok = (z > 0.2) & (xi >= 0) & (xi < W) & (yi >= 0) & (yi < H)
    xi, yi, z, C, A = xi[ok], yi[ok], z[ok], C[ok], A[ok]
    vis = depth[yi, xi] > z - 0.03
    xi, yi, C, A = xi[vis], yi[vis], C[vis], A[vis]
    o = np.argsort(A)                                  # strongest sample wins each pixel (no summing to white)
    lay = np.zeros((H, W, 3), np.float32)
    la = np.zeros((H, W), np.float32)
    lay[yi[o], xi[o]] = C[o]
    la[yi[o], xi[o]] = A[o]
    img += (255.0 - np.clip(img, 0, 255)) * (lay / 255.0) * la[..., None]          # screen blend


# ---------------------------------------------------------------- focus (locked on the rig)
def on_screen(cam, P, margin=12):
    z = cam.depth(P)
    s = cam.wp(P)
    return (z > 0.15) & (s[:, 0] > -margin) & (s[:, 0] < W + margin) & (s[:, 1] > -margin) & (s[:, 1] < H + margin)


def focus(cam, P_bob):
    """zf on the rig centre; the sharp band covers every bob, string top and stand foot that is on screen"""
    zf = cam.depth(S.SUBJECT)[0]
    tops = np.stack([np.full(T.K, S.RIG_X), np.full(T.K, S.TOP + 0.03), S.ZK], 1)
    feet = np.array([[S.RIG_X + sx * 0.21, 0, z] for sx in (-1, 1) for z in (S.ZF, S.ZB)])
    pts = np.concatenate([P_bob, tops, feet])
    pts = pts[on_screen(cam, pts, 30)]
    zr = cam.depth(pts) if len(pts) else np.array([zf])
    sharp = float(np.clip(np.abs(1 - zf / np.maximum(zr, 0.2)).max() + 0.04, 0.05, 2.5))
    K = 0.012 * cam.f * min(1.2, 2.0 / zf)
    return zf, sharp, K


def subject_blur(cam, P_bob, zf, sharp, K):
    """worst circle of confusion over every on-screen bob (centre and rim) (must be 0)"""
    vis = on_screen(cam, P_bob)
    if not vis.any():
        return 0.0
    Pv = P_bob[vis]
    pts = np.concatenate([Pv + d for d in ([0, 0, 0], [S.BOB_R, 0, 0], [-S.BOB_R, 0, 0], [0, S.BOB_R, 0], [0, -S.BOB_R, 0], [0, 0, S.BOB_R], [0, 0, -S.BOB_R])])
    zr = cam.depth(pts)
    return float(coc_map(zr, zf, K, sharp).max())


# ---------------------------------------------------------------- overlays
def center_text(img, txt, x, y, col, scale=1, outline=(10, 20, 26)):
    m = text_mask(txt, scale)
    stamp(img, m, int(round(x - m.shape[1] / 2)), int(round(y)), col, outline=outline)


def line_add(img, p0, p1, col, a, dash=(4, 3), width=1):
    """dashed screen line (group tie-lines), so it never reads as another string"""
    n = int(max(abs(p1[0] - p0[0]), abs(p1[1] - p0[1]))) + 1
    u = np.arange(n)
    on = (u % (dash[0] + dash[1])) < dash[0]
    xs = np.linspace(p0[0], p1[0], n)[on]
    ys = np.linspace(p0[1], p1[1], n)[on]
    vertical = abs(p1[1] - p0[1]) > abs(p1[0] - p0[0])
    for o in range(width):
        xi = (xs + (o if vertical else 0)).astype(int)
        yi = (ys + (0 if vertical else o)).astype(int)
        ok = (xi >= 0) & (xi < W) & (yi >= 0) & (yi < H)
        img[yi[ok], xi[ok]] = img[yi[ok], xi[ok]] * (1 - a) + np.array(col, float) * a


GROUP_COLS = [(124, 246, 200), (255, 198, 92), (255, 118, 150), (140, 200, 255), (255, 240, 130), (200, 150, 255)]


def tie_groups(t):
    """(g, alpha, class per pendulum) around a milestone with g <= 6 groups, else None"""
    g, a = T.tie_alpha(t)
    if not g:
        return None
    tm = min((tm for tm, gg in T.GROUPS if gg == g), key=lambda x: abs(x - t))
    return g, a, T.group_class(tm, g)


LINE_TIE = 0.9


def tie_lines(img, cam, P_bob, t):
    if T.T_LINE <= t < T.T_LINE + LINE_TIE:            # the line-up: one coral tie through all 100
        a = 1 - (t - T.T_LINE) / LINE_TIE
        scr = cam.wp(P_bob + [0, -S.BOB_R * 1.8, 0])
        for k0 in range(T.K - 1):
            line_add(img, scr[k0], scr[k0 + 1], (255, 118, 96), 0.9 * a, dash=(9, 0), width=2)
        return
    tg = tie_groups(t)
    if not tg:
        return
    g, a, cls = tg
    scr = cam.wp(P_bob + [0, -S.BOB_R * 1.8, 0])
    for c in range(g):
        ks = [k for k in range(T.K) if cls[k] == c]
        for k0, k1 in zip(ks, ks[1:]):
            line_add(img, scr[k0], scr[k1], GROUP_COLS[c], 0.85 * a, dash=(5, 2), width=2)


def tags(img, cam, P_bob, t):
    out = []
    a0, a1 = T.LEN_TAGS
    b0, b1 = T.SWING_TAGS
    for k, cm, sw in ((0, '34 CM', '51/MIN'), (T.K - 1, '4 CM', '150/MIN')):
        s = cam.wp(P_bob[k] + [0, -0.03, 0])
        for (c0, c1, txt, col) in ((a0, a1, cm, SKYB), (b0, b1, sw, AMBER)):
            if c0 <= t < c1:
                u = ss(c0, c0 + 0.25, t) * (1 - ss(c1 - 0.15, c1, t))
                if u > 0.05 and -20 < s[0] < W + 20 and 150 < s[1] < H - 20:
                    out.append((txt, s[0], s[1] + 5 + 6 * (1 - u), col, 2))
    gn = T.groups_now(t, 1.1)
    if gn and gn[1] in T.POP_GROUPS:                 # group-count pop-ups before the HUD exists
        g, tm, c = gn
        u = ss(tm - 1.1, tm - 0.7, t) * (1 - ss(tm + 0.75, tm + 1.1, t))
        out.append((f'{g} GROUPS', W / 2, 470 + 8 * (1 - u), MINT, 3))
    for txt, x, y, col, sc in out:
        half = (len(txt) * ADV * sc) / 2
        center_text(img, txt, float(np.clip(x, half + 6, W - half - 6)), y, col, sc)


def count_caption(img, t):
    """44.5-49 s: '8... 10... 15...' revealed as each group pattern lands"""
    a, parts, end = T.COUNT_CAP
    if not (a <= t < end):
        return
    line = ''.join(seg for tp_, seg in parts if t >= tp_).rstrip()
    slide = int(round(6 * (1 - ease((t - a) / 0.25))))
    fade = 1 - ss(end - 0.12, end, t)
    rich_line(img, line, W // 2, 42 + slide, 3, CREAM, AMBER, SKYB, alpha=fade)


def draw_hud(out, t):
    """phase ring: every pendulum is a dot going round once per swing; clumps = groups. Group counter."""
    if not (T.HUD_IN - 0.4 <= t < T.HUD_OUT + 0.4):
        return out
    off = slide_offset(t, T.HUD_IN, T.HUD_OUT, 180)
    y0 = 528 + off
    pil = Image.fromarray(out)
    dr = ImageDraw.Draw(pil)
    panel(dr, 14, y0, 346, y0 + 96)
    cx, cy, R = 66, y0 + 48, 36
    dr.ellipse([cx - R, cy - R, cx + R, cy + R], outline=(70, 84, 124))
    dr.ellipse([cx - 3, cy - R - 5, cx + 3, cy - R + 1], fill=(255, 236, 190))
    ph = T.phase(t)
    fl = T.flash(t)
    for k in range(T.K):
        a = ph[k]
        rr = R - 3 - 5 * (k / (T.K - 1))                 # long pendulums on the outer edge, short inside
        x, y = cx + rr * math.sin(a), cy - rr * math.cos(a)
        s = 1.4 + 1.2 * fl[k]
        c = tuple(int(v) for v in np.minimum(255, BOB_COL[k] * (0.85 + 0.3 * fl[k])))
        dr.ellipse([x - s, y - s, x + s, y + s], fill=c)
    tg = tie_groups(t)
    if tg:                                             # a marker on each clump, in its tie-line colour
        g, al, cls = tg
        for c in range(g):
            m = cls == c
            ang = math.atan2(np.sin(ph[m]).mean(), np.cos(ph[m]).mean())
            x, y = cx + (R + 5) * math.sin(ang), cy - (R + 5) * math.cos(ang)
            col = tuple(int(v * al + 16 * (1 - al)) for v in GROUP_COLS[c])
            dr.ellipse([x - 3, y - 3, x + 3, y + 3], fill=col, outline=(12, 16, 30))
    out = np.asarray(pil).copy()
    center_text(out, 'GROUPS', 236, y0 + 12, SKYB, 2)
    if t >= T.RACE_FROM:
        g = T.race_count(t)
        col = CORAL if g == 1 else AMBER
        center_text(out, str(g), 236, y0 + 34, col, 4)
    else:
        gn = T.groups_now(t, 1.0)
        if gn:
            center_text(out, str(gn[0]), 236, y0 + 34, AMBER, 4)
        else:
            center_text(out, '?', 236, y0 + 34, (150, 160, 190), 4)
    left = T.T_LINE - t
    if left > 0:
        center_text(out, f'1 LINE IN {left:4.1f} S', 236, y0 + 74, MINT, 1)
    else:
        center_text(out, f'LOOP IN {60.0 - t:4.1f} S', 236, y0 + 74, MINT, 1)
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
    L, ks = make_lights(t, cam)
    fr = Frame(sky_gradient(cam, (40, 30, 70), (128, 76, 104), step=7.0))
    fr.bob_mask = bob_shadow_mask(P_bob, th)
    fr.bob_lm = bob_lightmap(P_bob, fl)
    cut_edges(cam, fr)
    sun_scr = shell_pass(fr, cam, t, L, ks, P_bob, fl)
    glows = []
    items = [(np.linalg.norm(o.ctr - cam.pos), 'obj', o) for o in S.OBJS]
    for k in range(T.K):
        mid = np.array([S.RIG_X, 0.5 * (P_bob[k][1] + S.TOP), S.ZK[k]])
        items.append((np.linalg.norm(mid - cam.pos), 'slot', k))
    for i, (c, r, col, per) in enumerate(S.WALL_CLOCKS):
        items.append((np.linalg.norm(c - cam.pos) - 0.05, 'wclock', i))
    items.append((np.linalg.norm(S.GC_DIAL - cam.pos) - 0.05, 'gcdial', None))
    items.append((np.linalg.norm(S.GC_PIVOT + [0, -0.4, 0.03] - cam.pos) - 0.06, 'gcpend', None))
    items.append((np.linalg.norm(S.CAT_P + [0, 0.15, 0] - cam.pos), 'cat', None))
    items.append((np.linalg.norm(S.LAMP2 + [0, 0.3, 0] - cam.pos), 'chain', S.CHAIN))
    items.append((np.linalg.norm(S.LAMP4 + [0, 0.3, 0] - cam.pos), 'chain', S.CHAIN4))
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
            draw_clock_hands(fr, cam, t, c, r * 0.84 if not per else r * 0.85, zd, [1.1, 4.2, 2.6, 5.9][o])
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
            faces, eyes, blink = S.cat_faces(t, P_bob[S.CAT_LOOK][0] - S.RIG_X)
            for name, fcs in faces:
                draw_faces(fr, cam, L, fcs, (226, 140, 72) if name != 'tail' else (210, 124, 62), -11)
            for e in eyes:
                zc = cam.depth(e)[0]
                if zc > 0.1 and np.dot(cam.pos - e, nrm(e - (S.CAT_P + [0, 0.28, 0]))) > 0:
                    s = cam.wp(e)
                    if blink:
                        fr.draw.line([s[0] - 1, s[1], s[0] + 1, s[1]], fill=(60, 36, 26))
                    else:
                        fr.draw.rectangle([s[0] - 0.6, s[1] - 0.6, s[0] + 0.6, s[1] + 0.6], fill=(120, 214, 110))
        elif kind == 'chain':
            fr.line(cam, list(o), (70, 64, 58), 1)
    for i, (lp, R, k) in enumerate(S.LAMPS):                  # lamp flames + glows
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
    draw_trails(img, cam, depth, t)
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
