"""Static geometry: the Mountain King's crystal cavern, cut away like a diorama, and the outside of its mountain.

Coordinates: y up, floor y = 0, the rings hang in the plane z = 0 around C = (0, 3.9, 0). The cavern runs
x -7.5..7.5, z -6 (back wall) .. 4 (the cut-away front face). A crack in the roof above and behind the rings
lets in a shaft of moonlight (the key light). Outside: the mountain is a floating island in a night sky; its
summit has a notch where the crack comes out; a giant ring with a gap hangs round the whole thing (seen only
in the closing zoom-out).

Objects carry a 'kind' used by the renderer: shell (walls/ceiling), solid (props), crystal (emissive, pulses),
statue, torch, cut (front face), outside (mountain exterior), island (underside)."""
import math
import numpy as np
from reelkit.geometry import face, box, prism, icosa, lathe, new_group
from reelkit.config import dirv

rng = np.random.default_rng(1875)

X0, X1, Z0, Z1 = -7.5, 7.5, -6.0, 4.0
KEY = dirv(-12, 74)                      # toward the crack / moon (shading + floor shadows)
CRACK = np.array([-0.45, 9.3, -1.7])
POOL_C = CRACK - KEY * (CRACK[1] / KEY[1])          # shaft footprint on the floor
SUMMIT = np.array([-1.1, 16.7, -3.3])
GIANT_C = np.array([0.0, 4.5, 0.0])
GIANT_R = 15.5

ROCK = np.array([74, 66, 116])
ROCK2 = np.array([92, 78, 128])
MOSS = np.array([70, 140, 132])


def ceil_y(x, z):
    """cavern roof height (arched, highest over the rings)"""
    return 9.9 - 0.05 * x * x - 0.022 * (z - 0.5) ** 2


class Part:
    def __init__(self, faces, color, kind, name='', grp=None, emit=0.0, cast=True, glow=None):
        self.faces = faces
        self.color = np.array(color, float)
        self.kind = kind
        self.name = name
        self.grp = new_group() if grp is None else grp
        self.emit = emit
        self.cast = cast
        self.glow = glow                         # (pos, colour, radius) for emissive props
        V = np.concatenate([f[0] for f in faces])
        self.lo, self.hi = V.min(0), V.max(0)
        self.ctr = V.mean(0)
        self.tone = 1 + rng.uniform(-0.05, 0.05, len(faces))


PARTS = []


def jit_grid(P, nu, nv, amp, seed):
    r = np.random.default_rng(seed)
    G = np.array([[P(i / nu, j / nv) for j in range(nv + 1)] for i in range(nu + 1)])
    J = r.normal(0, amp, G.shape)
    J[[0, -1], :, :] *= 0.3
    return G + J


def grid_faces(G, inward, skip=None):
    out = []
    nu, nv = G.shape[0] - 1, G.shape[1] - 1
    for i in range(nu):
        for j in range(nv):
            if skip and skip(i, j):
                continue
            q = [G[i, j], G[i + 1, j], G[i + 1, j + 1], G[i, j + 1]]
            for tri in ((q[0], q[1], q[2]), (q[0], q[2], q[3])):
                Q, n = face(tri)
                if np.dot(n, inward(Q.mean(0))) < 0:
                    n = -n
                out.append((Q, n))
    return out


# ------------------------------------------------------------------ cavern shell
back = jit_grid(lambda u, v: [X0 + (X1 - X0) * u, ceil_y(X0 + (X1 - X0) * u, Z0) * v, Z0 - 0.6 * math.sin(math.pi * u) - 0.3 * v],
                14, 9, 0.18, 1)
PARTS.append(Part(grid_faces(back, lambda c: np.array([0, 0, 1.0])), ROCK, 'shell', 'back'))
for sx, seed in ((-1, 2), (1, 3)):
    xw = X0 if sx < 0 else X1
    side = jit_grid(lambda u, v: [xw + sx * 0.4 * math.sin(math.pi * u) + sx * 0.25 * v, ceil_y(xw, Z0 + (Z1 + 0.6 - Z0) * u) * v,
                                  Z0 + (Z1 + 0.6 - Z0) * u], 8, 8, 0.16, seed)
    PARTS.append(Part(grid_faces(side, lambda c, sx=sx: np.array([-sx, 0, 0.0])), ROCK * 0.95, 'shell', 'side'))


