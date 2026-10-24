"""#32 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them. #28's helpers (rings, masks, darts, the shield ring split) are
imported from projects/ballfight_drums_vs_violin/styles.py, not copied.

    style 'bell_wave'  the tuba's pulse: a slow, huge brass sound wave (gold leading edge, indigo body, two ripples)
    'pull_field'       the tuba's gravity: dashed rings sliding in toward the ball (canvas light, per pixel)
    style 'darts'      the flute's darts: tiny silver needles with a bright tip (volley + TRILL arc)
    'brass_wall'       BRASS WALL (shield): a ring of brass plates around the ball, a gleam sweeping round
    'bell_slam'        SUBSONIC (slam): a giant tuba bell hovering over the predicted spot, then slamming down mouth-first
    'piccolo'          PICCOLO (wave): a thin, shivering silver ring (a high, fine ripple) across the whole ring
    flute_sprite       the flute inside its ball: the Pixel Pitch model is a 1 px line at ball size, so the ball shows a
                       pixel-art flute instead (silver tube, lip plate, keys, ink outline), like #28's snare fallback

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import importlib.util
import math
import os
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location('styles28', os.path.join(HERE, '..', 'ballfight_drums_vs_violin', 'styles.py'))
S28 = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(S28)

R = S.R_BALL
annulus_polys, mask_of, pulse_state, cymbal_polys = S28.annulus_polys, S28.mask_of, S28.pulse_state, S28.cymbal_polys

BRASS = np.array([232, 184, 78])
BRASS_HI = np.array([255, 236, 160])
BRASS_DK = np.array([150, 102, 36])
SILVER = np.array([206, 214, 228])
SILVER_HI = np.array([250, 252, 255])
SILVER_DK = np.array([120, 128, 150])
INK = np.array([26, 20, 36])


# ---------------------------------------------------------------- the tuba's pulse: a brass sound wave
def draw_bell_wave(arr, cam, cx, cz, r, a, col, main=True, y=0.36):
    """the leading edge (brass, bright) + an indigo body band behind it + two thinner ripples trailing inside"""
    k = a * (1.0 if main else 0.6)
    w_edge = 0.08 if main else 0.05
    m_edge = mask_of(annulus_polys(cam, cx, cz, max(R * 0.9, r - w_edge), r, y, n=48))
    m_body = mask_of(annulus_polys(cam, cx, cz, max(R * 0.9, r - w_edge - 0.22), r - w_edge, y, n=48))
    dz = np.floor(BAYER_FULL * 0.9 + 0.55 * k * 2) / 2 if k < 0.6 else 1.0
    body = np.clip(np.array(col) * 0.75 + 30, 0, 255)
    arr += (body[None, None] - arr) * (m_body * 0.42 * k * dz)[..., None]
    arr += (np.clip(BRASS * 0.75 + np.array(col) * 0.35, 0, 255)[None, None] - arr) * (m_edge * 0.92 * k)[..., None]
    for j, f in enumerate((0.78, 0.6) if main else (0.75,)):
        rr = R + (r - R) * f
        if rr - 0.035 <= R:
            continue
        m = mask_of(annulus_polys(cam, cx, cz, rr - 0.035, rr, y, n=40))
        arr += (np.clip(np.array(col) + 40, 0, 255)[None, None] - arr) * (m * (0.5 - 0.15 * j) * k)[..., None]


def bell_wave_light(sxp, szp, cx, cz, r, a, main, ys, xs):
    """canvas light under a wave: a bright band at the edge, a softer glow inside (returns per-pixel intensity)"""
    rd = np.hypot(sxp - cx, szp - cz)
    band = np.clip(1 - np.abs(rd - r) / 0.26, 0, 1) * (rd < r + 0.05)
    inner = np.clip((rd - R) / max(1e-3, r - R), 0, 1) * (rd < r) * 0.25
    v = np.floor((band + inner) * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
    return v * a * (0.6 if main else 0.32)


# ---------------------------------------------------------------- the tuba's gravity (canvas decal)
def pull_light(sxp, szp, cx, cz, pull, pull_max, t, period=0.9, r_out=3.4):
    """three dashed rings sliding inward toward the tuba (a slow, steady suck), brighter as the pull grows"""
    rd = np.hypot(sxp - cx, szp - cz)
    ang = np.arctan2(szp - cz, sxp - cx)
    k = 0.14 + 0.26 * min(1.0, pull / pull_max)
    out = np.zeros_like(rd)
    for j in range(3):
        ph = ((t / period) + j / 3) % 1.0                      # 0 -> 1: the ring travels from r_out in to the ball
        rr = r_out - (r_out - R) * ph
        fade = math.sin(math.pi * ph)
        dash = (np.floor((ang + 0.6 * ph * 2 * math.pi + math.pi) * 6) % 2) == 0   # dashes swirl as they fall in
        out += np.clip(1 - np.abs(rd - rr) / 0.05, 0, 1) * dash * fade
    return out * k


# ---------------------------------------------------------------- the flute's darts
def dart_quads(x, z, ang, y=0.46):
    return S28.dart_quads(x, z, ang, y=y, L_=0.26, Wd=0.05)


def draw_dart(fr, cam, L, shade, x, z, ang, col, y=0.46):
    head, body = dart_quads(x, z, ang, y)
    fr.poly(cam, body, np.clip(SILVER_DK * 0.6 + np.array(col) * 0.3, 0, 255))
    c = shade(L, np.array([[0, 1.0, 0]]), head.mean(0)[None], SILVER * 0.7 + np.array(col) * 0.3, rimk=0.5)[0]
    fr.poly(cam, head, np.clip(c + 30, 0, 255))
    tip = cam.wp(S.to_world(x + 0.13 * math.cos(ang), z + 0.13 * math.sin(ang), y)[None])[0]
    xi, yi = int(round(tip[0])), int(round(tip[1]))
    if 0 <= xi < W and 0 <= yi < H:
        fr.draw.point([(xi, yi)], fill=(255, 255, 255))


# ---------------------------------------------------------------- BRASS WALL (shield)
def draw_brass_wall(arr, cam, cx, cz, t, k, half, y=R * 1.02):
    """brass plates around the ball (alternating tone so it reads as a wall of panels), a gleam sweeping round"""
    polys, ang = cymbal_polys(cam, cx, cz, None, y, half)
    if not polys:
        return
    sweep = t * 4.0
    for j, (pts, a) in enumerate(zip(polys, ang)):
        plate = (int((a + 4 * math.pi) / (2 * math.pi) * 10) % 2)
        g = 0.55 + 0.45 * math.cos(a - 1.0)                     # lit from the spot side
        g2 = max(0.0, math.cos(a - sweep)) ** 16
        col = BRASS_DK + (BRASS - BRASS_DK) * g * (0.86 if plate else 1.0) + BRASS_HI * 0.4 * g2
        m = mask_of([pts])
        arr += (np.clip(col, 0, 255)[None, None] - arr) * (m * 0.9 * k)[..., None]
    # rivets: a dark ring line at mid radius
    a = np.linspace(0, 2 * math.pi, 41)
    P = S.to_world(cx + R * 1.27 * np.cos(a), cz + R * 1.27 * np.sin(a), y + 0.005)
    s, dz = cam.wp(P), cam.depth(P)
    c = cam.depth(S.to_world(cx, cz, y)[None])[0]
    im = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(im)
    for j in range(40):
        if ((dz[j] + dz[j + 1]) / 2 < c) == bool(half) and j % 2 == 0:
            d.point([tuple(s[j])], fill=255)
    mk = np.asarray(im, np.float32) / 255.0
    arr += ((BRASS_DK * 0.6)[None, None] - arr) * (mk * 0.8 * k)[..., None]


# ---------------------------------------------------------------- SUBSONIC (slam): the giant bell
def bell_height(t, t0, t_slam):
    """the bell's rim height: rises into view, hovers (a slow menacing bob) during the telegraph, slams down at t_slam,
    sits for a moment, then lifts away"""
    if t < t0:
        return None
    if t < t_slam - 0.12:
        return 3.4 - 0.5 * min(1.0, (t - t0) / 0.35) + 0.08 * math.sin(2 * math.pi * (t - t0) / 0.6)
    if t < t_slam:
        u = (t - (t_slam - 0.12)) / 0.12
        return 2.9 - 2.75 * u * u
    if t < t_slam + 0.3:
        return 0.15
    if t < t_slam + 0.7:
        return 0.15 + 3.2 * ((t - t_slam - 0.3) / 0.4) ** 2
    return None


def draw_bell(arr, cam, cx, cz, rad, h, col, alpha=1.0):
    """a flared tuba bell seen from above, mouth down: the brass flare (outer rim -> throat) and the tube going up and
    back; the dark mouth only shows at the rim"""
    a = np.linspace(0, 2 * math.pi, 33)[:-1]
    rim_r = rad * 0.8
    rings = [(h, rim_r), (h + 0.35, rim_r * 0.62), (h + 0.8, rim_r * 0.38), (h + 1.5, rim_r * 0.24)]
    for j, (yy, rr) in enumerate(rings):                       # flare: widest at the rim, stacked toward the throat
        P = S.to_world(cx + rr * np.cos(a), cz + rr * np.sin(a), yy)
        m = mask_of([cam.wp(P)])
        g = 0.62 + 0.14 * j
        tint = BRASS * g + np.array(col) * 0.12
        arr += (np.clip(tint, 0, 255)[None, None] - arr) * (m * alpha)[..., None]
    # the rim: a bright lip
    P = S.to_world(cx + rim_r * np.cos(a), cz + rim_r * np.sin(a), h)
    s = cam.wp(P)
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(q) for q in np.vstack([s, s[:1]])], fill=255, width=1)
    mk = np.asarray(im, np.float32) / 255.0
    arr += (BRASS_HI[None, None] - arr) * (mk * alpha)[..., None]
    # the tube up and back (toward the valves, off-frame)
    top = S.to_world(cx - 0.5, cz + 0.5, h + 3.0)
    thr = S.to_world(cx, cz, h + 1.5)
    s0, s1 = cam.wp(np.array([thr, top]))
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(s0), tuple(s1)], fill=255, width=4)
    mk = np.asarray(im, np.float32) / 255.0
    arr += ((BRASS * 0.8)[None, None] - arr) * (mk * alpha)[..., None]
    # a highlight down the flare on the spot side
    P = S.to_world(cx + np.array([rim_r * 0.8, rim_r * 0.45, rim_r * 0.2]) * math.cos(-2.2),
                   cz + np.array([rim_r * 0.8, rim_r * 0.45, rim_r * 0.2]) * math.sin(-2.2),
                   np.array([h + 0.05, h + 0.5, h + 1.0]))
    s = cam.wp(P)
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(q) for q in s], fill=255, width=1)
    mk = np.asarray(im, np.float32) / 255.0
    arr += (BRASS_HI[None, None] - arr) * (mk * 0.8 * alpha)[..., None]


# ---------------------------------------------------------------- PICCOLO (wave): canvas ring shape
def piccolo_rd(rd, ang, t):
    """the piccolo's shiver: a fine, fast ripple on the ring's radius (a high note), not #28's jagged screech"""
    return rd + 0.045 * np.sin(31 * ang + 90 * t) + 0.02 * np.sin(57 * ang - 140 * t)


