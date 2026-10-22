"""Chaos Makes a Perfect Curve - renderer.

    python -m reelkit.sheet render.py test/sheet.png 0.5 5 9 14 18 23 29 36 43 48 55 59.5
    python -m reelkit.video render.py audio.wav galton_board.mp4

Per frame: ray-traced hall shell (parquet floor + stage with sun patches and the machine's
shadow, back wall with chalkboard and the machine's long shadow, left wall with two arched
windows and a painted sunset-side sky, right wall, coffered ceiling) -> objects behind the
machine -> ray-traced maple backplate (static peg/divider/post shadows + every ball's moving
shadow) -> the machine (pegs that flash when struck, balls, dividers, posts, base, hopper with
its pile, elevator) -> benches, pawn spectators, lamps -> locked depth of field (focus on the
board) -> sunbeams + motes -> glows -> bloom -> grade -> vignette/scrim -> overlays (Pascal
numbers, L/R arrows, bin counts, the ideal curve, counter plate) -> captions."""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, nrm, mixc
from reelkit.camera import Cam, interp_keys
from reelkit.raster import Frame, painter, apply_glows
from reelkit.lighting import Lights, shade, sky_gradient
from reelkit.shadows import MaskGrid, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, slide_offset, MINT, CREAM, AMBER, CORAL, SKYB, INK
from reelkit import font as RF
from reelkit.font import text_mask, stamp, rich_line, ADV
from reelkit.geometry import Obj, face, icosa
import timeline as T
import board as B
import scene as S
import motion as M

for ch, rows in {'→': ".....|..#..|...#.|#####|...#.|..#..|.....",
                 '←': ".....|..#..|.#...|#####|.#...|..#..|.....",
                 '·': ".....|.....|.....|..#..|.....|.....|....."}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

SUN = S.SUN
WARM = np.array([1.0, 0.66, 0.34])
SKYFILL = (nrm([-1.0, 0.35, 0.3]), np.array([0.08, 0.1, 0.14]))       # cool light from the windows
BOUNCE = (np.array([-0.4, 0.6, 0.0]), 2.4, np.array([0.16, 0.12, 0.08]))  # warm bounce off the sunlit stage


# ---------------------------------------------------------------- shadow masks (static casters)
def proj_plane(Q, axis, val, d=SUN):
    """project points along the sun direction onto the plane coord[axis] = val -> the other two coords"""
    s = (Q[:, axis] - val) / d[axis]
    P = Q - s[:, None] * d[None]
    keep = [a for a in range(3) if a != axis]
    return P[:, keep]


def plane_mask(grid, faces, axis, val, blur, d=SUN):
    polys = []
    for Q in faces:
        if axis == 1 and Q[:, 1].max() <= val + 0.01:
            continue
        polys.append(proj_plane(Q, axis, val, d))
    return grid.raster(polys, blur)


BACKPLATE_BOX = Obj(S.box(0, 0.5 * (B.Y_BF + B.Y_TOP), B.ZB - 0.02, 0.72, 0.5 * (B.Y_TOP - B.Y_BF), 0.02), S.MAPLE, 'backplate', grp=S.MG)
MACH_CAST = [o for o in S.MACH if o.cast] + [BACKPLATE_BOX, Obj(S.HOP_BACK, S.WALNUT_D, 'hopback', grp=S.MG)]
BACK_CAST = [o for o in S.BACK if o.cast]
FRONT_CAST = [o for o in S.FRONT if o.cast]
PAWN_FACES0 = []
for pw in S.PAWNS:
    b, h, ht = S.pawn_faces(pw, 0.0)
    PAWN_FACES0 += [Q for Q, n in b + h + ht]

STAGE_GRID = MaskGrid((S.SX0, S.SX1, S.HZ0, S.SZ1, 0.01))
STAGE_MASK = plane_mask(STAGE_GRID, [Q for o in MACH_CAST + BACK_CAST + FRONT_CAST if o.name not in ('bench',) for Q, n in o.faces], 1, S.YS, 1.5)
FLOOR_GRID = MaskGrid((S.HX0, S.HX1, S.HZ0, S.HZ1, 0.015))
FLOOR_MASK = plane_mask(FLOOR_GRID, [Q for o in FRONT_CAST for Q, n in o.faces] + PAWN_FACES0, 1, 0.0, 1.5)
WALL_GRID = MaskGrid((S.HX0, S.HX1, 0.0, S.HY, 0.012))       # back wall, coords (x, y)
WALL_MASK = plane_mask(WALL_GRID, [Q for o in MACH_CAST + BACK_CAST + FRONT_CAST if o.name not in ('gallery',) for Q, n in o.faces], 2, S.HZ0 + 0.001, 1.5)
GAL_MASK = plane_mask(WALL_GRID, [Q for o in S.BACK if o.name == 'gallery' for Q, n in o.faces], 2, S.HZ0 + 0.001, 1.0)
WALL_MASK = np.maximum(WALL_MASK, GAL_MASK)
# backplate: pegs, dividers, posts, hopper rim and the bin floor shadow onto the maple
BP_GRID = MaskGrid((B.X0 - 0.06, B.X1 + 0.06, B.Y_BF - 0.02, B.Y_TOP + 0.05, 0.0028))       # coords (x, y)
BP_STATIC = plane_mask(BP_GRID, [Q for f in S.PEG_FACES for Q, n in f] + [Q for d in S.DIVIDERS for Q, n in d] +
                       [Q for o in S.POSTS for Q, n in o.faces] + [Q for Q, n in S.HOP_RIM], 2, B.ZB, 0.8)
OCC = Occluders(S.MACH + S.BACK + S.FRONT + [BACKPLATE_BOX])
MACH_OCC = Occluders([Obj(S.box(0.1, 2.4, B.ZB, 0.98, 2.0, 0.17), S.WALNUT, grp=S.MG)])


def flicker(t, i):
    w = 2 * math.pi / 60.0
    return 0.93 + 0.04 * math.sin(w * (97 + 13 * i) * t + i) * math.sin(w * (131 + 7 * i) * t) + 0.03 * math.sin(w * (233 + 11 * i) * t + 2 * i)


def make_lights(t, cam):
    L = Lights(amb=(0.4, 0.38, 0.4), sky=(0.06, 0.06, 0.07))
    L.view = cam.view
    L.key('sun', SUN, S.SUN_RGB * 1.1)
    L.fill(*SKYFILL)
    ks = []
    for i, (lp, R, k) in enumerate(S.LAMPS):
        kk = k * flicker(t, i)
        ks.append(kk)
        L.point(lp, WARM * kk * 0.85, R)
    L.point(BOUNCE[0], BOUNCE[2], BOUNCE[1])
    return L, ks


# ---------------------------------------------------------------- camera
BOARD_C = np.array([0.1, 2.2, B.ZB])
HOOK = (0.0, np.array([0.04, 2.7, B.ZB]), 3.05, 9, 4, 44, 410)
KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    HOOK,
    (4.2, np.array([0.04, 2.76, B.ZB]), 2.8, 6, 4, 44, 410),
    (4.9, np.array([0.02, 2.8, B.ZB]), 2.75, 5, 4, 44, 405),               # freeze + rewind, the hero sets off
    (6.5, np.array([-1.35, 3.0, 1.1]), 9.6, 22, 9, 52, 352),                # establishing: window, sunbeam, hall
    (7.5, np.array([-1.0, 2.95, 0.8]), 8.6, 20, 8, 52, 356),
    (8.9, np.array([0.02, 2.93, B.ZB]), 2.45, 6, 3, 40, 372),               # second ball: upper peg field
    (11.6, np.array([0.02, 2.88, B.ZB]), 2.4, -4, 3, 40, 372),
    (13.0, np.array([0.0, 2.64, B.ZB]), 3.2, 0, 2, 40, 392),                # Pascal: whole peg field
    (15.6, np.array([0.0, 2.56, B.ZB]), 3.4, 0, 2, 40, 398),
    (17.4, np.array([0.04, 2.15, B.ZB]), 4.6, -9, 4, 44, 388),              # stream: board + bins
    (20.5, np.array([-0.06, 2.95, B.ZB]), 2.1, -10, 3, 40, 380),            # golden ball
    (22.6, np.array([-0.3, 2.5, B.ZB]), 2.25, -15, 4, 40, 380),
    (23.65, np.array([-0.44, 2.28, B.ZB]), 2.3, -17, 5, 40, 380),
    (24.55, np.array([-0.52, 1.12, B.ZB]), 2.3, -18, 6, 40, 380),            # follow the fall into the edge bin
    (25.7, np.array([-0.46, 1.16, B.ZB]), 2.45, -16, 6, 40, 380),
    (27.2, np.array([0.08, 2.2, B.ZB]), 4.9, 14, 5, 44, 386),               # 100 balls: equaliser
    (32.0, np.array([0.08, 2.2, B.ZB]), 4.7, 6, 6, 44, 386),
    (39.6, np.array([0.06, 2.15, B.ZB]), 4.5, -12, 9, 44, 384),             # 300 balls
    (45.6, np.array([0.06, 2.0, B.ZB]), 4.4, -2, 6, 44, 384),               # flood
    (47.0, np.array([0.0, 1.62, B.ZB]), 3.4, 0, 4, 42, 384),                # the curve
    (49.4, np.array([0.02, 1.62, B.ZB]), 3.3, 5, 5, 42, 384),
    (51.8, np.array([0.05, 2.0, B.ZB]), 6.4, -14, 6, 46, 396),             # pull back: the hall, gallery spectators
    (54.6, np.array([0.18, 2.3, B.ZB]), 6.4, -6, 8, 48, 380),              # drain + elevator
    (57.3, np.array([0.12, 2.55, B.ZB]), 4.4, 8, 5, 44, 400),
    (60.0,) + HOOK[1:],
]


