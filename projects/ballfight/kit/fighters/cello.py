"""CELLO (rust), new in #47 Cello vs Contrabass: the only grappler. THE EMBRACE (every bar it rushes in, wraps its bow
arm round the enemy, swings it round for a beat and flings it into the ropes; a rope hit right after costs extra),
VIBRATO (a held enemy shakes and takes a hit every 16th), PRELUDE (Bach's rocking Cello Suite No. 1 prelude: a ring of
note-waves on every 16th for a bar, their reaches rocking with the figure), SWAN SONG (it rises, glides across the ring
like a swan and dives onto the enemy: Saint-Saens' The Swan).

Look: the Pixel Pitch cello (a little smaller in its glass than usual: the double bass towers over it). The bow is its
arm: on a rush it reaches out ahead (stick, ivory hair, ebony frog), on a grab it wraps round the far side of the enemy,
which is swept round off its feet with a rust motion trail, flung away with a burst of rosin dust; a rope slam rings
the ropes. VIBRATO: the held enemy trembles inside rust zig-zags. PRELUDE: thin amber rings rock out across the canvas,
eighth notes riding each one. SWAN SONG: it lifts off on two white wings (its shadow falling away below), glides on a
curve, folds its wings and dives into the canvas like into a lake: a splash ring, foam and feathers.
Sound: the Pixel Pitch cello sings the tango's melody (config LINES, a note a beat; repeated notes tie into long bows),
fuller as it leads, thinner and dirtier as it loses; a rush slides up into the next note, a grab swells it with a
double stop, the hold bows with a deep vibrato, a throw is a marcato stab and a rope slam a crunching double stop (the
band stops for that beat: Sound.stops()). VIBRATO trembles on the 16ths; PRELUDE plays Bach's rocking figure over the
chords (one note per note-wave); SWAN SONG plays The Swan's opening phrase, in the episode's key."""
import math

import numpy as np
from reelkit.config import W, H, BAYER_FULL, ss
from reelkit import fx
import scene as S

from . import base
from ._reeds import _mask, blend, add, INK, WHITE

R = S.R_BALL
STICK = np.array([92, 52, 30], float)              # pernambuco bow stick
HAIR = np.array([240, 230, 206], float)            # the bow's ivory hair
FROG = np.array([24, 20, 22], float)               # ebony frog
RUST_DUST = [(240, 220, 170), (220, 190, 140), (255, 246, 222)]   # rosin dust
WING = np.array([250, 250, 246], float)
WING_D = np.array([196, 200, 214], float)
WATER = np.array([196, 230, 255], float)
BODY = np.array([188, 84, 40], float)              # the cello's varnish (the Pixel Pitch model's)
EBONY = np.array([30, 24, 26], float)

# Bach, Cello Suite No. 1 in G, BWV 1007, Prelude, bar 1 (public domain): G2 D3 B3 A3 B3 D3 B3 D3. As chord degrees over
# whatever chord is playing: (chord tone index, octave above the root) - root, fifth, third + 8ve, the step under it, ...
PRELUDE_FIG = [(0, 0), (2, 0), (1, 1), ('step', 1), (1, 1), (2, 0), (1, 1), (2, 0)]
# Saint-Saens, Le cygne (The Swan, 1886, public domain), the cello's opening phrase in G: G F# B | E D G | A B C - B, as
# (beat, semitones above the key's tonic, beats); the 3rd and 6th follow the episode's scale (harmonic minor: the
# leading tone F# stays raised)
SWAN = [(0.0, 12, 1.5), (1.5, 11, 0.5), (2.0, 4, 2.0), (4.0, 9, 1.5), (5.5, 7, 0.5), (6.0, 0, 2.0),
        (8.0, 2, 0.5), (8.5, 4, 0.5), (9.0, 5, 1.0), (10.0, 4, 2.0)]


def _seg_pts(cam, P):
    return cam.wp(S.to_world(P[:, 0], P[:, 1], P[:, 2]))


def draw_bow(img, cam, P, k=1.0, frog=True, col=None):
    """a bow along ring-frame points P (n, 3): an inked stick, the ivory hair beside it, the ebony frog at P[0]"""
    if k <= 0.02 or len(P) < 2:
        return img
    sp = _seg_pts(cam, P)
    img = blend(img, _mask([], [sp], 4), INK, 0.6 * k)
    img = blend(img, _mask([], [sp], 2), STICK if col is None else col, k)
    hp = _seg_pts(cam, P + np.array([0.0, 0.0, -0.05]))
    img = blend(img, _mask([], [hp], 1), HAIR, 0.95 * k)
    if frog:
        f = sp[0]
        x, y = int(round(f[0])), int(round(f[1]))
        if 2 <= x < W - 2 and 2 <= y < H - 2:
            img[y - 2:y + 2, x - 2:x + 2] = img[y - 2:y + 2, x - 2:x + 2] * (1 - k) + FROG * k
    return img


