"""Sand That Draws Sound: low-poly desert ruin, night to sunrise.
Flat-shaded 3D rendered at 360x640 and upscaled 3x nearest-neighbour."""
import math, os, sys
import numpy as np
from PIL import Image, ImageDraw
import timeline as T
from modes import mode_val_grad, nodal_image
from font import text_mask, stamp, rich_line

W, H = 360, 640
UP = 3
FPS = T.FPS
DRV = np.load('drive.npz')
G = int(DRV['G'])
SAND = np.memmap('sand_pos.f32', dtype=np.float32, mode='r', shape=(T.NF, G, 2))
SANDH = np.memmap('sand_h.f16', dtype=np.float16, mode='r', shape=(T.NF, G))
VDRV = DRV['V']

PY = 1.25            # plate top height
PS = 1.0             # plate half size
rng = np.random.default_rng(8)

CREAM = (252, 240, 214)
MINT = (124, 246, 200)
CORAL = (255, 118, 96)
AMBER = (255, 198, 92)
SKYB = (140, 200, 255)
INK = (12, 16, 30)


def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def ease(x):
    return T.ease(x)


def lerp(a, b, u):
    return a + (b - a) * u


def lerp3(a, b, u):
    return tuple(lerp(np.array(a, float), np.array(b, float), u))


# ----------------------------------------------------------------- time of day
NU_T = [0, 13, 22, 30, 36, 41, 43.4, 47, 60]
NU_V = [0, 0, 0.2, 0.42, 0.6, 0.74, 0.84, 1.0, 1.0]
STOPS = [0.0, 0.5, 0.8, 1.0]
PAL = {
    'zen':  [(7, 11, 30), (30, 42, 88), (66, 92, 146), (96, 142, 192)],
    'hsun': [(26, 36, 70), (204, 112, 112), (255, 168, 96), (255, 214, 156)],
    'haway': [(18, 26, 54), (86, 82, 130), (152, 132, 162), (190, 194, 208)],
    'amb':  [(0.14, 0.19, 0.38), (0.32, 0.29, 0.43), (0.43, 0.37, 0.43), (0.50, 0.47, 0.47)],
}


def pal(name, nu):
    st = PAL[name]
    for k in range(3):
        if nu <= STOPS[k + 1]:
            u = (nu - STOPS[k]) / (STOPS[k + 1] - STOPS[k])
            return np.array(lerp3(st[k], st[k + 1], u))
    return np.array(st[-1], float)


def dirv(az, el):
    a, e = math.radians(az), math.radians(el)
    return np.array([math.sin(a) * math.cos(e), math.sin(e), -math.cos(a) * math.cos(e)])


SUN_AZ = 9.0
MOON_AZ, MOON_EL = -13.0, 22.0


def sun_el(t):
    return float(np.interp(t, [0, 20, 35, 41, 43.4, 47, 52, 60], [-14, -11, -6, -2.0, 1.9, 5.0, 7.5, 10.0]))


class Light:
    def __init__(self, t):
        nu = float(np.interp(t, NU_T, NU_V))
        self.t, self.nu = t, nu
        self.zen = pal('zen', nu)
        self.hsun = pal('hsun', nu)
        self.haway = pal('haway', nu)
        self.amb = pal('amb', nu)
        self.sun_el = sun_el(t)
        self.sun_dir = dirv(SUN_AZ, self.sun_el)
        self.moon_dir = dirv(MOON_AZ, MOON_EL)
        mk = float(np.interp(t, [0, 22, 34, 42, 46], [0.62, 0.55, 0.32, 0.1, 0.0]))
        sk = 1.05 * ss(42.8, 46.5, t)
        suncol = np.array(lerp3((255, 172, 104), (255, 220, 170), ss(44, 58, t))) / 255
        fk = 0.30 * ss(20, 40, t) * (1 - ss(46, 52, t))
        side = dirv(-115, 38)
        self.lights = [(self.moon_dir, np.array([0.40, 0.50, 0.80]) * mk),
                       (side, np.array([0.45, 0.55, 0.85]) * mk * 0.9),
                       (np.array([0.2, 0.28, -1]) / np.linalg.norm([0.2, 0.28, -1]), np.array([1.0, 0.6, 0.5]) * fk)]
        if sk > 0:
            sd = dirv(SUN_AZ, max(self.sun_el, 3.0))
            self.lights.append((sd, suncol * sk))
        flick = 0.9 + 0.1 * math.sin(t * 13.1) * math.sin(t * 7.3 + 1)
        self.brazier_k = (1.0 - 0.62 * ss(38, 48, t)) * flick
        self.star_a = 1 - ss(18, 34, t)
        self.window_k = 1 - ss(40, 47, t)
        self.sun_k = sk

    def horizon(self, az_rel_sun):
        """horizon colour for azimuth difference to the sun (radians)."""
        u = (1 + np.cos(az_rel_sun)) / 2
        u = u ** 2.2
        return self.haway[None] * (1 - u[:, None]) + self.hsun[None] * u[:, None]


BRAZIERS = [np.array([-2.35, 1.35, -1.55]), np.array([2.35, 1.35, -1.55])]
FIRE = np.array([1.0, 0.62, 0.30])


def shade(L, nrm, ctr, base, braz=True, gloss=0.0):
    I = np.tile(L.amb, (len(nrm), 1))
    for d, c in L.lights:
        I += np.clip(nrm @ d, 0, None)[:, None] * c[None]
    if braz and L.brazier_k > 0.01:
        for bp in BRAZIERS:
            v = bp[None] - ctr
            dist = np.linalg.norm(v, axis=1) + 1e-6
            lam = np.clip((nrm * v).sum(1) / dist, 0, None)
            I += (lam * 0.85 * L.brazier_k / (1 + (dist / 1.6) ** 2))[:, None] * FIRE[None]
    col = base * I
    return np.clip(np.round(col / 5) * 5, 0, 255)


def fog(L, col, ctr, campos, d0=24, d1=150, mx=0.85):
    d = np.linalg.norm(ctr - campos[None], axis=1)
    k = np.clip((d - d0) / (d1 - d0), 0, mx)[:, None]
    az = np.arctan2(ctr[:, 0], -ctr[:, 2]) - math.radians(SUN_AZ)
    hc = L.horizon(az)
    return col * (1 - k) + hc * k


# ----------------------------------------------------------------- camera
class Cam:
    def __init__(self, tgt, d, yaw, pitch, fov, sy, roll=0.0):
        yr, pr = math.radians(yaw), math.radians(pitch)
        self.pos = tgt + d * np.array([math.sin(yr) * math.cos(pr), math.sin(pr), math.cos(yr) * math.cos(pr)])
        fwd = tgt - self.pos
        fwd /= np.linalg.norm(fwd)
        right = np.cross(fwd, [0, 1, 0]); right /= np.linalg.norm(right)
        up = np.cross(right, fwd)
        if roll:
            c, s = math.cos(math.radians(roll)), math.sin(math.radians(roll))
            right, up = right * c + up * s, -right * s + up * c
        self.M = np.stack([right, up, fwd])
        self.f = (H / 2) / math.tan(math.radians(fov) / 2)
        self.cx, self.cy = W / 2, sy

    def to_cam(self, P):
        return (P - self.pos) @ self.M.T

    def proj(self, Pc):
        z = np.maximum(Pc[..., 2], 1e-3)
        return np.stack([self.cx + self.f * Pc[..., 0] / z, self.cy - self.f * Pc[..., 1] / z], -1)

    def world_proj(self, P):
        return self.proj(self.to_cam(P))


NEAR = 0.1


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


def poly(draw, cam, P, col):
    Pc = cam.to_cam(P)
    if (Pc[:, 2] < NEAR).any():
        if (Pc[:, 2] < NEAR).all():
            return
        Pc = clip_near(Pc)
        if Pc is None:
            return
    s = cam.proj(Pc)
    draw.polygon([tuple(p) for p in s], fill=tuple(int(c) for c in col))


# ----------------------------------------------------------------- camera script
def K(t, tx, ty, tz, d, yaw, pitch, fov=40, sy=300, roll=0.0):
    return (t, np.array([tx, ty, tz], float), d, yaw, pitch, fov, sy, roll)


