import os
"""Crickets Are Thermometers, diorama cut: 2.5D side view, near-static camera,
low-poly 3D objects, pixel upscale."""
import math, sys
import numpy as np
from PIL import Image, ImageDraw, ImageChops, ImageFilter
from scipy.signal import fftconvolve
import timeline as T
import dscene as D
from font import text_mask, stamp, rich_line, ADV

W, H = 360, 640
UP = 3
FPS = T.FPS
CREAM = (255, 246, 226)
MINT = (110, 255, 200)
CYAN = (90, 230, 255)
PINK = (255, 92, 190)
AMBER = (255, 210, 80)
INK = (12, 10, 30)
HERO_CHIRPS = [c[0] for c in T.CHIRPS]
BEATS = [c[0] for c in T.CHIRPS if c[1] in ('beat', 'cool', 'real')]
B = list(T.BAR_T)


def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


def mixc(a, b, u):
    return np.array(a, float) * (1 - u) + np.array(b, float) * u


def dirv(az, el):
    a, e = math.radians(az), math.radians(el)
    return np.array([math.sin(a) * math.cos(e), math.sin(e), -math.cos(a) * math.cos(e)])


MOON_AZ, MOON_EL = -4.2, 7.0
MOON_D = dirv(MOON_AZ, MOON_EL)
KEY_D = dirv(MOON_AZ, 12.0)                      # moonlight from the moon in frame (backlight, long shadows)
RIM_D = D.nrm(np.array([0.3, 0.45, -0.85]))      # moon rim from behind
FILL_D = D.nrm(np.array([0.55, 0.3, 0.78]))      # weak fill from front-right
CAST = []        # world polygons that cast shadows this frame
BLOBS = []       # contact shadows (x, z, r)


class Lights:
    amb = np.zeros(3); key = np.zeros(3); rim = np.zeros(3); fill = np.zeros(3); sky = np.zeros(3)
    points = []
    extra = []
    view = np.array([0, 0, 1.0])
    moon_perp = np.array([0, 1.0, 0])


LT = Lights()


class Pal:
    def __init__(self, t):
        h = T.heat(t)
        self.h = h
        self.zen = mixc((10, 9, 56), (58, 6, 74), h)
        self.hor = mixc((40, 134, 210), (250, 104, 74), h)
        self.hor2 = mixc((40, 52, 146), (170, 36, 112), h)
        self.amb = mixc((0.15, 0.17, 0.32), (0.32, 0.15, 0.27), h)
        self.key = mixc((1.25, 1.36, 1.65), (1.75, 1.08, 0.80), h)
        self.rim = mixc((0.60, 0.88, 1.20), (1.30, 0.62, 0.80), h)
        self.fill = mixc((0.16, 0.19, 0.34), (0.30, 0.14, 0.25), h)
        self.sky = mixc((0.16, 0.20, 0.38), (0.30, 0.13, 0.26), h)
        self.shadow = tuple(int(c) for c in mixc((3, 3, 12), (12, 2, 10), h))
        self.grass = mixc((34, 196, 150), (168, 226, 60), h)
        self.grass2 = mixc((28, 150, 190), (240, 160, 50), h)
        self.ring = tuple(int(c) for c in mixc((130, 255, 235), (255, 160, 235), h))
        self.moon = mixc((255, 248, 220), (255, 196, 130), h)
        self.ff = mixc((210, 255, 90), (255, 236, 100), h)
        self.soil = h


MASK = {}


def big_casters():
    out = []
    for name, faces, col, rim in D.BACK_SOLIDS:
        out += [Q for Q, n in faces]
    for name, faces, col, rim in D.SOLIDS:
        if name in ('post', 'postcap', 'fpost'):
            out += [Q for Q, n in faces]
    b = D.BOARD
    out.append(np.array([[b['x0'], b['y0'], b['z']], [b['x1'], b['y0'], b['z']], [b['x1'], b['y1'], b['z']], [b['x0'], b['y1'], b['z']]]))
    return out


def moon_mask():
    if not MASK:
        x0, z0, sc = -1000.0, -1600.0, 3.0
        Wm, Hm = int(2000 / sc), int(2600 / sc)
        im = Image.new('L', (Wm, Hm), 0)
        dr = ImageDraw.Draw(im)
        for Q in big_casters():
            Qg = to_plane(Q, 1, 0.0)
            dr.polygon([((p[0] - x0) / sc, (p[2] - z0) / sc) for p in Qg], fill=255)
        im = im.filter(ImageFilter.GaussianBlur(2.0))
        MASK.update(dict(a=np.asarray(im).astype(np.float32) / 255.0, x0=x0, z0=z0, sc=sc))
    return MASK


def lit_frac(pos):
    """1 = in moonlight, 0 = inside the long shadow of a tree / post / the thermometer board"""
    m = moon_mask()
    g = pos - (pos[1] / KEY_D[1]) * KEY_D
    i, j = int((g[2] - m['z0']) / m['sc']), int((g[0] - m['x0']) / m['sc'])
    a = m['a']
    if 0 <= i < a.shape[0] and 0 <= j < a.shape[1]:
        return 1.0 - float(a[i, j])
    return 1.0


def shade(P, n, base, rimk=0.6, pos=None, emit=0.0, trans=0.0, sm=True):
    n = np.asarray(n, float)
    lf = lit_frac(pos) if (pos is not None and sm) else 1.0
    I = LT.amb + LT.key * max(0.0, float(np.dot(n, KEY_D))) * lf + LT.fill * max(0.0, float(np.dot(n, FILL_D))) \
        + LT.sky * max(0.0, float(n[1])) + emit
    # backlight rim: silhouette edges that face the moon catch a thin bright line
    facing = abs(float(np.dot(n, LT.view)))
    side = max(0.0, float(np.dot(n, LT.moon_perp)))
    I = I + LT.rim * ((1 - facing) ** 3) * side * rimk * 1.6 * lf
    # thin leaves / blades glow when the moon shines through them
    if trans:
        I = I + LT.key * np.array([0.8, 1.0, 0.75]) * trans * lf * max(0.0, -float(np.dot(LT.view, KEY_D)))
    for d_, c_ in LT.extra:
        I = I + c_ * max(0.0, float(np.dot(n, d_)))
    if pos is not None:
        for lp, lc, R in LT.points:
            v = lp - pos
            dist = math.sqrt(float(v @ v)) + 1e-6
            if dist > 3.2 * R:
                continue
            lam = 0.35 + 0.65 * max(0.0, float(np.dot(n, v)) / dist)
            I = I + lc * (lam / (1 + (dist / R) ** 2))
    m = float(I.max())
    if m > 0.95:                       # soft highlight roll-off keeps colours from blowing out
        I = I * (0.95 + 0.6 * (1 - math.exp(-(m - 0.95) / 0.6))) / m
    return np.clip(np.round(np.array(base, float) * I / 4) * 4, 0, 255)


def thin_quad(a, b, w):
    a, b = np.asarray(a, float), np.asarray(b, float)
    d = b - a
    p = np.cross(d, [0, 0, 1.0])
    p = p / (np.linalg.norm(p) + 1e-9) * w / 2
    return np.array([a + p, b + p, b - p, a - p])


# ----------------------------------------------------------------- camera
class Cam:
    def __init__(self, tgt, d, yaw, pitch, fov, sy):
        yr, pr = math.radians(yaw), math.radians(pitch)
        self.pos = np.array(tgt, float) + d * np.array([math.sin(yr) * math.cos(pr), math.sin(pr), math.cos(yr) * math.cos(pr)])
        fwd = D.nrm(np.array(tgt, float) - self.pos)
        right = D.nrm(np.cross(fwd, [0, 1, 0]))
        up = np.cross(right, fwd)
        self.M = np.stack([right, up, fwd])
        self.f = (H / 2) / math.tan(math.radians(fov) / 2)
        self.cx, self.cy = W / 2, sy

    def to_cam(self, P):
        return (np.asarray(P, float) - self.pos) @ self.M.T

    def proj(self, Pc):
        z = np.maximum(Pc[..., 2], 1e-4)
        return np.stack([self.cx + self.f * Pc[..., 0] / z, self.cy - self.f * Pc[..., 1] / z], -1)

    def wp(self, P):
        return self.proj(self.to_cam(P))

    def ppu(self, P):
        return self.f / max(1e-3, self.to_cam(np.asarray(P, float)[None])[0, 2])


NEAR = 1.0


def proj_poly(cam, P):
    Pc = cam.to_cam(P)
    if (Pc[:, 2] < NEAR).any():
        return None, 0
    s = cam.proj(Pc)
    if s[:, 0].max() < -4 or s[:, 0].min() > W + 4 or s[:, 1].max() < -4 or s[:, 1].min() > H + 4:
        return None, 0
    return s, float(Pc[:, 2].mean())


CLOSE = ((-8.0, 114.0, 0.0), 16.5, 300)
WIDE = ((-4.0, 100.0, -30.0), 25.5, 330)
WING = ((-64.0, 138.0, 14.0), 12.0, 300)
PUSH = ((-14.0, 108.0, -12.0), 22.0, 322)


def lerp_frame(a, b, u):
    return (tuple(np.array(a[0]) + (np.array(b[0]) - np.array(a[0])) * u), a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u)


def framing(t):
    if t < T.T_MUSIC:
        return CLOSE
    if t < 11.6:
        return lerp_frame(CLOSE, WIDE, ease((t - T.T_MUSIC) / (11.6 - T.T_MUSIC)))
    if B[5] <= t < B[7]:
        return WING
    if B[20] <= t < T.T_COOL0:
        return lerp_frame(WIDE, PUSH, ease((t - B[20]) / (T.T_COOL0 - B[20])))
    if T.T_COOL0 <= t < T.T_COOL1:
        return lerp_frame(PUSH, WIDE, ease((t - T.T_COOL0) / (T.T_COOL1 - T.T_COOL0)))
    if T.T_COOL1 + 0.4 <= t < 57.0:
        return lerp_frame(WIDE, CLOSE, ease((t - T.T_COOL1 - 0.4) / (57.0 - T.T_COOL1 - 0.4)))
    if t >= 57.0:
        return CLOSE
    return WIDE


def cam_at(t):
    tgt, fov, sy = framing(t)
    yaw = 0.5 * math.sin(2 * math.pi * t / 60.0)       # gentle, loops
    if B[5] <= t < B[7]:
        yaw += -8.0
    pitch = 3.0 + 0.3 * math.sin(4 * math.pi * t / 60.0)
    bi = T.bar_at(t)
    if 0 <= bi < T.NBARS and t < T.T_COOL0:
        fov *= 1 - 0.012 * math.exp(-(t - T.BAR_T[bi]) / 0.09)
    return Cam(tgt, 1000.0, yaw, pitch, fov, sy)


