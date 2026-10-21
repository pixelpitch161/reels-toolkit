"""Static geometry + builders for the moving parts of the Viennese salon diorama.

A cut-away room (back wall + left wall) floating in a night-blue void:
- left wall: arched window (moon key light, cat on the sill, velvet curtains), wall calendar
- back wall: the giant parchment staff (bars land on it), the sum histogram, two candle sconces
- floor: parquet, an oval rug, the round gueridon with the dice tray (the SUBJECT), the 1792 table
  as a parchment chart that unrolls across the floor, a self-playing fortepiano, a chair,
  a standing candelabra and a potted palm in the foreground

The room shell (floor + walls + window backdrop) is ray-traced per pixel in render.py, so here it
is only described by planes/extents; everything else is low-poly faces (reelkit.geometry)."""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, new_group

rng = np.random.default_rng(7)

# ---------------------------------------------------------------- room shell
RX0, RX1, RZ0, RZ1 = -1.7, 1.7, -1.7, 1.7
WALL_H = 3.0
WT = 0.12                       # wall thickness (visible at the cut edges)
SLAB = 0.22                     # floor slab thickness
RAIL_Y = 0.82                   # chair rail height (wainscot below)
CROWN_Y = 2.86

# arched window in the left wall (x = RX0)
WIN_Z0, WIN_Z1, WIN_Y0, WIN_Y1 = -1.25, -0.35, 0.92, 2.15
WIN_ZC = 0.5 * (WIN_Z0 + WIN_Z1)
WIN_R = 0.5 * (WIN_Z1 - WIN_Z0)
MULL_W = 0.022
BACKDROP_X = -4.2               # painted night sky + Vienna roofs behind the window


def win_open(y, z):
    """inside the arched window opening (y, z on the left-wall plane)"""
    rect = (z >= WIN_Z0) & (z <= WIN_Z1) & (y >= WIN_Y0) & (y <= WIN_Y1)
    arch = ((y - WIN_Y1) ** 2 + (z - WIN_ZC) ** 2 <= WIN_R ** 2) & (y > WIN_Y1)
    return rect | arch


def win_mullion(y, z):
    on = np.abs(z - WIN_ZC) < MULL_W
    on |= (np.abs(y - 1.52) < MULL_W) & (y < WIN_Y1 + 0.01)
    on |= np.abs(y - WIN_Y1) < MULL_W
    r = np.sqrt((y - WIN_Y1) ** 2 + (z - WIN_ZC) ** 2)
    on |= (y > WIN_Y1) & (np.abs(r - WIN_R * 0.55) < MULL_W * 0.9)
    edge = win_open(y, z) & ~win_open(y + 0.035, z) | (np.abs(z - WIN_Z0) < 0.03) | (np.abs(z - WIN_Z1) < 0.03) | (np.abs(y - WIN_Y0) < 0.03)
    return on | edge


def win_lit(P, d):
    """1 where the ray from P toward the moon (direction d) passes through the window glass"""
    P = np.atleast_2d(np.asarray(P, float))
    out = np.zeros(len(P), np.float32)
    if d[0] >= -1e-3:
        return out
    ok = P[:, 0] > RX0 - 1e-3
    lit = np.ones(len(P), bool)
    for xw in (RX0, RX0 - WT):                       # both faces of the wall: the reveal clips oblique rays
        t = (xw - P[:, 0]) / d[0]
        y, z = P[:, 1] + t * d[1], P[:, 2] + t * d[2]
        lit &= win_open(y, z)
        if xw == RX0:
            lit &= ~win_mullion(y, z)
    out[ok & lit] = 1.0
    return out