def camera(t):
    return interp_keys(KEYS, t % T.DUR)


# ---------------------------------------------------------------- procedural textures
def hash2(a, b):
    v = np.sin(a * 127.1 + b * 311.7) * 43758.5453
    return v - np.floor(v)


def floor_albedo(X, Z):
    """oak parquet: planks along z, staggered"""
    pw = 0.14
    ci = np.floor(X / pw)
    off = hash2(ci, 3.0) * 1.2
    row = np.floor((Z + off) / 0.9)
    h = hash2(ci, row)
    base = np.array([168, 112, 70], float)[None] * (0.82 + 0.26 * h)[:, None]
    base *= (0.95 + 0.06 * np.sin(Z * 19 + ci * 2.1))[:, None]
    u = (X / pw) % 1.0
    v = ((Z + off) / 0.9) % 1.0
    base[(np.abs(u - 0.5) > 0.46) | (np.abs(v - 0.5) > 0.493)] *= 0.68
    return base


def stage_albedo(X, Z):
    pw = 0.12
    ci = np.floor(Z / pw)
    off = hash2(ci, 9.0) * 1.4
    row = np.floor((X + off) / 1.1)
    h = hash2(ci, row)
    base = np.array([124, 78, 52], float)[None] * (0.85 + 0.22 * h)[:, None]
    u = (Z / pw) % 1.0
    v = ((X + off) / 1.1) % 1.0
    base[(np.abs(u - 0.5) > 0.46) | (np.abs(v - 0.5) > 0.494)] *= 0.66
    return base


def make_chalkboard():
    """chalkboard texture: a bell curve, Pascal's first rows, 'GALTON 1889'"""
    w, h = 300, 260
    im = np.zeros((h, w, 3), np.float32)
    im[:] = (40, 58, 50)
    nz = np.random.default_rng(3).uniform(-6, 6, (h, w, 1))
    im += nz
    def chalk(x, y, a=1.0):
        x, y = int(x), int(y)
        if 0 <= x < w and 0 <= y < h:
            im[y, x] = im[y, x] * (1 - 0.8 * a) + np.array([226, 230, 220]) * 0.8 * a
    for xi in range(20, 280):
        u = (xi - 150) / 42.0
        y = 210 - 120 * math.exp(-u * u / 2)
        chalk(xi, y); chalk(xi, y + 1, 0.6)
    for xi in range(16, 286):
        chalk(xi, 212, 0.7)
    for k in range(13):
        x = 22 + k * 21.5
        hgt = int(118 * math.comb(12, k) / 924)
        for yy in range(212 - hgt, 212, 6):
            chalk(x, yy, 0.5)
    rowsx = ["1", "1 1", "1 2 1", "1 3 3 1", "1 4 6 4 1"]
    for r, s in enumerate(rowsx):
        m = text_mask(s, 1)
        y0, x0 = 18 + r * 11, 70 - m.shape[1] // 2
        ys_, xs_ = np.nonzero(m)
        for a, b in zip(ys_, xs_):
            chalk(x0 + b, y0 + a, 0.85)
    m = text_mask("GALTON, 1889", 1)
    ys_, xs_ = np.nonzero(m)
    for a, b in zip(ys_, xs_):
        chalk(200 + b, 24 + a, 0.85)
    return im


CHALK = make_chalkboard()
CB = (-2.55, -1.15, 1.35, 2.55)          # chalkboard on the back wall (x0, x1, y0, y1)


def back_albedo(X, Y):
    col = np.zeros((len(X), 3))
    n = hash2(np.floor(X * 7), np.floor(Y * 7))
    col[:] = np.array([214, 200, 172.0])[None] * (0.95 + 0.05 * n)[:, None]           # plaster
    wain = Y < S.WAIN
    pan = ((X * 1.0) % 0.7)
    col[wain] = np.array([58, 92, 74.0])
    frame = wain & ((np.abs(pan - 0.35) > 0.29) | (np.abs(Y - S.WAIN / 2) > S.WAIN / 2 - 0.09))
    col[frame] = [46, 74, 60]
    col[np.abs(Y - S.WAIN) < 0.035] = [120, 82, 52]                                     # dado rail
    col[(Y > S.GAL_Y + 0.6) & (Y < S.GAL_Y + 0.66)] = [176, 156, 120]                 # picture rail
    col[Y > S.HY - 0.2] = [190, 176, 150]                                               # cornice
    cb = (X > CB[0]) & (X < CB[1]) & (Y > CB[2]) & (Y < CB[3])
    if cb.any():
        u = ((X[cb] - CB[0]) / (CB[1] - CB[0]) * (CHALK.shape[1] - 1)).astype(int)
        v = ((CB[3] - Y[cb]) / (CB[3] - CB[2]) * (CHALK.shape[0] - 1)).astype(int)
        col[cb] = CHALK[v, u]
    rim = (X > CB[0] - 0.06) & (X < CB[1] + 0.06) & (Y > CB[2] - 0.06) & (Y < CB[3] + 0.06) & ~cb
    col[rim] = [104, 70, 44]
    return col


def side_albedo(Z, Y, right=False):
    col = np.zeros((len(Z), 3))
    n = hash2(np.floor(Z * 7), np.floor(Y * 7) + (5 if right else 0))
    col[:] = np.array([212, 198, 170.0])[None] * (0.95 + 0.05 * n)[:, None]
    wain = Y < S.WAIN
    col[wain] = [58, 92, 74]
    pan = (Z % 0.7)
    col[wain & ((np.abs(pan - 0.35) > 0.29) | (np.abs(Y - S.WAIN / 2) > S.WAIN / 2 - 0.09))] = [46, 74, 60]
    col[np.abs(Y - S.WAIN) < 0.035] = [120, 82, 52]
    col[Y > S.HY - 0.2] = [190, 176, 150]
    if right:
        door = (np.abs(Z - 0.3) < 0.55) & (Y < 2.6)
        col[door] = np.array([96, 58, 38.0])
        col[door & ((np.abs(np.abs(Z - 0.3) - 0.27) < 0.02) | (np.abs(Y - 1.3) < 0.02))] = [70, 42, 28]
        col[(np.abs(Z - 0.3) < 0.66) & (np.abs(Y - 2.7) < 0.08)] = [120, 82, 52]
        for zc, yc, c in ((2.2, 2.8, (130, 60, 50)), (3.6, 2.9, (60, 80, 120))):
            fr_ = (np.abs(Z - zc) < 0.42) & (np.abs(Y - yc) < 0.55)
            col[fr_] = [176, 136, 60]
            inner = (np.abs(Z - zc) < 0.35) & (np.abs(Y - yc) < 0.47)
            col[inner] = np.array(c, float) * (0.8 + 0.3 * hash2(np.floor(Z * 20), np.floor(Y * 20)))[inner, None]
    return col


def ceil_albedo(X, Z):
    col = np.zeros((len(X), 3))
    col[:] = [226, 214, 190]
    u, v = (X / 0.93) % 1.0, (Z / 0.85) % 1.0
    beam = (np.abs(u - 0.5) > 0.42) | (np.abs(v - 0.5) > 0.42)
    inner = (np.abs(u - 0.5) < 0.3) & (np.abs(v - 0.5) < 0.3)
    col[~beam & ~inner] = [204, 190, 164]
    col[beam] = [110, 76, 52]
    rose = (np.abs(u - 0.5) ** 2 + np.abs(v - 0.5) ** 2) < 0.006
    col[rose] = [200, 160, 90]
    return col


