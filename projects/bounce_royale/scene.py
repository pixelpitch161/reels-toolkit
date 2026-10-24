"""Neon rooftop at night in a rain-soaked retro-futurist city. Static geometry + the dynamic pieces' anchors.

    roof (y = 0, wet concrete, drawn per pixel)        the hologram ring floats above it (plane z = 0)
    projector under the ring (emits the hologram)      stairwell hut with a warm door lamp (left)
    water tank on legs (far left), antenna mast (right) with a blinking light, a neon sign stand (right)
    AC units with fans (foreground), vent with steam, cables, a satellite dish
    city: towers across the street (lit windows, neon banners, blinking antenna lights), fogged with distance
    sky: indigo -> magenta city glow, a magenta moon (the key light), drones on loops (small life)"""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, lathe, icosa, new_group, catenary
from reelkit.config import dirv

rng = np.random.default_rng(24)

RW = 1.2                                   # ring radius (m) when the sim ring = 1
RC = np.array([0.0, 2.25, 0.0])            # ring centre
TILE_TH = 0.065                            # tile half-thickness (radial)
TILE_DZ = 0.05                             # tile half-depth
BALL_R = 0.06 * RW                         # = royale rb in metres

MOON_DRAW = dirv(-7.5, 13.5)              # the magenta moon, top left of frame
KEY = dirv(-24.0, 34.0)                    # its light (shading/shadows), same side, higher
KEY_COL = np.array([0.62, 0.38, 0.78])     # magenta moonlight
AMB = (0.30, 0.27, 0.46)
SKY_AMB = (0.08, 0.07, 0.15)

STATIC = []


def add(faces, col, name='', grp=None, cast=True, jitter=0.04):
    o = Obj(faces, col, name, grp=grp, cast=cast, jitter=jitter, rng=rng)
    STATIC.append(o)
    return o


# ---- projector (under the ring)
g = new_group()
PROJ = np.array([0.0, 0.0, 0.0])
add(prism(0, 0, 0.42, 0.0, 0.12, 8, r1=0.38, rot=0.39), (70, 72, 92), 'proj_base', g)
add(prism(0, 0, 0.30, 0.12, 0.30, 8, r1=0.22, rot=0.39), (96, 98, 120), 'proj_body', g)
add(prism(0, 0, 0.16, 0.30, 0.36, 8, rot=0.39), (120, 240, 255), 'proj_lens', g, cast=False)
PROJ_LENS = np.array([0.0, 0.37, 0.0])

# ---- stairwell hut (left) with a door + warm lamp
g = new_group()
HUT = (-2.75, -2.2)
add(box(HUT[0], 1.15, HUT[1], 0.75, 1.15, 0.9), (78, 74, 98), 'hut', g)
add(box(HUT[0], 2.36, HUT[1], 0.85, 0.06, 1.0), (58, 56, 76), 'hut_roof', g)
add(box(HUT[0] + 0.76, 0.95, HUT[1] + 0.1, 0.02, 0.95, 0.38), (44, 40, 54), 'door', g)
DOOR_LAMP = np.array([HUT[0] + 0.84, 2.05, HUT[1] + 0.1])
add(box(DOOR_LAMP[0], DOOR_LAMP[1], DOOR_LAMP[2], 0.07, 0.06, 0.1), (255, 196, 120), 'lamp', g, cast=False)

# ---- water tank on legs (far left)
g = new_group()
TANK = (-3.6, -5.2)
for dx, dz in ((-0.55, -0.55), (0.55, -0.55), (0.55, 0.55), (-0.55, 0.55)):
    add(box(TANK[0] + dx, 0.85, TANK[1] + dz, 0.05, 0.85, 0.05), (60, 56, 70), 'leg', g)
add(lathe(TANK[0], TANK[1], [(1.7, 0.85), (2.9, 0.85), (3.25, 0.5), (3.4, 0.0)], n=10, cy=0.0), (96, 78, 86), 'tank', g)

# ---- antenna mast (right) with a blinking light on top
g = new_group()
MAST = (2.35, -4.6)
add(prism(MAST[0], MAST[1], 0.05, 0.0, 5.0, 4), (90, 90, 110), 'mast', g)
for h in (1.2, 2.4, 3.6):
    add(box(MAST[0], h, MAST[1], 0.32, 0.02, 0.02), (90, 90, 110), 'mast_bar', g)
MAST_TOP = np.array([MAST[0], 5.08, MAST[1]])

