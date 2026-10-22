"""A Victorian lecture hall in late-afternoon sun, and the giant brass-and-walnut Galton machine.

Room shell (ray-traced per pixel in render.py, here only planes, extents and texture helpers):
    floor y = 0, stage top y = 0.4, back wall z = HZ0, left wall x = HX0 with two tall arched
    plate-glass windows (the low sun = key light, streaming in from front-left), right wall x = HX1,
    coffered ceiling y = HY, cut-away front at z = HZ1.
The machine stands on the stage facing the audience: walnut base cabinet with a brass counter
plate, maple backplate (balls and pegs throw their shadows on it), 78 brass pegs, 13 bins with
brass dividers and trapdoor floors, a glass hopper with a pile of pearl balls, a crest, and a
bucket elevator on the right that carries the balls back up at the end.
Small life: pawn-figure spectators on tiered benches (they bob, turn and applaud), a pigeon on
the gallery rail, gas-lamp flames, the wall clock, dust motes in the sunbeams.
Every periodic motion has a whole number of cycles per minute (seamless loop)."""
import math
import numpy as np
from reelkit.config import dirv, nrm
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, new_group
import board as B
import timeline as T

rng = np.random.default_rng(14)

# ---------------------------------------------------------------- room shell
HX0, HX1, HZ0, HZ1, HY = -2.8, 2.8, -1.8, 5.8, 7.0
WT = 0.25                                   # wall thickness (window reveals, cut edges)
SLAB = 0.3
SX0, SX1, SZ1 = -2.1, 2.1, 0.25             # stage extent (it runs back to the back wall)
YS = B.Y_STAGE
WAIN = 1.15                                 # wainscot height on the walls
GAL_Y, GAL_Z = 4.55, HZ0 + 0.42             # gallery floor height and front edge
RAIL_Y = GAL_Y + 0.5

SUN = dirv(226, 23)                         # toward the sun: front-left, low
SUN_RGB = np.array([1.0, 0.86, 0.66])

# two tall arched windows in the left wall: rectangle up to the spring line + semicircular arch
WINDOWS = [(1.2, 3.35), (-1.45, 0.25), (3.95, 5.35)]   # z ranges: W1 lights the machine, W2 the back-left, W3 the benches
WIN_Y0, WIN_SPRING = 1.4, 5.8
TRANSOM = 0.035
MULL = 0.03


def win_open(z, y):
    z, y = np.asarray(z, float), np.asarray(y, float)
    m = np.zeros(np.shape(z), bool)
    for z0, z1 in WINDOWS:
        zc, r = 0.5 * (z0 + z1), 0.5 * (z1 - z0)
        rect = (z >= z0) & (z <= z1) & (y >= WIN_Y0) & (y <= WIN_SPRING)
        arch = ((z - zc) ** 2 + (y - WIN_SPRING) ** 2 <= r * r) & (y > WIN_SPRING)
        m |= rect | arch
    return m


def win_bars(z, y):
    """glazing bars: a transom at the spring line, one mullion in the arch, a thin frame"""
    z, y = np.asarray(z, float), np.asarray(y, float)
    on = np.zeros(np.shape(z), bool)
    for z0, z1 in WINDOWS:
        zc, r = 0.5 * (z0 + z1), 0.5 * (z1 - z0)
        inside = (z >= z0 - 1e-3) & (z <= z1 + 1e-3)
        on |= inside & (np.abs(y - WIN_SPRING) < TRANSOM)
        on |= inside & (y > WIN_SPRING) & (np.abs(z - zc) < MULL * 0.5)
        on |= inside & (y <= WIN_SPRING) & ((z - z0 < 0.04) | (z1 - z < 0.04) | (y - WIN_Y0 < 0.05))
        rr = np.sqrt((z - zc) ** 2 + (y - WIN_SPRING) ** 2)
        on |= (y > WIN_SPRING) & (rr > r - 0.04) & (rr <= r)
    return on


