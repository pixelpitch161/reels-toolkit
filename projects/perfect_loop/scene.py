"""The clockmaker's attic at sunset - static geometry + builders for the moving parts.

A cut-away attic floating in a dusk void:
- back wall: a gable end with a round window (the low sun = key light, streaming toward the camera),
  timber framing, a shelf, wall clocks and a grandfather clock (second hands + pendulums move)
- left: a knee wall and the sloped ceiling with rafters; a workbench with an oil lamp and gears
- floor: wide planks. The SUBJECT stands in the sunbeam: a wooden frame with 15 lantern-bobs.
  The sun throws every bob's shadow forward onto the floor, so the floor draws the wave again.
- small life: a cat that watches the bobs, clock hands and pendulums, flames, dust motes, birds

The room shell (floor, walls, ceiling, window backdrop) is ray-traced per pixel in render.py;
here it is only planes and extents. Everything else is low-poly faces (reelkit.geometry).
Every periodic motion has a whole number of cycles in 60 s, so the loop stays seamless."""
import math
import numpy as np
from reelkit.config import dirv, nrm
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, new_group
import timeline as T

rng = np.random.default_rng(11)

# ---------------------------------------------------------------- room shell
RX0, RX1, RZ0, RZ1 = -1.7, 1.7, -1.9, 1.4
KNEE = 0.9                      # knee wall height on the left
SLOPE = 1.0                     # 45 degree roof
RIDGE_X = 0.85
RIDGE_H = KNEE + (RIDGE_X - RX0) * SLOPE       # 3.45
WT = 0.12                       # wall thickness (cut edges)
SLAB = 0.2
CEIL_N = nrm([SLOPE, -1.0, 0.0])               # sloped ceiling normal, into the room


def gable_top(x):
    x = np.asarray(x, float)
    return np.where(x <= RIDGE_X, KNEE + (x - RX0) * SLOPE, RIDGE_H - (x - RIDGE_X) * SLOPE)


# the subject (row along z, swings along x); k = 0 (longest, red, lowest) nearest the camera
RIG_X, RIG_Z = 0.0, -0.35
SP = 0.07                                      # spacing between pendulums
TOP = 0.52                                     # pivot line (underside of the top beam)
ZK = RIG_Z + (7 - np.arange(T.K)) * SP
ZF, ZB = ZK[0] + 0.1, ZK[-1] - 0.1             # the two A-frame stands
BOB_R = 0.024
SUBJECT = np.array([RIG_X, 0.3, RIG_Z])        # locked focus point
# the low sun streams in through the round window, across the rig, toward the camera
SUN = dirv(-10, 33)
SUN_RGB = np.array([1.0, 0.77, 0.52])

# round window in the back wall: aimed so the beam bathes the whole row of bobs. Rose-window
# glazing with a clear centre (ring + spokes outside it), so no bar shadow crosses the rig.
_LIFT = np.tan(np.arcsin(SUN[1])) / abs(SUN[2] / np.hypot(SUN[0], SUN[2]))     # rise of the sun ray per unit of -z


def _wall_hit(x, y, z):
    s = (RZ0 - z) / SUN[2]
    return x + s * SUN[0], y + s * SUN[1]


_ys = [_wall_hit(RIG_X, TOP - L, z)[1] for L, z in zip(T.LEN, ZK)]
WCX = _wall_hit(RIG_X, 0.25, RIG_Z)[0]
WCY = 0.5 * (min(_ys) + max(_ys))
WR = 0.45
RING_R = 0.33
MULL_W = 0.018
BACKDROP_Z = RZ0 - 3.0


def win_open(x, y):
    return (x - WCX) ** 2 + (y - WCY) ** 2 <= WR ** 2


def win_mullion(x, y):
    dx, dy = x - WCX, y - WCY
    r = np.sqrt(dx * dx + dy * dy)
    on = np.abs(r - RING_R) < MULL_W
    for a in np.arange(8) * math.pi / 4 + math.pi / 8:            # 8 spokes from the ring out to the frame
        ca, sa = math.cos(a), math.sin(a)
        on |= (np.abs(-dx * sa + dy * ca) < MULL_W * 0.8) & (dx * ca + dy * sa > RING_R)
    on |= r > WR - 0.03
    return on & (r <= WR)


