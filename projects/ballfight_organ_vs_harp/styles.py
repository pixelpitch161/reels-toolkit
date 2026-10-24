"""#35 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'pipes'    ORGAN: a row of tin organ pipes erupting out of the canvas toward the enemy (hex prisms with a dark
                     mouth, a wine glow in each top as it speaks), rising and sinking in sequence along the row
    style 'strings'  HARP: glowing tripwire strings stretched rope to rope (drawn out from the harp's side when laid,
                     vibrating when crossed, fading when the harp lays one too many)
    'dart'           PLUCK: a plucked-string dart with a bright pearl head (arc projectiles)
    'dome'           CATHEDRAL (shield): an echoing gothic dome over the organ ball: pointed-arch ribs, stained-glass tint,
                     echo rings
    'wings'          ANGEL WINGS (shield): two feathered wings folded around the harp ball + a golden halo
    'forest'         TOCCATA (slam): a forest of pipes erupting inside the target circle
    'snap'           47 STRINGS (wave): string ends whipping outward with the wave front

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL

# ---------------------------------------------------------------- organ pipes (line)
TIN = np.array([222, 228, 242])
TIN_DK = np.array([128, 132, 150])
MOUTH = np.array([30, 22, 34])
PIPE_DX = 0.34                       # spacing along a row
PIPE_R = 0.15
RISE, SINK = 0.07, 0.12              # s a pipe takes to shoot up / sink back


def pipe_height(x, L):
    """a row ramps up toward the enemy (a crescendo of pipes), with a little organ-front rhythm in it"""
    u = x / max(L, 1e-6)
    return 0.95 + 0.95 * u + 0.14 * math.cos(x * 7.3)


def row_pipes(line, t, t_end=None):
    """[(sx, sz, height, glow)] of one row's pipes that are out of the floor at time t (t_end: freeze at the hit-stop)"""
    out = []
    L = line['L']
    if L <= 0.05:
        return out
    n = max(1, int(L / PIPE_DX))
    for j in range(n):
        x = (j + 0.5) * L / n
        te = line['t0'] + x / line['speed']
        u = t - te
        if u < 0:
            continue
        up = min(1.0, u / RISE)
        down = 1.0 - min(1.0, max(0.0, (u - line['linger']) / SINK))
        k = min(up, down)
        if k <= 0:
            continue
        k = 1 - (1 - k) ** 2
        p = line['p0'] + line['u'] * x
        h = pipe_height(x, L) * k
        glow = math.exp(-u / 0.18)
        out.append((float(p[0]), float(p[1]), h, glow))
    return out


def pipe_faces(cx, cz, h, r=PIPE_R, n=6, y0=0.0):
    """side quads + top cap of a hex pipe in world coords: [(Q (k,3), normal, kind)]"""
    a = np.arange(n) * 2 * math.pi / n + math.pi / 6
    ring = np.stack([cx + r * np.cos(a), cz + r * np.sin(a)], 1)
    bot = S.to_world(ring[:, 0], ring[:, 1], y0)
    top = S.to_world(ring[:, 0], ring[:, 1], y0 + h)
    out = []
    for j in range(n):
        j2 = (j + 1) % n
        Q = np.array([bot[j], bot[j2], top[j2], top[j]])
        mid = (ring[j] + ring[j2]) / 2 - np.array([cx, cz])
        nw = S.to_world(mid[0], mid[1], 0.0)
        nw = nw / (np.linalg.norm(nw) + 1e-9)
        out.append((Q, nw, 'side', j))
    out.append((top, np.array([0.0, 1.0, 0.0]), 'top', -1))
    return out


