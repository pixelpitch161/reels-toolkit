"""The set: a low-poly opera house, dim, seen from the upper balcony. The round stage (the arena) sits at the centre,
its boards at y = 0, ringed at the back by red velvet curtains under a gold-fringed valance; tiers of gilded boxes
curve round both sides (red interiors, gold parapets, warm sconces, two pawn-figure patrons); a crystal chandelier
hangs high above the back of the stage (the key light); footlights line the front rim; the orchestra pit below the
stage front holds music stands with little lamps and the conductor on his podium.

Angles in the xz plane: phi measured from +x toward +z, so the camera side (front) is phi = 90 deg."""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, new_group

rng = np.random.default_rng(22)

R_ARENA = 3.4                 # sim radius (piece centres stay inside R_ARENA - r)
R_BOARD = 3.56                # boards
R_RIM0, R_RIM1 = 3.56, 3.74   # gilded rim band
STAGE_Y0 = -0.95              # auditorium floor level (stage front wall drops to here)
PIT_Y = -1.75
SUBJECT = np.array([0.0, 0.0, 0.0])

GOLD = (214, 168, 74)
GOLD_D = (168, 122, 52)
VELVET = (150, 24, 40)
VELVET_D = (104, 14, 30)
WOOD_D = (70, 44, 34)

OBJS = []


def P(r, phi, y):
    return np.array([r * math.cos(phi), y, r * math.sin(phi)])


def ring_band(r0, r1, y0, y1, phi0, phi1, nseg, outer=True):
    """vertical cylinder band (r0 at y0 .. y1) facing outward (or inward)"""
    faces = []
    for k in range(nseg):
        a0 = phi0 + (phi1 - phi0) * k / nseg
        a1 = phi0 + (phi1 - phi0) * (k + 1) / nseg
        q = [P(r0, a0, y0), P(r0, a1, y0), P(r1, a1, y1), P(r1, a0, y1)]
        am = (a0 + a1) / 2
        c = np.array([0, (y0 + y1) / 2, 0]) if outer else P(r0 * 3, am, (y0 + y1) / 2)
        faces.append(face(q, c))
    return faces


def annulus(r0, r1, y, nseg, phi0=0.0, phi1=2 * math.pi):
    faces = []
    for k in range(nseg):
        a0 = phi0 + (phi1 - phi0) * k / nseg
        a1 = phi0 + (phi1 - phi0) * (k + 1) / nseg
        faces.append(face([P(r0, a0, y), P(r1, a0, y), P(r1, a1, y), P(r0, a1, y)], up=True))
    return faces


# ------------------------------------------------------------------ the stage
g = new_group()
RIM = Obj(annulus(R_RIM0, R_RIM1, 0.04, 48) + ring_band(R_RIM1, R_RIM1, 0.0, 0.04, 0, 2 * math.pi, 48), GOLD, 'rim', grp=g,
          cast=False, jitter=0.03)
APRON = Obj(ring_band(R_RIM1, R_RIM1, STAGE_Y0, 0.0, 0.0, math.pi, 24), (96, 30, 36), 'apron', grp=g, cast=False, jitter=0.05)
APRON_TRIM = Obj(ring_band(R_RIM1 + 0.01, R_RIM1 + 0.01, -0.22, -0.12, 0.0, math.pi, 24), GOLD_D, 'trim', grp=g, cast=False)
# footlights: little brass shells on the front rim
FOOT = []
for k in range(13):
    phi = math.radians(18 + 144 * k / 12)
    FOOT.append(P(R_RIM1 - 0.02, phi, 0.09))
FOOT_SHELLS = [Obj(prism(f[0], f[2], 0.07, 0.03, 0.1, 6, r1=0.05), (120, 92, 54), 'shell', cast=False) for f in FOOT]

# ------------------------------------------------------------------ curtains (back half), valance, floor behind
CURT = []
nf = 34
phi0, phi1 = math.radians(186), math.radians(354)
R_C = 4.55
CURT_TOP = 6.4
for k in range(nf):
    a0 = phi0 + (phi1 - phi0) * k / nf
    a1 = phi0 + (phi1 - phi0) * (k + 1) / nf
    am = (a0 + a1) / 2
    ra, rm = R_C, R_C - 0.22                       # folds: in, out, in
    q1 = [P(ra, a0, 0.0), P(rm, am, 0.0), P(rm, am, CURT_TOP), P(ra, a0, CURT_TOP)]
    q2 = [P(rm, am, 0.0), P(ra, a1, 0.0), P(ra, a1, CURT_TOP), P(rm, am, CURT_TOP)]
    CURT.append(face(q1, np.array([0, 3.0, 0]) * 0 + P(ra * 2, am, 3.0)))
    CURT.append(face(q2, P(ra * 2, am, 3.0)))
