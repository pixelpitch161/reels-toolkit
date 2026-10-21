"""Static geometry for the summer-night meadow (units: centimetres)."""
import math
import numpy as np

rng = np.random.default_rng(4)


def ss(a, b, x):
    x = np.clip((x - a) / (b - a), 0, 1)
    return x * x * (3 - 2 * x)


# ------------------------------------------------------------------ landmarks
HERO = np.array([0.0, 40.45, 0.0])          # cricket body centre
POND = np.array([-650.0, 0.0, -1250.0])
OAK = np.array([1250.0, 0.0, -2700.0])
FARM = np.array([-3300.0, 0.0, -5800.0])
BARN = np.array([-1900.0, 0.0, -6700.0])
PUMP = np.array([2900.0, 0.0, -6200.0])
POST_X, FENCE_Z = 22.0, -30.0


def ground_h(x, z):
    r = math.hypot(x, z)
    amp = float(ss(400, 3000, r)) * min(520.0, 40 + 0.02 * r)
    h = amp * (0.5 + 0.5 * math.sin(0.0011 * x + 0.0007 * z + 0.4) * math.sin(0.0008 * z - 0.0005 * x + 1.3))
    h += float(ss(15000, 30000, r)) * 1100 * (0.5 + 0.5 * math.sin(math.atan2(x, -z) * 5 + 1.0)) ** 1.5
    for c, rad in ((POND, 700), (FARM, 1500), (BARN, 1300), (PUMP, 900), (OAK, 700)):
        h *= float(ss(rad * 0.45, rad, math.hypot(x - c[0], z - c[2])))
    return h


# ------------------------------------------------------------------ terrain
RINGS = [60, 120, 220, 380, 600, 900, 1300, 1800, 2500, 3400, 4600, 6200, 8300, 11000, 15000, 20000, 27000, 36000]
SEG = 48
_v = {}
for ri, r in enumerate(RINGS):
    for k in range(SEG):
        th = 2 * math.pi * (k + (0.5 if ri % 2 else 0)) / SEG
        rr = r * (1 + (rng.uniform(-0.07, 0.07) if ri > 1 else 0))
        th += (rng.uniform(-0.2, 0.2) * 2 * math.pi / SEG) if ri > 1 else 0
        x, z = rr * math.sin(th), -rr * math.cos(th)
        _v[(ri, k)] = np.array([x, ground_h(x, z), z])
_tri = []
for ri in range(len(RINGS) - 1):
    for k in range(SEG):
        a, b = _v[(ri, k)], _v[(ri, (k + 1) % SEG)]
        c, d = _v[(ri + 1, k)], _v[(ri + 1, (k + 1) % SEG)]
        _tri += [(a, b, c), (b, d, c)] if ri % 2 == 0 else [(a, b, d), (a, d, c)]
TER = np.array(_tri)
TER_N = np.cross(TER[:, 1] - TER[:, 0], TER[:, 2] - TER[:, 0])
TER_N /= np.linalg.norm(TER_N, axis=1)[:, None]
TER_N[TER_N[:, 1] < 0] *= -1
TER_C = TER.mean(1)
TER_TONE = rng.uniform(0.85, 1.12, len(TER))
# colour patches across the meadow (teal / green / lime)
TER_PATCH = (0.5 + 0.5 * np.sin(TER_C[:, 0] * 0.0013 + 1.0) * np.sin(TER_C[:, 2] * 0.0011 + 0.3))
# dirt path from the fence toward the farmhouse
PATH = []
for k in range(26):
    u = k / 25
    x = -40 - 3200 * u + 380 * math.sin(u * 5.0)
    z = -60 - 5200 * u
    PATH.append((x, z, 60 + 90 * u))
CENTER_DISC = np.array([[60 * math.sin(a), 0.0, -60 * math.cos(a)] for a in np.linspace(0, 2 * math.pi, SEG + 1)[:-1]])

