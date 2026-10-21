"""Late-Cretaceous floodplain at sunset: ground patches, a winding river with sand banks, ferns,
horsetails, cycads, tree ferns, araucaria conifers, rocks, a fallen log, far hills, a volcano with a
glowing crater and smoke, clouds. Small life (dragonflies, fireflies, pterosaurs, fish rings) is
animated in render.py from the data here.

World: metres, y up. The herd stands around x 0..9, z 0..7.3; cameras look from +z toward -z.
The sun is low behind the herd (slightly left), so shadows run toward the camera."""
import math
import numpy as np
from reelkit.geometry import Obj, box, prism, icosa, new_group, face
from reelkit.config import dirv

rng = np.random.default_rng(75)
SUN = dirv(-31.0, 8.0)

# Layout frame: the main cameras look along -VIEW (toward the sun). "depth" grows toward the camera,
# "lat" grows to screen-right. The herd recedes from front-right (small) to back-left (the big adult).
VIEW = np.array([0.5, 0.866])          # (x, z) toward the camera (az 150)
RIGHT = np.array([0.866, -0.5])        # (x, z) screen-right


def wpos(depth, lat):
    p = depth * VIEW + lat * RIGHT
    return float(p[0]), float(p[1])


def dl(x, z):
    """world (x, z) -> (depth, lat)"""
    return 0.5 * x + 0.866 * z, 0.866 * x - 0.5 * z


# ------------------------------------------------------------------ river (crosses behind the herd)
def river_depth(lat):
    return -21.0 + 3.0 * np.sin(np.asarray(lat) * 0.075 + 0.4) + 1.4 * np.sin(np.asarray(lat) * 0.19 + 1.3)


RIVER_HW = 3.4
SAND_W = 2.2


def blocked(x, z, pad=0.0):
    dp, lt = dl(x, z)
    if -6 - pad <= dp <= 9.5 + pad and -9 - pad <= lt <= 10 + pad:      # herd + walk-in paths
        return True
    if 9.5 <= dp <= 40 and -3.5 <= lt <= 9.5:                           # camera lane
        return True
    if abs(dp - float(river_depth(lt))) < RIVER_HW + 0.6:
        return True
    return False


# ------------------------------------------------------------------ ground albedo patches
GRASS = [np.array(c, float) for c in [(100, 130, 62), (112, 138, 66), (92, 120, 60), (122, 142, 72), (86, 112, 58)]]
SAND = np.array([214, 178, 128], float)
MUD = np.array([150, 120, 88], float)
GROUND = []      # (Q (3,3), colour, is_water)
_G = 2.6
for i, gx in enumerate(np.arange(-60, 66, _G)):
    for j, gz in enumerate(np.arange(-76, 48, _G)):
        jx = lambda a, b: (a + rng.uniform(-0.45, 0.45) * _G * 0.4, b + rng.uniform(-0.45, 0.45) * _G * 0.4)
        p00, p10, p11, p01 = (gx, gz), (gx + _G, gz), (gx + _G, gz + _G), (gx, gz + _G)
        for tri in ((p00, p10, p11), (p00, p11, p01)):
            cx = sum(p[0] for p in tri) / 3
            cz = sum(p[1] for p in tri) / 3
            dp_, lt_ = dl(cx, cz)
            dz = abs(dp_ - float(river_depth(lt_)))
            if dz < RIVER_HW:
                col, wat = np.array([60, 90, 130], float), True
            elif dz < RIVER_HW + 0.7:
                col, wat = MUD * rng.uniform(0.92, 1.05), False
            elif dz < RIVER_HW + SAND_W:
                col, wat = SAND * rng.uniform(0.92, 1.05), False
            else:
                n = 0.5 + 0.5 * math.sin(cx * 0.13 + cz * 0.07) * math.sin(cz * 0.11 - cx * 0.05)
                col = GRASS[int(rng.integers(0, len(GRASS)))] * (0.9 + 0.2 * n) * rng.uniform(0.95, 1.04)
                wat = False
            GROUND.append((np.array([[tri[0][0], 0, tri[0][1]], [tri[1][0], 0, tri[1][1]], [tri[2][0], 0, tri[2][1]]]), col, wat))
FAR_GROUND = (100, 112, 66)

# ------------------------------------------------------------------ static objects (cast shadows)
OBJS = []
FEET = []          # contact AO: (x, z, r, strength)


