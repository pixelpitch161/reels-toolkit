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


# ================================================================ v2 (BALLFIGHT v2 redesign): SURF'S UP vs SIX-STRING RAKE
def _perp(u):
    return np.array([-u[1], u[0]])


def overlay_polys(arr, polys, alpha_max=1.0):
    """draw [(screen pts, rgb, alpha)] onto one RGBA overlay in the given order (later ones cover earlier ones), then
    composite it once: fast for many small faces"""
    if not polys:
        return arr
    ov = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    d = ImageDraw.Draw(ov)
    for pts, rgb, a in polys:
        if len(pts) >= 3:
            d.polygon([tuple(q) for q in pts], fill=tuple(int(v) for v in np.clip(rgb, 0, 255)) + (int(255 * a),))
    o = np.asarray(ov, np.float32)
    k = o[..., 3:4] / 255.0 * alpha_max
    return arr * (1 - k) + o[..., :3] * k


# ---------------------------------------------------------------- SURF'S UP: the rolling low-poly wave
WATER = {'deep': np.array([40, 128, 170]), 'mid': np.array([72, 182, 212]), 'lite': np.array([150, 230, 242]),
         'foam': np.array([252, 250, 252]), 'streak': np.array([206, 244, 250])}
# the wave's cross-section, front line at x = 0, x running back (away from where it rolls), y up (fractions of L_back, H)
PROFILE = [(1.35, 0.0), (0.95, 0.30), (0.62, 0.62), (0.32, 0.88), (0.12, 1.0), (-0.12, 0.92), (-0.08, 0.6),
           (0.02, 0.28), (0.06, 0.0)]
PROF_COL = ['mid', 'mid', 'lite', 'foam', 'foam', 'lite', 'mid', 'deep']


def wave_quads(c, u, hw, Hh, t=0.0, L_back=0.95, M=7, lip=0.0):
    """world quads [(Q (4, 3), colour key, face id)] of a curling wave whose front line passes through c (ring frame),
    rolling along u, half-width hw, height Hh; `lip` (0..1) throws the lip forward as it crashes"""
    n = _perp(u)
    vs = np.linspace(-1, 1, M + 1)
    grid = []
    for j, v in enumerate(vs):
        hj = Hh * max(0.0, 1 - abs(v) ** 3) ** 0.6
        wob = 0.05 * math.sin(3.1 * v + t * 9.0)                       # the lip rolls along the wave
        bow = 0.28 * (1 - v * v)                                        # the front bows forward in the middle
        row = []
        for i, (px, py) in enumerate(PROFILE):
            x = px * L_back - bow
            if i in (4, 5, 6):
                x += wob - lip * 0.35 * py
            q = c - u * x + n * (v * hw)
            row.append(S.to_world(q[0], q[1], max(0.0, py * hj)))
        grid.append(row)
    out = []
    for j in range(M):
        for i in range(len(PROFILE) - 1):
            Q = np.array([grid[j][i], grid[j + 1][i], grid[j + 1][i + 1], grid[j][i + 1]])
            out.append((Q, PROF_COL[i], j * 13 + i))
    return out


def draw_wave(fr, cam, L, shade, c, u, hw, Hh, t, rim_col, lip=0.0):
    quads = wave_quads(c, u, hw, Hh, t, lip=lip)
    if not quads:
        return
    dep = cam.depth(np.array([Q.mean(0) for Q, _, _ in quads]))
    for idx in np.argsort(-dep):
        Q, key, fid = quads[idx]
        nrm = np.cross(Q[1] - Q[0], Q[3] - Q[0])
        nn = np.linalg.norm(nrm)
        if nn < 1e-9:
            continue
        nrm = nrm / nn
        if np.dot(nrm, cam.pos - Q.mean(0)) < 0:
            nrm = -nrm
        jit = 1.0 + 0.07 * math.sin(fid * 12.9898) * (key != 'foam')
        if key == 'mid' and (fid * 7 + int(t * 6)) % 5 == 0:
            key = 'streak'                                              # foam streaks running down the back
        col = shade(L, nrm[None], Q.mean(0)[None], WATER[key] * jit, rimk=0.5)[0]
        col = col * 0.82 + WATER[key] * 0.18                            # water glows a little in the spot
        if key in ('lite', 'foam'):
            col = col * 0.85 + np.asarray(rim_col, float) * 0.15        # the uke's pink catches the crest
        fr.poly(cam, Q, np.clip(col, 0, 255))
    # the lip: a foam line with a pink rim
    n = _perp(u)
    pts = []
    for v in np.linspace(-0.92, 0.92, 11):
        hj = Hh * max(0.0, 1 - abs(v) ** 3) ** 0.6
        x = PROFILE[5][0] * 0.95 - 0.28 * (1 - v * v) + 0.05 * math.sin(3.1 * v + t * 9.0) - lip * 0.35 * PROFILE[5][1]
        q = c - u * x + n * (v * hw)
        pts.append(S.to_world(q[0], q[1], PROFILE[5][1] * hj + 0.02))
    fr.line(cam, np.array(pts), np.clip(np.asarray(rim_col, float) * 0.5 + 128, 0, 255), 2)


