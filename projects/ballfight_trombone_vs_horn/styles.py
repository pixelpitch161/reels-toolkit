"""#34 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'slide'   the trombone's slide shot out like a lance: two brass tubes, braces, the crook (U-bend) at the tip
    style 'arrow'   a hunting-call arrow: copper head, pale shaft, fletching in the fighter's colour (+ a curving trail)
    'bellrim'       (v1-v4) the STOPPED HORN shield: a copper bell rim around the ball (the hand-stopped bell)
    'giant bell'    PEDAL TONE: a giant brass bell, mouth down, stomping the predicted spot (the slam)
    v5 (HORN v3)    ECHO CALL ghosts (draw_ghost), blast rings (ring_line, echo_band), the past path (path_line); the
                    STOPPED HORN hand (draw_hand: an open copper palm); FINAL FLICK's giant white glove on a copper spring
                    (draw_glove, glove_shadow_polys) and its wind-up lane (aim_lane). v4 (THE HUNT hounds) kept in
                    styles_v4_backup.py.

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
    base = BRASS * 0.42 + np.asarray(col) * 0.06       # dark enough that the spot, attack lights + bloom land it in gold
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


# ---------------------------------------------------------------- v5 HORN v3: ECHO CALL, the STOPPED HORN hand, FINAL FLICK
GHOST = np.array([255, 226, 196])
INK_ = np.array([34, 22, 34])


def draw_ghost(arr, s, rp, spr, col, alpha, flash=0.0):
    """an echo ghost: a translucent copper copy of the horn's ball (screen centre s, radius rp px) with the horn inside
    as a copper silhouette; `flash` (0..1) when it repeats its blast. Blends into arr (float, H x W x 3)"""
    if alpha <= 0.01:
        return
    ri = int(math.ceil(rp)) + 2
    x0, y0 = int(round(s[0])) - ri, int(round(s[1])) - ri
    xa, ya, xb, yb = max(0, x0), max(0, y0), min(W, x0 + 2 * ri + 1), min(H, y0 + 2 * ri + 1)
    if xb <= xa or yb <= ya:
        return
    yy, xx = np.mgrid[ya:yb, xa:xb].astype(np.float32)
    d = np.hypot(xx + 0.5 - s[0], yy + 0.5 - s[1]) / rp
    col = np.asarray(col, float)
    body = np.clip(1 - d, 0, 1) ** 0.35 * (d <= 1)
    rim = np.clip(1 - np.abs(d - 0.96) / 0.09, 0, 1)
    tint = col * 0.75 + GHOST * 0.25
    reg = arr[ya:yb, xa:xb]
    a_body = (0.42 + 0.25 * flash) * alpha * body
    reg += (tint[None, None] * (1.0 + 0.4 * flash) - reg) * np.clip(a_body, 0, 1)[..., None]
    a_rim = (0.85 + 0.15 * flash) * alpha * rim
    reg += (np.clip(col * 1.15 + 40 + 80 * flash, 0, 255)[None, None] - reg) * np.clip(a_rim, 0, 1)[..., None]
    if spr is not None:                                          # the horn inside, as a copper silhouette
        sn = spr.shape[0]
        sx0, sy0 = int(round(s[0])) - sn // 2, int(round(s[1])) - sn // 2
        ax0, ay0, ax1, ay1 = max(0, sx0), max(0, sy0), min(W, sx0 + sn), min(H, sy0 + sn)
        if ax1 > ax0 and ay1 > ay0:
            sub = spr[ay0 - sy0:ay1 - sy0, ax0 - sx0:ax1 - sx0]
            m = (sub[..., 3] > 0).astype(np.float32)
            lum = sub[..., :3].astype(np.float32).mean(-1, keepdims=True) / 255.0
            c = np.clip(col * (0.55 + 0.9 * lum) + 30 * flash, 0, 255)
            rg = arr[ay0:ay1, ax0:ax1]
            rg += (c - rg) * (m * 0.8 * alpha)[..., None]


def ring_line(arr, cam, cx, cz, r, y, col, alpha, width=1, n=48):
    """a blast ring at height y (ring frame), drawn as a projected circle"""
    if alpha <= 0.01 or r <= 0.02:
        return
    a = np.linspace(0, 2 * math.pi, n + 1)
    P = S.to_world(cx + r * np.cos(a), cz + r * np.sin(a), y)
    sp = cam.wp(P)
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=255, width=width)
    m = np.asarray(im, np.float32) / 255.0
    arr += (np.clip(np.asarray(col, float), 0, 255)[None, None] - arr) * (m * min(1.0, alpha))[..., None]


def path_line(arr, cam, pts, col, alpha):
    """the horn's past path on the canvas: points [(x, z, a)] (oldest first), a fading copper thread"""
    if len(pts) < 2:
        return
    P = np.array([S.to_world(x, z, 0.012) for x, z, _ in pts])
    sp = cam.wp(P)
    im = Image.new('L', (W, H), 0)
    dr = ImageDraw.Draw(im)
    for j in range(len(pts) - 1):
        v = int(255 * max(0.0, min(1.0, pts[j + 1][2])))
        if v > 3:
            dr.line([tuple(sp[j]), tuple(sp[j + 1])], fill=v, width=2)
    m = np.asarray(im, np.float32) / 255.0
    arr += (np.clip(np.asarray(col, float), 0, 255)[None, None] - arr) * (m * alpha)[..., None]


