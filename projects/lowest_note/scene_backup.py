"""Static geometry + builders for the moving parts.

The diorama: a floating low-poly island (top at y = 0) whose middle is a round basin of glowing hot
gas (the Perseus cluster). The central galaxy (black hole + jets) floats above the gas; its bubbles
launch the pressure rings that run across the gas surface. Around the basin rim: a tiny observatory
with a warm lamp, radio dishes, crystals and rocks. A Chandra-style satellite hovers in front;
member galaxies orbit; stars, nebula and far galaxies fill the backdrop."""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, lathe, icosa, face, new_group
from reelkit.config import dirv

rng = np.random.default_rng(21)

R_TOP = 5.0                    # island top radius
R_POOL = 3.4                   # gas basin radius
R_LIP = 3.62
POOL_Y = 0.18                  # gas surface height
LIP_Y = 0.26
CORE = np.array([0.0, 2.0, 0.0])
SUBJECT = CORE                 # locked focus target
JET_AX = np.array([-0.12, 1.0, 0.05]) / np.linalg.norm([-0.12, 1.0, 0.05])
STAR_D = dirv(38, 36)          # the bright blue star (visible key/rim light) - back right, high
LAMP = None                    # set below

OBJS = []                      # static, shaded props (painter-sorted with the moving parts)
UNDER = []                     # island underside (drawn before the floor)


def _jag(r, k=0.06):
    return r * (1 + rng.uniform(-k, k))


# ---------------------------------------------------------------- island underside (inverted rocky cone)
def _island():
    n = 18
    rings = [(0.0, R_TOP, 0.0), (-0.42, R_TOP * 0.99, 0.01), (-0.55, R_TOP * 0.955, 0.03), (-1.5, 3.9, 0.08), (-2.7, 2.3, 0.12), (-3.7, 0.9, 0.15), (-4.4, 0.0, 0.0)]
    V = []
    for y, r, j in rings:
        row = []
        for k in range(n):
            a = 2 * math.pi * (k + (0.5 if len(V) % 2 else 0)) / n
            rr = r * (1 + rng.uniform(-j, j)) if r > 0 else 0
            yy = y + (rng.uniform(-0.15, 0.15) if 0 < len(V) < len(rings) - 1 and j > 0.05 else 0)
            row.append(np.array([rr * math.cos(a), yy, rr * math.sin(a)]))
        V.append(row)
    cols = [(150, 120, 170), (176, 136, 140), (138, 104, 160), (124, 92, 156), (112, 80, 150), (100, 70, 140)]
    for i in range(len(V) - 1):
        for k in range(n):
            k2 = (k + 1) % n
            a, b, c, d = V[i][k], V[i][k2], V[i + 1][k2], V[i + 1][k]
            ctr = np.array([0, (a[1] + d[1]) / 2, 0])
            col = cols[min(i, len(cols) - 1)]
            if i == 0:
                col = (214, 176, 112) if k % 2 == 0 else (200, 160, 100)        # gold trim band
            UNDER.append(Obj([face([a, b, c], ctr), face([a, c, d], ctr)], col, 'under', cast=False, jitter=0.07))


_island()

# top edge polygon (for the floor albedo)
TOP_POLY = np.array([[R_TOP * math.cos(a), 0.0, R_TOP * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 37)[:-1]])

# regolith: triangulated annulus between the lip and the island edge (per-face tones)
REGO = []
_rad = [R_LIP - 0.02, 3.95, 4.3, 4.65, R_TOP + 0.01]
_nseg = [36, 40, 44, 48, 52]
_pts = []
for ri, (r, n) in enumerate(zip(_rad, _nseg)):
    off = rng.uniform(0, 1)
    row = []
    for k in range(n):
        a = 2 * math.pi * (k + off) / n
        rj = r if ri in (0, len(_rad) - 1) else r + rng.uniform(-0.08, 0.08)
        row.append((rj * math.cos(a + rng.uniform(-0.02, 0.02)), rj * math.sin(a)))
    _pts.append(row)