# ---------------------------------------------------------------- the flute inside its ball
def flute_sprite(r, yaw, roll):
    """RGBA pixel-art flute fitting a radius-r sprite: a silver tube (banded: highlight / body / shadow), the crown, the
    lip plate with its dark embouchure hole, a row of key cups, an ink outline (the Pixel Pitch look). `yaw` swings the
    flute a little (foreshortening), `roll` is its tilt in the ball (deg)."""
    n = 2 * int(r) + 5
    c = (n - 1) / 2
    L = 1.95 * r * (0.88 + 0.12 * math.cos(math.radians(yaw) * 1.6))
    T = max(3.0, round(r * 0.2))
    a = math.radians(-roll)
    d = np.array([math.cos(a), math.sin(a)])
    nr = np.array([-d[1], d[0]])
    yy, xx = np.mgrid[0:n, 0:n].astype(float)
    px, py = xx - c, yy - c
    u = px * d[0] + py * d[1]
    v = px * nr[0] + py * nr[1]
    body = (np.abs(u) <= L / 2) & (np.abs(v) <= T / 2)
    crown = (u >= -L / 2 - 1.2) & (u < -L / 2 + 1.5) & (np.abs(v) <= T / 2 + 0.7)
    keys = np.zeros_like(body)
    key_c = []
    for j in range(7):
        uk = -L / 2 + L * (0.42 + 0.075 * j)
        vk = -T * 0.25
        key_c.append((uk, vk))
        keys |= (u - uk) ** 2 + (v - vk) ** 2 <= (T * 0.52) ** 2
    lip_u = -L / 2 + L * 0.13
    lip = ((u - lip_u) / 2.0) ** 2 + ((v + T * 0.3) / 1.4) ** 2 <= 1.0
    shape = body | crown | keys | lip
    img = np.zeros((n, n, 4), np.uint8)
    # banded silver by v (light from the top of the ball)
    band = np.where(v < -T * 0.22, 2, np.where(v < T * 0.2, 1, 0))
    tone = np.stack([SILVER_DK, SILVER, SILVER_HI])[band]
    img[..., :3] = tone.astype(np.uint8)
    img[keys] = np.append(SILVER_HI * 0.96, 255).astype(np.uint8)
    img[lip] = np.append(SILVER_HI, 255).astype(np.uint8)
    img[crown] = np.append(SILVER * 0.9, 255).astype(np.uint8)
    for uk, vk in key_c:                                       # a dark pad ring centre in each key cup
        m = (u - uk) ** 2 + (v - vk) ** 2 <= (T * 0.2) ** 2
        img[m & keys, :3] = (SILVER_DK * 0.8).astype(np.uint8)
    hole = ((u - lip_u) / 0.9) ** 2 + ((v + T * 0.3) / 0.7) ** 2 <= 1.0
    img[hole, :3] = INK
    img[..., 3] = np.where(shape, 255, 0)
    # ink outline: one pixel around the shape
    o = np.zeros_like(shape)
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            o |= np.roll(np.roll(shape, dy, 0), dx, 1)
    edge = o & ~shape
    img[edge, :3] = INK
    img[edge, 3] = 255
    return img


