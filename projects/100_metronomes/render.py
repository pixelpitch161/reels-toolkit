"""100 Metronomes: low-res flat-shaded 3D (low-poly) rendered to a 360x640
pixel canvas, then upscaled 3x with nearest-neighbour to 1080x1920."""
import math, sys, os
import numpy as np
from PIL import Image, ImageDraw
from font import text_mask, stamp, rich_line
from sim import FPS, N, HOLDOUT, T_UNLOCK, AMP, COLS, NF

W, H = 360, 640
UP = 3
TL = np.load('timeline.npz')
THETA = TL['theta']; FLAGS = TL['flags']; COUNT = TL['count']
RR = TL['R']; PSI = TL['psi']
T_DROP = float(TL['t_drop']); T_HOLD_IN = float(TL['t_hold_in'])
BEAT = 0.5
T_MUSIC_END = T_DROP + 32 * BEAT          # 8 bars
ROW = np.arange(N) // COLS
COL = np.arange(N) % COLS
MX = (COL - 4.5) * 1.0
MZ = (ROW - 4.5) * 1.0
HX, HZ = MX[HOLDOUT], MZ[HOLDOUT]

# ---------------------------------------------------------------- palette
SKY_TOP = np.array([11, 22, 31])
SKY_HOR = np.array([38, 74, 80])
FOG = SKY_HOR.astype(float)
CREAM = (250, 238, 214)
MINT = (120, 246, 196)
CORAL = (255, 112, 92)
AMBER = (255, 196, 84)
INK = (10, 20, 26)
MAT = {
    'body':   np.array([238, 226, 200]),
    'cap':    np.array([214, 120, 86]),
    'plinth': np.array([96, 64, 50]),
    'panel':  np.array([44, 36, 52]),
    'board':  np.array([214, 172, 116]),
    'bside':  np.array([168, 126, 82]),
    'can_a':  np.array([236, 104, 78]),
    'can_b':  np.array([74, 190, 166]),
    'canend': np.array([196, 204, 210]),
    'floor':  np.array([30, 58, 64]),
    'hill':   np.array([26, 52, 60]),
    'crys':   np.array([82, 168, 160]),
}
LIGHT = np.array([-0.45, 0.85, 0.5]); LIGHT /= np.linalg.norm(LIGHT)
RIM = np.array([0.55, 0.15, -0.8]); RIM /= np.linalg.norm(RIM)


def shade(n, base, steps=0.075):
    lam = 0.34 + 0.62 * max(0.0, float(np.dot(n, LIGHT))) + 0.10 * max(0.0, float(np.dot(n, RIM)))
    lam = round(lam / steps) * steps
    return np.clip(base * lam, 0, 255)


def sstep(x):
    x = min(1.0, max(0.0, x))
    return x * x * (3 - 2 * x)


def ease(x):  # smootherstep
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


def lerp(a, b, t):
    return a + (b - a) * t


# ---------------------------------------------------------------- tip colours
def sync_color(d):
    """|relative phase| -> colour: mint = with the crowd, coral = way off."""
    stops = [(0.0, (120, 250, 196)), (0.30, (178, 250, 140)), (0.85, (255, 200, 84)),
             (1.7, (255, 116, 92)), (math.pi, (238, 84, 128))]
    d = min(abs(d), math.pi)
    for (a, ca), (b, cb) in zip(stops, stops[1:]):
        if d <= b:
            t = (d - a) / (b - a)
            return tuple(int(lerp(ca[k], cb[k], t)) for k in range(3))
    return stops[-1][1]


LED = [(120, 250, 196), (110, 200, 255), (176, 140, 255), (255, 120, 200),
       (255, 116, 92), (255, 200, 84), (178, 250, 140)]

HEART = ["..........",
         ".##....##.",
         "####..####",
         "##########",
         "##########",
         ".########.",
         "..######..",
         "...####...",
         "....##....",
         ".........."]


def led_color(i, t):
    """LED-show colours for the tips during the drop (beat-locked)."""
    b = (t - T_DROP) / BEAT
    bi = int(math.floor(b))
    fr = b - bi
    r, c = ROW[i], COL[i]
    pulse = math.exp(-fr * 3.0)
    if bi < 8:                                    # bars 1-2: whole-board flashes
        col = LED[0] if bi % 2 == 0 else LED[1]
        return col, 0.55 + 0.45 * pulse
    if bi < 16:                                   # bars 3-4: rings ripple out from centre
        dist = math.hypot(r - 4.5, c - 4.5)
        k = int(math.floor(dist - (b - 8) * 2.0)) % len(LED)
        return LED[k], 0.75 + 0.25 * pulse
    if bi < 24:                                   # bars 5-6: checkerboard swaps on the beat
        on = ((r + c) % 2 == 0) ^ (bi % 2 == 1)
        diag = int(math.floor((r + c) / 3 + b * 2)) % 2
        col = LED[3] if on else (LED[5] if diag else LED[2])
        return col, (0.6 + 0.4 * pulse) if on else 0.45
    if bi < 28:                                   # bar 7: a heart
        on = HEART[r][c] == '#'
        return (LED[3], 0.7 + 0.3 * pulse) if on else ((40, 60, 70), 0.25)
    # bar 8: rainbow sweep across the board
    k = int(math.floor(c * 0.7 + r * 0.3 - b * 3)) % len(LED)
    return LED[k], 0.8 + 0.2 * pulse


# ---------------------------------------------------------------- geometry
def octagon(wx, wz, c, y):
    pts = [(wx - c, wz), (wx, wz - c), (wx, -wz + c), (wx - c, -wz),
           (-wx + c, -wz), (-wx, -wz + c), (-wx, wz - c), (-wx + c, wz)]
    return [(x, y, z) for x, z in pts]


def face_normal(V, idx, inside):
    a, b, c = (np.array(V[k]) for k in idx[:3])
    n = np.cross(b - a, c - a)
    n /= np.linalg.norm(n) + 1e-12
    ctr = np.mean([V[k] for k in idx], axis=0)
    if np.dot(n, ctr - inside) < 0:
        n = -n
        idx = idx[::-1]
    return n, idx


Y_PL = 0.08
Y_TOP = 1.0
WXB, WZB, CB = 0.28, 0.22, 0.06
WXT, WZT, CT = 0.11, 0.085, 0.025


