"""#37 weapon styles (new): the CLARINET's TWELFTH JUMP (vanish / appear bursts and the lavender zip line, arcing like a
leap), the OBOE's TUNING A (the reed at the rim, the narrow beam with the A's sine running along it, ENDLESS NOTE's held
beam), DOUBLE REED's scissoring blades, the OUT OF TUNE wobble, SQUEAK's shrill ring and stun stars. Styles live in the
episode's folder so later crossovers can import them (series rule).

All functions draw on a float (H, W, 3) image at the internal resolution with a reelkit camera; positions are in the
ring frame (x, z) at a height y, mapped through scene.to_world."""
import math
import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL
import scene as S

CANE = np.array([232, 210, 152], float)
CANE_D = np.array([170, 140, 84], float)
THREAD = np.array([60, 150, 86], float)
INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)


def _mask(polys, lines=(), width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(map(float, p)) for p in pts], fill=255)
    for pts in lines:
        if len(pts) >= 2:
            d.line([tuple(map(float, p)) for p in pts], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


def blend(img, mk, col, alpha):
    if alpha <= 0:
        return img
    img += (np.asarray(col, float)[None, None] - img) * (mk * alpha)[..., None]
    return img


def add(img, mk, col, k):
    img += mk[..., None] * np.asarray(col, float)[None, None] * k
    return img


def ribbon(cam, P, hw):
    """screen polygon of a ribbon along ring-frame points P (n, 3: x, z, y) with half-width hw (world units),
    widened across the path in the ring plane"""
    P = np.asarray(P, float)
    d = np.gradient(P[:, :2], axis=0)
    d /= np.linalg.norm(d, axis=1, keepdims=True) + 1e-9
    n = np.stack([-d[:, 1], d[:, 0]], 1)
    hw = np.broadcast_to(np.asarray(hw, float), (len(P),))
    L = P[:, :2] + n * hw[:, None]
    Rr = P[:, :2] - n * hw[:, None]
    a = cam.wp(S.to_world(L[:, 0], L[:, 1], P[:, 2]))
    b = cam.wp(S.to_world(Rr[:, 0], Rr[:, 1], P[:, 2]))
    return np.vstack([a, b[::-1]])


# ---------------------------------------------------------------- TWELFTH JUMP
def zip_path(p0, p1, y, n=14):
    """the jump's path: a straight line in the ring plane that arcs up like a leap (higher for longer jumps)"""
    u = np.linspace(0, 1, n)
    xy = p0[None] + (p1 - p0)[None] * u[:, None]
    hop = 0.18 + 0.09 * float(np.hypot(*(p1 - p0)))
    return np.column_stack([xy, y + hop * np.sin(np.pi * u)])


def draw_zip(img, cam, p0, p1, age, col, y, life=0.34, hot=1.0):
    """the zip line from where the clarinet was to where it lands: a tapered lavender bolt with a white core, drawn
    the moment it jumps and fading over `life` s (the tail end first)"""
    if age < 0 or age >= life:
        return img
    k = 1 - age / life
    P = zip_path(np.asarray(p0, float), np.asarray(p1, float), y)
    n = len(P)
    tail = np.clip(np.linspace(0, 1, n) - (1 - k) * 0.9, 0, 1) ** 0.6       # the start fades first
    c = np.asarray(col, float)
    glow = ribbon(cam, P, 0.26 * (0.4 + 0.6 * tail) * (0.6 + 0.4 * hot))
    img = add(img, _mask([glow]), c * 0.6 + 30, 0.34 * k)                     # a luminous lavender halo
    ink = ribbon(cam, P, 0.13 * (0.3 + 0.7 * tail))
    img = blend(img, _mask([ink]), INK, 0.75 * k)                             # ink edge: reads on the lilac canvas
    mid = ribbon(cam, P, 0.085 * (0.3 + 0.7 * tail))
    img = blend(img, _mask([mid]), np.clip(c * 0.85 + 60, 0, 255), 1.0 * k)
    sp = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))
    img = blend(img, _mask([], [sp[int(n * (1 - k) * 0.6):]], 2), WHITE, 1.0 * k)
    return img


def draw_poof(img, cam, p, age, col, y, life=0.22, n=10, r0=0.25, r1=0.95, seed=0):
    """vanish / appear burst: a ring of lavender sparks expanding from the spot (no flash, a few pixels each)"""
    if age < 0 or age >= life:
        return img
    u = age / life
    rr = r0 + (r1 - r0) * (1 - (1 - u) ** 2)
    k = (1 - u) ** 1.2
    rng = np.random.default_rng(seed)
    ph = rng.uniform(0, 2 * math.pi)
    a = ph + np.arange(n) * 2 * math.pi / n
    pts = cam.wp(S.to_world(p[0] + rr * np.cos(a), p[1] + rr * np.sin(a), y + 0.25 * np.sin(a * 3 + ph) * u))
    c = np.clip(np.asarray(col) * 0.8 + 50, 0, 255)
    for x, yy in pts:
        xi, yi = int(round(x)), int(round(yy))
        if 2 <= xi < W - 2 and 2 <= yi < H - 2:
            for dx, dy in ((2, 0), (-2, 0), (0, 2), (0, -2), (1, 1), (-1, 1), (1, -1), (-1, -1)):
                img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - 0.7 * k) + INK * 0.7 * k
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - k) + c * k
            img[yi, xi] = img[yi, xi] * (1 - k) + WHITE * k
    return img


