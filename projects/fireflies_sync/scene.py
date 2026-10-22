"""Static geometry: a low-poly Elkmont-style cove forest at night.

World: y up, metres. The clearing is centred on the origin; the hero male hovers at HERO0. The camera
mostly looks toward -z, up the cove. The moon (key light) hangs low over the far ridges, straight up the
valley, so every tulip-poplar trunk throws a long shadow stripe toward the camera across the leaf litter.

Layers: foreground ferns + mossy boulders -> clearing (hero, swarm) -> creek with a mossy log and stones ->
tulip poplars, rhododendron, a red-lit trail with seated visitors (NPS red-light rule) -> open meadow ->
layered blue ridges (the "smoky" haze) under the moon."""
import math
import numpy as np
from reelkit.config import dirv
from reelkit.geometry import Obj, face, box, prism, icosa, new_group

rng = np.random.default_rng(15)

HERO0 = np.array([0.0, 1.25, 0.0])
MOON_AZ, MOON_EL = -2.0, 11.0
MOON = dirv(MOON_AZ, MOON_EL)

OBJS = []
FEET = []          # (x, z, r, strength) contact AO


def tube(a, b, r0, r1=None, n=7, rot=0.0):
    """cylinder / frustum between two points (logs, branches)"""
    a, b = np.asarray(a, float), np.asarray(b, float)
    r1 = r0 if r1 is None else r1
    d = b - a
    L = np.linalg.norm(d)
    w = d / L
    u = np.cross(w, [0, 1.0, 0])
    if np.linalg.norm(u) < 1e-6:
        u = np.cross(w, [1.0, 0, 0])
    u /= np.linalg.norm(u)
    v = np.cross(w, u)
    ring = lambda c, r: [c + r * (math.cos(rot + 2 * math.pi * k / n) * u + math.sin(rot + 2 * math.pi * k / n) * v) for k in range(n)]
    A, B = ring(a, r0), ring(b, r1)
    ctr = (a + b) / 2
    out = [face([A[k], A[(k + 1) % n], B[(k + 1) % n], B[k]], ctr) for k in range(n)]
    out.append(face(A[::-1], ctr))
    out.append(face(B, ctr))
    return out


# ================================================================== tulip poplars
BARK = [(142, 122, 100), (132, 116, 96), (150, 128, 102), (126, 112, 94)]
TRUNKS = []        # (x, z, r)
_near = [(-2.35, 1.7, 0.30), (2.75, 0.6, 0.36), (-3.7, -2.6, 0.42), (4.1, -3.4, 0.38), (-1.45, -7.0, 0.40),
         (1.95, -8.4, 0.34), (-5.6, -9.2, 0.46), (5.3, -12.6, 0.40), (-3.1, -13.6, 0.38), (0.7, -17.0, 0.44),
         (-7.4, -16.5, 0.50), (8.4, -18.5, 0.46), (-5.1, 3.4, 0.34), (5.6, 2.8, 0.40), (-9.0, -4.0, 0.48),
         (9.4, -6.5, 0.44), (-2.4, -21.5, 0.40), (3.4, -23.5, 0.42), (-10.5, -22.0, 0.52), (11.0, -25.0, 0.5)]
TRUNKS += _near
for _ in range(400):
    if len(TRUNKS) >= 46:
        break
    x, z = rng.uniform(-22, 22), rng.uniform(-30, -5)
    if abs(x) < 1.6 and z > -26:
        continue
    if min(math.hypot(x - a, z - b) for a, b, r in TRUNKS) < 2.6:
        continue
    TRUNKS.append((x, z, rng.uniform(0.34, 0.55)))