def zfront(y):
    return WZB + (WZT - WZB) * (y - Y_PL) / (Y_TOP - Y_PL)


def build_metronome():
    V, F = [], []
    # plinth
    px, pz = 0.33, 0.27
    base = len(V)
    for y in (0.0, Y_PL):
        V += [(px, y, pz), (px, y, -pz), (-px, y, -pz), (-px, y, pz)]
    inside = np.array([0, Y_PL / 2, 0])
    quads = [(0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7), (4, 5, 6, 7)]
    for q in quads:
        idx = [base + k for k in q]
        n, idx = face_normal(V, idx, inside)
        F.append(dict(idx=idx, mat='plinth', n=n, part=0))
    # body: chamfered octagonal frustum
    b0 = len(V); V += octagon(WXB, WZB, CB, Y_PL)
    t0 = len(V); V += octagon(WXT, WZT, CT, Y_TOP)
    inside = np.array([0, 0.5, 0])
    for k in range(8):
        k2 = (k + 1) % 8
        idx = [b0 + k, b0 + k2, t0 + k2, t0 + k]
        n, idx = face_normal(V, idx, inside)
        F.append(dict(idx=idx, mat='body', n=n, part=1, front=(k == 7)))
    # cap pyramid
    apex = len(V); V.append((0.0, Y_TOP + 0.13, 0.0))
    inside = np.array([0, Y_TOP + 0.03, 0])
    for k in range(8):
        k2 = (k + 1) % 8
        idx = [t0 + k, t0 + k2, apex]
        n, idx = face_normal(V, idx, inside)
        F.append(dict(idx=idx, mat='cap', n=n, part=2))
    # dark panel inlay on the front face
    p0 = len(V)
    yb, yt = 0.17, 0.88
    hb, ht = 0.15, 0.055
    V += [(hb, yb, zfront(yb) + 0.004), (-hb, yb, zfront(yb) + 0.004),
          (-ht, yt, zfront(yt) + 0.004), (ht, yt, zfront(yt) + 0.004)]
    nf = F[5 + 7]['n']
    F.append(dict(idx=[p0, p0 + 1, p0 + 2, p0 + 3], mat='panel', n=nf, part=3))
    return np.array(V, float), F


MV, MF = build_metronome()
PIVOT = np.array([0.0, 0.21, zfront(0.21) + 0.04])
ROD = 0.74
FRONT_FACE = [j for j, f in enumerate(MF) if f.get('front')][0]


def box_faces(cx, cy, cz, hx, hy, hz, mat_top, mat_side):
    V = []
    for y in (cy - hy, cy + hy):
        V += [(cx + hx, y, cz + hz), (cx + hx, y, cz - hz), (cx - hx, y, cz - hz), (cx - hx, y, cz + hz)]
    inside = np.array([cx, cy, cz])
    out = []
    for q, m in [((0, 1, 5, 4), mat_side), ((1, 2, 6, 5), mat_side), ((2, 3, 7, 6), mat_side),
                 ((3, 0, 4, 7), mat_side), ((4, 5, 6, 7), mat_top), ((0, 3, 2, 1), mat_side)]:
        n, idx = face_normal(V, list(q), inside)
        out.append((np.array([V[k] for k in idx]), n, m))
    return out


rng = np.random.default_rng(3)

# board top triangulation tone jitter (subtle low-poly)
BOARD_HALF = 5.3
BOARD_T = 0.35
BT_N = 6
BT_JIT = rng.uniform(-0.06, 0.06, (BT_N, BT_N, 2))

# floor: low-poly triangles; flat near the board, gently rolling far away
FLOOR_Y = -BOARD_T
FL_STEP = 4.5
FL_R = 8
fx = np.arange(-FL_R, FL_R + 1) * FL_STEP
FGX, FGZ = np.meshgrid(fx, fx)
FGX = FGX + rng.uniform(-1.2, 1.2, FGX.shape)
FGZ = FGZ + rng.uniform(-1.2, 1.2, FGZ.shape)
dist0 = np.hypot(FGX, FGZ)
FGY = FLOOR_Y + np.where(dist0 > 14, rng.uniform(-0.2, 0.9, FGX.shape) * np.clip((dist0 - 14) / 12, 0, 1), 0)
FLOOR_TRIS = []
for a in range(2 * FL_R):
    for b in range(2 * FL_R):
        p = lambda i, j: np.array([FGX[i, j], FGY[i, j], FGZ[i, j]])
        q = [p(a, b), p(a, b + 1), p(a + 1, b + 1), p(a + 1, b)]
        for tri in ([q[0], q[1], q[2]], [q[0], q[2], q[3]]):
            n = np.cross(tri[1] - tri[0], tri[2] - tri[0])
            n /= np.linalg.norm(n)
            if n[1] < 0:
                n = -n
            tone = 1 + rng.uniform(-0.07, 0.07)
            FLOOR_TRIS.append((np.array(tri), n, tone))

# distant hills: ring of faceted peaks
HILL_TRIS = []
NH = 40
for k in range(NH):
    a0, a1 = 2 * math.pi * k / NH, 2 * math.pi * (k + 1) / NH
    am = (a0 + a1) / 2
    r0 = 64
    hpk = rng.uniform(5, 15)
    b0 = np.array([math.sin(a0) * r0, FLOOR_Y, -math.cos(a0) * r0])
    b1 = np.array([math.sin(a1) * r0, FLOOR_Y, -math.cos(a1) * r0])
    pk = np.array([math.sin(am) * (r0 + 3), FLOOR_Y + hpk, -math.cos(am) * (r0 + 3)])
    for tri in ([b0, pk, b1],):
        n = np.cross(tri[1] - tri[0], tri[2] - tri[0]); n /= np.linalg.norm(n)
        if np.dot(n, -pk) < 0:
            n = -n
        HILL_TRIS.append((np.array(tri), n))
    # a second, lower shoulder peak for more facets
    pk2 = np.array([math.sin(a1) * (r0 - 2), FLOOR_Y + hpk * rng.uniform(0.4, 0.7), -math.cos(a1) * (r0 - 2)])
    tri = [pk, b1, pk2]
    n = np.cross(tri[1] - tri[0], tri[2] - tri[0]); n /= np.linalg.norm(n)
    if np.dot(n, -pk) < 0:
        n = -n
    HILL_TRIS.append((np.array(tri), n))

