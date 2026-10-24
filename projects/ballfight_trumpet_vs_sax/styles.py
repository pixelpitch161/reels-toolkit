"""#29 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    'bell'        the trumpet's bell poking out of its glass ball along the aim (glows while it charges)
    style 'beam'  the charge's aiming sight (a flickering dashed line) and the blast itself (gold body, white core)
    style 'smoke' a smoke ring: a wobbly torus standing across its flight, with a fading puff trail
    'note'        a BEBOP RUN projectile: an eighth note (head, stem, flag)
    'smoke_screen' the SMOKE SCREEN shield: billowing puffs orbiting the sax (back half / front half for painter order)
    'honk_bell'   HONK: a giant sax bell hovering mouth-down over the predicted spot, then the blast

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL
BRASS = np.array([232, 184, 72])
BRASS_HI = np.array([255, 236, 160])
BRASS_DK = np.array([150, 104, 34])
PEARL = np.array([238, 236, 228])


def mask_of(polys, width=None):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if width:
            d.line([tuple(q) for q in pts], fill=255, width=width)
        elif len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def blend(arr, m, col, a):
    arr += (np.asarray(col, float)[None, None] - arr) * (m * a)[..., None]


# ---------------------------------------------------------------- the trumpet's bell (on the ball, along the aim)
def bell_quads(p, th, y=R):
    """a flared bell from just inside the glass to a little beyond it: (tube quad, flare quad, mouth centre, mouth half-width)"""
    d = np.array([math.cos(th), math.sin(th)])
    n = np.array([-d[1], d[0]])
    r0, r1, r2 = R * 0.78, R + 0.06, R + 0.24
    tube = [p + d * r0 - n * 0.045, p + d * r1 - n * 0.05, p + d * r1 + n * 0.05, p + d * r0 + n * 0.045]
    flare = [p + d * r1 - n * 0.05, p + d * r2 - n * 0.17, p + d * r2 + n * 0.17, p + d * r1 + n * 0.05]
    W_ = lambda Q: np.array([S.to_world(q[0], q[1], y) for q in Q])
    return W_(tube), W_(flare), S.to_world(*(p + d * r2), y), 0.17


def draw_bell(fr, cam, L, shade, p, th, charge=0.0, y=R):
    tube, flare, mouth, hw = bell_quads(p, th, y)
    up = np.array([[0, 1.0, 0]])
    fr.poly(cam, tube, shade(L, up, tube.mean(0)[None], BRASS_DK, rimk=0.4)[0])
    fr.poly(cam, flare, np.clip(shade(L, up, flare.mean(0)[None], BRASS, rimk=0.5)[0] + 60 * charge, 0, 255))
    s = cam.wp(flare)
    fr.draw.line([tuple(s[1]), tuple(s[2])], fill=tuple(int(v) for v in np.clip(BRASS_HI + 40 * charge, 0, 255)), width=1)
    return cam.wp(mouth[None])[0]


# ---------------------------------------------------------------- the beam: charge sight + blast
def beam_quad(o, a, length, width, y=R, frac=1.0):
    d = np.array([math.cos(a), math.sin(a)])
    n = np.array([-d[1], d[0]])
    e = o + d * length * frac
    Q = [o - n * width / 2, e - n * width / 2 * 0.85, e + n * width / 2 * 0.85, o + n * width / 2]
    return np.array([S.to_world(q[0], q[1], y) for q in Q])


def draw_sight(arr, cam, p, a, length, k, t, col):
    """the locked aim while the bell charges: a dashed line marching outward, brighter as the charge fills"""
    d = np.array([math.cos(a), math.sin(a)])
    o = p + d * (R + 0.26)
    segs = []
    n = max(2, int(length / 0.22))
    off = (t * 3.0) % 1.0
    for j in range(n):
        u0 = (j + off) / n
        u1 = (j + off + 0.45) / n
        if u1 > 1:
            continue
        a0 = o + d * length * u0
        a1 = o + d * length * u1
        segs.append(cam.wp(np.array([S.to_world(a0[0], a0[1], R), S.to_world(a1[0], a1[1], R)])))
    m = mask_of(segs, width=1)
    blend(arr, m, np.clip(np.asarray(col) * 0.7 + 90, 0, 255), 0.25 + 0.6 * k)


def beam_state(t, t_fire, hold=0.12):
    """(extension 0..1, alpha) of a blast fired at t_fire: it shoots out in 40 ms, holds, then fades over 0.18 s"""
    u = t - t_fire
    if u < 0 or u > hold + 0.18:
        return None
    ext = min(1.0, u / 0.04)
    a = 1.0 if u <= hold else 1 - (u - hold) / 0.18
    return ext, a


def draw_beam(arr, cam, o, a, length, width, ext, alpha, col, y=R):
    """the blast: a soft gold halo, the body in the fighter's colour, a white-hot core"""
    for wk, c, ak in ((2.6, np.asarray(col) * 0.75 + 50, 0.35), (1.0, np.asarray(col) * 0.9 + 40, 0.9),
                      (0.38, np.array([255, 252, 236]), 1.0)):
        Q = beam_quad(o, a, length, max(0.03, width * wk), y, ext)
        m = mask_of([cam.wp(Q)])
        blend(arr, m, np.clip(c, 0, 255), ak * alpha)


