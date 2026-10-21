"""Count the Thunder: static geometry + builders for the moving parts.

A cut-away prairie log cabin (no roof; front and right walls removed) on an open prairie at night:
- inside: plank floor and rag rug, stone fireplace in the left wall, a four-pane window in the back
  wall with the cat on its sill, a small table with an oil lamp, a rocking chair, a shelf of jars
- outside: fence line running out toward the storm with distance markers, a farm windmill, a red barn
  with a lightning rod (the final strike), grass tufts, hay bales, a far line of low hills
- the storm cell (low-poly cloud blobs + rain curtain) crossing from the horizon to the barn

The room shell (floor, back wall, left wall, window opening) is ray-traced per pixel in render.py,
so here it is described by planes; everything else is low-poly faces (reelkit.geometry)."""
import math
import numpy as np
from scipy.interpolate import PchipInterpolator
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, gable, new_group, ridge
import timeline as T

rng = np.random.default_rng(13)

# ---------------------------------------------------------------- room shell
RX0, RX1, RZ0, RZ1 = -1.35, 1.35, -1.15, 1.25
WALL_H = 2.0
WT = 0.16                        # log wall thickness
LOG = 0.21                       # log diameter (wall texture)
SLAB = 0.18                      # floor slab / foundation height shown at the cut edge

# four-pane window in the back wall (z = RZ0)
WX0, WX1, WY0, WY1 = -0.4, 0.46, 0.72, 1.5
WXC, WYC = 0.5 * (WX0 + WX1), 0.5 * (WY0 + WY1)
MULL = 0.028
SILL_Y = WY0
WINDOW_C = np.array([WXC, WYC, RZ0])


def win_open(x, y):
    return (x >= WX0) & (x <= WX1) & (y >= WY0) & (y <= WY1)


def win_mullion(x, y):
    on = (np.abs(x - WXC) < MULL) | (np.abs(y - WYC) < MULL)
    on |= (np.abs(x - WX0) < MULL * 1.4) | (np.abs(x - WX1) < MULL * 1.4) | (np.abs(y - WY0) < MULL * 1.4) | (np.abs(y - WY1) < MULL * 1.4)
    return on


def win_lit(P, d):
    """1 where the ray from P toward a light in direction d passes through the window glass"""
    P = np.atleast_2d(np.asarray(P, float))
    out = np.zeros(len(P), np.float32)
    if d[2] >= -1e-3:
        return out
    ok = P[:, 2] > RZ0 - 1e-3
    lit = np.ones(len(P), bool)
    for zw in (RZ0, RZ0 - WT):            # inner and outer faces: the reveal clips oblique rays
        t = (zw - P[:, 2]) / d[2]
        x, y = P[:, 0] + t * d[0], P[:, 1] + t * d[1]
        lit &= win_open(x, y)
        if zw == RZ0:
            lit &= ~win_mullion(x, y)
    out[ok & lit] = 1.0
    return out


# ---------------------------------------------------------------- small builders
def extrude(outline, y0, y1):
    out = []
    P = [np.array(p, float) for p in outline]
    ctr = np.mean(P, 0)
    for a, b in zip(P, P[1:] + P[:1]):
        out.append(face([[a[0], y0, a[1]], [b[0], y0, b[1]], [b[0], y1, b[1]], [a[0], y1, a[1]]], (ctr[0], (y0 + y1) / 2, ctr[1])))
    out.append(face([[p[0], y1, p[1]] for p in P[::-1]], up=True))
    return out


def cyl(a, b, r, n=6):
    """cylinder between points a and b (rods, legs, rockers)"""
    a, b = np.asarray(a, float), np.asarray(b, float)
    ax = b - a
    L = np.linalg.norm(ax)
    ax /= L + 1e-12
    u = np.cross(ax, [0, 1, 0] if abs(ax[1]) < 0.9 else [1, 0, 0])
    u /= np.linalg.norm(u)
    v = np.cross(ax, u)
    faces = []
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        d0 = u * math.cos(a0) + v * math.sin(a0)
        d1 = u * math.cos(a1) + v * math.sin(a1)
        Q = np.array([a + d0 * r, b + d0 * r, b + d1 * r, a + d1 * r])
        nn = d0 + d1
        faces.append((Q, nn / np.linalg.norm(nn)))
    return faces


