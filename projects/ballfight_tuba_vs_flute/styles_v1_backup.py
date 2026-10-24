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