def araucaria(x, z, h, s=1.0, col=(54, 92, 58)):
    """monkey-puzzle style conifer: tall bare trunk + tiers of flattened crowns at the top"""
    g = new_group()
    trunk = (96, 70, 52)
    OBJS.append(Obj(prism(x, z, 0.28 * s, 0.0, h * 0.86, 6, r1=0.16 * s), trunk, 'trunk', grp=g))
    tiers = 4
    for k in range(tiers):
        y = h * (0.6 + 0.1 * k)
        r = (2.6 - 0.5 * k) * s
        OBJS.append(Obj(prism(x, z, r, y, y + 0.9 * s, 7, r1=r * 0.55, rot=k * 0.4), np.array(col) * (0.9 + 0.05 * k), 'crown', grp=g))
    OBJS.append(Obj(icosa([x, h * 1.0, z], 0.9 * s, (1, 0.8, 1), rng=rng), np.array(col) * 1.1, 'crown', grp=g))
    FEET.append((x, z, 0.3 * s, 0.35))


def cycad(x, z, h, s=1.0):
    """squat palm-like cycad: barrel trunk + a star of stiff leaves"""
    g = new_group()
    OBJS.append(Obj(prism(x, z, 0.32 * s, 0.0, h, 7, r1=0.26 * s), (124, 96, 66), 'trunk', grp=g))
    faces = []
    for k in range(9):
        a = 2 * math.pi * k / 9 + rng.uniform(-0.1, 0.1)
        L = rng.uniform(1.3, 1.7) * s
        tip = np.array([x + L * math.cos(a), h + 0.35 * s, z + L * math.sin(a)])
        mid = np.array([x + 0.55 * L * math.cos(a), h + 0.55 * s, z + 0.55 * L * math.sin(a)])
        side = np.array([-math.sin(a), 0, math.cos(a)]) * 0.18 * s
        base = np.array([x, h, z])
        faces.append(face([base, mid + side, tip, mid - side], up=True))
    OBJS.append(Obj(faces, (86, 132, 66), 'leaf', grp=g, double=True))
    FEET.append((x, z, 0.35 * s, 0.35))


def tree_fern(x, z, h, s=1.0):
    g = new_group()
    OBJS.append(Obj(prism(x, z, 0.2 * s, 0.0, h, 6, r1=0.16 * s), (92, 70, 54), 'trunk', grp=g))
    faces = []
    for k in range(8):
        a = 2 * math.pi * k / 8 + 0.2
        L = 2.1 * s
        base = np.array([x, h, z])
        p1 = base + np.array([0.5 * L * math.cos(a), 0.35 * s, 0.5 * L * math.sin(a)])
        p2 = base + np.array([L * math.cos(a), -0.35 * s, L * math.sin(a)])
        side = np.array([-math.sin(a), 0, math.cos(a)]) * 0.32 * s
        faces.append(face([base, p1 + side, p2, p1 - side], up=True))
    OBJS.append(Obj(faces, (74, 128, 62), 'leaf', grp=g, double=True))
    FEET.append((x, z, 0.25 * s, 0.3))


def rock(x, z, r, sq=(1, 0.6, 1)):
    OBJS.append(Obj(icosa([x, r * sq[1] * 0.55, z], r, sq, rng=rng), (150, 136, 124), 'rock'))
    FEET.append((x, z, r * 0.8, 0.45))


# framing trees: well left/right of the sun line so their long shadows streak past the herd, not over it
for (dp, lt, h, s) in [(-30, -16, 19, 1.1), (-40, -24, 22, 1.2), (-27, -30, 16, 0.9), (-33, 17, 18, 1.0), (-45, 25, 23, 1.2),
                       (-26, 28, 15, 0.9), (-52, -12, 17, 1.0), (-55, 14, 18, 1.0), (-14, -22, 14, 0.9), (-12, 21, 13, 0.9)]:
    araucaria(*wpos(dp, lt), h, s)
for (dp, lt, h, s) in [(-9, -9, 1.1, 1.0), (-7, 12, 0.9, 0.9), (4, -12, 0.8, 0.8), (12, 13, 1.0, 1.0), (-13, 6, 0.7, 0.7)]:
    cycad(*wpos(dp, lt), h, s)
for (dp, lt, h, s) in [(-12, -13, 3.4, 1.0), (-11, 15, 3.8, 1.1), (1, -15, 2.8, 0.9), (6, 16, 3.2, 1.0)]:
    tree_fern(*wpos(dp, lt), h, s)