TRUNK_H = 26.0
for i, (x, z, r) in enumerate(TRUNKS):
    g = new_group()
    col = np.array(BARK[i % 4], float)
    OBJS.append(Obj(prism(x, z, r * 1.55, 0.0, 0.55, 7, r1=r * 1.02, rot=i * 0.7), col * 0.92, 'trunk', grp=g))
    OBJS.append(Obj(prism(x, z, r * 1.02, 0.55, TRUNK_H, 7, r1=r * 0.72, rot=i * 0.7), col, 'trunk', grp=g, jitter=0.07))
    FEET.append((x, z, r * 1.5, 0.45))
    if z < -16:              # crowns only where a wide shot can see them
        rr = np.random.default_rng(100 + i)
        for k in range(3):
            c = [x + rr.uniform(-1.5, 1.5), rr.uniform(17, 23), z + rr.uniform(-1.5, 1.5)]
            OBJS.append(Obj(icosa(c, rr.uniform(2.6, 3.6), (1.2, 0.8, 1.2), rng=rr), (46, 74, 58), 'crown', grp=g, cast=False))

# ================================================================== creek (centreline in XZ), log, stones
CREEK = np.array([[-16, -3.4], [-9, -4.6], [-4, -5.3], [0, -5.6], [3.5, -6.6], [7, -9.5], [10, -14], [12, -22], [13, -34], [12, -48], [9, -64]], float)
CREEK_HW = 0.62
for (x, z, r) in [(-2.6, -4.5, 0.28), (-1.1, -6.15, 0.22), (1.4, -5.15, 0.3), (2.9, -7.2, 0.24), (4.6, -7.5, 0.34),
                  (-5.3, -4.3, 0.36), (6.2, -9.9, 0.26), (-0.2, -5.0, 0.18), (8.3, -10.4, 0.3)]:
    OBJS.append(Obj(icosa([x, r * 0.35, z], r, (1.3, 0.62, 1.0), rng=rng), (112, 116, 112), 'stone'))
    FEET.append((x, z, r, 0.35))
g = new_group()
OBJS.append(Obj(tube([-3.8, 0.24, -6.6], [-0.6, 0.3, -4.1], 0.24, 0.2, n=7), (96, 84, 70), 'log', grp=g))
OBJS.append(Obj(tube([-3.0, 0.42, -5.95], [-1.0, 0.46, -4.4], 0.12, 0.1, n=5), (88, 132, 70), 'moss', grp=g, cast=False))

# ================================================================== mossy boulders
BOULDERS = [(-1.9, 2.6, 0.55), (1.55, 2.2, 0.38), (-4.3, -1.0, 0.75), (3.4, -1.6, 0.6), (-6.4, -6.2, 0.9),
            (6.0, -5.0, 0.7), (-0.9, -10.5, 0.8), (4.0, -15.5, 0.9)]
for (x, z, r) in BOULDERS:
    g = new_group()
    rr = np.random.default_rng(int(abs(x * 100 + z * 7)))
    faces = icosa([x, r * 0.42, z], r, (1.3, 0.72, 1.1), rng=rr, jit=0.18)
    OBJS.append(Obj([f for f in faces if f[1][1] < 0.45], (118, 120, 114), 'rock', grp=g))
    OBJS.append(Obj([f for f in faces if f[1][1] >= 0.45], (94, 138, 74), 'moss', grp=g))
    FEET.append((x, z, r * 1.1, 0.5))

# ================================================================== rhododendron thickets (mid / back)
for (x, z, r) in [(-6.5, -3.0, 1.2), (-8.0, -11.0, 1.5), (6.8, -8.2, 1.3), (-4.5, -17.0, 1.4), (7.5, -15.5, 1.5),
                  (-11.5, -8.0, 1.6), (11.5, -12.0, 1.6), (-1.6, -14.5, 1.1), (2.6, -19.5, 1.3), (-6.0, 5.5, 1.3)]:
    g = new_group()
    rr = np.random.default_rng(int(abs(x * 31 + z * 17)))
    for k in range(3):
        c = [x + rr.uniform(-0.7, 0.7) * r, r * rr.uniform(0.55, 0.8), z + rr.uniform(-0.5, 0.5) * r]
        OBJS.append(Obj(icosa(c, r * rr.uniform(0.6, 0.85), (1.25, 0.8, 1.1), rng=rr), (52, 92, 62), 'rhodo', grp=g))
    FEET.append((x, z, r, 0.5))

# ================================================================== ferns: rosettes of arching fronds
FERN_COLS = [(86, 140, 70), (98, 150, 76), (78, 128, 64), (104, 148, 72)]
FERNS = []         # (x, z, size) - fireflies rise from these