# =================================================================================================== v2 (BALLFIGHT v2)
# OOM-PAH SWALLOW (tuba) and SNAKE CHARMER (flute): the instrument's body / the way it's played is the weapon.
WICKER = np.array([196, 150, 92])
WICKER_DK = np.array([120, 84, 46])
WICKER_HI = np.array([232, 196, 132])
SCALE_DK = np.array([96, 104, 128])
HOOD_MARK = np.array([40, 34, 62])


# ---------------------------------------------------------------- the tuba's bell (OOM-PAH SWALLOW)
def bell_rings(p, th, open_k=0.0, flare=0.0, y=R * 1.05):
    """the bell growing out of the tuba ball toward its aim: [(centre world, radius)] from the throat to the mouth, plus
    the mouth's axis. open_k (0..1) widens the mouth while it inhales, flare (0..1) is the "pah" punch"""
    d = np.array([math.cos(th), math.sin(th)])
    L = 0.50 + 0.12 * open_k + 0.18 * flare
    rm = 0.36 + 0.22 * open_k + 0.16 * flare
    out = []
    for u, rr in ((0.0, 0.13), (0.45, 0.15), (0.75, 0.2), (0.92, rm * 0.78), (1.0, rm)):
        c = p + d * (R * 0.85 + L * u)
        out.append((S.to_world(c[0], c[1], y), rr))
    return out, S.to_world(d[0], d[1], 0.0) - S.to_world(0.0, 0.0, 0.0)