for ri in range(len(_pts) - 1):
    A, B = _pts[ri], _pts[ri + 1]
    i = j = 0
    na, nb = len(A), len(B)
    while i < na or j < nb:
        aa = math.atan2(A[i % na][1], A[i % na][0]) % (2 * math.pi) + (2 * math.pi if i >= na else 0)
        bb = math.atan2(B[j % nb][1], B[j % nb][0]) % (2 * math.pi) + (2 * math.pi if j >= nb else 0)
        if (aa <= bb and i < na) or j >= nb:
            tri = [A[i % na], A[(i + 1) % na], B[j % nb]]
            i += 1
        else:
            tri = [A[i % na], B[(j + 1) % nb], B[j % nb]]
            j += 1
        base = np.array([150, 138, 178]) * rng.uniform(0.86, 1.1)
        if rng.uniform() < 0.12:
            base = np.array([176, 150, 150]) * rng.uniform(0.9, 1.05)
        REGO.append((np.array([[p[0], 0.002, p[1]] for p in tri]), base))

CRATERS = [(4.3, 2.3, 0.28), (4.4, -0.6, 0.2), (-4.2, 1.6, 0.24), (2.0, 4.0, 0.22), (-1.8, 4.2, 0.18), (3.0, -3.3, 0.26)]

# ---------------------------------------------------------------- basin lip (does not cast AABB shadows)
LIP = []
_n = 40
for k in range(_n):
    a0, a1 = 2 * math.pi * k / _n, 2 * math.pi * (k + 1) / _n
    c0, s0, c1, s1 = math.cos(a0), math.sin(a0), math.cos(a1), math.sin(a1)
    am = (a0 + a1) / 2
    ctr = np.array([((R_POOL + R_LIP) / 2) * math.cos(am), 0.1, ((R_POOL + R_LIP) / 2) * math.sin(am)])
    outer = face([[R_LIP * c0, 0, R_LIP * s0], [R_LIP * c1, 0, R_LIP * s1], [R_LIP * c1, LIP_Y, R_LIP * s1], [R_LIP * c0, LIP_Y, R_LIP * s0]], ctr)
    top = face([[R_LIP * c0, LIP_Y, R_LIP * s0], [R_LIP * c1, LIP_Y, R_LIP * s1], [R_POOL * c1, LIP_Y, R_POOL * s1], [R_POOL * c0, LIP_Y, R_POOL * s0]], up=True)
    inner = face([[R_POOL * c0, LIP_Y, R_POOL * s0], [R_POOL * c1, LIP_Y, R_POOL * s1], [R_POOL * c1, POOL_Y - 0.02, R_POOL * s1], [R_POOL * c0, POOL_Y - 0.02, R_POOL * s0]], ctr)
    LIP.append(Obj([outer, top, inner], (226, 196, 150) if k % 2 else (212, 182, 138), 'lip', cast=False, jitter=0.03))
OBJS += LIP

# ---------------------------------------------------------------- rim props
FEET = []


def at(r, deg):
    a = math.radians(deg)
    return r * math.cos(a), r * math.sin(a)


# observatory (back left) with a warm-lit slit + windows, and a lamp post (the warm local light)
ox, oz = at(4.35, 228)
g = new_group()
OBJS.append(Obj(prism(ox, oz, 0.5, 0.0, 0.55, 10, rot=0.1), (232, 222, 210), 'obs', grp=g))
OBJS.append(Obj(lathe(ox, oz, [(0.0, 0.53), (0.18, 0.5), (0.34, 0.38), (0.46, 0.2), (0.5, 0.0)], n=10, cy=0.55), (210, 214, 232), 'dome', grp=g))
_tow = -np.array([ox, 0, oz]); _tow /= np.linalg.norm(_tow)
_side = np.cross([0, 1, 0], _tow)
_sc = np.array([ox, 0.86, oz]) + _tow * 0.36
OBJS.append(Obj([face([_sc - _side * 0.07, _sc + _side * 0.07, _sc + _side * 0.07 + _tow * 0.08 + [0, 0.17, 0], _sc - _side * 0.07 + _tow * 0.08 + [0, 0.17, 0]], _sc - _tow)],
                (255, 196, 110), 'slit', grp=g, cast=False))
FEET.append((ox, oz, 0.5, 0.45))
# windows facing the camera side
WINDOWS = []
for da in (-0.55, 0.15):
    a = math.pi * 0.5 + da
    nrmw = np.array([math.cos(a), 0, math.sin(a)])
    c = np.array([ox, 0.3, oz]) + nrmw * 0.505
    tg = np.cross([0, 1, 0], nrmw)
    WINDOWS.append(face([c - tg * 0.07 - [0, 0.08, 0], c + tg * 0.07 - [0, 0.08, 0], c + tg * 0.07 + [0, 0.08, 0], c - tg * 0.07 + [0, 0.08, 0]], c - nrmw))