def win_lit(P, d=SUN):
    """1 where the ray from P toward the sun leaves through a window's glass"""
    P = np.atleast_2d(np.asarray(P, float))
    out = np.zeros(len(P), np.float32)
    lit = np.ones(len(P), bool)
    for xw, bars in ((HX0, True), (HX0 - WT, False)):          # both faces of the wall: the reveal clips oblique rays
        s = (xw - P[:, 0]) / d[0]
        z, y = P[:, 2] + s * d[2], P[:, 1] + s * d[1]
        lit &= win_open(z, y)
        if bars:
            lit &= ~win_bars(z, y)
    ok = P[:, 0] > HX0 - 1e-3
    out[ok & lit] = 1.0
    return out


# ---------------------------------------------------------------- builders
def bar(a, b, w, h=None, up=(0, 1, 0), caps=True):
    """square beam between two points"""
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


def cyl_z(cx, cy, z0, z1, r, n=10, rot=0.0, cap=True):
    """cylinder along z (pegs, clock face), front cap at z1"""
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
    return out


def quad_z(x0, x1, y0, y1, z, back=False):
    n = np.array([0, 0, -1.0 if back else 1.0])
    return [(np.array([[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]], float), n)]


def xform(faces, R, t):
    R = np.asarray(R, float)
    return [(Q @ R.T + t, n @ R.T) for Q, n in faces]


def rot_y(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def rot_x(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])


def rot_z(a):
    c, s = math.cos(a), math.sin(a)
    return np.array([[c, -s, 0], [s, c, 0], [0, 0, 1]])


WALNUT = (104, 62, 40)
WALNUT_D = (78, 46, 32)
BRASS = (230, 178, 82)
BRASS_D = (176, 128, 56)
MAPLE = (150, 102, 74)      # warm cherry-walnut backplate (name kept for history)

# object lists by draw phase: BACK (behind the machine), MACH (the machine), FRONT (in front)
BACK, MACH, FRONT = [], [], []
FEET = []          # hall-floor contact AO  (x, z, r, k)
SFEET = []         # stage-top contact AO

# ---------------------------------------------------------------- the machine
MG = new_group()
CZ0, CZ1 = B.ZB - 0.16, B.ZB + 0.16                       # base cabinet depth
base = box(0, 0.5 * (YS + B.Y_BF), 0.5 * (CZ0 + CZ1), 0.86, 0.5 * (B.Y_BF - YS), 0.5 * (CZ1 - CZ0))
base += box(0, B.Y_BF - 0.033, 0.5 * (CZ0 + CZ1) + 0.01, 0.9, 0.03, 0.5 * (CZ1 - CZ0) + 0.02)     # moulding (top just under the bin floor)
base += box(0, YS + 0.03, 0.5 * (CZ0 + CZ1) + 0.01, 0.9, 0.03, 0.5 * (CZ1 - CZ0) + 0.02)
MACH.append(Obj(base, WALNUT, 'base', grp=MG))
for sx in (-1, 1):
    for z in (CZ0 + 0.04, CZ1 - 0.04):
        MACH.append(Obj(lathe(sx * 0.8, z, [(0, 0.035), (0.03, 0.045), (0.05, 0.03)], 6, cy=YS - 0.0), BRASS, 'foot', grp=MG))
SFEET.append((0, B.ZB, 0.85, 0.5))
PLATE = (-0.3, 0.3, YS + 0.1, YS + 0.27, CZ1 + 0.005)    # brass counter plate (x0, x1, y0, y1, z)
MACH.append(Obj(quad_z(*PLATE), BRASS, 'plate', grp=MG, cast=False))
# walnut posts either side of the backplate, top rail, glass-front frame strips
POST_Z0, POST_Z1 = B.ZB - 0.07, B.ZB + 0.095
POSTS = []
for sx in (-1, 1):
    POSTS.append(Obj(box(sx * 0.775, 0.5 * (B.Y_BF + B.Y_TOP + 0.12), 0.5 * (POST_Z0 + POST_Z1), 0.055, 0.5 * (B.Y_TOP + 0.12 - B.Y_BF),
                         0.5 * (POST_Z1 - POST_Z0)), WALNUT, 'post', grp=MG))
    POSTS.append(Obj(box(sx * 0.775, B.Y_TOP + 0.15, 0.5 * (POST_Z0 + POST_Z1), 0.07, 0.03, 0.5 * (POST_Z1 - POST_Z0) + 0.01), WALNUT_D, 'cap', grp=MG))
    POSTS.append(Obj(lathe(sx * 0.775, 0.5 * (POST_Z0 + POST_Z1), [(0, 0.04), (0.03, 0.045), (0.07, 0.025), (0.1, 0.0)], 6, cy=B.Y_TOP + 0.18),
                     BRASS, 'finial', grp=MG))
