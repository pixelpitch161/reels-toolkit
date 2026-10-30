"""HORN v3 (copper), as built in #34 v5 (user, Oct 25): ECHO CALL (a round blast ring on every beat; up to 4 copper echo
ghosts sit on the horn's own past path, one beat apart, each repeating the call a beat after it was sounded: a delay in
space; +1 ghost per landed blast), STOPPED HORN (a copper hand plugs the bell: shield), HORN RIP (haste: the echoes come
back on the 8ths), FINAL FLICK (below 25 HP: a giant cartoon glove bursts out of the bell on a copper spring, cocks for
a beat, then flicks the enemy across the ring into the ropes). Engine kinds `echo` / `flick` (fight.py, #34 v5).

Look: the echo ghosts (translucent copper copies of its ball, newest brightest, flashing as they repeat), a fading copper
thread along its path, the call / echo rings (canvas light + a raised ring + heat shimmer), the ghosts' copper glow on the canvas and
their aura, STOPPED HORN's open copper palm pressed into the bell (+ a dome of light), FINAL FLICK's lit lane, the glove
(with a real shadow) and the flung enemy's afterimages, copper embers, brass tubing / valve / bell-chip shards.
Sound (role 'melody'): every call is a round, mellow horn note of its line (config LINES['HORN'], one per beat); every
echo repeats the call it trails, quieter and darker each time, panned toward its ghost; a landed blast is a brighter stab
up the chord; the leader gets a horn a sixth below. STOPPED HORN is the buzzing stopped tone (config LINES['HORN THEME']
if given, else the line), HORN RIP rips up the harmonic series of the bar's root, FINAL FLICK a horn fanfare over a
slide-whistle wind-up and a spring "boing", then a THWACK and a heavy rope hit (the bed's `boom`).

Draw code from projects/ballfight_trombone_vs_horn/styles.py (#34 v5), copied here unchanged; the render layers from
#34's render.py and the sound from #34's audio.py (key-bound notes follow the config's KEY: D minor = #34's notes).
Check config: examples/trombone_vs_horn (#34 rebuilt)."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss, ease
from reelkit.shadows import proj_point
from reelkit import fx
import scene as S

from . import base

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


# ===================================================================================================== look (#34 render.py)
AGE_A = (0.90, 0.70, 0.52, 0.36)                   # ghost strength by slot: the newest (one beat back) brightest
G_SCALE = 1.45                                     # the giant glove: ~1.9 ring units cuff to fingertip (a ball: 1.2)


def _last(arr_, t):
    if arr_ is None or not len(arr_):
        return None
    j = np.searchsorted(arr_, t, 'right') - 1
    return float(arr_[j]) if j >= 0 else None


def _back_ease(u):
    c1 = 1.70158
    return 1 + (c1 + 1) * (u - 1) ** 3 + c1 * (u - 1) ** 2


class Look(base.Look):
    shards = ['tube'] * 6 + ['valve'] * 3 + ['bellbit'] * 6
    spark_big = base.Look.spark_big + ('flick', 'flick_rope')
    flash_hits = base.Look.flash_hits + ('flick', 'flick_rope')
    fringe_hits = base.Look.fringe_hits + ('flick', 'flick_rope')
    block_burst = (255, 220, 190)                  # the lance bangs off the copper hand

    def setup(self):
        P, k = self.P, self.k
        RUN, EV = P.RUN, P.EV
        self.echo = self.cfg['weapon']['kind'] == 'echo'
        self.flicker = self.cfg['abilities']['low']['kind'] == 'flick'
        self.G = RUN.get('ECHO_G')                 # (frames, 2, 4, (x, z, lag)): every ghost slot on its own past path
        self.N = RUN.get('ECHO_N')                 # (frames, 2): ghosts earned so far
        self.RINGS = RUN.get('ECHO_R')             # per frame: the live blast rings (owner, x, z, r, rmax, j, t0, call, hit)
        self.FLICK = RUN.get('FLICK')              # (frames, 2, (state, aim, ext, t0, t_hit, hit))
        ev = [(e[0], e[3], e[4], e[5], e[6], e[7]) for e in EV if e[1] == 'echo' and e[2] == k]
        self.REP_T = {j: np.array([t_ for t_, jj, c, x, z, rm in ev if jj == j]) for j in range(5)}
        self.GROW_T = {e[3]: e[0] for e in EV if e[1] == 'echo_grow' and e[2] == k}    # ghost n -> earned at
        self.BLOCK_T = np.array([e[0] for e in EV if e[1] == 'block' and e[2] == k])
        self.FLUNG = [(e[0], e[3]) for e in EV if e[1] == 'hit' and e[7] == 'flick' and e[2] == k]   # (t, victim)

    # ---------------------------------------------------------------- the horn's own state
    def ghosts_at(self, t):
        """its echo ghosts at t: [(slot, x, z, alpha, flash)]. In the fight: the run's ghosts on its own past path (frozen
        through the hit-stop); each fades in when it is earned and flashes when it repeats a blast. After the K.O. a
        winner's ghosts fade out, a loser's burst into embers: frame 0 has none."""
        P, k, T = self.P, self.k, self.T
        if self.G is None or not self.echo or t < T.T_FIGHT:
            return []
        tc = P.fx_clock(t)
        i = P.sim_i(min(t, P.T_KO))
        n = int(self.N[i, k] + 1e-9)
        out_k = 1.0
        if t >= P.T_HS:
            out_k = 0.0 if k == P.LOSE else 1 - ss(P.T_HS, P.T_HS + 0.8, t)
            if out_k <= 0:
                return []
        out = []
        for j in range(n):
            x, z = float(self.G[i, k, j, 0]), float(self.G[i, k, j, 1])
            tg = self.GROW_T.get(j + 1, T.T_FIGHT)
            a = AGE_A[j] * ss(tg, tg + 0.35, tc) * out_k
            te = _last(self.REP_T[j + 1], tc)
            fl = math.exp(-(tc - te) / 0.14) if te is not None else 0.0
            out.append((j, x, z, a, fl))
        return out

    def rings_at(self, t):
        """its live blast rings at t: [(owner, x, z, r, j, alpha)] (j = 0: the call, 1..4: the echoes), lingering as
        they fade"""
        P, T = self.P, self.T
        if self.RINGS is None or not (T.T_FIGHT <= t < P.T_HS):
            return []
        i = P.sim_i(min(t, P.T_KO))
        tc = P.fx_clock(t)
        out = []
        for owner, x, z, r, rmax, j, t0, call, hit in self.RINGS[i]:
            if int(owner) != self.k:
                continue
            w = self.cfg['weapon']
            r0 = R if j == 0 else R * 0.5
            te = t0 + (rmax - r0) / w['speed']
            rr = min(rmax, r0 + max(0.0, tc - t0) * w['speed'])
            a = 1.0 if tc < te else max(0.0, 1 - (tc - te) / w.get('linger', 0.25))
            out.append((int(owner), float(x), float(z), rr, int(j), a))
        return out

    def glove_at(self, t):
        """FINAL FLICK's glove at t: dict(p, aim, d, scale, pose, wob, light) or None. It bursts out of the bell
        (overshoot), creeps back with a growing tremble while it is cocked (one beat), snaps out to the enemy's surface in
        a frame, holds, then shrinks back into the bell"""
        P, k, T = self.P, self.k, self.T
        if self.FLICK is None or not self.flicker or not (T.T_FIGHT <= t < T.T_GLIDE):
            return None
        i = P.sim_i(min(t, P.T_KO))
        st_, aim, ext, t0, th, hit = (float(v) for v in self.FLICK[i, k])
        if math.isnan(st_):
            return None
        tc = P.fx_clock(t)
        p = P.fighter(k, t)['p']
        d_cock = R + 0.62 * G_SCALE
        if tc < th:
            tau = max(0.0, tc - t0)
            f = _back_ease(min(1.0, tau / 0.16))
            uw = min(1.0, tau / max(1e-3, th - t0))
            d = R * 0.7 + (d_cock + 0.14 * (1 - uw) - R * 0.7) * f
            wob = 0.09 * math.sin(2 * math.pi * 11 * tau) * uw ** 2
            return dict(p=p, aim=aim, d=d, scale=G_SCALE * max(0.0, f), pose=0.0, wob=wob, light=1.0 + 0.25 * uw)
        tau = tc - th
        full = max(R + 0.55, R + ext - 0.62 * G_SCALE)
        if tau < 0.034:
            d, sc, pose = d_cock + (full - d_cock) * ease(tau / 0.034), G_SCALE, 1.0
        elif tau < 0.2:
            d, sc, pose = full, G_SCALE, 1.0
        else:
            u = min(1.0, (tau - 0.2) / 0.38)
            if u >= 1.0:
                return None
            d, sc, pose = full + (R * 0.7 - full) * ease(u), G_SCALE * (1 - ease(u)), 1.0
        return dict(p=p, aim=aim, d=d, scale=sc, pose=pose, wob=0.0, light=1.0 + 0.6 * math.exp(-tau / 0.08) * hit)

    def hand_k(self, t, st):
        """STOPPED HORN: the copper hand's strength (fades in / out with the shield) and its flash on a block"""
        if st['shield'] <= 0:
            return 0.0, 0.0
        P = self.P
        dur = self.cfg['abilities']['unlock1']['dur_beats'] * self.T.BEAT
        a = min(1.0, st['shield'] / 0.2, (dur - st['shield']) / 0.1 + 0.15)
        tb = _last(self.BLOCK_T, P.fx_clock(t))
        return a, (math.exp(-(P.fx_clock(t) - tb) / 0.12) if tb is not None else 0.0)

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('echo', 'flick', 'flick_rope'):
            return {'echo': 0.35, 'flick': 2.5, 'flick_rope': 2.2}[e[7]]
        return super().shake(e)

    def hit_glow(self, e):
        if e[7] in ('flick', 'flick_rope', 'slam', 'wave', 'shockwave'):
            return 14, 0.35
        return None

    def fx_event(self, e):
        P = self.P
        up = P.up
        t_, kind = e[0], e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'echo':                                      # a blast lands: copper embers, a soft scorch ring
                P.BU.add(t_, pos, P.COL[att], 0.5 if e[8] == 0 else 0.35, bright=False)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.1, peak=0.45, decay=0.2)
                P.PA.emit(t_, pos, 'embers', n=6 if e[8] == 0 else 4)
            elif how == 'flick':                                   # FINAL FLICK lands: the big one
                P.BU.add(t_, pos, P.COL[att], 2.2)
                P.FL.add(t_, pos, (255, 236, 214), radius=2.4, peak=1.1, decay=0.45)
                P.PA.emit(t_, pos, 'embers', n=22, spread=180, speed=(0.8, 2.4))
                P.PA.emit(t_, pos, 'brass', n=10, spread=120)
            elif how == 'flick_rope':                              # ... and the enemy slams into the ropes
                P.BU.add(t_, pos, P.COL[att], 1.8)
                P.FL.add(t_, pos, P.FL_COL[att], radius=2.0, peak=0.9, decay=0.4)
                P.PA.emit(t_, up(x, z, 0.1), 'dust', n=18, spread=160, speed=(0.6, 1.6))
                P.PA.emit(t_, pos, 'brass', n=8, spread=140)
        elif kind == 'echo':                                       # a blast: the call (j = 0) or a ghost's repeat
            j = e[3]
            pos = up(e[5], e[6], 0.5)
            P.FL.add(t_, pos, P.FL_COL[e[2]], radius=1.3 if j == 0 else 1.0, peak=0.30 if j == 0 else 0.16 * 0.85 ** j,
                     decay=0.2)
            if j > 0:
                P.PA.emit(t_, up(e[5], e[6], 0.35), 'embers', n=2, speed=(0.3, 0.9), life=(0.3, 0.6))
        elif kind == 'echo_grow':                                  # a new ghost peels off the horn
            p_ = P.fighter(e[2], t_)['p']
            P.PA.emit(t_, up(p_[0], p_[1], 0.5), 'embers', n=8, spread=180, speed=(0.5, 1.4))
            P.FL.add(t_, up(p_[0], p_[1], 0.6), P.FL_COL[e[2]], radius=1.2, peak=0.35, decay=0.3)
        elif kind == 'flick_wind':                                 # the glove bursts out of the bell
            p_ = P.fighter(e[2], t_)['p']
            pos = up(p_[0] + 0.9 * math.cos(e[4]), p_[1] + 0.9 * math.sin(e[4]), R)
            P.PA.emit(t_, pos, 'embers', n=10, spread=120, speed=(0.6, 1.6))
            P.FL.add(t_, pos, (255, 236, 214), radius=1.6, peak=0.55, decay=0.35)
        elif kind == 'block':                                      # (the kit added the burst: block_burst)
            P.PA.emit(t_, up(e[4], e[5], R), 'embers', n=5)

    def fx_after(self):
        P = self.P
        if self.k == P.LOSE:                                       # its ghosts burst at the end of the hit-stop
            for j, x, z, a, fl in self.ghosts_at(P.T_HS - 1e-3):
                P.PA.emit(P.fx_clock(P.T_HS), P.up(x, z, R), 'embers', n=10, spread=180, speed=(0.8, 2.0))

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'embers', n=30, spread=180, speed=(0.8, 2.2))

    # ---------------------------------------------------------------- canvas
    def shadows_free(self, F, spot):
        g = self.glove_at(F.t)                                     # the glove casts a real shadow
        if g is None or g['scale'] <= 0.05:
            return []
        return [proj_point(Q, spot) for Q in glove_shadow_polys(g['p'], g['aim'], g['d'], g['scale'], g['pose'],
                                                                 g['wob'])]

    def canvas(self, Ic, C):
        P, k = self.P, self.k
        bay = BAYER_FULL[C.ys, C.xs]
        for owner, x, z, r, j, a in self.rings_at(C.t):           # the call / echo rings light the canvas
            echo_band(Ic, C.sxp, C.szp, x, z, r, 0.11 if j == 0 else 0.06,
                      a * (0.95 if j == 0 else 0.6 * 0.85 ** (j - 1)), P.COL[owner], bay)
        for j, gx, gz, ga, gf in self.ghosts_at(C.t):             # ghosts cast no shadow: a copper glow instead
            rd = np.hypot(C.sxp - gx, C.szp - gz)
            Ic += (np.clip(1 - rd / 0.75, 0, 1) ** 2 * (0.22 + 0.35 * gf) * ga)[:, None] * (P.COL[k] / 255.0 + 0.1)[None]
        g = self.glove_at(C.t)                                     # FINAL FLICK: the lane the glove will flick along
        if g is not None and g['pose'] < 0.5:
            pk = 0.25 + 0.2 * (0.5 + 0.5 * math.sin(C.t * 2 * math.pi * 8))
            aim_lane(Ic, C.sxp, C.szp, g['p'], g['aim'], self.cfg['abilities']['low']['reach'] + R, pk * g['scale'],
                     P.COL[k], bay)
        return Ic

    def canvas_overlay(self, F, arr):
        """a fading copper thread along its path, back to its oldest ghost"""
        P, k = self.P, self.k
        if F.in_fight and self.G is not None and self.echo:
            i = F.i
            n_ = int(self.N[i, k] + 1e-9)
            if n_ > 0:
                lag = float(self.G[i, k, n_ - 1, 2]) + 0.1
                m_ = int(lag * FPS)
                pts = []
                for back in range(m_, -1, -1):
                    q = P.RUN['P'][max(0, i - back), k]
                    pts.append((float(q[0]), float(q[1]), 0.55 * (1 - back / max(1, m_))))
                path_line(arr, F.cam, pts, np.clip(P.COL[k] * 0.9 + 30, 0, 255), 0.6)
        return arr

    # ---------------------------------------------------------------- the painter pass
    def items_ball(self, F, st):
        if st['shield'] > 0 and not st['shattered']:              # STOPPED HORN: just in front of its ball
            pw = S.to_world(st['p'][0], st['p'][1], R)
            return [(F.cam.depth(pw)[0] - 0.05, ('hand',))]
        return []

    def items_free(self, F):
        P, k, cam, t = self.P, self.k, F.cam, F.t
        items = []
        if self.rings_at(t):                                       # the blast rings, raised: first in the painter pass
            items.append((1e9, ('rings',)))                        # (after the back ropes, before posts + balls: #34)
        for gh in self.ghosts_at(t):                               # the echo ghosts
            items.append((cam.depth(S.to_world(gh[1], gh[2], R))[0] + 0.02, ('ghost',) + gh))
        g = self.glove_at(t)                                       # FINAL FLICK's glove
        if g is not None and g['scale'] > 0.02:
            c_ = g['p'] + np.array([math.cos(g['aim']), math.sin(g['aim'])]) * g['d']
            items.append((cam.depth(S.to_world(c_[0], c_[1], R))[0] - 0.02, ('glove', g)))
        tcf = P.fx_clock(t)
        for tf, vic in self.FLUNG:                                 # the flung enemy's afterimages
            if 0.0 <= tcf - tf < 0.5 and F.fs[vic]['vis']:
                for m_, a_ in ((1, 0.34), (2, 0.22), (3, 0.12)):
                    q = P.fighter(vic, max(tf, min(t, P.T_KO) - 0.035 * m_))['p']
                    items.append((cam.depth(S.to_world(q[0], q[1], R))[0] + 0.05 * m_,
                                  ('after', vic, q, a_ * (1 - (tcf - tf) / 0.5))))
        return items

    def draw_item(self, F, payload):
        P, k, fr, cam, t = self.P, self.k, F.fr, F.cam, F.t
        kind = payload[0]
        if kind == 'rings':                                        # a burst of sound: the call bright, echoes fainter
            arr, depth = fr.array(), fr.depth()
            for owner, x, z, r, j, a in self.rings_at(t):
                if j == 0:
                    ring_line(arr, cam, x, z, r, 0.32, np.clip(P.COL[owner] * 0.7 + np.array([255, 214, 150]) * 0.45, 0, 255),
                              0.9 * a, width=2)
                else:
                    ring_line(arr, cam, x, z, r, 0.32, np.clip(P.COL[owner] * 1.05 + 20, 0, 255), 0.75 * 0.85 ** (j - 1) * a)
            fr.set_array(arr, depth)
        elif kind == 'hand':                                         # STOPPED HORN: the copper palm plugs the bell
            st = F.fs[k]
            hk, hf = self.hand_k(t, st)
            pw = S.to_world(st['p'][0], st['p'][1], R)            # (its ball's screen spot, as draw_ball found it)
            Pc = cam.to_cam(pw[None])[0]
            s_ = cam.proj(Pc[None])[0]
            rp = R * cam.f / Pc[2] * st['scale']
            q = F.fs[1 - k]['p']
            so = cam.wp(S.to_world(q[0], q[1], R)[None])[0]
            dv = np.array([so[0] - s_[0], so[1] - s_[1]])
            dv = dv / (np.linalg.norm(dv) + 1e-9)
            arr, depth = fr.array(), fr.depth()
            yy_, xx_ = np.mgrid[0:H, 0:W]
            dome = np.clip(1 - np.abs(np.hypot(xx_ - s_[0], yy_ - s_[1]) - rp * 1.22) / 1.2, 0, 1)
            arr += (np.clip(P.COL[k] * 1.1 + 50, 0, 255)[None, None] - arr) * (dome * 0.55 * hk)[..., None]
            c_ = (s_[0] + dv[0] * rp * 0.72, s_[1] + dv[1] * rp * 0.72 + rp * 0.1)
            draw_hand(arr, c_, rp * (1.2 + 0.1 * hf), P.COL[k], hk, hf, 1.0 if dv[0] >= 0 else -1.0)
            fr.set_array(arr, depth)
        elif kind == 'ghost':
            _, j, gx, gz, ga, gf = payload
            pw = S.to_world(gx, gz, R)
            Pc = cam.to_cam(pw[None])[0]
            s_ = cam.proj(Pc[None])[0]
            rp = R * cam.f / Pc[2]
            yc, sw, roll = self.cfg['view']
            spr = P.sprite(k, rp * 0.80, yc + sw * math.sin(1.25 * gx + 0.45 * gz), roll)
            arr, depth = fr.array(), fr.depth()
            draw_ghost(arr, s_, rp, spr, P.COL[k], ga, gf)
            fr.set_array(arr, depth)
        elif kind == 'glove':
            g = payload[1]
            arr, depth = fr.array(), fr.depth()
            draw_glove(arr, cam, g['p'], g['aim'], g['d'], g['scale'], g['pose'], P.COL[k], 1.0, g['wob'], g['light'])
            fr.set_array(arr, depth)
        elif kind == 'after':
            _, vic, q, a_ = payload
            pw = S.to_world(q[0], q[1], R)
            Pc = cam.to_cam(pw[None])[0]
            arr, depth = fr.array(), fr.depth()
            draw_ghost(arr, cam.proj(Pc[None])[0], R * cam.f / Pc[2], None, P.COL[vic], a_, 0.3)
            fr.set_array(arr, depth)

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'tube':                                         # a curl of brass tubing
            pts = [np.array([sx_, sz_]) + 0.16 * np.array([math.cos(rot + j * 0.5 * (1 + curl)),
                                                           math.sin(rot + j * 0.5 * (1 + curl))]) for j in range(7)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in BRASS), width=2)
            return np.asarray(im).astype(np.float32)
        # valve (a rotary valve: a squat disc), bellbit (a curved chip of the copper bell)
        col = {'valve': BRASS_DK * 1.3, 'bellbit': COPPER}[kind]
        sz = size * (1.5 if kind == 'bellbit' else 0.8)
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, glows
    def shimmer_after(self, img, cam, t):
        for owner, x, z, r, j, a in self.rings_at(t):              # the echo rings shimmer while they expand
            if a >= 1.0:
                img = fx.shimmer_world(img, cam, S.to_world(x, z, 0.05), r, 0.24 if j == 0 else 0.16,
                                       2.2 if j == 0 else 1.4, t, k=0.9 * 0.85 ** j, seed=j + 3 * owner)
        return img

    def glows(self, F, drawn):
        for j, gx, gz, ga, gf in self.ghosts_at(F.t):             # a copper aura on every ghost, brighter as it echoes
            s_ = F.cam.wp(S.to_world(gx, gz, R)[None])[0]
            F.fr.glow(s_[0], s_[1], 12, tuple(self.col * 0.7 + 60), (0.10 + 0.28 * gf) * ga)

    def focus_points(self, t, st):
        out = []
        for j, gx, gz, ga, gf in self.ghosts_at(t):               # the echo ghosts (ball-sized)
            for dy in (0.0, 2 * R):
                out.append((S.to_world(gx, gz, dy), 'echo ghost'))
        g = self.glove_at(t)                                       # the glove (cuff and fingertip)
        if g is not None:
            u = np.array([math.cos(g['aim']), math.sin(g['aim'])])
            for d in (g['d'] - 0.66 * g['scale'], g['d'] + 0.62 * g['scale']):
                q = g['p'] + u * d
                out.append((S.to_world(q[0], q[1], R), 'glove'))
        return out