# floating crystals (octahedra) behind the stage
CRYSTALS = []
for k, (ax, ay, az, s) in enumerate([(-15, 6.5, -20, 1.3), (12, 8.5, -24, 1.7), (-24, 10, -30, 2.2),
                                     (22, 5.0, -17, 1.0), (2, 12, -34, 2.4), (-8, 4.2, -15, 0.8),
                                     (28, 11, -32, 2.0), (-30, 5.5, -22, 1.4)]):
    CRYSTALS.append(dict(c=np.array([ax, ay, az], float), s=s, ph=rng.uniform(0, 6.28),
                         sp=rng.uniform(0.25, 0.55) * rng.choice([-1, 1])))

# ---------------------------------------------------------------- timing helpers
def appear_order():
    order = {HOLDOUT: -1.0}   # already there on frame 0
    rest_row = [3, 5, 2, 6, 1, 7, 0, 8, 9]
    for j, c in enumerate(rest_row):
        order[9 * COLS + c] = 1.5 + 0.14 * j
    others = [i for i in range(N) if ROW[i] < 9]
    others.sort(key=lambda i: (math.hypot(MX[i] - HX, MZ[i] - HZ), i))
    for j, i in enumerate(others):
        order[i] = 3.0 + 1.85 * (j / (len(others) - 1)) ** 0.85
    return np.array([order[i] for i in range(N)])


APPEAR = appear_order()
_far = np.argsort(-np.hypot(MX - HX, MZ - HZ))
FADE_START = np.zeros(N)
for j, i in enumerate(_far):
    FADE_START[i] = 54.6 + 1.6 * j / (N - 2)
FADE_START[HOLDOUT] = 99


def pop_scale(i, t):
    u = (t - APPEAR[i]) / 0.28
    if u <= 0:
        return 0.0
    if u >= 1:
        return 1.0
    return 1 + math.sin(u * math.pi * 1.5) * (1 - u) * 0.35 - (1 - u) ** 3


def presence(i, t):
    if i == HOLDOUT:
        return 1.0
    return 1 - sstep((t - FADE_START[i]) / 0.9)


def tips_on(i, t):
    if i == HOLDOUT:
        return 1.0
    return 1 - sstep((t - (FADE_START[i] - 0.5)) / 0.35)


T_LIFT0, T_LIFT1, T_SETTLE = 10.6, 11.5, T_UNLOCK
CAN_X = 3.2
CAN_R = 0.42
ON_CANS_TOP = 2 * CAN_R          # board top height once on the cans


def board_state(f, t):
    """board top height, board x-offset, can x positions, can roll angle"""
    if t < T_LIFT0:
        top = 0.0
    elif t < T_LIFT1:
        top = (ON_CANS_TOP + 0.55) * ease((t - T_LIFT0) / (T_LIFT1 - T_LIFT0))
    elif t < 11.95:
        top = ON_CANS_TOP + 0.55
    elif t < T_SETTLE:
        top = ON_CANS_TOP + 0.55 * (1 - ease((t - 11.95) / (T_SETTLE - 11.95)) ** 1.5)
    else:
        u = t - T_SETTLE
        top = ON_CANS_TOP - 0.05 * math.exp(-u * 7) * math.cos(u * 30)
    # cans roll in
    if t < 10.9:
        cx = 14.0
    else:
        cx = CAN_X + (14.0 - CAN_X) * (1 - ease((t - 10.9) / 1.05))
    dx = 0.0
    if t >= T_SETTLE:
        w = np.array([presence(i, t) * (1.0 if pop_scale(i, t) > 0 else 0) for i in range(N)])
        dx = -0.36 * float((w * np.sin(THETA[f])).sum()) / N * sstep((t - T_SETTLE) / 0.6)
    return top, dx, cx


# ---------------------------------------------------------------- camera
# (time, target, dist, yaw, pitch, fov, screen_y_of_target)
def KF(t, tx, ty, tz, d, yaw, pitch, fov=38, sy=290):
    return (t, np.array([tx, ty, tz], float), d, yaw, pitch, fov, sy)


B1 = ON_CANS_TOP
KEYS = [
    KF(0.0, HX, 0.62, HZ, 3.0, -6, 10, 38, 312),
    KF(1.45, HX, 0.60, HZ, 2.75, 3, 12, 38, 312),
    KF(2.95, 0.0, 0.5, HZ, 9.5, 58, 14, 46, 290),
    KF(5.0, 0.0, 0.2, 0.3, 25.0, -8, 38, 50, 262),
    KF(10.4, 0.0, 0.25, 0.3, 24.0, 8, 33, 50, 262),
    KF(12.7, 0.0, 0.5, 0.6, 22.0, 16, 16, 50, 255),
    KF(14.6, 0.0, 0.6, 0.6, 22.0, 20, 18, 50, 255),
    KF(20.6, 0.0, 0.7, 1.5, 15.0, 38, 12, 50, 250),
    KF(26.2, 0.0, B1, 0.3, 24.0, -6, 42, 50, 262),
    KF(31.4, 0.0, B1, 0.3, 24.5, -14, 34, 50, 262),
    KF(33.2, HX, B1 + 0.62, HZ, 4.6, -10, 20, 40, 290),
    KF(T_DROP - 0.02, HX, B1 + 0.62, HZ, 3.9, 7, 18, 40, 290),
]
BAR = 4 * BEAT
# camera segments during the drop; each segment cuts in on a bar line
DROP_SEGS = [
    (T_DROP, T_DROP + 2 * BAR,
     KF(0, 0.0, B1, 0.0, 21.0, -4, 52, 50, 262), KF(0, 0.0, B1, 0.0, 19.5, 10, 50, 50, 262)),
    (T_DROP + 2 * BAR, T_DROP + 4 * BAR,
     KF(0, 0.0, B1 + 0.4, 1.8, 11.5, 38, 15, 50, 262), KF(0, 0.0, B1 + 0.4, 1.0, 12.5, 20, 22, 50, 262)),
    (T_DROP + 4 * BAR, T_DROP + 6 * BAR,
     KF(0, 0.0, B1, 0.0, 19.5, -22, 60, 50, 262), KF(0, 0.0, B1, 0.0, 18.5, 16, 58, 50, 262)),
    (T_DROP + 6 * BAR, T_DROP + 7 * BAR,
     KF(0, 0.0, B1, -0.3, 19.5, 0, 66, 50, 262), KF(0, 0.0, B1, -0.3, 19.0, 3, 66, 50, 262)),
    (T_DROP + 7 * BAR, T_MUSIC_END,
     KF(0, 0.0, B1, 0.5, 16.5, -26, 34, 50, 262), KF(0, 0.0, B1, 0.5, 18.0, -8, 38, 50, 262)),
]
KEYS_OUT = [
    KF(T_MUSIC_END, 0.0, B1, 0.5, 18.0, -8, 38, 50, 262),
    KF(57.6, HX, B1 + 0.62, HZ, 3.3, -4, 12, 38, 312),
    KF(60.1, HX, B1 + 0.62, HZ, 3.0, -6, 10, 38, 312),
]


