"""BASSOON (forest green), from #39 Bassoon vs Bari Sax (v4), ported for #52 Oboe vs Bassoon: the only sneak. TIPTOE
(the ball stands still between exaggerated staccato hops behind the enemy, a stomp on every landing, x2.5 from
behind), STACCATO (hops on the 8ths), BOCAL HOOK (the silver crook swings out, hooks the enemy and spins it to face
away), MOUNTAIN KING (20 accelerating hops round the enemy: Grieg's theme).

Look (#39's styles.py + render.py layers, unchanged): it crouches, springs up in a tall arc (stretching as it rises and
falls), lands flat (squash) with a stomp ring and a puff of maple dust, tiptoe prints fading on the canvas, a faint
dotted arc of each hop; a backstab adds crossed green slashes, a bigger double ring and a claw mark. BOCAL HOOK = the
silver crook drawn over both balls, the hooked enemy whipping round (swirl, after the grade). MOUNTAIN KING = the
circle of landing spots round the enemy and a green light that keeps up the dance. It bobs on its toes in the corner;
as champion it tiptoes a little hop on every solo note. It shatters into maple sticks, nickel keys and cane blades.
Not ported: #39 v2's leap over a swinging WRECKING BALL (drawn 1.45 high): it waits for the BARI SAX port.

Sound (#39's audio.py part, unchanged; its notes follow the config's KEY: E minor = #39's): a staccato dot per hop
landing from its line (config LINES, 8 a bar; STACCATO's 16th landings take the passing note), a missed stomp ghosted, a
backstab accented with a grace up the chord, the leader a sixth under; BOCAL HOOK a comic blat sliding up + the crook's
clank; MOUNTAIN KING = Grieg's theme in the key, one note per hop, so it accelerates with the hops. New with the port
(silent in #39, where nothing detunes): while the enemy has it OUT OF TUNE (the oboe's TUNING A) every note plays flat
by up to a quarter-tone against a beating sour twin, the Grieg quote included."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL, ss, ease
import scene as S

from . import base

R = S.R_BALL
INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)
NICKEL = np.array([216, 220, 232], float)
CANE = np.array([232, 208, 150], float)
THREAD = np.array([200, 46, 50], float)
WOOD = np.array([120, 56, 52], float)                    # #39's shatter maple (as loser)
NICKEL_SH = np.array([214, 216, 226], float)
CANE_SH = np.array([232, 208, 150], float)
MAPLE_DUST = [(214, 176, 140), (186, 140, 110), (240, 214, 186)]
GREEN_SP = [(150, 240, 180), (60, 190, 120), (240, 255, 240)]

# Grieg, In the Hall of the Mountain King (1875, public domain): the theme's opening, as semitones above the key's
# tonic in the bassoon's tenor register (#39: E minor, 64 = E4): one note per hop, so it accelerates with the hops
MK_THEME = [0, 2, 3, 5, 7, 3, 7,      # E F# G A B G B
            6, 2, 6,                  # A# F# A#
            5, 1, 5,                  # A F A
            0, 2, 3, 5, 7, 3, 7]      # E F# G A B G B
MK_BASE = 24                          # above T.KEY_ROOT (C2 octave): E2 40 -> E4 64


# ===================================================================================================== #39 styles.py
def _mask(polys, lines=(), width=1):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(map(float, p)) for p in pts], fill=255)
    for pts in lines:
        if len(pts) >= 2:
            d.line([tuple(map(float, p)) for p in pts], fill=255, width=width)
    return np.asarray(m, np.float32) / 255.0


def blend(img, mk, col, alpha):
    if alpha <= 0:
        return img
    img += (np.asarray(col, float)[None, None] - img) * (mk * alpha)[..., None]
    return img


def add(img, mk, col, k):
    img += mk[..., None] * np.asarray(col, float)[None, None] * k
    return img


def hop_height(L):
    """peak height of a hop of length L (ring units): a tall, exaggerated tiptoe even for short hops"""
    return 0.42 + 0.16 * min(2.5, L)


def hop_state(u, L):
    """(height, squash-x, squash-y) of a hopping ball at hop progress u (0..1): it crouches as it leaves, stretches
    tall while it rises and falls fast, round at the top"""
    hgt = hop_height(L) * 4 * u * (1 - u)
    v = abs(1 - 2 * u)                                               # vertical speed (0 at the top)
    sy = 1.0 + 0.16 * v
    return hgt, 1.0 / math.sqrt(sy), sy


def land_squash(age):
    """squash on landing: flat and wide for a couple of frames, then a little bounce back"""
    if age < 0 or age > 0.2:
        return 1.0, 1.0
    if age < 0.07:
        k = 1 - age / 0.07 * 0.3
        return 1.0 + 0.24 * k, 1.0 - 0.26 * k
    u = (age - 0.07) / 0.13
    s = 0.06 * math.sin(math.pi * u) * (1 - u)
    return 1.0 - s, 1.0 + s


def stomp_light(sxp, szp, x, z, age, radius, back=False):
    """(N,) light of a stomp's ring on the canvas: a quick expanding band (a backstab's is bigger and double)"""
    life = 0.32 if back else 0.24
    if age < 0 or age >= life:
        return None
    u = age / life
    rd = np.hypot(sxp - x, szp - z)
    r = 0.35 + (radius + 0.5 + 0.5 * back) * (1 - (1 - u) ** 2)
    band = np.clip(1 - np.abs(rd - r) / (0.07 + 0.05 * back), 0, 1) * (1 - u)
    if back:
        band = np.maximum(band, np.clip(1 - np.abs(rd - r * 0.62) / 0.05, 0, 1) * (1 - u) * 0.8)
    return band


def draw_hop_arc(img, cam, p0, p1, age_k, col, y0, peak):
    """a faint dotted arc of the hop's path (the tiptoe's trajectory), fading out"""
    if age_k <= 0.02:
        return img
    u = np.linspace(0, 1, 9)
    P = p0[None] + (p1 - p0)[None] * u[:, None]
    hy = y0 + peak * 4 * u * (1 - u)
    sp = cam.wp(S.to_world(P[:, 0], P[:, 1], hy))
    c = np.clip(np.asarray(col) * 0.7 + 90, 0, 255)
    for j, (x, yy) in enumerate(sp[1:-1]):
        xi, yi = int(round(x)), int(round(yy))
        if 1 <= xi < W - 1 and 1 <= yi < H - 1:
            img[yi, xi] = img[yi, xi] * (1 - age_k) + c * age_k
    return img


def backstab_slash(img, cam, x, z, aim, age, col, y=0.35, life=0.28):
    """a backstab: two crossed slashes in the bassoon's green, inked, flicking across the enemy's back"""
    if age < 0 or age >= life:
        return img
    k = 1 - age / life
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    grow = min(1.0, age / 0.06)
    for sgn in (1, -1):
        a = np.array([x, z]) - d * 0.22 + nrm * 0.62 * sgn
        b = np.array([x, z]) + d * 0.22 - nrm * 0.62 * sgn
        b = a + (b - a) * grow
        P = np.array([a, b])
        sp = cam.wp(S.to_world(P[:, 0], P[:, 1], np.array([y + 0.3, y - 0.1])))
        img = blend(img, _mask([], [sp], 5), INK, 0.75 * k)
        img = blend(img, _mask([], [sp], 3), np.clip(np.asarray(col) * 0.7 + 90, 0, 255), 1.0 * k)
        img = blend(img, _mask([], [sp], 1), WHITE, 0.9 * k)
    return img


def crook_points(p, aim, ext, R_, curl, y):
    """the bocal as a hook: out of the ball's rim along the aim for `ext` (ring units), then curling round by `curl`
    (rad) at the end; returns (n, 3) ring-frame points (x, z, height)"""
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    base_ = np.asarray(p, float) + d * R_ * 0.9
    n1 = 10
    s = np.linspace(0, 1, n1)
    shaft = base_[None] + d[None] * (ext * s)[:, None] + nrm[None] * (0.18 * np.sin(np.pi * s) * ext / 1.6)[:, None]
    hy = y + 0.10 + 0.25 * np.sin(np.pi * s) * min(1.0, ext / 1.2)
    pts = [np.column_stack([shaft, hy])]
    if curl > 0.05:                                                  # the hook at the end, curling back round
        rc = 0.22
        c = shaft[-1] + nrm * rc
        a0 = math.atan2(-nrm[1], -nrm[0])
        aa = a0 + np.linspace(0, curl, 8)[1:]
        hook = c[None] + rc * np.column_stack([np.cos(aa), np.sin(aa)])
        pts.append(np.column_stack([hook, np.full(len(aa), hy[-1])]))
    return np.vstack(pts)


def draw_crook(img, cam, p, aim, ext, R_, curl, y, col, glow=0.0):
    """BOCAL HOOK: a silver crook swinging out of the ball (inked, a cane-coloured reed nub at the root, lit green while
    it hooks)"""
    P = crook_points(p, aim, ext, R_, curl, y)
    sp = cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))
    img = blend(img, _mask([], [sp], 7), INK, 0.85)
    img = blend(img, _mask([], [sp], 4), NICKEL * 0.82 + np.asarray(col) * 0.18, 1.0)
    img = blend(img, _mask([], [sp[: max(2, len(sp) * 2 // 3)]], 1), WHITE, 0.9)
    if glow > 0.02:
        img = add(img, _mask([], [sp], 7), np.asarray(col) * 0.7 + 60, 0.35 * glow)
    r0 = sp[0]
    xi, yi = int(round(r0[0])), int(round(r0[1]))
    img[max(0, yi - 1):yi + 2, max(0, xi - 1):xi + 2] = THREAD
    return img


def swirl(img, x, y, rp, k, t, col):
    """a hooked enemy spun round: two curved speed arcs whipping round the ball"""
    if k <= 0.02:
        return img
    c = np.clip(np.asarray(col) * 0.6 + 100, 0, 255)
    for j in range(2):
        a0 = t * 18.0 + j * math.pi
        aa = a0 + np.linspace(0, 1.6, 9)
        pts = np.column_stack([x + rp * 1.25 * np.cos(aa), y + rp * 1.25 * np.sin(aa)])
        img = blend(img, _mask([], [pts], 3), INK, 0.5 * k)
        img = blend(img, _mask([], [pts], 1), c, 1.0 * k)
    return img


def mk_trail(cam, centre, ang0, step, n_done, R_, gap, y=0.06):
    """MOUNTAIN KING's dance: the circle of landing spots round the enemy (the ones so far), as screen points"""
    rr = 2 * R_ + gap
    a = ang0 + math.radians(step) * np.arange(n_done)
    P = np.column_stack([centre[0] + rr * np.cos(a), centre[1] + rr * np.sin(a)])
    return cam.wp(S.to_world(P[:, 0], P[:, 1], np.full(len(P), y)))


