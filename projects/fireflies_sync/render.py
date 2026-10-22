"""Fireflies That Sync renderer.

sky (moon, stars) -> ridges + far meadow trees (fogged) -> per-pixel leaf litter (moon shadow stripes from
every trunk, creek with moon glints, trail with red-light pools, light thrown by the flashes, contact AO) ->
painter-sorted objects (cached moon visibility, rim light, flash light) -> near firefly bodies + the hero
(detailed low-poly model in the macro) -> locked depth of field on the hero -> flashes as glows (sharp) or
octagonal bokeh (out of focus), females, red lights -> god rays, bloom, grade -> HUD + captions.

    python -m reelkit.sheet render.py test/sheet.png 1 5.5 10 14 19 22 27 30 36 44 49 54 58
    python -m reelkit.video render.py audio.wav fireflies_sync.mp4"""
import math
import os
import sys
import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, '..', '..'))
from reelkit.config import W, H, FPS, BAYER_FULL, XX, YY, ss, ease, dirv, mixc
from reelkit.camera import Cam, clip_near
from reelkit.raster import Frame, add_glow, SKY_DEPTH
from reelkit.lighting import Lights, shade
from reelkit.shadows import MaskGrid, directional_mask, Occluders, contact_ao
from reelkit.lens import coc_map, dof, add_bokeh
from reelkit.post import god_rays, bloom, grade, vignette, top_scrim
from reelkit.ui import panel, label, CREAM, AMBER, MINT
from reelkit import font as RF
from reelkit.font import text_mask, stamp, rich_line
from scipy.ndimage import gaussian_filter
import timeline as T
import scene as S
import sim as M

for ch, rows in {'≈': ".....|.##.#|#..#.|.....|.##.#|#..#.|.....",
                 '→': ".....|..#..|...#.|#####|...#.|..#..|....."}.items():
    RF.GLYPHS[ch] = np.array([[c == '#' for c in r] for r in rows.split('|')])

_SIM = np.load(os.path.join(HERE, 'sim.npz'))
IM, IF = _SIM['IM'], _SIM['IF']
MOON = S.MOON
W0 = 2 * math.pi / T.DUR

FLASH = np.array([206, 255, 104], float)         # P. carolinus yellow-green
FLASH_CORE = np.array([246, 255, 214], float)
FEM = np.array([96, 236, 150], float)
RED = np.array([255, 52, 40], float)
FOG = np.array([46, 66, 108], float)
GREEN = (150, 255, 120)

# ------------------------------------------------------------------ static moon shadows + per-face caches
GRID = MaskGrid((-30.0, 30.0, -70.0, 12.0, 0.04))
_cast = [Q for o in S.OBJS if o.cast for Q, n in o.faces]
MOON_MASK = directional_mask(GRID, _cast, MOON, blur=1.4)
OCC = Occluders(S.OBJS)
for o in S.OBJS:
    o.C = np.array([Q.mean(0) for Q, n in o.faces])
    o.N = np.array([n for Q, n in o.faces])
    o.kv = OCC.vis(o.C + o.N * 0.03, o.grp, d=MOON)[:, None]
    o.base = o.color[None] * o.tone[:, None]
    o.sway = o.name == 'fern'
    o.ph = (o.ctr[0] * 3.1 + o.ctr[2] * 1.7) % 6.283

# flash light: splat flashing males onto a ground grid each frame (near swarm + valley)
LG_N = (-10.0, 10.0, -14.0, 6.0, 0.1)
LG_V = (-30.0, 30.0, -62.0, -2.0, 0.5)


def light_grid(ext, P, I):
    x0, x1, z0, z1, res = ext
    nx, nz = int(round((x1 - x0) / res)), int(round((z1 - z0) / res))
    g = np.zeros((nz, nx), np.float32)
    if len(P):
        j = np.clip(((P[:, 0] - x0) / res).astype(int), 0, nx - 1)
        i = np.clip(((P[:, 2] - z0) / res).astype(int), 0, nz - 1)
        w = I / (1 + (P[:, 1] / 0.9) ** 2)
        np.add.at(g, (i, j), w)
    return g


def sample_grid(ext, g, X, Z):
    x0, x1, z0, z1, res = ext
    j = ((X - x0) / res).astype(int)
    i = ((Z - z0) / res).astype(int)
    ok = (i >= 0) & (i < g.shape[0]) & (j >= 0) & (j < g.shape[1])
    out = np.zeros(np.shape(X), np.float32)
    out[ok] = g[i[ok], j[ok]]
    return out


# ------------------------------------------------------------------ camera: log-distance zooms, hero tracking
def Hk(off):
    return ('H', np.array(off, float))