for (dp, lt, r) in [(-8, -4, 0.9), (-3, 12.5, 0.7), (14, -5, 0.5), (16, 12, 0.8), (-12, 1, 0.6), (8, -13, 0.7)]:
    rock(*wpos(dp, lt), r)
# fallen log by the bank
g = new_group()
lx, lz = wpos(-15.5, 7.0)
OBJS.append(Obj(box(lx, 0.35, lz, 3.2, 0.32, 0.32, rot=0.5), (122, 92, 68), 'log', grp=g))
FEET.append((lx, lz, 2.0, 0.3))

# ------------------------------------------------------------------ ferns (animated sway)
class Fern:
    def __init__(self, x, z, s, col):
        self.x, self.z, self.s = x, z, s
        self.col = np.array(col, float)
        self.grp = new_group()
        n = int(rng.integers(6, 10))
        self.quads = []        # list of (4,3) base points
        self.w = []            # sway weight per point
        for k in range(n):
            a = 2 * math.pi * k / n + rng.uniform(-0.25, 0.25)
            L = s * rng.uniform(0.8, 1.15)
            lift = rng.uniform(0.55, 0.9)
            pts = []
            for u in np.linspace(0, 1, 4):
                r = L * u
                y = s * lift * math.sin(math.pi * 0.85 * u) + 0.03
                pts.append(np.array([x + r * math.cos(a), y, z + r * math.sin(a)]))
            side = np.array([-math.sin(a), 0, math.cos(a)])
            widths = [0.02, 0.16, 0.13, 0.02]
            for i in range(3):
                q = np.array([pts[i] + side * widths[i] * s, pts[i + 1] + side * widths[i + 1] * s,
                              pts[i + 1] - side * widths[i + 1] * s, pts[i] - side * widths[i] * s])
                self.quads.append(q)
                self.w.append(np.array([i, i + 1, i + 1, i], float) / 3.0)
        self.quads = np.array(self.quads)
        self.w = np.array(self.w)
        self.ph = rng.uniform(0, 6.283)
        V = self.quads.reshape(-1, 3)
        self.lo, self.hi = V.min(0), V.max(0)
        self.ctr = V.mean(0)
        self.tone = 1 + rng.uniform(-0.06, 0.06, len(self.quads))

    def at(self, t):
        sway = 0.06 * self.s * (math.sin(t * 1.3 + self.ph) + 0.4 * math.sin(t * 2.9 + self.ph * 1.7))
        off = np.array([sway, 0, 0.5 * sway])
        return self.quads + self.w[..., None] * off[None, None]


FERNS = []
FERN_COLS = [(84, 140, 64), (98, 150, 70), (72, 126, 60), (110, 152, 74)]
# clumps that hide the walk-in paths (left) and walk-off (right), plus scattered ferns
# clumps (depth, lat, n): left ones hide the walk-in paths, right ones the walk-off, front ones frame wide shots
CLUMPS = [(-6.5, -10.5, 7), (-3.0, -12.0, 6), (0.5, -11.0, 5), (-9.0, -6.5, 5), (13.5, -5.5, 4), (15.5, 11.5, 5),
          (11.0, 12.5, 5), (-4.0, 12.0, 5), (-10.0, 9.0, 4), (19.0, -7.0, 4), (21.0, 13.0, 4), (-14.0, -2.0, 4)]
for dp0, lt0, n in CLUMPS:
    for _ in range(n):
        x, z = wpos(dp0 + rng.normal(0, 1.2), lt0 + rng.normal(0, 1.3))
        if blocked(x, z, 0.3):
            continue
        FERNS.append(Fern(x, z, rng.uniform(0.9, 1.7), FERN_COLS[int(rng.integers(0, 4))]))
for _ in range(80):
    x, z = wpos(rng.uniform(-17, 30), rng.uniform(-28, 30))
    if blocked(x, z, 0.8):
        continue
    FERNS.append(Fern(x, z, rng.uniform(0.6, 1.3), FERN_COLS[int(rng.integers(0, 4))]))
# beyond the river
for _ in range(45):
    dp, lt = rng.uniform(-50, -26), rng.uniform(-50, 50)
    if abs(dp - float(river_depth(lt))) < RIVER_HW + 2.5:
        continue
    FERNS.append(Fern(*wpos(dp, lt), rng.uniform(0.8, 1.6), FERN_COLS[int(rng.integers(0, 4))]))
for f in FERNS:
    FEET.append((f.x, f.z, 0.25 * f.s, 0.25))