MACH += POSTS
BASE_TOP = Obj(box(0, B.Y_BF + 0.005, B.ZB + 0.035, 0.72, 0.006, 0.04), BRASS_D, 'binfloor', grp=MG, cast=False)

# pegs (static) and bin dividers
PEG_FACES = [cyl_z(x, y, B.ZB, B.ZB + B.PEG_LEN, B.R_PEG, 8) for x, y in B.PEG_XY]
DIV_X = [(b - 6.5) * B.DX for b in range(B.NB + 1)]
DIVIDERS = []
for i, x in enumerate(DIV_X):
    xx = float(np.clip(x, B.X0 + 0.004, B.X1 - 0.004))
    DIVIDERS.append(box(xx, 0.5 * (B.Y_BF + B.Y_BT), B.ZB + 0.019, B.DIV_T / 2, 0.5 * (B.Y_BT - B.Y_BF), 0.019))
DIV_CAPS = [box(float(np.clip(x, B.X0 + 0.006, B.X1 - 0.006)), B.Y_BT + 0.004, B.ZB + 0.02, 0.007, 0.006, 0.021) for x in DIV_X]

# glass hopper (flat funnel) above the peg field, and the crest
HOP_Y0, HOP_Y1 = B.Y_TOP, 4.05                             # neck .. top
HOP_W0, HOP_W1 = 0.05, 0.52


def hop_w(y):
    return HOP_W0 + (np.asarray(y, float) - HOP_Y0) * (HOP_W1 - HOP_W0) / (HOP_Y1 - HOP_Y0)


HOP_RIM = []
for sx in (-1, 1):
    HOP_RIM += bar([sx * HOP_W0, HOP_Y0, B.ZBALL], [sx * HOP_W1, HOP_Y1, B.ZBALL], 0.022, 0.08, up=(0, 0, 1))
HOP_RIM += bar([-HOP_W1 - 0.02, HOP_Y1, B.ZBALL], [HOP_W1 + 0.02, HOP_Y1, B.ZBALL], 0.026, 0.085, up=(0, 0, 1))
HOP_RIM += box(0, 0.5 * (B.Y_OUT + HOP_Y0) - 0.005, B.ZBALL, 0.032, 0.5 * (HOP_Y0 - B.Y_OUT) + 0.01, 0.035)    # neck collar
MACH.append(Obj(HOP_RIM, BRASS, 'hoprim', grp=MG))
HOP_GLASS = [(np.array([[-HOP_W0, HOP_Y0, B.ZGLASS], [HOP_W0, HOP_Y0, B.ZGLASS], [HOP_W1, HOP_Y1, B.ZGLASS], [-HOP_W1, HOP_Y1, B.ZGLASS]]),
              np.array([0, 0, 1.0]))]
# hopper back panel (walnut) so the pile reads against something dark
HOP_BACK = [(np.array([[-HOP_W0, HOP_Y0, B.ZB], [HOP_W0, HOP_Y0, B.ZB], [HOP_W1, HOP_Y1, B.ZB], [-HOP_W1, HOP_Y1, B.ZB]]), np.array([0, 0, 1.0]))]
crest = box(0, HOP_Y1 + 0.09, B.ZB + 0.02, 0.6, 0.06, 0.07)
crest += [face([[-0.6, HOP_Y1 + 0.15, B.ZB + 0.09], [0.6, HOP_Y1 + 0.15, B.ZB + 0.09], [0, HOP_Y1 + 0.36, B.ZB + 0.09]], [0, HOP_Y1 + 0.2, B.ZB]),
          face([[-0.6, HOP_Y1 + 0.15, B.ZB - 0.05], [0, HOP_Y1 + 0.36, B.ZB - 0.05], [0.6, HOP_Y1 + 0.15, B.ZB - 0.05]], [0, HOP_Y1 + 0.2, B.ZB + 0.2]),
          face([[0.6, HOP_Y1 + 0.15, B.ZB + 0.09], [0.6, HOP_Y1 + 0.15, B.ZB - 0.05], [0, HOP_Y1 + 0.36, B.ZB - 0.05], [0, HOP_Y1 + 0.36, B.ZB + 0.09]], [-0.3, HOP_Y1, B.ZB]),
          face([[-0.6, HOP_Y1 + 0.15, B.ZB - 0.05], [-0.6, HOP_Y1 + 0.15, B.ZB + 0.09], [0, HOP_Y1 + 0.36, B.ZB + 0.09], [0, HOP_Y1 + 0.36, B.ZB - 0.05]], [0.3, HOP_Y1, B.ZB])]