# normals must face the stage (inward)
CURT = [(Q, -n if np.dot(n, -Q.mean(0) * [1, 0, 1]) < 0 else n) for Q, n in CURT]
g = new_group()
CURTAINS = Obj(CURT, VELVET, 'curtain', grp=g, cast=False, jitter=0.08)
VAL = []
for k in range(nf):
    a0 = phi0 + (phi1 - phi0) * k / nf
    a1 = phi0 + (phi1 - phi0) * (k + 1) / nf
    am = (a0 + a1) / 2
    sag = 0.32
    q = [P(R_C - 0.3, a0, CURT_TOP), P(R_C - 0.3, a1, CURT_TOP), P(R_C - 0.32, a1, CURT_TOP - 0.55), P(R_C - 0.34, am, CURT_TOP - 0.55 - sag),
         P(R_C - 0.32, a0, CURT_TOP - 0.55)]
    Q, n = face(q)
    if np.dot(n, -Q.mean(0) * [1, 0, 1]) < 0:
        n = -n
    VAL.append((Q, n))
VALANCE = Obj(VAL, (176, 34, 48), 'valance', grp=g, cast=False, jitter=0.05)
FRINGE = []
for k in range(nf):
    a0 = phi0 + (phi1 - phi0) * k / nf
    a1 = phi0 + (phi1 - phi0) * (k + 1) / nf
    am = (a0 + a1) / 2
    yb0, yb1 = CURT_TOP - 0.55, CURT_TOP - 0.55 - 0.32
    q = [P(R_C - 0.345, a0, yb0 - 0.0), P(R_C - 0.355, am, yb1), P(R_C - 0.355, am, yb1 - 0.12), P(R_C - 0.345, a0, yb0 - 0.12)]
    Q, n = face(q)
    if np.dot(n, -Q.mean(0) * [1, 0, 1]) < 0:
        n = -n
    FRINGE.append((Q, n))
    q = [P(R_C - 0.355, am, yb1), P(R_C - 0.345, a1, yb0), P(R_C - 0.345, a1, yb0 - 0.12), P(R_C - 0.355, am, yb1 - 0.12)]
    Q, n = face(q)
    if np.dot(n, -Q.mean(0) * [1, 0, 1]) < 0:
        n = -n
    FRINGE.append((Q, n))
FRINGE_O = Obj(FRINGE, (232, 184, 84), 'fringe', grp=g, cast=False, jitter=0.0)
# tie-back tassels at each end of the curtain ring
TASSELS = [Obj(icosa(P(R_C - 0.4, a, 2.4), 0.13, (1, 1.4, 1), rng=rng), (230, 186, 90), 'tassel', cast=False) for a in (phi0 + 0.06, phi1 - 0.06)]
# the backstage floor strip between the boards and the curtains (dark wood)
BACKFLOOR = Obj(annulus(R_RIM1, R_C, 0.0, 30, phi0 - 0.2, phi1 + 0.2), (78, 52, 40), 'backfloor', cast=False, jitter=0.03)