# ---------------------------------------------------------------- builders
def extrude(outline, y0, y1, bottom=False):
    """vertical prism from an xz outline (counter-clockwise seen from above)"""
    out = []
    P = [np.array(p, float) for p in outline]
    ctr = np.mean(P, 0)
    for a, b in zip(P, P[1:] + P[:1]):
        out.append(face([[a[0], y0, a[1]], [b[0], y0, b[1]], [b[0], y1, b[1]], [a[0], y1, a[1]]], (ctr[0], (y0 + y1) / 2, ctr[1])))
    out.append(face([[p[0], y1, p[1]] for p in P[::-1]], up=True))
    if bottom:
        Q, n = face([[p[0], y0, p[1]] for p in P])
        out.append((Q, -np.abs(n) * np.array([0, 1, 0]) if n[1] > 0 else n))
    return out


def cyl_x(x0, x1, y, z, r, n=8):
    """cylinder lying along x (scroll rods, the chart roll)"""
    faces = []
    ang = [2 * math.pi * k / n for k in range(n)]
    for k in range(n):
        a0, a1 = ang[k], ang[(k + 1) % n]
        Q = np.array([[x0, y + r * math.sin(a0), z + r * math.cos(a0)], [x1, y + r * math.sin(a0), z + r * math.cos(a0)],
                      [x1, y + r * math.sin(a1), z + r * math.cos(a1)], [x0, y + r * math.sin(a1), z + r * math.cos(a1)]])
        am = 0.5 * (a0 + a1)
        faces.append((Q, np.array([0, math.sin(am), math.cos(am)])))
    for xe, sgn in ((x0, -1), (x1, 1)):
        faces.append((np.array([[xe, y + r * math.sin(a), z + r * math.cos(a)] for a in ang]), np.array([sgn, 0, 0.0])))
    return faces


def rot_y(faces, ang, cx, cz):
    c, s = math.cos(ang), math.sin(ang)
    R = np.array([[c, 0, -s], [0, 1, 0], [s, 0, c]])
    o = np.array([cx, 0, cz])
    return [((Q - o) @ R.T + o, n @ R.T) for Q, n in faces]


def transform(faces, R, t):
    return [(Q @ R.T + t, n @ R.T) for Q, n in faces]


OBJS = []

# ---------------------------------------------------------------- the subject: gueridon + dice tray
TABLE_C = np.array([-0.05, 0.0, -0.42])
TOP_Y = 0.78
TRAY_C = np.array([TABLE_C[0], TOP_Y + 0.012, TABLE_C[2]])
TRAY_HX, TRAY_HZ = 0.34, 0.23
FELT_Y = TRAY_C[1] + 0.012
SUBJECT = TRAY_C + np.array([0.0, 0.06, 0.0])          # focus point (locked)
g = new_group()
TABLE_G = g
OBJS.append(Obj(prism(TABLE_C[0], TABLE_C[2], 0.47, TOP_Y - 0.035, TOP_Y, 14, rot=0.1), (92, 46, 32), 'tabletop', grp=g))
OBJS.append(Obj(prism(TABLE_C[0], TABLE_C[2], 0.43, TOP_Y - 0.07, TOP_Y - 0.035, 14, rot=0.1), (76, 38, 28), 'apron', grp=g))
OBJS.append(Obj(lathe(TABLE_C[0], TABLE_C[2], [(0.08, 0.05), (0.2, 0.065), (0.32, 0.04), (0.5, 0.045), (0.6, 0.07), (0.66, 0.05), (0.71, 0.06)], 8),
                (84, 42, 30), 'pedestal', grp=g))
for k in range(3):
    a = 0.4 + k * 2.094
    ft = []
    x1, z1 = TABLE_C[0] + 0.32 * math.cos(a), TABLE_C[2] + 0.32 * math.sin(a)
    ft += box(0.5 * (TABLE_C[0] + x1), 0.06, 0.5 * (TABLE_C[2] + z1), 0.17, 0.03, 0.035, rot=-a)
    ft += prism(x1, z1, 0.04, 0.0, 0.04, 6)
    OBJS.append(Obj(ft, (80, 40, 28), 'foot', grp=g))