def win_lit(P, d=SUN):
    """1 where the ray from P toward the sun passes through the window glass"""
    P = np.atleast_2d(np.asarray(P, float))
    out = np.zeros(len(P), np.float32)
    ok = P[:, 2] > RZ0 - 1e-3
    lit = np.ones(len(P), bool)
    for zw in (RZ0, RZ0 - WT):                  # both faces of the wall: the reveal clips oblique rays
        t = (zw - P[:, 2]) / d[2]
        x, y = P[:, 0] + t * d[0], P[:, 1] + t * d[1]
        lit &= win_open(x, y)
        if zw == RZ0:
            lit &= ~win_mullion(x, y)
    out[ok & lit] = 1.0
    return out


# ---------------------------------------------------------------- builders
def bar(a, b, w, h=None, up=(0, 1, 0), caps=True):
    """square-ish beam between two points"""
    a, b = np.asarray(a, float), np.asarray(b, float)
    h = w if h is None else h
    ax = b - a
    ax /= np.linalg.norm(ax)
    upv = np.asarray(up, float)
    if abs(np.dot(upv, ax)) > 0.95:
        upv = np.array([1.0, 0, 0]) if abs(ax[0]) < 0.9 else np.array([0, 0, 1.0])
    s = np.cross(ax, upv)
    s /= np.linalg.norm(s)
    u = np.cross(s, ax)
    cs = [s * w / 2 + u * h / 2, -s * w / 2 + u * h / 2, -s * w / 2 - u * h / 2, s * w / 2 - u * h / 2]
    ctr = (a + b) / 2
    out = [face([a + cs[i], b + cs[i], b + cs[(i + 1) % 4], a + cs[(i + 1) % 4]], ctr) for i in range(4)]
    if caps:
        out += [face([a + c for c in cs], ctr), face([b + c for c in cs], ctr)]
    return out


def cyl_z(cx, cy, z0, z1, r, n=12, rot=0.0):
    """cylinder along z (clock faces on the back wall)"""
    out = []
    ang = [rot + 2 * math.pi * k / n for k in range(n)]
    for k in range(n):
        a0, a1 = ang[k], ang[(k + 1) % n]
        Q = np.array([[cx + r * math.cos(a0), cy + r * math.sin(a0), z0], [cx + r * math.cos(a1), cy + r * math.sin(a1), z0],
                      [cx + r * math.cos(a1), cy + r * math.sin(a1), z1], [cx + r * math.cos(a0), cy + r * math.sin(a0), z1]])
        am = 0.5 * (a0 + a1)
        out.append((Q, np.array([math.cos(am), math.sin(am), 0.0])))
    out.append((np.array([[cx + r * math.cos(a), cy + r * math.sin(a), z1] for a in ang]), np.array([0, 0, 1.0])))
    return out


def extrude_xy(outline, z0, z1):
    """prism from an xy outline (gears seen face-on), front face at z1"""
    P = [np.array(p, float) for p in outline]
    c = np.mean(P, 0)
    out = []
    for a, b in zip(P, P[1:] + P[:1]):
        out.append(face([[a[0], a[1], z0], [b[0], b[1], z0], [b[0], b[1], z1], [a[0], a[1], z1]], (c[0], c[1], (z0 + z1) / 2)))
    out.append((np.array([[p[0], p[1], z1] for p in P]), np.array([0, 0, 1.0])))
    return out


def gear_outline(r, teeth, depth=0.18):
    pts = []
    for k in range(teeth):
        a = 2 * math.pi * k / teeth
        da = 2 * math.pi / teeth
        for u, rr in ((0.0, r * (1 - depth)), (0.18, r), (0.5, r), (0.68, r * (1 - depth))):
            pts.append((rr * math.cos(a + u * da), rr * math.sin(a + u * da)))
    return pts


def xform(faces, R, t):
    R = np.asarray(R, float)
    return [(Q @ R.T + t, n @ R.T) for Q, n in faces]


def rot_x(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])


def rot_y(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def rot_z(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])


OBJS = []
FEET = []                                       # (x, z, radius, strength) for contact AO