def layer_blend(arr, polys, col, alpha):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for p in polys:
        d.polygon([tuple(q) for q in p], fill=255)
    mk = np.asarray(m, np.float32)[..., None] / 255.0 * alpha
    arr[:] = arr * (1 - mk) + np.asarray(col, float)[None, None] * mk


# ===================================================================================================== look
class Look(base.Look):
    shards = ['wood'] * 6 + ['key'] * 8 + ['reed'] * 3          # #39's: maple sticks, nickel keys, cane blades
    flash_hits = ('hook',)
    rope_kinds = {}
    speed_lift = True                                             # #39: speed lines measured at the hop's height

    # ---------------------------------------------------------------- setup
    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.HOPS = [(e[0], e[2], np.array([e[3], e[4]]), np.array([e[5], e[6]]), e[7], e[8])
                     for e in EV if e[1] == 'hop' and e[2] == k]
        self.HOP_T = np.array([h[0] for h in self.HOPS])
        self.STOMPS = [(e[0], e[2], e[3], e[4], e[5], e[6], e[7], e[8]) for e in EV if e[1] == 'stomp' and e[2] == k]
        self.STOMP_T = np.array([q[0] for q in self.STOMPS])
        self.HOOK_OPEN = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'hook_open' and e[2] == k]
        self.HOOKS = [(e[0], e[2], e[3], e[4], e[5]) for e in EV if e[1] == 'hook' and e[2] == k]
        self.HOOK_REC = np.asarray(P.RUN['HOOK'], float) if 'HOOK' in P.RUN else None
        self.MKS = []
        for e in EV:
            if e[1] == 'mountain' and e[2] == k:
                ends = [q[0] for q in EV if q[1] == 'mountain_end' and q[2] == k and q[0] >= e[0]]
                self.MKS.append((e[0], min(ends + [P.T_KO]), k))

    # ---------------------------------------------------------------- clocks (#39 render.py)
    def hop_at(self, t):
        ts = self.HOP_T
        if not len(ts) or t >= self.P.T_KO or t < self.T.T_FIGHT:
            return None, None
        j = int(np.searchsorted(ts, t, 'right')) - 1
        if j < 0:
            return None, None
        h = self.HOPS[j]
        if h[0] <= t < h[4]:
            return h, (t - h[0]) / max(1e-6, h[4] - h[0])
        return None, None

    def last_stomp(self, t):
        ts = self.STOMP_T
        j = int(np.searchsorted(ts, t, 'right')) - 1
        return (t - ts[j]) if j >= 0 else 99.0

    def hooked_k(self, t):
        """how hard the enemy is whipping round after being hooked (0..1)"""
        for th, kk, hit, aim, reach in self.HOOKS:
            if hit and th <= t < th + 0.4 and th < self.P.T_KO:
                return 1 - (t - th) / 0.4
        return 0.0

    def crook_state(self, t):
        """(aim, ext, curl, glow) of BOCAL HOOK's crook at t, or None"""
        P, k = self.P, self.k
        if not (self.T.T_FIGHT <= t < P.T_KO):
            return None
        for t0, kk, t_hook, aim in self.HOOK_OPEN:
            if not (t0 <= t < t_hook + 0.42):
                continue
            ab = self.cfg['abilities']['unlock2']
            i = P.sim_i(min(t, P.T_KO))
            row = self.HOOK_REC[i, k] if self.HOOK_REC is not None else None
            a = float(row[1]) if row is not None and not np.isnan(row[1]) else aim
            res = [q for q in self.HOOKS if abs(q[0] - t_hook) < 0.02]
            hit = bool(res and res[0][2])
            if res:
                a = res[0][3] if t >= t_hook else a
            full = R * 0.4 + ab['reach'] + R * 0.6
            if t < t_hook:
                u = ease((t - t0) / max(1e-6, t_hook - t0))
                return a, 0.35 + (full - 0.35) * u, 0.0, 0.3 * u
            v = (t - t_hook) / 0.42
            curl = (2.4 if hit else 0.6) * min(1.0, v * 4)
            return a, full * (1 - ease(max(0.0, v - 0.45) / 0.55)) + 0.2, curl * (1 - ease(max(0.0, v - 0.6) / 0.4)), \
                (1.0 if hit else 0.4) * (1 - v)
        return None

    def live_hops(self, t):
        """[(p0, p1, fade, peak)] of the hops whose dotted arcs show at t (the arc fades over the hop + 0.2 s)"""
        out = []
        if not (self.T.T_FIGHT <= t < self.P.T_KO):
            return out
        j = int(np.searchsorted(self.HOP_T, t, 'right'))
        for h in self.HOPS[max(0, j - 2):j]:
            age = t - h[0]
            life = (h[4] - h[0]) + 0.2
            if 0 <= age < life:
                out.append((h[2], h[3], 0.55 * (1 - age / life), hop_height(float(np.hypot(*(h[3] - h[2]))))))
        return out

    def live_stomps(self, t):
        if not (self.T.T_FIGHT <= t < self.P.T_HS):
            return []
        tc = min(t, self.P.T_KO)
        return [(x, z, tc - t_, rad, back) for t_, k, x, z, hit, back, rad, kind in self.STOMPS if t_ <= tc < t_ + 0.34]

    def live_slashes(self, t):
        P = self.P
        out = []
        if not (self.T.T_FIGHT <= t < P.T_HS):
            return out
        tc = min(t, P.T_KO)
        for _, e in P.events_between(tc - 0.3, tc, ('hit',)):
            if e[2] == self.k and e[7] == 'stomp' and e[8]:
                v = P.RUN['V'][P.sim_i(e[0]), e[3]]
                out.append((e[5], e[6], math.atan2(v[1], v[0]), tc - e[0]))
        return out

    def mk_state(self, t):
        P = self.P
        for t0, t1, k in self.MKS:
            if t0 <= t < min(t1 + 0.4, P.T_KO):
                hs = [h for h in self.HOPS if t0 - 1e-6 <= h[0] <= t and h[5] == 2]
                if not hs:
                    return None
                i = P.sim_i(t)
                c = P.RUN['P'][i, 1 - k]
                d = hs[0][3] - P.RUN['P'][P.sim_i(hs[0][4]), 1 - k]
                return c, math.atan2(d[1], d[0]), len(hs), (1.0 if t < t1 else 1 - (t - t1) / 0.4)
        return None

    # ---------------------------------------------------------------- the ball
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k:
            return
        T, P = self.T, self.P
        st.setdefault('sx', 1.0)
        st.setdefault('sy', 1.0)
        if phase == 'rev':                                       # it bobs on its toes in the corner
            b = T.beat_pulse(t, 0.1)
            st['sx'], st['sy'] = 1.0 + 0.06 * b, 1.0 - 0.07 * b
            return
        if phase == 'fight':
            if t < P.T_KO:
                hop, u = self.hop_at(t)
                if hop is not None:
                    st['p'] = hop[2] + (hop[3] - hop[2]) * u
                    L = float(np.hypot(*(hop[3] - hop[2])))
                    st['lift'], st['sx'], st['sy'] = hop_state(u, L)
                    st['glow'] = 0.15 + 0.25 * (hop[5] == 2)
                else:
                    st['sx'], st['sy'] = land_squash(self.last_stomp(t))
            self._air(st)
            return
        # tail: back on its toes in the corner; as champion a little hop on every solo note
        if t >= T.T_REV:
            b = T.beat_pulse(t, 0.1) * ss(T.T_REV, T.T_REV + 0.3, t)
            st['sx'], st['sy'] = 1.0 + 0.06 * b, 1.0 - 0.07 * b
        if kk == P.WIN and P.T_SOLO <= t < T.T_REASSEMBLE:
            e = P.solo_env(t)
            st['lift'] = 0.35 * (1 - e) * e * 4
        self._air(st)

    @staticmethod
    def _air(st):
        """#39: an airborne ball is painted and depth-written 0.6 nearer (it clears whatever it hops past), its painter
        depth taken at the ball's resting height"""
        if st['lift'] > 0.02:
            st['air_bias'] = 0.6
            st['paint_h'] = R

    # ---------------------------------------------------------------- events -> camera, sparks, glows
    def shake(self, e):
        if e[1] == 'hit' and e[0] <= self.P.T_KO:
            if e[7] == 'stomp':
                return 0.4 if e[8] == 0 else 0.9
            if e[7] == 'hook':
                return 1.0
        return None

    def rope(self, e):
        return None

    def spark(self, e):
        col = self.P.COL[e[2]] * 0.6 + 100
        how = e[7]
        if how == 'stomp':
            n, sp, life = (12, 2.8, 0.26) if e[8] else (5, 1.6, 0.18)       # a backstab throws more
        elif how == 'hook':
            n, sp, life = 16, 3.2, 0.3
        else:
            n, sp, life = 8, 2.4, 0.22
        return col, n, sp, life

    def _big(self, e):
        return e[7] == 'hook' or (e[7] == 'stomp' and e[8] == 1)

    def hit_glow(self, e):
        return (8, 0.2) if self._big(e) else None

    def fringe(self, e):
        return self._big(e)

    # ---------------------------------------------------------------- effects (reelkit.fx), in #39's order
    def fx_setup(self):
        """TIPTOE: every landing a stomp: maple dust, a print on the canvas, a green flare (#39 built these first)"""
        P = self.P
        up = P.up
        for t_, k, x, z, hit, back, rad, kind in self.STOMPS:
            if t_ > P.T_KO:
                break
            P.PA.emit(t_, up(x, z, 0.05), 'dust', n=3 + 2 * back, col=MAPLE_DUST, scale=0.8)
            P.MK.add(t_, 'scorch', x, z, size=0.13, col=(40, 30, 30), life=2.4, seed=int(t_ * 100), strength=0.3)
            P.FL.add(t_, up(x, z, R * 0.5), P.FL_COL[k], radius=1.2 + 0.6 * back,
                     peak=0.35 + 0.45 * back + 0.2 * (kind == 2), decay=0.18)

    def fx_event(self, e):
        P = self.P
        if e[1] != 'hit':
            return
        t_, att, x, z, how = e[0], e[2], e[5], e[6], e[7]
        pos = P.up(x, z, R)
        if how == 'stomp' and e[8]:                                # a backstab: green slashes, wood chips
            P.BU.add(t_, pos, P.COL[att], 1.1)
            P.PA.emit(t_, pos, 'chips', n=8, col=GREEN_SP + MAPLE_DUST[:1])
            v = P.RUN['V'][P.sim_i(t_), e[3]]
            P.MK.add(t_, 'claw', x, z, size=0.5, angle=math.atan2(v[1], v[0]) + 1.2, col=(20, 60, 36), life=2.6, n=2,
                     seed=int(t_ * 100), strength=0.6)
        elif how == 'stomp':
            P.PA.emit(t_, pos, 'sparks', n=4, col=GREEN_SP)
        elif how == 'hook':                                        # BOCAL HOOK lands
            P.BU.add(t_, pos, (220, 226, 240), 1.0)
            P.FL.add(t_, pos, P.FL_COL[att], radius=1.6, peak=0.8, decay=0.3)
            P.PA.emit(t_, pos, 'sparks', n=10, col=[(226, 228, 240), (255, 255, 255)] + GREEN_SP[:1])

    def fx_after(self):
        """MOUNTAIN KING: a green light on the enemy that keeps up the dance (every 8th)"""
        P = self.P
        for t0, t1, k in self.MKS:
            tb = t0
            while tb < t1:
                i = P.sim_i(tb)
                p = P.RUN['P'][i, 1 - k]
                P.FL.add(tb, P.up(p[0], p[1], R), P.FL_COL[k], radius=2.0, peak=0.5, decay=0.3)
                tb += self.T.BEAT / 2

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'chips', n=26, spread=180, speed=(0.8, 2.2),
                       col=[tuple(WOOD), (214, 216, 226), (232, 208, 150)])

    # ---------------------------------------------------------------- the canvas
    def canvas(self, Ic, C):
        """the stomp rings (a backstab's bigger and double)"""
        col = self.col
        for x_, z_, age, rad, back in self.live_stomps(C.t):
            band = stomp_light(C.sxp, C.szp, x_, z_, age, rad, back)
            if band is not None:
                band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                Ic += (band * (0.9 + 0.5 * back))[:, None] * (col / 255.0 * 1.2 + 0.25)[None]
        return Ic

    # ---------------------------------------------------------------- painter: BOCAL HOOK's crook over both balls
    def items_ball(self, F, st):
        if F.in_fight and self.crook_state(F.t) is not None:
            return [(-1e9, 'crook')]
        return []

    def draw_item(self, F, payload):
        if payload != 'crook':
            return
        st = F.fs[self.k]
        arr, depth = F.fr.array(), F.fr.depth()
        a_, ext, curl, g_ = self.crook_state(F.t)
        arr = draw_crook(arr, F.cam, st['p'], a_, ext, R, curl, R + st['lift'], self.col, glow=g_)
        F.fr.set_array(arr, depth)

    # ---------------------------------------------------------------- over the ring: hop arcs, slashes, the dance
    def over_ring(self, F, arr, depth):
        cam, t, col = F.cam, F.t, self.col
        for p0, p1, fade, peak in self.live_hops(t):
            arr = draw_hop_arc(arr, cam, p0, p1, fade, col, R, peak)
        for x_, z_, a_, age in self.live_slashes(t):
            arr = backstab_slash(arr, cam, x_, z_, a_, age, col, y=R)
        mk = self.mk_state(t) if F.in_fight else None
        if mk is not None:
            c_m, a0_m, n_m, kk_m = mk
            ab_m = self.cfg['abilities']['low']
            pts = mk_trail(cam, c_m, a0_m, ab_m['step'], n_m, R, ab_m['gap'])
            ccol = np.clip(col * 0.7 + 90, 0, 255)
            for j, (x_, y_) in enumerate(pts):
                xi, yi = int(round(x_)), int(round(y_))
                if 2 <= xi < W - 2 and 2 <= yi < H - 2:
                    kj = kk_m * (0.5 + 0.5 * (j == len(pts) - 1))
                    arr[yi - 1:yi + 2, xi - 1:xi + 2] = arr[yi - 1:yi + 2, xi - 1:xi + 2] * (1 - kj) + ccol * kj
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """#39's: a stick of the maple body / a cane blade, a nickel key chip"""
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        to = lambda pts: cam.wp(np.atleast_2d(np.array([S.to_world(q[0], q[1], p[1]) for q in pts])))
        if kind in ('wood', 'reed'):
            ln_, wd_ = (0.26, 0.035) if kind == 'wood' else (0.12, 0.03)
            q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            layer_blend(arr, [to(q4)], WOOD if kind == 'wood' else CANE_SH, 1.0)
        else:
            col = NICKEL_SH if kind == 'key' else WOOD * 1.3
            sz = size * (0.7 if kind == 'key' else 1.5)
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [to(tri)], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- after the grade: the hooked enemy whips round
    def after_grade(self, img, F, drawn):
        if F.in_fight and self.enemy in drawn:
            s_, rp = drawn[self.enemy]
            img = swirl(img, s_[0], s_[1], rp, self.hooked_k(F.t), F.t, self.col)
        return img

    # ---------------------------------------------------------------- checks
    def focus_points(self, t, st):
        p = st['p']
        return [(S.to_world(p[0], p[1], R + st['lift']), 'bassoon')]


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.5, 0.25                                  # #39's mix
    band = (300, 1400)                                           # #39's band check: the staccato line's fundamentals
    colour_args = dict(cut0=40, cut_span=200, drive=4.0)         # #39's colour()

    def __init__(self, M, k):
        super().__init__(M, k)
        self.BNM = M.B.module('bassoon_lowpoly')                 # the Pixel Pitch bassoon's double reed (read-only)
        self._vc = {}

    def k0(self, off):
        """a pitch in the key: semitones above the tonic in the C2 octave (#39, E minor: 40 = E2)"""
        return self.T.KEY_ROOT + off

    def bsn(self, m, dur, vel=0.85, growl=0.0):
        """one bassoon note (staccato when short), dur s (#39's bsn: fitted, 20 ms fade, level-matched)"""
        key = (round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))
        if key not in self._vc:
            M, BNM = self.M, self.BNM
            x = M.fit(M.B._resample(np.asarray(BNM.bassoon_reed(BNM._midi_hz(m), dur, vel, growl=growl), float)),
                      dur + 0.09, fade=0.02)
            self._vc[key] = x * (0.16 / (np.sqrt((x[:int(0.12 * M.SR)] ** 2).mean()) + 1e-9))
        return self._vc[key]

    def voice(self, m, dur, vel=0.85):
        return self.bsn(m, dur, vel)

    def sour(self, m, dur, vel, t, growl=0.0):
        """the bassoon as the run has it at t: while OUT OF TUNE flat by up to a quarter-tone with a beating sour twin
        (nothing detunes it in #39, so there it is the plain note)"""
        dep = self.detune(t)
        if dep <= 0:
            return self.bsn(m, dur, vel, growl)
        x = self.bsn(m - 0.5 * dep, dur, vel, growl)
        if dep > 0.25:
            x = x + 0.32 * dep * self.bsn(m - 0.5 * dep - 0.22, dur, vel * 0.9, growl)
        return x

    def note(self, m, dur, vel, t, drive_k=1.0, growl=0.0):
        return self.colour(self.sour(m, dur, vel, t, growl), t, drive_k)

    def title_blast(self, mix, t):
        """a comic low blat sliding up, then a dot an octave up"""
        hits = self.T.TITLE_HITS.get(self.name, (self.k0(0), self.k0(12)))
        mix.place(self.bus, self.M.gliss(self.bsn(hits[0], 0.35, 1.0, growl=0.25), -3.0, 0.08), t, 0.48)
        mix.place(self.bus, self.bsn(hits[-1], 0.12, 0.9), t + 0.20, 0.30)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, q16, S16 = M.A, M.q16, T.S16
        dropped = False
        last_slot = None
        last_played = -9.0
        mk_i = 0
        mk_hops = [e for e in M.EV if e[1] == 'hop' and e[2] == k and e[8] == 2]
        for e in M.EV:                                         # a staccato dot per hop landing
            t = e[0]
            if t > M.T_KO + 0.01:
                break
            if e[1] != 'stomp' or e[2] != k:
                continue
            hit, back, kind = e[5], e[6], e[8]
            tq = q16(t)
            slot = round(tq / S16)
            if kind == 2:                                    # MOUNTAIN KING: Grieg's theme, one note per hop
                m = self.k0(MK_BASE + MK_THEME[min(mk_i, len(MK_THEME) - 1)])
                nxt = mk_hops[mk_i + 1][0] if mk_i + 1 < len(mk_hops) else t + T.BEAT / 2
                mk_i += 1
                dur = float(np.clip((nxt - t) * 0.7, 0.06, 0.2))
                sig, g, _ = self.note(m, dur, 0.95, t, 0.5)
                mix.place(self.bus, sig, t, 0.50 * g, pan=-0.2)
                mix.place(self.bus, self.note(m - 12, dur, 0.7, t, 0.5)[0], t, 0.18 * g, pan=-0.2)
                continue
            if slot == last_slot:
                continue
            last_slot = slot
            hp = M.hp_at(k, t) / 100.0
            dropped = M.thin_37(slot, 3, hp, dropped) and tq - last_played < 0.5   # sparser when hurt
            if dropped:
                continue
            last_played = tq
            m = T.line_at(self.name, tq)
            if slot % 2 == 1:                                # STACCATO's 16th landings: a passing note
                nxt = T.line_at(self.name, tq + S16)
                pas = [x for x in T.scale_notes(min(m, nxt), max(m, nxt), T.chord_at(tq)) if x not in (m, nxt)]
                m = pas[len(pas) // 2] if pas else nxt
            vel = 0.92 if back else (0.8 if hit else 0.6)
            sig, g, lead = self.note(m, 0.085, vel, t)
            mix.place(self.bus, sig, tq, (0.46 if hit else 0.30) * g, pan=-0.22)
            if lead > 0.05 and hit:                          # the leader: a sixth under (fuller)
                under = T.chord_tones(m - 9, m - 5, T.chord_at(tq))
                if under:
                    mix.place(self.bus, self.note(under[0], 0.085, 0.6, t)[0], tq, 0.16 * g, pan=-0.3)
            if back:                                         # a backstab: a grace up the chord, the stab
                up_ = [x for x in T.chord_tones(m + 3, m + 9, T.chord_at(tq))]
                if up_:
                    mix.place(self.bus, self.note(up_[0], 0.06, 0.9, t)[0], tq + 0.035, 0.26 * g, pan=-0.15)
                mix.place('fx', A.whoosh(0.07, 2000, 9000), tq - 0.03, 0.05)
        ab = M.CFG[k]['abilities']
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'hook_open' and e[2] == k:            # BOCAL HOOK: a comic blat sliding up as it swings out
                sig, g, _ = self.note(self.k0(6), 0.16, 1.0, t, growl=0.3)
                mix.place(self.bus, M.gliss(sig, 4.0, 0.12), q16(t), 0.40 * g, pan=-0.2)
            elif kind == 'hook' and e[2] == k:
                tq = q16(t)
                mix.place('fx', self.clank(91, int(t * 7)), tq, 0.16 if e[3] else 0.08, pan=0.1)
                if e[3]:
                    mix.place('fx', A.whoosh(0.3, 800, 6000), tq, 0.10)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[3] == ab['unlock1']['name']:            # STACCATO: a quick staccato run up
                    for j, off in enumerate((24, 27, 31, 36)):
                        mix.place(self.bus, self.note(self.k0(off), 0.06, 0.9, t)[0], tq + j * S16, 0.30, pan=-0.2)
                elif e[3] == ab['low']['name']:              # MOUNTAIN KING: a crash as the dance starts
                    mix.place('drums', A.crash(1.6), tq, 0.08)

    def clank(self, m, seed=0):
        """#39's clank: a key or the crook hitting something, a short metallic ring"""
        A = self.M.A
        n = int(0.3 * A.SR)
        t = A.tt(n)
        f = A.mtof(m)
        x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.09), (2.76, 0.045), (5.4, 0.02)))
        return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)

    def fall_apart(self, mix):                               # a falling blat, keys clattering
        M = self.M
        x = self.bsn(self.k0(12), 1.0, 1.0, growl=0.4)
        mix.place(self.bus, M.bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), M.P.T_HS, 0.42)
        for j, m in enumerate((86, 81, 91, 77, 84)):
            mix.place('fx', self.clank(m, 20 + j), M.P.T_HS + 0.08 * j, 0.07, pan=(j - 2) / 5)

    def stinger(self, mix, tw):
        for j, off in enumerate((24, 27, 31, 36)):
            mix.place('solo', self.bsn(self.k0(off), 0.08, 1.0), tw + j * self.T.S16, 0.42)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.bsn(m, max(0.07, d * T.BEAT * 0.6), 0.95), ts, 0.52)

    def reassemble(self, mix):
        T, M = self.T, self.M
        run = [self.k0(o) for o in (24, 27, 31, 34, 36, 39)]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.bsn(m, 0.08, 0.7), tj, 0.24)