def backdrop(Zb, Yb, t, sun_zy):
    """the sky outside the windows (coords on a far plane), sun pinned in the first window"""
    sz, sy = sun_zy
    k = np.clip((Yb - sy + 1.5) / 6.0, 0, 1)
    c0, c1, c2 = np.array([255, 214, 150.0]), np.array([150, 196, 236.0]), np.array([92, 140, 214.0])
    col = np.where((k < 0.35)[:, None], c0 * (1 - k[:, None] / 0.35) + c1 * (k[:, None] / 0.35),
                   c1 * (1 - (k[:, None] - 0.35) / 0.65) + c2 * ((k[:, None] - 0.35) / 0.65))
    d = np.sqrt((Zb - sz) ** 2 + (Yb - sy) ** 2)
    col += (np.clip(1 - d / 2.6, 0, 1) ** 2)[:, None] * np.array([90, 60, 10.0])[None]
    col[d < 0.32] = [255, 246, 214]
    ring = (d >= 0.32) & (d < 0.42)
    col[ring] = col[ring] * 0.4 + np.array([255, 226, 150.0]) * 0.6
    for cy_, amp, sp, wd in ((1.2, 0.12, 0.05, 0.08), (2.3, 0.15, -0.04, 0.07), (3.4, 0.1, 0.03, 0.06)):
        cl = sy + cy_ + amp * np.sin((Zb - sz) * 1.3 + 2 * math.pi * t / 60.0 + cy_)
        band = np.abs(Yb - cl) < wd * (1 + 0.7 * np.sin((Zb - sz) * 2.7 + cy_ * 5))
        col[band] = col[band] * 0.4 + np.array([255, 240, 226.0]) * 0.6
    rz = Zb - sz
    hroof = sy - 1.3 + 0.35 * np.sin(rz * 1.5 + 0.6) + 0.25 * (np.floor(rz * 1.1) % 2) + 0.12 * hash2(np.floor(rz * 2.3), 1.0)
    chim = (np.abs(((rz * 1.6) % 1.0) - 0.5) < 0.06) & (hash2(np.floor(rz * 1.6), 3.0) > 0.45)
    hroof = np.where(chim, hroof + 0.3, hroof)
    tw = np.abs(rz - 2.2) < 0.22
    spire = np.abs(rz - 2.2) < 0.22 * np.clip((sy + 1.6 - Yb) / 1.1, 0, 1)
    roof = (Yb < hroof) | (tw & (Yb < sy + 0.5)) | (spire & (Yb < sy + 1.6))
    col[roof] = [132, 112, 132]
    tree = (Yb < sy - 1.0 + 0.35 * np.abs(np.sin(rz * 3.1)) + 0.2 * hash2(np.floor(rz * 6), 2.0)) & (rz < -0.8)
    col[tree] = [96, 120, 92]
    for i, (v, y0, ph) in enumerate(((3, 1.6, 0.0), (4, 2.4, 0.4))):
        u = (v * t / 60.0 + ph) % 1.0
        bz = sz - 3.5 + 7.0 * u
        by = sy + y0 + 0.08 * math.sin(2 * math.pi * 9 * t / 60.0 + i)
        flap = 0.05 * math.sin(2 * math.pi * 70 * t / 60.0 + i)
        dz = np.abs(Zb - bz)
        bird = (dz < 0.13) & (np.abs((Yb - by) - dz * 0.5 + flap * (dz / 0.13)) < 0.025)
        col[bird] = [70, 60, 80]
    return col


# ---------------------------------------------------------------- the hall shell, per pixel
def shell_pass(fr, cam, t, L, ks):
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    INF = 1e9
    hits = []
    with np.errstate(divide='ignore', invalid='ignore'):
        def plane(axis, val, sign):
            den = dw[..., axis]
            tt = np.where(den * sign < -1e-7, (val - p[axis]) / den, INF)
            return np.where(tt > 0, tt, INF)
        tf = plane(1, 0.0, 1)
        Xf, Zf = p[0] + tf * dw[..., 0], p[2] + tf * dw[..., 2]
        okf = (Xf >= S.HX0) & (Xf <= S.HX1) & (Zf >= S.HZ0) & (Zf <= S.HZ1) & ~((Xf > S.SX0) & (Xf < S.SX1) & (Zf < S.SZ1))
        ts = plane(1, S.YS, 1)
        Xs, Zs = p[0] + ts * dw[..., 0], p[2] + ts * dw[..., 2]
        oks = (Xs >= S.SX0) & (Xs <= S.SX1) & (Zs >= S.HZ0) & (Zs <= S.SZ1)
        tsf = plane(2, S.SZ1, 1)
        Xsf, Ysf = p[0] + tsf * dw[..., 0], p[1] + tsf * dw[..., 1]
        oksf = (Xsf >= S.SX0) & (Xsf <= S.SX1) & (Ysf >= 0) & (Ysf <= S.YS)
        tsl = plane(0, S.SX0, -1)                       # stage left side, facing -x (seen from the left)
        Ysl, Zsl = p[1] + tsl * dw[..., 1], p[2] + tsl * dw[..., 2]
        oksl = (Ysl >= 0) & (Ysl <= S.YS) & (Zsl >= S.HZ0) & (Zsl <= S.SZ1)
        tsr = plane(0, S.SX1, 1)                        # stage right side, facing +x
        Ysr, Zsr = p[1] + tsr * dw[..., 1], p[2] + tsr * dw[..., 2]
        oksr = (Ysr >= 0) & (Ysr <= S.YS) & (Zsr >= S.HZ0) & (Zsr <= S.SZ1)
        tb = plane(2, S.HZ0, 1)
        Xb, Yb = p[0] + tb * dw[..., 0], p[1] + tb * dw[..., 1]
        okb = (Xb >= S.HX0) & (Xb <= S.HX1) & (Yb >= 0) & (Yb <= S.HY)
        tl = plane(0, S.HX0, 1)
        Yl, Zl = p[1] + tl * dw[..., 1], p[2] + tl * dw[..., 2]
        okl = (Yl >= 0) & (Yl <= S.HY) & (Zl >= S.HZ0) & (Zl <= S.HZ1)
        tr = plane(0, S.HX1, -1)
        Yr, Zr = p[1] + tr * dw[..., 1], p[2] + tr * dw[..., 2]
        okr = (Yr >= 0) & (Yr <= S.HY) & (Zr >= S.HZ0) & (Zr <= S.HZ1)
        tc = plane(1, S.HY, -1)
        Xc, Zc = p[0] + tc * dw[..., 0], p[2] + tc * dw[..., 2]
        okc = (Xc >= S.HX0) & (Xc <= S.HX1) & (Zc >= S.HZ0) & (Zc <= S.HZ1)
    TT = [np.where(m, tt, INF) for m, tt in ((okf, tf), (oks, ts), (oksf, tsf), (oksl, tsl), (oksr, tsr), (okb, tb), (okl, tl), (okr, tr), (okc, tc))]
    tmin = np.minimum.reduce(TT)
    which = np.argmin(np.stack(TT), 0)
    which[tmin >= INF] = -1

    def lamps_at(P, nv, k_=1.0):
        I = np.zeros((len(P), 3))
        for i, (lp, R, k) in enumerate(S.LAMPS):
            v = lp[None] - P
            dist = np.linalg.norm(v, axis=1) + 1e-6
            cosv = (v * nv[None]).sum(1) / dist
            lam = np.clip(cosv, 0, None) * 0.85 + 0.15 * (cosv > -0.05)
            I += (lam * ks[i] / (1 + (dist / R) ** 2))[:, None] * WARM[None]
        bp, bR, bc = BOUNCE
        v = bp[None] - P
        dist = np.linalg.norm(v, axis=1) + 1e-6
        I += ((np.clip((v * nv[None]).sum(1) / dist, 0, None) * 0.7 + 0.3) / (1 + (dist / bR) ** 2))[:, None] * bc[None]
        return I * k_

    def put(ys, xs, col, d):
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = d

    amb = L.amb + L.sky * 0
    sunc = L.keys[0][2]
    fill_d, fill_c = SKYFILL
    # floor
    ys, xs = np.nonzero(which == 0)
    if len(xs):
        X, Z = Xf[ys, xs], Zf[ys, xs]
        P = np.stack([X, np.zeros_like(X), Z], 1)
        I = np.tile(amb + L.sky, (len(X), 1)) + lamps_at(P, np.array([0, 1.0, 0]))
        sl = S.win_lit(P + [0, 0.002, 0]) * (1 - FLOOR_GRID.sample(FLOOR_MASK, X, Z))
        I += (sl * SUN[1] * 1.6)[:, None] * sunc[None]
        I *= contact_ao(X, Z, S.FEET, 0.25)[:, None]
        I *= (1 - 0.35 * np.exp(-np.maximum(Z - S.SZ1, 0) / 0.18) * ((X > S.SX0) & (X < S.SX1)))[:, None]
        I *= (1 - 0.3 * np.exp(-(X - S.HX0) / 0.3) - 0.3 * np.exp(-(S.HX1 - X) / 0.3))[:, None]
        put(ys, xs, floor_albedo(X, Z) * I, tf[ys, xs])
    # stage top
    ys, xs = np.nonzero(which == 1)
    if len(xs):
        X, Z = Xs[ys, xs], Zs[ys, xs]
        P = np.stack([X, np.full_like(X, S.YS), Z], 1)
        I = np.tile(amb + L.sky, (len(X), 1)) + lamps_at(P, np.array([0, 1.0, 0]))
        sl = S.win_lit(P + [0, 0.002, 0]) * (1 - STAGE_GRID.sample(STAGE_MASK, X, Z))
        I += (sl * SUN[1] * 1.6)[:, None] * sunc[None]
        I *= contact_ao(X, Z, S.SFEET, 0.2)[:, None]
        I *= (1 - 0.3 * np.exp(-(Z - S.HZ0) / 0.25))[:, None]
        put(ys, xs, stage_albedo(X, Z) * I, ts[ys, xs])
    # stage front + sides (walnut panels, brass nosing)
    for code, tt_, A_, Bc_, nv in ((2, tsf, Xsf, Ysf, np.array([0, 0, 1.0])), (3, tsl, Zsl, Ysl, np.array([-1.0, 0, 0])), (4, tsr, Zsr, Ysr, np.array([1.0, 0, 0]))):
        ys, xs = np.nonzero(which == code)
        if not len(xs):
            continue
        a, yv = A_[ys, xs], Bc_[ys, xs]
        if code == 2:
            P = np.stack([a, yv, np.full_like(a, S.SZ1)], 1)
        else:
            P = np.stack([np.full_like(a, S.SX0 if code == 3 else S.SX1), yv, a], 1)
        alb = np.tile(np.array([96, 58, 38.0]), (len(a), 1))
        pan = (a % 0.6)
        alb[(np.abs(pan - 0.3) > 0.25) | (np.abs(yv - 0.2) > 0.13)] = [78, 46, 30]
        alb[yv > S.YS - 0.035] = [214, 166, 80]
        I = np.tile(amb, (len(a), 1)) + lamps_at(P, nv) + max(0.0, float(nv @ fill_d)) * fill_c
        sl = S.win_lit(P + nv * 0.003) * max(0.0, float(nv @ SUN))
        I += (sl * 1.2)[:, None] * sunc[None]
        put(ys, xs, alb * I, tt_[ys, xs])
    # back wall
    ys, xs = np.nonzero(which == 5)
    if len(xs):
        X, Y = Xb[ys, xs], Yb[ys, xs]
        P = np.stack([X, Y, np.full_like(X, S.HZ0)], 1)
        I = np.tile(amb * 0.95, (len(X), 1)) + lamps_at(P, np.array([0, 0, 1.0]))
        sl = S.win_lit(P + [0, 0, 0.003]) * (1 - WALL_GRID.sample(WALL_MASK, X, Y))
        I += (sl * SUN[2] * 1.5)[:, None] * sunc[None]
        I += max(0.0, float(np.array([0, 0, 1.0]) @ fill_d)) * fill_c
        ao = (1 - 0.3 * np.exp(-np.abs(Y - S.YS * ((X > S.SX0) & (X < S.SX1))) / 0.2)) * (1 - 0.25 * np.exp(-(X - S.HX0) / 0.3)) * (1 - 0.25 * np.exp(-(S.HX1 - X) / 0.3))
        ao *= 1 - 0.35 * np.exp(-np.maximum(S.GAL_Y - 0.12 - Y, 0) / 0.15) * (Y < S.GAL_Y)
        put(ys, xs, back_albedo(X, Y) * I * ao[:, None], tb[ys, xs])
    # left wall + windows
    ys, xs = np.nonzero(which == 6)
    sun_scr = None
    if len(xs):
        Z, Y = Zl[ys, xs], Yl[ys, xs]
        wo = S.win_open(Z, Y)
        d2 = dw[ys, xs]
        trv = (S.HX0 - S.WT - p[0]) / d2[:, 0]
        reveal = wo & ~S.win_open(p[2] + trv * d2[:, 2], p[1] + trv * d2[:, 1])
        bars = wo & S.win_bars(Z, Y)
        glass = wo & ~reveal & ~bars
        P = np.stack([np.full_like(Z, S.HX0), Y, Z], 1)
        I = np.tile(amb * 0.92, (len(Z), 1)) + lamps_at(P, np.array([1.0, 0, 0]))
        col = side_albedo(Z, Y) * I
        ao = (1 - 0.25 * np.exp(-(Z - S.HZ0) / 0.3)) * (1 - 0.3 * np.exp(-Y / 0.2))
        col *= ao[:, None]
        rc = np.array([220, 204, 176.0])[None] * (I * 0.7 + np.array([0.55, 0.45, 0.3]))
        col[reveal] = rc[reveal]
        col[bars] = (np.array([70, 52, 40.0])[None] * (I * 0.9))[bars]
        if glass.any():
            BDX = S.HX0 - 7.0
            tbd = (BDX - p[0]) / d2[glass, 0]
            Zbd, Ybd = p[2] + tbd * d2[glass, 2], p[1] + tbd * d2[glass, 1]
            pin = np.array([S.HX0, 3.2, 2.25])
            dirs = pin - p
            s_ = (BDX - p[0]) / dirs[0]
            sun_zy = (p[2] + s_ * dirs[2], p[1] + s_ * dirs[1])
            col[glass] = backdrop(Zbd, Ybd, t, sun_zy)
            sun_scr = cam.wp(np.array([BDX, sun_zy[1], sun_zy[0]]))
        put(ys, xs, col, np.where(glass, 5e5, tl[ys, xs]))
    # right wall
    ys, xs = np.nonzero(which == 7)
    if len(xs):
        Z, Y = Zr[ys, xs], Yr[ys, xs]
        P = np.stack([np.full_like(Z, S.HX1), Y, Z], 1)
        I = np.tile(amb * 0.95, (len(Z), 1)) + lamps_at(P, np.array([-1.0, 0, 0]))
        sl = S.win_lit(P - [0.003, 0, 0])
        I += (sl * (-SUN[0]) * 1.3)[:, None] * sunc[None]
        ao = (1 - 0.25 * np.exp(-(Z - S.HZ0) / 0.3)) * (1 - 0.3 * np.exp(-Y / 0.2))
        put(ys, xs, side_albedo(Z, Y, True) * I * ao[:, None], tr[ys, xs])
    # ceiling
    ys, xs = np.nonzero(which == 8)
    if len(xs):
        X, Z = Xc[ys, xs], Zc[ys, xs]
        P = np.stack([X, np.full_like(X, S.HY), Z], 1)
        I = np.tile(amb * 0.85, (len(X), 1)) + lamps_at(P, np.array([0, -1.0, 0]), 1.3)
        put(ys, xs, ceil_albedo(X, Z) * I, tc[ys, xs])
    fr.set_array(arr, depth)
    return sun_scr


