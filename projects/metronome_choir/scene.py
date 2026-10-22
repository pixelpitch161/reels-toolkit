"""A low-poly concert hall in the afternoon, and the choir loft.

Room shell (ray-cast per pixel in render.py; only planes, extents and lighting helpers here): floor y = 0,
back wall z = HZ0 (wood panelling + an organ facade), side walls x = +-HX with stage wings (dark velvet),
balcony bands and sconces, coffered ceiling y = HY with a big round skylight: the sun (key light) pours
through it onto the loft. Warm stage spots on a truss over the front, footlights along the platform edge.
The loft: 5 carpeted steps on one shared platform; each step carries a riser board with its section of
metronomes (#1's model: cream body, cap and board edges in the section colour, tip = phase light).
In front: the golden soloist on a podium, a grand piano with a ginger cat, timpani, and the front rows of
pawn listeners. Every periodic idle motion completes whole cycles per minute (seamless loop)."""
import math
import numpy as np
from reelkit.config import dirv, nrm
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, new_group
import timeline as T
import layout as LY

rng = np.random.default_rng(16)

# ---------------------------------------------------------------- hall shell + light
HX, HZ0, HZ1, HY = 16.0, -25.0, 30.0, 18.0
SUN = dirv(-15, 64)                          # toward the sun: high, from behind-left, through the skylight
SUN_RGB = np.array([1.0, 0.9, 0.74])
SKY_C = np.array([-1.6, -12.8])              # skylight centre (x, z) on the ceiling
SKY_R = 11.4
WING_Z = (-23.0, 7.5)                        # stage wings (dark velvet openings) in both side walls
WING_Y = 10.5
WARM = np.array([1.0, 0.66, 0.36])


def sky_lit(P, soft=0.7):
    """0..1: the ray from P toward the sun leaves through the skylight (soft edge = the sun's disc + frame)"""
    P = np.atleast_2d(np.asarray(P, float))
    s = (HY - P[:, 1]) / SUN[1]
    x = P[:, 0] + s * SUN[0] - SKY_C[0]
    z = P[:, 2] + s * SUN[2] - SKY_C[1]
    d = np.sqrt(x * x + z * z)
    return np.clip((SKY_R - d) / soft + 0.5, 0, 1).astype(np.float32) * (P[:, 1] < HY)


# stage lights: (position, colour, radius). Spots on the truss + footlights (approximated by three lights)
TRUSS_Y, TRUSS_Z = 16.7, 12.5
SPOTS = [np.array([x, TRUSS_Y - 0.5, TRUSS_Z]) for x in (-7.5, -2.6, 2.6, 7.5)]
FOOT_Y = 0.12

# ---------------------------------------------------------------- builders
def cyl_z(cx, cy, z0, z1, r, n=8, rot=0.0, cap=True):
    """cylinder along z, rot = roll angle; returns side faces (+ end caps)"""
    out = []
    ang = [rot + 2 * math.pi * k / n for k in range(n)]
    for k in range(n):
        a0, a1 = ang[k], ang[(k + 1) % n]
        Q = np.array([[cx + r * math.cos(a0), cy + r * math.sin(a0), z0], [cx + r * math.cos(a1), cy + r * math.sin(a1), z0],
                      [cx + r * math.cos(a1), cy + r * math.sin(a1), z1], [cx + r * math.cos(a0), cy + r * math.sin(a0), z1]])
        am = 0.5 * (a0 + a1)
        out.append((Q, np.array([math.cos(am), math.sin(am), 0.0])))
    if cap:
        out.append((np.array([[cx + r * math.cos(a), cy + r * math.sin(a), z1] for a in ang]), np.array([0, 0, 1.0])))
        out.append((np.array([[cx + r * math.cos(a), cy + r * math.sin(a), z0] for a in ang[::-1]]), np.array([0, 0, -1.0])))
    return out