# ------------------------------------------------------------------ grass tufts (wide)
_bl = []
for k in range(1300):
    r = 80 + 3400 * rng.uniform() ** 1.8
    a = rng.uniform(0, 2 * math.pi)
    x, z = r * math.sin(a), -r * math.cos(a)
    if math.hypot(x - POND[0], (z - POND[2]) * 1.6) < 380:
        continue
    if r < 120 and z > 0:
        continue
    y0 = ground_h(x, z)
    for j in range(3):
        bx, bz = x + rng.uniform(-6, 6), z + rng.uniform(-6, 6)
        hgt = rng.uniform(18, 48) * (1 + 0.6 * float(ss(1500, 3400, r)))
        w = rng.uniform(2.5, 4.0) * (1 + float(ss(1500, 3400, r)))
        ang = rng.uniform(0, math.pi)
        dx, dz = math.cos(ang) * w, math.sin(ang) * w
        lean = rng.uniform(-0.35, 0.35) * hgt
        _bl.append((bx - dx, y0, bz - dz, bx + dx, y0, bz + dz, bx + lean, y0 + hgt, bz + rng.uniform(-3, 3),
                    rng.uniform(0.8, 1.2), rng.uniform(0, 6.28)))
TUFT = np.array(_bl)

# ------------------------------------------------------------------ macro grass near the cricket
_mb = []
for k in range(150):
    r = 7 + 75 * rng.uniform() ** 1.3
    a = rng.uniform(0, 2 * math.pi)
    x, z = r * math.sin(a), -r * math.cos(a)
    hgt = rng.uniform(16, 68)
    w = rng.uniform(0.5, 1.0)
    ang = rng.uniform(0, math.pi)
    _mb.append((x, z, hgt, w, ang, rng.uniform(-0.3, 0.3), rng.uniform(0.8, 1.2), rng.uniform(0, 6.28)))
MACRO = np.array(_mb)

# ------------------------------------------------------------------ flowers
FLOWERS = []
for k in range(26):    # near the cricket
    r = 9 + 70 * rng.uniform()
    a = rng.uniform(0, 2 * math.pi)
    FLOWERS.append((r * math.sin(a), rng.uniform(8, 36), -r * math.cos(a), rng.uniform(1.2, 2.2), int(rng.integers(3))))
for k in range(320):   # out in the meadow
    r = 120 + 3000 * rng.uniform() ** 1.6
    a = rng.uniform(0, 2 * math.pi)
    x, z = r * math.sin(a), -r * math.cos(a)
    if math.hypot(x - POND[0], (z - POND[2]) * 1.6) < 380:
        continue
    FLOWERS.append((x, ground_h(x, z) + rng.uniform(15, 40), z, rng.uniform(3.0, 6.0), int(rng.integers(3))))
FL = np.array([f[:4] for f in FLOWERS])
FL_KIND = np.array([f[4] for f in FLOWERS])

# ------------------------------------------------------------------ pond
POND_POLY = np.array([[POND[0] + 340 * math.cos(a) * rng.uniform(0.88, 1.08), 1.0,
                       POND[2] + 210 * math.sin(a) * rng.uniform(0.88, 1.08)] for a in np.linspace(0, 2 * math.pi, 15)[:-1]])
PADS = [(POND[0] + rng.uniform(-220, 220), POND[2] + rng.uniform(-120, 120), rng.uniform(18, 30)) for _ in range(8)]

# ------------------------------------------------------------------ solids (faces with normals)


def box(cx, cy, cz, hx, hy, hz, rot=0.0):
    c, s = math.cos(rot), math.sin(rot)
    P = lambda x, y, z: np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    V = [P(sx * hx, sy * hy, sz * hz) for sy in (-1, 1) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    quads = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7), (0, 3, 2, 1)]
    ctr = np.array([cx, cy, cz])
    out = []
    for q in quads:
        Q = np.array([V[i] for i in q])
        n = np.cross(Q[1] - Q[0], Q[2] - Q[0]); n /= np.linalg.norm(n)
        if np.dot(n, Q.mean(0) - ctr) < 0:
            n = -n
        out.append((Q, n))
    return out


