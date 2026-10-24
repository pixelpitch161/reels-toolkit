"""Maze Race scene: a carved wooden maze board on a scholar's desk at night.

World units: 1 = one maze cell (~2.5 cm). y up; the camera looks from +z (the near side of the desk) toward -z.
The board floor is at height YB; board-local coordinates have their origin at the floor centre, cell (r, c) spans
x c-10..c-9, z r-10..r-9 (row 0 = far side). The board rocks on a brass pivot (tilt from the timeline); everything
on it (floor, rim, walls, explorers) is built in board-local coordinates and rotated each frame.

Around it: the desk (per-pixel walnut planks), a brass architect's lamp at the back left (the key light, visible),
two candles (warm local lights), a mantel clock, books, a window with the moon in the back wall, and each explorer's
own prop by its start corner (inkwell + quill, a spool of red thread, a brass compass, a dish of marbles)."""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, thin_quad, new_group

rng = np.random.default_rng(23)
N = 20
HALF = N / 2
YB = 2.2                                   # board floor height (world, untilted)
RIM_W, RIM_H = 0.9, 0.55                   # frame around the maze
SLAB = 1.0                                 # board thickness below the floor
WALL_T, WALL_H = 0.17, 0.42                # carved walls
PIVOT = np.array([0.0, YB, 0.0])           # the board rotates about its floor centre

# ------------------------------------------------------------------ board parts (board-local)
WOOD_RIM = (126, 74, 44)
WOOD_WALL = (214, 168, 112)
WOOD_SIDE = (110, 64, 38)


def rim_boxes():
    a = HALF + RIM_W
    out = []
    for (cx, cz, hx, hz) in [(0, -(HALF + RIM_W / 2), a, RIM_W / 2), (0, HALF + RIM_W / 2, a, RIM_W / 2),
                             (-(HALF + RIM_W / 2), 0, RIM_W / 2, HALF), (HALF + RIM_W / 2, 0, RIM_W / 2, HALF)]:
        out.append(box(cx, (RIM_H - SLAB) / 2, cz, hx, (RIM_H + SLAB) / 2, hz))
    return out


def wall_boxes(walls):
    """one box per unit wall segment (local); returns (faces array (F, 4, 3), normals (F, 3), box id per face, centres)"""
    Qs, Ns, ids, ctrs = [], [], [], []
    for k, (kind, a, b0, b1) in enumerate(walls):
        if kind == 'h':                     # along x at z = a
            cx, cz = (b0 + b1) / 2 - HALF, a - HALF
            hx, hz = 0.5 + WALL_T / 2, WALL_T / 2
        else:                               # along z at x = a
            cx, cz = a - HALF, (b0 + b1) / 2 - HALF
            hx, hz = WALL_T / 2, 0.5 + WALL_T / 2
        fs = box(cx, WALL_H / 2, cz, hx, WALL_H / 2, hz)
        for j, (Q, n) in enumerate(fs):
            if n[1] < -0.5:
                continue                    # bottom face never shows
            Qs.append(Q)
            Ns.append(n)
            ids.append(k)
        ctrs.append((cx, WALL_H / 2, cz))
    return np.array(Qs), np.array(Ns), np.array(ids), np.array(ctrs)


# ------------------------------------------------------------------ the desk and its props (world, static)
DESK_X, DESK_Z0, DESK_Z1 = 44.0, -34.0, 72.0
WALL_Z = -34.0                              # back wall
WINDOW = (6.0, 22.0, 3.0, 17.0)             # x0, x1, y0, y1 on the back wall
MOON_POS = np.array([10.5, 7.2, WALL_Z - 0.5])      # moon disc seen through the window (screen-space disc)

LAMP_BASE = np.array([-17.5, 0.0, -15.0])
LAMP_JOINT = np.array([-15.0, 12.5, -16.5])
LAMP_HEAD = np.array([-9.0, 18.5, -11.0])            # shade mouth centre
LAMP_AIM = np.array([0.5, YB, 0.0])
LAMP_AX = (LAMP_AIM - LAMP_HEAD) / np.linalg.norm(LAMP_AIM - LAMP_HEAD)
LAMP = LAMP_HEAD + LAMP_AX * 0.35                    # the bulb (light position)
CANDLES = [np.array([15.0, 0.0, 12.5]), np.array([-15.5, 0.0, 3.0])]
CANDLE_H = [4.2, 5.4]
FLAME_Y = [h + 0.55 for h in CANDLE_H]
CLOCK = np.array([14.5, 0.0, -17.5])