def cut_edges(cam, fr):
    """the diorama's sawn front edges (only seen in the widest shot)"""
    z = S.HZ1
    for Q, c in (([[S.HX0 - S.WT, -S.SLAB, z], [S.HX1 + S.WT, -S.SLAB, z], [S.HX1 + S.WT, 0, z], [S.HX0 - S.WT, 0, z]], (86, 58, 40)),
                 ([[S.HX0 - S.WT, 0, z], [S.HX0, 0, z], [S.HX0, S.HY, z], [S.HX0 - S.WT, S.HY, z]], (170, 150, 124)),
                 ([[S.HX1, 0, z], [S.HX1 + S.WT, 0, z], [S.HX1 + S.WT, S.HY, z], [S.HX1, S.HY, z]], (170, 150, 124)),
                 ([[S.HX0 - S.WT, S.HY, z], [S.HX1 + S.WT, S.HY, z], [S.HX1 + S.WT, S.HY + S.SLAB, z], [S.HX0 - S.WT, S.HY + S.SLAB, z]], (150, 128, 104))):
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


def draw_obj(fr, cam, L, o, emit=None):
    if emit is None:
        emit = 0.9 if o.name == 'chimney' else 0.0
    draw_faces(fr, cam, L, o.faces, o.color, o.grp, emit=emit, tone=o.tone, double=o.double)


def on_screen(cam, ctr, rad):
    zc = cam.depth(ctr)[0]
    if zc < -rad:
        return False
    if zc < 0.3:
        return True
    s = cam.wp(ctr)
    r = rad * cam.f / zc
    return -r < s[0] < W + r and -r < s[1] < H + r


def draw_pawn(fr, cam, L, pw, t):
    body, head, hat = S.pawn_faces(pw, t)
    g = 5000 + id(pw) % 997
    draw_faces(fr, cam, L, body, pw['col'], g)
    draw_faces(fr, cam, L, head, S.SKIN, g)
    if hat:
        draw_faces(fr, cam, L, hat, (36, 34, 40) if pw['hat'] == 1 else (120, 80, 60), g)


def hand(fr, cam, c, z, ang, length, w, col, back=0.15):
    u = np.array([math.sin(ang), math.cos(ang), 0.0])
    v = np.array([math.cos(ang), -math.sin(ang), 0.0])
    c = np.array([c[0], c[1], z])
    Q = np.array([c - u * length * back - v * w, c + u * length - v * w * 0.4, c + u * length + v * w * 0.4, c - u * length * back + v * w])
    fr.poly(cam, Q, col)


def draw_clock(fr, cam, t):
    c, r, z = S.CLOCK, 0.29, S.HZ0 + 0.068
    for k in range(12):
        a = 2 * math.pi * k / 12
        fr.line(cam, [[c[0] + math.sin(a) * r * 0.86, c[1] + math.cos(a) * r * 0.86, z], [c[0] + math.sin(a) * r * 0.72, c[1] + math.cos(a) * r * 0.72, z]], (60, 44, 36), 1)
    hand(fr, cam, c, z, 4.6, r * 0.5, r * 0.07, (40, 30, 28))
    hand(fr, cam, c, z + 0.001, 0.4, r * 0.75, r * 0.05, (40, 30, 28))
    hand(fr, cam, c, z + 0.002, 2 * math.pi * t / 60.0, r * 0.82, r * 0.02, (196, 52, 40), 0.25)


# ---------------------------------------------------------------- the machine
UNIT_BALL = icosa([0, 0, 0], 1.0, jit=0.0, rng=np.random.default_rng(1))


def ball_light(L):
    """lit / shade multipliers for balls on the board (all in the sunbeam)"""
    lit = L.amb + L.keys[0][2] * 0.62 + 0.04
    sh = L.amb * 0.92
    return lit, sh