gt = new_group()
TRAY_G = gt
tray = []
for sx in (-1, 1):
    tray += box(TRAY_C[0] + sx * (TRAY_HX - 0.015), TRAY_C[1] + 0.02, TRAY_C[2], 0.015, 0.022, TRAY_HZ)
for sz in (-1, 1):
    tray += box(TRAY_C[0], TRAY_C[1] + 0.02, TRAY_C[2] + sz * (TRAY_HZ - 0.015), TRAY_HX, 0.022, 0.015)
OBJS.append(Obj(tray, (150, 96, 50), 'tray', grp=gt))
OBJS.append(Obj([face([[TRAY_C[0] - TRAY_HX + 0.03, FELT_Y, TRAY_C[2] - TRAY_HZ + 0.03], [TRAY_C[0] - TRAY_HX + 0.03, FELT_Y, TRAY_C[2] + TRAY_HZ - 0.03],
                       [TRAY_C[0] + TRAY_HX - 0.03, FELT_Y, TRAY_C[2] + TRAY_HZ - 0.03], [TRAY_C[0] + TRAY_HX - 0.03, FELT_Y, TRAY_C[2] - TRAY_HZ + 0.03]], up=True)],
                (38, 112, 74), 'felt', grp=gt, cast=False, jitter=0.0))
CUP_HOME = TRAY_C + np.array([TRAY_HX + 0.1, -0.012, 0.06])     # cup A rests on the tabletop right of the tray

# ---------------------------------------------------------------- fortepiano (back right, keyboard toward the camera)
PIANO_KX0, PIANO_KX1, PIANO_KZ = 0.66, 1.52, -0.36
pg = new_group()
PIANO_G = pg
outline = [(0.62, -0.36), (1.62, -0.36), (1.62, -1.66), (1.3, -1.66), (1.05, -1.42), (0.8, -1.05), (0.66, -0.7)]
OBJS.append(Obj(extrude(outline, 0.58, 0.86), (54, 26, 22), 'piano_case', grp=pg))
OBJS.append(Obj(extrude([(x, z) for x, z in outline], 0.86, 0.885), (70, 34, 28), 'piano_lid', grp=pg))
for (x, z) in [(0.72, -0.48), (1.55, -0.48), (1.42, -1.55)]:
    OBJS.append(Obj(lathe(x, z, [(0.0, 0.03), (0.08, 0.045), (0.3, 0.035), (0.5, 0.05), (0.58, 0.045)], 6), (70, 34, 24), 'piano_leg', grp=pg))
OBJS.append(Obj(box(1.09, 0.66, -0.28, 0.47, 0.05, 0.085), (54, 26, 22), 'keybed', grp=pg))
OBJS.append(Obj(box(1.09, 0.775, -0.378, 0.47, 0.06, 0.014), (54, 26, 22), 'fallboard', grp=pg))
for xe in (PIANO_KX0 - 0.03, PIANO_KX1 + 0.03):
    OBJS.append(Obj(box(xe, 0.74, -0.28, 0.025, 0.04, 0.085), (54, 26, 22), 'cheek', grp=pg))
OBJS.append(Obj(box(1.09, 0.96, -0.55, 0.22, 0.09, 0.008, rot=0.0), (90, 50, 34), 'music_desk', grp=pg))
KEYS_Y = 0.715
WHITE = []
BLACK = []
_white_idx = 0
for m in range(29, 90):                       # F1 .. E6, a five-octave Viennese fortepiano
    if m % 12 in (1, 3, 6, 8, 10):
        BLACK.append(m)
    else:
        WHITE.append(m)
KEY_W = (PIANO_KX1 - PIANO_KX0) / len(WHITE)

