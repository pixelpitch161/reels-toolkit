"""CONTRABASS (steel blue), new in #47 Cello vs Contrabass: the only juggernaut. WALKING BASS (no gliding: one heavy step
a beat, relentlessly toward the enemy, each footfall a small floor-quake that grows per landed quake; while it steps it
can't be knocked back or grabbed and it never gives way), SLAP BASS (a sharp percussive knockback burst that breaks a
grab), DOUBLE TIME (it walks on the 8ths), THE ELEPHANT (an unstoppable stampede across the ring, the floor shaking:
Saint-Saens' The Elephant).

Look: the new Pixel Pitch double bass (contrabass_lowpoly: modelled from reference photos, x1.2 so it towers over the
cello; rendered with a wider crop so the size fits). Each step lifts the ball a little and plants it with a dust puff, a
steel-blue quake ring across the canvas (inked, jagged) and a footprint; SLAP BASS snaps a white string across its glass
and fires a hard white-blue ring; DOUBLE TIME's quick steps kick up more dust; THE ELEPHANT grows grey-blue ears that
flap and a trunk that swings ahead, dust plumes and cracks at every stomp. It shatters into its wood, the scroll, the
endpin and strings. VS card: rows of heavy footprints.
Sound: its steps ARE the walking bass line (config LINES, a note a beat: the footfalls land on the beat; DOUBLE TIME's
8th steps add chromatic approach notes), plucked jazz pizzicato from the new double-bass voice; a landed quake adds a
ghost slap; SLAP BASS is a rockabilly slap with ghost slaps after it; THE ELEPHANT plays the elephant's waltz tune
bowed (arco) and heavy over the stampede's stomps; the title card gets a slapped hit."""
import math

import numpy as np
from reelkit.config import W, H, BAYER_FULL
from reelkit import fx
import scene as S

from . import base
from ._reeds import _mask, blend, add, INK, WHITE

R = S.R_BALL
DUST = [(206, 196, 180), (170, 160, 148), (232, 224, 210)]
EAR = np.array([112, 108, 118], float)                  # elephant grey (clear of the steel-blue glass)
EAR_IN = np.array([214, 150, 160], float)
WOOD = np.array([138, 60, 30], float)
STEEL = np.array([196, 200, 212], float)
EBONY = np.array([28, 22, 24], float)
PAD = 1.25                                               # the sprite's crop margin (the bridge's x1.2 size)
# Saint-Saens, L'elephant (The Carnival of the Animals, 1886, public domain): the double bass's pompous waltz tune, as
# (beat, semitones above the key's tonic, beats) in the bass's register; the 3rd / 6th follow the episode's scale
ELEPHANT = [(0.0, -5, 0.75), (0.75, 0, 0.75), (1.5, 4, 0.75), (2.25, 7, 0.75), (3.0, 12, 1.5), (4.5, 11, 0.5),
            (5.0, 9, 0.5), (5.5, 7, 1.0), (6.5, 5, 0.5), (7.0, 4, 0.5), (7.5, 2, 0.5), (8.0, 0, 1.5)]


def padded_sprite(name, r, yaw, roll, pad=PAD):
    """the Pixel Pitch model rendered like sprites.render_sprite, with a wider crop (its x1.2 size passes the 3 px
    margin above r ~ 20 px)"""
    import sprites as SP                                 # (imports pp_bridge, which puts the series on the path)
    import piano_lowpoly as PL
    R0 = int(round(r * pad))
    cx = cy = R0 + 3
    img = np.zeros((PL.RH, PL.RW, 3), np.uint8)
    zb = np.zeros((PL.RH, PL.RW))
    gb = np.zeros((PL.RH, PL.RW), np.int8)
    cam = SP._Cam(yaw, r, cx + 0.5, cy + 0.5, roll)
    PL.render_geo(SP._geo(name), cam, img, zb, gb, scale=1.0)
    PL.outline(img, zb, gb)
    n = 2 * R0 + 7
    return np.dstack([img[:n, :n], (gb[:n, :n] > 0).astype(np.uint8) * 255])


