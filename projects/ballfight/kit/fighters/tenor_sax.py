"""TENOR SAX (burnt orange), new in #51 Alto Sax vs Tenor Sax: the only fighter that uses the crowd. CROWD PLEASER (every
bar it strikes a pose and honks, the hype meter fills and the crowd hurls tomatoes, roses and foam fingers at the enemy
from the stands - they only reach so far in, so the middle of the ring is safe; more throws, thrown harder and further,
per landed volley), WALK THE BAR (it struts down a spotlit runway across the ring, honking on every beat), SUBTONE (the
crowd hushes and leans in, then throws twice as hard), STAGE DIVE (it leaps into the crowd, crowd-surfs along the
stands and is launched back like a cannonball).

Look: the tenor built from Commons reference photos (tenorsax_lowpoly: the neck's hump, the deep amber lacquer, x1.06 so
it reads bigger than the alto). A pose is a little hop with a glow and honk lines from the bell; a hype meter of six
pips arcs over its ball. Every throw arcs out of the crowd (from the front rows, visibly in the stands) to its landing
spot, its shadow growing on the canvas as it falls; tomatoes splat red, roses burst into petals, foam fingers bounce.
WALK THE BAR lights a catwalk across the canvas with chaser bulbs along its edges; the honks are burnt-orange rings.
STAGE DIVE: an arc over the ropes into the stands, carried on raised hands, then a low cannonball arc with streaks and
a cratering landing ring. It shatters into amber brass, key cups and pearls. VS card: a row of cheering crowd
silhouettes, hands up.
Sound: the husky, honking low line. Each pose is a big honk, each throw a note of its riff (config LINES, 8ths: the
more hype, the longer the riff), a landed throw a growl and a squish; WALK THE BAR honks up the line on every beat;
SUBTONE is a breathy subtone note over the crowd's hush, then one fat growl; STAGE DIVE a rising scream into the crowd's
roar, a high honk at the launch and the lowest honk at the landing."""
import math

import numpy as np
from reelkit.config import W, H, BAYER_FULL
from reelkit import fx
import scene as S

from . import base
from ._reeds import _mask, blend, add, INK, WHITE

R = S.R_BALL
AMBER = np.array([218, 156, 52], float)
KEYG = np.array([250, 196, 92], float)
PEARL = np.array([250, 246, 236], float)
TOMATO = np.array([220, 44, 34], float)
LEAF = np.array([60, 150, 60], float)
ROSE = np.array([236, 60, 110], float)
STAND_R, HAND_Y = 6.55, -0.25                          # where the crowd's hands are (the front rows, world)
ITEM_COLS = [[(230, 50, 40), (255, 120, 90), (120, 20, 16)],       # tomato splat
             [(240, 90, 140), (255, 180, 200), (200, 40, 90)],     # rose petals
             [(230, 110, 30), (255, 190, 110), (255, 255, 255)]]   # foam finger
CATWALK = np.array([1.0, 0.86, 0.66])


def title_pattern(base):
    """TENOR SAX: a row of cheering crowd silhouettes along the divider, hands up"""
    import title as TT
    XX, YY, SIDE = TT.XX, TT.YY, TT.SIDE
    u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
    v = np.abs(SIDE)
    cu = (u % 34.0) - 17.0
    band = (v % 120.0)
    head = cu ** 2 + (band - 44) ** 2 < 64
    body = (np.abs(cu) < 11) & (band > 50) & (band < 80)
    arms = ((np.abs(np.abs(cu) - 10 + (band - 40) * 0.25) < 2.0) & (band > 18) & (band < 48))
    mark = head | body | arms
    return np.where(mark[..., None], base * 0.35 + np.array([40, 18, 6]) * 0.65, base)


def stand_point(x, z):
    """the crowd's hands behind the ring-frame point (x, z): straight out from the centre, in the front rows"""
    w = S.to_world(x, z, 0.0)
    d = np.array([w[0], 0.0, w[2]])
    d = d / (np.linalg.norm(d) + 1e-9)
    return np.array([d[0] * STAND_R, HAND_Y, d[2] * STAND_R])


