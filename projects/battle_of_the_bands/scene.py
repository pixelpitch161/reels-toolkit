"""The floating festival stage in the vibrant void (static geometry + a few animated props).

World: y up, the round LED wall faces +z (toward the camera), its centre at WC, LED radius RW.
  - octagonal stage slab with dark boards, a warm footlight strip along the front edge
  - the LED wall: bezel ring, plinth, square truss with four moving heads and pennants
  - four speaker stacks in team colours: two hung line arrays (top corners), two ground stacks
  - a prop per team on the wall rim: SAND hopper on a rail (slides with the pour), BALL launcher tube,
    RIPPLE emitter cone, VINE planter
  - the crowd in front (built per frame: bobbing, raised arms, glowing phones), a drone, sky searchlights
Team order everywhere: 0 SAND, 1 BALL, 2 RIPPLE, 3 VINE (= battle team index)."""
import math
import numpy as np
from reelkit.config import dirv
from reelkit.geometry import Obj, box, prism, icosa, lathe, face, new_group

import episode as E

rng = np.random.default_rng(20)
N = E.N
WC = np.array([0.0, 3.1, 0.0])       # LED wall centre
RW = 2.0                             # LED radius (m): tile = 4 / 72 m
BEZ = 0.26                           # bezel width
TILE = 2 * RW / N

TEAM_COL = [np.array(c, float) for c in ((255, 186, 60), (176, 96, 255), (40, 222, 206), (120, 232, 72))]
TEAM_DARK = [c * 0.42 for c in TEAM_COL]

# key light: the moon is drawn at MOON_DRAW (visible at the top of frame); shading/shadows use KEY (a little higher,
# from behind-left, so the stack shadows land on the stage boards)
MOON_DRAW = dirv(-9, 9.5)
KEY = dirv(-32, 30)
KEY_COL = np.array([0.62, 0.66, 0.92])


def tile_xy(i, j):
    """world (x, y) of tile centre (row i, column j) on the wall plane z = 0"""
    u = (np.asarray(j) + 0.5) / N * 2 - 1
    v = 1 - (np.asarray(i) + 0.5) / N * 2
    return WC[0] + u * RW, WC[1] + v * RW


def rim_point(deg, r=RW + BEZ * 0.5, z=0.0):
    a = math.radians(deg)
    return np.array([WC[0] + r * math.cos(a), WC[1] + r * math.sin(a), z])


OBJS = []

# ------------------------------------------------------------------ stage slab (top y = 0)
STAGE_C = np.array([0.0, 0.0, 1.15])
STAGE_R = 4.1
SLAB = new_group()
OBJS.append(Obj(prism(STAGE_C[0], STAGE_C[2], STAGE_R, -0.55, -0.02, 8, r1=STAGE_R, rot=math.pi / 8), (62, 58, 84), 'slab', grp=SLAB, cast=False))
OBJS.append(Obj(prism(STAGE_C[0], STAGE_C[2], STAGE_R * 0.86, -1.6, -0.55, 8, r1=STAGE_R, rot=math.pi / 8), (46, 40, 70), 'slab_under', grp=SLAB, cast=False))
OBJS.append(Obj(prism(STAGE_C[0], STAGE_C[2], STAGE_R * 0.35, -3.6, -1.6, 8, r1=STAGE_R * 0.86, rot=math.pi / 8), (36, 30, 58), 'slab_root', grp=SLAB, cast=False))

# ------------------------------------------------------------------ the wall: bezel ring, plinth, back
WALL = new_group()


def ring_faces(c, r0, r1, z0, z1, n=36):
    out = []
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        p = lambda r, a, z: np.array([c[0] + r * math.cos(a), c[1] + r * math.sin(a), z])
        out.append(face([p(r0, a0, z1), p(r1, a0, z1), p(r1, a1, z1), p(r0, a1, z1)], c + [0, 0, -5]))      # front
        am = (a0 + a1) / 2
        out.append(face([p(r1, a0, z1), p(r1, a0, z0), p(r1, a1, z0), p(r1, a1, z1)], c + [0, 0, (z0 + z1) / 2]))   # outer rim
        out.append(face([p(r0, a0, z0), p(r0, a0, z1), p(r0, a1, z1), p(r0, a1, z0)], p(r0 + 1, am, (z0 + z1) / 2)))  # inner lip
    return out


