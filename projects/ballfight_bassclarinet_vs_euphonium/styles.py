"""#38 weapon styles (new): the BASS CLARINET's ENDPIN ANCHOR (the floor peg driven into the canvas, the long black body
as the whirling arm, the swept-light trail, MULTIPHONICS' ghost twin arm), SLAP TONGUE's slapping reed paddle, LIGHTS
OUT's shadow pool and eruption ring; the EUPHONIUM's BELL MORTAR (brass shells on real arcs with smoke trails, the
growing shadow circles where they will land, the splash). Styles live in the episode's folder so later crossovers can
import them (series rule).

All functions draw on a float (H, W, 3) image at the internal resolution with a reelkit camera; positions are in the
ring frame (x, z) at a height y, mapped through scene.to_world. The canvas decals return per-pixel weights for the
renderer's canvas pass (ring-frame pixel coordinates sxp, szp)."""
import math
import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL
import scene as S

INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)
BLACK = np.array([30, 26, 32], float)            # grenadilla
NICKEL = np.array([214, 216, 228], float)
CANE = np.array([232, 210, 152], float)
BRASS = np.array([236, 186, 84], float)
BRASS_HI = np.array([255, 240, 190], float)
SMOKE = np.array([196, 188, 176], float)


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


def wp(cam, x, z, y):
    return cam.wp(S.to_world(np.atleast_1d(x), np.atleast_1d(z), np.atleast_1d(y)))


# ---------------------------------------------------------------- ENDPIN ANCHOR
def whirl_light(sxp, szp, c, r, th, om, R, age):
    """canvas decal: the swept trail behind the whirling arm (the circle it smashes through), brightest just behind the
    arm, fading back over ~200 deg; a bright ring at the ball's path. Returns weights 0..1."""
    dx, dz = sxp - c[0], szp - c[1]
    d = np.hypot(dx, dz)
    a = np.arctan2(dz, dx)
    sgn = 1.0 if om >= 0 else -1.0
    behind = ((th - a) * sgn) % (2 * math.pi)                  # 0 at the arm, growing backward
    trail = np.clip(1 - behind / math.radians(200), 0, 1) ** 1.6
    disc = np.clip(1 - (d - (r + R * 0.9)) / 0.12, 0, 1) * (d < r + R + 0.12)
    path = np.clip(1 - np.abs(d - r) / (R * 0.75), 0, 1)
    k = min(1.0, age / 0.12)
    return (0.45 * disc * trail + 0.95 * path * trail) * k


def peg_light(sxp, szp, c, age):
    """canvas decal: the spot where the peg went in - a dark dent with a bright ring, fresh for ~0.3 s"""
    d = np.hypot(sxp - c[0], szp - c[1])
    dent = np.clip(1 - d / 0.13, 0, 1)
    ring = np.clip(1 - np.abs(d - (0.14 + 0.9 * min(age, 0.25))) / 0.05, 0, 1) * max(0.0, 1 - age / 0.3)
    return dent, ring