# ---------------------------------------------------------------- TUNING A
def reed_polys(cam, p, aim, y, R, opening=0.0, length=0.34, half_w=0.07):
    """the oboe's reed pointing out of the ball at `aim`: two cane blades hinged at the rim; opening (rad) spreads them
    (DOUBLE REED's scissors); returns [(screen poly, shade)]"""
    out = []
    base = np.asarray(p, float)
    for sgn in (1, -1):
        a = aim + sgn * opening
        d = np.array([math.cos(a), math.sin(a)])
        nrm = np.array([-d[1], d[0]])
        b0 = base + np.array([math.cos(aim), math.sin(aim)]) * (R * 0.92) + nrm * 0.03 * sgn
        tip = b0 + d * length
        q = np.array([b0 + nrm * 0.03, tip + nrm * half_w, tip - nrm * half_w * 0.3, b0 - nrm * 0.03])
        out.append((cam.wp(S.to_world(q[:, 0], q[:, 1], np.full(len(q), y + 0.04 * sgn))), 1.0 if sgn > 0 else 0.82))
    return out


def draw_reed(img, cam, p, aim, y, R, opening=0.0, length=0.34, glow=0.0, col=None):
    big = opening > 0.02 or length > 0.4                     # DOUBLE REED: the blades out, inked and edged in olive
    for poly, sh in reed_polys(cam, p, aim, y, R, opening, length, 0.07 + 0.07 * min(1.0, opening * 2)):
        if big:
            img = blend(img, _mask([], [list(poly) + [poly[0]]], 3), INK, 0.85)
        mk = _mask([poly])
        img = blend(img, mk, CANE * sh, 1.0)
        edge = _mask([], [list(poly) + [poly[0]]], 1)
        img = blend(img, edge, (np.asarray(col) * 0.8 if (big and col is not None) else CANE_D * 0.55), 0.9)
    # the green thread where it meets the ball
    q = np.asarray(p) + np.array([math.cos(aim), math.sin(aim)]) * (R * 0.98)
    s_ = cam.wp(S.to_world(q[0], q[1], y)[None])[0]
    xi, yi = int(round(s_[0])), int(round(s_[1]))
    img[max(0, yi - 1):yi + 2, max(0, xi - 1):xi + 2] = THREAD
    if glow > 0.02 and col is not None:                    # the A gathering at the tip before it sounds
        tip = np.asarray(p) + np.array([math.cos(aim), math.sin(aim)]) * (R + length)
        s2 = cam.wp(S.to_world(tip[0], tip[1], y)[None])[0]
        yy, xx = np.mgrid[0:H, 0:W]
        rr = np.hypot(xx - s2[0], yy - s2[1])
        img = add(img, np.exp(-(rr / (3 + 4 * glow)) ** 2).astype(np.float32), np.asarray(col) * 0.6 + 80, 0.55 * glow)
    return img


def draw_beam(img, cam, o, aim, length, width, k, t, col, y, held=False):
    """the tuning A: a narrow beam with the A's sine running along it (scrolling outward), a warm white core"""
    if k <= 0.01:
        return img
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    length = min(length, ring_exit(o, d))
    s = np.linspace(0, length, 48)
    P = np.column_stack([o[0] + d[0] * s, o[1] + d[1] * s, np.full(len(s), y)])
    glow = ribbon(cam, P, 0.11 + 0.04 * held)
    img = blend(img, _mask([glow]), np.asarray(col) * 0.7 + 70, 0.30 * k)
    amp = width * 0.75 + 0.02
    wave = o[None] + d[None] * s[:, None] + nrm[None] * (amp * np.sin(2 * np.pi * (s / 0.42 - t * 7.0)))[:, None]
    Wv = np.column_stack([wave, np.full(len(s), y)])
    sp = cam.wp(S.to_world(Wv[:, 0], Wv[:, 1], Wv[:, 2]))
    img = blend(img, _mask([], [sp], 2), np.clip(np.asarray(col) * 0.6 + 110, 0, 255), 0.85 * k)
    core = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))
    img = blend(img, _mask([], [core], 1), WHITE, 0.95 * k)
    return img


def ring_exit(o, d, A=None):
    """distance from o along d to the ropes (the ring is |x|, |z| <= A): beams stop there"""
    A = S.A if A is None else A
    out = 99.0
    for i in (0, 1):
        if abs(d[i]) > 1e-9:
            out = min(out, (math.copysign(A, d[i]) - o[i]) / d[i])
    return max(0.0, out)