def rot_y(faces, ang, cx, cz):
    c, s = math.cos(ang), math.sin(ang)
    R = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
    o = np.array([cx, 0, cz])
    return [((Q - o) @ R.T + o, n @ R.T) for Q, n in faces]


def transform(faces, R, t):
    return [(Q @ R.T + t, n @ R.T) for Q, n in faces]


def axis_rot(ax, ang):
    ax = np.asarray(ax, float)
    ax = ax / (np.linalg.norm(ax) + 1e-12)
    x, y, z = ax
    c, s = math.cos(ang), math.sin(ang)
    C = 1 - c
    return np.array([[c + x * x * C, x * y * C - z * s, x * z * C + y * s],
                     [y * x * C + z * s, c + y * y * C, y * z * C - x * s],
                     [z * x * C - y * s, z * y * C + x * s, c + z * z * C]])


OBJS_IN = []      # inside the cabin (lit through the window + lamp + fire)
OBJS_OUT = []     # outdoors (moon / lightning keys, ground shadows)

# ---------------------------------------------------------------- fireplace (back wall, left of the window)
FP_X0, FP_X1 = RX0 + 0.02, -0.5
FP_D = 0.34                       # how far the stone breast stands out from the back wall
FP_H = 1.14
FB_X0, FB_X1, FB_H = -1.1, -0.72, 0.58      # firebox opening
FPZ = RZ0 + FP_D                  # front face of the breast
FIRE_P = np.array([0.5 * (FB_X0 + FB_X1), 0.18, RZ0 + 0.17])
fpg = new_group()
FP_G = fpg
stones = []
for row in range(7):
    y0 = row * FP_H / 7
    y1 = (row + 1) * FP_H / 7
    xx = FP_X0
    while xx < FP_X1 - 1e-3:
        w = 0.15 + 0.11 * rng.random()
        x1 = min(FP_X1, xx + w)
        if FP_X1 - x1 < 0.06:
            x1 = FP_X1
        inset = 0.012 * rng.random()
        if not (y1 <= FB_H + 1e-3 and x1 > FB_X0 + 0.01 and xx < FB_X1 - 0.01):
            stones.append(box(0.5 * (xx + x1), 0.5 * (y0 + y1), RZ0 + 0.5 * FP_D - inset, 0.5 * (x1 - xx) - 0.006, 0.5 * (y1 - y0) - 0.006, 0.5 * FP_D))
        xx = x1
for st in stones:
    c = np.array([128, 118, 108]) * (0.86 + 0.24 * rng.random())
    OBJS_IN.append(Obj(st, c, 'stone', grp=fpg, jitter=0.03))
OBJS_IN.append(Obj(box(0.5 * (FB_X0 + FB_X1), 0.5 * (FP_H + WALL_H), RZ0 + 0.13, 0.25, 0.5 * (WALL_H - FP_H), 0.13), (118, 108, 100), 'stone', grp=fpg))
OBJS_IN.append(Obj(box(0.5 * (FB_X0 + FB_X1), FB_H / 2, RZ0 + 0.03, 0.5 * (FB_X1 - FB_X0), FB_H / 2, 0.03), (40, 30, 28), 'firebox', grp=fpg, cast=False))
OBJS_IN.append(Obj(box(0.5 * (FP_X0 + FP_X1), 0.03, RZ0 + 0.34, 0.5 * (FP_X1 - FP_X0) + 0.06, 0.03, 0.34), (112, 100, 92), 'hearth', grp=fpg, cast=False))
MANTEL_Y = FP_H + 0.04
OBJS_IN.append(Obj(box(0.5 * (FP_X0 + FP_X1), MANTEL_Y, FPZ + 0.02, 0.5 * (FP_X1 - FP_X0) + 0.05, 0.045, 0.09), (106, 66, 40), 'mantel', grp=fpg))
for k in range(3):
    a = 0.3 + k * 0.9
    p0 = FIRE_P + np.array([0.17 * math.cos(a), -0.12, 0.06 * math.sin(a)])
    p1 = FIRE_P + np.array([-0.17 * math.cos(a), -0.1, -0.06 * math.sin(a)])
    OBJS_IN.append(Obj(cyl(p0, p1, 0.035, 5), (70, 44, 30), 'firelog', grp=fpg, cast=False))
