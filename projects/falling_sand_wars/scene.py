"""The set: a low-poly desert ruin at night. Two sandstone pillars carry a cracked lintel; from it a brass ring
hangs on chains, and the square glass box sits inside the ring's face (box + ring + spokes turn together about
the box centre). Four hoppers hang from the lintel above the box, two braziers burn on stone stands in front,
dunes roll away to far mesas under a low full moon. Everything here is static geometry; the turning parts are
built in box-local coordinates (x right, y up, about BC) and rotated per frame by render.py."""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, lathe, icosa, mound, ridge, new_group, face

rng = np.random.default_rng(21)

BC = np.array([0.0, 4.0, 0.0])        # box centre (the subject; focus is locked here)
HALF = 1.0                            # inner half side (the sand grid spans -HALF..HALF)
FR = 0.075                            # frame bar width
ZF, ZB = 0.12, -0.12                  # front / back glass
R_IN, R_OUT, RZ0, RZ1 = 1.56, 1.68, -0.44, -0.32      # the ring (behind the box)
R_SWEEP = (HALF + FR) * math.sqrt(2)  # how far the box corners reach when it turns

INLET_X = [(u * 2 - 1) * HALF for u in (0.13, 0.38, 0.62, 0.87)]
NOZZLE_Y = [BC[1] + math.sqrt(R_SWEEP ** 2 - x * x) + 0.06 for x in INLET_X]
HOP_Z = 0.0

PILLAR_X, PILLAR_Z, PILLAR_H = 2.9, -0.38, 8.05
LINTEL_Y0, LINTEL_Y1 = 8.05, 8.6
BRAZIERS = [np.array([-2.45, 0.0, 1.0]), np.array([2.45, 0.0, 1.0])]
FLAME_Y = 1.62
SAND_FLOOR = (222, 170, 112)

# ------------------------------------------------------------------ static objects
OBJS = []
STONE = (214, 158, 108)
STONE_D = (186, 128, 88)
for sx in (-1, 1):
    g = new_group()
    x = sx * PILLAR_X
    OBJS.append(Obj(box(x, 0.18, PILLAR_Z, 0.42, 0.18, 0.42), STONE_D, 'plinth', grp=g))
    OBJS.append(Obj(prism(x, PILLAR_Z, 0.3, 0.36, PILLAR_H - 0.45, 8, r1=0.27, rot=0.39), STONE, 'pillar', grp=g))
    OBJS.append(Obj(box(x, PILLAR_H - 0.22, PILLAR_Z, 0.4, 0.22, 0.4), STONE_D, 'capital', grp=g))
g = new_group()
OBJS.append(Obj(box(-1.3, (LINTEL_Y0 + LINTEL_Y1) / 2, PILLAR_Z, 1.95, (LINTEL_Y1 - LINTEL_Y0) / 2, 0.38), STONE, 'lintel', grp=g))
OBJS.append(Obj(box(1.98, (LINTEL_Y0 + LINTEL_Y1) / 2 - 0.02, PILLAR_Z, 1.38, (LINTEL_Y1 - LINTEL_Y0) / 2, 0.38, rot=0.0), STONE_D, 'lintel2', grp=g))
OBJS.append(Obj(box(0.43, LINTEL_Y1 + 0.12, PILLAR_Z + 0.05, 0.22, 0.12, 0.3, rot=0.2), STONE_D, 'chip', grp=g))

# hoppers (fixed in the world; nozzles above the box's sweep) + rods to the lintel
HOPPERS = []
for k, (x, ny) in enumerate(zip(INLET_X, NOZZLE_Y)):
    g = new_group()
    prof = [(0.0, 0.035), (0.08, 0.045), (0.42, 0.21), (0.58, 0.23), (0.62, 0.0)]
    HOPPERS.append(dict(x=x, ny=ny, grp=g, body=lathe(x, HOP_Z, prof, n=8, cy=ny),
                        band=lathe(x, HOP_Z, [(0.42, 0.212), (0.53, 0.226)], n=8, cy=ny),
                        rod=box(x, (ny + 0.62 + LINTEL_Y0) / 2, HOP_Z, 0.018, (LINTEL_Y0 - ny - 0.62) / 2, 0.018)))

# braziers: stone stands + iron bowls
for b in BRAZIERS:
    g = new_group()
    OBJS.append(Obj(prism(b[0], b[2], 0.26, 0.0, 0.18, 6, rot=0.2), STONE_D, 'stand0', grp=g))
    OBJS.append(Obj(prism(b[0], b[2], 0.16, 0.18, 1.08, 6, r1=0.13, rot=0.2), STONE, 'stand1', grp=g))
    OBJS.append(Obj(lathe(b[0], b[2], [(1.08, 0.1), (1.2, 0.26), (1.36, 0.34), (1.38, 0.0)], n=8), (70, 58, 54), 'bowl', grp=g))
    OBJS.append(Obj(icosa(b + [0, 1.38, 0], 0.2, (1, 0.45, 1), rng=rng), (255, 150, 60), 'embers', grp=g, cast=False))