# ---- neon sign stand (right, behind the ring)
g = new_group()
SIGN = dict(x=2.55, z=-2.4, y0=1.15, y1=2.35, hw=0.55)
add(box(SIGN['x'] - 0.5, 0.6, SIGN['z'], 0.04, 0.6, 0.04), (60, 58, 72), 'sign_leg', g)
add(box(SIGN['x'] + 0.5, 0.6, SIGN['z'], 0.04, 0.6, 0.04), (60, 58, 72), 'sign_leg', g)
add(box(SIGN['x'], (SIGN['y0'] + SIGN['y1']) / 2, SIGN['z'] - 0.06, SIGN['hw'] + 0.05, (SIGN['y1'] - SIGN['y0']) / 2 + 0.05, 0.04),
    (36, 30, 48), 'sign_back', g)
SIGN_C = np.array([SIGN['x'], (SIGN['y0'] + SIGN['y1']) / 2, SIGN['z'] - 0.01])

# ---- AC units (foreground) with fans, vent with steam, a dish
AC = []
for (x, z, rot) in ((-1.15, 3.2, 0.12), (1.25, 4.3, -0.2), (-2.05, -0.9, 0.3)):
    g = new_group()
    add(box(x, 0.42, z, 0.55, 0.42, 0.4, rot), (128, 132, 150), 'ac', g)
    add(box(x, 0.86, z, 0.5, 0.02, 0.36, rot), (98, 102, 122), 'ac_top', g)
    AC.append((x, z, rot))
g = new_group()
VENT = np.array([1.35, 0.0, -1.3])
add(prism(VENT[0], VENT[2], 0.12, 0.0, 0.85, 6), (110, 108, 126), 'vent', g)
add(prism(VENT[0], VENT[2], 0.18, 0.85, 0.95, 6, r1=0.1), (90, 88, 104), 'vent_cap', g)
VENT_TOP = np.array([VENT[0], 1.0, VENT[2]])
g = new_group()
add(prism(-1.2, -3.6, 0.04, 0.0, 0.9, 4), (90, 90, 110), 'dish_pole', g)
add(lathe(-1.2, -3.6, [(0.0, 0.0), (0.1, 0.25), (0.2, 0.42)], n=8, cy=0.9), (150, 150, 170), 'dish', g)

# ---- parapet (back edge) + a low rail
PAR_Z = -6.0
g = new_group()
add(box(0.0, 0.45, PAR_Z, 9.0, 0.45, 0.15), (70, 66, 86), 'parapet', g)
for k in range(-8, 9):
    add(box(k * 0.9, 1.05, PAR_Z, 0.025, 0.15, 0.025), (84, 84, 104), 'rail_post', g, cast=False)
add(box(0.0, 1.2, PAR_Z, 9.0, 0.02, 0.02), (96, 96, 118), 'rail', g, cast=False)

CABLES = [catenary(MAST_TOP - [0, 1.4, 0], np.array([HUT[0] + 0.7, 2.3, HUT[1] - 0.8]), 0.5, 18),
          catenary(MAST_TOP - [0, 2.6, 0], np.array([SIGN['x'] - 0.5, 1.2, SIGN['z']]), 0.25, 10)]

FEET = [(0, 0, 0.45, 0.45), (HUT[0], HUT[1], 0.95, 0.35), (TANK[0], TANK[1], 0.5, 0.2), (MAST[0], MAST[1], 0.1, 0.25),
        (VENT[0], VENT[2], 0.15, 0.3)] + [(x, z, 0.6, 0.4) for x, z, _ in AC]

CASTERS = [Q for o in STATIC if o.cast for Q, n in o.faces]

# ---- roof puddles (static, reflective): (x, z, rx, rz)
PUDDLES = [(0.5, 1.6, 1.6, 0.7), (-1.0, 4.2, 1.2, 0.6), (1.8, 5.6, 1.4, 0.5), (-2.2, -0.6, 1.0, 0.5), (0.9, -2.8, 1.3, 0.5),
           (-0.6, 6.8, 2.0, 0.8)]