# ---------------------------------------------------------------- the subject: the frame
RIG_G = new_group()
WOOD = (112, 72, 46)
for zf in (ZF, ZB):
    leg = []
    for sx in (-1, 1):
        leg += bar([RIG_X + sx * 0.2, 0.0, zf], [RIG_X + sx * 0.025, TOP + 0.03, zf], 0.03, 0.025, up=(0, 0, 1))
        leg += box(RIG_X + sx * 0.2, 0.012, zf, 0.035, 0.012, 0.05)
        FEET.append((RIG_X + sx * 0.2, zf, 0.03, 0.35))
    leg += bar([RIG_X - 0.17, 0.12, zf], [RIG_X + 0.17, 0.12, zf], 0.025, 0.022, up=(0, 0, 1))
    OBJS.append(Obj(leg, WOOD, 'stand', grp=RIG_G))
# top beam: drawn per slot (painter order with the bobs); one shadow caster for the whole length
BEAM_Y0, BEAM_Y1, BEAM_W = TOP, TOP + 0.036, 0.034
BEAM_SEG = []
edges = [ZF + 0.03] + [0.5 * (ZK[k] + ZK[k + 1]) for k in range(T.K - 1)] + [ZB - 0.03]
for k in range(T.K):
    BEAM_SEG.append(box(RIG_X, 0.5 * (BEAM_Y0 + BEAM_Y1), 0.5 * (edges[k] + edges[k + 1]), BEAM_W / 2, (BEAM_Y1 - BEAM_Y0) / 2,
                        abs(edges[k] - edges[k + 1]) / 2))
BEAM_OBJ = Obj([f for s in BEAM_SEG for f in s], (122, 80, 52), 'beam', grp=RIG_G)
BRASS = (226, 172, 74)
# brass hooks under the beam: the two strings of each bob meet there
STR_SPREAD = 0.026

# ---------------------------------------------------------------- the lantern-bob (local coords, origin = bob centre)
BOB_GLASS = icosa([0, 0, 0], BOB_R, (1, 1.08, 1), jit=0.04, rng=np.random.default_rng(3))
# brass cap (strings meet at its tip) and a small foot, lathed around the y axis
BOB_CAP = lathe(0, 0, [(0.0, BOB_R * 0.95), (0.006, BOB_R * 0.9), (0.012, BOB_R * 0.55), (0.02, BOB_R * 0.32), (0.026, 0.0)], 8, cy=BOB_R * 0.82)
BOB_FOOT = lathe(0, 0, [(0.0, 0.0), (0.005, BOB_R * 0.35), (0.011, BOB_R * 0.6)], 6, cy=-BOB_R * 1.18)
RAINBOW = [(255, 70, 72), (255, 112, 64), (255, 150, 56), (255, 190, 60), (250, 226, 70), (196, 236, 70), (120, 232, 96), (70, 224, 150),
           (60, 214, 214), (70, 180, 255), (88, 140, 255), (118, 110, 255), (156, 98, 255), (200, 92, 240), (240, 96, 210)]


def bob_pos(t):
    """(K,3) bob centres and (K,) swing angles"""
    th = T.theta(t)
    P = np.stack([RIG_X + T.LEN * np.sin(th), TOP - T.LEN * np.cos(th), ZK], 1)
    return P, th


def bob_faces(k, P, th):
    """glass, cap and foot of bob k, tilted with its strings (rotation about the z axis)"""
    R = rot_z(th)
    return xform(BOB_GLASS, R, P), xform(BOB_CAP + BOB_FOOT, R, P)


def string_pts(k, P, th):
    """the two strings of bob k: from the beam (z +- spread) to the cap tip"""
    tip = P + rot_z(th) @ np.array([0, BOB_R * 0.82 + 0.026, 0])
    return [(np.array([RIG_X, TOP, ZK[k] + s * STR_SPREAD]), tip) for s in (-1, 1)]