OBJS.append(Obj(ring_faces(WC, RW - 0.01, RW + BEZ, -0.22, 0.12), (78, 82, 112), 'bezel', grp=WALL))
OBJS.append(Obj(ring_faces(WC, RW + BEZ, RW + BEZ + 0.05, -0.18, 0.08), (196, 200, 228), 'bezel_trim', grp=WALL, cast=False))
_back = []
for k in range(24):
    a0, a1 = 2 * math.pi * k / 24, 2 * math.pi * (k + 1) / 24
    _back.append(face([WC + [0, 0, -0.22], WC + [(RW + BEZ) * math.cos(a1), (RW + BEZ) * math.sin(a1), -0.22],
                       WC + [(RW + BEZ) * math.cos(a0), (RW + BEZ) * math.sin(a0), -0.22]], WC + [0, 0, 1]))
OBJS.append(Obj(_back, (40, 42, 60), 'wall_back', grp=WALL))
PLINTH_TOP = WC[1] - RW - BEZ + 0.08
OBJS.append(Obj(box(0, PLINTH_TOP / 2, -0.05, 1.05, PLINTH_TOP / 2, 0.42), (70, 66, 96), 'plinth', grp=WALL))
OBJS.append(Obj(box(0, 0.06, 0.55, 1.6, 0.06, 0.4), (90, 86, 118), 'step', grp=WALL))

# ------------------------------------------------------------------ truss + moving heads + pennants
TRUSS = new_group()
TX, TY, TZ = 2.95, 6.55, -0.35
for sx in (-1, 1):
    OBJS.append(Obj(box(sx * TX, TY / 2, TZ, 0.13, TY / 2, 0.13), (150, 156, 176), 'truss_post', grp=TRUSS))
    OBJS.append(Obj(box(sx * TX, 0.07, TZ, 0.34, 0.07, 0.34), (90, 92, 110), 'truss_foot', grp=TRUSS))
OBJS.append(Obj(box(0, TY, TZ, TX + 0.13, 0.13, 0.13), (150, 156, 176), 'truss_beam', grp=TRUSS))
TRUSS_LINES = []                    # lattice diagonals (drawn as lines)
for sx in (-1, 1):
    for k in range(13):
        y0, y1 = k * TY / 13, (k + 1) * TY / 13
        TRUSS_LINES.append((np.array([sx * TX - 0.13 * sx, y0, TZ + 0.135]), np.array([sx * TX + 0.13 * sx, y1, TZ + 0.135])))
for k in range(18):
    x0, x1 = -TX + k * 2 * TX / 18, -TX + (k + 1) * 2 * TX / 18
    TRUSS_LINES.append((np.array([x0, TY - 0.13, TZ + 0.135]), np.array([x1, TY + 0.13, TZ + 0.135])))
HEADS = [np.array([x, TY - 0.32, TZ + 0.1]) for x in (-1.9, -0.65, 0.65, 1.9)]
for h in HEADS:
    g = new_group()
    OBJS.append(Obj(box(h[0], h[1] + 0.12, h[2], 0.14, 0.06, 0.12), (60, 62, 80), 'yoke', grp=g))
    OBJS.append(Obj(lathe(h[0], h[2], [(-0.16, 0.0), (-0.15, 0.11), (0.05, 0.13), (0.06, 0.0)], n=8, cy=h[1]), (52, 54, 72), 'head', grp=g))
PENNANTS = [(-2.2, 0), (-0.75, 1), (0.75, 2), (2.2, 3)]       # (x, team) hanging below the beam

# ------------------------------------------------------------------ speaker stacks (one per team)
# hung line arrays at the top corners (SAND left, BALL right), ground stacks (RIPPLE left, VINE right)
STACKS = []      # per team: dict(cabinets=[(centre, half, tilt)], cones=[(centre, r, normal)], leds=[(p0, p1)], meter=(...))


def cabinet(c, hx, hy, hz, tilt=0.0):
    """speaker cabinet box tilted about x (down-tilt for line arrays)"""
    fs = box(0, 0, 0, hx, hy, hz)
    ca, sa = math.cos(tilt), math.sin(tilt)
    R = np.array([[1, 0, 0], [0, ca, -sa], [0, sa, ca]])
    return [(Q @ R.T + c, n @ R.T) for Q, n in fs], R