for xx, h, c in ((-1.12, 0.16, (70, 110, 150)), (-0.95, 0.11, (160, 90, 60)), (-0.65, 0.13, (90, 130, 90))):
    OBJS_IN.append(Obj(lathe(xx, FPZ - 0.01, [(0, 0.04), (h * 0.8, 0.045), (h, 0.03)], 6, cy=MANTEL_Y + 0.045), c, 'jar', grp=fpg))
OBJS_IN.append(Obj(lathe(-0.62, RZ0 + 0.52, [(0, 0.07), (0.07, 0.1), (0.14, 0.07), (0.17, 0.03)], 7, cy=0.06), (84, 84, 92), 'kettle'))

# exterior fieldstone chimney behind the fireplace, ground to above the wall top
chg = new_group()
CHIM_G = chg
CHIM_X, CHIM_Z = 0.5 * (FB_X0 + FB_X1), RZ0 - WT - 0.17
CHIM_TOP = 2.78
_y = -0.2
while _y < CHIM_TOP - 1e-3:
    _h = 0.17 + 0.06 * rng.random()
    _y1 = min(CHIM_TOP, _y + _h)
    split = CHIM_X + (0.06 if int(_y * 10) % 2 else -0.08)
    for (x0, x1) in ((CHIM_X - 0.28, split), (split, CHIM_X + 0.28)):
        c = np.array([132, 122, 112]) * (0.82 + 0.26 * rng.random())
        OBJS_OUT.append(Obj(box(0.5 * (x0 + x1), 0.5 * (_y + _y1), CHIM_Z, 0.5 * (x1 - x0) - 0.005, 0.5 * (_y1 - _y) - 0.005, 0.17), c, 'chimney', grp=chg))
    _y = _y1
OBJS_OUT.append(Obj(box(CHIM_X, CHIM_TOP + 0.03, CHIM_Z, 0.32, 0.035, 0.21), (98, 90, 84), 'chimney', grp=chg))
SMOKE_P = np.array([CHIM_X, CHIM_TOP + 0.1, CHIM_Z])

# ---------------------------------------------------------------- window parts + cat sill
wg = new_group()
WIN_G = wg
OBJS_IN.append(Obj(box(WXC, SILL_Y - 0.025, RZ0 + 0.07, 0.48, 0.025, 0.09), (150, 102, 62), 'sill', grp=wg))
OBJS_IN.append(Obj(box(WXC, SILL_Y - 0.09, RZ0 + 0.02, 0.45, 0.04, 0.02), (120, 80, 50), 'apron', grp=wg))
CAT_SILL = np.array([WXC - 0.16, SILL_Y, RZ0 + 0.09])

# ---------------------------------------------------------------- lamp table + oil lamp (right of the window)
TBL = np.array([0.9, 0.0, -0.84])
TBL_Y = 0.66
tg = new_group()
TBL_G = tg
OBJS_IN.append(Obj(box(TBL[0], TBL_Y, TBL[2], 0.26, 0.025, 0.2), (126, 80, 48), 'table', grp=tg))
for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
    OBJS_IN.append(Obj(prism(TBL[0] + sx * 0.21, TBL[2] + sz * 0.15, 0.022, 0.0, TBL_Y - 0.02, 5), (108, 68, 40), 'table', grp=tg))
LAMP_BASE = np.array([TBL[0] - 0.05, TBL_Y + 0.025, TBL[2]])
lg = new_group()
LAMP_G = lg
OBJS_IN.append(Obj(lathe(LAMP_BASE[0], LAMP_BASE[2], [(0, 0.075), (0.03, 0.08), (0.1, 0.06), (0.13, 0.03), (0.15, 0.025)], 8, cy=LAMP_BASE[1]), (176, 120, 60), 'lamp_base', grp=lg))
OBJS_IN.append(Obj(lathe(LAMP_BASE[0], LAMP_BASE[2], [(0.15, 0.035), (0.2, 0.05), (0.26, 0.04), (0.33, 0.025)], 8, cy=LAMP_BASE[1]), (255, 222, 150), 'lamp_glass', grp=lg, cast=False))
LAMP_P = LAMP_BASE + np.array([0, 0.22, 0])
# a stack of books + a mug beside the lamp
OBJS_IN.append(Obj(box(TBL[0] + 0.14, TBL_Y + 0.05, TBL[2] + 0.04, 0.08, 0.025, 0.06, rot=0.2), (140, 50, 46), 'book', grp=tg))
OBJS_IN.append(Obj(box(TBL[0] + 0.14, TBL_Y + 0.095, TBL[2] + 0.04, 0.07, 0.02, 0.055, rot=-0.1), (60, 90, 130), 'book', grp=tg))
OBJS_IN.append(Obj(lathe(TBL[0] - 0.12, TBL[2] + 0.1, [(0, 0.03), (0.07, 0.032), (0.075, 0.03)], 6, cy=TBL_Y + 0.025), (220, 210, 190), 'mug', grp=tg))