# ---------------------------------------------------------------- workbench under the slope (left)
BENCH_G = new_group()
BX0, BX1, BZ0, BZ1, BY = -1.66, -1.18, -1.62, -0.42, 0.7
bench = box(0.5 * (BX0 + BX1), BY - 0.03, 0.5 * (BZ0 + BZ1), (BX1 - BX0) / 2, 0.03, (BZ1 - BZ0) / 2)
for x in (BX0 + 0.04, BX1 - 0.04):
    for z in (BZ0 + 0.05, BZ1 - 0.05):
        bench += box(x, (BY - 0.06) / 2, z, 0.025, (BY - 0.06) / 2, 0.025)
        FEET.append((x, z, 0.03, 0.4))
bench += box(0.5 * (BX0 + BX1), 0.12, 0.5 * (BZ0 + BZ1), (BX1 - BX0) / 2 - 0.04, 0.012, (BZ1 - BZ0) / 2 - 0.05)   # low shelf
OBJS.append(Obj(bench, (128, 82, 50), 'bench', grp=BENCH_G))
OBJS.append(Obj(box(BX1 - 0.002, BY - 0.12, -1.02, 0.004, 0.05, 0.16), (110, 70, 44), 'drawer', grp=BENCH_G))
OBJS.append(Obj(box(BX1 + 0.004, BY - 0.12, -1.02, 0.006, 0.01, 0.03), BRASS, 'knob', grp=BENCH_G))
# oil lamp on the bench
LAMP1 = np.array([-1.42, BY + 0.2, -0.62])
g = new_group()
OBJS.append(Obj(lathe(LAMP1[0], LAMP1[2], [(0.0, 0.055), (0.03, 0.06), (0.06, 0.04), (0.075, 0.02), (0.09, 0.025)], 8, cy=BY), BRASS, 'lampbase', grp=g))
OBJS.append(Obj(lathe(LAMP1[0], LAMP1[2], [(0.0, 0.03), (0.04, 0.05), (0.1, 0.045), (0.14, 0.028), (0.17, 0.03)], 8, cy=BY + 0.09), (255, 214, 150), 'chimney', grp=g, cast=False))
# gears + a clock movement on the bench (gears lie flat)
for (x, z, r, n, c) in [(-1.5, -1.25, 0.09, 14, BRASS), (-1.33, -1.38, 0.06, 10, (200, 150, 70)), (-1.55, -0.95, 0.05, 9, (190, 196, 200))]:
    gf = extrude_xy(gear_outline(r, n), 0, 0.012)
    OBJS.append(Obj(xform(gf, rot_x(-math.pi / 2), [x, BY, z]), c, 'gear', grp=new_group()))
OBJS.append(Obj(box(-1.36, BY + 0.06, -0.92, 0.07, 0.06, 0.05), (90, 96, 104), 'vise', grp=new_group()))
OBJS.append(Obj(lathe(-1.58, -0.5, [(0.0, 0.04), (0.11, 0.045), (0.12, 0.05)], 7, cy=BY), (120, 170, 160), 'jar', grp=new_group()))
OBJS.append(Obj(box(-1.585, BY + 0.17, -0.5, 0.004, 0.06, 0.004), (180, 180, 190), 'screwdriver', grp=new_group(), cast=False))
# big gear leaning on the knee wall, front left
gf = extrude_xy(gear_outline(0.26, 16, 0.14), 0, 0.03)
OBJS.append(Obj(xform(gf, rot_z(0.15) @ rot_y(math.pi / 2) @ rot_z(0.2), [RX0 + 0.05, 0.255, 0.6]), (176, 128, 60), 'biggear', grp=new_group()))
FEET.append((RX0 + 0.12, 0.6, 0.06, 0.35))