def _stack_hung(sx, team):
    cabs, cones, leds = [], [], []
    y = 6.05
    for k in range(4):
        tilt = math.radians(-4 - 7 * k)
        c = np.array([sx * 2.42, y, 0.95])
        fs, R = cabinet(c, 0.46, 0.17, 0.32, tilt)
        cabs.append((fs, c, R))
        front = c + R @ np.array([0, 0, 0.325])
        for dx in (-0.22, 0.22):
            cones.append((front + R @ np.array([dx, 0, 0]), 0.1, R @ np.array([0, 0, 1.0])))
        leds.append((front + R @ np.array([-0.44, -0.15, 0.005]), front + R @ np.array([0.44, -0.15, 0.005])))
        y -= 0.36 + 0.02 * k
    return dict(cabinets=cabs, cones=cones, leds=leds, team=team, hung=True,
                rig=(np.array([sx * 2.42, 6.2, 0.95]), np.array([sx * 2.42, TY, TZ + 0.2])))


def _stack_ground(sx, team):
    cabs, cones, leds = [], [], []
    base = np.array([sx * 2.3, 0.0, 1.9])
    specs = [(0.52, 0.42, 0.42, 0.42, 0.3), (0.52, 0.42, 0.42, 1.27, 0.3), (0.46, 0.26, 0.36, 1.97, 0.14)]
    for k, (hx, hy, hz, cy, cr) in enumerate(specs):
        c = base + [0, cy, 0]
        fs, R = cabinet(c, hx, hy, hz, 0.0 if k < 2 else math.radians(6))
        cabs.append((fs, c, R))
        front = c + R @ np.array([0, 0, hz + 0.005])
        if k < 2:
            cones.append((front, cr, R @ np.array([0, 0, 1.0])))
        else:
            for dx in (-0.2, 0.2):
                cones.append((front + R @ np.array([dx, 0, 0]), cr, R @ np.array([0, 0, 1.0])))
        leds.append((front + R @ np.array([-hx + 0.03, -hy + 0.04, 0]), front + R @ np.array([hx - 0.03, -hy + 0.04, 0])))
    return dict(cabinets=cabs, cones=cones, leds=leds, team=team, hung=False)


STACKS = [_stack_hung(-1, 0), _stack_hung(1, 1), _stack_ground(-1, 2), _stack_ground(1, 3)]
for st in STACKS:
    g = new_group()
    st['grp'] = g
    for fs, c, R in st['cabinets']:
        OBJS.append(Obj(fs, (44, 44, 58), f"cab{st['team']}", grp=g))

# ------------------------------------------------------------------ team props on the wall rim
HOP_RAIL_Y = WC[1] + RW + BEZ + 0.42
HOP_X0, HOP_X1 = -1.55, 0.75
OBJS.append(Obj(box((HOP_X0 + HOP_X1) / 2, HOP_RAIL_Y + 0.33, 0.12, (HOP_X1 - HOP_X0) / 2, 0.035, 0.05), (150, 156, 176), 'rail', grp=TRUSS))
for x in (HOP_X0 + 0.05, HOP_X1 - 0.05):
    OBJS.append(Obj(box(x, (HOP_RAIL_Y + 0.33 + TY) / 2, 0.12 - 0.0, 0.025, (TY - HOP_RAIL_Y - 0.33) / 2, 0.025), (130, 136, 156), 'rail_hanger', grp=TRUSS))


def hopper_faces(x):
    """the sand hopper funnel on its rail at world x"""
    prof = [(-0.16, 0.035), (0.02, 0.05), (0.3, 0.27), (0.42, 0.29), (0.44, 0.0)]
    fs = lathe(x, 0.12, prof, n=8, cy=HOP_RAIL_Y - 0.1)
    car = box(x, HOP_RAIL_Y + 0.38, 0.12, 0.09, 0.05, 0.07)
    return fs, car


def col_to_x(col):
    return WC[0] + (col / N * 2 - 1) * RW


BALL_TUBE_A = 52.0
_bt0 = rim_point(BALL_TUBE_A, RW + BEZ + 0.55, 0.16)
_bt1 = rim_point(BALL_TUBE_A, RW + BEZ - 0.05, 0.16)
BALL_TUBE = (_bt0, _bt1)
RIPPLE_A = 196.0
VINE_A = -32.0


def tube_faces(p0, p1, r, n=8):
    d = p1 - p0
    L = np.linalg.norm(d)
    d = d / L
    a = np.cross(d, [0, 0, 1.0])
    if np.linalg.norm(a) < 1e-6:
        a = np.array([1.0, 0, 0])
    a /= np.linalg.norm(a)
    b = np.cross(d, a)
    out = []
    ring = lambda p, k: p + r * (math.cos(2 * math.pi * k / n) * a + math.sin(2 * math.pi * k / n) * b)
    for k in range(n):
        out.append(face([ring(p0, k), ring(p0, k + 1), ring(p1, k + 1), ring(p1, k)], (p0 + p1) / 2))
    return out