MACH.append(Obj(crest, WALNUT, 'crest', grp=MG))
CREST_PLATE = (-0.3, 0.3, HOP_Y1 + 0.05, HOP_Y1 + 0.13, B.ZB + 0.092)
MACH.append(Obj(quad_z(*CREST_PLATE), BRASS, 'crestplate', grp=MG, cast=False))
MACH.append(Obj(lathe(0, B.ZB + 0.02, [(0, 0.05), (0.04, 0.06), (0.09, 0.035), (0.13, 0.0)], 8, cy=HOP_Y1 + 0.35), BRASS, 'finial', grp=MG))

# bucket elevator on the right
EL_X0, EL_X1 = 0.86, 1.06
EL_Y0, EL_Y1 = YS + 0.06, 4.2
EL_XC = 0.5 * (EL_X0 + EL_X1)
el = box(EL_XC, 0.5 * (EL_Y0 + EL_Y1), B.ZB - 0.04, 0.1, 0.5 * (EL_Y1 - EL_Y0), 0.02)          # back board
for sx in (-1, 1):
    el += box(EL_XC + sx * 0.1, 0.5 * (EL_Y0 + EL_Y1), B.ZB + 0.01, 0.012, 0.5 * (EL_Y1 - EL_Y0), 0.05)
MACH.append(Obj(el, WALNUT_D, 'elevator', grp=MG))
SPROCKETS = [np.array([EL_XC, EL_Y0 + 0.12, B.ZB + 0.03]), np.array([EL_XC, EL_Y1 - 0.12, B.ZB + 0.03])]
CHUTE = (np.array([EL_XC - 0.02, EL_Y1 - 0.05, B.ZBALL]), np.array([0.36, HOP_Y1 - 0.02, B.ZBALL]))
MACH.append(Obj(bar(CHUTE[0] + [0, -0.03, 0], CHUTE[1] + [0, -0.03, 0], 0.03, 0.07, up=(0, 0, 1)), BRASS_D, 'chute', grp=MG))
BUCKET_GAP = 0.32
EL_RUN = (EL_Y0 + 0.16, EL_Y1 - 0.16)


def bucket_offset(t):
    """chain position (units); whole bucket gaps per minute, so the loop is seamless"""
    t = t % T.DUR
    a, b = T.T_LIFT[0] - 0.4, T.T_LIFT[1] + 0.4
    u = min(1.0, max(0.0, (t - a) / (b - a)))
    u = u * u * u * (u * (6 * u - 15) + 10)
    return BUCKET_GAP * (15 * t / 60.0 + 11 * u)


# ---------------------------------------------------------------- hopper pile (front layer of pearl balls)
def _pile():
    pts = []
    y = HOP_Y0 + B.R_BALL
    r = 0
    while y < HOP_Y1 - B.R_BALL:
        w = float(hop_w(y)) - B.R_BALL * 1.1
        off = (r % 2) * B.R_BALL
        xs = np.arange(-w + off, w + 1e-6, 2 * B.R_BALL)
        for x in xs:
            pts.append((x + rng.uniform(-0.002, 0.002), y + rng.uniform(-0.002, 0.002)))
        y += B.ROW_H
        r += 1
    return np.array(pts)


PILE = _pile()
PILE_TINT = rng.uniform(0.88, 1.04, (len(PILE), 1)) * np.array([1, 1, 1.0])[None]

