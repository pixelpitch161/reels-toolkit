"""Static geometry + textures: a golden-hour Illinois woodland edge cut away like an ant farm.

World: y up, metres-ish. The cut plane is z = 0: soil below y = 0 for z < 0 is seen face-on from
the camera (z > 0). Behind the cut the ground surface runs back to the horizon with trees, a
split-rail fence along x = 0 (the divider: left = Brood XIX, right = Brood XIII), a farmhouse with
a porch lamp, a barn + silo and far tree lines. The low sun sits in the gap above the fence.

A lantern hangs from a hook on the front fence post, just in front of the cut, as the warm local
light on the soil face (the face itself faces away from the sun)."""
import math
import numpy as np
from PIL import Image, ImageDraw
from reelkit.geometry import Obj, box, prism, icosa, gable, new_group, face
from reelkit.config import dirv

rng = np.random.default_rng(1803)
SUN = dirv(-1.0, 6.5)
LANTERN = np.array([0.0, 0.84, 0.30])        # hangs just in front of the cut
PORCH = np.array([3.55, 1.25, -16.9])

# ================================================================== nymphs (chamber + tunnel to the surface)
# side, chamber centre (x, y), rest angle (deg from +x, head up the tunnel), size, exit x on the surface
NYMPHS = [
    ('L', (-0.46, -1.06), 78, 1.0, -0.50), ('R', (0.46, -1.06), 102, 1.0, 0.50),
    ('L', (-1.32, -1.52), 70, 0.86, -1.20), ('R', (1.30, -1.46), 108, 0.86, 1.18),
    ('L', (-2.12, -0.90), 96, 0.84, -2.02), ('R', (2.08, -0.94), 84, 0.84, 1.96),
    ('L', (-2.86, -1.62), 74, 0.82, -2.64), ('R', (2.84, -1.66), 104, 0.82, 2.66),
    ('L', (-1.70, -2.12), 88, 0.8, -1.62), ('R', (1.72, -2.10), 92, 0.8, 1.64),
]


def _tunnel_path(cx, cy, ang, ex, seed):
    """chamber -> surface polyline: leaves along the nymph's heading, wiggles, ends at (ex, 0)"""
    r = np.random.default_rng(seed)
    a = math.radians(ang)
    p0 = np.array([cx, cy])
    p1 = p0 + 0.22 * np.array([math.cos(a), math.sin(a)])
    p3 = np.array([ex, 0.02])
    p2 = np.array([ex + r.uniform(-0.12, 0.12), cy * 0.4])
    pts = []
    for u in np.linspace(0, 1, 60):
        b = (1 - u) ** 3 * p0 + 3 * (1 - u) ** 2 * u * p1 + 3 * (1 - u) * u * u * p2 + u ** 3 * p3
        b[0] += 0.025 * math.sin(u * 17 + seed) * math.sin(u * math.pi)
        pts.append(b)
    return np.array(pts)


TUNNELS = [_tunnel_path(c[0], c[1], ang, ex, 11 + k) for k, (s, c, ang, sz, ex) in enumerate(NYMPHS)]
TUN_LEN = [np.r_[0, np.cumsum(np.linalg.norm(np.diff(p, axis=0), axis=1))] for p in TUNNELS]


def tunnel_point(k, s):
    """point + unit tangent along tunnel k at fraction s (0 chamber, 1 exit)"""
    L = TUN_LEN[k]
    d = s * L[-1]
    i = int(np.clip(np.searchsorted(L, d) - 1, 0, len(L) - 2))
    u = (d - L[i]) / max(L[i + 1] - L[i], 1e-9)
    P = TUNNELS[k][i] * (1 - u) + TUNNELS[k][i + 1] * u
    tg = TUNNELS[k][i + 1] - TUNNELS[k][i]
    return P, tg / (np.linalg.norm(tg) + 1e-9)


# ================================================================== trees
OBJS = []
FEET = []
TREES = []        # dict(x, z, side, lobes=[(c, r, sq)], spots, trunk_pts, base)
LEAF = [(74, 122, 56), (88, 136, 62), (66, 112, 54), (98, 142, 66), (80, 128, 50)]