def _crack_hole(i, j, nu=14, nv=8):
    x = X0 + (X1 - X0) * (i + 0.5) / nu
    z = Z0 + (Z1 + 0.6 - Z0) * (j + 0.5) / nv
    return abs(x - CRACK[0]) < 0.75 and abs(z - CRACK[2]) < 0.85


roof = jit_grid(lambda u, v: [X0 + (X1 - X0) * u, ceil_y(X0 + (X1 - X0) * u, Z0 + (Z1 + 0.6 - Z0) * v), Z0 + (Z1 + 0.6 - Z0) * v],
                14, 8, 0.2, 4)
PARTS.append(Part(grid_faces(roof, lambda c: np.array([0, -1.0, 0]), skip=_crack_hole), ROCK * 0.85, 'shell', 'roof'))
# crack walls (a short chimney with a bright sky disc at the top)
for a in range(6):
    a0, a1 = 2 * math.pi * a / 6, 2 * math.pi * (a + 1) / 6
    r0, r1 = 0.62, 0.42
    y0, y1 = ceil_y(CRACK[0], CRACK[2]) - 0.1, CRACK[1] + 1.4
    P = lambda r, ang, y: CRACK + np.array([r * math.cos(ang), y - CRACK[1], r * math.sin(ang)])
    Q = np.array([P(r0, a0, y0), P(r0, a1, y0), P(r1, a1, y1), P(r1, a0, y1)])
    n = -np.array([math.cos((a0 + a1) / 2), 0, math.sin((a0 + a1) / 2)])
    PARTS.append(Part([(Q, n)], ROCK * 0.7, 'shell', 'chimney'))
CRACK_SKY = CRACK + np.array([0, 1.38, 0])

# stalactites (roof) and stalagmites (floor)
for k in range(26):
    x = rng.uniform(X0 + 0.8, X1 - 0.8)
    z = rng.uniform(Z0 + 0.5, Z1 - 0.3)
    if abs(x - CRACK[0]) < 1.0 and abs(z - CRACK[2]) < 1.1:
        continue
    if abs(x) < 3.8 and abs(z) < 1.2:
        continue                                                 # keep the rings' airspace clear
    y = ceil_y(x, z) + 0.15
    L = rng.uniform(0.5, 1.6)
    PARTS.append(Part(prism(x, z, rng.uniform(0.12, 0.26), y, y - L, 5, r1=0.01, rot=rng.uniform(0, 6)), ROCK2 * 0.9, 'solid', 'stalactite',
                      cast=False))
for k in range(16):
    x = rng.choice([rng.uniform(X0 + 0.5, -4.2), rng.uniform(4.2, X1 - 0.5)])
    z = rng.uniform(Z0 + 0.4, 2.8)
    L = rng.uniform(0.4, 1.3)
    PARTS.append(Part(prism(x, z, rng.uniform(0.16, 0.3), 0.0, L, 5, r1=0.02, rot=rng.uniform(0, 6)), ROCK2, 'solid', 'stalagmite'))
for k in range(14):
    x = rng.choice([rng.uniform(X0 + 0.6, -3.9), rng.uniform(3.9, X1 - 0.6)])
    z = rng.uniform(Z0 + 0.6, 3.0)
    r = rng.uniform(0.25, 0.6)
    PARTS.append(Part(icosa([x, r * 0.55, z], r, (1, 0.62, 1), rng=rng), ROCK2 * rng.uniform(0.85, 1.05), 'solid', 'boulder'))

# ledges (left low, right higher, back shelf)
LEDGES = [(-6.0, 2.3, -2.6, 1.4, 2.4), (6.0, 3.2, -2.2, 1.4, 2.6), (-2.6, 1.0, -5.0, 2.2, 0.8), (3.2, 1.6, -5.1, 1.9, 0.75)]
for (x, y, z, hx, hz) in LEDGES:
    g = new_group()
    PARTS.append(Part(box(x, y - 0.2, z, hx, 0.2, hz, rot=rng.uniform(-0.1, 0.1)), ROCK2 * 1.05, 'solid', 'ledge', grp=g))
    PARTS.append(Part(prism(x, z, min(hx, hz) * 0.8, 0.0, y - 0.4, 6, r1=min(hx, hz) * 0.95, rot=0.3), ROCK * 0.9, 'solid', 'ledge_base', grp=g))