# the STOPPED HORN hand: an open copper palm (screen space, x right, y down, 1 unit = the ball's radius), fingers up:
# "talk to the hand", pressed into the bell
HAND_PALM = [(-0.44, -0.10), (-0.40, 0.42), (-0.16, 0.62), (0.22, 0.60), (0.42, 0.36), (0.44, -0.12)]
HAND_FINGERS = [(-0.33, -0.12, 0.17, 0.62), (-0.11, -0.12, 0.18, 0.78), (0.11, -0.12, 0.18, 0.72), (0.32, -0.10, 0.15, 0.52)]
HAND_THUMB = [(-0.42, 0.34), (-0.70, 0.06), (-0.80, -0.08), (-0.68, -0.14), (-0.44, 0.08)]


def _finger(cx, base_y, w, ln, lean=0.0):
    """a finger as a rounded column (screen space, up = -y), leaning `lean` x its length sideways at the tip"""
    pts = []
    for a in np.linspace(0, math.pi, 7):
        pts.append((cx + lean * ln - w / 2 * math.cos(a), base_y - ln - w / 2 * math.sin(a) + w / 2))
    return [(cx - w / 2, base_y)] + pts + [(cx + w / 2, base_y)]


def draw_hand(arr, s, rp, col, k, flash=0.0, side=1.0):
    """STOPPED HORN: an open copper palm at screen centre s (size from the ball's radius rp px), alpha k; `flash` on a
    block; `side` (+-1) mirrors it (thumb toward the ball)"""
    if k <= 0.01:
        return
    sc = rp * 1.05
    col = np.asarray(col, float)
    base = COPPER * 0.65 + col * 0.35
    parts = []
    tf = lambda pts: [(s[0] + side * x * sc, s[1] + y * sc) for x, y in pts]
    parts.append((tf(HAND_THUMB), base * 0.86))
    for j, (cx, by, w, ln) in enumerate(HAND_FINGERS):
        lean = (j - 1.5) * 0.06
        parts.append((tf(_finger(cx, by, w, ln, lean)), base * (1.04 if j % 2 else 0.95)))
    parts.append((tf(HAND_PALM), base * 1.0))
    im = Image.new('RGB', (W, H), (0, 0, 0))
    mk = Image.new('L', (W, H), 0)
    dr, dm = ImageDraw.Draw(im), ImageDraw.Draw(mk)
    for pts, c in parts:
        cc = tuple(int(v) for v in np.clip(c * (1 + 0.5 * flash) + 60 * flash, 0, 255))
        dr.polygon(pts, fill=cc, outline=tuple(int(v) for v in INK_))
        dm.polygon(pts, fill=255, outline=255)
    hi = tuple(int(v) for v in np.clip(base * 1.5 + 60, 0, 255))        # a metal glint down the palm and the fingers
    pal = tf([(-0.20, 0.42), (-0.24, 0.05)])
    dr.line(pal, fill=hi, width=1)
    for cx, by, w, ln in HAND_FINGERS[1:3]:
        dr.line(tf([(cx - w * 0.2, by - 0.1), (cx - w * 0.2, by - ln * 0.8)]), fill=hi, width=1)
    rgb = np.asarray(im, np.float32)
    m = np.asarray(mk, np.float32)[..., None] / 255.0 * k
    arr += (rgb - arr) * m