# ---------------------------------------------------------------- stage dressing
# lectern with a gas lamp, front right of the stage
LEC = np.array([1.55, YS, 0.0])
g = new_group()
lec = box(LEC[0], YS + 0.5, LEC[2], 0.2, 0.5, 0.16)
lec += [face([[LEC[0] - 0.24, YS + 1.0, LEC[2] - 0.2], [LEC[0] + 0.24, YS + 1.0, LEC[2] - 0.2], [LEC[0] + 0.24, YS + 1.12, LEC[2] + 0.2],
              [LEC[0] - 0.24, YS + 1.12, LEC[2] + 0.2]], [LEC[0], YS, LEC[2]])]
FRONT.append(Obj(lec, WALNUT, 'lectern', grp=g))
FRONT.append(Obj(box(LEC[0] - 0.05, YS + 1.1, LEC[2] + 0.02, 0.1, 0.012, 0.07, rot=0.2), (236, 226, 200), 'notes', grp=g, cast=False))
SFEET.append((LEC[0], LEC[2], 0.2, 0.45))
LAMP_L = np.array([LEC[0] + 0.14, YS + 1.38, LEC[2] - 0.06])
g = new_group()
FRONT.append(Obj(lathe(LAMP_L[0], LAMP_L[2], [(0, 0.05), (0.02, 0.05), (0.12, 0.015), (0.2, 0.02)], 6, cy=YS + 1.06), BRASS, 'lampbase', grp=g))
FRONT.append(Obj(lathe(LAMP_L[0], LAMP_L[2], [(0, 0.03), (0.05, 0.055), (0.12, 0.05), (0.16, 0.03)], 8, cy=LAMP_L[1] - 0.1), (255, 222, 160), 'chimney', grp=g, cast=False))
# tall gas lamp standard, front left of the stage
LAMP_S = np.array([-1.5, YS + 2.1, -0.15])
g = new_group()
ls = prism(LAMP_S[0], LAMP_S[2], 0.03, YS, LAMP_S[1] - 0.12, 6) + prism(LAMP_S[0], LAMP_S[2], 0.14, YS, YS + 0.06, 8)
FRONT.append(Obj(ls, (60, 56, 52), 'lampstd', grp=g))
FRONT.append(Obj(lathe(LAMP_S[0], LAMP_S[2], [(0, 0.04), (0.08, 0.075), (0.2, 0.07), (0.26, 0.04)], 8, cy=LAMP_S[1] - 0.13), (255, 222, 160), 'chimney', grp=g, cast=False))
FRONT.append(Obj(lathe(LAMP_S[0], LAMP_S[2], [(0, 0.1), (0.05, 0.06), (0.1, 0.02)], 8, cy=LAMP_S[1] + 0.12), (54, 60, 56), 'shade', grp=g))
SFEET.append((LAMP_S[0], LAMP_S[2], 0.14, 0.4))
# small table with a globe and a crate of spare balls, left of the machine
TAB = np.array([-1.3, YS, -0.95])
g = new_group()
tb = prism(TAB[0], TAB[2], 0.26, YS + 0.62, YS + 0.66, 10) + prism(TAB[0], TAB[2], 0.03, YS, YS + 0.62, 6) + prism(TAB[0], TAB[2], 0.16, YS, YS + 0.04, 8)
BACK.append(Obj(tb, WALNUT, 'table', grp=g))
BACK.append(Obj(prism(TAB[0], TAB[2], 0.04, YS + 0.66, YS + 0.76, 6), BRASS, 'globestand', grp=g))
BACK.append(Obj(icosa([TAB[0], YS + 0.9, TAB[2]], 0.15, jit=0.02, rng=np.random.default_rng(2)), (70, 130, 150), 'globe', grp=g))
SFEET.append((TAB[0], TAB[2], 0.18, 0.4))
g = new_group()
BACK.append(Obj(box(-1.75, YS + 0.16, -0.55, 0.2, 0.16, 0.15, rot=0.3), (150, 110, 70), 'crate', grp=g))
SFEET.append((-1.75, -0.55, 0.2, 0.4))
CRATE_BALLS = [(-1.75 + rng.uniform(-0.15, 0.15), YS + 0.33, -0.55 + rng.uniform(-0.1, 0.1)) for _ in range(14)]
# bookcase against the back wall, right
g = new_group()
BC = (1.3, 2.55, HZ0 + 0.02, HZ0 + 0.36, YS, YS + 2.5)
bx0, bx1, bz0, bz1, by0, by1 = BC
bc = box(0.5 * (bx0 + bx1), 0.5 * (by0 + by1), bz0 + 0.02, 0.5 * (bx1 - bx0), 0.5 * (by1 - by0), 0.02)
for sx in (bx0, bx1):
    bc += box(sx, 0.5 * (by0 + by1), 0.5 * (bz0 + bz1), 0.025, 0.5 * (by1 - by0), 0.5 * (bz1 - bz0))