# ---------------------------------------------------------------- shelf of jars + herbs (left wall)
SH_Y = 1.42
SH_Z = -0.25
shg = new_group()
OBJS_IN.append(Obj(box(RX0 + 0.09, SH_Y, SH_Z, 0.09, 0.02, 0.42), (126, 82, 50), 'shelf', grp=shg))
for k, (dz, h, c) in enumerate(((-0.3, 0.17, (200, 120, 60)), (-0.12, 0.13, (110, 160, 110)), (0.06, 0.2, (210, 190, 120)), (0.25, 0.12, (150, 80, 90)))):
    OBJS_IN.append(Obj(lathe(RX0 + 0.09, SH_Z + dz, [(0, 0.045), (h * 0.85, 0.05), (h, 0.035)], 6, cy=SH_Y + 0.02), c, 'jar', grp=shg))
for dz in (-0.25, 0.0, 0.22):
    OBJS_IN.append(Obj(prism(RX0 + 0.1, SH_Z + dz, 0.05, SH_Y - 0.28, SH_Y - 0.02, 5, r1=0.012), (110, 130, 70), 'herbs', grp=shg, cast=False))
# coat on a peg near the front of the left wall
OBJS_IN.append(Obj(box(RX0 + 0.07, 1.15, 0.75, 0.06, 0.32, 0.17), (70, 86, 120), 'coat', grp=shg))

# ---------------------------------------------------------------- rocking chair (faces the fire, front left)
RC = np.array([0.8, 0.0, 0.62])
RC_YAW = 2.1


def rocking_chair(t):
    """rocks very gently (small life); returns faces"""
    a = 0.035 * math.sin(2 * math.pi * t / 6.0)
    f = []
    f += box(0, 0.42, 0, 0.21, 0.025, 0.2)
    f += box(-0.19, 0.78, 0.0, 0.025, 0.36, 0.19)                  # back (toward -x local)
    for zz in (-0.17, -0.06, 0.06, 0.17):
        f += box(-0.17, 0.78, zz, 0.012, 0.3, 0.018)
    for sx in (-1, 1):
        for sz in (-1, 1):
            f += prism(sx * 0.17, sz * 0.17, 0.018, 0.07, 0.42, 5)
        f += box(0.02, 0.62, sz * 0.2, 0.18, 0.015, 0.02)              # arm rests
    for sz in (-1, 1):                                                # rockers: arcs under the legs
        pts = [np.array([x, 0.06 + 0.5 * (x * x) * 0.9, sz * 0.17]) for x in np.linspace(-0.3, 0.3, 6)]
        for p, q in zip(pts, pts[1:]):
            f += cyl(p, q, 0.016, 4)
    R = axis_rot([0, 0, 1], a)
    f = [((Q - [0, 0.0, 0]) @ R.T, n @ R.T) for Q, n in f]
    f = rot_y(f, RC_YAW, 0, 0)
    return [(Q + RC, n) for Q, n in f]


RC_G = new_group()
# a knitted blanket on the rug side + broom in the corner
OBJS_IN.append(Obj(cyl([RX1 - 0.12, 0.02, RZ0 + 0.14], [RX1 - 0.2, 1.25, RZ0 + 0.2], 0.014, 4) +
                   prism(RX1 - 0.115, RZ0 + 0.135, 0.075, 0.0, 0.28, 6, r1=0.03), (170, 140, 80), 'broom'))

