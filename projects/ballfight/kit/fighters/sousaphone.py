"""SOUSAPHONE (marching crimson), new in #53 Flute vs Sousaphone: the only marcher. DRILL SET (no gliding: every 2 bars
a crisp straight drill path across the ring, 8 steps on the 8ths, trampling the enemy - bulldozed on ahead - and its
baskets, then a halt and a huge cone bell blast straight ahead; the blast grows per landed hit), HORN FLASH (the horn
spins 360 deg, the blast sweeping all round), DOUBLE TIME (it marches on the 16ths, a set every bar), COMPANY FRONT (a
full-width wall of ghost sousaphones marches rope to rope across the ring: Sousa's Stars and Stripes trio).

Look: the new Pixel Pitch sousaphone (sousaphone_lowpoly: built from Commons photos; white fiberglass bell, brass wrap;
x1.12 in the bridge, rendered with a wider crop). Each set chalks its drill line across the canvas (a dashed yard line)
and leaves a footprint at every step, the ball bobbing on each footfall; the halt plants it; the blast is a crimson-white
cone of light on the canvas with sound-wave arcs rolling out along it; a trample kicks up turf. HORN FLASH sweeps a lit
lane round the ball; COMPANY FRONT is a row of translucent crimson ghost sousaphones across the ring, stepping on every
beat behind a chalk front line. It shatters into fiberglass bell pieces, brass tubing and valve caps. VS card: the coil
and the huge bell, crimson-lit.
Sound: its footfalls ARE the bass line (role 'bass', config LINES: 8 a bar; the marching 8ths while it marches, quarter
notes on the beat while it marks time between sets), in the new punchy marching voice; the blast is a big accented blat
on the root with its octave; a landed trample adds a low thud; HORN FLASH a rip up into a blat; DOUBLE TIME adds 16th
approach notes; COMPANY FRONT plays the trio of The Stars and Stripes Forever (Sousa, 1896, public domain; from the
Mutopia Project's public-domain edition of the John Church piano score, bars 39-46) in low unison octaves, a half note a
beat, in the episode's key - and when the enemy's desperation move fires during it, the enemy's melody gives way to the
config's OBBLIGATO line (music.line_override, opt-in)."""
import math

import numpy as np
from reelkit.config import BAYER_FULL
import scene as S

from . import base
from ._reeds import _mask, blend, INK, WHITE
from .contrabass import padded_sprite

R = S.R_BALL
CHALK = np.array([238, 236, 226], float)
TURF = [(120, 150, 90), (90, 120, 70), (190, 180, 150)]
FIBER = np.array([240, 238, 230], float)
BRASS = np.array([226, 180, 84], float)
NICK = np.array([214, 216, 226], float)
PAD = 1.22                                                # the sprite's crop margin (the bridge's x1.12 size)
# The Stars and Stripes Forever, trio (Sousa, 1896, public domain), bars 39-46 of the Mutopia Project's edition of the
# John Church piano score (in A-flat): (half-note beat, semitones above the trio's tonic, half-note beats)
TRIO = [(0.0, 7, 1.0), (1.0, 5, 0.5), (1.5, 4, 0.5), (2.0, 4, 1.0), (3.0, 3, 0.5), (3.5, 4, 0.5),
        (4.0, 4, 3.0), (7.0, 3, 0.5), (7.5, 4, 0.5), (8.0, 4, 1.0), (9.0, 3, 0.5), (9.5, 4, 0.5),
        (10.0, 7, 1.0), (11.0, 4, 0.75), (11.75, 7, 0.25), (12.0, 5, 2.0), (14.0, 2, 1.0), (15.0, 2, 1.0)]


def title_pattern(base_):
    """SOUSAPHONE: the coiled wrap (rings round a centre) and the huge bell (a bright disc with a dark throat)"""
    import title as TT
    XX, YY = TT.XX, TT.YY
    u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
    cu = (u % 220.0) - 110.0
    cv = (np.abs(TT.SIDE) % 220.0) - 110.0
    r = np.hypot(cu, cv)
    coil = (np.abs(r - 70) < 9) | (np.abs(r - 50) < 5)
    bell = (np.hypot(cu - 26, cv + 30) < 40) & ~(np.hypot(cu - 26, cv + 30) < 18)
    out = np.where(coil[..., None], base_ * 0.35 + np.array([60, 20, 26]) * 0.65, base_)
    return np.where(bell[..., None], out * 0.3 + np.array([255, 236, 230]) * 0.7, out)


