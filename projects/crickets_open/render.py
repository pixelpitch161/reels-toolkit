"""Crickets Are Thermometers: vibrant low-poly + pixel meadow at night.
Rendered at 360x640, upscaled 3x nearest-neighbour."""
import math, sys
import numpy as np
from PIL import Image, ImageDraw
import timeline as T
import scene as S
import cricket as C
from font import text_mask, stamp, rich_line, ADV

W, H = 360, 640
UP = 3
FPS = T.FPS
rng = np.random.default_rng(9)

CREAM = (255, 244, 220)
MINT = (110, 255, 200)
CYAN = (90, 230, 255)
PINK = (255, 92, 190)
AMBER = (255, 206, 80)
HOT = (255, 92, 70)
INK = (12, 10, 30)

HERO_CHIRPS = [c[0] for c in T.CHIRPS]


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


MOON_AZ, MOON_EL = -32.0, 26.0
MOON_D = dirv(MOON_AZ, MOON_EL)
RIM_D = dirv(150, 30)


class Pal:
    def __init__(self, t):
        h = T.heat(t)
        self.h = h
        self.zen = mixc((8, 12, 54), (60, 10, 86), h)
        self.hor = mixc((30, 120, 186), (255, 98, 100), h)
        self.hor2 = mixc((24, 64, 140), (190, 40, 120), h)
        self.amb = mixc((0.30, 0.36, 0.66), (0.66, 0.34, 0.56), h)
        self.key = mixc((0.70, 0.78, 1.0), (1.0, 0.70, 0.60), h)
        self.rim = mixc((0.20, 0.95, 1.0), (1.0, 0.40, 0.80), h)
        self.grass = mixc((36, 176, 138), (150, 214, 70), h)
        self.ring = tuple(int(c) for c in mixc((120, 255, 230), (255, 150, 230), h))
        self.moon = mixc((255, 246, 214), (255, 190, 130), h)
        self.ff = mixc((200, 255, 90), (255, 230, 90), h)


def shade(P, n, base, rimk=0.5):
    I = P.amb + P.key * max(0.0, float(np.dot(n, MOON_D))) * 0.85 + P.rim * max(0.0, float(np.dot(n, RIM_D))) * rimk
    return np.clip(np.round(np.array(base, float) * I / 4) * 4, 0, 255)


def shade_v(P, N, base, rimk=0.5):
    I = P.amb[None] + np.clip(N @ MOON_D, 0, None)[:, None] * P.key[None] * 0.85 + np.clip(N @ RIM_D, 0, None)[:, None] * P.rim[None] * rimk
    return np.clip(np.round(base * I / 4) * 4, 0, 255)


def fog_k(dist):
    return np.clip((dist - 1800) / 30000, 0, 0.82)


# ----------------------------------------------------------------- camera
class Cam:
    def __init__(self, tgt, d, yaw, pitch, fov, sy):
        yr, pr = math.radians(yaw), math.radians(pitch)
        self.pos = tgt + d * np.array([math.sin(yr) * math.cos(pr), math.sin(pr), math.cos(yr) * math.cos(pr)])
        fwd = tgt - self.pos; fwd /= np.linalg.norm(fwd)
        right = np.cross(fwd, [0, 1, 0]); right /= np.linalg.norm(right)
        up = np.cross(right, fwd)
        self.M = np.stack([right, up, fwd])
        self.f = (H / 2) / math.tan(math.radians(fov) / 2)
        self.cx, self.cy = W / 2, sy
        self.pitch = pr

    def to_cam(self, P):
        return (P - self.pos) @ self.M.T

    def proj(self, Pc):
        z = np.maximum(Pc[..., 2], 1e-4)
        return np.stack([self.cx + self.f * Pc[..., 0] / z, self.cy - self.f * Pc[..., 1] / z], -1)

    def wp(self, P):
        return self.proj(self.to_cam(P))


NEAR = 0.4


def clip_near(Pc):
    out = []
    n = len(Pc)
    for k in range(n):
        a, b = Pc[k], Pc[(k + 1) % n]
        ia, ib = a[2] >= NEAR, b[2] >= NEAR
        if ia:
            out.append(a)
        if ia != ib:
            u = (NEAR - a[2]) / (b[2] - a[2])
            out.append(a + (b - a) * u)
    return np.array(out) if len(out) >= 3 else None


def proj_poly(cam, P):
    Pc = cam.to_cam(P)
    if (Pc[:, 2] < NEAR).any():
        if (Pc[:, 2] < NEAR).all():
            return None, 0
        Pc = clip_near(Pc)
        if Pc is None:
            return None, 0
    s = cam.proj(Pc)
    if s[:, 0].max() < -5 or s[:, 0].min() > W + 5 or s[:, 1].max() < -5 or s[:, 1].min() > H + 5:
        return None, 0
    return s, float(Pc[:, 2].mean())


def K(tgt, d, yaw, pitch, fov=44, sy=300):
    return (np.array(tgt, float), d, yaw, pitch, fov, sy)