STATIC = []                                 # (Obj) props: shadow casters on the desk + painter's items
EMISSIVE = {}                               # obj name -> emit


def add(faces, color, name='', grp=None, cast=True, jitter=0.04, emit=0.0):
    o = Obj(faces, color, name, grp=grp, cast=cast, jitter=jitter)
    STATIC.append(o)
    if emit:
        EMISSIVE[name] = emit
    return o


# brass pivot under the board
add(lathe(0, 0, [(0.0, 3.4), (0.35, 3.2), (0.8, 2.2), (1.2, 1.4)], n=12), (196, 150, 70), 'pivot')

# architect's lamp: weighted base, two arm struts, conical shade
g = new_group()
add(prism(LAMP_BASE[0], LAMP_BASE[2], 2.6, 0.0, 0.7, 12, r1=2.2), (70, 64, 60), 'lampbase', grp=g)
add(prism(LAMP_BASE[0], LAMP_BASE[2], 0.6, 0.7, 1.4, 8), (186, 146, 76), 'lampcollar', grp=g)


def strut(a, b, w, col, name, grp):
    a, b = np.asarray(a, float), np.asarray(b, float)
    d = b - a
    L = np.linalg.norm(d)
    u = d / L
    side = np.cross(u, [0, 0, 1.0])
    side = side / (np.linalg.norm(side) + 1e-9) * w
    up = np.cross(side / (np.linalg.norm(side) + 1e-9), u) * w
    P = [a + side + up, a - side + up, a - side - up, a + side - up]
    Qn = [p + d for p in P]
    fs = []
    for i in range(4):
        j = (i + 1) % 4
        fs.append(face([P[i], P[j], Qn[j], Qn[i]], (a + b) / 2))
    add(fs, col, name, grp=grp)


strut(LAMP_BASE + [0, 1.3, 0], LAMP_JOINT, 0.28, (186, 146, 76), 'arm1', g)
strut(LAMP_JOINT, LAMP_HEAD - LAMP_AX * 1.6, 0.28, (186, 146, 76), 'arm2', g)
add(icosa(LAMP_JOINT, 0.55, jit=0.0), (150, 116, 60), 'joint', grp=g)


def cone_shade(head, ax, r0, r1, length, n=10):
    """open cone: narrow end at head - ax*length, mouth (r1) at head"""
    ax = ax / np.linalg.norm(ax)
    t1 = np.cross(ax, [0, 1.0, 0])
    if np.linalg.norm(t1) < 1e-6:
        t1 = np.array([1.0, 0, 0])
    t1 /= np.linalg.norm(t1)
    t2 = np.cross(ax, t1)
    back = head - ax * length
    outer, inner = [], []
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        p = lambda c, r, a: c + r * (math.cos(a) * t1 + math.sin(a) * t2)
        Q = np.array([p(back, r0, a0), p(back, r0, a1), p(head, r1, a1), p(head, r1, a0)])
        nn = np.cross(Q[1] - Q[0], Q[3] - Q[0])
        nn /= np.linalg.norm(nn)
        mid = (Q[0] + Q[2]) / 2
        if np.dot(nn, mid - (back + head) / 2) < 0:
            nn = -nn
        outer.append((Q, nn))
        inner.append((Q[::-1].copy(), -nn))
    cap = (np.array([back + r0 * (math.cos(2 * math.pi * k / n) * t1 + math.sin(2 * math.pi * k / n) * t2) for k in range(n)]), -ax)
    return outer + [cap], inner


SHADE_OUT, SHADE_IN = cone_shade(LAMP_HEAD, LAMP_AX, 0.9, 3.2, 3.4)
_sh = add(SHADE_OUT + SHADE_IN, (36, 92, 70), 'shade', grp=g)        # green enamel shade, glowing inside
_sh.face_col = np.array([(36, 92, 70)] * len(SHADE_OUT) + [(255, 236, 190)] * len(SHADE_IN), float)
_sh.face_emit = np.array([0.0] * len(SHADE_OUT) + [1.4] * len(SHADE_IN))

# candles: brass dish, wax column
for k, (c, h) in enumerate(zip(CANDLES, CANDLE_H)):
    g = new_group()
    add(prism(c[0], c[2], 1.6, 0.0, 0.35, 10, r1=1.3), (190, 146, 66), f'dish{k}', grp=g)
    add(prism(c[0], c[2], 0.62, 0.35, h, 8), (238, 224, 196), f'wax{k}', grp=g)
    add(prism(c[0], c[2], 0.66, h - 0.6, h - 0.35, 8, r1=0.5), (246, 236, 214), f'drip{k}', grp=g)