# ===================================================================================================== sound (#34 audio.py)
def _h01_34(*xs):
    """#34's deterministic hash in [0, 1) (its calls thin as #34 thinned them)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


def snap(A, SR, seed=0):
    """a finger snap / the glove's flick: a dry click with a little low body"""
    n = int(0.06 * SR)
    t = A.tt(n)
    x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 900, 4500) * np.exp(-t / 0.008)
    return x + 0.5 * np.sin(2 * np.pi * 180 * t) * np.exp(-t / 0.015)


def slide_whistle(A, SR, dur, f0=520.0, f1=2100.0):
    """FINAL FLICK's wind-up: a slide whistle rising over the beat (cartoon tension)"""
    n = int(dur * SR)
    t = A.tt(n)
    f = f0 * (f1 / f0) ** (t / dur) * (1 + 0.012 * np.sin(2 * np.pi * 6 * t))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.15 * A.bp(np.random.default_rng(5).normal(0, 1, n), 1500, 6000)
    return x * np.clip(t / 0.04, 0, 1) * np.clip((dur - t) / 0.02, 0, 1)


def boing(A, SR, seed=0, f0=190.0, dur=0.45):
    """a spring: the glove bursting out of the bell (a wobbling, falling twang)"""
    n = int(dur * SR)
    t = A.tt(n)
    f = f0 * (1 + 0.35 * np.exp(-t / 0.12) * np.sin(2 * np.pi * 18 * t)) * (1 - 0.25 * t / dur)
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.3 * np.sin(2 * np.pi * np.cumsum(f * 2.01) / SR)
    return x * np.exp(-t / 0.16) + 0.25 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 7000) * np.exp(-t / 0.01)