KEYS = [  # (t, target, dist, yaw, pitch, fov, sy)
    (0.0, Hk((0.0, 0.2, -1.4)), 7.4, -5.0, 7.0, 50, 330),
    (3.7, Hk((0.0, 0.12, -1.0)), 6.5, -3.0, 7.0, 50, 330),
    (6.1, Hk((0.0, 0.0, 0.0)), 0.085, -24.0, 12.0, 40, 345),
    (8.5, Hk((0.0, 0.0, 0.0)), 0.095, -12.0, 10.0, 40, 345),
    (11.8, Hk((0.0, 0.0, -0.35)), 2.5, -5.0, 6.0, 46, 345),
    (16.5, Hk((0.0, 0.05, -0.45)), 3.2, 5.0, 7.0, 46, 345),
    (20.8, Hk((0.0, 0.0, -1.3)), 6.6, 0.0, 10.0, 50, 338),
    (25.2, Hk((0.0, 0.0, -1.4)), 5.8, -5.0, 9.0, 50, 338),
    (27.6, (0.1, 0.66, -0.1), 2.6, 12.0, 1.0, 50, 340),
    (30.4, (0.1, 0.68, -0.15), 2.9, 6.0, 1.5, 50, 340),
    (33.7, (0.0, 1.3, -2.4), 9.6, 0.0, 8.0, 50, 330),
    (38.6, (0.0, 1.3, -2.4), 9.2, 2.0, 8.0, 50, 330),
    (46.8, (0.0, 0.6, -15.0), 25.0, -2.0, 16.0, 52, 330),
    (51.8, (0.0, 0.6, -15.0), 23.8, -0.5, 16.0, 52, 330),
    (55.3, (0.0, 0.6, -15.0), 23.4, 1.0, 16.0, 52, 330),
    (60.0, Hk((0.0, 0.2, -1.4)), 7.4, -5.0, 7.0, 50, 330),
]


def _tgt(spec, t):
    if isinstance(spec, tuple) and len(spec) == 2 and spec[0] == 'H':
        return M.hero(t) + spec[1]
    return np.array(spec, float)


def cam_params(t):
    if t <= KEYS[0][0]:
        a = b = KEYS[0]
        u = 0.0
    elif t >= KEYS[-1][0]:
        a = b = KEYS[-1]
        u = 0.0
    else:
        for a, b in zip(KEYS, KEYS[1:]):
            if t <= b[0]:
                break
        u = ease((t - a[0]) / (b[0] - a[0]))
    d = math.exp(math.log(a[2]) * (1 - u) + math.log(b[2]) * u)
    # on big zooms the target slides in proportion to the distance, so the subject stays put on screen
    w = (d - b[2]) / (a[2] - b[2]) if abs(math.log(a[2] / b[2])) > 0.5 else 1 - u
    tg = _tgt(a[1], t) * w + _tgt(b[1], t) * (1 - w)
    yaw, pitch, fov, sy = [a[i] * (1 - u) + b[i] * u for i in range(3, 7)]
    # a soft bump on every synced flash during the drops
    ph, k, bb = T.phase(t)
    if ph == 'burst' and k >= 1:
        fb = (bb % 1.0)
        fov -= 0.9 * math.exp(-fb * 6) * (0.6 + 0.2 * k)
    return tg, d, yaw, pitch, fov, sy


def camera(t):
    tg, d, yaw, pitch, fov, sy = cam_params(t)
    return Cam(tg, d, yaw, pitch, fov, sy, near=0.01 if d < 1.0 else 0.1)


def focus(cam, t):
    """locked on the hero for the whole video: CoC 0 over his whole body"""
    h = M.hero(t)
    zf = cam.depth(h)[0]
    pts = h[None] + np.array([[0.008, 0, 0], [-0.008, 0, 0], [0, 0.006, 0], [0, -0.006, 0], [0, 0, 0.008], [0, 0, -0.008]])
    zs = cam.depth(pts)
    need = float(np.abs(1 - zf / zs).max())
    sharp = max(need + 0.02, min(0.28, 1.0 / max(zf, 0.05)))
    K = 0.009 * cam.f * min(1.2, 4.5 / max(zf, 0.05))
    return zf, sharp, K


# ------------------------------------------------------------------ lights
def lights(t, cam, flash_lvl):
    L = Lights(amb=(0.32, 0.4, 0.56), sky=(0.1, 0.14, 0.22))
    L.view = cam.view
    L.key('moon', MOON, (0.86, 0.92, 1.06))
    L.fill(dirv(170, 24), (0.3, 0.36, 0.5))          # cool bounce from the camera side (keeps the lit level)
    L.fill(dirv(60, 70), (0.08, 0.09, 0.16))
    for p in S.RED_LIGHTS:
        L.point(p, np.array([0.9, 0.16, 0.12]), 0.6)
    return L


# ------------------------------------------------------------------ helpers
def hash2(i, j, k):
    return (np.sin(i * 127.1 + j * 311.7 + k * 74.7) * 43758.5453) % 1.0


LITTER = np.array([(122, 92, 62), (108, 84, 58), (136, 104, 68), (98, 80, 58), (126, 100, 64), (114, 96, 70)], float)
MOSS = np.array([84, 122, 66], float)
DIRT = np.array([150, 128, 98], float)
WATER = np.array([30, 56, 84], float)