# ---------------------------------------------------------------- outdoors: barn with the lightning rod
BARN_C = np.array([1.3, 0.0, -5.4])
BARN_HX, BARN_HZ, BARN_WALL, BARN_RIDGE = 0.95, 1.25, 1.6, 1.15
bg_ = new_group()
BARN_G = bg_
OBJS_OUT.append(Obj(box(BARN_C[0], BARN_WALL / 2, BARN_C[2], BARN_HX, BARN_WALL / 2, BARN_HZ), (176, 52, 44), 'barn', grp=bg_))
OBJS_OUT.append(Obj(gable(BARN_C[0], BARN_WALL, BARN_C[2], BARN_HZ, BARN_HX, BARN_RIDGE, rot=math.pi / 2, over=0.08), (84, 70, 70), 'barn_roof', grp=bg_))
# gable end (front, +z) triangle + white trim + doors
gz = BARN_C[2] + BARN_HZ + 0.005
OBJS_OUT.append(Obj([face([[BARN_C[0] - BARN_HX, BARN_WALL, gz], [BARN_C[0] + BARN_HX, BARN_WALL, gz], [BARN_C[0], BARN_WALL + BARN_RIDGE, gz]], (BARN_C[0], 1, BARN_C[2]))], (176, 52, 44), 'barn', grp=bg_))
OBJS_OUT.append(Obj(box(BARN_C[0], 0.62, gz + 0.012, 0.42, 0.62, 0.012), (150, 40, 36), 'barn_door', grp=bg_, cast=False))
for a in (1, -1):
    p0 = np.array([BARN_C[0] - 0.4, 0.04, gz + 0.03])
    p1 = np.array([BARN_C[0] + 0.4, 1.2, gz + 0.03])
    if a < 0:
        p0, p1 = np.array([BARN_C[0] + 0.4, 0.04, gz + 0.03]), np.array([BARN_C[0] - 0.4, 1.2, gz + 0.03])
    OBJS_OUT.append(Obj(cyl(p0, p1, 0.022, 4), (236, 226, 210), 'barn_trim', grp=bg_, cast=False))
OBJS_OUT.append(Obj(box(BARN_C[0], 1.42, gz + 0.02, 0.18, 0.13, 0.012), (40, 30, 30), 'loft', grp=bg_, cast=False))
ROD_BASE = np.array([BARN_C[0], BARN_WALL + BARN_RIDGE, BARN_C[2] + BARN_HZ - 0.12])
ROD_TOP = ROD_BASE + np.array([0, 0.62, 0])
OBJS_OUT.append(Obj(cyl(ROD_BASE - [0, 0.05, 0], ROD_TOP, 0.016, 4) + icosa(ROD_TOP, 0.045, jit=0.0), (190, 196, 210), 'rod', grp=bg_))
VANE_P = np.array([BARN_C[0], BARN_WALL + BARN_RIDGE + 0.02, BARN_C[2] - BARN_HZ + 0.15])
# hay bales beside the barn
for (x, z, r) in ((BARN_C[0] - 1.35, BARN_C[2] + 0.6, 0.0), (BARN_C[0] - 1.25, BARN_C[2] - 0.3, 0.5), (BARN_C[0] + 1.3, BARN_C[2] + 1.0, 1.1)):
    OBJS_OUT.append(Obj(box(x, 0.17, z, 0.3, 0.17, 0.2, rot=r), (200, 168, 92), 'hay'))

# ---------------------------------------------------------------- windmill (behind left)
WM_C = np.array([-4.3, 0.0, -9.6])
WM_H = 4.3
wmg = new_group()
WM_G = wmg
legs = []
for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
    legs += cyl([WM_C[0] + sx * 0.45, 0, WM_C[2] + sz * 0.45], [WM_C[0] + sx * 0.08, WM_H, WM_C[2] + sz * 0.08], 0.03, 4)
for k in range(4):
    y = 0.6 + k * 0.95
    s = 0.45 - (0.37 * y / WM_H)
    for (a, b) in (((-1, -1), (1, -1)), ((1, -1), (1, 1)), ((1, 1), (-1, 1)), ((-1, 1), (-1, -1))):
        legs += cyl([WM_C[0] + a[0] * s, y, WM_C[2] + a[1] * s], [WM_C[0] + b[0] * s, y, WM_C[2] + b[1] * s], 0.018, 4)
OBJS_OUT.append(Obj(legs, (150, 150, 158), 'wm_tower', grp=wmg))
OBJS_OUT.append(Obj(box(WM_C[0], WM_H + 0.06, WM_C[2], 0.12, 0.08, 0.12), (120, 120, 130), 'wm_head', grp=wmg))
OBJS_OUT.append(Obj(box(WM_C[0] + 0.5, 0.25, WM_C[2] + 0.35, 0.32, 0.25, 0.2), (110, 96, 80), 'trough'))
WM_HUB = np.array([WM_C[0], WM_H + 0.08, WM_C[2] + 0.2])
WM_YAW = 0.55                     # fan faces the incoming wind (toward the storm, back-left)