def arm_wrap(me, en, h, rw=None, open_=1.25, n=18):
    """the bow arm hugging the enemy: from the cello's surface toward it, round the enemy's far side"""
    d = en - me
    phi = math.atan2(d[1], d[0])
    rw = R * 1.1 if rw is None else rw
    th = np.linspace(phi + math.pi - open_, phi - math.pi + open_, n)       # round the far side
    arc = np.column_stack([en[0] + rw * np.cos(th), en[1] + rw * np.sin(th), np.full(n, h)])
    start = np.array([me[0] + math.cos(phi) * R * 0.85, me[1] + math.sin(phi) * R * 0.85, h])
    return np.vstack([start[None], arc])


def wing_poly(cx, cy, side, span, flap, fold):
    """one swan wing in screen space: a fan of three feathers from the ball's shoulder"""
    pts = [(cx + side * 3, cy - 2)]
    base = -0.25 + 0.75 * flap * (1 - fold)                 # radians above horizontal
    for j, (ln, da) in enumerate(((1.0, 0.0), (0.85, -0.35), (0.62, -0.7))):
        a = base + da * (1 - 0.6 * fold) + fold * 1.1
        L = span * ln * (1 - 0.45 * fold)
        pts.append((cx + side * (3 + L * math.cos(a)), cy - 2 - L * math.sin(a)))
    pts.append((cx + side * 3, cy + 3))
    return pts


def title_pattern(base):
    """CELLO: bow-hair stripes along the divider with rows of f-holes between them"""
    import title as TT
    XX, YY, SIDE = TT.XX, TT.YY, TT.SIDE
    u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
    v = SIDE
    stripe = np.abs(((np.abs(v) + 30) % 84.0) - 42.0) < 1.3
    base = np.where(stripe[..., None], base * 0.45 + HAIR * 0.55, base)
    cu, cv = (u % 70.0) - 35.0, (np.abs(v) % 84.0) - 42.0
    f = (np.abs(cv - 0.5 * np.sin(cu / 6.0) * 6.0 - cu * 0.25) < 2.2) & (np.abs(cu) < 15)
    eyes = ((cu - 14) ** 2 + (cv - 3) ** 2 < 10) | ((cu + 14) ** 2 + (cv + 11) ** 2 < 10)
    return np.where((f | eyes)[..., None], base * 0.25 + np.array([30, 14, 10]) * 0.75, base)


