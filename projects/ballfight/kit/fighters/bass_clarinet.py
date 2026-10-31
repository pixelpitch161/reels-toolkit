"""BASS CLARINET (slate blue), as built in #38 (Bass Clarinet vs Euphonium) and fought again unchanged in #40 (Bari Sax vs
Bass Clarinet): ENDPIN ANCHOR (every bar it drives its floor peg into the canvas and whirls round it on its long black
body like a hammer-thrower; the orbit grows per landed smash), SLAP TONGUE (the reed slaps out like a paddle: a heavy
knockback smack), MULTIPHONICS (a ghost twin whirls on the far side of the peg), LIGHTS OUT (below 25 HP: the arena dims,
it sinks into a shadow pool for a bar, then erupts).

Look: the silver endpin rod down to the peg (black collar, a glint where it bites), the swept trail of the whirl on the
canvas + the peg's dent, MULTIPHONICS' translucent twin and its ghost rod, SLAP TONGUE's cane paddle, LIGHTS OUT's
shadow pool, the sinking ball (kit draw_ball's optional `sink` key), the eased arena dim (soft keep circles round both
balls; never black, never a flash) and the cracked eruption, grenadilla / key / neck / bell shards. Draw code from
projects/ballfight_bassclarinet_vs_euphonium/styles.py (#38), copied here unchanged; the render layers from #40's
render.py (projects/ballfight_barisax_vs_bassclarinet), its latest episode, as Look hooks.
Sound (#40's audio.py, unchanged): the chalumeau growl. The peg going in plays the bar's root with a growl, every 8th of a
whirl plays its line (config LINES, 8 a bar), it growls a held fifth as it rolls on, a smash pops a slap tongue on the
line's pitch; MULTIPHONICS adds the twelfth (two notes at once) and opens with a split, buzzing chord; SLAP TONGUE the
big slap; LIGHTS OUT a low growl swelling in the dark, the eruption a huge slap over a thunder thud. Key-bound notes
follow the config's CHORDS (#40's G minor gives #40's notes). Its octave follows its role: #40 played a counter-line an
octave over the bari's bass (role 'counter', anything but 'bass'); role 'bass' plays the same line down at the bass
(#55: the growling bass line), keeps its fundamental as it thins, and pumps the root into the title slam (last_bar).

Ported for #55 Bass Clarinet vs Bassoon. Check config: examples/barisax_vs_bassclarinet (#40 v2, seed 128)."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL, ss
import scene as S

from . import base

R = S.R_BALL

# ---------------------------------------------------------------- #38 styles.py (the BASS CLARINET), unchanged
INK = np.array([18, 10, 26], float)
WHITE = np.array([255, 252, 240], float)
BLACK = np.array([30, 26, 32], float)            # grenadilla
NICKEL = np.array([214, 216, 228], float)
CANE = np.array([232, 210, 152], float)


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


def wp(cam, x, z, y):
    return cam.wp(S.to_world(np.atleast_1d(x), np.atleast_1d(z), np.atleast_1d(y)))


def whirl_light(sxp, szp, c, r, th, om, R, age):
    """canvas decal: the swept trail behind the whirling arm (the circle it smashes through), brightest just behind the
    arm, fading back over ~200 deg; a bright ring at the ball's path. Returns weights 0..1."""
    dx, dz = sxp - c[0], szp - c[1]
    d = np.hypot(dx, dz)
    a = np.arctan2(dz, dx)
    sgn = 1.0 if om >= 0 else -1.0
    behind = ((th - a) * sgn) % (2 * math.pi)                  # 0 at the arm, growing backward
    trail = np.clip(1 - behind / math.radians(200), 0, 1) ** 1.6
    disc = np.clip(1 - (d - (r + R * 0.9)) / 0.12, 0, 1) * (d < r + R + 0.12)
    path = np.clip(1 - np.abs(d - r) / (R * 0.75), 0, 1)
    k = min(1.0, age / 0.12)
    return (0.45 * disc * trail + 0.95 * path * trail) * k


def peg_light(sxp, szp, c, age):
    """canvas decal: the spot where the peg went in - a dark dent with a bright ring, fresh for ~0.3 s"""
    d = np.hypot(sxp - c[0], szp - c[1])
    dent = np.clip(1 - d / 0.13, 0, 1)
    ring = np.clip(1 - np.abs(d - (0.14 + 0.9 * min(age, 0.25))) / 0.05, 0, 1) * max(0.0, 1 - age / 0.3)
    return dent, ring