def stopped(A, x):
    """stopped horn: the hand closes the bell: thin, buzzing, brassy (cuivre)"""
    y = A.hp(x, 500)
    y = np.tanh(3.5 * y / (np.abs(y).max() + 1e-9)) * 0.5
    return y + 0.6 * A.bp(y, 1800, 4200)


class Sound(base.Sound):
    bus_gain, send = 1.25, 0.55                                    # #34's horn bus
    band = (250, 2500)                                             # the calls D4-D5 + their horn harmonics
    colour_args = dict(cut0=70, cut_span=420, drive=4.5)           # #34's colour()

    def horn(self, m, dur=0.3, vel=0.8, cap=None):
        return self.M.note('HORN', m, round(dur, 3), cap or round(dur + 0.35, 2), round(vel, 1))

    def tonic(self, lo=60):
        """the key's tonic at or above midi lo (D -> D4 = 62: #34's calls)"""
        return lo + (self.T.KEY_PC - lo) % 12

    def theme_at(self, t):
        """STOPPED HORN's note: config LINES['HORN THEME'] (#34's HORN_LINE, a note per 2 beats) if given, else the line"""
        T = self.T
        th = T.LINES.get('HORN THEME')
        if not th:
            return T.line_at(self.name, t)
        return th[T.loop_index(t, len(th) // len(T.LOOP))]

    def _hit(self, mix, t, taiko_g, seed, braam_dur=None, braam_g=0.0):
        """a heavy hit from the bed: the trailer's taiko (+ braam), or another mood's boom"""
        bed = self.M.bed
        if hasattr(bed, 'trailer_hit'):
            bed.trailer_hit(mix, t, taiko_g, seed, braam_dur, braam_g)
        else:
            bed.boom(mix, t, taiko_g)

    # ---------------------------------------------------------------- the card, the count-in
    def title_blast(self, mix, t):
        """a hunting call: two horns in fifths, a leap up a fifth (D4 -> A4 in D)"""
        hits = self.T.TITLE_HITS.get(self.name)
        r = self.tonic()
        for dt_, m, d in (((0.0, r, 0.14), (0.15, r + 7, 0.42)) if not hits else
                          [(0.15 * j, m, 0.14 if j < len(hits) - 1 else 0.42) for j, m in enumerate(hits)]):
            mix.place(self.bus, self.horn(m, d, 1.0), t + dt_, 0.32)
            mix.place(self.bus, self.horn(m - 7 if m - 7 >= 55 else m - 5, d, 0.85), t + dt_, 0.18)

    def count_in(self, mix):
        T = self.T
        ch = T.CHORDS[1]
        r = T.root_above(ch, 65)                                   # #34: A4 + C#5 over the A chord
        pcs = T.CHORD_PCS[ch]
        third = r + (pcs[1] - pcs[0]) % 12
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.horn(r, 0.22, 1.0), tc, 0.22)
            mix.place(self.bus, self.horn(third, 0.22, 0.9), tc, 0.14)

    # ---------------------------------------------------------------- the fight
    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, EV, q16 = M.A, M.EV, M.q16
        S16 = T.S16
        call_note = {}                                             # call id -> (time, midi): the echoes repeat it
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'echo' and e[2] == k:
                j, call = e[3], e[4]
                tq = q16(t)
                pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
                if j == 0:                                         # the call: a round, mellow horn note on the beat
                    m = T.line_at(self.name, tq)
                    call_note[call] = (tq, m)
                    hp = M.hp_at(k, t) / 100.0
                    if _h01_34(round(tq / S16), 3) < 0.5 * (1 - hp) ** 1.6:     # sparse when hurt
                        continue
                    sig, g, lead = self.colour(self.horn(m, 0.55 * T.BEAT, 0.8), t)
                    mix.place(self.bus, sig, tq, 0.24 * g, pan=pan)
                    if lead > 0.04:                                # the leader: a horn a sixth below (its own harmony)
                        under = T.chord_tones(m - 9, m - 3, T.chord_at(tq))
                        if under:
                            mix.place(self.bus, self.colour(self.horn(under[-1], 0.55 * T.BEAT, 0.65), t)[0], tq,
                                      0.10 * g, pan=pan)
                elif call in call_note:                            # an echo: the same note, quieter + darker each time
                    m = call_note[call][1]
                    x = self.horn(m, 0.4 * T.BEAT, 0.6)
                    x = A.lp(x, 2600 / 1.55 ** j)
                    mix.place(self.bus, self.colour(x, t, 0.5)[0], tq, 0.26 * 0.56 ** j, pan=pan)
            elif kind == 'hit' and e[2] == k and e[7] == 'echo':   # a blast lands: a brighter stab up the chord
                tq = q16(t)
                ct = T.chord_tones(65, 79, T.chord_at(tq))
                m = ct[(int(e[8]) + int(_h01_34(t, 5) * 3)) % len(ct)]
                pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
                mix.place(self.bus, self.colour(self.horn(m, 0.1, 0.9), t)[0], tq, 0.12 if e[8] == 0 else 0.08, pan=pan)
                mix.place('fx', A.thud(0.2, 160, 70, seed=int(t * 11)), tq, 0.06 if e[8] == 0 else 0.04, pan=pan)
            elif kind == 'echo_grow' and e[2] == k:                # a new ghost peels off: a soft copper shimmer
                mix.place('fx', A.chime(T.line_at(self.name, q16(t)) + 24, 0.6), q16(t), 0.025)
            elif kind == 'flick_wind' and e[2] == k:               # FINAL FLICK: the glove bursts out, winds up
                tq = q16(t)
                mix.place('fx', boing(A, M.SR, int(t * 7)), tq, 0.16)
                mix.place('fx', slide_whistle(A, M.SR, e[3] - tq), tq, 0.035)
                mix.place('fx', A.whoosh(e[3] - tq, 300, 4000), tq, 0.08)
            elif kind == 'flick' and e[2] == k:                    # ... and flicks: THWACK
                ts = e[0]
                mix.place('fx', snap(A, M.SR, int(ts * 13)), ts, 0.30)
                mix.place('fx', A.whoosh(0.25, 800, 7000, rise=False), ts - 0.02, 0.14)
                if e[3]:
                    mix.place('fx', A.thud(0.7, 140, 34), ts, 0.6)
                    self._hit(mix, ts, 0.55, 81, 0.9, 0.22)
            elif kind == 'hit' and e[2] == k and e[7] == 'flick_rope':   # flung into the ropes: the heavy one
                ts = e[0]
                mix.place('fx', A.thud(0.6, 85, 36), ts, 0.55, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
                self._hit(mix, ts, 0.45, 83)
                mix.place('fx', M.clank(70, 12), ts, 0.08)
            elif kind == 'ability' and e[2] == k:
                name, tq = e[3], q16(t)
                ab = M.CFG[k]['abilities']
                if name == ab['unlock1']['name']:                  # STOPPED HORN: the brassy, buzzing stopped tone
                    m = self.theme_at(tq)
                    mix.place(self.bus, stopped(A, self.horn(m, T.BEAT * 1.5, 1.0)), tq, 0.18)
                    dur = ab['unlock1']['dur_beats'] * T.BEAT
                    n = int(dur * M.SR)
                    buzz = A.bp(np.random.default_rng(4).normal(0, 1, n), 3000, 9000) * A.adsr(n, 0.05, 0.2, 0.5, 0.4)
                    mix.place('fx', buzz, tq, 0.02)
                elif name == ab['unlock2']['name']:                # HORN RIP: up the harmonic series of the bar's root
                    root = T.root_above(T.chord_at(tq), 45)
                    ms = [root + iv for iv in (24, 28, 31, 34, 36)]          # partials 4 5 6 7 8
                    while max(ms) > 77:
                        ms = [m - 12 for m in ms]
                    for j, m in enumerate(ms):
                        last = j == len(ms) - 1
                        x = self.horn(m, 0.6 if last else 0.07, 0.95)
                        x = M.gliss(M.vibrato(x, 0.4, 5.5, 0.15) if last else x, 2.0, 0.03)
                        mix.place(self.bus, self.colour(x, t)[0], tq + j * S16 / 2, 0.26 if last else 0.17)
                    mix.place('fx', A.whoosh(0.35, 500, 6000), tq, 0.06)
                elif name == ab['low']['name']:                    # FINAL FLICK: a horn fanfare over the wind-up
                    r = self.tonic()
                    for bt, m, d in ((0, r, .25), (.25, r + 7, .25), (.5, r + 12, .5)):
                        mix.place(self.bus, self.horn(m, d * T.BEAT, 1.0), tq + bt * T.BEAT, 0.28)
                        mix.place(self.bus, self.horn(m - 5 if m - 5 >= 57 else m, d * T.BEAT, 0.8), tq + bt * T.BEAT, 0.13)

    # ---------------------------------------------------------------- the K.O. and after
    def fall_apart(self, mix):                                     # a falling wail, clattering tubing
        M = self.M
        x = self.horn(69, 0.9, 1.0, 1.3)
        mix.place(self.bus, M.gliss(x, -12, 0.9) * np.linspace(1, 0.3, len(x)), M.P.T_HS, 0.22)
        for j, m in enumerate((86, 81, 91, 77, 84, 89)):
            mix.place('fx', M.clank(m, 20 + j), M.P.T_HS + 0.09 * j + 0.04 * _h01_34(j, 4), 0.08, pan=(j - 2.5) / 6)

    def stinger(self, mix, tw):
        r = self.tonic()
        for m, g in ((r, 0.26), (r + 7, 0.22), (r + 12, 0.2)):
            mix.place('solo', self.horn(m, 1.0, 1.0, 1.5), tw, g)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:                         # a late K.O. leaves less room: stop at the reassembly
                break
            mix.place('solo', self.horn(m, d * T.BEAT * 0.95, 0.95), ts, 0.44)

    def reassemble(self, mix):                                     # its pieces fly home as a rising call
        T, M = self.T, self.M
        r = self.tonic()
        run = [r + iv for iv in (0, 3, 7, 8, 12, 15)]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.horn(m, 0.12, 0.7), tj, 0.14)