def windmill_fan(ang):
    """12-blade fan + tail vane at rotation angle ang (radians)"""
    f = []
    nb = 12
    for k in range(nb):
        a = ang + 2 * math.pi * k / nb
        r0, r1 = 0.12, 0.78
        w0, w1 = 0.05, 0.12
        c, s = math.cos(a), math.sin(a)
        cp, sp = math.cos(a + math.pi / 2), math.sin(a + math.pi / 2)
        pts = [np.array([c * r0 + cp * w0, s * r0 + sp * w0, 0.0]), np.array([c * r1 + cp * w1, s * r1 + sp * w1, 0.03]),
               np.array([c * r1 - cp * w1 * 0.3, s * r1 - sp * w1 * 0.3, -0.03]), np.array([c * r0 - cp * w0 * 0.3, s * r0 - sp * w0 * 0.3, 0.0])]
        f.append(face(pts, (0, 0, -1)))
    ring = []
    for k in range(16):
        a0, a1 = 2 * math.pi * k / 16, 2 * math.pi * (k + 1) / 16
        ring += cyl([0.8 * math.cos(a0), 0.8 * math.sin(a0), 0], [0.8 * math.cos(a1), 0.8 * math.sin(a1), 0], 0.015, 3)
    f += ring
    f += icosa([0, 0, 0], 0.09, jit=0.0)
    # tail vane behind the hub
    f += cyl([0, 0, 0], [0, 0, -0.9], 0.02, 4)
    f.append(face([[0, 0.05, -0.55], [0, 0.32, -1.0], [0, -0.22, -1.05], [0, -0.05, -0.6]], (1, 0, 0)))
    R = axis_rot([0, 1, 0], WM_YAW)
    return [(Q @ R.T + WM_HUB, n @ R.T) for Q, n in f]


# ---------------------------------------------------------------- the prairie sits a little below the cabin floor
# (the cabin stands on a low fieldstone foundation). Exterior pieces above were built on y = 0; drop them.
GROUND_Y = -0.2
_SHIFT = np.array([0, GROUND_Y, 0])


def _drop(o):
    return Obj([(Q + _SHIFT, n) for Q, n in o.faces], o.color, o.name, grp=o.grp, cast=o.cast, double=o.double)


OBJS_OUT = [o if o.name == 'chimney' else _drop(o) for o in OBJS_OUT]
BARN_C = BARN_C + _SHIFT
ROD_BASE, ROD_TOP, VANE_P = ROD_BASE + _SHIFT, ROD_TOP + _SHIFT, VANE_P + _SHIFT
WM_C, WM_HUB = WM_C + _SHIFT, WM_HUB + _SHIFT

# ---------------------------------------------------------------- distance mapping (stylised diorama)
# real km -> world distance from the cabin. The barn (rod) is "here" (0 s); far strikes sit near the
# horizon. Strikes, fence markers and the sound ring all use the same mapping.
ROD_D = float(np.hypot(ROD_TOP[0], ROD_TOP[2]))


def dmap(km):
    return ROD_D + 11.5 * max(km, 0.0) ** 0.72


STORM_AZ_FAR = math.radians(-8.0)
ROD_AZ = math.atan2(ROD_TOP[0], -ROD_TOP[2])


def az_for(km):
    u = min(1.0, km / 1.4) ** 0.8
    return ROD_AZ + (STORM_AZ_FAR - ROD_AZ) * u


def ground_at(D, az):
    return np.array([D * math.sin(az), GROUND_Y, -D * math.cos(az)])


AZ_JIT = [0.0, 3.0, -4.0, 4.0, -5.0, 3.0, -7.0, -2.0, -6.0, -3.0, -7.0, -3.0, 0.0]
STRIKE_P = []
for i in range(T.N_STRIKES):
    km = T.true_km(i)
    if i == T.N_STRIKES - 1:
        STRIKE_P.append(ROD_TOP.copy())
    else:
        STRIKE_P.append(ground_at(dmap(km), az_for(km) + math.radians(AZ_JIT[i])))
STRIKE_P = np.array(STRIKE_P)