def cam_key(t):
    if t < T_DROP:
        return interp_keys(KEYS, t)
    for t0, t1, ka, kb in DROP_SEGS:
        if t0 <= t < t1:
            u = (t - t0) / (t1 - t0)
            return (t, lerp(ka[1], kb[1], u), lerp(ka[2], kb[2], u), lerp(ka[3], kb[3], u),
                    lerp(ka[4], kb[4], u), lerp(ka[5], kb[5], u), lerp(ka[6], kb[6], u))
    return interp_keys(KEYS_OUT, t)


def interp_keys(keys, t):
    if t <= keys[0][0]:
        return keys[0]
    for a, b in zip(keys, keys[1:]):
        if t <= b[0]:
            u = ease((t - a[0]) / (b[0] - a[0]))
            return (t, lerp(a[1], b[1], u), lerp(a[2], b[2], u), lerp(a[3], b[3], u),
                    lerp(a[4], b[4], u), lerp(a[5], b[5], u), lerp(a[6], b[6], u))
    return keys[-1]


def beat_env(t, decay=0.09):
    if t < T_DROP or t > T_MUSIC_END + 0.4:
        return 0.0
    b = (t - T_DROP) / BEAT
    return math.exp(-(b - math.floor(b)) * BEAT / decay)


class Cam:
    def __init__(self, t):
        k = cam_key(t)
        _, tgt, d, yaw, pitch, fov, sy = k
        tgt = tgt.copy()
        # shakes: board clunk and the drop
        shakes = [(T_SETTLE, 0.05, 0.4), (T_DROP, 0.35, 0.7)] + \
            [(T_DROP + k * 2 * BAR, 0.12, 0.35) for k in (1, 2, 3)]
        for t0, amp, dur in shakes:
            if t0 <= t < t0 + dur:
                a = amp * (1 - (t - t0) / dur) ** 2
                tgt += a * np.array([math.sin(t * 91.0), math.sin(t * 77.0 + 1), math.sin(t * 63.0 + 2)])
        fov *= 1 - 0.022 * beat_env(t)
        yr, pr = math.radians(yaw), math.radians(pitch)
        self.pos = tgt + d * np.array([math.sin(yr) * math.cos(pr), math.sin(pr), math.cos(yr) * math.cos(pr)])
        fwd = tgt - self.pos; fwd /= np.linalg.norm(fwd)
        right = np.cross(fwd, [0, 1, 0]); right /= np.linalg.norm(right)
        up = np.cross(right, fwd)
        self.M = np.stack([right, up, fwd])
        self.f = (H / 2) / math.tan(math.radians(fov) / 2)
        self.cx = W / 2
        self.cy = sy
        self.pitch = pr

    def to_cam(self, P):
        return (P - self.pos) @ self.M.T

    def proj(self, Pc):
        z = np.maximum(Pc[..., 2], 1e-3)
        return np.stack([self.cx + self.f * Pc[..., 0] / z, self.cy - self.f * Pc[..., 1] / z], -1)

    def horizon_y(self):
        return self.cy - self.f * math.tan(self.pitch)


NEAR = 0.08


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


def draw_poly_world(draw, cam, P, color):
    Pc = cam.to_cam(P)
    if (Pc[:, 2] < NEAR).any():
        Pc = clip_near(Pc)
        if Pc is None:
            return
    s = cam.proj(Pc)
    draw.polygon([tuple(p) for p in s], fill=tuple(int(c) for c in color))


def fogged(col, dist, d0=16.0, d1=62.0, mx=0.88):
    f = min(mx, max(0.0, (dist - d0) / (d1 - d0)))
    return col * (1 - f) + FOG * f