OBJS.append(Obj(WINDOWS, (255, 200, 120), 'win', grp=g, cast=False))
lx, lz = at(4.25, 252)
LAMP = np.array([lx, 0.95, lz])
g = new_group()
OBJS.append(Obj(prism(lx, lz, 0.035, 0.0, 0.86, 6), (70, 60, 76), 'post', grp=g))
OBJS.append(Obj(prism(lx, lz, 0.09, 0.86, 1.04, 6, r1=0.06), (255, 200, 110), 'lamp', grp=g, cast=False))
FEET.append((lx, lz, 0.05, 0.3))

# radio dishes (turn slowly to track the core)
DISHES = []
for r, deg, s in ((4.45, 205, 0.36), (4.5, 168, 0.3), (4.3, 300, 0.26)):
    x, z = at(r, deg)
    DISHES.append((np.array([x, 0.0, z]), s, deg))
    g = new_group()
    OBJS.append(Obj(prism(x, z, 0.04, 0.0, 0.42 * s / 0.36, 6), (190, 186, 210), 'mast', grp=g))
    FEET.append((x, z, 0.08, 0.3))

# crystal clusters (cool, slightly emissive)
CRYSTALS = []
for r, deg, col in ((4.25, 330, (110, 236, 230)), (4.55, 20, (255, 140, 210)), (4.1, 62, (120, 220, 255)), (4.6, 105, (140, 255, 200)),
                    (4.4, 140, (255, 150, 200)), (4.55, 278, (130, 210, 255))):
    x0, z0 = at(r, deg)
    g = new_group()
    for j in range(rng.integers(3, 6)):
        dx, dz = rng.uniform(-0.18, 0.18, 2)
        h = rng.uniform(0.28, 0.7)
        rr = rng.uniform(0.05, 0.09)
        tilt = (rng.uniform(-0.35, 0.35), rng.uniform(-0.35, 0.35))
        x, z = x0 + dx, z0 + dz
        base = prism(x, z, rr, 0.0, h, 6, r1=rr * 0.8, rot=rng.uniform(0, 1))
        tip_top = np.array([x + tilt[0] * h, h + rr * 2.2, z + tilt[1] * h])
        faces = []
        for Q, n in base[:-1]:
            Q = Q.copy()
            Q[:, 0] += tilt[0] * Q[:, 1]
            Q[:, 2] += tilt[1] * Q[:, 1]
            faces.append(face(Q, np.array([x + tilt[0] * h / 2, h / 2, z + tilt[1] * h / 2])))
        cap = base[-1][0].copy()
        cap[:, 0] += tilt[0] * cap[:, 1]
        cap[:, 2] += tilt[1] * cap[:, 1]
        for k in range(len(cap)):
            faces.append(face([cap[k], cap[(k + 1) % len(cap)], tip_top], np.array([x + tilt[0] * h, h, z + tilt[1] * h])))
        o = Obj(faces, np.array(col) * rng.uniform(0.9, 1.05), 'crystal', grp=g, jitter=0.08)
        o.emit = 0.18
        OBJS.append(o)
        CRYSTALS.append((np.array([x + tilt[0] * h, h, z + tilt[1] * h]), col))
    FEET.append((x0, z0, 0.22, 0.35))

# rocks
for r, deg, s in ((4.0, 10, 0.16), (4.75, 48, 0.22), (3.95, 86, 0.12), (4.7, 128, 0.18), (3.9, 190, 0.14), (4.8, 236, 0.2),
                  (4.05, 262, 0.13), (4.75, 312, 0.2), (3.92, 352, 0.1), (4.1, 160, 0.12)):
    x, z = at(r, deg)
    OBJS.append(Obj(icosa([x, s * 0.55, z], s, (1.2, 0.75, 1.0), rng=rng), (150, 130, 170), 'rock', jitter=0.08))
    FEET.append((x, z, s * 0.9, 0.35))

for o in OBJS:
    if not hasattr(o, 'emit'):
        o.emit = 0.0

# ---------------------------------------------------------------- backdrop
STARS = []
for _ in range(520):
    v = rng.normal(0, 1, 3)
    v /= np.linalg.norm(v)
    b = rng.uniform(0.25, 1.0) ** 2.2
    tw = rng.integers(1, 6)
    STARS.append((v, b, tw, rng.uniform(0, 6.28), rng.choice([0, 1, 2], p=[0.6, 0.25, 0.15])))