shelves = [by0 + 0.05 + i * 0.48 for i in range(6)]
for y in shelves:
    bc += box(0.5 * (bx0 + bx1), y, 0.5 * (bz0 + bz1), 0.5 * (bx1 - bx0), 0.02, 0.5 * (bz1 - bz0))
BACK.append(Obj(bc, WALNUT_D, 'bookcase', grp=g))
books = []
bcols = [(150, 46, 42), (44, 74, 120), (196, 156, 70), (60, 104, 70), (120, 60, 96), (176, 120, 70), (90, 40, 30)]
for y in shelves[:-1]:
    x = bx0 + 0.04
    while x < bx1 - 0.08:
        w = rng.uniform(0.035, 0.07)
        hgt = rng.uniform(0.28, 0.4)
        books.append((box(x + w / 2, y + 0.02 + hgt / 2, bz1 - 0.12, w / 2 - 0.003, hgt / 2, 0.11), bcols[int(rng.integers(len(bcols)))]))
        x += w
for fb, c in books:
    BACK.append(Obj(fb, c, 'book', grp=g, cast=False))
# wall sconces (gas) on the back wall either side of the machine
SCONCES = [np.array([-1.0, 2.75, HZ0 + 0.16]), np.array([1.15, 3.15, HZ0 + 0.16])]
for sp in SCONCES:
    g = new_group()
    BACK.append(Obj(bar([sp[0], sp[1] - 0.25, HZ0], [sp[0], sp[1] - 0.12, sp[2]], 0.025, up=(0, 0, 1)) + box(sp[0], sp[1] - 0.25, HZ0 + 0.01, 0.05, 0.08, 0.01),
                    BRASS, 'sconce', grp=g))
    BACK.append(Obj(lathe(sp[0], sp[2], [(0, 0.03), (0.05, 0.06), (0.13, 0.05), (0.17, 0.03)], 8, cy=sp[1] - 0.12), (255, 222, 160), 'chimney', grp=g, cast=False))
# gallery along the back wall: floor slab + balustrade
g = new_group()
gal = box(0, GAL_Y - 0.06, 0.5 * (HZ0 + GAL_Z), 0.5 * (HX1 - HX0), 0.06, 0.5 * (GAL_Z - HZ0))
gal += box(0, RAIL_Y, GAL_Z - 0.03, 0.5 * (HX1 - HX0), 0.035, 0.04)
for x in np.arange(HX0 + 0.1, HX1, 0.16):
    gal += prism(x, GAL_Z - 0.03, 0.022, GAL_Y, RAIL_Y - 0.03, 5)
BACK.append(Obj(gal, WALNUT, 'gallery', grp=g))
GAL_PAWNS = [(-2.2, 0), (1.6, 1), (2.15, 2)]               # a few standing spectators up there (x, look)
# wall clock above the gallery
CLOCK = np.array([0.0, 6.05, HZ0 + 0.04])
g = new_group()
BACK.append(Obj(cyl_z(CLOCK[0], CLOCK[1], HZ0, HZ0 + 0.06, 0.34, 16), WALNUT, 'clockcase', grp=g))
BACK.append(Obj(cyl_z(CLOCK[0], CLOCK[1], HZ0 + 0.06, HZ0 + 0.065, 0.29, 16), (240, 230, 204), 'dial', grp=g, cast=False))
# gasolier hanging over the audience
GASO = np.array([0.0, 5.5, 1.6])
g = new_group()
gs = prism(GASO[0], GASO[2], 0.02, GASO[1], HY, 6) + prism(GASO[0], GASO[2], 0.08, GASO[1] - 0.1, GASO[1], 8)
GASO_ARMS = []
for k in range(5):
    a = 2 * math.pi * k / 5 + 0.3
    tip = GASO + [0.42 * math.cos(a), -0.2, 0.42 * math.sin(a)]
    gs += bar(GASO + [0, -0.05, 0], tip, 0.025, up=(0, 1, 0))
    GASO_ARMS.append(tip + [0, 0.1, 0])