HF = S.HERO + np.array([0.1, 0.25, 0.0])
B = list(T.BAR_T)
THC = np.array([S.POST_X, 72.0, S.THERMO['z']])
SHOTS = [
    (0.0, T.T_FF0, K(HF, 7.0, 8, 8, 40, 300), K(HF, 6.4, -2, 9, 40, 300), 'ease'),
    (T.T_FF0, T.T_FF1, K(HF + [0.5, 0.2, 0], 8.0, 40, 12, 42, 290), K(HF + [0.5, 0.2, 0], 7.4, 30, 12, 42, 290), 'lin'),
    (T.T_FF1, B[0], K(HF + [0.5, 0.2, 0], 7.4, 30, 12, 42, 290), K(HF + [0.5, 0.2, 0], 7.2, 27, 12, 42, 290), 'lin'),
    (B[0], B[1], K(HF, 7.2, 27, 12, 42, 290), K([0, 260, -1700], 4800, 16, 9, 54, 360), 'zoom'),
    (B[1], B[2], K([0, 260, -1700], 4600, 16, 9, 54, 360), K([-500, 260, -2300], 4300, -12, 8, 54, 360), 'ease'),
    (B[2], B[3], K(HF, 1100, 36, 3, 48, 320), K(HF + [2, 3, -3], 30, 22, 7, 44, 300), 'exp'),
    (B[3], B[4], K(HF + [1, 1.5, -1], 20, -26, 9, 44, 300), K(HF + [1, 1.5, -1], 17, -14, 10, 44, 300), 'lin'),
    (B[4], B[5], K(HF, 6.6, 52, 10, 40, 300), K(HF, 6.0, 38, 12, 40, 300), 'lin'),
    (B[5], B[7], K(HF + [-0.15, 0.5, 0], 5.6, -138, 30, 42, 310), K(HF + [-0.15, 0.5, 0], 5.0, -100, 26, 42, 310), 'lin'),
    (B[7], B[9], K([0, 100, -900], 2700, -12, 18, 54, 340), K([0, 100, -900], 2400, 6, 16, 54, 340), 'lin'),
    (B[9], B[11], K(HF + [0, 0.3, 0], 30, -MOON_AZ + 1.5, -MOON_EL + 0.5, 30, 300), K(HF + [0, 0.3, 0], 26, -MOON_AZ - 1.5, -MOON_EL - 0.5, 30, 300), 'lin'),
    (B[11], B[13], K(HF, 15, 62, 14, 44, 300), K(HF, 13, -32, 16, 44, 300), 'lin'),
    (B[13], B[14], K(THC + [0, -6, 0], 100, 16, 4, 44, 300), K(THC + [0, -6, 0], 88, 6, 5, 44, 300), 'lin'),
    (B[14], B[15], K(S.POND + [0, 60, 0], 1250, 22, 12, 50, 320), K(S.POND + [0, 60, 0], 1080, 10, 10, 50, 320), 'lin'),
    (B[15], B[16], K(HF, 6.2, 62, 8, 40, 300), K(HF, 5.7, 76, 8, 40, 300), 'lin'),
    (B[16], B[17], K(S.PUMP + [-600, 1100, 0], 4200, 8, 6, 52, 330), K(S.PUMP + [-600, 1100, 0], 3800, -2, 5, 52, 330), 'lin'),
    (B[17], B[18], K(HF, 6.6, 0, 6, 40, 300), K(HF, 6.0, -12, 8, 40, 300), 'lin'),
    (B[18], B[19], K(S.OAK + [0, 650, 0], 3300, -10, 8, 52, 330), K(S.OAK + [0, 650, 0], 2950, 4, 5, 52, 330), 'lin'),
    (B[19], B[20], K(HF, 9.5, 20, 68, 42, 300), K(HF, 8.4, 52, 72, 42, 300), 'lin'),
    (B[20], B[21], K(HF, 12, -62, 14, 44, 300), K(HF, 11, 32, 14, 44, 300), 'lin'),
    (B[21], B[22], K([0, 100, -700], 2100, -4, 16, 54, 340), K([0, 100, -700], 1850, 16, 14, 54, 340), 'lin'),
    (B[22], T.T_COOL0, K(HF, 6.6, 22, 8, 40, 300), K(HF, 6.0, -20, 10, 40, 300), 'lin'),
    (T.T_COOL0, T.T_COOL1, K(HF, 8.5, -12, 10, 44, 300), K([0, 260, -1500], 4200, 10, 9, 54, 360), 'zoom'),
    (T.T_COOL1, 57.0, K([0, 260, -1500], 4200, 10, 9, 54, 360), K(HF, 7.0, 8, 8, 40, 300), 'zoomin'),
    (57.0, 60.1, K(HF, 7.0, 8, 8, 40, 300), K(HF, 7.0, 8, 8, 40, 300), 'lin'),
]


def cam_at(t):
    for t0, t1, a, b, mode in SHOTS:
        if t0 <= t < t1:
            u = (t - t0) / (t1 - t0)
            break
    else:
        t0, t1, a, b, mode = SHOTS[-1]
        u = 1.0
    if mode == 'ease':
        ue = ease(u)
        ud = ue
        ut = ue
    elif mode in ('zoom', 'exp', 'zoomin'):
        ue = ease(u)
        ud = ue
        ut = ss(0.35, 1.0, u) if mode == 'zoom' else (ss(0.0, 0.65, u) if mode == 'zoomin' else ue)
    else:
        ue = ud = ut = u
    tgt = a[0] + (b[0] - a[0]) * ut
    d = math.exp(math.log(a[1]) + (math.log(b[1]) - math.log(a[1])) * ud)
    yaw, pitch, fov, sy = [a[i] + (b[i] - a[i]) * ue for i in (2, 3, 4, 5)]
    # downbeat punch
    bi = T.bar_at(t)
    if 0 <= bi < T.NBARS and t < T.T_COOL0:
        fov *= 1 - 0.018 * math.exp(-(t - T.BAR_T[bi]) / 0.09)
    if t >= T.BAR_T[20] and t < T.T_COOL0:      # climax shake
        for bt in T.BEAT_T:
            if bt <= t < bt + 0.12:
                k = (1 - (t - bt) / 0.12) * 0.012 * d
                tgt = tgt + k * np.array([math.sin(t * 91), math.sin(t * 73), 0])
    return Cam(tgt, d, yaw, pitch, fov, sy)


# ----------------------------------------------------------------- sky
BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
BAYER_FULL = np.tile(BAYER, (H // 4, W // 4))
YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)
STAR_D = np.array([dirv(a, e) for a, e, _, _, _ in S.STARS])
STAR_B = np.array([b for _, _, b, _, _ in S.STARS])
STAR_P = np.array([p for _, _, _, p, _ in S.STARS])
STAR_MW = np.array([mw for *_, mw in S.STARS])


def sky(cam, P, t):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2]) - math.radians(MOON_AZ)
    u = ((1 + np.cos(az)) / 2) ** 1.6
    hor = P.hor2[None, None] * (1 - u[..., None]) + P.hor[None, None] * u[..., None]
    k = np.clip(el / 0.8, 0, 1) ** 0.55
    col = hor * (1 - k[..., None]) + P.zen[None, None] * k[..., None]
    # moon halo
    cosang = (d * MOON_D[None, None]).sum(-1)
    g = np.clip((cosang - 0.93) / 0.07, 0, 1) ** 2
    col += g[..., None] * P.moon[None, None] * 0.35
    step = 8.0
    col = np.floor(col / step + BAYER_FULL[..., None]) * step
    return np.clip(col, 0, 255)


def screen_dir(cam, dv, dist=1e6):
    pc = cam.M @ (dv * dist)
    if pc[2] <= 1:
        return None
    return cam.proj(pc[None])[0]


def draw_stars(img, cam, P, t):
    Pc = STAR_D @ cam.M.T
    vis = Pc[:, 2] > 0.05
    s = cam.proj(Pc[vis] * 1e5)
    b = STAR_B[vis] * (0.65 + 0.35 * np.sin(t * 3.3 + STAR_P[vis] * 7))
    mw = STAR_MW[vis]
    x, y = s[:, 0].astype(int), s[:, 1].astype(int)
    ok = (x >= 1) & (x < W - 1) & (y >= 1) & (y < H - 1)
    for xi, yi, bi, m in zip(x[ok], y[ok], b[ok], mw[ok]):
        c = (np.array([255, 170, 240]) if m else np.array([235, 240, 255])) * bi * (0.75 if m else 1.0)
        img[yi, xi] = np.maximum(img[yi, xi], c)
        if bi > 0.7 and not m:
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                img[yi + dy, xi + dx] = np.maximum(img[yi + dy, xi + dx], c * 0.45)


