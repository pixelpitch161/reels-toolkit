"""Prime-Number Cicadas renderer.

sky (golden hour, sun disc, clouds) -> per-pixel field behind the cut (sun shadows from the static
scene, lantern / porch pools, contact AO, emergence holes, haze) and per-pixel soil face (strata
texture, roots, tunnels, chambers, brood-coloured tunnel glow, lantern light) -> far layer ->
painter-sorted objects (precomputed box-occlusion sun shadows, rim light) + grass tufts -> nymphs in
their chambers -> cicada swarms, ghost broods, birds, eggs, fireflies -> locked depth of field ->
glows, god rays, bloom, grade -> HUD (13:17 lanes + year counter), labels, captions.

    python -m reelkit.sheet render.py test/sheet.png 1 6 10 14 18 22 26 30 36 45 51.3 55 59
    python -m reelkit.video render.py audio.wav prime_cicadas.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, dirv, mixc
from reelkit.camera import Cam, interp_keys, clip_near
from reelkit.raster import Frame, apply_glows, add_glow, SKY_DEPTH
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import draw_captions, panel, label, slide_offset, CREAM
from reelkit import font as RF
from reelkit.font import text_mask, stamp
import timeline as T
import scene as S

for ch, rows in {'→': ".....|..#..|...#.|#####|...#.|..#..|....."}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

SUN = S.SUN
W0 = 2 * math.pi / T.DUR          # every periodic motion uses integer multiples -> seamless loop
COLS = {s: np.array(T.COL[s], float) for s in T.SIDES}
FLASH = {'L': np.array([255, 104, 24], float), 'R': np.array([40, 230, 214], float)}
GHOST = np.array(T.GHOST_COL, float)
HAZE = np.array([240, 178, 132], float)

# ------------------------------------------------------------------ static shadows (sun is fixed)
GRID = MaskGrid((-24.0, 24.0, -48.0, 0.6, 0.04))
_casters = [Q for o in S.OBJS if o.cast and o.ctr[2] < 0.0 for Q, n in o.faces]
_casters += [Q for faces, col, name in S.FAR if name == 'treeline' for Q, n in faces if Q[:, 2].mean() > -47]
STATIC_MASK = directional_mask(GRID, _casters, SUN, blur=1.3)
HOLE_MASK = GRID.raster([[(x + 0.05 * math.cos(a), z + 0.035 * math.sin(a)) for a in np.linspace(0, 6.283, 9)[:-1]]
                         for x, z in S.HOLES], blur=0.4)
OCC = Occluders(S.OBJS)
for o in S.OBJS:          # per-face data, cached once
    o.C = np.array([Q.mean(0) for Q, n in o.faces])
    o.N = np.array([n for Q, n in o.faces])
    o.kv = OCC.vis(o.C + o.N * 0.03, o.grp, d=SUN)[:, None] if o.cast or o.name in ('lamp',) else np.ones((len(o.faces), 1))
    o.base = o.color[None] * o.tone[:, None]
    o.dist_fog = 0.0
SIDE_TREES = {s: [k for k, tr in enumerate(S.TREES) if tr['side'] == s] for s in T.SIDES}

# ------------------------------------------------------------------ camera
CLOSE = ((0.0, -0.98, 0.0), 4.6, -1.0, 2.0, 40, 340)
SPLIT = ((0.0, -0.74, 0.0), 7.8, 0.0, 2.5, 40, 334)
RACE = ((0.0, 1.0, -0.6), 14.0, 0.0, 4.0, 40, 340)
WIDE = ((0.0, 1.25, -0.8), 16.0, -1.5, 4.2, 40, 334)


def K(t, pose, dist=None, yaw=None, pitch=None, tgt=None, sy=None):
    p_tgt, p_d, p_yaw, p_pitch, p_fov, p_sy = pose
    return (t, np.array(tgt if tgt is not None else p_tgt, float), dist if dist is not None else p_d,
            yaw if yaw is not None else p_yaw, pitch if pitch is not None else p_pitch, p_fov, sy if sy is not None else p_sy)


KEYS = [
    K(0.0, CLOSE),
    K(2.4, CLOSE, dist=4.45, yaw=0.0),
    K(4.4, CLOSE, tgt=(0.0, -0.15, -0.2), dist=6.8, pitch=3.0),
    K(7.4, WIDE),
    K(8.2, WIDE, yaw=0.5),
    K(9.6, SPLIT),
    K(12.2, SPLIT, yaw=1.2, dist=7.6),
    K(15.6, SPLIT, yaw=-1.0, dist=7.9),
    K(16.7, RACE, dist=13.6, yaw=-1.5),
    K(20.2, RACE, yaw=1.5),
    K(23.2, RACE),
    K(29.5, RACE, yaw=-1.2),
    K(30.0, RACE, dist=13.0, tgt=(0.0, 0.85, -0.6)),
    K(31.6, RACE),
    K(44.3, RACE, yaw=1.6),
    K(44.8, RACE, dist=13.0, tgt=(0.0, 0.85, -0.6)),
    K(46.6, RACE),
    K(50.6, RACE, dist=14.6),
    K(51.5, WIDE, dist=16.4),
    K(54.0, WIDE, yaw=1.0),
    K(56.7, SPLIT, dist=7.4),
    K(T.T_HOME, CLOSE),
    K(60.0, CLOSE),
]


def camera(t):
    return interp_keys(KEYS, t)


# ------------------------------------------------------------------ sky
ZEN = np.array([92, 120, 198], float)
MID = np.array([230, 142, 128], float)
HOR = np.array([252, 152, 86], float)


def flare_amt(t):
    b = ss(T.T_2024 - 0.1, T.T_2024 + 0.4, t) * (1 - ss(T.T_END - 0.5, T.T_END + 1.2, t))
    if T.T_2024 <= t < T.T_END:
        b += 0.3 * math.exp(-(((t - T.GRID0) % T.BEAT) / T.BEAT) * 5)
    return b


def sky_color(d, flare=0.0):
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    k = np.clip(el / 0.7, 0, 1)
    k1 = np.clip(k / 0.25, 0, 1)[..., None]
    k2 = np.clip((k - 0.25) / 0.75, 0, 1)[..., None] ** 0.8
    col = HOR * (1 - k1) + MID * k1
    col = col * (1 - k2) + ZEN * k2
    cosang = (d * SUN).sum(-1)
    g = np.clip((cosang - 0.994) / 0.006, 0, 1) ** 2
    halo = np.clip((cosang - 0.5) / 0.5, 0, 1) ** 6
    col = col + (g * (0.22 + 0.22 * flare))[..., None] * np.array([255, 196, 120]) \
        + (halo * (0.07 + 0.2 * flare))[..., None] * np.array([255, 160, 100])
    return col


# ------------------------------------------------------------------ lights
def lights(t, cam):
    fl = flare_amt(t)
    L = Lights(amb=(0.5, 0.43, 0.47), sky=(0.14, 0.12, 0.2))
    L.view = cam.view
    L.key('sun', SUN, np.array([1.15, 0.84, 0.52]) * (1 + 0.18 * fl))
    L.fill(dirv(176, 22), (0.44, 0.35, 0.32))        # warm sky bounce from the camera side
    L.fill(dirv(90, 70), (0.12, 0.12, 0.2))          # cool zenith
    flick = 0.92 + 0.05 * math.sin(W0 * 397 * t) + 0.03 * math.sin(W0 * 731 * t + 1.0)
    L.point(S.LANTERN, np.array([1.15, 0.72, 0.36]) * flick, 1.3)
    return L, fl, flick


# ------------------------------------------------------------------ helpers
def hash2(i, j, k):
    return (np.sin(i * 127.1 + j * 311.7 + k * 74.7) * 43758.5453) % 1.0


GRASS = np.array([(104, 136, 64), (116, 146, 70), (96, 128, 60), (126, 150, 76), (110, 140, 58)], float)
DIRT = np.array([178, 148, 106], float)


def ground_albedo(X, Z, dist):
    u, v = X / 0.85, Z / 0.85
    i, j = np.floor(u), np.floor(v)
    tri = ((u - i) > (v - j)).astype(float)
    h = hash2(i, j, tri)
    col = GRASS[(h * 5).astype(int) % 5]
    n = 0.5 + 0.5 * np.sin(X * 0.41 + Z * 0.17) * np.sin(Z * 0.29 - X * 0.13)
    col = col * (0.92 + 0.14 * n)[:, None]
    fade = np.clip((dist - 30) / 60, 0, 1)[:, None]
    col = col * (1 - fade) + GRASS.mean(0)[None] * fade
    # dirt path beside the fence to the farmhouse (right side)
    pw = 0.32 + 0.02 * np.sin(Z * 1.7)
    m = (Z < -0.7) & (Z > -17.6) & (X > 0.3) & (X < 0.3 + pw + 0.25 * np.clip((-Z - 12) / 5, 0, 1) * (X > 0))
    col[m] = DIRT * (0.95 + 0.08 * hash2(np.floor(X[m] / 0.2), np.floor(Z[m] / 0.2), 1.0))[:, None]
    lip = Z > -0.05
    col[lip] = col[lip] * 0.72
    return col


def proj_faces(cam, Qs):
    out = []
    for Q in Qs:
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] < cam.near).any():
            if (Pc[:, 2] < cam.near).all():
                out.append(None)
                continue
            Pc = clip_near(Pc, cam.near)
            if Pc is None:
                out.append(None)
                continue
        out.append((cam.proj(Pc), float(Pc[:, 2].mean())))
    return out


def draw_shaded(fr, cam, L, Qs, N, C, base, kv, double=False, trans=0.0, emit=0.0, fogk=None):
    view = cam.pos[None] - C
    facing = (N * view).sum(1)
    keep = np.nonzero(facing > 0)[0] if not double else np.arange(len(Qs))
    if not len(keep):
        return
    Nk = N[keep].copy()
    if double:
        flip = facing[keep] < 0
        Nk[flip] = -Nk[flip]
    Ck = C[keep]
    z = (Ck - cam.pos) @ cam.M[2]
    order = np.argsort(-z)
    keep, Nk, Ck = keep[order], Nk[order], Ck[order]
    cols = shade(L, Nk, Ck, base[keep] if base.ndim == 2 else base, kvis=kv[keep], trans=trans,
                 emit=emit if np.isscalar(emit) else emit[keep])
    if fogk is not None:
        fk = fogk[keep]
        cols = cols * (1 - fk) + HAZE[None] * fk
    for j, c in zip(keep, cols):
        fr.poly(cam, Qs[j], c)


def onscreen(cam, c, r, margin=30):
    z = cam.depth(c)[0]
    if z < -r:
        return False
    if z < 0.4:
        return True
    s = cam.wp(c)
    m = r * cam.f / z + margin
    return -m < s[0] < W + m and -m < s[1] < H + m


# ------------------------------------------------------------------ nymphs (half-tube relief in the soil face)
NYM_BODY = np.array([186, 136, 88], float)


def nymph_faces(center, fwd, size, eyes_red, wig):
    f = np.array([fwd[0], fwd[1], 0.0])
    p = np.array([-fwd[1], fwd[0], 0.0])
    o = np.array([0.0, 0.0, 1.0])
    Lb = 0.36 * size
    prof = [(-0.5, 0.02), (-0.44, 0.08), (-0.34, 0.12), (-0.22, 0.135), (-0.1, 0.135), (0.0, 0.12),
            (0.08, 0.125), (0.24, 0.12), (0.3, 0.1), (0.42, 0.095), (0.5, 0.05)]
    th = np.linspace(0, math.pi, 6)
    rings = []
    for k, (u, r) in enumerate(prof):
        bend = 0.04 * size * math.sin(wig) * (u + 0.5) ** 2
        c = center + f * u * Lb + p * bend
        rings.append([c + (math.cos(a) * p + math.sin(a) * o) * r * Lb * 1.15 for a in th])
    faces = []
    for k in range(len(rings) - 1):
        u0 = prof[k][0]
        part = 'abd' if u0 < -0.02 else ('thx' if u0 < 0.28 else 'head')
        stripe = 0.86 if (part == 'abd' and k % 2 == 0) else 1.0
        for i in range(len(th) - 1):
            Q = np.array([rings[k][i], rings[k + 1][i], rings[k + 1][i + 1], rings[k][i + 1]])
            mid = Q.mean(0)
            axis_c = center + f * np.dot(mid - center, f)
            n = mid - axis_c
            n /= np.linalg.norm(n) + 1e-9
            col = NYM_BODY * stripe * (0.9 if part == 'head' else 1.0) * (1.06 if part == 'thx' else 1.0)
            faces.append((Q, n, col))
    # legs: stout digging forelegs + two thinner pairs, splayed in the face plane
    for sgn in (1, -1):
        for (u0, ang, L, w) in [(0.16, 0.55, 0.42, 0.05), (0.06, -0.5, 0.36, 0.03), (-0.02, -1.0, 0.4, 0.03)]:
            a = center + f * u0 * Lb + p * sgn * 0.1 * Lb + o * 0.04 * Lb
            dvec = (f * math.cos(ang) + p * sgn * math.sin(abs(ang) + 0.6)) * L * Lb
            kne = a + dvec * 0.55 + o * 0.02 * Lb
            b = a + dvec + f * 0.12 * Lb * (1 if ang > 0 else -0.6)
            for p0, p1 in ((a, kne), (kne, b)):
                d = p1 - p0
                s = np.cross(d, o)
                s = s / (np.linalg.norm(s) + 1e-9) * w * Lb
                faces.append((np.array([p0 + s, p1 + s * 0.8, p1 - s * 0.8, p0 - s]), o * 1.0, NYM_BODY * 0.82))
    # eyes
    ec = np.array([226, 52, 40], float) if eyes_red else np.array([234, 226, 210], float)
    for sgn in (1, -1):
        c = center + f * 0.38 * Lb + p * sgn * 0.085 * Lb + o * 0.09 * Lb
        r = 0.045 * Lb
        Q = np.array([c + f * r, c + p * r, c - f * r, c - p * r]) + o * 0.01
        faces.append((Q, o * 1.0, ec))
    return faces


def nymph_list(t):
    """[(k, side, centre(3), fwd(2), size, eyes_red, s, ds)] for nymphs currently underground"""
    out = []
    for k, (side, (cx, cy), ang, sz, ex) in enumerate(S.NYMPHS):
        s, size, present = T.nymph_state(side, t)
        if not present:
            continue
        s2, _, _ = T.nymph_state(side, t + 0.04)
        ds = s2 - s
        P, tg = S.tunnel_point(k, s)
        if s < 0.02:
            a = math.radians(ang)
            fwd = np.array([math.cos(a), math.sin(a)])
        else:
            fwd = tg if ds >= 0 else -tg     # climbing: head up the tunnel; burrowing: head-first downward
        climbing = ds > 1e-4
        eyes_red = t < T.T_POP0 or t > T.T_BURROW1[1] or climbing or (T.T_1803 - 0.4 < t < T.T_1803 + 0.05)
        out.append((k, side, np.array([P[0], P[1], 0.0]), fwd, sz * size, eyes_red, s, ds))
    return out


# ------------------------------------------------------------------ cicada swarms
class Swarm:
    def __init__(self, e):
        r = np.random.default_rng(1000 + e['id'])
        self.e = e
        side = e['side']
        trees = SIDE_TREES[side]
        wts = np.array([S.TREES[k]['lobes'][0][1] ** 2 * (1.6 if S.TREES[k]['z'] > -2 else 1.0) for k in trees])
        n = e['n']
        self.tree = r.choice(trees, n, p=wts / wts.sum())
        self.spot = np.array([S.TREES[k]['spots'][r.integers(len(S.TREES[k]['spots']))] for k in self.tree])
        base = np.array([[S.TREES[k]['x'], 0.02, S.TREES[k]['z']] for k in self.tree])
        a = r.uniform(0, 6.283, n)
        rr = r.uniform(0.2, 1.2, n)
        self.start = base + np.stack([rr * np.cos(a), np.zeros(n), np.minimum(rr * np.sin(a) * 0.7, -base[:, 2] - 0.05)], 1)
        if e['kind'] in ('hook', 'climax'):
            m = r.uniform(0, 1, n) < 0.4
            xs_ = r.uniform(0.15, 3.2, m.sum()) * (-1 if side == 'L' else 1)
            self.start[m] = np.stack([xs_, np.full(m.sum(), 0.02), -r.uniform(0.04, 0.5, m.sum())], 1)
        hero = [k for k, nm in enumerate(S.NYMPHS) if nm[0] == side]
        for j, k in enumerate(hero):
            if j < n:
                self.start[j] = [S.NYMPHS[k][4], 0.02, -0.04]
        spread = {'race': 0.28, 'joint': 0.4, 'hook': 0.9, 'climax': 1.0}[e['kind']]
        self.delay = r.uniform(0, spread, n) ** 1.3 / max(spread, 1e-6) ** 0.3
        self.fl = r.uniform(0.28, 0.48, n)
        self.flier = r.uniform(0, 1, n) < (0.3 if e['kind'] != 'climax' else 0.42)
        lob = [S.TREES[k]['lobes'][0] for k in self.tree]
        self.oc = np.array([c for c, rr_, sq in lob]) + np.stack([r.normal(0, 0.3, n), r.uniform(-0.2, 0.8, n), r.uniform(0.2, 0.9, n)], 1)
        self.orad = np.array([rr_ for c, rr_, sq in lob]) * r.uniform(1.1, 1.9, n)
        self.ow = r.choice([-1, 1], n) * r.uniform(1.8, 3.4, n)
        self.oph = r.uniform(0, 6.283, n)
        self.head = r.uniform(-1.0, 1.0, n) - math.pi / 2
        self.life = e['life'] - r.uniform(0, 0.25, n)
        self.ph = r.uniform(0, 6.283, n)

    def state(self, t):
        """positions (n,3), alpha (n,), flying (n,) bool; None if inactive"""
        age = t - self.e['t'] - self.delay
        on = (age >= 0) & (age <= self.life)
        if not on.any():
            return None
        n = len(age)
        P = np.zeros((n, 3))
        orb = self.oc + np.stack([self.orad * np.cos(self.ow * age + self.oph), 0.25 * np.sin(1.7 * self.ow * age + self.oph),
                                  0.45 * self.orad * np.sin(self.ow * age + self.oph)], 1)
        dest = np.where(self.flier[:, None], orb, self.spot)
        u = np.clip(age / self.fl, 0, 1)[:, None]
        mid = (self.start + dest) / 2 + np.array([0, 0.9, 0.25])
        bez = (1 - u) ** 2 * self.start + 2 * (1 - u) * u * mid + u * u * dest
        P = np.where(u < 1, bez, dest)
        jit = 0.012 * np.stack([np.sin(t * 9 + self.ph), np.sin(t * 7 + self.ph * 1.3), np.zeros(n)], 1)
        P = P + jit
        endu = np.clip((age - (self.life - 0.3)) / 0.3, 0, 1)
        P[:, 1] -= 0.7 * endu ** 2
        alpha = (1 - endu) * on
        flying = (u[:, 0] < 1) | self.flier
        return P, alpha, flying


SWARMS = [Swarm(e) for e in T.EMERGE]

# extra sky-fillers at the climax (both broods, big lazy orbits above the crowns)
_r = np.random.default_rng(2024)
SKY_N = 260
SKY_C = np.stack([_r.uniform(-3.4, 3.4, SKY_N), _r.uniform(3.6, 6.6, SKY_N), _r.uniform(-3.0, 0.6, SKY_N)], 1)
SKY_R = _r.uniform(0.4, 1.4, SKY_N)
SKY_W = _r.choice([-1, 1], SKY_N) * _r.uniform(1.2, 2.6, SKY_N)
SKY_PH = _r.uniform(0, 6.283, SKY_N)
SKY_D = _r.uniform(0, 1.6, SKY_N)
SKY_SIDE = np.where(SKY_C[:, 0] < 0, 0, 1)

# ghost broods (12 / 18 years): pale puffs around the front tree on each side
GHOST_PUFFS = [('L', tg) for tg in T.G12] + [('R', tg) for tg in T.G18]
_g = np.random.default_rng(1218)
GH_N = 40
GH_OFF = _g.normal(0, 1, (GH_N, 3)) * np.array([0.9, 0.6, 0.5])
GH_OFF[:, 2] = np.abs(GH_OFF[:, 2]) + 0.3

# eggs falling from twigs at the end
_e = np.random.default_rng(55)
EGG_N = 180
EGG_TREE = _e.choice(SIDE_TREES['L'] + SIDE_TREES['R'], EGG_N)
EGG_P = np.array([S.TREES[k]['spots'][_e.integers(len(S.TREES[k]['spots']))] for k in EGG_TREE])
EGG_T = _e.uniform(T.T_EGGS[0], T.T_EGGS[1] - 0.6, EGG_N)

CIC_BODY = np.array([40, 32, 30], float)
CIC_EYE = (236, 46, 36)


def draw_cicada(fr, dep_arr, x, y, z, Lpx, heading, wing_col, flying, alpha, t, ph, ghost=False):
    if not (-6 < x < W + 6 and -6 < y < H + 6):
        return
    xi, yi = int(x), int(y)
    if 0 <= xi < W and 0 <= yi < H and z > dep_arr[yi, xi] + 0.3:
        return                                   # hidden behind a crown / trunk
    if alpha < 0.5 and (int(t * 30) + int(ph * 10)) % 2:
        return                                   # dithered fade-out
    d = fr.draw
    if ghost:
        col = tuple(int(c) for c in GHOST * 0.95)
        d.rectangle([xi, yi, xi + max(0, int(Lpx * 0.5)), yi], fill=col)
        if Lpx > 2.5:
            d.point([(xi - 1, yi - 1), (xi + int(Lpx * 0.5) + 1, yi - 1)], fill=(236, 230, 255))
        return
    if Lpx < 2.6:
        d.point([(xi, yi)], fill=(34, 28, 28))
        d.point([(xi, yi - 1)], fill=CIC_EYE)
        if flying and (int(t * 40 + ph * 7) % 2):
            d.point([(xi - 1, yi), (xi + 1, yi)], fill=tuple(int(c) for c in wing_col))
        fr.ddraw.point([(xi, yi)], fill=float(z))
        return
    c, s = math.cos(heading), math.sin(heading)
    fx, fy = c, s
    px, py = -s, c
    L, w = Lpx, Lpx * 0.42
    body = [(x + fx * L * 0.5, y + fy * L * 0.5), (x + fx * L * 0.2 + px * w * 0.5, y + fy * L * 0.2 + py * w * 0.5),
            (x - fx * L * 0.45 + px * w * 0.35, y - fy * L * 0.45 + py * w * 0.35), (x - fx * L * 0.55, y - fy * L * 0.55),
            (x - fx * L * 0.45 - px * w * 0.35, y - fy * L * 0.45 - py * w * 0.35), (x + fx * L * 0.2 - px * w * 0.5, y + fy * L * 0.2 - py * w * 0.5)]
    wc = tuple(int(v) for v in wing_col)
    if flying:
        flap = math.sin(t * 70 + ph * 5)
        for sg in (1, -1):
            ex = x + px * sg * L * (0.7 + 0.25 * flap) - fx * L * 0.25
            ey = y + py * sg * L * (0.7 + 0.25 * flap) - fy * L * 0.25
            d.polygon([(x + fx * L * 0.15, y + fy * L * 0.15), (ex, ey), (x - fx * L * 0.35, y - fy * L * 0.35)],
                      fill=tuple(int(v * 0.55 + 230 * 0.45) for v in wing_col))
    else:
        for sg in (1, -1):
            d.line([(x + fx * L * 0.25 + px * sg * w * 0.45, y + fy * L * 0.25 + py * sg * w * 0.45),
                    (x - fx * L * 0.85 + px * sg * w * 0.25, y - fy * L * 0.85 + py * sg * w * 0.25)], fill=wc, width=1)
    d.polygon(body, fill=tuple(int(v) for v in CIC_BODY))
    fr.ddraw.polygon(body, fill=float(z))
    er = max(0.6, L * 0.12)
    for sg in (1, -1):
        exx = x + fx * L * 0.38 + px * sg * w * 0.42
        eyy = y + fy * L * 0.38 + py * sg * w * 0.42
        if er < 1.0:
            d.point([(exx, eyy)], fill=CIC_EYE)
        else:
            d.ellipse([exx - er, eyy - er, exx + er, eyy + er], fill=CIC_EYE)


# ------------------------------------------------------------------ birds
def bird_state(b, t):
    t0, dur, side = b
    u = (t - t0) / dur
    if not 0 <= u <= 1:
        return None
    tr = S.TREES[SIDE_TREES[side][0]]
    c = tr['lobes'][0][0] + np.array([0.0, 0.2, tr['lobes'][0][1] * 0.9])
    sg = -1 if side == 'L' else 1
    p0 = np.array([sg * 7.5, 5.2, -2.5])
    p2 = np.array([-sg * 1.2, 6.4, -1.0])
    p1 = 2 * c - 0.5 * (p0 + p2)
    P = (1 - u) ** 2 * p0 + 2 * (1 - u) * u * p1 + u * u * p2
    V = 2 * (1 - u) * (p1 - p0) + 2 * u * (p2 - p1)
    return P, V / (np.linalg.norm(V) + 1e-9), u


def draw_bird(fr, cam, P, V, u, t, carry):
    z = cam.depth(P)[0]
    if z < 0.5:
        return
    side = np.cross(V, [0, 1, 0])
    side /= np.linalg.norm(side) + 1e-9
    s = 0.2
    flap = math.sin(t * 22) * 0.9
    body = [P + V * s * 1.1, P - V * s * 0.9 + [0, 0.03, 0], P - V * s * 0.9 - [0, 0.03, 0]]
    tail = [P - V * s * 0.7, P - V * s * 1.5 + side * s * 0.25, P - V * s * 1.5 - side * s * 0.25]
    for wsg in (1, -1):
        wing = [P + V * s * 0.3, P + side * wsg * s * 1.9 + np.array([0, flap * s, 0]), P - V * s * 0.4]
        fr.poly(cam, np.array(wing), (64, 70, 92))
    fr.poly(cam, np.array(tail), (52, 56, 76))
    fr.poly(cam, np.array(body), (58, 64, 86))
    if carry:
        sp = cam.wp(P + V * s * 1.25)
        fr.draw.point([(sp[0], sp[1])], fill=(34, 28, 28))
        fr.draw.point([(sp[0] + 1, sp[1])], fill=CIC_EYE)


# ------------------------------------------------------------------ focus (locked on the nymphs + the cicada trees)
NYM_PTS = np.array([[c[0], c[1], 0.03] for s, c, a, sz, ex in S.NYMPHS])
TREE_PTS = []
for k in SIDE_TREES['L'] + SIDE_TREES['R']:
    tr = S.TREES[k]
    TREE_PTS.append([tr['x'], 0.3, tr['z']])
    for c, r, sq in tr['lobes']:
        TREE_PTS += [c + [0, 0, r * 0.8], c + [r, 0, 0], c - [r, 0, 0], c + [0, r * 0.8, 0]]
TREE_PTS = np.array(TREE_PTS)
EXTRA_PTS = np.array([S.LANTERN, [0.0, 0.6, -0.22], [0, -0.1, 0.0]])


def subject_points(cam):
    P = np.vstack([NYM_PTS, TREE_PTS, EXTRA_PTS])
    z = cam.depth(P)
    s = cam.wp(P)
    on = (z > cam.near) & (s[:, 0] > -20) & (s[:, 0] < W + 20) & (s[:, 1] > -20) & (s[:, 1] < H + 20)
    return z[on]


def focus(cam):
    zf = cam.depth(np.array([0.0, -1.0, 0.0]))[0]
    zs = subject_points(cam)
    need = float(np.abs(1 - zf / zs).max()) if len(zs) else 0.0
    sharp = max(0.06, need + 0.03)
    Kk = 0.011 * cam.f * min(1.2, 4.5 / zf)
    return zf, sharp, Kk


def subject_blur(cam):
    zf, sharp, Kk = focus(cam)
    zs = subject_points(cam)
    return float(np.clip(Kk * np.maximum(0.0, np.abs(1 - zf / zs) - sharp), 0, 10.5).max()) if len(zs) else 0.0


# ------------------------------------------------------------------ main render
def render(f):
    t = f / FPS
    cam = camera(t)
    L, fl, flick = lights(t, cam)
    # ---------------- sky
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    dn = d / np.linalg.norm(d, axis=-1, keepdims=True)
    sky = np.clip(np.floor(sky_color(dn, fl) / 7 + BAYER_FULL[..., None]) * 7, 0, 255)
    fr = Frame(sky)
    for Q in S.CLOUDS:
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] < 1).any():
            continue
        el = math.degrees(math.asin(Q[:, 1].mean() / 900.0))
        u = min(1.0, max(0.0, (el - 8) / 14))
        col = mixc((255, 206, 168), (236, 178, 186), u) * (1 + 0.12 * fl)
        fr.screen_poly(cam.proj(Pc), np.floor(col / 6) * 6, z=900.0)
    sp = cam.screen_of_dir(SUN)
    if sp is not None:
        rs = (0.017 + 0.004 * fl) * cam.f
        fr.disc(sp, rs * 1.3, (255, 222, 150), n=16, z=SKY_DEPTH)
        fr.disc(sp, rs, (255, 252, 232), n=16, z=SKY_DEPTH)
    arr, depth = fr.array(), fr.depth()
    # ---------------- per-pixel field (behind the cut) and soil face (the cut)
    pos = cam.pos
    dy, dz = d[..., 1], d[..., 2]
    tg = np.where(dy < -1e-6, -pos[1] / np.minimum(dy, -1e-6), np.inf)
    Zg = pos[2] + tg * dz
    tf_all = np.where(dz < -1e-6, -pos[2] / np.minimum(dz, -1e-6), np.inf)
    face = np.isfinite(tf_all) & (pos[1] + tf_all * dy < 0)          # reaches the cut plane below ground
    ground = ~face & np.isfinite(tg) & (Zg < 0) & (tg < 3000)
    # field
    ys, xs = np.nonzero(ground)
    if len(xs):
        tt = tg[ys, xs]
        X = pos[0] + tt * d[ys, xs, 0]
        Z = Zg[ys, xs]
        lit = 1 - GRID.sample(STATIC_MASK, X, Z)
        I = np.tile(L.amb * 0.72 + L.sky, (len(X), 1))
        I += (lit * 0.66)[:, None] * L.keys[0][2][None]
        I += 0.4 * L.fills[0][1][None] + 0.8 * L.fills[1][1][None]
        lp = S.LANTERN
        dist = np.sqrt((lp[0] - X) ** 2 + lp[1] ** 2 + (lp[2] - Z) ** 2)
        I += (0.9 * flick / (1 + (dist / 1.0) ** 2))[:, None] * np.array([1.0, 0.64, 0.32])[None]
        dist = np.sqrt((S.PORCH[0] - X) ** 2 + S.PORCH[1] ** 2 + (S.PORCH[2] - Z) ** 2)
        I += (0.7 / (1 + (dist / 1.4) ** 2))[:, None] * np.array([1.0, 0.7, 0.36])[None]
        I *= contact_ao(X, Z, S.FEET)[:, None]
        I *= (1 - 0.65 * GRID.sample(HOLE_MASK, X, Z))[:, None]
        col = ground_albedo(X, Z, tt) * I
        k = np.clip((tt - 26.0) / 260.0, 0, 0.82)[:, None]
        col = col * (1 - k) + HAZE[None] * k
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tt
    # soil face
    ys, xs = np.nonzero(face)
    nym = nymph_list(t)
    if len(xs):
        tf = -pos[2] / d[ys, xs, 2]
        X = pos[0] + tf * d[ys, xs, 0]
        Y = pos[1] + tf * d[ys, xs, 1]
        tj = np.floor((X - S.TX0) / S.TRES).astype(np.int64)
        ti = np.floor((S.TY1 - Y) / S.TRES).astype(np.int64)
        inside = (tj >= 0) & (tj < S.TW) & (ti >= 0) & (ti < S.TH)
        alb = np.zeros((len(X), 3))
        alb[inside] = S.SOIL[ti[inside], tj[inside]]
        if (~inside).any():
            alb[~inside] = S.strata_col(X[~inside], Y[~inside])
        tid = np.zeros(len(X), np.int64)
        tpar = np.zeros(len(X))
        tid[inside] = S.TUN_ID[ti[inside], tj[inside]]
        tpar[inside] = S.TUN_PAR[ti[inside], tj[inside]] / 255.0
        # light: ambient + camera-side bounce + lantern; deeper = a touch darker
        I = np.tile(L.amb * 0.9 + 0.95 * L.fills[0][1], (len(X), 1))
        v = S.LANTERN[None] - np.stack([X, Y, np.zeros_like(X)], 1)
        dist = np.linalg.norm(v, axis=1)
        lam = 0.35 + 0.65 * np.clip(v[:, 2] / dist, 0, 1)
        I += (1.5 * flick * lam / (1 + (dist / 1.5) ** 2))[:, None] * np.array([1.0, 0.66, 0.36])[None]
        I *= (1 - 0.16 * np.clip(-Y / 3.0, 0, 1))[:, None]
        tint = np.where((X < 0)[:, None], np.array([1.03, 1.0, 0.95]), np.array([0.96, 1.0, 1.04]))
        col = alb * I * tint
        # tunnel glow in brood colour
        base_g = 0.0
        if 8.0 <= t < T.T_1803 + 0.5:
            base_g = 0.14 * ss(8.0, 9.0, t) * (1 - ss(15.8, 16.6, t)) + 0.1 * ss(20.0, 21.0, t)
        elif T.T_1803 <= t < T.T_2024 + 1.0:
            base_g = 0.07
        for (k, side, P, fwd, size, er, s, ds) in nym:
            m = tid == k + 1
            if not m.any():
                continue
            g = np.full(m.sum(), base_g)
            if ds > 1e-4:
                g = np.maximum(g, 0.55 * np.clip(1 - (s - tpar[m]) / 0.6, 0, 1) * (tpar[m] <= s + 0.02))
            elif ds < -1e-4:
                g = np.maximum(g, 0.45 * np.clip(1 - (tpar[m] - s) / 0.6, 0, 1) * (tpar[m] >= s - 0.02))
            col[m] = col[m] * (1 - 0.25 * g[:, None]) + COLS[side][None] * g[:, None] * 0.95
        # rings / pulses: chambers flash when the 8-12 s rings complete
        for side in T.SIDES:
            dt_ = t - T.RING_DONE[side]
            if 0 <= dt_ < 0.6:
                k = [i for i, nm in enumerate(S.NYMPHS) if nm[0] == side][0]
                m = tid == k + 1
                col[m] = col[m] * 0.7 + COLS[side][None] * 0.9 * (1 - dt_ / 0.6)
        # divider: dashed line down the middle of the face
        dm = (np.abs(X) < 0.011 * (tf / 8.0 + 0.6)) & (Y < -0.05) & (((-Y) / 0.14) % 1.0 < 0.55)
        col[dm] = col[dm] * 0.45 + np.array([250, 236, 206]) * 0.55
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tf
    fr.set_array(arr, depth)
    face_img, face_dep = arr.copy(), depth.copy()
    # ---------------- far layer
    for faces, col, name in S.FAR:
        C = np.array([Q.mean(0) for Q, n in faces])
        dist = np.linalg.norm(C - cam.pos, axis=1)
        N = np.array([n for Q, n in faces])
        kf = np.clip((dist - 30) / 160, 0, 0.8)[:, None]
        draw_shaded(fr, cam, L, [Q for Q, n in faces], N, C, np.tile(col, (len(faces), 1)), np.ones((len(faces), 1)), fogk=kf)
    # ---------------- objects + tufts, painter order
    items = []
    for o in S.OBJS:
        if o.ctr[2] < -40 or not onscreen(cam, o.ctr, np.linalg.norm(o.hi - o.lo) / 2 + 0.5):
            continue
        items.append((np.linalg.norm(o.ctr - cam.pos), 0, o))
    for tf_ in S.TUFTS:
        if onscreen(cam, tf_.ctr, 0.5):
            items.append((np.linalg.norm(tf_.ctr - cam.pos), 1, tf_))
    items.sort(key=lambda a: -a[0])
    for dist, kind, o in items:
        if kind == 0:
            emit = 0.0
            if o.name == 'lamp':
                emit = 0.9 * flick
            kf = None
            if o.ctr[2] < -8:
                kf = np.full((len(o.faces), 1), min(0.55, (dist - 10) / 70))
            draw_shaded(fr, cam, L, [Q for Q, n in o.faces], o.N, o.C, o.base, o.kv, double=o.double,
                        trans=0.22 if o.name == 'crown' else 0.0, emit=emit, fogk=kf)
        else:
            sway = 0.012 * math.sin(W0 * 41 * t + o.ph)
            for j, Q in enumerate(o.blades):
                Q2 = Q.copy()
                Q2[2, 0] += sway
                c = o.col * o.tone[j] * (L.amb + 0.5 * L.fills[0][1] + 0.35 * L.keys[0][2] * 0.6)
                fr.poly(cam, Q2, np.clip(np.round(c / 5) * 5, 0, 255))
    # farmhouse windows + porch lamp (warm local lights in the distance)
    for Q in S.WINDOWS:
        fr.poly(cam, Q, (255, 206, 120))
    # anything behind the cut plane that landed on the soil face is hidden by the soil (low cameras)
    a2, d2 = fr.array(), fr.depth()
    hid = face & (d2 > face_dep + 0.01)
    if hid.any():
        a2[hid], d2[hid] = face_img[hid], face_dep[hid]
        fr.set_array(a2, d2)
    # ---------------- nymphs in the face
    for (k, side, P, fwd, size, er, s, ds) in sorted(nym, key=lambda a: a[0]):
        if size < 0.05:
            continue
        wig = W0 * 120 * t + k * 1.3 if s < 0.02 else W0 * 900 * t + k
        fcs = nymph_faces(P, fwd, size, er, wig)
        C = np.array([q.mean(0) for q, n, c in fcs])
        N = np.array([n for q, n, c in fcs])
        B = np.array([c for q, n, c in fcs])
        cols = shade(L, N, C, B, kvis=np.zeros((len(fcs), 1)), rimk=0.0)
        edge = np.clip(1 - np.abs(N[:, 2]), 0, 1) ** 2
        cols = np.clip(cols + edge[:, None] * COLS[side][None] * 0.35, 0, 255)
        order = np.argsort(C[:, 2])
        for j in order:
            fr.poly(cam, fcs[j][0], cols[j])
    # ---------------- cicadas, ghosts, birds, eggs, shells
    dep_arr = fr.depth()
    for side in T.SIDES:
        ne = T.n_emerged(side, t)
        if ne == 0:
            continue
        fade = 1 - ss(55.5, 58.5, t)
        if fade <= 0:
            continue
        for k in SIDE_TREES[side]:
            tr = S.TREES[k]
            nshow = min(len(tr['trunk_pts']), 5 * ne)
            for p in tr['trunk_pts'][:nshow]:
                z = cam.depth(p)[0]
                if z < 0.5:
                    continue
                sx, sy_ = cam.wp(p)
                if fade < 0.6 and (int(sx * 7 + sy_ * 3) % 3 == 0):
                    continue
                if 0 <= sx < W and 0 <= sy_ < H and z <= dep_arr[int(sy_), int(sx)] + 0.2:
                    fr.draw.point([(sx, sy_)], fill=(196, 150, 96))
    for sw in SWARMS:
        st = sw.state(t)
        if st is None:
            continue
        P, alpha, flying = st
        on = np.nonzero(alpha > 0)[0]
        if not len(on):
            continue
        z = cam.depth(P[on])
        scr = cam.wp(P[on])
        wing = COLS[sw.e['side']] * 0.75 + np.array([240, 200, 140]) * 0.25
        for j, i in enumerate(on):
            if z[j] < 0.4:
                continue
            Lpx = 0.11 * cam.f / z[j]
            draw_cicada(fr, dep_arr, scr[j, 0], scr[j, 1], z[j], Lpx, sw.head[i] + 0.3 * math.sin(t * 3 + sw.ph[i]),
                        wing, flying[i], alpha[i], t, sw.ph[i])
    if T.SKY_FILL[0] <= t <= T.SKY_FILL[1]:
        age = t - T.SKY_FILL[0] - SKY_D
        a = age > 0
        endk = 1 - ss(T.SKY_FILL[1] - 0.8, T.SKY_FILL[1], t)
        Pk = SKY_C + np.stack([SKY_R * np.cos(SKY_W * t + SKY_PH), 0.3 * np.sin(2 * SKY_W * t + SKY_PH), 0.5 * SKY_R * np.sin(SKY_W * t + SKY_PH)], 1)
        rise = np.clip(age / 0.8, 0, 1)
        Pk[:, 1] = 2.0 + (Pk[:, 1] - 2.0) * rise
        z = cam.depth(Pk)
        scr = cam.wp(Pk)
        for i in np.nonzero(a)[0]:
            if z[i] < 0.4:
                continue
            side = 'L' if SKY_SIDE[i] == 0 else 'R'
            wing = COLS[side] * 0.75 + np.array([240, 200, 140]) * 0.25
            draw_cicada(fr, dep_arr, scr[i, 0], scr[i, 1], z[i], 0.11 * cam.f / z[i], -math.pi / 2 + SKY_W[i] * 0.3,
                        wing, True, endk, t, SKY_PH[i])
    for side, tg_ in GHOST_PUFFS:
        age = t - tg_
        if not 0 <= age < 0.34:
            continue
        tr = S.TREES[SIDE_TREES[side][0]]
        c = tr['lobes'][0][0]
        P = c[None] + GH_OFF * (1 + age * 1.5)
        z = cam.depth(P)
        scr = cam.wp(P)
        for i in range(GH_N):
            if z[i] > 0.4:
                draw_cicada(fr, dep_arr, scr[i, 0], scr[i, 1], z[i], 0.09 * cam.f / z[i], 0, GHOST, True, 1 - age / 0.34, t, i, ghost=True)
    carry_t = {}
    for b in T.BIRDS:
        st = bird_state(b, t)
        if st is None:
            continue
        P, V, u = st
        draw_bird(fr, cam, P, V, u, t, carry=u > 0.5)
    if T.T_EGGS[0] <= t <= T.T_EGGS[1]:
        for i in range(EGG_N):
            age = t - EGG_T[i]
            if age < 0:
                continue
            p = EGG_P[i] + np.array([0.05 * math.sin(age * 5 + i), -0.5 * 9.8 * 0.18 * age ** 2 - 0.4 * age, 0])
            if p[1] < 0.02:
                continue
            z = cam.depth(p)[0]
            if z < 0.4:
                continue
            sx, sy_ = cam.wp(p)
            if 0 <= sx < W and 0 <= sy_ < H and z <= dep_arr[int(sy_), int(sx)] + 0.3:
                fr.draw.point([(sx, sy_)], fill=(250, 244, 226))
    # fireflies (warm points, periodic over 60 s)
    ffp = S.FIREFLIES + np.stack([0.3 * np.sin(W0 * 7 * t + S.FF_PH), 0.12 * np.sin(W0 * 11 * t + 2 * S.FF_PH), 0.3 * np.cos(W0 * 5 * t + S.FF_PH)], 1)
    ffb = np.clip(np.sin(W0 * 37 * t + S.FF_PH * 3) * 1.6 - 0.4, 0, 1)
    # ---------------- lens: focus locked on the nymphs (+ every visible cicada tree inside the sharp band)
    img, depth = fr.array(), fr.depth()
    zf, sharp, Kk = focus(cam)
    img = dof(img, coc_map(depth, zf, Kk, sharp=sharp))
    if os.environ.get('FOCUS_LOG'):
        with open(os.environ['FOCUS_LOG'], 'a') as fh:
            fh.write(f'{f} {subject_blur(cam):.4f}\n')
    # ---------------- glows
    glows = []

    def vis(p, z):
        sx, sy_ = cam.wp(p)
        if not (0 <= sx < W and 0 <= sy_ < H):
            return True
        return z <= depth[int(sy_), int(sx)] + 0.08

    ls = cam.wp(S.LANTERN)
    lz = cam.depth(S.LANTERN)[0]
    if lz > 0.3:
        glows.append((ls[0], ls[1], min(50, 0.45 * cam.f / lz), (255, 176, 90), 0.42 * flick))
        glows.append((ls[0], ls[1], min(14, 0.1 * cam.f / lz), (255, 230, 170), 0.6))
    pz = cam.depth(S.PORCH)[0]
    if pz > 1 and vis(S.PORCH, pz):
        s_ = cam.wp(S.PORCH)
        glows.append((s_[0], s_[1], max(3, 0.6 * cam.f / pz), (255, 190, 100), 0.5))
    for p, b in zip(ffp, ffb):
        z = cam.depth(p)[0]
        if z > 0.5 and b > 0.05 and vis(p, z):
            s_ = cam.wp(p)
            glows.append((s_[0], s_[1], max(1.5, 0.05 * cam.f / z), (255, 228, 110), 0.55 * b))
    # crown flash in brood colour on every emergence (the polyrhythm you can see)
    for e in T.EMERGE:
        age = t - e['t']
        if not -0.02 <= age <= e['life'] + 0.3:
            continue
        a = (0.55 if e['kind'] != 'race' else 0.42) * math.exp(-max(age, 0) / 0.32) + 0.1 * (1 - ss(e['life'] - 0.3, e['life'] + 0.3, age))
        for k in SIDE_TREES[e['side']]:
            tr = S.TREES[k]
            c = tr['lobes'][0][0]
            z = cam.depth(c)[0]
            if z > 0.5 and vis(c, z - tr['lobes'][0][1]):
                s_ = cam.wp(c)
                glows.append((s_[0], s_[1], min(60, 1.5 * tr['lobes'][0][1] * cam.f / z), tuple(FLASH[e['side']]), a * (1.15 if e['side'] == 'L' else 1.0) * (1.0 if tr['z'] > -4 else 0.6)))
    for side, tg_ in GHOST_PUFFS:
        age = t - tg_
        if 0 <= age < 0.4:
            tr = S.TREES[SIDE_TREES[side][0]]
            c = tr['lobes'][0][0]
            z = cam.depth(c)[0]
            if not vis(c, z - tr['lobes'][0][1]):
                continue
            s_ = cam.wp(c)
            meet = any(abs(tg_ - m) < 1e-6 for m in T.G_MEET)
            glows.append((s_[0], s_[1], min(60, 1.3 * tr['lobes'][0][1] * cam.f / z), tuple(GHOST), (0.5 if meet else 0.28) * (1 - age / 0.4)))
    if sp is not None:
        glows.append((sp[0], sp[1], min(40, 0.026 * cam.f * (1 + 0.4 * fl)), (255, 170, 90), 0.12 + 0.14 * fl))
    img = apply_glows(img, [g for g in glows if g[-1] > 0])
    if sp is not None:
        img = god_rays(img, depth, sp, (255, 176, 110), 0.2 + 0.12 * fl)
    img = grade(bloom(np.clip(img, 0, 255), thr=212, k=0.3 + 0.1 * fl), contrast=0.18, sat=1.16)
    img = np.clip(img * vignette(0.8, 0.6, 0.32) * top_scrim(), 0, 255).astype(np.uint8)
    return draw_ui(img, t, cam, nym)


# ------------------------------------------------------------------ UI
def big_text(out, s, cx, y, col, scale, alpha=1.0, clamp=True):
    m = text_mask(s, scale)
    x = cx - m.shape[1] / 2
    if clamp:
        x = min(max(x, 6), W - 6 - m.shape[1])
    stamp(out, m, int(x), int(y), tuple(int(c) for c in col), alpha=alpha)
    return m.shape[1]


def draw_panel(pil, out_texts, t):
    if not (T.T_PANEL_IN <= t <= T.T_PANEL_OUT + 0.4):
        return
    d = ImageDraw.Draw(pil)
    off = slide_offset(t, T.T_PANEL_IN, T.T_PANEL_OUT, dist=140)
    y0 = 528 + off
    x0, x1 = 12, 348
    panel(d, x0, y0, x1, y0 + 96)
    tx0, tx1, play = 64, 342, 206
    lanes = (y0 + 44, y0 + 70)
    morph = ss(T.T_LANES, T.T_LANES + 0.6, t)
    ghost = t < T.T_LANES + 0.3
    if ghost:
        gy = T.ghost_year(t)
        ppy = 2.0
        cur = gy
        periods = (12, 18)
        cols = (GHOST * 0.9 + np.array([30, 0, 0]), GHOST)
        names = ("12-YR", "18-YR")
        joint = 36
    else:
        cur = T.race_year(t) if t >= T.T_1803 - 0.01 else T.Y0 + (t - T.T_1803) / T.S16
        ppy = 6.0
        periods = (13, 17)
        cols = (COLS['L'], COLS['R'])
        names = ("XIX", "XIII")
        joint = 221
    a = 1 - abs(morph - 0.5) * 2 if T.T_LANES - 0.1 < t < T.T_LANES + 0.7 else 1.0
    # beat grid (1 year = 1 sixteenth): dots between the lanes
    mid = (lanes[0] + lanes[1]) // 2
    y_first = math.floor(cur - (play - tx0) / ppy)
    for yv in range(int(y_first), int(cur + (tx1 - play) / ppy) + 2):
        x = play + (yv - cur) * ppy
        if not tx0 <= x <= tx1:
            continue
        rel = yv - (T.Y0 if not ghost else 0)
        if not ghost:
            bright = 120 if rel % 16 == 13 else (80 if rel % 4 == 1 else 44)   # bar lines fall where 2024 does
            d.point([(x, mid)], fill=(bright, bright, int(bright * 1.2)))
        elif rel % 6 == 0:
            d.point([(x, mid)], fill=(70, 70, 90))
    for li, (P, col) in enumerate(zip(periods, cols)):
        yl = lanes[li]
        d.line([tx0, yl, tx1, yl], fill=(34, 42, 70))
        base = 0 if ghost else T.Y0
        lo = int(math.floor((cur - (play - tx0) / ppy - base) / P)) - 1
        hi = int(math.ceil((cur + (tx1 - play) / ppy - base) / P)) + 1
        for kk in range(max(lo, 0), hi + 1):
            yv = base + kk * P
            if not ghost and yv > T.Y1:
                continue
            x = play + (yv - cur) * ppy
            if not tx0 - 2 <= x <= tx1 + 2:
                continue
            is_joint = (yv - base) % joint == 0
            past = yv < cur - 0.5
            near = abs(yv - cur) < 0.8
            c = np.array(col, float) * (0.5 if past else 1.0)
            if near:
                c = np.minimum(c * 1.4 + 60, 255)
            hgt = 9 if not near else 12
            d.rectangle([x - 1, yl - hgt, x + 1, yl + hgt // 3], fill=tuple(int(v * a + 16 * (1 - a)) for v in c))
            if is_joint and li == 0:
                jc = (255, 250, 230) if not past else (150, 146, 136)
                d.rectangle([x - 1, lanes[0] - 12, x + 1, lanes[1] + 4], fill=jc)
    # near-miss brackets
    if not ghost:
        for (y_a, y_b) in T.NEAR:
            xa, xb = play + (y_a - cur) * ppy, play + (y_b - cur) * ppy
            if tx0 < xa < tx1 and cur > y_a - 30:
                d.line([xa, lanes[1] + 8, xa, lanes[1] + 11, xb, lanes[1] + 11, xb, lanes[1] + 8], fill=(255, 240, 200))
                out_texts.append(("1 YR", int((xa + xb) / 2) - 11, lanes[1] + 14, (255, 240, 200), 1))
    d.line([play, y0 + 30, play, y0 + 90], fill=(220, 226, 250))
    for li in range(2):
        out_texts.append((names[li], x0 + 6, lanes[li] - 9, tuple(int(c) for c in cols[li]), 1))
        if not ghost:
            out_texts.append((str(periods[li]), x0 + 6 + 6 * len(names[li]) + 4, lanes[li] - 9, (200, 200, 214), 1))
    if ghost:
        if t >= T.GH_T0:
            out_texts.append(('BIG', f"+{int(cur):03d} YRS", 180, y0 + 8, (226, 220, 255), 2))
            for m in T.G_MEET:
                if 0 <= t - m < 0.45:
                    out_texts.append(("MEET!", play + 6, y0 + 32, (236, 230, 255), 1))
    else:
        yr = int(math.floor(cur + 1e-6))
        out_texts.append(('BIG', f"{max(T.Y0, yr)}", 180, y0 + 6, (252, 240, 214), 3))


def draw_ui(img, t, cam, nym):
    pil = Image.fromarray(img)
    texts = []
    draw_panel(pil, texts, t)
    d = ImageDraw.Draw(pil)
    hero = {s: [n for n in nym if n[0] == [i for i, nm in enumerate(S.NYMPHS) if nm[0] == s][0]] for s in T.SIDES}
    # 8-12 s progress rings around the hero nymphs
    if T.RING_T0 - 0.3 <= t <= T.T_PRIME + 0.2:
        a = ss(T.RING_T0 - 0.3, T.RING_T0, t) * (1 - ss(T.T_PRIME, T.T_PRIME + 0.2, t))
        for s in T.SIDES:
            if not hero[s]:
                continue
            P = hero[s][0][2]
            z = cam.depth(P)[0]
            c = cam.wp(P)
            r = 0.3 * cam.f / z
            fr_ = T.ring_frac(s, t)
            col = tuple(int(v) for v in COLS[s])
            done = t >= T.RING_DONE[s]
            d.ellipse([c[0] - r - 2, c[1] - r - 2, c[0] + r + 2, c[1] + r + 2], outline=(20, 16, 14), width=4)
            d.arc([c[0] - r, c[1] - r, c[0] + r, c[1] + r], -90, -90 + 360 * fr_, fill=(255, 250, 230) if done and int(t * 10) % 2 else col, width=3)
            yrs = int(fr_ * T.PERIOD[s] + 1e-6)
            texts.append(('C', f"{yrs}/{T.PERIOD[s]}", c[0], c[1] + r + 6, col, 2, a))
    out = np.asarray(pil).copy()
    # nymph tags (hook + the last seconds -> seamless loop)
    tag_a = 1 - ss(4.1, 4.4, t) if t < 5 else ss(T.T_HOME - 0.2, T.T_HOME + 0.2, t)
    if tag_a > 0.02:
        for s in T.SIDES:
            if hero[s]:
                P = hero[s][0][2]
                z = cam.depth(P)[0]
                c = cam.wp(P + np.array([0, 0.3, 0]))
                big_text(out, f"{T.PERIOD[s]} YR", c[0], c[1] - 10, COLS[s], 2, alpha=tag_a)
    # brood names over the trees (4.8-8 s)
    if 4.8 <= t <= 8.0:
        a = ss(4.8, 5.1, t) * (1 - ss(7.7, 8.0, t))
        for s in T.SIDES:
            tr = S.TREES[SIDE_TREES[s][0]]
            c = cam.wp(tr['top'] + np.array([0, 0.35, 0]))
            big_text(out, f"BROOD {T.NAME[s]}", c[0], max(122, c[1] - 8), COLS[s], 2, alpha=a)
    # 12-23 s: the numbers themselves (13 / 17, ghosts 12 / 18)
    if T.T_PRIME <= t <= T.T_1803:
        for s in T.SIDES:
            if not hero[s]:
                continue
            P = hero[s][0][2]
            sgn = -1 if s == 'L' else 1
            c = cam.wp(P + np.array([sgn * 0.62, 0.16, 0]))
            ghost_k = ss(15.9, 16.4, t) * (1 - ss(19.9, 20.4, t))
            a = ss(T.T_PRIME, T.T_PRIME + 0.3, t) * (1 - ss(T.T_1803 - 0.4, T.T_1803, t))
            if a <= 0.02:
                continue
            ping = math.exp(-max(0.0, t - T.T_PING) / 0.25) if t >= T.T_PING else 0.0
            col = np.minimum(COLS[s] * (1 - 0.6 * ghost_k) + ping * 80, 255)
            sc = 5 if cam.f / max(cam.depth(P)[0], 0.1) > 90 else 4
            big_text(out, str(T.PERIOD[s]), c[0], c[1] - 20, col, sc, alpha=a * (1 - 0.5 * ghost_k))
            if t < 16.0:
                big_text(out, f"1 × {T.PERIOD[s]}", c[0], c[1] + 20, (236, 226, 204), 1, alpha=a * ss(T.T_PING, T.T_PING + 0.3, t))
            if ghost_k > 0.02:
                big_text(out, "12" if s == 'L' else "18", c[0], c[1] - 64, GHOST, sc, alpha=ghost_k)
    for item in texts:
        if item[0] == 'BIG':
            _, s_, cx, y, col, sc = item
            big_text(out, s_, cx, y, col, sc)
        elif item[0] == 'C':
            _, s_, cx, y, col, sc, a = item
            big_text(out, s_, cx, y, col, sc, alpha=a)
        else:
            s_, x, y, col, sc = item
            label(out, s_, x, y, col, sc)
    for t0, t1, s_, col, y in T.TAGS:
        if t0 <= t <= t1:
            big_text(out, s_, W / 2, y, col, 1, alpha=ss(t0, t0 + 0.25, t) * (1 - ss(t1 - 0.2, t1, t)))
    draw_captions(out, t, T.CAPTIONS, hi=T.COL['L'], alt=T.COL['R'])
    return out