def prism(cx, cz, r, y0, y1, n=8, r1=None):
    r1 = r if r1 is None else r1
    out = []
    for k in range(n):
        a0, a1 = 2 * math.pi * k / n, 2 * math.pi * (k + 1) / n
        Q = np.array([[cx + r * math.cos(a0), y0, cz + r * math.sin(a0)], [cx + r * math.cos(a1), y0, cz + r * math.sin(a1)],
                      [cx + r1 * math.cos(a1), y1, cz + r1 * math.sin(a1)], [cx + r1 * math.cos(a0), y1, cz + r1 * math.sin(a0)]])
        nn = np.array([math.cos((a0 + a1) / 2), 0.15, math.sin((a0 + a1) / 2)]); nn /= np.linalg.norm(nn)
        out.append((Q, nn))
    return out


def icosa(c, r, squash=1.0):
    t = (1 + 5 ** 0.5) / 2
    V = np.array([[-1, t, 0], [1, t, 0], [-1, -t, 0], [1, -t, 0], [0, -1, t], [0, 1, t], [0, -1, -t], [0, 1, -t],
                  [t, 0, -1], [t, 0, 1], [-t, 0, -1], [-t, 0, 1]], float)
    V /= np.linalg.norm(V[0])
    V = V * r * rng.uniform(0.85, 1.12, (12, 1))
    V[:, 1] *= squash
    F = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
         (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9), (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
    out = []
    for f in F:
        Q = V[list(f)] + c
        n = np.cross(Q[1] - Q[0], Q[2] - Q[0]); n /= np.linalg.norm(n)
        if np.dot(n, Q.mean(0) - c) < 0:
            n = -n
        out.append((Q, n))
    return out


def gable_roof(cx, cy, cz, hx, hz, hr, rot, over=40):
    c, s = math.cos(rot), math.sin(rot)
    P = lambda x, y, z: np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    a, b = hx + over, hz + over
    ridge0, ridge1 = P(-a, hr, 0), P(a, hr, 0)
    e = [P(-a, 0, b), P(a, 0, b), P(a, 0, -b), P(-a, 0, -b)]
    out = []
    for Q in (np.array([e[0], e[1], ridge1, ridge0]), np.array([e[2], e[3], ridge0, ridge1])):
        n = np.cross(Q[1] - Q[0], Q[2] - Q[0]); n /= np.linalg.norm(n)
        if n[1] < 0:
            n = -n
        out.append((Q, n))
    for Q in (np.array([e[1], e[2], ridge1]), np.array([e[3], e[0], ridge0])):
        n = np.cross(Q[1] - Q[0], Q[2] - Q[0]); n /= np.linalg.norm(n)
        if np.dot(n, Q.mean(0) - np.array([cx, cy, cz])) < 0:
            n = -n
        out.append((Q, n))
    return out


SOLIDS = []      # (name, faces, base colour)
# oak tree
SOLIDS.append(('oak_trunk', prism(OAK[0], OAK[2], 55, 0, 520, 8, 38), (92, 64, 58)))
for (dx, dy, dz, r) in [(0, 900, 0, 520), (-380, 760, 120, 380), (360, 780, -60, 400), (80, 1180, -40, 380), (-160, 1080, 260, 300)]:
    SOLIDS.append(('oak_crown', icosa(OAK + np.array([dx, dy, dz]), r, 0.8), (60, 176, 118)))
# farmhouse + barn
SOLIDS.append(('house', box(FARM[0], 280, FARM[2], 520, 280, 360, 0.35), (232, 214, 196)))
SOLIDS.append(('roof', gable_roof(FARM[0], 560, FARM[2], 520, 360, 300, 0.35), (78, 66, 110)))
SOLIDS.append(('chimney', box(FARM[0] + 260, 880, FARM[2] - 60, 45, 120, 45, 0.35), (150, 90, 80)))
SOLIDS.append(('barn', box(BARN[0], 360, BARN[2], 520, 360, 420, -0.2), (214, 52, 60)))
SOLIDS.append(('barnroof', gable_roof(BARN[0], 720, BARN[2], 520, 420, 380, -0.2), (66, 54, 80)))
# fence posts + rails near the cricket
for px in (-218.0, -98.0, POST_X, 142.0, 262.0):
    SOLIDS.append(('post', box(px, 55, FENCE_Z, 4.5, 55, 4.5, 0.0), (128, 98, 84)))
for ry in (45.0, 88.0):
    SOLIDS.append(('rail', box(22, ry, FENCE_Z - 2.5, 240, 2.5, 1.2, 0.0), (140, 110, 92)))
# thermometer board on the post
THERMO = dict(x0=POST_X - 5.0, x1=POST_X + 5.0, y0=50.0, y1=96.0, z=FENCE_Z + 4.7)
TH_T0, TH_T1 = 30.0, 110.0     # scale range (F)


def thermo_y(T):
    return THERMO['y0'] + 6.0 + (T - TH_T0) / (TH_T1 - TH_T0) * (THERMO['y1'] - THERMO['y0'] - 9.0)


# farmhouse windows (emissive), on the front face
def _house_pt(x, y, z):
    c, s = math.cos(0.35), math.sin(0.35)
    return np.array([FARM[0] + x * c - z * s, y, FARM[2] + x * s + z * c])


WINDOWS = []
for wx in (-330, -110, 110, 330):
    for wy in (150, 380):
        if wx == 110 and wy == 150:
            continue
        WINDOWS.append(np.array([_house_pt(wx - 55, wy - 60, 361), _house_pt(wx + 55, wy - 60, 361),
                                 _house_pt(wx + 55, wy + 60, 361), _house_pt(wx - 55, wy + 60, 361)]))
DOOR = np.array([_house_pt(60, 0, 361), _house_pt(160, 0, 361), _house_pt(160, 210, 361), _house_pt(60, 210, 361)])
PORCH_LIGHT = _house_pt(220, 240, 380)

# ------------------------------------------------------------------ fireflies + chorus crickets
FF = []
for k in range(150):
    if k < 14:   # a few close to the hero
        r = rng.uniform(14, 70); a = rng.uniform(0, 2 * math.pi)
        FF.append((r * math.sin(a), rng.uniform(25, 75), -r * math.cos(a) - 10, rng.uniform(4, 12), rng.uniform(0, 6.28), rng.uniform(0.3, 0.9)))
    else:
        r = rng.uniform(150, 2800); a = rng.uniform(0, 2 * math.pi)
        x, z = r * math.sin(a), -r * math.cos(a)
        FF.append((x, ground_h(x, z) + rng.uniform(25, 220), z, rng.uniform(40, 160), rng.uniform(0, 6.28), rng.uniform(0.3, 0.9)))
FF = np.array(FF)

CHORUS = []
for k in range(42):
    r = 150 + 2600 * rng.uniform() ** 1.3
    a = rng.uniform(-math.pi * 0.85, math.pi * 0.85)
    x, z = r * math.sin(a), -r * math.cos(a)
    CHORUS.append((x, ground_h(x, z) + rng.uniform(25, 45), z, rng.uniform(-0.45, 0.45)))
CHORUS = np.array(CHORUS)

# ------------------------------------------------------------------ sky
STARS = []
for k in range(1300):
    if k < 800:
        az, el = rng.uniform(-180, 180), math.degrees(math.asin(rng.uniform(0.04, 1)))
    else:
        s = rng.uniform(-1, 1)
        az = 20 + 120 * s + rng.normal(0, 7)
        el = 18 + 60 * (1 - s * s) + rng.normal(0, 5)
    STARS.append((az, el, rng.uniform(0.3, 1.0) ** 1.5, rng.uniform(0, 6.28), k >= 800))
CLOUDS = []
for k in range(6):
    az0, el0, w = rng.uniform(-70, 80), rng.uniform(5, 13), rng.uniform(10, 22)
    blobs = []
    for b in range(3):
        cx, ce = az0 + rng.uniform(-w / 2, w / 2), el0 + rng.uniform(-0.8, 1.2)
        bw, bh = rng.uniform(4, 9), rng.uniform(0.7, 1.6)
        blobs.append([(cx + bw * math.cos(a), ce + bh * math.sin(a)) for a in np.linspace(0, 2 * math.pi, 8)[:-1]])
    CLOUDS.append((blobs, rng.uniform(0.1, 0.3)))