TOP = dict()
CAM_KEYS = [
    K(0.0, 0, PY, 0.0, 3.9, 0, 88.5, 40, 300),
    K(2.1, 0, PY, 0.0, 3.75, 4, 88.5, 40, 300),
    K(4.15, 0, PY, 0.0, 3.6, 8, 88.5, 40, 300),
    K(5.9, 0, 3.2, -2.5, 14.5, 16, 3, 60, 372),
    K(7.25, 0, 3.2, -2.5, 14.0, -12, 3, 60, 372),
    K(8.3, 0, PY, 0.0, 4.6, 26, 33, 44, 320),
    K(14.4, 0, PY, 0.0, 4.4, 42, 30, 44, 320),
    K(15.6, 0, PY, 0.0, 6.6, 22, 30, 56, 330),
    K(23.5, 0, PY, 0.0, 6.0, -8, 36, 56, 325),
    K(27.6, 0, PY, 0.0, 5.3, -20, 46, 52, 315),
    K(30.2, 0, PY, 0.0, 4.6, -22, 62, 46, 305),
    K(31.4, 0, PY, 0.0, 4.5, -14, 82, 42, 300),
    K(32.7, 0, PY, 0.0, 4.4, -12, 84, 42, 300),
    K(34.2, 0, 2.0, -1.4, 9.6, 0, 5, 56, 300),
    K(35.3, 0, 2.0, -1.4, 9.2, 3, 6, 56, 298),
]
# song: camera segments cut on the bar lines
SONG_SEGS = [
    (T.T_SONG + 0 * T.BAR, K(0, 0, PY, 0, 4.5, 14, 74, 42, 310), K(0, 0, PY, 0, 4.3, -4, 76, 42, 310)),
    (T.T_SONG + 1 * T.BAR, K(0, 0, PY, 0, 5.4, -38, 32, 52, 320), K(0, 0, PY, 0, 5.1, -26, 36, 52, 320)),
    (T.T_SONG + 2 * T.BAR, K(0, 0, PY, 0, 4.1, 0, 87, 40, 305), K(0, 0, PY, 0, 4.0, 18, 87, 40, 305)),
    (T.T_SONG + 3 * T.BAR, K(0, 0, PY + 0.2, -0.5, 6.0, 22, 40, 46, 300), K(0, 0, 2.0, -1.4, 9.0, 4, 6, 56, 290)),
    (T.T_SONG + 4 * T.BAR, K(0, 0, 2.0, -1.4, 9.0, 2, 5, 56, 290), K(0, 0, 2.0, -1.2, 8.4, -3, 8, 56, 285)),
    (T.T_SONG + 5 * T.BAR, K(0, 0, PY, 0, 6.0, 34, 26, 54, 325), K(0, 0, PY, 0, 5.2, 16, 48, 50, 315)),
    (T.T_SONG + 6 * T.BAR, K(0, 0, PY, 0, 4.0, -6, 87, 40, 305), K(0, 0, PY, 0, 3.9, -18, 87, 40, 305)),
    (T.T_SONG + 7 * T.BAR, K(0, 0, PY, 0, 5.8, -34, 30, 54, 320), K(0, 0, PY, 0, 5.4, -24, 36, 52, 318)),
]
OUT_KEYS = [
    K(T.T_FINAL, 0, PY, 0, 5.4, -24, 36, 52, 318),
    K(55.0, 0, PY, 0, 4.0, -6, 86, 40, 305),
    K(57.5, 0, PY, 0, 3.9, 0, 88.5, 40, 300),
    K(60.1, 0, PY, 0, 3.9, 0, 88.5, 40, 300),
]


def interp_keys(keys, t):
    if t <= keys[0][0]:
        return keys[0]
    for a, b in zip(keys, keys[1:]):
        if t <= b[0]:
            u = ease((t - a[0]) / (b[0] - a[0]))
            return tuple([t] + [lerp(a[i], b[i], u) for i in range(1, 8)])
    return keys[-1]


def cam_at(t):
    if t < T.T_SONG:
        k = interp_keys(CAM_KEYS, t)
    elif t < T.T_FINAL:
        k = None
        for i, (t0, ka, kb) in enumerate(SONG_SEGS):
            t1 = SONG_SEGS[i + 1][0] if i + 1 < len(SONG_SEGS) else T.T_FINAL
            if t0 <= t < t1:
                u = (t - t0) / (t1 - t0)
                u = u if i not in (3,) else ease(u)
                k = tuple([t] + [lerp(ka[j], kb[j], u) for j in range(1, 8)])
        if k is None:
            k = interp_keys(OUT_KEYS, t)
    else:
        k = interp_keys(OUT_KEYS, t)
    _, tgt, d, yaw, pitch, fov, sy, roll = k
    tgt = np.array(tgt, float).copy()
    # punches and shakes
    for t0, amp, dur in [(T.HOOK[1][1], 0.05, 0.35), (T.T_SCRAMBLE1, 0.09, 0.5), (T.T_SUNRISE, 0.06, 0.6),
                         (T.T_SCRAMBLE2, 0.06, 0.4)]:
        if t0 <= t < t0 + dur:
            a = amp * (1 - (t - t0) / dur) ** 2
            tgt += a * np.array([math.sin(t * 83), math.sin(t * 71 + 1), math.sin(t * 59 + 2)])
    if T.HOOK[1][1] <= t < T.HOOK[1][1] + 0.25:
        d *= 1 - 0.03 * (1 - (t - T.HOOK[1][1]) / 0.25)
    return Cam(tgt, d, yaw, pitch, fov, sy, roll)


# ----------------------------------------------------------------- geometry builders
def octa_prism(cx, cz, r, y0, y1, n=8, tilt=(0.0, 0.0), rot=0.0):
    ring = [(cx + r * math.cos(rot + 2 * math.pi * k / n), cz + r * math.sin(rot + 2 * math.pi * k / n)) for k in range(n)]
    bot = [np.array([x, y0, z]) for x, z in ring]
    top = [np.array([x, y1 + tilt[0] * (x - cx) + tilt[1] * (z - cz), z]) for x, z in ring]
    faces = []
    for k in range(n):
        k2 = (k + 1) % n
        q = np.array([bot[k], bot[k2], top[k2], top[k]])
        nrm = np.array([(ring[k][0] + ring[k2][0]) / 2 - cx, 0, (ring[k][1] + ring[k2][1]) / 2 - cz])
        nrm /= np.linalg.norm(nrm)
        faces.append((q, nrm))
    tq = np.array(top[::-1])
    tn = np.cross(tq[1] - tq[0], tq[2] - tq[0]); tn /= np.linalg.norm(tn)
    if tn[1] < 0:
        tn = -tn
    faces.append((tq, tn))
    return faces


def box(cx, cy, cz, hx, hy, hz, rot=0.0):
    c, s = math.cos(rot), math.sin(rot)
    def P(x, y, z):
        return np.array([cx + x * c - z * s, cy + y, cz + x * s + z * c])
    V = [P(sx * hx, sy * hy, sz * hz) for sy in (-1, 1) for sx, sz in ((1, 1), (1, -1), (-1, -1), (-1, 1))]
    quads = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7), (0, 3, 2, 1)]
    ctr = np.array([cx, cy, cz])
    faces = []
    for q in quads:
        Q = np.array([V[i] for i in q])
        n = np.cross(Q[1] - Q[0], Q[2] - Q[0]); n /= np.linalg.norm(n)
        if np.dot(n, Q.mean(0) - ctr) < 0:
            n = -n
        faces.append((Q, n))
    return faces


class Obj:
    def __init__(self, faces, color, name='', after=None, braz=True, jitter=0.04):
        self.faces = faces
        self.color = np.array(color, float)
        self.name = name
        self.after = after
        self.braz = braz
        self.ctr = np.mean([f[0].mean(0) for f in faces], axis=0)
        self.tone = 1 + rng.uniform(-jitter, jitter, len(faces))


SANDSTONE = (206, 160, 112)
STONE2 = (188, 150, 112)
OBJS = []
# ruined columns: (x, z, height, broken)
COLS = [(-4.1, -2.2, 3.7, False), (4.2, -2.0, 3.7, False), (-3.0, -4.1, 2.3, True),
        (3.0, -4.2, 3.7, False), (-4.6, 1.5, 1.4, True), (4.7, 2.1, 1.9, True)]
for (x, z, h, broken) in COLS:
    OBJS.append(Obj(box(x, 0.18, z, 0.62, 0.18, 0.62), STONE2))
    tilt = (rng.uniform(-0.35, 0.35), rng.uniform(-0.35, 0.35)) if broken else (0, 0)
    OBJS.append(Obj(octa_prism(x, z, 0.42, 0.36, h, 8, tilt, rot=0.39), SANDSTONE))
    if not broken:
        OBJS.append(Obj(box(x, h + 0.16, z, 0.6, 0.16, 0.6), STONE2))