def ground_albedo(X, Z, dist):
    u, v = X / 0.22, Z / 0.22
    i, j = np.floor(u), np.floor(v)
    tri = ((u - i) > (v - j)).astype(float)
    h = hash2(i, j, tri)
    col = LITTER[(h * 6).astype(int) % 6]
    n = 0.5 + 0.5 * np.sin(X * 0.73 + Z * 0.31) * np.sin(Z * 0.53 - X * 0.27 + 1.3)
    moss = np.clip((n - 0.62) / 0.16, 0, 1)
    col = col * (1 - moss[:, None] * 0.85) + MOSS[None] * moss[:, None] * 0.85
    fade = np.clip((dist - 14) / 40, 0, 1)[:, None]
    col = col * (1 - fade) + LITTER.mean(0)[None] * fade
    td = S.trail_dist(X, Z)
    m = td < S.TRAIL_HW
    col[m] = DIRT * (0.92 + 0.1 * hash2(np.floor(X[m] / 0.15), np.floor(Z[m] / 0.15), 2.0))[:, None]
    return col


def proj_faces_draw(fr, cam, Qs, cols):
    for Q, c in zip(Qs, cols):
        fr.poly(cam, Q, c)


def draw_obj(fr, cam, L, o, t, flash_lg):
    view = cam.pos[None] - o.C
    facing = (o.N * view).sum(1)
    keep = np.nonzero(facing > 0)[0] if not o.double else np.arange(len(o.faces))
    if not len(keep):
        return
    Nk = o.N[keep].copy()
    if o.double:
        flip = facing[keep] < 0
        Nk[flip] = -Nk[flip]
    Ck = o.C[keep]
    z = (Ck - cam.pos) @ cam.M[2]
    order = np.argsort(-z)
    keep, Nk, Ck, z = keep[order], Nk[order], Ck[order], z[order]
    emit = 0.0
    if flash_lg is not None:
        fl = sample_grid(LG_N, flash_lg, Ck[:, 0], Ck[:, 2]) * np.clip(1.3 - Ck[:, 1] * 0.25, 0.2, 1.3)
        emit = np.clip(fl, 0, 0.9)
    cols = shade(L, Nk, Ck, o.base[keep], kvis=o.kv[keep], trans=0.25 if o.name in ('fern', 'crown', 'rhodo') else 0.0)
    if np.ndim(emit):
        cols = np.clip(cols + emit[:, None] * FLASH[None] * 0.55 * (o.base[keep] / 160.0), 0, 255)
    dist = np.linalg.norm(Ck - cam.pos, axis=1)
    kf = np.clip((dist - 14) / 90, 0, 0.62)[:, None]
    cols = cols * (1 - kf) + FOG[None] * kf
    sw = None
    if o.sway:
        sw = np.array([0.025 * math.sin(W0 * 23 * t + o.ph), 0, 0.02 * math.sin(W0 * 17 * t + o.ph * 1.3)])
    for j, c in zip(keep, cols):
        Q = o.faces[j][0]
        if sw is not None:
            Q = Q + sw[None] * np.clip(Q[:, 1:2] / 0.4, 0, 1.2)
        fr.poly(cam, Q, c)


def onscreen(cam, lo, hi, margin=40):
    c = (lo + hi) / 2
    r = np.linalg.norm(hi - lo) / 2
    z = cam.depth(c)[0]
    if z < -r:
        return False
    if z < r + 0.5:
        return True
    s = cam.wp(c)
    m = r * cam.f / z + margin
    return -m < s[0] < W + m and -m < s[1] < H + m


def sort_key(cam, o):
    p = np.clip(cam.pos, o.lo, o.hi)
    return float(np.linalg.norm(p - cam.pos)) + 0.02 * float(np.linalg.norm(o.hi - o.lo))


# ------------------------------------------------------------------ the hero: a low-poly Photinus carolinus male
def _part_box(c, hx, hy, hz):
    from reelkit.geometry import box
    return box(c[0], c[1], c[2], hx, hy, hz)