STAR_V = np.array([s[0] for s in STARS])
STAR_B = np.array([s[1] for s in STARS])
STAR_TW = np.array([s[2] for s in STARS])
STAR_PH = np.array([s[3] for s in STARS])
STAR_C = np.array([[(255, 250, 240), (180, 210, 255), (255, 200, 220)][s[4]] for s in STARS], float)

FAR_GALAXIES = [  # centre, radius, normal, colours (core, arm), spin speed (rev per 60 s)
    (np.array([-60.0, 34.0, -170.0]), 26.0, np.array([0.3, 0.75, 0.6]), (255, 220, 200), (150, 120, 230), 1),
    (np.array([95.0, -40.0, -210.0]), 34.0, np.array([-0.4, 0.5, 0.75]), (255, 210, 240), (110, 150, 255), -1),
    (np.array([20.0, 70.0, -240.0]), 18.0, np.array([0.1, 0.4, 0.9]), (255, 240, 210), (230, 130, 200), 1),
]

# ---------------------------------------------------------------- moving parts (builders)


def ridge(R, h, w, y0=POOL_Y, n=None):
    """one pressure-wave crest: a low triangular ridge running round the basin (inner + outer slope)"""
    n = n or int(max(14, min(40, 10 * R + 10)))
    out = []
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        c0, s0, c1, s1 = math.cos(a0), math.sin(a0), math.cos(a1), math.sin(a1)
        am = (a0 + a1) / 2
        cen = np.array([R * math.cos(am), y0 + h / 3, R * math.sin(am)])
        ib0, ib1 = [(R - w) * c0, y0, (R - w) * s0], [(R - w) * c1, y0, (R - w) * s1]
        t0, t1 = [R * c0, y0 + h, R * s0], [R * c1, y0 + h, R * s1]
        ob0, ob1 = [(R + w) * c0, y0, (R + w) * s0], [(R + w) * c1, y0, (R + w) * s1]
        out.append(face([ib0, ib1, t1, t0], cen))
        out.append(face([t0, t1, ob1, ob0], cen))
    return out


def sphere(c, r, n=12, m=7):
    """rounder low-poly sphere (bubbles)"""
    prof = [(r * (1 - math.cos(math.pi * j / m)), r * math.sin(math.pi * j / m)) for j in range(m + 1)]
    prof[0] = (0.0, 0.0)
    prof[-1] = (2 * r, 0.0)
    return lathe(c[0], c[2], prof, n=n, cy=c[1] - r)


def ellipsoid(c, r, sq, rng_seed):
    return icosa(c, r, sq, jit=0.08, rng=np.random.default_rng(rng_seed))


def cone(base, tip, r, n=6):
    """open cone (jets)"""
    base, tip = np.asarray(base, float), np.asarray(tip, float)
    ax = tip - base
    ax /= np.linalg.norm(ax)
    u = np.cross(ax, [0, 0, 1.0])
    if np.linalg.norm(u) < 1e-3:
        u = np.cross(ax, [1.0, 0, 0])
    u /= np.linalg.norm(u)
    v = np.cross(ax, u)
    ring = [base + r * (math.cos(2 * math.pi * k / n) * u + math.sin(2 * math.pi * k / n) * v) for k in range(n)]
    return [face([ring[k], ring[(k + 1) % n], tip], base) for k in range(n)]