FRONT.append(Obj(gs, BRASS, 'gasolier', grp=g, cast=False))
for tip in GASO_ARMS:
    FRONT.append(Obj(lathe(tip[0], tip[2], [(0, 0.025), (0.04, 0.045), (0.1, 0.04), (0.13, 0.025)], 6, cy=tip[1] - 0.1), (255, 222, 160), 'chimney', grp=g, cast=False))

LAMPS = [  # (position, radius, strength): warm local pools, kept small so the sun stays the key
    (SCONCES[0] - [0, 0.05, 0], 0.9, 0.7),
    (SCONCES[1] - [0, 0.05, 0], 0.9, 0.7),
    (LAMP_L - [0, 0.05, 0], 0.9, 0.75),
    (LAMP_S - [0, 0.05, 0], 1.0, 0.75),
    (GASO + [0, -0.2, 0], 1.7, 0.8),
]

# ---------------------------------------------------------------- tiered benches + pawn spectators
ROWS_Z = [1.25, 1.9, 2.55, 3.2, 3.85, 4.5]
RISE = 0.2
BLOCKS = [(-2.65, -0.85), (0.85, 2.65)]
BENCH_G = new_group()
benches = []
for i, z in enumerate(ROWS_Z):
    y0 = i * RISE
    for x0, x1 in BLOCKS:
        xc, hw = 0.5 * (x0 + x1), 0.5 * (x1 - x0)
        fb = []
        if y0 > 0:
            fb += box(xc, y0 / 2, z, hw, y0 / 2, 0.33)                        # riser step
        fb += box(xc, y0 + 0.36, z + 0.05, hw, 0.03, 0.14)                   # seat
        fb += box(xc, y0 + 0.62, z + 0.22, hw, 0.2, 0.02)                    # backrest
        fb += box(xc, y0 + 0.17, z - 0.06, hw - 0.02, 0.17, 0.015)           # front board
        benches.append(Obj(fb, (132, 84, 54), 'bench', grp=new_group()))
        FEET.append((xc, z + 0.05, hw * 0.6, 0.25))
FRONT += benches

PAWN_COLS = [(52, 58, 92), (110, 44, 46), (64, 84, 58), (92, 64, 48), (40, 40, 46), (120, 96, 64), (70, 52, 92)]
SKIN = (238, 206, 170)
PAWNS = []        # dicts: base position, colour, hat, phase
for i, z in enumerate(ROWS_Z):
    y0 = i * RISE
    for x0, x1 in BLOCKS:
        x = x0 + 0.24
        while x < x1 - 0.2:
            if rng.uniform() > 0.27:
                PAWNS.append(dict(p=np.array([x + rng.uniform(-0.04, 0.04), y0 + 0.39, z + 0.06]), col=PAWN_COLS[int(rng.integers(len(PAWN_COLS)))],
                                  hat=int(rng.integers(0, 3)), ph=float(rng.uniform(0, 1)), s=float(rng.uniform(0.9, 1.06))))
            x += 0.4
for gx, look in GAL_PAWNS:
    PAWNS.append(dict(p=np.array([gx, GAL_Y, HZ0 + 0.22]), col=PAWN_COLS[look + 2], hat=1, ph=0.3 * look, s=1.15, stand=True))

PAWN_BODY = lathe(0, 0, [(0.0, 0.14), (0.04, 0.13), (0.07, 0.09), (0.2, 0.065), (0.3, 0.05), (0.33, 0.085), (0.36, 0.075)], 8)
PAWN_HEAD = icosa([0, 0.44, 0], 0.085, jit=0.03, rng=np.random.default_rng(7))