# ===================================================================================================== look
class Look(base.Look):
    shards = ['body'] * 5 + ['neck'] * 2 + ['bow'] * 2 + ['string'] * 4
    spark_big = ('ropeslam', 'swan', 'throw')
    spark_flash = ('grab',)
    flash_hits = ('ropeslam', 'swan')
    fringe_hits = ('ropeslam', 'swan')
    quiet_hits = ('vibrato', 'prelude')
    rope_kinds = {}
    sprite_k = 0.74                                   # a little small in its glass: the double bass towers over it
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.HOLDS = [dict(t0=e[0], t1=e[5], a0=e[3], a1=e[4]) for e in EV if e[1] == 'hold' and e[2] == k]
        self.THROWS = [dict(t=e[0], a=e[3], power=e[4], x=e[5], y=e[6]) for e in EV if e[1] == 'throw' and e[2] == k]
        self.SLAMS = [(e[0], e[3], e[4]) for e in EV if e[1] == 'ropeslam' and e[2] == k]
        self.GRABS = [(e[0], e[3], e[4], e[5]) for e in EV if e[1] == 'grab' and e[2] == k]
        self.VIB = [(e[0], e[3]) for e in EV if e[1] == 'vibrato' and e[2] == k]
        self.NOTES = [dict(t=e[0], j=e[3], x=e[4], y=e[5], rmax=e[6]) for e in EV if e[1] == 'prelude_note' and e[2] == k]
        self.SWANS = [dict(t0=e[3], tg=e[4], td=e[5], tl=e[6], p0=np.array([e[7], e[8]]), c=np.array([e[9], e[10]]),
                           p1=np.array([e[11], e[12]]), h=e[13]) for e in EV if e[1] == 'swan' and e[2] == k]
        self.LANDS = [(e[0], e[3], e[4], e[5]) for e in EV if e[1] == 'swan_land' and e[2] == k]
        self.RELEASES = [e[0] for e in EV if e[1] == 'release' and e[2] == k]

    # ---------------------------------------------------------------- clocks
    def show(self, t):
        return self.T.T_FIGHT <= t < self.P.T_HS + 0.12

    def ta(self, t):
        return min(t, self.P.T_KO)

    def grab_at(self, t):
        """(state, arm angle, t0, t1, reach) of its grapple at t (the run's record), or None"""
        P = self.P
        if not (self.T.T_FIGHT <= t < P.T_HS) or 'GRAB' not in P.RUN:
            return None
        g = P.RUN['GRAB'][P.sim_i(min(t, P.T_KO)), self.k]
        return (int(g[0]), float(g[1]), float(g[2]), float(g[3]), float(g[4])) if g[0] > 0 else None

    def vib_on(self, t):
        return any(t0 <= t < t1 for t0, t1 in self.VIB)

    def swan_at(self, t):
        """(swan, height, phase 0 rise / 1 glide / 2 dive, u) while airborne (attack clock), else None"""
        tc = self.P.fx_clock(t)
        for sw in self.SWANS:
            if sw['t0'] <= tc < sw['tl']:
                h = sw['h']
                if tc < sw['tg']:
                    u = (tc - sw['t0']) / (sw['tg'] - sw['t0'])
                    return sw, h * (u * u * (3 - 2 * u)), 0, u
                if tc < sw['td']:
                    u = (tc - sw['tg']) / (sw['td'] - sw['tg'])
                    return sw, h * (1 + 0.08 * math.sin(2 * math.pi * u * 2)), 1, u
                u = (tc - sw['td']) / (sw['tl'] - sw['td'])
                return sw, h * (1 - u * u), 2, u
        return None

    # ---------------------------------------------------------------- state
    def state(self, kk, st, t, phase, gl=0.0):
        if phase != 'fight':
            return
        g = self.grab_at(t)
        if kk == self.k:
            sw = self.swan_at(t)
            if sw is not None:
                st['lift'] = sw[1]
                st['glow'] = max(st['glow'], 0.35)
            if g is not None and g[0] == 2:
                st['glow'] = max(st['glow'], 0.25 + 0.25 * self.vib_on(self.ta(t)))
            return
        if g is not None and g[0] == 2:                        # the held enemy: swept off its feet, trembling
            u = (self.ta(t) - g[2]) / max(1e-6, g[3] - g[2])
            st['lift'] = 0.16 * math.sin(math.pi * min(1.0, u))
            if self.vib_on(self.ta(t)):
                f = int(round(t * 30))
                st['p'] = st['p'] + 0.05 * np.array([math.sin(f * 2.7), math.cos(f * 3.9)])

    # ---------------------------------------------------------------- events -> camera, ropes, sparks, glows
    def shake(self, e):
        if e[1] == 'ropeslam':
            return 1.8
        if e[1] == 'swan_land':
            return 2.4 if e[5] else 1.4
        if e[1] == 'throw':
            return 0.8
        if e[1] == 'hit' and e[7] == 'grab':
            return 0.5
        return None

    def rope(self, e):
        if e[1] == 'ropeslam':
            return 1.0
        if e[1] == 'swan_land':
            return 0.6
        return None

    def spark(self, e):
        how = e[7]
        if how == 'ropeslam':
            return np.array([255, 190, 140], float), 18, 3.4, 0.34
        if how == 'swan':
            return WATER, 20, 3.2, 0.4
        if how in ('vibrato', 'prelude'):
            return np.array([255, 200, 140], float), 3, 1.2, 0.14
        return np.array([255, 196, 150], float), 8, 2.2, 0.2

    def hit_glow(self, e):
        if e[7] in ('ropeslam', 'swan'):
            return 16, 0.42
        if e[7] in ('grab', 'throw'):
            return 11, 0.32
        return None

    # ---------------------------------------------------------------- effects (reelkit.fx)
    def fx_setup(self):
        P, k = self.P, self.k
        col = P.FL_COL[k]
        for t0, how, x, y in self.GRABS:
            if t0 > P.T_KO:
                break
            pos = P.up(x, y, R)
            if how == 1:                                         # the embrace: rosin dust off the bow
                P.PA.emit(t0, pos, 'rosin', n=10, col=RUST_DUST)
                P.FL.add(t0, pos, col, radius=1.2, peak=0.4, decay=0.2)
            elif how == 2:                                       # it slipped off a walking bass: a puff
                P.PA.emit(t0, pos, 'puffs', n=4, col=RUST_DUST)
        for th in self.THROWS:
            if th['t'] > P.T_KO:
                break
            pos = P.up(th['x'], th['y'], R)
            P.BU.add(th['t'], pos, P.COL[k], 0.9)
            P.PA.emit(th['t'], pos, 'rosin', n=14, col=RUST_DUST, speed=(0.6, 1.6))
            P.FL.add(th['t'], pos, col, radius=1.6, peak=0.6, decay=0.22)
        for t0, x, y in self.SLAMS:
            if t0 > P.T_KO:
                break
            pos = P.up(x, y, R * 0.9)
            P.BU.add(t0, pos, P.COL[k], 1.5)
            P.FL.add(t0, pos, col, radius=2.0, peak=0.9, decay=0.3)
            P.PA.emit(t0, pos, 'sparks', n=16, col=[(255, 210, 160), (255, 160, 100), (255, 246, 222)])
            P.MK.add(t0, 'claw', x, y, size=0.5, life=4.0, seed=int(t0 * 100) % 997, strength=0.5)
        for nt in self.NOTES:                                    # PRELUDE: each note-wave lights the cello
            if nt['t'] > P.T_KO:
                break
            P.FL.add(nt['t'], P.up(nt['x'], nt['y'], R), col, radius=1.0 + 0.5 * nt['rmax'], peak=0.25, decay=0.12)
        for sw in self.SWANS:                                    # SWAN SONG: feathers on take-off
            if sw['t0'] > P.T_KO:
                break
            P.PA.emit(sw['t0'] + 0.1, P.up(sw['p0'][0], sw['p0'][1], R), 'shreds', n=8, col=[(250, 250, 246), (220, 224, 236)])
        for t0, x, y, hit in self.LANDS:                         # the dive: a splash into the canvas
            if t0 > P.T_KO:
                break
            pos = P.up(x, y, 0.1)
            P.BU.add(t0, pos, (210, 236, 255), 1.8)
            P.FL.add(t0, P.up(x, y, R), (190, 220, 255), radius=2.6, peak=1.0, decay=0.4)
            P.PA.emit(t0, pos, 'foam', n=18, col=[(230, 246, 255), (190, 225, 255), (255, 255, 255)])
            P.PA.emit(t0, P.up(x, y, R), 'shreds', n=10, col=[(250, 250, 246), (220, 224, 236)])
            P.MK.add(t0, 'water', x, y, size=1.2, life=5.0, seed=17, strength=0.8)

    def fx_event(self, e):
        P = self.P
        if e[1] != 'hit':
            return
        how = e[7]
        pos = S.to_world(e[5], e[6], R)
        if how == 'grab':
            P.BU.add(e[0], pos, P.COL[e[2]], 0.6, bright=False)
        elif how == 'vibrato':
            P.PA.emit(e[0], pos, 'rosin', n=2, col=RUST_DUST, speed=(0.3, 0.8))

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'sparks', n=26, spread=180, speed=(0.8, 2.4), col=[(255, 200, 150), (240, 230, 206)])
        P.PA.emit(P.T_KO, pos, 'chips', n=10, col=[tuple(BODY), (30, 24, 26), (240, 230, 206)])

    # ---------------------------------------------------------------- the canvas: PRELUDE's rings, the splash
    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind != 'prelude' or owner != self.k:
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)
        band = np.clip(1 - np.abs(rd - r) / 0.09, 0, 1) * (1 - r / rmax) ** 0.6
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * 0.85)[:, None] * (np.array([255, 196, 110]) / 255.0)[None]
        return Ic

    def canvas_after(self, Ic, C):
        tc = self.P.fx_clock(C.t)
        for t0, x, y, hit in self.LANDS:                          # the splash ring spreading like water
            age = tc - t0
            if 0 <= age < 0.9:
                rd = np.hypot(C.sxp - x, C.szp - y)
                for j in range(2):
                    rr = 0.3 + (2.2 - 0.5 * j) * (1 - (1 - min(1.0, age / 0.9)) ** 2)
                    band = np.clip(1 - np.abs(rd - rr) / 0.07, 0, 1) * (1 - age / 0.9)
                    band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                    Ic += (band * (0.9 - 0.3 * j))[:, None] * (WATER / 255.0)[None]
        return Ic

    # ---------------------------------------------------------------- over the ring: the bow arm, rings, wings
    def over_ring(self, F, arr, depth):
        P, cam, k = self.P, F.cam, self.k
        t = F.t
        col = P.COL[k]
        if not self.show(t):
            return arr, depth
        st, en_st = F.fs[k], F.fs[self.enemy]
        me, en = st['p'], en_st['p']
        g = self.grab_at(t)
        sw = self.swan_at(t)
        if g is not None and sw is None and st['vis']:
            state, a, t0, t1, reach = g
            if state == 1:                                        # the rush: the bow arm reaches out ahead
                u = min(1.0, (self.ta(t) - t0) / 0.08)
                d = np.array([math.cos(a), math.sin(a)])
                L = R * 0.6 + (reach + 0.45) * u
                Pp = np.array([[me[0] + d[0] * R * 0.7, me[1] + d[1] * R * 0.7, R * 1.15],
                               [me[0] + d[0] * L, me[1] + d[1] * L, R * 1.05]])
                Pp = np.vstack([Pp[0] + (Pp[1] - Pp[0]) * s_ for s_ in np.linspace(0, 1, 6)])
                arr = draw_bow(arr, cam, Pp, 1.0)
            elif state == 2:                                      # the embrace: the arm round the far side
                h = R * 1.05 + en_st.get('lift', 0.0) * 0.6
                ta = self.ta(t)
                u = (ta - t0) / max(1e-6, t1 - t0)
                # the swing's trail: where the enemy was over the last 0.12 s
                trail = []
                for dt_ in np.linspace(0.12, 0.0, 7):
                    q = P.fighter(self.enemy, max(t0, ta - dt_))['p']
                    trail.append([q[0], q[1], R * 0.9])
                tr = np.array(trail)
                if len(tr) > 1:
                    sp = _seg_pts(cam, tr)
                    arr = add(arr, _mask([], [sp], 7), col * 0.6 + 40, 0.22 * min(1.0, u * 3))
                    arr = blend(arr, _mask([], [sp], 3), np.clip(col * 0.8 + 60, 0, 255), 0.5 * min(1.0, u * 3))
                arr = draw_bow(arr, cam, arm_wrap(me, en, h), 1.0)
                if self.vib_on(ta):                               # VIBRATO: zig-zags trembling round it
                    for sgn in (-1, 1):
                        d = en - me
                        phi = math.atan2(d[1], d[0]) + sgn * 1.2
                        th = np.linspace(phi - 0.5, phi + 0.5, 12)
                        rr = R * 1.45 + 0.05 * np.sin(np.arange(12) * 2.6 + t * 60)
                        Z = np.column_stack([en[0] + rr * np.cos(th), en[1] + rr * np.sin(th), np.full(12, R)])
                        sp = _seg_pts(cam, Z)
                        arr = blend(arr, _mask([], [sp], 3), INK, 0.5)
                        arr = blend(arr, _mask([], [sp], 1), np.clip(col * 0.8 + 70, 0, 255), 1.0)
        # a grab that slipped off a walking bass: a quick scrape
        tc = self.ta(t)
        for t0, how, x, y in self.GRABS:
            if how == 2 and 0 <= tc - t0 < 0.18:
                kk = 1 - (tc - t0) / 0.18
                d = np.array([x, y]) - me
                n = d / (np.hypot(*d) + 1e-9)
                nn = np.array([-n[1], n[0]])
                Z = np.array([[x + nn[0] * s_ * 0.3, y + nn[1] * s_ * 0.3, R] for s_ in (-1, 1)])
                arr = blend(arr, _mask([], [_seg_pts(cam, Z)], 2), HAIR, 0.8 * kk)
        # PRELUDE: eighth notes riding the rings
        if F.in_fight:
            for owner, cx, cz, r, rmax, kind, t0 in P.RUN['WAVES'][F.i]:
                if kind != 'prelude' or owner != k:
                    continue
                kk = (1 - r / rmax) ** 0.6
                for j in range(4):
                    a = j * math.pi / 2 + t0 * 3.1 + r * 0.9
                    q = cam.wp(S.to_world(cx + r * math.cos(a), cz + r * math.sin(a), 0.12)[None])[0]
                    x_, y_ = int(round(q[0])), int(round(q[1]))
                    if 3 <= x_ < W - 3 and 6 <= y_ < H - 3:
                        c_ = np.clip(col * 0.6 + 110, 0, 255)
                        arr[y_ - 1:y_ + 2, x_ - 2:x_ + 1] = arr[y_ - 1:y_ + 2, x_ - 2:x_ + 1] * (1 - kk) + c_ * kk
                        arr[y_ - 5:y_, x_] = arr[y_ - 5:y_, x_] * (1 - kk) + c_ * kk     # the stem
                        arr[y_ - 5, x_ + 1] = arr[y_ - 5, x_ + 1] * (1 - kk) + c_ * kk  # the flag
        # SWAN SONG: white wings
        if sw is not None and st['vis']:
            swd, h, ph, u = sw
            c = cam.wp(S.to_world(me[0], me[1], R + h)[None])[0]
            rp = R * cam.f / cam.depth(S.to_world(me[0], me[1], R + h))[0]
            flap = 0.5 + 0.5 * math.sin(2 * math.pi * (t - swd['t0']) * (2.2 if ph == 0 else 1.3))
            fold = 0.0 if ph < 2 else min(1.0, u * 1.6)
            if ph == 0:
                flap = 0.3 + 0.7 * flap
            for side in (-1, 1):
                wp_ = wing_poly(c[0], c[1], side, rp * 1.9, flap, fold)
                arr = blend(arr, _mask([wp_]), INK, 0.7)
                inner = [(c[0] + (x - c[0]) * 0.9, c[1] + (y - c[1]) * 0.9) for x, y in wp_]
                arr = blend(arr, _mask([inner]), WING, 1.0)
                for x, y in wp_[1:4]:                              # feather lines
                    arr = blend(arr, _mask([], [[(c[0] + side * 3, c[1] - 1), (x, y)]], 1), WING_D, 0.8)
            if ph == 2:                                            # the dive's streak
                top = cam.wp(S.to_world(me[0], me[1], R + h + 0.9)[None])[0]
                arr = add(arr, _mask([], [[(c[0], c[1] - rp), (top[0], top[1])]], 3), WING * 0.6, 0.5 * (1 - u * 0.5))
        return arr, depth

    def glows(self, F, drawn):
        P = self.P
        tc = self.ta(F.t)
        for th in self.THROWS:
            if 0 <= tc - th['t'] < 0.15:
                s_ = F.cam.wp(S.to_world(th['x'], th['y'], R)[None])[0]
                F.fr.glow(s_[0], s_[1], 14, tuple(P.COL[self.k] * 0.8 + 50), 0.5 * (1 - (tc - th['t']) / 0.15))

    # ---------------------------------------------------------------- its parts (K.O. shatter)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([sx_, sz_])
        if kind == 'string':
            pts = [c + d * (j - 2) * 0.12 + nn * curl * 0.06 * math.sin(j * 1.7) for j in range(5)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            return blend(arr, _mask([], [sp], 1), np.array([214, 210, 200], float), 0.9)
        if kind == 'bow':
            pts = [c + d * (j - 2) * 0.16 + nn * curl * 0.03 * j for j in range(5)]
            Z = np.array([[q[0], q[1], p[1]] for q in pts])
            return draw_bow(arr, cam, Z, 0.9, frog=False)
        if kind == 'neck':
            ln_, wd_ = 0.28, 0.045
            q4 = [c + d * ln_ * a1 + nn * wd_ * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], EBONY, 1.0)
            return arr
        sz = size * 1.5
        tri = [c + d * sz, c - d * sz * 0.7 + nn * sz * 0.8, c - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        P.layer_blend(arr, [sp], BODY * (0.75 + 0.35 * abs(math.cos(rot))), 1.0)
        return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), np.array([60, 26, 14], float), 0.8)

    def focus_points(self, t, st):
        if st.get('lift', 0.0) > 0:
            p = st['p']
            return [(S.to_world(p[0], p[1], R + st['lift']), 'cello in flight')]
        return []


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.55, 0.38
    band = (180, 1400)                                       # its singing register, above the bandoneon's stabs
    colour_args = dict(cut0=80, cut_span=480, drive=3.5)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.pp = M.CFG[k]['pp']
        self.pan = -0.18 if k == 0 else 0.18
        lo, hi = M.CFG[k]['register']
        self.lo, self.hi = lo, hi
        EV = M.EV
        self.HOLDS = [(e[0], e[5]) for e in EV if e[1] == 'hold' and e[2] == k]
        self.VIB = [(e[0], e[3]) for e in EV if e[1] == 'vibrato' and e[2] == k]
        self.PREL = [(e[0], e[3]) for e in EV if e[1] == 'prelude' and e[2] == k]
        self.SWAN_T = [e[0] for e in EV if e[1] == 'swan' and e[2] == k]

    # ---------------------------------------------------------------- the voice
    def bow(self, m, dur, vel=0.85, vib=0.0):
        x = self.M.note(self.pp, int(m), round(max(0.08, dur), 3), round(max(0.3, dur + 0.25), 2), round(vel, 2))
        if vib > 0:
            x = self.M.vibrato(x, vib, 5.6, 0.06)
        return x

    def fit_reg(self, m, lo=None, hi=None):
        lo, hi = lo or self.lo, hi or self.hi
        while m > hi:
            m -= 12
        while m < lo:
            m += 12
        return m

    def in_spans(self, spans, t):
        return any(a <= t < b for a, b in spans)

    def stops(self):
        """the band's tango stops (a beat's cut) on its rope slams"""
        M = self.M
        return [M.q16(e[0]) for e in M.EV if e[1] == 'ropeslam' and e[2] == self.k and e[0] <= M.T_KO]

    def root_of(self, t):
        return self.T.root_above(self.T.chord_at(t), 48)

    def prelude_note(self, t, j):
        """bar 1 of Bach's prelude as degrees of the chord playing at t"""
        T = self.T
        ch = T.chord_at(t)
        pcs = T.CHORD_PCS[ch]
        r = T.root_above(ch, 43)
        deg, octv = PRELUDE_FIG[j % 8]
        if deg == 'step':                                        # the step under the third (+8ve): a scale tone
            third = r + 12 + ((pcs[1] - pcs[0]) % 12)
            below = [m for m in T.scale_notes(third - 3, third - 1, ch)]
            return below[-1] if below else third - 2
        return r + 12 * octv + ((pcs[deg] - pcs[0]) % 12 if deg else 0)

    def swan_notes(self, t0):
        """The Swan's opening phrase from t0 in the episode's key (its scale decides the 3rd / 6th)"""
        T = self.T
        tonic = 48 + (T.KEY_PC - 48) % 12
        out = []
        for bt, semi, d in SWAN:
            m = tonic + semi
            if semi % 12 == 4 and (T.KEY_PC + 4) % 12 not in [(T.KEY_PC + s) % 12 for s in T.SCALE]:
                m -= 1                                           # minor third
            if semi % 12 == 9 and (T.KEY_PC + 9) % 12 not in [(T.KEY_PC + s) % 12 for s in T.SCALE]:
                m -= 1                                           # minor sixth
            out.append((t0 + bt * T.BEAT, self.fit_reg(m + 12, self.lo + 7, self.hi + 2), d * T.BEAT))
        return out

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """a cello swell: a bowed double stop crescendo (config TITLE_HITS: its notes, else root + fifth)"""
        M = self.M
        hits = self.T.TITLE_HITS.get(self.name) or (self.root_of(0.0), self.root_of(0.0) + 7)
        dur = self.T.BEAT * 0.95
        for j, m in enumerate(hits[:2]):
            x = self.bow(m, dur, 1.0, 0.25)
            n = len(x)
            x = x * np.clip(0.55 + 0.6 * np.linspace(0, 1, n) ** 0.7, 0, 1.1)
            mix.place(self.bus, x, t, 0.42 - 0.08 * j, pan=self.pan)
        for j, m in enumerate(hits[2:]):
            mix.place(self.bus, self.bow(m, 0.12, 0.9), t + 0.05 + 0.05 * j, 0.24, pan=self.pan)

    def count_in(self, mix):
        T = self.T
        r = self.root_of(T.BAR)
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.bow(r + (0, 7, 12)[j], 0.2, 0.9), tc, 0.34, pan=self.pan)
        m = self.fit_reg(T.line_at(self.name, T.T_FIGHT))
        x = self.M.gliss(self.bow(m, T.BEAT * 0.9, 0.95), 7.0, 0.25)
        mix.place(self.bus, x, T.BAR + 3 * T.BEAT, 0.30, pan=self.pan)

    def melody(self):
        """the melody's notes over the fight: (t, midi, beats) with repeats tied, gaps for PRELUDE / SWAN SONG"""
        M, T = self.M, self.T
        cut = list(self.PREL) + [(ts, ts + 12 * T.BEAT) for ts in self.SWAN_T]
        notes = []
        b = int(math.ceil(T.T_FIGHT / T.BEAT - 1e-6))
        while b * T.BEAT < M.T_KO - 0.05:
            t = b * T.BEAT
            if not any(a - 1e-6 <= t < c for a, c in cut):
                m = self.fit_reg(T.line_at(self.name, t))
                if notes and notes[-1][1] == m and abs(notes[-1][0] + notes[-1][2] * T.BEAT - t) < 1e-6 and \
                        notes[-1][2] < 4:
                    notes[-1][2] += 1
                else:
                    notes.append([t, m, 1])
            b += 1
        return notes

    def play(self, mix):
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16 = M.q16
        th = M.Thinner(47 + k)
        rushes = [e[0] for e in M.EV if e[1] == 'rush' and e[2] == k]
        grabs = [e[0] for e in M.EV if e[1] == 'grab' and e[2] == k and e[3] == 1]
        for t, m, beats in self.melody():                      # the singing line
            hp01 = M.hp_at(k, t) / 100.0
            if not th.keep(round(t / T.S16), hp01):
                continue
            dur = beats * T.BEAT * 0.97
            vib = 0.32 if self.in_spans(self.HOLDS, t) else 0.18
            if self.in_spans(self.VIB, t):
                vib = 0.6
            x = self.bow(m, dur, 0.8 + 0.05 * min(beats, 3), vib)
            if any(0 <= t - r_ < T.BEAT for r_ in rushes):      # a rush slides up into the note
                x = M.gliss(x, 4.0, 0.12)
            sig, g, _ = self.colour(x, t)
            mix.place(self.bus, sig, t, 0.34 * g, pan=self.pan)
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if e[2] != k:
                continue
            tq = q16(t)
            if kind == 'grab' and e[3] == 1:                     # the embrace: a swelling double stop
                m = self.fit_reg(T.line_at(self.name, tq))
                lo = max(self.lo, m - 9)
                x = self.bow(lo, T.BEAT * 0.9, 0.9, 0.3)
                x = x * np.clip(np.linspace(0.6, 1.1, len(x)), 0, 1.1)
                mix.place(self.bus, self.colour(x, t)[0], tq, 0.24 * self.colour(x, t)[1], pan=self.pan)
            elif kind == 'throw':                                # marcato stab
                r = self.root_of(tq)
                x = self.bow(r + 12, 0.16, 1.0)
                mix.place(self.bus, self.colour(x, t)[0], tq, 0.36, pan=self.pan)
                mix.place('fx', A.whoosh(0.3, 500, 4000), tq, 0.08, pan=self.pan)
            elif kind == 'ropeslam':                             # a crunching double stop on the band's stop
                r = self.root_of(tq)
                for m, gg in ((r, 0.36), (r + 7, 0.28), (r + 12, 0.18)):
                    x = self.colour(self.bow(m, 0.35, 1.0), t, drive_k=2.0)[0]
                    mix.place(self.bus, x, tq, gg, pan=self.pan)
                mix.place('fx', A.thud(0.5, 140, 40, seed=int(t * 9)), tq, 0.32)
            elif kind == 'hit' and e[7] == 'vibrato':            # VIBRATO: a bowed tremolo shake on the 16ths
                m = self.fit_reg(T.line_at(self.name, tq))
                mix.place(self.bus, self.bow(m, 0.07, 0.75), tq, 0.12, pan=self.pan)
            elif kind == 'prelude_note':                         # PRELUDE: Bach's rocking figure, a note per wave
                m = self.prelude_note(tq, e[3])
                x = self.bow(self.fit_reg(m, self.lo - 5, self.hi), T.S16 * 1.6, 0.85)
                sig, g, _ = self.colour(x, t)
                mix.place(self.bus, sig, tq, (0.34 if e[3] % 8 == 0 else 0.27) * g, pan=self.pan)
            elif kind == 'ability' and e[4] == 'swan':           # SWAN SONG: The Swan's opening phrase
                for ts, m, d in self.swan_notes(M.q16(t)):
                    if ts > M.T_KO + 0.05:
                        break
                    x = self.bow(m, d * 0.98, 0.95, 0.35)
                    if d >= T.BEAT:
                        x = M.gliss(x, 1.0, 0.08)
                    sig, g, _ = self.colour(x, ts)
                    mix.place(self.bus, sig, ts, 0.42 * g, pan=self.pan * 0.5)
            elif kind == 'swan_land':                            # the dive into the lake
                mix.place('fx', A.thud(0.8, 120, 30, seed=7), tq, 0.4)
                n = int(0.8 * M.SR)
                tt_ = A.tt(n)
                splash = A.bp(np.random.default_rng(5).normal(0, 1, n), 600, 7000) * np.exp(-tt_ / 0.18)
                mix.place('fx', splash, tq, 0.22)

    def fall_apart(self, mix):
        """the strings let go: a falling glissando and a snapped string"""
        M, A = self.M, self.M.A
        t0 = M.P.T_HS
        r = self.root_of(t0)
        mix.place(self.bus, M.bend(self.bow(r + 12, 1.4, 1.0, 0.3), -14.0, 0.05, 1.3), t0, 0.40, pan=self.pan)
        n = int(0.5 * M.SR)
        tt_ = A.tt(n)
        snap = np.sin(2 * np.pi * 1800 * tt_ * np.exp(-tt_ * 3)) * np.exp(-tt_ / 0.05)
        mix.place('fx', snap, t0 + 0.08, 0.12)

    def stinger(self, mix, tw):
        r = self.root_of(0.0)
        for m, g in ((r, 0.4), (r + 7, 0.32), (r + 12, 0.24)):
            mix.place('solo', self.bow(m, 1.1, 1.0, 0.3), tw, g)

    def solo(self, mix, t0):
        """config SOLO: the cello's tango cadenza, long notes slid into and bowed with a deep vibrato"""
        T, M = self.T, self.M
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            x = self.bow(m, max(0.1, d * T.BEAT * 0.96), 0.95, 0.4 if d >= 1 else 0.15)
            if d >= 1.0:
                x = M.gliss(x, 2.0, 0.1)
            mix.place('solo', x, ts, 0.44)

    def reassemble(self, mix):
        """its pieces fly home as rising pizzicato"""
        T, M = self.T, self.M
        r = self.root_of(0.0)
        run = [r - 12, r - 5, r, r + 3, r + 7, r + 12, r + 15, r + 19]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place('solo', self.bow(self.fit_reg(m, self.lo - 12), 0.09, 0.8), tj, 0.20)

    def last_bar(self, mix):
        """into the title slam: a tremolo swell on the dominant's fifth"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        m = self.fit_reg(T.root_above(ch, 48) + 7)
        n16 = int(round((T.DUR - t_last) / (T.S16 / 2)))
        for j in range(n16):
            tj = t_last + j * T.S16 / 2
            mix.place(self.bus, self.bow(m, T.S16 * 0.45, 0.8), tj, 0.10 + 0.10 * j / max(1, n16), pan=self.pan)
