"""#34 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'slide'   the trombone's slide shot out like a lance: two brass tubes, braces, the crook (U-bend) at the tip
    style 'arrow'   a hunting-call arrow: copper head, pale shaft, fletching in the fighter's colour (+ a curving trail)
    'bellrim'       the STOPPED HORN shield: a copper bell rim around the ball (the hand-stopped bell), sweeping glint
    'giant bell'    PEDAL TONE: a giant brass bell, mouth down, stomping the predicted spot (the slam)

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H

R = S.R_BALL

BRASS = np.array([232, 188, 92])
BRASS_HI = np.array([255, 238, 176])
BRASS_DK = np.array([150, 104, 40])
COPPER = np.array([214, 120, 66])
COPPER_DK = np.array([128, 62, 30])
SHAFT = np.array([226, 210, 172])


def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


# ---------------------------------------------------------------- the slide lance (a thrusting blade)
TUBE_GAP, TUBE_W, OUTER_W = 0.075, 0.034, 0.05


def slide_geom(p, th, ext, y=R):
    """world quads [(Q, colour, kind)] of the slide from inside the ball to the crook at R + ext"""
    d = np.array([math.cos(th), math.sin(th)])
    n = np.array([-d[1], d[0]])
    r0, r1 = R * 0.8, R + ext
    Wp = lambda r, w: S.to_world(*(p + d * r + n * w), y)
    out = []
    r_out = max(r0 + 0.08, r1 - max(0.35, 0.62 * (r1 - r0)))     # the outer slide (thicker) rides on the inner tubes
    for side in (-1, 1):
        c = side * TUBE_GAP
        out.append((np.array([Wp(r0, c - TUBE_W / 2), Wp(r1, c - TUBE_W / 2), Wp(r1, c + TUBE_W / 2),
                              Wp(r0, c + TUBE_W / 2)]), BRASS, 'tube'))
        out.append((np.array([Wp(r_out, c - OUTER_W / 2), Wp(r1 - 0.02, c - OUTER_W / 2), Wp(r1 - 0.02, c + OUTER_W / 2),
                              Wp(r_out, c + OUTER_W / 2)]), BRASS * 1.04, 'outer'))
    for rb in (r0 + 0.14, r_out + 0.06):                          # the braces across the two tubes
        if rb < r1 - 0.1:
            out.append((np.array([Wp(rb - 0.025, -TUBE_GAP), Wp(rb + 0.025, -TUBE_GAP), Wp(rb + 0.025, TUBE_GAP),
                                  Wp(rb - 0.025, TUBE_GAP)]), BRASS_DK * 1.2, 'brace'))
    # the crook: a U-bend beyond the tube ends (8 segments)
    a = np.linspace(-math.pi / 2, math.pi / 2, 9)
    for j in range(8):
        pts = []
        for aa, rr in ((a[j], TUBE_GAP + OUTER_W / 2), (a[j + 1], TUBE_GAP + OUTER_W / 2),
                       (a[j + 1], TUBE_GAP - OUTER_W / 2), (a[j], TUBE_GAP - OUTER_W / 2)):
            pts.append(Wp(r1 + rr * math.cos(aa) * 0.9, rr * math.sin(aa)))
        out.append((np.array(pts), BRASS, 'crook'))
    tip = S.to_world(*(p + d * (r1 + TUBE_GAP)), y)
    return out, tip


def draw_slide(fr, cam, L, shade, st, glow_col=None, glow=0.0):
    quads, tip = slide_geom(st['p'], st['th'], st['ext'])
    up = np.array([[0, 1.0, 0]])
    for Q, col, kind in quads:
        c = shade(L, up, Q.mean(0)[None], col, rimk=0.45)[0]
        if glow_col is not None and glow > 0 and kind in ('outer', 'crook'):
            c = c * (1 - 0.45 * glow) + np.asarray(glow_col) * 0.45 * glow + 30 * glow
        fr.poly(cam, Q, np.clip(c, 0, 255))
    for Q, col, kind in quads:                                    # a glint line along each outer slide (round tubes)
        if kind == 'outer':
            s = cam.wp(Q)
            fr.draw.line([tuple((s[0] + s[3]) / 2), tuple((s[1] + s[2]) / 2)], fill=tuple(int(v) for v in BRASS_HI), width=1)
    return cam.wp(tip[None])[0]


def slide_trail_polys(cam, st, ext0):
    """the streak left by a fast thrust: a soft band from the previous tip position to the current one"""
    d = np.array([math.cos(st['th']), math.sin(st['th'])])
    n = np.array([-d[1], d[0]])
    p = st['p']
    a, b = R + ext0, R + st['ext'] + TUBE_GAP
    if b - a < 0.05:
        return []
    w = TUBE_GAP + OUTER_W
    pts = [p + d * a + n * w * 0.6, p + d * b + n * w, p + d * b - n * w, p + d * a - n * w * 0.6]
    return [cam.wp(np.array([S.to_world(q[0], q[1], R) for q in pts]))]


# ---------------------------------------------------------------- hunting-call arrows (curving volley projectiles)
def arrow_quads(x, z, ang, y=0.5, L_=0.46, Wd=0.075):
    d = np.array([math.cos(ang), math.sin(ang)])
    n = np.array([-d[1], d[0]])
    c = np.array([x, z])
    head = [c + d * L_ * 0.5, c + d * L_ * 0.2 - n * Wd, c + d * L_ * 0.27, c + d * L_ * 0.2 + n * Wd]
    shaft = [c + d * L_ * 0.25 - n * 0.014, c - d * L_ * 0.5 - n * 0.014, c - d * L_ * 0.5 + n * 0.014,
             c + d * L_ * 0.25 + n * 0.014]
    fl = []
    for sgn in (-1, 1):
        fl.append([c - d * L_ * 0.28, c - d * L_ * 0.5 + n * sgn * Wd * 0.9, c - d * L_ * 0.55 + n * sgn * Wd * 0.9,
                   c - d * L_ * 0.5])
    w = lambda Q: np.array([S.to_world(q[0], q[1], y) for q in Q])
    return w(head), w(shaft), [w(f) for f in fl]


def draw_arrow(fr, cam, L, shade, x, z, ang, col, glow=0.0):
    head, shaft, fl = arrow_quads(x, z, ang)
    up = np.array([[0, 1.0, 0]])
    for f in fl:
        fr.poly(cam, f, np.clip(np.asarray(col) * 0.85 + 20, 0, 255))
    fr.poly(cam, shaft, shade(L, up, shaft.mean(0)[None], SHAFT, rimk=0.3)[0])
    c = shade(L, up, head.mean(0)[None], COPPER * 0.6 + np.asarray(col) * 0.4, rimk=0.5)[0]
    fr.poly(cam, head, np.clip(c + 25 + 60 * glow, 0, 255))


def trail_points(cam, pts, y=0.5):
    """screen points of an arrow's recent positions (oldest first)"""
    return cam.wp(np.array([S.to_world(q[0], q[1], y) for q in pts]))