g = new_group()
OBJS.append(Obj(tube_faces(_bt0, _bt1, 0.11), (150, 110, 230), 'balltube', grp=g))
OBJS.append(Obj(tube_faces(_bt0 + (_bt0 - _bt1) * 0.08, _bt0, 0.15), (90, 70, 140), 'balltube_cap', grp=g))
_rp = rim_point(RIPPLE_A, RW + BEZ + 0.12, 0.2)
_rd = (WC + [0, 0, 0.2] - _rp)
_rd /= np.linalg.norm(_rd)
g = new_group()
OBJS.append(Obj(tube_faces(_rp - _rd * 0.28, _rp - _rd * 0.02, 0.12), (40, 150, 150), 'ripple_body', grp=g))
OBJS.append(Obj(tube_faces(_rp - _rd * 0.02, _rp + _rd * 0.12, 0.2), (60, 200, 196), 'ripple_horn', grp=g))
RIPPLE_EMIT = _rp + _rd * 0.12
_vp = rim_point(VINE_A, RW + BEZ + 0.2, 0.18)
g = new_group()
OBJS.append(Obj(lathe(_vp[0], _vp[2], [(0.0, 0.13), (0.22, 0.2), (0.26, 0.21), (0.27, 0.0)], n=8, cy=_vp[1] - 0.26), (150, 92, 60), 'pot', grp=g))
for q in range(5):
    a = q * 1.3
    OBJS.append(Obj(icosa(_vp + [0.12 * math.cos(a), 0.06 + 0.07 * q % 0.2, 0.1 * math.sin(a)], 0.11, rng=rng), (90, 190, 60), 'leaf', grp=g))
VINE_POT = _vp

# ------------------------------------------------------------------ footlights along the stage front edge
FOOT_Z = 4.55
FOOTLIGHTS = [np.array([x, 0.08, FOOT_Z]) for x in np.linspace(-3.4, 3.4, 9)]
for p in FOOTLIGHTS:
    OBJS.append(Obj(box(p[0], 0.06, p[2], 0.12, 0.06, 0.09), (40, 40, 52), 'foot', cast=False))
# the crowd pit: a lower floating deck in front of the stage, with a barrier
PIT_Y = -1.15
OBJS.append(Obj(box(0, PIT_Y - 0.3, 7.6, 5.4, 0.3, 2.6), (40, 36, 62), 'pit', cast=False))
OBJS.append(Obj(box(0, PIT_Y + 0.5, 5.05, 4.4, 0.5, 0.04), (66, 68, 90), 'barrier', cast=False))

# ------------------------------------------------------------------ crowd (positions fixed; pose per frame)
CROWD = []          # (x, z, height, phase, team they cheer for, phone?)
for row, z in enumerate([5.3, 5.95, 6.6, 7.25]):
    n = 9 + row
    for k in range(n):
        x = -4.2 + 8.4 * (k + rng.uniform(0.15, 0.85)) / n
        CROWD.append((x, z + rng.uniform(-0.2, 0.2), rng.uniform(1.6, 1.85), rng.uniform(0, 1), int(rng.integers(4)),
                      rng.random() < 0.45, rng.uniform(0.85, 1.1)))
CROWD_COLS = [np.array(c, float) for c in ((52, 46, 86), (72, 52, 96), (44, 60, 92), (66, 62, 104), (86, 56, 80))]

# ------------------------------------------------------------------ searchlights behind the stage (sweep the sky)
SEARCH = [(np.array([-3.6, 0.2, -1.6]), 0.0, (200, 220, 255)), (np.array([3.6, 0.2, -1.6]), 0.5, (200, 220, 255)),
          (np.array([-1.4, 0.2, -2.4]), 0.25, (255, 200, 240)), (np.array([1.4, 0.2, -2.4]), 0.75, (255, 200, 240))]

# ------------------------------------------------------------------ shadow casters on the stage boards
FEET = [(0, -0.05, 1.0, 0.55), (-TX, TZ, 0.3, 0.5), (TX, TZ, 0.3, 0.5), (-2.3, 1.9, 0.55, 0.55), (2.3, 1.9, 0.55, 0.55)]
SUBJECT = WC.copy()