# ruin: broken wall pieces behind, a toppled block, rocks
for (x, z, w, h, rot) in [(-3.6, -2.6, 0.9, 1.7, 0.1), (-2.5, -3.1, 0.6, 1.1, -0.05), (3.4, -2.8, 0.8, 2.3, -0.12),
                          (4.3, -2.2, 0.5, 0.9, 0.3), (-4.6, -1.8, 0.5, 0.7, 0.2)]:
    OBJS.append(Obj(box(x, h / 2, z, w, h / 2, 0.24, rot=rot), STONE_D, 'wall'))
OBJS.append(Obj(box(-1.6, 0.22, 2.2, 0.42, 0.22, 0.28, rot=0.5), STONE_D, 'block'))
OBJS.append(Obj(prism(1.75, 2.6, 0.28, 0.0, 0.42, 8, rot=0.1, tilt=(0.18, 0.05)), STONE, 'drum'))
for (x, z, r) in [(-3.2, 1.6, 0.28), (3.3, 1.9, 0.22), (-0.9, 3.4, 0.16), (1.1, -1.6, 0.2), (-2.3, -0.8, 0.18)]:
    OBJS.append(Obj(icosa([x, r * 0.5, z], r, (1.2, 0.7, 1), rng=rng), (150, 112, 82), 'rock'))

# dunes: mid dunes behind, a foreground lip either side (low and wide)
DUNES = []
for (x, z, rx, rz, h, col) in [(-5.5, -6.0, 4.5, 2.6, 1.4, (206, 150, 98)), (4.8, -7.5, 5.0, 3.0, 1.9, (200, 144, 94)),
                               (-0.5, -11.0, 7.0, 3.5, 2.4, (190, 134, 92)), (7.5, -13.0, 5.5, 3.0, 3.0, (178, 126, 90)),
                               (-8.0, -14.0, 6.0, 3.0, 2.7, (176, 122, 88))]:
    DUNES.append(Obj(mound(x, z, rx, rz, h, n=7, rot=0.3), col, 'dune', cast=False))
FORE = [Obj(mound(-3.4, 4.1, 2.3, 1.2, 0.55, n=6, rot=0.2), (210, 156, 104), 'fore', cast=False),
        Obj(mound(3.6, 3.6, 2.0, 1.1, 0.45, n=6, rot=-0.3), (210, 156, 104), 'fore', cast=False)]
FAR = ridge(-60.0, 5.0, 1.2, 28, 140.0, seed=7)
FAR2 = ridge(-36.0, 2.6, 0.2, 22, 90.0, seed=11)

# palms (silhouettes at the back, left and right)
PALMS = []
for (x, z, hgt, lean) in [(-4.4, -4.2, 4.6, 0.25), (5.1, -5.0, 5.3, -0.2)]:
    g = new_group()
    pts = [np.array([x + lean * (s ** 1.6) * hgt * 0.35, s * hgt, z]) for s in np.linspace(0, 1, 6)]
    trunk = []
    for a, b in zip(pts[:-1], pts[1:]):
        r0, r1 = 0.13, 0.11
        for kk in range(5):
            a0, a1 = 2 * math.pi * kk / 5, 2 * math.pi * (kk + 1) / 5
            q = [a + [r0 * math.cos(a0), 0, r0 * math.sin(a0)], a + [r0 * math.cos(a1), 0, r0 * math.sin(a1)],
                 b + [r1 * math.cos(a1), 0, r1 * math.sin(a1)], b + [r1 * math.cos(a0), 0, r1 * math.sin(a0)]]
            trunk.append(face(q, (a + b) / 2))
    PALMS.append(Obj(trunk, (120, 86, 62), 'trunk', grp=g))
    top = pts[-1]
    fronds = []
    for kk in range(7):
        a = 2 * math.pi * kk / 7 + 0.3
        tip = top + [1.5 * math.cos(a), -0.55 - 0.2 * (kk % 2), 0.9 * math.sin(a)]
        mid = top + [0.8 * math.cos(a), 0.25, 0.48 * math.sin(a)]
        side = np.array([-math.sin(a), 0, math.cos(a)]) * 0.22
        fronds.append(face([top, mid + side, tip, mid - side], top + [0, 1, 0]))
    PALMS.append(Obj(fronds, (66, 120, 84), 'frond', grp=g, cast=True))