# ---------------------------------------------------------------- STOPPED HORN (shield): a copper bell rim
def rim_polys(cam, cx, cz, y, half, r_in=R * 0.92, r_out=R * 1.58, n=40):
    """the rim ring split in the back half (half=0) and front half (half=1) as seen by the camera (painter order)"""
    a = np.linspace(0, 2 * math.pi, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    dz_o = cam.depth(oi)
    c = cam.depth(S.to_world(cx, cz, y)[None])[0]
    polys, ang = [], []
    for j in range(n):
        if ((dz_o[j] + dz_o[j + 1]) / 2 < c) == bool(half):
            polys.append([so[j], so[j + 1], si[j + 1], si[j]])
            ang.append((a[j] + a[j + 1]) / 2)
    return polys, ang


def draw_bellrim(arr, cam, cx, cz, t, k, half, col, y=R * 1.02):
    """k: 0..1 strength; a copper flare (lighter outside, dark throat inside) with a glint sweeping round"""
    polys, ang = rim_polys(cam, cx, cz, y, half)
    if not polys:
        return
    spin = t * 6.0
    base = COPPER * 0.7 + np.asarray(col) * 0.3
    for pts, a in zip(polys, ang):
        g = 0.5 + 0.5 * math.cos(a - spin)
        g2 = max(0.0, math.cos(a - spin)) ** 16
        c = COPPER_DK + (base - COPPER_DK) * g + np.array([255, 236, 210]) * 0.4 * g2
        m = mask_of([pts])
        arr += (np.clip(c, 0, 255)[None, None] - arr) * (m * 0.86 * k)[..., None]
    for rr, shade_k in ((1.0, 0.35), (1.5, 0.55)):                # the throat edge and the outer lip
        a = np.linspace(0, 2 * math.pi, 41)
        P = S.to_world(cx + R * rr * np.cos(a), cz + R * rr * np.sin(a), y + 0.005)
        s = cam.wp(P)
        dz = cam.depth(P)
        c = cam.depth(S.to_world(cx, cz, y)[None])[0]
        im = Image.new('L', (W, H), 0)
        d = ImageDraw.Draw(im)
        for j in range(40):
            if ((dz[j] + dz[j + 1]) / 2 < c) == bool(half):
                d.line([tuple(s[j]), tuple(s[j + 1])], fill=255)
        mk = np.asarray(im, np.float32) / 255.0
        tgt = COPPER_DK * 0.6 if rr < 1.2 else np.array([255, 214, 170])
        arr += (tgt[None, None] - arr) * (mk * shade_k * k)[..., None]


# ---------------------------------------------------------------- PEDAL TONE (slam): the giant bell
def bell_height(t, t0, t_slam):
    """the bell drops in from high above, hovers (bobbing with the held pedal note) during the telegraph, slams down in
    the last 0.12 s, sits on the canvas for a moment, then flies back up out of frame"""
    if t < t0:
        return None
    if t < t_slam - 0.12:
        u = min(1.0, (t - t0) / 0.3)
        return 3.2 + 5.0 * (1 - u) ** 2 + 0.08 * math.sin(2 * math.pi * 3.0 * (t - t0))
    if t < t_slam:
        u = (t - (t_slam - 0.12)) / 0.12
        return 3.2 - 3.1 * u * u
    if t < t_slam + 0.3:
        return 0.10
    if t < t_slam + 0.75:
        return 0.10 + 8.0 * ((t - t_slam - 0.3) / 0.45) ** 2
    return None


BELL_PROFILE = [(0.0, 1.0), (0.04, 0.93), (0.13, 0.74), (0.28, 0.54), (0.48, 0.40), (0.78, 0.30), (1.15, 0.24),
                (1.6, 0.21)]       # (height above the rim, radius / rim radius): a flared bell, mouth down


def draw_giant_bell(fr, cam, L, shade, cx, cz, rad, h, col, nseg=18):
    """mouth down: a flat-shaded low-poly surface of revolution (painter-sorted quads), a bright lip, and the leadpipe
    rising out of frame"""
    rim = rad * 1.05
    a = np.linspace(0, 2 * math.pi, nseg + 1)
    rings = [S.to_world(cx + rim * rr * np.cos(a), cz + rim * rr * np.sin(a), h + dy) for dy, rr in BELL_PROFILE]
    base = BRASS * 0.50 + np.asarray(col) * 0.08       # dark enough that the spot + bloom land it in gold, not white
    quads = []
    for j in range(len(rings) - 1):
        lo, hi = rings[j], rings[j + 1]
        for i in range(nseg):
            Q = np.array([lo[i], lo[i + 1], hi[i + 1], hi[i]])
            nrm = np.cross(Q[1] - Q[0], Q[3] - Q[0])
            nrm = nrm / (np.linalg.norm(nrm) + 1e-9)
            if nrm[1] < 0:
                nrm = -nrm
            if np.dot(nrm, cam.pos - Q.mean(0)) <= 0:
                continue
            quads.append((cam.depth(Q.mean(0)[None])[0], Q, nrm, j))
    quads.sort(key=lambda q: -q[0])
    for dz, Q, nrm, j in quads:
        c = shade(L, nrm[None], Q.mean(0)[None], base * (0.92 + 0.03 * j), rimk=0.6)[0]
        fr.poly(cam, Q, np.clip(c, 0, 255))
    s = cam.wp(rings[0])
    fr.draw.line([tuple(q) for q in s], fill=tuple(int(v) for v in BRASS_HI), width=1)
    top = S.to_world(cx, cz, h + BELL_PROFILE[-1][0])
    up = S.to_world(cx + 0.6, cz - 0.6, h + 6.0)
    pw = rim * BELL_PROFILE[-1][1] * 0.7
    side = np.array([pw, 0.0, pw]) * 0.7
    Q = np.array([top - side, top + side, up + side, up - side])
    fr.poly(cam, Q, np.clip(shade(L, np.array([[0.3, 0.6, -0.7]]), Q.mean(0)[None], base, rimk=0.6)[0], 0, 255))