def hero_parts(t, flash):
    """local frame (mm, scaled to m): +x forward, +y up, +z left. Returns [([Q], col, emissive)]"""
    out = []
    mm = 0.001

    def add(pts, col, em=0.0):
        out.append((np.array(pts, float) * mm, col, em))

    # abdomen: dark segments, the lantern (last two ventral segments) pale cream -> yellow-green
    for k, (x0, x1, w) in enumerate([(2.6, -0.2, 2.1), (-0.2, -2.6, 2.0), (-2.6, -4.6, 1.75), (-4.6, -6.4, 1.3)]):
        lant = k >= 2
        col = mixc((238, 230, 178), (238, 255, 176), flash) if lant else (64, 54, 48)
        em = flash if lant else 0.0
        y0, y1 = -0.9, 0.5
        add([[x0, y0, w], [x1, y0, w * 0.9], [x1, y0, -w * 0.9], [x0, y0, -w]], col, em)
        for sd in (1, -1):
            add([[x0, y0, w * sd], [x0, y1, w * sd * 0.95], [x1, y1, w * sd * 0.86], [x1, y0, w * sd * 0.9]], col, em)
        add([[x1, y0, w * 0.9], [x1, y1, w * 0.86], [x1, y1, -w * 0.86], [x1, y0, -w * 0.9]], col, em)
    # head + eyes under the front of the shield, antennae
    add([[6.6, 0.1, 0.9], [6.6, 0.1, -0.9], [6.6, 1.1, -0.9], [6.6, 1.1, 0.9]], (40, 34, 32))
    for sd in (1, -1):
        add([[5.6, 0.1, 1.0 * sd], [6.6, 0.1, 0.9 * sd], [6.6, 1.1, 0.9 * sd], [5.6, 1.2, 1.0 * sd]], (28, 24, 26))
        a0 = np.array([6.6, 0.8, 0.5 * sd])
        a1 = a0 + np.array([2.2, 1.5, 1.6 * sd])
        a2 = a1 + np.array([1.8, 0.7, 1.4 * sd])
        for p, q in ((a0, a1), (a1, a2)):
            out.append((_ribbon(p * mm, q * mm, 0.00017), (40, 34, 32), 0.0))
    # pronotum: domed shield, yellow rim, pink flanks, black centre spot
    ring = [np.array([2.6, 1.35, -2.6])]
    for k in range(11):
        ang = -math.pi / 2 + math.pi * k / 10
        ring.append(np.array([3.0 + 3.5 * max(0.0, math.cos(ang)) ** 0.8, 1.25, 2.8 * math.sin(ang)]))
    ring.append(np.array([2.6, 1.35, 2.6]))
    c0 = np.array([4.4, 2.05, 0.0])
    for p, q in zip(ring, ring[1:]):
        m1, m2 = c0 + (p - c0) * 0.62, c0 + (q - c0) * 0.62
        n1, n2 = c0 + (p - c0) * 0.3, c0 + (q - c0) * 0.3
        add([p, q, m2, m1], (240, 208, 112))
        add([m1, m2, n2, n1], (232, 116, 112))
        add([n1, n2, c0], (34, 28, 28))
    # elytra: domed plates, dark brown with a pale yellow outer margin; opened a little in flight
    cl, sl = math.cos(math.radians(20)), math.sin(math.radians(20))
    for sd in (1, -1):
        hinge = np.array([2.7, 1.5, 0.3 * sd])
        cs, ssp = math.cos(math.radians(13)), math.sin(math.radians(13)) * sd

        def tf(p):
            x, y, z = p - hinge
            x, y = x * cl + y * sl, -x * sl + y * cl          # lift: the tip rises
            x, z = x * cs + z * ssp, -x * ssp + z * cs        # spread: the tip swings out
            return hinge + np.array([x, y, z])

        xs = [2.7, -0.6, -3.8, -6.6]
        ws = [2.5, 2.6, 2.3, 1.2]
        for j in range(3):
            xa, xb, wa, wb = xs[j], xs[j + 1], ws[j], ws[j + 1]
            sut_a, sut_b = np.array([xa, 1.75, 0.15 * sd]), np.array([xb, 1.75 - 0.15 * j, 0.15 * sd])
            mid_a, mid_b = np.array([xa, 1.95, 0.6 * wa * sd]), np.array([xb, 1.85 - 0.15 * j, 0.6 * wb * sd])
            edg_a, edg_b = np.array([xa, 1.25, wa * sd]), np.array([xb, 1.2 - 0.1 * j, wb * sd])
            add([tf(sut_a), tf(sut_b), tf(mid_b), tf(mid_a)], (62, 50, 44))
            add([tf(mid_a), tf(mid_b), tf(edg_b), tf(edg_a)], (220, 190, 100))
    # legs: short, dark, tucked under in flight
    for k, x in enumerate((4.2, 2.0, -0.2)):
        for sd in (1, -1):
            p0 = np.array([x, -0.8, 1.2 * sd])
            p1 = p0 + np.array([0.5 - 0.6 * k, -1.2, 1.1 * sd])
            p2 = p1 + np.array([-0.9, -0.9, 0.2 * sd])
            out.append((_ribbon(p0 * mm, p1 * mm, 0.00022), (44, 36, 32), 0.0))
            out.append((_ribbon(p1 * mm, p2 * mm, 0.00016), (44, 36, 32), 0.0))
    return [([Q], col, em) for Q, col, em in out]


def _box_local(c, hx, hy, hz):
    return [Q for Q, n in _part_box(c, hx, hy, hz)]


def _ribbon(a, b, w):
    d = b - a
    p = np.cross(d, [0, 1.0, 0.3])
    p = p / (np.linalg.norm(p) + 1e-12) * w
    return np.array([a + p, b + p, b - p, a - p])


def hero_frame(t):
    """hero heading (body axis) + slight bob; faces left-forward across the frame"""
    yaw = math.radians(-70 + 10 * math.sin(W0 * 2 * t + 0.4))
    pitch = math.radians(6 + 3 * math.sin(W0 * 5 * t))
    fwd = np.array([math.cos(yaw) * math.cos(pitch), math.sin(pitch), -math.sin(yaw) * math.cos(pitch)])
    up = np.array([0, 1.0, 0])
    left = np.cross(up, fwd)
    left /= np.linalg.norm(left)
    up = np.cross(fwd, left)
    return np.stack([fwd, up, left])          # rows: local x, y, z axes in world


