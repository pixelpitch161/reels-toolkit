"""Diorama geometry (stylised units). Side-view cutaway of a summer meadow:
ground top at y=0, front cut face at z=+30 (camera looks toward -z)."""
import math
import numpy as np

rng = np.random.default_rng(7)
FRONT_Z = 30.0


def nrm(v):
    return v / (np.linalg.norm(v) + 1e-12)


def face(Q, inside=None, up=False):
    Q = np.array(Q, float)
    n = nrm(np.cross(Q[1] - Q[0], Q[2] - Q[0]))
    if inside is not None and np.dot(n, Q.mean(0) - inside) < 0:
        n = -n
    if up and n[1] < 0:
        n = -n
    return (Q, n)


def box(cx, cy, cz, hx, hy, hz, rot=0.0):
    c, s = math.cos(rot), math.sin(rot)
    P = lambda x, y, z: np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    V = [P(sx * hx, sy * hy, sz * hz) for sy in (-1, 1) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    quads = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7), (0, 3, 2, 1)]
    ctr = np.array([cx, cy, cz])
    return [face([V[i] for i in q], ctr) for q in quads]


def prism(cx, cz, r, y0, y1, n=8, r1=None, rot=0.0):
    r1 = r if r1 is None else r1
    out = []
    for k in range(n):
        a0, a1 = rot + 2 * math.pi * k / n, rot + 2 * math.pi * (k + 1) / n
        Q = [[cx + r * math.cos(a0), y0, cz + r * math.sin(a0)], [cx + r * math.cos(a1), y0, cz + r * math.sin(a1)],
             [cx + r1 * math.cos(a1), y1, cz + r1 * math.sin(a1)], [cx + r1 * math.cos(a0), y1, cz + r1 * math.sin(a0)]]
        out.append(face(Q, np.array([cx, (y0 + y1) / 2, cz])))
    out.append(face([[cx + r1 * math.cos(rot + 2 * math.pi * k / n), y1, cz + r1 * math.sin(rot + 2 * math.pi * k / n)] for k in range(n)], up=True))
    return out


ICO_V = None