def draw_balls(fr, cam, L, pos, col, dim, alpha, z=B.ZBALL, glows=None, big_ok=True, outline=True):
    """balls as shaded pixel discs (or low-poly spheres in close-ups), far to near"""
    n = len(pos)
    if not n:
        return
    P = np.concatenate([pos, np.full((n, 1), z)], 1)
    Pc = cam.to_cam(P)
    zc = Pc[:, 2]
    scr = cam.proj(Pc)
    rr = B.R_BALL * cam.f / np.maximum(zc, 1e-3)
    lit, sh = ball_light(L)
    ls = cam.M @ SUN
    lsx, lsy = ls[0], -ls[1]
    ln = math.hypot(lsx, lsy) + 1e-6
    lsx, lsy = lsx / ln, lsy / ln
    order = np.argsort(-zc)
    dr = fr.draw
    ddr = fr.ddraw
    for i in order:
        if zc[i] < 0.15:
            continue
        x, y, r = scr[i, 0], scr[i, 1], rr[i]
        if x < -r - 2 or x > W + r + 2 or y < -r - 2 or y > H + r + 2:
            continue
        c = col[i]
        dm = dim[i]
        if dm > 0:
            g = c.mean()
            c = (c * (1 - 0.6 * dm) + g * 0.6 * dm) * (1 - 0.55 * dm)
        a = alpha[i]
        if a < 1:
            c = c * a + np.array([210, 170, 120.0]) * (1 - a)
        if r > 5.5 and big_ok:
            fcs = [(Q * B.R_BALL + P[i], nn) for Q, nn in UNIT_BALL]
            draw_faces(fr, cam, L, fcs, c, -7, emit=0.08)
            hx, hy = x + lsx * r * 0.42, y + lsy * r * 0.42
            fr.draw.ellipse([hx - r * 0.16, hy - r * 0.16, hx + r * 0.16, hy + r * 0.16], fill=(255, 250, 236))
            continue
        cs = tuple(int(v) for v in np.clip(c * sh, 0, 255))
        cl = tuple(int(v) for v in np.clip(c * lit, 0, 255))
        if r < 1.15:
            dr.rectangle([int(x), int(y), int(x) + (1 if r > 0.7 else 0), int(y) + (1 if r > 0.7 else 0)], fill=cl)
            ddr.rectangle([int(x), int(y), int(x) + 1, int(y) + 1], fill=float(zc[i]))
            continue
        ro = r + (0.75 if outline else 0.0)
        if outline:
            dr.ellipse([x - ro, y - ro, x + ro, y + ro], fill=tuple(int(v * 0.3) for v in cs))
        dr.ellipse([x - r, y - r, x + r, y + r], fill=cs)
        ddr.ellipse([x - ro, y - ro, x + ro, y + ro], fill=float(zc[i]))
        r2 = r * 0.72
        ox, oy = lsx * r * 0.28, lsy * r * 0.28
        dr.ellipse([x + ox - r2, y + oy - r2, x + ox + r2, y + oy + r2], fill=cl)
        if r > 1.8:
            hx, hy = x + lsx * r * 0.45, y + lsy * r * 0.45
            hc = tuple(int(v) for v in np.clip(np.array(cl) * 0.4 + 255 * 0.6, 0, 255))
            dr.rectangle([int(hx), int(hy), int(hx), int(hy)], fill=hc)


def draw_pegs(fr, cam, L, flash, fcol, glows):
    lit, sh = ball_light(L)
    base = np.array(S.BRASS, float)
    Pb = np.concatenate([B.PEG_XY, np.full((len(B.PEGS), 1), B.ZB + 0.004)], 1)
    Pt = np.concatenate([B.PEG_XY, np.full((len(B.PEGS), 1), B.ZB + B.PEG_LEN)], 1)
    sb, st = cam.wp(Pb), cam.wp(Pt)
    zt = cam.depth(Pt)
    rr = B.R_PEG * cam.f / np.maximum(zt, 1e-3)
    dark = tuple(int(v) for v in np.clip(np.array(S.BRASS_D, float) * sh * 0.85, 0, 255))
    for i in range(len(B.PEGS)):
        r = rr[i]
        (x0, y0), (x1, y1) = sb[i], st[i]
        if x1 < -5 or x1 > W + 5 or y1 < -5 or y1 > H + 5:
            continue
        fr.draw.ellipse([x0 - r, y0 - r, x0 + r, y0 + r], fill=dark)
        dx, dy = x1 - x0, y1 - y0
        ln = math.hypot(dx, dy)
        if ln > 0.3:
            nx, ny = -dy / ln * r, dx / ln * r
            fr.draw.polygon([(x0 + nx, y0 + ny), (x1 + nx, y1 + ny), (x1 - nx, y1 - ny), (x0 - nx, y0 - ny)], fill=dark)
        f = min(1.0, flash[i])
        c = base * lit * 0.78 * (1 - 0.6 * f) + (fcol[i] * 0.6 + 255 * 0.4) * f * 1.1
        fr.draw.ellipse([x1 - r, y1 - r, x1 + r, y1 + r], fill=tuple(int(v) for v in np.clip(c, 0, 255)))
        fr.ddraw.ellipse([x1 - r, y1 - r, x1 + r, y1 + r], fill=float(zt[i]))
        if r > 1.6:
            fr.draw.rectangle([int(x1 - r * 0.35), int(y1 - r * 0.4), int(x1 - r * 0.35), int(y1 - r * 0.4)], fill=(255, 244, 210))
        if f > 0.15:
            glows.append((x1, y1, (2.2 + 2.5 * f) * max(1.0, r * 0.8), tuple(fcol[i] * 0.7 + 70), 0.35 * f))


def draw_pile(fr, cam, L, t):
    lvl = T.hopper_level(t)
    if lvl <= 0.01:
        return
    yl = S.HOP_Y0 + lvl * (S.HOP_Y1 - S.HOP_Y0 - 0.025)
    w = S.hop_w(S.PILE[:, 1])
    mound = 0.035 * (1 - (S.PILE[:, 0] / np.maximum(w, 1e-3)) ** 2) * min(1.0, lvl * 3)
    m = S.PILE[:, 1] < yl + mound
    if not m.any():
        return
    col = B.PEARL[None] * S.PILE_TINT[m]
    draw_balls(fr, cam, L, S.PILE[m], col, np.full(m.sum(), 0.12), np.ones(m.sum()), z=B.ZBALL, big_ok=False, outline=False)


def draw_elevator(fr, cam, L, t, glows):
    off = S.bucket_offset(t)
    y0, y1 = S.EL_RUN
    span = y1 - y0
    # chain
    for xo in (-0.05, 0.05):
        pts = [[S.EL_XC + xo, y0 - 0.06, B.ZB + 0.02], [S.EL_XC + xo, y1 + 0.06, B.ZB + 0.02]]
        fr.line(cam, pts, (60, 54, 50), 1)
    for sp in S.SPROCKETS:
        s = cam.wp(sp)
        r = 0.07 * cam.f / max(cam.depth(sp)[0], 0.1)
        fr.disc(s, r, (150, 112, 60), n=10, rot=off * 6, z=float(cam.depth(sp)[0]))
        fr.disc(s, r * 0.35, (90, 70, 44), n=6, z=float(cam.depth(sp)[0]))
    lift = T.T_LIFT[0] - 0.2 <= (t % T.DUR) < T.T_LIFT[1] + 0.3
    rng = np.random.default_rng(int(off * 1000) % 7)
    nb = int(span / S.BUCKET_GAP) + 1
    loaded_pos, loaded_col = [], []
    for k in range(nb + 1):
        yb = y0 + ((k * S.BUCKET_GAP + off) % (nb * S.BUCKET_GAP))
        if yb > y1:
            continue
        bx = S.EL_XC
        Q = S.box(bx, yb, B.ZB + 0.05, 0.06, 0.03, 0.035)
        draw_faces(fr, cam, L, Q, S.BRASS, S.MG + 1, sunvis=True)
        if lift:
            kid = int(round((k * S.BUCKET_GAP + off) / S.BUCKET_GAP))
            for j in range(3):
                loaded_pos.append((bx - 0.026 + 0.026 * j, yb + 0.03 + 0.004 * (j % 2)))
                loaded_col.append(B.RAINBOW[(kid * 5 + j * 3) % 13])
    if loaded_pos:
        draw_balls(fr, cam, L, np.array(loaded_pos), np.array(loaded_col), np.zeros(len(loaded_pos)), np.ones(len(loaded_pos)), z=B.ZB + 0.09, big_ok=False)
        # balls rolling down the chute into the hopper
        a, b = S.CHUTE
        tt = t % T.DUR
        us = ((tt * 2.2 + np.arange(6) / 6.0) % 1.0)
        cp = a[None, :2] + (b - a)[None, :2] * us[:, None]
        draw_balls(fr, cam, L, cp, B.RAINBOW[(np.arange(6) * 4) % 13], np.zeros(6), np.ones(6), z=B.ZBALL, big_ok=False)