# candelabra on the piano + one standing candelabra (front left) + two wall sconces (back wall)
CANDLES = []
cg = new_group()
cb = np.array([1.36, 0.885, -1.38])
OBJS.append(Obj(lathe(cb[0], cb[2], [(0.0, 0.07), (0.03, 0.05), (0.12, 0.02), (0.32, 0.025), (0.36, 0.04)], 6, cy=cb[1]), (214, 170, 80), 'candelabra', grp=cg))
for dx in (-0.12, 0.0, 0.12):
    arm = box(cb[0] + dx * 0.5, cb[1] + 0.36, cb[2], abs(dx) * 0.5 + 0.01, 0.01, 0.01)
    OBJS.append(Obj(arm + prism(cb[0] + dx, cb[2], 0.022, cb[1] + 0.36, cb[1] + 0.4, 6), (214, 170, 80), 'candelabra', grp=cg))
    OBJS.append(Obj(prism(cb[0] + dx, cb[2], 0.013, cb[1] + 0.4, cb[1] + 0.5 + (0.03 if dx == 0 else 0), 6), (246, 238, 214), 'candle', grp=cg, cast=False))
    CANDLES.append(np.array([cb[0] + dx, cb[1] + 0.53 + (0.03 if dx == 0 else 0), cb[2]]))
sg_ = new_group()
fc = np.array([-1.32, 0.0, 1.18])
OBJS.append(Obj(lathe(fc[0], fc[2], [(0.0, 0.16), (0.04, 0.12), (0.1, 0.04), (1.25, 0.03), (1.32, 0.06), (1.36, 0.07)], 6), (204, 160, 76), 'floor_candelabra', grp=sg_))
for a in (0.0, 2.094, 4.188):
    px, pz = fc[0] + 0.15 * math.cos(a), fc[2] + 0.15 * math.sin(a)
    OBJS.append(Obj(box(0.5 * (fc[0] + px), 1.38, 0.5 * (fc[2] + pz), 0.08, 0.01, 0.01, rot=-a) + prism(px, pz, 0.03, 1.36, 1.41, 6), (204, 160, 76), 'floor_candelabra', grp=sg_))
    OBJS.append(Obj(prism(px, pz, 0.015, 1.41, 1.56, 6), (246, 238, 214), 'candle', grp=sg_, cast=False))
    CANDLES.append(np.array([px, 1.6, pz]))
SCONCES = [np.array([-1.5, 1.95, RZ0 + 0.06]), np.array([1.5, 1.95, RZ0 + 0.06])]
for s in SCONCES:
    g2 = new_group()
    OBJS.append(Obj(box(s[0], s[1] - 0.06, RZ0 + 0.03, 0.05, 0.08, 0.03) + box(s[0], s[1] - 0.12, s[2], 0.06, 0.012, 0.05), (214, 170, 80), 'sconce', grp=g2))
    OBJS.append(Obj(prism(s[0], s[2], 0.014, s[1] - 0.11, s[1] + 0.02, 6), (246, 238, 214), 'candle', grp=g2, cast=False))
    CANDLES.append(s + np.array([0, 0.05, 0]))

# ---------------------------------------------------------------- chair, palm, footstool
chg = new_group()
chair_c = np.array([-0.88, -0.98])
ch = box(chair_c[0], 0.44, chair_c[1], 0.21, 0.03, 0.2, rot=0.5)
for sx, sz in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
    lx = chair_c[0] + 0.17 * sx * math.cos(0.5) - 0.16 * sz * math.sin(0.5)
    lz = chair_c[1] + 0.17 * sx * math.sin(0.5) + 0.16 * sz * math.cos(0.5)
    ch += prism(lx, lz, 0.022, 0.0, 0.42, 5)