def tree(x, z, h, cr, side, seed, kind='oak'):
    r = np.random.default_rng(seed)
    g = new_group()
    trunk = (104, 82, 64)
    tw = 0.13 + 0.03 * h
    OBJS.append(Obj(prism(x, z, tw, 0.0, h * 0.78, 6, r1=tw * 0.55, rot=r.uniform(0, 1)), trunk, 'trunk', grp=g))
    ch = h * 0.74
    lobes = [(np.array([x, ch, z]), cr, (1.05, 0.82, 1.0))]
    for k in range(4 if kind == 'oak' else 3):
        a = 2 * math.pi * k / 4 + r.uniform(-0.4, 0.4)
        off = np.array([math.cos(a) * cr * 0.62, r.uniform(-0.25, 0.3) * cr, math.sin(a) * cr * 0.5])
        lobes.append((lobes[0][0] + off, cr * r.uniform(0.55, 0.7), (1.1, 0.8, 1.0)))
    lobes.append((lobes[0][0] + np.array([0, cr * 0.55, 0]), cr * 0.6, (1.0, 0.85, 1.0)))
    for k, (c, rr, sq) in enumerate(lobes):
        col = np.array(LEAF[int(r.integers(0, len(LEAF)))], float) * r.uniform(0.94, 1.05)
        OBJS.append(Obj(icosa(c, rr, sq, rng=r), col, 'crown', grp=g))
    # two visible branches from the trunk into side lobes
    for c, rr, sq in lobes[1:3]:
        a = np.array([x, h * 0.5, z])
        b = c * 0.8 + a * 0.2
        d = b - a
        L = np.linalg.norm(d)
        n = 5
        bf = []
        for i in range(n):
            t0, t1 = i / n, (i + 1) / n
            p0, p1 = a + d * t0, a + d * t1
            w0, w1 = 0.06 * (1 - t0 * 0.6), 0.06 * (1 - t1 * 0.6)
            side_v = np.cross(d / L, [0, 0, 1.0])
            side_v /= np.linalg.norm(side_v) + 1e-9
            bf.append(face([p0 + side_v * w0, p1 + side_v * w1, p1 - side_v * w1, p0 - side_v * w0], up=False))
        OBJS.append(Obj(bf, trunk, 'branch', grp=g, double=True))
    # cicada perch spots: on the crown surfaces, biased toward the camera and the top
    spots = []
    for c, rr, sq in lobes:
        for _ in range(110):
            d = r.normal(0, 1, 3)
            d[2] = abs(d[2]) * 1.2 + 0.15
            d[1] += 0.25
            d /= np.linalg.norm(d)
            spots.append(c + d * rr * np.array(sq) * 1.0)
    trunk_pts = []
    for _ in range(60):
        yy = r.uniform(0.2, h * 0.7)
        a = r.uniform(-1.3, 1.3)               # front half of the trunk
        rr = tw * (1 - 0.45 * yy / (h * 0.78)) + 0.01
        trunk_pts.append(np.array([x + rr * math.sin(a), yy, z + rr * math.cos(a)]))
    TREES.append(dict(x=x, z=z, h=h, side=side, lobes=lobes, spots=np.array(spots), trunk_pts=np.array(trunk_pts),
                      top=lobes[-1][0] + [0, lobes[-1][1], 0], grp=g))
    FEET.append((x, z, tw * 1.4, 0.45))


# front + second rows per side (XIX left, XIII right); they frame the gap where the sun sets
for (x, z, h, cr, side, sd) in [(-2.15, -1.05, 3.9, 1.0, 'L', 1), (2.15, -1.0, 3.8, 1.0, 'R', 2),
                                (-3.25, -3.3, 4.7, 1.2, 'L', 3), (3.3, -3.4, 4.8, 1.2, 'R', 4),
                                (-1.75, -6.8, 3.2, 0.85, 'L', 5), (1.9, -7.2, 3.3, 0.85, 'R', 6)]:
    tree(x, z, h, cr, side, sd)