def draw_hero(fr, cam, L, t, flash):
    h = M.hero(t)
    R = hero_frame(t)
    parts = hero_parts(t, flash)
    polys = []
    for Qs, col, em in parts:
        for Q in Qs:
            Pw = h[None] + Q @ R
            c = Pw.mean(0)
            n = np.cross(Pw[1] - Pw[0], Pw[2] - Pw[0])
            nn = np.linalg.norm(n)
            if nn < 1e-14:
                continue
            n /= nn
            if np.dot(n, cam.pos - c) < 0:
                n = -n
            polys.append((cam.depth(c)[0], Pw, n, np.array(col, float), em))
    polys.sort(key=lambda a: -a[0])
    if not polys:
        return
    N = np.array([p[2] for p in polys])
    C = np.array([p[1].mean(0) for p in polys])
    B = np.array([p[3] for p in polys])
    E = np.array([p[4] for p in polys])
    cols = shade(L, N, C, B, kvis=np.ones((len(polys), 1)), rimk=0.9, emit=(E * 1.4 + 0.25)[:, None])
    # the lantern lights the body from below when it flashes
    lit = np.clip(-(C - h) @ R[1] / 0.003 + 0.5, 0, 1) * flash
    cols = np.clip(cols + lit[:, None] * FLASH[None] * 0.35, 0, 255)
    for p, c in zip(polys, cols):
        fr.poly(cam, p[1], c)
    # hind wings: translucent blur discs either side
    return h, R


def hind_wings(img, cam, t, h, R):
    """two translucent, motion-blurred hind wings (alpha-blended after the body is drawn)"""
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for s in (1, -1):
        root = h + R[0] * 0.0022 + R[1] * 0.0012 + R[2] * 0.0006 * s
        pts = []
        for a in np.linspace(-0.35, 1.25, 9):
            ang = a + 0.25 * math.sin(t * 61 + s)
            v = (-R[0] * math.cos(ang) * 0.6 + R[2] * s * math.sin(ang) + R[1] * 0.45 * math.cos(ang * 0.7)) * 0.0105
            pts.append(root + v)
        P = np.vstack([root[None], np.array(pts)])
        Pc = cam.to_cam(P)
        if (Pc[:, 2] < cam.near).any():
            continue
        d.polygon([tuple(p) for p in cam.proj(Pc)], fill=255)
    a = np.asarray(m, np.float32)[..., None] / 255.0 * 0.2
    return img * (1 - a) + np.array([150, 170, 210], float)[None, None] * a