# ---------------------------------------------------------------- background
BAYER = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]) / 16.0
BAYER_FULL = np.tile(BAYER, (H // 4, W // 4))


def sky(cam, t):
    hy = cam.horizon_y()
    ys = np.arange(H)[:, None]
    u = np.clip((hy - ys) / 420.0, 0, 1)                  # 0 at horizon, 1 high up
    levels = 9
    q = np.floor(u * levels + BAYER_FULL) / levels
    q = np.clip(q, 0, 1)[..., None]
    top = SKY_TOP.astype(float)
    if t >= T_DROP and t < T_MUSIC_END + 1.5:              # stage light tint on each bar
        bar = int((t - T_DROP) / (4 * BEAT))
        tint = np.array(LED[(bar * 2 + 1) % len(LED)], float)
        k = 0.10 + 0.10 * beat_env(t, 0.25)
        top = top * (1 - k) + tint * k * 0.6
    img = SKY_HOR * (1 - q) + top * q
    below = ys >= hy
    img = np.where(below[..., None], FOG, img)
    return img.astype(np.uint8)


VIG = None


def vignette():
    global VIG
    if VIG is None:
        yy, xx = np.mgrid[0:H, 0:W]
        r = np.hypot((xx - W / 2) / (W * 0.62), (yy - H * 0.47) / (H * 0.62))
        v = np.clip(r - 0.55, 0, 1) * 0.9
        v = np.floor(v * 5 + BAYER_FULL * 0.999) / 5
        VIG = (1 - np.clip(v, 0, 0.5))[..., None]
    return VIG


SCRIM = None


def top_scrim(t):
    """Dithered dark band behind the captions so text reads over busy shots."""
    global SCRIM
    if SCRIM is None:
        ys = np.arange(H)[:, None] * np.ones((1, W))
        v = np.clip(1 - (ys - 40) / 150.0, 0, 1) * 0.62
        v = np.floor(v * 6 + BAYER_FULL * 0.999) / 6
        SCRIM = (1 - v)[..., None]
    return SCRIM


# ---------------------------------------------------------------- glow sprites
def glow_kernel(r):
    yy, xx = np.mgrid[-r:r + 1, -r:r + 1]
    d = np.hypot(xx, yy) / (r + 0.5)
    v = np.clip(1 - d, 0, 1) ** 1.6
    b = np.tile(BAYER, ((2 * r + 1) // 4 + 1, (2 * r + 1) // 4 + 1))[:2 * r + 1, :2 * r + 1]
    return (np.floor(v * 4 + b * 0.999) / 4)


GLOW = {r: glow_kernel(r) for r in range(1, 26)}


def add_glow(img, x, y, r, col, inten):
    r = int(max(1, min(25, round(r))))
    k = GLOW[r] * inten
    x, y = int(round(x)), int(round(y))
    x0, y0, x1, y1 = x - r, y - r, x + r + 1, y + r + 1
    if x1 <= 0 or y1 <= 0 or x0 >= W or y0 >= H:
        return
    kx0, ky0 = max(0, -x0), max(0, -y0)
    kx1, ky1 = k.shape[1] - max(0, x1 - W), k.shape[0] - max(0, y1 - H)
    sub = k[ky0:ky1, kx0:kx1, None] * np.array(col, float)[None, None]
    img[max(0, y0):min(H, y1), max(0, x0):min(W, x1)] += sub


# ---------------------------------------------------------------- particles
def make_particles():
    P = []
    pr = np.random.default_rng(11)
    # drop burst: confetti from every tip
    for i in range(N):
        for k in range(3):
            P.append(dict(i=i, t0=T_DROP, v=np.array([pr.normal(0, 1.4), pr.uniform(2.5, 5.5), pr.normal(0, 1.4)]),
                          life=pr.uniform(0.9, 1.6), col=LED[pr.integers(len(LED))]))
    # small sparkles on every bar downbeat
    for bar in range(1, 8):
        for k in range(26):
            i = int(pr.integers(N))
            P.append(dict(i=i, t0=T_DROP + bar * 4 * BEAT, v=np.array([pr.normal(0, 0.9), pr.uniform(1.5, 3.5), pr.normal(0, 0.9)]),
                          life=pr.uniform(0.6, 1.0), col=LED[pr.integers(len(LED))]))
    return P


PARTS = make_particles()

# ---------------------------------------------------------------- captions
CAPS = [
    (5.25, 8.4, ["ALL STARTED", "AT *RANDOM*"]),
    (8.4, 10.6, ["ON A *FIXED* TABLE", "THEY STAY A MESS"]),
    (10.6, 13.6, ["NOW PUT THE BOARD", "ON *2 SODA CANS*"]),
    (13.6, 18.6, ["EVERY *SWING*", "SHOVES THE BOARD"]),
    (18.6, 23.6, ["THE BOARD", "SHOVES *BACK*"]),
    (23.6, 28.0, ["NOBODY IS", "IN CHARGE"]),
    (28.0, 31.5, ["THE CROWD", "PULLS *EVERYONE* IN"]),
    (31.5, 34.9, ["99 IN SYNC.", "~ONE HOLDOUT~"]),
    (34.9, T_DROP, ["...GETTING", "DRAGGED *IN*"]),
    (T_DROP, T_DROP + 3.2, ["*100/100*", "IN SYNC!"]),
    (44.2, 49.85, ["SAME MATH SYNCS", "*FIREFLIES*,", "*HEART CELLS* AND", "*POWER GRIDS*"]),
    (T_MUSIC_END + 0.3, 60.1, ["TRY IT:", "*2 METRONOMES*", "+ 2 SODA CANS"]),
]


def draw_captions(img, t):
    for t0, t1, lines in CAPS:
        if t0 <= t < t1:
            u = t - t0
            nreveal = int(u * 60) + 1
            slide = int(round(6 * (1 - ease(u / 0.25))))
            y = 52 + slide
            used = 0
            big = t0 == T_DROP
            for ln in lines:
                plain_len = len(ln.replace('*', '').replace('~', ''))
                scale = 3 if plain_len <= 18 else 2
                if big and ln.startswith('*100'):
                    scale = 5
                    if u < 0.3:
                        scale = 6
                rich_line(img, ln, W // 2, y, scale, CREAM, MINT, CORAL, nchars=nreveal - used)
                used += plain_len
                y += 7 * scale + 7


def draw_intro_counter(img, t):
    if t >= 5.25:
        return
    n = int((APPEAR <= t).sum())
    label = "METRONOME" if n == 1 else "METRONOMES"
    # bounce whenever the number changes
    last = APPEAR[APPEAR <= t].max() if n else 0
    b = math.exp(-(t - last) * 14)
    scale = 9 if b < 0.5 else 10
    s = str(n)
    m = text_mask(s, scale)
    stamp(img, m, W // 2 - m.shape[1] // 2, 40 - (scale - 9) * 3, MINT)
    lm = text_mask(label, 3)
    stamp(img, lm, W // 2 - lm.shape[1] // 2, 40 + 7 * 9 + 10, CREAM)
    if t >= 0.35:
        rich_line(img, "CAN THEY FIND *ONE BEAT*?", W // 2, 40 + 7 * 9 + 10 + 30, 2,
                  (196, 214, 206), AMBER, CORAL, nchars=int((t - 0.35) * 40) + 1)


# ---------------------------------------------------------------- HUD
HUD_Y = 414


def hud_y0(t):
    a_in = ease((t - 5.25) / 0.35)
    a_out = 1 - ease((t - (T_DROP + 3.0)) / 0.5)
    return HUD_Y + int(round((1 - min(a_in, a_out)) * 140))


def hud_count(f, t):
    cnt = int(COUNT[f])
    frac = cnt / 100
    numcol = sync_color((1 - frac) * 2.2) if cnt < 100 else MINT
    return cnt, numcol


def draw_hud_text(img, f, t):
    if t < 5.25 or t > T_DROP + 3.6:
        return
    y0 = hud_y0(t)
    x0 = 16
    cnt, numcol = hud_count(f, t)
    stamp(img, text_mask("IN SYNC", 2), x0 + 92, y0 + 9, (150, 196, 190))
    sc = 5
    for ms in (25, 50, 75, 100):
        fr = np.nonzero(COUNT >= ms)[0]
        if len(fr) and 0 <= t - fr[0] / FPS < 0.18:
            sc = 6
    m = text_mask(f"{cnt}", sc)
    stamp(img, m, x0 + 92, y0 + 26 - (sc - 5) * 3, numcol)
    stamp(img, text_mask("/100", 2), x0 + 92 + m.shape[1] + 6, y0 + 46, (150, 196, 190))


def draw_hud(img, draw, f, t):
    if t < 5.25 or t > T_DROP + 3.6:
        return
    y0 = hud_y0(t)
    # panel
    x0, x1 = 16, 304
    draw.rectangle([x0, y0, x1, y0 + 82], fill=(12, 26, 32))
    draw.rectangle([x0, y0, x1, y0 + 82], outline=(46, 88, 92))
    draw.rectangle([x0 + 1, y0 + 1, x1 - 1, y0 + 1], fill=(30, 60, 66))
    # phase ring
    rcx, rcy, rr = x0 + 44, y0 + 41, 31
    draw.ellipse([rcx - rr, rcy - rr, rcx + rr, rcy + rr], outline=(52, 96, 100))
    draw.ellipse([rcx - 2, rcy - 2, rcx + 2, rcy + 2], fill=(52, 96, 100))
    th = THETA[f]
    psi = PSI[f]
    vis = APPEAR <= t
    for i in np.nonzero(vis)[0]:
        rel = math.atan2(math.sin(th[i] - psi), math.cos(th[i] - psi))
        ang = th[i] - math.pi / 2
        px = rcx + rr * math.cos(ang)
        py = rcy + rr * math.sin(ang)
        col = sync_color(rel) if i != HOLDOUT or FLAGS[f, i] or t < 31 else CORAL
        size = 1 if i != HOLDOUT else 2
        draw.rectangle([px - size, py - size, px + size, py + size], fill=col)
    # order-parameter arrow
    z = np.exp(1j * th[vis]).mean() if vis.any() else 0
    ax = rcx + (rr - 4) * abs(z) * math.cos(np.angle(z) - math.pi / 2)
    ay = rcy + (rr - 4) * abs(z) * math.sin(np.angle(z) - math.pi / 2)
    draw.line([rcx, rcy, ax, ay], fill=MINT, width=2)
    cnt, numcol = hud_count(f, t)
    # 10-segment bar
    bx, by = x0 + 92, y0 + 68
    for k in range(10):
        sx = bx + k * 19
        draw.rectangle([sx, by, sx + 16, by + 6], fill=(28, 50, 56))
        fill = min(1.0, max(0.0, (cnt - k * 10) / 10))
        if fill > 0:
            draw.rectangle([sx, by, sx + int(round(16 * fill)), by + 6], fill=numcol)


# ---------------------------------------------------------------- main frame
def render(f):
    t = f / FPS
    cam = Cam(t)
    img = sky(cam, t).astype(np.uint8)
    pil = Image.fromarray(img)
    draw = ImageDraw.Draw(pil)
    campos = cam.pos

    # hills
    for tri, n in HILL_TRIS:
        col = shade(n, MAT['hill'], 0.1)
        col = col * 0.45 + FOG * 0.55
        draw_poly_world(draw, cam, tri, col)

    # floor
    for tri, n, tone in FLOOR_TRIS:
        c = tri.mean(0)
        if np.dot(n, campos - c) <= 0:
            continue
        col = shade(n, MAT['floor'] * tone, 0.06)
        col = fogged(col, np.linalg.norm(c - campos), 10, 58, 0.95)
        draw_poly_world(draw, cam, tri, col)

    # crystals
    for k, cr in enumerate(CRYSTALS):
        a = cr['ph'] + cr['sp'] * t
        bob = 0.4 * math.sin(t * 0.7 + cr['ph'])
        s = cr['s']
        ca, sa = math.cos(a), math.sin(a)
        Rm = np.array([[ca, 0, sa], [0, 1, 0], [-sa, 0, ca]])
        V = np.array([[s, 0, 0], [-s, 0, 0], [0, 1.5 * s, 0], [0, -1.5 * s, 0], [0, 0, s], [0, 0, -s]]) @ Rm.T
        V = V + cr['c'] + [0, bob, 0]
        faces = [(0, 2, 4), (4, 2, 1), (1, 2, 5), (5, 2, 0), (4, 3, 0), (1, 3, 4), (5, 3, 1), (0, 3, 5)]
        glowk = 0.0
        if T_DROP <= t < T_MUSIC_END + 1:
            glowk = 0.35 + 0.65 * beat_env(t, 0.2)
        for fc in faces:
            P = V[list(fc)]
            n = np.cross(P[1] - P[0], P[2] - P[0]); n /= np.linalg.norm(n)
            if np.dot(n, P.mean(0) - V.mean(0)) < 0:
                n = -n
            if np.dot(n, campos - P.mean(0)) <= 0:
                continue
            base = MAT['crys']
            if glowk > 0:
                tint = np.array(LED[(k + int((t - T_DROP) / (4 * BEAT))) % len(LED)], float)
                base = base * (1 - 0.5 * glowk) + tint * 0.5 * glowk
            col = fogged(shade(n, base, 0.1), np.linalg.norm(P.mean(0) - campos), 14, 70, 0.7)
            draw_poly_world(draw, cam, P, col)

    top, bdx, canx = board_state(f, t)
    by = top - BOARD_T / 2

    # cans (rollers) under the board, farther one first
    can_items = []
    if canx < 13.9:
        roll = -(bdx / 2) / CAN_R + (canx - CAN_X) / CAN_R
        for sgn, mat in ((-1, 'can_a'), (1, 'can_b')):
            cxp = sgn * canx + bdx / 2
            can_items.append((np.linalg.norm(np.array([cxp, CAN_R + FLOOR_Y, 0]) - campos), cxp, mat, sgn))
        can_items.sort(key=lambda a: -a[0])
        for _, cxp, mat, sgn in can_items:
            cyp = FLOOR_Y + CAN_R
            ang0 = roll * (-sgn if False else 1)
            ring = [(cxp + CAN_R * math.cos(ang0 + k * math.pi / 4), cyp + CAN_R * math.sin(ang0 + k * math.pi / 4)) for k in range(8)]
            L = 5.6
            for k in range(8):
                (x0, y0), (x1, y1) = ring[k], ring[(k + 1) % 8]
                P = np.array([[x0, y0, L], [x1, y1, L], [x1, y1, -L], [x0, y0, -L]])
                n = np.array([(x0 + x1) / 2 - cxp, (y0 + y1) / 2 - cyp, 0]); n /= np.linalg.norm(n)
                if np.dot(n, campos - P.mean(0)) <= 0:
                    continue
                base = MAT[mat] * (1.0 if k % 4 else 0.82)
                draw_poly_world(draw, cam, P, shade(n, base, 0.07))
            for zc, nz in ((L, 1), (-L, -1)):
                n = np.array([0, 0, nz], float)
                if np.dot(n, campos - np.array([cxp, cyp, zc])) <= 0:
                    continue
                P = np.array([[x, y, zc] for x, y in ring])
                draw_poly_world(draw, cam, P, shade(n, MAT['canend'], 0.07))
                inner = np.array([[cxp + 0.62 * (x - cxp), cyp + 0.62 * (y - cyp), zc + nz * 0.002] for x, y in ring])
                draw_poly_world(draw, cam, inner, shade(n, MAT['canend'] * 0.82, 0.07))

    # board
    for P, n, m in box_faces(bdx, by, 0.0, BOARD_HALF, BOARD_T / 2, BOARD_HALF, 'board', 'bside'):
        if np.dot(n, campos - P.mean(0)) <= 0:
            continue
        if m == 'board':
            step = 2 * BOARD_HALF / BT_N
            for a in range(BT_N):
                for b in range(BT_N):
                    x0 = bdx - BOARD_HALF + a * step; z0 = -BOARD_HALF + b * step
                    q = np.array([[x0, top, z0], [x0 + step, top, z0], [x0 + step, top, z0 + step], [x0, top, z0 + step]])
                    for tri, jt in (((0, 1, 2), BT_JIT[a, b, 0]), ((0, 2, 3), BT_JIT[a, b, 1])):
                        col = shade(n, MAT['board'] * (1 + jt), 0.06)
                        draw_poly_world(draw, cam, q[list(tri)], col)
        else:
            draw_poly_world(draw, cam, P, shade(n, MAT[m], 0.07))

    # metronomes ------------------------------------------------------
    th = THETA[f]
    psi = PSI[f]
    scales = np.array([pop_scale(i, t) for i in range(N)])
    pres = np.array([presence(i, t) for i in range(N)])
    alive = np.nonzero((scales > 0) & (pres > 0.02))[0]
    base_pos = np.stack([MX + bdx, np.full(N, top), MZ], 1)
    centers = base_pos + [0, 0.5, 0]
    depth = cam.to_cam(centers)[:, 2]
    order = alive[np.argsort(-depth[alive])]
    face_cols = np.array([shade(fc['n'], MAT[fc['mat']]) for fc in MF])
    face_n = np.array([fc['n'] for fc in MF])
    face_ctr_local = np.array([MV[fc['idx']].mean(0) for fc in MF])
    in_drop = T_DROP <= t < T_MUSIC_END
    glows = []
    rel_all = np.angle(np.exp(1j * (th - psi)))
    for i in order:
        s = scales[i]
        Wv = base_pos[i] + s * MV
        Pc = cam.to_cam(Wv)
        S = cam.proj(Pc)
        dist = np.linalg.norm(centers[i] - campos)
        fogk = min(0.6, max(0.0, (dist - 16) / 46))
        p = pres[i]
        flash = max(0.0, 1 - (t - APPEAR[i]) / 0.12) if t >= APPEAR[i] else 0
        ctrs = base_pos[i] + s * face_ctr_local
        vis = ((campos - ctrs) * face_n).sum(1) > 0
        led = led_color(i, t) if in_drop else None
        for j, fc in enumerate(MF):
            if not vis[j] and fc['part'] != 3:
                continue
            if fc['part'] == 3 and not vis[FRONT_FACE]:
                continue
            col = face_cols[j] * (1 - fogk) + FOG * fogk
            if led is not None and fc['part'] == 3:
                # the front panel becomes a pixel of a 10x10 light show
                lc, lb = led
                col = np.array(lc, float) * (0.18 + 0.72 * lb) + face_cols[j] * 0.2
                pc = S[fc['idx']].mean(0)
                pr = max(3.0, abs(S[fc['idx'][0]][0] - S[fc['idx'][1]][0]) * 0.9)
                glows.append((pc[0], pc[1], pr * 1.6, lc, 0.22 * lb))
            if flash > 0:
                col = col + (255 - col) * flash * 0.8
            if p < 1:
                col = col * p + np.array([16, 32, 40]) * (1 - p)
            draw.polygon([tuple(S[k]) for k in fc['idx']], fill=tuple(int(c) for c in col))
        if not vis[FRONT_FACE]:
            continue
        # pendulum
        phi = AMP * math.sin(th[i])
        piv = base_pos[i] + s * PIVOT
        dirv = np.array([math.sin(phi), math.cos(phi), 0.0])
        tip = piv + s * ROD * dirv
        wgt = piv + s * ROD * 0.52 * dirv
        sp = cam.proj(cam.to_cam(np.stack([piv, tip, wgt])))
        zc = cam.to_cam(piv)[2]
        ppu = cam.f / max(zc, 0.1)
        rodw = max(1, int(round(0.022 * ppu * s)))
        rodcol = np.array([206, 212, 218]) * p + np.array([16, 32, 40]) * (1 - p)
        draw.line([tuple(sp[0]), tuple(sp[1])], fill=tuple(int(c) for c in rodcol), width=rodw)
        # sliding weight
        wr = 0.05 * ppu * s
        perp = np.array([math.cos(phi), -math.sin(phi)])
        along = np.array([math.sin(phi), -math.cos(phi)])   # screen-space approx (y down)
        cw = sp[2]
        quad = [cw + perp * wr * 1.3 - along * wr * 0.8, cw - perp * wr * 1.3 - along * wr * 0.8,
                cw - perp * wr * 0.9 + along * wr * 0.8, cw + perp * wr * 0.9 + along * wr * 0.8]
        wcol = np.array([70, 74, 92]) * p + np.array([16, 32, 40]) * (1 - p)
        draw.polygon([tuple(q) for q in quad], fill=tuple(int(c) for c in wcol))
        if wr > 3:
            draw.line([tuple(quad[0]), tuple(quad[1])], fill=(150, 156, 176), width=1)
        # tip light
        on = tips_on(i, t)
        if in_drop:
            tcol, tint = led_color(i, t)
        else:
            tcol = sync_color(rel_all[i]) if (t >= 5.25 or i == HOLDOUT) else sync_color(rel_all[i])
            # flash at each extreme (tick)
            tint = 0.55 + 0.45 * abs(math.sin(th[i])) ** 8
            if i == HOLDOUT and 31.0 <= t < T_HOLD_IN:
                tcol = sync_color(rel_all[i])
        if t >= T_MUSIC_END:
            tcol = sync_color(0.0) if i == HOLDOUT else tcol
        tr = max(1.2, 0.055 * ppu * s) * (1.5 if in_drop else 1.0)
        tc = np.array(tcol, float)
        core = tc * (0.55 + 0.45 * tint) * on + np.array([40, 44, 52]) * (1 - on)
        core = core * p + np.array([16, 32, 40]) * (1 - p)
        x, y = sp[1]
        oct_pts = [(x + tr * math.cos(k * math.pi / 4 + 0.39), y + tr * math.sin(k * math.pi / 4 + 0.39)) for k in range(8)]
        draw.polygon(oct_pts, fill=tuple(int(c) for c in np.clip(core, 0, 255)))
        if on * p > 0.05:
            glows.append((x, y, tr * 3.2, tc, 0.38 * tint * on * p))
    if 13.8 <= t < 23.6:
        # show the board being shoved: an arrow under its front edge
        _, bdx_n, _ = board_state(min(f + 1, NF - 1), (f + 1) / FPS)
        _, bdx_p, _ = board_state(max(f - 1, 0), (f - 1) / FPS)
        vel = (bdx_n - bdx_p) * FPS / 2
        a = np.array([[bdx, top - BOARD_T, BOARD_HALF], [bdx + 1.0, top - BOARD_T, BOARD_HALF]])
        ac = cam.to_cam(a)
        if (ac[:, 2] > NEAR).all():
            sa = cam.proj(ac)
            dirx = sa[1] - sa[0]
            dirx /= np.linalg.norm(dirx) + 1e-9
            ln = min(70.0, abs(vel) * 260)
            if ln > 3:
                sgn = 1 if vel > 0 else -1
                c0 = sa[0] + np.array([0, 16])
                c0[0] = min(W - 80, max(80, c0[0]))
                tip = c0 + dirx * sgn * ln
                nrm = np.array([-dirx[1], dirx[0]])
                head = [tuple(tip + dirx * sgn * 9), tuple(tip + nrm * 7), tuple(tip - nrm * 7)]
                draw.line([tuple(c0 + np.array([1, 1])), tuple(tip + np.array([1, 1]))], fill=INK, width=4)
                draw.polygon([(x + 1, y + 1) for x, y in head], fill=INK)
                draw.line([tuple(c0), tuple(tip)], fill=AMBER, width=4)
                draw.polygon(head, fill=AMBER)
    if 31.5 <= t < T_DROP:
        capp = base_pos[HOLDOUT] + np.array([0, Y_TOP + 0.32 + 0.06 * math.sin(t * 9), 0])
        mc = cam.to_cam(capp[None])[0]
        if mc[2] > NEAR:
            mx, my = cam.proj(mc[None])[0]
            sz = max(4, min(14, 0.11 * cam.f / mc[2]))
            col = CORAL if t < T_HOLD_IN else MINT
            tri = [(mx - sz, my - sz * 1.2), (mx + sz, my - sz * 1.2), (mx, my)]
            draw.polygon([(x + 1, y + 1) for x, y in tri], fill=INK)
            draw.polygon(tri, fill=col)
    out = np.asarray(pil).astype(np.float32)
    for x, y, r, col, inten in glows:
        add_glow(out, x, y, r, col, inten)

    # particles
    if t >= T_DROP:
        for pt in PARTS:
            u = t - pt['t0']
            if u < 0 or u > pt['life']:
                continue
            i = pt['i']
            phi = AMP * math.sin(th[i])
            p0 = base_pos[i] + PIVOT + ROD * np.array([math.sin(phi), math.cos(phi), 0])
            P = p0 + pt['v'] * u + np.array([0, -4.5, 0]) * u * u
            pc = cam.to_cam(P[None])[0]
            if pc[2] < NEAR:
                continue
            sx, sy = cam.proj(pc[None])[0]
            sz = 2 if pc[2] < 14 else 1
            fade = 1 - u / pt['life']
            xi, yi = int(sx), int(sy)
            if 0 <= xi < W - sz and 0 <= yi < H - sz:
                out[yi:yi + sz, xi:xi + sz] = out[yi:yi + sz, xi:xi + sz] * (1 - fade) + np.array(pt['col']) * fade

    out = np.clip(out, 0, 255)
    out = out * vignette()
    out = out * top_scrim(t)

    # flash on the drop
    if T_DROP <= t < T_DROP + 0.4:
        k = (1 - (t - T_DROP) / 0.4) ** 2 * 0.8
        out = out * (1 - k) + np.array([230, 255, 240]) * k
    img = out.astype(np.uint8)

    # UI
    pil = Image.fromarray(img)
    draw = ImageDraw.Draw(pil)
    draw_hud(None, draw, f, t)        # panel shapes via PIL
    img2 = np.asarray(pil).copy()
    draw_hud_text(img2, f, t)         # text stamped on top
    draw_intro_counter(img2, t)
    draw_captions(img2, t)
    # watermark-ish handle, small, bottom left above IG UI
    return img2


if __name__ == '__main__':
    if sys.argv[1] == 'test':
        os.makedirs('test', exist_ok=True)
        for tt in sys.argv[2:]:
            f = int(round(float(tt) * FPS))
            im = render(f)
            Image.fromarray(im).resize((W * UP, H * UP), Image.NEAREST).save(f'test/f_{float(tt):05.1f}.png')
            print('wrote', tt)