bx, bz = chair_c[0] - 0.19 * math.sin(-0.5) * 0 - 0.17 * math.sin(0.5), chair_c[1] - 0.17 * math.cos(0.5)
ch += box(chair_c[0] + 0.08, 0.72, chair_c[1] - 0.17, 0.17, 0.26, 0.02, rot=0.5)
OBJS.append(Obj(ch, (128, 74, 46), 'chair', grp=chg))
OBJS.append(Obj(box(chair_c[0], 0.48, chair_c[1], 0.19, 0.02, 0.18, rot=0.5), (150, 44, 52), 'cushion', grp=chg))
OBJS.append(Obj(box(chair_c[0] + 0.08, 0.74, chair_c[1] - 0.15, 0.13, 0.19, 0.012, rot=0.5), (150, 44, 52), 'chairback', grp=chg))
pal = new_group()
pc = np.array([1.34, 1.22])
OBJS.append(Obj(lathe(pc[0], pc[1], [(0.0, 0.12), (0.06, 0.16), (0.32, 0.19), (0.36, 0.2)], 8), (176, 92, 60), 'pot', grp=pal))
for k in range(7):
    a = k * 0.9 + 0.3
    tip = np.array([pc[0] + 0.42 * math.cos(a), 0.62 + 0.22 * math.sin(k * 1.7), pc[1] + 0.42 * math.sin(a)])
    base = np.array([pc[0], 0.36, pc[1]])
    mid = 0.5 * (base + tip) + np.array([0, 0.28, 0])
    side = np.cross(tip - base, [0, 1, 0]); side = side / (np.linalg.norm(side) + 1e-9) * 0.07
    leaf = [face([base, mid + side, tip], mid + np.array([0, -1, 0])), face([base, tip, mid - side], mid + np.array([0, -1, 0]))]
    OBJS.append(Obj(leaf, (70, 150, 84) if k % 2 else (58, 128, 72), 'palm', grp=pal, double=True))

# ---------------------------------------------------------------- window parts (left wall)
wg = new_group()
WIN_G = wg
OBJS.append(Obj(box(RX0 + 0.06, WIN_Y0 - 0.02, WIN_ZC, 0.09, 0.02, WIN_R + 0.1), (232, 214, 186), 'sill', grp=wg))
FEET = [(TABLE_C[0], TABLE_C[2], 0.3, 0.45), (chair_c[0], chair_c[1], 0.2, 0.35), (fc[0], fc[2], 0.12, 0.4),
        (pc[0], pc[1], 0.18, 0.5), (0.72, -0.48, 0.04, 0.4), (1.55, -0.48, 0.04, 0.4), (1.42, -1.55, 0.04, 0.4), (1.1, -1.0, 0.4, 0.25)]

# ---------------------------------------------------------------- the parchment staff (back wall)
STAFF_X0, STAFF_X1 = -1.28, 1.28
STAFF_Y0, STAFF_Y1 = 1.6, 2.42
STAFF_Z = RZ0 + 0.035
LINE_DY = 0.03                          # staff line spacing
SYS_BOTTOM = [2.13, 1.755]              # bottom (E4) line of system 1 (bars 1-8) and system 2 (bars 9-16)
BAR_X0, BAR_X1 = -1.12, 1.2             # music area (after the clef)
rg = new_group()
OBJS.append(Obj(cyl_x(STAFF_X0 - 0.06, STAFF_X1 + 0.06, STAFF_Y1 + 0.02, STAFF_Z + 0.02, 0.028), (120, 70, 40), 'rod', grp=rg))
OBJS.append(Obj(cyl_x(STAFF_X0 - 0.04, STAFF_X1 + 0.04, STAFF_Y0 - 0.02, STAFF_Z + 0.02, 0.032), (120, 70, 40), 'rod', grp=rg))
for xe in (STAFF_X0 - 0.07, STAFF_X1 + 0.07):
    OBJS.append(Obj(icosa([xe, STAFF_Y1 + 0.02, STAFF_Z + 0.02], 0.04, rng=rng), (214, 170, 80), 'finial', grp=rg, jitter=0.0))


def staff_xy(slot, frac):
    """(x, bottom-line y) on the back wall for bar slot 0-15 at fraction 0..1 through the bar"""
    row, col = divmod(slot, 8)
    bw = (BAR_X1 - BAR_X0) / 8
    return BAR_X0 + (col + 0.1 + 0.82 * frac) * bw, SYS_BOTTOM[row]


