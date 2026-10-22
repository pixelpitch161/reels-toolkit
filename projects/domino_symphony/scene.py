"""The gym: room shell (ray-traced per pixel in render.py), the late-afternoon sun through tall windows in the
left wall, the maple floor (planks, court lines, tape marks) and the low-poly props (plank ledge + post, work
lamps, spare-domino boxes, bleachers, hoop, robot vacuum).

The windows decide where the sun lands: win_lit(P) is 1 when the ray from P toward the sun leaves through a
window opening (so floor patches, wall patches, beams and the dominoes all agree)."""
import math
import numpy as np
from reelkit.config import dirv
from reelkit.geometry import Obj, box, prism, lathe, icosa, new_group, face
import layout as LY

# ---------------------------------------------------------------- room
RX0, RX1 = -7.0, 7.0          # left (window) wall, right (bleacher) wall
RZ0, RZ1 = -13.0, 13.0        # far wall (hoop), near wall
RH = 9.0                      # ceiling
SUN = dirv(-72, 16)           # toward the sun: from the left wall, a little toward the far end; low = long shadows
WIN_Y0, WIN_Y1 = 1.40, 6.50   # window openings in the left wall
WIN_MULL_Y = 4.80             # one horizontal mullion (its shadow lands outside the layout)
BAY_PITCH, BAY_OPEN = 3.9, 3.3
BAY_C = -4.19 + BAY_PITCH * np.arange(-2, 5)        # opening centres along z
PAD_H = 1.30                  # blue wall pads


def win_open(z, y):
    """inside a window opening (left wall coordinates z, y)"""
    m = np.zeros(np.shape(z), bool)
    for c in BAY_C:
        m |= np.abs(z - c) < BAY_OPEN / 2
    m &= (y > WIN_Y0) & (y < WIN_Y1) & (np.abs(y - WIN_MULL_Y) > 0.06)
    return m


def win_frame(z, y):
    """window frame / mullion pixels (for drawing the wall)"""
    inside = np.zeros(np.shape(z), bool)
    for c in BAY_C:
        inside |= np.abs(z - c) < BAY_OPEN / 2 + 0.07
    inside &= (y > WIN_Y0 - 0.07) & (y < WIN_Y1 + 0.07)
    return inside & ~win_open(z, y)


def win_lit(X, Y, Z):
    """1 where the ray toward the sun from (X, Y, Z) passes through a window"""
    t = (RX0 - np.asarray(X, float)) / SUN[0]
    y = np.asarray(Y, float) + t * SUN[1]
    z = np.asarray(Z, float) + t * SUN[2]
    return (win_open(z, y) & (t > 0)).astype(np.float32)


# ---------------------------------------------------------------- floor albedo
MAPLE = np.array([206, 158, 104], float)
_r = np.random.default_rng(31)
PLANK_W = 0.075
N_PL = int((RX1 - RX0) / PLANK_W) + 2
PL_OFF = _r.uniform(0, 3.0, N_PL)
PL_LEN = _r.uniform(1.4, 2.8, N_PL)
TONE = _r.uniform(0.9, 1.08, (N_PL, 64))


def floor_albedo(X, Z):
    i = np.clip(((X - RX0) / PLANK_W).astype(int), 0, N_PL - 1)
    j = np.floor((Z + 20 + PL_OFF[i]) / PL_LEN[i]).astype(int) % 64
    col = MAPLE[None] * TONE[i, j][:, None]
    # plank seams
    fx = ((X - RX0) / PLANK_W) % 1.0
    fz = ((Z + 20 + PL_OFF[i]) / PL_LEN[i]) % 1.0
    seam = (fx < 0.05) | (fz < 0.004)
    col = np.where(seam[:, None], col * 0.82, col)
    # grain streaks
    g = np.sin(Z * 37.0 + i * 1.7) * np.sin(Z * 11.0 + i * 0.3)
    col = col * (1 + 0.035 * g)[:, None]
    # court lines (white 5 cm): centre line, centre circle, side lines, end lines, keys
    r = np.hypot(X, Z)
    line = (np.abs(Z) < 0.025) | (np.abs(r - 1.8) < 0.025) | (np.abs(np.abs(X) - 6.4) < 0.025) | (np.abs(np.abs(Z) - 12.4) < 0.025)
    key = (np.abs(X) < 2.45) & (np.abs(Z) > 6.6) & (np.abs(Z) < 12.4)
    keyedge = key & ((np.abs(np.abs(X) - 2.45) < 0.05) | (np.abs(np.abs(Z) - 6.6) < 0.025))
    col = np.where(key[:, None], col * np.array([0.78, 0.72, 0.95]) + np.array([10, 18, 60]), col)
    col = np.where((line | keyedge)[:, None], np.array([236, 232, 222]), col)
    # tape marks (blue painter's tape crosses + strips)
    for (tx, tz, a) in TAPE:
        ca, sa = math.cos(a), math.sin(a)
        u = (X - tx) * ca + (Z - tz) * sa
        v = -(X - tx) * sa + (Z - tz) * ca
        m = ((np.abs(u) < 0.12) & (np.abs(v) < 0.022)) | ((np.abs(v) < 0.12) & (np.abs(u) < 0.022))
        col = np.where(m[:, None], np.array([70, 140, 230]), col)
    return col