def draw_arm(img, cam, c, p, R, col, ghost=False, k=1.0):
    """the endpin: a bright silver rod out of the ball's rim down to the peg driven into the canvas at c (the ball's
    long body is the hammer, the peg the pivot); a slate rim light, the black rubber collar at the canvas and a white
    glint where it bites in. ghost = MULTIPHONICS' twin (pale, translucent)."""
    d = np.asarray(c, float) - np.asarray(p, float)
    L = float(np.hypot(*d))
    u_ = d / L if L > 1e-6 else np.array([1.0, 0.0])
    a = np.asarray(p, float) + u_ * R * 0.62                           # from the glass's edge ...
    n = 7
    w = np.linspace(0, 1, n)
    xs = a[0] + (c[0] - a[0]) * w
    zs = a[1] + (c[1] - a[1]) * w
    ys = R * 0.55 * (1 - w) + 0.02
    sp = wp(cam, xs, zs, ys)
    zc = cam.depth(S.to_world(p[0], p[1], R)[None])[0]
    width = max(2, int(round(0.075 * cam.f / max(1e-6, zc))))
    body = _mask([], [sp], width=width + 2)
    core = _mask([], [sp], width=width)
    hi = _mask([], [sp + np.array([0, -1.0])], width=1)
    if ghost:
        img = blend(img, body, np.clip(col * 0.4 + 150, 0, 255), 0.45 * k)
        img = blend(img, hi, WHITE, 0.5 * k)
        return img
    img = blend(img, body, INK, 0.9 * k)                                   # dark outline
    img = blend(img, core, NICKEL, 0.95 * k)                               # the silver rod
    img = blend(img, hi, np.clip(col * 0.4 + 170, 0, 255), 0.9 * k)       # slate-lit top edge
    collar = wp(cam, [xs[-2], xs[-1]], [zs[-2], zs[-1]], [ys[-2], ys[-1]])
    img = blend(img, _mask([], [collar], width=width + 2), BLACK, 0.95 * k)
    base_ = wp(cam, c[0], c[1], 0.0)[0]
    xi, yi = int(base_[0]), int(base_[1])
    for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1)):
        if 0 <= xi + dx < W and 0 <= yi + dy < H:
            img[yi + dy, xi + dx] = img[yi + dy, xi + dx] * (1 - k) + WHITE * k
    return img


def draw_ghost_ball(img, cam, p, R, col, k):
    """MULTIPHONICS: the twin's ball, a translucent outline on the far side of the peg"""
    s = wp(cam, p[0], p[1], R)[0]
    rp = R * cam.f / max(1e-6, cam.depth(S.to_world(p[0], p[1], R)[None])[0])
    a = np.linspace(0, 2 * math.pi, 25)
    ring = np.stack([s[0] + rp * np.cos(a), s[1] + rp * np.sin(a)], 1)
    disc = _mask([ring])
    edge = _mask([], [ring], width=1)
    img = blend(img, disc, np.clip(col * 0.35 + 150, 0, 255), 0.22 * k)
    img = blend(img, edge, np.clip(col * 0.5 + 130, 0, 255), 0.75 * k)
    return img


def draw_slap(img, cam, p, aim, R, ext, open_k, hit_k, col):
    """the reed as a paddle: a broad, flat cane blade on a black mouthpiece shoots out of the ball at the enemy and slaps
    (open_k: wind-up 0..1, the paddle cocked back; ext: reach beyond the surface; hit_k: the slap's flash 1..0)"""
    d = np.array([math.cos(aim), math.sin(aim)])
    nrm = np.array([-d[1], d[0]])
    base_ = p + d * (R * 0.85)
    tip = p + d * (R + ext)
    cock = nrm * 0.18 * open_k                                             # wound back sideways before it slaps
    mid = base_ + (tip - base_) * 0.35 + cock * 0.5
    hw0, hw1 = 0.05, 0.16
    q = [base_ + nrm * hw0, mid + nrm * hw0 * 1.2, tip + cock + nrm * hw1, tip + cock - nrm * hw1, mid - nrm * hw0 * 1.2,
         base_ - nrm * hw0]
    sp = wp(cam, [a[0] for a in q], [a[1] for a in q], [R] * len(q))
    m = _mask([sp])
    img = blend(img, m, CANE, 0.95)
    stem = wp(cam, [base_[0], mid[0]], [base_[1], mid[1]], [R, R])
    img = blend(img, _mask([], [stem], width=2), BLACK, 0.95)
    edge = _mask([], [np.vstack([sp, sp[:1]])], width=1)
    img = blend(img, edge, np.clip(col * 0.6 + 60, 0, 255), 0.8)
    if hit_k > 0:
        img = blend(img, m, WHITE, 0.7 * hit_k)
    return img