# ---------------------------------------------------------------- grandfather clock (back right)
GC_X, GC_Z = 1.2, RZ0 + 0.16
GC_G = new_group()
gc = box(GC_X, 0.05, GC_Z, 0.22, 0.05, 0.16)                      # plinth
gc += box(GC_X, 0.7, GC_Z, 0.17, 0.6, 0.12)                       # trunk
gc += box(GC_X, 1.55, GC_Z, 0.22, 0.26, 0.15)                     # hood
gc += box(GC_X, 1.85, GC_Z, 0.24, 0.04, 0.17)                     # cornice
gc += [face([[GC_X - 0.2, 1.89, GC_Z + 0.16], [GC_X + 0.2, 1.89, GC_Z + 0.16], [GC_X, 2.0, GC_Z + 0.16]], [GC_X, 1.9, GC_Z])]
OBJS.append(Obj(gc, (104, 54, 36), 'gclock', grp=GC_G))
GC_DIAL = np.array([GC_X, 1.56, GC_Z + 0.152])
OBJS.append(Obj(cyl_z(GC_DIAL[0], GC_DIAL[1], GC_Z + 0.15, GC_DIAL[2], 0.15, 16), (236, 222, 190), 'dial', grp=GC_G))
GC_WIN = (GC_X - 0.1, GC_X + 0.1, 0.42, 1.18, GC_Z + 0.122)       # glass in the trunk door
GC_PIVOT = np.array([GC_X, 1.2, GC_Z + 0.1])
FEET.append((GC_X, GC_Z, 0.2, 0.45))

# ---------------------------------------------------------------- wall clocks, shelf (back wall)
WALL_CLOCKS = [  # (centre, radius, case colour, pendulum period or 0)
    (np.array([0.58, 1.86, RZ0 + 0.05]), 0.15, (120, 64, 40), 0.0),
    (np.array([-1.0, 1.22, RZ0 + 0.05]), 0.1, (60, 92, 96), 0.0),
    (np.array([0.62, 1.32, RZ0 + 0.07]), 0.085, (150, 90, 52), 1.0),   # box clock with a 1 s pendulum below
]
for c, r, col, per in WALL_CLOCKS:
    g = new_group()
    if per:
        OBJS.append(Obj(box(c[0], c[1] - 0.03, RZ0 + 0.04, r + 0.03, r + 0.06, 0.04), col, 'boxclock', grp=g))
        OBJS.append(Obj(box(c[0], c[1] - r - 0.2, RZ0 + 0.03, 0.05, 0.12, 0.025), (80, 50, 34), 'pendcase', grp=g))
        OBJS.append(Obj(cyl_z(c[0], c[1], RZ0 + 0.08, RZ0 + 0.085, r * 0.85, 12), (240, 228, 196), 'dial', grp=g))
    else:
        OBJS.append(Obj(cyl_z(c[0], c[1], RZ0, c[2], r, 14), col, 'wclock', grp=g))
        OBJS.append(Obj(cyl_z(c[0], c[1], c[2], c[2] + 0.006, r * 0.84, 14), (240, 228, 196), 'dial', grp=g))
SHELF_Y = 0.92
g = new_group()
OBJS.append(Obj(box(0.6, SHELF_Y, RZ0 + 0.12, 0.3, 0.015, 0.12) + box(0.36, SHELF_Y - 0.08, RZ0 + 0.04, 0.015, 0.06, 0.04) +
                box(0.84, SHELF_Y - 0.08, RZ0 + 0.04, 0.015, 0.06, 0.04), (136, 88, 54), 'shelf', grp=g))
for (x, h, r, col) in [(0.38, 0.12, 0.04, (90, 150, 170)), (0.48, 0.08, 0.035, (200, 120, 70)), (0.74, 0.1, 0.03, (150, 180, 110))]:
    OBJS.append(Obj(lathe(x, RZ0 + 0.12, [(0.0, r), (h * 0.8, r), (h, r * 0.6), (h + 0.01, r * 0.7)], 7, cy=SHELF_Y + 0.015), col, 'jar', grp=new_group()))
for i, (w, h, col) in enumerate([(0.025, 0.16, (160, 52, 48)), (0.03, 0.14, (52, 90, 140)), (0.022, 0.17, (196, 160, 70))]):
    OBJS.append(Obj(box(0.6 + i * 0.03, SHELF_Y + 0.015 + h / 2, RZ0 + 0.1, w / 2 - 0.002, h / 2, 0.06), col, 'book', grp=new_group()))

