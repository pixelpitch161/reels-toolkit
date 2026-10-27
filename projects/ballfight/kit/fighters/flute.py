"""FLUTE (silver), as built in #32 v2 and drawn / played in #41: SNAKE CHARMER (drops woven baskets where it stands, up
to 2; silver cobras rise, sway and strike on the beat; strike reach + speed grow per landed bite), FLUTTER TONGUE
(haste), SNAKE DANCE (the cobras leave their baskets and slither after the enemy), PICCOLO (a shrill wave).

Look: the pixel-art flute in its cool steel glass (the Pixel Pitch model is a 1 px line at ball size), the baskets
(wicker dust where they land) and the cobras (rise, sway, strike, bite marks, SNAKE DANCE's slither, swatted flat by a
lance under Rules(swat_cobras=True)), PICCOLO's shivering ring and the glass cracks it leaves, silver shards; the VS
card's key-cup panel.
Sound: the high melody (role 'melody'): while its cobras are up, one note of its line (config LINES) per 8th; FLUTTER
TONGUE flutter-tongues it with 16th grace notes, SNAKE DANCE slithers a semitone down after each note; a hiss-click on
every strike, a high accent on every bite, PICCOLO a piercing shriek; a slapped cobra hisses.
Draw code from projects/ballfight_tuba_vs_flute/styles.py (#32 v2) and #41's render.py, copied unchanged; sound from
#41's audio.py."""
import math

import numpy as np
from reelkit.config import W, H, BAYER_FULL, ss
from reelkit.shadows import proj_point
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL
SILVER = np.array([206, 214, 228])
SILVER_HI = np.array([250, 252, 255])
SILVER_DK = np.array([120, 128, 150])
INK = np.array([26, 20, 36])
WICKER = np.array([196, 150, 92])
WICKER_DK = np.array([120, 84, 46])
WICKER_HI = np.array([232, 196, 132])
SCALE_DK = np.array([96, 104, 128])
HOOD_MARK = np.array([40, 34, 62])
BASKET_R, BASKET_H = 0.34, 0.30
SILV = np.array([226, 234, 248])
WICKER_DUST = [(196, 150, 92), (150, 112, 64), (226, 196, 140)]


# ---------------------------------------------------------------- the flute inside its ball (#32 styles.py)
def piccolo_rd(rd, ang, t):
    """the piccolo's shiver: a fine, fast ripple on the ring's radius (a high note), not #28's jagged screech"""
    return rd + 0.045 * np.sin(31 * ang + 90 * t) + 0.02 * np.sin(57 * ang - 140 * t)


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


# ---------------------------------------------------------------- SNAKE CHARMER: baskets + cobras (#32 v2 styles.py)
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
            base_ = WICKER if (i + j) % 2 == 0 else WICKER_DK * 1.15
            c = shade(L, nrm[None], q.mean(0)[None], base_, rimk=0.3)[0]
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
    base_ = np.array([cp[0], cp[1]])
    y_base = BASKET_H * 0.9 if not away else 0.04
    head_rest = base_ + side * sway + d_aim * 0.16 * e
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
        tgt = np.array([base_[0] + d_s[0] * s_len, base_[1] + d_s[1] * s_len, R * 0.85])
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
        q = base_ + (head[:2] - base_) * s_ + side * 0.07 * math.sin(math.pi * s_ * 1.5 + ph) * e
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
        base_ = body_col if j % 2 == 0 else SCALE_DK * 1.45
        c = shade(L, np.array([[0, 1.0, 0]]), q[None], base_, rimk=0.5)[0]
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


def flute_keys(base_):
    """the VS card panel: rows of key cups along the divider (tone holes), symmetric about it so it reads the same on
    either side; a steel silver (a near-white fighter's panel would read as white)"""
    import title as TT
    base_ = base_ * np.array([0.70, 0.72, 0.80])
    u = (TT.XX * TT.DD[0] + TT.YY * TT.DD[1]) / 22.0
    v = TT.SIDE / 30.0
    cu, cv = (u % 1.0) - 0.5, (v % 1.0) - 0.5
    d = np.hypot(cu * 22.0, cv * 30.0)
    cup = d < 6.5
    hole = d < 3.0
    base_ = np.where(cup[..., None], base_ * 1.07 + 10, base_ * 0.95)
    return np.where(hole[..., None], base_ * 0.62, base_)