STATIC = OBJS + PALMS
CASTERS = [Q for o in STATIC if o.cast for Q, n in o.faces]
CASTERS += [Q for h in HOPPERS for Q, n in h['body'] + h['rod']]
FEET = [(sx * PILLAR_X, PILLAR_Z, 0.45, 0.45) for sx in (-1, 1)] + [(b[0], b[2], 0.28, 0.4) for b in BRAZIERS] + \
       [(-1.6, 2.2, 0.4, 0.3), (1.75, 2.6, 0.3, 0.35)]


# ------------------------------------------------------------------ the turning assembly (box-local coords)
def _bar(cx, cy, hx, hy, z0=ZB - 0.02, z1=ZF + 0.02):
    return box(cx, cy, (z0 + z1) / 2, hx, hy, (z1 - z0) / 2)


def frame_parts():
    """box frame bars (local coords about BC): (faces, name)"""
    out = []
    a = HALF + FR / 2
    out.append((_bar(0, a, HALF + FR, FR / 2), 'bar'))
    out.append((_bar(0, -a, HALF + FR, FR / 2), 'bar'))
    out.append((_bar(a, 0, FR / 2, HALF), 'bar'))
    out.append((_bar(-a, 0, FR / 2, HALF), 'bar'))
    for sx in (-1, 1):
        for sy in (-1, 1):
            out.append((box(sx * a, sy * a, 0, FR * 0.75, FR * 0.75, ZF + 0.035), 'cap'))
    return out


def ring_parts(nseg=40):
    """the brass ring band + 8 tick marks (every 45 deg) + 4 spokes from the box corners"""
    faces = []
    for k in range(nseg):
        a0, a1 = 2 * math.pi * k / nseg, 2 * math.pi * (k + 1) / nseg
        p = lambda r, a, z: np.array([r * math.cos(a), r * math.sin(a), z])
        faces.append(face([p(R_IN, a0, RZ1), p(R_OUT, a0, RZ1), p(R_OUT, a1, RZ1), p(R_IN, a1, RZ1)], (0, 0, RZ1 - 1)))
        faces.append(face([p(R_OUT, a0, RZ0), p(R_OUT, a1, RZ0), p(R_OUT, a1, RZ1), p(R_OUT, a0, RZ1)], (0, 0, (RZ0 + RZ1) / 2)))
        faces.append(face([p(R_IN, a1, RZ0), p(R_IN, a0, RZ0), p(R_IN, a0, RZ1), p(R_IN, a1, RZ1)], (0, 0, (RZ0 + RZ1) / 2), ))
    ticks = []
    for k in range(8):
        a = math.pi / 2 + k * math.pi / 4
        c, s = math.cos(a), math.sin(a)
        w = 0.035
        q = [np.array([R_IN * c - w * s, R_IN * s + w * c, RZ1 + 0.012]), np.array([R_OUT * c - w * s, R_OUT * s + w * c, RZ1 + 0.012]),
             np.array([R_OUT * c + w * s, R_OUT * s - w * c, RZ1 + 0.012]), np.array([R_IN * c + w * s, R_IN * s - w * c, RZ1 + 0.012])]
        ticks.append(face(q, (0, 0, RZ1 - 1)))
    spokes = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            a = math.atan2(sy, sx)
            r0 = (HALF + FR) * math.sqrt(2) * 0.96
            p0 = np.array([r0 * math.cos(a), r0 * math.sin(a), ZB - 0.03])
            p1 = np.array([R_IN * math.cos(a), R_IN * math.sin(a), RZ1])
            d = np.array([-math.sin(a), math.cos(a), 0]) * 0.035
            spokes.append(face([p0 + d, p1 + d, p1 - d, p0 - d], (0, 0, -5)))
    return faces, ticks, spokes


FRAME = frame_parts()
RING, TICKS, SPOKES = ring_parts()

# cradle: two rollers on the ring top in a yoke, hung by chains from the lintel (fixed in the world)
CRADLE = []
g = new_group()
for sx in (-1, 1):
    a = math.radians(90 + sx * 13)
    c = BC + [(R_OUT + 0.06) * math.cos(a), (R_OUT + 0.06) * math.sin(a), (RZ0 + RZ1) / 2]
    CRADLE.append(Obj(prism(c[0], c[2], 0.07, c[1] - 0.06, c[1] + 0.06, 8), (96, 84, 70), 'roller', grp=g))
YOKE_Y = BC[1] + R_OUT + 0.2
CRADLE.append(Obj(box(0, YOKE_Y, (RZ0 + RZ1) / 2, 0.52, 0.05, 0.09), (110, 92, 70), 'yoke', grp=g))
CHAINS = [(np.array([sx * 0.46, YOKE_Y + 0.04, (RZ0 + RZ1) / 2]), np.array([sx * 0.62, LINTEL_Y0, PILLAR_Z + 0.02])) for sx in (-1, 1)]
CASTERS += [Q for o in CRADLE for Q, n in o.faces]