def _ring_pts(c, axis, r, n=14):
    """a vertical circle of radius r around world centre c, facing world direction `axis` (horizontal)"""
    ax = axis / (np.linalg.norm(axis) + 1e-9)
    up = np.array([0.0, 1.0, 0.0])
    side = np.cross(up, ax)
    side /= np.linalg.norm(side) + 1e-9
    a = np.linspace(0, 2 * math.pi, n + 1)[:-1]
    return c[None] + r * (np.cos(a)[:, None] * side[None] + np.sin(a)[:, None] * up[None])


def draw_tuba_bell(fr, cam, L, shade, p, th, open_k, flare, col, glow=0.0):
    """brass flare (banded segments throat -> mouth, lit from the spot side), a bright lip and, when the mouth faces
    the camera, the dark throat inside (indigo-tinted while it inhales)"""
    rings, axis = bell_rings(p, th, open_k, flare)
    pts = [_ring_pts(c, axis, r) for c, r in rings]
    n = len(pts[0])
    for j in range(len(pts) - 1):
        A_, B_ = pts[j], pts[j + 1]
        for i in range(n):
            q = np.array([A_[i], A_[(i + 1) % n], B_[(i + 1) % n], B_[i]])
            nrm = np.cross(q[1] - q[0], q[3] - q[0])
            nrm = nrm / (np.linalg.norm(nrm) + 1e-9)
            if np.dot(nrm, cam.pos - q.mean(0)) < 0:
                nrm = -nrm
            base = BRASS * (0.9 + 0.1 * j / 3) + np.array(col) * 0.06
            c = shade(L, nrm[None], q.mean(0)[None], base, rimk=0.6)[0]
            fr.poly(cam, q, np.clip(c * (1 + 0.5 * glow) + 30 * glow, 0, 255))
    mouth_c, rm = rings[-1]
    facing = np.dot(axis, cam.pos - mouth_c) > 0
    lip = _ring_pts(mouth_c, axis, rm, n=20)
    if facing:
        inner = _ring_pts(mouth_c - axis * 0.02, axis, rm * 0.82, n=20)
        ink = np.array(col) * (0.18 + 0.35 * open_k) + np.array([10, 6, 16])
        fr.poly(cam, inner, np.clip(ink, 0, 255))
    fr.line(cam, np.vstack([lip, lip[:1]]), np.clip(BRASS_HI * (1 + 0.3 * glow), 0, 255), 1)
    return mouth_c