class Look(base.Look):
    glass = (150, 160, 196)                       # the cool, darker glass (a silver fighter in cream glass vanishes)
    flare_scale = 0.85                            # near-white: keep its attack lights from whitening the canvas
    shards = ['tube_s'] * 6 + ['key'] * 7
    title_pattern = staticmethod(flute_keys)

    def title_closeup(self, r):
        return flute_sprite(r, 0.0, self.cfg['view'][2])

    def sprite(self, r, yaw, roll):
        return flute_sprite(r, yaw, roll)

    def setup(self):
        P, k = self.P, self.k
        T = P.T
        self.STRIKES = {}                                           # basket id -> [(t0, t_hit, t_end, aim, len)]
        for e in P.EV:
            if e[1] == 'strike' and e[2] == k:
                self.STRIKES.setdefault(e[3], []).append((e[0], e[0] + e[6], e[0] + e[6] * 2.6, e[4], e[5]))
        self.BASKETS = {}                                           # id -> dict(t_drop, home, t_gone, owner)
        for e in P.EV:
            if e[1] == 'basket' and e[2] == k:
                self.BASKETS[e[3]] = dict(t_drop=e[0], home=np.array([e[4], e[5]]), t_gone=99.0, owner=e[2])
                if e[6] >= 0 and e[6] in self.BASKETS:
                    self.BASKETS[e[6]]['t_gone'] = e[0]
        self.DANCES = [(e[0], e[0] + self.cfg['abilities']['unlock2'].get('dur_beats', 4) * T.BEAT)
                       for e in P.EV if e[1] == 'ability' and e[4] == 'dance' and e[2] == k]
        # the lance swats cobras (opt-in rule, #41): ('swat', lance owner, cobra owner, basket id, x, z); the cobra is
        # down for Rules.swat_beats (2) beats, curling back up over the second
        self.SWAT_DOWN = 2 * T.BEAT
        self.SWAT_EV = [(e[0], e[2], e[3], e[4], e[5], e[6]) for e in P.EV if e[1] == 'swat' and e[3] == k]
        self.SWATS = {}
        for e in self.SWAT_EV:
            self.SWATS.setdefault(e[3], []).append(e[0])
        self.SOLO_BK = [np.array([-0.85, 0.85]), np.array([0.85, -0.85])]   # its victory: two baskets beside it

    # ---------------------------------------------------------------- cobra state (#32 v2 / #41)
    def strike_at(self, bid, t):
        for q in self.STRIKES.get(bid, ()):
            if q[0] <= t < q[2]:
                return q
        return None

    def swat_level(self, bid, t):
        """1 = up; after a swat the cobra is slapped flat in ~0.07 s (0.12 of its height), lies there for a beat, then
        curls back up over the second beat (the 2 beats it can't strike)"""
        lv = 1.0
        for ts in self.SWATS.get(bid, ()):
            u = t - ts
            if 0 <= u < self.SWAT_DOWN:
                down = 0.12 + 0.88 * (1 - min(1.0, u / 0.07))
                up = ss(self.SWAT_DOWN * 0.5, self.SWAT_DOWN, u)
                lv = min(lv, min(down, 0.12 + 0.88 * up) if u >= 0.07 else down)
        return lv

    def swat_lean(self, bid, t):
        """(time since the swat, 0..1 strength) of the most recent swat on this cobra (for the knocked-back lean)"""
        for ts in reversed(self.SWATS.get(bid, ())):
            if 0 <= t - ts < self.SWAT_DOWN:
                return t - ts, 1 - ss(self.SWAT_DOWN * 0.4, self.SWAT_DOWN, t - ts)
        return None

    @staticmethod
    def basket_k(b, t):
        """a basket pops in when dropped (a little bounce) and shrinks away when the flute takes it back"""
        u = t - b['t_drop']
        if u < 0:
            return 0.0
        k = min(1.0, u / 0.12) * (1 + 0.18 * math.sin(min(1.0, u / 0.3) * math.pi)) if u < 0.3 else 1.0
        return k * (1 - ss(b['t_gone'], b['t_gone'] + 0.2, t))

    def dancing_k(self, t):
        return max([ss(a, a + 0.2, t) * (1 - ss(b, b + 0.3, t)) for a, b in self.DANCES] + [0.0])

    def cobras_at(self, t, fs):
        """[(basket centre, basket k, cobra cp, t_drop for the rise, strike, aim, seed)] of every basket on screen"""
        P, k = self.P, self.k
        T = P.T
        out = []
        enemy = fs[1 - k]['p']
        if T.T_FIGHT <= t < P.T_HS + 0.6:
            i = P.sim_i(min(t, P.T_KO))
            cob = {c[0]: c for c in P.RUN['COB'][i][k]} if 'COB' in P.RUN else {}
            fade = 1 - ss(P.T_HS, P.T_HS + 0.6, t)
            for bid, b in self.BASKETS.items():
                bk = self.basket_k(b, min(t, P.T_KO)) * fade
                if bk <= 0.01:
                    continue
                c = cob.get(bid)
                cp = np.array([c[3], c[4]]) if c is not None else b['home']
                S_ = self.strike_at(bid, min(t, P.T_KO)) if t < P.T_KO else None
                sw = self.swat_lean(bid, min(t, P.T_KO))
                if sw is not None and S_ is not None and S_[0] <= t - sw[0] + 1e-6:
                    S_ = None                                       # the strike the lance swatted: no lunge after it
                aim = math.atan2(enemy[1] - cp[1], enemy[0] - cp[0])
                out.append((b['home'], bk, cp, b['t_drop'], S_, aim, bid))
        if P.WIN == k and P.T_SOLO - 0.8 <= t < T.T_GLIDE:
            ctr = fs[k]['p']
            k_in = ss(P.T_SOLO - 0.8, P.T_SOLO - 0.5, t) * (1 - ss(T.T_REASSEMBLE + 0.6, T.T_GLIDE, t))
            for j, off in enumerate(self.SOLO_BK):
                hb = ctr + off
                aim = math.atan2(ctr[1] - hb[1], ctr[0] - hb[0]) + math.pi              # they face out, to the crowd
                out.append((hb, k_in, hb, P.T_SOLO - 0.5 + 0.12 * j, None, aim, 100 + j))
        return out

    def cobs(self, F):
        key = ('cobs', self.k)
        if key not in F.__dict__:
            F.__dict__[key] = self.cobras_at(F.t, F.fs)
        return F.__dict__[key]

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('wave', 'fang'):
            return {'wave': 2.0, 'fang': 0.35}[e[7]]
        return super().shake(e)

    def hit_glow(self, e):
        if e[7] == 'wave':
            return 14, 0.35
        if e[7] == 'fang':
            return 7, 0.2
        return None

    def fx_setup(self):
        P = self.P
        up = P.up
        for bid, b in self.BASKETS.items():                         # a basket lands: a puff of wicker dust (#32)
            if b['t_drop'] < P.T_KO:
                P.PA.emit(b['t_drop'] + 0.05, up(b['home'][0], b['home'][1], 0.05), 'dust', n=5, col=WICKER_DUST,
                          speed=(0.3, 0.8))
                P.MK.add(b['t_drop'] + 0.05, 'dust', b['home'][0], b['home'][1], size=0.35, col=(176, 140, 96), life=2.0,
                         seed=bid, strength=0.5)
        for t_, att, vic, bid, x, z in self.SWAT_EV:                # #41: a lance swats one of its cobras flat
            if t_ > P.T_KO:
                break
            pos = up(x, z, 0.6)
            P.BU.add(t_, pos, P.COL[att], 0.55, bright=False)
            P.FL.add(t_, pos, P.FL_COL[att], radius=1.2, peak=0.45, decay=0.2)
            P.PA.emit(t_, pos, 'scales', n=6)
            P.PA.emit(t_, pos, 'brass', n=4)

    def fx_event(self, e):
        P = self.P
        t_ = e[0]
        up = P.up
        if e[1] == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'fang':                                      # a cobra bites (#32)
                P.BU.add(t_, pos, SILV, 0.55, bright=False)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.2, peak=0.4, decay=0.2)
                P.PA.emit(t_, pos, 'scales', n=5)
                P.MK.add(t_, 'claw', x, z, size=0.18, angle=float(np.arctan2(z, x)), n=2, col=(60, 54, 84), life=2.5,
                         seed=int(t_ * 100), strength=0.7)
            elif how == 'wave':                                    # PICCOLO lands (#32)
                P.BU.add(t_, pos, SILV, 1.4)
                P.PA.emit(t_, pos, 'glass', n=12)
        elif e[1] == 'ability' and e[3] == self.cfg['abilities']['low']['name']:   # PICCOLO: glass cracks (#32)
            k = e[2]
            p = P.RUN['P'][P.sim_i(t_), k]
            ab = self.cfg['abilities']['low']
            P.FL.add(t_, up(p[0], p[1], 1.0), SILV * 0.85, radius=3.0, peak=0.8, decay=0.6)
            rng = np.random.default_rng(int(t_ * 10))
            for j in range(9):
                r_ = rng.uniform(1.0, 4.0)
                a_ = rng.uniform(0, 2 * math.pi)
                q = p + r_ * np.array([math.cos(a_), math.sin(a_)])
                if np.abs(q).max() > S.AP - 0.1:
                    continue
                tq = t_ + (r_ - R) / ab['speed']
                P.MK.add(tq, 'glass', q[0], q[1], size=0.45, angle=a_, n=6, life=5.0, seed=int(tq * 100) + j)
                P.PA.emit(tq, up(q[0], q[1], 0.05), 'glass', n=4)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'scales', n=30, spread=180, speed=(0.8, 2.2))

    # ---------------------------------------------------------------- canvas
    def shadows_free(self, F, spot):
        polys = []
        P = self.P
        a = np.linspace(0, 2 * math.pi, 11)[:-1]
        for home, bk, cp, t_drop, S_, aim, bid in self.cobs(F):    # baskets + the cobras' raised hoods
            if bk < 0.05:
                continue
            w = S.to_world(home[0], home[1], BASKET_H * bk)
            rb = BASKET_R * bk
            polys.append(proj_point(np.array([[w[0] + rb * math.cos(u), w[1], w[2] + rb * math.sin(u)] for u in a]), spot))
            e_ = min(1.0, max(0.0, (F.t - t_drop) / (P.T.BEAT * 1.0))) * self.swat_level(bid, min(F.t, P.T_KO))
            if e_ > 0.1:
                c = S.to_world(cp[0], cp[1], 0.2 + 0.6 * e_)
                polys.append(proj_point(np.array([[c[0] + 0.1 * math.cos(u), c[1], c[2] + 0.1 * math.sin(u)] for u in a]), spot))
        return polys

    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind != 'wave':
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)                      # PICCOLO: a thin ring with a fine, fast shiver (#32)
        k_ = (1 - r / rmax) * 1.3
        rd = piccolo_rd(rd, np.arctan2(C.szp - cz, C.sxp - cx), C.t)
        band = np.clip(1 - np.abs(rd - r) / 0.18, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * k_ * 0.9)[:, None] * (self.P.COL[owner] / 255.0 * 1.4 + 0.35)[None]
        return Ic

    # ---------------------------------------------------------------- baskets + cobras in the painter pass
    def items_free(self, F):
        out = []
        for c in self.cobs(F):
            home, bk, cp, t_drop, S_, c_aim, bid = c
            dzb = F.cam.depth(S.to_world(home[0], home[1], 0.12)[None])[0]
            out.append((dzb, ('basket', c)))
            dzc = min(dzb - 0.01, F.cam.depth(S.to_world(cp[0], cp[1], 0.5)[None])[0])
            out.append((dzc, ('cobra', c)))
        return out

    def draw_item(self, F, payload):
        P, kf = self.P, self.k
        T = P.T
        kind, (home, bk, cp, t_drop, S_, c_aim, bid) = payload
        t = F.t
        if kind == 'basket':
            draw_basket(F.fr, F.cam, F.L, P.shade, home[0], home[1], bk)
            return
        dk = self.dancing_k(t) if t < P.T_KO else 0.0
        rise_t = T.BEAT * self.cfg['weapon']['rise']
        if bk < 0.98 and t > t_drop + rise_t:                      # a basket being taken back: the cobra sinks in
            t_drop = t - rise_t * max(0.0, bk)
        lv = self.swat_level(bid, min(t, P.T_KO)) if bid < 100 else 1.0
        if lv < 1.0 and t > t_drop + rise_t:                       # swatted flat, curling back up
            t_drop = t - rise_t * lv
        sw = self.swat_lean(bid, min(t, P.T_KO)) if bid < 100 else None
        if sw is not None:                                          # knocked back: it faces away from the lance
            c_aim = c_aim + math.pi * 0.55 * sw[1]
        pts, head, face, e_ = cobra_pose(home, cp, t, t_drop, rise_t, c_aim, S_, dk, T.BEAT, seed=bid)
        hasted = F.in_fight and t < P.T_KO and float(P.RUN['HA'][F.i, kf]) > 0
        gl = 0.6 if (S_ is not None and t < S_[1] + 0.05) else (0.35 if hasted else 0.0)
        if sw is not None and sw[0] < 0.12:                         # the slap: a flash of the lance's colour
            gl = max(gl, 0.8 * (1 - sw[0] / 0.12))
        if t >= P.T_SOLO - 0.8:
            gl = max(gl, 0.6 * P.solo_env(t))
        draw_cobra(F.fr, F.cam, F.L, P.shade, pts, head, face, e_ * min(1.0, bk), P.COL[kf], gl)

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'tube_s':                                       # a straight silver body section (#32)
            from PIL import Image, ImageDraw
            Lk, Wk = 0.40, 0.05
            q4 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], np.clip(SILVER * (0.8 + 0.2 * math.cos(rot * 2)), 0, 255), 1.0)
            hi = P.screen(cam, np.array([S.to_world(*(np.array([sx_, sz_]) + d * Lk / 2 * a1 - nn * Wk * 0.25), p[1]) for a1 in (-0.8, 0.8)]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in hi], fill=tuple(int(v) for v in SILVER_HI))
            return np.asarray(im).astype(np.float32)
        if kind == 'key':                                          # a key cup (silver, dark pad, #32)
            rr = 0.07
            a8 = np.linspace(0, 2 * math.pi, 9)[:-1]
            sp = P.screen(cam, np.array([S.to_world(sx_ + rr * math.cos(u), sz_ + rr * math.sin(u), p[1]) for u in a8]))
            P.layer_blend(arr, [sp], SILVER_HI, 1.0)
            cq = P.screen(cam, S.to_world(sx_, sz_, p[1])[None])[0]
            xi, yi = int(round(cq[0])), int(round(cq[1]))
            if 0 <= xi < W and 0 <= yi < H:
                arr[yi, xi] = SILVER_DK * 0.7
        return arr

    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'wave':                                         # the piccolo ring (#32's numbers)
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.3, 2.4, t, k=1 - r / rmax, seed=owner)
        return img

    def focus_points(self, t, st):
        F = type('F', (), {})()
        F.t, F.fs = t, [self.P.fighter(k, t) for k in (0, 1)]
        out = []
        for home, bk, cp, t_drop, S_, aim, bid in self.cobras_at(F.t, F.fs):   # baskets, raised hoods (~1.3 high),
            out.append((S.to_world(home[0], home[1], 0.0), 'basket'))         # a strike's head out to its reach
            out.append((S.to_world(cp[0], cp[1], 1.3), 'cobra'))
            if S_ is not None:
                out.append((S.to_world(cp[0] + S_[4] * math.cos(S_[3]), cp[1] + S_[4] * math.sin(S_[3]), R), 'strike'))
        return out