def draw_pipe(fr, cam, L, shade, cx, cz, h, glow, col, r=PIPE_R):
    """one tin pipe; `col` = the owner's colour (the top glows in it as the pipe speaks)"""
    if h < 0.03:
        return
    for Q, nw, kind, j in pipe_faces(cx, cz, h, r):
        if kind == 'side':
            if np.dot(nw, cam.pos - Q.mean(0)) <= 0:
                continue
            base = TIN if j % 2 == 0 else TIN * 0.92
            c = shade(L, nw[None], Q.mean(0)[None], base, rimk=0.55)[0]
            fr.poly(cam, Q, c)
        else:
            c = TIN_DK * (0.75 - 0.35 * glow) + np.array(col) * 0.75 * glow + 30 * glow
            fr.poly(cam, Q, np.clip(c, 0, 255))
    # the mouth: a dark slot a third of the way up, facing the camera
    v = cam.pos - S.to_world(cx, cz, 0.0)
    ang = math.atan2(v[2], v[0])
    d = np.array([math.cos(ang), math.sin(ang)])
    # back to the ring frame: world X = (sx - sz)/sqrt2, Z = (sx + sz)/sqrt2
    sx_d, sz_d = (d[0] + d[1]) / math.sqrt(2), (d[1] - d[0]) / math.sqrt(2)
    m = np.array([cx + sx_d * r * 1.01, cz + sz_d * r * 1.01])
    t_ = np.array([-sz_d, sx_d]) * r * 0.45
    ym, hm = min(0.32, h * 0.3), min(0.1, h * 0.12)
    Q = np.array([S.to_world(*(m - t_), ym), S.to_world(*(m + t_), ym), S.to_world(*(m + t_ * 0.6), ym + hm),
                  S.to_world(*(m - t_ * 0.6), ym + hm)])
    fr.poly(cam, Q, MOUTH)
    # an ink outline round the pipe's silhouette, so a row reads as separate pipes
    a = np.arange(6) * math.pi / 3 + math.pi / 6
    ring = np.stack([cx + r * np.cos(a), cz + r * np.sin(a)], 1)
    P = np.vstack([S.to_world(ring[:, 0], ring[:, 1], 0.0), S.to_world(ring[:, 0], ring[:, 1], h)])
    sp = cam.wp(P)
    from scipy.spatial import ConvexHull
    try:
        hull = sp[ConvexHull(sp).vertices]
        fr.draw.polygon([tuple(q) for q in hull], outline=(40, 30, 48))
    except Exception:
        pass


def pipe_shadow_quad(cx, cz, h, r=PIPE_R):
    """a vertical card through the pipe (for the spot's shadow on the canvas)"""
    return np.array([S.to_world(cx - r, cz + r, 0.0), S.to_world(cx + r, cz - r, 0.0),
                     S.to_world(cx + r, cz - r, h), S.to_world(cx - r, cz + r, h)])


# ---------------------------------------------------------------- tripwire strings (trap)
PEARL = np.array([255, 246, 222])
STR_Y = 0.50                          # string height (just under the balls' centres)
DRAW_IN, FADE_OUT = 0.16, 0.35        # s to stretch a new string across / to fade a dropped one


def string_state(st, t):
    """(alpha, drawn fraction) of a string at time t, or None when it's gone"""
    if t < st['t0']:
        return None
    drawn = min(1.0, (t - st['t0']) / DRAW_IN)
    a = 1.0
    if st['t_off'] is not None and t >= st['t_off']:
        a = 1 - (t - st['t_off']) / FADE_OUT
        if a <= 0:
            return None
    return a, drawn


def string_points(st, t, n=17, harp_p=None):
    """world points of a string: straight, plucked into a decaying standing wave where it was crossed; while it's being
    laid it grows from the end nearer the harp"""
    a, b = st['a'], st['b']
    sst = string_state(st, t)
    if sst is None:
        return None, 0.0
    alpha, drawn = sst
    if harp_p is not None and np.linalg.norm(harp_p - b) < np.linalg.norm(harp_p - a):
        a, b = b, a
    u = np.linspace(0, 1, n) * drawn
    P = a[None] + (b - a)[None] * u[:, None]
    d = b - a
    nrm = np.array([-d[1], d[0]]) / (np.linalg.norm(d) + 1e-9)
    disp = np.zeros(n)
    for th in st['hits']:
        dt = t - th
        if 0 <= dt < 0.9:
            disp += 0.16 * math.exp(-dt / 0.22) * math.sin(2 * math.pi * 9.0 * dt) * np.sin(np.pi * u)
    P = P + nrm[None] * disp[:, None]
    y = STR_Y + 0.05 * np.abs(disp)
    return S.to_world(P[:, 0], P[:, 1], y), alpha


def string_twang(st, t):
    """0..1: how hard the string is ringing right now (for its glow)"""
    k = 0.0
    for th in st['hits']:
        if 0 <= t - th < 0.6:
            k = max(k, math.exp(-(t - th) / 0.2))
    return k


def draw_string(arr, cam, P, alpha, col, twang=0.0):
    """a 1-px bright core over a 3-px soft halo, alpha-blended"""
    s = cam.wp(P)
    for width, k, c in ((3, 0.30 + 0.35 * twang, np.array(col) * 0.75 + 40), (1, 0.95, np.clip(PEARL + 40 * twang, 0, 255))):
        im = Image.new('L', (W, H), 0)
        ImageDraw.Draw(im).line([tuple(q) for q in s], fill=255, width=width)
        mk = np.asarray(im, np.float32) / 255.0 * alpha * k
        arr += (np.clip(c, 0, 255)[None, None] - arr) * mk[..., None]
    for q in (s[0], s[-1]):                                  # the knots on the ropes
        xi, yi = int(round(q[0])), int(round(q[1]))
        if 1 <= xi < W - 1 and 1 <= yi < H - 1:
            arr[yi - 1:yi + 1, xi - 1:xi + 1] = arr[yi - 1:yi + 1, xi - 1:xi + 1] * (1 - alpha) + PEARL * alpha