# ----------------------------------------------------------------- glow helpers
def glow_kernel(r):
    yy, xx = np.mgrid[-r:r + 1, -r:r + 1]
    dd = np.hypot(xx, yy) / (r + 0.5)
    v = np.clip(1 - dd, 0, 1) ** 1.6
    b = np.tile(BAYER, ((2 * r + 1) // 4 + 1, (2 * r + 1) // 4 + 1))[:2 * r + 1, :2 * r + 1]
    return np.floor(v * 5 + b * 0.999) / 5


GLOW = {r: glow_kernel(r) for r in range(1, 81)}


def add_glow(img, x, y, r, col, inten):
    r = int(max(1, min(80, round(r))))
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


# ----------------------------------------------------------------- per-frame helpers
def last_chirp(t, times=None):
    times = HERO_CHIRPS if times is None else times
    i = np.searchsorted(times, t, side='right') - 1
    return times[i] if i >= 0 else -9


def beat_len(t):
    return 1.0 / T.chirp_rate(T.temp(t))


def wind(t):
    return 0.6 + 0.4 * math.sin(t * 0.37)


# ----------------------------------------------------------------- the world draw list
def build_world(cam, P, t, items, glows):
    cp = cam.pos
    # --- terrain
    tc = cam.to_cam(S.TER_C)
    vis = tc[:, 2] > -600
    N = S.TER_N
    alt = mixc((30, 150, 190), (220, 120, 60), P.h)
    base = (P.grass[None] * (1 - S.TER_PATCH[:, None] * 0.55) + alt[None] * S.TER_PATCH[:, None] * 0.55) * S.TER_TONE[:, None] * 0.8
    cols = shade_v(P, N, base, 0.35)
    dist = np.linalg.norm(S.TER_C - cp[None], axis=1)
    fk = fog_k(dist)[:, None]
    cols = cols * (1 - fk) + P.hor[None] * fk
    for i in np.nonzero(vis)[0]:
        s, z = proj_poly(cam, S.TER[i])
        if s is not None:
            items.append((z + 50, 'poly', s, cols[i]))
    # dirt path
    for (xa, za, wa), (xb, zb, wb) in zip(S.PATH, S.PATH[1:]):
        Qp = np.array([[xa - wa, S.ground_h(xa, za) + 3, za], [xa + wa, S.ground_h(xa, za) + 3, za],
                       [xb + wb, S.ground_h(xb, zb) + 3, zb], [xb - wb, S.ground_h(xb, zb) + 3, zb]])
        sp, zp = proj_poly(cam, Qp)
        if sp is not None:
            dist = float(np.linalg.norm(Qp.mean(0) - cp))
            fk = float(fog_k(dist))
            items.append((zp + 40, 'poly', sp, shade(P, np.array([0, 1.0, 0]), (176, 140, 120)) * (1 - fk) + P.hor * fk))
    s, z = proj_poly(cam, S.CENTER_DISC)
    if s is not None:
        items.append((z + 60, 'poly', s, shade(P, np.array([0, 1, 0]), P.grass * 0.7)))
    # --- pond (reflects the sky) + lily pads
    s, z = proj_poly(cam, S.POND_POLY)
    if s is not None:
        pc = mixc(P.zen, P.hor, 0.55) * 0.9 + np.array([0, 10, 25])
        items.append((z + 30, 'poly', s, pc))
        for (px, pz, pr) in S.PADS:
            pad = np.array([[px + pr * math.cos(a), 1.5, pz + pr * 0.8 * math.sin(a)] for a in np.linspace(0.3, 2 * math.pi, 9)])
            s2, z2 = proj_poly(cam, pad)
            if s2 is not None:
                items.append((z2 + 29, 'poly', s2, shade(P, np.array([0, 1, 0]), (60, 190, 90))))
        # moon glints on the water
        mg = S.POND + np.array([120, 2, 60])
        g = cam.wp(mg[None])[0]
        glows.append((g[0], g[1], 6, tuple(P.moon), 0.5))
    # --- grass tufts (wide)
    tu = S.TUFT
    sway = (np.sin(t * 1.6 + tu[:, 10]) * 0.12 * wind(t))[:, None]
    A = tu[:, 0:3]; Bp = tu[:, 3:6]; Tp = tu[:, 6:9].copy()
    hgt = (Tp[:, 1] - A[:, 1])[:, None]
    Tp[:, 0:1] += sway * hgt
    allp = np.stack([A, Bp, Tp], 1)
    ctr = allp.mean(1)
    cc = cam.to_cam(ctr)
    ok = (cc[:, 2] > NEAR + 2)
    d3 = np.linalg.norm(ctr - cp[None], axis=1)
    ok &= d3 < 4500
    idx = np.nonzero(ok)[0]
    if len(idx):
        Pc = cam.to_cam(allp[idx].reshape(-1, 3)).reshape(-1, 3, 3)
        good = (Pc[:, :, 2] > NEAR).all(1)
        sc = cam.proj(Pc)
        bcol = shade_v(P, np.tile([0.0, 0.45, 0.89], (len(idx), 1)), P.grass[None] * tu[idx, 9:10], 0.6)
        fk = fog_k(d3[idx])[:, None]
        bcol = bcol * (1 - fk) + P.hor[None] * fk
        for j in np.nonzero(good)[0]:
            sp = sc[j]
            if sp[:, 0].max() < 0 or sp[:, 0].min() > W or sp[:, 1].max() < 0 or sp[:, 1].min() > H:
                continue
            items.append((cc[idx[j], 2], 'poly', sp, bcol[j]))
    # --- flowers (screen-space low-poly discs)
    fc = cam.to_cam(S.FL[:, :3])
    for i in np.nonzero(fc[:, 2] > NEAR + 1)[0]:
        x, y = cam.proj(fc[i][None])[0]
        r = S.FL[i, 3] * cam.f / fc[i, 2]
        if r < 0.5 or x < -20 or x > W + 20 or y < -20 or y > H + 20:
            continue
        kind = S.FL_KIND[i]
        col = [(255, 250, 240), (255, 80, 190), (90, 170, 255)][kind]
        cen = [(255, 210, 40), (255, 160, 230), (220, 240, 255)][kind]
        items.append((fc[i, 2], 'flower', (x, y, r, col, cen, kind), None))
        if r > 2:
            glows.append((x, y, r * 1.6, col, 0.10 + 0.1 * P.h))
    # --- macro grass blades around the hero (skip any that would hide it)
    hz = cam.to_cam(S.HERO[None])[0, 2]
    hb = None
    if hz > NEAR:
        kp = cam.wp(S.HERO[None] + np.array([[-1.1, -0.5, 0], [3.6, 0.6, 1.4], [3.6, 0.6, -1.4], [0.0, 1.25, 0], [1.0, -0.5, 0]]))
        hb = (kp[:, 0].min() - 6, kp[:, 1].min() - 6, kp[:, 0].max() + 6, kp[:, 1].max() + 6)
    mb = S.MACRO
    for (x, z, hgt, w, ang, lean, tone, ph) in mb:
        sw = math.sin(t * 1.4 + ph) * 0.06 * wind(t) * hgt
        dx, dz = math.cos(ang) * w, math.sin(ang) * w
        base0 = np.array([x - dx, 0, z - dz]); base1 = np.array([x + dx, 0, z + dz])
        mid = np.array([x + lean * hgt * 0.35 + sw * 0.4, hgt * 0.55, z])
        tip = np.array([x + lean * hgt + sw, hgt, z])
        Q = np.array([base0, base1, mid + [dx * 0.6, 0, dz * 0.6], tip, mid - [dx * 0.6, 0, dz * 0.6]])
        s, zz = proj_poly(cam, Q)
        if s is None:
            continue
        if hb is not None and zz < hz and s[:, 0].max() > hb[0] and s[:, 0].min() < hb[2] and s[:, 1].max() > hb[1] and s[:, 1].min() < hb[3]:
            continue
        n = np.array([-dz, 0.3, dx]); n /= np.linalg.norm(n)
        if np.dot(n, cp - Q.mean(0)) < 0:
            n = -n
        col = shade(P, n, P.grass * tone, 0.8)
        items.append((zz, 'blade', s, col))
    # --- solids
    for name, faces, base in S.SOLIDS:
        for Q, n in faces:
            ctr = Q.mean(0)
            if np.dot(n, cp - ctr) <= 0:
                continue
            s, z = proj_poly(cam, Q)
            if s is None:
                continue
            col = shade(P, n, base, 1.3 if name == 'oak_crown' else 0.5)
            dist = np.linalg.norm(ctr - cp)
            fk = float(fog_k(dist))
            col = col * (1 - fk) + P.hor * fk
            items.append((z, 'poly', s, col))
    # windows (always lit), porch light
    for Q in S.WINDOWS:
        s, z = proj_poly(cam, Q)
        if s is not None:
            items.append((z - 1, 'poly', s, (255, 214, 110)))
            c = s.mean(0)
            glows.append((c[0], c[1], max(3, (s[:, 0].max() - s[:, 0].min()) * 1.4), (255, 170, 70), 0.35))
    s, z = proj_poly(cam, S.DOOR)
    if s is not None:
        items.append((z - 1, 'poly', s, (120, 70, 60)))
    g = cam.to_cam(S.PORCH_LIGHT[None])[0]
    if g[2] > NEAR:
        x, y = cam.proj(g[None])[0]
        glows.append((x, y, max(4, 220 * cam.f / g[2] * 0.25), (255, 190, 90), 0.5))
    # --- windpump
    windpump(cam, P, t, items)
    # --- thermometer on the fence post
    thermo_board(cam, P, t, items, glows)


def windpump(cam, P, t, items):
    base = S.PUMP
    top = base + np.array([0, 1700, 0])
    dist = np.linalg.norm(base - cam.pos)
    z = cam.to_cam(base[None])[0, 2]
    if z < 10:
        return
    col = tuple(int(c) for c in mixc(P.zen * 0.6, P.hor, 0.3) * 0.7)
    legs = [base + np.array([sx * 220, 0, sz * 220]) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    tops = [top + np.array([sx * 35, 0, sz * 35]) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    wpx = max(1, int(round(14 * cam.f / z)))
    for a, b in zip(legs, tops):
        s = cam.wp(np.array([a, b]))
        items.append((z, 'line', s, col, wpx))
    for lv in (0.3, 0.55, 0.78):
        ring = [a + (b - a) * lv for a, b in zip(legs, tops)]
        s = cam.wp(np.array(ring + [ring[0]]))
        items.append((z, 'line', s, col, max(1, wpx // 2)))
    hub = top + np.array([0, 60, 40])
    ang0 = t * 1.8
    for k in range(18):
        a = ang0 + 2 * math.pi * k / 18
        p1 = hub + np.array([math.cos(a) * 60, math.sin(a) * 60, 0])
        p2 = hub + np.array([math.cos(a + 0.12) * 280, math.sin(a + 0.12) * 280, 0])
        p3 = hub + np.array([math.cos(a - 0.05) * 280, math.sin(a - 0.05) * 280, 0])
        s = cam.wp(np.array([p1, p2, p3]))
        items.append((z - 5, 'poly', s, mixc(col, (220, 220, 240), 0.25 + 0.15 * (k % 2))))
    vane = np.array([hub + [0, 0, -40], hub + [0, 60, -520], hub + [0, -60, -520]])
    items.append((z - 4, 'poly', cam.wp(vane), mixc(col, (200, 60, 80), 0.4)))


def thermo_board(cam, P, t, items, glows):
    th = S.THERMO
    zf = th['z']
    Q = np.array([[th['x0'], th['y0'], zf], [th['x1'], th['y0'], zf], [th['x1'], th['y1'], zf], [th['x0'], th['y1'], zf]])
    s, z = proj_poly(cam, Q)
    if s is None:
        return
    items.append((z - 0.5, 'poly', s, shade(P, np.array([0, 0, 1.0]), (236, 236, 226), 0.3)))
    xc = (th['x0'] + th['x1']) / 2
    Tn = T.temp(t)
    yb = th['y0'] + 3.5
    tube = np.array([[xc - 0.7, yb, zf + 0.2], [xc + 0.7, yb, zf + 0.2], [xc + 0.7, th['y1'] - 2.5, zf + 0.2], [xc - 0.7, th['y1'] - 2.5, zf + 0.2]])
    s2, z2 = proj_poly(cam, tube)
    if s2 is not None:
        items.append((z - 0.6, 'poly', s2, (200, 210, 220)))
    lvl = S.thermo_y(Tn)
    merc = np.array([[xc - 0.45, yb, zf + 0.3], [xc + 0.45, yb, zf + 0.3], [xc + 0.45, lvl, zf + 0.3], [xc - 0.45, lvl, zf + 0.3]])
    s3, z3 = proj_poly(cam, merc)
    mc = merc_color(Tn)
    if s3 is not None:
        items.append((z - 0.7, 'poly', s3, mc))
    bulb = np.array([[xc + 1.6 * math.cos(a), yb + 1.6 * math.sin(a), zf + 0.35] for a in np.linspace(0, 2 * math.pi, 9)[:-1]])
    s4, z4 = proj_poly(cam, bulb)
    if s4 is not None:
        items.append((z - 0.8, 'poly', s4, mc))
        c = s4.mean(0)
        glows.append((c[0], c[1], max(3, (s4[:, 0].max() - s4[:, 0].min()) * 1.5), mc, 0.35))
    for Tm in range(40, 110, 10):
        y = S.thermo_y(Tm)
        tick = cam.wp(np.array([[xc + 1.2, y, zf + 0.2], [xc + 3.2, y, zf + 0.2]]))
        items.append((z - 0.65, 'line', tick, (60, 60, 80), 1))


def merc_color(Tn):
    u = (Tn - 55) / 35
    stops = [(0.0, (70, 220, 255)), (0.35, (160, 120, 255)), (0.65, (255, 80, 170)), (1.0, (255, 70, 50))]
    for (a, ca), (b, cb) in zip(stops, stops[1:]):
        if u <= b:
            v = (max(u, a) - a) / (b - a)
            return tuple(int(x) for x in mixc(ca, cb, v))
    return stops[-1][1]


# ----------------------------------------------------------------- hero cricket
def build_hero(cam, P, t, items, glows, alpha_items):
    cp = cam.pos
    if np.linalg.norm(HF - cp) > 400:
        return
    env = C.chirp_env(t, HERO_CHIRPS)
    cr = C.build(t, env, P.h)
    # stem + umbel
    for a, b in zip(C.STEM, C.STEM[1:]):
        s = cam.wp(np.array([a, b]))
        z = cam.to_cam(((a + b) / 2)[None])[0, 2]
        if z > NEAR:
            items.append((z, 'line', s, tuple(shade(P, np.array([0, 0, 1.0]), (90, 190, 90), 0.6)), max(1, int(0.75 * cam.f / z))))
    u = cam.to_cam(C.UMBEL[None])[0]
    if u[2] > NEAR:
        x, y = cam.proj(u[None])[0]
        items.append((u[2], 'flower', (x, y, 2.6 * cam.f / u[2], (250, 250, 255), (255, 240, 200), 0), None))
    # leaf
    cam_above = cp[1] > S.HERO[1] - 0.45 or (B[9] <= t < B[11])
    hero_z = cam.to_cam(S.HERO[None])[0, 2]
    for Q, n, i, sgn in C.LEAF:
        s, z = proj_poly(cam, Q)
        if s is None:
            continue
        nn = n if np.dot(n, cp - Q.mean(0)) > 0 else -n
        col = shade(P, nn, (70, 214, 110) if (i + (sgn > 0)) % 2 else (60, 196, 104), 0.7)
        zz = hero_z + 2.0 if cam_above else hero_z - 2.0
        items.append((zz + 0.001 * z, 'poly', s, col))
    # body
    for Q, n, col in cr['faces']:
        ctr = Q.mean(0)
        if np.dot(n, cp - ctr) <= 0:
            continue
        s, z = proj_poly(cam, Q)
        if s is None:
            continue
        c = shade(P, n, col, 0.9)
        c = c + np.array(P.ring) * 0.25 * env
        items.append((z, 'poly', s, np.clip(c, 0, 255)))
    for E in cr['eyes']:
        s, z = proj_poly(cam, E)
        if s is not None:
            items.append((z - 0.01, 'poly', s, (40, 30, 50)))
    # legs + antennae (thin)
    for a, b, w, sgn in cr['legs']:
        za = cam.to_cam(np.array([a, b]))
        if (za[:, 2] < NEAR).any():
            continue
        s = cam.proj(za)
        px = max(1, int(round(w * cam.f / za[:, 2].mean())))
        items.append((za[:, 2].mean(), 'line', s, tuple(shade(P, np.array([0, 0.3, sgn * 0.9]), (170, 226, 140), 0.8)), px))
    for pts, sgn in cr['ant']:
        Pc = cam.to_cam(np.array(pts))
        if (Pc[:, 2] < NEAR).any():
            continue
        s = cam.proj(Pc)
        items.append((Pc[:, 2].mean(), 'line', s, (210, 236, 170), 1))
    # wings: translucent
    for Wp, sgn in cr['wings']:
        s, z = proj_poly(cam, Wp)
        if s is None:
            continue
        col = mixc((200, 255, 210), P.ring, 0.35 + 0.5 * env)
        alpha_items.append((z, s, col, 0.45 + 0.35 * env))
    # where the sound comes from (top of the wings)
    wt = cam.to_cam((S.HERO + np.array([0.0, 0.9, 0.0]))[None])[0]
    return env, (cam.proj(wt[None])[0] if wt[2] > NEAR else None), wt[2]


# ----------------------------------------------------------------- HUD
def panel(draw, x0, y0, x1, y1, edge=(90, 70, 160), fill=(18, 12, 40)):
    draw.rectangle([x0 + 3, y0 + 3, x1 + 3, y1 + 3], fill=(6, 4, 16))
    draw.rectangle([x0, y0, x1, y1], fill=fill)
    draw.rectangle([x0, y0, x1, y1], outline=edge)
    draw.rectangle([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=tuple(int(c * 0.6) for c in edge))


THX0, THY0, THY1 = 22, 176, 392


def thermo_hud(draw, t):
    Tn = T.temp(t)
    a = 1.0
    x0 = THX0
    # tube
    draw.rectangle([x0 - 3, THY0 - 6, x0 + 19, THY1 + 30], fill=(6, 4, 16))
    draw.rectangle([x0, THY0 - 4, x0 + 16, THY1 + 4], fill=(30, 24, 60), outline=(150, 140, 220))
    draw.ellipse([x0 - 6, THY1 + 2, x0 + 22, THY1 + 30], fill=(30, 24, 60), outline=(150, 140, 220))
    mc = merc_color(Tn)
    lvl = THY1 - (Tn - 50) / 45 * (THY1 - THY0)
    draw.rectangle([x0 + 4, lvl, x0 + 12, THY1 + 8], fill=mc)
    draw.ellipse([x0 - 2, THY1 + 6, x0 + 18, THY1 + 26], fill=mc)
    draw.rectangle([x0 + 5, lvl + 2, x0 + 6, THY1 + 2], fill=tuple(min(255, c + 70) for c in mc))
    for Tm in range(50, 96, 5):
        y = THY1 - (Tm - 50) / 45 * (THY1 - THY0)
        draw.line([x0 + 17, y, x0 + (24 if Tm % 10 == 0 else 21), y], fill=(150, 140, 220))
    return lvl, mc


def thermo_hud_text(img, t, lvl, mc):
    Tn = T.temp(t)
    for Tm in range(60, 96, 10):
        y = int(THY1 - (Tm - 50) / 45 * (THY1 - THY0))
        stamp(img, text_mask(str(Tm), 1), THX0 + 27, y - 3, (190, 180, 240))
    s = f"{int(round(Tn))}°F"
    sc = 3
    # pop on each 5-degree milestone
    for ms in range(60, 91, 5):
        tm = milestone_time(ms)
        if tm is not None and 0 <= t - tm < 0.25:
            sc = 4
    m = text_mask(s, sc)
    stamp(img, m, 14, THY0 - 34 - (sc - 3) * 4, mc)


_MS = {}


def milestone_time(ms):
    if ms not in _MS:
        tt = None
        for f in range(int(T.T_RISE0 * FPS), int(T.T_RISE1 * FPS) + 2):
            if T.temp(f / FPS) >= ms - 1e-6:
                tt = f / FPS
                break
        _MS[ms] = tt
    return _MS[ms]


PY0 = 470


def formula_panel(draw, t):
    if t < T.T_FF0:
        return None
    if T.T_FF0 <= t < T.T_FF1 + 0.95:
        panel(draw, 14, PY0, 346, PY0 + 74)
        return 'ff'
    if t >= 57.0:
        return None
    panel(draw, 14, PY0, 346, PY0 + 74)
    return 'live'


def formula_text(img, draw_unused, t, mode):
    if mode == 'ff':
        if t < T.T_FF1:
            u = (t - T.T_FF0) / (T.T_FF1 - T.T_FF0)
            secs = u * 15
            n = sum(1 for c in T.CHIRPS if c[1] == 'ff' and c[0] <= t)
            # fast-forward badge
            stamp(img, text_mask(">> 4X SPEED", 2), 24, PY0 + 8, AMBER)
            stamp(img, text_mask(f"{secs:4.1f} S", 2), 334 - 6 * 12, PY0 + 8, CREAM)
            img[PY0 + 28:PY0 + 36, 24:336] = (40, 30, 80)
            img[PY0 + 28:PY0 + 36, 24:24 + int(312 * u)] = AMBER
            for k in range(16):
                x = 24 + int(312 * k / 15)
                img[PY0 + 37:PY0 + 40, x:x + 1] = (150, 140, 220)
            m = text_mask(f"{n}", 4)
            stamp(img, m, 24, PY0 + 44, MINT)
            stamp(img, text_mask("CHIRPS", 2), 24 + m.shape[1] + 10, PY0 + 54, CREAM)
        else:
            u = t - T.T_FF1
            sc = 4 if u > 0.12 else 5
            line = [("15", MINT), (" + 40 = ", CREAM), ("55°F", merc_color(55))]
            total = sum(len(s) for s, _ in line) * ADV * sc
            x = 180 - total // 2
            for s, col in line:
                m = text_mask(s, sc)
                stamp(img, m, x, PY0 + 24 - (sc - 4) * 3, col)
                x += len(s) * ADV * sc
            stamp(img, text_mask("CHIRPS IN 15 S", 1), 34, PY0 + 8, (190, 180, 240))
        return
    Tn = T.temp(t)
    n = int(round(Tn - 40))
    stamp(img, text_mask("CHIRPS IN 15 S", 2), 24, PY0 + 8, (190, 180, 240))
    stamp(img, text_mask("TEMP", 2), 268, PY0 + 8, (190, 180, 240))
    m = text_mask(f"{n}", 5)
    stamp(img, m, 24, PY0 + 28, MINT)
    stamp(img, text_mask("+40 =", 3), 24 + 70, PY0 + 34, CREAM)
    s = f"{int(round(Tn))}°F"
    m2 = text_mask(s, 4)
    stamp(img, m2, 334 - m2.shape[1], PY0 + 32, merc_color(Tn))


def badge(img, draw, t):
    """genre + tempo badge above the formula panel"""
    if t < T.BAR_T[0] + 0.1 or t >= T.T_COOL0:
        return
    b = T.bar_at(t)
    g = T.BAR_GENRE[b]
    name = T.GENRES[g][1]
    if b >= 20:
        name = 'MAX TEMPO'
    Tn = T.temp(t)
    cols = [(90, 170, 255), (180, 120, 255), (255, 160, 60), (60, 230, 200), (110, 140, 255), (255, 90, 160), (255, 220, 70)]
    col = cols[g] if b < 20 else (255, 80, 80)
    first = b == 0 or T.BAR_GENRE[b - 1] != g or (b == 20)
    pop = math.exp(-(t - T.BAR_T[b]) * 8) if first else 0.0
    m = text_mask(name, 2)
    w = m.shape[1] + 16
    y0 = PY0 - 26 - int(4 * pop)
    draw_box = Image.fromarray(img)
    dr = ImageDraw.Draw(draw_box)
    dr.rectangle([16, y0 + 2, 16 + w + 2, y0 + 24], fill=(6, 4, 16))
    dr.rectangle([14, y0, 14 + w, y0 + 22], fill=col)
    img[:] = np.asarray(draw_box)
    stamp(img, m, 22, y0 + 4, INK, outline=None)
    bpm = f"{int(round(T.bpm(Tn)))} BPM"
    mb = text_mask(bpm, 2)
    stamp(img, mb, 346 - mb.shape[1], y0 + 4, col)


def callout(img, draw, x, y, tx, ty, label, col):
    draw.line([(x, y), (tx, ty)], fill=col, width=1)


# ----------------------------------------------------------------- captions
def caps():
    c = [
        (0.0, T.T_FF0 - 0.05, ["CHIRPS + 40 = °F", "THIS CRICKET IS A", "*THERMOMETER*"], 3),
        (T.T_FF0, T.T_FF1, ["COUNT ITS CHIRPS", "FOR *15 SECONDS*..."], 3),
        (T.T_FF1, 10.3, ["...ADD *40*:", "IT'S *55°F*. SPOT ON!"], 3),
        (10.3, 13.6, ["*1897*: PHYSICIST AMOS", "DOLBEAR WROTE IT DOWN"], 2),
        (13.6, B[3], ["NOW LET'S *WARM UP*", "THE NIGHT..."], 3),
        (B[3], B[5], ["THE CHIRPS ARE", "THE *HI-HAT*"], 3),
        (B[5], B[7], ["ONLY *MALES* SING:", "WINGS UP, RUBBED", "*TOGETHER*"], 3),
        (B[7], B[10], ["NEIGHBORS *SYNC UP*:", "THE MEADOW CHIRPS", "*IN UNISON*"], 3),
        (B[10], B[13], ["WHY FASTER? THEY'RE", "*COLD-BLOODED*"], 2),
        (B[13], B[16], ["WARMER BODY =", "*FASTER WINGS*"], 3),
        (B[16], B[20], ["IT'S GETTING", "*HOT* IN HERE..."], 3),
        (B[20], T.T_COOL0, ["*90°F* = *200*", "CHIRPS A MINUTE!"], 3),
        (T.T_COOL0, T.T_COOL1, ["...UNTIL THE NIGHT", "*COOLS DOWN*"], 3),
        (T.T_COOL1, 57.0, ["IN °C? COUNT FOR", "*8 SECONDS*, ADD *5*"], 2),
        (57.0, 60.0, ["TRY IT", "*TONIGHT*!"], 4),
    ]
    return c


CAPS = caps()


def draw_captions(img, t, P):
    hi = AMBER
    for t0, t1, lines, sc in CAPS:
        if t0 <= t < t1:
            u = t - t0
            nrev = int(u * 60) + 1
            slide = int(round(6 * (1 - ease(u / 0.22))))
            y = 40 + slide
            used = 0
            for k, ln in enumerate(lines):
                pl = len(ln.replace('*', '').replace('~', ''))
                s2 = sc
                if t0 == 0.0 and k > 0:
                    s2 = 2
                if pl * ADV * s2 > 344:
                    s2 = 2
                rich_line(img, ln, W // 2, y, s2, CREAM, hi, PINK, nchars=nrev - used)
                used += pl
                y += 7 * s2 + 7


# ----------------------------------------------------------------- frame
VIG = None
SCRIM = None


def vignette():
    global VIG
    if VIG is None:
        r = np.hypot((XX - W / 2) / (W * 0.62), (YY - H * 0.47) / (H * 0.62))
        v = np.clip(r - 0.55, 0, 1) * 0.9
        v = np.floor(v * 5 + BAYER_FULL * 0.999) / 5
        VIG = (1 - np.clip(v, 0, 0.5))[..., None]
    return VIG


def top_scrim():
    global SCRIM
    if SCRIM is None:
        v = np.clip(1 - (YY - 40) / 150.0, 0, 1) * 0.55
        v = np.floor(v * 6 + BAYER_FULL * 0.999) / 6
        SCRIM = (1 - v)[..., None]
    return SCRIM


def chorus_offsets(t):
    """sync emerges around 28-32 s"""
    k = 1 - ss(B[7] - 0.5, B[8] + 0.5, t)
    return S.CHORUS[:, 3] * (0.08 + 0.92 * k)


def render(f):
    t = f / FPS
    P = Pal(t)
    cam = cam_at(t)
    skyimg = sky(cam, P, t)
    draw_stars(skyimg, cam, P, t)
    pil = Image.fromarray(skyimg.astype(np.uint8))
    draw = ImageDraw.Draw(pil)
    glows = []
    # moon
    ms = screen_dir(cam, MOON_D)
    if ms is not None:
        r = 0.058 * cam.f
        pts = [(ms[0] + r * math.cos(2 * math.pi * k / 14), ms[1] + r * math.sin(2 * math.pi * k / 14)) for k in range(14)]
        draw.polygon(pts, fill=tuple(int(c) for c in P.moon))
        for (dx, dy, rr) in ((-0.3, -0.2, 0.22), (0.25, 0.15, 0.18), (0.05, 0.42, 0.14), (-0.35, 0.3, 0.1)):
            cx, cy = ms[0] + dx * r, ms[1] + dy * r
            draw.polygon([(cx + rr * r * math.cos(a), cy + rr * r * math.sin(a)) for a in np.linspace(0, 2 * math.pi, 8)[:-1]],
                         fill=tuple(int(c * 0.86) for c in P.moon))
        glows.append((ms[0], ms[1], r * 2.6, tuple(P.moon), 0.22))
    # shooting stars
    for ts0, az0, el0 in ((10.6, -60, 50), (48.9, 30, 45)):
        if ts0 <= t < ts0 + 0.7:
            u = (t - ts0) / 0.7
            a = screen_dir(cam, dirv(az0 + 30 * u, el0 - 12 * u))
            b = screen_dir(cam, dirv(az0 + 30 * max(0, u - 0.3), el0 - 12 * max(0, u - 0.3)))
            if a is not None and b is not None:
                draw.line([tuple(b), tuple(a)], fill=(255, 255, 255), width=2)
                glows.append((a[0], a[1], 6, (255, 220, 255), 0.6 * (1 - u)))
    # clouds
    for blobs, spd in S.CLOUDS:
        for bi, pts in enumerate(blobs):
            sp = []
            for az, el in pts:
                s = screen_dir(cam, dirv(az + spd * t, el))
                if s is None:
                    sp = None
                    break
                sp.append(tuple(s))
            if sp:
                cl = mixc(P.zen, P.hor, 0.55 + 0.15 * bi)
                draw.polygon(sp, fill=tuple(int(c) for c in cl))

    items = []
    alpha_items = []
    build_world(cam, P, t, items, glows)
    hero = build_hero(cam, P, t, items, glows, alpha_items)
    items.sort(key=lambda it: -it[0])
    # alpha (wings) are drawn right after the nearest-farther opaque item
    alpha_items.sort(key=lambda it: -it[0])
    ai = 0
    for it in items:
        while ai < len(alpha_items) and alpha_items[ai][0] > it[0]:
            z, s, col, a = alpha_items[ai]
            draw_alpha(pil, s, col, a)
            draw = ImageDraw.Draw(pil)
            ai += 1
        kind = it[1]
        if kind in ('poly', 'blade'):
            draw.polygon([tuple(p) for p in it[2]], fill=tuple(int(c) for c in it[3]))
        elif kind == 'line':
            draw.line([tuple(p) for p in it[2]], fill=tuple(int(c) for c in it[3]), width=int(it[4]))
        elif kind == 'flower':
            x, y, r, col, cen, k = it[2]
            col = tuple(int(c) for c in (np.array(col) * (P.amb * 0.6 + 0.55)).clip(0, 255))
            if r < 1.2:
                draw.point((x, y), fill=col)
            else:
                n = 8 if k != 1 else 6
                draw.polygon([(x + r * math.cos(2 * math.pi * j / n + 0.3), y + r * math.sin(2 * math.pi * j / n + 0.3)) for j in range(n)], fill=col)
                rc = r * 0.38
                draw.polygon([(x + rc * math.cos(2 * math.pi * j / 6), y + rc * math.sin(2 * math.pi * j / 6)) for j in range(6)], fill=cen)
    while ai < len(alpha_items):
        z, s, col, a = alpha_items[ai]
        draw_alpha(pil, s, col, a)
        ai += 1

    img = np.asarray(pil).astype(np.float32)
    # fireflies
    nff = int(40 + 100 * P.h)
    for k in range(len(S.FF)):
        if k >= 14 and k - 14 >= nff:
            continue
        x, y, z, amp, ph, sp = S.FF[k]
        p = np.array([x + amp * 0.6 * math.sin(t * sp + ph), y + amp * 0.25 * math.sin(t * sp * 1.7 + ph * 2), z + amp * 0.6 * math.cos(t * sp * 0.8 + ph)])
        pc = cam.to_cam(p[None])[0]
        if pc[2] < NEAR * 4:
            continue
        sx, sy = cam.proj(pc[None])[0]
        blink = max(0.0, math.sin(t * (1.6 + sp) + ph * 3))
        if blink < 0.15:
            continue
        rr = min(14, max(2.2, 3.0 * cam.f / pc[2]))
        glows.append((sx, sy, rr * 2.2, tuple(P.ff), 0.55 * blink))
        if 0 <= sx < W and 0 <= sy < H:
            img[int(sy), int(sx)] = np.maximum(img[int(sy), int(sx)], np.array([255, 255, 210]) * blink)
    # chorus crickets: rings + flashes, syncing up
    offs = chorus_offsets(t)
    beat_t = [c[0] for c in T.CHIRPS if c[1] in ('beat', 'cool', 'real')]
    for k in range(len(S.CHORUS)):
        x, y, z, _ = S.CHORUS[k]
        pc = cam.to_cam(np.array([[x, y, z]]))[0]
        if pc[2] < 30:
            continue
        bl = beat_len(t)
        lt = last_chirp(t - offs[k] * bl, beat_t)
        age = t - offs[k] * bl - lt
        life = min(0.45, 0.85 * bl)
        if 0 <= age < life:
            u = age / life
            R = (10 + 60 * u) * max(1.0, pc[2] / 700)
            ring = np.array([[x + R * math.cos(a), y - 18, z + R * math.sin(a)] for a in np.linspace(0, 2 * math.pi, 17)])
            Pr = cam.to_cam(ring)
            if (Pr[:, 2] > NEAR).all():
                sp = cam.proj(Pr)
                lay = Image.new('L', (W, H), 0)
                ImageDraw.Draw(lay).line([tuple(p) for p in sp], fill=int(255 * (1 - u)), width=1)
                img += np.asarray(lay, np.float32)[..., None] / 255 * np.array(P.ring)[None, None] * 0.9
            sx, sy = cam.proj(pc[None])[0]
            glows.append((sx, sy, max(2, 6 * cam.f / pc[2]), P.ring, 0.8 * (1 - u)))
    for x, y, r, col, inten in glows:
        add_glow(img, x, y, r, col, inten)
    # hero chirp rings + pops
    if hero is not None:
        env, wsp, wz = hero
        if wsp is not None:
            lt = last_chirp(t)
            age = t - lt
            bl = 1.0 if t < T.T_MUSIC else beat_len(t)
            if T.T_FF0 <= t < T.T_FF1:
                bl = 0.25
            life = min(0.42, 0.9 * bl)
            if 0 <= age < life:
                u = age / life
                for j, (r0, r1) in enumerate(((6, 70), (2, 44))):
                    add_ring(img, wsp[0], wsp[1], r0 + (r1 - r0) * ease(u), 2, P.ring, (1 - u) * (0.9 - 0.3 * j))
                add_glow(img, wsp[0], wsp[1], 22, P.ring, 0.25 * (1 - u))
    # beat flash on the genre switches
    b = T.bar_at(t)
    if 0 <= b < T.NBARS and (b == 0 or T.BAR_GENRE[b] != T.BAR_GENRE[b - 1] or b == 20):
        k = math.exp(-(t - T.BAR_T[b]) / 0.07) * 0.3
        img = img * (1 - k) + np.array([255, 245, 255]) * k
    img = np.clip(img, 0, 255)
    # heat shimmer when it's hot
    if P.h > 0.72 and t < T.T_COOL1:
        kk = (P.h - 0.72) / 0.28
        shift = np.round(kk * 2.0 * np.sin(np.arange(H) * 0.21 + t * 11)).astype(int)
        rows = np.arange(H)
        band = (rows > 200) & (rows < H)
        for y in np.nonzero(band & (shift != 0))[0]:
            img[y] = np.roll(img[y], shift[y], axis=0)
    # saturation pop
    gray = img.mean(-1, keepdims=True)
    img = gray + (img - gray) * 1.12
    img = np.clip(img, 0, 255) * vignette() * top_scrim()

    # UI ------------------------------------------------------------
    pil = Image.fromarray(np.clip(img, 0, 255).astype(np.uint8))
    draw = ImageDraw.Draw(pil)
    lvl, mc = thermo_hud(draw, t)
    mode = formula_panel(draw, t)
    # cricket callout in the hook
    call = None
    if t < T.T_FF0 and hero is not None and hero[1] is not None:
        hx = cam.wp(S.HERO[None])[0]
        draw.line([(hx[0] + 20, hx[1] + 40), (240, 432)], fill=MINT, width=2)
        draw.rectangle([128, 432, 346, 456], fill=MINT)
        call = True
    out = np.asarray(pil).copy()
    thermo_hud_text(out, t, lvl, mc)
    if mode:
        formula_text(out, None, t, mode)
    badge(out, None, t)
    if call:
        stamp(out, text_mask("SNOWY TREE CRICKET", 2), 133, 437, INK, outline=None)
    # "CHIRP!" pops in the hook
    if t < T.T_FF0 and hero is not None and hero[1] is not None:
        lt = last_chirp(t)
        age = t - lt
        if 0 <= age < 0.45:
            k = HERO_CHIRPS.index(lt)
            m = text_mask("CHIRP!", 2 if age > 0.06 else 3)
            x = int(hero[1][0] + (40 if k % 2 else -110))
            y = int(hero[1][1] - 40 - age * 30)
            stamp(out, m, x, y, CYAN if k % 2 else PINK)
    draw_captions(out, t, P)
    return out


def draw_alpha(pil, s, col, a):
    mask = Image.new('L', (W, H), 0)
    ImageDraw.Draw(mask).polygon([tuple(p) for p in s], fill=int(255 * a))
    pil.paste(tuple(int(c) for c in col), (0, 0, W, H), mask)


if __name__ == '__main__':
    if sys.argv[1] == 'sheet':
        ts = [float(x) for x in sys.argv[3:]]
        cols = 7
        rows = (len(ts) + cols - 1) // cols
        sheet = Image.new('RGB', (cols * 366, rows * 646), (255, 255, 255))
        for k, tt in enumerate(ts):
            im = Image.fromarray(render(int(round(tt * FPS))))
            sheet.paste(im, ((k % cols) * 366, (k // cols) * 646))
        sheet.save(sys.argv[2])
        print('saved', sys.argv[2])
    elif sys.argv[1] == 'frame':
        tt = float(sys.argv[3])
        Image.fromarray(render(int(round(tt * FPS)))).resize((W * UP, H * UP), Image.NEAREST).save(sys.argv[2])