# ---------------------------------------------------------------- smoke rings (the sax's volley)
SMOKE = np.array([196, 214, 210])


def ring_points(x, z, ang, r, y=0.5, n=18, tilt=0.0):
    """a vertical ring standing across its flight direction (its axis along the flight), optionally tilted"""
    d = np.array([math.cos(ang), math.sin(ang)])
    side = np.array([-d[1], d[0]])
    u = np.linspace(0, 2 * math.pi, n + 1)
    hx = np.cos(u)[:, None] * side[None] + (np.sin(u) * tilt)[:, None] * d[None]
    hy = np.sin(u)
    return S.to_world(x + r * hx[:, 0], z + r * hx[:, 1], y + r * hy)


def draw_smoke_ring(arr, cam, x, z, ang, kid, t, col, r=0.30, y=0.5, alpha=1.0):
    """a smoke ring, drawn as a donut turned three-quarters to the camera (an edge-on ring would vanish): a soft smoky
    body, a darker outer roll, a bright inner lip, squashed along its flight, wobbling, with a fading puff trail"""
    wob = 1 + 0.12 * math.sin(2 * math.pi * 2.6 * t + kid * 1.7)
    tint = SMOKE * 0.55 + np.asarray(col) * 0.45
    d = np.array([math.cos(ang), math.sin(ang)])
    for back, rr, a in ((0.30, 0.12, 0.26), (0.55, 0.08, 0.14)):          # the trail: two fading puffs behind
        c = np.array([x, z]) - d * back
        draw_puff(arr, cam, c[0], c[1], y, rr, tint, a * alpha)
    c3 = S.to_world(x, z, y)
    s0 = cam.wp(c3[None])[0]
    k = cam.f / cam.depth(c3[None])[0]
    s1 = cam.wp(S.to_world(x + d[0], z + d[1], y)[None])[0]
    dv = s1 - s0
    dv = dv / (np.hypot(*dv) + 1e-9)
    pv = np.array([-dv[1], dv[0]])
    rp = r * k * wob
    sq = 0.62 + 0.1 * math.sin(2 * math.pi * 1.3 * t + kid)
    u = np.linspace(0, 2 * math.pi, 21)
    pts = s0[None] + rp * (np.cos(u)[:, None] * pv[None] + (np.sin(u) * sq)[:, None] * dv[None])
    inner = s0[None] + 0.55 * rp * (np.cos(u)[:, None] * pv[None] + (np.sin(u) * sq)[:, None] * dv[None])
    body = mask_of([pts]) - mask_of([inner]) * 0.7
    blend(arr, np.clip(body, 0, 1), tint * 0.9, 0.42 * alpha)
    blend(arr, mask_of([pts], width=2), tint * 0.72, 0.85 * alpha)
    lip = s0[None] + 0.62 * rp * (np.cos(u)[:, None] * pv[None] + (np.sin(u) * sq)[:, None] * dv[None]) - dv[None] * 0.6
    blend(arr, mask_of([lip], width=1), np.clip(tint * 1.25 + 30, 0, 255), 0.9 * alpha)