TAPE = [(-1.25, 5.6, 0.3), (1.3, 5.7, -0.2), (-0.95, 1.2, 0.1), (1.05, -0.2, 0.5), (-1.1, -3.3, 0.2), (1.05, -4.6, -0.3),
        (-0.6, -5.7, 0.4), (0.7, 2.6, 0.2)]


# ---------------------------------------------------------------- props
PROPS = []
rng = np.random.default_rng(12)

# the ledge: a round post at the spiral centre carrying a plank out over the spiral
C_S = LY.C_S
PL = LY.PLANK
LEDGE_G = new_group()
PROPS.append(Obj(prism(C_S[0], C_S[1], LY.POST_R, 0.0, PL['y'] - 0.015, 10), (150, 104, 66), 'post', grp=LEDGE_G))
PROPS.append(Obj(box(C_S[0], PL['y'] - 0.0075, 0.5 * (PL['z0'] + PL['z1']), PL['w'] / 2, 0.0075, (PL['z0'] - PL['z1']) / 2),
                 (196, 146, 92), 'plank', grp=LEDGE_G))
PROPS.append(Obj(box(C_S[0], (PL['y'] - 0.015) / 2, PL['z1'] + 0.035, 0.055, (PL['y'] - 0.015) / 2, 0.03),
                 (150, 104, 66), 'post', grp=LEDGE_G))

# work lamps (tripod + head), warm local lights
LAMPS = [np.array([-2.25, 1.15, 4.55]), np.array([2.35, 1.2, -1.35]), np.array([2.3, 1.1, -4.6]), np.array([2.3, 1.15, 3.1])]
LAMP_AIM = []
for k, lp in enumerate(LAMPS):
    g = new_group()
    aim = np.array([0.0, 0.0, lp[2]]) - lp
    aim /= np.linalg.norm(aim)
    LAMP_AIM.append(aim)
    for a in range(3):
        ang = a * 2.094 + 0.4
        foot = np.array([lp[0] + 0.32 * math.cos(ang), 0.0, lp[2] + 0.32 * math.sin(ang)])
        top = lp + np.array([0, -0.45, 0])
        d = top - foot
        n = np.cross(d, [0, 1, 0])
        n = n / (np.linalg.norm(n) + 1e-9) * 0.012
        PROPS.append(Obj([face([foot + n, top + n, top - n, foot - n], foot + np.cross(n, d))], (60, 60, 66), 'leg', grp=g, double=True))
    PROPS.append(Obj(prism(lp[0], lp[2], 0.018, lp[1] - 0.45, lp[1] - 0.06, 6), (70, 70, 78), 'pole', grp=g))
    yaw = math.atan2(aim[0], aim[2])
    PROPS.append(Obj(box(lp[0], lp[1], lp[2], 0.13, 0.09, 0.05, rot=-yaw + math.pi / 2), (230, 196, 40), 'lamphead', grp=g))
    PROPS.append(Obj(box(lp[0] + aim[0] * 0.052, lp[1], lp[2] + aim[2] * 0.052, 0.11, 0.07, 0.006, rot=-yaw + math.pi / 2),
                     (255, 236, 190), 'lamplens', grp=g, cast=False))