# horsetail clusters (static thin segmented stems)
for (dp, lt) in [(-6.0, -5.0), (-5.0, 11.0), (-17.0, 3.0), (-17.5, -6.0), (-16.5, 12.0), (2.0, 11.5)]:
    cx, cz = wpos(dp, lt)
    g = new_group()
    faces = []
    for _ in range(9):
        x, z = cx + rng.normal(0, 0.45), cz + rng.normal(0, 0.45)
        h = rng.uniform(0.9, 1.8)
        faces += prism(x, z, 0.035, 0.0, h, 4, r1=0.02)
    OBJS.append(Obj(faces, (110, 150, 80), 'horsetail', grp=g))

# magnolia-like flowering shrubs (early flowering plants)
FLOWERS = []
for (dp, lt, s) in [(-2.0, -14.5, 1.0), (9.0, 14.0, 0.9), (-14.0, -9.0, 1.1), (-9.0, 17.0, 1.0)]:
    x, z = wpos(dp, lt)
    g = new_group()
    OBJS.append(Obj(icosa([x, 0.9 * s, z], 1.0 * s, (1.2, 0.85, 1.1), rng=rng), (70, 116, 62), 'bush', grp=g))
    for _ in range(7):
        a, e = rng.uniform(0, 6.283), rng.uniform(0.1, 1.2)
        FLOWERS.append(np.array([x + 1.05 * s * math.cos(a) * math.cos(e), 0.9 * s + 0.8 * s * math.sin(e), z + 1.0 * s * math.sin(a) * math.cos(e)]))
    FEET.append((x, z, 0.9 * s, 0.4))
FLOWERS = np.array(FLOWERS)

# ------------------------------------------------------------------ far layer
FAR = []           # (faces, colour, emit, name) - drawn after the ground, fogged, never shadowed


def far_add(faces, col, emit=0.0, name=''):
    FAR.append((faces, np.array(col, float), emit, name))


def ridge_strip(z, base, amp, x0, x1, n, seed, col):
    r = np.random.default_rng(seed)
    xs = np.linspace(x0, x1, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(xs * 0.012 + r.uniform(0, 6))) + r.uniform(-amp * 0.3, amp * 0.3, n)
    faces = []
    for i in range(n - 1):
        Q = np.array([[xs[i], ys[i], z], [xs[i + 1], ys[i + 1], z], [xs[i + 1], 0, z + 8], [xs[i], 0, z + 8]])
        faces.append((Q, np.array([0, 0.5, 0.86])))
    far_add(faces, col, 0.0, 'ridge')
    return xs, ys


# volcano (to the right of the sun), glowing crater, lava streaks
_vd = dirv(-19.0, 0) * 560.0
VOLC = np.array([_vd[0], 0.0, _vd[2]])
VOLC_H = 74.0
VOLC_R = 62.0
vf = prism(VOLC[0], VOLC[2], VOLC_R, 0.0, VOLC_H, 10, r1=11.0, rot=0.2)
far_add(vf[:-1], (122, 84, 104), 0.0, 'volcano')
far_add([vf[-1]], (255, 120, 60), 0.9, 'crater')
LAVA = []
for a in (2.0, 2.35, 2.9):
    pts = []
    for u in np.linspace(0, 1, 6):
        r = 11 + (VOLC_R - 11) * u * 0.62
        y = VOLC_H * (1 - u * 0.62) + 0.5
        jit = rng.uniform(-0.03, 0.03)
        pts.append(np.array([VOLC[0] + r * math.cos(a + jit), y, VOLC[2] + r * math.sin(a + jit)]))
    for i in range(5):
        p, q = pts[i], pts[i + 1]
        side = np.array([-(q - p)[2], 0, (q - p)[0]])
        side = side / (np.linalg.norm(side) + 1e-9) * 1.1 * (1 - i * 0.12)
        LAVA.append(np.array([p + side, q + side * 0.8, q - side * 0.8, p - side]))