# FINAL FLICK: a giant white cartoon glove on a copper spring (jack-in-the-box), flat on the plane of the balls' centres,
# local frame u forward (toward the enemy), v left; origin = the middle of the back of the hand; 1 unit = 1 ring unit
GLOVE_CUFF = [(-0.66, -0.17), (-0.62, -0.24), (-0.44, -0.22), (-0.40, -0.15), (-0.40, 0.15), (-0.44, 0.22), (-0.62, 0.24),
              (-0.66, 0.17)]
GLOVE_BACK = [(-0.42, -0.17), (-0.33, -0.26), (0.04, -0.27), (0.13, -0.20), (0.13, 0.20), (0.04, 0.27), (-0.33, 0.26),
              (-0.42, 0.17)]
GLOVE_KNUCKLES = [(-0.19, 0.10), (-0.065, 0.12), (0.065, 0.12), (0.19, 0.11)]      # (v, width): pinky .. index
FLICKER = 1                                                      # the middle finger (index into GLOVE_KNUCKLES)
WHITE = np.array([248, 246, 240])


def _bump(u0, v, w, ln, n=7):
    """a finger seen from above: a rounded strip from u0 forward by ln (width w)"""
    pts = [(u0, v - w / 2)]
    for a in np.linspace(-math.pi / 2, math.pi / 2, n):
        pts.append((u0 + ln - w / 2 + w / 2 * math.cos(a), v + w / 2 * math.sin(a)))
    pts.append((u0, v + w / 2))
    return pts


def glove_parts(pose):
    """[(local pts, shade, kind)] back to front. pose 0 = cocked (the middle finger curled under the thumb), 1 = flicked
    (the middle finger straight out)"""
    out = [(GLOVE_CUFF, 0.86, 'cuff')]
    for j, (v, w) in enumerate(GLOVE_KNUCKLES):
        if j == FLICKER and pose > 0.5:
            out.append((_bump(0.02, v, w * 0.95, 0.62), 0.97, 'finger'))
        else:
            out.append((_bump(0.02, v, w * 1.05, 0.21 if j != FLICKER else 0.18), 0.92, 'knuckle'))
    out.append((GLOVE_BACK, 1.0, 'back'))
    if pose > 0.5:                                               # the thumb relaxed at the side
        out.append(([(-0.20, 0.22), (-0.02, 0.34), (0.08, 0.40), (0.14, 0.36), (0.06, 0.26), (-0.10, 0.16)], 0.9, 'thumb'))
    else:                                                        # the thumb curled forward over the middle finger
        out.append(([(-0.20, 0.22), (0.02, 0.24), (0.20, 0.10), (0.26, 0.00), (0.20, -0.06), (0.08, 0.02), (-0.10, 0.12)],
                    0.9, 'thumb'))
    return out


def glove_world(p, aim, d, scale, pose, wob=0.0, y=R):
    """world polygons [(Q (n, 3), shade, kind)] of the glove: origin at the ball p + aim * d, scaled, wobbling by `wob` rad"""
    a = aim + wob
    du = np.array([math.cos(a), math.sin(a)])
    dv = np.array([-du[1], du[0]])
    o = p + np.array([math.cos(aim), math.sin(aim)]) * d
    out = []
    for pts, sh, kind in glove_parts(pose):
        Q = np.array([S.to_world(*(o + du * u * scale + dv * v * scale), y) for u, v in pts])
        out.append((Q, sh, kind))
    stitches = []
    for v in (-0.10, 0.0, 0.10):                                 # the three stitch lines on the back of a cartoon glove
        stitches.append(np.array([S.to_world(*(o + du * u * scale + dv * v * scale), y) for u in (-0.30, -0.04)]))
    cuff_back = S.to_world(*(o + du * (-0.66) * scale), y)
    return out, stitches, cuff_back