# ----------------------------------------------------------------- sky & helpers
BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
BAYER_FULL = np.tile(BAYER, (H // 4, W // 4))
YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)
STAR_D = np.array([dirv(a, e) for a, e, *_ in D.STARS])
STAR_B = np.array([s[2] for s in D.STARS]); STAR_P = np.array([s[3] for s in D.STARS]); STAR_MW = np.array([s[4] for s in D.STARS])


def sky(cam, P, t):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2]) - math.radians(MOON_AZ)
    u = np.clip(1 - np.abs(az) / 0.6, 0, 1) ** 1.5
    hor = P.hor2[None, None] * (1 - u[..., None]) + P.hor[None, None] * u[..., None]
    k = np.clip(el / 0.32, 0, 1) ** 0.6
    col = hor * (1 - k[..., None]) + P.zen[None, None] * k[..., None]
    cosang = (d * MOON_D[None, None]).sum(-1)
    g = np.clip((cosang - 0.985) / 0.015, 0, 1) ** 2
    col += g[..., None] * P.moon[None, None] * 0.35
    col = np.floor(col / 8.0 + BAYER_FULL[..., None]) * 8.0
    return np.clip(col, 0, 255)


def screen_dir(cam, dv):
    pc = cam.M @ (dv * 1e6)
    if pc[2] <= 1:
        return None
    return cam.proj(pc[None])[0]


def draw_stars(img, cam, t, beat_twinkle):
    Pc = STAR_D @ cam.M.T
    vis = Pc[:, 2] > 0.05
    s = cam.proj(Pc[vis] * 1e5)
    b = STAR_B[vis] * (0.65 + 0.35 * np.sin(t * 3.3 + STAR_P[vis] * 7)) * (1 + 0.5 * beat_twinkle)
    mw = STAR_MW[vis]
    x, y = s[:, 0].astype(int), s[:, 1].astype(int)
    ok = (x >= 1) & (x < W - 1) & (y >= 1) & (y < H - 1)
    for xi, yi, bi, m in zip(x[ok], y[ok], b[ok], mw[ok]):
        c = np.clip((np.array([255, 170, 240]) * 0.8 if m else np.array([235, 240, 255])) * bi, 0, 255)
        img[yi, xi] = np.maximum(img[yi, xi], c)
        if bi > 0.75 and not m:
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                img[yi + dy, xi + dx] = np.maximum(img[yi + dy, xi + dx], c * 0.45)