def inhale_light(sxp, szp, p, th, reach, k, t, ys, xs, half=0.8):
    """the suction cone on the canvas: a wedge in front of the bell (half-angle `half` rad, out to the reach) with
    streaks of air racing IN toward the mouth. Returns (bright, dark) per pixel: thin bright streaks over darker gaps"""
    rel_x, rel_z = sxp - p[0], szp - p[1]
    rd = np.hypot(rel_x, rel_z)
    ang = np.arctan2(rel_z, rel_x)
    da = (ang - th + math.pi) % (2 * math.pi) - math.pi
    rmax = R * 2 + reach
    inside = (np.abs(da) < half) & (rd > R * 0.95) & (rd < rmax)
    fall = np.clip(1 - np.abs(da) / half, 0, 1) ** 0.7 * np.clip(1 - (rd - R) / (rmax - R), 0, 1) ** 0.5
    lane = np.sin(da * 9.0) > 0.55                                   # radial lanes
    flow = (rd * 3.2 + t * 9.0 + np.floor(da * 9.0 / math.pi) * 0.37) % 1.0   # dashes travelling inward
    bright = (lane & (flow < 0.28)) * fall * inside
    dark = fall * inside * 0.5
    q = lambda v: np.floor(v * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
    return q(bright) * k, q(dark) * k


# ---------------------------------------------------------------- BRASS WALL: the bell becomes a brass dome
def draw_dome(arr, cam, cx, cz, k, t, col):
    """a brass dome over the tuba ball: banded hemisphere (seen from the 3/4 camera), a gleam sweeping round, a rim
    ring on the canvas; translucent at the edges so the tuba still reads inside"""
    rr = R * 1.45
    a = np.linspace(0, 2 * math.pi, 41)
    for j, (h, f) in enumerate(((0.0, 1.0), (rr * 0.45, 0.9), (rr * 0.75, 0.68), (rr * 0.93, 0.38))):
        P = S.to_world(cx + rr * f * np.cos(a), cz + rr * f * np.sin(a), h)
        m = mask_of([cam.wp(P)])
        g = 0.62 + 0.12 * j
        col_j = BRASS * g + np.array(col) * 0.10
        arr += (np.clip(col_j, 0, 255)[None, None] - arr) * (m * (0.30 + 0.12 * j) * k)[..., None]
    sweep = t * 3.0
    P = S.to_world(cx + rr * 0.7 * math.cos(sweep), cz + rr * 0.7 * math.sin(sweep), rr * 0.55)
    s = cam.wp(P[None])[0]
    yy, xx = int(round(s[1])), int(round(s[0]))
    if 1 <= yy < H - 1 and 1 <= xx < W - 1:
        arr[yy - 1:yy + 2, xx - 1:xx + 2] = arr[yy - 1:yy + 2, xx - 1:xx + 2] * (1 - k) + BRASS_HI * k
    P = S.to_world(cx + rr * np.cos(a), cz + rr * np.sin(a), 0.02)
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(q) for q in cam.wp(P)], fill=255, width=1)
    mk = np.asarray(im, np.float32) / 255.0
    arr += (BRASS_HI[None, None] - arr) * (mk * 0.8 * k)[..., None]


