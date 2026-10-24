"""#33 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'soundhole'  the ACOUSTIC's cone: three strummed sound-wave arcs fanning out of the ball toward the enemy
    style 'uke_dash'   the UKULELE's body-slam dash: pink afterimages + speed streaks behind the ball
    'capo'             CAPO (haste): a dark capo clamped across the acoustic's ball
    'dart'             FINGERPICK: a plucked string segment with a bright head (arc projectiles; as #28's pizzicato)
    'pick'             BIG STRUM (slam): a giant tortoiseshell plectrum hovering over the predicted spot, then the
                       downstroke
    'tornado'          UKE-NADO (wave): pink whirl arcs spinning round the uke while its wave crosses the ring

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL


def mask_of(polys, width=None):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if width:
            d.line([tuple(q) for q in pts], fill=255, width=width)
        elif len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def blend(arr, m, col, k):
    arr += (np.clip(np.asarray(col, float), 0, 255)[None, None] - arr) * (m * k)[..., None]


# ---------------------------------------------------------------- the sound-hole cone (ACOUSTIC)
CREAM = np.array([255, 240, 214])


def cone_state(t, t0, length, speed, fade=0.24):
    """(front radius from the ball's centre, alpha, expanding?) of a cone fired at t0 that stops at R + length"""
    u = t - t0
    if u < 0:
        return None
    r = R + min(length, speed * u)
    t_full = length / speed
    a = 1.0 if u <= t_full else max(0.0, 1 - (u - t_full) / fade)
    if a <= 0:
        return None
    return r, a, u <= t_full


def arc_band(cam, cx, cz, aim, half, r_in, r_out, y, n=10):
    """screen polygon of an annular sector (one band of the cone)"""
    a = np.linspace(aim - half, aim + half, n)
    outer = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    inner = S.to_world(cx + r_in * np.cos(a[::-1]), cz + r_in * np.sin(a[::-1]), y)
    return cam.wp(np.vstack([outer, inner]))


def draw_cone(arr, cam, cx, cz, aim, half, r, a, col, main=True, y=0.42):
    """three sound-wave arcs: the front one bright (cream edge on the fighter's colour), two fainter ones behind it,
    and a dithered wash filling the wedge from the ball out to the front"""
    k = a * (1.0 if main else 0.7)
    col = np.asarray(col, float)
    wash = arc_band(cam, cx, cz, aim, half * 0.92, R * 0.95, max(R, r - 0.05), y, 12)
    mw = mask_of([wash])
    dz = (BAYER_FULL < 0.32).astype(np.float32)
    blend(arr, mw * dz, col * 0.6 + 90, 0.38 * k)
    for j, (dr, w, kk) in enumerate(((0.0, 0.13, 1.0), (0.26, 0.09, 0.7), (0.50, 0.07, 0.45))):
        rr = r - dr
        if rr - w <= R * 0.9:
            continue
        hw = half * (0.75 + 0.25 * min(1.0, (rr - R) / 0.6))          # the arcs widen as they leave the ball
        band = arc_band(cam, cx, cz, aim, hw, rr - w, rr, y, 12)
        m = mask_of([band])
        blend(arr, m, np.clip(col * 1.05 + 30, 0, 255), 0.85 * k * kk)
        if j == 0:
            edge = arc_band(cam, cx, cz, aim, hw, rr - 0.035, rr + 0.01, y + 0.01, 12)
            blend(arr, mask_of([edge]), CREAM, 0.95 * k)


# ---------------------------------------------------------------- the dash (UKULELE)
def draw_dash_trail(arr, cam, trail, col, k):
    """afterimages: ghost discs at the ball's recent positions (oldest faintest) + three speed streaks"""
    if k <= 0.02 or len(trail) < 2:
        return
    col = np.asarray(col, float)
    n = len(trail)
    for j, p in enumerate(trail[:-1]):
        u = (j + 1) / n
        a = np.linspace(0, 2 * math.pi, 17)[:-1]
        P = S.to_world(p[0] + R * 0.92 * np.cos(a), p[1] + R * 0.92 * np.sin(a), R * 0.6)
        sp = cam.wp(P)
        m = mask_of([sp])
        blend(arr, m * (BAYER_FULL < 0.55 + 0.45 * u), col * 0.95, 0.70 * k * u)
        blend(arr, mask_of([np.vstack([sp, sp[:1]])], width=1), np.clip(col * 0.5 + 140, 0, 255), 0.85 * k * u)
    p0, p1 = trail[0], trail[-1]
    d = np.array(p1) - np.array(p0)
    L = float(np.hypot(*d))
    if L < 0.2:
        return
    d = d / L
    nrm = np.array([-d[1], d[0]])
    for off, h in ((-0.62, 0.7), (-0.25, 1.0), (0.25, 1.0), (0.62, 0.7)):
        a_ = np.array(p1) - d * R * 0.5 + nrm * off * R
        b_ = a_ - d * (L * 1.3 + 0.6) * h
        s = cam.wp(np.array([S.to_world(a_[0], a_[1], R * 0.9), S.to_world(b_[0], b_[1], R * 0.9)]))
        blend(arr, mask_of([s], width=1), np.clip(col * 0.45 + 160, 0, 255), 0.9 * min(1.0, 1.3 * k) * h)


# ---------------------------------------------------------------- CAPO (haste on the acoustic)
CAPO_METAL = np.array([44, 40, 52])


def draw_capo(arr, s, rp, k, col, t):
    """a capo clamped across the glass: a dark bar with a tension screw and a highlight in the fighter's colour"""
    if k <= 0.02:
        return
    x, y = s
    yb = y - rp * 0.30
    w = rp * 0.92
    m = mask_of([[(x - w, yb - 2), (x + w, yb - 2), (x + w, yb + 1.5), (x - w, yb + 1.5)]])
    blend(arr, m, CAPO_METAL, 0.95 * k)
    hl = mask_of([[(x - w, yb - 2), (x + w, yb - 2)]], width=1)
    blend(arr, hl, np.clip(np.asarray(col, float) * 0.9 + 60, 0, 255), 0.9 * k)
    sc = mask_of([[(x + w - 1, yb - 4), (x + w + 2, yb - 4), (x + w + 2, yb + 3), (x + w - 1, yb + 3)]])
    blend(arr, sc, np.array([210, 210, 222]), 0.9 * k)


# ---------------------------------------------------------------- FINGERPICK darts (arc projectiles)
DART_COL = np.array([226, 232, 244])


def dart_quads(x, z, ang, y=0.48, L_=0.34, Wd=0.07):
    d = np.array([math.cos(ang), math.sin(ang)])
    n = np.array([-d[1], d[0]])
    c = np.array([x, z])
    head = [c + d * L_ * 0.5, c + d * L_ * 0.15 - n * Wd, c + d * L_ * 0.05, c + d * L_ * 0.15 + n * Wd]
    body = [c + d * L_ * 0.1 - n * 0.012, c - d * L_ * 0.5 - n * 0.012, c - d * L_ * 0.5 + n * 0.012, c + d * L_ * 0.1 + n * 0.012]
    return (np.array([S.to_world(q[0], q[1], y) for q in head]), np.array([S.to_world(q[0], q[1], y) for q in body]))


def draw_dart(fr, cam, L, shade, x, z, ang, col):
    head, body = dart_quads(x, z, ang)
    fr.poly(cam, body, np.clip(np.array([212, 172, 112]) * 0.8 + 40, 0, 255))     # a bronze string segment
    c = shade(L, np.array([[0, 1.0, 0]]), head.mean(0)[None], np.array(col) * 0.45 + DART_COL * 0.55, rimk=0.4)[0]
    fr.poly(cam, head, c + 20)


# ---------------------------------------------------------------- BIG STRUM (slam): the giant pick
TORT_A = np.array([132, 62, 30])        # tortoiseshell
TORT_B = np.array([214, 132, 54])
TORT_C = np.array([58, 26, 18])


def pick_height(t, t0, t_slam):
    """hovers over the spot during the telegraph (rising a little as it winds up), the downstroke onto the spot, stays
    a beat, lifts away"""
    if t < t0:
        return None
    if t < t_slam - 0.10:
        u = min(1.0, (t - t0) / 0.3)
        return 1.55 - 0.25 * u + 0.30 * max(0.0, (t - t0) / max(0.01, t_slam - 0.10 - t0))
    if t < t_slam:
        u = (t - (t_slam - 0.10)) / 0.10
        return 1.60 - 1.35 * u * u
    if t < t_slam + 0.25:
        return 0.25
    if t < t_slam + 0.6:
        return 0.25 + 2.4 * ((t - t_slam - 0.25) / 0.35) ** 2
    return None


def pick_outline(cx, cz, rad, tilt):
    """a plectrum in the ring plane: the convex hull of two broad shoulders and a small rounded tip (tip toward tilt)"""
    from scipy.spatial import ConvexHull
    d = np.array([math.cos(tilt), math.sin(tilt)])
    n = np.array([-d[1], d[0]])
    c = np.array([cx, cz])
    pts = []
    for ctr, rr in ((c - d * rad * 0.30 + n * rad * 0.42, rad * 0.46), (c - d * rad * 0.30 - n * rad * 0.42, rad * 0.46),
                    (c + d * rad * 0.78, rad * 0.14)):
        for a in np.linspace(0, 2 * math.pi, 20, endpoint=False):
            pts.append(ctr + rr * np.array([math.cos(a), math.sin(a)]))
    pts = np.array(pts)
    return pts[ConvexHull(pts).vertices]


def draw_pick(arr, cam, cx, cz, rad, h, alpha, tilt, swing=0.0):
    """the pick: a dark edge layer under the top face, a dithered two-tone tortoiseshell top with a shine; `swing`
    (0..1) tips it through the downstroke"""
    pts = pick_outline(cx, cz, rad, tilt)
    d = np.array([math.cos(tilt), math.sin(tilt)])
    for yy, top in ((h, False), (h + 0.07, True)):
        P = np.array([S.to_world(x, z, yy + swing * 0.35 * ((np.array([x, z]) - [cx, cz]) @ d) / max(rad, 1e-6))
                      for x, z in pts])
        sp = cam.wp(P)
        m = mask_of([sp])
        if not top:
            blend(arr, m, TORT_C, alpha)
            continue
        yy_, xx_ = np.nonzero(m > 0.5)
        if len(xx_):
            mot = np.sin(xx_ * 0.37 + yy_ * 0.21 + 1.3) + np.sin(xx_ * 0.17 - yy_ * 0.43)
            q = np.floor(np.clip((mot + 2) / 4, 0, 1) * 2 + BAYER_FULL[yy_, xx_] * 0.95) / 2
            c = TORT_A[None] * (1 - q[:, None]) + TORT_B[None] * q[:, None]
            arr[yy_, xx_] = arr[yy_, xx_] * (1 - alpha) + c * alpha
        edge = mask_of([np.vstack([sp, sp[:1]])], width=1)
        blend(arr, edge, np.array([255, 214, 150]), 0.6 * alpha)
        sh = cam.wp(np.array([S.to_world(*(np.array([cx, cz]) - d * rad * 0.25 + np.array([-d[1], d[0]]) * rad * 0.3), h + 0.08),
                              S.to_world(*(np.array([cx, cz]) - d * rad * 0.05 + np.array([-d[1], d[0]]) * rad * 0.45), h + 0.08)]))
        blend(arr, mask_of([sh], width=1), np.array([255, 246, 226]), 0.8 * alpha)


# ---------------------------------------------------------------- UKE-NADO: whirl arcs round the uke
def draw_tornado(arr, cam, p, t, k, col):
    """three pink arcs spinning fast round the ball at rising heights (a little tornado)"""
    if k <= 0.02:
        return
    col = np.asarray(col, float)
    for j in range(3):
        rr = R * (1.25 + 0.35 * j)
        y = R * (0.4 + 0.7 * j)
        a0 = t * (16 - 3 * j) + j * 2.1
        a = np.linspace(a0, a0 + 2.2, 12)
        P = S.to_world(p[0] + rr * np.cos(a), p[1] + rr * np.sin(a), y + 0.1 * np.sin(a * 2))
        s = cam.wp(P)
        blend(arr, mask_of([s], width=2), np.clip(col * 0.7 + 90, 0, 255), 0.8 * k)
        blend(arr, mask_of([s[len(s) // 2:]], width=1), np.array([255, 246, 250]), 0.8 * k)