def draw_glove(arr, cam, p, aim, d, scale, pose, col, k=1.0, wob=0.0, light=1.0, y=R):
    """the glove + its copper spring from the ball's surface to the cuff; copper-lit (the horn's colour on the edges)"""
    if k <= 0.01 or scale <= 0.02:
        return
    col = np.asarray(col, float)
    parts, stitches, cuff_back = glove_world(p, aim, d, scale, pose, wob, y)
    im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
    dr = ImageDraw.Draw(im)
    u = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-u[1], u[0]])
    s0 = R * 0.85
    s1 = d - 0.66 * scale
    if s1 > s0 + 0.02:                                           # the spring: a copper zig-zag coil
        ncoil = 9
        pts = []
        for j in range(2 * ncoil + 1):
            q = p + u * (s0 + (s1 - s0) * j / (2 * ncoil)) + nrm * (0.11 * scale * (1 if j % 2 else -1) if 0 < j < 2 * ncoil else 0)
            pts.append(S.to_world(q[0], q[1], y))
        sp = cam.wp(np.array(pts))
        cdk = tuple(int(v) for v in COPPER_DK)
        chi = tuple(int(v) for v in np.clip(COPPER * 1.25 + 30, 0, 255))
        dr.line([tuple(q) for q in sp], fill=cdk, width=4)
        dr.line([tuple(q) for q in sp], fill=chi, width=2)
    tint = WHITE * 0.86 + col * 0.14
    for Q, sh, kind in parts:
        sp = cam.wp(Q)
        c = np.clip(tint * sh * light, 0, 255)
        dr.polygon([tuple(q) for q in sp], fill=tuple(int(v) for v in c), outline=tuple(int(v) for v in INK_))
    for St in stitches:
        sp = cam.wp(St)
        dr.line([tuple(q) for q in sp], fill=(176, 160, 160), width=1)
    rim = tuple(int(v) for v in np.clip(col * 1.1 + 40, 0, 255))   # copper light on the edge facing the horn
    back = parts[-2][0] if parts[-1][2] == 'thumb' else parts[-1][0]
    sp = cam.wp(back)
    dr.line([tuple(q) for q in sp[:3]], fill=rim, width=1)
    new = np.asarray(im, np.float32)
    arr += (new - arr) * k


def glove_shadow_polys(p, aim, d, scale, pose, wob=0.0, y=R):
    parts, _, _ = glove_world(p, aim, d, scale, pose, wob, y)
    return [Q for Q, sh, kind in parts]


def echo_band(Ic, sxp, szp, cx, cz, r, width, k, col, bayer=None):
    """a blast ring's light on the canvas"""
    rd = np.hypot(sxp - cx, szp - cz)
    band = np.clip(1 - np.abs(rd - r) / width, 0, 1)
    if bayer is not None:
        band = np.floor(band * 3 + bayer * 0.9) / 3
    Ic += (band * k)[:, None] * (np.asarray(col) / 255.0 * 1.3 + 0.12)[None]


def aim_lane(Ic, sxp, szp, p, aim, length, k, col, bayer=None, w=0.26):
    """FINAL FLICK's wind-up: the line the glove will flick along, lit on the canvas"""
    d_ = np.array([math.cos(aim), math.sin(aim)])
    rx, rz = sxp - p[0], szp - p[1]
    along = rx * d_[0] + rz * d_[1]
    across = np.abs(-rx * d_[1] + rz * d_[0])
    lane = np.clip(1 - across / w, 0, 1) * (along > R) * (along < R + length)
    if bayer is not None:
        lane = np.floor(lane * 3 + bayer * 0.9) / 3
    Ic += (lane * k)[:, None] * (np.asarray(col) / 255.0 * 0.9 + 0.15)[None]