# ------------------------------------------------------------------ gilded boxes (both sides, three tiers)
BOXES = []          # Obj list
SCONCES = []        # warm lamp positions
PATRONS = []        # (base position, facing, group) for pawn silhouettes
R_BOX = 5.35
TIERS = [(STAGE_Y0 + 1.35, STAGE_Y0 + 2.45), (STAGE_Y0 + 2.95, STAGE_Y0 + 4.05), (STAGE_Y0 + 4.55, STAGE_Y0 + 5.65)]
SIDE_PHIS = {'L': [math.radians(a) for a in (136, 152, 168, 184, 200)], 'R': [math.radians(a) for a in (44, 28, 12, -4, -20)]}
DPHI = math.radians(16)
for side, phis in SIDE_PHIS.items():
    for ti, (y0, y1) in enumerate(TIERS):
        for bi, ph in enumerate(phis):
            gg = new_group()
            a0, a1 = ph - DPHI / 2 + 0.012, ph + DPHI / 2 - 0.012
            # dark red interior (back wall at a larger radius), floor
            back = [P(R_BOX + 0.9, a0, y0), P(R_BOX + 0.9, a1, y0), P(R_BOX + 0.9, a1, y1 + 0.35), P(R_BOX + 0.9, a0, y1 + 0.35)]
            Qb, nb = face(back, np.array([0, y0, 0]) * 0 + P(R_BOX * 3, ph, y0))
            nb = nb if np.dot(nb, -Qb.mean(0) * [1, 0, 1]) > 0 else -nb
            BOXES.append(Obj([(Qb, nb)], (92, 18, 28), 'boxback', grp=gg, cast=False, jitter=0.06))
            fl = [P(R_BOX, a0, y0), P(R_BOX, a1, y0), P(R_BOX + 0.9, a1, y0), P(R_BOX + 0.9, a0, y0)]
            BOXES.append(Obj([face(fl, up=True)], (70, 20, 26), 'boxfloor', grp=gg, cast=False))
            # gold parapet (front face + top lip)
            par = ring_band(R_BOX, R_BOX, y0, y0 + 0.62, a0, a1, 2, outer=False)
            par = [(Q, n if np.dot(n, -Q.mean(0) * [1, 0, 1]) > 0 else -n) for Q, n in par]
            BOXES.append(Obj(par, GOLD, 'parapet', grp=gg, cast=False, jitter=0.05))
            BOXES.append(Obj(annulus(R_BOX - 0.06, R_BOX + 0.08, y0 + 0.62, 2, a0, a1), (236, 196, 104), 'lip', grp=gg, cast=False))
            # pilasters between boxes (gold), with a sconce on every other one
            pc = P(R_BOX, a0 - 0.012, 0)
            BOXES.append(Obj(box(pc[0], (y0 + y1 + 0.35) / 2 - 0.1, pc[2], 0.07, (y1 - y0 + 0.55) / 2, 0.07, rot=-a0), GOLD_D, 'pilaster',
                             grp=gg, cast=False))
            if (bi + ti) % 2 == 0:
                SCONCES.append(P(R_BOX - 0.12, a0 - 0.012, y0 + 0.95))
            # patrons: pawn silhouettes in a few boxes
            if (side, ti, bi) in (('L', 0, 1), ('R', 1, 2), ('L', 2, 3), ('R', 0, 0)):
                PATRONS.append((P(R_BOX + 0.45, ph, y0), ph, gg))
    # the cornice above the top tier and the wall below the first tier
    a0, a1 = phis[-1] - DPHI / 2, phis[0] + DPHI / 2
    lo, hi = min(a0, a1), max(a0, a1)
    wall = ring_band(R_BOX + 0.02, R_BOX + 0.02, STAGE_Y0, TIERS[0][0], lo, hi, 6, outer=False)
    wall = [(Q, n if np.dot(n, -Q.mean(0) * [1, 0, 1]) > 0 else -n) for Q, n in wall]
    BOXES.append(Obj(wall, (110, 30, 38), 'boxwall', cast=False, jitter=0.05))
    for ti in range(3):
        y1 = TIERS[ti][1] + 0.35
        y2 = TIERS[ti + 1][0] if ti < 2 else y1 + 0.5
        band = ring_band(R_BOX + 0.02, R_BOX + 0.02, y1, y2, lo, hi, 6, outer=False)
        band = [(Q, n if np.dot(n, -Q.mean(0) * [1, 0, 1]) > 0 else -n) for Q, n in band]
        BOXES.append(Obj(band, (124, 34, 42) if ti < 2 else GOLD_D, 'band', cast=False, jitter=0.05))


def pawn(base, s=1.0):
    """a chess-pawn figure (lathe), base on the box floor"""
    prof = [(0.0, 0.2 * s), (0.08 * s, 0.2 * s), (0.12 * s, 0.13 * s), (0.42 * s, 0.09 * s), (0.5 * s, 0.14 * s), (0.54 * s, 0.07 * s),
            (0.6 * s, 0.12 * s), (0.72 * s, 0.13 * s), (0.82 * s, 0.08 * s), (0.86 * s, 0.0)]
    return lathe(base[0], base[2], prof, n=7, cy=base[1])


# ------------------------------------------------------------------ the orchestra pit + conductor
PIT_Z0, PIT_Z1 = 4.15, 6.3
g = new_group()
PIT_FLOOR_Q = (np.array([[-4.2, PIT_Y, PIT_Z0], [4.2, PIT_Y, PIT_Z0], [4.6, PIT_Y, PIT_Z1], [-4.6, PIT_Y, PIT_Z1]]))
PIT_WALL = Obj([face([[-4.6, PIT_Y, PIT_Z1], [4.6, PIT_Y, PIT_Z1], [4.6, STAGE_Y0 + 0.25, PIT_Z1], [-4.6, STAGE_Y0 + 0.25, PIT_Z1]], [0, 0, 0])],
               (84, 26, 32), 'pitwall', grp=g, cast=False)