# fence line out toward the storm, with distance markers on taller posts
FENCE_AZ = math.radians(-17.0)
FENCE_DIR = np.array([math.sin(FENCE_AZ), 0, -math.cos(FENCE_AZ)])
FENCE_START = np.array([RX0 - 0.55, GROUND_Y, RZ0 - 0.9])
FENCE_LEN = 44.0
MARKERS = [('1 KM', 1.0), ('1 MI', 1.609), ('2 MI', 3.219)]
fence = []
d = 0.0
while d <= FENCE_LEN:
    p = FENCE_START + FENCE_DIR * d
    fence.append(prism(p[0], p[2], 0.075, GROUND_Y, GROUND_Y + 1.0, 5, rot=d))
    d += 1.45
OBJS_OUT.append(Obj([f for post in fence for f in post], (168, 138, 104), 'fence_post', cast=True))
MARKER_POS = []
for txt, km in MARKERS:
    # put the marker where the mapped distance from the cabin meets the fence line
    D = dmap(km)
    fs = FENCE_START * np.array([1, 0, 1])
    s = float(np.roots([1, 2 * FENCE_DIR @ fs, fs @ fs - D * D]).real.max())
    p = FENCE_START + FENCE_DIR * s
    mg = new_group()
    OBJS_OUT.append(Obj(prism(p[0], p[2], 0.09, GROUND_Y, GROUND_Y + 1.75, 5), (168, 138, 104), 'marker_post', grp=mg))
    OBJS_OUT.append(Obj(box(p[0], GROUND_Y + 1.55, p[2] + 0.08, 0.55, 0.24, 0.025), (236, 228, 204), 'marker_sign', grp=mg, cast=False))
    MARKER_POS.append((txt, np.array([p[0], GROUND_Y + 1.55, p[2] + 0.11]), D))
FENCE_WIRE_Y = (GROUND_Y + 0.5, GROUND_Y + 0.85)

# grass tufts (static positions; they sway in the wind in render.py)
TUFTS = []
for k in range(140):
    while True:
        x, z = rng.uniform(-9, 9), rng.uniform(-22, 4.5)
        inside = (RX0 - 0.4 < x < RX1 + 0.2) and (RZ0 - 0.4 < z < RZ1 + 0.3)
        barn = abs(x - BARN_C[0]) < 1.3 and abs(z - BARN_C[2]) < 1.6
        if not inside and not barn:
            break
    TUFTS.append((x, z, rng.uniform(0.18, 0.42), rng.uniform(0, 6.28)))
FAR = ridge(-75.0, 4.0, 0.0, 30, 160.0, seed=5)
FAR2 = ridge(-110.0, 7.0, 0.5, 30, 220.0, seed=8)

FEET = [(TBL[0], TBL[2], 0.2, 0.35), (RC[0], RC[2], 0.22, 0.3), (RX1 - 0.12, RZ0 + 0.14, 0.06, 0.3)]
OUT_FEET = [(BARN_C[0], BARN_C[2], 1.0, 0.45), (WM_C[0], WM_C[2], 0.35, 0.3)]

# ---------------------------------------------------------------- storm cell
# blob layout relative to the ground point under the cell: (offset, radius, squash)
CELL = []
r2 = np.random.default_rng(21)
for k in range(9):                                     # dark rain-free base, flat
    a = 2 * math.pi * k / 9 + r2.uniform(-0.2, 0.2)
    rr = 2.6 + r2.uniform(-0.5, 0.6)
    CELL.append((np.array([rr * math.cos(a), 6.7 + r2.uniform(-0.2, 0.3), rr * math.sin(a)]), 2.0 + r2.uniform(0, 0.5), (1.35, 0.5, 1.35)))
CELL.append((np.array([0, 6.9, 0]), 2.6, (1.4, 0.55, 1.4)))
for k in range(6):                                     # the tower
    a = 2 * math.pi * k / 6 + 0.4
    CELL.append((np.array([1.3 * math.cos(a), 8.6 + 0.9 * (k % 3), 1.3 * math.sin(a)]), 2.1 + r2.uniform(0, 0.4), (1.0, 0.95, 1.0)))
CELL.append((np.array([0.2, 10.6, 0.1]), 2.3, (1.0, 0.9, 1.0)))
for k in range(8):                                     # the anvil, spread wide and flat
    a = 2 * math.pi * k / 8
    rr = 3.4 + r2.uniform(-0.4, 0.8)
    CELL.append((np.array([rr * math.cos(a), 12.6 + r2.uniform(-0.3, 0.3), rr * math.sin(a) + 0.8]), 2.5 + r2.uniform(0, 0.6), (1.5, 0.38, 1.5)))