def _hat(kind):
    if kind == 1:      # top hat
        return lathe(0, 0, [(0.0, 0.11), (0.012, 0.11), (0.014, 0.06), (0.14, 0.065)], 8, cy=0.5)
    if kind == 2:      # bonnet / bowler
        return lathe(0, 0, [(0.0, 0.1), (0.01, 0.1), (0.02, 0.075), (0.07, 0.06), (0.09, 0.0)], 8, cy=0.49)
    return []


HATS = {k: _hat(k) for k in range(3)}


def pawn_state(pw, t):
    """offset, lean and turn of a pawn: idle bob, follow the golden ball, applaud at the drop"""
    w = 2 * math.pi / 60.0
    bob = 0.012 * math.sin(w * (8 + int(pw['ph'] * 5)) * t + pw['ph'] * 6.28)
    turn = 0.25 * math.sin(w * 3 * t + pw['ph'] * 9)
    ap = 0.0
    if T.T_DROP + 1.0 <= t < T.T_DROP + 7.5:               # applause: quick hops on the beat
        u = (t - T.T_DROP + pw['ph'] * 0.08) % T.BEAT
        ap = 0.05 * math.exp(-u / 0.1) * min(1.0, (T.T_DROP + 7.5 - t) / 1.5)
    if 20.5 <= t < 25.5:                                   # everyone leans toward the edge bin
        turn += 0.45 * min(1, (t - 20.5) / 0.8) * min(1, (25.5 - t) / 0.8)
    return bob + ap, turn


def pawn_faces(pw, t):
    off, turn = pawn_state(pw, t)
    R = rot_y(turn)
    s = pw['s']
    base = pw['p'] + [0, off, 0]
    body = [(Q * s @ R.T + base, n @ R.T) for Q, n in PAWN_BODY]
    head = [(Q * s @ R.T + base, n @ R.T) for Q, n in PAWN_HEAD]
    hat = [(Q * s @ R.T + base, n @ R.T) for Q, n in HATS[pw['hat']]]
    return body, head, hat


# ---------------------------------------------------------------- the pigeon on the gallery rail
def pigeon(t):
    """position, heading and pose: walks along the rail (2 round trips per minute), head bobs, a flap"""
    w = 2 * math.pi / 60.0
    ph = (2 * t / 60.0) % 1.0
    tri = 2 * ph if ph < 0.5 else 2 - 2 * ph
    tri = tri * tri * (3 - 2 * tri)
    x = -2.3 + 1.25 * tri
    heading = 1.0 if ph < 0.5 else -1.0
    stride = abs(math.sin(w * 60 * t))
    bob = 0.012 * stride
    flap = 1.0 if (t % 30.0) > 27.6 else 0.0
    return np.array([x, RAIL_Y + 0.035 + 0.003 * stride, GAL_Z - 0.03]), heading, bob, flap


def pigeon_faces(t):
    p, hd, bob, flap = pigeon(t)
    r2 = np.random.default_rng(31)
    body = icosa([0, 0.055, 0], 0.05, (1.4, 0.9, 0.85), jit=0.05, rng=r2)
    head = icosa([0.06 + 0.012 * bob / 0.012, 0.115, 0], 0.026, jit=0.03, rng=r2)
    beak = [face([[0.084, 0.115, -0.006], [0.084, 0.115, 0.006], [0.104, 0.108, 0]], [0.06, 0.115, 0])]
    tail = [face([[-0.06, 0.06, -0.025], [-0.06, 0.06, 0.025], [-0.12, 0.045, 0]], [0, 0, 0])]
    wings = []
    for sz in (-1, 1):
        lift = 0.07 * flap
        wings.append(face([[0.03, 0.08, 0.035 * sz], [-0.05, 0.075, 0.04 * sz], [-0.01, 0.07 + lift, (0.06 + 0.05 * flap) * sz]], [0, 0.05, 0]))
    R = rot_y(0 if hd > 0 else math.pi)
    out = {}
    for name, fcs in (('body', body + tail), ('head', head), ('beak', beak), ('wing', wings)):
        out[name] = [(Q @ R.T + p, n @ R.T) for Q, n in fcs]
    return out


# ---------------------------------------------------------------- dust motes in the sunbeams
MOTES = np.random.default_rng(5).uniform([-2.6, 0.3, -1.6], [1.8, 5.5, 4.4], (260, 3))
SUBJECT = np.array([0.08, 2.2, B.ZBALL])