# ---------------------------------------------------------------- the 1792 table as a floor chart
CHART_X0, CHART_X1 = -0.98, 1.06
CHART_Z0, CHART_Z1 = 0.24, 1.32
GRID_X0 = CHART_X0 + 0.15            # leave a margin for the row labels 2..12
GRID_Z0 = CHART_Z0 + 0.1             # header row
CELL_W = (CHART_X1 - 0.04 - GRID_X0) / 16
CELL_D = (CHART_Z1 - 0.04 - GRID_Z0) / 11


def cell_rect(row, col):
    x0 = GRID_X0 + col * CELL_W
    z0 = GRID_Z0 + row * CELL_D
    return x0, x0 + CELL_W, z0, z0 + CELL_D


# ---------------------------------------------------------------- histogram board (back wall, lower left)
HIST_X0, HIST_X1 = -1.58, -0.62
HIST_Y0, HIST_Y1 = 0.98, 1.47
HIST_Z = RZ0 + 0.03
hg = new_group()
OBJS.append(Obj(box(0.5 * (HIST_X0 + HIST_X1), 0.5 * (HIST_Y0 + HIST_Y1), RZ0 + 0.012, 0.5 * (HIST_X1 - HIST_X0) + 0.04, 0.5 * (HIST_Y1 - HIST_Y0) + 0.04, 0.012),
                (140, 92, 46), 'hist_frame', grp=hg))
OBJS.append(Obj([face([[HIST_X0, HIST_Y0, HIST_Z - 0.004], [HIST_X1, HIST_Y0, HIST_Z - 0.004], [HIST_X1, HIST_Y1, HIST_Z - 0.004], [HIST_X0, HIST_Y1, HIST_Z - 0.004]], (0, 1, -5))],
                (34, 58, 52), 'hist_board', grp=hg, jitter=0.0, cast=False))

# ---------------------------------------------------------------- wall calendar (left wall, front part)
CAL_Z0, CAL_Z1, CAL_Y0, CAL_Y1 = 0.12, 0.66, 1.3, 2.02
CAL_X = RX0 + 0.02
calg = new_group()
OBJS.append(Obj(box(CAL_X - 0.005, CAL_Y1 + 0.03, 0.5 * (CAL_Z0 + CAL_Z1), 0.012, 0.035, 0.5 * (CAL_Z1 - CAL_Z0) + 0.02), (150, 44, 52), 'cal_top', grp=calg))

# ---------------------------------------------------------------- curtains + rod (left wall, beside the window)
CURT_ROD_Y = 2.78
def cyl_z(z0, z1, x, y, r, n=6):
    """cylinder lying along z"""
    faces = []
    ang = [2 * math.pi * k / n for k in range(n)]
    for k in range(n):
        a0, a1 = ang[k], ang[(k + 1) % n]
        Q = np.array([[x + r * math.cos(a0), y + r * math.sin(a0), z0], [x + r * math.cos(a0), y + r * math.sin(a0), z1],
                      [x + r * math.cos(a1), y + r * math.sin(a1), z1], [x + r * math.cos(a1), y + r * math.sin(a1), z0]])
        am = 0.5 * (a0 + a1)
        faces.append((Q, np.array([math.cos(am), math.sin(am), 0])))
    return faces


OBJS.append(Obj(cyl_z(WIN_Z0 - 0.44, WIN_Z1 + 0.44, RX0 + 0.09, CURT_ROD_Y, 0.016), (214, 170, 80), 'curtain_rod'))