def fern(x, z, s, seed):
    r = np.random.default_rng(seed)
    nf = int(r.integers(5, 8))
    faces = []
    for k in range(nf):
        a = 2 * math.pi * k / nf + r.uniform(-0.3, 0.3)
        L = s * r.uniform(0.75, 1.0)
        rise = r.uniform(0.55, 0.85)
        pts, ws = [], []
        for j in range(5):
            u = j / 4
            rad = L * u
            y = 0.02 + L * rise * (u * 1.35 - u * u * 0.95)
            pts.append(np.array([x + math.cos(a) * rad, y, z + math.sin(a) * rad]))
            ws.append(L * 0.13 * math.sin(math.pi * min(0.98, u * 0.9 + 0.08)))
        side = np.array([-math.sin(a), 0, math.cos(a)])
        for j in range(4):
            p0, p1 = pts[j], pts[j + 1]
            q = [p0 - side * ws[j], p1 - side * ws[j + 1], p1 + side * ws[j + 1], p0 + side * ws[j]]
            faces.append(face(q, up=True))
    return faces


for _ in range(900):
    if len(FERNS) >= 120:
        break
    x, z = rng.uniform(-8, 8), rng.uniform(-9, 6)
    ax_, az_, bx_, bz_ = 0.4, 3.6, 0.05, -0.4             # keep the low-camera lane to the hero clear
    u_ = min(1, max(0, ((x - ax_) * (bx_ - ax_) + (z - az_) * (bz_ - az_)) / ((bx_ - ax_) ** 2 + (bz_ - az_) ** 2)))
    if math.hypot(x - ax_ - u_ * (bx_ - ax_), z - az_ - u_ * (bz_ - az_)) < 0.8:
        continue
    if abs(np.interp(x, CREEK[:, 0], CREEK[:, 1]) - z) < CREEK_HW + 0.35:
        continue
    if min(math.hypot(x - a, z - b) - r for a, b, r in TRUNKS) < 0.35:
        continue
    if min(math.hypot(x - a, z - b) - r for a, b, r in BOULDERS) < 0.25:
        continue
    if FERNS and min(math.hypot(x - a, z - b) for a, b, s in FERNS) < 0.75:
        continue
    s = rng.uniform(0.35, 0.62)
    FERNS.append((x, z, s))
    OBJS.append(Obj(fern(x, z, s, len(FERNS) * 13 + 5), FERN_COLS[len(FERNS) % 4], 'fern', double=True))
    FEET.append((x, z, s * 0.5, 0.35))

# ================================================================== the trail (beyond the creek, right) + visitors
TRAIL = np.array([[2.0, -8.6], [3.6, -10.8], [4.6, -14.0], [5.4, -18.0], [6.3, -22.5], [7.0, -28.0], [7.4, -36.0]], float)
TRAIL_HW = 0.42
RED_LIGHTS = []    # dim red-filtered lights pointed at the ground
VISITORS = []
for (x, z, face_dir) in [(4.05, -12.2, -0.6), (5.0, -16.2, -0.9), (6.0, -20.6, -0.5), (6.7, -25.4, -0.8)]:
    g = new_group()
    # a seated low-poly figure (dark jacket), facing the cove
    c, s = math.cos(face_dir), math.sin(face_dir)
    OBJS.append(Obj(box(x, 0.28, z, 0.17, 0.28, 0.12, rot=face_dir), (44, 50, 66), 'visitor', grp=g))
    OBJS.append(Obj(icosa([x, 0.68, z], 0.11, rng=rng), (60, 54, 52), 'visitor', grp=g))
    OBJS.append(Obj(box(x + 0.16 * s, 0.1, z - 0.16 * c, 0.15, 0.08, 0.1, rot=face_dir), (40, 46, 60), 'visitor', grp=g))
    VISITORS.append((x, z))
    RED_LIGHTS.append(np.array([x + 0.28 * s - 0.1, 0.12, z - 0.28 * c + 0.1]))
    FEET.append((x, z, 0.2, 0.4))