FRONT_TREES = [k for k, tr in enumerate(TREES) if tr['z'] > -4.0]
# background trees (no cicada role)
for (x, z, h, cr, sd) in [(-4.6, -9.5, 5.0, 1.3, 7), (4.4, -10.5, 5.2, 1.3, 8), (-6.2, -13.5, 5.5, 1.5, 9),
                          (6.6, -12.5, 5.0, 1.4, 10), (-3.6, -15.5, 4.4, 1.1, 11), (-8.0, -8.0, 5.0, 1.4, 12),
                          (8.2, -7.5, 5.2, 1.4, 13), (-9.5, -17.0, 5.6, 1.5, 14), (9.6, -19.5, 5.8, 1.5, 15)]:
    tree(x, z, h, cr, 'B', sd)

# eastern red cedars mid-field: their low branches throw long stripes across the field toward the cut
for (x, z, h, r, sd) in [(-3.1, -19.0, 4.6, 0.95, 31), (-1.55, -22.5, 5.0, 0.9, 32), (1.45, -21.5, 4.8, 0.9, 33), (3.2, -18.0, 4.4, 1.0, 34),
                         (-4.8, -24.0, 5.2, 1.1, 35), (5.0, -23.0, 5.0, 1.0, 36)]:
    g = new_group()
    OBJS.append(Obj(prism(x, z, 0.12, 0.0, 0.5, 5), (96, 74, 60), 'trunk', grp=g))
    OBJS.append(Obj(prism(x, z, r, 0.35, h * 0.62, 7, r1=r * 0.55, rot=sd), (58, 94, 62), 'cedar', grp=g))
    OBJS.append(Obj(prism(x, z, r * 0.6, h * 0.6, h, 7, r1=0.05, rot=sd + 0.4), (64, 100, 64), 'cedar', grp=g))
    FEET.append((x, z, r, 0.35))

# ================================================================== fence (the divider) + lantern hook post
FENCE_G = new_group()
POST = (118, 96, 76)
RAIL = (146, 120, 92)
fz = [-0.22] + list(np.arange(-2.2, -15.0, -2.0))
fence_faces = []
for k, z in enumerate(fz):
    h = 1.32 if k == 0 else 0.95
    OBJS.append(Obj(box(0.0, h / 2, z, 0.06, h / 2, 0.06), POST, 'post', grp=FENCE_G))
    FEET.append((0.0, z, 0.08, 0.35))
for a, b in zip(fz, fz[1:]):
    for y in (0.45, 0.8):
        zc = (a + b) / 2
        OBJS.append(Obj(box(0.02, y, zc, 0.035, 0.04, abs(a - b) / 2 + 0.05), RAIL, 'rail', grp=FENCE_G))
# hook arm from the front post out over the cut, lantern hangs at LANTERN
OBJS.append(Obj(box(0.0, 1.28, 0.05, 0.03, 0.03, 0.29), (70, 62, 58), 'arm', grp=FENCE_G))
OBJS.append(Obj(box(0.0, 1.07, 0.30, 0.008, 0.2, 0.008), (60, 56, 54), 'chain', grp=FENCE_G, cast=False))
LANT_G = new_group()
OBJS.append(Obj(box(LANTERN[0], LANTERN[1] + 0.13, LANTERN[2], 0.07, 0.025, 0.07), (58, 52, 50), 'lantcap', grp=LANT_G, cast=False))
OBJS.append(Obj(box(LANTERN[0], LANTERN[1], LANTERN[2], 0.052, 0.1, 0.052), (255, 196, 110), 'lamp', grp=LANT_G, cast=False))
OBJS.append(Obj(box(LANTERN[0], LANTERN[1] - 0.115, LANTERN[2], 0.065, 0.02, 0.065), (58, 52, 50), 'lantbase', grp=LANT_G, cast=False))

# ================================================================== farmhouse (porch lamp), barn + silo
HOUSE_G = new_group()
hx, hz = 4.2, -18.6
OBJS.append(Obj(box(hx, 1.05, hz, 1.5, 1.05, 1.15), (228, 214, 194), 'house', grp=HOUSE_G))
OBJS.append(Obj(gable(hx, 2.1, hz, 1.5, 1.15, 1.05, over=0.12), (126, 72, 66), 'roof', grp=HOUSE_G))
OBJS.append(Obj(box(hx - 0.5, 1.48, hz + 1.6, 1.0, 0.04, 0.45), (120, 76, 68), 'porchroof', grp=HOUSE_G))
for px in (hx - 1.4, hx + 0.4):
    OBJS.append(Obj(box(px, 0.74, hz + 1.98, 0.04, 0.74, 0.04), (236, 226, 210), 'porchpost', grp=HOUSE_G))