def draw_puff(arr, cam, x, z, y, rr, col, a):
    s = cam.wp(S.to_world(x, z, y)[None])[0]
    rp = rr * cam.f / cam.depth(S.to_world(x, z, y)[None])[0]
    pts = [(s[0] + rp * math.cos(q), s[1] + rp * 0.9 * math.sin(q)) for q in np.linspace(0, 2 * math.pi, 13)[:-1]]
    m = mask_of([pts])
    dz = np.floor(BAYER_FULL * 0.9 + a * 1.6) / 1.6 if a < 0.6 else 1.0
    blend(arr, m * dz, col, min(1.0, a))


# ---------------------------------------------------------------- BEBOP RUN: eighth notes
def draw_note(arr, cam, x, z, ang, col, y=0.5):
    s = cam.wp(S.to_world(x, z, y)[None])[0]
    k = cam.f / cam.depth(S.to_world(x, z, y)[None])[0]
    hr = 0.075 * k
    head = [(s[0] + hr * 1.25 * math.cos(q) * 0.96 - hr * 0.3 * math.sin(q), s[1] + hr * 0.85 * math.sin(q))
            for q in np.linspace(0, 2 * math.pi, 11)[:-1]]
    c = np.clip(np.asarray(col) * 0.55 + 120, 0, 255)
    blend(arr, mask_of([head]), c, 1.0)
    sx, sy = s[0] + hr * 1.1, s[1]
    top = sy - 0.30 * k
    blend(arr, mask_of([[(sx, sy), (sx, top)]], width=1), c, 1.0)
    blend(arr, mask_of([[(sx, top), (sx + 0.10 * k, top + 0.10 * k)]], width=1), c, 1.0)


# ---------------------------------------------------------------- SMOKE SCREEN (shield)
def screen_puffs(cx, cz, t, k_, kid=0, n=11):
    """[(x, z, y, radius, depth-side angle)] billowing puffs around the ball, slowly swirling"""
    out = []
    for j in range(n):
        a = 2 * math.pi * j / n + 0.9 * t + 0.3 * math.sin(1.7 * t + j)
        rr = R * (1.25 + 0.22 * math.sin(2.3 * t + 1.9 * j))
        y = R * (0.55 + 0.9 * ((j * 0.37) % 1.0)) + 0.06 * math.sin(3 * t + j)
        size = (0.20 + 0.06 * math.sin(2.9 * t + j * 2.1)) * (0.6 + 0.4 * k_)
        out.append((cx + rr * math.cos(a), cz + rr * math.sin(a), y, size, a))
    return out


def draw_smoke_screen(arr, cam, cx, cz, t, k_, half, col):
    c0 = cam.depth(S.to_world(cx, cz, R)[None])[0]
    tint = SMOKE * 0.6 + np.asarray(col) * 0.4
    for x, z, y, rr, a in screen_puffs(cx, cz, t, k_):
        front = cam.depth(S.to_world(x, z, y)[None])[0] < c0
        if front == bool(half):
            draw_puff(arr, cam, x, z, y, rr, tint * (0.85 + 0.15 * math.sin(a)), 0.78 * k_)


# ---------------------------------------------------------------- HONK (slam): the giant bell
BELL_H = 1.0                     # the honk bell's height, mouth to throat (kept low: the whole bell stays in focus)