# mantel clock (body, arched top, dial face toward the camera)
g = new_group()
add(box(CLOCK[0], 2.6, CLOCK[2], 2.4, 2.6, 1.3), (96, 54, 32), 'clockbody', grp=g)
add(prism(CLOCK[0], CLOCK[2], 1.6, 5.2, 5.9, 8, r1=1.0), (110, 64, 38), 'clocktop', grp=g)
add(box(CLOCK[0], 0.2, CLOCK[2], 2.7, 0.2, 1.5), (80, 46, 28), 'clockfoot', grp=g)
CLOCK_DIAL = np.array([CLOCK[0], 3.1, CLOCK[2] + 1.32])
add([(np.array([CLOCK_DIAL + [1.6 * math.cos(a), 1.6 * math.sin(a), 0] for a in np.linspace(0, 2 * math.pi, 17)[:-1]]), np.array([0, 0, 1.0]))],
    (232, 220, 190), 'dial', grp=g, cast=False, jitter=0.0)
add([(np.array([CLOCK_DIAL + [1.85 * math.cos(a), 1.85 * math.sin(a), -0.05] for a in np.linspace(0, 2 * math.pi, 17)[:-1]]), np.array([0, 0, 1.0]))],
    (196, 152, 70), 'bezel', grp=g, cast=False, jitter=0.0)

# books: a stack at the back left, two leaning on the back
for k, (dx, dz, w, d, h, col, rot) in enumerate([(-24, -24, 4.5, 3.2, 0.8, (120, 40, 44), 0.1), (-24, -24, 4.2, 3.0, 0.7, (40, 70, 110), -0.06),
                                                 (-24, -24, 4.0, 2.9, 0.9, (150, 112, 52), 0.18), (-24, -24, 3.6, 2.6, 0.6, (60, 96, 70), 0.0)]):
    y0 = sum(v[4] for v in [(-24, -24, 4.5, 3.2, 0.8), (-24, -24, 4.2, 3.0, 0.7), (-24, -24, 4.0, 2.9, 0.9), (-24, -24, 3.6, 2.6, 0.6)][:k])
    add(box(dx, y0 + h / 2, dz, w, h / 2, d, rot=rot), col, f'book{k}')
for k, (x, col) in enumerate([(-4.0, (130, 46, 50)), (-2.6, (52, 82, 120)), (-1.4, (160, 120, 60)), (-0.1, (70, 104, 78)), (1.2, (104, 60, 110))]):
    add(box(x, 4.5, WALL_Z + 2.4, 0.6, 4.5 - k * 0.3, 2.2), col, f'spine{k}')

# each explorer's prop by its start corner
g = new_group()                                               # FLOOD: inkwell + quill (blue ink)
add(prism(-14.0, -12.5, 1.3, 0.0, 1.6, 8, r1=1.0), (30, 50, 110), 'inkwell', grp=g)
add(prism(-14.0, -12.5, 0.6, 1.6, 2.1, 8), (40, 40, 48), 'inkneck', grp=g)
add([face(thin_quad([-14.0, 1.9, -12.5], [-11.5, 7.0, -13.4], 0.5), [0, 0, -100])], (214, 210, 200), 'quill', grp=g, cast=False)
g = new_group()                                               # DIVER: spool of red thread
add(prism(13.8, -12.6, 1.5, 0.0, 0.35, 10), (150, 110, 64), 'spool0', grp=g)
add(prism(13.8, -12.6, 1.05, 0.35, 2.15, 10), (214, 36, 46), 'spool1', grp=g)
add(prism(13.8, -12.6, 1.5, 2.15, 2.5, 10), (150, 110, 64), 'spool2', grp=g)
g = new_group()                                               # SEEKER: a brass compass
add(prism(14.0, 13.6, 1.8, 0.0, 0.5, 12), (204, 160, 72), 'compass', grp=g)
add(prism(14.0, 13.6, 1.45, 0.5, 0.56, 12), (236, 226, 196), 'compassface', grp=g, cast=False)
g = new_group()                                               # MARBLE: a dish of spare marbles
add(prism(-14.0, 13.6, 2.0, 0.0, 0.45, 12, r1=2.3), (100, 70, 50), 'marbledish', grp=g)
for k, (dx, dz, col) in enumerate([(-0.7, -0.3, (214, 222, 236)), (0.6, 0.2, (140, 200, 220)), (-0.1, 0.8, (230, 190, 140))]):
    add(icosa([-14.0 + dx, 0.85, 13.6 + dz], 0.42, jit=0.0), col, f'spare{k}', grp=g, cast=False)