OBJS.append(Obj(box(hx + 1.0, 3.0, hz - 0.3, 0.14, 0.45, 0.14), (150, 92, 80), 'chimney', grp=HOUSE_G))
WINDOWS = [np.array([[hx + x0, y0, hz + 1.151], [hx + x0 + 0.42, y0, hz + 1.151], [hx + x0 + 0.42, y0 + 0.5, hz + 1.151],
                     [hx + x0, y0 + 0.5, hz + 1.151]]) for x0, y0 in [(-1.2, 0.5), (0.5, 0.5), (-1.2, 1.35), (0.5, 1.35)]]
FEET.append((hx, hz, 1.4, 0.4))
BARN_G = new_group()
bx, bz = -6.4, -27.0
OBJS.append(Obj(box(bx, 1.3, bz, 1.8, 1.3, 1.5), (176, 66, 54), 'barn', grp=BARN_G))
OBJS.append(Obj(gable(bx, 2.6, bz, 1.8, 1.5, 1.2, over=0.1), (92, 70, 66), 'barnroof', grp=BARN_G))
OBJS.append(Obj(box(bx, 1.0, bz + 1.51, 0.6, 1.0, 0.01), (232, 220, 200), 'barndoor', grp=BARN_G, cast=False))
SILO_G = new_group()
OBJS.append(Obj(prism(bx + 2.6, bz - 0.6, 0.85, 0.0, 4.6, 10), (178, 178, 186), 'silo', grp=SILO_G))
OBJS.append(Obj(prism(bx + 2.6, bz - 0.6, 0.86, 4.6, 5.3, 10, r1=0.1), (142, 146, 160), 'silotop', grp=SILO_G))
FEET.append((bx, bz, 1.8, 0.35))

# shrubs and stones along the cut edge
for (x, z, r, sd) in [(-0.95, -0.55, 0.32, 21), (1.0, -0.6, 0.28, 22), (-3.0, -0.5, 0.42, 23), (2.95, -0.45, 0.4, 24),
                      (-1.5, -2.6, 0.36, 25), (1.45, -2.4, 0.34, 26)]:
    rr = np.random.default_rng(sd)
    OBJS.append(Obj(icosa([x, r * 0.75, z], r, (1.3, 0.85, 1.1), rng=rr), np.array(LEAF[sd % 5]) * 0.95, 'bush'))
    FEET.append((x, z, r, 0.4))
for (x, z, r) in [(-0.3, -0.35, 0.09), (0.62, -1.4, 0.12), (-2.5, -0.3, 0.1), (2.4, -1.9, 0.11)]:
    OBJS.append(Obj(icosa([x, r * 0.4, z], r, (1.2, 0.6, 1.0), rng=rng), (150, 140, 128), 'stone'))

# ================================================================== grass tufts (cut edge lip + field near the cut)
class Tufts:
    def __init__(self, blades, col):
        self.blades = blades                      # list of (3,3) triangles
        self.col = np.array(col, float)
        self.grp = new_group()
        V = np.concatenate(blades)
        self.lo, self.hi, self.ctr = V.min(0), V.max(0), V.mean(0)
        self.ph = rng.uniform(0, 6.283)
        self.tone = 1 + rng.uniform(-0.08, 0.08, len(blades))


TUFTS = []
_GCOL = [(104, 150, 62), (118, 160, 70), (92, 138, 58), (130, 166, 76)]
for cx in np.arange(-5.6, 5.7, 0.62):
    bl = []
    for _ in range(14):
        x = cx + rng.uniform(-0.3, 0.3)
        z = rng.uniform(-0.08, 0.04) if rng.uniform() < 0.6 else rng.uniform(-0.6, -0.08)
        h = rng.uniform(0.06, 0.16)
        lean = rng.uniform(-0.05, 0.05)
        w = rng.uniform(0.015, 0.03)
        bl.append(np.array([[x - w, 0.0, z], [x + w, 0.0, z], [x + lean, h, z + rng.uniform(-0.02, 0.06)]]))
    TUFTS.append(Tufts(bl, _GCOL[int(rng.integers(0, 4))]))