# fallen column drums
for (x, z, r) in [(2.0, 3.6, 0.3), (-2.6, 3.3, -0.8), (5.6, -0.4, 1.2)]:
    OBJS.append(Obj(box(x, 0.36, z, 0.36, 0.36, 0.55, rot=r), SANDSTONE))
# the arch behind the plate (the sun rises through it)
AX, AZ = 0.0, -6.3
OBJS.append(Obj(box(AX - 2.05, 2.2, AZ, 0.42, 2.2, 0.48), SANDSTONE))
OBJS.append(Obj(box(AX + 2.05, 1.7, AZ, 0.42, 1.7, 0.48), SANDSTONE))
OBJS.append(Obj(box(AX - 0.85, 4.62, AZ, 1.75, 0.26, 0.55, rot=0.0), STONE2))
OBJS.append(Obj(box(AX + 2.4, 0.3, AZ + 1.1, 0.7, 0.3, 0.42, rot=0.5), STONE2))
# low broken wall behind
for k, a in enumerate(np.linspace(-0.95, 0.95, 7)):
    if k == 3:
        continue
    r = 7.6
    x, z = r * math.sin(a), -r * math.cos(a)
    h = [0.9, 0.6, 1.1, 0, 0.7, 1.0, 0.5][k]
    OBJS.append(Obj(box(x, h / 2, z, 0.75, h / 2, 0.35, rot=-a), STONE2))
# pedestal
OBJS.append(Obj(octa_prism(0, 0, 0.78, 0.0, 0.22, 8, rot=0.39), STONE2, 'ped0'))
OBJS.append(Obj(octa_prism(0, 0, 0.55, 0.22, 0.86, 8, rot=0.39), SANDSTONE, 'ped1'))
OBJS.append(Obj(octa_prism(0, 0, 0.66, 0.86, 0.98, 8, rot=0.39), STONE2, 'ped2'))
OBJS.append(Obj(octa_prism(0, 0, 0.30, 0.98, 1.16, 10, rot=0.0), (214, 168, 84), 'driver', jitter=0.0))
# generator box + stone block
OBJS.append(Obj(box(1.75, 0.3, 1.25, 0.45, 0.3, 0.45, rot=0.3), STONE2))
GEN = Obj(box(1.75, 0.82, 1.25, 0.36, 0.22, 0.26, rot=0.3), (96, 92, 98), 'gen', jitter=0.0)
OBJS.append(GEN)
# braziers: bowl on a short stand
for bp in BRAZIERS:
    OBJS.append(Obj(octa_prism(bp[0], bp[2], 0.1, 0.0, 1.0, 6), (70, 58, 50), 'stand'))
    bowl = octa_prism(bp[0], bp[2], 0.42, 1.0, 1.28, 8)
    OBJS.append(Obj(bowl, (176, 118, 62), 'bowl', braz=False))


def plate_faces():
    t0, t1 = PY - 0.035, PY
    return box(0, (t0 + t1) / 2, 0, PS, (t1 - t0) / 2, PS)


PLATE = Obj(plate_faces(), (64, 68, 78), 'plate', jitter=0.0)
OBJS.append(PLATE)

# ----------------------------------------------------------------- courtyard tiles
TILES = []
TS = 1.15
for i in range(-7, 7):
    for j in range(-7, 7):
        cx, cz = (i + 0.5) * TS, (j + 0.5) * TS
        r = math.hypot(cx, cz)
        if r > 6.3:
            continue
        q = []
        for (dx, dz) in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
            q.append(np.array([cx + dx * TS * 0.46 + rng.uniform(-0.05, 0.05), 0.01, cz + dz * TS * 0.46 + rng.uniform(-0.05, 0.05)]))
        sandy = r > 4.9 and rng.uniform() < (r - 4.9) / 1.6
        col = np.array([214, 172, 120]) if sandy else np.array([168, 150, 128]) * rng.uniform(0.9, 1.08)
        TILES.append((np.array(q), col))
TILE_C = np.array([q.mean(0) for q, _ in TILES])
TILE_COL = np.array([c for _, c in TILES])

# ----------------------------------------------------------------- terrain (low-poly dunes)
OASIS = np.array([-8.6, 0.0, -13.5])
KASBAH = np.array([10.5, 0.0, -40.0])


def dune_h(x, z):
    r = math.hypot(x, z)
    base = ss(7.0, 15.0, r)
    amp = min(7.0, 0.5 + 0.11 * r)
    az = math.atan2(x, -z)
    # keep the eastern horizon low so the sunrise can clear it
    amp *= 1 - 0.55 * math.exp(-((az - math.radians(SUN_AZ)) / 0.5) ** 2) * ss(30, 60, r)
    w1 = (0.5 + 0.5 * math.sin(0.17 * x + 0.11 * z + 0.6 * math.sin(0.05 * z))) ** 1.7
    w2 = (0.5 + 0.5 * math.sin(-0.09 * x + 0.21 * z + 1.7)) ** 1.5
    w3 = (0.5 + 0.5 * math.sin(0.31 * x - 0.27 * z + 0.4)) ** 2.0
    h = amp * (0.55 * w1 + 0.30 * w2 + 0.15 * w3) * base
    h *= ss(4.2, 9.0, math.hypot(x - OASIS[0], z - OASIS[2]))
    h *= ss(8.0, 15.0, math.hypot(x - KASBAH[0], z - KASBAH[2]))
    return h


RINGS = [6.3, 7.6, 9.2, 11.0, 13.2, 15.8, 18.8, 22.4, 26.8, 32, 38.5, 46, 55, 66, 80, 98, 122, 155]
SEG = 56
verts = {}
for ri, r in enumerate(RINGS):
    for k in range(SEG):
        th = 2 * math.pi * (k + (0.5 if ri % 2 else 0)) / SEG
        rr = r * (1 + (rng.uniform(-0.06, 0.06) if ri > 0 else 0))
        th += rng.uniform(-0.18, 0.18) * 2 * math.pi / SEG if ri > 0 else 0
        x, z = rr * math.sin(th), -rr * math.cos(th)
        verts[(ri, k)] = np.array([x, dune_h(x, z), z])
TRI = []
for ri in range(len(RINGS) - 1):
    for k in range(SEG):
        a, b = verts[(ri, k)], verts[(ri, (k + 1) % SEG)]
        c, d = verts[(ri + 1, k)], verts[(ri + 1, (k + 1) % SEG)]
        if ri % 2 == 0:
            TRI += [(a, b, c), (b, d, c)]
        else:
            TRI += [(a, b, d), (a, d, c)]
TRI = np.array(TRI)                                        # [n,3,3]
TN = np.cross(TRI[:, 1] - TRI[:, 0], TRI[:, 2] - TRI[:, 0])
TN /= np.linalg.norm(TN, axis=1)[:, None]
TN[TN[:, 1] < 0] *= -1
TC = TRI.mean(1)
TCOL = np.array([216, 170, 116])[None] * rng.uniform(0.95, 1.05, (len(TRI), 1))
# water where the oasis is
TWATER = np.linalg.norm(TC[:, [0, 2]] - OASIS[[0, 2]], axis=1) < 2.4

# oasis pool (low-poly 10-gon) + palms
POOL = np.array([[OASIS[0] + 2.7 * math.cos(a) * rng.uniform(0.85, 1.1), 0.04,
                  OASIS[2] + 1.9 * math.sin(a) * rng.uniform(0.85, 1.1)] for a in np.linspace(0, 2 * math.pi, 11)[:-1]])
PALMS = [(OASIS + np.array([-2.6, 0, -0.8]), 4.6, 0.35, 0.0), (OASIS + np.array([2.4, 0, -1.4]), 3.8, -0.4, 1.3),
         (OASIS + np.array([0.6, 0, 1.9]), 3.2, 0.2, 2.1), (np.array([7.8, 0, -9.0]), 4.0, -0.3, 0.7)]

# kasbah blocks
KAS = []
for (dx, dy, dz, hx, hy, hz) in [(0, 2.2, 0, 4.2, 2.2, 3.0), (-4.6, 3.6, -0.6, 1.2, 3.6, 1.2), (4.4, 3.2, 0.8, 1.1, 3.2, 1.1),
                                  (1.2, 3.0, -2.2, 1.6, 3.0, 1.0), (-1.6, 5.0, -0.2, 0.9, 0.6, 0.9)]:
    KAS += [(Q, n) for Q, n in box(KASBAH[0] + dx, dy, KASBAH[2] + dz, hx, hy, hz, rot=0.25)]