# notebook with a sketched maze, loose papers (flat, near the camera)
add(box(-4.0, 0.08, 19.5, 5.0, 0.08, 3.6, rot=0.12), (150, 132, 104), 'notebook', cast=False, jitter=0.01)
add(box(-4.0, 0.17, 19.5, 4.6, 0.02, 3.2, rot=0.12), (182, 166, 136), 'page', cast=False, jitter=0.01)
add(box(9.0, 0.05, -17.0, 4.0, 0.05, 3.0, rot=-0.2), (220, 206, 172), 'paper', cast=False, jitter=0.01)

# back wall + window frame
WALL_FACES = []
for i in range(-10, 10):
    for j in range(0, 8):
        x0, x1, y0, y1 = i * 4.0, (i + 1) * 4.0, j * 4.0, (j + 1) * 4.0
        cx = (x0 + x1) / 2
        cy = (y0 + y1) / 2
        if WINDOW[0] <= cx <= WINDOW[1] and WINDOW[2] <= cy <= WINDOW[3]:
            continue
        WALL_FACES.append(np.array([[x0, y0, WALL_Z], [x1, y0, WALL_Z], [x1, y1, WALL_Z], [x0, y1, WALL_Z]]))
WALL_TONE = rng.uniform(0.94, 1.05, len(WALL_FACES))
g = new_group()
wx0, wx1, wy0, wy1 = WINDOW
for (cx, cy, hx, hy) in [((wx0 + wx1) / 2, wy0, (wx1 - wx0) / 2 + 0.6, 0.45), ((wx0 + wx1) / 2, wy1, (wx1 - wx0) / 2 + 0.6, 0.45),
                         (wx0, (wy0 + wy1) / 2, 0.45, (wy1 - wy0) / 2), (wx1, (wy0 + wy1) / 2, 0.45, (wy1 - wy0) / 2),
                         ((wx0 + wx1) / 2, (wy0 + wy1) / 2, 0.22, (wy1 - wy0) / 2), ((wx0 + wx1) / 2, (wy0 + wy1) / 2, (wx1 - wx0) / 2, 0.22)]:
    add(box(cx, cy, WALL_Z + 0.4, hx, hy, 0.4), (150, 104, 66), 'winframe', grp=g, cast=False)
add(box((wx0 + wx1) / 2, wy0 - 0.6, WALL_Z + 1.2, (wx1 - wx0) / 2 + 1.2, 0.3, 1.2), (160, 112, 70), 'sill', grp=g, cast=False)
PANE = np.array([[wx0, wy0, WALL_Z + 0.05], [wx1, wy0, WALL_Z + 0.05], [wx1, wy1, WALL_Z + 0.05], [wx0, wy1, WALL_Z + 0.05]])

# shadow casters on the desk: props + the board (slab + rim as one box)
BOARD_BOX = box(0, YB - SLAB / 2 + RIM_H / 2, 0, HALF + RIM_W, (SLAB + RIM_H) / 2, HALF + RIM_W)
CASTERS = [Q for o in STATIC if o.cast for Q, n in o.faces] + [Q for Q, n in BOARD_BOX]
FEET = [(0, 0, 3.0, 0.45), (LAMP_BASE[0], LAMP_BASE[2], 2.5, 0.4), (CANDLES[0][0], CANDLES[0][2], 1.5, 0.4),
        (CANDLES[1][0], CANDLES[1][2], 1.5, 0.4), (CLOCK[0], CLOCK[2] + 1.0, 2.2, 0.35), (-24, -24, 3.5, 0.3),
        (-14.0, -12.5, 1.2, 0.35), (13.8, -12.6, 1.4, 0.35), (14.0, 13.6, 1.7, 0.3), (-14.0, 13.6, 2.0, 0.3)]

# small life: a moth circling the lamp, dust in its beam
MOTH_R, MOTH_PERIOD = 2.6, 61.0 / 26
DUST = rng.uniform(0, 1, (90, 5))           # (u along the beam, angle, radius, phase, speed)