for _ in range(60):
    cx, cz = rng.uniform(-5.5, 5.5), rng.uniform(-7.0, -0.7)
    if abs(cx) < 0.25:
        continue
    bl = []
    for _ in range(6):
        x, z = cx + rng.normal(0, 0.08), cz + rng.normal(0, 0.06)
        h = rng.uniform(0.08, 0.2)
        bl.append(np.array([[x - 0.025, 0.0, z], [x + 0.025, 0.0, z], [x + rng.uniform(-0.04, 0.04), h, z]]))
    TUFTS.append(Tufts(bl, _GCOL[int(rng.integers(0, 4))]))

# ================================================================== far layer (fogged, unshadowed)
FAR = []    # (faces, colour, name)
for _ in range(130):
    x, z = rng.uniform(-60, 60), rng.uniform(-88, -32)
    if abs(x - z * math.tan(math.radians(-1.0))) < 1.6 + 0.02 * (-z - 36):
        continue                               # keep the sun's patch of horizon clear
    h = rng.uniform(5, 10)
    r = rng.uniform(1.6, 3.2)
    f = prism(x, z, 0.3, 0, h * 0.5, 4) + icosa([x, h * 0.62, z], r, (1.15, 0.8, 1.0), rng=rng)
    FAR.append((f, np.array([70, 104, 66]) * rng.uniform(0.9, 1.08), 'treeline'))
for (z, base, amp, n, seed, col) in [(-160, 2.5, 6, 30, 3, (126, 128, 132))]:
    r = np.random.default_rng(seed)
    xs = np.linspace(-220, 220, n)
    ys = base + amp * (0.5 + 0.5 * np.sin(xs * 0.02 + r.uniform(0, 6))) + r.uniform(-1, 1, n)
    faces = [(np.array([[xs[i], ys[i], z], [xs[i + 1], ys[i + 1], z], [xs[i + 1], 0, z + 6], [xs[i], 0, z + 6]]),
              np.array([0, 0.5, 0.866])) for i in range(n - 1)]
    FAR.append((faces, np.array(col, float), 'ridge'))
FAR.sort(key=lambda it: -np.mean([abs(Q[:, 2].mean()) for Q, n in it[0]]))

CLOUDS = []
for (az, el, w, h) in [(-30, 10.0, 100, 9), (-8, 17.0, 140, 11), (22, 12.0, 120, 9), (-48, 19.0, 120, 10), (40, 21.0, 130, 10)]:
    d = dirv(az, el)
    c = d * 900.0
    right = np.array([math.cos(math.radians(az)), 0, math.sin(math.radians(az))])
    pts = []
    for k in range(7):
        u = k / 6
        pts.append(((u - 0.5) * 2 * w, h * (0.6 + 0.4 * math.sin(math.pi * u)) * rng.uniform(0.8, 1.2)))
    top = [c + right * x + np.array([0, y, 0]) for x, y in pts]
    bot = [c + right * x + np.array([0, -h * 0.3 * rng.uniform(0.6, 1.0), 0]) for x, _ in pts[::-1]]
    CLOUDS.append(np.array(top + bot))

# ================================================================== small life data
FIREFLIES = np.array([[rng.uniform(-5, 5), rng.uniform(0.15, 1.1), rng.uniform(-6, -0.4)] for _ in range(26)])
FF_PH = rng.uniform(0, 6.283, len(FIREFLIES))
# emergence holes on the ground near the cut (decals), plus the nymph exits
HOLES = [(float(ex), -0.06) for (s, c, a, sz, ex) in NYMPHS]
for tr in TREES[:6]:
    for _ in range(14):
        a, rr = rng.uniform(0, 6.283), rng.uniform(0.25, 1.1)
        x, z = tr['x'] + rr * math.cos(a), tr['z'] + rr * math.sin(a) * 0.8
        if z < -0.1:
            HOLES.append((x, z))

# ================================================================== soil texture (cut face, world x/y plane)
TX0, TX1, TY0, TY1, TRES = -6.4, 6.4, -4.4, 0.0, 0.008
TW, TH = int((TX1 - TX0) / TRES), int((TY1 - TY0) / TRES)
STRATA = [(0.0, (78, 54, 40)), (0.11, (140, 94, 62)), (0.88, (164, 112, 74)), (1.75, (184, 140, 96)), (2.7, (132, 116, 108))]