# ------------------------------------------------------------------ torches (warm local lights)
TORCHES = []
for (x, y, z) in [(-5.4, 2.3, -1.6), (5.5, 3.2, -1.2), (-4.8, 0.0, -4.6), (5.3, 0.0, -4.4)]:
    g = new_group()
    post_h = 0.9 if y < 0.5 else 0.35
    PARTS.append(Part(prism(x, z, 0.05, y, y + post_h, 6), (70, 52, 40), 'torch', 'post', grp=g))
    PARTS.append(Part(lathe(x, z, [(0, 0.05), (0.08, 0.17), (0.16, 0.2)], 7, cy=y + post_h), (120, 92, 70), 'torch', 'bowl', grp=g))
    TORCHES.append(np.array([x, y + post_h + 0.3, z]))

# ------------------------------------------------------------------ crystal clusters (emissive, pulse on notes)
CRY_COLS = [(90, 236, 255), (255, 96, 214), (176, 120, 255), (120, 255, 170), (255, 180, 90)]
CRYSTALS = []          # (centre, colour, size)


def crystal_cluster(c, col, size, n, seed, down=False):
    r = np.random.default_rng(seed)
    faces = []
    for i in range(n):
        ang = r.uniform(0, 2 * math.pi)
        tilt = r.uniform(0.1, 0.55)
        L = size * r.uniform(0.6, 1.25)
        w = size * r.uniform(0.12, 0.2)
        base = np.asarray(c, float) + np.array([math.cos(ang), 0, math.sin(ang)]) * size * r.uniform(0, 0.25)
        axis = np.array([math.cos(ang) * math.sin(tilt), math.cos(tilt) * (-1 if down else 1), math.sin(ang) * math.sin(tilt)])
        u = np.cross(axis, [0.31, 0.2, 0.93]); u /= np.linalg.norm(u)
        v = np.cross(axis, u)
        ring0 = [base + (u * math.cos(a) + v * math.sin(a)) * w for a in np.linspace(0, 2 * math.pi, 7)[:-1]]
        ring1 = [p + axis * L * 0.78 for p in ring0]
        tip = base + axis * L
        for j in range(6):
            faces.append(face([ring0[j], ring0[(j + 1) % 6], ring1[(j + 1) % 6], ring1[j]], base + axis * L * 0.4))
            faces.append(face([ring1[j], ring1[(j + 1) % 6], tip], base + axis * L * 0.6))
    return faces


_cl = [((-3.2, 0.0, 2.6), 0, 0.95, 6), ((3.6, 0.0, 2.9), 1, 1.0, 6), ((-6.2, 2.3, -3.2), 2, 0.7, 5), ((6.3, 3.2, -2.9), 3, 0.7, 5),
       ((-1.4, 0.0, -5.4), 3, 0.8, 5), ((1.6, 0.0, -5.6), 2, 0.9, 6), ((-6.7, 0.0, 1.4), 1, 0.9, 5), ((6.8, 0.0, 0.8), 0, 0.9, 5),
       ((4.6, 0.0, -3.0), 4, 0.6, 4), ((-4.4, 0.0, -0.6), 4, 0.55, 4), ((-2.6, 1.0, -5.2), 0, 0.5, 4), ((3.4, 1.6, -5.2), 1, 0.5, 4)]
for i, (c, ci, s, n) in enumerate(_cl):
    PARTS.append(Part(crystal_cluster(c, CRY_COLS[ci], s, n, 50 + i), CRY_COLS[ci], 'crystal', 'crystal', emit=0.55,
                      glow=(np.array(c) + [0, s * 0.5, 0], CRY_COLS[ci], s)))
    CRYSTALS.append((np.array(c) + [0, s * 0.5, 0], np.array(CRY_COLS[ci], float), s))
for i, (x, z) in enumerate([(-3.0, -2.6), (2.4, -3.2), (4.2, 1.6), (-4.6, 2.0)]):          # hanging from the roof
    c = (x, ceil_y(x, z) + 0.05, z)
    ci = i % 4
    PARTS.append(Part(crystal_cluster(c, CRY_COLS[ci], 0.6, 4, 80 + i, down=True), CRY_COLS[ci], 'crystal', 'crystal', emit=0.55,
                      glow=(np.array(c) - [0, 0.3, 0], CRY_COLS[ci], 0.6), cast=False))
    CRYSTALS.append((np.array(c) - [0, 0.3, 0], np.array(CRY_COLS[ci], float), 0.6))

# ------------------------------------------------------------------ troll statues (generic stone figures)
STATUE_EYES = []