# spare-domino boxes (open cardboard boxes) outside the frame
CRATES = [(-2.35, 6.4, 0.25), (2.45, 5.55, -0.4), (2.5, -5.4, 0.15), (-2.4, 0.4, 0.5)]
for (x, z, rot) in CRATES:
    g = new_group()
    w, d, h = 0.22, 0.16, 0.14
    c, s = math.cos(rot), math.sin(rot)
    for (ox, oz, hx, hz) in ((0, d, w, 0.006), (0, -d, w, 0.006), (w, 0, 0.006, d), (-w, 0, 0.006, d)):
        PROPS.append(Obj(box(x + ox * c - oz * s, h / 2, z + ox * s + oz * c, hx, h / 2, hz, rot=rot), (196, 150, 96), 'crate', grp=g))
    PROPS.append(Obj(box(x, 0.01, z, w, 0.01, d, rot=rot), (176, 132, 84), 'crate', grp=g))
    for k in range(9):
        px, pz = rng.uniform(-w + 0.03, w - 0.03), rng.uniform(-d + 0.03, d - 0.03)
        col = LY.hsv(rng.random(), 0.75, 1.0)
        PROPS.append(Obj(box(x + px * c - pz * s, h * 0.75 + rng.uniform(0, 0.02), z + px * s + pz * c, 0.0125, 0.025, 0.004,
                             rot=rng.uniform(0, 3)), col, 'spare', grp=g, cast=False))
    # a short leaning row of spares beside the box
    for k in range(5):
        PROPS.append(Obj(box(x + (w + 0.06 + k * 0.035) * c, 0.022, z + (w + 0.06 + k * 0.035) * s, 0.004, 0.022, 0.0125, rot=rot),
                         LY.hsv(0.1 * k + rot, 0.7, 1.0), 'spare', grp=g))

# folded bleachers along the right wall
for k in range(5):
    g = new_group()
    PROPS.append(Obj(box(RX1 - 0.35 - k * 0.32, 0.25 + k * 0.42, -1.0, 0.18, 0.25 + k * 0.42, 9.5),
                     (170, 120, 74) if k % 2 else (186, 134, 84), 'bleacher', grp=g))
# basketball hoop on the far wall
g = new_group()
PROPS.append(Obj(box(0.0, 3.35, RZ0 + 0.35, 0.9, 0.53, 0.02), (240, 240, 236), 'board', grp=g))
PROPS.append(Obj(box(0.0, 3.1, RZ0 + 0.37, 0.3, 0.2, 0.006), (210, 60, 40), 'boardsq', grp=g, cast=False))
for k in range(10):
    a0, a1 = k * 0.628, (k + 1) * 0.628
    p0 = np.array([0.23 * math.cos(a0), 3.05, RZ0 + 0.6 + 0.23 * math.sin(a0)])
    p1 = np.array([0.23 * math.cos(a1), 3.05, RZ0 + 0.6 + 0.23 * math.sin(a1)])
    PROPS.append(Obj([face([p0, p1, p1 + [0, 0.02, 0], p0 + [0, 0.02, 0]], [0, 3.05, RZ0 + 0.6])], (240, 110, 40), 'rim', grp=g, double=True))
PROPS.append(Obj(box(0.0, 3.06, RZ0 + 0.3, 0.05, 0.02, 0.06), (240, 110, 40), 'rim', grp=g))


# ---------------------------------------------------------------- robot vacuum (small life, outside the frame)
def vacuum_pos(t):
    """loops a long rounded track on the right, outside the frame"""
    per = 52.0
    u = (t % per) / per
    L1 = 11.0
    xr, z0, z1 = 3.1, -5.5, 5.5
    s = u * (2 * L1 + 2 * math.pi * 0.5)
    if s < L1:
        return np.array([xr, z1 - s]), math.pi
    s -= L1
    if s < math.pi * 0.5:
        a = s / 0.5
        return np.array([xr + 0.5 - 0.5 * math.cos(a), z0 - 0.5 * math.sin(a)]), math.pi + a
    s -= math.pi * 0.5
    if s < L1:
        return np.array([xr + 1.0, z0 + s]), 0.0
    s -= L1
    a = s / 0.5
    return np.array([xr + 0.5 + 0.5 * math.cos(a), z1 + 0.5 * math.sin(a)]), a


def vacuum_faces(t):
    p, yaw = vacuum_pos(t)
    f = prism(p[0], p[1], 0.17, 0.0, 0.08, 12) + prism(p[0], p[1], 0.06, 0.08, 0.09, 8)
    return f, p