def wave_shadow_quads(c, u, hw, Hh):
    """the crest strip, for the spot's canvas shadow"""
    return [Q for Q, key, _ in wave_quads(c, u, hw, Hh) if key in ('mid', 'foam', 'lite')]


# ---------------------------------------------------------------- SIX-STRING RAKE: six glowing string lines
STRINGS6 = [np.array(c, float) for c in ((198, 150, 86), (206, 160, 96), (214, 172, 110), (226, 218, 204),
                                          (234, 228, 218), (242, 238, 230))]   # wound bronze E A D, plain G B E


def rake_segments(c, u, n, s, L, gap, R0=R):
    """the six strings (pairs of ring-frame points) of a rake whose bundle sits at lateral offset s"""
    out = []
    for j in range(6):
        o = (j - 2.5) * gap
        a = c + u * R0 + n * (s + o)
        b = c + u * (R0 + L) + n * (s + o)
        out.append((a, b))
    return out


def draw_strings(arr, cam, segs, glow_col, k, y=0.5, prev=None, thick=(2, 2, 2, 1, 1, 1)):
    """six strings: a glow in the fighter's colour under a bright core; `prev` (the bundle a frame ago) adds a motion
    fan between the two positions"""
    if k <= 0.02:
        return arr
    glow_col = np.asarray(glow_col, float)
    polys = []
    if prev is not None:
        for (a, b), (pa, pb) in zip(segs, prev):
            q = cam.wp(np.array([S.to_world(*a, y), S.to_world(*b, y), S.to_world(*pb, y), S.to_world(*pa, y)]))
            polys.append((q, glow_col * 0.5 + 120, 0.22 * k))
    arr = overlay_polys(arr, polys)
    gm = mask_of([cam.wp(np.array([S.to_world(*a, y), S.to_world(*b, y)])) for a, b in segs], width=5)
    blend(arr, gm, np.clip(glow_col * 1.05 + 30, 0, 255), 0.55 * k)              # one warm glow under the bundle
    for j, (a, b) in enumerate(segs):
        s = cam.wp(np.array([S.to_world(*a, y), S.to_world(*b, y)]))
        blend(arr, mask_of([s], width=thick[j]), np.clip(STRINGS6[j] * 0.6 + 110, 0, 255), 0.98 * k)
    return arr


def draw_whip(arr, cam, p, ang, ln, tau, t_ext, glow_col, y=0.5, j=0):
    """one FINGERPICK string flicking out like a whip and snapping back: a travelling wave along it, a bright tip"""
    if tau < 0 or tau > 2 * t_ext:
        return arr
    ext = tau / t_ext if tau <= t_ext else 1 - (tau - t_ext) / t_ext
    u = np.array([math.cos(ang), math.sin(ang)])
    n = _perp(u)
    Lc = ln * max(0.0, ext)
    if Lc < 0.05:
        return arr
    pts = []
    for x in np.linspace(0, 1, 12):
        lat = 0.09 * math.sin(math.pi * x * 2.5 - tau * 60 + j) * x * (1 - 0.5 * ext)
        q = p + u * (R + Lc * x) + n * lat
        pts.append(S.to_world(q[0], q[1], y))
    s = cam.wp(np.array(pts))
    glow_col = np.asarray(glow_col, float)
    blend(arr, mask_of([s], width=3), np.clip(glow_col * 0.8 + 50, 0, 255), 0.5)
    blend(arr, mask_of([s], width=1), STRINGS6[j % 6] * 1.05 + 20, 0.95)
    tx, ty = int(round(s[-1][0])), int(round(s[-1][1]))
    if 0 <= tx < W and 0 <= ty < H:
        arr[max(0, ty - 1):ty + 2, max(0, tx - 1):tx + 2] = np.array([255, 248, 230])
    return arr