def curtain(t, side):
    """one velvet drape: zig-zag folds, hem sways a little (small life)"""
    z_in = WIN_Z0 - 0.02 if side < 0 else WIN_Z1 + 0.02
    z_out = z_in + side * 0.34
    faces = []
    n = 7
    sway = 0.012 * math.sin(t * 0.9 + side) + 0.008 * math.sin(t * 2.3 + 1.7 * side)
    tie_y = 1.05
    for k in range(n):
        u0, u1 = k / n, (k + 1) / n
        zt0, zt1 = z_in + (z_out - z_in) * u0, z_in + (z_out - z_in) * u1
        # gathered at the tie-back: the inner edge is pulled toward the outer edge
        zb0 = z_out + (z_in - z_out) * (1 - u0) * 0.35
        zb1 = z_out + (z_in - z_out) * (1 - u1) * 0.35
        x0 = RX0 + 0.05 + (0.035 if k % 2 else 0.0)
        x1 = RX0 + 0.05 + (0.0 if k % 2 else 0.035)
        top = [[x0, CURT_ROD_Y, zt0], [x1, CURT_ROD_Y, zt1]]
        mid = [[x1, tie_y, zb1], [x0, tie_y, zb0]]
        faces.append(face(top + mid, (RX0 - 1, 1.9, 0.5 * (zt0 + zt1))))
        hem0, hem1 = zb0 + sway * (1 - u0), zb1 + sway * (1 - u1)
        low = [[x0, tie_y, zb0], [x1, tie_y, zb1], [x1 + 0.01, 0.02, hem1 + side * 0.04 * u1], [x0 + 0.01, 0.02, hem0 + side * 0.04 * u0]]
        faces.append(face(low, (RX0 - 1, 0.5, 0.5 * (zb0 + zb1))))
    return faces


def valance():
    f = []
    f += box(RX0 + 0.07, CURT_ROD_Y + 0.05, WIN_ZC, 0.03, 0.09, WIN_R + 0.46)
    return f


CURT_G = new_group()
OBJS.append(Obj(valance(), (150, 40, 50), 'valance', grp=CURT_G))


# ---------------------------------------------------------------- the cat on the window sill
CAT_P = np.array([RX0 + 0.02, WIN_Y0, WIN_ZC + 0.12])


def cat_faces(t, look):
    """sitting cat (body along z, head turns toward 'look' angle), tail swishes over the sill edge"""
    r2 = np.random.default_rng(11)
    out = []
    c = CAT_P
    body = icosa(c + [0, 0.11, 0], 0.1, (0.85, 1.15, 1.25), jit=0.08, rng=r2)
    out.append(('body', body))
    chest = icosa(c + [0.03, 0.15, 0.07], 0.07, (0.9, 1.1, 0.9), jit=0.06, rng=r2)
    out.append(('body', chest))
    hc = c + np.array([0.03, 0.27, 0.09])
    head = icosa(hc, 0.062, (1.0, 0.92, 1.05), jit=0.05, rng=r2)
    ca, sa = math.cos(look), math.sin(look)
    ears = []
    for sgn in (-1, 1):
        b0 = np.array([-0.02 * ca, 0.03, 0.035 * sgn])
        e = [hc + np.array([-0.025, 0.04, 0.02 * sgn]), hc + np.array([0.02, 0.04, 0.045 * sgn]), hc + np.array([0.0, 0.1, 0.035 * sgn])]
        ears.append(face(e, hc + np.array([-1, 0, 0])))
    hf = head + ears
    # turn the head about its own vertical axis
    hf = [((Q - hc) @ np.array([[ca, 0, -sa], [0, 1, 0], [sa, 0, ca]]).T + hc, n @ np.array([[ca, 0, -sa], [0, 1, 0], [sa, 0, ca]]).T) for Q, n in hf]
    out.append(('head', hf))
    # tail: hangs over the sill edge into the room and swishes
    tail = []
    base = c + np.array([0.09, 0.03, -0.08])
    p = base
    for k in range(7):
        sw = 0.5 * math.sin(t * 1.7 - k * 0.55) + 0.25 * math.sin(t * 0.6)
        step = np.array([0.03 + 0.005 * k, -0.045, 0.03 * sw])
        q = p + step
        w = 0.03 - 0.003 * k
        tail.append(face([p + [0, 0, -w], p + [0, 0, w], q + [0, 0, w * 0.9], q + [0, 0, -w * 0.9]], p + [-1, 0, 0]))
        tail.append(face([p + [0, w, 0], p + [0, -w, 0], q + [0, -w * 0.9, 0], q + [0, w * 0.9, 0]], p + [0, 0, -1]))
        p = q
    out.append(('tail', tail))
    return out