def rot_y(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def xform(faces, R, t):
    R = np.asarray(R, float)
    return [(Q @ R.T + t, n @ R.T) for Q, n in faces]


def shift(faces, d):
    d = np.asarray(d, float)
    return [(Q + d, n) for Q, n in faces]


# ---------------------------------------------------------------- the metronome (#1's model)
Y_PL, Y_TOP = 0.08, 1.0
WXB, WZB, CB = 0.28, 0.22, 0.06
WXT, WZT, CT = 0.11, 0.085, 0.025


def _octagon(wx, wz, c, y):
    pts = [(wx - c, wz), (wx, wz - c), (wx, -wz + c), (wx - c, -wz), (-wx + c, -wz), (-wx, -wz + c), (-wx, wz - c), (-wx + c, wz)]
    return [np.array([x, y, z]) for x, z in pts]


def _zfront(y):
    return WZB + (WZT - WZB) * (y - Y_PL) / (Y_TOP - Y_PL)


def build_metronome():
    faces, mats = [], []
    px, pz = 0.33, 0.27
    for Q, n in box(0, Y_PL / 2, 0, px, Y_PL / 2, pz):
        if n[1] < -0.5:
            continue
        faces.append((Q, n)); mats.append('plinth')
    b = _octagon(WXB, WZB, CB, Y_PL)
    tp = _octagon(WXT, WZT, CT, Y_TOP)
    for k in range(8):
        k2 = (k + 1) % 8
        faces.append(face([b[k], b[k2], tp[k2], tp[k]], (0, 0.5, 0))); mats.append('front' if k == 7 else 'body')
    apex = np.array([0.0, Y_TOP + 0.13, 0.0])
    for k in range(8):
        faces.append(face([tp[k], tp[(k + 1) % 8], apex], (0, Y_TOP + 0.03, 0))); mats.append('cap')
    yb, yt, hb, ht = 0.17, 0.88, 0.15, 0.055
    nf = faces[mats.index('front')][1]
    Q = np.array([[hb, yb, _zfront(yb) + 0.004], [-hb, yb, _zfront(yb) + 0.004], [-ht, yt, _zfront(yt) + 0.004], [ht, yt, _zfront(yt) + 0.004]])
    faces.append((Q, nf)); mats.append('panel')
    return faces, mats


MET_FACES, MET_MATS = build_metronome()
MET_Q = np.array([np.pad(Q, ((0, 5 - len(Q)), (0, 0)), mode='edge') for Q, n in MET_FACES])     # (F, 5, 3) padded
MET_NV = np.array([len(Q) for Q, n in MET_FACES])
MET_N = np.array([n for Q, n in MET_FACES])
MET_C = np.array([Q.mean(0) for Q, n in MET_FACES])
PANEL = MET_MATS.index('panel')
FRONT = MET_MATS.index('front')
PIVOT = np.array([0.0, 0.21, _zfront(0.21) + 0.04])
ROD = 0.74
MET_H = Y_TOP + 0.13
CREAM = np.array([232, 220, 196.0])
PLINTH = np.array([96, 64, 50.0])
PANEL_C = np.array([44, 36, 52.0])
SOLO_BODY = np.array(T.GOLD, float)
SOLO_CAP = np.array([214, 96, 70.0])


def met_colours(sec):
    """albedo per face for a metronome of section sec (-1 = soloist)"""
    cap = SOLO_CAP if sec < 0 else np.array(T.SEC_COL[sec], float)
    body = SOLO_BODY if sec < 0 else CREAM
    plinth = np.array([150, 104, 40.0]) if sec < 0 else PLINTH
    out = []
    for m in MET_MATS:
        out.append({'plinth': plinth, 'body': body, 'front': body, 'cap': cap, 'panel': PANEL_C}[m])
    return np.array(out)


MET_COLS = [met_colours(k) for k in range(T.NSEC)] + [met_colours(-1)]

# ---------------------------------------------------------------- loft pieces (rest positions; render offsets them)
CARPET = (120, 34, 44)
STEP_WOOD = (92, 58, 40)
BOARD_TOP = (198, 152, 100)
PLAT_WOOD = (70, 50, 40)


def step_faces(k):
    """step k box faces except the top (per-pixel pass); front face split: wood + section-colour band"""
    z0, z1 = LY.ZT[k] - LY.STEP_HZ, LY.ZT[k] + LY.STEP_HZ
    y0, y1 = LY.PLAT_T, LY.YT[k]
    out = []
    hx = LY.STEP_HX
    yb = y1 - 0.2
    out.append((np.array([[-hx, y0, z1], [hx, y0, z1], [hx, yb, z1], [-hx, yb, z1]]), np.array([0, 0, 1.0]), STEP_WOOD))
    out.append((np.array([[-hx, yb, z1], [hx, yb, z1], [hx, y1, z1], [-hx, y1, z1]]), np.array([0, 0, 1.0]), T.SEC_COL[k]))
    for sx in (-1, 1):
        Q = np.array([[sx * hx, y0, z0], [sx * hx, y0, z1], [sx * hx, y1, z1], [sx * hx, y1, z0]])
        out.append((Q, np.array([sx, 0, 0.0]), STEP_WOOD))
    return out


def board_faces(k):
    """board side faces in local coords (x from the board centre, y from its bottom, z from ZT[k])"""
    out = []
    for Q, n in box(0, LY.BOARD_T / 2, 0, LY.BOARD_HX, LY.BOARD_T / 2, LY.BOARD_HZ):
        if abs(n[1]) > 0.5:
            continue
        out.append((Q, n))
    return out


BOARD_SIDES = board_faces(0)

CAN_COLS = [(236, 104, 78), (74, 190, 166)]
CAN_END = (196, 204, 210)


def can_faces(cx, cy, cz, r, half_len, roll, n=8):
    side = cyl_z(cx, cy, cz - half_len, cz + half_len, r, n, rot=roll, cap=False)
    ends = cyl_z(cx, cy, cz - half_len, cz + half_len, r, n, rot=roll, cap=True)[n:]
    inner = cyl_z(cx, cy, cz - half_len - 0.003, cz + half_len + 0.003, r * 0.72, n, rot=roll, cap=True)[n:]
    return side, ends, inner


# ---------------------------------------------------------------- front of stage: podium, piano + cat, timpani
POD_FACES = prism(LY.POD[0], LY.POD[1], LY.POD_R, LY.PLAT_T, LY.PLAT_T + LY.POD_H - 0.06, 8, rot=math.pi / 8)
POD_TRIM = prism(LY.POD[0], LY.POD[1], LY.POD_R + 0.06, LY.PLAT_T + LY.POD_H - 0.06, LY.PLAT_T + LY.POD_H, 8, rot=math.pi / 8)
POD_COL, POD_TRIM_COL = (60, 40, 34), (230, 178, 82)

PIANO_P = np.array([-6.9, 0.0, 9.6])
PIANO_ROT = math.radians(-35)                 # keyboard faces the stage centre / audience-right
PIANO_G = new_group()
FRONT_OBJS = []        # static objects in front of the loft (drawn in painter order with pawns etc.)


def _piano():
    # outline of a grand piano (top view), keyboard edge along +z ... tail toward -z
    pts = [(-0.75, 0.75), (0.75, 0.75), (0.75, -0.6), (0.62, -1.1), (0.35, -1.55), (0.0, -1.85), (-0.3, -1.95), (-0.55, -1.75),
           (-0.7, -1.2), (-0.75, -0.4)]
    y0, y1 = 0.62, 1.0
    R = rot_y(PIANO_ROT)
    faces = []
    n = len(pts)
    for i in range(n):
        (x0, z0), (x1, z1) = pts[i], pts[(i + 1) % n]
        faces.append(face([[x0, y0, z0], [x1, y0, z1], [x1, y1, z1], [x0, y1, z0]], (0, 0.8, -0.6)))
    faces.append((np.array([[x, y1, z] for x, z in pts[::-1]]), np.array([0, 1.0, 0])))
    body = xform(faces, R, PIANO_P)
    FRONT_OBJS.append(Obj(body, (40, 36, 44), 'piano', grp=PIANO_G))
    kb = box(0, 0.78, 0.86, 0.72, 0.05, 0.13)
    FRONT_OBJS.append(Obj(xform(kb, R, PIANO_P), (240, 236, 226), 'keys', grp=PIANO_G, cast=False))
    kb2 = box(0, 0.84, 0.95, 0.74, 0.04, 0.05)
    FRONT_OBJS.append(Obj(xform(kb2, R, PIANO_P), (36, 32, 38), 'keyrail', grp=PIANO_G, cast=False))
    legs = []
    for lx, lz in ((-0.6, 0.6), (0.6, 0.6), (-0.2, -1.6)):
        legs += prism(lx, lz, 0.07, 0.0, 0.62, 6)
    FRONT_OBJS.append(Obj(xform(legs, R, PIANO_P), (34, 30, 36), 'pianolegs', grp=PIANO_G))
    bench = box(0, 0.5, 1.55, 0.5, 0.06, 0.2) + prism(-0.42, 1.55, 0.04, 0, 0.45, 4) + prism(0.42, 1.55, 0.04, 0, 0.45, 4)
    FRONT_OBJS.append(Obj(xform(bench, R, PIANO_P), (40, 36, 44), 'bench', grp=new_group()))


_piano()
PIANO_TOP = 1.0
CAT_P = PIANO_P + rot_y(PIANO_ROT) @ np.array([0.05, 0.0, -0.75]) + np.array([0, PIANO_TOP, 0])

TIMPANI = [np.array([6.6, 0.0, 9.0]), np.array([8.1, 0.0, 10.1])]
for tp, r in zip(TIMPANI, (0.62, 0.52)):
    g = new_group()
    bowl = lathe(tp[0], tp[2], [(0.16, 0.0), (0.2, r * 0.5), (0.34, r * 0.86), (0.52, r * 1.0), (0.78, r * 1.03), (0.86, r * 1.03)], 10)
    FRONT_OBJS.append(Obj(bowl, (196, 112, 58), 'kettle', grp=g))
    FRONT_OBJS.append(Obj([(np.array([[tp[0] + r * 1.0 * math.cos(a), 0.87, tp[2] + r * 1.0 * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 11)[:-1]][::-1]),
                            np.array([0, 1.0, 0]))], (236, 226, 204), 'head', grp=g, cast=False))
    legs = []
    for a in (0.3, 2.4, 4.5):
        legs += prism(tp[0] + 0.45 * math.cos(a), tp[2] + 0.45 * math.sin(a), 0.035, 0, 0.3, 4)
    FRONT_OBJS.append(Obj(legs, (60, 60, 66), 'tlegs', grp=g))

# potted plants on the step ends
PLANTS = []
for k in range(T.NSEC):
    for sx in (-1, 1):
        if (k + (sx > 0)) % 2:
            continue
        PLANTS.append((k, sx * 8.6, LY.ZT[k] - 0.6 * sx, 0.55 + 0.15 * ((k * 7) % 3)))


def plant_faces(k, x, z, s):
    r2 = np.random.default_rng(int(abs(x) * 10 + k))
    pot = lathe(x, z, [(0, 0.22), (0.38, 0.28), (0.42, 0.3)], 8)
    crown = icosa([x, 0.42 + 0.55 * s, z], 0.55 * s, (1, 1.1, 1), rng=r2)
    crown2 = icosa([x + 0.2, 0.42 + 0.95 * s, z - 0.1], 0.38 * s, rng=r2)
    return pot, crown + crown2


# ---------------------------------------------------------------- audience: raked seats + pawn listeners
SEAT_Z0, SEAT_DZ, SEAT_RISE = 12.6, 1.15, 0.22
SEAT_ROWS = 10
SEAT_BLOCKS = [(-13.5, -1.4), (1.4, 13.5)]
SEATS = []
for i in range(SEAT_ROWS):
    z = SEAT_Z0 + i * SEAT_DZ
    y0 = i * SEAT_RISE
    for x0, x1 in SEAT_BLOCKS:
        xc, hw = 0.5 * (x0 + x1), 0.5 * (x1 - x0)
        fb = []
        if y0 > 0:
            fb += box(xc, y0 / 2, z, hw, y0 / 2, SEAT_DZ / 2)
        fb += box(xc, y0 + 0.42, z + 0.05, hw, 0.06, 0.24)
        fb += box(xc, y0 + 0.72, z + 0.3, hw, 0.28, 0.05)
        SEATS.append(Obj(fb, (150, 36, 46), 'seats', grp=new_group()))

PAWN_COLS = [(52, 58, 92), (110, 44, 46), (64, 84, 58), (92, 64, 48), (40, 40, 46), (120, 96, 64), (70, 52, 92), (44, 92, 104)]
SKIN = (238, 206, 170)
PAWNS = []
for i in range(3):
    z = SEAT_Z0 + i * SEAT_DZ
    y0 = i * SEAT_RISE
    for x0, x1 in SEAT_BLOCKS:
        x = x0 + 0.4
        while x < x1 - 0.3:
            if rng.uniform() > (0.35 if i == 0 else 0.5):
                PAWNS.append(dict(p=np.array([x + rng.uniform(-0.05, 0.05), y0 + 0.48, z + 0.08]), col=PAWN_COLS[int(rng.integers(len(PAWN_COLS)))],
                                  hat=int(rng.integers(0, 3)), ph=float(rng.uniform(0, 1)), s=float(rng.uniform(1.05, 1.25))))
            x += 0.75

PAWN_BODY = lathe(0, 0, [(0.0, 0.14), (0.04, 0.13), (0.07, 0.09), (0.2, 0.065), (0.3, 0.05), (0.33, 0.085), (0.36, 0.075)], 8)
PAWN_HEAD = icosa([0, 0.44, 0], 0.085, jit=0.03, rng=np.random.default_rng(7))


def _hat(kind):
    if kind == 1:
        return lathe(0, 0, [(0.0, 0.11), (0.012, 0.11), (0.014, 0.06), (0.14, 0.065)], 8, cy=0.5)
    if kind == 2:
        return lathe(0, 0, [(0.0, 0.1), (0.01, 0.1), (0.02, 0.075), (0.07, 0.06), (0.09, 0.0)], 8, cy=0.49)
    return []


HATS = {k: _hat(k) for k in range(3)}


def pawn_state(pw, t):
    """idle bob, follow the soloist in the spotlight, bounce on the beat after the drop"""
    w = 2 * math.pi / 60.0
    bob = 0.012 * math.sin(w * (8 + int(pw['ph'] * 5)) * t + pw['ph'] * 6.28)
    turn = 0.22 * math.sin(w * 3 * t + pw['ph'] * 9)
    if T.T_DROP <= t < T.T_JOLT:
        u = (t - T.T_DROP + pw['ph'] * 0.06) % T.BEAT
        bob += 0.05 * math.exp(-u / 0.1) * min(1.0, (t - T.T_DROP) / 0.5)
    if T.T_JOLT <= t < T.T_JOLT + 0.6:                     # everyone jumps at the thump
        bob += 0.12 * math.sin(math.pi * (t - T.T_JOLT) / 0.6)
    return bob, turn


def pawn_faces(pw, t):
    off, turn = pawn_state(pw, t)
    R = rot_y(turn)
    s = pw['s']
    base = pw['p'] + [0, off, 0]
    body = [(Q * s @ R.T + base, n @ R.T) for Q, n in PAWN_BODY]
    head = [(Q * s @ R.T + base, n @ R.T) for Q, n in PAWN_HEAD]
    hat = [(Q * s @ R.T + base, n @ R.T) for Q, n in HATS[pw['hat']]]
    return body, head, hat


# ---------------------------------------------------------------- the cat on the piano
def cat_faces(t):
    """ginger cat lying on the piano lid: watches the soloist, tail swishes, bobs to the song, startles at the thump"""
    w = 2 * math.pi / 60.0
    look = 0.35 * math.sin(w * 4 * t)
    if 33.5 <= t < T.T_DROP + 1:
        look = look * (1 - min(1, (t - 33.5) / 0.6)) + 0.75 * min(1, (t - 33.5) / 0.6)     # turns to the soloist
    hop = 0.0
    if T.T_JOLT <= t < T.T_JOLT + 0.55:
        hop = 0.22 * math.sin(math.pi * (t - T.T_JOLT) / 0.55)
    bob = 0.025 * T.beat_env(t, 0.12)
    R = rot_y(PIANO_ROT + 0.4)
    r2 = np.random.default_rng(5)
    base = CAT_P + [0, hop, 0]
    body = icosa([0, 0.16, 0], 0.2, (1.0, 0.75, 1.6), jit=0.05, rng=r2)
    hp = np.array([0.0, 0.32 + bob, 0.36])
    Rh = rot_y(look)
    head = [(Q @ Rh.T + hp, n @ Rh.T) for Q, n in icosa([0, 0, 0], 0.13, (1.1, 0.95, 1.0), jit=0.04, rng=r2)]
    ears = []
    for sx in (-1, 1):
        e = [face([[sx * 0.05, 0.08, 0.02], [sx * 0.12, 0.08, -0.02], [sx * 0.1, 0.2 - 0.08 * (hop > 0.05), 0.0]], (0, 0, -1))]
        ears += [(Q @ Rh.T + hp, n @ Rh.T) for Q, n in e]
    tail = []
    sw = math.sin(w * 30 * t) * 0.9
    p0 = np.array([0.0, 0.12, -0.3])
    prev = p0
    for j in range(1, 6):
        a = sw * j / 5
        p = p0 + np.array([0.18 * j / 5 * math.sin(a) * 1.4, 0.02 * j, -0.11 * j * math.cos(a * 0.5)])
        d = p - prev
        side = np.cross(d, [0, 1, 0]); side = side / (np.linalg.norm(side) + 1e-9) * 0.035
        tail.append(face([prev - side, p - side, p + side, prev + side], prev + [0, -0.5, 0], up=True))
        prev = p
    out = {}
    for name, fcs in (('body', body), ('head', head), ('ears', ears), ('tail', tail)):
        out[name] = [(Q @ R.T + base, n @ R.T) for Q, n in fcs]
    return out


# ---------------------------------------------------------------- truss + spot fixtures (high front)
TRUSS_G = new_group()
TRUSS = box(0, TRUSS_Y, TRUSS_Z, 10.5, 0.12, 0.12) + box(0, TRUSS_Y + 0.5, TRUSS_Z, 10.5, 0.08, 0.08)
for x in np.arange(-10, 10.1, 1.25):
    TRUSS += box(x, TRUSS_Y + 0.25, TRUSS_Z, 0.03, 0.25, 0.03)
FIXTURES = []
for sp in SPOTS:
    aim = nrm(np.array([0.0, 4.0, -6.0]) - sp)
    FIXTURES.append((sp, aim))

# dust motes (they sparkle only inside the sunbeam)
MOTES = np.random.default_rng(5).uniform([-10, 1.0, -22], [8, 16, 4], (420, 3))