CRATER = VOLC + np.array([0, VOLC_H + 2, 0])
# far hills and forest silhouettes
def ridge_strip_dl(dp, base, amp, n, seed, col, span=420):
    r = np.random.default_rng(seed)
    lts = np.linspace(-span, span, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(lts * 0.012 + r.uniform(0, 6))) + r.uniform(-amp * 0.3, amp * 0.3, n)
    faces = []
    for i in range(n - 1):
        a, b = wpos(dp, lts[i]), wpos(dp, lts[i + 1])
        a2, b2 = wpos(dp + 8, lts[i]), wpos(dp + 8, lts[i + 1])
        Q = np.array([[a[0], ys[i], a[1]], [b[0], ys[i + 1], b[1]], [b2[0], 0, b2[1]], [a2[0], 0, a2[1]]])
        nn = np.array([VIEW[0] * 0.86, 0.5, VIEW[1] * 0.86])
        faces.append((Q, nn / np.linalg.norm(nn)))
    far_add(faces, col, 0.0, 'ridge')


ridge_strip_dl(-330, 10, 26, 34, 3, (120, 96, 140))
ridge_strip_dl(-210, 6, 14, 30, 5, (104, 104, 120), 300)
FOREST = []
while len(FOREST) < 60:
    dp, lt = rng.uniform(-150, -60), rng.uniform(-160, 160)
    sun_lat = -0.017 * (22 - dp) + 3       # keep a clearing where the sun sets, as seen from the main cameras
    if sun_lat - 22 < lt < sun_lat + 16:
        continue
    x, z = wpos(dp, lt)
    FOREST.append((x, z, rng.uniform(12, 24)))
for (x, z, h) in FOREST:
    faces = prism(x, z, 0.4, 0, h * 0.7, 4) + prism(x, z, 3.2, h * 0.58, h * 0.78, 6, r1=1.8) + prism(x, z, 2.3, h * 0.78, h, 6, r1=0.6)
    far_add(faces, (58, 86, 70), 0.0, 'forest')
FAR.sort(key=lambda it: -np.mean([abs(Q[:, 2].mean()) for Q, n in it[0]]))

# clouds: long low-poly strips far away, lit from below by the sun
CLOUDS = []
for (az, el, w, h) in [(-50, 9.0, 90, 10), (-22, 14.0, 130, 12), (-34, 21.0, 160, 14), (-5, 6.5, 70, 7), (-65, 16, 110, 10), (10, 18, 120, 11)]:
    d = dirv(az, el)
    c = d * 900.0
    right = np.array([math.cos(math.radians(az)), 0, math.sin(math.radians(az))])
    pts = []
    n = 7
    for k in range(n):
        u = k / (n - 1)
        x = (u - 0.5) * 2 * w
        top = h * (0.6 + 0.4 * math.sin(math.pi * u)) * rng.uniform(0.8, 1.2)
        pts.append((x, top))
    poly_top = [c + right * x + np.array([0, y, 0]) for x, y in pts]
    poly_bot = [c + right * x + np.array([0, -h * 0.35 * rng.uniform(0.6, 1.0), 0]) for x, _ in pts[::-1]]
    CLOUDS.append(np.array(poly_top + poly_bot))

# small life data
def wp3(dp, lt, y):
    x, z = wpos(dp, lt)
    return np.array([x, y, z])


DRAGONFLIES = [dict(c=wp3(10, -3, 1.4), r=(1.6, 0.4, 1.2), w=(0.9, 1.7, 1.3), col=(60, 200, 220)),
               dict(c=wp3(9, 11, 1.2), r=(1.4, 0.35, 1.1), w=(1.1, 1.9, 0.8), col=(90, 120, 255)),
               dict(c=wp3(-8, 4, 1.8), r=(2.0, 0.5, 1.0), w=(0.7, 1.4, 1.1), col=(250, 120, 90)),
               dict(c=wp3(3.0, 2.5, 4.4), r=(1.0, 0.4, 0.8), w=(1.0, 2.1, 1.4), col=(80, 230, 160))]
FIREFLIES = np.array([wp3(rng.uniform(-14, 22), rng.uniform(-18, 20), rng.uniform(0.3, 1.4)) for _ in range(30)])
FIREFLIES = np.array([p for p in FIREFLIES if not blocked(p[0], p[2], 0.6)])
FF_PH = rng.uniform(0, 6.283, len(FIREFLIES))
PTEROS = [dict(az0=-60, az1=0, el=16, t0=-5, t1=40, size=2.2), dict(az0=10, az1=-45, el=12, t0=18, t1=64, size=1.6),
          dict(az0=-40, az1=15, el=24, t0=38, t1=75, size=2.6)]
FISH = []
for lt, ft in zip(rng.uniform(-10, 16, 14), np.sort(rng.uniform(1, 59, 14))):
    FISH.append((*wpos(float(river_depth(lt)), lt), ft))