# ------------------------------------------------------------------ main render
def render(f):
    t = f / FPS
    cam = camera(t)
    Im = IM[f].astype(np.float32)
    If = IF[f].astype(np.float32)
    on = np.nonzero(Im > 0.02)[0]
    Pm = M.pos(on, t) if len(on) else np.zeros((0, 3))
    Iv = Im[on]
    Ig = Iv ** 0.55                  # perceptual: a 100-150 ms flash lingers to the eye
    near = on < 500
    lg_n = light_grid(LG_N, Pm[near], Iv[near])
    lg_n = gaussian_filter(lg_n, 6.0) * 12.0
    lg_v = gaussian_filter(light_grid(LG_V, Pm[~near], Iv[~near]), 2.5) * 2.6
    fo_all = np.nonzero(IF[f] > 0.02)[0]
    fnear = fo_all[fo_all < M.N_FN]
    lg_f = gaussian_filter(light_grid(LG_N, M.PF[fnear], IF[f][fnear].astype(np.float32) * 0.2), 2.2) * 9.0 if len(fnear) else None
    L = lights(t, cam, 0.0)
    # ---------------- sky
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    dn = d / np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.clip(dn[..., 1], -1, 1)
    k = np.clip(el / 0.55, 0, 1) ** 0.7
    sky = np.array([58, 88, 142], float) * (1 - k[..., None]) + np.array([10, 18, 58], float) * k[..., None]
    cosm = (dn * MOON).sum(-1)
    sky += (np.clip((cosm - 0.96) / 0.04, 0, 1) ** 3 * 0.4)[..., None] * np.array([150, 160, 200])
    sky += (np.clip((cosm - 0.6) / 0.4, 0, 1) ** 4 * 0.18)[..., None] * np.array([70, 90, 140])
    sky = np.clip(np.floor(sky / 7 + BAYER_FULL[..., None]) * 7, 0, 255)
    fr = Frame(sky)
    for sdir, b in zip(S.STARS, S.STAR_B):
        sp = cam.screen_of_dir(sdir)
        if sp is not None and 0 <= sp[0] < W and 0 <= sp[1] < H:
            tw = b * (0.75 + 0.25 * math.sin(t * 3.1 + b * 40))
            fr.draw.point([tuple(sp)], fill=tuple(int(c) for c in mixc(sky[int(sp[1]), int(sp[0])], (230, 236, 255), tw)))
    ms = cam.screen_of_dir(MOON)
    if ms is not None:
        fr.disc(ms, 0.021 * cam.f, (196, 206, 236), n=16, z=SKY_DEPTH)
        fr.disc(ms, 0.017 * cam.f, (246, 246, 236), n=16, z=SKY_DEPTH)
        fr.disc((ms[0] + 0.004 * cam.f, ms[1] - 0.003 * cam.f), 0.005 * cam.f, (222, 224, 220), n=8, z=SKY_DEPTH)
    # ---------------- ridges (layered haze) + far meadow trees
    for faces, col in S.RIDGES[::-1]:
        for Q, n in faces:
            zc = Q[:, 2].mean()
            kf = np.clip((-zc - 120) / 300, 0, 0.6)
            c = col * (1 - kf) + FOG * kf
            fr.poly(cam, Q, np.floor(c / 6) * 6)
    far_items = []
    for faces, col, name in S.FAR:
        c = np.mean([Q.mean(0) for Q, n in faces], axis=0)
        far_items.append((np.linalg.norm(c - cam.pos), faces, col))
    far_items.sort(key=lambda a: -a[0])
    for dist, faces, col in far_items:
        N = np.array([n for Q, n in faces])
        C = np.array([Q.mean(0) for Q, n in faces])
        cols = shade(L, N, C, col, kvis=np.ones((len(faces), 1)) * 0.7, trans=0.2)
        kf = min(0.8, max(0.0, (dist - 25) / 90))
        cols = cols * (1 - kf) + FOG[None] * kf
        for (Q, n), c in zip(faces, cols):
            if np.dot(n, cam.pos - Q.mean(0)) > 0:
                fr.poly(cam, Q, c)
    arr, depth = fr.array(), fr.depth()
    # ---------------- per-pixel leaf litter / creek / trail
    pos = cam.pos
    tg = np.where(d[..., 1] < -1e-6, -pos[1] / np.minimum(d[..., 1], -1e-6), np.inf)
    ground = np.isfinite(tg) & (tg < depth) & (tg < 900)
    ys, xs = np.nonzero(ground)
    if len(xs):
        tt = tg[ys, xs]
        X = pos[0] + tt * d[ys, xs, 0]
        Z = pos[2] + tt * d[ys, xs, 2]
        lit = 1 - GRID.sample(MOON_MASK, X, Z)
        I = np.tile(L.amb * 0.8 + L.sky, (len(X), 1))
        I += (lit * 0.62)[:, None] * L.keys[0][2][None]
        I += 0.45 * L.fills[0][1][None]
        for p in S.RED_LIGHTS:
            dd = np.sqrt((p[0] - X) ** 2 + p[1] ** 2 + (p[2] - Z) ** 2)
            I += (0.9 / (1 + (dd / 0.32) ** 2))[:, None] * np.array([1.0, 0.18, 0.12])[None]
        I *= contact_ao(X, Z, S.FEET)[:, None]
        alb = ground_albedo(X, Z, tt)
        col = alb * I
        fl = sample_grid(LG_N, lg_n, X, Z) + sample_grid(LG_V, lg_v, X, Z)
        col += np.clip(fl, 0, 0.75)[:, None] * FLASH[None] * 0.3 * (alb / 130.0)
        if lg_f is not None:
            col += np.clip(sample_grid(LG_N, lg_f, X, Z), 0, 1.0)[:, None] * FEM[None] * 0.55 * (alb / 120.0)
        # creek: dark water, moon reflection streak + twinkling glints
        cd = S.creek_dist(X, Z)
        wm = cd < S.CREEK_HW
        if wm.any():
            dw = d[ys[wm], xs[wm]]
            dw = dw / np.linalg.norm(dw, axis=1, keepdims=True)
            rr = dw * np.array([1, -1, 1])
            spec = np.clip(((rr * MOON).sum(1) - 0.94) / 0.06, 0, 1) ** 2
            gl = (hash2(np.floor(X[wm] / 0.07), np.floor(Z[wm] / 0.05), np.floor(t * 6)) > 0.93) * (0.4 + 0.8 * spec)
            edge = np.clip((S.CREEK_HW - cd[wm]) / 0.12, 0, 1)
            wc = WATER[None] * (0.8 + 0.4 * lit[wm, None]) + (spec * 150 + gl * 120)[:, None] * np.array([0.8, 0.88, 1.0])[None]
            wc += np.clip(fl[wm], 0, 1)[:, None] * FLASH[None] * 0.3
            col[wm] = col[wm] * (1 - edge[:, None]) + wc * edge[:, None]
        kf = np.clip((tt - 14.0) / 90.0, 0, 0.78)[:, None]
        col = col * (1 - kf) + FOG[None] * kf
        arr[ys, xs] = np.floor(col / 6 + BAYER_FULL[ys, xs, None]) * 6
        depth[ys, xs] = tt
    fr.set_array(arr, depth)
    # ---------------- objects, painter order
    items = [(sort_key(cam, o), o) for o in S.OBJS if onscreen(cam, o.lo, o.hi)]
    items.sort(key=lambda a: -a[0])
    flash_on = lg_n if lg_n.max() > 0.01 else None
    for _, o in items:
        draw_obj(fr, cam, L, o, t, flash_on)
    # ---------------- firefly bodies close to the lens (silhouettes) + the hero
    hz = cam.depth(M.hero(t))[0]
    body_ids = np.arange(0 if hz >= 2.2 else 1, 500)
    Pb = M.pos(body_ids, t)
    zb = cam.depth(Pb)
    close = np.nonzero((zb > cam.near * 2) & (zb < 2.6))[0]
    sb = cam.wp(Pb[close]) if len(close) else np.zeros((0, 2))
    for j, (sx, sy_) in zip(close, sb):
        if not (-10 < sx < W + 10 and -10 < sy_ < H + 10):
            continue
        i = body_ids[j]
        if M.ARR[i] > t and M.ARR[i] > 0 and Pb[j, 1] < 0.2:
            continue
        r = max(0.7, 0.0045 * cam.f / zb[j])
        fr.draw.ellipse([sx - r * 1.3, sy_ - r * 0.7, sx + r * 1.3, sy_ + r * 0.7], fill=(30, 28, 34))
        lc = mixc((150, 146, 110), (230, 255, 160), float(Im[i]))
        fr.draw.point([(sx - r * 0.9, sy_)], fill=tuple(int(c) for c in lc))
    hero_w = None
    if hz < 2.2 and hz > cam.near:
        hero_w = draw_hero(fr, cam, L, t, float(Im[0]))
    img, depth = fr.array(), fr.depth()
    if hero_w is not None and hz < 0.8:
        img = hind_wings(img, cam, t, *hero_w)
    # ---------------- lens: focus locked on the hero
    zf, sharp, K = focus(cam, t)
    coc = coc_map(depth, zf, K, sharp=sharp)
    img = dof(img, coc)
    # ---------------- flashes: glows when sharp, bokeh when not
    glows = []
    if len(on):
        zz = cam.depth(Pm)
        sc = cam.wp(Pm)
        for (sx, sy_), z, I0, i in zip(sc, zz, Ig, on):
            if z < cam.near * 2 or not (-30 < sx < W + 30 and -30 < sy_ < H + 30):
                continue
            ix, iy = int(min(W - 1, max(0, sx))), int(min(H - 1, max(0, sy_)))
            if 0 <= sx < W and 0 <= sy_ < H and z > depth[iy, ix] * 1.02 + 0.05 and coc[iy, ix] < 1.0:
                continue                                   # hidden behind a trunk / fern
            r_c = K * max(0.0, abs(1 - zf / z) - sharp)
            size = 0.0035 * cam.f / z
            if i == 0 and z < 0.6:
                continue                                   # the hero's lantern is drawn on his body
            if r_c > 1.6:
                r = min(30.0, r_c * 1.3 + size)
                add_bokeh(img, sx, sy_, r, FLASH * 0.95 + 10, min(1.0, 1.1 * I0), coc=coc, depth=depth, zb=z)
            else:
                if size < 0.1:
                    glows.append((sx, sy_, 2, tuple(FLASH), 0.26 * I0))
                    glows.append((sx, sy_, 1, tuple(FLASH_CORE), 0.8 * I0))
                else:
                    sn = min(1.0, size / 0.3)
                    glows.append((sx, sy_, min(16, 1.6 + 6.0 * size + 3.0 * I0 * sn), tuple(FLASH), (0.5 + 0.3 * sn) * I0))
                    glows.append((sx, sy_, max(1, min(5, 1 + 1.5 * size)), tuple(FLASH_CORE), (0.75 + 0.25 * sn) * I0))
    # the hero's lantern glow (close shots)
    if Im[0] > 0.02 and hz < 0.6:
        h, R = M.hero(t), hero_frame(t)
        lp = h + R[0] * -0.0052 + R[1] * -0.0012
        s_ = cam.wp(lp)
        glows.append((s_[0], s_[1], min(60, 0.016 * cam.f / hz), tuple(FLASH), 0.55 * Im[0]))
        glows.append((s_[0], s_[1], min(24, 0.005 * cam.f / hz), tuple(FLASH_CORE), 0.85 * Im[0]))
    # females: dim green answers in the leaf litter
    fo = np.nonzero(If > 0.02)[0]
    if len(fo):
        Pf = M.PF[fo]
        zz = cam.depth(Pf)
        sc = cam.wp(Pf)
        for (sx, sy_), z, I0 in zip(sc, zz, If[fo] ** 0.55):
            if z < 0.1 or not (0 <= sx < W and 0 <= sy_ < H):
                continue
            if z > depth[int(sy_), int(sx)] * 1.03 + 0.08:
                continue
            r_c = K * max(0.0, abs(1 - zf / z) - sharp)
            size = 0.003 * cam.f / z
            if r_c > 1.6:
                add_bokeh(img, sx, sy_, min(24, r_c), FEM, 0.5 * I0, coc=coc, depth=depth, zb=z)
            else:
                glows.append((sx, sy_, min(15, 3.4 + 5 * size + 2.0 * I0), tuple(FEM), 0.85 * I0))
                glows.append((sx, sy_, max(1, min(3, 1 + size)), (200, 255, 210), 0.95 * I0))
    # red-filtered trail lights (dim, pointed down)
    for p in S.RED_LIGHTS:
        z = cam.depth(p)[0]
        if z < 0.5:
            continue
        s_ = cam.wp(p)
        if 0 <= s_[0] < W and 0 <= s_[1] < H and z > depth[int(s_[1]), int(s_[0])] * 1.03 + 0.1:
            continue
        glows.append((s_[0], s_[1], max(2, min(9, 0.25 * cam.f / z)), tuple(RED), 0.45))
        glows.append((s_[0], s_[1], 1, (255, 170, 150), 0.7))
    if ms is not None:
        glows.append((ms[0], ms[1], min(40, 0.05 * cam.f), (150, 170, 230), 0.16))
    for g in glows:
        add_glow(img, *g)
    if ms is not None:
        img = god_rays(img, depth, ms, (140, 160, 220), 0.14)
    img = grade(bloom(np.clip(img, 0, 255), thr=196, k=0.38), contrast=0.2, sat=1.28)
    img = np.clip(img * vignette(0.8, 0.62, 0.3) * top_scrim(), 0, 255).astype(np.uint8)
    return draw_ui(img, t)