def statue(x, y, z, s, yaw, seed):
    r = np.random.default_rng(seed)
    g = new_group()
    col = np.array([104, 112, 104]) * r.uniform(0.92, 1.05)
    cy_, sy_ = math.cos(yaw), math.sin(yaw)
    rot = lambda v: np.array([v[0] * cy_ + v[2] * sy_, v[1], -v[0] * sy_ + v[2] * cy_])
    at = lambda v: np.array([x, y, z]) + rot(np.array(v, float)) * s
    PARTS.append(Part(prism(x, z, 0.42 * s, y, y + 0.16 * s, 7, r1=0.38 * s), col * 0.8, 'statue', 'plinth', grp=g))
    PARTS.append(Part(icosa(at([0, 0.62, 0]), 0.5 * s, (1.0, 0.95, 0.8), rng=r), col, 'statue', 'body', grp=g))
    PARTS.append(Part(icosa(at([0, 1.22, 0.08]), 0.3 * s, (1.0, 0.9, 0.95), rng=r), col * 1.05, 'statue', 'head', grp=g))
    nose = at([0, 1.2, 0.36])
    PARTS.append(Part(icosa(nose, 0.11 * s, (0.8, 0.8, 1.4), rng=r), col * 1.08, 'statue', 'nose', grp=g))
    for sx in (-1, 1):
        PARTS.append(Part(icosa(at([sx * 0.31, 1.3, 0.0]), 0.08 * s, (0.6, 1.2, 0.6), rng=r), col, 'statue', 'ear', grp=g))
        PARTS.append(Part(icosa(at([sx * 0.42, 0.55, 0.18]), 0.17 * s, (0.8, 1.3, 0.8), rng=r), col * 0.95, 'statue', 'arm', grp=g))
        STATUE_EYES.append(at([sx * 0.11, 1.3, 0.3]))
    PARTS.append(Part(icosa(at([0, 1.5, 0]), 0.12 * s, (1.3, 0.5, 1.0), rng=r), MOSS, 'statue', 'moss', grp=g))


statue(-6.0, 2.3, -2.9, 1.0, 0.55, 7)
statue(6.0, 3.2, -2.5, 1.0, -0.55, 8)
statue(-2.3, 1.0, -5.1, 0.75, 0.15, 9)
statue(3.5, 1.6, -5.2, 0.7, -0.2, 10)

# ------------------------------------------------------------------ the cut-away front face and the mountain outside
OPEN = [(X0, 0.0)] + [(x, ceil_y(x, Z1)) for x in np.linspace(X0, X1, 15)] + [(X1, 0.0)]     # the opening outline (x, y)
SIL = [(-9.6, -1.2), (-10.4, 2.2), (-9.8, 5.0), (-10.9, 7.4), (-8.6, 9.6), (-7.9, 11.9), (-6.2, 12.6), (-4.6, 14.9),
       (-3.6, 14.6), (-2.2, 16.9), (-1.45, 16.4), (-0.85, 16.4), (-0.3, 17.8), (1.0, 16.2), (2.3, 16.6), (3.8, 14.2),
       (5.4, 13.6), (6.5, 11.4), (8.4, 10.6), (9.3, 8.0), (10.6, 6.1), (9.9, 3.2), (10.3, 0.6), (9.4, -1.2)]
ZF = Z1 + 0.6


def _cut_faces():
    """the front cut face = silhouette with the opening as a hole: fan of quads between the two outlines"""
    out = []
    sil = np.array(SIL)
    # resample both outlines by angle around a centre inside the opening
    cx, cy = 0.0, 4.0
    angs = np.linspace(-math.pi, math.pi, 49)[:-1]

    def hit(poly, a):
        d = np.array([math.cos(a), math.sin(a)])
        best = None
        P = list(poly) + [poly[0]]
        for p, q in zip(P, P[1:]):
            p, q = np.array(p) - [cx, cy], np.array(q) - [cx, cy]
            e = q - p
            den = d[0] * (-e[1]) - d[1] * (-e[0])
            if abs(den) < 1e-9:
                continue
            s = (p[0] * (-e[1]) - p[1] * (-e[0])) / den
            u = (d[0] * p[1] - d[1] * p[0]) / den
            if s > 0 and 0 <= u <= 1 and (best is None or s < best):
                best = s
        return np.array([cx, cy]) + d * best

    opening = [(X0, 0.0), (X1, 0.0)] + [(x, ceil_y(x, ZF)) for x in np.linspace(X1, X0, 15)]
    inner = [hit(opening, a) for a in angs]
    outer = [hit(sil, a) for a in angs]
    for i in range(len(angs)):
        j = (i + 1) % len(angs)
        q = np.array([[*inner[i], ZF], [*outer[i], ZF], [*outer[j], ZF], [*inner[j], ZF]])
        out.append((q, np.array([0, 0, 1.0])))
    return out