# squat far away (fits the sky band at the horizon), rises as it nears so the near bolts drop from
# the top of the frame: original layout re-scaled (wider, flatter), lift added in cell_lift()
CELL = [(np.array([o[0] * 1.35, 3.9 + (o[1] - 6.7) * 0.82, o[2] * 1.35]), r * 1.25, (sq[0], sq[1] * 0.78, sq[2])) for o, r, sq in CELL]
CELL_FACES = []
for k, (off, r, sq) in enumerate(CELL):
    CELL_FACES.append((off, icosa([0, 0, 0], r, sq, jit=0.16, rng=np.random.default_rng(100 + k))))
CELL_BOTTOM = min(float((Q + off)[:, 1].min()) for off, faces in CELL_FACES for Q, n in faces)


def cell_lift(D):
    """how far the cell is raised at ground distance D from the cabin (0 far away, ~4.4 overhead)"""
    u = min(1.0, max(0.0, (D - 7.0) / 23.0))
    return 4.4 * (1 - u * u * (3 - 2 * u))

# cell path: passes over each strike near its flash time, ends over the barn at the climax,
# then sweeps overhead and off; a fresh distant cell fades in at the horizon for the loop.
_kt = [0.0, 0.5, 12.0, 21.0, 27.0, 34.0, 38.5, 42.5, 46.0, 50.0, 53.5, 56.5]
_kd = [dmap(T.true_km(0)) + 0.6, dmap(T.true_km(0)) + 0.3, dmap(T.true_km(1)) + 0.4, dmap(T.true_km(2)), dmap(T.true_km(3)),
       dmap(T.true_km(4)), dmap(T.true_km(6)) + 0.3, dmap(T.true_km(8)), dmap(T.true_km(10)), ROD_D, -6.0, -18.0]
_DPATH = PchipInterpolator(_kt, _kd)


_P_ROD = ground_at(ROD_D, ROD_AZ)
_P_PRE = ground_at(dmap(T.true_km(10)), az_for(T.true_km(10)))
_HEAD = (_P_ROD - _P_PRE) / np.linalg.norm(_P_ROD - _P_PRE)


def cell_ground(t):
    """ground point under the main cell at time t"""
    t = min(max(t, 0.0), 56.5)
    D = float(_DPATH(t))
    if D < ROD_D:                                      # past the barn: keep heading on, overhead and away
        return _P_ROD + _HEAD * (ROD_D - D)
    km = ((D - ROD_D) / 11.5) ** (1 / 0.72)
    return ground_at(D, az_for(km))


START_CELL = cell_ground(0.0)


# ---------------------------------------------------------------- lightning bolts (one per strike)
def make_bolt(i):
    """jagged channel from the cloud base to the strike point, plus two short branches"""
    rb = np.random.default_rng(300 + i)
    end = STRIKE_P[i]
    top = end + np.array([rb.uniform(-0.6, 0.6), 0, rb.uniform(-0.6, 0.6)])
    D = float(np.hypot(*cell_ground(T.STRIKES[i][0])[[0, 2]]))
    top[1] = CELL_BOTTOM + cell_lift(D) + 0.25
    n = 14
    main = [top]
    for k in range(1, n):
        u = k / n
        p = top + (end - top) * u
        p = p + np.array([rb.uniform(-0.32, 0.32), rb.uniform(-0.08, 0.08), rb.uniform(-0.32, 0.32)]) * (1 - u * 0.6)
        main.append(p)
    main.append(end.copy())
    branches = []
    for b in range(2):
        k0 = rb.integers(2, 7)
        p = main[k0].copy()
        br = [p]
        side = np.array([rb.choice([-1, 1]) * 0.35, -0.42, rb.uniform(-0.3, 0.3)])
        for k in range(rb.integers(3, 6)):
            p = p + side + np.array([rb.uniform(-0.15, 0.15), rb.uniform(-0.1, 0.05), rb.uniform(-0.15, 0.15)])
            br.append(p.copy())
        branches.append(np.array(br))
    return np.array(main), branches


BOLTS = [make_bolt(i) for i in range(T.N_STRIKES)]
CABIN_C = np.array([0.0, 0.0, 0.0])