# ---------------------------------------------------------------- dice + cups
DIE = 0.086
PIPS = {1: [(0, 0)], 2: [(-1, -1), (1, 1)], 3: [(-1, -1), (0, 0), (1, 1)], 4: [(-1, -1), (1, -1), (-1, 1), (1, 1)],
        5: [(-1, -1), (1, -1), (0, 0), (-1, 1), (1, 1)], 6: [(-1, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (1, 1)]}
# local face normal -> value (opposite faces sum to 7)
FACE_VAL = [((0, 1, 0), 1), ((0, -1, 0), 6), ((1, 0, 0), 3), ((-1, 0, 0), 4), ((0, 0, 1), 2), ((0, 0, -1), 5)]


def die_faces(pips=True):
    """unit die centred at the origin, half size DIE/2: list of (Q, n, value)"""
    h = DIE / 2
    out = []
    for nrm, v in FACE_VAL:
        n = np.array(nrm, float)
        a = np.array([0, 0, 1.0]) if abs(n[1]) > 0.5 else np.array([0, 1.0, 0])
        b = np.cross(n, a)
        c = n * h
        Q = np.array([c + (a + b) * h, c + (a - b) * h, c + (-a - b) * h, c + (-a + b) * h])
        out.append((Q, n, v, a, b))
    return out


DIE_FACES = die_faces()


def rot_to_top(v, yaw):
    """rotation matrix that puts the face with value v on top, then yaws by 'yaw'"""
    n = np.array([nn for nn, val in FACE_VAL if val == v][0], float)
    up = np.array([0, 1.0, 0])
    if np.allclose(n, up):
        R = np.eye(3)
    elif np.allclose(n, -up):
        R = np.diag([1.0, -1.0, -1.0])
    else:
        ax = np.cross(n, up); ax /= np.linalg.norm(ax)
        ang = math.acos(np.clip(np.dot(n, up), -1, 1))
        R = axis_rot(ax, ang)
    return axis_rot(up, yaw) @ R


def axis_rot(ax, ang):
    ax = np.asarray(ax, float); ax = ax / (np.linalg.norm(ax) + 1e-12)
    x, y, z = ax
    c, s = math.cos(ang), math.sin(ang)
    C = 1 - c
    return np.array([[c + x * x * C, x * y * C - z * s, x * z * C + y * s],
                     [y * x * C + z * s, c + y * y * C, y * z * C - x * s],
                     [z * x * C - y * s, z * y * C + x * s, c + z * z * C]])


CUP_PROF = [(0.0, 0.07), (0.012, 0.075), (0.15, 0.088), (0.17, 0.094), (0.185, 0.094)]


def cup_faces(pos, tilt, tilt_dir=math.pi):
    """leather dice cup at pos (base centre), tilted by 'tilt' rad toward azimuth tilt_dir"""
    body = lathe(0, 0, CUP_PROF, 9)
    # open cup: drop the top cap, add a dark inner disc a little below the rim
    body = [f for f in body if not (f[1][1] > 0.99 and f[0][:, 1].min() > 0.18)]
    inner = (np.array([[0.085 * math.cos(a), 0.17, 0.085 * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 10)[:-1][::-1]]), np.array([0, 1.0, 0]))
    ax = np.array([math.sin(tilt_dir), 0, math.cos(tilt_dir)])
    ax = np.cross([0, 1, 0], ax)
    R = axis_rot(ax, tilt) if tilt else np.eye(3)
    body = transform(body, R, pos)
    inner = transform([inner], R, pos)
    rim = [f for f in body if f[0][:, 1].min() > pos[1] + 0.14]
    return body, inner, R