CUT = _cut_faces()
PARTS.append(Part(CUT, (112, 92, 132), 'cut', 'cut', cast=False))

# mountain exterior: the silhouette extruded back (sides) + a roof over the top, + the island underside
OUT = []
sil = SIL + [SIL[0]]
for (x0, y0), (x1, y1) in zip(sil, sil[1:]):
    zb = -11.5
    sh = 0.72                                                   # back profile shrinks (a mountain, not a slab)
    q = np.array([[x0, y0, ZF], [x1, y1, ZF], [x1 * sh, 4.0 + (y1 - 4.0) * sh, zb], [x0 * sh, 4.0 + (y0 - 4.0) * sh, zb]])
    Q, n = face(q)
    if np.dot(n, Q.mean(0) - np.array([0, 4.0, (ZF + zb) / 2])) < 0:
        n = -n
    OUT.append((Q, n))
PARTS.append(Part(OUT, (96, 100, 142), 'outside', 'mountain', cast=False))
ISLAND = []
r_ = np.random.default_rng(12)
for a in range(12):
    a0, a1 = 2 * math.pi * a / 12, 2 * math.pi * (a + 1) / 12
    rr0, rr1 = 9.6 * r_.uniform(0.9, 1.05), 9.6 * r_.uniform(0.9, 1.05)
    tip = np.array([r_.uniform(-0.6, 0.6), -9.5, -3.45])
    p0 = np.array([rr0 * math.cos(a0), -1.2, -3.45 + 8.05 * math.sin(a0)])
    p1 = np.array([rr1 * math.cos(a1), -1.2, -3.45 + 8.05 * math.sin(a1)])
    mid = (p0 + p1) / 2 * 0.6 + tip * 0.4 + r_.normal(0, 0.5, 3)
    for tri in ((p0, p1, mid), (p0, mid, tip), (mid, p1, tip)):
        Q, n = face(tri)
        if np.dot(n, Q.mean(0) - np.array([0, -4.5, -3.5])) < 0:
            n = -n
        ISLAND.append((Q, n))
PARTS.append(Part(ISLAND, (84, 72, 104), 'island', 'island', cast=False))

# ------------------------------------------------------------------ floor albedo cells (flagstone-like rock)
FLOOR_CELLS = []
_nx, _nz = 15, 10
_pts = np.array([[[X0 + (X1 - X0) * i / _nx, 0.0, Z0 + (Z1 + 0.6 - Z0) * j / _nz] for j in range(_nz + 1)] for i in range(_nx + 1)])
_pts[1:-1, 1:-1, 0] += rng.uniform(-0.3, 0.3, (_nx - 1, _nz - 1))
_pts[1:-1, 1:-1, 2] += rng.uniform(-0.3, 0.3, (_nx - 1, _nz - 1))
for i in range(_nx):
    for j in range(_nz):
        q = np.array([_pts[i, j], _pts[i + 1, j], _pts[i + 1, j + 1], _pts[i, j + 1]])
        c = q.mean(0)
        sh = 0.85 + 0.15 * ((i * 7 + j * 3) % 5) / 4
        col = np.array([88, 78, 120]) * sh * rng.uniform(0.94, 1.06)
        if rng.uniform() < 0.12:
            col = MOSS * 0.75 * rng.uniform(0.9, 1.1)
        FLOOR_CELLS.append((q - (q - c) * 0.04, col))
FLOOR_BG = np.array([[X0, 0, Z0], [X1, 0, Z0], [X1, 0, ZF], [X0, 0, ZF]])

FEET = ([(c[0], c[2], s * 0.35, 0.35) for c, col, s in CRYSTALS if c[1] < 1.2] +
        [(x, z, 0.25, 0.3) for (x, y, z) in [(-4.8, 0, -4.6), (5.3, 0, -4.4)]] +
        [(p.ctr[0], p.ctr[2], 0.3, 0.3) for p in PARTS if p.name == 'boulder'] +
        [(x, z, max(hx, hz) * 0.8, 0.35) for (x, y, z, hx, hz) in LEDGES])

STATIC = [p for p in PARTS if p.kind not in ('outside', 'island', 'cut')]