# ================================================================== the city across the street
STREET_Y = -32.0
TOWERS = []
_tr = np.random.default_rng(7)
for k in range(46):
    side = _tr.uniform(-1, 1)
    z = -_tr.uniform(14, 95)
    x = side * (8 + 0.42 * -z) + _tr.normal(0, 3)
    hw = _tr.uniform(2.2, 5.0)
    hd = _tr.uniform(2.2, 5.0)
    top = _tr.uniform(-4, 16) + (10 if _tr.random() < 0.25 else 0) + 0.06 * -z
    col = np.array([52, 44, 86.0]) * _tr.uniform(0.8, 1.2) + np.array([0, 0, 10.0]) * _tr.random()
    TOWERS.append(dict(x=x, z=z, hw=hw, hd=hd, top=top, col=col, rot=_tr.uniform(-0.15, 0.15)))
# keep the moon clear: towers in its direction stay well below it (camera ~ (0, 2.4, 8.7))
for d in TOWERS:
    dx, dz = d['x'] - 0.0, d['z'] - 8.7
    az = math.degrees(math.atan2(dx, -dz))
    if -15.0 < az < 0.0:
        d['top'] = min(d['top'], 2.4 + math.hypot(dx, dz) * math.tan(math.radians(6.5)))
TOWERS.sort(key=lambda d: d['z'])           # far first

# lit windows: points on the camera-facing (+z) face and the side facing the centre
WIN = []                                    # (point, colour, blink-cycles or 0)
_wc = [np.array([255, 214, 140.0]), np.array([255, 170, 110.0]), np.array([150, 220, 255.0]), np.array([255, 120, 200.0])]
for d in TOWERS:
    c, s = math.cos(d['rot']), math.sin(d['rot'])
    for face in ('front', 'side'):
        nx = int(d['hw'] * 2 / 0.9) if face == 'front' else int(d['hd'] * 2 / 0.9)
        ny = int((d['top'] - STREET_Y) / 1.1)
        for i in range(nx):
            for j in range(ny):
                if _tr.random() > 0.3:
                    continue
                u = (i + 0.5) / nx * 2 - 1
                y = STREET_Y + (j + 0.6) * 1.1
                if y > d['top'] - 0.5:
                    continue
                if face == 'front':
                    lx, lz = u * d['hw'] * 0.92, d['hd'] + 0.02
                else:
                    sg = -1 if d['x'] > 0 else 1
                    lx, lz = sg * (d['hw'] + 0.02), u * d['hd'] * 0.92
                p = np.array([d['x'] + lx * c - lz * s, y, d['z'] + lx * s + lz * c])
                WIN.append((p, _wc[int(_tr.integers(0, 4))] * _tr.uniform(0.55, 1.0), 0))
WIN_P = np.array([w[0] for w in WIN])
WIN_C = np.array([w[1] for w in WIN])

# neon banners on towers: vertical / horizontal emissive strips
BANNERS = []
_bc = [(255, 60, 180), (60, 230, 255), (255, 150, 60), (180, 90, 255), (255, 70, 110)]
for d in TOWERS:
    if _tr.random() < 0.45 and d['z'] > -70:
        vert = _tr.random() < 0.6
        c, s = math.cos(d['rot']), math.sin(d['rot'])
        lx = _tr.uniform(-0.6, 0.6) * d['hw']
        y1 = d['top'] - _tr.uniform(1.0, 6.0)
        h = _tr.uniform(3.0, 7.0) if vert else 0.7
        w = 0.55 if vert else _tr.uniform(2.0, 3.5)
        lz = d['hd'] + 0.05
        P = lambda x, y: np.array([d['x'] + x * c - lz * s, y, d['z'] + x * s + lz * c])
        q = np.array([P(lx - w / 2, y1 - h), P(lx + w / 2, y1 - h), P(lx + w / 2, y1), P(lx - w / 2, y1)])
        BANNERS.append((q, np.array(_bc[int(_tr.integers(0, len(_bc)))], float), _tr.uniform(0, 6.28), int(_tr.integers(3, 11))))

# antenna lights on the tallest towers
BEACONS = []
for d in sorted(TOWERS, key=lambda d: -d['top'])[:9]:
    BEACONS.append((np.array([d['x'], d['top'] + 1.2, d['z']]), _tr.uniform(0, 1), int(_tr.integers(20, 40))))

# drones: (centre, radius x/z, height, cycles per video, phase)
DRONES = [(np.array([-6.0, 9.0, -22.0]), 7.0, 2.5, 3, 0.4), (np.array([7.0, 11.5, -34.0]), 9.0, 3.0, 2, 2.1),
          (np.array([1.0, 7.0, -15.0]), 5.0, 1.5, 4, 4.0)]