# ------------------------------------------------------------------ UI: captions + HUD
def captions(img, t):
    for t0, t1, lines, delays in T.CAPTIONS:
        if not (t0 <= t < t1):
            continue
        lens = [len(l.replace('*', '').replace('~', '')) for l in lines]
        scale = 3 if max(lens) <= 18 else 2
        y = 42 + int(round(6 * (1 - ease((t - t0) / 0.25))))
        fade = 1 - ss(t1 - 0.12, t1, t)
        for ln, pl, dl in zip(lines, lens, delays):
            u = t - t0 - dl
            if u >= 0:
                yy = y + (int(round(5 * (1 - ease(u / 0.22)))) if dl > 0 else 0)
                rich_line(img, ln, W // 2, yy, scale, CREAM, AMBER, GREEN, nchars=int(u * 55) + 1, alpha=fade)
            y += 7 * scale + 7


HUD_Y0 = 548
RASTER_IDS = None


def _raster_ids():
    global RASTER_IDS
    if RASTER_IDS is None:
        d = np.linalg.norm(M.P[1:20] - M.P[0], axis=1)
        RASTER_IDS = [0] + [int(1 + j) for j in np.argsort(d)[:7]]
    return RASTER_IDS


EV_BY_ID = {}


def _events(i):
    if i not in EV_BY_ID:
        EV_BY_ID[i] = M.EV_T[M.EV_I == i]
    return EV_BY_ID[i]


def draw_ui(img, t):
    out = img
    a_in = ease((t - T.HUD_IN) / 0.4) * (1 - ease((t - T.HUD_OUT) / 0.4))
    if a_in > 0.01:
        off = int(round((1 - a_in) * 110))
        y0 = HUD_Y0 + off
        pil = Image.fromarray(out)
        dr = ImageDraw.Draw(pil)
        panel(dr, 12, y0, 348, y0 + 78)
        # raster: one row per tracked fly, flashes scroll left; columns = sync
        x0r, x1r = 104, 338
        win = 5 * T.BEAT
        ids = _raster_ids()
        for r_, i in enumerate(ids):
            yy = y0 + 9 + r_ * 6
            dr.line([x0r, yy, x1r, yy], fill=(30, 38, 64))
        dr.line([x1r, y0 + 6, x1r, y0 + 54], fill=(70, 84, 120))
        out = np.asarray(pil).copy()
        for r_, i in enumerate(ids):
            yy = y0 + 9 + r_ * 6
            ev = _events(i)
            ev = ev[(ev <= t) & (ev > t - win)]
            for te in ev:
                x = int(round(x1r - (t - te) / win * (x1r - x0r)))
                age = t - te
                c = np.array(AMBER if i == 0 else FLASH, float) * (0.55 + 0.45 * math.exp(-age / 0.25))
                out[yy - 2:yy + 3, max(0, x - 1):x + 2] = np.clip(c, 0, 255).astype(np.uint8)
        label(out, 'YOU', 20, y0 + 6, AMBER, 1)
        label(out, 'NEIGHBOURS', 20, y0 + 16, (170, 220, 130), 1)
        # count of flies in sync
        n = M.sync_count(t)
        shown = n
        for tb, a, b in [(T.T_B1, 20, 100), (T.T_B2, 100, 400), (T.T_B3, 400, 3000)]:
            if tb - 0.6 <= t < tb + 0.6:
                shown = int(round(a + (b - a) * ease((t - tb + 0.6) / 1.0)))
        label(out, f'{shown}', 20, y0 + 30, MINT, 2)
        label(out, 'IN SYNC', 20, y0 + 48, MINT, 1)
        # bottom row: 8 flash pips during a burst, a filling bar in the dark
        ph, k, bb = T.phase(t)
        yb = y0 + 62
        if ph == 'burst':
            nlit = int(bb) + 1
            for p_ in range(8):
                x = 104 + p_ * 29
                lit = p_ < nlit
                fresh = lit and p_ == nlit - 1 and (bb % 1.0) < 0.35
                col = np.array(FLASH_CORE if fresh else (FLASH if lit else (44, 52, 80)), float)
                out[yb - 3:yb + 4, x:x + 22] = col.astype(np.uint8)
            label(out, f'{min(8, nlit)}/8', 28, yb - 3, CREAM, 1)
        else:
            dp = T.dark_progress(t)
            if dp is not None:
                out[yb - 2:yb + 3, 104:338] = (30, 36, 60)
                out[yb - 2:yb + 3, 104:104 + int(234 * dp)] = (84, 104, 168)
                label(out, 'DARK', 22, yb - 3, (150, 170, 230), 1)
            elif ph == 'neigh':
                label(out, 'FINDING', 22, yb - 3, (170, 220, 130), 1)
                label(out, 'THE BEAT', 104, yb - 3, (170, 220, 130), 1)
    captions(out, t)
    return out