def shadow_pool(sxp, szp, p, R, k):
    """canvas decal: the dark pool the bass clarinet slides in while it is sunk into shadow"""
    d = np.hypot(sxp - p[0], szp - p[1])
    return np.clip(1 - d / (R * 1.5), 0, 1) ** 0.7 * k


def erupt_light(sxp, szp, c, age, radius):
    """canvas decal: the eruption - a cracked ring racing out to the radius, a hot centre"""
    if not 0 <= age < 0.6:
        return np.zeros_like(sxp)
    d = np.hypot(sxp - c[0], szp - c[1])
    r = radius * min(1.0, age / 0.22) ** 0.6
    ring = np.clip(1 - np.abs(d - r) / 0.16, 0, 1)
    core = np.clip(1 - d / 0.6, 0, 1) * max(0.0, 1 - age / 0.25)
    return (ring + core) * (1 - age / 0.6)


# ---------------------------------------------------------------- #40 render.py's colours
WOOD = np.array([34, 26, 30], float)                 # grenadilla (the body's shards)
SHARD_NICKEL = np.array([214, 216, 226], float)      # (#40's render NICKEL: keys, neck, bell)
SLATE_SPARK = [(196, 210, 236), (110, 132, 170), (240, 244, 255)]
CHIPS = [(40, 34, 38), (214, 216, 226), (24, 20, 24)]
DUST_PEG = [(170, 160, 190), (130, 122, 150)]