# ---------------------------------------------------------------- PLUCK darts (arc projectiles)
def dart_quads(x, z, ang, y=0.48, L_=0.34, Wd=0.07):
    d = np.array([math.cos(ang), math.sin(ang)])
    n = np.array([-d[1], d[0]])
    c = np.array([x, z])
    head = [c + d * L_ * 0.5, c + d * L_ * 0.15 - n * Wd, c + d * L_ * 0.05, c + d * L_ * 0.15 + n * Wd]
    body = [c + d * L_ * 0.1 - n * 0.012, c - d * L_ * 0.5 - n * 0.012, c - d * L_ * 0.5 + n * 0.012, c + d * L_ * 0.1 + n * 0.012]
    return (np.array([S.to_world(q[0], q[1], y) for q in head]), np.array([S.to_world(q[0], q[1], y) for q in body]))


def draw_dart(fr, cam, L, shade, x, z, ang, col):
    head, body = dart_quads(x, z, ang)
    fr.poly(cam, body, np.clip(np.array(col) * 0.6 + 90, 0, 255))
    c = shade(L, np.array([[0, 1.0, 0]]), head.mean(0)[None], np.array(col) * 0.5 + PEARL * 0.5, rimk=0.4)[0]
    fr.poly(cam, head, np.clip(c + 25, 0, 255))