def strata_depth(x, y):
    return -y + 0.045 * np.sin(1.7 * x + 0.3) + 0.028 * np.sin(4.3 * x + 1.1) + 0.014 * np.sin(9.7 * x + 2.0)


def strata_col(x, y):
    """procedural soil colour (no features) for any x, y < 0 (also used outside the texture)"""
    dd = strata_depth(x, y)
    col = np.zeros(np.shape(x) + (3,))
    for (d0, c0), nxt in zip(STRATA, STRATA[1:] + [(99.0, None)]):
        m = (dd >= d0) & (dd < nxt[0])
        if not m.any():
            continue
        span = min(nxt[0], 6.0) - d0
        u = np.clip((dd[m] - d0) / span, 0, 1)
        shade = 1.06 - 0.14 * u + 0.05 * np.sin(x[m] * 3.1 + dd[m] * 11.0) * np.sin(dd[m] * 23.0)
        col[m] = np.array(c0, float) * shade[:, None]
    return col


def _build_soil():
    jj, ii = np.meshgrid(np.arange(TW), np.arange(TH))
    X = TX0 + (jj + 0.5) * TRES
    Y = TY1 - (ii + 0.5) * TRES
    col = strata_col(X, Y)
    # thin bright/dark seams at the layer boundaries
    dd = strata_depth(X, Y)
    for d0, _ in STRATA[1:]:
        col[np.abs(dd - d0) < 0.012] *= 0.82
    img = Image.fromarray(np.clip(col, 0, 255).astype(np.uint8))
    dr = ImageDraw.Draw(img)
    r = np.random.default_rng(17)
    px = lambda x, y: ((x - TX0) / TRES, (TY1 - y) / TRES)
    # pebbles (more and greyer deeper down)
    for _ in range(900):
        x, y = r.uniform(TX0, TX1), -abs(r.normal(0, 1.6)) - 0.1
        if y < TY0:
            continue
        rad = r.uniform(0.012, 0.05) * (1 + 0.5 * (-y > 1.8))
        c = np.array(strata_col(np.array([x]), np.array([y]))[0])
        c = c * r.uniform(0.75, 1.25) * np.array([1.0, 1.0, 1.08]) + 10
        a, b = px(x, y)
        rr = rad / TRES
        sq = r.uniform(0.55, 1.0)
        pts = [(a + rr * math.cos(t), b + rr * sq * math.sin(t)) for t in np.linspace(0, 6.283, 9)[:-1]]
        dr.polygon(pts, fill=tuple(int(v) for v in np.clip(c, 0, 255)))
        dr.line([pts[5], pts[6], pts[7]], fill=tuple(int(v) for v in np.clip(c * 1.18, 0, 255)))
    # grass rootlets in the topsoil
    for _ in range(1400):
        x = r.uniform(TX0, TX1)
        L = r.uniform(0.04, 0.16)
        a, b = px(x, -0.02)
        c, d = px(x + r.uniform(-0.04, 0.04), -0.02 - L)
        dr.line([(a, b), (c, d)], fill=(118, 88, 62), width=1)
    # tree roots from the front trees (they reach the cut) + a feeder root past every nymph chamber
    def root(x, y, ang, L, w, depth):
        pts = [(x, y)]
        for k in range(8):
            ang += r.uniform(-0.28, 0.28)
            x += math.cos(ang) * L / 8
            y += math.sin(ang) * L / 8
            if y > -0.03:
                y = -0.03
            pts.append((x, y))
        P = [px(a, b) for a, b in pts]
        for k in range(len(P) - 1):
            ww = max(1, int((w * (1 - 0.6 * k / 8)) / TRES))
            dr.line([P[k], P[k + 1]], fill=(198, 160, 118), width=ww + 2)
            dr.line([P[k], P[k + 1]], fill=(222, 186, 140), width=ww)
        if depth > 0:
            for k in (3, 6):
                root(pts[k][0], pts[k][1], ang + r.choice([-1, 1]) * r.uniform(0.5, 1.0), L * 0.55, w * 0.6, depth - 1)
    for tr in TREES:
        if tr['z'] > -4.0:
            for k in range(4):
                a0 = -math.pi / 2 + (k - 1.5) * 0.55 + r.uniform(-0.15, 0.15)
                root(tr['x'] + r.uniform(-0.15, 0.15), -0.04, a0, r.uniform(0.9, 1.5) * (1.3 if tr['z'] > -2 else 0.8), 0.04, 2)
    for k, (s, (cx, cy), ang, sz, ex) in enumerate(NYMPHS):
        a = math.radians(ang)
        side = np.array([math.cos(a - math.pi / 2), math.sin(a - math.pi / 2)]) * (0.16 * sz) * (1 if s == 'L' else -1)
        p_lo = np.array([cx, cy]) + side + np.array([0, -0.5])
        p_hi = np.array([cx, cy]) + side * 1.1 + np.array([0, 0.45])
        P = [px(*(p_lo * (1 - u) + p_hi * u + np.array([0.03 * math.sin(u * 7 + k), 0]))) for u in np.linspace(0, 1, 9)]
        dr.line(P, fill=(198, 160, 118), width=int(0.034 / TRES) + 2)
        dr.line(P, fill=(222, 186, 140), width=int(0.034 / TRES))
    # old abandoned tunnels (faint)
    for _ in range(26):
        x, y = r.uniform(TX0, TX1), r.uniform(-2.2, -0.3)
        pts = []
        for k in range(10):
            pts.append(px(x, y))
            x += r.uniform(-0.06, 0.06)
            y += r.uniform(0.02, 0.08)
            if y > -0.05:
                break
        c = np.array(strata_col(np.array([x]), np.array([min(y, -0.06)]))[0]) * 0.86
        dr.line(pts, fill=tuple(int(v) for v in c), width=int(0.035 / TRES))
    # active tunnels + chambers; id + along-tunnel maps for the glow
    tid = Image.new('L', (TW, TH), 0)
    tpar = Image.new('L', (TW, TH), 0)
    di, dp = ImageDraw.Draw(tid), ImageDraw.Draw(tpar)
    for k, P in enumerate(TUNNELS):
        Q = [px(*p) for p in P]
        n = len(Q)
        for i in range(n - 1):
            dr.line([Q[i], Q[i + 1]], fill=(92, 62, 44), width=int(0.085 / TRES))
        for i in range(n - 1):
            dr.line([Q[i], Q[i + 1]], fill=(50, 32, 24), width=int(0.06 / TRES))
            di.line([Q[i], Q[i + 1]], fill=k + 1, width=int(0.06 / TRES))
            dp.line([Q[i], Q[i + 1]], fill=int(255 * (i + 0.5) / (n - 1)), width=int(0.06 / TRES))
        s, (cx, cy), ang, sz, ex = NYMPHS[k]
        a = math.radians(ang)
        ux, uy = math.cos(a), math.sin(a)
        for rad_k, fill in ((1.22, (92, 62, 44)), (1.0, (42, 28, 22))):
            pts = []
            for t in np.linspace(0, 6.283, 20)[:-1]:
                lx, ly = 0.24 * sz * rad_k * math.cos(t), 0.13 * sz * rad_k * math.sin(t)
                pts.append(px(cx + lx * ux - ly * uy, cy + lx * uy + ly * ux))
            dr.polygon(pts, fill=fill)
            if rad_k == 1.0:
                di.polygon(pts, fill=k + 1)
                dp.polygon(pts, fill=0)
        # emergence hole at the top of the face
        a2, b2 = px(ex, -0.005)
        dr.ellipse([a2 - 0.05 / TRES, b2 - 0.02 / TRES, a2 + 0.05 / TRES, b2 + 0.03 / TRES], fill=(40, 28, 22))
    # sod lip: dark band + fringe at the very top
    arr = np.asarray(img).astype(np.float32)
    top = (TY1 - Y) < 0.035
    arr[top] = arr[top] * 0.55 + np.array([40, 52, 26]) * 0.45
    return arr.astype(np.uint8), np.asarray(tid).astype(np.uint8), np.asarray(tpar).astype(np.uint8)


SOIL, TUN_ID, TUN_PAR = _build_soil()