# ---------------------------------------------------------------- lamps (warm local lights)
LAMP2 = np.array([-0.62, 1.72, 0.42])                     # hanging oil lamp, front left
CEIL_AT = lambda x: KNEE + (x - RX0) * SLOPE
g = new_group()
OBJS.append(Obj(lathe(LAMP2[0], LAMP2[2], [(0.0, 0.0), (0.02, 0.045), (0.05, 0.05), (0.06, 0.03)], 8, cy=LAMP2[1] - 0.12), BRASS, 'lamp2base', grp=g))
OBJS.append(Obj(lathe(LAMP2[0], LAMP2[2], [(0.0, 0.035), (0.05, 0.05), (0.11, 0.04), (0.14, 0.025)], 8, cy=LAMP2[1] - 0.06), (255, 214, 150), 'chimney', grp=g, cast=False))
OBJS.append(Obj(lathe(LAMP2[0], LAMP2[2], [(0.0, 0.09), (0.03, 0.06), (0.05, 0.02)], 8, cy=LAMP2[1] + 0.08), (60, 70, 66), 'shade', grp=g))
CHAIN = (LAMP2 + [0, 0.13, 0], np.array([LAMP2[0], CEIL_AT(LAMP2[0]) - 0.02, LAMP2[2]]))
LAMP3 = np.array([0.66, 0.6, -0.86])                      # lantern on a stool, behind right of the rig
g = new_group()
st = prism(LAMP3[0], LAMP3[2], 0.13, 0.4, 0.44, 10)
for a in (0.3, 2.4, 4.5):
    st += bar([LAMP3[0] + 0.11 * math.cos(a), 0.0, LAMP3[2] + 0.11 * math.sin(a)], [LAMP3[0] + 0.08 * math.cos(a), 0.4, LAMP3[2] + 0.08 * math.sin(a)], 0.025, up=(0, 0, 1))
OBJS.append(Obj(st, (118, 74, 46), 'stool', grp=g))
FEET.append((LAMP3[0], LAMP3[2], 0.12, 0.35))
g = new_group()
lant = prism(LAMP3[0], LAMP3[2], 0.06, 0.44, 0.47, 4, rot=0.4) + prism(LAMP3[0], LAMP3[2], 0.065, 0.62, 0.65, 4, rot=0.4)
lant += prism(LAMP3[0], LAMP3[2], 0.04, 0.65, 0.69, 4, r1=0.0, rot=0.4)
OBJS.append(Obj(lant, (52, 48, 46), 'lantern', grp=g))
OBJS.append(Obj(prism(LAMP3[0], LAMP3[2], 0.052, 0.47, 0.62, 4, rot=0.4), (255, 206, 130), 'chimney', grp=g, cast=False))
LAMPS = [  # (light position, radius, strength)
    (LAMP1 + [0, 0.05, 0], 1.6, 0.95),
    (LAMP2 - [0, 0.02, 0], 1.9, 1.05),
    (LAMP3 + [0, -0.06, 0], 1.3, 0.9),
]

# ---------------------------------------------------------------- crates + clutter (front right)
g = new_group()
OBJS.append(Obj(box(1.2, 0.18, 0.78, 0.22, 0.18, 0.2, rot=0.15), (156, 116, 72), 'crate', grp=g))
OBJS.append(Obj(box(1.18, 0.36 + 0.11, 0.74, 0.14, 0.11, 0.13, rot=-0.35), (176, 132, 82), 'crate2', grp=new_group()))
for dy in (0.06, 0.3):
    OBJS.append(Obj(box(1.2, dy, 0.78, 0.226, 0.012, 0.206, rot=0.15), (120, 84, 52), 'slat', grp=g, cast=False))
FEET.append((1.2, 0.78, 0.22, 0.4))
g = new_group()
OBJS.append(Obj(box(1.38, 0.035, 0.3, 0.13, 0.035, 0.09, rot=0.6) + box(1.37, 0.09, 0.3, 0.11, 0.02, 0.08, rot=0.4), (110, 60, 50), 'books', grp=g))
OBJS.append(Obj(prism(-1.2, 0.95, 0.12, 0.0, 0.28, 9), (112, 120, 96), 'barrel', grp=new_group()))
OBJS.append(Obj(prism(-1.2, 0.95, 0.125, 0.07, 0.09, 9) + prism(-1.2, 0.95, 0.125, 0.2, 0.22, 9), (80, 80, 84), 'hoops', grp=new_group(), cast=False))
FEET.append((-1.2, 0.95, 0.12, 0.35))