def draw_arm(img, cam, c, p, R, col, ghost=False, k=1.0):
    """the endpin: a bright silver rod out of the ball's rim down to the peg driven into the canvas at c (the ball's
    long body is the hammer, the peg the pivot); a slate rim light, the black rubber collar at the canvas and a white
    glint where it bites in. ghost = MULTIPHONICS' twin (pale, translucent)."""
    d = np.asarray(c, float) - np.asarray(p, float)
    L = float(np.hypot(*d))
    u_ = d / L if L > 1e-6 else np.array([1.0, 0.0])
    a = np.asarray(p, float) + u_ * R * 0.62                           # from the glass's edge ...
    n = 7
    w = np.linspace(0, 1, n)
    xs = a[0] + (c[0] - a[0]) * w
    zs = a[1] + (c[1] - a[1]) * w
    ys = R * 0.55 * (1 - w) + 0.02
    sp = wp(cam, xs, zs, ys)
    zc = cam.depth(S.to_world(p[0], p[1], R)[None])[0]
    width = max(2, int(round(0.075 * cam.f / max(1e-6, zc))))
    body = _mask([], [sp], width=width + 2)
    core = _mask([], [sp], width=width)
    hi = _mask([], [sp + np.array([0, -1.0])], width=1)
    if ghost:
        img = blend(img, body, np.clip(col * 0.4 + 150, 0, 255), 0.45 * k)
        img = blend(img, hi, WHITE, 0.5 * k)
        return img
    img = blend(img, body, INK, 0.9 * k)                                   # dark outline
    img = blend(img, core, NICKEL, 0.95 * k)                               # the silver rod
    img = blend(img, hi, np.clip(col * 0.4 + 170, 0, 255), 0.9 * k)       # slate-lit top edge
    collar = wp(cam, [xs[-2], xs[-1]], [zs[-2], zs[-1]], [ys[-2], ys[-1]])
    img = blend(img, _mask([], [collar], width=width + 2), BLACK, 0.95 * k)
    base = wp(cam, c[0], c[1], 0.0)[0]
    xi, yi = int(base[0]), int(base[1])
    for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1)):
        if 0 <= xi + dx < W and 0 <= yi + dy < H:
            img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - k) + WHITE * k
    return img


def draw_ghost_ball(img, cam, p, R, col, k):
    """MULTIPHONICS: the twin's ball, a translucent outline on the far side of the peg"""
    s = wp(cam, p[0], p[1], R)[0]
    rp = R * cam.f / max(1e-6, cam.depth(S.to_world(p[0], p[1], R)[None])[0])
    a = np.linspace(0, 2 * math.pi, 25)
    ring = np.stack([s[0] + rp * np.cos(a), s[1] + rp * np.sin(a)], 1)
    disc = _mask([ring])
    edge = _mask([], [ring], width=1)
    img = blend(img, disc, np.clip(col * 0.35 + 150, 0, 255), 0.22 * k)
    img = blend(img, edge, np.clip(col * 0.5 + 130, 0, 255), 0.75 * k)
    return img


# ---------------------------------------------------------------- SLAP TONGUE (snip kind, drawn as a slap)
def draw_slap(img, cam, p, aim, R, ext, open_k, hit_k, col):
    """the reed as a paddle: a broad, flat cane blade on a black mouthpiece shoots out of the ball at the enemy and slaps
    (open_k: wind-up 0..1, the paddle cocked back; ext: reach beyond the surface; hit_k: the slap's flash 1..0)"""
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    base = p + d * (R * 0.85)
    tip = p + d * (R + ext)
    cock = nrm * 0.18 * open_k                                             # wound back sideways before it slaps
    mid = base + (tip - base) * 0.35 + cock * 0.5
    hw0, hw1 = 0.05, 0.16
    q = [base + nrm * hw0, mid + nrm * hw0 * 1.2, tip + cock + nrm * hw1, tip + cock - nrm * hw1, mid - nrm * hw0 * 1.2,
         base - nrm * hw0]
    sp = wp(cam, [a[0] for a in q], [a[1] for a in q], [R] * len(q))
    m = _mask([sp])
    img = blend(img, m, CANE, 0.95)
    stem = wp(cam, [base[0], mid[0]], [base[1], mid[1]], [R, R])
    img = blend(img, _mask([], [stem], width=2), BLACK, 0.95)
    edge = _mask([], [np.vstack([sp, sp[:1]])], width=1)
    img = blend(img, edge, np.clip(col * 0.6 + 60, 0, 255), 0.8)
    if hit_k > 0:
        img = blend(img, m, WHITE, 0.7 * hit_k)
    return img


# ---------------------------------------------------------------- LIGHTS OUT
def shadow_pool(sxp, szp, p, R, k):
    """canvas decal: the dark pool the bass clarinet slides in while it is sunk into shadow"""
    d = np.hypot(sxp - p[0], szp - p[1])
    return np.clip(1 - d / (R * 1.5), 0, 1) ** 0.7 * k