def disc_galaxy(c, R, nrm, spin, arms=2, n=14):
    """flat spiral galaxy: a ring of wedge triangles, arms = brighter wedges twisting with radius"""
    nrm = np.asarray(nrm, float) / np.linalg.norm(nrm)
    u = np.cross(nrm, [0, 1.0, 0.001])
    u /= np.linalg.norm(u)
    v = np.cross(nrm, u)
    out = []
    for ring_i, (r0, r1) in enumerate(((0.0, 0.42), (0.42, 0.75), (0.75, 1.0))):
        for k in range(n):
            a0 = spin + 2 * math.pi * k / n + ring_i * 0.55
            a1 = spin + 2 * math.pi * (k + 1) / n + ring_i * 0.55
            P = lambda rr, a: c + R * rr * (math.cos(a) * u + math.sin(a) * v)
            arm = (k * arms // 2 + ring_i) % (n // arms) < 2 if False else ((k + ring_i * 2) % (n // arms)) < 3
            if r0 == 0:
                Q = [c, P(r1, a0), P(r1, a1)]
            else:
                Q = [P(r0, a0), P(r0, a1), P(r1, a1), P(r1, a0)]
            out.append((np.array(Q), nrm, ring_i, arm))
    return out


def satellite(c, yaw, roll=0.0, sc=1.0):
    """Chandra-style X-ray telescope: long gold/silver tube, bus, two solar wings. Returns [(faces, colour, emit)]"""
    c = np.asarray(c, float)
    cy, sy = math.cos(yaw), math.sin(yaw)
    cr, sr = math.cos(roll), math.sin(roll)

    def T(p):
        x, y, z = p
        y, z = y * cr - z * sr, y * sr + z * cr
        return c + sc * np.array([x * cy - z * sy, y, x * sy + z * cy])

    def tf(faces):
        return [face([T(q) for q in Q], T([0, 0, 0])) for Q, n in faces]
    parts = []
    tube = []
    n = 8
    L0, L1, rt = -0.9, 0.75, 0.17
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        Q = [[L0, rt * math.cos(a0), rt * math.sin(a0)], [L1, rt * math.cos(a0), rt * math.sin(a0)],
             [L1, rt * math.cos(a1), rt * math.sin(a1)], [L0, rt * math.cos(a1), rt * math.sin(a1)]]
        tube.append(face(Q, [(L0 + L1) / 2, 0, 0]))
    parts.append((tf(tube[:4]), (255, 206, 96), 0.2))
    parts.append((tf(tube[4:]), (236, 236, 248), 0.1))
    cap = [[L0, rt * math.cos(2 * math.pi * k / n), rt * math.sin(2 * math.pi * k / n)] for k in range(n)]
    parts.append((tf([face(cap, [0, 0, 0])]), (70, 70, 96), 0.0))
    parts.append((tf(box(L1 + 0.16, 0, 0, 0.16, 0.2, 0.2)), (226, 180, 90), 0.0))
    for sg in (1, -1):
        wing = []
        for j in range(3):
            z0 = sg * (0.26 + j * 0.3)
            z1 = z0 + sg * 0.27
            wing.append(face([[L1 + 0.02, 0.0, z0], [L1 + 0.3, 0.0, z0], [L1 + 0.3, 0.0, z1], [L1 + 0.02, 0.0, z1]], [L1 + 0.16, -1, z0]))
        parts.append((tf(wing), (70, 120, 255), 0.3))
        parts.append((tf([face([[L1 + 0.15, -0.02, sg * 0.2], [L1 + 0.17, -0.02, sg * 0.2], [L1 + 0.17, 0.02, sg * 0.26], [L1 + 0.15, 0.02, sg * 0.26]], [L1 + 0.16, 0, 0])]), (180, 180, 200), 0.0))
    door = [[L0, rt, -rt], [L0, rt, rt], [L0 - 0.3, rt + 0.22, rt], [L0 - 0.3, rt + 0.22, -rt]]
    parts.append((tf([face(door, [L0, 0, 0])]), (226, 226, 236), 0.0))
    return parts, T([L1 + 0.34, 0.0, 0.0]), T([L0, 0.0, 0.0])


GALAXIES = [  # kind, orbit r, height, phase, rev per 60 s, size, colour
    ('spiral', 3.5, 4.4, 3.53, 1, 0.42, (255, 214, 170)),
    ('ellip', 3.05, 1.05, 2.1, 1, 0.2, (255, 190, 150)),
    ('spiral', 4.3, 3.6, 3.7, -1, 0.5, (220, 200, 255)),
    ('ellip', 2.4, 4.75, 0.92, 1, 0.16, (255, 226, 190)),
    ('spiral', 4.55, 1.55, 1.15, 1, 0.34, (255, 190, 220)),
    ('ellip', 3.75, 2.3, 5.5, -1, 0.22, (255, 236, 200)),
    ('ellip', 4.8, 4.3, 0.9, -1, 0.14, (255, 210, 180)),
]

ROCKS = [(rng.uniform(4.4, 6.0), rng.uniform(-3.4, -0.8), rng.uniform(0, 6.28), rng.choice([-1, 1]), rng.uniform(0.12, 0.3), int(rng.integers(0, 999))) for _ in range(9)]
MOTES = [(rng.uniform(0.6, 4.2), rng.uniform(0.5, 3.6), rng.uniform(0, 6.28), int(rng.choice([1, 2, -1])), rng.uniform(0, 6.28)) for _ in range(46)]