PIT_RAIL = Obj(box(0, STAGE_Y0 + 0.3, PIT_Z1 + 0.04, 4.7, 0.05, 0.06), GOLD, 'rail', grp=g, cast=False)
STANDS = []
STAND_LAMPS = []
for k, (x, z) in enumerate([(-3.1, 4.9), (-2.2, 5.4), (-1.2, 4.75), (1.2, 4.75), (2.2, 5.4), (3.1, 4.9), (-3.4, 5.8), (3.4, 5.8)]):
    gg = new_group()
    STANDS.append(Obj(prism(x, z, 0.02, PIT_Y, PIT_Y + 0.75, 4), (40, 32, 30), 'stem', grp=gg, cast=False))
    STANDS.append(Obj([face([[x - 0.2, PIT_Y + 0.75, z - 0.05], [x + 0.2, PIT_Y + 0.75, z - 0.05], [x + 0.2, PIT_Y + 1.0, z - 0.17],
                             [x - 0.2, PIT_Y + 1.0, z - 0.17]], [x, PIT_Y + 0.8, z + 1])], (236, 226, 196), 'sheet', grp=gg, cast=False))
    STAND_LAMPS.append(np.array([x, PIT_Y + 1.06, z - 0.12]))
PODIUM = Obj(box(0, PIT_Y + 0.15, 4.75, 0.32, 0.15, 0.32), (60, 40, 34), 'podium', cast=False)
COND_BASE = np.array([0.0, PIT_Y + 0.3, 4.75])

# ------------------------------------------------------------------ chandelier (high above the back of the stage)
CHAND = np.array([0.0, 6.4, -1.6])
CH_PARTS = []
gch = new_group()
CH_PARTS.append(Obj(lathe(CHAND[0], CHAND[2], [(-0.55, 0.0), (-0.42, 0.2), (-0.22, 0.62), (-0.1, 0.72), (0.0, 0.66), (0.12, 0.3), (0.5, 0.12),
                                               (0.9, 0.06), (1.0, 0.0)], n=10, cy=CHAND[1]), GOLD, 'chand', grp=gch, cast=False, jitter=0.06))
CANDLES = []
for k in range(10):
    a = 2 * math.pi * k / 10
    CANDLES.append(CHAND + [0.72 * math.cos(a), 0.12, 0.72 * math.sin(a)])
for k in range(6):
    a = 2 * math.pi * (k + 0.5) / 6
    CANDLES.append(CHAND + [0.36 * math.cos(a), 0.5, 0.36 * math.sin(a)])
CRYSTALS = []
for k in range(16):
    a = 2 * math.pi * k / 16
    CRYSTALS.append((CHAND + [0.62 * math.cos(a), -0.35 - 0.12 * (k % 2), 0.62 * math.sin(a)], k))
for k in range(8):
    a = 2 * math.pi * (k + 0.5) / 8
    CRYSTALS.append((CHAND + [0.3 * math.cos(a), -0.62, 0.3 * math.sin(a)], 16 + k))
CRYSTALS.append((CHAND + [0, -0.85, 0], 24))
CHAIN = (CHAND + [0, 1.0, 0], CHAND + [0, 6.0, 0])

# ------------------------------------------------------------------ proscenium arch (far, dark gold) + dark house
ARCH = []
for side in (-1, 1):
    ARCH.append(Obj(box(side * 5.15, 3.2, -3.9, 0.35, 4.6, 0.3), (120, 84, 46), 'arch', cast=False, jitter=0.05))
ARCH.append(Obj(box(0, 7.95, -3.9, 5.5, 0.4, 0.3), (132, 94, 50), 'arch', cast=False, jitter=0.05))

STATIC = [RIM, APRON, APRON_TRIM, CURTAINS, VALANCE, FRINGE_O, BACKFLOOR, PIT_WALL, PIT_RAIL, PODIUM] + FOOT_SHELLS + TASSELS + BOXES + \
         STANDS + CH_PARTS + ARCH

# ------------------------------------------------------------------ the stalls (rows of red seats in front of the pit) + a few patrons
CARPET_Q = np.array([[-10.0, STAGE_Y0, PIT_Z1 + 0.1], [10.0, STAGE_Y0, PIT_Z1 + 0.1], [12.0, STAGE_Y0 + 0.7, 13.0], [-12.0, STAGE_Y0 + 0.7, 13.0]])
SEATS = []
STALL_PATRONS = []
for k, rr in enumerate([7.05, 7.85, 8.65, 9.45]):
    y0 = STAGE_Y0 + 0.16 * k
    n = 15 + 2 * k
    for i in range(n):
        ph = math.radians(58 + 64 * (i + 0.5) / n)
        c = P(rr, ph, 0)
        gg = new_group()
        SEATS.append(Obj(box(c[0], y0 + 0.3, c[2], 0.17, 0.3, 0.05, rot=-(ph - math.pi / 2)), (150, 30, 42), 'seat', grp=gg, cast=False, jitter=0.08))
        if (k, i) in ((0, 4), (1, 12), (2, 7), (3, 15)):
            STALL_PATRONS.append((P(rr - 0.18, ph, y0 + 0.2), ph))
STATIC += SEATS