WINDOWS = [KASBAH + np.array(p) for p in [(-4.0, 5.1, 1.4), (-4.0, 3.2, 1.4), (4.9, 4.6, 2.2), (0.6, 2.6, 3.3), (2.2, 2.6, 3.3),
                                          (-1.5, 2.6, 3.4), (1.4, 4.8, -0.9)]]

# far mesas (silhouettes)
MESAS = []
for k in range(18):
    a = math.radians(-160 + k * 20 + rng.uniform(-6, 6))
    if abs(a - math.radians(SUN_AZ)) < 0.3:
        continue
    r = rng.uniform(230, 300)
    w = rng.uniform(0.06, 0.13)
    hgt = rng.uniform(10, 26)
    pts = []
    for (da, hh) in [(-w, 0), (-w * 0.7, hgt * 0.95), (-w * 0.2, hgt), (w * 0.4, hgt * 0.9), (w * 0.75, hgt * 0.7), (w, 0)]:
        pts.append(np.array([r * math.sin(a + da), hh - 2, -r * math.cos(a + da)]))
    MESAS.append(np.array(pts))

# sky elements
STARS = []
for k in range(1500):
    if k < 900:
        az, el = rng.uniform(-180, 180), math.degrees(math.asin(rng.uniform(0.05, 1)))
    else:   # milky way band
        s = rng.uniform(-1, 1)
        az = -70 + 130 * s + rng.normal(0, 6)
        el = 20 + 55 * (1 - s * s) + rng.normal(0, 5)
    STARS.append((dirv(az, el), rng.uniform(0.3, 1.0) ** 1.6 * (0.8 if k >= 900 else 1.0), rng.uniform(0, 6.28)))
STAR_D = np.array([s[0] for s in STARS]); STAR_B = np.array([s[1] for s in STARS]); STAR_P = np.array([s[2] for s in STARS])
CLOUDS = []
for k in range(7):
    az0 = rng.uniform(-75, 70); el0 = rng.uniform(9, 24); w = rng.uniform(9, 20)
    blobs = []
    for b in range(4):
        cx = az0 + rng.uniform(-w / 2, w / 2); ce = el0 + rng.uniform(-1.2, 1.5)
        bw, bh = rng.uniform(3, 8), rng.uniform(0.8, 2.0)
        blobs.append([(cx + bw * math.cos(a), ce + bh * math.sin(a)) for a in np.linspace(0, 2 * math.pi, 8)[:-1]])
    CLOUDS.append((blobs, rng.uniform(0.15, 0.35)))

# sand grain colours
GRAIN_TONE = rng.uniform(0.85, 1.08, G)

# thumbnails for the 12 collected patterns
THUMBS = {nm: nodal_image(*T.MODES[nm], res=22, thr=0.10) for nm in T.SWEEP_NAMES}