def _ghost(spr, tint, alpha):
    """a sprite as a translucent ghost in the fighter's colour: (rgb, a)"""
    rgb = spr[..., :3].astype(float)
    lum = rgb.mean(-1, keepdims=True) / 255.0
    col = np.asarray(tint, float)[None, None] * (0.45 + 0.75 * lum)
    return np.clip(col, 0, 255), spr[..., 3:4].astype(float) / 255.0 * alpha


def _paste(arr, rgb, a, cx, cy):
    h, w = a.shape[:2]
    x0, y0 = int(round(cx - w / 2)), int(round(cy - h / 2))
    H_, W_ = arr.shape[:2]
    xa, ya, xb, yb = max(0, x0), max(0, y0), min(W_, x0 + w), min(H_, y0 + h)
    if xa >= xb or ya >= yb:
        return arr
    sa = a[ya - y0:yb - y0, xa - x0:xb - x0]
    arr[ya:yb, xa:xb] += (rgb[ya - y0:yb - y0, xa - x0:xb - x0] - arr[ya:yb, xa:xb]) * sa
    return arr


# ===================================================================================================== look
class Look(base.Look):
    shards = ['bell'] * 5 + ['tube'] * 4 + ['valve'] * 3
    spark_big = ('blast', 'trample', 'front', 'flash')
    spark_flash = ('blast',)
    flash_hits = ('blast', 'front')
    fringe_hits = ('blast', 'front', 'flash')
    rope_kinds = {'company_front': 1.0, 'flash_spin': 0.5}
    sprite_k = 0.86
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.SETS = [dict(t0=e[0], p0=np.array([e[3], e[4]]), p1=np.array([e[5], e[6]]), t1=e[8], n=e[9], kind=e[10],
                          t_blast=None) for e in EV if e[1] == 'drill' and e[2] == k]
        self.STEPS = [(e[0], e[4], e[5], e[3]) for e in EV if e[1] == 'drill_step' and e[2] == k]
        self.BLASTS = [(e[0], e[3], e[4], e[5], e[6], e[7], e[8]) for e in EV if e[1] == 'blast' and e[2] == k]
        for S_ in self.SETS:                                    # each set's blast (the first after its last step)
            for b in self.BLASTS:
                if b[0] >= S_['t1'] - 1e-6:
                    S_['t_blast'] = b[0] if b[0] < S_['t1'] + 1.0 else None
                    break
        self.TRAMPLES = [(e[0], e[3], e[4]) for e in EV if e[1] == 'drill_trample' and e[2] == k]
        self.BASKETS = [(e[0], e[4], e[5]) for e in EV if e[1] == 'trample_basket' and e[2] == k]
        self.FLASH = [(e[0], e[4]) for e in EV if e[1] == 'horn_flash' and e[2] == k]
        self.FRONT = [(e[0], e[3], e[4], e[6]) for e in EV if e[1] == 'company_front' and e[2] == k]
        self.FSTEPS = [(e[0], e[3], e[4]) for e in EV if e[1] == 'front_step' and e[2] == k]
        self.STEP_T = np.array([s[0] for s in self.STEPS]) if self.STEPS else np.zeros(0)
        self._spr = {}
        w = self.cfg['weapon']
        self.spread = float(w.get('spread', w['width']))
        self.fl_w = float(self.cfg['abilities']['unlock1'].get('width', 0.5))

    def sprite(self, r, yaw, roll):
        return padded_sprite(self.cfg['pp'], r, yaw, roll, PAD)

    def title_closeup(self, r):
        yc, sw, roll = self.cfg['view']
        return padded_sprite(self.cfg['pp'], r, float(yc), float(roll), PAD)

    def ghost_sprite(self, r):
        r = max(6, int(r))
        if r not in self._spr:
            yc, sw, roll = self.cfg['view']
            self._spr[r] = padded_sprite(self.cfg['pp'], r, float(yc), float(roll), PAD)
        return self._spr[r]

    # ---------------------------------------------------------------- clocks
    def ta(self, t):
        return min(t, self.P.T_KO)

    def row(self, t):
        P = self.P
        if 'DRILL' not in P.RUN or not (self.T.T_FIGHT <= t < P.T_KO):
            return None
        return P.RUN['DRILL'][P.sim_i(t), self.k]

    def set_at(self, t):
        for S_ in self.SETS:
            end = (S_['t_blast'] or S_['t1']) + 0.45
            if S_['t0'] <= t < end:
                return S_
        return None

    # ---------------------------------------------------------------- state
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k or phase != 'fight':
            return
        rw = self.row(t)
        if rw is None:
            return
        if rw[0] == 1:                                          # marching: a bob on every footfall
            S_ = self.set_at(t)
            if S_ is not None:
                st16 = (S_['t1'] - S_['t0']) / max(1, S_['n'])
                fr = ((t - S_['t0']) / st16) % 1.0
                st['lift'] = 0.07 * math.sin(math.pi * fr)
        if not np.isnan(rw[4]):                                 # the blast: the ball glows from the bell
            st['glow'] = max(st['glow'], 0.55 * (1 - rw[4]))
        if not np.isnan(rw[7]) or not np.isnan(rw[9]):
            st['glow'] = max(st['glow'], 0.3)

    # ---------------------------------------------------------------- events -> camera, ropes, sparks
    def shake(self, e):
        if e[1] == 'blast':
            return 1.1
        if e[1] == 'drill_trample':
            return 1.3
        if e[1] == 'front_step':
            return 0.5
        if e[1] == 'front_hit':
            return 2.0
        if e[1] == 'drill_step':
            return 0.15
        return None

    def rope(self, e):
        if e[1] == 'ability':
            return self.rope_kinds.get(e[4])
        if e[1] == 'front_step':
            return 0.25
        return None

    def spark(self, e):
        how = e[7]
        if how in ('blast', 'flash'):
            return np.array([255, 220, 210], float), 18, 3.6, 0.32
        if how in ('trample', 'front'):
            return np.array([200, 190, 170], float), 14, 2.8, 0.34
        return np.array([230, 160, 170], float), 6, 1.8, 0.2

    def hit_glow(self, e):
        if e[7] in ('blast', 'front'):
            return 16, 0.45
        if e[7] in ('trample', 'flash'):
            return 12, 0.35
        return None

    # ---------------------------------------------------------------- effects
    def fx_setup(self):
        P, k = self.P, self.k
        col = P.FL_COL[k]
        for i, (t0, x, y, n) in enumerate(self.STEPS):          # a footprint per step, alternating feet
            if t0 > P.T_KO:
                break
            P.MK.add(t0, 'dust', x, y, size=0.18, life=2.2, seed=int(t0 * 97) % 991, strength=0.35, col=(214, 206, 190))
            P.PA.emit(t0, P.up(x, y, 0.05), 'dust', n=2, col=TURF)
        for t0, ox, oy, a, ln, wd, w1 in self.BLASTS:
            if t0 > P.T_KO:
                break
            pos = P.up(ox, oy, R)
            P.FL.add(t0, pos, col, radius=2.6, peak=1.0, decay=0.3)
            P.BU.add(t0, pos, (255, 236, 230), 1.2)
            far = P.up(ox + math.cos(a) * ln * 0.6, oy + math.sin(a) * ln * 0.6, R)
            P.FL.add(t0 + 0.04, far, col, radius=2.0, peak=0.6, decay=0.25)
        for t0, x, y in self.TRAMPLES:
            if t0 > P.T_KO:
                break
            P.PA.emit(t0, P.up(x, y, 0.1), 'dust', n=9, col=TURF, speed=(0.5, 1.4))
            P.MK.add(t0, 'claw', x, y, size=0.3, life=3.0, seed=int(t0 * 31) % 997, strength=0.4)
        for t0, x, y in self.BASKETS:                           # a trampled basket: wicker chips
            if t0 > P.T_KO:
                break
            P.PA.emit(t0, P.up(x, y, 0.12), 'chips', n=10, col=[(196, 150, 92), (120, 84, 46), (230, 200, 140)])
            P.MK.add(t0, 'dust', x, y, size=0.3, life=3.0, seed=int(t0 * 41) % 997, strength=0.5, col=(196, 150, 92))
        for t0, i, pos in self.FSTEPS:                          # COMPANY FRONT: a line of dust at every step
            if t0 > P.T_KO:
                break
            for (tf, ax, sg, t1) in self.FRONT:
                if tf <= t0 <= t1 + 0.01:
                    for j in range(7):
                        v = -S.A + (j + 0.5) * 2 * S.A / 7
                        x, y = (pos, v) if ax == 0 else (v, pos)
                        P.PA.emit(t0, P.up(x, y, 0.05), 'dust', n=2, col=TURF)
                    break

    def fx_event(self, e):
        P = self.P
        if e[1] != 'hit' or e[2] != self.k:
            return
        pos = S.to_world(e[5], e[6], R)
        if e[7] == 'blast':
            P.BU.add(e[0], pos, (255, 236, 230), 1.0)
        elif e[7] == 'front':
            P.BU.add(e[0], pos, P.COL[e[2]], 1.4)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'sparks', n=28, spread=180, speed=(0.8, 2.4), col=[(255, 200, 200), (240, 238, 230)])
        P.PA.emit(P.T_KO, pos, 'chips', n=12, col=[tuple(FIBER), tuple(BRASS), tuple(NICK)])

    # ---------------------------------------------------------------- the canvas: chalk lines, the cone, the sweep
    def canvas(self, Ic, C):
        P, k = self.P, self.k
        t = C.t
        sxp, szp, ys, xs = C.sxp, C.szp, C.ys, C.xs
        col = P.COL[k] / 255.0
        tc = self.ta(t)
        S_ = self.set_at(tc) if self.T.T_FIGHT <= t < P.T_KO else None
        if S_ is not None:                                      # the drill line: a dashed chalk yard line
            a, b = S_['p0'], S_['p1']
            d = b - a
            L = float(np.hypot(*d)) + 1e-9
            d = d / L
            rx, rz = sxp - a[0], szp - a[1]
            along = rx * d[0] + rz * d[1]
            off = np.abs(-rx * d[1] + rz * d[0])
            end = S_['t_blast'] or S_['t1']
            fade = min(1.0, (tc - S_['t0']) / 0.15) * (1 - max(0.0, (tc - end) / 0.45))
            dash = ((along / 0.22) % 1.0) < 0.6
            lane = (off < 0.055) & (along > -0.1) & (along < L + 0.1) & dash
            Ic += (lane * 0.55 * fade)[:, None] * (CHALK / 255.0)[None]
        rw = self.row(t)
        if rw is None:
            return Ic
        if not np.isnan(rw[4]):                                 # the bell blast: a cone of light
            o = np.array([rw[12], rw[13]])
            a_ = rw[2]
            d = np.array([math.cos(a_), math.sin(a_)])
            ln, w0 = rw[5], rw[6]
            rx, rz = sxp - o[0], szp - o[1]
            along = rx * d[0] + rz * d[1]
            off = np.abs(-rx * d[1] + rz * d[0])
            hw = 0.5 * (w0 + (self.spread - w0) * np.clip(along / max(ln, 1e-6), 0, 1))
            band = np.clip(1 - off / (hw + 0.15), 0, 1) * (along > 0) * (along < ln * (0.4 + 0.6 * min(1, rw[4] * 3)))
            band = np.floor(band * 4 + BAYER_FULL[ys, xs] * 0.9) / 4
            Ic += (band * 0.9 * (1 - rw[4]) ** 0.6)[:, None] * (col * 0.8 + 0.35)[None]
        if not np.isnan(rw[7]):                                 # HORN FLASH: a lane sweeping round
            st = C.fs[k]
            a_ = rw[7]
            d = np.array([math.cos(a_), math.sin(a_)])
            rx, rz = sxp - st['p'][0], szp - st['p'][1]
            along = rx * d[0] + rz * d[1]
            off = np.abs(-rx * d[1] + rz * d[0])
            band = np.clip(1 - off / (self.fl_w * 0.5 + 0.12), 0, 1) * (along > R * 0.8) * (along < R + rw[8])
            band = np.floor(band * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (band * 0.8)[:, None] * (col * 0.8 + 0.3)[None]
        if not np.isnan(rw[9]):                                 # COMPANY FRONT: the chalk front line, lit behind
            ax, sg, pos = int(rw[10]), rw[11], rw[9]
            c_ = sxp if ax == 0 else szp
            rel = (c_ - pos) * sg
            line = (np.abs(rel) < 0.04)
            glow = np.clip(1 + rel / 0.9, 0, 1) * (rel < 0)
            glow = np.floor(glow * 3 + BAYER_FULL[ys, xs] * 0.9) / 3
            Ic += (line * 0.5)[:, None] * (CHALK / 255.0)[None] + (glow * 0.35)[:, None] * col[None]
        return Ic

    # ---------------------------------------------------------------- over the ring: sound waves, the ghost line
    def over_ring(self, F, arr, depth):
        P, cam, k = self.P, F.cam, self.k
        t = F.t
        col = P.COL[k]
        tc = self.ta(t)
        if self.T.T_FIGHT <= t:                                 # the bell blasts: a filled cone + sound-wave arcs
            for t0, ox, oy, a_, ln, w0, w1 in self.BLASTS:
                age = tc - t0
                if not (0 <= age < 0.45):
                    continue
                kk = 1 - age / 0.45
                reach = ln * min(1.0, 0.35 + age / 0.08)
                hw0, hw1 = 0.5 * w0, 0.5 * (w0 + (w1 - w0) * reach / max(ln, 1e-6))
                d = np.array([math.cos(a_), math.sin(a_)])
                nn = np.array([-d[1], d[0]])
                o = np.array([ox, oy])
                quad = [o + nn * hw0, o + d * reach + nn * hw1, o + d * reach - nn * hw1, o - nn * hw0]
                sp = cam.wp(S.to_world(np.array([q[0] for q in quad]), np.array([q[1] for q in quad]),
                                       np.full(4, R * 0.9)))
                arr = blend(arr, _mask([sp]), np.clip(col * 0.55 + 140, 0, 255), 0.42 * kk)
                core = [o + nn * hw0 * 0.4, o + d * reach * 0.8 + nn * hw1 * 0.3, o + d * reach * 0.8 - nn * hw1 * 0.3,
                        o - nn * hw0 * 0.4]
                sp2 = cam.wp(S.to_world(np.array([q[0] for q in core]), np.array([q[1] for q in core]),
                                        np.full(4, R * 0.9)))
                arr = blend(arr, _mask([sp2]), WHITE, 0.45 * kk)
                for j in range(3):                              # arcs rolling out along the cone
                    s_ = ((age / 0.45 * 1.6 + j / 3.0) % 1.0) * reach
                    hw = hw0 + (hw1 - hw0) * s_ / max(reach, 1e-6)
                    ang = math.atan2(hw, max(s_, 0.05))
                    th = np.linspace(-ang, ang, 9)
                    rr = math.hypot(s_, hw)
                    Z = np.stack([o[0] + rr * np.cos(a_ + th), o[1] + rr * np.sin(a_ + th), np.full(9, R)], 1)
                    spa = cam.wp(S.to_world(Z[:, 0], Z[:, 1], Z[:, 2]))
                    arr = blend(arr, _mask([], [spa], 3), INK, 0.4 * kk)
                    arr = blend(arr, _mask([], [spa], 1), WHITE, 0.95 * kk)
        rw = self.row(t)
        fade = 1.0
        if rw is None and P.T_KO <= t < P.T_KO + 0.6 and 'DRILL' in P.RUN:   # the wall holds at the K.O., fading
            rw = P.RUN['DRILL'][P.sim_i(P.T_KO - 1e-3), k]
            fade = 1 - (t - P.T_KO) / 0.6
        if rw is None:
            return arr, depth
        if not np.isnan(rw[9]):                                 # the ghost sousaphones, a step a beat
            ax, sg, pos = int(rw[10]), rw[11], rw[9]
            bob = abs(math.sin(math.pi * ((min(t, P.T_KO) - self.T.T_FIGHT) / self.T.BEAT)))
            n = 7
            items = []
            for j in range(n):
                v = -S.A + (j + 0.5) * 2 * S.A / n
                x, y = (pos - sg * 0.1, v) if ax == 0 else (v, pos - sg * 0.1)
                w_ = S.to_world(x, y, R + 0.05 * bob)
                items.append((cam.depth(w_)[0], w_))
            for dz, w_ in sorted(items, key=lambda q: -q[0]):
                c = cam.wp(w_[None])[0]
                rp = R * cam.f / dz
                al = 0.72 * min(1.0, rw[15] * 8) * min(1.0, (1 - rw[15]) * 8) * fade
                rgb, a = _ghost(self.ghost_sprite(rp * 1.9), col, al)
                arr = _paste(arr, rgb, a, c[0], c[1])
        return arr, depth

    # ---------------------------------------------------------------- its parts (K.O. shatter)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([p[0], p[2]])
        if kind == 'tube':                                      # a curved length of brass tubing
            pts = [c + d * (j - 2.5) * 0.06 + nn * curl * 0.03 * (j - 2.5) ** 2 for j in range(6)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            arr = blend(arr, _mask([], [sp], 3), np.array([90, 60, 20], float), 0.8)
            return blend(arr, _mask([], [sp], 2), BRASS, 1.0)
        if kind == 'valve':
            pts = [c - d * 0.05, c + d * 0.05]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            return blend(arr, _mask([], [sp], 3), NICK, 1.0)
        sz = size * 1.8                                         # a curved shard of the fiberglass bell
        tri = [c + d * sz, c - d * sz * 0.6 + nn * sz * 0.9, c - d * sz * 0.5 - nn * sz * 0.6]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        P.layer_blend(arr, [sp], FIBER * (0.8 + 0.25 * abs(math.cos(rot))), 1.0)
        return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), np.array([170, 40, 50], float), 0.8)


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.6, 0.2
    band = (40, 300)                                        # the marching bass's fundamentals (the bed has no low brass)
    colour_args = dict(cut0=40, cut_span=90, drive=2.8)

    def __init__(self, M, k):
        super().__init__(M, k)
        import pp_bridge as B
        self.B = B
        self.mod = B.module('sousaphone_lowpoly')
        self.pan = -0.1 if k == 0 else 0.1
        self.lo, self.hi = M.CFG[k]['register']
        self._c = {}
        self.g0 = M.inst_gain(M.CFG[k]['pp'])

    def tone(self, m, dur, vel=0.9, blat=0.0):
        key = (int(m), round(dur, 3), round(vel, 2), round(blat, 2))
        if key not in self._c:
            x = self.mod.sousa_tone(self.mod.midi_hz(int(m)), dur, vel, blat=blat)
            self._c[key] = self.B._resample(np.asarray(x, float)) * self.g0
        return self._c[key]

    def voice(self, m, dur, vel=0.85):
        return self.tone(m, dur, vel, 0.25)

    def fit_reg(self, m, lo=None, hi=None):
        lo, hi = lo or self.lo, hi or self.hi
        while m > hi:
            m -= 12
        while m < lo:
            m += 12
        return m

    def line_note(self, t):
        return self.fit_reg(self.T.line_at(self.name, t), self.lo, self.lo + 22)

    def root_of(self, t):
        return self.T.root_above(self.T.chord_at(t), self.lo + 5)

    def trio_notes(self, t0, beats):
        """the trio (TRIO, a half note per beat here) in the episode's key: its tonic = the key's relative major when
        the key is minor (the trio's own major), in the sousaphone's register"""
        T = self.T
        tonic = self.fit_reg(T.KEY_PC + 36 + (3 if self.minor() else 0), self.lo + 2, self.lo + 14)
        out = []
        for bt, semi, d in TRIO:
            if bt >= beats - 1e-6:
                break
            out.append((t0 + bt * T.BEAT, tonic + semi, min(d, beats - bt) * T.BEAT))
        return out

    def minor(self):
        T = self.T
        return (T.KEY_PC + 3) % 12 in [(T.KEY_PC + s) % 12 for s in T.SCALE]

    def col(self, x, t, **kw):
        return self.colour(x, t, low=True, **kw)

    # ---------------------------------------------------------------- the score
    def prepare(self):
        """COMPANY FRONT under the enemy's desperation move: the enemy's melody gives way to the config's OBBLIGATO
        (opt-in: only when the episode sets OBBLIGATO for the enemy)"""
        M, T, k = self.M, self.T, self.k
        ob = getattr(T, 'OBBLIGATO', {}).get(M.NAMES[self.enemy])
        if not ob:
            return
        low = M.CFG[self.enemy]['abilities']['low']['name']
        for e in M.EV:
            if e[1] == 'company_front' and e[2] == k:
                t0, t1 = M.q16(e[0]), e[6]
                for f in M.EV:
                    if f[1] == 'ability' and f[2] == self.enemy and f[3] == low and t0 - 1e-6 <= f[0] < t1:
                        M.line_override(self.enemy, M.q16(f[0]), min(t1, M.T_KO), t0, ob)

    def title_blast(self, mix, t):
        """a huge blat on the tonic with its octave, then a pickup of marching 8ths"""
        hits = self.T.TITLE_HITS.get(self.name) or (self.root_of(0.0),)
        r = self.fit_reg(hits[0])
        mix.place(self.bus, self.tone(r, 0.7, 1.0, 1.0), t, 0.62, pan=self.pan)
        mix.place(self.bus, self.tone(r + 12, 0.6, 0.9, 0.8), t, 0.30, pan=self.pan)
        T = self.T
        for j, m in enumerate(hits[1:4]):
            if t + T.BEAT + j * T.BEAT / 2 < T.BAR - 0.05:
                mix.place(self.bus, self.tone(self.fit_reg(m), T.BEAT * 0.4, 0.9), t + T.BEAT + j * T.BEAT / 2, 0.42,
                          pan=self.pan)

    def count_in(self, mix):
        """it marks time on the count-in: root on every beat, an 8th pickup into FIGHT!"""
        T = self.T
        r = self.root_of(T.BAR)
        for j in range(4):
            mix.place(self.bus, self.tone(r, T.BEAT * 0.45, 0.95), T.BAR + j * T.BEAT, 0.48, pan=self.pan)
        nxt = self.line_note(T.T_FIGHT)
        mix.place(self.bus, self.tone(nxt - 2 if nxt - 2 >= self.lo else nxt + 2, T.BEAT * 0.4, 0.9),
                  T.BAR + 3.5 * T.BEAT, 0.40, pan=self.pan)

    def play(self, mix):
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16 = M.q16
        th = M.Thinner(530 + k)
        marching = []                                           # (t0, t_blast) of every set
        look = M.P.LOOKS[k]
        for S_ in look.SETS:
            marching.append((S_['t0'], S_['t_blast'] or S_['t1']))
        fronts = [(tf, t1) for tf, ax, sg, t1 in look.FRONT]

        def marking(t):
            return not any(a - 1e-6 <= t < b - 1e-6 for a, b in marching) and \
                not any(a - 1e-6 <= t < b for a, b in fronts)
        # marking time between sets: a note on every beat (the line's beat notes)
        nb0, nb1 = int(math.ceil(T.T_FIGHT / T.BEAT)), int(M.T_KO / T.BEAT)
        for nb in range(nb0, nb1 + 1):
            tq = nb * T.BEAT
            if not marking(tq):
                continue
            if not th.keep(nb * 4, M.hp_at(k, tq) / 100.0):
                continue
            sig, g, _ = self.col(self.tone(self.line_note(tq), T.BEAT * 0.5, 0.9), tq)
            mix.place(self.bus, sig, tq, 0.48 * g, pan=self.pan)
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if e[2] != k:
                continue
            tq = q16(t)
            if kind == 'drill_step':                            # a footfall = a note of the marching line
                if not th.keep(round(tq / T.S16), M.hp_at(k, t) / 100.0):
                    continue
                on8 = abs(tq / (2 * T.S16) - round(tq / (2 * T.S16))) < 1e-6
                if on8:
                    m, dur = self.line_note(tq), T.S16 * 1.6
                else:                                           # DOUBLE TIME's 16ths: chromatic approach notes
                    nxt = self.line_note(tq + T.S16)
                    m, dur = (nxt - 1 if nxt > self.lo else nxt + 1), T.S16 * 0.9
                sig, g, _ = self.col(self.tone(m, dur, 0.92), t)
                mix.place(self.bus, sig, tq, 0.52 * g, pan=self.pan)
                mix.place('fx', A.thud(0.12, 90, 50, seed=int(t * 13)), tq, 0.05)
            elif kind == 'blast':                               # the bell blast: a big blat on the root + its octave
                r = self.root_of(tq)
                sig, g, _ = self.col(self.tone(r, T.BEAT * 1.4, 1.0, 1.0), t, drive_k=1.3)
                mix.place(self.bus, sig, tq, 0.62 * g, pan=self.pan)
                mix.place(self.bus, self.tone(r + 12, T.BEAT * 1.2, 0.9, 0.8), tq, 0.30 * g, pan=self.pan)
                mix.place('fx', A.thud(0.5, 120, 45, seed=int(t * 7)), tq, 0.16)
            elif kind == 'drill_trample':
                mix.place('fx', A.thud(0.5, 110, 32, seed=int(t * 17)), tq, 0.30)
            elif kind == 'trample_basket':
                mix.place('fx', A.thud(0.2, 260, 140, seed=int(t * 19)), tq, 0.10)
            elif kind == 'horn_flash':                          # a rip up into a blat
                r = self.root_of(tq)
                for j, m in enumerate((r, r + 4, r + 7, r + 10)):
                    mix.place(self.bus, self.tone(m, T.S16 * 0.8, 0.85), tq + j * T.S16 / 2, 0.34, pan=self.pan)
                mix.place(self.bus, self.tone(r + 12, T.BEAT, 1.0, 1.0), tq + 2 * T.S16, 0.48, pan=self.pan)
            elif kind == 'company_front':                       # the trio, in low unison octaves
                beats = int(round((e[6] - e[5]) / T.BEAT))
                for ts, m, d in self.trio_notes(tq, beats):
                    if ts > M.T_KO + 0.05:
                        break
                    sig, g, _ = self.col(self.tone(m, d * 0.92, 1.0, 0.5), ts, drive_k=0.6)
                    mix.place(self.bus, sig, ts, 0.50 * g, pan=self.pan * 0.4)
                    mix.place(self.bus, self.tone(m + 12, d * 0.9, 0.9, 0.3), ts, 0.26 * g, pan=-self.pan)
            elif kind == 'front_step':
                mix.place('fx', A.thud(0.25, 100, 40, seed=int(t * 23)), tq, 0.12)

    def fall_apart(self, mix):
        """the horn sags: a falling blat and the valves rattling"""
        M, A = self.M, self.M.A
        t0 = M.P.T_HS
        r = self.root_of(t0)
        mix.place(self.bus, M.bend(self.tone(r, 1.3, 1.0, 1.0), -9.0, 0.1, 1.2), t0, 0.5, pan=self.pan)
        for j in range(5):
            mix.place('fx', A.thud(0.06, 900, 600, seed=40 + j), t0 + 0.06 + 0.07 * j, 0.06)

    def stinger(self, mix, tw):
        r = self.root_of(0.0)
        mix.place('solo', self.tone(r, 0.25, 1.0, 0.6), tw, 0.44)
        mix.place('solo', self.tone(r + 7, 0.25, 1.0, 0.6), tw + self.T.S16 * 2, 0.44)
        mix.place('solo', self.tone(r + 12, 1.0, 1.0, 1.0), tw + self.T.S16 * 4, 0.50)

    def solo(self, mix, t0):
        """config SOLO: a strutting marching-bass solo, the long notes blatted"""
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.tone(self.fit_reg(m), max(0.1, d * T.BEAT * 0.9), 0.95, 0.8 if d >= 1.5 else 0.2),
                      ts, 0.6)

    def reassemble(self, mix):
        T, M = self.T, self.M
        r = self.root_of(0.0)
        run = [r, r + 4, r + 7, r + 12, r + 16, r + 19]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place('solo', self.tone(self.fit_reg(m, self.lo, self.lo + 28), 0.12, 0.85), tj, 0.3)

    def last_bar(self, mix):
        """marching up into the title slam: 8ths climbing the dominant to the tonic on frame 0"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        tgt = self.fit_reg(self.root_of(0.0) + 12, self.lo + 7, self.lo + 22)
        steps = [tgt - 5, tgt - 5, tgt - 3, tgt - 3, tgt - 2, tgt - 2, tgt - 1, tgt - 1]
        for j, m in enumerate(steps):
            mix.place(self.bus, self.tone(m, T.BEAT * 0.4, 0.95), t_last + j * T.BEAT / 2, 0.5, pan=self.pan)