def erupt_light(sxp, szp, c, age, radius):
    """canvas decal: the eruption - a cracked ring racing out to the radius, a hot centre"""
    if not 0 <= age < 0.6:
        return np.zeros_like(sxp)
    d = np.hypot(sxp - c[0], szp - c[1])
    r = radius * min(1.0, age / 0.22) ** 0.6
    ring = np.clip(1 - np.abs(d - r) / 0.16, 0, 1)
    core = np.clip(1 - d / 0.6, 0, 1) * max(0.0, 1 - age / 0.25)
    return (ring + core) * (1 - age / 0.6)


# ---------------------------------------------------------------- BELL MORTAR
def shell_pos(sh, t, R):
    """a shell's ring-frame position (x, z, y) at t: a straight line over the canvas, a parabola up (apex grows with the
    range), launched from the top of the euphonium's bell"""
    owner, x0, z0, x1, z1, t0, t1 = sh[:7]
    s = min(1.0, max(0.0, (t - t0) / max(1e-6, t1 - t0)))
    x, z = x0 + (x1 - x0) * s, z0 + (z1 - z0) * s
    rng_ = math.hypot(x1 - x0, z1 - z0)
    apex = 2.2 + 0.35 * rng_
    y = R * 1.7 * (1 - s) + 0.05 * s + 4 * apex * s * (1 - s)
    return x, z, y, s


def draw_shell(img, cam, sh, t, R, col):
    """a brass shell in flight: an amber slug with a hot highlight and a short grey smoke trail"""
    x, z, y, s = shell_pos(sh, t, R)
    if s >= 1.0:
        return img
    pts = [shell_pos(sh, t - dt, R) for dt in (0.0, 0.05, 0.10, 0.15)]
    sp = wp(cam, [q[0] for q in pts], [q[1] for q in pts], [q[2] for q in pts])
    trail = _mask([], [sp], width=1)
    img = blend(img, trail, SMOKE, 0.45)
    c = sp[0]
    rp = max(1.4, 0.10 * cam.f / max(1e-6, cam.depth(S.to_world(x, z, y)[None])[0]))
    xi0, xi1 = int(c[0] - rp - 1), int(c[0] + rp + 2)
    yi0, yi1 = int(c[1] - rp - 1), int(c[1] + rp + 2)
    for yi in range(max(0, yi0), min(H, yi1)):
        for xi in range(max(0, xi0), min(W, xi1)):
            dd = ((xi + 0.5 - c[0]) / (rp * 1.25)) ** 2 + ((yi + 0.5 - c[1]) / rp) ** 2
            if dd <= 1.0:
                hi = (xi + 0.5 - c[0]) < -0.2 * rp and (yi + 0.5 - c[1]) < -0.1 * rp
                img[yi, xi] = BRASS_HI if hi else np.clip(BRASS * 0.85 + col * 0.15, 0, 255)
            elif dd <= 1.6:
                img[yi, xi] = img[yi, xi] * 0.4 + INK * 0.6
    return img


def target_light(sxp, szp, sh, t, splash):
    """canvas decals of one incoming shell: (shade, ring) - the shadow circle growing and darkening as it falls, and a
    thin ring in the owner's colour at the splash radius"""
    owner, x0, z0, x1, z1, t0, t1 = sh[:7]
    s = (t - t0) / max(1e-6, t1 - t0)
    if not 0.0 <= s < 1.0:
        return None, None
    d = np.hypot(sxp - x1, szp - z1)
    rr = 0.12 + (splash * 0.85 - 0.12) * s
    shade = np.clip(1 - d / rr, 0, 1) ** 0.6 * (0.25 + 0.6 * s)
    ring = np.clip(1 - np.abs(d - splash) / 0.035, 0, 1) * (0.3 + 0.6 * s)
    return shade, ring


def splash_light(sxp, szp, x, z, age, splash):
    """canvas decal: a landed shell's splash - a ring racing out to the splash radius, fading over 0.4 s"""
    if not 0 <= age < 0.4:
        return None
    d = np.hypot(sxp - x, szp - z)
    r = splash * min(1.0, age / 0.12) ** 0.5
    return np.clip(1 - np.abs(d - r) / 0.10, 0, 1) * (1 - age / 0.4)