def draw_machine(fr, cam, L, t, st, glows):
    """everything on the machine, in a fixed front-to-back order (the camera is always in front)"""
    flash, fcol = M.peg_flash(t)
    tt_ = t % T.DUR
    if T.PASCAL_T0 - 0.05 <= tt_ < T.PASCAL_T0 + T.PASCAL_STEP * 12 + 0.6:          # Pascal sweep: pegs flash as they light
        for i, (k, c) in enumerate(B.PEGS):
            tp = T.PASCAL_T0 + T.PASCAL_STEP * k + ((c + k) // 2) * 0.2 / (k + 1)
            if tp <= tt_ < tp + 0.5:
                w = math.exp(-(tt_ - tp) / 0.1)
                fcol[i] = (fcol[i] * flash[i] + np.array([255, 236, 170.0]) * w) / (flash[i] + w)
                flash[i] = min(1.6, flash[i] + w)
    for o in S.MACH:
        if o.name in ('base', 'foot', 'plate'):
            draw_obj(fr, cam, L, o)
    draw_faces(fr, cam, L, S.HOP_BACK, S.WALNUT_D, S.MG)
    draw_pile(fr, cam, L, t)
    draw_pegs(fr, cam, L, flash, fcol, glows)
    landed = st['landed']
    # bin balls first, then the ones in flight
    for sel in (landed, ~landed):
        if sel.any():
            draw_balls(fr, cam, L, st['pos'][sel], st['col'][sel], st['dim'][sel], st['alpha'][sel], glows=glows)
    for j, d in enumerate(S.DIVIDERS):
        draw_faces(fr, cam, L, d, (118, 72, 46) if j not in (0, 13) else S.WALNUT_D, S.MG + 2 + j)
        draw_faces(fr, cam, L, S.DIV_CAPS[j], S.BRASS, S.MG + 2 + j)
    # trapdoor floor: dark slots open during the drain
    tt = t % T.DUR
    if T.T_DRAIN - 0.1 <= tt < T.T_LIFT[0] + 0.6:
        o = min(1.0, (tt - T.T_DRAIN + 0.1) / 0.2) * min(1.0, (T.T_LIFT[0] + 0.6 - tt) / 0.3)
        for b in range(B.NB):
            x = B.bin_x(b)
            Q = np.array([[x - 0.04, B.Y_BF + 0.004, B.ZB + 0.005], [x + 0.04, B.Y_BF + 0.004, B.ZB + 0.005],
                          [x + 0.04, B.Y_BF + 0.004, B.ZB + 0.065], [x - 0.04, B.Y_BF + 0.004, B.ZB + 0.065]])
            fr.poly(cam, Q + [0, -0.012 * o, 0], (30, 20, 16))
    for o in S.MACH:
        if o.name in ('hoprim', 'crest', 'crestplate', 'finial', 'base', 'foot', 'plate'):
            continue
        draw_obj(fr, cam, L, o)
    draw_elevator(fr, cam, L, t, glows)
    # hopper glass: a faint sheen
    for o in S.MACH:
        if o.name in ('hoprim', 'crest', 'crestplate', 'finial'):
            draw_obj(fr, cam, L, o)


# ---------------------------------------------------------------- sunbeams (volumetric, quarter res) + motes
QH, QW = H // 4, W // 4
_QY, _QX = np.mgrid[0:QH, 0:QW].astype(np.float32)


def sunbeam(cam, depth):
    dirs = cam.pixel_rays(_QX * 4 + 2, _QY * 4 + 2)
    dq = np.minimum(depth[2::4, 2::4], 16.0)
    n = 18
    acc = np.zeros((QH, QW), np.float32)
    jit = BAYER_FULL[2::4, 2::4] - 0.5
    for i in range(n):
        s = (i + 0.5 + jit) / n * dq
        P = cam.pos[None, None] + dirs * s[..., None]
        Pf = P.reshape(-1, 3)
        inside = (Pf[:, 0] > S.HX0) & (Pf[:, 0] < S.HX1) & (Pf[:, 2] > S.HZ0) & (Pf[:, 2] < S.HZ1) & (Pf[:, 1] > 0) & (Pf[:, 1] < S.HY)
        lit = S.win_lit(Pf) * inside
        m = lit > 0
        if m.any():
            lit[m] *= MACH_OCC.vis(Pf[m], -99, d=SUN)
        acc += lit.reshape(QH, QW) * dq / n
    beam = 1 - np.exp(-acc * 0.2)
    im = Image.fromarray((beam * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(1.3))
    return np.asarray(im.resize((W, H), Image.BILINEAR), np.float32) / 255.0


def draw_motes(img, cam, depth, t):
    Mo = S.MOTES
    w = 2 * math.pi / 60.0
    P = Mo + np.stack([0.06 * np.sin(w * 3 * t + Mo[:, 1] * 5), 0.08 * np.sin(w * 2 * t + Mo[:, 0] * 3), 0.06 * np.cos(w * 4 * t + Mo[:, 2] * 4)], 1)
    lit = S.win_lit(P)
    zc = cam.depth(P)
    scr = cam.wp(P)
    tw = 0.55 + 0.45 * np.sin(w * 17 * t + Mo[:, 0] * 40)
    for z, l, (x, y), a in zip(zc, lit, scr, tw):
        if l <= 0 or z < 0.4:
            continue
        xi, yi = int(x), int(y)
        if 0 <= xi < W and 0 <= yi < H and depth[yi, xi] > z:
            img[yi, xi] = img[yi, xi] * (1 - 0.55 * a) + np.array([255, 240, 200.0]) * 0.55 * a


# ---------------------------------------------------------------- focus (locked on the machine)
MACH_PTS = np.array([[x, y, z] for x in (-0.9, 1.08) for y in (B.Y_STAGE, 4.42) for z in (B.ZB - 0.17, B.ZB + 0.17)] +
                    [[0, B.Y_BF, B.ZBALL], [0, 3.3, B.ZBALL], [0, 2.2, B.ZBALL]])


def focus(cam):
    zf = cam.depth(np.array([0.08, 2.3, B.ZBALL]))[0]
    zr = cam.depth(MACH_PTS)
    sharp = float(np.clip(np.abs(1 - zf / np.maximum(zr, 0.2)).max() + 0.03, 0.05, 0.9))
    K = 0.012 * cam.f * min(1.3, 3.0 / zf)
    return zf, sharp, K


def subject_blur(cam):
    zf, sharp, K = focus(cam)
    return float(coc_map(cam.depth(MACH_PTS), zf, K, sharp).max())


# ---------------------------------------------------------------- overlays (after post, crisp)
def center_text(img, txt, x, y, col, scale=1, alpha=1.0):
    m = text_mask(txt, scale)
    stamp(img, m, int(round(x - m.shape[1] / 2)), int(round(y - m.shape[0] / 2)), col, alpha=alpha)


def board_scr(cam, x, y, z=B.ZB + B.PEG_LEN):
    return cam.wp(np.array([x, y, z]))


def pascal_overlay(img, cam, t):
    """12.25-16.3 s: path counts on every peg, row by row, then on the bins; a running PATHS counter"""
    if not (T.PASCAL_T0 - 0.1 <= t < 16.4):
        return
    fade = 1 - ss(15.95, 16.35, t)
    for i, (k, c) in enumerate(B.PEGS):
        tk = T.PASCAL_T0 + T.PASCAL_STEP * k
        if t < tk:
            continue
        v = math.comb(k, (c + k) // 2)
        tk = tk + ((c + k) // 2) * 0.2 / (k + 1)
        if t < tk:
            continue
        s = board_scr(cam, *B.PEG_XY[i])
        col = CREAM if v < 100 else AMBER
        if t - tk < 0.12:
            col = (255, 255, 255)
        center_text(img, str(v), s[0], s[1] - 1, col, 1, alpha=fade)
    tb = T.PASCAL_T0 + T.PASCAL_STEP * 12
    if t >= tb:
        for b in range(B.NB):
            s = board_scr(cam, B.bin_x(b), B.Y_BT - 0.06)
            col = AMBER if b == 6 else (SKYB if b in (0, 12) else CREAM)
            if t - tb < 0.15:
                col = (255, 255, 255)
            center_text(img, str(math.comb(12, b)), s[0], s[1], col, 1, alpha=fade)
    kk = int(min(13, math.floor((t - T.PASCAL_T0) / T.PASCAL_STEP) + 1))
    if kk >= 1:
        s = board_scr(cam, 0, B.Y_BT - 0.3)
        center_text(img, f'{2 ** (kk - 1):,} PATHS', s[0], s[1], AMBER if kk == 13 else CREAM, 2, alpha=fade)


def path_trail(img, cam, i, t, fade, gold=False):
    """the path a ball has taken so far, one segment per bounce (cool = left, warm = right)"""
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    pos = M.eval_balls(np.array([i]), np.array([t]))[0][0]
    for k in range(12):
        if t < M.TC[i, k]:
            break
        right = M.CC[i, k + 1] > M.CC[i, k]
        p0 = board_scr(cam, M.XC[i, k], B.Y_C[k], B.ZBALL)
        if k < 11 and t >= M.TC[i, k + 1]:
            p1 = board_scr(cam, M.XC[i, k + 1], B.Y_C[k + 1], B.ZBALL)
        else:
            p1 = board_scr(cam, pos[0], pos[1], B.ZBALL)
        col = (255, 206, 80) if gold else (AMBER if right else SKYB)
        bg = img[int(np.clip(p0[1], 0, H - 1)), int(np.clip(p0[0], 0, W - 1))]
        col = tuple(int(c * fade + v * (1 - fade)) for c, v in zip(col, bg))
        d.line([tuple(p0), tuple(p1)], fill=col, width=2)
    img[:] = np.asarray(pil)


def arrows_overlay(img, cam, t):
    """8.3-12 s: the second ball's path so far (cool = left, warm = right) and an arrow at each peg"""
    a0, a1 = T.ARROWS
    if not (a0 <= t < a1 + 0.8):
        return
    i = M.SECOND
    fade = 1 - ss(a1 + 0.3, a1 + 0.8, t)
    path_trail(img, cam, i, t, fade)
    for k in range(12):
        tk = M.TC[i, k]
        if not (tk <= t < tk + 0.9):
            continue
        u = t - tk
        al = min(1.0, u / 0.05) * (1 - ss(0.6, 0.9, u)) * fade
        right = M.CC[i, k + 1] > M.CC[i, k]
        x, y = B.PEG_XY[B.PEG_ID[(k, int(M.CC[i, k]))]]
        s = board_scr(cam, x + (0.045 if right else -0.045), y + 0.03)
        center_text(img, '→' if right else '←', s[0], s[1], AMBER if right else SKYB, 2, alpha=al)


def bin_labels(img, cam, t, counts):
    a0, a1 = T.BIN_WAYS
    if a0 <= t < a1:
        al = ss(a0, a0 + 0.3, t) * (1 - ss(a1 - 0.3, a1, t))
        for b, txt, col in ((6, '924', AMBER), (0, '1', SKYB), (12, '1', SKYB)):
            s = board_scr(cam, B.bin_x(b), B.Y_BT - 0.1)
            center_text(img, txt, s[0], s[1], col, 2, alpha=al)
    c0, c1 = T.COUNTS
    if c0 <= t < c1:
        al = ss(c0, c0 + 0.3, t) * (1 - ss(c1 - 0.3, c1, t))
        for b in range(B.NB):
            yt = B.stack_top(counts[b])
            s = board_scr(cam, B.bin_x(b), yt + 0.045, B.ZBALL)
            center_text(img, str(int(counts[b])), s[0], s[1], CREAM if b not in (0, 12) else SKYB, 1, alpha=al)


from scipy.interpolate import PchipInterpolator
_PM = PchipInterpolator(np.arange(13), [1000 * math.comb(12, k) / 4096 for k in range(13)])


def curve_y(x):
    """ideal stack height over position x for 1,000 balls (binomial expectation, smoothly interpolated)"""
    b = np.clip(np.asarray(x, float) / B.DX + 6, 0, 12)
    n = np.maximum(_PM(b), 0)
    return B.Y_BF + 2 * B.R_BALL + np.maximum(n / 3.5 - 0.6, 0) * B.ROW_H


def curve_overlay(img, cam, t, glows_post):
    a0, a1 = T.CURVE
    if not (a0 <= t < T.CURVE_OUT):
        return
    fade = 1 - ss(T.CURVE_OUT - 0.4, T.CURVE_OUT, t)
    head = min(1.0, (t - a0) / (a1 - a0))
    xs = np.linspace(-0.66, 0.66, 140)
    xs = xs[: max(2, int(len(xs) * head))]
    ys_ = curve_y(xs)
    pts = cam.wp(np.stack([xs, ys_, np.full_like(xs, B.ZGLASS)], 1))
    pil = Image.fromarray(img)
    d = ImageDraw.Draw(pil)
    pulse = 0.8 + 0.2 * math.exp(-((t - T.T_DROP) % T.BEAT) / 0.15)
    col = tuple(int(v * fade + c * (1 - fade)) for v, c in zip((255, 246, 214), (200, 190, 170)))
    d.line([tuple(p) for p in pts + [0, 1]], fill=(40, 26, 20), width=3)
    d.line([tuple(p) for p in pts], fill=col, width=2)
    out = np.asarray(pil).copy()
    img[:] = out
    if head < 1:
        glows_post.append((pts[-1][0], pts[-1][1], 9, (255, 220, 140), 0.9))
    if t > a1 + 0.3:
        al = ss(a1 + 0.3, a1 + 0.6, t) * fade
        s = cam.wp(np.array([0.0, curve_y(np.array([0.0]))[0] + 0.07, B.ZGLASS]))
        center_text(img, 'IDEAL', s[0], s[1], (255, 246, 214), 1, alpha=al)


def plate_text(img, cam, t, total):
    x0, x1, y0, y1, z = S.PLATE
    a, b = cam.wp(np.array([x0, y1, z])), cam.wp(np.array([x1, y0, z]))
    w = b[0] - a[0]
    if w < 30:
        return
    sc = 2 if w > 120 else 1
    center_text(img, f'BALLS {total:4d}', 0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), (60, 34, 20), sc)


def crest_text(img, cam):
    x0, x1, y0, y1, z = S.CREST_PLATE
    a, b = cam.wp(np.array([x0, y1, z])), cam.wp(np.array([x1, y0, z]))
    w = b[0] - a[0]
    if w < 60:
        return
    center_text(img, 'THE QUINCUNX', 0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), (70, 40, 22), 1)


def hero_ring(img, cam, st, t):
    """a thin ring around the hero ball (4-8.2 s) and the second ball (8-12 s)"""
    for bi, a0, a1 in ((M.HERO, T.T_FREEZE, M.TLAND[M.HERO] + 0.4), (M.SECOND, T.SECOND_T0 - 0.35, M.TLAND[M.SECOND] + 0.3),
                       (M.GOLD, M.TLAND[M.GOLD] - 0.05, T.GOLD_FADE[0] + 0.2)):
        if not (a0 <= t < a1):
            continue
        m = np.where(st['idx'] == bi)[0]
        if not len(m):
            continue
        p = st['pos'][m[0]]
        s = cam.wp(np.array([p[0], p[1], B.ZBALL]))
        r = B.R_BALL * cam.f / cam.depth(np.array([p[0], p[1], B.ZBALL]))[0] + 3.5
        al = min(1.0, (t - a0) / 0.2) * min(1.0, (a1 - t) / 0.3)
        pil = Image.fromarray(img)
        ImageDraw.Draw(pil).ellipse([s[0] - r, s[1] - r, s[0] + r, s[1] + r], outline=(255, 250, 230))
        img[:] = (np.asarray(pil) * al + img * (1 - al)).astype(np.uint8)


# ---------------------------------------------------------------- render
def render(f):
    t = f / FPS
    cam = camera(t)
    L, ks = make_lights(t, cam)
    fr = Frame((40, 34, 40))
    cut_edges(cam, fr)
    sun_scr = shell_pass(fr, cam, t, L, ks)
    glows = []
    # ---- behind the machine
    items = [(np.linalg.norm(o.ctr - cam.pos), 'obj', o) for o in S.BACK]
    items.append((np.linalg.norm(S.CLOCK - cam.pos) - 0.1, 'clock', None))
    pg = S.pigeon(t)[0]
    items.append((np.linalg.norm(pg - cam.pos) - 0.05, 'pigeon', None))
    for pw in S.PAWNS:
        if pw.get('stand'):
            items.append((np.linalg.norm(pw['p'] - cam.pos), 'pawn', pw))
    for _, kind, o in painter(items):
        if kind == 'obj':
            if on_screen(cam, o.ctr, np.linalg.norm(o.hi - o.lo) * 0.6):
                draw_obj(fr, cam, L, o)
        elif kind == 'clock':
            draw_clock(fr, cam, t)
        elif kind == 'pigeon':
            for name, fcs in S.pigeon_faces(t).items():
                c = {'body': (150, 150, 168), 'head': (110, 120, 140), 'beak': (230, 180, 90), 'wing': (128, 128, 146)}[name]
                draw_faces(fr, cam, L, fcs, c, -21, double=(name == 'wing'))
        elif kind == 'pawn':
            draw_pawn(fr, cam, L, o, t)
    # ---- the backplate (maple), per pixel, with every shadow on it
    st = M.state(t)
    backplate_pass(fr, cam, t, L, ks, st)
    # ---- the machine
    draw_machine(fr, cam, L, t, st, glows)
    glass_sheen(fr, cam)
    # ---- in front: lectern, lamps, gasolier, benches, pawns
    items = [(np.linalg.norm(o.ctr - cam.pos), 'obj', o) for o in S.FRONT]
    for pw in S.PAWNS:
        if not pw.get('stand'):
            items.append((np.linalg.norm(pw['p'] + [0, 0.25, 0] - cam.pos), 'pawn', pw))
    for _, kind, o in painter(items):
        if kind == 'obj':
            if on_screen(cam, o.ctr, np.linalg.norm(o.hi - o.lo) * 0.6):
                draw_obj(fr, cam, L, o)
        else:
            if on_screen(cam, o['p'] + [0, 0.25, 0], 0.4):
                draw_pawn(fr, cam, L, o, t)
    # lamp flames + glows
    for i, (lp, R, k) in enumerate(S.LAMPS):
        zc = cam.depth(lp)[0]
        if zc < 0.3:
            continue
        s = cam.wp(lp)
        f_ = flicker(t, i)
        r = 0.02 * cam.f / zc
        fr.draw.ellipse([s[0] - r * 0.5, s[1] - r * 1.4, s[0] + r * 0.5, s[1] + r * 0.4], fill=(255, 236, 170))
        glows.append((s[0], s[1] - r * 0.3, min(60, 0.24 * cam.f / zc * f_), (255, 150, 60), 0.45 * f_))
    if sun_scr is not None:
        glows.append((sun_scr[0], sun_scr[1], min(60, 0.9 * cam.f / 9.0), (255, 200, 120), 0.35))
    # ---- lens (focus locked on the machine), sunbeams, glows, grade
    img, depth = fr.array(), fr.depth()
    zf, sharp, K = focus(cam)
    img = dof(img, coc_map(depth, zf, K, sharp=sharp))
    beam = sunbeam(cam, depth)
    img = img + beam[..., None] * np.array([255, 222, 170.0])[None, None] * 0.13
    draw_motes(img, cam, depth, t)
    # equaliser: each landing flashes its bin top in the bin's colour
    bf = M.bin_flash(t)
    counts = M.bin_counts(t)
    for b in range(B.NB):
        if bf[b] > 0.05:
            yt = B.stack_top(counts[b]) + 0.006
            s = cam.wp(np.array([B.bin_x(b), yt, B.ZBALL]))
            glows.append((s[0], s[1], 0.06 * cam.f / zf, tuple(B.RAINBOW[b]), min(0.7, 0.25 * bf[b])))
            a_ = min(1.0, 0.7 * bf[b])
            p0 = cam.wp(np.array([B.bin_x(b) - 0.042, yt, B.ZBALL]))
            p1 = cam.wp(np.array([B.bin_x(b) + 0.042, yt, B.ZBALL]))
            n_ = int(abs(p1[0] - p0[0])) + 1
            xs_ = np.linspace(p0[0], p1[0], n_).astype(int)
            ys_ = np.linspace(p0[1], p1[1], n_).astype(int)
            ok = (xs_ >= 0) & (xs_ < W) & (ys_ >= 1) & (ys_ < H)
            cb = B.RAINBOW[b] * 0.6 + 110
            for dy_ in (0, -1):
                img[ys_[ok] + dy_, xs_[ok]] = img[ys_[ok] + dy_, xs_[ok]] * (1 - a_) + cb * a_
    # glows around the hero / golden ball
    tt = t % T.DUR
    for bi, a0, a1, col in ((M.HERO, T.T_FREEZE, M.TLAND[M.HERO] + 0.3, (255, 250, 220)), (M.GOLD, T.GOLD_T0 - 0.4, T.GOLD_FADE[1], (255, 200, 80))):
        m = np.where(st['idx'] == bi)[0]
        if a0 <= tt < a1 and len(m):
            p = st['pos'][m[0]]
            s = cam.wp(np.array([p[0], p[1], B.ZBALL]))
            glows.append((s[0], s[1], (0.09 if bi == M.GOLD else 0.05) * cam.f / zf, col, (0.95 if bi == M.GOLD else 0.6) * st['alpha'][m[0]]))
    img = apply_glows(img, fr.glows + glows)
    # golden-ball spotlight: the hall dims around it
    if T.GOLD_T0 - 0.5 <= tt < T.GOLD_FADE[1]:
        m = np.where(st['idx'] == M.GOLD)[0]
        if len(m):
            p = st['pos'][m[0]]
            s = cam.wp(np.array([p[0], p[1], B.ZBALL]))
            a = ss(T.GOLD_T0 - 0.5, T.GOLD_T0, tt) * (1 - ss(T.GOLD_FADE[0], T.GOLD_FADE[1], tt))
            spot = np.exp(-((XX - s[0]) ** 2 + (YY - s[1]) ** 2) / (2 * 70.0 ** 2))
            img = img * (1 - a * 0.32 * (1 - spot))[..., None]
    img = grade(bloom(np.clip(img, 0, 255), thr=215, k=0.35))
    img = np.clip(img * vignette(0.7, 0.6, 0.35) * top_scrim(0.5, 30, 140), 0, 255)
    out = np.clip(img, 0, 255).astype(np.uint8)
    # ---- overlays
    post_glows = []
    pascal_overlay(out, cam, tt)
    arrows_overlay(out, cam, tt)
    if T.GOLD_T0 <= tt < T.GOLD_FADE[1]:
        path_trail(out, cam, M.GOLD, tt, 1 - ss(T.GOLD_FADE[0], T.GOLD_FADE[1], tt), gold=True)
    bin_labels(out, cam, tt, counts)
    curve_overlay(out, cam, tt, post_glows)
    if post_glows:
        o2 = apply_glows(out.astype(np.float32), post_glows)
        out = np.clip(o2, 0, 255).astype(np.uint8)
    if T.GOLD_FADE[0] <= tt < T.GOLD_FADE[1] + 0.4:
        u = (tt - T.GOLD_FADE[0]) / (T.GOLD_FADE[1] - T.GOLD_FADE[0])
        p = np.array([M.XS[M.GOLD], M.YS[M.GOLD]])
        sp = []
        for k in range(10):
            a = k * 2.4
            q = p + np.array([math.cos(a) * 0.03 * u, 0.12 * u * (0.5 + (k % 3) * 0.3) + 0.01 * math.sin(a)])
            s = cam.wp(np.array([q[0], q[1], B.ZBALL]))
            sp.append((s[0], s[1], 2 + (k % 2), (255, 220, 120), 0.8 * (1 - min(1, u)) * (0.6 + 0.4 * math.sin(tt * 30 + k))))
        out = np.clip(apply_glows(out.astype(np.float32), sp), 0, 255).astype(np.uint8)
    hero_ring(out, cam, st, tt)
    plate_text(out, cam, tt, int(counts.sum()))
    crest_text(out, cam)
    draw_captions(out, tt, T.CAPTIONS)
    return out


def glass_sheen(fr, cam):
    """faint diagonal reflections in the glass front (they slide a little as the camera moves)"""
    arr = fr.array()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    with np.errstate(divide='ignore', invalid='ignore'):
        tg = np.where(dw[..., 2] < -1e-7, (B.ZGLASS - p[2]) / dw[..., 2], -1)
    X, Y = p[0] + tg * dw[..., 0], p[1] + tg * dw[..., 1]
    m = (tg > 0) & (X >= B.X0) & (X <= B.X1) & (Y >= B.Y_BF) & (Y <= B.Y_TOP)
    u = (X - 0.62 * Y + 0.18 * p[0]) / 1.05
    band = 0.075 * np.clip(np.cos(2 * math.pi * u), 0, 1) ** 10 + 0.045 * np.clip(np.cos(2 * math.pi * (u + 0.23)), 0, 1) ** 24
    band = np.floor(band * 40 + BAYER_FULL * 0.999) / 40
    arr = np.where(m[..., None], arr + band[..., None] * (255 - arr), arr)
    fr.set_array(arr)


def backplate_pass(fr, cam, t, L, ks, st):
    arr, depth = fr.array(), fr.depth()
    dw = cam.pixel_rays(XX, YY)
    p = cam.pos
    with np.errstate(divide='ignore', invalid='ignore'):
        tb = np.where(dw[..., 2] < -1e-7, (B.ZB - p[2]) / dw[..., 2], -1)
    X, Y = p[0] + tb * dw[..., 0], p[1] + tb * dw[..., 1]
    m = (tb > 0) & (X >= B.X0) & (X <= B.X1) & (Y >= B.Y_BF) & (Y <= B.Y_TOP)
    ys, xs = np.nonzero(m)
    if not len(xs):
        return
    X, Y = X[ys, xs], Y[ys, xs]
    # ball shadows (dynamic)
    pos = st['pos']
    polys = []
    if len(pos):
        off = -(B.ZBALL - B.ZB) / SUN[2] * SUN[[0, 1]]
        ang = np.linspace(0, 2 * math.pi, 9)[:-1]
        circ = np.stack([np.cos(ang), np.sin(ang)], 1) * B.R_BALL * 0.95
        sel = (pos[:, 1] > B.Y_BF - 0.05) & (pos[:, 1] < B.Y_TOP + 0.1)
        for q in pos[sel]:
            polys.append(q + off + circ)
    dyn = BP_GRID.raster(polys, 0.6) if polys else 0
    sh = np.maximum(BP_GRID.sample(BP_STATIC, X, Y), BP_GRID.sample(dyn, X, Y) if polys else 0)
    P = np.stack([X, Y, np.full_like(X, B.ZB)], 1)
    sl = S.win_lit(P + [0, 0, 0.004]) * (1 - sh * 0.72)
    # cherry: vertical boards, fine grain streaks, brass stringing inside the edge
    bw = 0.18
    ci = np.floor((X - B.X0) / bw)
    grain = 0.94 + 0.05 * hash2(ci, 2.0) + 0.05 * (hash2(np.floor(X * 260), np.floor(Y * 5 + ci)) - 0.5) + 0.025 * np.sin(Y * 9 + ci * 2.0)
    alb = np.array(S.MAPLE, float)[None] * grain[:, None]
    seam = np.abs(((X - B.X0) / bw) % 1.0 - 0.5) > 0.49
    alb[seam] *= 0.8
    inl = (np.abs(X - B.X0 - 0.035) < 0.005) | (np.abs(B.X1 - 0.035 - X) < 0.005) | (np.abs(Y - B.Y_TOP + 0.035) < 0.005)
    alb[inl] = [214, 166, 84]
    I = np.tile(L.amb + 0.06, (len(X), 1))
    I += (sl * SUN[2] * 1.35)[:, None] * L.keys[0][2][None]
    for i, (lp, R, k) in enumerate(S.LAMPS):
        v = lp[None] - P
        dist = np.linalg.norm(v, axis=1) + 1e-6
        lam = np.clip(v[:, 2] / dist, 0, None)
        I += (lam * ks[i] / (1 + (dist / R) ** 2))[:, None] * WARM[None]
    I *= (1 - 0.18 * np.exp(-(Y - B.Y_BF) / 0.05))[:, None]
    col = alb * I
    arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
    depth[ys, xs] = tb[ys, xs]
    fr.set_array(arr, depth)