# ---------------------------------------------------------------- SUBSONIC: the floor ripples like water
def ripple_light(sxp, szp, cx, cz, t, t0, t_slam, rad, ys, xs):
    """telegraph: rings sucked IN toward the predicted spot (a sub note building); the hit: water ripples racing OUT
    across the canvas, fading over ~1.6 s. Returns (light add, albedo darken) per pixel"""
    rd = np.hypot(sxp - cx, szp - cz)
    add = np.zeros_like(rd)
    dark = np.zeros_like(rd)
    if t < t_slam:
        u = (t - t0) / max(1e-6, t_slam - t0)
        for j in range(3):
            ph = (u * 3 + j / 3) % 1.0
            r_ = rad * 1.6 * (1 - ph)
            add += np.clip(1 - np.abs(rd - r_) / 0.05, 0, 1) * math.sin(math.pi * ph) * 0.5
        dark += (rd < rad) * 0.45 * u
    else:
        u = t - t_slam
        if u < 1.6:
            for j in range(4):
                r_ = 0.2 + 3.2 * max(0.0, u - j * 0.12)
                if r_ <= 0.2:
                    continue
                band = np.cos(np.clip((rd - r_) / 0.16, -1.5, 1.5) * math.pi / 3) * (np.abs(rd - r_) < 0.24)
                add += band * (1 - u / 1.6) * (0.75 - 0.12 * j)
                dark += np.clip(1 - np.abs(rd - (r_ - 0.12)) / 0.08, 0, 1) * (1 - u / 1.6) * 0.35
    q = lambda v: np.floor(v * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
    return q(np.clip(add, 0, 1.2)), q(np.clip(dark, 0, 0.8))


# ---------------------------------------------------------------- SNAKE CHARMER: baskets + silver cobras
BASKET_R, BASKET_H = 0.34, 0.30


def draw_basket(fr, cam, L, shade, cx, cz, k=1.0, lid=0.0):
    """a woven basket: banded wicker sides (alternating weave rows), a darker rim, the dark opening on top; k (0..1)
    scales it in / out (drop-in pop, pick-up)"""
    if k <= 0.02:
        return
    r = BASKET_R * k
    h = BASKET_H * k
    n = 16
    a = np.linspace(0, 2 * math.pi, n + 1)
    rows = 3
    for j in range(rows):
        y0, y1 = h * j / rows, h * (j + 1) / rows
        r0, r1 = r * (0.86 + 0.14 * j / rows), r * (0.86 + 0.14 * (j + 1) / rows)
        for i in range(n):
            q = np.array([S.to_world(cx + r0 * math.cos(a[i]), cz + r0 * math.sin(a[i]), y0),
                          S.to_world(cx + r0 * math.cos(a[i + 1]), cz + r0 * math.sin(a[i + 1]), y0),
                          S.to_world(cx + r1 * math.cos(a[i + 1]), cz + r1 * math.sin(a[i + 1]), y1),
                          S.to_world(cx + r1 * math.cos(a[i]), cz + r1 * math.sin(a[i]), y1)])
            nrm = S.to_world(math.cos((a[i] + a[i + 1]) / 2), math.sin((a[i] + a[i + 1]) / 2), 0.0) - S.to_world(0, 0, 0)
            if np.dot(nrm, cam.pos - q.mean(0)) <= 0:
                continue
            base = WICKER if (i + j) % 2 == 0 else WICKER_DK * 1.15
            c = shade(L, nrm[None], q.mean(0)[None], base, rimk=0.3)[0]
            fr.poly(cam, q, c)
    top = S.to_world(cx + r * np.cos(a[:-1]), cz + r * np.sin(a[:-1]), h)
    fr.poly(cam, top, WICKER_HI * 0.9)
    inner = S.to_world(cx + r * 0.78 * np.cos(a[:-1]), cz + r * 0.78 * np.sin(a[:-1]), h + 0.002)
    fr.poly(cam, inner, np.array([34, 24, 18]))


def cobra_pose(home, cp, t, t_drop, rise_t, aim, strike, dancing_k, beat, seed=0):
    """the cobra's body as world points (tail -> head) + the head point and its facing (ring-frame unit vector).
    It rises out of the basket over `rise_t`, sways (a slow figure-eight, 2 beats), and lunges on a strike
    (strike = (t0, t_hit, t_end, aim, len)); away from the basket (SNAKE DANCE) its body trails on the canvas"""
    u_rise = min(1.0, max(0.0, (t - t_drop) / max(1e-6, rise_t)))
    e = u_rise * u_rise * (3 - 2 * u_rise)
    d_aim = np.array([math.cos(aim), math.sin(aim)])
    side = np.array([-d_aim[1], d_aim[0]])
    ph = 2 * math.pi * (t / (2 * beat)) + seed * 1.7
    sway = 0.15 * math.sin(ph) * e
    bob = 0.05 * math.sin(2 * ph) * e
    away = float(np.hypot(*(cp - home))) > 0.05
    base = np.array([cp[0], cp[1]])
    y_base = BASKET_H * 0.9 if not away else 0.04
    head_rest = base + side * sway + d_aim * 0.16 * e
    hy = y_base + (0.25 + 0.85 * e) * (1 - 0.25 * dancing_k) + bob
    head = np.array([head_rest[0], head_rest[1], hy])
    if strike is not None:
        t0, t_hit, t_end, s_aim, s_len = strike
        d_s = np.array([math.cos(s_aim), math.sin(s_aim)])
        if t < t_hit:
            k = (t - t0) / max(1e-6, t_hit - t0)
            k = 1 - (1 - k) ** 2
        else:
            k = max(0.0, 1 - (t - t_hit) / max(1e-6, t_end - t_hit))
        tgt = np.array([base[0] + d_s[0] * s_len, base[1] + d_s[1] * s_len, R * 0.85])
        head = head + (tgt - head) * k
        d_aim = d_s if k > 0.2 else d_aim
    # body: from the basket mouth (or the canvas trail) up to the head, an S-curve
    pts = []
    if away:                                                    # slithering: a wavy trail behind on the canvas
        back = home - cp
        bl = float(np.hypot(*back))
        b_u = back / bl if bl > 1e-6 else -d_aim
        b_n = np.array([-b_u[1], b_u[0]])
        for j in range(7, 0, -1):
            s_ = j / 7 * min(0.9, bl + 0.2)
            q = cp + b_u * s_ + b_n * 0.08 * math.sin(6 * s_ - 9 * t)
            pts.append(S.to_world(q[0], q[1], 0.04))
    n = 9
    for j in range(n + 1):
        s_ = j / n
        q = base + (head[:2] - base) * s_ + side * 0.07 * math.sin(math.pi * s_ * 1.5 + ph) * e
        yq = y_base + (head[2] - y_base) * (s_ ** 0.8)
        pts.append(S.to_world(q[0], q[1], yq))
    return np.array(pts), S.to_world(head[0], head[1], head[2]), d_aim, e


def draw_cobra(fr, cam, L, shade, pts, head_w, face, rise_e, col, glow=0.0, wpx=None):
    """a silver cobra with an ink outline (the Pixel Pitch look): a thick scaled body (belly segments darker), the flared
    hood (with the dark spectacle mark) facing its target, a wedge head and two eye pixels (red while it strikes)"""
    if rise_e <= 0.02 or len(pts) < 2:
        return
    depth = cam.depth(pts)
    f = cam.f / max(1e-3, float(depth.mean()))
    w_body = max(3, int(round(0.16 * f * (0.5 + 0.5 * rise_e)))) if wpx is None else wpx
    body_col = np.clip(SILVER * 0.86 + np.array([150, 190, 255]) * 0.08 + np.array(col) * 0.06, 0, 255)
    nseg = len(pts) - 1
    widths = [max(2, int(round(w_body * (0.55 + 0.45 * j / nseg)))) for j in range(nseg)]
    for j in range(nseg):                                        # ink outline pass
        fr.line(cam, pts[j:j + 2], INK, widths[j] + 2)
    for j in range(nseg):
        q = pts[j:j + 2].mean(0)
        base = body_col if j % 2 == 0 else SCALE_DK * 1.45
        c = shade(L, np.array([[0, 1.0, 0]]), q[None], base, rimk=0.5)[0]
        fr.line(cam, pts[j:j + 2], np.clip(c * (1 + 0.4 * glow) + 25 * glow, 0, 255), widths[j])
    # hood: a flat oval behind the head, perpendicular to the facing direction
    fa = S.to_world(face[0], face[1], 0.0) - S.to_world(0, 0, 0)
    fa = fa / (np.linalg.norm(fa) + 1e-9)
    up = np.array([0.0, 1.0, 0.0])
    to_cam = cam.pos - head_w
    to_cam[1] = 0.0
    to_cam /= np.linalg.norm(to_cam) + 1e-9
    hn = 0.45 * fa - 0.55 * to_cam                               # the hood turns partly to camera so it always reads
    hn /= np.linalg.norm(hn) + 1e-9
    side = np.cross(up, hn)
    side /= np.linalg.norm(side) + 1e-9
    hc = head_w - fa * 0.05 - up * 0.09
    a = np.linspace(0, 2 * math.pi, 13)[:-1]
    hw, hh = 0.24 * rise_e, 0.19 * rise_e
    hood_o = hc[None] + (hw + 0.035) * np.cos(a)[:, None] * side[None] + (hh + 0.035) * np.sin(a)[:, None] * up[None]
    hood = hc[None] + hw * np.cos(a)[:, None] * side[None] + hh * np.sin(a)[:, None] * up[None]
    back_seen = np.dot(-fa, cam.pos - hc) > 0
    fr.poly(cam, hood_o - hn * 0.003, INK)
    c = shade(L, (-hn if np.dot(-hn, cam.pos - hc) > 0 else hn)[None], hc[None], body_col, rimk=0.6)[0]
    fr.poly(cam, hood, np.clip(c * (1 + 0.4 * glow) + 20 * glow, 0, 255))
    if back_seen:                                                 # the hood's back: the spectacle mark
        for sg in (-1, 1):
            mk = hc[None] + side[None] * 0.07 * sg * rise_e + 0.04 * rise_e * np.cos(a)[:, None] * side[None] \
                + 0.035 * rise_e * np.sin(a)[:, None] * up[None]
            fr.poly(cam, mk + hn * 0.005, HOOD_MARK)
    else:                                                         # the front: pale belly scales down the hood
        belly = hc[None] + 0.06 * rise_e * np.cos(a)[:, None] * side[None] + 0.13 * rise_e * np.sin(a)[:, None] * up[None]
        fr.poly(cam, belly + hn * 0.005, np.clip(SILVER_HI * 0.92, 0, 255))
    head = np.array([head_w + side * 0.07 - up * 0.01, head_w + fa * 0.17, head_w - side * 0.07 - up * 0.01,
                     head_w - fa * 0.03 + up * 0.04])
    fr.poly(cam, head * 1.0 + (head - head.mean(0)) * 0.25, INK)
    c = shade(L, up[None], head_w[None], SILVER_HI * 0.95, rimk=0.6)[0]
    fr.poly(cam, head, c)
    for sgn in (-1, 1):
        ep = cam.wp((head_w + fa * 0.05 + side * 0.04 * sgn + up * 0.03)[None])[0]
        xi, yi = int(round(ep[0])), int(round(ep[1]))
        if 0 <= xi < W and 0 <= yi < H:
            fr.draw.point([(xi, yi)], fill=(255, 70, 110) if glow > 0.3 else (120, 255, 220))