def glow_kernel(r):
    yy, xx = np.mgrid[-r:r + 1, -r:r + 1]
    dd = np.hypot(xx, yy) / (r + 0.5)
    v = np.clip(1 - dd, 0, 1) ** 1.6
    b = np.tile(BAYER, ((2 * r + 1) // 4 + 1, (2 * r + 1) // 4 + 1))[:2 * r + 1, :2 * r + 1]
    return np.floor(v * 5 + b * 0.999) / 5


GLOW = {r: glow_kernel(r) for r in range(1, 91)}


def add_glow(img, x, y, r, col, inten):
    r = int(max(1, min(90, round(r))))
    k = GLOW[r] * inten
    x, y = int(round(x)), int(round(y))
    x0, y0, x1, y1 = x - r, y - r, x + r + 1, y + r + 1
    if x1 <= 0 or y1 <= 0 or x0 >= W or y0 >= H:
        return
    kx0, ky0 = max(0, -x0), max(0, -y0)
    kx1, ky1 = k.shape[1] - max(0, x1 - W), k.shape[0] - max(0, y1 - H)
    img[max(0, y0):min(H, y1), max(0, x0):min(W, x1)] += k[ky0:ky1, kx0:kx1, None] * np.array(col, float)[None, None]


def add_ring(img, x, y, r, width, col, inten):
    if r < 1 or inten <= 0.01:
        return
    x0, x1 = int(max(0, x - r - width - 1)), int(min(W, x + r + width + 2))
    y0, y1 = int(max(0, y - r - width - 1)), int(min(H, y + r + width + 2))
    if x1 <= x0 or y1 <= y0:
        return
    dd = np.hypot(XX[y0:y1, x0:x1] - x, YY[y0:y1, x0:x1] - y)
    m = (np.abs(dd - r) <= width / 2).astype(np.float32)
    img[y0:y1, x0:x1] += m[..., None] * np.array(col, float)[None, None] * inten


def last_beat(t, times):
    i = np.searchsorted(times, t, side='right') - 1
    return times[i] if i >= 0 else -9.0


def beat_env(t, decay=0.1):
    lb = last_beat(t, BEATS)
    return math.exp(-(t - lb) / decay) if t - lb < 1.0 else 0.0


def genre_now(t):
    b = T.bar_at(t)
    if b < 0 or t >= T.T_COOL0:
        return -1, b
    return T.BAR_GENRE[b], b


def chirp_env(t, times):
    lb = last_beat(t, times)
    return math.exp(-(t - lb) / 0.07) if 0 <= t - lb < 0.2 else 0.0


def merc_color(Tn):
    u = (Tn - 55) / 35
    stops = [(0.0, (70, 220, 255)), (0.35, (170, 120, 255)), (0.65, (255, 80, 180)), (1.0, (255, 70, 50))]
    for (a, ca), (b, cb) in zip(stops, stops[1:]):
        if u <= b:
            v = (max(u, a) - a) / (b - a)
            return tuple(int(x) for x in mixc(ca, cb, v))
    return stops[-1][1]


# ----------------------------------------------------------------- scene drawing
def add_faces(items, cam, P, faces, base, rim=0.6, fogk=0.0, bias=0.0, cull=True, tint=None, cast=False, lit=True, sm=True):
    for Q, n in faces:
        if cast:
            CAST.append(Q)
        if cull and np.dot(n, cam.pos - Q.mean(0)) <= 0:
            continue
        s, z = proj_poly(cam, Q)
        if s is None:
            continue
        c = shade(P, n, base, rim, Q.mean(0) if lit else None, sm=sm)
        if fogk == 'dist':
            fk = min(0.42, max(0.06, (-Q[:, 2].mean() - 500) / 9000))
            c = c * (1 - fk) + P.hor * fk
        if tint is not None:
            c = c * (1 - tint[1]) + np.array(tint[0]) * tint[1]
        if fogk and fogk != 'dist':
            c = c * (1 - fogk) + P.hor * fogk
        items.append((z + bias, 'poly', s, c))


def add_cricket(items, alpha_items, cam, P, cr, t, env, ph, bright=1.0):
    o = cr.build(t, env, ph)
    for Q, n, col in o['faces']:
        CAST.append(Q)
        if np.dot(n, cam.pos - Q.mean(0)) <= 0:
            continue
        s, z = proj_poly(cam, Q)
        if s is None:
            continue
        c = shade(P, n, col, 0.9, Q.mean(0), trans=0.05) * bright + np.array(P.ring) * 0.09 * env
        items.append((z, 'poly', s, np.clip(c, 0, 255)))
    for E in o['eyes']:
        s, z = proj_poly(cam, E)
        if s is not None:
            items.append((z - 0.05, 'poly', s, (40, 26, 60)))
    for a, b, w, sg in o['legs']:
        CAST.append(thin_quad(a, b, w))
        Pc = cam.to_cam(np.array([a, b]))
        if (Pc[:, 2] < NEAR).any():
            continue
        px = max(1, int(round(w * cam.f / Pc[:, 2].mean())))
        items.append((Pc[:, 2].mean(), 'line', cam.proj(Pc), tuple(shade(P, np.array([-0.5, 0.3, 0.8 * sg]), (176, 232, 140), 0.8, (a + b) / 2) * bright), px))
    for pts in o['ant']:
        Pc = cam.to_cam(np.array(pts))
        if (Pc[:, 2] < NEAR).any():
            continue
        items.append((Pc[:, 2].mean(), 'line', cam.proj(Pc), (220, 240, 176), 1))
    for Wp in o['wings']:
        CAST.append(Wp)
        s, z = proj_poly(cam, Wp)
        if s is None:
            continue
        alpha_items.append((z, s, mixc((140, 205, 190), P.ring, 0.3 + 0.5 * env), 0.34 + 0.34 * env))
    return o['ring_at']


def build_scene(cam, P, t, items, alpha_items, glows, rings):
    g, bar = genre_now(t)
    be = beat_env(t, 0.12)
    # ---------------- far ridges + ground plane
    for k, rid in enumerate(D.RIDGES):
        fk = [0.62, 0.48, 0.32][k]
        for Q, n, col in rid:
            s, z = proj_poly(cam, Q)
            if s is not None:
                c = shade(P, n, mixc(col, P.grass2, 0.35), 0.8) * (1 - fk) + P.hor * fk
                items.append((1e7 - k, 'poly', s, c))
    # ground: low-poly strips from the cut edge to the far field
    zs = [D.FRONT_Z, 0, -40, -90, -150, -230, -330, -480, -700, -1000, -1500, -2200, -3200, -4600, -6500, -9000, -12000]
    for i in range(len(zs) - 1):
        span = 260 + 0.9 * abs(zs[i + 1])
        for j in range(12):
            x0, x1 = -span + j * span / 6, -span + (j + 1) * span / 6
            jz = (i * 7 + j * 3) % 5
            Q = np.array([[x0, 0, zs[i]], [x1, 0, zs[i]], [x1, 0, zs[i + 1]], [x0, 0, zs[i + 1]]])
            s, z = proj_poly(cam, Q)
            if s is None:
                continue
            base = mixc(P.grass, P.grass2, 0.25 + 0.12 * jz) * (0.62 + 0.04 * jz)
            fk = min(0.75, max(0.0, (-zs[i + 1] - 400) / 7000))
            vd = D.nrm(Q.mean(0) - cam.pos)
            sheen = 0.40 + 0.45 * max(0.0, float(np.dot(vd, D.nrm(KEY_D * [1, 0, 1])))) ** 30
            c = np.clip(np.array(base) * (P.amb + P.sky * 0.4 + P.key * sheen), 0, 255) * (1 - fk) + mixc(P.hor2, P.hor, 0.4) * fk
            items.append((5e6 - i, 'poly', s, c))
    # ---------------- background farm
    for name, faces, col, rim in D.BACK_SOLIDS:
        add_faces(items, cam, P, faces, col, rim, fogk='dist', lit=False, cast=True)
    for Q in D.HOUSE_WINDOWS:
        s, z = proj_poly(cam, Q)
        if s is not None:
            items.append((z - 1, 'poly', s, (255, 220, 110)))
    windpump(cam, P, t, items, g)
    # ---------------- pond, cattails, frog
    s, z = proj_poly(cam, D.POND_POLY)
    if s is not None:
        items.append((z + 30, 'poly', s, mixc(P.zen, P.hor, 0.6) + np.array([0, 15, 30])))
        for (px, pz, pr) in D.PADS:
            pad = np.array([[px + pr * math.cos(a), 1.2, pz + pr * 0.45 * math.sin(a)] for a in np.linspace(0.35, 2 * math.pi, 9)])
            s2, z2 = proj_poly(cam, pad)
            if s2 is not None:
                items.append((z2 + 29, 'poly', s2, shade(P, np.array([0, 1.0, 0]), (70, 210, 100))))
        mg = cam.wp(np.array([[D.POND[0] - 10, 1, D.POND[2] + 8]]))[0]
        glows.append((mg[0], mg[1], 7, tuple(P.moon), 0.5))
    for (cx, cz, hgt) in D.CATTAILS:
        sw = 2 * math.sin(t * 1.3 + cx)
        st = cam.wp(np.array([[cx, 0, cz], [cx + sw, hgt, cz]]))
        z = cam.to_cam(np.array([[cx, hgt / 2, cz]]))[0, 2]
        items.append((z, 'line', st, tuple(shade(P, np.array([-0.6, 0, 0.8]), (80, 170, 90), 0.6, np.array([cx, hgt / 2, cz]))), 2))
        CAST.append(thin_quad([cx, 0, cz], [cx + sw, hgt, cz], 2.0))
        CAST.append(thin_quad([cx + sw * 0.9, hgt * 0.78, cz], [cx + sw, hgt * 0.95, cz], 5.0))
        BLOBS.append((cx, cz, 5))
        head = cam.wp(np.array([[cx + sw * 0.9, hgt * 0.78, cz], [cx + sw, hgt * 0.95, cz]]))
        items.append((z - 0.1, 'line', head, (120, 70, 60), 4))
    frog(items, cam, P, t, g, bar)
    # ---------------- fence + owl
    for name, faces, col, rim in D.SOLIDS:
        add_faces(items, cam, P, faces, col, rim, cast=(name not in ('rail',)), fogk=(0.1 if name in ('fpost', 'rail') else 0.0),
                  sm=(name not in ('post', 'postcap', 'fpost')))
    for px in D.FENCE_POSTS:
        BLOBS.append((px, D.FENCE_Z, 9))
    BLOBS.append((D.POST_X, D.POST_Z, 12))
    for (mx, mz, r, hh) in D.MUSHROOMS:
        BLOBS.append((mx, mz, r * 1.2))
    BLOBS.append((D.FROG[0], D.FROG[2], 14))
    # haze layers between depth planes
    for (zq, xs, yh, a) in ((-430, 900, 60, 0.16), (-1700, 2600, 190, 0.22), (-4300, 6500, 420, 0.24)):
        Qm = np.array([[-xs, 0, zq], [xs, 0, zq], [xs, yh, zq], [-xs, yh, zq]])
        sm, zm = proj_poly(cam, Qm)
        if sm is not None:
            sm = sm.copy()
            alpha_items.append((zm, sm, mixc(P.hor2, P.hor, 0.55), a))
    owl(items, cam, P, t, g, glows)
    # ---------------- meadow details
    pebbles(items, cam, P)
    blades(items, cam, P, t, g)
    flowers(items, cam, P, t, g, glows)
    # chorus crickets on their stems
    offs = chorus_offsets(t)
    bl = 1.0 / T.chirp_rate(T.temp(t))
    for k, cr in enumerate(D.CHORUS_CR):
        x, z, h = D.CHORUS[k]
        st = cam.wp(np.array([[x - 2, 0, z], [x + 1, h * 0.55, z], [x, h - 0.4 * cr.s, z]]))
        zz = cam.to_cam(np.array([[x, h / 2, z]]))[0, 2]
        items.append((zz + 0.5, 'line', st, tuple(shade(P, np.array([-0.6, 0, 0.8]), (90, 200, 110), 0.6, np.array([x, h / 2, z]))), max(1, int(2.0 * cam.f / zz))))
        CAST.append(thin_quad([x - 2, 0, z], [x, h - 0.4 * cr.s, z], 2.0))
        BLOBS.append((x, z, 6))
        lv = np.array([[x - 12, h - 0.42 * cr.s - 1, z], [x + 12, h - 0.42 * cr.s - 1, z + 4], [x + 14, h - 0.42 * cr.s + 1, z - 3]])
        sl, zl = proj_poly(cam, lv)
        if sl is not None:
            items.append((zz + 0.4, 'poly', sl, shade(P, np.array([-0.3, 1.0, 0.2]), (70, 214, 110), 0.6, lv.mean(0), trans=0.2)))
        CAST.append(lv)
        tk = t - offs[k] * bl
        env = chirp_env(tk, BEATS)
        ra = add_cricket(items, alpha_items, cam, P, cr, t, env, k * 1.3, 0.92)
        lb = last_beat(tk, BEATS)
        rings.append((ra, tk - lb, bl, 0.55))
    # ---------------- hero plant + cricket
    for a, b in zip(D.STEM, D.STEM[1:]):
        Pc = cam.to_cam(np.array([a, b]))
        CAST.append(thin_quad(a, b, 3.2))
        items.append((Pc[:, 2].mean() + 1, 'line', cam.proj(Pc), tuple(shade(P, np.array([-0.7, 0.1, 0.7]), (96, 200, 100), 0.7, (a + b) / 2)), max(1, int(3.2 * cam.f / Pc[:, 2].mean()))))
    BLOBS.append((D.STEM_X, D.STEM_Z, 8))
    for (base, sg, ln) in D.SMALL_LEAVES:
        Q = np.array([base, base + [sg * ln * 0.5, 6, 6], base + [sg * ln, 2, 0], base + [sg * ln * 0.5, -2, -5]])
        CAST.append(Q)
        s, z = proj_poly(cam, Q)
        if s is not None:
            items.append((z, 'poly', s, shade(P, np.array([-0.3, 0.6, 0.8]), (70, 210, 104), 0.6, Q.mean(0), trans=0.25)))
    CAST.append(np.array([D.UMBEL + [11 * math.cos(a), 11 * 0.6 * math.sin(a), 0] for a in np.linspace(0, 2 * math.pi, 9)[:-1]]))
    u = cam.wp(D.UMBEL[None])[0]
    items.append((cam.to_cam(D.UMBEL[None])[0, 2], 'flower', (u[0], u[1], 11 * cam.f / cam.to_cam(D.UMBEL[None])[0, 2], (250, 250, 255), (255, 236, 190), 0, 1.0), None))
    hz = cam.to_cam(D.HERO.o[None])[0, 2]
    for (Q, n), i, sg in D.LEAF:
        CAST.append(Q)
        s, z = proj_poly(cam, Q)
        if s is None:
            continue
        col = shade(P, n, (74, 224, 112) if (i + (sg > 0)) % 2 else (60, 204, 104), 0.7, Q.mean(0), trans=0.16)
        items.append((hz + 30 + 0.01 * z, 'poly', s, col))
    env = chirp_env(t, HERO_CHIRPS)
    ra = add_cricket(items, alpha_items, cam, P, D.HERO, t, env, 0.0)
    lb = last_beat(t, HERO_CHIRPS)
    blh = 0.25 if T.T_FF0 <= t < T.T_FF1 else (1.0 if t < T.T_MUSIC else bl)
    rings.append((ra, t - lb, blh, 1.0))
    # ---------------- thermometer
    thermometer(items, cam, P, t, glows)
    # ---------------- soil cross-section (front cut face)
    soil(items, cam, P, t)
    return ra


BOARD_RECV = []
BOARD_DEPTH = [0.0]


def fence_receivers(cam):
    out = []
    dmin = 1e9
    for name, faces, col, rim in D.SOLIDS:
        if name not in ('rail', 'fpost'):
            continue
        for Q, n in faces:
            if n[2] > 0.5:
                s, z = proj_poly(cam, Q)
                if s is not None:
                    out.append(s)
                    dmin = min(dmin, z)
    return out, dmin


def to_plane(Q, axis, val):
    tq = (Q[:, axis] - val) / KEY_D[axis]
    return Q - tq[:, None] * KEY_D[None]


def clip_z(Q, zmax):
    out = []
    n = len(Q)
    for k in range(n):
        a, b = Q[k], Q[(k + 1) % n]
        ia, ib = a[2] <= zmax, b[2] <= zmax
        if ia:
            out.append(a)
        if ia != ib:
            u = (zmax - a[2]) / (b[2] - a[2])
            out.append(a + (b - a) * u)
    return np.array(out) if len(out) >= 3 else None


def build_shadows(cam, P, items):
    a = 0.66
    gb = np.array(mixc(P.grass, P.grass2, 0.45)) * 0.66
    col = tuple(int(c) for c in np.clip(gb * (P.amb + P.sky * 0.6) * 0.8, 0, 255))
    if os.environ.get('DBG_SH'):
        col, a = (255, 0, 0), 0.9
    # ground: everything casts a long moon shadow toward the camera
    polys = []
    for Q in CAST:
        if Q[:, 1].max() < 0.6:
            continue
        Qg = to_plane(np.maximum(Q, [-1e9, 0.0, -1e9]), 1, 0.15)
        Qg = clip_z(Qg, D.FRONT_Z)
        if Qg is None:
            continue
        s, z = proj_poly(cam, Qg)
        if s is not None:
            polys.append(s)
    for (x, z, r) in BLOBS:
        E = np.array([[x + 1.4 * r * math.cos(q), 0.2, z + 0.9 * r * math.sin(q)] for q in np.linspace(0, 2 * math.pi, 11)[:-1]])
        s, zz = proj_poly(cam, E)
        if s is not None:
            polys.append(s)
    items.append((4.9e6, 'shadow', (None, polys, a, col), None))
    return
    # (board / fence receivers are unused: the backlit moon cannot reach their front faces)
    if BOARD_RECV:
        z0 = D.BOARD['z'] + 0.2
        bp = []
        for Q in CAST:
            if Q[:, 2].mean() < z0 + 1.0:
                continue
            Qb = to_plane(Q, 2, z0)
            if Qb[:, 0].max() < D.BOARD['x0'] - 2 or Qb[:, 0].min() > D.BOARD['x1'] + 2 or Qb[:, 1].max() < D.BOARD['y0'] or Qb[:, 1].min() > D.BOARD['y1'] + 12:
                continue
            s, zz = proj_poly(cam, Qb)
            if s is not None:
                bp.append(s)
        items.append((BOARD_DEPTH[0] - 0.9, 'shadow', (list(BOARD_RECV), bp, 0.42, col), None))
    # fence
    recv, dmin = fence_receivers(cam)
    if recv:
        z0 = D.FENCE_Z + 5.0
        fp = []
        for Q in CAST:
            zc = Q[:, 2].mean()
            if zc < z0 + 3 or zc > z0 + 160:
                continue
            Qf = to_plane(Q, 2, z0)
            s, zz = proj_poly(cam, Qf)
            if s is not None:
                fp.append(s)
        items.append((dmin - 0.05, 'shadow', (recv, fp, 0.5, col), None))


def chorus_offsets(t):
    k = 1 - ss(B[7] - 0.5, B[8] + 0.5, t)
    return D.CHORUS_OFF * (0.06 + 0.94 * k)


def blades(items, cam, P, t, g):
    be = beat_env(t, 0.14) if g >= 3 else 0.0
    for (x, z, h, w, ang, lean, tone, ph) in D.BLADES:
        sw = (math.sin(t * 1.3 + ph) * 0.05 + 0.08 * be * math.sin(ph * 5)) * h
        dx, dz = math.cos(ang) * w, math.sin(ang) * w * 0.4
        Q = np.array([[x - dx, 0, z - dz], [x + dx, 0, z + dz], [x + lean * h * 0.4 + sw * 0.4 + dx * 0.5, h * 0.55, z],
                      [x + lean * h + sw, h, z], [x + lean * h * 0.4 + sw * 0.4 - dx * 0.5, h * 0.55, z]])
        s, zz = proj_poly(cam, Q)
        if s is None:
            continue
        n = D.nrm(np.array([-dz - 0.35, 0.25, dx + 0.6]))
        base = mixc(P.grass, P.grass2, 0.5 + 0.5 * math.sin(ph * 3)) * tone
        c = shade(P, n, base, 0.55, Q.mean(0), trans=0.3)
        if z > 15:                      # dark foreground layer
            c = c * 0.5 + P.zen * 0.12
        elif z < -150:                  # hazier far layer
            c = c * 0.82 + P.hor * 0.18
        items.append((zz, 'poly', s, c))


def flowers(items, cam, P, t, g, glows):
    be = beat_env(t, 0.13) if g >= 3 else 0.0
    for (x, y, z, r, kind) in D.FLOWERS:
        kind = int(kind)
        st = cam.wp(np.array([[x, 0, z], [x + 1, y, z]]))
        zc = cam.to_cam(np.array([[x, y, z]]))[0, 2]
        hp_ = np.array([x + 1, y, z])
        items.append((zc + 0.3, 'line', st, tuple(shade(P, np.array([-0.6, 0, 0.8]), (80, 180, 90), 0.5, hp_)), 1))
        BLOBS.append((x, z, 3))
        rr = r * cam.f / zc * (1 + 0.28 * be)
        col = [(255, 252, 245), (255, 80, 196), (100, 170, 255), (255, 250, 230)][kind]
        cen = [(255, 210, 40), (255, 170, 230), (220, 240, 255), (255, 255, 255)][kind]
        lit = shade(P, np.array([-0.55, 0.3, 0.78]), np.array(col), 0.6, hp_, trans=0.2) * 1.15
        lcen = tuple(int(c) for c in np.clip(np.array(cen) * (0.55 + 0.45 * lit / (np.array(col) + 1)), 0, 255))
        if z > 15:
            lit = lit * 0.6
        p = cam.wp(np.array([[x + 1, y, z]]))[0]
        items.append((zc, 'flower', (p[0], p[1], rr, tuple(np.clip(lit, 0, 255)), tuple(lcen), kind, 1.0 + 0.4 * be, True), None))
        if g >= 3 and be > 0.05:
            glows.append((p[0], p[1], rr * 2.2, col, 0.28 * be))


def pebbles(items, cam, P):
    for (x, z, r) in D.PEBBLES:
        Q = np.array([[x - r, 0.3, z], [x - r * 0.5, r * 0.6, z + 0.5], [x + r * 0.6, r * 0.5, z + 0.5], [x + r, 0.3, z]])
        s, zz = proj_poly(cam, Q)
        if s is not None:
            items.append((zz, 'poly', s, shade(P, np.array([0, 0.6, 0.8]), (176, 170, 200))))


def thermometer(items, cam, P, t, glows):
    b = D.BOARD
    z = b['z']
    Q = np.array([[b['x0'], b['y0'], z], [b['x1'], b['y0'], z], [b['x1'], b['y1'], z], [b['x0'], b['y1'], z]])
    s, zz = proj_poly(cam, Q)
    if s is None:
        return
    BOARD_RECV.clear()
    BOARD_RECV.append(s)
    BOARD_DEPTH[0] = zz
    CAST.append(Q)
    items.append((zz - 0.5, 'poly', s, shade(P, D.nrm(np.array([0, 0.05, 1.0])), (246, 240, 226), 0.2, Q.mean(0) + [0, 0, 1], emit=0.2)))
    # rounded top cap
    cap = np.array([[b['x0'] + (b['x1'] - b['x0']) * (0.5 + 0.5 * math.cos(a)), b['y1'] + 10 * math.sin(a), z] for a in np.linspace(0, math.pi, 9)])
    s2, _ = proj_poly(cam, cap)
    if s2 is not None:
        BOARD_RECV.append(s2)
        items.append((zz - 0.5, 'poly', s2, shade(P, np.array([0, 0.05, 1.0]), (246, 240, 226), 0.2, cap.mean(0), emit=0.2)))
    xc = (b['x0'] + b['x1']) / 2 - 4
    Tn = T.temp(t)
    mc = merc_color(Tn)
    yb = b['y0'] + 12
    tube = np.array([[xc - 3, yb, z + 0.3], [xc + 3, yb, z + 0.3], [xc + 3, b['y1'] - 6, z + 0.3], [xc - 3, b['y1'] - 6, z + 0.3]])
    s3, _ = proj_poly(cam, tube)
    if s3 is not None:
        items.append((zz - 0.6, 'poly', s3, (214, 222, 236)))
    lvl = D.thermo_y(Tn)
    merc = np.array([[xc - 1.8, yb, z + 0.5], [xc + 1.8, yb, z + 0.5], [xc + 1.8, lvl, z + 0.5], [xc - 1.8, lvl, z + 0.5]])
    s4, _ = proj_poly(cam, merc)
    if s4 is not None:
        items.append((zz - 0.7, 'poly', s4, mc))
    bulb = np.array([[xc + 7 * math.cos(a), yb + 7 * math.sin(a), z + 0.6] for a in np.linspace(0, 2 * math.pi, 11)[:-1]])
    s5, _ = proj_poly(cam, bulb)
    if s5 is not None:
        items.append((zz - 0.8, 'poly', s5, mc))
        c = s5.mean(0)
        glows.append((c[0], c[1], (s5[:, 0].max() - s5[:, 0].min()) * 1.6, mc, 0.45))
    for Tm in range(40, 101, 5):
        y = D.thermo_y(Tm)
        L = 9 if Tm % 10 == 0 else 5
        tick = cam.wp(np.array([[xc + 4, y, z + 0.3], [xc + 4 + L, y, z + 0.3]]))
        items.append((zz - 0.65, 'line', tick, (60, 50, 90), 1))
    # lantern glow
    lp_ = cam.wp(D.LANTERN[None])[0]
    flick = 0.85 + 0.15 * math.sin(t * 13) * math.sin(t * 7.7)
    items.append((cam.to_cam(D.LANTERN[None])[0, 2] - 6, 'poly',
                  cam.wp(np.array([D.LANTERN + [-3.5, -5, 5.1], D.LANTERN + [3.5, -5, 5.1], D.LANTERN + [3.5, 5, 5.1], D.LANTERN + [-3.5, 5, 5.1]])),
                  (255, 220, 120)))
    glows.append((lp_[0], lp_[1], 26 * cam.ppu(D.LANTERN) / 1.4, (255, 170, 70), 0.45 * flick))
    for k in range(3):
        a = t * (3.1 + k * 0.7) + k * 2.1
        mp = D.LANTERN + np.array([16 * math.cos(a), 6 * math.sin(a * 1.7), 12 * math.sin(a)])
        q = cam.wp(mp[None])[0]
        flap = 1 + (int(t * 24 + k) % 2)
        items.append((cam.to_cam(mp[None])[0, 2], 'line', np.array([[q[0] - flap, q[1] - 1], [q[0], q[1]], [q[0] + flap, q[1] - 1]]), (255, 246, 220), 1))


def thermo_labels(img, cam, t):
    b = D.BOARD
    xc = (b['x0'] + b['x1']) / 2 - 4
    ppu = cam.ppu(np.array([xc, 100, b['z']]))
    sc = 2 if ppu > 2.0 else 1
    for Tm in range(40, 101, 10):
        p = cam.wp(np.array([[xc + 15, D.thermo_y(Tm), b['z']]]))[0]
        stamp(img, text_mask(str(Tm), sc), int(p[0]), int(p[1] - 3.5 * sc), (26, 18, 44), outline=None)
    p = cam.wp(np.array([[xc - 2, b['y1'] - 2, b['z']]]))[0]
    stamp(img, text_mask("°F", sc), int(p[0] - 5 * sc), int(p[1]), (200, 60, 90), outline=None)


def windpump(cam, P, t, items, g):
    base = D.PUMP
    top = base + np.array([0, 520, 0])
    z = cam.to_cam(base[None])[0, 2]
    col = tuple(int(c) for c in mixc(P.zen, P.hor, 0.45) * 0.8)
    legs = [base + np.array([sx * 70, 0, sz * 70]) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    tops = [top + np.array([sx * 12, 0, sz * 12]) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    for a, b in zip(legs, tops):
        items.append((z, 'line', cam.wp(np.array([a, b])), col, 1))
    for lv in (0.3, 0.6):
        ring = [a + (b - a) * lv for a, b in zip(legs, tops)]
        items.append((z, 'line', cam.wp(np.array(ring + [ring[0]])), col, 1))
    hub = top + np.array([0, 18, 14])
    spd = 1.2 + (6.0 if g >= 5 else 0.0) + 2 * T.heat(t)
    ang0 = t * spd
    for k in range(16):
        a = ang0 + 2 * math.pi * k / 16
        p1 = hub + np.array([math.cos(a) * 18, math.sin(a) * 18, 0])
        p2 = hub + np.array([math.cos(a + 0.13) * 92, math.sin(a + 0.13) * 92, 0])
        p3 = hub + np.array([math.cos(a - 0.04) * 92, math.sin(a - 0.04) * 92, 0])
        items.append((z - 3, 'poly', cam.wp(np.array([p1, p2, p3])), mixc(col, (240, 240, 255), 0.3 + 0.15 * (k % 2))))
    vane = np.array([hub + [0, 0, -12], hub + [0, 22, -170], hub + [0, -22, -170]])
    items.append((z - 2, 'poly', cam.wp(vane), mixc(col, (230, 70, 90), 0.5)))


def owl(items, cam, P, t, g, glows):
    base = D.OWL_POST
    bob = 0.0
    if g >= 1:
        lb = last_beat(t, BEATS)
        bob = -3.0 * math.exp(-(t - lb) / 0.1)
    body = D.icosa(base + np.array([0, 18, 0]), 15, (0.95, 1.15, 0.8), jit=0.0)
    add_faces(items, cam, P, body, (150, 110, 170), 0.9)
    hc = base + np.array([0, 40 + bob, 0])
    head = D.icosa(hc, 12.5, (1.1, 0.95, 0.9), jit=0.0)
    add_faces(items, cam, P, head, (166, 124, 186), 0.9, bias=-0.5)
    for sg in (1, -1):
        ear = np.array([hc + [sg * 6, 9, 2], hc + [sg * 12, 20, 0], hc + [sg * 10, 8, 2]])
        s, z = proj_poly(cam, ear)
        if s is not None:
            items.append((z - 1, 'poly', s, shade(P, np.array([0, 0.3, 1.0]), (140, 100, 160))))
    face = np.array([hc + [-11, 6, 10], hc + [11, 6, 10], hc + [11, -8, 10], hc + [-11, -8, 10]])
    s, zf = proj_poly(cam, face)
    if s is not None:
        items.append((zf - 1, 'poly', s, shade(P, np.array([0, 0, 1.0]), (236, 214, 230))))
    blink = (int(t * 10) % 37 == 0)
    for sg in (1, -1):
        ec = hc + np.array([sg * 5.2, -0.5, 10.6])
        p = cam.wp(ec[None])[0]
        r = 4.4 * cam.ppu(ec)
        if blink:
            items.append((zf - 2, 'line', np.array([[p[0] - r, p[1]], [p[0] + r, p[1]]]), (60, 40, 70), 1))
        else:
            items.append((zf - 2, 'flower', (p[0], p[1], r, (255, 220, 60), (20, 14, 30), 9, 1.0), None))
            glows.append((p[0], p[1], r * 2.2, (255, 220, 80), 0.25))
    beak = np.array([hc + [-2, -3, 11], hc + [2, -3, 11], hc + [0, -8, 11.5]])
    s, z = proj_poly(cam, beak)
    if s is not None:
        items.append((zf - 2.5, 'poly', s, (255, 170, 60)))
    for sg in (1, -1):
        wing = np.array([base + [sg * 12, 30, 2], base + [sg * 16, 14, 3], base + [sg * 9, 2, 4]])
        s, z = proj_poly(cam, wing)
        if s is not None:
            items.append((z - 1, 'poly', s, shade(P, np.array([sg * 0.5, 0.2, 0.85]), (120, 84, 146))))


def frog(items, cam, P, t, g, bar):
    c = D.FROG.copy()
    hop = 0.0
    if bar >= 0 and t < T.T_COOL0 and (bar == 0 or T.BAR_GENRE[bar] != T.BAR_GENRE[bar - 1]):
        u = (t - T.BAR_T[bar]) / 0.45
        if 0 <= u < 1:
            hop = 14 * 4 * u * (1 - u)
    c = c + np.array([0, hop, 0])
    body = D.icosa(c + np.array([0, 9, 0]), 13, (1.25, 0.72, 1.0), jit=0.0)
    add_faces(items, cam, P, body, (90, 220, 90), 0.9)
    for sg in (1, -1):
        eye = D.icosa(c + np.array([sg * 7, 18, 6]), 4.6, (1, 1, 1), jit=0.0)
        add_faces(items, cam, P, eye, (120, 240, 110), 0.9, bias=-0.3)
        p = cam.wp((c + np.array([sg * 7, 18.5, 10.5]))[None])[0]
        r = 1.9 * cam.ppu(c)
        items.append((cam.to_cam(c[None])[0, 2] - 4, 'flower', (p[0], p[1], max(0.8, r), (20, 20, 30), (20, 20, 30), 9, 1.0), None))
    sac = 0.0
    if g >= 2:
        lb = last_beat(t, BEATS)
        idx = BEATS.index(lb) if lb in BEATS else 0
        if idx % 2 == 1:
            sac = math.exp(-(t - lb) / 0.16)
    if sac > 0.05:
        sc = c + np.array([0, 6, 12])
        p = cam.wp(sc[None])[0]
        r = (3.5 + 6.5 * sac) * cam.ppu(sc)
        items.append((cam.to_cam(sc[None])[0, 2] - 5, 'flower', (p[0], p[1], r, (255, 210, 230), (255, 230, 240), 9, 1.0), None))


def soil(items, cam, P, t):
    zf = D.FRONT_Z
    h = P.h
    for (y0, y1, col) in D.LAYERS:
        xs = np.linspace(-420, 420, 15)
        top = [np.array([x, y0 + (2.5 * math.sin(x * 0.05 + y0) if y0 < 0 else 0), zf]) for x in xs]
        bot = [np.array([x, y1 + 2.5 * math.sin(x * 0.05 + y1), zf]) for x in xs]
        for i in range(len(xs) - 1):
            Q = np.array([top[i], top[i + 1], bot[i + 1], bot[i]])
            s, z = proj_poly(cam, Q)
            if s is None:
                continue
            c = mixc(col, mixc(col, (200, 80, 60), 0.5), h) * (0.94 + 0.06 * ((i * 3) % 4) / 3)
            depthk = 1.0 - 0.45 * min(1.0, -y1 / 300.0)
            items.append((-1e5 + 2, 'poly', s, shade(P, np.array([-0.35, 0.1, 0.93]), c, 0.1) * 1.25 * depthk))
    edge = cam.wp(np.array([[-420, 0.2, zf + 0.2], [420, 0.2, zf + 0.2]]))
    items.append((-1e5 - 4, 'line', edge, tuple(np.clip(np.array(P.grass) * 1.3 + 30, 0, 255)), 2))
    for pts in D.STONES:
        Q = np.array([[x, y, zf + 0.3] for x, y in pts])
        s, z = proj_poly(cam, Q)
        if s is not None:
            items.append((-1e5 + 1, 'poly', s, shade(P, np.array([0, 0.3, 1.0]), (196, 170, 200), 0.2)))
    for pts in D.ROOTS:
        Q = np.array([[x, y, zf + 0.4] for x, y in pts])
        Pc = cam.to_cam(Q)
        if (Pc[:, 2] > NEAR).all():
            items.append((-1e5, 'line', cam.proj(Pc), (110, 88, 76), max(1, int(1.6 * cam.f / Pc[:, 2].mean()))))
    # ant tunnel + ants
    tun = [(-260 + 30 * k, -118 + 9 * math.sin(k * 0.8)) for k in range(18)]
    Q = np.array([[x, y, zf + 0.5] for x, y in tun])
    Pc = cam.to_cam(Q)
    if (Pc[:, 2] > NEAR).all():
        sp = cam.proj(Pc)
        items.append((-1e5 - 1, 'line', sp, (60, 36, 50), max(2, int(5 * cam.f / Pc[:, 2].mean()))))
        L = len(tun) - 1
        for k in range(9):
            u = ((t * 0.05 + k / 9) % 1.0) * L
            i = int(u); fr = u - i
            p = sp[i] * (1 - fr) + sp[min(L, i + 1)] * fr
            items.append((-1e5 - 2, 'flower', (p[0], p[1], max(1.0, 1.5 * cam.f / Pc[:, 2].mean()), (30, 16, 24), (200, 60, 60), 9, 1.0), None))
    # worm
    wpts = []
    x0 = -60 + 25 * math.sin(t * 0.25)
    for k in range(12):
        wpts.append([x0 + k * 4.6, -92 + 3 * math.sin(k * 0.9 - t * 3), zf + 0.6])
    Pc = cam.to_cam(np.array(wpts))
    if (Pc[:, 2] > NEAR).all():
        items.append((-1e5 - 3, 'line', cam.proj(Pc), (255, 150, 170), max(2, int(3.6 * cam.f / Pc[:, 2].mean()))))


# ----------------------------------------------------------------- UI
def box(draw, x0, y0, x1, y1, edge=(236, 232, 255), fill=(10, 8, 28)):
    draw.rectangle([x0 + 3, y0 + 3, x1 + 3, y1 + 3], fill=(4, 2, 12))
    draw.rectangle([x0, y0, x1, y1], fill=fill)
    draw.rectangle([x0, y0, x1, y1], outline=edge)
    draw.rectangle([x0 + 2, y0 + 2, x1 - 2, y1 - 2], outline=(40, 34, 80))


def caps():
    return [
        (0.0, T.T_FF0 - 0.05, ["CHIRPS + 40 = °F"], 3),
        (T.T_FF0, T.T_FF1, ["COUNT ITS CHIRPS", "FOR *15 SECONDS*..."], 2),
        (T.T_FF1, 10.3, ["...ADD *40*:", "IT'S *55°F*. SPOT ON!"], 2),
        (10.3, 13.6, ["*1897*: PHYSICIST AMOS", "DOLBEAR WROTE IT DOWN"], 2),
        (13.6, B[3], ["NOW LET'S *WARM UP*", "THE NIGHT..."], 2),
        (B[3], B[5], ["THE CHIRPS ARE", "THE *HI-HAT*"], 3),
        (B[5], B[7], ["ONLY *MALES* SING:", "WINGS UP, RUBBED TOGETHER"], 2),
        (B[7], B[10], ["NEIGHBORS *SYNC UP*:", "THE MEADOW CHIRPS *AS ONE*"], 2),
        (B[10], B[13], ["WHY FASTER? CRICKETS", "ARE *COLD-BLOODED*"], 2),
        (B[13], B[16], ["WARMER BODY =", "*FASTER WINGS*"], 3),
        (B[16], B[20], ["IT'S GETTING", "*HOT* IN HERE..."], 3),
        (B[20], T.T_COOL0, ["*90°F* = *200*", "CHIRPS A MINUTE!"], 3),
        (T.T_COOL0, T.T_COOL1, ["...UNTIL THE NIGHT", "*COOLS DOWN*"], 2),
        (T.T_COOL1, 57.0, ["IN °C? COUNT FOR", "*8 SECONDS*, ADD *5*"], 2),
        (57.0, 60.0, ["TRY IT *TONIGHT*!"], 3),
    ]


CAPS = caps()


def header(draw, t):
    for t0, t1, lines, sc in CAPS:
        if t0 <= t < t1:
            plain = [l.replace('*', '').replace('~', '') for l in lines]
            sc2 = sc
            if max(len(p) for p in plain) * ADV * sc > 330:
                sc2 = 2
            wmax = max(len(p) for p in plain) * ADV * sc2
            hgt = len(lines) * (7 * sc2 + 6) + 12
            pop = 1 - ease((t - t0) / 0.18)
            x0 = 180 - wmax // 2 - 12
            y0 = 30 - int(4 * pop)
            box(draw, x0, y0, 180 + wmax // 2 + 12, y0 + hgt)
            return (lines, sc2, y0, t0)
    return None


def header_text(img, hdr, t):
    if hdr is None:
        return
    lines, sc, y0, t0 = hdr
    nrev = int((t - t0) * 70) + 1
    used = 0
    y = y0 + 9
    for ln in lines:
        pl = len(ln.replace('*', '').replace('~', ''))
        rich_line(img, ln, 180, y, sc, CREAM, AMBER, PINK, nchars=nrev - used)
        used += pl
        y += 7 * sc + 6


PY0 = 486


def bottom_box(draw, t):
    if T.T_FF0 <= t < T.T_FF1 + 0.95:
        box(draw, 16, PY0, 344, PY0 + 70)
        return 'ff'
    if t < T.T_FF0 or t >= 57.0:
        return None
    box(draw, 16, PY0, 344, PY0 + 70)
    return 'live'


def bottom_text(img, t, mode):
    if mode == 'ff':
        if t < T.T_FF1:
            u = (t - T.T_FF0) / (T.T_FF1 - T.T_FF0)
            n = sum(1 for c in T.CHIRPS if c[1] == 'ff' and c[0] <= t)
            stamp(img, text_mask(">> 4X SPEED", 2), 28, PY0 + 9, AMBER)
            stamp(img, text_mask(f"{u * 15:4.1f} S", 2), 332 - 6 * 12, PY0 + 9, CREAM)
            img[PY0 + 27:PY0 + 34, 28:332] = (40, 30, 80)
            img[PY0 + 27:PY0 + 34, 28:28 + int(304 * u)] = AMBER
            m = text_mask(f"{n}", 4)
            stamp(img, m, 28, PY0 + 40, MINT)
            stamp(img, text_mask("CHIRPS", 2), 28 + m.shape[1] + 10, PY0 + 48, CREAM)
        else:
            sc = 4 if t - T.T_FF1 > 0.12 else 5
            line = [("15", MINT), (" + 40 = ", CREAM), ("55°F", merc_color(55))]
            total = sum(len(s) for s, _ in line) * ADV * sc
            x = 180 - total // 2
            for s, col in line:
                stamp(img, text_mask(s, sc), x, PY0 + 22 - (sc - 4) * 3, col)
                x += len(s) * ADV * sc
            stamp(img, text_mask("CHIRPS IN 15 S", 1), 30, PY0 + 8, (190, 180, 240))
        return
    Tn = T.temp(t)
    stamp(img, text_mask("CHIRPS IN 15 S", 2), 28, PY0 + 9, (190, 180, 240))
    g, bar = genre_now(t)
    if g >= 0:
        name = 'MAX TEMPO' if bar >= 20 else T.GENRES[g][1]
        cols = [(90, 170, 255), (180, 120, 255), (255, 160, 60), (60, 230, 200), (120, 150, 255), (255, 90, 160), (255, 220, 70)]
        col = (255, 80, 80) if bar >= 20 else cols[g]
        m = text_mask(name, 1)
        x1 = 332; x0 = x1 - m.shape[1] - 8
        img[PY0 + 8:PY0 + 19, x0:x1] = col
        stamp(img, m, x0 + 4, PY0 + 10, INK, outline=None)
    m = text_mask(f"{int(round(Tn - 40))}", 5)
    stamp(img, m, 28, PY0 + 28, MINT)
    stamp(img, text_mask("+40 =", 3), 28 + 70, PY0 + 34, CREAM)
    s = f"{int(round(Tn))}°F"
    m2 = text_mask(s, 4)
    stamp(img, m2, 332 - m2.shape[1], PY0 + 32, merc_color(Tn))


def bpm_label(draw, img, cam, t, ring_at, phase):
    """small boxed label next to the hero: its chirp rate"""
    if ring_at is None:
        return None
    Tn = T.temp(t)
    p = cam.wp(ring_at[None])[0]
    rate = 60 * T.chirp_rate(Tn)
    txt = f"{int(round(rate))} BPM"
    sc = 2
    m = text_mask(txt, sc)
    lb = last_beat(t, HERO_CHIRPS)
    pop = math.exp(-(t - lb) / 0.08) if t - lb < 0.5 else 0
    x0 = int(p[0] - m.shape[1] - 34)
    y0 = int(p[1] - 30)
    x0 = max(8, x0)
    if phase == 'draw':
        draw.line([(x0 + m.shape[1] + 10, y0 + 10), (p[0] - 6, p[1] - 4)], fill=(236, 232, 255), width=1)
        col = mixc((10, 8, 28), (60, 40, 120), pop)
        draw.rectangle([x0 + 2, y0 + 2, x0 + m.shape[1] + 12, y0 + 22], fill=(4, 2, 12))
        draw.rectangle([x0, y0, x0 + m.shape[1] + 10, y0 + 20], fill=tuple(int(c) for c in col), outline=(236, 232, 255))
    else:
        stamp(img, m, x0 + 5, y0 + 4, MINT if pop < 0.5 else CREAM, outline=None)


# ----------------------------------------------------------------- frame
VIG = None


def vignette():
    global VIG
    if VIG is None:
        r = np.hypot((XX - W / 2) / (W * 0.62), (YY - H * 0.47) / (H * 0.62))
        v = np.clip(r - 0.48, 0, 1) * 1.0
        v = np.floor(v * 5 + BAYER_FULL * 0.999) / 5
        VIG = (1 - np.clip(v, 0, 0.42))[..., None]
    return VIG


# ----------------------------------------------------------------- lens
APERTURE = 0.0085          # CoC radius at infinity = APERTURE * focal length (px) -- a macro lens around f/8
LEVELS = [0.0, 1.0, 2.0, 3.4, 5.2, 7.6, 10.5, 14.0, 18.0]


def oct_kernel(r):
    n = int(math.ceil(r))
    yy, xx = np.mgrid[-n:n + 1, -n:n + 1].astype(float)
    # octagonal aperture (8 blades)
    ang = np.arctan2(yy, xx)
    rad = r * math.cos(math.pi / 8) / np.cos(((ang + math.pi / 8) % (math.pi / 4)) - math.pi / 8)
    k = (np.hypot(xx, yy) <= rad + 0.35).astype(float)
    return k / k.sum()


KERNELS = {r: oct_kernel(r) for r in LEVELS[1:]}
KCUR = [1.0, 1000.0]


SHARP = 0.055        # relative depth band around the focus plane that stays fully sharp


def coc_arr(z, zf, K):
    return np.clip(K * np.maximum(0.0, np.abs(1 - zf / np.maximum(z, 1.0)) - SHARP), 0, LEVELS[-1])


def coc_of(z, zf, K):
    return float(coc_arr(np.array(z, float), zf, K))


def focus(t, cam):
    """locked focus: halfway between the cricket and the thermometer, so both are always sharp"""
    K = APERTURE * cam.f
    zh = float(cam.to_cam(D.HERO.o[None])[0, 2])
    b = D.BOARD
    zt = float(cam.to_cam(np.array([[(b['x0'] + b['x1']) / 2, (b['y0'] + b['y1']) / 2, b['z']]]))[0, 2])
    return 0.5 * (zh + zt), K


def ground_depth(cam):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1)
    dw = d @ cam.M
    tt_ = -cam.pos[1] / np.minimum(dw[..., 1], -1e-4)
    return np.clip(tt_ * d[..., 2], 1, 1e6)


def dof(img, depth, zf, K):
    coc = coc_arr(depth, zf, K)
    out = img.copy()
    prev = img
    for i in range(1, len(LEVELS)):
        r = LEVELS[i]
        lo = LEVELS[i - 1]
        sel = coc > lo
        if not sel.any():
            break
        m = (coc >= r * 0.7).astype(np.float32)
        kern = KERNELS[r]
        den = fftconvolve(m, kern, mode='same')
        num = np.stack([fftconvolve(img[..., c] * m, kern, mode='same') for c in range(3)], -1)
        blur = np.where(den[..., None] > 1e-3, num / np.maximum(den[..., None], 1e-3), img)
        w = np.clip((coc - lo) / (r - lo), 0, 1)[..., None]
        band = (coc > lo)[..., None]
        out = np.where(band, prev * (1 - w) + blur * w, out)
        prev = blur
    return out


def add_bokeh(img, depth, x, y, r, col, inten, zb):
    n = int(math.ceil(r)) + 1
    x0, y0 = int(round(x)) - n, int(round(y)) - n
    yy, xx = np.mgrid[-n:n + 1, -n:n + 1].astype(float)
    ang = np.arctan2(yy, xx)
    rad = r * math.cos(math.pi / 8) / np.cos(((ang + math.pi / 8) % (math.pi / 4)) - math.pi / 8)
    d = np.hypot(xx, yy)
    disc = np.clip(rad + 0.6 - d, 0, 1)
    rim = disc * np.clip(1.6 - (rad - d), 0, 1)
    a = (disc * 0.62 + rim * 0.3) * inten * 1.5 / (1 + 0.025 * r * r)
    xa, ya = max(0, x0), max(0, y0)
    xb, yb = min(W, x0 + 2 * n + 1), min(H, y0 + 2 * n + 1)
    if xb <= xa or yb <= ya:
        return
    dz = depth[ya:yb, xa:xb]
    cz = coc_arr(dz, KCUR[1], KCUR[0])
    vis = ((dz > zb * 0.93) | (cz > 0.6 * r)).astype(float)    # only sharp foreground hides it
    a = a[ya - y0:yb - y0, xa - x0:xb - x0] * vis
    c = np.array(col, float)[None, None]
    sub = img[ya:yb, xa:xb]
    img[ya:yb, xa:xb] = sub + (c - sub * 0.45) * np.clip(a, 0, 1)[..., None]


def grade(img):
    x = np.clip(img / 255.0, 0, 1)
    x = np.clip((x - 0.012) / 0.988, 0, 1) ** 1.04        # gentle black point
    x = x + 0.22 * (x * x * (3 - 2 * x) - x)              # mild S-curve contrast
    return x * 255.0


def draw_alpha(pil, s, col, a):
    mask = Image.new('L', (W, H), 0)
    ImageDraw.Draw(mask).polygon([tuple(p) for p in s], fill=int(255 * a))
    pil.paste(tuple(int(c) for c in col), (0, 0, W, H), mask)


def ff_positions(t, P, g):
    nff = int(45 + 75 * P.h)
    swirl = g >= 4
    out = []
    for k in range(nff):
        x, y, z, amp, ph, sp = D.FF[k]
        p = np.array([x + amp * math.sin(t * sp + ph), y + 0.4 * amp * math.sin(t * sp * 1.7 + ph), z + amp * math.cos(t * sp * 0.8 + ph)])
        if swirl and k % 3 == 0:
            a = t * 1.6 + k
            rad = 40 + (k % 7) * 9
            p = np.array([D.STEM_X - 20 + rad * math.cos(a), 60 + (k % 11) * 12 + 8 * math.sin(t * 2 + k), D.STEM_Z - 40 + rad * 0.6 * math.sin(a)])
        blink = max(0.0, math.sin(t * (1.6 + sp) + ph * 3))
        out.append((p, blink))
    for k in range(9):
        ph = k * 2.17
        p = np.array([-150 + 37 * k + 30 * math.sin(t * 0.4 + ph), 40 + 130 * ((k * 0.37) % 1.0) + 20 * math.sin(t * 0.7 + ph),
                      180 + 300 * ((k * 0.61) % 1.0)])
        out.append((p, max(0.0, math.sin(t * 1.1 + ph * 3)) * 0.9))
    return out


def setup_lights(P, t, g, bar, be, cam):
    LT.amb, LT.key, LT.rim, LT.fill, LT.sky = P.amb, P.key, P.rim, P.fill, P.sky
    LT.view = -cam.M[2]
    mp = KEY_D - np.dot(KEY_D, LT.view) * LT.view
    LT.moon_perp = mp / (np.linalg.norm(mp) + 1e-9)
    LT.points = []
    LT.extra = []
    flick = 0.85 + 0.15 * math.sin(t * 13) * math.sin(t * 7.7)
    LT.points.append((D.LANTERN.copy(), np.array([1.15, 0.66, 0.30]) * flick, 85.0))
    henv = chirp_env(t, HERO_CHIRPS)
    if henv > 0.03:
        LT.points.append((D.HERO.W([0.0, 0.9, 0.0]), np.array(P.ring) / 255 * 0.35 * henv, 55.0))
    offs = chorus_offsets(t)
    bl = 1.0 / T.chirp_rate(T.temp(t))
    for k, cr in enumerate(D.CHORUS_CR):
        e = chirp_env(t - offs[k] * bl, BEATS)
        if e > 0.05:
            LT.points.append((cr.W([0.0, 0.9, 0.0]), np.array(P.ring) / 255 * 0.6 * e, 32.0))
    ffs = ff_positions(t, P, g)
    near = sorted([(abs(p[2]) + abs(p[0]) * 0.3, p, b) for p, b in ffs if b > 0.3 and p[2] > -160], key=lambda a: a[0])[:7]
    for _, p, b in near:
        LT.points.append((p, np.array(P.ff) / 255 * 0.55 * b, 26.0))
    if (g >= 6 or bar >= 20) and t < T.T_COOL0:
        for j, (c, ph) in enumerate((((1.3, 0.3, 1.0), 0.0), ((0.3, 1.0, 1.3), math.pi))):
            a = t * 2.4 + ph
            pos = D.HERO.o + np.array([110 * math.cos(a), 30 + 20 * math.sin(a * 1.3), 60 * math.sin(a)])
            LT.points.append((pos, np.array(c) * (0.35 + 0.45 * be), 120.0))
    if bar >= 20 and t < T.T_COOL0:
        age = t - T.BAR_T[bar]
        if age < 0.25:
            LT.extra.append((D.nrm(np.array([0.1, 0.35, -1.0])), np.array([1.5, 1.3, 1.7]) * (1 - age / 0.25)))
    return ffs


def moon_rays(cam, P, t):
    ms = screen_dir(cam, MOON_D)
    if ms is None:
        return None
    ang = np.arctan2(YY - ms[1], XX - ms[0])
    dist = np.hypot(XX - ms[0], YY - ms[1])
    k = 0.5 + 0.5 * np.cos(ang * 9 + t * 0.15)
    k = k ** 6
    fall = np.clip(1 - dist / 520, 0, 1) ** 1.6 * np.clip(dist / 40, 0, 1)
    m = k * fall
    m = np.floor(m * 4 + BAYER_FULL * 0.999) / 4
    return m[..., None] * np.array(P.moon)[None, None] * 0.07


GR_Y, GR_X = np.mgrid[0:H // 2, 0:W // 2].astype(np.float32)


def god_rays(img, depth, cam, P, t):
    """volumetric moon shafts: the sky glow is smeared toward the camera, and every silhouette
    (poplars, fence, cricket, thermometer) cuts a dark shadow shaft out of the moonlit haze"""
    ms = screen_dir(cam, MOON_D)
    if ms is None:
        return img
    mx, my = ms[0] / 2, ms[1] / 2
    open_sky = (depth[::2, ::2] > 3e4).astype(np.float32)
    d = np.hypot(GR_X - mx, GR_Y - my)
    src = open_sky * (np.exp(-d / 70.0) + 0.25 * np.exp(-d / 200.0))
    acc = np.zeros_like(src)
    tot, w = 0.0, 1.0
    N = 36
    for k in range(N):
        sc = 0.9 * k / N
        sx = np.clip((GR_X + (mx - GR_X) * sc).astype(np.int32), 0, W // 2 - 1)
        sy = np.clip((GR_Y + (my - GR_Y) * sc).astype(np.int32), 0, H // 2 - 1)
        acc += src[sy, sx] * w
        tot += w
        w *= 0.965
    acc /= tot
    acc = acc - 0.6 * src                      # keep the sky itself from double-brightening
    dd = depth[::2, ::2]
    far_sil = ((dd > 1500) & (dd <= 3e4)).astype(np.float32)   # distant tree / farm silhouettes stay dark
    acc = acc * (1 - 0.7 * far_sil)
    up = np.asarray(Image.fromarray(np.clip(acc, 0, None)).resize((W, H), Image.BILINEAR))
    return img + up[..., None] * np.array(P.moon, np.float32)[None, None] * 0.6


def render(f):
    t = f / FPS
    P = Pal(t)
    cam = cam_at(t)
    g, bar = genre_now(t)
    be = beat_env(t, 0.15)
    CAST.clear()
    BLOBS.clear()
    ffs = setup_lights(P, t, g, bar, be, cam)
    skyimg = sky(cam, P, t)
    draw_stars(skyimg, cam, t, be if g >= 4 else 0.0)
    pil = Image.fromarray(skyimg.astype(np.uint8))
    draw = ImageDraw.Draw(pil)
    glows = []
    # clouds
    for blobs, spd in D.CLOUDS:
        for bi_, pts in enumerate(blobs):
            sp = []
            for az, el in pts:
                s = screen_dir(cam, dirv(az + spd * t, el))
                if s is None:
                    sp = None
                    break
                sp.append(tuple(s))
            if sp:
                draw.polygon(sp, fill=tuple(int(c) for c in mixc(P.zen, P.hor, 0.5 + 0.15 * bi_)))
    # heat lightning (max tempo): distant flashes on the downbeats
    if bar >= 20 and t < T.T_COOL0:
        age = t - T.BAR_T[bar]
        if age < 0.25:
            k = (1 - age / 0.25)
            az = [-18, 14, -6][bar % 3]
            a = screen_dir(cam, dirv(az, 3.0))
            if a is not None:
                pts = [tuple(a)]
                x, y = a
                rr = np.random.default_rng(bar)
                for j in range(6):
                    x += rr.uniform(-10, 10); y += 12
                    pts.append((x, y))
                draw.line(pts, fill=(255, 240, 255), width=2)
                glows.append((a[0], a[1] + 30, 70, (255, 200, 255), 0.5 * k))
    # moon
    ms = screen_dir(cam, MOON_D)
    if ms is not None:
        r = 0.05 * cam.f
        pulse = 1 + (0.04 * be if g >= 6 else 0)
        r *= pulse
        draw.polygon([(ms[0] + r * math.cos(2 * math.pi * k / 16), ms[1] + r * math.sin(2 * math.pi * k / 16)) for k in range(16)],
                     fill=tuple(int(c) for c in P.moon))
        for (dx, dy, rr) in ((-0.3, -0.2, 0.22), (0.25, 0.15, 0.18), (0.05, 0.42, 0.14), (-0.35, 0.3, 0.1)):
            cx, cy = ms[0] + dx * r, ms[1] + dy * r
            draw.polygon([(cx + rr * r * math.cos(a), cy + rr * r * math.sin(a)) for a in np.linspace(0, 2 * math.pi, 8)[:-1]],
                         fill=tuple(int(c * 0.86) for c in P.moon))
        glows.append((ms[0], ms[1], r * 2.4, tuple(P.moon), 0.25))
    # shooting stars (one early, then every bar in drum & bass)
    shoots = [(10.6, -20, 12)]
    if g >= 5 and bar < 20:
        shoots.append((T.BAR_T[bar], [-15, 12, -5, 18][bar % 4], 13))
    for ts0, az0, el0 in shoots:
        if ts0 <= t < ts0 + 0.6:
            u = (t - ts0) / 0.6
            a = screen_dir(cam, dirv(az0 + 8 * u, el0 - 5 * u))
            bb = screen_dir(cam, dirv(az0 + 8 * max(0, u - 0.3), el0 - 5 * max(0, u - 0.3)))
            if a is not None and bb is not None:
                draw.line([tuple(bb), tuple(a)], fill=(255, 255, 255), width=2)
                glows.append((a[0], a[1], 6, (255, 220, 255), 0.6 * (1 - u)))

    items, alpha_items, rings = [], [], []
    hero_ring = build_scene(cam, P, t, items, alpha_items, glows, rings)
    build_shadows(cam, P, items)
    rays = moon_rays(cam, P, t)
    if rays is not None:
        items.append((1900.0, 'rays', rays, None))
    # warm pool of lantern light on the ground
    pool = np.array([[D.LANTERN[0] + 70 * math.cos(q), 0.3, D.POST_Z + 40 * math.sin(q)] for q in np.linspace(0, 2 * math.pi, 13)[:-1]])
    sp_, zp_ = proj_poly(cam, pool)
    if sp_ is not None:
        alpha_items.append((zp_ + 40, sp_, (255, 170, 80), 0.22))
    items.sort(key=lambda it: -it[0])
    alpha_items.sort(key=lambda it: -it[0])
    dep = Image.new('F', (W, H), 1e6)
    ddraw = ImageDraw.Draw(dep)
    ai = 0
    for it in items:
        while ai < len(alpha_items) and alpha_items[ai][0] > it[0]:
            za, s, col, a = alpha_items[ai]
            draw_alpha(pil, s, col, a)
            if a >= 0.32:                     # cricket wings: solid enough to own their depth
                ddraw.polygon([tuple(p) for p in s], fill=float(za))
            draw = ImageDraw.Draw(pil)
            ai += 1
        kind = it[1]
        zr = it[0]
        if zr > 4e6:          # ground strips / far ridges: real depth filled in analytically
            zr = -1.0 if zr < 9e6 else 4e4
        elif zr < 0:          # soil face
            zr = float(cam.to_cam(np.array([[0, -60, D.FRONT_Z]]))[0, 2])
        if kind == 'poly':
            pts = [tuple(p) for p in it[2]]
            draw.polygon(pts, fill=tuple(int(c) for c in np.clip(it[3], 0, 255)))
            ddraw.polygon(pts, fill=float(zr))
        elif kind == 'line':
            pts = [tuple(p) for p in it[2]]
            draw.line(pts, fill=tuple(int(c) for c in it[3]), width=int(it[4]))
            ddraw.line(pts, fill=float(zr), width=int(it[4]))
        elif kind == 'shadow':
            recv, polys, a, scol = it[2]
            if polys:
                Sm = Image.new('L', (W, H), 0)
                dS = ImageDraw.Draw(Sm)
                for p in polys:
                    dS.polygon([tuple(q) for q in p], fill=255)
                if recv is not None:
                    Rm = Image.new('L', (W, H), 0)
                    dR = ImageDraw.Draw(Rm)
                    for p in recv:
                        dR.polygon([tuple(q) for q in p], fill=255)
                    Sm = ImageChops.multiply(Sm, Rm)
                Sm = Sm.point(lambda v, a=a: int(v * a))
                pil.paste(scol, (0, 0, W, H), Sm)
                draw = ImageDraw.Draw(pil)
        elif kind == 'rays':
            arr = np.asarray(pil).astype(np.float32) + it[2]
            pil = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            draw = ImageDraw.Draw(pil)
        elif kind == 'flower':
            x, y, r, col, cen, k, bright = it[2][:7]
            if len(it[2]) > 7:
                col = tuple(int(c) for c in np.clip(np.array(col) * bright, 0, 255))
            else:
                col = tuple(int(c) for c in np.clip(np.array(col) * (P.amb * 0.55 + 0.6) * bright, 0, 255))
            ddraw.ellipse([x - r, y - r, x + r, y + r], fill=float(zr))
            if k == 9:
                draw.ellipse([x - r, y - r, x + r, y + r], fill=col)
                if cen != col:
                    rc = r * 0.45
                    draw.ellipse([x - rc, y - rc, x + rc, y + rc], fill=cen)
            elif r < 1.2:
                draw.point((x, y), fill=col)
            else:
                n = [8, 6, 5, 10][k]
                draw.polygon([(x + r * math.cos(2 * math.pi * j / n + 0.3), y + r * math.sin(2 * math.pi * j / n + 0.3)) for j in range(n)], fill=col)
                rc = r * (0.38 if k != 3 else 0.7)
                draw.polygon([(x + rc * math.cos(2 * math.pi * j / 6), y + rc * math.sin(2 * math.pi * j / 6)) for j in range(6)], fill=cen)
    while ai < len(alpha_items):
        _, s, col, a = alpha_items[ai]
        draw_alpha(pil, s, col, a)
        ai += 1

    img = np.asarray(pil).astype(np.float32)
    depth = np.asarray(dep).copy()
    gmask = depth < 0
    if gmask.any():
        depth[gmask] = ground_depth(cam)[gmask]
    zf, K = focus(t, cam)
    KCUR[:] = [K, zf]
    if not os.environ.get('NODOF'):
        img = dof(img, depth, zf, K)
    img = god_rays(img, depth, cam, P, t)
    bokeh = []
    # fireflies (swirl around the hero plant in trance+); out of focus ones become bokeh
    for p, blink in ffs:
        pc = cam.to_cam(p[None])[0]
        if pc[2] < 10:
            continue
        sx, sy = cam.proj(pc[None])[0]
        if blink < 0.15:
            continue
        coc = coc_of(pc[2], zf, K)
        if coc >= 1.6:
            bokeh.append((sx, sy, coc, P.ff, 1.1 * blink, pc[2]))
            continue
        rr = min(10, max(2.0, 2.4 * cam.f / pc[2]))
        glows.append((sx, sy, rr * 2.2, tuple(P.ff), 0.6 * blink))
        if 0 <= sx < W and 0 <= sy < H:
            img[int(sy), int(sx)] = np.maximum(img[int(sy), int(sx)], np.array([255, 255, 215]) * blink)
    # distant window lights + lantern as bokeh when defocused
    for Q in D.HOUSE_WINDOWS:
        c3 = Q.mean(0)
        pc = cam.to_cam(c3[None])[0]
        if pc[2] > 10:
            q = cam.proj(pc[None])[0]
            coc = coc_of(pc[2], zf, K)
            if coc >= 1.6:
                bokeh.append((q[0], q[1], coc, (255, 150, 40), 0.8, pc[2]))
    pc = cam.to_cam(D.LANTERN[None])[0]
    coc = coc_of(pc[2], zf, K)
    if coc >= 1.6:
        q = cam.proj(pc[None])[0]
        bokeh.append((q[0], q[1], coc + 1, (255, 160, 60), 1.0, pc[2]))
    for b_ in bokeh:
        add_bokeh(img, depth, *b_)
    # sparkle bursts on the beats (chiptune+)
    if g >= 6 and be > 0.05:
        lb = last_beat(t, BEATS)
        rr = np.random.default_rng(int(lb * 1000))
        hp = cam.wp(D.HERO.o[None])[0]
        for j in range(10):
            a = rr.uniform(0, 2 * math.pi); d = (t - lb) * rr.uniform(150, 320)
            glows.append((hp[0] + d * math.cos(a), hp[1] + d * math.sin(a) - 30, 3, [PINK, CYAN, AMBER][j % 3], 0.9 * be))
    for lp, lc, R in LT.points:
        if R >= 120:
            q = cam.wp(lp[None])[0]
            glows.append((q[0], q[1], 16, tuple(np.clip(lc * 180, 0, 255)), 0.7))
    for x, y, r, col, inten in glows:
        add_glow(img, x, y, r, col, inten)
    # sound rings
    for (ra, age, bl, size) in rings:
        life = min(0.42, 0.9 * bl)
        if 0 <= age < life:
            u = age / life
            p = cam.wp(ra[None])[0]
            sc = cam.ppu(ra) * (18 if size >= 1 else 9)
            for j, (r0, r1) in enumerate(((0.15, 1.6), (0.05, 1.0))):
                add_ring(img, p[0], p[1], sc * (r0 + (r1 - r0) * ease(u)), 2 if size >= 1 else 1, P.ring, (1 - u) * (0.95 - 0.3 * j) * (1 if size >= 1 else 0.8))
            add_glow(img, p[0], p[1], sc * 0.6, P.ring, 0.25 * (1 - u))
    # genre-change flash
    if 0 <= bar < T.NBARS and t < T.T_COOL0 and (bar == 0 or T.BAR_GENRE[bar] != T.BAR_GENRE[bar - 1] or bar == 20):
        k = math.exp(-(t - T.BAR_T[bar]) / 0.07) * 0.28
        img = img * (1 - k) + np.array([255, 245, 255]) * k
    img = np.clip(img, 0, 255)
    if False:
        kk = (P.h - 0.8) / 0.2
        shift = np.round(kk * 1.2 * np.sin(np.arange(H) * 0.23 + t * 11)).astype(int)
        for y in range(110, 300):
            if shift[y]:
                img[y] = np.roll(img[y], shift[y], axis=0)
    gray = img.mean(-1, keepdims=True)
    img = np.clip(gray + (img - gray) * 1.2, 0, 255)
    img = grade(img) * vignette()
    img = np.floor(img / 5.0 + BAYER_FULL[..., None] * 0.999) * 5.0

    # UI
    pil = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8))
    draw = ImageDraw.Draw(pil)
    hdr = header(draw, t)
    mode = bottom_box(draw, t)
    show_bpm = (T.T_MUSIC <= t < 57.0) and not (B[5] <= t < B[7])
    if show_bpm:
        bpm_label(draw, None, cam, t, hero_ring, 'draw')
    callout = t < T.T_FF0
    if callout:
        hp = cam.wp((D.HERO.o + np.array([-20, -14, 0]))[None])[0]
        draw.line([(hp[0], hp[1]), (hp[0] - 4, 440)], fill=MINT, width=2)
        draw.rectangle([12, 440, 210, 462], fill=MINT)
    out = np.asarray(pil).copy()
    thermo_labels(out, cam, t)
    header_text(out, hdr, t)
    if mode:
        bottom_text(out, t, mode)
    if show_bpm:
        bpm_label(None, out, cam, t, hero_ring, 'text')
    if callout:
        stamp(out, text_mask("SNOWY TREE CRICKET", 2), 17, 445, INK, outline=None)
        lb = last_beat(t, HERO_CHIRPS)
        if 0 <= t - lb < 0.45:
            k = HERO_CHIRPS.index(lb)
            p = cam.wp(hero_ring[None])[0]
            m = text_mask("CHIRP!", 2 if t - lb > 0.06 else 3)
            stamp(out, m, max(4, int(p[0] + (30 if k % 2 else -100))), int(p[1] - 30 - (t - lb) * 30), CYAN if k % 2 else PINK)
    return out


if __name__ == '__main__':
    if sys.argv[1] == 'sheet':
        ts = [float(x) for x in sys.argv[3:]]
        cols = 7
        rows = (len(ts) + cols - 1) // cols
        sheet = Image.new('RGB', (cols * 366, rows * 646), (255, 255, 255))
        for k, tt in enumerate(ts):
            sheet.paste(Image.fromarray(render(int(round(tt * FPS)))), ((k % cols) * 366, (k // cols) * 646))
        sheet.save(sys.argv[2])
        print('saved', sys.argv[2])
    elif sys.argv[1] == 'frame':
        tt = float(sys.argv[3])
        Image.fromarray(render(int(round(tt * FPS)))).resize((W * UP, H * UP), Image.NEAREST).save(sys.argv[2])