def icosa(c, r, sq=(1, 1, 1), jit=0.1):
    t = (1 + 5 ** 0.5) / 2
    V = np.array([[-1, t, 0], [1, t, 0], [-1, -t, 0], [1, -t, 0], [0, -1, t], [0, 1, t], [0, -1, -t], [0, 1, -t],
                  [t, 0, -1], [t, 0, 1], [-t, 0, -1], [-t, 0, 1]], float)
    V /= np.linalg.norm(V[0])
    V = V * r * rng.uniform(1 - jit, 1 + jit, (12, 1)) * np.array(sq)
    F = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4), (11, 10, 2), (10, 7, 6), (7, 1, 8),
         (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8), (3, 8, 9), (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
    return [face(V[list(f)] + c, np.array(c, float)) for f in F]


def gable(cx, cy, cz, hx, hz, hr, rot=0.0, over=12):
    c, s = math.cos(rot), math.sin(rot)
    P = lambda x, y, z: np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    a, b = hx + over, hz + over
    r0, r1 = P(-a, hr, 0), P(a, hr, 0)
    e = [P(-a, 0, b), P(a, 0, b), P(a, 0, -b), P(-a, 0, -b)]
    ctr = np.array([cx, cy - 10, cz])
    return [face([e[0], e[1], r1, r0], ctr), face([e[2], e[3], r0, r1], ctr), face([e[1], e[2], r1], ctr), face([e[3], e[0], r0], ctr)]


# ===================================================================== cricket
XS = [-0.98, -0.82, -0.55, -0.25, 0.04, 0.2, 0.42, 0.55, 0.68, 0.76]
RS = [0.04, 0.12, 0.185, 0.19, 0.165, 0.17, 0.15, 0.14, 0.11, 0.04]
YS = [0.03, 0.01, 0.0, 0.0, 0.02, 0.04, 0.045, 0.05, 0.04, 0.02]
SEGCOL = [(206, 246, 120), (206, 246, 120), (200, 244, 128), (194, 242, 134), (184, 240, 146),
          (174, 238, 154), (168, 234, 158), (164, 230, 160), (158, 226, 160)]


def _body():
    rings = []
    for x, r, y in zip(XS, RS, YS):
        rings.append([np.array([x, y + r * math.cos(a), 0.86 * r * math.sin(a)]) for a in np.linspace(0, 2 * math.pi, 9)[:-1]])
    out = []
    for i in range(len(rings) - 1):
        for k in range(8):
            k2 = (k + 1) % 8
            Q = np.array([rings[i][k], rings[i][k2], rings[i + 1][k2], rings[i + 1][k]])
            c = Q.mean(0)
            n = nrm(np.cross(Q[1] - Q[0], Q[3] - Q[0]))
            if np.dot(n, c - np.array([c[0], YS[i], 0])) < 0:
                n = -n
            col = np.array(SEGCOL[i], float) * (np.array([0.9, 0.95, 0.88]) if k in (0, 1, 7) else 1)
            out.append((Q, n, col))
    return out


BODY = _body()
LEGS = [((0.5, -0.06, 0.09), (0.72, -0.2, 0.3), (0.95, -0.42, 0.36), 0.05, 0.03),
        ((0.25, -0.08, 0.11), (0.2, -0.24, 0.4), (0.05, -0.42, 0.53), 0.05, 0.03),
        ((0.02, -0.06, 0.13), (-0.44, 0.22, 0.25), (-1.06, -0.42, 0.4), 0.1, 0.035)]
WING = [(0.23, 0.0), (0.29, 0.42), (0.22, 0.84), (0.05, 1.03), (-0.16, 1.0), (-0.3, 0.78), (-0.27, 0.36), (-0.2, 0.0)]
WING_BASE = np.array([0.07, 0.17, 0.0])


class Cricket:
    def __init__(self, origin, scale, facing=1, yaw=0.0):
        self.o = np.array(origin, float)
        self.s = scale
        ang = yaw + (0 if facing > 0 else math.pi)
        c, s = math.cos(ang), math.sin(ang)
        self.R = np.array([[c, 0, -s], [0, 1, 0], [s, 0, c]])

    def W(self, p):
        return self.o + self.s * (self.R @ np.asarray(p, float))

    def build(self, t, env, ph=0.0):
        out = {'faces': [], 'legs': [], 'ant': [], 'wings': [], 'eyes': []}
        bob = np.array([0, 0.004 * math.sin(t * 3.1 + ph), 0])
        for Q, n, col in BODY:
            out['faces'].append((np.array([self.W(q + bob) for q in Q]), self.R @ n, col))
        for (h, k, f, w1, w2) in LEGS:
            for sg in (1, -1):
                H = self.W(np.array([h[0], h[1], h[2] * sg]) + bob)
                Kp = self.W([k[0], k[1], k[2] * sg]); F = self.W([f[0], f[1], f[2] * sg])
                out['legs'].append((H, Kp, w1 * self.s, sg)); out['legs'].append((Kp, F, w2 * self.s, sg))
        for sg in (1, -1):
            pts = []
            for j in range(12):
                u = j / 11
                wig = 0.12 * math.sin(t * 1.7 + u * 3 + sg + ph) * u * u
                pts.append(self.W([0.74 + 2.6 * u, 0.07 + 1.1 * u - 0.95 * u * u + wig, sg * (0.05 + 1.0 * u * u)]))
            out['ant'].append(pts)
        for sg in (1, -1):
            oa = math.radians(4 + 10 * env * (0.5 + 0.5 * math.sin(t * 2 * math.pi * 46 + (0 if sg > 0 else math.pi))))
            tilt = math.radians(-14)
            pts = []
            for (u, v) in WING:
                x = u * math.cos(tilt) - v * math.sin(tilt)
                y = u * math.sin(tilt) + v * math.cos(tilt)
                z = sg * (0.015 + (x + 0.3) * math.sin(oa) * 0.5)
                pts.append(self.W(WING_BASE + np.array([x, y, z]) + bob))
            out['wings'].append(np.array(pts))
        for sg in (1, -1):
            c = np.array([0.62, 0.1, 0.105 * sg]) + bob
            out['eyes'].append(np.array([self.W(c + d) for d in ([0.045, 0, 0], [0, 0.045, 0], [-0.045, 0, 0], [0, -0.045, 0])]))
        out['ring_at'] = self.W([0.0, 1.05, 0.0])
        return out


# ===================================================================== layout
HERO = Cricket((-50.0, 116.0, 14.0), 56.0, facing=1)            # on the big leaf, facing the thermometer
STEM_X, STEM_Z = 4.0, 12.0
LEAF_Y = HERO.o[1] - 0.46 * HERO.s


def hero_leaf():
    out = []
    xs = np.linspace(STEM_X, -118.0, 11)
    L = xs[0] - xs[-1]
    for i in range(len(xs) - 1):
        xa, xb = xs[i], xs[i + 1]
        wa = 15 * max(0.0, math.sin(math.pi * (xs[0] - xa) / L)) ** 0.7
        wb = 15 * max(0.0, math.sin(math.pi * (xs[0] - xb) / L)) ** 0.7
        ya = LEAF_Y - 0.0016 * (xa + 46) ** 2 + 2
        yb = LEAF_Y - 0.0016 * (xb + 46) ** 2 + 2
        for sg in (1, -1):
            Q = [[xa, ya, STEM_Z], [xb, yb, STEM_Z], [xb, yb + 3.5 * wb / 15, STEM_Z + sg * wb], [xa, ya + 3.5 * wa / 15, STEM_Z + sg * wa]]
            out.append((face(Q, up=True), i, sg))
    return out


LEAF = hero_leaf()
STEM = [np.array([STEM_X - 3, 0, STEM_Z]), np.array([STEM_X - 1, 60, STEM_Z]), np.array([STEM_X, LEAF_Y, STEM_Z]),
        np.array([STEM_X + 3, 160, STEM_Z - 2]), np.array([STEM_X + 5, 205, STEM_Z - 3])]
SMALL_LEAVES = [(np.array([STEM_X, 60, STEM_Z]), 1, 34), (np.array([STEM_X + 2, 150, STEM_Z - 2]), -1, 28)]
UMBEL = np.array([STEM_X + 5, 210, STEM_Z - 3])

# thermometer on its own post
POST_X, POST_Z = 52.0, -6.0
BOARD = dict(x0=33.0, x1=71.0, y0=26.0, y1=200.0, z=POST_Z + 5.2)
TH_T0, TH_T1 = 40.0, 100.0


def thermo_y(T):
    return BOARD['y0'] + 22 + (T - TH_T0) / (TH_T1 - TH_T0) * (BOARD['y1'] - BOARD['y0'] - 38)


SOLIDS = []   # (name, faces, colour, rim)
SOLIDS.append(('post', box(POST_X, 108, POST_Z, 5, 108, 5), (150, 104, 84), 0.6))
SOLIDS.append(('postcap', box(POST_X, 219, POST_Z, 7, 3, 7), (120, 84, 70), 0.6))
SOLIDS.append(('hook', box(POST_X + 12, 210, POST_Z, 9, 1.2, 1.2), (60, 50, 60), 0.3))
LANTERN = np.array([POST_X + 20, 196, POST_Z])
SOLIDS.append(('lantern', box(LANTERN[0], LANTERN[1], LANTERN[2], 5, 7, 5), (60, 46, 50), 0.5))
SOLIDS.append(('lanterncap', prism(LANTERN[0], LANTERN[2], 6.5, LANTERN[1] + 7, LANTERN[1] + 12, 4, 1.0, rot=0.785), (60, 46, 50), 0.5))

# back fence + owl post
FENCE_Z = -112.0
FENCE_POSTS = [-176.0, -96.0, -16.0, 64.0, 144.0]
for px in FENCE_POSTS:
    SOLIDS.append(('fpost', box(px, 50, FENCE_Z, 4.5, 50, 4.5), (132, 96, 86), 0.7))
for ry in (38.0, 78.0):
    SOLIDS.append(('rail', box(-16, ry, FENCE_Z + 4.5, 190, 2.4, 1.0), (146, 108, 94), 0.7))
OWL_POST = np.array([-96.0, 100.0, FENCE_Z])

# mushrooms (front)
MUSHROOMS = [(-150, 20, 9, 13), (-136, 24, 6, 9), (118, 22, 8, 12), (-20, 26, 5, 8)]
for (mx, mz, r, h) in MUSHROOMS:
    SOLIDS.append(('mstem', prism(mx, mz, r * 0.35, 0, h, 6, r * 0.3), (240, 230, 214), 0.5))
    SOLIDS.append(('mcap', prism(mx, mz, r, h, h + r * 0.45, 7, r * 0.45), (236, 60, 70), 0.6))

# pond (behind the fence, right) + frog
POND = np.array([-58.0, 0.6, -36.0])
POND_POLY = np.array([[POND[0] + 50 * math.cos(a) * rng.uniform(0.92, 1.05), 0.6, POND[2] + 17 * math.sin(a) * rng.uniform(0.92, 1.05)]
                      for a in np.linspace(0, 2 * math.pi, 13)[:-1]])
PADS = [(POND[0] - 26, POND[2] + 5, 10), (POND[0] + 16, POND[2] + 2, 13), (POND[0] + 36, POND[2] + 8, 8)]
FROG = np.array([POND[0] + 16, 2.0, POND[2] + 2])
CATTAILS = [(POND[0] - 48, POND[2] - 8, 62), (POND[0] - 42, POND[2] - 13, 74), (POND[0] + 48, POND[2] - 10, 58)]

# background landscape
FARM = np.array([120.0, 0.0, -6000.0])
BARN = np.array([-720.0, 0.0, -5600.0])
PUMP = np.array([330.0, 0.0, -3600.0])
OAK = np.array([-420.0, 0.0, -2900.0])


def ridge(z, amp, base, n, seed, col):
    r = np.random.default_rng(seed)
    xs = np.linspace(-2600, 2600, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(xs * 0.0021 + r.uniform(0, 6))) + r.uniform(-amp * 0.25, amp * 0.25, n)
    tris = []
    for i in range(n - 1):
        a = np.array([xs[i], ys[i], z]); b = np.array([xs[i + 1], ys[i + 1], z])
        c = np.array([xs[i + 1], -40, z + 120]); d = np.array([xs[i], -40, z + 120])
        tris.append((np.array([a, b, c, d]), nrm(np.array([0, 1.0, 0.35 + r.uniform(-0.25, 0.25)])), col))
    return tris


def ridge2(z, amp, base, n, seed, col, span):
    r = np.random.default_rng(seed)
    xs = np.linspace(-span, span, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(xs * (2.6 / span) + r.uniform(0, 6))) + r.uniform(-amp * 0.25, amp * 0.25, n)
    tris = []
    for i in range(n - 1):
        a = np.array([xs[i], ys[i], z]); b = np.array([xs[i + 1], ys[i + 1], z])
        c = np.array([xs[i + 1], -60, z + 400]); d = np.array([xs[i], -60, z + 400])
        tris.append((np.array([a, b, c, d]), nrm(np.array([0, 1.0, 0.35 + r.uniform(-0.25, 0.25)])), col))
    return tris


RIDGES = [ridge2(-11000, 900, 350, 22, 1, (80, 60, 150), 6000), ridge2(-7800, 520, 160, 26, 2, (60, 90, 160), 4500),
          ridge2(-5400, 260, 40, 30, 3, (50, 120, 150), 3200)]
BACK_SOLIDS = []
BACK_SOLIDS.append(('house', box(FARM[0], 80, FARM[2], 130, 80, 85, 0.2), (240, 224, 204), 0.6))
BACK_SOLIDS.append(('roof', gable(FARM[0], 160, FARM[2], 130, 85, 80, 0.2, over=16), (110, 74, 170), 0.8))
BACK_SOLIDS.append(('chim', box(FARM[0] + 65, 230, FARM[2] - 10, 12, 32, 12, 0.2), (190, 96, 96), 0.5))
BACK_SOLIDS.append(('barn', box(BARN[0], 120, BARN[2], 150, 120, 110, -0.15), (240, 52, 76), 0.6))
BACK_SOLIDS.append(('barnroof', gable(BARN[0], 240, BARN[2], 150, 110, 110, -0.15, over=16), (90, 60, 130), 0.8))
BACK_SOLIDS.append(('oak_trunk', prism(OAK[0], OAK[2], 26, 0, 260, 7, 17), (120, 74, 66), 0.5))
for (dx, dy, dz, r) in [(0, 390, 0, 180), (-130, 315, 40, 130), (134, 330, -20, 138), (26, 500, -12, 134), (-74, 462, 56, 112)]:
    BACK_SOLIDS.append(('oak', icosa(OAK + np.array([dx, dy, dz]), r, (1, 0.82, 1)), (54, 196, 124), 1.2))
for k in range(12):    # tree line
    x = -2400 + k * 420 + rng.uniform(-60, 60)
    z = -4800 + rng.uniform(-250, 250)
    BACK_SOLIDS.append(('tree', icosa(np.array([x, 200 + rng.uniform(-30, 50), z]), rng.uniform(150, 220), (1, 1.3, 1)), (40, 140, 116), 1.0))
HOUSE_WINDOWS = []
c0, s0 = math.cos(0.2), math.sin(0.2)
for wx in (-84, -28, 34, 90):
    for wy in (46, 114):
        if wx == 34 and wy == 46:
            continue
        pts = []
        for (dx, dy) in ((-15, -16), (15, -16), (15, 16), (-15, 16)):
            x, z = wx + dx, 86
            pts.append([FARM[0] + x * c0 - z * s0, wy + dy, FARM[2] + x * s0 + z * c0])
        HOUSE_WINDOWS.append(np.array(pts))

# grass blades (meadow surface)
BLADES = []
for k in range(330):
    if k < 90:      # fringe along the cut edge
        x = rng.uniform(-260, 260); z = rng.uniform(16, 29); h = rng.uniform(10, 26)
    else:
        x = rng.uniform(-260, 260); z = rng.uniform(-250, 14); h = rng.uniform(22, 92) * (1 if z > -150 else 0.8)
    if abs(x - POND[0]) < 56 and abs(z - POND[2]) < 22:
        continue
    BLADES.append((x, z, h, rng.uniform(2.0, 3.6), rng.uniform(0, math.pi), rng.uniform(-0.35, 0.35), rng.uniform(0.8, 1.2), rng.uniform(0, 6.28)))
BLADES = np.array(BLADES)

FLOWERS = []      # (x, y, z, r, kind)  kind: 0 daisy, 1 clover/pink, 2 bluebell, 3 dandelion puff
for k in range(60):
    x = rng.uniform(-200, 200); z = rng.uniform(-240, 26)
    if abs(x - POND[0]) < 56 and abs(z - POND[2]) < 22:
        continue
    FLOWERS.append((x, rng.uniform(14, 58), z, rng.uniform(4.0, 7.0), int(rng.integers(4))))
FLOWERS = np.array(FLOWERS)
PEBBLES = [(rng.uniform(-220, 220), rng.uniform(-200, 26), rng.uniform(2.5, 5.5)) for _ in range(26)]

# chorus crickets on their own stems: (x, z, height)
CHORUS = [(-150, -30, 66), (-70, -160, 82), (8, -64, 56), (100, -20, 54), (-190, -210, 86), (150, -150, 70), (60, -240, 76)]
CHORUS_CR = [Cricket((x, h, z), 28.0 * (1 if z > -100 else 0.9), facing=(1 if k % 2 else -1), yaw=0.25 * (k % 3 - 1))
             for k, (x, z, h) in enumerate(CHORUS)]
CHORUS_OFF = rng.uniform(-0.45, 0.45, len(CHORUS))

# fireflies
FF = np.array([(rng.uniform(-240, 240), rng.uniform(15, 230), rng.uniform(-300, 25), rng.uniform(6, 22), rng.uniform(0, 6.28),
                rng.uniform(0.4, 1.0)) for _ in range(120)])

# soil cross-section (front face at z=FRONT_Z)
LAYERS = [(0, -13, (70, 46, 74)), (-13, -58, (122, 70, 74)), (-58, -118, (176, 92, 68)), (-118, -300, (86, 66, 118))]
STONES = []
for k in range(46):
    x = rng.uniform(-260, 260); y = rng.uniform(-290, -16)
    r = rng.uniform(3, 11) * (1.4 if y < -118 else 1)
    n = int(rng.integers(5, 8))
    STONES.append([(x + r * math.cos(a) * rng.uniform(0.75, 1.15), y + r * 0.7 * math.sin(a) * rng.uniform(0.75, 1.15))
                   for a in np.linspace(0, 2 * math.pi, n + 1)[:-1]])
ROOTS = []
for (rx, depth) in [(STEM_X, 110), (-150, 70), (122, 60), (-62, 50), (190, 66), (-20, 40), (60, 48)]:
    pts = [(rx, -2)]
    x, y = rx, -2
    for j in range(7):
        x += rng.uniform(-9, 9); y -= depth / 7
        pts.append((x, y))
    ROOTS.append(pts)

# sky
STARS = []
for k in range(1100):
    if k < 700:
        az, el = rng.uniform(-40, 40), math.degrees(math.asin(rng.uniform(0.01, 0.6)))
    else:
        s = rng.uniform(-1, 1)
        az = 25 * s + rng.normal(0, 3)
        el = 3 + 22 * (s + 1) / 2 + rng.normal(0, 2.5)
    STARS.append((az, el, rng.uniform(0.3, 1.0) ** 1.4, rng.uniform(0, 6.28), k >= 700))
CLOUDS = []
for k in range(5):
    az0, el0, w = rng.uniform(-30, 30), rng.uniform(2.5, 7.5), rng.uniform(5, 11)
    blobs = []
    for b in range(3):
        cx, ce = az0 + rng.uniform(-w / 2, w / 2), el0 + rng.uniform(-0.4, 0.6)
        bw, bh = rng.uniform(2, 5), rng.uniform(0.4, 0.9)
        blobs.append([(cx + bw * math.cos(a), ce + bh * math.sin(a)) for a in np.linspace(0, 2 * math.pi, 8)[:-1]])
    CLOUDS.append((blobs, rng.uniform(0.05, 0.15)))


# two slender poplars in the mid-ground: silhouettes against the moon glow, and their long
# moon shadows stream across the field and over the meadow slab toward the camera
POPLARS = [np.array([-250.0, 0.0, -1550.0]), np.array([-72.0, 0.0, -1680.0])]
for k, PP in enumerate(POPLARS):
    hk = [1.0, 1.12][k]
    BACK_SOLIDS.append(('ptrunk', prism(PP[0], PP[2], 9, 0, 150 * hk, 6, 6), (70, 50, 52), 0.5))
    for (dy, r, sq) in [(250, 62, 2.0), (175, 50, 1.3), (380, 40, 1.6)]:
        BACK_SOLIDS.append(('poplar', icosa(PP + np.array([0.0, dy * hk, 0.0]), r, (1, sq, 1), jit=0.14), (36, 112, 92), 1.0))