def beam_lane(sxp, szp, o, aim, length, width, ys, xs):
    """(N,) light on the canvas under a live beam: a thin lit lane (dithered)"""
    d = np.array([math.cos(aim), math.sin(aim)])
    length = min(length, ring_exit(o, d))
    rx, rz = sxp - o[0], szp - o[1]
    along = rx * d[0] + rz * d[1]
    across = np.abs(-rx * d[1] + rz * d[0])
    lane = np.clip(1 - across / (0.16 + width), 0, 1) * (along > 0) * (along < length)
    return np.floor(lane * 3 + BAYER_FULL[ys, xs] * 0.9) / 3


# ---------------------------------------------------------------- OUT OF TUNE, SQUEAK, stun
def wobble_rim(img, x, y, rp, depth, t, col, seed=0):
    """OUT OF TUNE: a doubled, warped outline around the ball (a sour ghost a few px off, its rim waving), stronger
    with the detune depth"""
    if depth <= 0.02:
        return img
    yy, xx = np.mgrid[0:H, 0:W]
    pad = int(rp + 8)
    x0, x1 = max(0, int(x) - pad), min(W, int(x) + pad + 1)
    y0, y1 = max(0, int(y) - pad), min(H, int(y) + pad + 1)
    if x1 <= x0 or y1 <= y0:
        return img
    sub = img[y0:y1, x0:x1]
    X, Y = xx[y0:y1, x0:x1], yy[y0:y1, x0:x1]
    for j, (ox, oy, a_) in enumerate(((2.2, -1.2, 0.85), (-1.8, 1.4, 0.55))):
        sh = depth * (1.0 + 0.6 * math.sin(t * 9.0 + j * 2.0 + seed))
        cx, cy = x + ox * sh, y + oy * sh
        ang = np.arctan2(Y - cy, X - cx)
        rr = np.hypot(X - cx, Y - cy)
        wav = rp * (1.0 + 0.10 * depth * np.sin(5 * ang + t * 13.0 + j * 1.7))
        ring = np.abs(rr - wav) < 0.9
        c = np.asarray(col) * 0.55 + np.array([120, 130, 40]) * 0.45
        sub[ring] = sub[ring] * (1 - a_ * min(1.0, depth * 1.4)) + c * a_ * min(1.0, depth * 1.4)
    return img


def stun_stars(img, x, y, rp, t, k, col):
    """stunned: three little inked stars circling above the ball"""
    if k <= 0.02:
        return img
    c = np.clip(np.asarray(col) * 0.5 + 140, 0, 255)
    for j in range(3):
        a = t * 7.0 + j * 2 * math.pi / 3
        sx, sy = x + rp * 0.95 * math.cos(a), y - rp * 1.1 + rp * 0.28 * math.sin(a)
        xi, yi = int(round(sx)), int(round(sy))
        for dx, dy, cc, w_ in ((0, -2, INK, .8), (0, 2, INK, .8), (-2, 0, INK, .8), (2, 0, INK, .8), (1, 1, INK, .5),
                               (-1, 1, INK, .5), (1, -1, INK, .5), (-1, -1, INK, .5), (0, -1, c, 1.), (0, 1, c, 1.),
                               (-1, 0, c, 1.), (1, 0, c, 1.), (0, 0, WHITE, 1.)):
            if 0 <= xi + dx < W and 0 <= yi + dy < H:
                img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - k * w_) + cc * k * w_
    return img


def draw_squeak(img, cam, cx, cz, r, rmax, t, col, y=0.25):
    """SQUEAK: the shrill burst as a jagged, inked white ring expanding from the clarinet (a squeal drawn as a sawtooth)"""
    k = 1 - r / rmax
    if k <= 0.02:
        return img
    a = np.linspace(0, 2 * math.pi, 73)
    jag = r + 0.12 * np.where(np.arange(len(a)) % 2 == 0, 1.0, -0.6)
    P = cam.wp(S.to_world(cx + jag * np.cos(a), cz + jag * np.sin(a), np.full(len(a), y)))
    img = blend(img, _mask([], [P], 3), INK, 0.6 * k)
    img = blend(img, _mask([], [P], 1), WHITE * 0.7 + np.asarray(col) * 0.3, 1.0 * k)
    return img


def squeak_light(sxp, szp, cx, cz, r, rmax, t):
    """SQUEAK's shrill ring on the canvas: a jagged, high-frequency band (a squeal drawn as a sawtooth front)"""
    rd = np.hypot(sxp - cx, szp - cz)
    ang = np.arctan2(szp - cz, sxp - cx)
    jag = 0.07 * np.abs(((ang * 18 / math.pi + t * 30) % 2) - 1)
    band = np.clip(1 - np.abs(rd - r - jag) / 0.08, 0, 1)
    return band * (1 - r / rmax)