def jag_ring(cx, cz, r, seed, y=0.05, n=56, amp=0.05):
    a = np.linspace(0, 2 * math.pi, n + 1)
    rng = np.random.default_rng(seed)
    jag = r + amp * rng.uniform(-1, 1, n + 1)
    jag[-1] = jag[0]
    return np.column_stack([cx + jag * np.cos(a), cz + jag * np.sin(a), np.full(n + 1, y)])


def title_pattern(base):
    """CONTRABASS: rows of heavy round footprints marching along the divider"""
    import title as TT
    XX, YY, SIDE = TT.XX, TT.YY, TT.SIDE
    u = (XX - TT.DA[0]) * TT.DD[0] + (YY - TT.DA[1]) * TT.DD[1]
    v = SIDE
    row = np.floor(np.abs(v) / 64.0)
    cu = ((u + 40 * (row % 2)) % 80.0) - 40.0
    cv = (np.abs(v) % 64.0) - 32.0
    pad = cu ** 2 / 15.0 ** 2 + cv ** 2 / 11.0 ** 2 < 1
    toes = np.zeros_like(pad)
    for dx in (-9, 0, 9):
        toes |= (cu - dx) ** 2 + (cv + 15) ** 2 < 14
    mark = pad | toes
    return np.where(mark[..., None], base * 0.3 + np.array([20, 26, 46]) * 0.7, base)