class Look(base.Look):
    shards = ['wood'] * 6 + ['key'] * 8 + ['neck'] * 2 + ['bell'] * 2
    spark_big = ('snip', 'erupt')
    flash_hits = ('erupt', 'snip', 'anchor')          # #40's FLASH_EV kinds
    fringe_hits = ('snip', 'erupt', 'anchor')          # (an anchor smash only on its first contact: fringe())
    rope_kinds = {}                                    # only the eruption shakes the ropes (rope())

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.ANCH = [(e[0], e[2], np.array([e[3], e[4]]), e[5], e[6]) for e in EV if e[1] == 'anchor' and e[2] == k]
        self.LOUT = [(e[0], e[2], e[3]) for e in EV if e[1] == 'lights_out' and e[2] == k]
        self.ERUPTS = [(e[0], e[2], np.array([e[3], e[4]]), e[5]) for e in EV if e[1] == 'erupt' and e[2] == k]
        self.TWINS = [(e[0], e[2], e[3]) for e in EV if e[1] == 'twin' and e[2] == k]
        self.SNIP_OPEN = [(e[0], e[2], e[3], e[4]) for e in EV if e[1] == 'snip_open' and e[2] == k]
        self.SLAP_AB = self.cfg['abilities']['unlock1']
        self.LOW = self.cfg['abilities']['low']
        self.anchored = self.cfg['weapon']['kind'] == 'anchor' and 'ANC' in P.RUN

    # ---------------------------------------------------------------- state (#40's sink_at / twin_at / anc_at)
    def sink_at(self, t):
        """LIGHTS OUT: how far it has sunk into shadow (0..1): eased down over 0.3 s, popping up at the eruption"""
        for t0, kk, te in self.LOUT:
            if t0 <= t < te + 0.16 and t0 < self.P.T_KO:
                if t < te:
                    return ss(t0, t0 + 0.3, t)
                return 1 - ss(te, te + 0.12, t)
        return 0.0

    def dim_at(self, t):
        """LIGHTS OUT's arena dim: eased in over 0.35 s, held, eased back over 0.35 s after the eruption (<= 0.40)"""
        d = 0.0
        for t0, kk, te in self.LOUT:
            if t0 < self.P.T_KO and t0 <= t < te + 0.4:
                d = max(d, 0.40 * ss(t0, t0 + 0.35, t) * (1 - ss(te + 0.05, te + 0.4, t)))
        return d

    def twin_at(self, t):
        for t0, kk, until in self.TWINS:
            if t0 <= t < until and t0 < self.P.T_KO:
                return min(1.0, (t - t0) / 0.15) * (1 - ss(until - 0.15, until, t))
        return 0.0

    def anc_at(self, t):
        """the whirl at t: (peg (x, z), th, r, twin, age, om) or None (from the recorded ANC)"""
        P, k = self.P, self.k
        if not (self.T.T_FIGHT <= t < P.T_KO) or not self.anchored:
            return None
        ANC = P.RUN['ANC']
        row = ANC[P.sim_i(t), k]
        if np.isnan(row[0]):
            return None
        t0 = max([a[0] for a in self.ANCH if a[0] <= t] + [self.T.T_FIGHT])
        i0 = P.sim_i(t)
        th_prev = ANC[max(0, i0 - 1), k][3]
        om = (row[3] - th_prev) * P.FPS if not np.isnan(th_prev) else 1.0
        return np.array([row[1], row[2]]), float(row[3]), float(row[4]), float(row[5]), t - t0, om

    def snip_state(self, t):
        """(aim, ext, wind-up, slap flash) of SLAP TONGUE's paddle at t, or None"""
        P, k = self.P, self.k
        for t0, kk, t_snap, aim in self.SNIP_OPEN:
            if t0 <= t < t_snap + 0.22 and t0 < P.T_KO:
                i = P.sim_i(min(t, P.T_KO))
                row = P.RUN['SNIP'][i, k]
                a = float(row[1]) if not np.isnan(row[1]) else aim
                if t < t_snap:
                    u = (t - t0) / max(1e-6, t_snap - t0)
                    return a, 0.15 + 0.1 * u, u, 0.0
                u = (t - t_snap) / 0.22
                ext = self.SLAP_AB['reach'] * (math.sin(math.pi * min(1.0, u * 1.6)) if u < 0.6 else (1 - u) / 0.4 * 0.6)
                return a, max(0.12, ext), 0.0, max(0.0, 1 - u / 0.4)
        return None

    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k:
            return
        st['sink'], st['twin'], st['anc'] = 0.0, 0.0, None
        if phase != 'fight':
            return
        st['sink'] = self.sink_at(t)
        st['twin'] = self.twin_at(t)
        st['anc'] = self.anc_at(t)
        if st['anc'] is not None:
            st['glow'] = max(st['glow'], 0.35 * max(0.0, 1 - st['anc'][4] / 0.4))

    # ---------------------------------------------------------------- events -> camera, ropes, sparks
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('anchor', 'snip', 'erupt'):
            return {'anchor': 1.1 if e[8] == 0 else 0.7, 'snip': 1.3, 'erupt': 2.4}[e[7]]
        if e[1] == 'anchor':
            return 0.5                                            # the peg going in
        return None

    def rope(self, e):
        return 1.0 if e[1] == 'erupt' else None

    def spark(self, e):
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how in self.spark_big else ((10, 2.6, 0.24) if how == 'anchor' else (7, 2.2, 0.22))
        return self.P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        if e[7] in ('snip', 'erupt'):
            return 14, 0.35
        if e[7] == 'anchor':
            return 8, 0.2
        return None

    def fringe(self, e):
        return e[7] in ('snip', 'erupt') or (e[7] == 'anchor' and e[8] == 0)

    # ---------------------------------------------------------------- effects (#40's _fx_build, in its order)
    def fx_setup(self):
        P, up, col = self.P, self.P.up, self.col
        for t_, k, c, orbit, te in self.ANCH:                     # ENDPIN ANCHOR: the peg goes in
            if t_ > P.T_KO:
                break
            P.FL.add(t_, up(c[0], c[1], 0.3), col, radius=1.8, peak=0.5, decay=0.3)
            P.PA.emit(t_, up(c[0], c[1], 0.05), 'dust', n=8, col=DUST_PEG)
            P.MK.add(t_, 'crack', c[0], c[1], size=0.32, angle=t_ * 3.0, col=(56, 48, 72), life=3.5, n=3,
                     seed=int(t_ * 10), strength=0.7)
        for t_, k, p, hit in self.ERUPTS:                         # LIGHTS OUT erupts
            if t_ > P.T_KO:
                break
            pos = up(p[0], p[1], 0.2)
            P.BU.add(t_, pos, col * 0.6 + 100, 1.8)
            P.FL.add(t_, pos, col, radius=2.6, peak=1.2, decay=0.45)
            P.PA.emit(t_, pos, 'dust', n=22, spread=180, col=[(170, 160, 190), (110, 100, 130)], speed=(1.5, 3.2))
            P.PA.emit(t_, pos, 'chips', n=10, col=[(60, 52, 76), (110, 132, 170)])
            P.MK.add(t_, 'crack', p[0], p[1], size=1.2, angle=0.4, col=(40, 34, 56), life=6.0, n=6, seed=int(t_ * 10),
                     strength=0.85)

    def fx_event(self, e):
        if e[1] != 'hit':
            return
        P = self.P
        att, x, z, how = e[2], e[5], e[6], e[7]
        pos = P.up(x, z, R)
        if how == 'anchor':                                       # the whirling body smashes in
            P.BU.add(e[0], pos, P.COL[att] * 0.6 + 100, 1.1 if e[8] == 0 else 0.8)
            P.FL.add(e[0], pos, P.COL[att], radius=1.6, peak=0.7, decay=0.25)
            P.PA.emit(e[0], pos, 'sparks', n=10, col=SLATE_SPARK)
        elif how == 'snip':                                       # SLAP TONGUE: a cane smack
            P.BU.add(e[0], pos, (240, 220, 160), 1.3)
            P.FL.add(e[0], pos, P.COL[att], radius=1.8, peak=0.9, decay=0.3)
            P.PA.emit(e[0], pos, 'chips', n=10, col=[(232, 210, 152), (196, 168, 110)])
        elif how == 'erupt':
            P.BU.add(e[0], pos, (240, 236, 255), 1.4)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'sparks', n=26, spread=180, speed=(0.8, 2.2), col=CHIPS)

    # ---------------------------------------------------------------- the canvas
    def canvas(self, Ic, C):
        """the whirl's swept trail (+ the twin's, MULTIPHONICS) and the peg's bright ring, LIGHTS OUT's eruption"""
        if not C.in_fight:
            return Ic
        k, col = self.k, self.col
        st = C.fs[k]
        if st.get('anc') is not None:
            c, th, r, tw, age, om = st['anc']
            w_ = whirl_light(C.sxp, C.szp, c, r, th, om, R, age)
            if tw > 0.5:
                w_ = w_ + 0.6 * whirl_light(C.sxp, C.szp, c, r, th + math.pi, om, R, age)
            w_ = np.floor(w_ * 4 + BAYER_FULL[C.ys, C.xs] * 0.9) / 4
            Ic += (w_ * 0.9)[:, None] * (col / 255.0 * 1.2 + 0.25)[None]
            dent, ring = peg_light(C.sxp, C.szp, c, age)
            Ic += (ring * 0.8)[:, None] * (col / 255.0 + 0.3)[None]
        for t_, kk, p, hit in self.ERUPTS:
            if t_ <= C.t < t_ + 0.6:
                e_ = erupt_light(C.sxp, C.szp, p, C.t - t_, self.LOW['radius'])
                e_ = np.floor(e_ * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                Ic += (e_ * 1.2)[:, None] * (col / 255.0 * 1.2 + 0.3)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """the peg's dent and LIGHTS OUT's shadow pool darken the canvas last (#40 did the darkening after every light)"""
        if not C.in_fight:
            return Ic
        st = C.fs[self.k]
        shade_c = None
        if st.get('anc') is not None:
            dent, ring = peg_light(C.sxp, C.szp, st['anc'][0], st['anc'][4])
            shade_c = dent * 0.6
        if st.get('sink', 0.0) > 0.02:
            pool = shadow_pool(C.sxp, C.szp, st['p'], R, st['sink']) * 0.75
            shade_c = pool if shade_c is None else np.maximum(shade_c, pool)
        if shade_c is not None:
            Ic *= (1 - shade_c)[:, None]
        return Ic

    # ---------------------------------------------------------------- over the ring: the endpin, the twin, the paddle
    def over_ring(self, F, arr, depth):
        if not F.in_fight:
            return arr, depth
        st = F.fs[self.k]
        if st.get('anc') is not None and st['vis']:
            c, th, r, tw, age, om = st['anc']
            k_in = min(1.0, age / 0.06)
            if tw > 0.5:                                          # MULTIPHONICS: the twin, opposite the peg
                q = c + r * np.array([math.cos(th + math.pi), math.sin(th + math.pi)])
                arr = draw_ghost_ball(arr, F.cam, q, R, self.col, st['twin'] * k_in)
                arr = draw_arm(arr, F.cam, c, q, R, self.col, ghost=True, k=k_in * st['twin'])
            arr = draw_arm(arr, F.cam, c, st['p'], R, self.col, k=k_in)
        sn = self.snip_state(F.t)
        if sn is not None and st['vis']:
            a_, ext, wind, flash = sn
            arr = draw_slap(arr, F.cam, st['p'], a_, R, ext, wind, flash, self.col)
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind in ('wood', 'neck'):                              # a stick of the body / the neck
            ln_, wd_ = {'wood': (0.30, 0.04), 'neck': (0.16, 0.03)}[kind]
            q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], WOOD if kind == 'wood' else SHARD_NICKEL, 1.0)
            return arr
        sz = size * (0.7 if kind == 'key' else 1.6)               # key (a chip), bell (a big curved chip)
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(SHARD_NICKEL * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, LIGHTS OUT's dim
    def shimmer_after(self, img, cam, t):
        from reelkit import fx
        for t_, k, p, hit in self.ERUPTS:                         # the eruption's heat ripple
            u = t - t_
            if 0 <= u < 0.5 and t_ < self.P.T_KO:
                img = fx.shimmer_world(img, cam, S.to_world(p[0], p[1], 0.05), 0.4 + 2.2 * u / 0.5, 0.4, 3.0, t,
                                       k=1 - u / 0.5, seed=k)
        return img

    def after_grade(self, img, F, drawn):
        """LIGHTS OUT: the whole arena eases darker (<= 40 %); both balls stay readable (soft keep circles round them)"""
        d = self.dim_at(F.t)
        if d <= 0:
            return img
        P = self.P
        keep = np.zeros((H, W), np.float32)
        for k, (s_, rp) in drawn.items():
            r = rp * 1.8
            x0, x1 = int(max(0, s_[0] - r)), int(min(W, s_[0] + r + 1))
            y0, y1 = int(max(0, s_[1] - r)), int(min(H, s_[1] + r + 1))
            if x1 > x0 and y1 > y0:
                dd = np.hypot(P.XX_[y0:y1, x0:x1] - s_[0], P.YY_[y0:y1, x0:x1] - s_[1]) / r
                keep[y0:y1, x0:x1] = np.maximum(keep[y0:y1, x0:x1], np.clip(1.4 - dd * 1.4, 0, 1))
        for L in P.LOOKS:                                         # anything another look asks to keep lit (a crane line)
            extra = getattr(L, 'dim_keep', None)
            if extra is not None and L is not self:
                keep = np.maximum(keep, extra(F))
        return img * (1 - d * (1 - keep))[..., None]

    def focus_points(self, t, st):
        if st.get('anc') is not None:
            c = st['anc'][0]
            return [(S.to_world(c[0], c[1], 0.0), 'peg')]
        return []


# ===================================================================================================== sound (#40)
_VC = {}
_MODS = {}


def _pp(name):
    if name not in _MODS:
        import pp_bridge as B
        _MODS[name] = B.module(name)
    return _MODS[name]


class Sound(base.Sound):
    bus_gain, send = 1.5, 0.25                                    # #40's BUS_GAIN['bcl'] and its reverb send
    band = (150, 900)                                             # #40's band check (the chalumeau growl)
    colour_args = dict(cut0=40, cut_span=200, drive=4.0)          # #40's colour() for the bass clarinet

    def __init__(self, M, k):
        super().__init__(M, k)
        self.oct = 0 if self.role == 'bass' else 12               # #40's counter-line sat an octave over the bass
        if self.role == 'bass':                                   # #55: the bass line keeps its fundamental
            self.band = (60, 500)
            self.colour_args = dict(cut0=40, cut_span=200, drive=4.0, low=True, low_cut=(40, 70))

    # ---------------------------------------------------------------- its voice (#40's bcl() / slap(): level-matched)
    def bcl(self, m, dur, vel=0.85, growl=0.0):
        key = ('b', round(m, 3), round(dur, 3), round(vel, 2), round(growl, 2))
        if key not in _VC:
            import pp_bridge as B
            from reelkit import audio as A
            BCM = _pp('bassclarinet_lowpoly')
            x = BCM.bass_reed(BCM._midi_hz(m), dur, vel, growl=growl)
            x = np.asarray(B._resample(np.asarray(x, float)), float)
            n = int((dur + 0.16) * A.SR)                          # #40's _fit (cap = dur + 0.16, 20 ms fade)
            x = x[:n].copy()
            if len(x) < n:
                x = np.pad(x, (0, n - len(x)))
            fd = min(len(x) // 3, int(0.02 * A.SR))
            x[-fd:] *= np.linspace(1, 0, fd)
            r = np.sqrt((x[:int(0.12 * A.SR)] ** 2).mean()) + 1e-9   # #40's _norm (ref 0.16)
            _VC[key] = x * (0.16 / r)
        return _VC[key]

    def slap(self, m, vel=1.0):
        key = ('s', round(m, 2), round(vel, 2))
        if key not in _VC:
            import pp_bridge as B
            BCM = _pp('bassclarinet_lowpoly')
            _VC[key] = B._resample(np.asarray(BCM.slap(BCM._midi_hz(m), vel), float)) * 0.9
        return _VC[key]

    def voice(self, m, dur, vel=0.85):
        return self.bcl(m, dur, vel)

    # ---------------------------------------------------------------- the key (#40: G minor, register 34-62)
    def root(self, t):
        """the bar's root for its part: the chord root at the bottom of its register, an octave up unless it is the bass
        (#40: G3 Eb3 C3 D3)"""
        lo = self.M.CFG[self.k]['register'][0]
        return self.T.root_above(self.T.chord_at(t), lo) + self.oct

    def tonic(self):
        """the title card's chord root in its register (#40: G3 = 55)"""
        lo = self.M.CFG[self.k]['register'][0]
        return self.T.root_above(self.T.CHORDS[0], lo) + 12

    def line_at(self, t):
        return self.T.line_at(self.name, t)

    def twin_on(self, t):
        return any(e[1] == 'twin' and e[2] == self.k and e[0] <= t < e[3] for e in self.M.EV)

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """a growl + its fifth + a slap (#40: G3, D4, the slap on G4)"""
        t0 = self.tonic()
        hits = self.T.TITLE_HITS.get(self.name, (t0, t0 + 7, t0 + 12))
        mix.place(self.bus, self.bcl(hits[0], 0.9 * self.T.BEAT, 1.0, growl=0.7), t, 0.50, pan=-0.2)
        mix.place(self.bus, self.bcl(hits[1], 0.9 * self.T.BEAT, 0.8, growl=0.4), t, 0.22, pan=-0.2)
        mix.place(self.bus, self.slap(hits[2], 1.0), t + 0.004, 0.50, pan=-0.2)

    def play(self, mix):
        """#40's bcl_part: the peg's root, 8ths through every whirl, slap pops on the smashes, the abilities"""
        M, T, k = self.M, self.T, self.k
        A, EV, q16 = M.A, M.EV, M.q16
        look = M.P.LOOKS[k]
        ANCH = look.ANCH
        if not ANCH:
            return
        dropped = False
        first = min(a[0] for a in ANCH)                           # FIGHT!: it rolls out of its corner, growling
        sig, g, _ = self.colour(self.bcl(self.root(T.T_FIGHT), first - T.T_FIGHT - 0.05, 0.75, growl=0.2), T.T_FIGHT)
        mix.place(self.bus, sig, T.T_FIGHT, 0.26, pan=-0.2)
        for t0, kk, c, orbit, te in ANCH:
            if t0 > M.T_KO + 0.01:
                break
            tq = q16(t0)
            root = self.root(tq)                                  # the peg goes in: the bar's root, growling
            sig, g, lead = self.colour(self.bcl(root, 0.9 * T.BEAT, 0.95, growl=0.35), t0)
            mix.place(self.bus, sig, tq, 0.46 * g, pan=-0.2)
            mix.place('fx', M.clank(84, int(t0 * 10)), tq, 0.05, pan=-0.2)
            tr = q16(te)                                          # it pulls the peg and rolls on: a held growl (fifth)
            nxt = min([a[0] for a in ANCH if a[0] > t0 + 0.1] + [M.T_KO])
            if nxt - tr > 0.3:
                sig, g, _ = self.colour(self.bcl(self.root(tr) + 7, min(nxt - tr, 2.2 * T.BEAT) - 0.05, 0.7,
                                                 growl=0.1), tr)
                mix.place(self.bus, sig, tr, 0.24 * g, pan=-0.2)
            e8 = q16(t0 + T.BEAT / 2)
            while e8 < te - 0.02 and e8 < M.T_KO:                 # the rumble through the whirl, on the 8ths
                slot = round(e8 / T.S16)
                hp = M.hp_at(k, e8) / 100.0
                dropped = M.thin_37(slot, 3, hp, dropped)            # (#40's thin() = the kit's #37 one)
                if not dropped:
                    m = self.line_at(e8)
                    sig, g, lead = self.colour(self.bcl(m, 0.42 * T.BEAT, 0.8, growl=0.25), e8)
                    mix.place(self.bus, sig, e8, 0.42 * g, pan=-0.2)
                    if self.twin_on(e8):                          # MULTIPHONICS: the twelfth above at once
                        mix.place(self.bus, self.colour(self.bcl(m + 19, 0.42 * T.BEAT, 0.6), e8)[0], e8, 0.16 * g,
                                  pan=-0.1)
                    if lead > 0.04 and slot % 4 == 0:             # the leader: the octave under (fuller)
                        mix.place(self.bus, self.colour(self.bcl(m - 12, 0.42 * T.BEAT, 0.6), e8)[0], e8, 0.10 * g)
                e8 += T.BEAT / 2
        multi = self.M.CFG[k]['abilities']['unlock2']['name']
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'hit' and e[2] == k and e[7] == 'anchor':  # a smash: a slap-tongue pop on the line's pitch
                tq = q16(t)
                mix.place(self.bus, self.colour(self.slap(self.line_at(tq) + 12, 1.0), t)[0], tq, 0.40, pan=-0.15)
                mix.place('fx', A.thud(0.3, 150, 55, seed=int(t * 9)), tq, 0.18)
            elif kind == 'snip' and e[2] == k:                    # SLAP TONGUE: the big slap
                tq = q16(t)
                mix.place(self.bus, self.slap(self.root(tq) + 12, 1.0), tq, 0.55 if e[3] else 0.32, pan=-0.15)
                if e[3]:
                    mix.place('fx', A.thud(0.35, 180, 60, seed=int(t * 7)), tq, 0.30)
            elif kind == 'snip_open' and e[2] == k:               # the tongue sticks to the reed: a tiny click
                mix.place(self.bus, self.slap(84, 0.3), q16(t), 0.10)
            elif kind == 'ability' and e[2] == k and e[3] == multi:
                tq = q16(t)                                       # a split, buzzing chord: root + twelfth + 11th
                root = self.root(tq)
                for m, g_ in ((root, 0.34), (root + 19, 0.20), (root + 17.5, 0.11)):
                    mix.place(self.bus, self.bcl(m, 1.5 * T.BEAT, 0.9, growl=0.2), tq, g_, pan=-0.2)
            elif kind == 'lights_out' and e[2] == k:              # LIGHTS OUT: a low growl swelling in the dark
                tq = q16(t)
                x = self.bcl(T.KEY_ROOT + 3, e[3] - t, 1.0, growl=0.6)      # (#40: Bb2 in G minor)
                mix.place(self.bus, x * np.linspace(0.5, 1.2, len(x)), tq, 0.40, pan=-0.2)
                mix.place('fx', A.whoosh(0.8, 200, 4000, rise=False), tq, 0.18)
            elif kind == 'erupt' and e[2] == k:                   # ... and erupts
                tq = q16(t)
                mix.place(self.bus, self.slap(self.root(tq) + 12, 1.0), tq, 0.65, pan=-0.1)
                mix.place('fx', M.thunder(1.6, int(t * 10)), tq, 0.45)
                if hasattr(M.bed, 'erupt'):                       # (the bed's own crash: #40's sheet metal)
                    M.bed.erupt(mix, tq)

    def fall_apart(self, mix):                                    # a falling growl (#40)
        M = self.M
        x = self.bcl(self.tonic(), 1.0, 1.0, growl=0.5)
        mix.place(self.bus, M.bend(x, -9.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), M.P.T_HS, 0.42)

    def stinger(self, mix, tw):
        t0 = self.tonic()
        for j, m in enumerate((t0, t0 + 7, t0 + 12)):
            mix.place('solo', self.bcl(m, 0.9 - 0.06 * j, 1.0, growl=0.2), tw + j * 0.03, 0.30)
        mix.place('solo', self.slap(t0 + 12, 1.0), tw, 0.3)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.bcl(m, d * T.BEAT * 0.95, 0.95, growl=0.15), ts, 0.62)

    def reassemble(self, mix):
        T, M = self.T, self.M
        t0 = self.tonic()
        run = [t0 + d for d in (0, 3, 7, 10, 12, 15)]             # #40: G3 Bb3 D4 F4 G4 Bb4
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.bcl(m, 0.12, 0.7), tj, 0.24, pan=-0.2)

    def last_bar(self, mix):
        """as the bass (#55): it growls the root and fifth on the beats into the title slam (#40's bari pump); as a
        counter-line (#40) it rests here"""
        if self.role != 'bass':
            return
        T = self.T
        for b in (T.N_BARS - 2, T.N_BARS - 1):
            for bt in range(4):
                tb = b * T.BAR + bt * T.BEAT
                if tb >= T.DUR - T.BEAT / 2:
                    continue
                m = self.root(tb) + (7 if bt == 2 else 0)
                mix.place(self.bus, self.bcl(m, 0.42 * T.BEAT, 0.9, growl=0.4 if bt == 0 else 0.25), tb,
                          0.50 if bt == 0 else 0.40, pan=-0.2)