def clip_segment(a, b, lim):
    """Liang-Barsky: the part of segment a-b inside the square |x|, |z| <= lim (or None)"""
    t0, t1 = 0.0, 1.0
    d = b - a
    for p, q in ((-d[0], a[0] + lim), (d[0], lim - a[0]), (-d[1], a[1] + lim), (d[1], lim - a[1])):
        if abs(p) < 1e-12:
            if q < 0:
                return None
            continue
        r = q / p
        if p < 0:
            t0 = max(t0, r)
        else:
            t1 = min(t1, r)
        if t0 > t1:
            return None
    return a + d * t0, a + d * t1


def sweep_segments(c, u, front, lim, gap=0.14):
    """BIG STRUM: six strings lying across the whole ring (perpendicular to u), the first at `front` from c, the rest
    trailing behind it, clipped to the canvas"""
    n = _perp(u)
    out = []
    for j in range(6):
        m = c + u * (front - j * gap)
        seg = clip_segment(m - n * 9.0, m + n * 9.0, lim)
        if seg is not None:
            out.append(seg)
    return out


# ---------------------------------------------------------------- UKE-NADO: the waterspout
SPOUT_H = 2.5


def spout_radius(y):
    """funnel: a wide splash at the base, a pinched waist, flaring at the top"""
    v = y / SPOUT_H
    return 0.62 * (1 - v) ** 3 + 0.22 + 0.55 * v ** 2


def draw_spout(arr, cam, p, t, k, rim_col):
    """a spinning funnel of water: stacked bands with stripes turning fast, back half first, then the front half"""
    if k <= 0.02:
        return arr
    rim_col = np.asarray(rim_col, float)
    ys = np.linspace(0.0, SPOUT_H * (0.35 + 0.65 * min(1.0, k * 1.4)), 12)
    na = 20
    angs = np.linspace(0, 2 * math.pi, na + 1)
    cdep = cam.depth(S.to_world(p[0], p[1], 0.8)[None])[0]
    back, front = [], []
    for i in range(len(ys) - 1):
        r0, r1 = spout_radius(ys[i]), spout_radius(ys[i + 1])
        sw = t * (9.0 + 3.0 * i) + i * 0.7
        for a in range(na):
            a0, a1 = angs[a] + sw, angs[a + 1] + sw
            P = np.array([S.to_world(p[0] + r0 * math.cos(a0), p[1] + r0 * math.sin(a0), ys[i]),
                          S.to_world(p[0] + r0 * math.cos(a1), p[1] + r0 * math.sin(a1), ys[i]),
                          S.to_world(p[0] + r1 * math.cos(a1), p[1] + r1 * math.sin(a1), ys[i + 1]),
                          S.to_world(p[0] + r1 * math.cos(a0), p[1] + r1 * math.sin(a0), ys[i + 1])])
            stripe = ((a * 2 + i * 3) // 4) % 4                        # spiral bands winding up the funnel
            col = (WATER['foam'], WATER['lite'], WATER['mid'], WATER['lite'])[stripe]
            col = col * 0.85 + rim_col * 0.15
            al = (0.8 if stripe == 0 else 0.55) * min(1.0, k) * (0.6 + 0.4 * (1 - i / len(ys)))
            q = cam.wp(P)
            if cam.depth(P.mean(0)[None])[0] > cdep:
                back.append((q, col * 0.8, al * 0.8))
            else:
                front.append((q, col, al))
    return overlay_polys(arr, back + front)