# ---------------------------------------------------------------- rafters on the sloped ceiling
RAFTER_Z = list(np.arange(RZ0 + 0.06, RZ1, 0.62)) + [RZ1 - 0.04]
RAFT_G = new_group()
for z in RAFTER_Z:
    off = -CEIL_N * 0.05
    OBJS.append(Obj(bar([RX0 + off[0], KNEE + off[1], z], [RIDGE_X + off[0], RIDGE_H + off[1], z], 0.07, 0.1, up=(0, 0, 1)),
                    (112, 74, 46), 'rafter', grp=RAFT_G, cast=False))

# ---------------------------------------------------------------- the cat (watches the bobs)
CAT_P = np.array([-0.46, 0.0, 0.28])
CAT_YAW = math.atan2(RIG_Z - CAT_P[2] + 0.2, RIG_X - CAT_P[0])      # faces the rig
FEET.append((CAT_P[0], CAT_P[2], 0.08, 0.4))


def cat_faces(t, look_x):
    """sitting cat; head follows a bob's swing (look_x = bob x offset), slow tail flick, blinks"""
    r2 = np.random.default_rng(12)
    c = CAT_P
    loc = []
    loc.append(('body', icosa([0, 0.1, 0], 0.095, (1.25, 1.05, 0.85), jit=0.07, rng=r2)))
    loc.append(('body', icosa([0.06, 0.17, 0], 0.07, (0.9, 1.15, 0.85), jit=0.05, rng=r2)))
    for sz in (-1, 1):
        loc.append(('body', icosa([0.11, 0.02, 0.04 * sz], 0.028, (1.3, 0.8, 0.9), jit=0.05, rng=r2)))
    hc = np.array([0.09, 0.28, 0.0])
    head = icosa(hc, 0.058, (1.0, 0.92, 1.08), jit=0.05, rng=r2)
    ears = []
    for sz in (-1, 1):
        ears.append(face([hc + [-0.02, 0.035, 0.018 * sz], hc + [0.02, 0.035, 0.04 * sz], hc + [0.0, 0.095, 0.034 * sz]], hc + [-1, 0, 0]))
    snout = icosa(hc + [0.052, -0.012, 0], 0.022, (0.8, 0.7, 1.0), jit=0.03, rng=r2)
    yaw = 0.55 * look_x / T.AMP
    tilt = 0.12 * math.sin(2 * math.pi * 2 * t / 60.0)
    R = rot_y(-yaw) @ rot_x(tilt)
    hf = [((Q - hc) @ R.T + hc, n @ R.T) for Q, n in head + ears + snout]
    loc.append(('head', hf))
    # tail curls round the feet, its tip flicks (3 slow flicks per minute: loop-safe)
    tail = []
    p = np.array([-0.11, 0.03, 0.0])
    fl = math.sin(2 * math.pi * 3 * t / 60.0) ** 9
    for k in range(8):
        a = 0.5 + k * 0.42
        q = np.array([-0.11 + 0.14 * math.sin(a) * 0.8, 0.012 + (0.03 * fl * k / 7) * (k > 4) * 2, 0.11 * (1 - math.cos(a)) * 0.9])
        w = 0.022 - 0.0018 * k
        tail.append(face([p + [0, 0.01, -w], p + [0, 0.01, w], q + [0, 0.01, w], q + [0, 0.01, -w]], p + [0, -1, 0]))
        tail.append(face([p + [0, w * 1.2, 0], p + [0, 0, 0], q + [0, 0, 0], q + [0, w * 1.2, 0]], p + [0, 0, -1]))
        p = q
    loc.append(('tail', tail))
    Ry = rot_y(-CAT_YAW)
    out = [(name, [(Q @ Ry.T + c, n @ Ry.T) for Q, n in fcs]) for name, fcs in loc]
    blink = (t % 7.5) < 0.12            # 8 blinks per minute
    eyes = [Ry @ (hc + R @ np.array([0.05, 0.012, 0.024 * sz])) + c for sz in (-1, 1)]
    return out, eyes, blink


# ---------------------------------------------------------------- dust motes in the sunbeam
MOTES = np.random.default_rng(5).uniform([-0.9, 0.05, -1.85], [0.6, 1.7, 0.6], (160, 3))