# ================================================================== a barred owl on a dead branch (small life)
OWL_TRUNK = 2               # (-3.7, -2.6)
OWL = np.array([-3.12, 3.05, -2.42])
g = new_group()
tx, tz, tr = TRUNKS[OWL_TRUNK]
OBJS.append(Obj(tube([tx + tr * 0.6, 2.86, tz + 0.08], [OWL[0] + 0.32, 2.93, OWL[2] + 0.06], 0.05, 0.03, n=5), (86, 80, 72), 'branch', grp=g))
OWL_BODY = Obj(icosa(OWL + [0, 0.17, 0], 0.15, (1.0, 1.35, 0.9), rng=np.random.default_rng(7)), (128, 112, 92), 'owl', grp=g)
OBJS.append(OWL_BODY)
OWL_GRP = g

# ================================================================== far layer: meadow trees, ridges
FAR = []           # (faces, colour, name) - fogged, unshadowed
for _ in range(220):
    x, z = rng.uniform(-80, 80), rng.uniform(-110, -32)
    if abs(x - (-z) * math.tan(math.radians(MOON_AZ))) < 5 + 0.24 * (-z - 30):     # keep the moon's corridor open
        continue
    h = rng.uniform(14, 22)
    rr = rng.uniform(2.6, 4.2)
    f = prism(x, z, 0.4, 0, h * 0.7, 5) + icosa([x, h * 0.78, z], rr, (1.1, 1.0, 1.1), rng=rng)
    FAR.append((f, np.array([44, 70, 62]) * rng.uniform(0.9, 1.1), 'tree'))
RIDGES = []        # (z, base, amp, n, seed, colour)
for (z, base, amp, n, seed, col) in [(-150, 12, 18, 34, 3, (58, 84, 112)), (-230, 26, 30, 30, 5, (76, 104, 140)),
                                      (-330, 30, 34, 26, 8, (98, 124, 164))]:
    r = np.random.default_rng(seed)
    xs = np.linspace(-420, 420, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(xs * 0.012 + r.uniform(0, 6))) * (0.6 + 0.4 * np.sin(xs * 0.031 + r.uniform(0, 6))) + r.uniform(-amp * 0.15, amp * 0.15, n)
    xm = (-z) * math.tan(math.radians(MOON_AZ))
    ys = ys - 0.45 * amp * np.exp(-((xs - xm) / 120.0) ** 2)                   # a saddle under the moon
    faces = [(np.array([[xs[i], ys[i], z], [xs[i + 1], ys[i + 1], z], [xs[i + 1], -20, z + 30], [xs[i], -20, z + 30]]),
              np.array([0, 0.6, 0.8])) for i in range(n - 1)]
    RIDGES.append((faces, np.array(col, float)))

STARS = rng.uniform(-1, 1, (260, 3))
STARS[:, 1] = np.abs(STARS[:, 1]) * 0.6 + 0.08
STARS /= np.linalg.norm(STARS, axis=1, keepdims=True)
STARS = STARS[(STARS[:, 2] < -0.2)]
STAR_B = rng.uniform(0.35, 1.0, len(STARS))


def creek_dist(X, Z):
    """distance (m) from ground points to the creek centreline"""
    d = np.full(np.shape(X), 1e9)
    for (x0, z0), (x1, z1) in zip(CREEK[:-1], CREEK[1:]):
        ex, ez = x1 - x0, z1 - z0
        u = np.clip(((X - x0) * ex + (Z - z0) * ez) / (ex * ex + ez * ez), 0, 1)
        d = np.minimum(d, np.hypot(X - x0 - u * ex, Z - z0 - u * ez))
    return d


def trail_dist(X, Z):
    d = np.full(np.shape(X), 1e9)
    for (x0, z0), (x1, z1) in zip(TRAIL[:-1], TRAIL[1:]):
        ex, ez = x1 - x0, z1 - z0
        u = np.clip(((X - x0) * ex + (Z - z0) * ez) / (ex * ex + ez * ez), 0, 1)
        d = np.minimum(d, np.hypot(X - x0 - u * ex, Z - z0 - u * ez))
    return d


def clear_of_trunks(x, z, margin):
    return min(math.hypot(x - a, z - b) - r for a, b, r in TRUNKS) > margin