# ---------------------------------------------------------------- masks
def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def line_mask(segs, width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for a, b in segs:
        d.line([tuple(a), tuple(b)], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


# ---------------------------------------------------------------- CATHEDRAL: the echoing dome (shield)
GLASS_A = np.array([120, 40, 120])
GLASS_B = np.array([60, 90, 200])
GOLD = np.array([255, 206, 110])


def dome_points(cx, cz, rb, hgt, n_rib=8, m=9):
    """ribs of a pointed (gothic) dome: n_rib arcs from the base ring to the apex"""
    ribs = []
    for j in range(n_rib):
        a = 2 * math.pi * j / n_rib + 0.2
        pts = []
        for i in range(m):
            v = i / (m - 1)
            r = rb * (1 - v) ** 0.7                          # a pointed arch: steep near the top
            y = hgt * math.sin(v * math.pi / 2) ** 0.8
            pts.append(S.to_world(cx + r * math.cos(a), cz + r * math.sin(a), y))
        ribs.append(np.array(pts))
    return ribs


def draw_dome(arr, cam, cx, cz, t, k, col, echo_t0):
    """k: 0..1 strength. Stained-glass tint inside the silhouette, golden ribs, and echo rings rolling out on the canvas"""
    rb, hgt = R * 1.55, R * 3.0
    ribs = dome_points(cx, cz, rb, hgt)
    pts = np.vstack([cam.wp(r) for r in ribs])
    from scipy.spatial import ConvexHull
    try:
        hull = pts[ConvexHull(pts).vertices]
    except Exception:
        return
    mk = mask_of([hull])
    yy = np.arange(H)[:, None] * np.ones((1, W))
    y0, y1 = hull[:, 1].min(), hull[:, 1].max()
    v = np.clip((yy - y0) / max(1.0, y1 - y0), 0, 1)
    tint = GLASS_A[None, None] * (1 - v[..., None]) + GLASS_B[None, None] * v[..., None]
    tint = tint * 0.5 + np.array(col)[None, None] * 0.5
    dz = np.floor(BAYER_FULL * 0.9 + 0.5) if k > 0.6 else np.floor(BAYER_FULL * 0.9 + k)
    arr += (tint - arr) * (mk * 0.30 * k * dz)[..., None]
    segs = []
    for r in ribs:
        s = cam.wp(r)
        segs += list(zip(s[:-1], s[1:]))
    a = np.linspace(0, 2 * math.pi, 33)
    base = cam.wp(S.to_world(cx + rb * np.cos(a), cz + rb * np.sin(a), 0.03))
    segs += list(zip(base[:-1], base[1:]))
    mr = line_mask(segs)
    arr += (np.clip(GOLD * 0.8 + np.array(col) * 0.3, 0, 255)[None, None] - arr) * (mr * 0.85 * k)[..., None]


def echo_rings(t, t0, period):
    """radii (beyond the dome) + alpha of the echo rings: one rolls out every `period` s while the dome stands"""
    out = []
    if t < t0:
        return out
    j = int((t - t0) // period)
    for q in range(max(0, j - 2), j + 1):
        u = t - (t0 + q * period)
        if 0 <= u < 1.0:
            out.append((R * 1.55 + 2.2 * u, (1 - u) * 0.6))
    return out


# ---------------------------------------------------------------- ANGEL WINGS (shield) + halo
FEATHER = np.array([250, 246, 236])
FEATHER_DK = np.array([206, 196, 182])


def wing_polys(cam, cx, cz, t, side, k):
    """one wing (side -1 / +1, screen left / right) as feather quads fanned up and out from behind the ball"""
    flap = 0.18 * math.sin(2 * math.pi * t / 0.95)
    base = S.to_world(cx, cz, R * 1.1)
    bs = cam.wp(base[None])[0]
    rp = R * cam.f / cam.depth(base[None])[0]
    polys = []
    for j in range(6):
        a = math.radians(18 + 22 * j) + flap * (1 + 0.3 * j)
        L_ = rp * (2.2 - 0.18 * j) * (0.4 + 0.6 * k)
        w_ = rp * 0.36
        d = np.array([side * math.cos(a), -math.sin(a)])
        nn = np.array([-d[1], d[0]])
        o = bs + np.array([side * rp * 0.45, -rp * 0.25])
        tip = o + d * L_
        polys.append(([o + nn * w_ * 0.25, o - nn * w_ * 0.25, tip - nn * w_ * 0.5 + d * -w_ * 0.4, tip,
                       tip + nn * w_ * 0.5 - d * w_ * 0.4], j))
    return polys


def draw_wings(arr, cam, cx, cz, t, k):
    for side in (-1, 1):
        for pts, j in wing_polys(cam, cx, cz, t, side, k):
            m = mask_of([pts])
            c = FEATHER if j % 2 == 0 else FEATHER_DK
            arr += (c[None, None] - arr) * (m * 0.92 * k)[..., None]


def draw_halo(arr, cam, cx, cz, t, k):
    a = np.linspace(0, 2 * math.pi, 29)
    y = R * 2.35 + 0.05 * math.sin(2 * math.pi * t / 0.95)
    P = S.to_world(cx + 0.42 * np.cos(a), cz + 0.42 * np.sin(a), y)
    s = cam.wp(P)
    m = line_mask(list(zip(s[:-1], s[1:])), width=2)
    arr += (GOLD[None, None] - arr) * (m * 0.95 * k)[..., None]


# ---------------------------------------------------------------- TOCCATA: a forest of pipes (slam)
def forest(cx, cz, rad, seed, n=13):
    """pipe positions + heights + stagger inside the slam circle (seeded per slam)"""
    rng = np.random.default_rng(seed)
    out = []
    for j in range(n):
        rr = rad * math.sqrt(rng.uniform(0.02, 1.0))
        a = rng.uniform(0, 2 * math.pi)
        out.append((cx + rr * math.cos(a), cz + rr * math.sin(a), rng.uniform(1.1, 2.5), rng.uniform(0, 0.12),
                    rng.uniform(0.14, 0.20)))
    return out


def forest_pipes(t, t_slam, cx, cz, rad, seed):
    """[(sx, sz, h, glow, r)] while the forest is up (from the slam to 0.65 s after)"""
    out = []
    for x, z, hmax, lag, r in forest(cx, cz, rad, seed):
        u = t - t_slam - lag
        if u < 0:
            continue
        up = min(1.0, u / 0.08)
        down = 1 - min(1.0, max(0.0, (u - 0.45) / 0.18))
        kk = min(up, down)
        if kk <= 0:
            continue
        out.append((x, z, hmax * (1 - (1 - kk) ** 2), math.exp(-u / 0.25), r))
    return out


# ---------------------------------------------------------------- 47 STRINGS: string ends whipping out with the wave
def snap_segments(cx, cz, r, t, t0, n=47):
    """47 short string lengths riding the wave front, each one whipped at its own angle"""
    out = []
    rng = np.random.default_rng(47)
    for j in range(n):
        a = 2 * math.pi * j / n + rng.uniform(-0.03, 0.03)
        curl = rng.uniform(-1, 1)
        L_ = 0.35 + 0.25 * rng.uniform()
        tw = curl * 0.9 * math.sin(2 * math.pi * 3.0 * (t - t0) + j)
        p = np.array([cx + r * math.cos(a), cz + r * math.sin(a)])
        d = np.array([math.cos(a + tw), math.sin(a + tw)])
        out.append((p - d * L_ * 0.5, p + d * L_ * 0.5))
    return out
