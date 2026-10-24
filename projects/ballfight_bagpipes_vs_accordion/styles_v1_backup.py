"""v1 BACKUP (drone aura vs bellows rings, seed 1581; the roster has since moved to v2, so this no longer reproduces v1). #30 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'drone'     the bagpipes' drone aura: a tartan band on the canvas around the ball, sound ripples rolling out
                      to it on every tick (8ths), hotter while the enemy is inside; DOUBLE DRONE doubles the band
    style 'bellows'   the accordion's strokes: a push is a pleated ring blasting out (magenta folds + dark creases); a
                      pull is a dashed ring collapsing inward with wind streaks pointing at the ball (the inhale)
    'squeezebox'      the SQUEEZEBOX shield: a closed bellows (pleated collar) folding shut around the ball
    'giant_bellows'   FULL BELLOWS (slam): a giant bellows hanging over the predicted spot, stretched open during the
                      telegraph, then squeezed shut onto it (the hit), then lifted away

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27 / #28's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL

# ---------------------------------------------------------------- shared helpers (same as #28's styles)
def annulus_polys(cam, cx, cz, r_in, r_out, y, n=36, a0=0.0, a1=2 * math.pi):
    """screen quads of a flat ring (or an arc a0..a1), one quad per segment"""
    a = np.linspace(a0, a1, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    return [[so[j], so[j + 1], si[j + 1], si[j]] for j in range(n)]


def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def blend(arr, m, col, k):
    arr += (np.array(col, float)[None, None] - arr) * (m * k)[..., None]


# ---------------------------------------------------------------- the drone aura (canvas decal, per pixel)
TARTAN = dict(dk=np.array([26, 96, 46]), lt=np.array([84, 206, 98]), red=np.array([214, 48, 44]),
              gold=np.array([246, 204, 72]))


def drone_layers(sxp, szp, bayer, cx, cz, r_aura, t, tick_t, tick_hit, hot, double, col):
    """the drone aura around (cx, cz) on the canvas pixels: returns (light (n, 3) added to the canvas light,
    paint (n, 3) RGB, paint weight (n,)): a tartan band painted at the ring (a sett along the angle and across the band,
    so it reads as cloth: green blocks with a red and a gold over-check), a faint green wash inside, a sound ripple
    rolling out from the ball to the ring after each tick (brighter when that tick landed), `hot` 0..1 while the enemy
    is inside (the band glows), `double` 0..1 while DOUBLE DRONE runs (a second band inside the first)"""
    dx, dz = sxp - cx, szp - cz
    rd = np.hypot(dx, dz)
    ang = np.arctan2(dz, dx)
    n = rd.shape[0]
    light = np.zeros((n, 3))
    paint = np.zeros((n, 3))
    wgt = np.zeros(n)
    inside = np.clip((r_aura - rd) / 0.12, 0, 1) * (rd > R * 0.9)
    light += (inside * (0.08 + 0.14 * hot))[:, None] * (col / 255.0)[None]
    bands = [(r_aura, 1.0)] + ([(r_aura - 0.30, double)] if double > 0.02 else [])
    for rb, kb in bands:
        w = 0.20
        u = (rd - (rb - w)) / w                                     # 0..1 across the band (inner -> outer)
        on = (u >= 0) & (u <= 1)
        if not on.any():
            continue
        s_ = (ang * rb / 0.24 + 0.5 * t) % 1.0                     # the sett creeps around the ring
        c = np.where((s_ < 0.5)[:, None], TARTAN['dk'][None], TARTAN['lt'][None]).astype(float)
        red = (np.abs(s_ - 0.25) < 0.06) | (np.abs(u - 0.5) < 0.10)
        gold = (np.abs(s_ - 0.75) < 0.035) | (np.abs(u - 0.18) < 0.05) | (np.abs(u - 0.82) < 0.05)
        c = np.where(red[:, None], TARTAN['red'][None], c)
        c = np.where(gold[:, None], TARTAN['gold'][None], c)
        c = c * (1.0 + 0.45 * hot)
        edge = np.clip(1 - np.abs(u - 0.5) * 2, 0, 1) ** 0.3
        wk = np.floor(edge * (0.80 + 0.15 * hot) * kb * 4 + bayer * 0.9) / 4 * on
        upd = wk > wgt
        paint[upd] = c[upd]
        wgt = np.maximum(wgt, wk)
        light += (on * edge * 0.25 * hot * kb)[:, None] * (col / 255.0)[None]
    # the ripple after each tick: from the ball's surface out to the ring
    if tick_t is not None:
        u = (t - tick_t) / 0.22
        if 0 <= u < 1:
            rr = R + (r_aura - R) * u
            band = np.clip(1 - np.abs(rd - rr) / 0.07, 0, 1) * (rd < r_aura + 0.05)
            band = np.floor(band * 3 + bayer * 0.9) / 3
            k = (1.0 if tick_hit else 0.30) * (1 - u)
            light += (band * k)[:, None] * (col / 255.0 * 0.9 + 0.35)[None]
    return light, paint, np.clip(wgt, 0, 1)


# ---------------------------------------------------------------- bellows strokes (rings over the canvas)
PLEAT_DK = np.array([46, 16, 44])
PLEAT_EDGE = np.array([255, 236, 250])


def stroke_state(t, t0, reach, speed, push, pull_k, pull_speed):
    """(radius, alpha) of a bellows stroke fired at t0: a push grows from R to R + reach, a pull shrinks from
    R + pull_k * reach to R; both fade quickly at the end"""
    u = t - t0
    if u < 0:
        return None
    if push:
        r = R + min(reach, speed * u)
        t_full = reach / speed
        a = 1.0 if u <= t_full else max(0.0, 1 - (u - t_full) / 0.18)
    else:
        r0 = R + reach * pull_k
        sp = speed * pull_speed
        r = max(R, r0 - sp * u)
        t_full = (r0 - R) / sp
        a = min(1.0, 0.35 + u / 0.12) if u <= t_full else max(0.0, 1 - (u - t_full) / 0.10)
    if a <= 0:
        return None
    return r, a


def draw_push(arr, cam, cx, cz, r, a, col, main=True, y=0.30):
    """a push: a pleated ring - magenta folds with dark creases and pale fold edges (the bellows' own pattern)"""
    n = 24
    w = 0.16 if main else 0.10
    polys = annulus_polys(cam, cx, cz, max(R * 0.9, r - w), r, y, n=n * 2)
    k = a * (1.0 if main else 0.65)
    fold = [p for j, p in enumerate(polys) if j % 2 == 0]
    crease = [p for j, p in enumerate(polys) if j % 2 == 1]
    blend(arr, mask_of(fold), np.clip(np.array(col) * 1.05 + 25, 0, 255), 0.88 * k)
    blend(arr, mask_of(crease), PLEAT_DK + np.array(col) * 0.25, 0.80 * k)
    if main and k > 0.3:                                      # pale fold edges on the outer rim
        a_ = np.linspace(0, 2 * math.pi, n * 2 + 1)
        P = cam.wp(S.to_world(cx + r * np.cos(a_), cz + r * np.sin(a_), y + 0.01))
        for j in range(0, n * 2, 2):
            xi, yi = int(round(P[j, 0])), int(round(P[j, 1]))
            if 0 <= xi < W and 0 <= yi < H:
                arr[yi, xi] = arr[yi, xi] * (1 - k) + PLEAT_EDGE * k


def draw_pull(arr, cam, cx, cz, r, a, col, ball_p, main=True, y=0.30):
    """a pull: a dashed ring closing in, with wind streaks from the ring toward the accordion (the inhale)"""
    n = 30
    w = 0.08 if main else 0.05
    polys = annulus_polys(cam, cx, cz, max(R * 0.9, r - w), r, y, n=n)
    dash = [p for j, p in enumerate(polys) if j % 3 != 2]
    k = a * (0.95 if main else 0.6)
    blend(arr, mask_of(dash), np.clip(np.array(col) * 0.9 + 60, 0, 255), 0.85 * k)
    # streaks: short lines from just outside the ring toward the ball, sliding inward
    im = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(im)
    for j in range(12):
        ang = j * math.pi / 6 + 0.26
        r1 = r + 0.42
        r2 = max(R * 1.05, r + 0.08)
        p1 = S.to_world(cx + r1 * math.cos(ang), cz + r1 * math.sin(ang), y + 0.02)
        p2 = S.to_world(cx + r2 * math.cos(ang), cz + r2 * math.sin(ang), y + 0.02)
        s1, s2 = cam.wp(np.array([p1, p2]))
        d.line([tuple(s1), tuple(s2)], fill=255)
    m = np.asarray(im, np.float32) / 255.0
    blend(arr, m, np.array([250, 230, 250]), 0.55 * k)


# ---------------------------------------------------------------- SQUEEZEBOX (shield): a closed bellows around the ball
def collar_polys(cam, cx, cz, r_in, r_out, y, half, n=32):
    """the collar's ring split in the back half (half=0) and front half (half=1) as the camera sees it"""
    a = np.linspace(0, 2 * math.pi, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    dz_o = cam.depth(oi)
    c = cam.depth(S.to_world(cx, cz, y)[None])[0]
    out = []
    for j in range(n):
        if ((dz_o[j] + dz_o[j + 1]) / 2 < c) == bool(half):
            out.append((j, [so[j], so[j + 1], si[j + 1], si[j]]))
    return out


def draw_squeezebox(arr, cam, cx, cz, t, k, half, col, close):
    """k: 0..1 strength; close: 0..1, 0 = wide open, 1 = folded shut (the collar tightens and stacks its pleats up)"""
    for layer in range(3):                                     # three stacked pleat rings = a closed bellows
        y = R * (0.62 + 0.38 * layer) * (1 - 0.15 * close) + 0.05
        r_out = R * (1.85 - 0.45 * close) - 0.04 * layer
        r_in = R * 0.96
        items = collar_polys(cam, cx, cz, r_in, r_out, y, half)
        fold = [p for j, p in items if (j + layer) % 2 == 0]
        crease = [p for j, p in items if (j + layer) % 2 == 1]
        blend(arr, mask_of(fold), np.clip(np.array(col) * (0.95 + 0.1 * layer) + 20, 0, 255), 0.86 * k)
        blend(arr, mask_of(crease), PLEAT_DK + np.array(col) * 0.2, 0.86 * k)
    # brass corner guards glinting as it turns
    for j in range(4):
        ang = j * math.pi / 2 + t * 2.0
        p = S.to_world(cx + R * (1.8 - 0.45 * close) * math.cos(ang), cz + R * (1.8 - 0.45 * close) * math.sin(ang), R * 1.0)
        s = cam.wp(p[None])[0]
        front = cam.depth(p[None])[0] < cam.depth(S.to_world(cx, cz, R)[None])[0]
        if front == bool(half):
            xi, yi = int(round(s[0])), int(round(s[1]))
            if 1 <= xi < W - 1 and 1 <= yi < H - 1:
                arr[yi - 1:yi + 1, xi - 1:xi + 1] = arr[yi - 1:yi + 1, xi - 1:xi + 1] * (1 - k) + np.array([255, 214, 120]) * k


# ---------------------------------------------------------------- FULL BELLOWS (slam): a giant bellows over the spot
def giant_bellows_open(t, t0, t_slam):
    """(height of the bottom plate, stretch 0..1) or None: stretched open while it hangs, squeezed shut on the hit"""
    if t < t0:
        return None
    if t < t_slam - 0.10:
        u = min(1.0, (t - t0) / 0.35)
        return 2.6 - 0.3 * u, 0.5 + 0.5 * u                  # drops in, then stretches open (the inhale)
    if t < t_slam:
        u = (t - (t_slam - 0.10)) / 0.10
        return 2.3 - 2.25 * u * u, 1.0 - 0.85 * u * u         # squeezed shut onto the spot
    if t < t_slam + 0.30:
        return 0.05, 0.15
    if t < t_slam + 0.65:
        u = (t - t_slam - 0.30) / 0.35
        return 0.05 + 3.2 * u * u, 0.15 + 0.3 * u             # lifted away
    return None


def _square(cx, cz, a, y, rot=0.0):
    """corners of a square of half-size a at height y (ring frame), turned by rot, as world points (4, 3)"""
    c, s_ = math.cos(rot), math.sin(rot)
    pts = []
    for u, v in ((-1, -1), (1, -1), (1, 1), (-1, 1)):
        x, z = u * a, v * a
        pts.append(S.to_world(cx + c * x - s_ * z, cz + s_ * x + c * z, y))
    return np.array(pts)


def draw_giant_bellows(arr, cam, cx, cz, rad, t, t0, t_slam, col, alpha=1.0):
    """a giant accordion bellows over the spot: a square box of zig-zag pleats (bright folds that face up, dark creases
    that face down, pale ridges with metal corner guards), the treble end on top with its keyboard"""
    st = giant_bellows_open(t, t0, t_slam)
    if st is None:
        return
    h0, stretch = st
    n_fold = 8
    gap = 0.07 + 0.17 * stretch                                # height of one half-fold
    a_out, a_in = rad * 0.86, rad * 0.66
    rot = 0.35
    rings = [(h0 + j * gap, a_out if j % 2 == 0 else a_in) for j in range(n_fold * 2 + 1)]
    sq = [_square(cx, cz, a_, y, rot) for y, a_ in rings]
    centre = S.to_world(cx, cz, h0 + gap * n_fold)
    col = np.array(col, float)
    bright, dark = np.clip(col * 1.08 + 28, 0, 255), PLEAT_DK + col * 0.22
    faces = []
    for j in range(len(rings) - 1):
        A, B = sq[j], sq[j + 1]
        up = rings[j + 1][1] < rings[j][1]                     # narrowing upward: the face tilts up toward the spot
        for e in range(4):
            Q = np.array([A[e], A[(e + 1) % 4], B[(e + 1) % 4], B[e]])
            n = np.cross(Q[1] - Q[0], Q[3] - Q[0])
            mid = Q.mean(0)
            if np.dot(n, mid - centre) < 0:
                n = -n
            if np.dot(n, cam.pos - mid) <= 0:
                continue
            faces.append((cam.depth(mid[None])[0], Q, bright if up else dark, j))
    faces.sort(key=lambda f: -f[0])
    for dz, Q, c, j in faces:
        blend(arr, mask_of([cam.wp(Q)]), c, alpha)
    # pale ridges on the outer folds + metal corner guards
    im = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(im)
    for j in range(0, len(rings), 2):
        P = cam.wp(sq[j])
        for e in range(4):
            mid = (sq[j][e] + sq[j][(e + 1) % 4]) / 2
            nrm = mid - S.to_world(cx, cz, rings[j][0])
            if np.dot(nrm, cam.pos - mid) > 0:
                d.line([tuple(P[e]), tuple(P[(e + 1) % 4])], fill=255)
    blend(arr, np.asarray(im, np.float32) / 255.0, PLEAT_EDGE, 0.75 * alpha)
    for j in range(0, len(rings), 2):
        for q in cam.wp(sq[j]):
            xi, yi = int(round(q[0])), int(round(q[1]))
            if 1 <= xi < W - 1 and 1 <= yi < H - 1:
                arr[yi - 1:yi + 1, xi - 1:xi + 1] = arr[yi - 1:yi + 1, xi - 1:xi + 1] * (1 - alpha) + np.array([214, 220, 236]) * alpha
    # the treble end on top: a dark lacquered board with the keyboard along one edge
    y_top = rings[-1][0] + 0.02
    top = _square(cx, cz, a_out * 1.06, y_top, rot)
    blend(arr, mask_of([cam.wp(top)]), np.array([40, 26, 44]), alpha)
    nk = 9
    for j in range(nk):                                        # white keys along the far-left edge, black keys between
        u0, u1 = -1 + 2 * j / nk + 0.02, -1 + 2 * (j + 1) / nk - 0.02
        quad = []
        for u, v in ((u0, 0.25), (u1, 0.25), (u1, 0.95), (u0, 0.95)):
            x, z = u * a_out, v * a_out
            c_, s_ = math.cos(rot), math.sin(rot)
            quad.append(S.to_world(cx + c_ * x - s_ * z, cz + s_ * x + c_ * z, y_top + 0.01))
        blend(arr, mask_of([cam.wp(np.array(quad))]), np.array([246, 242, 230]), alpha)
        if j % 7 not in (2, 6) and j < nk - 1:
            ub = u1 + 0.02
            quad = []
            for u, v in ((ub - 0.07, 0.25), (ub + 0.07, 0.25), (ub + 0.07, 0.58), (ub - 0.07, 0.58)):
                x, z = u * a_out, v * a_out
                quad.append(S.to_world(cx + c_ * x - s_ * z, cz + s_ * x + c_ * z, y_top + 0.02))
            blend(arr, mask_of([cam.wp(np.array(quad))]), np.array([22, 18, 26]), alpha)
    # a chrome grille strip and two buttons on the near half
    for b in range(3):
        q = S.to_world(cx + a_out * (-0.4 + 0.4 * b) * math.cos(rot) + a_out * 0.55 * math.sin(rot),
                       cz + a_out * (-0.4 + 0.4 * b) * math.sin(rot) - a_out * 0.55 * math.cos(rot), y_top + 0.01)
        s2 = cam.wp(q[None])[0]
        xi, yi = int(round(s2[0])), int(round(s2[1]))
        if 0 <= xi < W - 2 and 0 <= yi < H - 2:
            arr[yi:yi + 2, xi:xi + 2] = arr[yi:yi + 2, xi:xi + 2] * (1 - alpha) + np.array([246, 240, 226]) * alpha