# ----------------------------------------------------------------- helpers: dithering, glow
BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
BAYER_FULL = np.tile(BAYER, (H // 4, W // 4))
YY, XX = np.mgrid[0:H, 0:W].astype(np.float32)


def glow_kernel(r):
    yy, xx = np.mgrid[-r:r + 1, -r:r + 1]
    d = np.hypot(xx, yy) / (r + 0.5)
    v = np.clip(1 - d, 0, 1) ** 1.7
    b = np.tile(BAYER, ((2 * r + 1) // 4 + 1, (2 * r + 1) // 4 + 1))[:2 * r + 1, :2 * r + 1]
    return np.floor(v * 5 + b * 0.999) / 5


GLOW = {r: glow_kernel(r) for r in range(1, 61)}


def add_glow(img, x, y, r, col, inten):
    r = int(max(1, min(60, round(r))))
    k = GLOW[r] * inten
    x, y = int(round(x)), int(round(y))
    x0, y0, x1, y1 = x - r, y - r, x + r + 1, y + r + 1
    if x1 <= 0 or y1 <= 0 or x0 >= W or y0 >= H:
        return
    kx0, ky0 = max(0, -x0), max(0, -y0)
    kx1, ky1 = k.shape[1] - max(0, x1 - W), k.shape[0] - max(0, y1 - H)
    img[max(0, y0):min(H, y1), max(0, x0):min(W, x1)] += k[ky0:ky1, kx0:kx1, None] * np.array(col, float)[None, None]


# ----------------------------------------------------------------- sky
def sky(cam, L):
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    az = np.arctan2(d[..., 0], -d[..., 2]) - math.radians(SUN_AZ)
    u = ((1 + np.cos(az)) / 2) ** 2.2
    hor = L.haway[None, None] * (1 - u[..., None]) + L.hsun[None, None] * u[..., None]
    k = np.clip(el / 0.75, 0, 1) ** 0.6
    col = hor * (1 - k[..., None]) + L.zen[None, None] * k[..., None]
    # glow around the sun
    sd = L.sun_dir
    cosang = (d * sd[None, None]).sum(-1)
    g = np.clip((cosang - 0.80) / 0.20, 0, 1) ** 3
    sun_glow = 0.55 * ss(28, 44, L.t) + 0.25 * ss(44, 50, L.t)
    col = col + g[..., None] * np.array([255, 196, 130]) * sun_glow
    # quantize to bands with ordered dithering (pixel-art sky)
    step = 7.0
    col = np.floor(col / step + BAYER_FULL[..., None]) * step
    return np.clip(col, 0, 255)


def draw_stars(img, cam, L, t):
    if L.star_a <= 0.01:
        return
    Pc = STAR_D @ cam.M.T
    vis = Pc[:, 2] > 0.05
    s = cam.proj(Pc[vis] * 100)
    b = STAR_B[vis] * L.star_a * (0.7 + 0.3 * np.sin(t * 3.1 + STAR_P[vis] * 5))
    x, y = s[:, 0].astype(int), s[:, 1].astype(int)
    ok = (x >= 1) & (x < W - 1) & (y >= 1) & (y < H - 1)
    for xi, yi, bi in zip(x[ok], y[ok], b[ok]):
        c = np.array([230, 236, 255]) * bi
        img[yi, xi] = np.maximum(img[yi, xi], c)
        if bi > 0.6:
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                img[yi + dy, xi + dx] = np.maximum(img[yi + dy, xi + dx], c * 0.45)


def screen_of_dir(cam, d, dist=500.0):
    Pc = cam.M @ (d * dist)
    if Pc[2] <= 0.1:
        return None
    return cam.proj(Pc[None])[0]


def draw_disc(draw, center, r, col, n=12, rot=0.0):
    pts = [(center[0] + r * math.cos(rot + 2 * math.pi * k / n), center[1] + r * math.sin(rot + 2 * math.pi * k / n)) for k in range(n)]
    draw.polygon(pts, fill=tuple(int(c) for c in col))


# ----------------------------------------------------------------- palms
def palm_polys(base, height, lean, ph, t, cam, L, campos):
    out = []
    sway = 0.12 * math.sin(t * 0.9 + ph) + 0.05 * math.sin(t * 2.3 + ph * 2)
    pts = []
    for s in range(6):
        u = s / 5
        x = base[0] + (lean + sway * u) * u * u * height * 0.35
        pts.append(np.array([x, base[1] + u * height, base[2] + 0.1 * u]))
    trunk_col = shade(L, np.array([[0.4, 0.0, 0.9]]) / np.linalg.norm([0.4, 0, 0.9]), pts[2][None], np.array([[150, 104, 64]]))[0]
    trunk_col = fog(L, trunk_col[None], pts[2][None], campos)[0]
    for s in range(5):
        a, b = pts[s], pts[s + 1]
        w0, w1 = 0.2 * (1 - 0.35 * s / 5), 0.2 * (1 - 0.35 * (s + 1) / 5)
        q = np.array([a + [-w0, 0, 0], a + [w0, 0, 0], b + [w1, 0, 0], b + [-w1, 0, 0]])
        out.append((q, trunk_col * (0.9 if s % 2 else 1.0)))
    top = pts[-1]
    for k in range(8):
        ang = 2 * math.pi * k / 8 + ph
        dx, dz = math.cos(ang), math.sin(ang)
        fl = 1.7 + 0.3 * math.sin(k * 1.7 + ph)
        droop = 0.9 + 0.15 * math.sin(t * 1.4 + k + ph)
        mid = top + np.array([dx * fl * 0.55, 0.45, dz * fl * 0.55])
        tip = top + np.array([dx * fl, 0.45 - droop, dz * fl])
        side = np.array([-dz, 0, dx]) * 0.32
        n1 = np.cross(mid - top, side); n1 /= np.linalg.norm(n1)
        if n1[1] < 0:
            n1 = -n1
        col = shade(L, n1[None], mid[None], np.array([[70, 132, 66]]))[0]
        col = fog(L, col[None], mid[None], campos)[0]
        out.append((np.array([top, mid + side, tip, mid - side]), col))
    return out


# ----------------------------------------------------------------- the frame
def caption_list():
    return [
        (0.0, 2.1, ["WHAT DOES A", "*NOTE* LOOK LIKE?"]),
        (2.1, 4.2, ["EVERY NOTE DRAWS", "ITS OWN *PICTURE*"]),
        (4.3, 7.2, ["A *METAL PLATE*, SAND", "AND ONE *SPEAKER*"]),
        (7.3, 9.8, ["SLOWED WAY DOWN,", "THE PLATE *RIPPLES*"]),
        (9.8, 12.2, ["THE BUMPS *FLING*", "SAND AWAY..."]),
        (12.2, 14.6, ["...ONTO THE LINES", "THAT *STAY STILL*"]),
        (14.7, 18.1, ["NOW SLIDE THE", "*PITCH* UP"]),
        (18.1, 22.1, ["IT ONLY *SNAPS* ON", "THE PLATE'S OWN NOTES"]),
        (22.1, 25.7, ["HIGHER NOTE,", "*FINER* DRAWING"]),
        (25.7, 29.3, ["*1787*: CHLADNI DID", "THIS WITH A *VIOLIN BOW*"]),
        (29.3, 32.95, ["*1808*: NAPOLEON SAW IT", "AND OFFERED A *PRIZE*..."]),
        (33.0, 35.4, ["NOW GIVE IT", "A *SONG*"]),
        (35.5, 39.4, ["~GRIEG~", "*MORNING MOOD*"]),
        (T.T_SUNRISE + 0.1, 48.6, ["GRIEG WROTE IT FOR A", "*SUNRISE* IN THE", "*MOROCCAN DESERT*"]),
        (T.T_FINAL + 0.1, 55.6, ["*1816*: SOPHIE GERMAIN", "WON THE PRIZE"]),
        (55.6, 59.0, ["TRY IT: A SPEAKER,", "A METAL PLATE", "+ A PINCH OF *SALT*"]),
    ]


CAPS = caption_list()


def draw_captions(img, t):
    for t0, t1, lines in CAPS:
        if t0 <= t < t1:
            u = t - t0
            nreveal = int(u * 55) + 1
            slide = int(round(6 * (1 - ease(u / 0.25))))
            fade = 1 - ss(t1 - 0.12, t1, t)
            y = 42 + slide
            used = 0
            lens = [len(l.replace('*', '').replace('~', '')) for l in lines]
            scale = 3 if max(lens) <= 18 else 2
            for ln, pl in zip(lines, lens):
                rich_line(img, ln, W // 2, y, scale, CREAM, AMBER, SKYB, nchars=nreveal - used, alpha=fade)
                used += pl
                y += 7 * scale + 7


def note_color(p):
    return {'E5': (124, 246, 200), 'F#5': (120, 200, 255), 'G#5': (186, 150, 255), 'A5': (255, 150, 214),
            'B5': (255, 196, 92), 'C#6': (255, 128, 104)}.get(p, (255, 255, 255))


HUD_Y = 438


def panel(draw, x0, y0, x1, y1):
    draw.rectangle([x0 + 2, y0 + 2, x1 + 2, y1 + 2], fill=(6, 8, 18))
    draw.rectangle([x0, y0, x1, y1], fill=(16, 20, 38))
    draw.rectangle([x0, y0, x1, y1], outline=(62, 74, 112))
    draw.line([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=(40, 50, 84))


def slot_found_time(nm):
    for n2, ta, tl in T.SWEEP:
        if n2 == nm:
            return ta + 0.25
    return 99


def fmt_hz(f):
    return f"{int(round(f)):,}"


def hud_sweep(img, draw, t):
    a_in = ease((t - 14.75) / 0.35)
    a_out = 1 - ease((t - 33.0) / 0.4)
    off = int(round((1 - min(a_in, a_out)) * 160))
    if off >= 160:
        return None
    y0 = HUD_Y + off
    panel(draw, 14, y0, 346, y0 + 96)
    f = T.sweep_freq(t) or 60.0
    # slider (log 50 - 6000 Hz)
    def fx(fr):
        return 26 + (math.log(fr) - math.log(50)) / (math.log(6000) - math.log(50)) * 308
    sy = y0 + 49
    draw.rectangle([26, sy, 334, sy + 3], fill=(40, 48, 80))
    for nm, ta, tl in T.SWEEP:
        x = fx(T.res_hz(nm))
        found = t >= ta
        draw.rectangle([x - 1, sy - 4, x + 1, sy + 7], fill=MINT if found else (70, 80, 120))
    x = fx(f)
    draw.polygon([(x - 5, sy - 9), (x + 5, sy - 9), (x, sy - 2)], fill=AMBER)
    draw.rectangle([26, sy, x, sy + 3], fill=AMBER)
    # slots
    for k, nm in enumerate(T.SWEEP_NAMES):
        sx = 26 + k * 26
        sy2 = y0 + 64
        draw.rectangle([sx - 1, sy2 - 1, sx + 22, sy2 + 22], fill=(30, 36, 62))
    return y0


def hud_sweep_after(img, t, y0):
    f = T.sweep_freq(t) or 60.0
    cnt = sum(1 for nm in T.SWEEP_NAMES if t >= slot_found_time(nm))
    stamp(img, text_mask("PATTERNS", 2), 26, y0 + 8, (150, 164, 210))
    stamp(img, text_mask(f"{cnt}/12", 2), 26, y0 + 25, MINT if cnt else (150, 164, 210))
    # snap state
    on = None
    for nm, ta, tl in T.SWEEP:
        if ta <= t < tl:
            on = nm
    s = fmt_hz(f)
    m = text_mask(s, 4)
    col = MINT if on else CREAM
    stamp(img, m, 334 - m.shape[1] - 28, y0 + 7, col)
    stamp(img, text_mask("HZ", 2), 334 - 22, y0 + 19, (150, 164, 210))
    for k, nm in enumerate(T.SWEEP_NAMES):
        sx, sy2 = 26 + k * 26, y0 + 64
        tf = slot_found_time(nm)
        if sy2 + 22 > H:
            continue
        if t >= tf:
            th = THUMBS[nm]
            pop = math.exp(-(t - tf) * 10)
            colr = np.array(MINT) * (1 - pop) + np.array([255, 255, 255]) * pop
            reg = img[sy2:sy2 + 22, sx:sx + 22]
            reg[th] = colr.astype(np.uint8)
        else:
            stamp(img, text_mask("?", 2), sx + 6, sy2 + 4, (70, 80, 120), outline=None)


def hud_song(img, draw, t):
    a_in = ease((t - T.T_SONG + 0.2) / 0.35)
    a_out = 1 - ease((t - (T.T_FINAL + 2.0)) / 0.4)
    off = int(round((1 - min(a_in, a_out)) * 160))
    if off >= 160:
        return None
    y0 = HUD_Y + off
    panel(draw, 14, y0, 346, y0 + 96)
    # piano roll: 4 s window, playhead at x=110
    x_play, pps = 110, 64.0
    rows = {'E5': 0, 'F#5': 1, 'G#5': 2, 'A5': 3, 'B5': 4, 'C#6': 5}
    ry0 = y0 + 84
    draw.rectangle([22, y0 + 34, 338, y0 + 88], fill=(12, 15, 30))
    for t0, t1, p in T.SONG:
        xa, xb = x_play + (t0 - t) * pps, x_play + (t1 - t) * pps - 2
        if xb < 22 or xa > 338:
            continue
        yy = ry0 - rows[p] * 8
        active = t0 <= t < t1
        col = note_color(p) if (active or t1 <= t) else tuple(int(c * 0.45) for c in note_color(p))
        draw.rectangle([max(22, xa), yy - 5, min(338, xb), yy], fill=col)
    draw.line([x_play, y0 + 34, x_play, y0 + 88], fill=(240, 240, 255))
    return y0


def hud_song_after(img, t, y0):
    cur = None
    for t0, t1, p in T.SONG:
        if t0 <= t < t1:
            cur = (p, t0)
    if t >= T.T_FINAL:
        cur = ('E', T.T_FINAL)
    if cur is None:
        return
    p = cur[0]
    stamp(img, text_mask("NOTE", 2), 26, y0 + 9, (150, 164, 210))
    sc = 4 if t - cur[1] > 0.06 else 5
    m = text_mask(p if p != 'E' else 'E MAJOR', sc if p != 'E' else 3)
    stamp(img, m, 80, y0 + 6 - (sc - 4) * 2 if p != 'E' else y0 + 9, note_color(p) if p != 'E' else MINT)
    if p != 'E':
        hz = T.midi_hz(T.NOTE_MIDI[p])
        s = fmt_hz(hz) + " HZ"
        mm = text_mask(s, 2)
        stamp(img, mm, 334 - mm.shape[1], y0 + 12, (150, 164, 210))


def hud_slowmo(img, draw, t):
    if not (T.T_SLOW0 - 0.1 <= t < T.T_SLOW1 + 0.1):
        return
    a = min(ease((t - T.T_SLOW0) / 0.3), 1 - ease((t - T.T_SLOW1 + 0.2) / 0.3))
    off = int(round((1 - a) * 160))
    y0 = HUD_Y + 20 + off
    panel(draw, 14, y0, 346, y0 + 64)
    return y0


def hud_slowmo_after(img, t, y0):
    if y0 is None:
        return
    blink = (int(t * 2.5) % 2 == 0)
    if blink:
        stamp(img, text_mask("*", 2), 0, 0, CORAL, outline=None)
    stamp(img, text_mask("SLOW MOTION", 2), 26, y0 + 9, CORAL if blink else (200, 110, 100))
    items = [((240, 112, 92), "UP"), ((70, 190, 206), "DOWN"), (MINT, "STILL")]
    x = 26
    for col, lab in items:
        img[y0 + 34:y0 + 46, x:x + 12] = col
        stamp(img, text_mask(lab, 2), x + 18, y0 + 36, CREAM)
        x += 18 + len(lab) * 12 + 22


# ------------------------------------------------------------------ main
def mode_field_slow(P):
    a, _ = mode_val_grad(P, *T.MODES['S'])
    return a


def slow_disp(t):
    """plate displacement amplitude (world units) and phase in slow-motion"""
    if not (T.T_SLOW0 <= t < T.T_SLOW1):
        return 0.0, 0.0
    amp = 0.07 + 0.11 * ss(T.T_SLOW_FLING, T.T_SLOW_FLING + 0.6, t)
    amp *= ss(T.T_SLOW0, T.T_SLOW0 + 0.6, t) * (1 - ss(T.T_SLOW1 - 0.5, T.T_SLOW1, t))
    return amp, math.sin(2 * math.pi * 0.95 * (t - T.T_SLOW0))


MESH_N = 16
MX = np.linspace(-1, 1, MESH_N + 1)
MGX, MGZ = np.meshgrid(MX, MX)
MESH_A = mode_field_slow(np.stack([MGX.ravel(), MGZ.ravel()], 1)).reshape(MGX.shape)


def song_note(t):
    cur = None
    for t0, t1, p in T.SONG:
        if t0 <= t:
            cur = (t0, t1, p)
    return cur


def snap_env(t):
    e = 0.0
    for nm, ta, tl in T.SWEEP:
        if ta <= t < ta + 0.6:
            e = max(e, math.exp(-(t - ta) * 5))
    for nm, t0, t1 in T.HOOK:
        if t0 <= t < t0 + 0.6:
            e = max(e, math.exp(-(t - t0) * 5))
    return e


def render(f):
    t = f / FPS
    L = Light(t)
    cam = cam_at(t)
    campos = cam.pos
    sky_img = sky(cam, L)
    draw_stars(sky_img, cam, L, t)
    pil = Image.fromarray(sky_img.astype(np.uint8))
    draw = ImageDraw.Draw(pil)
    glows = []

    # shooting star
    if 5.0 <= t < 5.7:
        u = (t - 5.0) / 0.7
        a = screen_of_dir(cam, dirv(-55 + 40 * u, 46 - 14 * u))
        b = screen_of_dir(cam, dirv(-55 + 40 * max(0, u - 0.25), 46 - 14 * max(0, u - 0.25)))
        if a is not None and b is not None:
            draw.line([tuple(b), tuple(a)], fill=(255, 255, 240), width=2)
            glows.append((a[0], a[1], 6, (200, 220, 255), 0.6 * (1 - u)))
    # sun
    sun_s = screen_of_dir(cam, L.sun_dir)
    if sun_s is not None and L.sun_el > -3.5:
        r = 0.032 * cam.f
        draw_disc(draw, sun_s, r, (255, 238, 196), 12)
        draw_disc(draw, sun_s, r * 0.7, (255, 250, 228), 12)
    # clouds
    for blobs, spd in CLOUDS:
        for b_i, pts in enumerate(blobs):
            sp = []
            ok = True
            for az, el in pts:
                s = screen_of_dir(cam, dirv(az + spd * t, el))
                if s is None:
                    ok = False
                    break
                sp.append(tuple(s))
            if not ok:
                continue
            cl = np.array(lerp3((34, 42, 76), (196, 120, 140), ss(20, 38, t)))
            cl = np.array(lerp3(cl, (255, 214, 190), ss(42, 50, t)))
            if b_i % 2:
                cl = cl * 0.86
            draw.polygon(sp, fill=tuple(int(c) for c in cl))
    # far mesas
    for pts in MESAS:
        col = L.haway * 0.75 + L.zen * 0.15
        poly(draw, cam, pts, col)

    # moon (waning crescent, rises before the sun)
    if L.star_a > 0.02 or t < 40:
        ms = screen_of_dir(cam, L.moon_dir)
        if ms is not None:
            r = 0.046 * cam.f
            ma = float(np.interp(t, [0, 30, 40], [1, 0.8, 0.0]))
            if ma > 0.02:
                glows.append((ms[0], ms[1], r * 3.5, (150, 170, 230), 0.22 * ma))
                draw_disc(draw, ms, r, np.array([236, 232, 210]) * ma + L.zen * (1 - ma), 12)
                sh = ms + np.array([-0.45 * r, -0.3 * r])
                bgc = sky_img[int(np.clip(sh[1], 0, H - 1)), int(np.clip(sh[0], 0, W - 1))]
                draw_disc(draw, sh, r * 0.92, bgc, 12)
    # terrain + far things, painter sorted ----------------------------
    items = []
    Pc_c = cam.to_cam(TC)
    vis = (Pc_c[:, 2] > -2) & (((campos[None] - TC) * TN).sum(1) > 0)
    cols = shade(L, TN, TC, TCOL, braz=True)
    cols = fog(L, cols, TC, campos)
    for i in np.nonzero(vis)[0]:
        c = cols[i]
        if TWATER[i]:
            continue
        items.append((Pc_c[i, 2], TRI[i], c))
    for Q, n in KAS:
        ctr = Q.mean(0)
        if np.dot(n, campos - ctr) <= 0:
            continue
        c = shade(L, n[None], ctr[None], np.array([[196, 136, 96]]), braz=False)
        c = fog(L, c, ctr[None], campos)[0]
        items.append((cam.to_cam(ctr[None])[0, 2], Q, c))
    items.sort(key=lambda it: -it[0])
    for _, P, c in items:
        poly(draw, cam, P, c)
    # kasbah windows
    if L.window_k > 0.02:
        for wp in WINDOWS:
            s = cam.world_proj(wp[None])[0]
            if 0 <= s[0] < W and 0 <= s[1] < H:
                draw.rectangle([s[0], s[1], s[0] + 1, s[1] + 1], fill=(255, 200, 110))
                glows.append((s[0], s[1], 4, (255, 170, 80), 0.35 * L.window_k))
    # oasis pool + palms
    zen = L.zen
    pc = np.array(lerp3(L.haway, zen, 0.4)) * 0.85 + np.array([0, 10, 20])
    poly(draw, cam, POOL, pc)
    glint = POOL.mean(0) + np.array([0.6, 0.05, 0.3])
    gs = cam.world_proj(glint[None])[0]
    glows.append((gs[0], gs[1], 5, (220, 230, 255) if t < 40 else (255, 220, 160), 0.25))
    palm_items = []
    for base, hgt, lean, ph in PALMS:
        for P, c in palm_polys(base, hgt, lean, ph, t, cam, L, campos):
            palm_items.append((cam.to_cam(P.mean(0)[None])[0, 2], P, c))
    palm_items.sort(key=lambda it: -it[0])
    for _, P, c in palm_items:
        poly(draw, cam, P, c)
    # fireflies by the oasis
    if t < 32:
        fa = 1 - ss(24, 32, t)
        for k in range(22):
            ph = k * 2.39
            p = OASIS + np.array([3.2 * math.sin(t * 0.31 + ph), 0.6 + 0.5 * math.sin(t * 0.7 + ph * 1.3), 2.6 * math.cos(t * 0.27 + ph * 0.7)])
            s = cam.world_proj(p[None])[0]
            blink = max(0.0, math.sin(t * 2.2 + ph * 3.1))
            if blink > 0.2 and 0 <= s[0] < W and 0 <= s[1] < H:
                glows.append((s[0], s[1], 3, (200, 255, 120), 0.5 * blink * fa))

    # courtyard floor
    poly_floor = np.array([[6.35 * math.sin(a), 0.0, -6.35 * math.cos(a)] for a in np.linspace(0, 2 * math.pi, 25)[:-1]])
    fc = shade(L, np.array([[0, 1, 0]]), np.array([[0, 0, 0]]), np.array([[118, 102, 88]]))[0]
    poly(draw, cam, poly_floor, fc)
    tcol = shade(L, np.tile([0, 1, 0], (len(TILES), 1)).astype(float), TILE_C, TILE_COL)
    tdepth = cam.to_cam(TILE_C)[:, 2]
    for i in np.argsort(-tdepth):
        poly(draw, cam, TILES[i][0], tcol[i])
    # cable from the driver to the generator
    cab = [np.array([0.25, 0.02, 0.35]), np.array([0.9, 0.02, 0.9]), np.array([1.5, 0.02, 1.05])]
    cs = cam.world_proj(np.array(cab))
    draw.line([tuple(p) for p in cs], fill=(30, 30, 36), width=2)

    # objects --------------------------------------------------------
    amp, phs = slow_disp(t)
    mesh_mode = amp > 0.001
    odepth = [(np.linalg.norm(o.ctr - campos), o) for o in OBJS]
    odepth.sort(key=lambda a: -a[0])
    for _, o in odepth:
        if o.name == 'plate' and mesh_mode:
            draw_mesh_plate(draw, cam, L, t, amp, phs, campos)
            draw_sand(pil, draw, cam, L, f, t, amp, phs, glows)
            continue
        fcs = []
        for j, (Q, n) in enumerate(o.faces):
            ctr = Q.mean(0)
            if np.dot(n, campos - ctr) <= 0:
                continue
            fcs.append((cam.to_cam(ctr[None])[0, 2], Q, n, j))
        fcs.sort(key=lambda a: -a[0])
        for _, Q, n, j in fcs:
            c = shade(L, n[None], Q.mean(0)[None], (o.color * o.tone[j])[None], braz=o.braz)[0]
            if o.name == 'plate' and n[1] > 0.5:
                c = shade(L, n[None], Q.mean(0)[None], np.array([[54, 58, 70]]))[0]
            if o.name == 'bowl' and n[1] > 0.5:
                c = np.array([255, 150, 60]) * (0.6 + 0.4 * L.brazier_k)
            poly(draw, cam, Q, c)
        if o.name == 'gen':
            # glowing screen on the generator
            scr = np.array([[1.55, 0.88, 1.53], [1.92, 0.88, 1.42], [1.92, 0.72, 1.42], [1.55, 0.72, 1.53]])
            on = VDRV[f] > 0.01
            poly(draw, cam, scr, (90, 255, 200) if on else (30, 70, 64))
            if on:
                s = cam.world_proj(scr.mean(0)[None])[0]
                glows.append((s[0], s[1], 8, (90, 255, 200), 0.25))
        if o.name == 'bowl':
            draw_flame(draw, cam, o.ctr, t, L, glows)
        if o.name == 'plate':
            draw_sand(pil, draw, cam, L, f, t, 0.0, 0.0, glows)

    # rim glow on resonance / notes
    rim = np.array([[-PS, PY, -PS], [PS, PY, -PS], [PS, PY, PS], [-PS, PY, PS]])
    rs = cam.world_proj(rim)
    e = snap_env(t)
    rimcol = None
    if e > 0.05:
        rimcol, rk = MINT, e
    sn = song_note(t) if T.T_SONG <= t < T.T_FINAL else None
    if sn is not None:
        rimcol, rk = note_color(sn[2]), 0.5 + 0.5 * math.exp(-(t - sn[0]) * 6)
    if T.T_FINAL <= t < T.T_FINAL_OFF:
        rimcol, rk = AMBER, 0.7 * math.exp(-(t - T.T_FINAL) * 0.6)
    if rimcol is not None:
        pts = [tuple(p) for p in rs] + [tuple(rs[0])]
        draw.line(pts, fill=tuple(int(c * rk + 40 * (1 - rk)) for c in rimcol), width=2)
        for p in rs:
            glows.append((p[0], p[1], 10, rimcol, 0.25 * rk))

    # embers
    for bi, bp in enumerate(BRAZIERS):
        for k in range(10):
            ph = k * 0.73 + bi
            u = ((t * 0.55 + ph) % 1.0)
            p = bp + np.array([0.25 * math.sin(ph * 5 + t), 0.2 + u * 2.2, 0.2 * math.cos(ph * 3 + t)])
            s = cam.world_proj(p[None])[0]
            if 0 <= s[0] < W and 0 <= s[1] < H and cam.to_cam(p[None])[0, 2] > NEAR:
                glows.append((s[0], s[1], 2, (255, 150, 60), 0.6 * (1 - u) * L.brazier_k))

    # dust motes drifting on the breeze
    for k in range(70):
        ph = k * 1.618
        x = ((t * (0.35 + 0.25 * math.sin(ph * 3)) + ph * 2.3) % 14.0) - 7.0
        z = ((ph * 4.1) % 12.0) - 6.0
        yv = 0.15 + 1.6 * ((ph * 0.37) % 1.0) + 0.2 * math.sin(t * 1.3 + ph)
        p = np.array([[x, yv, z]])
        pc = cam.to_cam(p)[0]
        if pc[2] < 0.5:
            continue
        s2 = cam.proj(pc[None])[0]
        if 0 <= s2[0] < W and 0 <= s2[1] < H:
            mc = (np.array([150, 170, 230]) if t < 38 else np.array([255, 220, 170])) * (0.5 + 0.5 * math.sin(t * 2 + ph))
            glows.append((s2[0], s2[1], 1, tuple(mc), 0.5))

    # birds at sunrise
    if 44.5 <= t < 53:
        for k in range(5):
            u = (t - 44.5) / 8.5
            az = -40 + 70 * u + k * 3.2
            el = 12 + 2.5 * math.sin(k * 1.3) + 1.5 * u
            s = screen_of_dir(cam, dirv(az, el), 300)
            if s is None:
                continue
            flap = 2 if int(t * 7 + k) % 2 else -1
            draw.line([(s[0] - 4, s[1] - flap), (s[0], s[1]), (s[0] + 4, s[1] - flap)], fill=(40, 30, 40), width=1)

    img = np.asarray(pil).astype(np.float32)
    for x, y, r, col, inten in glows:
        add_glow(img, x, y, r, col, inten)

    # sunrise god rays + bloom
    if t >= T.T_SUNRISE - 0.3 and sun_s is not None:
        k = 0.35 * ss(T.T_SUNRISE - 0.3, T.T_SUNRISE + 0.4, t) * (1 - 0.6 * ss(T.T_SUNRISE + 1.5, 50, t))
        if k > 0.01:
            ray = Image.new('L', (W, H), 0)
            rd = ImageDraw.Draw(ray)
            for j in range(9):
                a0 = j * 2 * math.pi / 9 + t * 0.05
                a1 = a0 + 0.12
                R = 900
                rd.polygon([tuple(sun_s), (sun_s[0] + R * math.cos(a0), sun_s[1] + R * math.sin(a0)),
                            (sun_s[0] + R * math.cos(a1), sun_s[1] + R * math.sin(a1))], fill=255)
            rm = np.asarray(ray).astype(np.float32) / 255
            dist = np.hypot(XX - sun_s[0], YY - sun_s[1])
            fall = np.clip(1 - dist / 420, 0, 1) ** 1.5
            mask = (rm * fall)
            mask = np.floor(mask * 4 + BAYER_FULL * 0.999) / 4
            img += mask[..., None] * np.array([255, 200, 140]) * k
            add_glow(img, sun_s[0], sun_s[1], 50, (255, 210, 150), 0.45 * k / 0.35)
    if T.T_SUNRISE <= t < T.T_SUNRISE + 0.5:
        kk = (1 - (t - T.T_SUNRISE) / 0.5) ** 2 * 0.35
        img = img * (1 - kk) + np.array([255, 230, 190]) * kk

    img = np.clip(img, 0, 255)
    img = img * vignette() * top_scrim()

    # UI
    pil = Image.fromarray(img.astype(np.uint8))
    draw = ImageDraw.Draw(pil)
    y_sw = hud_sweep(None, draw, t) if 14.7 <= t < 33.5 else None
    y_so = hud_song(None, draw, t) if T.T_SONG - 0.3 <= t < T.T_FINAL + 2.6 else None
    y_sl = hud_slowmo(None, draw, t)
    out = np.asarray(pil).copy()
    if y_sw is not None:
        hud_sweep_after(out, t, y_sw)
    if y_so is not None:
        hud_song_after(out, t, y_so)
    if y_sl is not None:
        hud_slowmo_after(out, t, y_sl)
    draw_captions(out, t)
    return out


def draw_flame(draw, cam, ctr, t, L, glows):
    base = ctr + np.array([0, 0.16, 0])
    fl = L.brazier_k
    for k, (col, hgt, w) in enumerate([((255, 110, 40), 0.9, 0.3), ((255, 170, 60), 0.65, 0.22), ((255, 236, 150), 0.38, 0.12)]):
        wob = 0.08 * math.sin(t * 9 + k * 2 + ctr[0])
        hh = hgt * (0.85 + 0.25 * math.sin(t * 11.3 + k + ctr[0] * 3)) * (0.55 + 0.45 * fl)
        pts = np.array([base + [-w, 0, 0], base + [w, 0, 0], base + [wob, hh, 0]])
        s = cam.world_proj(pts)
        draw.polygon([tuple(p) for p in s], fill=col)
    s = cam.world_proj((base + [0, 0.35, 0])[None])[0]
    z = cam.to_cam(base[None])[0, 2]
    glows.append((s[0], s[1], min(60, 0.9 * cam.f / max(z, 0.5)), (255, 140, 50), 0.32 * fl))


def draw_mesh_plate(draw, cam, L, t, amp, phs, campos):
    Y = PY + amp * MESH_A * phs
    V = np.stack([MGX * PS, Y, MGZ * PS], -1)
    tris = []
    for i in range(MESH_N):
        for j in range(MESH_N):
            a, b, c, d = V[i, j], V[i, j + 1], V[i + 1, j + 1], V[i + 1, j]
            for tri in ((a, b, c), (a, c, d)):
                tris.append(np.array(tri))
    tris = np.array(tris)
    nrm = np.cross(tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0])
    nrm /= np.linalg.norm(nrm, axis=1)[:, None]
    nrm[nrm[:, 1] < 0] *= -1
    ctr = tris.mean(1)
    disp = (ctr[:, 1] - PY) / max(amp, 1e-6)
    base = np.tile(np.array([54.0, 58, 70]), (len(tris), 1))
    q = np.round(np.clip(np.abs(disp), 0, 1) ** 0.8 * 4) / 4
    up = np.array([240, 112, 92.0]); dn = np.array([70, 190, 206.0])
    tint = np.where((disp > 0)[:, None], up[None], dn[None])
    vis_k = ss(T.T_SLOW0, T.T_SLOW0 + 0.5, t) * (1 - ss(T.T_SLOW1 - 0.4, T.T_SLOW1, t))
    base = base * (1 - q[:, None] * 0.85 * vis_k) + tint * q[:, None] * 0.85 * vis_k
    cols = shade(L, nrm, ctr, base)
    depth = cam.to_cam(ctr)[:, 2]
    for i in np.argsort(-depth):
        poly(draw, cam, tris[i], cols[i])
    # still lines highlight
    if t >= 12.2:
        k = ss(12.2, 12.6, t) * (1 - ss(T.T_SLOW1 - 0.4, T.T_SLOW1, t))
        if k > 0.02:
            col = tuple(int(min(255, c * k * 1.1 + 54 * (1 - k))) for c in MINT)
            for i in range(MESH_N):
                for j in range(MESH_N):
                    for (p0, a0), (p1, a1) in ((((i, j), MESH_A[i, j]), ((i, j + 1), MESH_A[i, j + 1])),
                                               (((i, j), MESH_A[i, j]), ((i + 1, j), MESH_A[i + 1, j]))):
                        pass
            segs = marching(MESH_A_FINE)
            for (x0, z0), (x1, z1) in segs:
                pa = np.array([[x0 * PS, PY + 0.004, z0 * PS], [x1 * PS, PY + 0.004, z1 * PS]])
                s = cam.world_proj(pa)
                draw.line([tuple(s[0]), tuple(s[1])], fill=col, width=2)


def marching(A):
    n = A.shape[0] - 1
    xs = np.linspace(-1, 1, n + 1)
    segs = []
    for i in range(n):
        for j in range(n):
            c = [(xs[j], xs[i], A[i, j]), (xs[j + 1], xs[i], A[i, j + 1]), (xs[j + 1], xs[i + 1], A[i + 1, j + 1]), (xs[j], xs[i + 1], A[i + 1, j])]
            pts = []
            for k in range(4):
                (xa, za, va), (xb, zb, vb) = c[k], c[(k + 1) % 4]
                if (va > 0) != (vb > 0):
                    u = va / (va - vb)
                    pts.append((xa + (xb - xa) * u, za + (zb - za) * u))
            if len(pts) == 2:
                segs.append((pts[0], pts[1]))
            elif len(pts) == 4:
                segs.append((pts[0], pts[1])); segs.append((pts[2], pts[3]))
    return segs


_fx = np.linspace(-1, 1, 41)
_FX, _FZ = np.meshgrid(_fx, _fx)
MESH_A_FINE = mode_field_slow(np.stack([_FX.ravel(), _FZ.ravel()], 1)).reshape(_FX.shape)
# nudge exact zeros
MESH_A_FINE = np.where(MESH_A_FINE == 0, 1e-6, MESH_A_FINE)


def draw_sand(pil, draw, cam, L, f, t, amp, phs, glows):
    P = np.asarray(SAND[f])
    h = np.asarray(SANDH[f]).astype(np.float32)
    y = PY + 0.006 + h
    if amp > 0:
        a = mode_field_slow(P)
        y = y + amp * a * phs + h * 1.5
    W3 = np.stack([P[:, 0] * PS, y, P[:, 1] * PS], 1)
    Pc = cam.to_cam(W3)
    ok = Pc[:, 2] > NEAR
    s = cam.proj(Pc)
    x = s[:, 0].astype(int); yy = s[:, 1].astype(int)
    ok &= (x >= 0) & (x < W - 1) & (yy >= 0) & (yy < H - 1)
    # sand colour: lit from above, brightened so it always reads
    I = L.amb.copy()
    for d, c in L.lights:
        I += max(0.0, d[1]) * c
    for bp in BRAZIERS:
        I += FIRE * 1.2 * L.brazier_k * 0.3 / (1 + (np.linalg.norm(bp) / 2.0) ** 2)
    base = np.array([238, 228, 204]) * (I * 1.15 + 0.32)
    if T.T_SONG <= t < T.T_FINAL_OFF + 2:
        gold = ss(T.T_SUNRISE - 0.2, T.T_SUNRISE + 1.0, t)
        base = base * (1 - 0.3 * gold) + np.array([255, 222, 150]) * 0.3 * gold
    base = np.clip(base, 0, 255)
    arr = np.asarray(pil)
    arr = arr.copy()
    tone = GRAIN_TONE[ok][:, None]
    cols = np.clip(base[None] * tone, 0, 255).astype(np.uint8)
    # plate scale on screen: thicken grains in close-ups
    corners = cam.world_proj(np.array([[-PS, PY, -PS], [PS, PY, PS]]))
    span = np.linalg.norm(corners[1] - corners[0]) / 1.414
    xs, ys = x[ok], yy[ok]
    arr[ys, xs] = cols
    if span > 230:
        sel = GRAIN_TONE[ok] > 0.97
        arr[ys[sel], xs[sel] + 1] = cols[sel]
        sel2 = GRAIN_TONE[ok] < 0.9
        arr[ys[sel2] + 1, xs[sel2]] = cols[sel2]
    pil.paste(Image.fromarray(arr))


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
        v = np.clip(1 - (YY - 40) / 150.0, 0, 1) * 0.58
        v = np.floor(v * 6 + BAYER_FULL * 0.999) / 6
        SCRIM = (1 - v)[..., None]
    return SCRIM


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