class Look(base.Look):
    shards = ['brass'] * 6 + ['key'] * 4 + ['pearl'] * 3
    spark_big = ('throw', 'dive', 'strut')
    spark_flash = ('dive',)
    flash_hits = ('throw', 'dive')
    fringe_hits = ('dive',)
    quiet_hits = ()
    rope_kinds = {'stagedive': 1.0}
    sprite_k = 0.86
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.POSES = [(e[0], e[3], e[4]) for e in EV if e[1] == 'pose' and e[2] == k]
        self.POSE_T = np.array([p[0] for p in self.POSES]) if self.POSES else np.zeros(0)
        self.THROWS = [dict(id=e[3], p0=np.array([e[4], e[5]]), p1=np.array([e[6], e[7]]), t0=e[8], t1=e[9], item=e[10])
                       for e in EV if e[1] == 'throw' and e[2] == k]
        self.SPLAT = {e[3]: (e[0], e[4], e[5], e[6]) for e in EV if e[1] == 'splat' and e[2] == k}
        self.RUNWAY = [(e[0], np.array([e[3], e[4]]), np.array([e[5], e[6]]), e[8]) for e in EV
                       if e[1] == 'runway' and e[2] == k]
        self.HUSH = [(e[0], e[3]) for e in EV if e[1] == 'hush' and e[2] == k]
        dv = [e for e in EV if e[1] == 'dive' and e[2] == k]
        self.DIVES = []
        for e in dv:
            la = next((x for x in EV if x[1] == 'dive_launch' and x[2] == k and x[0] >= e[0]), None)
            ld = next((x for x in EV if x[1] == 'dive_land' and x[2] == k and x[0] >= e[0]), None)
            self.DIVES.append(dict(t0=e[0], out=np.array([e[3], e[4]]), t_surf=e[5], t_launch=e[6],
                                   l0=None if la is None else np.array([la[3], la[4]]),
                                   l1=None if la is None else np.array([la[5], la[6]]),
                                   t_land=None if la is None else la[7], landed=None if ld is None else ld[0]))

    # ---------------------------------------------------------------- clocks
    def ta(self, t):
        return min(t, self.P.T_KO)

    def pose_at(self, t):
        if not len(self.POSE_T):
            return None
        j = int(np.searchsorted(self.POSE_T, t, 'right')) - 1
        if j >= 0 and t < self.POSES[j][1] + 0.12:
            p = self.POSES[j]
            return (t - p[0]) / max(1e-6, p[1] + 0.12 - p[0]), p[2]
        return None

    def dive_at(self, t):
        for D in self.DIVES:
            end = D['t_land'] if D['t_land'] is not None else D['t_launch'] + 1.0
            if D['t0'] <= t < end:
                return D
        return None

    def runway_at(self, t):
        for t0, a, b, t1 in self.RUNWAY:
            if t0 <= t < t1 + 0.25:
                return t0, a, b, t1
        return None

    def hype(self, t):
        if 'CROWD' not in self.P.RUN or not (self.T.T_FIGHT <= t < self.P.T_KO):
            return float(self.cfg['weapon']['n0'])
        return float(self.P.RUN['CROWD'][self.P.sim_i(t)][self.k][0])

    # ---------------------------------------------------------------- state: the pose hop, the dive's flight path
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k or phase != 'fight':
            return
        tc = self.ta(t)
        ps = self.pose_at(tc)
        if ps is not None:
            st['lift'] = max(st['lift'], 0.14 * math.sin(math.pi * min(1.0, ps[0])))
            st['glow'] = max(st['glow'], 0.4 * (1 - ps[0]))
        if self.runway_at(tc) is not None:
            st['glow'] = max(st['glow'], 0.3 + 0.2 * self.T.beat_pulse(t))
        D = self.dive_at(tc)
        if D is None:
            return
        st['glow'] = max(st['glow'], 0.45)
        p = st['p']
        if tc < D['t_surf']:                                       # the leap: over the ropes, into the stands
            u = (tc - D['t0']) / max(1e-6, D['t_surf'] - D['t0'])
            w0 = S.to_world(*self._p_at(D['t0']), 0.0)
            w1 = stand_point(*D['out'])
            ue = u * u * (3 - 2 * u)
            w = w0 + (w1 - w0) * ue
            st['p'] = np.array(S.to_ring(w[0], w[2]))
            st['lift'] = 2.2 * math.sin(math.pi * u) + (HAND_Y + 0.15 - R * 0.0) * ue
        elif tc < D['t_launch'] or D['t_land'] is None:            # crowd-surfing, carried on raised hands
            w = stand_point(p[0], p[1])
            st['p'] = np.array(S.to_ring(w[0], w[2]))
            st['lift'] = HAND_Y + 0.15 + 0.07 * math.sin(2 * math.pi * (tc - D['t_surf']) * 4.0)
            st['surf'] = True
        else:                                                      # the cannonball: a low arc from the crowd
            u = (tc - D['t_launch']) / max(1e-6, D['t_land'] - D['t_launch'])
            w0 = stand_point(*D['l0'])
            w1 = S.to_world(*D['l1'], 0.0)
            w = w0 + (w1 - w0) * u
            st['p'] = np.array(S.to_ring(w[0], w[2]))
            st['lift'] = (HAND_Y + 0.15) * (1 - u) + 1.3 * math.sin(math.pi * u)
            st['cannon'] = u

    def _p_at(self, t):
        return self.P.RUN['P'][self.P.sim_i(t), self.k]

    def hides_hp(self, kk, t):
        return kk == self.k and self.dive_at(self.ta(t)) is not None

    # ---------------------------------------------------------------- events -> camera, sparks
    def shake(self, e):
        if e[1] == 'dive_land':
            return 2.2
        if e[1] == 'dive_hit':
            return 2.0
        if e[1] == 'pose':
            return 0.3
        if e[1] == 'strut_bump':
            return 1.0
        return None

    def rope(self, e):
        if e[1] == 'ability':
            return self.rope_kinds.get(e[4])
        return None

    def spark(self, e):
        how = e[7]
        if how == 'throw':
            return np.array([255, 140, 100], float), 10, 2.2, 0.28
        if how == 'dive':
            return np.array([255, 190, 120], float), 18, 3.6, 0.36
        return np.array([255, 170, 90], float), 8, 2.0, 0.24

    def hit_glow(self, e):
        if e[7] in ('dive', 'strut'):
            return 16, 0.4
        if e[7] == 'throw':
            return 9, 0.3
        return None

    # ---------------------------------------------------------------- effects
    def fx_setup(self):
        P, k = self.P, self.k
        col = P.FL_COL[k]
        for Th in self.THROWS:
            sp = self.SPLAT.get(Th['id'])
            if sp is None or sp[0] > P.T_KO:
                continue
            t0, x, y, hit = sp
            item = Th['item']
            pos = P.up(x, y, 0.08)
            if item == 0:                                         # a tomato splats
                P.PA.emit(t0, pos, 'shreds', n=7 if hit else 5, col=ITEM_COLS[0], speed=(0.8, 2.0))
                P.MK.add(t0, 'dust', x, y, size=0.30, life=4.0, col=(200, 40, 30), seed=int(t0 * 37) % 991,
                         strength=0.8)
            elif item == 1:                                       # a rose bursts into petals
                P.PA.emit(t0, pos, 'scales', n=7, col=ITEM_COLS[1], speed=(0.6, 1.6))
            else:                                                 # a foam finger bounces
                P.PA.emit(t0, pos, 'puffs', n=4, col=ITEM_COLS[2])
            if hit:
                P.FL.add(t0, P.up(x, y, R), col, radius=1.2, peak=0.4, decay=0.15)
        for t0, hype, _ in [(p[0], p[2], 0) for p in self.POSES]:
            if t0 > P.T_KO:
                break
            q = self._p_at(t0)
            P.FL.add(t0, P.up(q[0], q[1], R), col, radius=1.4, peak=0.35, decay=0.18)
        for D in self.DIVES:
            if D['landed'] is not None and D['landed'] <= P.T_KO:
                pos = P.up(D['l1'][0], D['l1'][1], 0.1)
                P.PA.emit(D['landed'], pos, 'dust', n=14, speed=(0.6, 1.6))
                P.MK.add(D['landed'], 'crack', D['l1'][0], D['l1'][1], size=0.8, life=5.0, seed=17, strength=0.8)
                P.BU.add(D['landed'], P.up(D['l1'][0], D['l1'][1], R), P.COL[k], 1.6)
                P.FL.add(D['landed'], pos, col, radius=2.6, peak=1.0, decay=0.35)
            if D['t_launch'] <= P.T_KO and D['l0'] is not None:
                w0 = stand_point(*D['l0'])
                fly = S.to_world(*D['l1'], 0.0) - w0
                P.PA.emit(D['t_launch'], w0 + np.array([0, R, 0]), 'streaks', n=10,
                          dir=tuple(fly / (np.linalg.norm(fly) + 1e-9)), col=[(255, 200, 140), (255, 255, 230)])
            if D['t0'] <= P.T_KO:
                P.PA.emit(D['t_surf'], stand_point(*D['out']) + np.array([0, 0.3, 0]), 'foam', n=10,
                          col=[(255, 220, 160), (255, 255, 255), tuple(P.COL[k])])

    def fx_event(self, e):
        P = self.P
        if e[1] == 'hit' and e[7] == 'strut':
            P.BU.add(e[0], S.to_world(e[5], e[6], R), P.COL[e[2]], 1.0)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'brass', n=18, col=[tuple(AMBER), tuple(KEYG), (255, 230, 180)])
        P.PA.emit(P.T_KO, pos, 'sparks', n=20, spread=180, speed=(0.8, 2.2), col=[(255, 160, 80), (255, 230, 200)])

    # ---------------------------------------------------------------- the canvas: the catwalk, the throws' shadows
    def canvas(self, Ic, C):
        if not C.in_fight:
            return Ic
        tc = self.ta(C.t)
        rw = self.runway_at(tc)
        if rw is not None:
            t0, a, b, t1 = rw
            fade = min(1.0, (tc - t0) / 0.12) * min(1.0, max(0.0, (t1 + 0.25 - tc) / 0.25))
            d = b - a
            L = float(np.hypot(*d)) + 1e-9
            u = d / L
            rel_x, rel_z = C.sxp - a[0], C.szp - a[1]
            along = rel_x * u[0] + rel_z * u[1]
            across = -rel_x * u[1] + rel_z * u[0]
            ext = 0.7
            inside = (along > -ext) & (along < L + ext)
            band = np.clip((0.42 - np.abs(across)) / 0.06, 0, 1) * inside
            band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic += (band * 0.42 * fade)[:, None] * CATWALK[None]
            step = 0.36                                              # chaser bulbs along both edges
            ph = (tc - t0) / (self.T.BEAT / 4)
            for side in (-1, 1):
                for j in range(int((L + 2 * ext) / step) + 1):
                    s_ = -ext + j * step
                    on = 1.0 if (j + int(ph)) % 3 == 0 else 0.35
                    bx, bz = a[0] + u[0] * s_ - u[1] * side * 0.5, a[1] + u[1] * s_ + u[0] * side * 0.5
                    m = np.clip(1 - np.hypot(C.sxp - bx, C.szp - bz) / 0.07, 0, 1)
                    Ic += (m * on * 1.1 * fade)[:, None] * np.array([1.0, 0.8, 0.45])[None]
        return Ic

    def canvas_after(self, Ic, C):
        if not C.in_fight:
            return Ic
        t = self.ta(C.t)
        for Th in self._flying(t):                                   # each throw's shadow grows as it falls
            u = (t - Th['t0']) / max(1e-6, Th['t1'] - Th['t0'])
            r = 0.06 + 0.12 * u
            m = np.clip(1 - np.hypot(C.sxp - Th['p1'][0], C.szp - Th['p1'][1]) / r, 0, 1)
            m = np.floor(m * 2 + BAYER_FULL[C.ys, C.xs] * 0.9) / 2
            Ic *= (1 - 0.45 * u * m)[:, None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if owner != self.k or kind not in ('honk', 'dive'):
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)
        wid = 0.10 if kind == 'honk' else 0.16
        band = np.clip(1 - np.abs(rd - r) / wid, 0, 1) * (1 - r / rmax) ** 0.6
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * (0.9 if kind == 'honk' else 1.2))[:, None] * (self.col / 255.0 * 0.8 + 0.2)[None]
        return Ic

    def _flying(self, t):
        return [Th for Th in self.THROWS if Th['t0'] <= t < Th['t1']]

    # ---------------------------------------------------------------- over the ring: the throws in flight, honk
    # lines, the hype pips, the crowd's hands under a stage diver
    def over_ring(self, F, arr, depth):
        P, cam, k = self.P, F.cam, self.k
        if not F.in_fight:
            return arr, depth
        t = self.ta(F.t)
        for Th in self._flying(t):
            u = (t - Th['t0']) / max(1e-6, Th['t1'] - Th['t0'])
            w0 = stand_point(*Th['p0'])
            w1 = S.to_world(*Th['p1'], 0.12)
            w = w0 + (w1 - w0) * u + np.array([0, 3.0 * 4 * u * (1 - u), 0])
            c = cam.wp(w[None])[0]
            rp = max(2.0, 0.17 * cam.f / cam.depth(w[None])[0])
            spin = (t - Th['t0']) * 9.0 + Th['id']
            item = Th['item']
            if item == 0:                                            # tomato: red ball, green top
                arr = blend(arr, _mask([[(c[0] + rp * 1.25 * math.cos(a), c[1] + rp * 1.25 * math.sin(a))
                                         for a in np.linspace(0, 2 * math.pi, 9)[:-1]]]), INK, 0.7)
                arr = blend(arr, _mask([[(c[0] + rp * math.cos(a), c[1] + rp * math.sin(a))
                                         for a in np.linspace(0, 2 * math.pi, 9)[:-1]]]), TOMATO, 1.0)
                arr = blend(arr, _mask([], [[(c[0] - rp * 0.5, c[1] - rp * 0.9), (c[0] + rp * 0.5, c[1] - rp * 0.9)]], 1),
                            LEAF, 1.0)
            elif item == 1:                                          # rose: a pink-red bloom on a green stem
                e = (math.cos(spin), math.sin(spin))
                arr = blend(arr, _mask([], [[(c[0], c[1]), (c[0] - e[0] * rp * 2.4, c[1] - e[1] * rp * 2.4)]], 1),
                            LEAF, 1.0)
                arr = blend(arr, _mask([[(c[0] + rp * 1.1 * math.cos(a), c[1] + rp * 1.1 * math.sin(a))
                                         for a in np.linspace(0, 2 * math.pi, 7)[:-1]]]), ROSE, 1.0)
            else:                                                    # foam finger: a big orange hand, finger up
                e = np.array([math.cos(spin * 0.5), math.sin(spin * 0.5)])
                n_ = np.array([-e[1], e[0]])
                hand = [c + e * rp * 1.2 + n_ * rp, c + e * rp * 1.2 - n_ * rp, c - e * rp * 0.6 - n_ * rp,
                        c - e * rp * 0.6 + n_ * rp]
                arr = blend(arr, _mask([hand]), INK, 0.6)
                arr = blend(arr, _mask([[q * 0.86 + c * 0.14 for q in hand]]), self.col, 1.0)
                arr = blend(arr, _mask([], [[tuple(c + e * rp * 1.1), tuple(c + e * rp * 3.0)]], 2), self.col, 1.0)
        st = F.fs[k]
        if st['vis'] and st['scale'] > 0.5:
            p = st['p']
            hgt = R + st['lift']
            c = cam.wp(S.to_world(p[0], p[1], hgt)[None])[0]
            rp = R * cam.f / cam.depth(S.to_world(p[0], p[1], hgt)[None])[0]
            ps = self.pose_at(t)
            if ps is not None and ps[0] < 0.9:                       # honk lines from the bell
                kk = 1 - ps[0]
                for j in range(3):
                    a = -0.9 + 0.5 * j
                    r0, r1 = rp * (1.15 + 0.5 * ps[0]), rp * (1.55 + 0.7 * ps[0])
                    seg = [(c[0] + r0 * math.cos(a), c[1] + r0 * math.sin(a)), (c[0] + r1 * math.cos(a), c[1] + r1 * math.sin(a))]
                    arr = blend(arr, _mask([], [seg], 2), np.clip(self.col * 0.6 + 110, 0, 255), kk)
            hy = self.hype(F.t)                                      # the hype meter: six pips over the ball
            n_on = int(hy + 1e-9)
            for j in range(int(self.cfg['weapon']['n_max'])):
                a = -math.pi / 2 + (j - 2.5) * 0.30
                q = (c[0] + rp * 1.28 * math.cos(a), c[1] + rp * 1.28 * math.sin(a))
                on = j < n_on
                colj = (np.clip(self.col * 0.7 + 90, 0, 255) if on else np.array([70, 60, 70], float))
                if ps is not None and on:
                    colj = np.clip(colj + 80 * (1 - ps[0]), 0, 255)
                x_, y_ = int(round(q[0])), int(round(q[1]))
                if 1 <= x_ < W - 1 and 1 <= y_ < H - 1:
                    arr[y_ - 1:y_ + 1, x_ - 1:x_ + 1] = INK
                    arr[y_, x_] = colj
            if st.get('surf'):                                       # the crowd's hands holding it up
                for j in range(5):
                    xx = c[0] - rp * 0.9 + j * rp * 0.45
                    wob = math.sin(F.t * 9 + j * 1.7) * 1.5
                    seg = [(xx, c[1] + rp * 0.9 + 6), (xx + wob, c[1] + rp * 0.7)]
                    arr = blend(arr, _mask([], [seg], 2), np.array([60, 40, 52], float), 0.9)
        return arr, depth

    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'dive' and owner == self.k:
            return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4, 2.2, t, k=1 - r / rmax, seed=owner)
        return img

    # ---------------------------------------------------------------- its parts (K.O. shatter)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([p[0], p[2]])
        if kind == 'pearl':
            sp = P.screen(cam, np.array([S.to_world(c[0], c[1], p[1])]))[0]
            x, y = int(sp[0]), int(sp[1])
            if 1 <= x < W - 1 and 1 <= y < H - 1:
                arr[y - 1:y + 1, x - 1:x + 1] = PEARL
            return arr
        if kind == 'key':
            ring = [c + np.array([math.cos(a), math.sin(a)]) * size * 1.0 for a in np.linspace(0, 2 * math.pi, 9)[:-1]]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in ring]))
            P.layer_blend(arr, [sp], KEYG * (0.8 + 0.3 * abs(math.cos(rot))), 1.0)
            return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), np.array([80, 40, 12], float), 0.7)
        pts = [c - d * size * 2.0 + nn * curl * 0.05, c + d * size * 2.0 - nn * curl * 0.05]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
        arr = blend(arr, _mask([], [sp], 4), np.array([90, 50, 14], float), 0.9)
        return blend(arr, _mask([], [sp], 3), AMBER * (0.8 + 0.35 * abs(math.cos(rot))), 1.0)


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.7, 0.22
    band = (160, 700)                                     # the honking low line's fundamentals + husk
    colour_args = dict(cut0=60, cut_span=200, drive=5.0)

    def __init__(self, M, k):
        super().__init__(M, k)
        import pp_bridge as B
        self.B = B
        self.mod = B.module('tenorsax_lowpoly')
        self.pan = -0.15 if k == 0 else 0.15
        self.lo, self.hi = M.CFG[k]['register']
        self.g0 = M.inst_gain(M.CFG[k]['pp'])
        self._c = {}

    def tone(self, m, dur, vel=0.85, honk=0.35, sub=0.0):
        key = (int(m), round(dur, 3), round(vel, 2), round(honk, 2), round(sub, 2))
        if key not in self._c:
            x = self.mod.tenor_tone(self.mod._midi_hz(int(m)), dur, vel, honk=honk, sub=sub)
            self._c[key] = self.M.fit(self.B._resample(np.asarray(x, float)), dur + 0.25) * self.g0
        return self._c[key]

    def voice(self, m, dur, vel=0.85):
        return self.tone(m, dur, vel)

    def fit_reg(self, m, lo=None, hi=None):
        lo, hi = lo or self.lo, hi or self.hi
        while m > hi:
            m -= 12
        while m < lo:
            m += 12
        return m

    def line(self, t):
        return self.fit_reg(self.T.line_at(self.name, t), self.lo, self.lo + 19)

    def root(self, t):
        return self.T.root_above(self.T.chord_at(t), self.lo)

    def col(self, x, t, **kw):
        return self.colour(x, t, **kw)

    def squish(self, seed):
        A = self.M.A
        n = int(0.16 * self.M.SR)
        t = A.tt(n)
        x = A.lp(np.random.default_rng(seed).normal(0, 1, n), 900) * np.exp(-t / 0.05)
        return x + np.sin(2 * np.pi * 95 * t) * np.exp(-t / 0.04) * 0.6

    def title_blast(self, mix, t):
        """a big rock 'n' roll honk (config TITLE_HITS: a low honk with an octave and a growl on top)"""
        hits = self.T.TITLE_HITS.get(self.name) or (self.root(0.0),)
        M = self.M
        mix.place(self.bus, M.gliss(self.tone(hits[0], 0.6, 1.0, honk=1.0), 2.0, 0.07), t, 0.62, pan=self.pan)
        for j, m in enumerate(hits[1:]):
            mix.place(self.bus, self.tone(m, 0.45, 0.9, honk=0.7), t + 0.02 * (j + 1), 0.30, pan=self.pan)

    def count_in(self, mix):
        T = self.T
        r = self.root(T.BAR)
        for j, (dt_, m) in enumerate(((0.0, r), (T.BEAT, r), (2 * T.BEAT, r + 7), (3 * T.BEAT, r + 5),
                                      (3.5 * T.BEAT, r + 3))):
            mix.place(self.bus, self.tone(self.fit_reg(m), T.BEAT * 0.4, 0.9, honk=0.8), T.BAR + dt_, 0.42,
                      pan=self.pan)

    def play(self, mix):
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16 = M.q16
        th = M.Thinner(530 + k)
        riff_j = {}
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if e[2] != k:
                continue
            tq = q16(t)
            if kind == 'pose':                                 # the pose: a big honk on the root
                m = self.root(tq)
                sig, g, _ = self.col(self.tone(m, T.BEAT * 0.7, 1.0, honk=1.0), t)
                mix.place(self.bus, M.gliss(sig, 1.5, 0.06), tq, 0.55 * g, pan=self.pan)
            elif kind == 'throw':                              # each throw: a note of the riff (8ths of its line)
                vol = e[11]
                j = riff_j.get(vol, 0)
                riff_j[vol] = j + 1
                if not th.keep(round(tq / T.S16), M.hp_at(k, tq) / 100.0):
                    continue
                m = self.line(tq)
                sig, g, _ = self.col(self.tone(m, T.S16 * 1.7, 0.9, honk=0.45), t)
                mix.place(self.bus, sig, tq, 0.44 * g, pan=self.pan + 0.1 * math.sin(j))
            elif kind == 'splat':
                if e[6]:                                       # a landed throw: a growl stab + the squish
                    m = self.line(tq) + 12
                    m = self.fit_reg(m, self.lo + 7, self.hi)
                    mix.place(self.bus, self.tone(m, T.S16 * 1.2, 1.0, honk=1.0), tq, 0.28, pan=self.pan)
                    mix.place('fx', self.squish(int(t * 100)), tq, 0.10)
                else:
                    mix.place('fx', self.squish(int(t * 100)), tq, 0.04)
            elif kind == 'hush':                               # SUBTONE: a breathy subtone over the crowd's hush
                t1 = e[3]
                m = self.root(tq) + 7
                mix.place(self.bus, self.tone(self.fit_reg(m), max(0.2, t1 - tq), 0.6, honk=0.0, sub=1.0), tq, 0.40,
                          pan=self.pan)
                n = int(max(0.2, t1 - tq) * M.SR)
                sh = A.bp(np.random.default_rng(7).normal(0, 1, n), 2500, 8000) * np.linspace(1, 0.2, n)
                mix.place('fx', sh, tq, 0.05)
            elif kind == 'subtone':                            # ... then one fat growl as the crowd lets fly
                m = self.root(tq)
                mix.place(self.bus, self.tone(m, T.BEAT, 1.0, honk=1.0), tq, 0.58, pan=self.pan)
                mix.place(self.bus, self.tone(m + 12, T.BEAT * 0.8, 0.9, honk=0.8), tq, 0.25, pan=self.pan)
                mix.place('fx', M.roar(1.4, seed=int(t * 3)), tq, 0.06)
            elif kind == 'runway':                             # WALK THE BAR: honks up the line on every beat
                t0, t1 = tq, e[8]
                nb = int(round((t1 - t0) / T.BEAT))
                for j in range(nb):
                    tj = t0 + j * T.BEAT
                    if tj > M.T_KO:
                        break
                    m = self.fit_reg(self.line(tj) + (0 if j % 2 == 0 else 5), self.lo, self.lo + 19)
                    sig, g, _ = self.col(self.tone(m, T.BEAT * 0.55, 1.0, honk=0.9), tj)
                    mix.place(self.bus, sig, tj, 0.48 * g, pan=self.pan)
                    mix.place('fx', A.thud(0.2, 90, 50, seed=j), tj, 0.08)
            elif kind == 'dive':                               # STAGE DIVE: a rising scream into the crowd's roar
                m = self.fit_reg(self.root(tq) + 12, self.lo + 12, self.hi)
                mix.place(self.bus, M.gliss(self.tone(m, T.BEAT * 1.6, 1.0, honk=0.9), 7.0, 0.35), tq, 0.45,
                          pan=self.pan)
                mix.place('fx', M.roar(2.5, seed=5), tq + 0.2, 0.10)
            elif kind == 'dive_launch':
                m = self.fit_reg(self.root(tq) + 19, self.lo + 12, self.hi)
                mix.place(self.bus, self.tone(m, T.BEAT * 0.8, 1.0, honk=1.0), tq, 0.42, pan=self.pan)
                mix.place('fx', A.whoosh(0.5, 300, 6000), tq, 0.14)
            elif kind == 'dive_land':
                m = self.lo
                mix.place(self.bus, self.tone(m, T.BEAT * 1.2, 1.0, honk=1.0), tq, 0.62, pan=self.pan)
                mix.place('fx', A.thud(0.8, 110, 28, seed=9), tq, 0.40)

    def fall_apart(self, mix):
        M = self.M
        t0 = M.P.T_HS
        m = self.root(t0)
        mix.place(self.bus, M.bend(self.tone(m, 1.3, 1.0, honk=1.0), -7.0, 0.1, 1.2), t0, 0.5, pan=self.pan)
        for j in range(4):
            mix.place('fx', M.clank(84 - 3 * j, seed=j + 4, dur=0.25), t0 + 0.06 + 0.07 * j, 0.05)

    def stinger(self, mix, tw):
        r = self.fit_reg(self.T.root_above(self.T.CHORDS[0], self.lo))
        mix.place('solo', self.M.gliss(self.tone(r, 0.9, 1.0, honk=1.0), 2.0, 0.08), tw, 0.6)
        mix.place('solo', self.tone(r + 12, 0.7, 0.9, honk=0.7), tw, 0.25)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            x = self.tone(m, max(0.1, d * T.BEAT * 0.95), 0.95, honk=0.6 if d < 1.5 else 0.3)
            if d >= 1.5:
                x = self.M.vibrato(x, 0.35, 5.0, 0.2)
            mix.place('solo', x, ts, 0.62)

    def reassemble(self, mix):
        T, M = self.T, self.M
        r = self.fit_reg(T.root_above(T.CHORDS[0], self.lo))
        for j, m in enumerate([r, r + 3, r + 5, r + 7, r + 10, r + 12, r + 15, r + 19]):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 8)
            mix.place('solo', self.tone(self.fit_reg(m, self.lo, self.hi), 0.12, 0.8, honk=0.4), tj, 0.30)

    def last_bar(self, mix):
        """the honking low line climbs into the title slam: root, fifth, flat seventh, octave honks on the beats"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        r = self.fit_reg(T.root_above(T.chord_at(t_last), self.lo))
        for bt, m in enumerate((r, r + 7, r + 10, r + 12)):
            mix.place(self.bus, self.tone(self.fit_reg(m), T.BEAT * 0.6, 1.0, honk=0.9), t_last + bt * T.BEAT, 0.50,
                      pan=self.pan)