# ===================================================================================================== sound (#41)
class Sound(base.Sound):
    bus_gain, send = 1.7, 0.45
    band = (600, 3200)                                             # the high melody + its breath

    def flute(self, m, dur=0.1, vel=0.8):
        return self.M.note('FLUTE', m, round(max(0.06, dur), 3), round(max(0.06, dur) + 0.18, 2), round(vel, 1))

    def breath(self, dur, seed=5):
        """flute breath: airy band noise (#32)"""
        A = self.M.A
        n = int(dur * A.SR)
        x = np.random.default_rng(seed).normal(0, 1, n)
        return A.bp(x, 1500, 7000) * A.expenv(n, dur * 0.5, 0.01)

    def flutter(self, x, rate=24.0):
        """flutter-tongue: a fast growl on the tone (#32)"""
        tt = self.M.A.tt(len(x))
        return x * (0.55 + 0.45 * np.sign(np.sin(2 * np.pi * rate * tt)))

    def shriek(self, dur=1.6, f0=2637.0):
        """PICCOLO: a piercing, wobbling high whistle with a hard edge (#32), on E7"""
        A = self.M.A
        n = int(dur * A.SR)
        tt = A.tt(n)
        f = f0 * (1 + 0.012 * np.sin(2 * np.pi * 7.5 * tt)) * (1 + 0.06 * np.clip(tt / 0.08, 0, 1))
        ph = 2 * np.pi * np.cumsum(f) / A.SR
        x = np.sin(ph) + 0.3 * np.sin(2 * ph) + 0.12 * np.sin(3 * ph)
        x = np.tanh(1.8 * x) * np.clip(tt / 0.02, 0, 1) * np.exp(-tt / (dur * 0.6))
        return x + 0.25 * self.breath(dur, 21)

    def title_blast(self, mix, t):
        """a shrieking flurry (config TITLE_HITS) up to a flutter-tongued top note"""
        hits = self.T.TITLE_HITS.get(self.name, (76, 79, 83, 84, 86, 87, 88))
        for j, m in enumerate(hits):
            mix.place(self.bus, self.flute(m, 0.06, 1.0), t + j * self.T.S16 / 2, 0.30, pan=-0.15)
        mix.place(self.bus, self.flutter(self.flute(hits[-1], 0.5, 1.0), 26), t + 3.5 * self.T.S16, 0.32, pan=-0.15)
        mix.place(self.bus, self.breath(0.4, 81), t, 0.10, pan=-0.15)

    def count_in(self, mix):
        """three rising chord tones of the count chord on 3 . 2 . 1, the middle one again on the pickup"""
        T = self.T
        ch = T.CHORDS[1]
        lo, hi = self.M.CFG[self.k]['register']
        tones = [m for m in T.chord_tones((lo + hi) // 2 + 1, hi + 12, ch)][:3]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.flute(tones[j], 0.12, 0.9), tc, 0.26, pan=-0.2)
        mix.place(self.bus, self.flute(tones[1], 0.25, 0.9), T.BAR + 3 * T.BEAT, 0.20, pan=-0.2)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, RUN, EV, q16, S16 = M.A, M.RUN, M.EV, M.q16, T.S16
        look = M.P.LOOKS[k]
        rise = self.M.CFG[k]['weapon']['rise'] * T.BEAT
        t_up = min([b['t_drop'] for b in look.BASKETS.values()] + [99.0]) + rise
        thin = M.Thinner(5)
        k0, k1 = int(math.ceil(t_up / (2 * S16))), int(M.T_KO / (2 * S16))
        for s8 in range(k0, k1 + 1):                               # the melody, one note per 8th while the cobras are up
            tq = s8 * 2 * S16
            m = T.line_at(self.name, tq)
            hp = M.hp_at(k, tq) / 100.0
            if not thin.keep(s8, hp):
                continue
            hasted = float(RUN['HA'][M.P.sim_i(tq), k]) > 0
            dancing = any(a_ <= tq < b_ for a_, b_ in look.DANCES)
            dur = 0.2 if not (hasted or dancing) else 0.11
            x = self.flute(m, dur, 0.8)
            if hasted:
                x = self.flutter(x)
            sig, g, lead = self.colour(x, tq, 0.9)
            mix.place(self.bus, sig, tq, 0.26 * g, pan=-0.2)
            if hasted or dancing:                                  # the 16th between: a grace note
                m2 = m - 1 if dancing else T.line_at(self.name, tq + 2 * S16)   # SNAKE DANCE slithers a semitone down
                if m2 != m or dancing:
                    x2 = self.flute(m2, 0.1, 0.7)
                    if hasted:
                        x2 = self.flutter(x2)
                    mix.place(self.bus, self.colour(x2, tq)[0], tq + S16, 0.20 * g, pan=-0.2)
            if (1 - hp) > 0.3:
                mix.place(self.bus, self.breath(0.1, s8), tq, 0.07 * (1 - hp), pan=-0.2)
            if lead > 0.04:                                        # the leader: a thin octave double underneath
                mix.place(self.bus, self.colour(self.flute(m - 12, dur, 0.6), tq)[0], tq, 0.08 * g, pan=-0.25)
        ab = M.CFG[k]['abilities']
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'strike' and e[2] == k:                     # a hiss-click on every strike (#32)
                tq = q16(t)
                n = int(0.09 * A.SR)
                tt_ = A.tt(n)
                click = np.sin(2 * np.pi * 2800 * tt_) * np.exp(-tt_ / 0.004)
                mix.place('fx', M.hiss(0.09, int(t * 50)) * 0.8 + click * 0.5, tq, 0.08,
                          pan=float(np.clip(math.cos(e[4]) * 0.4, -0.5, 0.5)))
            elif kind == 'hit' and e[2] == k and e[7] == 'fang':   # a bite lands: a high accent
                tq = q16(t)
                ct = T.chord_tones(84, 93, T.chord_at(tq + 0.01))
                m = ct[int(e[8]) % len(ct)] if ct else 88
                sig, g, _ = self.colour(self.flute(m, 0.07, 1.0), t, 0.9)
                mix.place(self.bus, sig, tq, 0.16 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
            elif kind == 'basket' and e[2] == k:                   # a basket lands: a soft wicker thump
                mix.place('fx', A.thud(0.18, 220, 120, seed=int(t * 9)), t + 0.05, 0.07)
            elif kind == 'hit' and e[2] == k and e[7] == 'wave':
                mix.place('fx', A.thud(0.45, 140, 60), q16(t), 0.26)
            elif kind == 'swat' and e[3] == k:                     # a lance slaps one of its cobras flat (#41)
                tq = q16(t)
                mix.place('fx', self.swat_sound(int(t * 31)), tq, 0.16, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
                mix.place(self.bus, M.hiss(0.16, int(t * 17), 3000, 9000, 0.05), tq + 0.02, 0.05, pan=-0.2)
            elif kind == 'ability' and e[2] == k:
                name, tq = e[3], q16(t)
                if name == ab['unlock1']['name']:                  # FLUTTER TONGUE
                    mix.place(self.bus, self.flutter(self.flute(88, 0.5, 0.9), 26), tq, 0.20, pan=-0.2)
                elif name == ab['unlock2']['name']:                # SNAKE DANCE: a rattle of hisses as the cobras leave
                    for j in range(4):
                        mix.place(self.bus, M.hiss(0.12, 70 + j, 4000, 11000, 0.05), tq + j * S16, 0.09, pan=0.3 * (j % 2 * 2 - 1))
                elif name == ab['low']['name']:                    # PICCOLO: the highest note, a piercing shriek on E7
                    mix.place(self.bus, self.shriek(1.7), tq, 0.13, pan=-0.2)
                    mix.place(self.bus, self.colour(self.flute(93, 0.9, 1.0), t, 2.0)[0], tq, 0.16)

    def swat_sound(self, seed=0):
        """a lance slaps a cobra flat: a leathery slap (a short low-mid smack) + a startled, falling hiss"""
        A = self.M.A
        n = int(0.3 * A.SR)
        tt_ = A.tt(n)
        rng = np.random.default_rng(seed)
        smack = A.bp(rng.normal(0, 1, n), 600, 3500) * np.exp(-tt_ / 0.012)
        body = np.sin(2 * np.pi * np.cumsum(260 * np.exp(-tt_ / 0.04) + 120) / A.SR) * np.exp(-tt_ / 0.05)
        hs = A.bp(rng.normal(0, 1, n), 2500, 9000) * np.clip((tt_ - 0.02) / 0.02, 0, 1) * np.exp(-np.maximum(tt_ - 0.04, 0) / 0.07)
        return smack * 1.2 + body * 0.6 + hs * 0.5

    def fall_apart(self, mix):                                     # the flute falls apart: a broken run down (#32)
        T_HS = self.M.P.T_HS
        for j, m in enumerate((93, 89, 84, 80, 77, 74)):
            mix.place(self.bus, self.flute(m, 0.08, 0.8), T_HS + 0.09 * j, 0.20, pan=(j - 2.5) / 6)
            mix.place(self.bus, self.breath(0.08, 40 + j), T_HS + 0.09 * j, 0.06)

    def stinger(self, mix, tw):
        for j, m in enumerate((76, 79, 83, 88)):
            mix.place('solo', self.flute(m, 0.12 if j < 3 else 0.8, 1.0), tw + j * self.T.S16 / 2, 0.30)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:                         # a late K.O. leaves less room: stop at the reassembly
                break
            mix.place('solo', self.flute(m, d * T.BEAT * 0.9, 0.9), ts, 0.50)

    def reassemble(self, mix):
        T, M = self.T, self.M
        for j, m in enumerate((76, 79, 83, 84, 88, 91)):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
            mix.place(self.bus, self.flute(m, 0.08, 0.7), tj, 0.16)