def honk_height(t, t0, t_slam):
    """the bell's mouth height: drops in, hovers lower and lower (shaking) through the telegraph, kicks on the blast,
    then lifts a little as it fades (it never leaves the lens's sharp band)"""
    if t < t0:
        return None
    if t < t0 + 0.25:
        return 2.9 - 0.7 * ((t - t0) / 0.25) ** 0.5
    if t < t_slam:
        return 2.2 - 0.7 * (t - t0 - 0.25) / max(0.05, t_slam - t0 - 0.25) + 0.03 * math.sin(t * 70)
    if t < t_slam + 0.10:
        return 1.5 + 0.4 * (t - t_slam) / 0.10
    if t < t_slam + 0.6:
        return 1.9 + 0.5 * ((t - t_slam - 0.10) / 0.5) ** 2
    return None


def draw_honk_bell(arr, cam, cx, cz, rad, h, alpha=1.0, blast=0.0, col=(36, 200, 176)):
    """a flared bell mouth-down: discs from the wide mouth (bottom) to the narrow throat (top, nearest the camera); old
    dark brass with the owner's colour on the rim, so it reads as the sax's move, not the trumpet's"""
    col = np.asarray(col, float)
    a = np.linspace(0, 2 * math.pi, 29)[:-1]
    mouth = rad * 0.72
    levels = 7
    for j in range(levels):
        u = j / (levels - 1)
        rr = 0.16 + (mouth - 0.16) * (1 - u) ** 2.2
        yy = h + BELL_H * u
        band = 0.72 + 0.28 * math.cos(u * 3.1)
        c = (BRASS_DK * 0.8 + (BRASS - BRASS_DK) * 0.55 * band) * 0.92 + col * 0.08 * (1 - u)
        if j == 0:
            c = col * 0.55 + BRASS_HI * 0.35 + 60 * blast
        P = cam.wp(S.to_world(cx + rr * np.cos(a), cz + rr * np.sin(a), yy))
        blend(arr, mask_of([P]), np.clip(c, 0, 255), alpha)
        if j == 0:                                          # the mouth's rim in the owner's colour
            blend(arr, mask_of([np.vstack([P, P[:1]])], width=2), np.clip(col * 1.1 + 40, 0, 255), alpha)
        if j == 2:                                          # a ring of the owner's colour round the flare
            blend(arr, mask_of([np.vstack([P, P[:1]])], width=1), np.clip(col * 0.9 + 30, 0, 255), 0.8 * alpha)
    for j in range(3):                                      # pearl key buttons up the side
        u = 0.45 + 0.17 * j
        rr = 0.16 + (mouth - 0.16) * (1 - u) ** 2.2
        s = cam.wp(S.to_world(cx + rr * math.cos(2.3), cz + rr * math.sin(2.3), h + BELL_H * u)[None])[0]
        xi, yi = int(round(s[0])), int(round(s[1]))
        if 1 <= xi < W - 1 and 1 <= yi < H - 1:
            arr[yi - 1:yi + 1, xi - 1:xi + 1] = arr[yi - 1:yi + 1, xi - 1:xi + 1] * (1 - alpha) + PEARL * alpha


def draw_note_rise(arr, cam, x, z, y, col, a):
    """a solo note floating up from the champion's horn (an eighth note, fading)"""
    if a <= 0:
        return
    s = cam.wp(S.to_world(x, z, y)[None])[0]
    k = cam.f / cam.depth(S.to_world(x, z, y)[None])[0]
    hr = 0.07 * k
    head = [(s[0] + hr * 1.25 * math.cos(q) - hr * 0.3 * math.sin(q), s[1] + hr * 0.85 * math.sin(q))
            for q in np.linspace(0, 2 * math.pi, 11)[:-1]]
    c = np.clip(np.asarray(col) * 0.5 + 128, 0, 255)
    blend(arr, mask_of([head]), c, a)
    sx, sy = s[0] + hr * 1.1, s[1]
    top = sy - 0.28 * k
    blend(arr, mask_of([[(sx, sy), (sx, top)], [(sx, top), (sx + 0.09 * k, top + 0.09 * k)]], width=1), c, a)
