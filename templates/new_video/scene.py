"""Static geometry for the template: a small courtyard diorama with a pedestal (the subject),
a lantern on a post, a few columns and far hills. Replace with your own subject/environment.

Rules of thumb that worked:
- the subject sits near the origin at a readable height; everything else frames it
- a foreground layer, a mid layer (props, plants) and a far layer (hills/sky) = depth
- every scene gets a key light that is visible in frame (sun, moon) and a warm local light
- add small living details: drifting motes, swaying cloth/plants, flickering flames"""
import numpy as np
from reelkit.geometry import Obj, box, prism, icosa, mound, ridge, new_group

rng = np.random.default_rng(4)
SUBJECT = np.array([0.0, 1.2, 0.0])         # focus point (locked DOF target)

OBJS = []
PED = new_group()
OBJS.append(Obj(prism(0, 0, 0.7, 0.0, 0.2, 8, rot=0.39), (188, 150, 112), 'ped0', grp=PED))
OBJS.append(Obj(prism(0, 0, 0.45, 0.2, 1.0, 8, rot=0.39), (206, 160, 112), 'ped1', grp=PED))
OBJS.append(Obj(icosa(SUBJECT + [0, 0.25, 0], 0.32, rng=rng), (120, 220, 200), 'gem', grp=PED, jitter=0.0))
for (x, z, h) in [(-2.6, -2.0, 3.2), (2.7, -2.2, 3.2), (-3.2, 1.2, 1.6)]:
    g = new_group()
    OBJS.append(Obj(box(x, 0.15, z, 0.5, 0.15, 0.5), (188, 150, 112), grp=g))
    OBJS.append(Obj(prism(x, z, 0.34, 0.3, h, 8, rot=0.39), (206, 160, 112), grp=g))
OBJS.append(Obj(icosa([-4.4, 1.6, -4.0], 1.1, (1, 0.8, 1), rng=rng), (96, 150, 100), 'bush'))
OBJS.append(Obj(mound(3.6, 2.4, 1.2, 0.8, 0.3), (224, 182, 128), 'drift', cast=False))
LANTERN = np.array([1.8, 1.6, 0.9])
g = new_group()
OBJS.append(Obj(prism(LANTERN[0], LANTERN[2], 0.05, 0.0, 1.45, 6), (70, 58, 50), 'post', grp=g))
OBJS.append(Obj(prism(LANTERN[0], LANTERN[2], 0.14, 1.45, 1.75, 6), (255, 190, 100), 'lamp', grp=g, cast=False))
FAR = ridge(-60.0, 10.0, 2.0, 24, 120.0, seed=2)

FLOOR_R = 5.0
TILES = []
for i in range(-5, 5):
    for j in range(-5, 5):
        cx, cz = (i + 0.5) * 1.0, (j + 0.5) * 1.0
        if np.hypot(cx, cz) > FLOOR_R:
            continue
        col = np.array([206, 170, 128] if (i + j) % 2 else [190, 128, 92]) * rng.uniform(0.94, 1.05)
        q = np.array([[cx - 0.46, 0.01, cz - 0.46], [cx + 0.46, 0.01, cz - 0.46], [cx + 0.46, 0.01, cz + 0.46], [cx - 0.46, 0.01, cz + 0.46]])
        TILES.append((q, col))
FEET = [(0, 0, 0.72, 0.5), (-2.6, -2.0, 0.6, 0.4), (2.7, -2.2, 0.6, 0.4), (-3.2, 1.2, 0.6, 0.4), (LANTERN[0], LANTERN[2], 0.1, 0.3)]