# ===================================================================================================== look
class Look(base.Look):
    shards = ['body'] * 6 + ['scroll'] * 1 + ['endpin'] * 2 + ['string'] * 4
    spark_big = ('slap', 'trample', 'stampede')
    spark_flash = ('slap',)
    flash_hits = ('slap', 'trample')
    fringe_hits = ('slap', 'trample')
    quiet_hits = ()
    rope_kinds = {'slap': 0.7}
    sprite_k = 0.84                                   # x the bridge's 1.2: it nearly fills its glass
    title_pattern = staticmethod(title_pattern)

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.STEPS = [dict(t0=e[0], t1=e[7], p0=np.array([e[3], e[4]]), p1=np.array([e[5], e[6]]), kind=e[8])
                      for e in EV if e[1] == 'step' and e[2] == k]
        self.STEP_T = np.array([s['t0'] for s in self.STEPS]) if self.STEPS else np.zeros(0)
        self.LAND = [(e[0], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'step_land' and e[2] == k]
        self.SLAPS = [(e[0], e[3]) for e in EV if e[1] == 'slap' and e[2] == k]
        self.STAMP = [(e[0], e[3]) for e in EV if e[1] == 'stampede' and e[2] == k]
        self.STOMPS = [(e[0], e[3], e[4], e[5]) for e in EV if e[1] == 'stampede_stomp' and e[2] == k]
        self.TRAMPLE = [(e[0], e[3], e[4]) for e in EV if e[1] == 'trample' and e[2] == k]

    def sprite(self, r, yaw, roll):
        return padded_sprite(self.cfg['pp'], r, yaw, roll)

    def title_closeup(self, r):
        yc, sw, roll = self.cfg['view']
        return padded_sprite(self.cfg['pp'], r, float(yc), float(roll))

    # ---------------------------------------------------------------- clocks
    def show(self, t):
        return self.T.T_FIGHT <= t < self.P.T_HS + 0.12

    def ta(self, t):
        return min(t, self.P.T_KO)

    def step_at(self, t):
        if not len(self.STEP_T) or not (self.T.T_FIGHT <= t < self.P.T_KO):
            return None
        j = int(np.searchsorted(self.STEP_T, t, 'right')) - 1
        if j >= 0 and t < self.STEPS[j]['t1']:
            s = self.STEPS[j]
            return s, (t - s['t0']) / max(1e-6, s['t1'] - s['t0'])
        return None

    def stamp_at(self, t):
        tc = self.ta(t)
        for t0, t1 in self.STAMP:
            if t0 <= tc < t1:
                return t0, t1
        return None

    # ---------------------------------------------------------------- state
    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k or phase != 'fight':
            return
        s = self.step_at(t)
        if s is not None:                                       # a heavy step: a small lift, set down hard
            st['lift'] = 0.10 * math.sin(math.pi * s[1]) * (0.7 if s[0]['kind'] else 1.0)
        tc = self.ta(t)
        for tl, x, y, reach, kind in self.LAND[max(0, self._land_i(tc) - 1):self._land_i(tc) + 1]:
            if 0 <= tc - tl < 0.12:
                st['glow'] = max(st['glow'], 0.35 * (1 - (tc - tl) / 0.12))
        sp = self.stamp_at(t)
        if sp is not None:
            st['glow'] = max(st['glow'], 0.3)
            st['lift'] = max(st['lift'], 0.05 * abs(math.sin(2 * math.pi * (tc - sp[0]) / self.T.BEAT)))

    def _land_i(self, tc):
        if not hasattr(self, '_LT'):
            self._LT = np.array([l[0] for l in self.LAND]) if self.LAND else np.zeros(0)
        return int(np.searchsorted(self._LT, tc, 'right'))

    # ---------------------------------------------------------------- events -> camera, ropes, sparks
    def shake(self, e):
        if e[1] == 'step_land':
            return 0.35 if e[6] == 0 else 0.25
        if e[1] == 'stampede_stomp':
            return 0.7
        if e[1] == 'slap':
            return 1.2 if e[3] else 0.9
        if e[1] == 'trample':
            return 1.8
        return None

    def rope(self, e):
        if e[1] == 'ability':
            return self.rope_kinds.get(e[4])
        if e[1] == 'stampede_stomp':
            return 0.3
        return None

    def spark(self, e):
        how = e[7]
        if how == 'slap':
            return np.array([210, 230, 255], float), 16, 3.4, 0.3
        if how in ('trample', 'stampede'):
            return np.array([200, 190, 176], float), 14, 2.8, 0.34
        return np.array([170, 190, 230], float), 6, 1.8, 0.2

    def hit_glow(self, e):
        if e[7] in ('slap', 'trample'):
            return 15, 0.4
        if e[7] == 'step':
            return 9, 0.28
        return None

    # ---------------------------------------------------------------- effects
    def fx_setup(self):
        P, k = self.P, self.k
        col = P.FL_COL[k]
        for tl, x, y, reach, kind in self.LAND:
            if tl > P.T_KO:
                break
            pos = P.up(x, y, 0.08)
            P.PA.emit(tl, pos, 'dust', n=5 if kind == 0 else 4, col=DUST)
            P.MK.add(tl, 'dust', x, y, size=0.32, life=3.0, seed=int(tl * 97) % 991, strength=0.45)
            P.FL.add(tl, P.up(x, y, R * 0.5), col, radius=1.0 + 0.4 * reach, peak=0.3, decay=0.14)
        for t0, broke in self.SLAPS:
            if t0 > P.T_KO:
                break
            p = P.RUN['P'][P.sim_i(t0), k]
            pos = P.up(p[0], p[1], R)
            P.BU.add(t0, pos, (220, 236, 255), 1.3 if broke else 1.0)
            P.FL.add(t0, pos, col, radius=2.0, peak=0.8, decay=0.22)
            P.PA.emit(t0, pos, 'sparks', n=14, col=[(230, 240, 255), (150, 180, 240), (255, 255, 255)])
        for t0, x, y, rmax in self.STOMPS:
            if t0 > P.T_KO:
                break
            pos = P.up(x, y, 0.1)
            P.PA.emit(t0, pos, 'dust', n=10, col=DUST, speed=(0.5, 1.3))
            P.MK.add(t0, 'crack', x, y, size=0.6, life=5.0, seed=int(t0 * 53) % 997, strength=0.7)
            P.FL.add(t0, P.up(x, y, R * 0.5), col, radius=1.8, peak=0.45, decay=0.18)
        for t0, x, y in self.TRAMPLE:
            if t0 > P.T_KO:
                break
            pos = P.up(x, y, R)
            P.BU.add(t0, pos, P.COL[k], 1.5)
            P.FL.add(t0, pos, col, radius=2.2, peak=0.9, decay=0.3)

    def fx_event(self, e):
        P = self.P
        if e[1] != 'hit':
            return
        pos = S.to_world(e[5], e[6], R)
        if e[7] == 'step':
            P.BU.add(e[0], pos, P.COL[e[2]], 0.55, bright=False)
        elif e[7] == 'slap':
            P.BU.add(e[0], pos, (230, 240, 255), 0.9)

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'sparks', n=28, spread=180, speed=(0.8, 2.4), col=[(180, 200, 240), (230, 236, 250)])
        P.PA.emit(P.T_KO, pos, 'chips', n=12, col=[tuple(WOOD), (28, 22, 24), tuple(STEEL)])

    # ---------------------------------------------------------------- the canvas: quake rings
    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if owner != self.k or kind not in ('step', 'stampede', 'slap'):
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)
        wid = {'step': 0.12, 'stampede': 0.18, 'slap': 0.08}[kind]
        gain = {'step': 0.75, 'stampede': 1.0, 'slap': 1.2}[kind]
        band = np.clip(1 - np.abs(rd - r) / wid, 0, 1) * (1 - r / rmax) ** 0.6
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        tint = (self.P.COL[self.k] / 255.0 * 0.8 + 0.2) if kind != 'slap' else np.array([0.85, 0.92, 1.0])
        Ic += (band * gain)[:, None] * tint[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass: ears behind, trunk in front
    def items_ball(self, F, st):
        if self.stamp_at(F.t) is None or not st['vis']:
            return []
        p = st['p']
        dz = F.cam.depth(S.to_world(p[0], p[1], R + st['lift']))[0]
        return [(dz + 0.02, ('ears', st)), (dz - 0.02, ('trunk', st))]

    def draw_item(self, F, payload):
        what, st = payload
        fr, cam, P = F.fr, F.cam, self.P
        arr, depth = fr.array(), fr.depth()
        p = st['p']
        h = R + st['lift']
        c = cam.wp(S.to_world(p[0], p[1], h)[None])[0]
        rp = R * cam.f / cam.depth(S.to_world(p[0], p[1], h))[0]
        t = F.t
        if what == 'ears':                                    # big flapping ears either side
            flap = math.sin(2 * math.pi * t * 2.0)
            for side in (-1, 1):
                a0 = side * (0.15 + 0.12 * flap)
                pts = []
                for j in range(9):
                    th = -1.1 + 2.2 * j / 8
                    rr = rp * (1.55 + 0.3 * math.cos(th * 1.3))
                    pts.append((c[0] + side * (rp * 0.6 + rr * 0.55 * math.cos(th + a0)), c[1] + rr * 0.6 * math.sin(th)))
                arr = blend(arr, _mask([pts]), INK, 0.8)
                inner = [(c[0] + (x - c[0]) * 0.92, c[1] + (y - c[1]) * 0.92) for x, y in pts]
                arr = blend(arr, _mask([inner]), EAR, 1.0)
                pink = [(c[0] + (x - c[0]) * 0.78 + side * 2, c[1] + (y - c[1]) * 0.7) for x, y in pts]
                arr = blend(arr, _mask([pink]), EAR_IN, 0.6)
        else:                                                  # the trunk swinging ahead, curling up at the end
            v = P.RUN['V'][P.sim_i(min(t, P.T_KO)), self.k]
            hd = math.atan2(v[1], v[0])
            swing = 0.5 * math.sin(2 * math.pi * t * 1.6)
            pts = []
            for j in range(12):                                # out low in front, then curling up and back at the tip
                u = j / 11
                a = hd + swing * u + 1.4 * u ** 3 * (1 if swing >= 0 else -1)
                d = R * 0.8 + 0.7 * u - 0.25 * u ** 4
                pts.append([p[0] + d * math.cos(a), p[1] + d * math.sin(a), h - 0.25 * u + 0.6 * u ** 3])
            sp = cam.wp(S.to_world(*np.array(pts).T))
            arr = blend(arr, _mask([], [sp], 9), INK, 0.85)
            arr = blend(arr, _mask([], [sp], 6), EAR, 1.0)
            arr = blend(arr, _mask([], [sp[1:]], 1), EAR * 0.75, 0.8)
        fr.set_array(arr, depth)

    # ---------------------------------------------------------------- effects over the ring
    def over_ring(self, F, arr, depth):
        P, cam, k = self.P, F.cam, self.k
        t = F.t
        col = P.COL[k]
        if F.in_fight:
            for owner, cx, cz, r, rmax, kind, t0 in P.RUN['WAVES'][F.i]:
                if owner != k or kind not in ('step', 'stampede', 'slap'):
                    continue
                kk = (1 - r / rmax) ** 0.7
                ring = jag_ring(cx, cz, r, int(t0 * 1000) % 9973, amp=0.04 if kind == 'step' else 0.07)
                sp = cam.wp(S.to_world(ring[:, 0], ring[:, 1], ring[:, 2]))
                arr = blend(arr, _mask([], [sp], 3), INK, 0.5 * kk)
                lc = np.clip(col * 0.6 + 100, 0, 255) if kind != 'slap' else np.array([235, 245, 255], float)
                arr = blend(arr, _mask([], [sp], 1), lc, kk)
        tc = self.ta(t)
        st = F.fs[k]
        for t0, broke in self.SLAPS:                             # SLAP BASS: a string snapping across the glass
            age = tc - t0
            if 0 <= age < 0.16 and st['vis']:
                p = st['p']
                kk = 1 - age / 0.16
                wob = 0.12 * math.sin(age * 120) * kk
                Z = np.array([[p[0] - R * 0.9, p[1] + wob, R * 1.2], [p[0], p[1] - wob, R * 1.35], [p[0] + R * 0.9, p[1] + wob, R * 1.2]])
                sp = cam.wp(S.to_world(Z[:, 0], Z[:, 1], Z[:, 2]))
                arr = blend(arr, _mask([], [sp], 3), INK, 0.6 * kk)
                arr = blend(arr, _mask([], [sp], 1), WHITE, kk)
        return arr, depth

    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'stampede' and owner == self.k:
            return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4, 2.2, t, k=1 - r / rmax, seed=owner)
        return img

    # ---------------------------------------------------------------- its parts (K.O. shatter)
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c = np.array([sx_, sz_])
        if kind == 'string':
            pts = [c + d * (j - 2) * 0.14 + nn * curl * 0.07 * math.sin(j * 1.5) for j in range(5)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            return blend(arr, _mask([], [sp], 1), np.array([206, 200, 188], float), 0.9)
        if kind == 'endpin':
            pts = [c - d * 0.16, c + d * 0.16]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            return blend(arr, _mask([], [sp], 2), STEEL, 1.0)
        if kind == 'scroll':
            a = np.linspace(0, 4 * math.pi, 16)
            rr = 0.02 + 0.012 * a
            pts = [c + np.array([math.cos(x + rot), math.sin(x + rot)]) * r_ for x, r_ in zip(a, rr)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            return blend(arr, _mask([], [sp], 2), np.array([152, 92, 50], float), 1.0)
        sz = size * 1.7
        tri = [c + d * sz, c - d * sz * 0.7 + nn * sz * 0.8, c - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        P.layer_blend(arr, [sp], WOOD * (0.75 + 0.35 * abs(math.cos(rot))), 1.0)
        return blend(arr, _mask([], [np.vstack([sp, sp[:1]])], 1), np.array([50, 22, 12], float), 0.8)


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.6, 0.18
    band = (40, 260)                                        # the walking line's fundamentals: the bed has no bass
    colour_args = dict(cut0=30, cut_span=60, drive=2.5)

    def __init__(self, M, k):
        super().__init__(M, k)
        import pp_bridge as B
        self.B = B
        self.mod = B.module('contrabass_lowpoly')
        self.pan = -0.1 if k == 0 else 0.1
        lo, hi = M.CFG[k]['register']
        self.lo, self.hi = lo, hi
        self._c = {}
        self.g0 = M.inst_gain(M.CFG[k]['pp'])

    # ---------------------------------------------------------------- the voice: arco, pizz, slap (the series' synth)
    def art(self, how, m, dur, vel=0.9):
        key = (how, None if m is None else int(m), round(dur, 3), round(vel, 2))
        if key not in self._c:
            f = None if m is None else self.mod._midi_hz(int(m))
            if how == 'arco':
                x = self.mod.bowed(f, dur, vel)
            elif how == 'slap':
                x = self.mod.slap(f, dur, vel)
            else:
                x = self.mod.pizz(f, dur, vel)
            self._c[key] = self.B._resample(np.asarray(x, float)) * self.g0
        return self._c[key]

    def voice(self, m, dur, vel=0.85):
        return self.art('pizz' if dur < 0.5 else 'arco', m, dur, vel)

    def fit_reg(self, m, lo=None, hi=None):
        lo, hi = lo or self.lo, hi or self.hi
        while m > hi:
            m -= 12
        while m < lo:
            m += 12
        return m

    def walk_note(self, t):
        return self.fit_reg(self.T.line_at(self.name, t), self.lo, self.lo + 24)

    def root_of(self, t):
        return self.T.root_above(self.T.chord_at(t), self.lo + 4)

    def col(self, x, t, **kw):
        return self.colour(x, t, low=True, **kw)

    def elephant_notes(self, t0):
        T = self.T
        tonic = self.root_of(0.0)
        out = []
        scale = [(T.KEY_PC + s) % 12 for s in T.SCALE]
        for bt, semi, d in ELEPHANT:
            m = tonic + semi
            if semi % 12 == 4 and (T.KEY_PC + 4) % 12 not in scale:
                m -= 1
            if semi % 12 == 9 and (T.KEY_PC + 9) % 12 not in scale:
                m -= 1
            out.append((t0 + bt * T.BEAT, self.fit_reg(m, self.lo, self.lo + 26), d * T.BEAT))
        return out

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """a slapped bass hit (config TITLE_HITS: its notes, else the tonic) with two ghost slaps after it"""
        hits = self.T.TITLE_HITS.get(self.name) or (self.root_of(0.0),)
        mix.place(self.bus, self.art('slap', hits[0], 0.5, 1.0), t, 0.62, pan=self.pan)
        mix.place(self.bus, self.art('arco', hits[0], 0.8, 0.9), t, 0.30, pan=self.pan)
        for j, m in enumerate(hits[1:]):
            mix.place(self.bus, self.art('pizz', m, 0.12, 0.9), t + 0.08 + 0.07 * j, 0.32, pan=self.pan)
        for j in (1, 2):
            mix.place(self.bus, self.art('slap', None, 0.08, 0.8), t + j * self.T.S16, 0.22, pan=self.pan)
        T = self.T
        if t + T.BEAT < T.BAR - 0.05:                            # it keeps walking under the card's last beat
            r = hits[0]
            mix.place(self.bus, self.art('pizz', r + 7, T.BEAT * 0.45, 0.9), t + T.BEAT, 0.42, pan=self.pan)
            mix.place(self.bus, self.art('pizz', r + 11, T.BEAT * 0.45, 0.9), t + 1.5 * T.BEAT, 0.42, pan=self.pan)

    def count_in(self, mix):
        """it walks the count-in bar (the groove from bar 1): root, third, fifth, then a chromatic step to FIGHT!'s root"""
        T = self.T
        ch = T.CHORDS[1]
        r = self.root_of(T.BAR)
        pcs = T.CHORD_PCS[ch]
        nxt = self.walk_note(T.T_FIGHT)
        notes = [r, r + (pcs[1] - pcs[0]) % 12, r + (pcs[2] - pcs[0]) % 12, nxt + (1 if nxt <= r + 6 else -1)]
        for j, m in enumerate(notes):
            mix.place(self.bus, self.art('pizz', self.fit_reg(m), T.BEAT * 0.9, 0.95), T.BAR + j * T.BEAT, 0.5,
                      pan=self.pan)

    def play(self, mix):
        M, T, k, A = self.M, self.T, self.k, self.M.A
        q16 = M.q16
        th = M.Thinner(470 + k)
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if e[2] != k:
                continue
            tq = q16(t)
            if kind == 'step_land':                               # a footfall = a note of the walking line
                hp01 = M.hp_at(k, t) / 100.0
                if not th.keep(round(tq / T.S16), hp01):
                    continue
                on_beat = abs(tq / T.BEAT - round(tq / T.BEAT)) < 1e-6
                if on_beat:
                    m, dur = self.walk_note(tq), T.BEAT * (0.9 if e[6] == 0 else 0.45)
                else:                                             # DOUBLE TIME's off-8th: a chromatic approach
                    nxt = self.walk_note(tq + T.BEAT / 2)
                    m, dur = nxt - 1 if nxt > self.lo else nxt + 1, T.BEAT * 0.45
                sig, g, _ = self.col(self.art('pizz', m, dur, 0.95), t)
                mix.place(self.bus, sig, tq, 0.56 * g, pan=self.pan)
                mix.place('fx', A.thud(0.18, 70, 40, seed=int(t * 13)), tq, 0.07)
            elif kind == 'hit' and e[7] == 'step':                # a landed quake: a ghost slap on top
                mix.place(self.bus, self.art('slap', None, 0.08, 0.7), tq, 0.16, pan=self.pan)
            elif kind == 'slap':                                  # SLAP BASS: slap-a-slap
                m = self.walk_note(tq)
                mix.place(self.bus, self.art('slap', m, 0.3, 1.0), tq, 0.52, pan=self.pan)
                for j in (1, 2):
                    mix.place(self.bus, self.art('slap', None, 0.08, 0.85), tq + j * T.BEAT / 3, 0.24, pan=self.pan)
                if e[3]:                                          # it broke a grab: an arco snarl under it
                    mix.place(self.bus, self.col(self.art('arco', m, 0.4, 1.0), t, drive_k=2.0)[0], tq, 0.30,
                              pan=self.pan)
            elif kind == 'ability' and e[4] == 'stampede':        # THE ELEPHANT: the waltz tune, bowed and heavy
                for ts, m, d in self.elephant_notes(tq):
                    if ts > M.T_KO + 0.05:
                        break
                    sig, g, _ = self.col(self.art('arco', m, d * 0.95, 1.0), ts, drive_k=1.5)
                    mix.place(self.bus, sig, ts, 0.50 * g, pan=self.pan * 0.5)
            elif kind == 'stampede_stomp':
                mix.place('fx', A.thud(0.5, 90, 28, seed=int(t * 11)), tq, 0.26)
                mix.place(self.bus, self.art('slap', None, 0.08, 0.7), tq, 0.14, pan=self.pan)
            elif kind == 'trample':
                mix.place('fx', A.thud(0.7, 110, 30, seed=int(t * 17)), tq, 0.36)

    def fall_apart(self, mix):
        """the strings give way: a sagging low pizz and a rattle down the fingerboard"""
        M, A = self.M, self.M.A
        t0 = M.P.T_HS
        r = self.root_of(t0)
        mix.place(self.bus, M.bend(self.art('arco', r, 1.3, 1.0), -7.0, 0.1, 1.2), t0, 0.5, pan=self.pan)
        for j in range(6):
            mix.place(self.bus, self.art('slap', None, 0.06, 0.7 - 0.08 * j), t0 + 0.05 + 0.06 * j, 0.14, pan=self.pan)

    def stinger(self, mix, tw):
        r = self.root_of(0.0)
        mix.place('solo', self.art('slap', r + 12, 0.4, 1.0), tw, 0.5)
        mix.place('solo', self.art('arco', r, 1.2, 1.0), tw, 0.42)

    def solo(self, mix, t0):
        """config SOLO: a walking-bass solo, plucked, slapped on the off-beat accents"""
        T, M = self.T, self.M
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            how = 'arco' if d >= 1.5 else ('slap' if (bt * 2) % 2 == 1 else 'pizz')
            mix.place('solo', self.art(how, m, max(0.1, d * T.BEAT * 0.95), 0.95), ts, 0.62)
            if how != 'arco':                                      # the octave ringing under the plucked line
                mix.place('solo', self.art('pizz', m - 12 if m - 12 >= self.lo else m, max(0.1, d * T.BEAT), 0.8),
                          ts, 0.26)

    def reassemble(self, mix):
        T, M = self.T, self.M
        r = self.root_of(0.0)
        run = [r, r + 3, r + 7, r + 10, r + 12, r + 15, r + 19, r + 24]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place('solo', self.art('pizz', self.fit_reg(m, self.lo, self.lo + 30), 0.12, 0.85), tj, 0.3)

    def last_bar(self, mix):
        """the walking line climbs into the title slam: four steps up to the tonic on frame 0"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        tgt = self.root_of(0.0) + 12
        tgt = self.fit_reg(tgt, self.lo + 5, self.lo + 20)
        for bt in range(4):
            m = tgt - 4 + bt if bt else tgt - 5
            mix.place(self.bus, self.art('pizz', m, T.BEAT * 0.9, 0.95), t_last + bt * T.BEAT, 0.52, pan=self.pan)
